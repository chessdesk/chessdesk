//! One open game. Several can be open at once: a game runs on, its opponent thinking
//! and its clock going, while another shows on the board.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use chessdesk_core::review::Verdict;
use chessdesk_core::{Annotations, Arrow, Game, Wdl};
use chessdesk_players::uci::Engine;
use chessdesk_players::{Opponent, Tokens};
use gpui::{Entity, SharedString, Subscription};
use shakmaty::san::SanPlus;
use shakmaty::{Chess, Color, Move, Outcome, Position, Square};

use super::clock::Clock;
use crate::board::{Slide, slide_paths};
use crate::i18n::{tr, tr_with};
use crate::setup::{OpponentPlan, Setup};
use crate::text_area::TextArea;

/// An even game, before anything has happened.
pub const EVEN: Wdl = Wdl {
    white: 500,
    draw: 0,
    black: 500,
};

/// Tells sessions apart for answers arriving from background threads.
pub type SessionId = u64;

/// A game being played, as opposed to one being looked at.
pub struct Live {
    pub setup: Setup,
    pub plan: OpponentPlan,
    pub player: Color,
    /// `None` until first needed, and while it thinks on a background thread.
    pub opponent: Option<Box<dyn Opponent>>,
    pub thinking: bool,
    /// Why the game stopped, until the player goes on.
    pub paused: Option<SharedString>,
    pub clock: Option<Clock>,
    /// Whether the player has heard that their time is running out.
    pub low_time_warned: bool,
    /// Stockfish for the live likelihood, if asked for.
    pub evaluator: Option<Engine>,
    pub evaluating: bool,
    /// The field for telling an agent how to play, while the game runs.
    pub instructions: Option<(Entity<TextArea>, Subscription)>,
}

/// A promotion waiting for its piece.
#[derive(Clone, Copy)]
pub struct Promotion {
    pub from: Square,
    pub to: Square,
}

pub struct Session {
    pub id: SessionId,
    pub game: Game,
    /// Which position shows, as moves from the start.
    pub ply: usize,
    pub orientation: Color,
    pub selected: Option<Square>,
    pub promotion: Option<Promotion>,
    pub live: Option<Live>,
    /// Bumped by takebacks, so that an answer to the position before is dropped.
    pub epoch: u64,
    /// Where the game is kept.
    pub path: Option<PathBuf>,
    pub tokens: Option<Tokens>,
    /// Moves analysed and moves in all, while a review runs.
    pub analysis: Option<(usize, usize)>,
    pub agent_review: bool,
    /// Pieces gliding to their squares right now. A quick reply starts while the move
    /// before it is still under way, so there can be several.
    pub slides: Vec<Slide>,
    /// The engine's move for a position, drawn while that position shows.
    pub suggestion: Option<Suggestion>,
    pub suggesting: bool,
    /// The engine's best move in each position it looked at, by ply.
    pub best_moves: BTreeMap<usize, Move>,
    /// The field for the player's notes, once there's a game to take them.
    pub notes: Option<(Entity<TextArea>, Subscription)>,
}

/// How the player's move rated in tutoring against an engine, and what it's measured by.
pub struct Feedback {
    pub verdict: Verdict,
    /// The move as it stands in the log: `14... Nf6`.
    pub numbered: String,
    /// The player's likelihood before the move and after it, from 0 to 1.
    pub likelihood: Option<(f32, f32)>,
    /// What the engine would have played instead, if not this.
    pub best: Option<String>,
}

/// A hint or the best move, and the position it's for.
#[derive(Clone, Copy)]
pub struct Suggestion {
    pub ply: usize,
    pub arrow: Arrow,
}

impl Session {
    pub fn new(id: SessionId, game: Game) -> Self {
        Self {
            id,
            game,
            ply: 0,
            orientation: Color::White,
            selected: None,
            promotion: None,
            live: None,
            epoch: 0,
            path: None,
            tokens: None,
            analysis: None,
            agent_review: false,
            slides: Vec::new(),
            suggestion: None,
            suggesting: false,
            best_moves: BTreeMap::new(),
            notes: None,
        }
    }

    pub fn at_latest(&self) -> bool {
        self.ply == self.game.moves().len()
    }

    /// Whether the player may move on the board right now.
    pub fn player_to_move(&self) -> bool {
        self.live.as_ref().is_some_and(|live| {
            !live.thinking
                && live.paused.is_none()
                && self.at_latest()
                && self.game.position().turn() == live.player
                && self.game.outcome() == Outcome::Unknown
        })
    }

    /// Whether there's no game here at all, played or loaded.
    pub fn is_blank(&self) -> bool {
        self.live.is_none() && self.game.moves().is_empty() && self.game.tags.is_empty()
    }

    /// The row of the moves' log holding the move on show. A game that starts with Black
    /// to move leaves White's half of its first row empty.
    pub fn log_row(&self) -> usize {
        let offset = usize::from(self.game.start().turn() == Color::Black);
        (self.ply + offset).saturating_sub(1) / 2
    }

