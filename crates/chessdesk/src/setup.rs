//! What the new game dialog decides, and how it turns into a running opponent.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use chessdesk_core::Game;
use chessdesk_players::Opponent;
use chessdesk_players::PlayerError;
use chessdesk_players::acp::{AgentConfig, AgentKind, AgentOpponent, Effort};
use chessdesk_players::adapters::{Readiness, readiness};
use chessdesk_players::download;
use chessdesk_players::uci::{Engine, EngineConfig};
use gpui::SharedString;
use serde::{Deserialize, Serialize};
use shakmaty::{Chess, Color};

use crate::i18n::{self, tr, tr_with};
use crate::scenario;
use crate::settings::Settings;

/// Which side the player takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    White,
    Black,
    Random,
}

/// What kind of game: how much help there is, and where it starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// Hints, take-backs and tutoring, with or without a clock.
    #[default]
    Casual,
    /// On the clock and on your own.
    Competition,
    /// From a set position, with the help of a casual game.
    Scenario,
}

/// How fast a clock is, split as Lichess does by how long a game of 40 moves takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    Bullet,
    Blitz,
    Rapid,
    Classical,
}

impl Speed {
    /// `None` for no clock, which no minutes are.
    pub fn of(minutes: u32, increment: u32) -> Option<Self> {
        (minutes > 0).then(|| match minutes * 60 + increment * 40 {
            ..180 => Self::Bullet,
            180..480 => Self::Blitz,
            480..1500 => Self::Rapid,
            _ => Self::Classical,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpponentKind {
    ClaudeCode,
    Codex,
    Copilot,
    OpenCode,
    Pi,
    Stockfish,
    Maia,
}

impl OpponentKind {
    pub const ALL: [Self; 7] = [
        Self::ClaudeCode,
        Self::Codex,
        Self::Copilot,
        Self::OpenCode,
        Self::Pi,
        Self::Stockfish,
        Self::Maia,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
            Self::Copilot => "Copilot",
            Self::OpenCode => "OpenCode",
            Self::Pi => "Pi",
            Self::Stockfish => "Stockfish",
            Self::Maia => "Maia",
        }
    }

    pub fn agent(self) -> Option<AgentKind> {
        match self {
            Self::ClaudeCode => Some(AgentKind::ClaudeCode),
            Self::Codex => Some(AgentKind::Codex),
            Self::Copilot => Some(AgentKind::Copilot),
            Self::OpenCode => Some(AgentKind::OpenCode),
            Self::Pi => Some(AgentKind::Pi),
            Self::Stockfish | Self::Maia => None,
        }
    }

    pub fn of_agent(kind: AgentKind) -> Self {
        match kind {
            AgentKind::ClaudeCode => Self::ClaudeCode,
            AgentKind::Codex => Self::Codex,
            AgentKind::Copilot => Self::Copilot,
            AgentKind::OpenCode => Self::OpenCode,
            AgentKind::Pi => Self::Pi,
        }
    }

    /// Whether Chessdesk can fetch what's missing itself, from the settings.
    pub fn downloadable(self) -> bool {
        match self {
            Self::Stockfish => download::can_download_stockfish(),
            Self::Maia => download::can_download_lc0(),
            agent => agent.agent().is_some_and(|kind| {
                readiness(kind, &Settings::adapters_dir().unwrap_or_default())
                    == Readiness::NoAdapter
            }),
        }
    }

    /// Models to pick from; the first is the cheap default. Empty leaves it to the agent.
    /// Why this opponent can't play on this machine, if so.
    pub fn missing(self, settings: &Settings) -> Option<String> {
        if let Some(kind) = self.agent() {
            let folder = Settings::adapters_dir().unwrap_or_default();
            return match readiness(kind, &folder) {
                Readiness::Ready => None,
                Readiness::NoNode => Some(tr("agent-needs-node").to_string()),
                Readiness::NoAdapter => {
                    Some(tr_with!("agent-adapter-missing", agent = self.name()).to_string())
                }
                Readiness::NoProgram(program) => {
                    Some(tr_with!("program-not-found", program = program).to_string())
                }
            };
        }
        match self {
            Self::Stockfish => which::which(&settings.stockfish)
                .is_err()
                .then(|| tr("stockfish-missing").to_string()),
            _ => (which::which(&settings.lc0).is_err() || !settings.maia_weights.is_dir())
                .then(|| tr("maia-missing").to_string()),
        }
    }
}

/// How an effort shows in the interface.
pub fn effort_label(effort: Effort) -> SharedString {
    tr(match effort {
        Effort::Low => "effort-low",
        Effort::Medium => "effort-medium",
        Effort::High => "effort-high",
    })
}

/// How a Maia network plays on Lichess, as opposed to the rating it was trained on. The bots
/// sit above their targets and much closer together: about 1450 to 1700 for 1100 to 1900.
pub fn maia_strength(level: u32) -> u32 {
    let strength = 1450 + level.saturating_sub(1100) * 250 / 800;
    (strength + 12) / 25 * 25
}

/// `haiku` as `Haiku`: models are lowercase on the command line only.
pub fn title_case(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// The dialog's choices, remembered for next time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Setup {
    pub mode: Mode,
    pub side: Side,
    pub opponent: OpponentKind,
    /// As the agent knows it; `None` is the agent's own default.
    pub model: Option<String>,
    /// The model as the agent names it, the default's too, to show who plays.
    pub model_name: Option<String>,
    pub effort: Effort,
    /// The player's own words to an agent on how to play.
    pub instructions: String,
    /// `None` is full strength.
    pub stockfish_elo: Option<u32>,
    pub maia_level: u32,
    /// Minutes each, and seconds added per move. No minutes is no clock.
    pub minutes: u32,
    pub increment: u32,
    /// Where a casual game or a scenario starts, if not from the usual position.
    pub fen: Option<String>,
    /// Tokens after which the game pauses, for agents that report them.
    pub token_budget: Option<u64>,
    pub annotate: bool,
    pub live_likelihood: bool,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            mode: Mode::Casual,
            side: Side::White,
            opponent: OpponentKind::Stockfish,
            model: None,
            model_name: None,
            effort: Effort::default(),
            instructions: String::new(),
            stockfish_elo: Some(1500),
            maia_level: 1500,
            minutes: 0,
            increment: 0,
            fen: None,
            token_budget: None,
            // On until turned off: the choice is saved with the rest when a game starts.
            annotate: true,
            live_likelihood: false,
        }
    }
}

impl Setup {
    fn path() -> Option<PathBuf> {
        Some(Settings::data_dir()?.join("last_game.toml"))
    }

