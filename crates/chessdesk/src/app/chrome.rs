//! What sits around the board: the menu row with the logo, and what comes and goes.

use std::rc::Rc;
use std::time::{Duration, Instant};

use chessdesk_core::review;
use chessdesk_players::acp::AgentKind;
use chessdesk_players::adapters::{self, Readiness};
use gpui::{
    Action, AnyElement, App, ClipboardItem, Context, Div, MouseButton, MouseDownEvent,
    MouseMoveEvent, Pixels, SharedString, Window, WindowControlArea, anchored, deferred, div, img,
    point, prelude::*, px,
};
use shakmaty::{Piece, Position, Role};

use super::session::Session;
use super::{
    Analyze, Chessdesk, ExportGame, FlipBoard, NewGameAction, OpenGame, OpenSettings, Quit, Resign,
    SaveGame, TakeBack, ToggleHistory, ToggleLikelihood, TogglePanel,
};
use crate::i18n::{tr, tr_with};
use crate::installs::{AppUpdate, Installs};
use crate::settings::Settings;
use crate::settings_view::SettingsPage;
use crate::trace;
use crate::ui::Activity;
use crate::ui::{self, Icon};

/// From the top of a menu label to its dropdown.
const DROPDOWN_OFFSET: Pixels = px(26.);
/// From the top of the logo's button, the height of the ribbon, to its dropdown.
const BRAND_DROPDOWN_OFFSET: Pixels = px(29.);
const LOGO_HEIGHT: Pixels = px(11.);
/// The room the macOS window buttons take at the left of the bar, placed by `main`.
const TRAFFIC_LIGHTS_WIDTH: Pixels = px(72.);
const PROMOTION_SIZE: Pixels = px(64.);
/// How long the trace's button says it was copied.
const COPIED_FOR: Duration = Duration::from_secs(2);
/// The widest the loading bar gets, in the room between the menus and the window controls.
const LOADING_BAR_WIDTH: Pixels = px(360.);
/// The menus, as the ids of their titles.
const MENUS: [&str; 3] = ["menu-game", "menu-review", "menu-view"];
/// The menu behind the logo, after the others among the open menus.
const BRAND_MENU: usize = MENUS.len();
const ABOUT_WIDTH: Pixels = px(360.);
const ABOUT_LOGOTYPE_HEIGHT: Pixels = px(22.);
const REPOSITORY: &str = "https://github.com/chessdesk/chessdesk";
/// How many of an update's notes show beneath its button, and how wide they run.
const NOTES_SHOWN: usize = 4;
const NOTES_WIDTH: Pixels = px(440.);
const COPYRIGHT: &str = "© 2026 Team Chessdesk";

type Run = Rc<dyn Fn(&mut Chessdesk, &mut Window, &mut Context<Chessdesk>)>;

/// A row of an open menu.
struct Entry {
    title: SharedString,
    key: Option<&'static str>,
    /// `None` for what isn't there yet: it shows, greyed out.
    run: Option<Run>,
}

fn action(id: &str, key: Option<&'static str>, action: &dyn Action) -> Entry {
    let action = action.boxed_clone();
    Entry {
        title: tr(id),
        key,
        run: Some(Rc::new(move |_, window, cx| {
            window.dispatch_action(action.boxed_clone(), cx);
        })),
    }
}

fn entry(
    title: impl Into<SharedString>,
    run: impl Fn(&mut Chessdesk, &mut Window, &mut Context<Chessdesk>) + 'static,
) -> Entry {
    Entry {
        title: title.into(),
        key: None,
        run: Some(Rc::new(run)),
    }
}

fn unavailable(title: impl Into<SharedString>) -> Entry {
    Entry {
        title: title.into(),
        key: None,
        run: None,
    }
}

