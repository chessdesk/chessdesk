//! Shadowchess: networks that play like human players of a rating, mistakes included, run
//! here on the CPU. The weights and how the network reads a position are described at
//! <https://github.com/chessdesk/shadowchess>.
//!
//! The network gives the moves a human of the rating would likely play. Picking the likeliest
//! every time plays far above the rating, as Maia does; sampling at the temperature fitted
//! for the level makes as many mistakes as such a player.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chessdesk_core::{Annotations, Game};
use half::f16;
use ndarray::{Array1, Array2, ArrayView2, Axis, Slice};
use safetensors::{Dtype, SafeTensors};
use shakmaty::uci::UciMove;
use shakmaty::{CastlingMode, Chess, Color, EnPassantMode, Move, Position, Role, Square};

use crate::{Decision, Opponent, PlayerError};

/// The ratings there are networks calibrated for.
pub const LEVELS: [u32; 3] = [500, 1000, 1500];

const PIECE_CODES: usize = 13;
const PLANES: usize = PIECE_CODES + 4;
const FLAG_EN_PASSANT: u8 = 1 << 4;
const FLAG_CASTLING_ROOK: u8 = 1 << 5;
const FLAG_LAST_FROM: u8 = 1 << 6;
const FLAG_LAST_TO: u8 = 1 << 7;

struct Linear {
    /// `outputs × inputs`, as `PyTorch` keeps it.
    weight: Array2<f32>,
    bias: Array1<f32>,
}

impl Linear {
    fn forward(&self, x: &ArrayView2<f32>) -> Array2<f32> {
        x.dot(&self.weight.t()) + &self.bias
    }
}

struct LayerNorm {
    weight: Array1<f32>,
    bias: Array1<f32>,
}

impl LayerNorm {
    fn forward(&self, x: &Array2<f32>) -> Array2<f32> {
        let mut out = x.clone();
        for mut row in out.rows_mut() {
            let mean = row.mean().unwrap_or(0.);
            let variance = row.mapv(|v| (v - mean).powi(2)).mean().unwrap_or(0.);
            let scale = 1. / (variance + 1e-5).sqrt();
            row.mapv_inplace(|v| (v - mean) * scale);
        }
        out * &self.weight + &self.bias
    }
}

struct Block {
    norm1: LayerNorm,
    qkv: Linear,
    proj: Linear,
    /// `heads` matrices of 64 × 64, added to the attention scores.
    bias: Vec<Array2<f32>>,
    q_scale: Array1<f32>,
    k_scale: Array1<f32>,
    norm2: LayerNorm,
    mlp_in: Linear,
    mlp_out: Linear,
}

/// Scales each row to unit root mean square, then by `scale`.
fn rms_norm(x: &mut Array2<f32>, scale: &Array1<f32>) {
    for mut row in x.rows_mut() {
        let rms = (row.mapv(|v| v * v).mean().unwrap_or(0.) + 1e-6).sqrt();
        row.zip_mut_with(scale, |v, s| *v = *v / rms * s);
    }
}

fn softmax_rows(x: &mut Array2<f32>) {
    for mut row in x.rows_mut() {
        let max = row.fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        row.mapv_inplace(|v| (v - max).exp());
        let sum = row.sum();
        row.mapv_inplace(|v| v / sum);
    }
}

/// The exact GELU, as `PyTorch` has it by default.
fn gelu(x: f32) -> f32 {
    0.5 * x * (1. + libm::erff(x * std::f32::consts::FRAC_1_SQRT_2))
}

impl Block {
    fn forward(&self, x: &Array2<f32>, heads: usize) -> Array2<f32> {
        let dim = x.ncols();
        let width = dim / heads;
        let qkv = self.qkv.forward(&self.norm1.forward(x).view());
        let mut attended = Array2::<f32>::zeros(x.raw_dim());
        for head in 0..heads {
            let columns = |part: usize| {
                let start = part * dim + head * width;
                qkv.slice_axis(Axis(1), Slice::from(start..start + width))
                    .to_owned()
            };
            let (mut q, mut k, v) = (columns(0), columns(1), columns(2));
            rms_norm(&mut q, &self.q_scale);
            rms_norm(&mut k, &self.k_scale);
            let mut scores = q.dot(&k.t()) / (width as f32).sqrt() + &self.bias[head];
            softmax_rows(&mut scores);
            attended
                .slice_axis_mut(Axis(1), Slice::from(head * width..(head + 1) * width))
                .assign(&scores.dot(&v));
        }
        let x = x + &self.proj.forward(&attended.view());
        let hidden = self
            .mlp_in
            .forward(&self.norm2.forward(&x).view())
            .mapv(gelu);
        &x + &self.mlp_out.forward(&hidden.view())
    }
}

