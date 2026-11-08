//! Engines and agent adapters being fetched. The window owns this rather than the settings
//! page that starts the work, so the work carries on, and shows, after the page closes.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use chessdesk_players::acp::AgentKind;
use chessdesk_players::{adapters, download};
use gpui::{AppContext, Context, EventEmitter, SharedString};

use crate::i18n::{tr, tr_with};
use crate::settings::Settings;

/// How often a download's progress is looked at.
const PROGRESS_TICK: Duration = Duration::from_millis(200);

/// An engine that can be fetched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Stockfish,
    Maia,
}

impl Engine {
    fn name(self) -> &'static str {
        match self {
            Self::Stockfish => "Stockfish",
            Self::Maia => "Maia",
        }
    }
}

pub enum Event {
    /// Something new is installed, and the settings point at it.
    Changed,
    Failed(String),
}

/// What a running download reports.
enum Update {
    Progress(f32),
    Done(Result<Vec<(&'static str, PathBuf)>, String>),
}

#[derive(Default)]
pub struct Installs {
    /// What's being fetched, and how far along it is, from 0 to 1.
    downloading: Option<(Engine, f32)>,
    /// The agent whose adapter is being installed.
    installing: Option<AgentKind>,
}

impl EventEmitter<Event> for Installs {}

/// Fetches an engine and says which settings now point at it.
fn install_engine(
    engine: Engine,
    root: &Path,
    progress: &mut dyn FnMut(f32),
) -> Result<Vec<(&'static str, PathBuf)>, String> {
    match engine {
        Engine::Stockfish => {
            let program = download::install_stockfish(root, progress)?;
            Ok(vec![("stockfish", program)])
        }
        Engine::Maia => {
            let (lc0, weights) = download::install_maia(root, progress)?;
            Ok(vec![("lc0", lc0), ("maia_weights", weights)])
        }
    }
}

impl Installs {
    pub fn downloading(&self) -> Option<(Engine, f32)> {
        self.downloading
    }

    pub fn installing(&self) -> Option<AgentKind> {
        self.installing
    }

    /// What's under way, for the title bar.
    pub fn activity(&self) -> Option<SharedString> {
        if let Some((engine, done)) = self.downloading {
            let percent = (done * 100.).round() as u32;
            return Some(tr_with!(
                "activity-downloading",
                engine = engine.name(),
                percent = percent
            ));
        }
        self.installing
            .map(|kind| tr_with!("activity-installing", agent = kind.name()))
    }

    /// Starts fetching an engine, reporting progress as it goes.
    pub fn download(&mut self, engine: Engine, cx: &mut Context<Self>) {
        let Some(root) = Settings::engines_dir() else {
            cx.emit(Event::Failed(tr("error-no-data-folder").to_string()));
            return;
        };
        self.downloading = Some((engine, 0.));
        cx.notify();
        let (updates, incoming) = mpsc::channel();
        cx.background_spawn(async move {
            let progress = updates.clone();
            let installed = install_engine(engine, &root, &mut |done| {
                let _ = progress.send(Update::Progress(done));
            });
            let _ = updates.send(Update::Done(installed));
        })
        .detach();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(PROGRESS_TICK).await;
                for update in incoming.try_iter() {
                    let finished = matches!(update, Update::Done(_));
                    if this
                        .update(cx, |this, cx| this.on_update(update, cx))
                        .is_err()
                        || finished
                    {
                        return;
                    }
                }
            }
        })
        .detach();
    }

    fn on_update(&mut self, update: Update, cx: &mut Context<Self>) {
        match update {
            Update::Progress(done) => {
                if let Some((_, progress)) = &mut self.downloading {
                    *progress = done;
                }
            }
            Update::Done(Ok(paths)) => {
                self.downloading = None;
                for (key, path) in paths {
                    store(key, path.to_string_lossy().as_ref(), cx);
                }
            }
            Update::Done(Err(error)) => {
                self.downloading = None;
                cx.emit(Event::Failed(
                    tr_with!("error-download", error = error).to_string(),
                ));
            }
        }
        cx.notify();
    }

    /// Installs an agent's adapter, off the UI thread.
    pub fn install_adapter(&mut self, kind: AgentKind, cx: &mut Context<Self>) {
        let Some(folder) = Settings::adapters_dir() else {
            cx.emit(Event::Failed(tr("error-no-data-folder").to_string()));
            return;
        };
        self.installing = Some(kind);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let installed = cx
                .background_spawn(async move { adapters::install(kind, &folder) })
                .await;
            this.update(cx, |this, cx| {
                this.installing = None;
                match installed {
                    Ok(()) => cx.emit(Event::Changed),
                    Err(error) => cx.emit(Event::Failed(
                        tr_with!("error-install", error = error).to_string(),
                    )),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

fn store(key: &str, value: &str, cx: &mut Context<Installs>) {
    match Settings::store(key, value) {
        Ok(()) => cx.emit(Event::Changed),
        Err(error) => cx.emit(Event::Failed(
            tr_with!("error-save-setting", error = error).to_string(),
        )),
    }
}
