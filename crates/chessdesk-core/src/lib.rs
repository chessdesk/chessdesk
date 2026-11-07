//! Chessdesk's game model, free of any UI or process handling: the rules (through
//! `shakmaty`), PGN, what can be drawn on the board, and plugins.

pub mod annotation;
pub mod eval;
pub mod game;
pub mod material;
pub mod pgn;
pub mod plugins;
pub mod review;
pub mod sound;

pub use annotation::{Annotations, Arrow, Highlight, Mark};
pub use eval::{Eval, Score, Wdl};
pub use game::{Ending, Game, PlayedMove};