    /// Who the player faces: the engine or the agent and its model.
    pub fn opponent_name(&self) -> String {
        if let Some(live) = &self.live {
            return live.setup.opponent_name();
        }
        let (white, black) = (self.game.tag("White"), self.game.tag("Black"));
        match (white, black) {
            (Some("You"), Some(name)) | (Some(name), Some("You")) => name.to_owned(),
            (Some(white), Some(black)) => {
                tr_with!("versus", white = white, black = black).to_string()
            }
            _ => tr("no-game").to_string(),
        }
    }

    /// The comment for the position on show: the move's own, or on an opponent's move
    /// without one, what was said just before.
    pub fn shown_comment(&self) -> Option<String> {
        let moves = self.game.moves();
        let shown = self.ply.min(moves.len());
        moves[..shown]
            .iter()
            .rev()
            .take(2)
            .map(|played| played.annotations.text.clone())
            .find(|text| !text.is_empty())
    }

    /// The legal moves from a square, as where they end up on the board.
    pub fn targets(&self) -> Vec<Square> {
        let Some(from) = self.selected else {
            return Vec::new();
        };
        let legal = self.game.position_at(self.ply).legal_moves();
        legal
            .iter()
            .filter(|played| played.from() == Some(from))
            .filter_map(|played| slide_paths(*played).first().map(|(_, to)| *to))
            .collect()
    }

    /// Whether the likelihood section shows: on request in a game, whenever there is
    /// something to show in a replay.
    pub fn shows_likelihood(&self) -> bool {
        match &self.live {
            Some(live) => live.setup.live_likelihood,
            None => self
                .game
                .moves()
                .iter()
                .any(|played| played.eval.wdl.is_some()),
        }
    }

    /// The latest likelihood known for the position on show. While a new evaluation is
    /// on its way this is the one before, so the bars have something to move from. Never
    /// one from later in the game: what follows says nothing about the position on show.
    pub fn known_wdl(&self) -> Option<Wdl> {
        let mut shown = self.game.moves().iter().take(self.ply).rev();
        shown
            .find_map(|played| played.eval.wdl)
            .or_else(|| self.start_wdl())
    }

    /// The likelihood to show for the start. Before a move, a normal game is simply even:
    /// the engine's numbers would read as if the game were decided by who moves first.
    /// Those stay in the game for grading the first move, and a set-up position keeps them.
    pub fn start_wdl(&self) -> Option<Wdl> {
        if *self.game.start() == Chess::default() {
            Some(EVEN)
        } else {
            self.game.start_wdl()
        }
    }

    /// The first position still waiting for its evaluation, by ply.
    pub fn next_to_evaluate(&self) -> Option<usize> {
        if self.game.start_wdl().is_none() {
            return Some(0);
        }
        let moves = self.game.moves();
        if let Some(index) = moves.iter().position(|played| played.eval.wdl.is_none()) {
            return Some(index + 1);
        }
        // In tutoring, the engine's choice where the player is to move, to tell them what
        // would have been better: a game picked up again has its evaluations but not that.
        let latest = moves.len();
        let tutored = self.live.as_ref().is_some_and(|live| {
            live.setup.grades_moves() && self.game.position().turn() == live.player
        });
        let unknown = !self.best_moves.contains_key(&latest);
        (tutored && unknown && self.game.outcome() == Outcome::Unknown).then_some(latest)
    }

    /// Whether Stockfish has work during this game.
    pub fn wants_evaluations(&self) -> bool {
        self.live
            .as_ref()
            .is_some_and(|live| live.setup.live_likelihood || live.setup.grades_moves())
    }

    /// How the player's latest move up to the one on show rated, once it has.
    pub fn latest_feedback(&self) -> Option<Feedback> {
        let live = self
            .live
            .as_ref()
            .filter(|live| live.setup.grades_moves())?;
        let moves = self.game.moves();
        let first_is_players = self.game.start().turn() == live.player;
        let mine = usize::from(!first_is_players);
        let shown = self.ply.min(moves.len());
        let index = (0..shown).rev().find(|index| index % 2 == mine)?;
        let played = &moves[index];
        let verdict = Verdict::of_move(played)?;
        let position = self.game.position_at(index);
        let before = match index.checked_sub(1) {
            Some(previous) => moves[previous].eval.wdl,
            None => self.start_wdl(),
        };
        // From the player's side: what the bar shows is White's.
        let mine = |wdl: Wdl| {
            let white = wdl.expected();
            if live.player.is_white() {
                white
            } else {
                1. - white
            }
        };
        let best = played
            .better
            .map(|better| SanPlus::from_move(position.clone(), better).to_string());
        let dots = if position.turn().is_white() {
            "."
        } else {
            "..."
        };
        Some(Feedback {
            verdict,
            numbered: format!("{}{dots} {}", position.fullmoves(), played.san),
            likelihood: before
                .zip(played.eval.wdl)
                .map(|(before, after)| (mine(before), mine(after))),
            best,
        })
    }

