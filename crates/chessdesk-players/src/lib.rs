//! Who plays and who analyses: UCI engines, Shadowchess and ACP agents, behind one small
//! interface.
//! Everything here blocks, so the app calls it from background threads.

pub mod acp;
pub mod adapters;
pub mod analysis;
pub mod download;
mod illegal;
mod pi;
pub mod prompt;
pub mod reply;
pub mod shadowchess;
pub mod uci;
pub mod update;

use std::fmt;
use std::ops::AddAssign;
use std::time::Duration;

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
    /// No reply within the time its effort allows.
    TimedOut(Duration),
}

impl fmt::Display for PlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Process(reason) => f.write_str(reason),
            Self::NoLegalMove { reply } => write!(f, "no legal move in the reply: {reply}"),
            Self::TimedOut(limit) => {
                write!(f, "no answer within {} minutes", limit.as_secs() / 60)
            }
        }
    }
}

impl std::error::Error for PlayerError {}

/// What an opponent says before the first move, when the player has it: words and markings
/// on the starting position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Intro {
    pub annotations: Annotations,
    pub tokens: Option<Tokens>,
}

/// Anything that can choose a move: an engine or an agent.
pub trait Opponent: Send {
    /// `instructions` are the player's own words on how to play, which agents heed.
    fn next_move(&mut self, game: &Game, instructions: &str) -> Result<Decision, PlayerError>;

    /// Has the opponent speak before the player's first move: how an agent that coaches
    /// can set out what's to come, such as an opening it was asked to teach. Engines have
    /// nothing to say.
    fn introduce(
        &mut self,
        _game: &Game,
        _instructions: &str,
    ) -> Result<Option<Intro>, PlayerError> {
        Ok(None)
    }
}
