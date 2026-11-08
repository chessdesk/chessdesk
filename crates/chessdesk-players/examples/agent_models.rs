//! Prints the models each agent offers, as it says when a session starts. Spends no tokens:
//! no prompt is sent. Missing adapters are installed into a folder of their own.
//!
//! ```powershell
//! cargo run -p chessdesk-players --example agent_models
//! ```

use chessdesk_players::acp::{AgentConfig, AgentKind, Effort, models};
use chessdesk_players::adapters::{Readiness, install, readiness};

fn main() {
    let adapters = std::env::temp_dir().join("chessdesk-adapters");
    for kind in AgentKind::ALL {
        if readiness(kind, &adapters) == Readiness::NoAdapter
            && let Err(error) = install(kind, &adapters)
        {
            println!("{}: could not install the adapter: {error}", kind.name());
            continue;
        }
        let state = readiness(kind, &adapters);
        if state != Readiness::Ready {
            println!("{}: {state:?}", kind.name());
            continue;
        }
        let config = AgentConfig {
            kind,
            model: None,
            effort: Effort::Low,
            adapters: adapters.clone(),
            reply_language: None,
        };
        match models(&config) {
            Ok(Some(models)) => {
                println!(
                    "{}: option {:?}, current {:?}",
                    kind.name(),
                    models.option,
                    models.current
                );
                for model in models.choices {
                    println!("  {} = {}  {:?}", model.id, model.name, model.description);
                }
            }
            Ok(None) => println!("{}: offers no model choice", kind.name()),
            Err(error) => println!("{}: {error}", kind.name()),
        }
    }
}
