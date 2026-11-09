//! The settings page. Every control writes straight to `config.toml`, which stays the
//! single source of truth: the file can still be edited by hand.

use chessdesk_players::acp::AgentKind;
use chessdesk_players::adapters::{self, Readiness};
use chessdesk_players::download::{can_download_lc0, can_download_stockfish};
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, MouseButton, PathPromptOptions, Pixels,
    SharedString, Subscription, Window, div, prelude::*, px,
};

use crate::audio;
use crate::i18n::{self, tr, tr_with};
use crate::installs::{AppUpdate, Engine, EngineRelease, Installs};
use crate::model_picker::{self, ModelPicker, Picked};
use crate::models::Catalog;
use crate::settings::{SOUND_OFF, Settings, THEME_BOARD};
use crate::setup::{OpponentKind, title_case};
use crate::ui::{self, Logo};

const CONTENT_WIDTH: Pixels = px(560.);
/// `board_size` in the file, in twentieths: from half the room to all of it.
const BOARD_SIZES: [f64; 11] = [0.5, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85, 0.9, 0.95, 1.];
const FONT_SIZES: [f64; 7] = [10., 11., 12., 13., 14., 15., 16.];
const CORNER_RADII: [f64; 9] = [0., 1., 2., 3., 4., 5., 6., 7., 8.];
/// `opponent_delay` in the file, in seconds.
const OPPONENT_DELAYS: [f64; 8] = [0., 0.5, 1., 1.5, 2., 3., 4., 5.];
const VOLUMES: [f64; 11] = [0., 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.];
/// Every key, as the Game menu's shortcuts and the board's own: the id of what it does,
/// and the key.
const KEYS: [(&str, &str); 15] = [
    ("new-game", "Ctrl+N"),
    ("open-game", "Ctrl+O"),
    ("save-game-as", "Ctrl+S"),
    ("export-game", "Ctrl+E"),
    ("settings", "Ctrl+,"),
    ("game-history", "Ctrl+B"),
    ("game-panel", "Ctrl+Shift+B"),
    ("flip-board", "Ctrl+F"),
    ("take-back", "Ctrl+Z"),
    ("previous-move", "Left"),
    ("next-move", "Right"),
    ("first-move", "Home"),
    ("last-move", "End"),
    ("quit", "Ctrl+Q"),
    ("close-settings", "Esc"),
];

/// The pages of the settings, in the order of the list on the left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPage {
    Appearance,
    Board,
    Sound,
    Engines,
    Agents,
    Keys,
    Plugins,
    Updates,
}

impl SettingsPage {
    pub const ALL: [Self; 8] = [
        Self::Appearance,
        Self::Board,
        Self::Sound,
        Self::Engines,
        Self::Agents,
        Self::Keys,
        Self::Plugins,
        Self::Updates,
    ];

    /// How a script names the page: `agents`.
    pub fn id(self) -> &'static str {
        match self {
            Self::Appearance => "appearance",
            Self::Board => "board",
            Self::Sound => "sound",
            Self::Engines => "engines",
            Self::Agents => "agents",
            Self::Keys => "keys",
            Self::Plugins => "plugins",
            Self::Updates => "updates",
        }
    }

    fn title(self) -> SharedString {
        tr(match self {
            Self::Appearance => "page-appearance",
            Self::Board => "page-board",
            Self::Sound => "page-sound",
            Self::Engines => "page-engines",
            Self::Agents => "page-agents",
            Self::Keys => "page-keys",
            Self::Plugins => "page-plugins",
            Self::Updates => "page-updates",
        })
    }
}

pub enum Event {
    Changed,
    Close,
    Failed(String),
}

pub struct SettingsView {
    page: SettingsPage,
    open_select: Option<&'static str>,
    /// Engines and adapters being fetched, which outlive this page.
    installs: Entity<Installs>,
    /// What each agent offers, for the models it reviews and gives hints with.
    catalog: Entity<Catalog>,
    /// Those models' pickers, made as each agent's row first shows, and what keeps their picks.
    pickers: Vec<(AgentKind, Entity<ModelPicker>, Subscription)>,
    _redraw: Subscription,
}

impl EventEmitter<Event> for SettingsView {}

/// A stored name as it shows: the few that aren't a plugin's own are in the interface's
/// language, and the rest are names.
fn shown(name: &str, settings: &Settings) -> SharedString {
    match name {
        i18n::SYSTEM => tr_with!(
            "language-system",
            language = i18n::detected(&settings.languages)
        ),
        SOUND_OFF => tr("sound-off"),
        THEME_BOARD => tr("board-follows-theme"),
        "dark" => tr("theme-dark"),
        "light" => tr("theme-light"),
        name => title_case(name).into(),
    }
}