/// A trained network, with the temperature fitted for each level.
pub struct Network {
    heads: usize,
    embed: Linear,
    position: Array2<f32>,
    elo_in: Linear,
    elo_out: Linear,
    blocks: Vec<Block>,
    norm: LayerNorm,
    move_from: Linear,
    move_to: Linear,
    temperatures: BTreeMap<u32, f32>,
}

/// Reads tensors of fp16 or fp32 from the weights file.
struct Weights<'a>(SafeTensors<'a>);

impl Weights<'_> {
    fn values(&self, name: &str) -> Result<(Vec<f32>, Vec<usize>), String> {
        let view = self
            .0
            .tensor(name)
            .map_err(|error| format!("{name}: {error}"))?;
        let data = view.data();
        let values = match view.dtype() {
            Dtype::F16 => data
                .as_chunks()
                .0
                .iter()
                .map(|b| f16::from_le_bytes(*b).to_f32())
                .collect(),
            Dtype::F32 => data
                .as_chunks()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect(),
            other => return Err(format!("{name}: unexpected type {other}")),
        };
        Ok((values, view.shape().to_vec()))
    }

    fn vector(&self, name: &str) -> Result<Array1<f32>, String> {
        Ok(Array1::from(self.values(name)?.0))
    }

    fn matrix(&self, name: &str) -> Result<Array2<f32>, String> {
        let (values, shape) = self.values(name)?;
        let columns = shape.iter().skip(1).product();
        Array2::from_shape_vec((shape[0], columns), values).map_err(|error| error.to_string())
    }

    fn linear(&self, name: &str) -> Result<Linear, String> {
        Ok(Linear {
            weight: self.matrix(&format!("{name}.weight"))?,
            bias: self.vector(&format!("{name}.bias"))?,
        })
    }

    fn layer_norm(&self, name: &str) -> Result<LayerNorm, String> {
        Ok(LayerNorm {
            weight: self.vector(&format!("{name}.weight"))?,
            bias: self.vector(&format!("{name}.bias"))?,
        })
    }

    fn block(&self, index: usize) -> Result<Block, String> {
        let name = |part: &str| format!("blocks.{index}.{part}");
        let bias = self.matrix(&name("bias"))?;
        let squares = bias.ncols().isqrt();
        Ok(Block {
            norm1: self.layer_norm(&name("norm1"))?,
            qkv: self.linear(&name("qkv"))?,
            proj: self.linear(&name("proj"))?,
            bias: bias
                .rows()
                .into_iter()
                .map(|head| {
                    head.to_owned()
                        .into_shape_with_order((squares, squares))
                        .map_err(|error| error.to_string())
                })
                .collect::<Result<_, _>>()?,
            q_scale: self.vector(&name("q_scale"))?,
            k_scale: self.vector(&name("k_scale"))?,
            norm2: self.layer_norm(&name("norm2"))?,
            mlp_in: self.linear(&name("mlp.0"))?,
            mlp_out: self.linear(&name("mlp.2"))?,
        })
    }
}

