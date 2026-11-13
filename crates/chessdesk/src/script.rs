//! Scripts that drive the app for a check by eye: start or open a game, play it, wait for the
//! opponent and the analysis, and take screenshots. `chessdesk script <file>` runs one; the
//! commands are in `docs/development.md`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chessdesk_players::acp::AgentKind;
use shakmaty::Color;

use crate::settings_view::SettingsPage;

/// Stockfish's strength in a scripted game unless the script says otherwise: the weakest, and
/// so the quickest to answer.
pub const DEFAULT_ELO: u32 = 1320;

/// One thing a script does. Each waits until the game on show has settled first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// A casual game against Stockfish, or Shadowchess at a level, from the usual start or a FEN.
    New {
        side: Color,
        elo: u32,
        shadowchess: Option<u32>,
        fen: Option<String>,
    },
    Open(PathBuf),
    /// The player's moves in SAN, each once the opponent has answered the one before.
    Play(Vec<String>),
    Resign,
    Analyze,
    /// An agent's review, on its default model unless one is named, as the agent names it.
    Review {
        agent: AgentKind,
        model: Option<String>,
    },
    Show(Ply),
    /// How long a command may wait for the game to settle, from here on.
    Timeout(Duration),
    Screenshot(PathBuf),
    /// Opens a page of the settings, in the board's place.
    Settings(SettingsPage),
    /// Moves the game on show to the Recycle Bin, so that it doesn't stay in the history.
    Discard,
}

/// A position of the game to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ply {
    At(usize),
    End,
}

/// A command and where it stands in the script, for saying what failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub line: usize,
    pub text: String,
    pub command: Command,
}

/// Reads a script. Blank lines and `#` comments are skipped; relative paths are taken from
/// `base`, the script's folder. Fails on the first line it can't read.
pub fn parse(text: &str, base: &Path) -> Result<Vec<Step>, String> {
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .map(|(line, text)| {
            let command = command(text, base).map_err(|error| format!("line {line}: {error}"))?;
            Ok(Step {
                line,
                text: text.to_owned(),
                command,
            })
        })
        .collect()
}

fn command(text: &str, base: &Path) -> Result<Command, String> {
    let (word, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    let rest = rest.trim();
    let path = || {
        if rest.is_empty() {
            Err(format!("`{word}` needs a path"))
        } else {
            Ok(base.join(rest))
        }
    };
    let alone = |command: Command| {
        if rest.is_empty() {
            Ok(command)
        } else {
            Err(format!("`{word}` takes nothing more"))
        }
    };
    match word {
        "new" => new_game(rest),
        "open" => path().map(Command::Open),
        "play" if !rest.is_empty() => Ok(Command::Play(
            rest.split_whitespace().map(str::to_owned).collect(),
        )),
        "play" => Err("`play` needs moves".to_owned()),
        "resign" => alone(Command::Resign),
        "analyze" => alone(Command::Analyze),
        "review" => review(rest),
        "show" => match rest {
            "end" => Ok(Command::Show(Ply::End)),
            ply => ply
                .parse()
                .map(|ply| Command::Show(Ply::At(ply)))
                .map_err(|_| "`show` takes a number of moves or `end`".to_owned()),
        },
        "timeout" => rest
            .parse()
            .map(|seconds| Command::Timeout(Duration::from_secs(seconds)))
            .map_err(|_| "`timeout` takes seconds".to_owned()),
        "screenshot" => path().map(Command::Screenshot),
        "settings" => SettingsPage::ALL
            .into_iter()
            .find(|page| page.id() == rest)
            .map(Command::Settings)
            .ok_or_else(|| format!("no settings page `{rest}`")),
        "discard" => alone(Command::Discard),
        _ => Err(format!("unknown command `{word}`")),
    }
}

/// `new white|black [elo N] [shadowchess LEVEL] [fen FEN]`, the FEN taking the rest of the line.
fn new_game(rest: &str) -> Result<Command, String> {
    let (side, mut rest) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let side = match side {
        "white" => Color::White,
        "black" => Color::Black,
        _ => return Err("`new` takes `white` or `black` first".to_owned()),
    };
    let mut elo = DEFAULT_ELO;
    let mut shadowchess = None;
    let mut fen = None;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let (word, after) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        let (value, next) = after.trim_start().split_once(' ').unwrap_or((after, ""));
        let number = || {
            value
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("`{word}` takes a number"))
        };
        match word {
            "elo" => {
                elo = number()?;
                rest = next;
            }
            "shadowchess" => {
                shadowchess = Some(number()?);
                rest = next;
            }
            "fen" if !after.trim().is_empty() => {
                fen = Some(after.trim().to_owned());
                break;
            }
            _ => return Err(format!("`new` doesn't know `{word}`")),
        }
    }
    Ok(Command::New {
        side,
        elo,
        shadowchess,
        fen,
    })
}

