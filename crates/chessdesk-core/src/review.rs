//! What an engine's evaluations say about the moves: how much each one cost, and which
//! moments of a game deserve an LLM's attention.

use shakmaty::{Color, Position};

use crate::{Game, PlayedMove, Wdl};

/// Glyphs for a move, by how much expected score it gave away.
const MISTAKES: [(f32, u8); 3] = [(0.30, 4), (0.15, 2), (0.07, 6)];

/// How much of the mover's expected score a move gave away, from 0 to 1. `before` is the
/// position's value for White ahead of the move.
pub fn loss(before: Wdl, after: Wdl, mover: Color) -> f32 {
    let drop = before.expected() - after.expected();
    (if mover == Color::White { drop } else { -drop }).max(0.)
}

/// The glyph a loss earns: `??`, `?`, `?!` or none.
pub fn glyph(loss: f32) -> Option<u8> {
    MISTAKES
        .iter()
        .find(|(threshold, _)| loss >= *threshold)
        .map(|(_, nag)| *nag)
}

/// How good a move was, for a tutor to say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Perfect,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
}

impl Verdict {
    /// The verdict on a move that lost `loss`, which `is_best` says the engine agrees with.
    pub fn of_loss(loss: f32, is_best: bool) -> Self {
        match glyph(loss) {
            Some(4) => Self::Blunder,
            Some(2) => Self::Mistake,
            Some(_) => Self::Inaccuracy,
            None if is_best => Self::Perfect,
            None => Self::Good,
        }
    }

    /// The glyph kept in the PGN, where a good move needs none.
    pub fn nag(self) -> Option<u8> {
        match self {
            Self::Perfect => Some(1),
            Self::Good => None,
            Self::Inaccuracy => Some(6),
            Self::Mistake => Some(2),
            Self::Blunder => Some(4),
        }
    }

    /// Reads the verdict back off a move that has been evaluated.
    pub fn of_move(played: &PlayedMove) -> Option<Self> {
        let glyph = |nag: u8| played.nags.contains(&nag);
        played.eval.wdl?;
        Some(match () {
            () if glyph(4) => Self::Blunder,
            () if glyph(2) => Self::Mistake,
            () if glyph(6) => Self::Inaccuracy,
            () if glyph(1) => Self::Perfect,
            () => Self::Good,
        })
    }
}

/// What each move lost, as `(index into moves, loss)`, for moves with an evaluation on
/// both sides. A start nothing has evaluated counts as even.
pub fn losses(game: &Game) -> Vec<(usize, f32)> {
    let even = Wdl {
        white: 0,
        draw: 1000,
        black: 0,
    };
    let mut before = Some(game.start_wdl().unwrap_or(even));
    let mut mover = game.start().turn();
    let mut found = Vec::new();
    for (index, played) in game.moves().iter().enumerate() {
        let after = played.eval.wdl;
        if let (Some(before), Some(after)) = (before, after) {
            found.push((index, loss(before, after, mover)));
        }
        before = after;
        mover = mover.other();
    }
    found
}

/// The `count` costliest moves in game order: what a review should look at, so that
/// it reads a handful of moments instead of every move.
pub fn critical_moves(game: &Game, count: usize) -> Vec<usize> {
    let mut costly = losses(game);
    costly.retain(|(_, loss)| glyph(*loss).is_some());
    costly.sort_by(|a, b| b.1.total_cmp(&a.1));
    costly.truncate(count);
    let mut moves: Vec<_> = costly.into_iter().map(|(index, _)| index).collect();
    moves.sort_unstable();
    moves
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Eval;

    fn game_with(values: &[(u16, u16, u16)]) -> Game {
        let mut game = Game::default();
        for (white, draw, black) in values {
            let played = game.position().legal_moves()[0];
            assert!(game.play(played));
            let last = game.moves_mut().last_mut().unwrap();
            last.eval = Eval {
                score: None,
                wdl: Some(Wdl {
                    white: *white,
                    draw: *draw,
                    black: *black,
                }),
            };
        }
        game
    }

    #[test]
    fn grades_by_lost_expected_score() {
        assert_eq!(glyph(0.5), Some(4));
        assert_eq!(glyph(0.2), Some(2));
        assert_eq!(glyph(0.1), Some(6));
        assert_eq!(glyph(0.01), None);
    }

    #[test]
    fn a_verdict_follows_the_loss_and_the_engine() {
        assert_eq!(Verdict::of_loss(0., true), Verdict::Perfect);
        assert_eq!(Verdict::of_loss(0.02, false), Verdict::Good);
        assert_eq!(Verdict::of_loss(0.1, false), Verdict::Inaccuracy);
        assert_eq!(Verdict::of_loss(0.2, false), Verdict::Mistake);
        assert_eq!(Verdict::of_loss(0.5, true), Verdict::Blunder);
    }

    #[test]
    fn a_verdict_survives_in_the_glyphs() {
        let mut game = game_with(&[(0, 1000, 0)]);
        let played = &mut game.moves_mut()[0];
        assert_eq!(Verdict::of_move(played), Some(Verdict::Good));
        played.nags.extend(Verdict::Blunder.nag());
        assert_eq!(Verdict::of_move(played), Some(Verdict::Blunder));
        played.eval.wdl = None;
        assert_eq!(Verdict::of_move(played), None);
    }

    #[test]
    fn a_gain_for_the_mover_is_no_loss() {
        // White improves, then Black lets White improve further.
        let game = game_with(&[(600, 400, 0), (900, 100, 0)]);
        let losses = losses(&game);
        assert!(losses[0].1.abs() < f32::EPSILON);
        assert!((losses[1].1 - 0.15).abs() < 1e-5);
    }

    #[test]
    fn critical_moves_come_in_game_order_and_skip_good_ones() {
        let game = game_with(&[
            (0, 1000, 0),
            (0, 1000, 0),
            (0, 200, 800), // White gives away 0.4.
            (500, 500, 0), // Black gives away 0.65.
            (500, 500, 0),
        ]);
        assert_eq!(critical_moves(&game, 8), [2, 3]);
        assert_eq!(critical_moves(&game, 1), [3]);
    }
}
