//! Starting Pi as a chess player rather than a coding agent. Its ACP adapter, `pi-acp`,
//! starts `pi` with fixed arguments and takes no prompt over ACP, so Pi would bring its
//! coding prompt, its tools, the user's `AGENTS.md`, extensions and MCP servers into every
//! game. The adapter does let `PI_ACP_PI_COMMAND` name the program, so Chessdesk names a
//! launcher of its own that starts `pi` with all of that switched off and its prompt in place.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};

/// What `pi` is started with besides the adapter's own arguments: no tools, nothing found on
/// disk, and Chessdesk's prompt from a file.
const SWITCHES: &str = "--no-tools --no-extensions --no-mcp --no-skills --no-context-files \
                        --no-prompt-templates --system-prompt";

/// Writes the launcher for a session with `system` as its prompt into `folder`, and returns
/// it. Each prompt has its own, so that a game and a hint can run side by side.
pub fn launcher(system: &str, folder: &Path) -> io::Result<PathBuf> {
    let pi = which::which("pi").map_err(io::Error::other)?;
    let mut hasher = DefaultHasher::new();
    system.hash(&mut hasher);
    let folder = folder.join(format!("pi-{:016x}", hasher.finish()));
    std::fs::create_dir_all(&folder)?;
    let prompt = folder.join("system.md");
    std::fs::write(&prompt, system)?;
    let launcher = folder.join(if cfg!(windows) { "pi.cmd" } else { "pi.sh" });
    std::fs::write(&launcher, script(&pi, &prompt))?;
    make_executable(&launcher)?;
    Ok(launcher)
}

/// The launcher's text: `pi` with the switches, then whatever the adapter passes.
fn script(pi: &Path, prompt: &Path) -> String {
    let (pi, prompt) = (pi.display(), prompt.display());
    if cfg!(windows) {
        // `call`, since `pi` is itself a batch file there.
        format!("@call \"{pi}\" {SWITCHES} \"{prompt}\" %*\r\n")
    } else {
        format!("#!/bin/sh\nexec '{pi}' {SWITCHES} '{prompt}' \"$@\"\n")
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
#[expect(clippy::unnecessary_wraps, reason = "matches the Unix version")]
fn make_executable(_: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_launcher_starts_pi_bare_with_chessdesks_prompt_and_passes_the_rest_on() {
        let text = script(Path::new("pi-bin"), Path::new("system.md"));
        for switch in [
            "--no-tools",
            "--no-extensions",
            "--no-mcp",
            "--no-context-files",
        ] {
            assert!(text.contains(switch), "{switch} missing");
        }
        assert!(text.contains("--system-prompt"));
        let forwards = if cfg!(windows) { "%*" } else { "\"$@\"" };
        assert!(text.trim_end().ends_with(forwards));
    }

    #[test]
    fn each_prompt_gets_its_own_launcher() {
        let Ok(_) = which::which("pi") else {
            return;
        };
        let folder = std::env::temp_dir().join("chessdesk-pi-test");
        let game = launcher("You play chess.", &folder).unwrap();
        let hint = launcher("You give hints.", &folder).unwrap();
        assert_ne!(game, hint);
        let prompt = game.parent().unwrap().join("system.md");
        assert_eq!(std::fs::read_to_string(prompt).unwrap(), "You play chess.");
    }
}