impl Network {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let (_, header) = SafeTensors::read_metadata(bytes).map_err(|error| error.to_string())?;
        let metadata = header.metadata().clone().unwrap_or_default();
        let number = |key: &str| -> Result<usize, String> {
            metadata
                .get(key)
                .and_then(|value| value.parse().ok())
                .ok_or_else(|| format!("no {key} in the weights' metadata"))
        };
        let temperatures: BTreeMap<String, f32> = metadata
            .get("temperatures")
            .map(|text| serde_json::from_str(text))
            .transpose()
            .map_err(|error| error.to_string())?
            .unwrap_or_default();
        let weights = Weights(SafeTensors::deserialize(bytes).map_err(|error| error.to_string())?);
        Ok(Self {
            heads: number("heads")?,
            embed: weights.linear("embed")?,
            position: weights.matrix("position")?,
            elo_in: weights.linear("elo.0")?,
            elo_out: weights.linear("elo.2")?,
            blocks: (0..number("depth")?)
                .map(|index| weights.block(index))
                .collect::<Result<_, _>>()?,
            norm: weights.layer_norm("norm")?,
            move_from: weights.linear("move_from")?,
            move_to: weights.linear("move_to")?,
            temperatures: temperatures
                .into_iter()
                .filter_map(|(level, t)| Some((level.parse().ok()?, t)))
                .collect(),
        })
    }

    /// The temperature fitted for `level`; 1, the network's own, for one it doesn't know.
    pub fn temperature(&self, level: u32) -> f32 {
        self.temperatures.get(&level).copied().unwrap_or(1.)
    }

    /// The legal moves, each with its logit for a player rated `elo`.
    pub fn logits(&self, position: &Chess, last: Option<Move>, elo: u32) -> Vec<(Move, f32)> {
        let turn = position.turn();
        let squares = encode(position, last);
        let mut planes = Array2::<f32>::zeros((64, PLANES));
        for (square, &code) in squares.iter().enumerate() {
            planes[[square, usize::from(code & 0x0F)]] = 1.;
            for flag in 0..4 {
                planes[[square, PIECE_CODES + flag]] = f32::from((code >> (4 + flag)) & 1);
            }
        }
        let rating = Array2::from_elem((1, 1), (elo as f32 - 1100.) / 400.);
        let rating = self
            .elo_out
            .forward(&self.elo_in.forward(&rating.view()).mapv(gelu).view());
        let mut x = self.embed.forward(&planes.view()) + &self.position + &rating;
        for block in &self.blocks {
            x = block.forward(&x, self.heads);
        }
        let x = self.norm.forward(&x);
        let dim = x.ncols();
        let queries = self.move_from.forward(&x.view());
        let keys = self.move_to.forward(&x.view());
        let scale = 1. / (dim as f32).sqrt();
        position
            .legal_moves()
            .into_iter()
            .map(|m| {
                let (promotion, from, to) = move_index(m, turn);
                let query = queries.row(from);
                let query =
                    query.slice_axis(Axis(0), Slice::from(promotion * dim..(promotion + 1) * dim));
                (m, query.dot(&keys.index_axis(Axis(0), to)) * scale)
            })
            .collect()
    }
}

/// Seen from the side to move: White's squares as they are, Black's mirrored.
fn relative(square: Square, turn: Color) -> usize {
    usize::from(square) ^ if turn == Color::White { 0 } else { 56 }
}

/// A move's squares as UCI has them, the king's two-square step for castling.
fn uci_squares(m: Move) -> Option<(Square, Square, Option<Role>)> {
    match m.to_uci(CastlingMode::Standard) {
        UciMove::Normal {
            from,
            to,
            promotion,
        } => Some((from, to, promotion)),
        _ => None,
    }
}

/// Where the network scores `m`: the promotion kind, then the from and to squares.
fn move_index(m: Move, turn: Color) -> (usize, usize, usize) {
    // Every move of standard chess has its UCI form; drops and null moves don't occur.
    let (from, to, promotion) = uci_squares(m).unwrap_or((m.to(), m.to(), None));
    let kind = match promotion {
        Some(Role::Knight) => 1,
        Some(Role::Bishop) => 2,
        Some(Role::Rook) => 3,
        _ => 0,
    };
    (kind, relative(from, turn), relative(to, turn))
}

/// One byte per square: the piece code in the low four bits (empty, own pawn to king, the
/// opponent's pawn to king), flags for en passant, castling rooks and the last move above.
fn encode(position: &Chess, last: Option<Move>) -> [u8; 64] {
    let turn = position.turn();
    let mut squares = [0u8; 64];
    for (square, piece) in position.board() {
        let offset = if piece.color == turn { 0 } else { 6 };
        squares[relative(square, turn)] = piece.role as u8 + offset;
    }
    if let Some(square) = position.ep_square(EnPassantMode::Always) {
        squares[relative(square, turn)] |= FLAG_EN_PASSANT;
    }
    for square in position.castles().castling_rights() {
        squares[relative(square, turn)] |= FLAG_CASTLING_ROOK;
    }
    if let Some((from, to, _)) = last.and_then(uci_squares) {
        squares[relative(from, turn)] |= FLAG_LAST_FROM;
        squares[relative(to, turn)] |= FLAG_LAST_TO;
    }
    squares
}

/// Shadowchess at one level, as an opponent.
pub struct Shadowchess {
    network: Network,
    level: u32,
    rng: fastrand::Rng,
}

impl Shadowchess {
    pub fn load(weights: &Path, level: u32) -> Result<Self, PlayerError> {
        Ok(Self {
            network: Network::load(weights).map_err(PlayerError::Process)?,
            level,
            rng: fastrand::Rng::new(),
        })
    }

