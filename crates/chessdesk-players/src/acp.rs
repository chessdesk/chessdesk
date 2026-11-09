//! AI agents over the Agent Client Protocol. A session runs on its own thread, since the
//! protocol is async and the rest of Chessdesk isn't.
//!
//! Tokens are the point of most of this file: the agent gets a tiny system prompt, no
//! tools, no project and no settings, and every permission request is refused.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError, mpsc as std_mpsc};
use std::thread;
use std::time::Duration;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::v1::ConnectionTo;
use agent_client_protocol::v1::schema::{
    ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SessionConfigKind, SessionConfigOption,
    SessionConfigOptionCategory, SessionConfigOptionValue, SessionConfigSelectOption,
    SessionConfigSelectOptions, SessionId, SessionNotification, SessionUpdate,
    SetSessionConfigOptionRequest, SystemPrompt, SystemPromptMode, TextContent,
};
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, Client};
use chessdesk_core::{Annotations, Game};
use futures::StreamExt;
use futures::channel::mpsc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use shakmaty::{Color, Position};

use crate::{Decision, Intro, Opponent, PlayerError, Tokens, adapters, pi, prompt, reply};

/// How hard an agent thinks about a move. More effort plays better and costs more.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effort {
    #[default]
    Low,
    Medium,
    High,
}

impl Effort {
    pub const ALL: [Self; 3] = [Self::Low, Self::Medium, Self::High];

    /// How long a reply may take before it's given up on. Thinking hard takes long: on high
    /// effort, minutes for one move are normal.
    pub fn answer_within(self) -> Duration {
        match self {
            Self::Low => Duration::from_mins(3),
            Self::Medium => Duration::from_mins(8),
            Self::High => Duration::from_mins(20),
        }
    }

    /// The word every agent uses for it.
    fn level(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// The agents Chessdesk knows how to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AgentKind {
    ClaudeCode,
    Codex,
    Copilot,
    OpenCode,
    Pi,
}

impl AgentKind {
    pub const ALL: [Self; 5] = [
        Self::ClaudeCode,
        Self::Codex,
        Self::Copilot,
        Self::OpenCode,
        Self::Pi,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
            Self::Copilot => "Copilot",
            Self::OpenCode => "OpenCode",
            Self::Pi => "Pi",
        }
    }

    /// What the agent's program is started with.
    fn arguments(self, model: Option<&str>, effort: Effort) -> Vec<String> {
        match self {
            Self::ClaudeCode | Self::Codex | Self::Pi => Vec::new(),
            Self::OpenCode => vec!["acp".to_owned()],
            Self::Copilot => {
                let mut arguments: Vec<String> = [
                    "--acp",
                    "--no-custom-instructions",
                    "--disable-builtin-mcps",
                    "--no-ask-user",
                    "--available-tools=",
                ]
                .map(str::to_owned)
                .into();
                if let Some(model) = model {
                    arguments.extend(["--model".to_owned(), model.to_owned()]);
                }
                arguments.extend(["--reasoning-effort".to_owned(), effort.level().to_owned()]);
                arguments
            }
        }
    }

    /// Settings this agent takes from its environment. `system` is Chessdesk's prompt.
    fn environment(self, effort: Effort, system: &str) -> Vec<(&'static str, String)> {
        match self {
            Self::Codex => {
                let config = json!({ "model_reasoning_effort": effort.level() });
                vec![("CODEX_CONFIG", config.to_string())]
            }
            Self::OpenCode => opencode_environment(system),
            Self::ClaudeCode | Self::Copilot | Self::Pi => Vec::new(),
        }
    }

    /// Whether the agent gets Chessdesk's prompt from its start, in place of its own,
    /// without the protocol: `OpenCode` as an agent of its own config, Pi from a launcher.
    fn prompt_from_start(self) -> bool {
        matches!(self, Self::OpenCode | Self::Pi)
    }

