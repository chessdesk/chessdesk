//! The lesson test: a new player, White, asks the agent to teach them a fast mate, and does
//! what they're told. One of the hardest things a coaching agent does in Chessdesk: it has to
//! read the request as the player's lesson, name their every move with `[%next]`, play along
//! as Black, and keep its comments readable. Each check below failed with some model once.
//!
//! The player plays the move the agent drew for them, or Stockfish at 1320 where it drew
//! none. Spends real tokens, a few thousand a run:
//!
//! ```powershell
//! $env:STOCKFISH = "$env:LOCALAPPDATA\chessdesk\engines\stockfish\stockfish\stockfish-windows-arm64-universal.exe"
//! cargo run -p chessdesk-players --example agent_lesson -- pi openrouter/z-ai/glm-5.3-flash low 3
//! cargo run -p chessdesk-players --example agent_lesson -- claude opus low
//! ```
//!
//! Arguments: agent, model (`""` for the agent's default), effort, and how many runs (1).
//! Exits with 1 if any check failed in any run. Each run's PGN goes to `agent-lesson-N.pgn`
//! in the temp folder.

mod common;

use std::process::ExitCode;
use std::time::Duration;

use chessdesk_core::{Game, pgn};
use chessdesk_players::acp::{AgentConfig, AgentOpponent};
use chessdesk_players::uci::{Engine, EngineConfig};
use chessdesk_players::{Opponent, Tokens};
use common::{add, agent_config, agents_turn, new_game, players_turn, pointed_move, stockfish};
use shakmaty::{Color, KnownOutcome, Outcome, Position};

/// The request, as a player typed it: it names no mate, so the agent has to pick one the
/// player gives.
const REQUEST: &str = "I'm a new player, teach me a super fun, fast way to mate.";
/// Scholar's Mate takes four moves; a lesson that hasn't mated in eight has lost its way.
const MAX_PLIES: usize = 16;
const STAND_IN_ELO: u32 = 1320;

/// What a run is judged on, in the order printed.
const CHECKS: [&str; 5] = [
    "the intro names the player's first move",
    "every move of the player's was named",
    "the player gives mate",
    "every reply has a comment, with no gaps",
    "comments leave Stockfish out",
];

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let config = agent_config(&mut args);
    let runs = args
        .next()
        .map_or(1, |runs| runs.parse().expect("a number of runs"));
    let stockfish = stockfish();

    let mut passed = [0; CHECKS.len()];
    for run in 1..=runs {
        println!("--- run {run} of {runs}");
        let lesson = run_lesson(&config, &stockfish);
        let path = std::env::temp_dir().join(format!("agent-lesson-{run}.pgn"));
        std::fs::write(&path, pgn::write(&lesson.game)).expect("the PGN saves");
        let results = lesson.judge();
        for (check, ok) in CHECKS.iter().zip(results) {
            println!("{} {check}", if ok { "PASS" } else { "FAIL" });
        }
        for (count, ok) in passed.iter_mut().zip(results) {
            *count += u32::from(ok);
        }
        println!(
            "{} plies, tokens {}, agent took {:.0}s, saved to {}",
            lesson.game.moves().len(),
            lesson
                .tokens
                .map_or("not reported".to_owned(), |tokens| format!("{tokens:?}")),
            lesson.thinking.as_secs_f32(),
            path.display()
        );
    }

    println!("--- {runs} runs");
    for (check, count) in CHECKS.iter().zip(passed) {
        println!("{count}/{runs} {check}");
    }
    if passed.iter().all(|count| *count == runs) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// One lesson, and what it takes to judge it.
struct Lesson {
    game: Game,
    intro_named_a_move: bool,
    every_move_taught: bool,
    /// The intro's comment and each reply's, in order.
    comments: Vec<String>,
    tokens: Option<Tokens>,
    thinking: Duration,
}

impl Lesson {
    fn judge(&self) -> [bool; CHECKS.len()] {
        let mated = self.game.position().is_checkmate()
            && self.game.outcome()
                == Outcome::Known(KnownOutcome::Decisive {
                    winner: Color::White,
                });
        [
            self.intro_named_a_move,
            self.every_move_taught,
            mated,
            self.comments.iter().all(|text| reads_whole(text)),
            !self.comments.iter().any(|text| {
                let text = text.to_lowercase();
                text.contains("stockfish") || text.contains("engine")
            }),
        ]
    }
}

/// Whether a comment is there and has no holes where markup was taken out of a sentence:
/// "Your first move: ." or "meet you with , like".
fn reads_whole(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && !text.ends_with(':')
        && ![" ,", " .", ":  ", ": .", "( )"]
            .iter()
            .any(|gap| text.contains(gap))
}

fn run_lesson(config: &AgentConfig, stockfish: &str) -> Lesson {
    let mut stand_in = Engine::start(&EngineConfig::stockfish(stockfish, Some(STAND_IN_ELO)))
        .expect("Stockfish starts as the player");
    let mut grader =
        Engine::start(&EngineConfig::stockfish(stockfish, None)).expect("Stockfish starts");
    let mut agent = AgentOpponent::start(config, Color::Black, true);
    let mut game = new_game(&mut grader);
    game.set_tag("White", "You (taking the lesson)");
    let model = config.model.as_deref().unwrap_or("default");
    game.set_tag("Black", format!("{} ({model})", config.kind.name()));

    let mut tokens = None;
    let mut thinking = Duration::ZERO;
    let mut comments = Vec::new();
    let mut drawn = Vec::new();
    match agent.introduce(&game, REQUEST) {
        Ok(intro) => {
            let said = intro.map(|intro| {
                count(&mut tokens, intro.tokens);
                intro.annotations
            });
            let said = said.unwrap_or_default();
            println!("intro: {}  arrows {:?}", said.text, said.arrows);
            comments.push(said.text);
            drawn = said.arrows;
        }
        Err(error) => println!("intro failed: {error}"),
    }
    let intro_named_a_move = pointed_move(&game.position(), &drawn).is_some();

    let mut every_move_taught = true;
    while game.outcome().is_unknown() && game.moves().len() < MAX_PLIES {
        if game.position().turn() == Color::White {
            every_move_taught &= players_turn(&mut game, &mut stand_in, &mut grader, &drawn);
        } else {
            match agents_turn(&mut game, &mut agent, REQUEST) {
                Ok(said) => {
                    count(&mut tokens, said.tokens);
                    thinking += said.took;
                    comments.push(said.annotations.text);
                    drawn = said.annotations.arrows;
                }
                Err(error) => {
                    println!("agent failed: {error}");
                    every_move_taught = false;
                    break;
                }
            }
        }
    }
    Lesson {
        game,
        intro_named_a_move,
        every_move_taught,
        comments,
        tokens,
        thinking,
    }
}

/// Adds what an agent reported, leaving the total unknown for one that reports nothing.
fn count(total: &mut Option<Tokens>, more: Option<Tokens>) {
    if more.is_some() {
        add(total.get_or_insert_default(), more);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comment_with_a_hole_does_not_read_whole() {
        assert!(reads_whole("Your first move: e4"));
        assert!(!reads_whole("Your first move:"));
        assert!(!reads_whole(
            "Now I'll meet you head on with , like most players."
        ));
        assert!(!reads_whole(""));
    }
}
