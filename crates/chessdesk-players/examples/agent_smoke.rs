//! Plays a few moves against a real agent over ACP. This spends real tokens, a few
//! thousand on a cheap model, so it's a manual check rather than a test:
//!
//! ```powershell
//! cargo run -p chessdesk-players --example agent_smoke -- claude haiku high
//! cargo run -p chessdesk-players --example agent_smoke -- copilot
//! ```

use chessdesk_core::Game;
use chessdesk_players::Opponent;
use chessdesk_players::acp::{AgentConfig, AgentKind, AgentOpponent, Effort};
use chessdesk_players::adapters::{Readiness, install, readiness};
use chessdesk_players::reply::parse;
use shakmaty::Color;

/// White's moves, which the agent answers as Black.
const WHITE_MOVES: [&str; 3] = ["e4", "Nf3", "d4"];

fn main() {
    let mut args = std::env::args().skip(1);
    let kind = match args.next().as_deref() {
        Some("codex") => AgentKind::Codex,
        Some("copilot") => AgentKind::Copilot,
        Some("opencode") => AgentKind::OpenCode,
        Some("pi") => AgentKind::Pi,
        _ => AgentKind::ClaudeCode,
    };
    let adapters = std::env::temp_dir().join("chessdesk-adapters");
    if readiness(kind, &adapters) == Readiness::NoAdapter {
        install(kind, &adapters).expect("the adapter installs");
    }
    let config = AgentConfig {
        adapters,
        kind,
        model: args.next(),
        effort: match args.next().as_deref() {
            Some("high") => Effort::High,
            Some("medium") => Effort::Medium,
            _ => Effort::Low,
        },
        reply_language: None,
    };
    let instructions = args.next().unwrap_or_default();
    let mut agent = AgentOpponent::start(&config, Color::Black, true);
    let mut game = Game::default();
    for white in WHITE_MOVES {
        let Some((played, _)) = parse(white, &game.position()) else {
            println!("{white} is no longer legal after the agent's replies");
            break;
        };
        assert!(game.play(played));
        match agent.next_move(&game, &instructions) {
            Ok(decision) => {
                assert!(game.play(decision.played));
                let san = &game.moves().last().expect("just played").san;
                println!(
                    "{white} {san}  tokens {:?}  {:?}",
                    decision.tokens, decision.annotations
                );
            }
            Err(error) => {
                println!("failed: {error}");
                return;
            }
        }
    }
}