/// What the update button says, if there's an update to show it for: Chessdesk's, which brings
/// the outdated engines along, or else the engines'.
fn update_label(installs: &Installs) -> Option<SharedString> {
    match installs.app_update() {
        AppUpdate::Available(release) => {
            Some(tr_with!("update-to", version = release.version.to_string()))
        }
        // Not while engines are still coming in: a restart would cut them off.
        AppUpdate::Ready(..) => (!installs.fetching_engines()).then(|| tr("restart-to-update")),
        AppUpdate::Downloading(..) => None,
        AppUpdate::Unknown | AppUpdate::Checking | AppUpdate::UpToDate => {
            match installs.outdated_engines().as_slice() {
                [] => None,
                [engine] => Some(tr_with!("update-engine", name = engine.name())),
                _ => Some(tr("update-engines")),
            }
        }
    }
}

/// Entries of a menu; `None` is a separator. `update` is the update button's label, if it shows.
fn menu_entries(
    index: usize,
    session: &Session,
    update: Option<SharedString>,
) -> Vec<Option<Entry>> {
    match index {
        0 => {
            // A competition is played as it stands.
            let assisted = session
                .live
                .as_ref()
                .is_none_or(|live| live.setup.assisted());
            vec![
                Some(action("new-game", Some("Ctrl+N"), &NewGameAction)),
                Some(action("open-game", Some("Ctrl+O"), &OpenGame)),
                Some(action("save-game-as", Some("Ctrl+S"), &SaveGame)),
                Some(action("export-game", Some("Ctrl+E"), &ExportGame)),
                None,
                assisted.then(|| action("take-back", Some("Ctrl+Z"), &TakeBack)),
                Some(action("resign", None, &Resign)),
            ]
        }
        1 => {
            // Stockfish goes over a game once, by itself when it ends.
            let mut entries = Vec::new();
            if !review::is_analysed(&session.game) {
                entries.extend([Some(action("analyze-with-stockfish", None, &Analyze)), None]);
            }
            let folder = Settings::adapters_dir().unwrap_or_default();
            entries.extend(AgentKind::ALL.map(|kind| {
                let title = tr_with!("review-with", agent = kind.name());
                // Greyed out until its adapter is set up, which the last entry leads to.
                Some(if adapters::readiness(kind, &folder) == Readiness::Ready {
                    entry(title, move |this, _, cx| this.review_with(kind, cx))
                } else {
                    unavailable(title)
                })
            }));
            entries.push(None);
            entries.push(Some(entry(tr("agent-settings"), |this, _, cx| {
                this.show_settings(SettingsPage::Agents, cx);
            })));
            entries
        }
        BRAND_MENU => vec![
            Some(entry(tr("about"), |this, _, cx| {
                this.about_open = true;
                cx.notify();
            })),
            Some(entry(tr("help"), |_, _, cx| {
                cx.open_url(&format!("{REPOSITORY}/wiki"));
            })),
            Some(entry(tr("github-repository"), |_, _, cx| {
                cx.open_url(REPOSITORY);
            })),
            Some(entry(
                update.unwrap_or_else(|| tr("check-for-updates")),
                |this, _, cx| this.update_app(cx),
            )),
            None,
            Some(action("settings", Some("Ctrl+,"), &OpenSettings)),
            None,
            Some(action("quit", Some("Ctrl+Q"), &Quit)),
        ],
        _ => {
            let mut entries = vec![
                Some(action("flip-board", Some("Ctrl+F"), &FlipBoard)),
                None,
                Some(action("game-history", Some("Ctrl+B"), &ToggleHistory)),
                Some(action("game-panel", Some("Ctrl+Shift+B"), &TogglePanel)),
            ];
            if session
                .live
                .as_ref()
                .is_some_and(|live| live.setup.assisted())
            {
                let title = if session.shows_likelihood() {
                    "hide-win-likelihood"
                } else {
                    "show-win-likelihood"
                };
                entries.push(Some(action(title, None, &ToggleLikelihood)));
            }
            entries
        }
    }
}

