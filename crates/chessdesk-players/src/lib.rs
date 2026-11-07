//! Who plays and who analyses: UCI engines and ACP agents, behind one small interface.
//! Everything here blocks, so the app calls it from background threads.

pub mod acp;
pub mod adapters;
pub mod analysis;
pub mod download;
pub mod prompt;
pub mod reply;
pub mod uci;

use std::fmt;
use std::ops::AddAssign;

use chessdesk_core::{Annotations, Game};
use shakmaty::Move;

/// Tokens an agent reports having used. Agents that report nothing leave this empty,
/// which the app shows as "not reported" rather than as zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tokens {
    pub total: u64,
    /// The part of `total` read from the provider's prompt cache, which costs far less.
    pub cached: u64,
}

impl AddAssign for Tokens {
    fn add_assign(&mut self, other: Self) {
        self.total += other.total;
        self.cached += other.cached;
    }
}

/// An opponent's move, and what came with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub played: Move,
    pub annotations: Annotations,
    pub tokens: Option<Tokens>,
}

/// Why a player could not answer.
#[derive(Debug)]
pub enum PlayerError {
    /// The program could not be started or stopped talking.
    Process(String),
    /// The agent's replies held no legal move, even after being shown the legal ones.
    NoLegalMove { reply: String },
}

impl fmt::Display for PlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Process(reason) => f.write_str(reason),
            Self::NoLegalMove { reply } => write!(f, "no legal move in the reply: {reply}"),
        }
    }
}

impl std::error::Error for PlayerError {}

/// Anything that can choose a move: an engine or an agent.
pub trait Opponent: Send {
    /// `instructions` are the player's own words on how to play, which agents heed.
    fn next_move(&mut self, game: &Game, instructions: &str) -> Result<Decision, PlayerError>;
}