/// `review AGENT [model MODEL]`.
fn review(rest: &str) -> Result<Command, String> {
    let (name, rest) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let agent = agent(name)?;
    let model = match rest.trim().split_once(char::is_whitespace) {
        None if rest.trim().is_empty() => None,
        Some(("model", model)) if !model.trim().is_empty() => Some(model.trim().to_owned()),
        _ => return Err("`review` takes an agent, then `model` and its name".to_owned()),
    };
    Ok(Command::Review { agent, model })
}

/// An agent as a script names it: its name in lower case, words joined by `-`.
fn agent(name: &str) -> Result<AgentKind, String> {
    AgentKind::from_id(name).ok_or_else(|| format!("unknown agent `{name}`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(line: &str) -> Result<Command, String> {
        parse(line, Path::new("base")).map(|mut steps| steps.remove(0).command)
    }

    #[test]
    fn reads_a_whole_script_and_skips_comments() {
        let text = "# Mate in one\n\nnew white fen 6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1\n\
                    play Ra8#\nshow end\nscreenshot shots/mate.png\ndiscard\n";
        let steps = parse(text, Path::new("base")).unwrap();
        assert_eq!(steps.len(), 5);
        assert_eq!(steps[0].line, 3);
        assert_eq!(
            steps[0].command,
            Command::New {
                side: Color::White,
                elo: DEFAULT_ELO,
                shadowchess: None,
                fen: Some("6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1".to_owned()),
            }
        );
        assert_eq!(steps[1].command, Command::Play(vec!["Ra8#".to_owned()]));
        assert_eq!(steps[2].command, Command::Show(Ply::End));
        assert_eq!(
            steps[3].command,
            Command::Screenshot(Path::new("base").join("shots/mate.png"))
        );
    }

    #[test]
    fn new_takes_a_strength_and_a_start() {
        assert_eq!(
            one("new black elo 2000"),
            Ok(Command::New {
                side: Color::Black,
                elo: 2000,
                shadowchess: None,
                fen: None
            })
        );
        assert_eq!(
            one("new white shadowchess 500"),
            Ok(Command::New {
                side: Color::White,
                elo: DEFAULT_ELO,
                shadowchess: Some(500),
                fen: None
            })
        );
        assert!(one("new white shadowchess weak").is_err());
        assert!(one("new green").is_err());
        assert!(one("new white elo strong").is_err());
        assert!(one("new white fen").is_err());
    }

    #[test]
    fn plays_several_moves_and_names_agents() {
        assert_eq!(
            one("play e4 Nf3"),
            Ok(Command::Play(vec!["e4".to_owned(), "Nf3".to_owned()]))
        );
        assert_eq!(
            one("review claude-code"),
            Ok(Command::Review {
                agent: AgentKind::ClaudeCode,
                model: None
            })
        );
        assert_eq!(
            one("review pi model openrouter/z-ai/glm-5.3-flash"),
            Ok(Command::Review {
                agent: AgentKind::Pi,
                model: Some("openrouter/z-ai/glm-5.3-flash".to_owned())
            })
        );
        assert!(one("review pi fast").is_err());
        assert_eq!(one("show 3"), Ok(Command::Show(Ply::At(3))));
        assert_eq!(
            one("settings agents"),
            Ok(Command::Settings(SettingsPage::Agents))
        );
        assert!(one("settings nowhere").is_err());
        assert_eq!(
            one("timeout 90"),
            Ok(Command::Timeout(Duration::from_secs(90)))
        );
    }

    #[test]
    fn says_which_line_is_wrong() {
        let error = parse("analyze\n\nfly away", Path::new("")).unwrap_err();
        assert_eq!(error, "line 3: unknown command `fly`");
        assert!(one("resign now").is_err());
        assert!(one("review deep-blue").is_err());
        assert!(one("screenshot").is_err());
    }
}