fn window_controls(settings: &Settings, maximized: bool) -> Div {
    let accent = settings.colors.accent;
    let muted = settings.colors.muted;
    let control = |id: &'static str,
                   label: SharedString,
                   area: WindowControlArea,
                   icon: Icon,
                   size: Pixels| {
        div()
            .id(id)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .h_full()
            .w(ui::WINDOW_CONTROL_WIDTH)
            .child(ui::hover_fill(accent, px(0.)))
            .tooltip(ui::tooltip(label))
            .window_control_area(area)
            .child(ui::icon(icon, size, muted))
    };
    let maximize = if maximized {
        Icon::WindowRestore
    } else {
        Icon::WindowMaximize
    };
    div()
        .flex()
        .flex_none()
        .h_full()
        .child(control(
            "minimize",
            tr("minimize"),
            WindowControlArea::Min,
            Icon::WindowMinimize,
            px(15.),
        ))
        .child(control(
            "maximize",
            tr(if maximized { "restore" } else { "maximize" }),
            WindowControlArea::Max,
            maximize,
            px(12.),
        ))
        .child(control(
            "close",
            tr("close"),
            WindowControlArea::Close,
            Icon::WindowClose,
            px(15.),
        ))
}

impl Chessdesk {
    /// The row across the top of the window: the logo, the menu and the window controls.
    pub(super) fn render_top_bar(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let line = div()
            .flex_none()
            .w(ui::hairline(window))
            .h_full()
            .bg(settings.colors.popover_border);
        let mut menus = div().flex().flex_none().items_center().px_1();
        for (index, title) in MENUS.into_iter().enumerate() {
            menus = menus.child(self.render_menu(index, title, window, cx));
        }
        let mut bar = div()
            .flex()
            .flex_none()
            .items_center()
            .h(ui::TITLE_BAR_HEIGHT)
            .text_color(settings.colors.muted)
            // Full screen hides the window buttons, and the room they take with them.
            .when(
                cfg!(target_os = "macos") && !window.is_fullscreen(),
                |bar| bar.pl(TRAFFIC_LIGHTS_WIDTH),
            )
            .child(self.render_brand(window, cx))
            .child(line)
            .child(menus);

        // The only draggable part: anything over it would swallow clicks on Windows. The
        // loading bar inside takes no clicks, so the window drags by it too.
        let loading = self.activity(cx).map(|activity| {
            div()
                .flex_1()
                .max_w(LOADING_BAR_WIDTH)
                .h_full()
                .child(ui::loading_bar(
                    "activity", &activity, &settings, window, cx,
                ))
        });
        bar = bar.child(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .px_4()
                .justify_center()
                .window_control_area(WindowControlArea::Drag)
                .children(loading),
        );
        let update = self.update_button("update-app", cx);
        bar = bar.children(update.map(|button| div().flex_none().pr_2().child(button)));
        if cfg!(windows) {
            bar = bar.child(window_controls(&settings, window.is_maximized()));
        }
        div()
            .flex()
            .flex_col()
            .flex_none()
            .child(bar)
            .child(ui::horizontal_line(window, &settings))
            .into_any_element()
    }

    /// A newer Chessdesk or newer engines to fetch, or a Chessdesk fetched to start: the way
    /// there from anywhere.
    pub(super) fn update_button(
        &self,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let label = update_label(self.installs.0.read(cx))?;
        let settings = Settings::get(cx);
        Some(
            ui::primary_button(id, label, &settings)
                .on_click(cx.listener(|this, _, _, cx| this.update_app(cx))),
        )
    }

    /// What the update button brings, to show beneath it: Chessdesk's notes since this version,
    /// then what the outdated engines' releases say, a few lines of it. The Updates page has
    /// the rest.
    pub(super) fn update_notes(&self, cx: &mut Context<Self>) -> Option<Div> {
        let installs = self.installs.0.read(cx);
        update_label(installs)?;
        let release = match installs.app_update() {
            AppUpdate::Available(release) | AppUpdate::Ready(release, _) => Some(release),
            _ => None,
        };
        let mut entries: Vec<SharedString> = release
            .iter()
            .flat_map(|release| &release.notes)
            .flat_map(|notes| notes.entries.iter().cloned().map(SharedString::from))
            .collect();
        entries.extend(
            installs
                .outdated_engines()
                .into_iter()
                .filter_map(|engine| {
                    let summary = installs.engine_summary(engine)?;
                    Some(format!("{}: {summary}", engine.name()).into())
                }),
        );
        if entries.is_empty() {
            return None;
        }
        let more = entries.len().saturating_sub(NOTES_SHOWN);
        entries.truncate(NOTES_SHOWN);
        if more > 0 {
            entries.push(tr_with!("notes-more", count = more));
        }
        let heading = match release {
            Some(release) => tr_with!(
                "whats-new-in",
                name = format!("Chessdesk {}", release.version)
            ),
            None => tr("whats-new"),
        };
        Some(ui::notes(heading, entries, &Settings::get(cx)).max_w(NOTES_WIDTH))
    }

