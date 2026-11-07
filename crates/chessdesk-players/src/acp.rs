//! AI agents over the Agent Client Protocol. A session runs on its own thread, since the
//! protocol is async and the rest of Chessdesk isn't.
//!
//! Tokens are the point of most of this file: the agent gets a tiny system prompt, no
//! tools, no project and no settings, and every permission request is refused.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::v1::ConnectionTo;
use agent_client_protocol::v1::schema::{
    ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SessionId, SessionNotification,
    SessionUpdate, SystemPrompt, SystemPromptMode, TextContent,
};
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, Client};
use chessdesk_core::{Annotations, Game};
use futures::StreamExt;
use futures::channel::{mpsc, oneshot};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use shakmaty::{Color, Position};

use crate::{Decision, Opponent, PlayerError, Tokens, adapters, prompt, reply};

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    /// Settings this agent takes from its environment.
    fn environment(self, effort: Effort) -> Option<(&'static str, String)> {
        (self == Self::Codex).then(|| {
            let config = json!({ "model_reasoning_effort": effort.level() });
            ("CODEX_CONFIG", config.to_string())
        })
    }

    /// Whether the usage a turn reports is the session's running total, not the turn's.
    /// Measured: Copilot's grows by a full context each turn, Claude's doesn't.
    fn reports_running_total(self) -> bool {
        self == Self::Copilot
    }

    /// Settings only this agent's adapter understands. For Claude, an empty list of each
    /// keeps a session at about 870 tokens, against 28,000 by default.
    fn session_meta(self, model: Option<&str>, effort: Effort) -> Option<Map<String, Value>> {
        if self != Self::ClaudeCode {
            return None;
        }
        let mut options = json!({
            "tools": [],
            "settingSources": [],
            "plugins": [],
            "skills": [],
            "strictMcpConfig": true,
            "effort": effort.level(),
            "extraArgs": { "strict-mcp-config": null },
        });
        if let Some(model) = model {
            options["model"] = json!(model);
        }
        let mut meta = Map::new();
        meta.insert("claudeCode".to_owned(), json!({ "options": options }));
        Some(meta)
    }
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
    answer: oneshot::Sender<Answer>,
}

/// A live session with an agent. Dropping it ends the session and the agent.
pub struct Session {
    asks: mpsc::UnboundedSender<Ask>,
}

impl Session {
    /// Starts the agent, with `system` replacing its own system prompt.
    pub fn start(config: &AgentConfig, system: String) -> Self {
        let (asks, incoming) = mpsc::unbounded();
        let config = config.clone();
        thread::spawn(move || {
            // Failures reach callers as a closed channel, and through the answer below.
            if let Err(error) = futures::executor::block_on(run(config, system, incoming)) {
                eprintln!("agent session ended: {error}");
            }
        });
        Self { asks }
    }

    /// Sends a prompt and waits for the whole reply.
    pub fn ask(&self, text: String) -> Answer {
        let (answer, reply) = oneshot::channel();
        self.asks
            .unbounded_send(Ask { text, answer })
            .map_err(|_| PlayerError::Process("the agent is not running".to_owned()))?;
        futures::executor::block_on(reply)
            .map_err(|_| PlayerError::Process("the agent stopped".to_owned()))?
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

/// Connects to the agent and serves prompts until the [`Session`] goes away.
async fn run(
    config: AgentConfig,
    system: String,
    mut incoming: mpsc::UnboundedReceiver<Ask>,
) -> Result<(), agent_client_protocol::Error> {
    let model = config.model.as_deref();
    let Some(mut command) = adapters::command(config.kind, &config.adapters) else {
        return Err(agent_client_protocol::Error::internal_error());
    };
    command.extend(config.kind.arguments(model, config.effort));
    let (program, args) = command.split_first().expect("commands are not empty");
    let mut process = AcpAgentConfig::new(program).args(args.iter().cloned());
    if let Some((name, value)) = config.kind.environment(config.effort) {
        process = process.env(name, value);
    }
    let agent = AcpAgent::new(process);
    let meta = config.kind.session_meta(model, config.effort);
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
            } else {
                rules = Some(system);
            }
            let session = connection
                .send_request(session)
                .block_task()
                .await?
                .session_id;
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

impl Opponent for AgentOpponent {
    fn next_move(&mut self, game: &Game, instructions: &str) -> Result<Decision, PlayerError> {
        let position = game.position();
        // A forced move needs no thought, and no tokens.
        let legal = position.legal_moves();
        if let [only] = legal.as_slice() {
            return Ok(Decision {
                played: *only,
                annotations: Annotations::default(),
                tokens: None,
            });
        }

        let text = if self.started {
            prompt::turn(game)
        } else {
            prompt::opening(game, self.side)
        };
        self.started = true;
        let instructions = instructions.trim();
        let text = prompt::with_instructions(text, instructions, &self.told);
        instructions.clone_into(&mut self.told);
        let (first, mut tokens) = self.session.ask(text)?;
        if let Some((played, annotations)) = reply::parse(&first, &position) {
            return Ok(Decision {
                played,
                annotations,
                tokens,
            });
        }

        let (second, more) = self.session.ask(prompt::retry(&position))?;
        add_tokens(&mut tokens, more);
        match reply::parse(&second, &position) {
            Some((played, annotations)) => Ok(Decision {
                played,
                annotations,
                tokens,
            }),
            None => Err(PlayerError::NoLegalMove { reply: second }),
        }
    }
}

/// Has an agent comment on a game in one call, and returns the comments added and the
/// tokens spent. Run the engine over the game first: the review reads its evaluations.
pub fn review_game(
    config: &AgentConfig,
    game: &mut Game,
) -> Result<(usize, Option<Tokens>), PlayerError> {
    let system = prompt::review_system(config.reply_language.as_deref());
    let session = Session::start(config, system);
    let (text, tokens) = session.ask(prompt::review(game))?;
    Ok((reply::apply_review(&text, game), tokens))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_gets_no_tools_settings_or_plugins() {
        let meta = AgentKind::ClaudeCode
            .session_meta(Some("haiku"), Effort::High)
            .unwrap();
        let options = &meta["claudeCode"]["options"];
        for key in ["tools", "settingSources", "plugins", "skills"] {
            assert_eq!(options[key], json!([]), "{key}");
        }
        assert_eq!(options["model"], "haiku");
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
        let (name, value) = AgentKind::Codex.environment(Effort::High).unwrap();
        assert_eq!(name, "CODEX_CONFIG");
        assert_eq!(value, r#"{"model_reasoning_effort":"high"}"#);
        assert_eq!(AgentKind::Copilot.environment(Effort::High), None);
    }
}
