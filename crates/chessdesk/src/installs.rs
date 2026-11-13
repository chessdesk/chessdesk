//! Engines, agent adapters, fonts and Chessdesk's own updates being fetched. The window owns
//! this rather than the settings page that starts the work, so the work carries on, and shows,
//! after the page closes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use chessdesk_core::plugins::Font;
use chessdesk_players::acp::AgentKind;
use chessdesk_players::update::{self, Release};
use chessdesk_players::{adapters, download};
use gpui::{App, AppContext, Context, Div, Entity, EventEmitter, prelude::*};

use crate::fonts;
use crate::i18n::{tr, tr_with};
use crate::settings::Settings;
use crate::setup::OpponentKind;
use crate::ui::{self, Activity};

/// How often a download's progress is looked at.
const PROGRESS_TICK: Duration = Duration::from_millis(200);

/// An engine that can be fetched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Engine {
    Stockfish,
    Shadowchess,
    Maia,
}

impl Engine {
    const ALL: [Self; 3] = [Self::Stockfish, Self::Shadowchess, Self::Maia];

    fn name(self) -> &'static str {
        match self {
            Self::Stockfish => "Stockfish",
            Self::Shadowchess => "Shadowchess",
            Self::Maia => "Maia",
        }
    }

    /// The folder whose release counts: for Maia, `lc0`'s. Its networks never change.
    fn program(self) -> &'static str {
        match self {
            Self::Stockfish => "stockfish",
            Self::Shadowchess => "shadowchess",
            Self::Maia => "lc0",
        }
    }

    fn latest(self) -> Result<String, String> {
        match self {
            Self::Stockfish => download::latest_stockfish(),
            Self::Shadowchess => download::latest_shadowchess(),
            Self::Maia => download::latest_lc0(),
        }
    }

    /// The engine an opponent plays with, if it's one.
    pub fn of(opponent: OpponentKind) -> Option<Self> {
        match opponent {
            OpponentKind::Stockfish => Some(Self::Stockfish),
            OpponentKind::Shadowchess => Some(Self::Shadowchess),
            OpponentKind::Maia => Some(Self::Maia),
            _ => None,
        }
    }
}

/// How an installed engine stands against its latest release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineRelease {
    /// Not asked yet, or asking failed.
    Unknown,
    Latest(String),
    /// A newer one is out, or which one is here isn't known: installed by hand, or by a
    /// Chessdesk before 2.3.0.
    Newer(String),
}

