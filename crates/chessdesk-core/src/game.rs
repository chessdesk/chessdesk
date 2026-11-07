//! A game: where it started, its moves and whatever has been learnt about them.

use std::time::Duration;

use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::{CastlingMode, Chess, EnPassantMode, KnownOutcome, Move, Outcome, Position};

use crate::{Annotations, Eval, Wdl};

/// Where a game keeps the likelihood of its starting position, which no move carries.
const START_WDL_TAG: &str = "ChessdeskStartWdl";

/// How a game ended, where the moves or tags say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    Checkmate,
    Stalemate,
    InsufficientMaterial,
    Time,
    Resignation,
}

/// A move of a game, with what is known about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayedMove {
    pub played: Move,
    pub san: SanPlus,
    /// Numeric annotation glyphs: 2 is `?`, 4 is `??`.
    pub nags: Vec<u8>,
    pub annotations: Annotations,
    pub eval: Eval,
    /// The mover's time left after the move.
    pub clock: Option<Duration>,
}

/// A game of standard chess.
#[derive(Clone, Debug)]
pub struct Game {
    /// PGN tags other than `Result`, `FEN` and `SetUp`, which follow from the game.
    pub tags: Vec<(String, String)>,
    start: Chess,
    moves: Vec<PlayedMove>,
    /// Set when a game ends off the board: resignation, time, agreement.
    adjudicated: Option<KnownOutcome>,
}

impl Default for Game {
    fn default() -> Self {
        Self::from_position(Chess::default())
    }
}

impl Game {
    pub fn from_position(start: Chess) -> Self {
        Self {
            tags: Vec::new(),
            start,
            moves: Vec::new(),
            adjudicated: None,
        }
    }

    /// Reads a FEN into a position to start from.
    pub fn parse_fen(fen: &str) -> Option<Chess> {
        fen.parse::<Fen>()
            .ok()?
            .into_position(CastlingMode::Standard)
            .ok()
    }

    pub fn start(&self) -> &Chess {
        &self.start
    }

    pub fn moves(&self) -> &[PlayedMove] {
        &self.moves
    }

    pub fn moves_mut(&mut self) -> &mut [PlayedMove] {
        &mut self.moves
    }

    /// The position after `ply` moves, capped at the last one.
    pub fn position_at(&self, ply: usize) -> Chess {
        let mut position = self.start.clone();
        for played in self.moves.iter().take(ply) {
            position.play_unchecked(played.played);
        }
        position
    }

    pub fn position(&self) -> Chess {
        self.position_at(self.moves.len())
    }

    /// The current position as FEN.
    pub fn fen(&self) -> String {
        fen_of(&self.position())
    }

    /// Plays a move if it is legal in the current position and the game is still on.
    pub fn play(&mut self, played: Move) -> bool {
        let position = self.position();
        if self.outcome().is_known() || !position.is_legal(played) {
            return false;
        }
        self.moves.push(PlayedMove {
            played,
            san: SanPlus::from_move(position, played),
            nags: Vec::new(),
            annotations: Annotations::default(),
            eval: Eval::default(),
            clock: None,
        });
        true
    }

    /// Takes back the last `count` moves, and with them any result.
    pub fn take_back(&mut self, count: usize) {
        self.moves.truncate(self.moves.len().saturating_sub(count));
        self.adjudicated = None;
    }

    /// Ends the game off the board.
    pub fn adjudicate(&mut self, outcome: KnownOutcome) {
        self.adjudicated = Some(outcome);
    }

    /// The result, by adjudication or else from the board. Draws by repetition and the
    /// 50-move rule need claiming, so only the board's own ends count here.
    pub fn outcome(&self) -> Outcome {
        match self.adjudicated {
            Some(outcome) => Outcome::Known(outcome),
            None => self.position().outcome(),
        }
    }

    /// How the game ended, if the moves or tags say.
    pub fn ending(&self) -> Option<Ending> {
        let position = self.position();
        if position.is_checkmate() {
            Some(Ending::Checkmate)
        } else if position.is_stalemate() {
            Some(Ending::Stalemate)
        } else if position.is_insufficient_material() {
            Some(Ending::InsufficientMaterial)
        } else if self.tag("Termination") == Some("Time forfeit") {
            Some(Ending::Time)
        } else if self.adjudicated.is_some_and(KnownOutcome::is_decisive) {
            Some(Ending::Resignation)
        } else {
            None
        }
    }

    /// The likelihood before any move, once an engine has looked at the start.
    pub fn start_wdl(&self) -> Option<Wdl> {
        self.tag(START_WDL_TAG).and_then(Wdl::parse)
    }

    pub fn set_start_wdl(&mut self, wdl: Wdl) {
        self.set_tag(START_WDL_TAG, wdl.to_string());
    }

    pub fn tag(&self, name: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn set_tag(&mut self, name: &str, value: impl Into<String>) {
        let value = value.into();
        match self.tags.iter_mut().find(|(key, _)| key == name) {
            Some((_, old)) => *old = value,
            None => self.tags.push((name.to_owned(), value)),
        }
    }
}

/// A position as FEN.
pub fn fen_of(position: &Chess) -> String {
    Fen::from_position(position, EnPassantMode::Legal).to_string()
}

#[cfg(test)]
mod tests {
    use shakmaty::Color;
    use shakmaty::uci::UciMove;

    use super::*;

    fn play(game: &mut Game, uci: &str) -> bool {
        let parsed: UciMove = uci.parse().unwrap();
        let played = parsed.to_move(&game.position()).unwrap();
        game.play(played)
    }

    #[test]
    fn fools_mate_ends_the_game() {
        let mut game = Game::default();
        for uci in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            assert!(play(&mut game, uci));
        }
        assert_eq!(game.moves()[3].san.to_string(), "Qh4#");
        assert_eq!(
            game.outcome(),
            Outcome::Known(KnownOutcome::Decisive {
                winner: Color::Black
            })
        );
    }

    #[test]
    fn a_game_knows_how_it_ended() {
        let mut game = Game::default();
        assert_eq!(game.ending(), None);
        game.adjudicate(KnownOutcome::Decisive {
            winner: Color::White,
        });
        assert_eq!(game.ending(), Some(Ending::Resignation));
        game.set_tag("Termination", "Time forfeit");
        assert_eq!(game.ending(), Some(Ending::Time));

        let mut mated = Game::default();
        for uci in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            assert!(play(&mut mated, uci));
        }
        assert_eq!(mated.ending(), Some(Ending::Checkmate));
    }

    #[test]
    fn the_start_likelihood_survives_in_the_tags() {
        let mut game = Game::default();
        assert_eq!(game.start_wdl(), None);
        let wdl = Wdl {
            white: 52,
            draw: 942,
            black: 6,
        };
        game.set_start_wdl(wdl);
        assert_eq!(game.start_wdl(), Some(wdl));
    }

    #[test]
    fn taking_back_reopens_a_finished_game() {
        let mut game = Game::default();
        assert!(play(&mut game, "e2e4"));
        game.adjudicate(KnownOutcome::Draw);
        game.take_back(1);
        assert_eq!(game.outcome(), Outcome::Unknown);
        assert_eq!(game.position(), Chess::default());
    }

    #[test]
    fn no_moves_after_resignation() {
        let mut game = Game::default();
        game.adjudicate(KnownOutcome::Decisive {
            winner: Color::White,
        });
        let legal = game.position().legal_moves()[0];
        assert!(!game.play(legal));
    }
}
