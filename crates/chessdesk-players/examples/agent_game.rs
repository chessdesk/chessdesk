//! Plays a whole tutoring game against a real agent, headless: Stockfish at a set strength
//! stands in for the player as White, a second Stockfish grades White's moves as the app does,
//! and the agent plays Black and coaches. Prints every move with its comment and tokens, and
//! saves the game as PGN. Spends real tokens, a game's worth, so it's a manual check:
//!
//! ```powershell
//! $env:STOCKFISH = "$env:LOCALAPPDATA\chessdesk\engines\stockfish\stockfish\stockfish-windows-arm64-universal.exe"
//! cargo run -p chessdesk-players --example agent_game -- claude sonnet low 1320 "I'm a new player"
//! cargo run -p chessdesk-players --example agent_game -- copilot "" low 1320 "I'm a new player"
//! ```
//!
//! Arguments: agent, model (`""` for the agent's default), effort, the player's engine Elo,
//! instructions, and `follow` to have the player take the lesson: play the move the agent
//! drew for it, where that's legal, as a learner would. The PGN goes to `agent-game.pgn` in
//! the temp folder. `agent_lesson` is a lesson with its checks.

mod common;

use std::time::Duration;

use chessdesk_core::pgn;
use chessdesk_players::acp::AgentOpponent;
use chessdesk_players::uci::{Engine, EngineConfig};
use chessdesk_players::{Opponent, Tokens};
use common::{add, agent_config, agents_turn, new_game, players_turn, stockfish};
use shakmaty::{Color, Position};

/// Where a game that hasn't ended is stopped, to keep the cost bounded.
const MAX_PLIES: usize = 160;

fn main() {
    let mut args = std::env::args().skip(1);
    let config = agent_config(&mut args);
    let elo = args
        .next()
        .map_or(1320, |elo| elo.parse().expect("an Elo number"));
    let instructions = args.next().unwrap_or_default();
    let follow = args.next().as_deref() == Some("follow");

    let stockfish = stockfish();
    let mut stand_in = Engine::start(&EngineConfig::stockfish(&stockfish, Some(elo)))
        .expect("Stockfish starts as the player");
    let mut grader =
        Engine::start(&EngineConfig::stockfish(&stockfish, None)).expect("Stockfish starts");
    let mut agent = AgentOpponent::start(&config, Color::Black, true);

    let mut game = new_game(&mut grader);
    game.set_tag("White", format!("Stockfish {elo} (as the player)"));
    let model = config.model.as_deref().unwrap_or("default");
    game.set_tag("Black", format!("{} ({model})", config.kind.name()));
    let mut spent = Tokens::default();
    let mut thinking = Duration::ZERO;

    // What the agent drew last: where a lesson points the player.
    let mut drawn = Vec::new();
    match agent.introduce(&game, &instructions) {
        Ok(Some(intro)) => {
            add(&mut spent, intro.tokens);
            println!(
                "intro: {}  {:?}  arrows {:?}",
                intro.annotations.text, intro.tokens, intro.annotations.arrows
            );
            drawn = intro.annotations.arrows;
        }
        Ok(None) => println!("intro: none"),
        Err(error) => println!("intro failed: {error}"),
    }

    while game.outcome().is_unknown() && game.moves().len() < MAX_PLIES {
        if game.position().turn() == Color::White {
            let lesson = if follow { drawn.as_slice() } else { &[] };
            players_turn(&mut game, &mut stand_in, &mut grader, lesson);
        } else {
            match agents_turn(&mut game, &mut agent, &instructions) {
                Ok(said) => {
                    add(&mut spent, said.tokens);
                    thinking += said.took;
                    drawn = said.annotations.arrows;
                }
                Err(error) => {
                    println!("agent failed: {error}");
                    break;
                }
            }
        }
    }

    game.set_tag("ChessdeskTokens", spent.total.to_string());
    let path = std::env::temp_dir().join("agent-game.pgn");
    std::fs::write(&path, pgn::write(&game)).expect("the PGN saves");
    println!(
        "result {:?}, {} plies, tokens {spent:?}, agent took {:.0}s, saved to {}",
        game.outcome(),
        game.moves().len(),
        thinking.as_secs_f32(),
        path.display()
    );
}