/// Where `installed` stands against `latest`.
fn compare(installed: Option<&str>, latest: Option<&str>) -> EngineRelease {
    match (installed, latest) {
        (_, None) => EngineRelease::Unknown,
        (Some(installed), Some(latest)) if installed == latest => {
            EngineRelease::Latest(latest.to_owned())
        }
        (_, Some(latest)) => EngineRelease::Newer(latest.to_owned()),
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

/// Where Chessdesk stands against its latest release.
#[derive(Clone, Debug, Default)]
pub enum AppUpdate {
    /// Not asked yet, or asking failed.
    #[default]
    Unknown,
    Checking,
    UpToDate,
    Available(Release),
    Downloading(Release, Progress),
    /// In the old one's place, to run from the next start: the program that starts it.
    Ready(Release, PathBuf),
}

#[derive(Default)]
pub struct Installs {
    downloading: Option<(Engine, Progress)>,
    /// The family of the font being fetched.
    font: Option<(String, Progress)>,
    /// The agent whose adapter is being installed.
    installing: Option<AgentKind>,
    /// Each engine's latest release, once asked for: once a run is enough.
    latest_engines: HashMap<Engine, String>,
    engines_checked: bool,
    app_update: AppUpdate,
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
        Engine::Shadowchess => {
            let weights = download::install_shadowchess(root, progress)?;
            Ok(vec![("shadowchess_weights", weights)])
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

    /// Where the installed `engine` stands against its latest release.
    pub fn engine_release(&self, engine: Engine) -> EngineRelease {
        let installed = Settings::engines_dir().and_then(|root| {
            download::installed_release(&download::engine_folder(&root, engine.program()))
        });
        compare(
            installed.as_deref(),
            self.latest_engines.get(&engine).map(String::as_str),
        )
    }

    /// Asks GitHub for each engine's latest release, once a run, quietly: without an answer
    /// the update buttons just stay off.
    pub fn check_engines(&mut self, cx: &mut Context<Self>) {
        if std::mem::replace(&mut self.engines_checked, true) {
            return;
        }
        cx.spawn(async move |this, cx| {
            let found = cx
                .background_spawn(async {
                    Engine::ALL
                        .into_iter()
                        .filter_map(|engine| Some((engine, engine.latest().ok()?)))
                        .collect::<Vec<_>>()
                })
                .await;
            this.update(cx, |this, cx| {
                this.latest_engines.extend(found);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn app_update(&self) -> &AppUpdate {
        &self.app_update
    }

    /// What's under way, for the title bar.
    pub fn activity(&self) -> Option<Activity> {
        let downloading = |name: &str, done: Progress| {
            let percent = (done * 100.).round() as u32;
            Activity::new(tr_with!(
                "activity-downloading",
                name = name,
                percent = percent
            ))
            .with_progress(done)
        };
        if let Some((engine, done)) = self.downloading {
            return Some(downloading(engine.name(), done));
        }
        if let AppUpdate::Downloading(release, done) = &self.app_update {
            return Some(downloading(
                &format!("Chessdesk {}", release.version),
                *done,
            ));
        }
        if let Some((family, done)) = &self.font {
            return Some(downloading(family, *done));
        }
        self.installing
            .map(|kind| Activity::new(tr_with!("activity-installing", agent = kind.name())))
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

    /// Starts fetching what `opponent` needs to play: its engine or its agent's adapter.
    /// Nothing happens while one of the same kind is under way: one at a time.
    pub fn fetch_for(&mut self, opponent: OpponentKind, cx: &mut Context<Self>) {
        let Some(engine) = Engine::of(opponent) else {
            if let Some(kind) = opponent.agent()
                && self.installing.is_none()
            {
                self.install_adapter(kind, cx);
            }
            return;
        };
        if self.downloading.is_none() {
            self.download(engine, cx);
        }
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

    /// Asks GitHub for a newer Chessdesk. `quiet` keeps a failure to itself, as at launch
    /// without a connection.
    pub fn check_for_update(&mut self, quiet: bool, cx: &mut Context<Self>) {
        self.app_update = AppUpdate::Checking;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let found = cx
                .background_spawn(async { update::check(env!("CARGO_PKG_VERSION")) })
                .await;
            this.update(cx, |this, cx| {
                this.app_update = match found {
                    Ok(Some(release)) => AppUpdate::Available(release),
                    Ok(None) => AppUpdate::UpToDate,
                    Err(error) => {
                        if !quiet {
                            cx.emit(Event::Failed(
                                tr_with!("error-update-check", error = error).to_string(),
                            ));
                        }
                        AppUpdate::Unknown
                    }
                };
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// What the update button does at each step: fetch the newer version once one is known,
    /// start it once it's in place, and ask for one otherwise.
    pub fn update_app(&mut self, cx: &mut Context<Self>) {
        match self.app_update.clone() {
            AppUpdate::Unknown | AppUpdate::UpToDate => self.check_for_update(false, cx),
            AppUpdate::Available(release) => self.download_update(release, cx),
            AppUpdate::Ready(_, program) => restart(&program, cx),
            AppUpdate::Checking | AppUpdate::Downloading(..) => {}
        }
    }

    fn download_update(&mut self, release: Release, cx: &mut Context<Self>) {
        self.app_update = AppUpdate::Downloading(release.clone(), 0.);
        cx.notify();
        let fetched = release.clone();
        Self::fetch(
            cx,
            move |progress| update::install(&fetched, progress),
            |this, done| {
                if let AppUpdate::Downloading(_, progress) = &mut this.app_update {
                    *progress = done;
                }
            },
            move |this, program, _| match program {
                Ok(program) => {
                    this.app_update = AppUpdate::Ready(release, program);
                    Ok(())
                }
                Err(error) => {
                    this.app_update = AppUpdate::Available(release);
                    Err(error)
                }
            },
        );
    }
}

/// Starts the new version and quits this one.
fn restart(program: &Path, cx: &mut Context<Installs>) {
    match Command::new(program).spawn() {
        Ok(_) => cx.quit(),
        Err(error) => cx.emit(Event::Failed(
            tr_with!("error-restart", error = error.to_string()).to_string(),
        )),
    }
}

/// What needs Stockfish when it isn't there: win likelihood, or tutoring against an engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeedsStockfish {
    Likelihood,
    Tutoring,
    Both,
}

impl NeedsStockfish {
    /// What a game set up to show likelihood and to have moves graded needs, if anything.
    pub fn of(likelihood: bool, tutoring: bool) -> Option<Self> {
        match (likelihood, tutoring) {
            (true, true) => Some(Self::Both),
            (true, false) => Some(Self::Likelihood),
            (false, true) => Some(Self::Tutoring),
            (false, false) => None,
        }
    }

    fn feature(self) -> &'static str {
        match self {
            Self::Likelihood => "likelihood",
            Self::Tutoring => "tutoring",
            Self::Both => "both",
        }
    }
}

/// A card saying Stockfish is missing for `need`, with the button that fetches it right
/// there; the download shows in the title bar.
pub fn stockfish_notice(need: NeedsStockfish, installs: &Entity<Installs>, cx: &App) -> Div {
    let settings = Settings::get(cx);
    let message = tr_with!("stockfish-needed", feature = need.feature());
    let card = ui::notice(message, Some(tr("stockfish-download-size")), &settings);
    if !download::can_download_stockfish() {
        return card;
    }
    let button = |busy| ui::notice_button("download-stockfish", tr("download"), busy, &settings);
    let action = match installs.read(cx).downloading() {
        Some((Engine::Stockfish, _)) => button(true),
        // Another engine, fetched first: one download at a time.
        Some(_) => button(true).opacity(0.5),
        None => {
            let installs = installs.clone();
            button(false).on_click(move |_, _, cx| {
                installs.update(cx, |installs, cx| installs.download(Engine::Stockfish, cx));
            })
        }
    };
    card.child(action)
}

fn store(key: &str, value: &str, cx: &mut Context<Installs>) {
    match Settings::store(key, value) {
        Ok(()) => cx.emit(Event::Changed),
        Err(error) => cx.emit(Event::Failed(
            tr_with!("error-save-setting", error = error).to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_update_is_offered_only_for_a_newer_or_unknown_release() {
        assert_eq!(
            compare(Some("sf_19"), Some("sf_19")),
            EngineRelease::Latest("sf_19".to_owned())
        );
        assert_eq!(
            compare(Some("sf_18"), Some("sf_19")),
            EngineRelease::Newer("sf_19".to_owned())
        );
        assert_eq!(
            compare(None, Some("sf_19")),
            EngineRelease::Newer("sf_19".to_owned())
        );
        assert_eq!(compare(Some("sf_19"), None), EngineRelease::Unknown);
    }
}