    /// A move as a player of the level would choose it, by chance in proportion to how
    /// likely they are to play it.
    fn choose(&mut self, position: &Chess, last: Option<Move>) -> Option<Move> {
        let logits = self.network.logits(position, last, self.level);
        let temperature = self.network.temperature(self.level);
        let max = logits
            .iter()
            .map(|(_, l)| *l)
            .fold(f32::NEG_INFINITY, f32::max);
        let weights: Vec<f32> = logits
            .iter()
            .map(|(_, l)| ((l - max) / temperature).exp())
            .collect();
        let mut pick = self.rng.f32() * weights.iter().sum::<f32>();
        for ((m, _), weight) in logits.iter().zip(&weights) {
            pick -= weight;
            if pick <= 0. {
                return Some(*m);
            }
        }
        logits.last().map(|(m, _)| *m)
    }
}

impl Opponent for Shadowchess {
    fn next_move(&mut self, game: &Game, _: &str) -> Result<Decision, PlayerError> {
        let last = game.moves().last().map(|played| played.played);
        let played = self
            .choose(&game.position(), last)
            .ok_or_else(|| PlayerError::Process("no legal move".to_owned()))?;
        Ok(Decision {
            played,
            annotations: Annotations::default(),
            tokens: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::Deserialize;
    use shakmaty::fen::Fen;

    use super::*;

    const TINY: &[u8] = include_bytes!("../tests/fixtures/shadowchess-tiny.safetensors");
    /// The encodings and `PyTorch`'s logits for a few positions under the tiny random network
    /// in `TINY`, both written by `python -m shadowchess.fixture` in the training project.
    const EXPECTED: &str = include_str!("../tests/fixtures/shadowchess-tiny.json");

    #[derive(Deserialize)]
    struct Case {
        fen: String,
        last: Option<String>,
        elo: u32,
        squares: Vec<u8>,
        logits: HashMap<String, f32>,
    }

    fn cases() -> Vec<(Chess, Option<Move>, Case)> {
        let cases: Vec<Case> = serde_json::from_str(EXPECTED).unwrap();
        cases
            .into_iter()
            .map(|case| {
                let position: Chess = case
                    .fen
                    .parse::<Fen>()
                    .unwrap()
                    .into_position(CastlingMode::Standard)
                    .unwrap();
                // The previous move only marks squares: it needn't be legal from anywhere here.
                let last = case.last.as_deref().map(|uci| {
                    let UciMove::Normal { from, to, .. } = uci.parse::<UciMove>().unwrap() else {
                        unreachable!()
                    };
                    Move::Normal {
                        role: Role::Pawn,
                        from,
                        capture: None,
                        to,
                        promotion: None,
                    }
                });
                (position, last, case)
            })
            .collect()
    }

    #[test]
    fn positions_are_encoded_as_in_training() {
        for (position, last, case) in cases() {
            assert_eq!(
                encode(&position, last).to_vec(),
                case.squares,
                "{}",
                case.fen
            );
        }
    }

    #[test]
    fn the_network_scores_moves_as_pytorch_does() {
        let network = Network::from_bytes(TINY).unwrap();
        for (position, last, case) in cases() {
            let logits = network.logits(&position, last, case.elo);
            assert_eq!(logits.len(), case.logits.len(), "{}", case.fen);
            for (m, logit) in logits {
                let uci = m.to_uci(CastlingMode::Standard).to_string();
                let expected = case.logits[&uci];
                assert!(
                    (logit - expected).abs() < 1e-3 * expected.abs().max(1.),
                    "{} {uci}: {logit} != {expected}",
                    case.fen
                );
            }
        }
    }

    #[test]
    fn a_played_double_step_leaves_its_en_passant_square() {
        let mut position = Chess::default();
        let push = Move::Normal {
            role: Role::Pawn,
            from: Square::E2,
            capture: None,
            to: Square::E4,
            promotion: None,
        };
        position.play_unchecked(push);
        let (_, _, after_e4) = cases().swap_remove(1);
        assert_eq!(encode(&position, Some(push)).to_vec(), after_e4.squares);
    }

    #[test]
    fn levels_play_at_their_own_temperature() {
        let network = Network::from_bytes(TINY).unwrap();
        assert!((network.temperature(500) - 1.05).abs() < 1e-6);
        assert!((network.temperature(1500) - 0.84).abs() < 1e-6);
        assert!((network.temperature(1000) - 1.).abs() < f32::EPSILON);
    }

    #[test]
    fn it_plays_legal_moves() {
        let mut player = Shadowchess {
            network: Network::from_bytes(TINY).unwrap(),
            level: 1000,
            rng: fastrand::Rng::with_seed(7),
        };
        let mut game = Game::default();
        for _ in 0..40 {
            if game.outcome().is_known() {
                break;
            }
            let decision = player.next_move(&game, "").unwrap();
            assert!(game.play(decision.played));
        }
    }
}