    /// Whether the usage a turn reports is the session's running total, not the turn's.
    /// Measured: Copilot's grows by a full context each turn, Claude's doesn't.
    fn reports_running_total(self) -> bool {
        self == Self::Copilot
    }

    /// Settings only this agent's adapter understands. For Claude, an empty list of each
    /// keeps a session at about 870 tokens, against 28,000 by default.
    fn session_meta(self, effort: Effort) -> Option<Map<String, Value>> {
        if self != Self::ClaudeCode {
            return None;
        }
        let options = json!({
            "tools": [],
            "settingSources": [],
            "plugins": [],
            "skills": [],
            "strictMcpConfig": true,
            "effort": effort.level(),
            "extraArgs": { "strict-mcp-config": null },
        });
        let mut meta = Map::new();
        meta.insert("claudeCode".to_owned(), json!({ "options": options }));
        Some(meta)
    }
}

/// `OpenCode` can't have its prompt replaced over ACP, and by default sends about 13,000
/// tokens a turn: its coding prompt, ten tool schemas, and the user's `~/.claude/CLAUDE.md`
/// and skills. It also runs its tools without asking, so a hint once downloaded Stockfish
/// to answer. This config gives it an agent with Chessdesk's prompt and every tool denied,
/// which leaves about 160 tokens of its own, and switches off the title it would generate
/// in a call of its own.
fn opencode_environment(system: &str) -> Vec<(&'static str, String)> {
    let config = json!({
        "default_agent": "chessdesk",
        "agent": {
            "chessdesk": {
                "mode": "primary",
                "prompt": system,
                "permission": { "*": "deny" },
            },
            "title": { "disable": true },
        },
        "autoupdate": false,
        "share": "disabled",
    });
    vec![
        ("OPENCODE_CONFIG_CONTENT", config.to_string()),
        ("OPENCODE_DISABLE_CLAUDE_CODE", "1".to_owned()),
    ]
}

/// How to run an agent for one game or review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentConfig {
    pub kind: AgentKind,
    pub model: Option<String>,
    pub effort: Effort,
    /// Where Chessdesk keeps adapters.
    pub adapters: PathBuf,
    /// What comments are written in, if not English.
    pub reply_language: Option<String>,
}

type Answer = Result<(String, Option<Tokens>), PlayerError>;

/// One prompt, and where its answer goes.
struct Ask {
    text: String,
    answer: std_mpsc::SyncSender<Answer>,
}

/// A live session with an agent. Dropping it ends the session and the agent.
pub struct Session {
    asks: mpsc::UnboundedSender<Ask>,
    /// How long a reply may take.
    limit: Duration,
}

impl Session {
    /// Starts the agent, with `system` replacing its own system prompt.
    pub fn start(config: &AgentConfig, system: String) -> Self {
        let (asks, incoming) = mpsc::unbounded();
        let thread_config = config.clone();
        thread::spawn(move || {
            // Failures reach callers as a closed channel, and through the answer below.
            if let Err(error) = futures::executor::block_on(run(thread_config, system, incoming)) {
                eprintln!("agent session ended: {error}");
            }
        });
        Self {
            asks,
            limit: config.effort.answer_within(),
        }
    }

    /// Sends a prompt and waits for the whole reply, as long as the effort allows. A session
    /// that ran out of time is still busy with that prompt: start a new one.
    pub fn ask(&self, text: String) -> Answer {
        let (answer, reply) = std_mpsc::sync_channel(1);
        self.asks
            .unbounded_send(Ask { text, answer })
            .map_err(|_| PlayerError::Process("the agent is not running".to_owned()))?;
        reply
            .recv_timeout(self.limit)
            .map_err(|error| match error {
                std_mpsc::RecvTimeoutError::Timeout => PlayerError::TimedOut(self.limit),
                std_mpsc::RecvTimeoutError::Disconnected => {
                    PlayerError::Process("the agent stopped".to_owned())
                }
            })?
    }
}