/// The stop nearest to a stored value, which may be hand-written.
fn stop_of(stops: &[f64], value: f64) -> usize {
    let distance = |stop: usize| (stops[stop] - value).abs();
    (0..stops.len())
        .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
        .unwrap_or(0)
}

impl SettingsView {
    fn write(key: &str, value: impl Into<toml_edit::Value>, cx: &mut Context<Self>) {
        match Settings::store(key, value) {
            Ok(()) => cx.emit(Event::Changed),
            Err(error) => cx.emit(Event::Failed(
                tr_with!("error-save-setting", error = error).to_string(),
            )),
        }
    }

    fn select(
        &self,
        id: &'static str,
        label: SharedString,
        current: &str,
        choices: &[String],
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = Settings::get(cx);
        let is_open = self.open_select == Some(id);
        let button = ui::select_button(id, shown(current, &settings), None, is_open, &settings)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.open_select = (!is_open).then_some(id);
                    cx.notify();
                }),
            );
        let list = is_open.then(|| {
            let rows = choices.iter().enumerate().map(|(index, name)| {
                let value = name.clone();
                ui::list_row(("choice", index), name == current, &settings)
                    .text_color(settings.colors.text)
                    .child(div().truncate().child(shown(name, &settings)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open_select = None;
                        Self::write(id, value.as_str(), cx);
                        if id == "sound_pack" {
                            audio::preview(&value, Settings::get(cx).sound_volume, cx);
                        }
                        cx.notify();
                    }))
            });
            ui::select_dropdown(
                ui::popover(window, &settings)
                    .id("list")
                    .occlude()
                    .w(ui::SELECT_WIDTH)
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        this.open_select = None;
                        cx.notify();
                    }))
                    .children(rows),
            )
        });
        ui::setting_row(label, None, &settings)
            .child(div().child(button).children(list))
            .into_any_element()
    }

    /// The language picked asks for a font that isn't here yet: what it is, and the way to
    /// fetch it, which shows in the title bar while it runs.
    fn font_notice(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let settings = Settings::get(cx);
        let font = settings.missing_font.clone()?;
        let message = tr_with!("font-needed", font = font.family.as_str());
        let size = font.bytes.map(|bytes| {
            let megabytes = format!("{:.1}", bytes as f64 / 1_000_000.);
            tr_with!("font-download-size", megabytes = megabytes)
        });
        let fetching = self
            .installs
            .read(cx)
            .font()
            .map(|(family, _)| family.clone());
        let button = |busy| ui::notice_button("download-font", tr("download"), busy, &settings);
        let action = match fetching {
            Some(family) if family == font.family => button(true),
            // Another language's font, picked before this one: one at a time.
            Some(_) => button(true).opacity(0.5),
            None => button(false).on_click(cx.listener(move |this, _, _, cx| {
                let font = font.clone();
                this.installs
                    .update(cx, |installs, cx| installs.download_font(font, cx));
            })),
        };
        Some(
            ui::notice(message, size, &settings)
                .mb_1()
                .child(action)
                .into_any_element(),
        )
    }

    /// A row with a stepped slider over numbers in the config file.
    fn number(
        key: &'static str,
        label: SharedString,
        (stops, value): (&'static [f64], f64),
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = Settings::get(cx);
        let view = cx.weak_entity();
        let index = stop_of(stops, value);
        let slider = ui::slider(
            key,
            stops.len(),
            index,
            &settings,
            window,
            cx,
            move |stop, _, cx| {
                let _ = view.update(cx, |_, cx| {
                    Self::write(key, stops[stop], cx);
                    if key == "sound_volume" {
                        let pack = Settings::get(cx).sound_pack.clone();
                        audio::preview(&pack, stops[stop] as f32, cx);
                    }
                });
            },
        );
        ui::setting_row(label, Some(text.into()), &settings)
            .child(slider)
            .into_any_element()
    }

    fn switch(
        key: &'static str,
        label: SharedString,
        hint: SharedString,
        on: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = Settings::get(cx);
        ui::setting_row(label, Some(hint), &settings)
            .child(
                ui::toggle(key, on, window, cx, &settings).on_click(cx.listener(
                    move |_, _, _, cx| {
                        Self::write(key, !on, cx);
                    },
                )),
            )
            .into_any_element()
    }

    fn appearance(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        let mut rows = vec![self.select(
            "language",
            tr("language"),
            &settings.language,
            &settings.language_names,
            window,
            cx,
        )];
        rows.extend(self.font_notice(cx));
        rows.extend([
            self.select(
                "theme",
                tr("theme"),
                &settings.theme,
                &settings.theme_names,
                window,
                cx,
            ),
            self.select(
                "piece_set",
                tr("pieces"),
                &settings.piece_set,
                &settings.piece_set_names,
                window,
                cx,
            ),
            Self::number(
                "ui_font_size",
                tr("font-size"),
                (&FONT_SIZES, f64::from(settings.ui_font_px)),
                tr_with!("pixels", value = settings.ui_font_px).to_string(),
                window,
                cx,
            ),
            Self::number(
                "corner_radius",
                tr("corner-radius"),
                (&CORNER_RADII, f64::from(settings.corner_radius_px)),
                tr_with!("pixels", value = settings.corner_radius_px).to_string(),
                window,
                cx,
            ),
        ]);
        rows
    }

    fn board(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        vec![
            self.select(
                "board",
                tr("board"),
                &settings.board_name,
                &settings.board_names,
                window,
                cx,
            ),
            Self::number(
                "board_size",
                tr("board-size"),
                (&BOARD_SIZES, f64::from(settings.board_size)),
                tr_with!(
                    "board-size-value",
                    percent = (settings.board_size * 100.).round() as u32
                )
                .to_string(),
                window,
                cx,
            ),
            Self::switch(
                "animate_moves",
                tr("animate-moves"),
                tr("animate-moves-hint"),
                settings.animate_moves,
                window,
                cx,
            ),
            Self::number(
                "opponent_delay",
                tr("opponent-delay"),
                (&OPPONENT_DELAYS, settings.opponent_delay.as_secs_f64()),
                tr_with!(
                    "opponent-delay-value",
                    seconds = settings.opponent_delay.as_secs_f64()
                )
                .to_string(),
                window,
                cx,
            ),
        ]
    }

    fn sound(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        let percent = (settings.sound_volume * 100.).round() as u32;
        vec![
            self.select(
                "sound_pack",
                tr("sound-pack"),
                &settings.sound_pack,
                &settings.sound_pack_names,
                window,
                cx,
            ),
            Self::number(
                "sound_volume",
                tr("volume"),
                (&VOLUMES, f64::from(settings.sound_volume)),
                tr_with!("volume-value", percent = percent).to_string(),
                window,
                cx,
            ),
        ]
    }

    /// Asks for a program or folder, and stores it under `key`.
    fn choose_path(key: &'static str, directories: bool, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: !directories,
            directories,
            multiple: false,
            prompt: Some(tr("choose")),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await
                && let Some(path) = paths.first()
            {
                let path = path.to_string_lossy().into_owned();
                this.update(cx, |_, cx| Self::write(key, path.as_str(), cx))
                    .ok();
            }
        })
        .detach();
    }

    /// An engine's row: whether it's ready, and how to get or point at it.
    fn engine_row(
        &self,
        engine: Engine,
        kind: OpponentKind,
        can_download: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = Settings::get(cx);
        let missing = kind.missing(&settings);
        let downloading = self.installs.read(cx).downloading();
        let release = self.installs.read(cx).engine_release(engine);
        let hint: SharedString = match (&downloading, &missing, &release) {
            (Some((fetching, done)), ..) if *fetching == engine => {
                tr_with!("downloading", percent = (done * 100.).round() as u32)
            }
            (_, Some(missing), _) => missing.clone().into(),
            (_, None, EngineRelease::Latest(version)) => {
                tr_with!("engine-latest", version = version.as_str())
            }
            (_, None, EngineRelease::Newer(version)) => {
                tr_with!("engine-newer", version = version.as_str())
            }
            (_, None, EngineRelease::Unknown) => tr("installed"),
        };
        // Updating fetches the latest again: only worth it when that's another one.
        let wanted = missing.is_some() || matches!(release, EngineRelease::Newer(_));
        let busy = downloading.is_some();
        let (id, key) = match engine {
            Engine::Stockfish => ("download-stockfish", "stockfish"),
            Engine::Maia => ("download-maia", "lc0"),
        };
        let label = tr(if missing.is_some() {
            "download"
        } else {
            "update"
        });
        let fetching = downloading.is_some_and(|(fetching, _)| fetching == engine);
        let download = if fetching {
            ui::busy_button(id, label.clone(), &settings).into_any_element()
        } else if can_download && !busy && wanted {
            ui::action_button(id, label.clone(), &settings)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.installs
                        .update(cx, |installs, cx| installs.download(engine, cx));
                }))
                .into_any_element()
        } else {
            ui::disabled_button(id, label, &settings).into_any_element()
        };
        let choose = ui::action_button(("choose", id.len()), tr("choose-program"), &settings)
            .on_click(cx.listener(move |_, _, _, cx| Self::choose_path(key, false, cx)));
        ui::setting_row(kind.name(), Some(hint), &settings)
            .child(div().flex().gap_1().child(choose).child(download))
            .into_any_element()
    }

    fn engines(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        let mut rows = vec![
            self.engine_row(
                Engine::Stockfish,
                OpponentKind::Stockfish,
                can_download_stockfish(),
                cx,
            ),
            self.engine_row(Engine::Maia, OpponentKind::Maia, can_download_lc0(), cx),
        ];
        let weights = settings.maia_weights.display().to_string();
        let hint: SharedString = if weights.is_empty() {
            tr("not-set")
        } else {
            weights.into()
        };
        rows.push(
            ui::setting_row(tr("maia-networks"), Some(hint), &settings)
                .child(
                    ui::action_button("choose-weights", tr("choose-folder"), &settings).on_click(
                        cx.listener(|_, _, _, cx| Self::choose_path("maia_weights", true, cx)),
                    ),
                )
                .into_any_element(),
        );
        rows
    }

    /// The picker of the model `kind` reviews and gives hints with, made the first time.
    fn picker(&mut self, kind: AgentKind, cx: &mut Context<Self>) -> Entity<ModelPicker> {
        if let Some((_, picker, _)) = self.pickers.iter().find(|(known, ..)| *known == kind) {
            return picker.clone();
        }
        let chosen = Settings::get(cx).analysis_model(kind);
        let catalog = self.catalog.clone();
        let picker = cx.new(|cx| ModelPicker::new(kind, chosen, catalog, cx));
        let store = cx.subscribe(&picker, move |_, _, Picked(model): &Picked, cx| {
            match Settings::store_analysis_model(kind, model.as_deref()) {
                Ok(()) => cx.emit(Event::Changed),
                Err(error) => cx.emit(Event::Failed(
                    tr_with!("error-save-setting", error = error).to_string(),
                )),
            }
        });
        self.pickers.push((kind, picker.clone(), store));
        picker
    }

    /// The model each agent that's set up reviews games and gives hints with.
    fn analysis_models(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let folder = Settings::adapters_dir().unwrap_or_default();
        let ready: Vec<_> = AgentKind::ALL
            .into_iter()
            .filter(|kind| adapters::readiness(*kind, &folder) == Readiness::Ready)
            .collect();
        if ready.is_empty() {
            return Vec::new();
        }
        let settings = Settings::get(cx);
        let mut rows = vec![
            div()
                .flex()
                .flex_col()
                .pt_4()
                .pb_1()
                .child(
                    div()
                        .text_color(settings.colors.text)
                        .child(tr("analysis-models")),
                )
                .child(
                    div()
                        .text_color(settings.colors.muted)
                        .child(tr("analysis-models-hint")),
                )
                .into_any_element(),
        ];
        for kind in ready {
            let picker = self.picker(kind, cx);
            rows.push(model_picker::row(kind, &picker, cx));
        }
        rows
    }

    /// Whether each agent can play, and the way there if it can't yet, then the model each one
    /// that can reviews and gives hints with.
    fn agents(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        let folder = Settings::adapters_dir().unwrap_or_default();
        let busy = self.installs.read(cx).installing();
        let mut rows: Vec<_> = AgentKind::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| {
                let state = adapters::readiness(kind, &folder);
                let installing = busy == Some(kind);
                let hint: SharedString = match state {
                    _ if installing => tr("installing-adapter"),
                    Readiness::Ready => tr("installed"),
                    Readiness::NoAdapter => tr("adapter-missing"),
                    Readiness::NoNode => tr("needs-node"),
                    Readiness::NoProgram(program) => {
                        tr_with!("program-not-found", program = program)
                    }
                };
                let can_install = state == Readiness::NoAdapter;
                let button = can_install.then(|| {
                    if installing {
                        ui::busy_button(("install", index), tr("install"), &settings)
                            .into_any_element()
                    } else if busy.is_some() {
                        ui::disabled_button(("install", index), tr("install"), &settings)
                            .into_any_element()
                    } else {
                        ui::action_button(("install", index), tr("install"), &settings)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.installs.update(cx, |installs, cx| {
                                    installs.install_adapter(kind, cx);
                                });
                            }))
                            .into_any_element()
                    }
                });
                ui::logo_setting_row(Logo::of_agent(kind), kind.name(), Some(hint), &settings)
                    .children(button)
                    .into_any_element()
            })
            .collect();
        rows.extend(self.analysis_models(cx));
        rows
    }

    fn keys(cx: &App) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        KEYS.into_iter()
            .map(|(action, key)| {
                ui::setting_row(tr(action), None, &settings)
                    .child(div().text_color(settings.colors.muted).child(key))
                    .into_any_element()
            })
            .collect()
    }

    fn plugins(cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        let folder = ui::setting_row(
            tr("plugins-folder"),
            Some(tr("plugins-folder-hint")),
            &settings,
        )
        .child(
            ui::action_button("open-plugins", tr("open"), &settings).on_click(cx.listener(
                |_, _, _, cx| {
                    if let Some(folder) = Settings::plugins_dir() {
                        cx.open_with_system(&folder);
                    }
                },
            )),
        );
        let heading = div()
            .pt_4()
            .text_color(settings.colors.muted)
            .child(tr("installed-plugins"));
        // Each with what its plugin says about itself.
        let row = |name: &str, description: Option<&String>| {
            let hint = description.map(|text| SharedString::from(text.clone()));
            ui::setting_row(name.to_owned(), hint, &settings).into_any_element()
        };
        let mut rows = vec![folder.into_any_element(), heading.into_any_element()];
        rows.extend(
            settings
                .piece_sets
                .iter()
                .map(|set| row(&set.name, set.description.as_ref())),
        );
        rows.extend(
            settings
                .themes
                .iter()
                .map(|theme| row(&theme.name, theme.description.as_ref())),
        );
        rows.extend(
            settings
                .board_themes
                .iter()
                .map(|board| row(&board.name, board.description.as_ref())),
        );
        rows.extend(
            settings
                .sound_packs
                .iter()
                .map(|pack| row(&pack.name, pack.description.as_ref())),
        );
        rows.extend(
            settings
                .languages
                .iter()
                .map(|language| row(&language.name, language.description.as_ref())),
        );
        rows
    }

    /// The version running and the latest, and the one button that takes each step between
    /// them, which the update button in the title bar presses too.
    fn updates(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        let update = self.installs.read(cx).app_update().clone();
        let current = env!("CARGO_PKG_VERSION");
        let version = |release: &chessdesk_players::update::Release| release.version.to_string();
        let hint = match &update {
            AppUpdate::Unknown => tr_with!("about-version", version = current),
            AppUpdate::Checking => tr("checking-for-updates"),
            AppUpdate::UpToDate => tr_with!("up-to-date", version = current),
            AppUpdate::Available(release) => tr_with!(
                "update-available",
                version = version(release),
                current = current
            ),
            AppUpdate::Downloading(_, done) => {
                tr_with!("downloading", percent = (done * 100.).round() as u32)
            }
            AppUpdate::Ready(release, _) => tr_with!("update-ready", version = version(release)),
        };
        let press = cx.listener(|this, _, _, cx| {
            this.installs.update(cx, Installs::update_app);
        });
        let button = match &update {
            AppUpdate::Unknown | AppUpdate::UpToDate => {
                ui::action_button("update-app", tr("check-now"), &settings)
                    .on_click(press)
                    .into_any_element()
            }
            AppUpdate::Checking => {
                ui::busy_button("update-app", tr("check-now"), &settings).into_any_element()
            }
            AppUpdate::Available(release) => ui::primary_button(
                "update-app",
                tr_with!("update-to", version = version(release)),
                &settings,
            )
            .on_click(press)
            .into_any_element(),
            AppUpdate::Downloading(release, _) => ui::busy_button(
                "update-app",
                tr_with!("update-to", version = version(release)),
                &settings,
            )
            .into_any_element(),
            AppUpdate::Ready(..) => {
                ui::primary_button("update-app", tr("restart-to-update"), &settings)
                    .on_click(press)
                    .into_any_element()
            }
        };
        let page = match &update {
            AppUpdate::Available(release)
            | AppUpdate::Downloading(release, _)
            | AppUpdate::Ready(release, _) => Some(release.page.clone()),
            _ => None,
        };
        let notes = page.map(|page| {
            ui::action_button("release-notes", tr("release-notes"), &settings)
                .on_click(move |_, _, cx| cx.open_url(&page))
        });
        vec![
            ui::setting_row("Chessdesk", Some(hint), &settings)
                .child(div().flex().gap_1().children(notes).child(button))
                .into_any_element(),
        ]
    }

    /// What a page needs to know when it opens: the engines' latest releases.
    fn on_page_shown(&self, cx: &mut Context<Self>) {
        if self.page == SettingsPage::Engines {
            self.installs.update(cx, Installs::check_engines);
        }
    }

    /// The pages, for the sidebar's place.
    pub fn render_nav(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let settings = Settings::get(cx);
        let rows = SettingsPage::ALL
            .into_iter()
            .enumerate()
            .map(|(index, page)| {
                ui::list_row(("page", index), page == self.page, &settings)
                    .text_color(settings.colors.text)
                    .child(page.title())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.page = page;
                        this.on_page_shown(cx);
                        cx.notify();
                    }))
            });
        div()
            .flex()
            .flex_col()
            .size_full()
            .p_1()
            .gap_0p5()
            .children(rows)
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_1()
                    .child(
                        ui::button("open-file", false, &settings)
                            .h(ui::CONTROL_HEIGHT)
                            .on_click(cx.listener(|_, _, _, cx| match Settings::config_file() {
                                Ok(path) => cx.open_with_system(&path),
                                Err(error) => cx.emit(Event::Failed(error)),
                            }))
                            .child(tr("open-toml")),
                    )
                    .child(
                        ui::button("close", true, &settings)
                            .h(ui::CONTROL_HEIGHT)
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(Event::Close)))
                            .child(tr("close")),
                    ),
            )
    }

    /// What makes Chessdesk look and sound like one's own, for the welcome on first start:
    /// rows like the pages' own, saved the same way.
    pub fn render_welcome(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        let mut rows = vec![self.select(
            "language",
            tr("language"),
            &settings.language,
            &settings.language_names,
            window,
            cx,
        )];
        rows.extend(self.font_notice(cx));
        rows.extend([
            self.select(
                "theme",
                tr("theme"),
                &settings.theme,
                &settings.theme_names,
                window,
                cx,
            ),
            self.select(
                "board",
                tr("board"),
                &settings.board_name,
                &settings.board_names,
                window,
                cx,
            ),
            self.select(
                "piece_set",
                tr("pieces"),
                &settings.piece_set,
                &settings.piece_set_names,
                window,
                cx,
            ),
            self.select(
                "sound_pack",
                tr("sound-pack"),
                &settings.sound_pack,
                &settings.sound_pack_names,
                window,
                cx,
            ),
        ]);
        rows
    }

    /// The current page, for the board's place.
    pub fn render_content(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let settings = Settings::get(cx);
        let rows = match self.page {
            SettingsPage::Appearance => self.appearance(window, cx),
            SettingsPage::Board => self.board(window, cx),
            SettingsPage::Sound => self.sound(window, cx),
            SettingsPage::Engines => self.engines(cx),
            SettingsPage::Agents => self.agents(cx),
            SettingsPage::Keys => Self::keys(cx),
            SettingsPage::Plugins => Self::plugins(cx),
            SettingsPage::Updates => self.updates(cx),
        };
        div()
            .id("settings-content")
            .flex_1()
            .min_w_0()
            .h_full()
            .px_6()
            .py_4()
            .overflow_y_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .max_w(CONTENT_WIDTH)
                    .gap_1()
                    .child(ui::title(self.page.title(), &settings))
                    .children(rows),
            )
    }
}

/// Opens the settings as a view, on `page`.
pub fn page(
    page: SettingsPage,
    installs: Entity<Installs>,
    catalog: Entity<Catalog>,
    cx: &mut App,
) -> Entity<SettingsView> {
    cx.new(|cx| {
        let redraw = cx.observe(&installs, |_, _, cx| cx.notify());
        let view = SettingsView {
            page,
            open_select: None,
            installs,
            catalog,
            pickers: Vec::new(),
            _redraw: redraw,
        };
        view.on_page_shown(cx);
        view
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hand_written_value_finds_the_nearest_stop() {
        assert_eq!(stop_of(&BOARD_SIZES, 0.8), 6);
        assert_eq!(stop_of(&BOARD_SIZES, 0.82), 6);
        assert_eq!(stop_of(&BOARD_SIZES, 2.), BOARD_SIZES.len() - 1);
    }
}