    /// What takes a while right now: in the game on show first, then in the others, then
    /// what's being installed, then agents asked for their models.
    fn activity(&self, cx: &App) -> Option<Activity> {
        let mut others = self
            .sessions
            .iter()
            .filter(|session| session.id != self.active);
        self.session()
            .activity()
            .or_else(|| others.find_map(Session::activity))
            .or_else(|| self.installs.0.read(cx).activity())
            .or_else(|| self.catalog.0.read(cx).activity())
    }

    /// The logo, the name and what's behind them: all of it one button.
    fn render_brand(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let settings = Settings::get(cx);
        let accent = settings.colors.accent;
        let muted = settings.colors.muted;
        let is_open = self.open_menu == Some(BRAND_MENU);
        let brand = div()
            .id("brand")
            .flex()
            .flex_none()
            .items_center()
            .gap_1p5()
            .h(ui::TITLE_BAR_HEIGHT)
            .pl_3()
            .pr_2()
            .cursor_pointer()
            .text_color(settings.colors.text)
            .when(is_open, |brand| brand.bg(accent))
            .child(ui::hover_fill(accent, px(0.)))
            // Acts on press, so that the root's click-away handler never sees it.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.open_menu = (!is_open).then_some(BRAND_MENU);
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, _: &MouseMoveEvent, _, cx| {
                if this.open_menu.is_some_and(|open| open != BRAND_MENU) {
                    this.open_menu = Some(BRAND_MENU);
                    cx.notify();
                }
            }))
            .child(ui::icon(Icon::Logo, LOGO_HEIGHT, settings.colors.text))
            .child("Chessdesk")
            .child(ui::icon(Icon::ChevronDown, settings.ui_font_size, muted));
        let dropdown = is_open.then(|| self.render_dropdown(BRAND_MENU, window, cx));
        div().flex().flex_col().child(brand).children(dropdown)
    }

    /// What Chessdesk is, as a dialog.
    pub(super) fn render_about(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.about_open {
            return None;
        }
        let settings = Settings::get(cx);
        let muted = settings.colors.muted;
        let version = tr_with!("about-version", version = env!("CARGO_PKG_VERSION"));
        let close = ui::action_button("about-close", tr("close"), &settings).on_click(cx.listener(
            |this, _, _, cx| {
                this.about_open = false;
                cx.notify();
            },
        ));
        Some(
            ui::fade_in(
                "about-shade",
                ui::modal("about-dialog", &settings).child(ui::rise_in(
                    "about-in",
                    ui::dialog(window, &settings)
                        .w(ABOUT_WIDTH)
                        .p_4()
                        .gap_2()
                        .text_color(settings.colors.text)
                        .child(ui::logotype(ABOUT_LOGOTYPE_HEIGHT, settings.colors.text))
                        .child(div().text_color(muted).child(version))
                        .child(tr("about-description"))
                        .child(div().text_color(muted).child(COPYRIGHT))
                        .child(div().flex().justify_end().pt_2().child(close)),
                )),
            )
            .into_any_element(),
        )
    }

    fn render_menu(
        &self,
        index: usize,
        title: &'static str,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let settings = Settings::get(cx);
        let is_open = self.open_menu == Some(index);
        let label = ui::button(title, is_open, &settings)
            // Acts on press, so that the root's click-away handler never sees it.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.open_menu = (!is_open).then_some(index);
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(move |this, _: &MouseMoveEvent, _, cx| {
                if this.open_menu.is_some_and(|open| open != index) {
                    this.open_menu = Some(index);
                    cx.notify();
                }
            }))
            .child(tr(title));
        let dropdown = is_open.then(|| self.render_dropdown(index, window, cx));
        div().flex().flex_col().child(label).children(dropdown)
    }

    fn render_dropdown(
        &self,
        index: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let settings = Settings::get(cx);
        let offset = if index == BRAND_MENU {
            BRAND_DROPDOWN_OFFSET
        } else {
            DROPDOWN_OFFSET
        };
        let update = update_label(self.installs.0.read(cx));
        let rows = menu_entries(index, self.session(), update)
            .into_iter()
            .enumerate()
            .map(|(row, entry)| {
                let Some(entry) = entry else {
                    return ui::separator(window, &settings).into_any_element();
                };
                let item = ui::menu_item(("menu-item", row), entry.title, entry.key, &settings);
                match entry.run {
                    Some(run) => item
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_menu = None;
                            run(this, window, cx);
                            cx.notify();
                        }))
                        .into_any_element(),
                    None => item
                        .cursor_default()
                        .text_color(settings.colors.muted)
                        .into_any_element(),
                }
            });
        deferred(anchored().offset(point(px(0.), offset)).child(ui::drop_in(
            "menu-in",
            ui::menu(window, &settings).children(rows),
        )))
        .with_priority(1)
    }

    /// The four pieces to promote to, over the board.
    pub(super) fn render_promotion(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let session = self.session();
        session.promotion?;
        let settings = Settings::get(cx);
        let color = session.game.position().turn();
        let choices = [Role::Queen, Role::Rook, Role::Bishop, Role::Knight].map(|role| {
            let image = settings.pieces.get(Piece { color, role });
            ui::button(("promote", role as usize), false, &settings)
                .child(img(image).size(PROMOTION_SIZE))
                .on_click(cx.listener(move |this, _, _, cx| this.promote(role, cx)))
        });
        Some(
            ui::fade_in(
                "promotion-shade",
                div()
                    .id("promotion")
                    .occlude()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(settings.colors.background.opacity(0.7))
                    .child(ui::rise_in(
                        "promotion-in",
                        ui::dialog(window, &settings)
                            .flex_row()
                            .gap_1()
                            .children(choices),
                    )),
            )
            .into_any_element(),
        )
    }

    /// Says "Copied" on the trace's button for a moment. A later copy starts the moment
    /// over, so an earlier one's end leaves it be.
    fn show_copied(&mut self, cx: &mut Context<Self>) {
        let copied = Instant::now();
        self.trace_copied = Some(copied);
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            this.update(cx, |this, cx| {
                if this.trace_copied == Some(copied) {
                    this.trace_copied = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// What went wrong, along the bottom until closed.
    pub(super) fn render_errors(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let shown = self.errors.first()?;
        // The rest of a long message comes along in the trace.
        let error = shown.lines().next().unwrap_or_default().to_owned();
        let report = self.trace.report(shown, &self.traced_game());
        let issue = trace::issue_url(&report);
        let settings = Settings::get(cx);
        Some(
            div()
                .flex()
                .flex_col()
                .flex_none()
                .child(ui::horizontal_line(window, &settings))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .h(ui::BAR_HEIGHT)
                        .pl(ui::BAR_INSET)
                        .pr_1()
                        .child(div().flex_1().min_w_0().truncate().child(error))
                        .child(
                            ui::button("copy-error", false, &settings)
                                .h(ui::CONTROL_HEIGHT)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        report.clone(),
                                    ));
                                    this.show_copied(cx);
                                }))
                                .child(tr(if self.trace_copied.is_some() {
                                    "copied"
                                } else {
                                    "copy-trace"
                                })),
                        )
                        .child(
                            ui::action_button("report-error", tr("report-issue"), &settings)
                                .on_click(move |_, _, cx| cx.open_url(&issue)),
                        )
                        .child(
                            ui::button("close-error", true, &settings)
                                .h(ui::CONTROL_HEIGHT)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.errors.remove(0);
                                    this.trace_copied = None;
                                    cx.notify();
                                }))
                                .child(tr("close")),
                        ),
                )
                .into_any_element(),
        )
    }
}