    /// The last choices, or the defaults if there are none yet.
    pub fn load() -> Self {
        Self::path()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path().ok_or("no data folder on this system")?;
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder).map_err(|error| error.to_string())?;
        }
        let text = toml::to_string(self).map_err(|error| error.to_string())?;
        fs::write(path, text).map_err(|error| error.to_string())
    }

    /// The side the player gets. `coin` settles [`Side::Random`].
    pub fn player_color(&self, coin: bool) -> Color {
        match self.side {
            Side::Random if coin => Color::White,
            Side::White => Color::White,
            Side::Black | Side::Random => Color::Black,
        }
    }

    /// Switches to `mode`, bringing along what it can't do without: a clock for a
    /// competition, a position for a scenario.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        match mode {
            Mode::Competition if self.minutes == 0 => (self.minutes, self.increment) = (5, 3),
            Mode::Scenario if self.fen.is_none() => {
                let first = &scenario::ALL[0];
                self.fen = Some(first.fen.to_owned());
                self.side = first.side;
            }
            _ => {}
        }
    }

    /// Whether the player may have help: hints, take-backs, tutoring and the likelihood.
    pub fn assisted(&self) -> bool {
        self.mode != Mode::Competition
    }

    /// Whether the player's moves get a verdict from Stockfish: tutoring without an agent
    /// to do the talking.
    pub fn grades_moves(&self) -> bool {
        self.tutoring() && self.opponent.agent().is_none()
    }

    fn tutoring(&self) -> bool {
        self.annotate && self.assisted()
    }

    pub fn shows_likelihood(&self) -> bool {
        self.live_likelihood && self.assisted()
    }

    pub fn clock(&self) -> Option<(Duration, Duration)> {
        (self.minutes > 0).then(|| {
            (
                Duration::from_secs(u64::from(self.minutes) * 60),
                Duration::from_secs(u64::from(self.increment)),
            )
        })
    }

    pub fn start_position(&self) -> Option<Chess> {
        match (self.mode, &self.fen) {
            // A competition starts as every game does.
            (Mode::Competition, _) | (_, None) => Some(Chess::default()),
            (_, Some(fen)) => Game::parse_fen(fen),
        }
    }

    /// How the opponent is named in the game and on the panel.
    pub fn opponent_name(&self) -> String {
        match self.opponent {
            OpponentKind::Stockfish => match self.stockfish_elo {
                Some(elo) => format!("Stockfish {elo}"),
                None => "Stockfish".to_owned(),
            },
            OpponentKind::Maia => format!("Maia ~{}", maia_strength(self.maia_level)),
            kind => {
                let model = self
                    .model_name
                    .clone()
                    .or_else(|| self.model.as_deref().map(title_case));
                match model {
                    Some(model) => format!("{} ({model})", kind.name()),
                    None => kind.name().to_owned(),
                }
            }
        }
    }

    pub fn agent_config(&self) -> Option<AgentConfig> {
        Some(AgentConfig {
            kind: self.opponent.agent()?,
            model: self.model.clone(),
            effort: self.effort,
            adapters: Settings::adapters_dir().unwrap_or_default(),
            reply_language: i18n::reply_language(),
        })
    }

    /// The opponent's configuration, all of it sendable to a background thread.
    pub fn plan(&self, settings: &Settings, opponent_color: Color) -> OpponentPlan {
        match self.agent_config() {
            Some(config) => OpponentPlan::Agent {
                config,
                side: opponent_color,
                annotate: self.tutoring(),
            },
            None if self.opponent == OpponentKind::Maia => {
                let weights = settings
                    .maia_weights
                    .join(format!("maia-{}.pb.gz", self.maia_level));
                OpponentPlan::Engine(EngineConfig::maia(
                    &settings.lc0,
                    &weights.to_string_lossy(),
                ))
            }
            None => OpponentPlan::Engine(EngineConfig::stockfish(
                &settings.stockfish,
                self.stockfish_elo,
            )),
        }
    }
}

