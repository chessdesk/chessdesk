//! Reviews a saved game with a real agent and prints what it wrote. This spends real
//! tokens, so it's a manual check rather than a test:
//!
//! ```powershell
//! cargo run -p chessdesk-players --example review_smoke -- game.pgn sonnet
//! ```

use chessdesk_core::pgn;
use chessdesk_players::acp::{AgentConfig, AgentKind, Effort, Session, review_game};
use chessdesk_players::adapters::{Readiness, install, readiness};
use chessdesk_players::prompt;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("a PGN file");
    let text = std::fs::read_to_string(path).expect("the file reads");
    let mut game = pgn::read(&text).expect("the game reads");
    let adapters = std::env::temp_dir().join("chessdesk-adapters");
    let kind = AgentKind::ClaudeCode;
    if readiness(kind, &adapters) == Readiness::NoAdapter {
        install(kind, &adapters).expect("the adapter installs");
    }
    let config = AgentConfig {
        adapters,
        kind,
        model: args.next(),
        effort: Effort::Medium,
        reply_language: None,
    };
    println!("--- prompt ---\n{}", prompt::review(&game));
    let session = Session::start(&config, prompt::review_system(None));
    match session.ask(prompt::review(&game)) {
        Ok((reply, tokens)) => println!("--- reply ({tokens:?}) ---\n{reply}"),
        Err(error) => println!("failed: {error}"),
    }
    drop(session);
    println!("--- applied ---\n{:?}", review_game(&config, &mut game));
    for played in game.moves() {
        println!("{} {:?}", played.san, played.annotations);
    }
}