/// An empty folder to run agents in, so that no project's instructions get loaded.
fn empty_folder() -> Result<PathBuf, agent_client_protocol::Error> {
    let folder = std::env::temp_dir().join("chessdesk-agent");
    std::fs::create_dir_all(&folder).map_err(agent_client_protocol::Error::into_internal_error)?;
    Ok(folder)
}

fn text_block(text: String) -> ContentBlock {
    ContentBlock::Text(TextContent::new(text))
}

/// The agent's process, as `config` asks for it, with `system` as its prompt where it
/// takes that from the start.
fn process(config: &AgentConfig, system: &str) -> Result<AcpAgent, agent_client_protocol::Error> {
    let Some(mut command) = adapters::command(config.kind, &config.adapters) else {
        return Err(agent_client_protocol::Error::internal_error());
    };
    command.extend(
        config
            .kind
            .arguments(config.model.as_deref(), config.effort),
    );
    let (program, args) = command.split_first().expect("commands are not empty");
    let mut process = AcpAgentConfig::new(program).args(args.iter().cloned());
    for (name, value) in config.kind.environment(config.effort, system) {
        process = process.env(name, value);
    }
    if config.kind == AgentKind::Pi {
        // Beside the agents' folder, not in it: that one stays empty.
        let folder = std::env::temp_dir().join("chessdesk-pi");
        let launcher = pi::launcher(system, &folder)
            .map_err(agent_client_protocol::Error::into_internal_error)?;
        process = process.env("PI_ACP_PI_COMMAND", launcher.display().to_string());
    }
    Ok(AcpAgent::new(process))
}

/// A model an agent offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// What the agent knows it by, to ask for it.
    pub id: String,
    /// What it shows as.
    pub name: String,
    pub description: Option<String>,
}

/// The models an agent offers, as it says when a session starts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Models {
    /// The session setting they're the choices of. `None` for an agent that takes its model
    /// only on its command line, and lists none.
    pub option: Option<String>,
    /// The one a session starts with: the agent's own default.
    pub current: String,
    pub choices: Vec<Model>,
}

impl Models {
    /// The model choice among a session's settings, if the agent offers one.
    fn from_options(options: &[SessionConfigOption]) -> Option<Self> {
        let option = options
            .iter()
            .find(|option| option.category == Some(SessionConfigOptionCategory::Model))?;
        let SessionConfigKind::Select(select) = &option.kind else {
            return None;
        };
        let model = |choice: &SessionConfigSelectOption| Model {
            id: choice.value.to_string(),
            name: choice.name.clone(),
            description: choice.description.clone(),
        };
        let choices = match &select.options {
            SessionConfigSelectOptions::Ungrouped(choices) => choices.iter().map(model).collect(),
            SessionConfigSelectOptions::Grouped(groups) => groups
                .iter()
                .flat_map(|group| group.options.iter().map(model))
                .collect(),
            _ => Vec::new(),
        };
        Some(Self {
            option: Some(option.id.to_string()),
            current: select.current_value.to_string(),
            choices,
        })
    }

    /// The model with `id`, if it's offered.
    pub fn find(&self, id: &str) -> Option<&Model> {
        self.choices.iter().find(|model| model.id == id)
    }
}

/// Copilot's model, as its own settings name it: it lists none over the protocol. `auto`
/// lets it pick, which is also what it does with none set.
fn copilot_model() -> String {
    let settings = std::env::var_os("COPILOT_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(std::env::home_dir()?.join(".copilot")))
        .map(|folder| folder.join("settings.json"));
    settings
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|settings| settings.get("model")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| "auto".to_owned())
}