/// How to bring up an opponent, which happens off the UI thread.
#[derive(Clone, Debug)]
pub enum OpponentPlan {
    Agent {
        config: AgentConfig,
        side: Color,
        annotate: bool,
    },
    Engine(EngineConfig),
}

impl OpponentPlan {
    pub fn start(&self) -> Result<Box<dyn Opponent>, PlayerError> {
        Ok(match self {
            Self::Agent {
                config,
                side,
                annotate,
            } => Box::new(AgentOpponent::start(config, *side, *annotate)),
            Self::Engine(config) => Box::new(Engine::start(config)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_choices_read_back() {
        let setup = Setup {
            opponent: OpponentKind::ClaudeCode,
            model: Some("haiku".to_owned()),
            token_budget: Some(50_000),
            ..Setup::default()
        };
        let text = toml::to_string(&setup).unwrap();
        assert_eq!(toml::from_str::<Setup>(&text).unwrap(), setup);
    }

    #[test]
    fn choices_from_an_older_version_still_load() {
        let setup: Setup = toml::from_str("minutes = 5").unwrap();
        assert_eq!(setup.minutes, 5);
        assert_eq!(setup.opponent, OpponentKind::Stockfish);
    }

    #[test]
    fn tutoring_is_on_unless_turned_off() {
        assert!(Setup::default().annotate);
        let off = Setup {
            annotate: false,
            ..Setup::default()
        };
        let text = toml::to_string(&off).unwrap();
        assert!(!toml::from_str::<Setup>(&text).unwrap().annotate);
    }

    #[test]
    fn a_bad_fen_is_no_position() {
        let setup = Setup {
            mode: Mode::Scenario,
            fen: Some("not a position".to_owned()),
            ..Setup::default()
        };
        assert!(setup.start_position().is_none());
    }

    #[test]
    fn a_competition_starts_from_the_usual_position() {
        let mut setup = Setup {
            fen: Some("8/8/8/4k3/8/8/8/3QK3 w - - 0 1".to_owned()),
            ..Setup::default()
        };
        assert_ne!(setup.start_position().unwrap(), Chess::default());
        setup.set_mode(Mode::Competition);
        assert_eq!(setup.start_position().unwrap(), Chess::default());
    }

    #[test]
    fn a_competition_has_a_clock_and_no_help() {
        let mut setup = Setup {
            live_likelihood: true,
            ..Setup::default()
        };
        setup.set_mode(Mode::Competition);
        assert!(setup.clock().is_some());
        assert!(!setup.grades_moves());
        assert!(!setup.shows_likelihood());
        // What casual games had on stays for the next one.
        setup.set_mode(Mode::Casual);
        assert!(setup.grades_moves());
        assert!(setup.shows_likelihood());
    }

    #[test]
    fn clocks_are_as_fast_as_on_lichess() {
        assert_eq!(Speed::of(0, 0), None);
        assert_eq!(Speed::of(1, 0), Some(Speed::Bullet));
        assert_eq!(Speed::of(2, 1), Some(Speed::Bullet));
        assert_eq!(Speed::of(3, 0), Some(Speed::Blitz));
        assert_eq!(Speed::of(5, 3), Some(Speed::Blitz));
        assert_eq!(Speed::of(10, 0), Some(Speed::Rapid));
        assert_eq!(Speed::of(15, 10), Some(Speed::Rapid));
        assert_eq!(Speed::of(30, 0), Some(Speed::Classical));
    }

    #[test]
    fn model_names_read_as_names() {
        let saved = Setup {
            opponent: OpponentKind::ClaudeCode,
            model: Some("haiku".to_owned()),
            ..Setup::default()
        };
        assert_eq!(saved.opponent_name(), "Claude Code (Haiku)");
        let named = Setup {
            model_name: Some("Haiku 5.5".to_owned()),
            ..saved
        };
        assert_eq!(named.opponent_name(), "Claude Code (Haiku 5.5)");
        let default = Setup {
            model: None,
            ..named
        };
        assert_eq!(default.opponent_name(), "Claude Code (Haiku 5.5)");
    }

    #[test]
    fn maia_is_rated_by_how_it_plays() {
        assert_eq!(maia_strength(1100), 1450);
        assert_eq!(maia_strength(1900), 1700);
    }

    #[test]
    fn names_say_how_strong() {
        assert_eq!(Setup::default().opponent_name(), "Stockfish 1500");
        let maia = Setup {
            opponent: OpponentKind::Maia,
            ..Setup::default()
        };
        assert_eq!(maia.opponent_name(), "Maia ~1575");
    }
}
