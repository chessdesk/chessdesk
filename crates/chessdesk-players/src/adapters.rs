//! What an agent needs before it can play. Some speak ACP themselves; the others need an
//! adapter, which Chessdesk keeps in its own folder at a version it was tested with.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::acp::AgentKind;
use crate::uci::hide_console;

/// An npm package that makes an agent speak ACP.
struct Adapter {
    package: &'static str,
    program: &'static str,
}

fn adapter(kind: AgentKind) -> Option<Adapter> {
    let (package, program) = match kind {
        AgentKind::ClaudeCode => (
            "@agentclientprotocol/claude-agent-acp@0.101.0",
            "claude-agent-acp",
        ),
        AgentKind::Codex => ("@agentclientprotocol/codex-acp@2.5.0", "codex-acp"),
        AgentKind::Pi => ("pi-acp@0.0.34", "pi-acp"),
        AgentKind::Copilot | AgentKind::OpenCode => return None,
    };
    Some(Adapter { package, program })
}

/// The agent's own program, for those Chessdesk can't bring along.
fn own_program(kind: AgentKind) -> Option<&'static str> {
    match kind {
        AgentKind::Copilot => Some("copilot"),
        AgentKind::OpenCode => Some("opencode"),
        AgentKind::Pi => Some("pi"),
        AgentKind::ClaudeCode | AgentKind::Codex => None,
    }
}

/// How far an agent is from being able to play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readiness {
    Ready,
    /// Its program isn't installed, which is up to the player.
    NoProgram(&'static str),
    /// Installing the adapter needs npm, and running it Node.js.
    NoNode,
    /// One press away.
    NoAdapter,
}

/// Where an adapter's program is: Chessdesk's own copy, or one on `PATH`.
fn adapter_program(adapter: &Adapter, folder: &Path) -> Option<PathBuf> {
    let bin = folder.join("node_modules").join(".bin");
    let name = if cfg!(windows) {
        format!("{}.cmd", adapter.program)
    } else {
        adapter.program.to_owned()
    };
    let own = bin.join(name);
    if own.is_file() {
        return Some(own);
    }
    which::which(adapter.program).ok()
}

pub fn readiness(kind: AgentKind, folder: &Path) -> Readiness {
    if let Some(program) = own_program(kind)
        && which::which(program).is_err()
    {
        return Readiness::NoProgram(program);
    }
    let Some(adapter) = adapter(kind) else {
        return Readiness::Ready;
    };
    if which::which("node").is_err() || which::which("npm").is_err() {
        return Readiness::NoNode;
    }
    if adapter_program(&adapter, folder).is_some() {
        Readiness::Ready
    } else {
        Readiness::NoAdapter
    }
}

/// How to start a program: Windows only runs a script through its shell.
fn launch(program: &Path) -> Vec<String> {
    let path = program.to_string_lossy().into_owned();
    let is_script = program
        .extension()
        .is_some_and(|extension| extension == "cmd" || extension == "bat");
    if is_script {
        vec!["cmd".to_owned(), "/c".to_owned(), path]
    } else {
        vec![path]
    }
}

/// The command that starts an agent's ACP side, before its arguments.
pub(crate) fn command(kind: AgentKind, folder: &Path) -> Option<Vec<String>> {
    let program = match adapter(kind) {
        Some(adapter) => adapter_program(&adapter, folder)?,
        None => which::which(own_program(kind)?).ok()?,
    };
    Some(launch(&program))
}

/// Installs an agent's adapter into `folder`.
pub fn install(kind: AgentKind, folder: &Path) -> Result<(), String> {
    let adapter = adapter(kind).ok_or("this agent needs no adapter")?;
    std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let npm = which::which("npm").map_err(|_| "npm not found")?;
    let mut parts = launch(&npm).into_iter();
    let mut command = Command::new(parts.next().ok_or("npm not found")?);
    command
        .args(parts)
        .args(["install", "--no-audit", "--no-fund", "--prefix"])
        .arg(folder)
        .arg(adapter.package);
    hide_console(&mut command);
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    let errors = String::from_utf8_lossy(&output.stderr);
    let reason = errors.lines().rfind(|line| !line.trim().is_empty());
    Err(reason.unwrap_or("npm failed").to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapters_are_pinned() {
        for kind in AgentKind::ALL {
            if let Some(adapter) = adapter(kind) {
                let version = adapter.package.rsplit_once('@').map(|(_, version)| version);
                assert!(version.is_some_and(|version| version.contains('.')));
            }
        }
    }

    #[test]
    fn every_agent_has_a_way_to_run() {
        for kind in AgentKind::ALL {
            assert!(adapter(kind).is_some() || own_program(kind).is_some());
        }
    }

    #[test]
    fn scripts_go_through_the_shell() {
        assert_eq!(launch(Path::new("a.cmd")), ["cmd", "/c", "a.cmd"]);
        assert_eq!(launch(Path::new("a.exe")), ["a.exe"]);
    }

    #[test]
    fn its_own_copy_of_an_adapter_counts() {
        let folder = std::env::temp_dir().join(format!("chessdesk-adapter-{}", std::process::id()));
        let bin = folder.join("node_modules/.bin");
        std::fs::create_dir_all(&bin).unwrap();
        let adapter = Adapter {
            package: "made-up@1.0.0",
            program: "chessdesk-made-up-adapter",
        };
        assert_eq!(adapter_program(&adapter, &folder), None);
        let name = if cfg!(windows) {
            "chessdesk-made-up-adapter.cmd"
        } else {
            "chessdesk-made-up-adapter"
        };
        std::fs::write(bin.join(name), "").unwrap();
        let found = adapter_program(&adapter, &folder);
        std::fs::remove_dir_all(&folder).unwrap();
        assert!(found.is_some());
    }
}
