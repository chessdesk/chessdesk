//! Runs a script against the app, as the player would: each command waits until the game on
//! show has settled, and any error the app reports fails the script. The process ends with the
//! script, exiting 0 if it all went through and 1 if not.

use std::time::{Duration, Instant};

use chessdesk_core::Annotations;
use gpui::{AppContext, AsyncWindowContext, Context, WeakEntity, Window};
use shakmaty::san::SanPlus;

use chessdesk_players::acp::AgentConfig;

use super::Chessdesk;
use super::flow::helper_config;
use crate::screenshot;
use crate::script::{Command, Ply, Step};
use crate::setup::{Setup, Side};

/// How long a command waits for the game to settle unless the script says otherwise: enough
/// for Stockfish to answer and analyse a short game.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
/// How often a waiting script looks again.
const POLL: Duration = Duration::from_millis(100);
/// Long enough for pieces to land and sections to unfold before a screenshot.
const ANIMATIONS: Duration = Duration::from_millis(600);

impl Chessdesk {
    /// Runs `steps`, then ends the process.
    pub fn run_script(steps: Vec<Step>, window: &Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let code = match run(&this, steps, cx).await {
                Ok(()) => {
                    eprintln!("script: done");
                    0
                }
                Err(error) => {
                    eprintln!("script: {error}");
                    1
                }
            };
            std::process::exit(code);
        })
        .detach();
    }

    /// Whether nothing is under way in the game on show: no opponent or agent at work, no
    /// evaluation, no pieces moving.
    fn is_settled(&self) -> bool {
        let session = self.session();
        let live_busy = session
            .live
            .as_ref()
            .is_some_and(|live| live.thinking || live.evaluating);
        !live_busy
            && session.analysis.is_none()
            && session.reviewer.is_none()
            && !session.suggesting
            && session.slides.is_empty()
    }

    fn run_command(&mut self, command: Command, cx: &mut Context<Self>) -> Result<(), String> {
        match command {
            Command::New { side, elo, fen } => {
                let setup = Setup {
                    side: if side.is_white() {
                        Side::White
                    } else {
                        Side::Black
                    },
                    stockfish_elo: Some(elo),
                    fen,
                    annotate: false,
                    ..Setup::default()
                };
                self.begin_game(setup, cx);
            }
            Command::Open(path) => self.open(&path, cx),
            Command::Resign => self.resign(cx),
            Command::Analyze => self.analyze(cx),
            Command::Review { agent, model } => {
                let config = AgentConfig {
                    model: model.or(helper_config(agent).model),
                    ..helper_config(agent)
                };
                self.review_using(config, cx);
            }
            Command::Show(ply) => {
                let ply = match ply {
                    Ply::At(ply) => ply,
                    Ply::End => self.session().game.moves().len(),
                };
                self.step_to(ply, cx);
            }
            Command::Discard => {
                let path = self
                    .session()
                    .path
                    .clone()
                    .ok_or("no saved game to discard")?;
                self.delete_saved(&path, cx);
            }
            // Handled by the loop, which needs the window or keeps the setting.
            Command::Play(_) | Command::Timeout(_) | Command::Screenshot(_) => {}
        }
        Ok(())
    }

    /// Plays the player's move, given in SAN, in the game on show.
    fn play_san(&mut self, san: &str, cx: &mut Context<Self>) -> Result<(), String> {
        let session = self.session();
        if !session.player_to_move() {
            return Err(format!("{san}: it's not the player's move"));
        }
        let position = session.game.position();
        let played = san
            .parse::<SanPlus>()
            .map_err(|error| format!("{san}: {error}"))?
            .san
            .to_move(&position)
            .map_err(|_| format!("{san}: not a legal move in {}", session.game.fen()))?;
        let id = self.active;
        self.play(id, played, Annotations::default(), true, cx);
        Ok(())
    }
}

async fn run(
    this: &WeakEntity<Chessdesk>,
    steps: Vec<Step>,
    cx: &mut AsyncWindowContext,
) -> Result<(), String> {
    // Errors from before the script, such as a setting that didn't load, aren't its own.
    let mut reported = this.update(cx, |this, _| this.errors.len()).map_err(gone)?;
    let mut timeout = DEFAULT_TIMEOUT;
    for step in steps {
        eprintln!("script: {}: {}", step.line, step.text);
        let at = |error: String| format!("line {}: {error}", step.line);
        settle(this, timeout, cx).await.map_err(at)?;
        match step.command {
            Command::Timeout(limit) => timeout = limit,
            Command::Play(moves) => {
                for san in moves {
                    settle(this, timeout, cx).await.map_err(at)?;
                    this.update(cx, |this, cx| this.play_san(&san, cx))
                        .map_err(gone)?
                        .map_err(at)?;
                }
            }
            Command::Screenshot(path) => {
                cx.background_executor().timer(ANIMATIONS).await;
                let shot = this
                    .update_in(cx, |_, window, _| screenshot::capture(window, &path))
                    .map_err(gone)?
                    .map_err(at)?;
                // Off the window's thread, which has to paint the picture meanwhile.
                cx.background_spawn(async move { screenshot::take(shot) })
                    .await
                    .map_err(at)?;
                eprintln!("script: saved {}", path.display());
            }
            command => this
                .update(cx, |this, cx| this.run_command(command, cx))
                .map_err(gone)?
                .map_err(at)?,
        }
        settle(this, timeout, cx).await.map_err(at)?;
        let errors: Vec<String> = this
            .update(cx, |this, _| {
                let new = this.errors[reported.min(this.errors.len())..].to_vec();
                reported = this.errors.len();
                new.into_iter().map(String::from).collect()
            })
            .map_err(gone)?;
        if !errors.is_empty() {
            return Err(at(errors.join("; ")));
        }
    }
    Ok(())
}

/// An app that has closed under the script.
fn gone(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Waits until the game on show has settled, for at most `timeout`.
async fn settle(
    this: &WeakEntity<Chessdesk>,
    timeout: Duration,
    cx: &mut AsyncWindowContext,
) -> Result<(), String> {
    let started = Instant::now();
    loop {
        let settled = this.update(cx, |this, _| this.is_settled()).map_err(gone)?;
        if settled {
            return Ok(());
        }
        if started.elapsed() > timeout {
            return Err(format!(
                "still busy after {} s; raise it with `timeout`",
                timeout.as_secs()
            ));
        }
        cx.background_executor().timer(POLL).await;
    }
}
