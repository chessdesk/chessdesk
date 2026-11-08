//! Engines, agent adapters and fonts being fetched. The window owns this rather than the
//! settings page that starts the work, so the work carries on, and shows, after the page
//! closes.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use chessdesk_core::plugins::Font;
use chessdesk_players::acp::AgentKind;
use chessdesk_players::{adapters, download};
use gpui::{AppContext, Context, EventEmitter, SharedString};

use crate::fonts;
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
enum Update<T> {
    Progress(f32),
    Done(Result<T, String>),
}

/// How far along a download is, from 0 to 1.
type Progress = f32;

#[derive(Default)]
pub struct Installs {
    downloading: Option<(Engine, Progress)>,
    /// The family of the font being fetched.
    font: Option<(String, Progress)>,
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
    pub fn downloading(&self) -> Option<(Engine, Progress)> {
        self.downloading
    }

    /// The font being fetched, by family.
    pub fn font(&self) -> Option<&(String, Progress)> {
        self.font.as_ref()
    }

    pub fn installing(&self) -> Option<AgentKind> {
        self.installing
    }

    /// What's under way, for the title bar.
    pub fn activity(&self) -> Option<SharedString> {
        let percent = |done: Progress| (done * 100.).round() as u32;
        if let Some((engine, done)) = self.downloading {
            return Some(tr_with!(
                "activity-downloading",
                name = engine.name(),
                percent = percent(done)
            ));
        }
        if let Some((family, done)) = &self.font {
            return Some(tr_with!(
                "activity-downloading",
                name = family.as_str(),
                percent = percent(*done)
            ));
        }
        self.installing
            .map(|kind| tr_with!("activity-installing", agent = kind.name()))
    }

    /// Starts fetching an engine; the settings point at it once it's here.
    pub fn download(&mut self, engine: Engine, cx: &mut Context<Self>) {
        let Some(root) = Settings::engines_dir() else {
            cx.emit(Event::Failed(tr("error-no-data-folder").to_string()));
            return;
        };
        self.downloading = Some((engine, 0.));
        cx.notify();
        Self::fetch(
            cx,
            move |progress| install_engine(engine, &root, progress),
            |this, done| {
                if let Some((_, progress)) = &mut this.downloading {
                    *progress = done;
                }
            },
            |this, installed, cx| {
                this.downloading = None;
                for (key, path) in installed? {
                    store(key, path.to_string_lossy().as_ref(), cx);
                }
                Ok(())
            },
        );
    }

    /// Starts fetching a language's font; it's the interface font once it's here.
    pub fn download_font(&mut self, font: Font, cx: &mut Context<Self>) {
        let Some(target) = Settings::fonts_dir().map(|folder| folder.join(font.file_name())) else {
            cx.emit(Event::Failed(tr("error-no-data-folder").to_string()));
            return;
        };
        self.font = Some((font.family.clone(), 0.));
        cx.notify();
        let fetched = target.clone();
        Self::fetch(
            cx,
            move |progress| download::install_font(&font.url, &fetched, progress),
            |this, done| {
                if let Some((_, progress)) = &mut this.font {
                    *progress = done;
                }
            },
            move |this, fetched, cx| {
                this.font = None;
                fetched?;
                match fonts::load(&target, cx) {
                    Ok(()) => cx.emit(Event::Changed),
                    Err(error) => cx.emit(Event::Failed(error)),
                }
                Ok(())
            },
        );
    }

    /// Runs `work` off the UI thread, passing on its progress and then its result to `done`,
    /// whose error is reported as a failed download.
    fn fetch<T: Send + 'static>(
        cx: &mut Context<Self>,
        work: impl FnOnce(&mut dyn FnMut(f32)) -> Result<T, String> + Send + 'static,
        progress: impl Fn(&mut Self, Progress) + 'static,
        done: impl FnOnce(&mut Self, Result<T, String>, &mut Context<Self>) -> Result<(), String>
        + 'static,
    ) {
        let (updates, incoming) = mpsc::channel();
        cx.background_spawn(async move {
            let sender = updates.clone();
            let result = work(&mut |done| {
                let _ = sender.send(Update::Progress(done));
            });
            let _ = updates.send(Update::Done(result));
        })
        .detach();
        cx.spawn(async move |this, cx| {
            let mut done = Some(done);
            loop {
                cx.background_executor().timer(PROGRESS_TICK).await;
                for update in incoming.try_iter() {
                    let finished = this.update(cx, |this, cx| {
                        match update {
                            Update::Progress(share) => {
                                progress(this, share);
                                cx.notify();
                                return false;
                            }
                            Update::Done(result) => {
                                if let Some(Err(error)) =
                                    done.take().map(|done| done(this, result, cx))
                                {
                                    cx.emit(Event::Failed(
                                        tr_with!("error-download", error = error).to_string(),
                                    ));
                                }
                            }
                        }
                        cx.notify();
                        true
                    });
                    if finished.unwrap_or(true) {
                        return;
                    }
                }
            }
        })
        .detach();
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