/// Asks the agent which models it offers: starts it, opens a session and stops again. No
/// prompt, so no tokens. `None` if it offers no choice.
pub fn models(config: &AgentConfig) -> Result<Option<Models>, PlayerError> {
    if config.kind == AgentKind::Copilot {
        return Ok(Some(Models {
            option: None,
            current: copilot_model(),
            choices: Vec::new(),
        }));
    }
    let process = |error: agent_client_protocol::Error| PlayerError::Process(error.to_string());
    let agent = self::process(config, "").map_err(process)?;
    futures::executor::block_on(Client.v1().connect_with(
        agent,
        async move |connection: ConnectionTo<Agent>| {
            connection
                .send_request(InitializeRequest::new(ProtocolVersion::V1))
                .block_task()
                .await?;
            let session = connection
                .send_request(NewSessionRequest::new(empty_folder()?))
                .block_task()
                .await?;
            Ok(session
                .config_options
                .as_deref()
                .and_then(Models::from_options))
        },
    ))
    .map_err(process)
}

/// Connects to the agent and serves prompts until the [`Session`] goes away.
async fn run(
    config: AgentConfig,
    system: String,
    mut incoming: mpsc::UnboundedReceiver<Ask>,
) -> Result<(), agent_client_protocol::Error> {
    let model = config.model.as_deref();
    let agent = process(&config, &system)?;
    let meta = config.kind.session_meta(config.effort);
    let reply_text = Arc::new(Mutex::new(String::new()));
    let collected = reply_text.clone();

    Client
        .v1()
        .on_receive_notification(
            async move |notification: SessionNotification, _| {
                if let SessionUpdate::AgentMessageChunk(chunk) = notification.update
                    && let ContentBlock::Text(text) = chunk.content
                {
                    collected
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push_str(&text.text);
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            // Chess needs no tools, so nothing is ever allowed.
            async move |_: RequestPermissionRequest, responder, _| {
                responder.respond(RequestPermissionResponse::new(
                    RequestPermissionOutcome::Cancelled,
                ))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(agent, async move |connection: ConnectionTo<Agent>| {
            let agent = connection
                .send_request(InitializeRequest::new(ProtocolVersion::V1))
                .block_task()
                .await?;
            let replaces_prompt = agent
                .agent_capabilities
                .session_capabilities
                .system_prompt
                .and_then(|capability| capability.mode)
                .and_then(|mode| mode.override_)
                .is_some();
            let mut session = NewSessionRequest::new(empty_folder()?).meta(meta);
            // Without its own prompt to replace, the rules have to come first in the talk.
            let mut rules = None;
            if replaces_prompt {
                let prompt = SystemPrompt::new(vec![text_block(system)]);
                session = session.system_prompt(prompt.mode(SystemPromptMode::Override));
            } else if !config.kind.prompt_from_start() {
                rules = Some(system);
            }
            let opened = connection.send_request(session).block_task().await?;
            let session = opened.session_id;
            let mut options = opened.config_options.unwrap_or_default();
            let offered = Models::from_options(&options);
            if let Some(change) = model.and_then(|model| model_change(offered.as_ref()?, model)) {
                options = connection
                    .send_request(SetSessionConfigOptionRequest::new(
                        session.clone(),
                        change.0,
                        SessionConfigOptionValue::value_id(change.1),
                    ))
                    .block_task()
                    .await?
                    .config_options;
            }
            // After the model, whose levels these are.
            if let Some((option, level)) = effort_change(&options, config.effort) {
                connection
                    .send_request(SetSessionConfigOptionRequest::new(
                        session.clone(),
                        option,
                        SessionConfigOptionValue::value_id(level),
                    ))
                    .block_task()
                    .await?;
            }
            let mut running = Tokens::default();
            while let Some(ask) = incoming.next().await {
                let text = match rules.take() {
                    Some(rules) => format!("{rules}\n\n{}", ask.text),
                    None => ask.text,
                };
                let answer = prompt_once(&connection, &session, text, &reply_text)
                    .await
                    .map(|(text, tokens)| {
                        let tokens = tokens.map(|tokens| {
                            if config.kind.reports_running_total() {
                                turn_share(tokens, &mut running)
                            } else {
                                tokens
                            }
                        });
                        (text, tokens)
                    });
                // The asker may have given up, which is no reason to stop.
                let _ = ask.answer.send(answer);
            }
            Ok(())
        })
        .await
}

/// The setting and value that switch a session from the model it started with to `model`,
/// if it needs switching and the agent takes it that way.
fn model_change(offered: &Models, model: &str) -> Option<(String, String)> {
    let option = offered.option.clone()?;
    (offered.current != model).then(|| (option, model.to_owned()))
}

/// The setting and value that set how hard a session thinks to `effort`, where the agent
/// offers that as a session setting with the same level: Pi does, and started on its own
/// default, which was `high`.
fn effort_change(options: &[SessionConfigOption], effort: Effort) -> Option<(String, String)> {
    let option = options
        .iter()
        .find(|option| option.category == Some(SessionConfigOptionCategory::ThoughtLevel))?;
    let SessionConfigKind::Select(select) = &option.kind else {
        return None;
    };
    let SessionConfigSelectOptions::Ungrouped(levels) = &select.options else {
        return None;
    };
    let level = effort.level();
    let offered = levels
        .iter()
        .any(|choice| choice.value.to_string() == level);
    (offered && select.current_value.to_string() != level)
        .then(|| (option.id.to_string(), level.to_owned()))
}

/// What a turn added to a running total.
fn turn_share(total: Tokens, running: &mut Tokens) -> Tokens {
    let share = Tokens {
        total: total.total.saturating_sub(running.total),
        cached: total.cached.saturating_sub(running.cached),
    };
    *running = total;
    share
}

async fn prompt_once(
    connection: &ConnectionTo<Agent>,
    session: &SessionId,
    text: String,
    reply_text: &Mutex<String>,
) -> Answer {
    reply_text
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    let response = connection
        .send_request(PromptRequest::new(session.clone(), vec![text_block(text)]))
        .block_task()
        .await
        .map_err(|error| PlayerError::Process(error.to_string()))?;
    let tokens = response.usage.map(|usage| Tokens {
        total: usage.total_tokens,
        cached: usage.cached_read_tokens.unwrap_or(0),
    });
    let text = reply_text
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    Ok((text, tokens))
}

/// An agent playing one side of a game.
pub struct AgentOpponent {
    session: Session,
    side: Color,
    /// Whether it coaches: only then has it something to say before the first move.
    annotate: bool,
    /// Whether the session has been told the game so far.
    started: bool,
    /// The instructions the session has heard.
    told: String,
}

impl AgentOpponent {
    pub fn start(config: &AgentConfig, side: Color, annotate: bool) -> Self {
        Self {
            session: Session::start(
                config,
                prompt::system(annotate, config.reply_language.as_deref()),
            ),
            side,
            annotate,
            started: false,
            told: String::new(),
        }
    }
}

fn add_tokens(total: &mut Option<Tokens>, more: Option<Tokens>) {
    if let Some(more) = more {
        *total.get_or_insert_default() += more;
    }
}

/// Reads a move from the session's reply to `text`, and if there's none, asks once more with
/// the legal moves listed.
fn ask_for_move(
    session: &Session,
    text: String,
    position: &shakmaty::Chess,
) -> Result<Decision, PlayerError> {
    let (first, mut tokens) = session.ask(text)?;
    if let Some((played, annotations)) = reply::parse(&first, position) {
        return Ok(Decision {
            played,
            annotations,
            tokens,
        });
    }
    let (second, more) = session.ask(prompt::retry(position))?;
    add_tokens(&mut tokens, more);
    match reply::parse(&second, position) {
        Some((played, annotations)) => Ok(Decision {
            played,
            annotations,
            tokens,
        }),
        // Twice nothing: Pi's adapter ends a turn the provider refused, for a bad key or an
        // empty quota, with no words and no error, and a model can think a turn away.
        None if second.trim().is_empty() => Err(PlayerError::Process(
            "the agent replied with nothing, twice: check its login, its quota and the model"
                .to_owned(),
        )),
        None => Err(PlayerError::NoLegalMove { reply: second }),
    }
}

impl Opponent for AgentOpponent {
    fn next_move(&mut self, game: &Game, instructions: &str) -> Result<Decision, PlayerError> {
        let position = game.position();
        // A forced move needs no thought, and no tokens, unless the agent coaches: the move
        // that forced it is often the player's worst, and needs a word most.
        let legal = position.legal_moves();
        if let [only] = legal.as_slice()
            && !self.annotate
        {
            return Ok(Decision {
                played: *only,
                annotations: Annotations::default(),
                tokens: None,
            });
        }

        let text = if self.started {
            prompt::turn(game, self.annotate)
        } else {
            prompt::opening(game, self.side)
        };
        self.started = true;
        let instructions = instructions.trim();
        let text = prompt::with_instructions(text, instructions, &self.told);
        instructions.clone_into(&mut self.told);
        ask_for_move(&self.session, text, &position)
    }

    fn introduce(&mut self, game: &Game, instructions: &str) -> Result<Option<Intro>, PlayerError> {
        if !self.annotate || self.started {
            return Ok(None);
        }
        // Told its side and the position here, the session takes the player's first move
        // as a turn like any other.
        self.started = true;
        let instructions = instructions.trim();
        let text = prompt::introduction(game, self.side);
        let text = prompt::with_instructions(text, instructions, &self.told);
        instructions.clone_into(&mut self.told);
        let (reply, tokens) = self.session.ask(text)?;
        let annotations = reply::remark(&reply, &game.position());
        Ok((!annotations.is_empty()).then_some(Intro {
            annotations,
            tokens,
        }))
    }
}

/// Has an agent suggest a move for whoever is to move in `position`, with why, in a session
/// of its own: the move to play, its comment, and the tokens spent.
pub fn hint(config: &AgentConfig, position: &shakmaty::Chess) -> Result<Decision, PlayerError> {
    let session = Session::start(
        config,
        prompt::hint_system(config.reply_language.as_deref()),
    );
    ask_for_move(&session, prompt::hint(position), position)
}

/// What a review added to the game, and what it cost.
#[derive(Debug)]
pub struct Review {
    /// How many moves got a comment.
    pub comments: usize,
    /// The agent's word on the whole game.
    pub summary: Option<String>,
    pub tokens: Option<Tokens>,
}

/// Has an agent comment on a game and sum it up, in one call. The comments go into the game;
/// the summary is the caller's to keep. Run the engine over the game first: the review reads
/// its evaluations.
pub fn review_game(config: &AgentConfig, game: &mut Game) -> Result<Review, PlayerError> {
    let system = prompt::review_system(config.reply_language.as_deref());
    let session = Session::start(config, system);
    let (text, tokens) = session.ask(prompt::review(game))?;
    Ok(Review {
        comments: reply::apply_review(&text, game),
        summary: reply::review_summary(&text),
        tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offered() -> Vec<SessionConfigOption> {
        let mode = SessionConfigOption::select(
            "mode",
            "Mode",
            "agent",
            vec![SessionConfigSelectOption::new("agent", "Agent")],
        )
        .category(SessionConfigOptionCategory::Mode);
        let model = SessionConfigOption::select(
            "model",
            "Model",
            "opus",
            vec![
                SessionConfigSelectOption::new("opus", "Opus"),
                SessionConfigSelectOption::new("haiku", "Haiku").description("Fastest"),
            ],
        )
        .category(SessionConfigOptionCategory::Model);
        vec![mode, model]
    }

    #[test]
    fn models_are_read_from_the_model_setting() {
        let models = Models::from_options(&offered()).unwrap();
        assert_eq!(models.option.as_deref(), Some("model"));
        assert_eq!(models.current, "opus");
        assert_eq!(
            models.find("haiku").unwrap().description.as_deref(),
            Some("Fastest")
        );
        assert!(Models::from_options(&offered()[..1]).is_none());
    }

    #[test]
    fn effort_is_set_where_the_agent_offers_that_level() {
        let thinking = |current| {
            let levels = ["off", "low", "medium", "high"]
                .map(|level| SessionConfigSelectOption::new(level, level))
                .to_vec();
            SessionConfigOption::select("thought_level", "Thinking", current, levels)
                .category(SessionConfigOptionCategory::ThoughtLevel)
        };
        assert_eq!(
            effort_change(&[thinking("high")], Effort::Low),
            Some(("thought_level".to_owned(), "low".to_owned()))
        );
        assert_eq!(effort_change(&[thinking("low")], Effort::Low), None);
        // No such setting: the effort goes some other way, or not at all.
        assert_eq!(effort_change(&offered(), Effort::Low), None);
    }

    #[test]
    fn a_session_switches_only_to_another_model() {
        let models = Models::from_options(&offered()).unwrap();
        assert_eq!(
            model_change(&models, "haiku"),
            Some(("model".to_owned(), "haiku".to_owned()))
        );
        assert_eq!(model_change(&models, "opus"), None);
        let command_line = Models {
            option: None,
            ..models
        };
        assert_eq!(model_change(&command_line, "haiku"), None);
    }

    #[test]
    fn claude_gets_no_tools_settings_or_plugins() {
        let meta = AgentKind::ClaudeCode.session_meta(Effort::High).unwrap();
        let options = &meta["claudeCode"]["options"];
        for key in ["tools", "settingSources", "plugins", "skills"] {
            assert_eq!(options[key], json!([]), "{key}");
        }
        assert_eq!(options["effort"], "high");
    }

    #[test]
    fn a_running_total_turns_into_each_turns_share() {
        let mut running = Tokens::default();
        let first = Tokens {
            total: 11660,
            cached: 0,
        };
        let second = Tokens {
            total: 23411,
            cached: 11556,
        };
        assert_eq!(turn_share(first, &mut running), first);
        let share = turn_share(second, &mut running);
        assert_eq!(share.total, 11751);
        assert_eq!(share.cached, 11556);
    }

    #[test]
    fn copilot_starts_without_tools_or_instructions() {
        let arguments = AgentKind::Copilot
            .arguments(Some("gpt-5.4"), Effort::Medium)
            .join(" ");
        assert!(arguments.starts_with("--acp --no-custom-instructions"));
        assert!(arguments.contains("--available-tools="));
        assert!(arguments.ends_with("--model gpt-5.4 --reasoning-effort medium"));
    }

    #[test]
    fn opencode_speaks_acp_itself() {
        assert_eq!(AgentKind::OpenCode.arguments(None, Effort::Low), ["acp"]);
    }

    #[test]
    fn codex_gets_its_effort_from_the_environment() {
        let environment = AgentKind::Codex.environment(Effort::High, "");
        assert_eq!(
            environment,
            [(
                "CODEX_CONFIG",
                r#"{"model_reasoning_effort":"high"}"#.to_owned()
            )]
        );
        assert_eq!(AgentKind::Copilot.environment(Effort::High, "").len(), 0);
    }

    #[test]
    fn opencode_gets_chessdesks_prompt_and_no_tools() {
        let environment = AgentKind::OpenCode.environment(Effort::Low, "You play chess.");
        let config = &environment
            .iter()
            .find(|(name, _)| *name == "OPENCODE_CONFIG_CONTENT")
            .unwrap()
            .1;
        let config: Value = serde_json::from_str(config).unwrap();
        let agent = &config["agent"][config["default_agent"].as_str().unwrap()];
        assert_eq!(agent["prompt"], "You play chess.");
        assert_eq!(agent["permission"]["*"], "deny");
        assert_eq!(config["agent"]["title"]["disable"], true);
        // Or it would read the user's own CLAUDE.md and skills into every turn.
        assert!(environment.contains(&("OPENCODE_DISABLE_CLAUDE_CODE", "1".to_owned())));
        assert!(AgentKind::OpenCode.prompt_from_start());
    }
}