    /// The moves from one square to another: more than one means a promotion. Castling
    /// goes to where the king ends up, or onto the rook it castles with.
    pub fn moves_between(&self, from: Square, to: Square) -> Vec<Move> {
        let legal = self.game.position().legal_moves();
        legal
            .iter()
            .filter(|played| {
                let lands = slide_paths(**played).first().map(|(_, to)| *to) == Some(to);
                let onto_rook = matches!(played, Move::Castle { rook, .. } if *rook == to);
                played.from() == Some(from) && (lands || onto_rook)
            })
            .copied()
            .collect()
    }

    /// The markings of the shown move.
    pub fn annotations(&self) -> Annotations {
        let mut annotations = self
            .shown_move()
            .map_or_else(Annotations::default, |played| played.annotations.clone());
        if let Some(suggestion) = self.suggestion.filter(|found| found.ply == self.ply) {
            annotations.arrows.push(suggestion.arrow);
        }
        annotations
    }

    pub fn shown_move(&self) -> Option<&chessdesk_core::PlayedMove> {
        self.game.moves().get(self.ply.checked_sub(1)?)
    }

    /// Stops the running clock, if any.
    pub fn stop_clock(&mut self) {
        if let Some(clock) = self.live.as_mut().and_then(|live| live.clock.as_mut()) {
            clock.stop(Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use chessdesk_core::Game;
    use shakmaty::Position;

    use super::*;

    const AFTER_THE_MOVE: Wdl = Wdl {
        white: 900,
        draw: 100,
        black: 0,
    };
    const AT_THE_START: Wdl = Wdl {
        white: 52,
        draw: 942,
        black: 6,
    };
    /// White is a queen up, so that it would be wrong to call this even.
    const QUEEN_UP: &str = "4k3/8/8/8/8/8/8/Q3K3 w - - 0 1";

    fn evaluated(mut game: Game) -> Game {
        let first = game.position().legal_moves()[0];
        assert!(game.play(first));
        game.moves_mut()[0].eval.wdl = Some(AFTER_THE_MOVE);
        game
    }

    fn from_a_position() -> Game {
        evaluated(Game::from_position(Game::parse_fen(QUEEN_UP).unwrap()))
    }

    #[test]
    fn the_king_castles_onto_its_rook_or_where_it_lands() {
        let mut game = Game::default();
        for san in ["e4", "e5", "Nf3", "Nc6", "Bc4", "Bc5"] {
            let played: shakmaty::san::San = san.parse().unwrap();
            assert!(game.play(played.to_move(&game.position()).unwrap()));
        }
        let session = Session::new(0, game);
        let castles = |to| {
            let found = session.moves_between(Square::E1, to);
            matches!(found.as_slice(), [Move::Castle { .. }])
        };
        assert!(castles(Square::H1));
        assert!(castles(Square::G1));
        // A step to the side is a step, not castling.
        assert!(matches!(
            session.moves_between(Square::E1, Square::F1).as_slice(),
            [Move::Normal { .. }]
        ));
    }

    #[test]
    fn the_log_row_follows_the_move_on_show() {
        let mut session = Session::new(0, Game::default());
        for (ply, row) in [(0, 0), (1, 0), (2, 0), (3, 1), (4, 1), (5, 2)] {
            session.ply = ply;
            assert_eq!(session.log_row(), row);
        }
        let black_first = "4k3/8/8/8/8/8/8/Q3K3 b - - 0 1";
        let game = Game::from_position(Game::parse_fen(black_first).unwrap());
        let mut session = Session::new(0, game);
        for (ply, row) in [(1, 0), (2, 1), (3, 1)] {
            session.ply = ply;
            assert_eq!(session.log_row(), row);
        }
    }

    #[test]
    fn before_any_move_a_normal_game_is_even() {
        let mut game = evaluated(Game::default());
        game.set_start_wdl(AT_THE_START);
        let mut session = Session::new(0, game);
        session.ply = 0;
        assert_eq!(session.known_wdl(), Some(EVEN));
        session.ply = 1;
        assert_eq!(session.known_wdl(), Some(AFTER_THE_MOVE));
        // Grading the first move still goes by what the engine said.
        assert_eq!(session.game.start_wdl(), Some(AT_THE_START));
    }

    #[test]
    fn a_set_up_position_shows_nothing_from_later_in_the_game() {
        let mut session = Session::new(0, from_a_position());
        session.ply = 0;
        assert_eq!(session.known_wdl(), None);
    }

    #[test]
    fn a_set_up_position_shows_its_own_likelihood() {
        let mut game = from_a_position();
        game.set_start_wdl(AT_THE_START);
        let mut session = Session::new(0, game);
        session.ply = 0;
        assert_eq!(session.known_wdl(), Some(AT_THE_START));
    }
}
