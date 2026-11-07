//! The window: the games that are open and the panes around them. `flow` is what happens
//! to games; `chrome`, `history` and `panel` draw the rest.

mod chrome;
mod clock;
mod flow;
mod history;
mod motion;
mod panel;
mod session;

use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chessdesk_core::Game;
use chessdesk_core::material::{self, Captures};
use gpui::{
    AnyElement, App, Axis, Context, Div, Entity, FocusHandle, KeyBinding, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollHandle, SharedString,
    Stateful, Subscription, Window, actions, div, prelude::*, px,
};
use notify::{RecursiveMode, Watcher};
use shakmaty::{Color, Role};

use crate::board::{self, Handlers, View};
use crate::export::{self, ExportDialog};
use crate::i18n::tr;
use crate::library::{self, Entry};
use crate::new_game::{self, NewGame};
use crate::settings::Settings;
use crate::settings_view::{self, SettingsPage, SettingsView};
use crate::setup::Setup;
use crate::ui::{self, Edge, Icon};
use motion::Motion;
use panel::Section;
use session::{Session, SessionId};

actions!(
    chessdesk,
    [
        NewGameAction,
        OpenGame,
        SaveGame,
        ExportGame,
        OpenSettings,
        CloseOverlay,
        ToggleHistory,
        TogglePanel,
        ToggleLikelihood,
        FlipBoard,
        TakeBack,
        Resign,
        Analyze,
        PreviousMove,
        NextMove,
        FirstMove,
        LastMove,
        Quit,
    ]
);

/// How long settings wait for a burst of file events to settle.
const WATCH_SETTLE: Duration = Duration::from_millis(250);
/// The room kept around the board.
const BOARD_MARGIN: Pixels = px(24.);
/// The rows above and below the board, and how far they keep from it.
const NO_GAME_LOGOTYPE_HEIGHT: Pixels = px(35.);
const MATERIAL_ROW: Pixels = px(22.);
const MATERIAL_GAP: Pixels = px(4.);
const MATERIAL_ICON_SCALE: f32 = 1.5;
const MATERIAL_ICON_GAP: Pixels = px(2.);
const MATERIAL_ROW_INSET: Pixels = px(2.);
const MIN_PANE: Pixels = px(160.);
const MIN_LOG: Pixels = px(72.);
const MAX_LOG: Pixels = px(600.);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-n", NewGameAction, None),
        KeyBinding::new("ctrl-o", OpenGame, None),
        KeyBinding::new("ctrl-s", SaveGame, None),
        KeyBinding::new("ctrl-e", ExportGame, None),
        KeyBinding::new("ctrl-,", OpenSettings, None),
        KeyBinding::new("escape", CloseOverlay, None),
        KeyBinding::new("ctrl-b", ToggleHistory, None),
        KeyBinding::new("ctrl-shift-b", TogglePanel, None),
        KeyBinding::new("ctrl-f", FlipBoard, None),
        KeyBinding::new("ctrl-z", TakeBack, None),
        KeyBinding::new("left", PreviousMove, None),
        KeyBinding::new("right", NextMove, None),
        KeyBinding::new("home", FirstMove, None),
        KeyBinding::new("end", LastMove, None),
        KeyBinding::new("ctrl-q", Quit, None),
    ]);
}

/// A divider being dragged.
#[derive(Clone, Copy)]
enum Dragging {
    History,
    Panel,
    /// Where the press was and how tall the log was then.
    Log {
        from: Pixels,
        height: Pixels,
    },
}

pub struct Chessdesk {
    focus: FocusHandle,
    /// Every open game. There is always one, an empty board if nothing else.
    sessions: Vec<Session>,
    active: SessionId,
    next_session: SessionId,
    next_slide: u64,
    clocks_running: bool,
    history: Vec<Entry>,
    show_history: bool,
    show_panel: bool,
    history_width: Pixels,
    panel_width: Pixels,
    log_height: Pixels,
    /// The game panel's sections folded into their titles.
    collapsed: HashSet<Section>,
    moves_scroll: ScrollHandle,
    /// The game and row of the log last scrolled to, so that it only follows a change and
    /// leaves the player's own scrolling alone.
    followed_row: Option<(SessionId, usize)>,
    /// Whether to scroll once more: GPUI goes by the log's place in the frame before, and
    /// the log moves when the panel changes along with the game.
    follow_again: bool,
    dragging: Option<Dragging>,
    /// The likelihood bars, on their way to the latest numbers.
    likelihood: Motion,
    open_menu: Option<usize>,
    /// Whether the Analyze button's choices are open.
    analyze_menu: bool,
    /// The game whose row was right-clicked, and where.
    context_menu: Option<(Entry, Point<Pixels>)>,
    /// The game waiting for a yes before it's deleted.
    confirm_delete: Option<Entry>,
    renaming: Option<history::Renaming>,
    about_open: bool,
    dialog: Option<(Entity<NewGame>, Subscription)>,
    export: Option<(Entity<ExportDialog>, Subscription)>,
    /// While open, the settings' pages take the history's place and their content the board's.
    settings_page: Option<(Entity<SettingsView>, [Subscription; 2])>,
    errors: Vec<SharedString>,
}

impl Chessdesk {
    pub fn new(errors: Vec<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        focus.focus(window);
        watch_settings(cx);
        let settings = Settings::get(cx);
        let mut this = Self {
            focus,
            sessions: vec![Session::new(0, Game::default())],
            active: 0,
            next_session: 0,
            next_slide: 0,
            clocks_running: false,
            history: library::list(),
            show_history: true,
            show_panel: true,
            history_width: settings.history_width,
            panel_width: settings.panel_width,
            log_height: settings.log_height,
            // Notes are there for when they're wanted, not in the way of the moves.
            collapsed: HashSet::from([Section::Notes]),
            moves_scroll: ScrollHandle::new(),
            followed_row: None,
            follow_again: false,
            dragging: None,
            likelihood: Motion::new(Instant::now()),
            open_menu: None,
            analyze_menu: false,
            context_menu: None,
            confirm_delete: None,
            renaming: None,
            about_open: false,
            dialog: None,
            export: None,
            settings_page: None,
            errors: errors.into_iter().map(SharedString::from).collect(),
        };
        if this.history.is_empty() {
            this.show_new_game(cx);
        }
        this
    }

    /// Keeps the move on show in view in the log.
    fn follow_moves(&mut self, window: &mut Window) {
        let session = self.session();
        let row = session.log_row();
        let shown = Some((session.id, row));
        if self.followed_row != shown {
            self.followed_row = shown;
            self.follow_again = true;
            self.moves_scroll.scroll_to_item(row);
            window.request_animation_frame();
        } else if std::mem::take(&mut self.follow_again) {
            self.moves_scroll.scroll_to_item(row);
        }
    }

    /// The game on show.
    fn session(&self) -> &Session {
        self.sessions
            .iter()
            .find(|session| session.id == self.active)
            .unwrap_or(&self.sessions[0])
    }

    fn session_mut(&mut self, id: SessionId) -> Option<&mut Session> {
        self.sessions.iter_mut().find(|session| session.id == id)
    }

    fn report(&mut self, error: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.errors.push(error.into());
        cx.notify();
    }

    fn show_new_game(&mut self, cx: &mut Context<Self>) {
        let dialog = new_game::dialog(Setup::load(), cx);
        let subscription = cx.subscribe(&dialog, |this, _, event, cx| {
            this.dialog = None;
            match event {
                new_game::Event::Start(setup) => this.start_game(setup.clone(), cx),
                new_game::Event::OpenSettings(page) => this.show_settings(*page, cx),
                new_game::Event::Cancel => {}
            }
            cx.notify();
        });
        self.dialog = Some((dialog, subscription));
        cx.notify();
    }

    fn show_settings(&mut self, page: SettingsPage, cx: &mut Context<Self>) {
        let view = settings_view::page(page, cx);
        // Its parts render here, so its changes have to redraw this view.
        let redraw = cx.observe(&view, |_, _, cx| cx.notify());
        let events = cx.subscribe(&view, |this, _, event, cx| match event {
            settings_view::Event::Changed => this.reload_settings(cx),
            settings_view::Event::Close => {
                this.settings_page = None;
                cx.notify();
            }
            settings_view::Event::Failed(error) => this.report(error.clone(), cx),
        });
        self.settings_page = Some((view, [redraw, events]));
        cx.notify();
    }

    fn show_export(&mut self, game: Game, path: &Path, cx: &mut Context<Self>) {
        let name = path
            .file_stem()
            .map_or("game".into(), |stem| stem.to_string_lossy());
        let dialog = export::dialog(game, &name, cx);
        let subscription = cx.subscribe(&dialog, |this, _, event, cx| {
            this.export = None;
            if let export::Event::Failed(error) = event {
                this.report(error.clone(), cx);
            }
            cx.notify();
        });
        self.export = Some((dialog, subscription));
        cx.notify();
    }

    fn reload_settings(&mut self, cx: &mut Context<Self>) {
        let (settings, errors) = Settings::load();
        Settings::set(settings, cx);
        self.errors = errors.into_iter().map(SharedString::from).collect();
        cx.refresh_windows();
    }

    /// Puts a game's row in the history, newest first.
    fn update_history(&mut self, entry: Entry) {
        match self
            .history
            .iter_mut()
            .find(|known| known.path == entry.path)
        {
            Some(known) => *known = entry,
            None => self.history.insert(0, entry),
        }
    }

    fn step_to(&mut self, ply: usize, cx: &mut Context<Self>) {
        self.show_ply(self.active, ply, true, cx);
    }

    fn board_handlers(cx: &mut Context<Self>) -> Handlers {
        let this = cx.entity().downgrade();
        let drop_target = this.clone();
        Handlers {
            press: Rc::new(move |square, _, cx| {
                let _ = this.update(cx, |this, cx| this.press_square(square, cx));
            }),
            drop: Rc::new(move |from, to, _, cx| {
                let _ = drop_target.update(cx, |this, cx| this.drop_piece(from, to, cx));
            }),
        }
    }

    fn board_view(&self, size: Pixels) -> View {
        let session = self.session();
        let last_move = session
            .shown_move()
            .and_then(|played| Some((played.played.from()?, played.played.to())));
        View {
            position: session.game.position_at(session.ply),
            orientation: session.orientation,
            selected: session.selected,
            targets: session.targets(),
            last_move,
            annotations: session.annotations(),
            size,
            slides: session.slides.clone(),
        }
    }

    /// The room a sidebar takes, its strip aside.
    fn history_room(&self) -> Pixels {
        if self.show_history {
            self.history_width
        } else {
            px(0.)
        }
    }

    fn panel_room(&self) -> Pixels {
        if self.show_panel {
            self.panel_width
        } else {
            px(0.)
        }
    }

    /// Whole pixels per square, so that no seams show between them.
    fn board_size(&self, window: &Window, cx: &App) -> Pixels {
        let settings = Settings::get(cx);
        let viewport = window.viewport_size();
        let side_room = self.history_room() + self.panel_room() + ui::EDGE_STRIP_SIZE * 2.;
        let wide = viewport.width - side_room - BOARD_MARGIN * 2.;
        let rows = (MATERIAL_ROW + MATERIAL_GAP) * 2.;
        let high = viewport.height - ui::TITLE_BAR_HEIGHT - BOARD_MARGIN * 2. - rows;
        let room = wide.min(high).max(MIN_PANE);
        (room * settings.board_size / 8.).floor() * 8.
    }

    fn on_drag(&mut self, event: &MouseMoveEvent, window: &Window, cx: &mut Context<Self>) {
        let Some(dragging) = self.dragging else {
            return;
        };
        if !event.dragging() {
            self.dragging = None;
            return;
        }
        let width = window.viewport_size().width;
        let widest = (width / 2.).max(MIN_PANE);
        match dragging {
            Dragging::History => {
                let from_edge = event.position.x - ui::EDGE_STRIP_SIZE;
                self.history_width = from_edge.clamp(MIN_PANE, widest);
            }
            Dragging::Panel => {
                let from_edge = width - event.position.x - ui::EDGE_STRIP_SIZE;
                self.panel_width = from_edge.clamp(MIN_PANE, widest);
            }
            Dragging::Log { from, height } => {
                self.log_height = (height + from - event.position.y).clamp(MIN_LOG, MAX_LOG);
            }
        }
        cx.notify();
    }

    fn render_divider(
        id: &'static str,
        axis: Axis,
        drag: impl Fn(&MouseDownEvent, &Self) -> Dragging + 'static,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let settings = Settings::get(cx);
        let on_press = cx.listener(move |this, event: &MouseDownEvent, _, cx| {
            this.dragging = Some(drag(event, this));
            cx.stop_propagation();
        });
        ui::divider(id, axis, window, &settings, on_press)
    }

    /// With no game to show there's no board: just the way to start one.
    fn render_no_game(cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        div()
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_6()
            .child(ui::logotype(NO_GAME_LOGOTYPE_HEIGHT, settings.colors.text))
            .child(
                ui::action_button("start-new", tr("new-game"), &settings)
                    .on_click(cx.listener(|this, _, _, cx| this.show_new_game(cx))),
            )
            .into_any_element()
    }

    fn render_board(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        if self.session().is_blank() {
            return Self::render_no_game(cx);
        }
        let size = self.board_size(window, cx);
        let view = self.board_view(size);
        let handlers = Self::board_handlers(cx);
        let session = self.session();
        let captures = material::captures(&session.game, session.ply);
        let orientation = view.orientation;
        let settings = Settings::get(cx);
        let board = board::render(&view, &handlers, cx);
        div()
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(MATERIAL_GAP)
                    .child(render_material(
                        &captures,
                        orientation.other(),
                        &settings,
                        window,
                    ))
                    .child(board)
                    .child(render_material(&captures, orientation, &settings, window)),
            )
            .into_any_element()
    }

    /// The nav and the page of the settings, to go where the history and the board go.
    fn render_settings(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(AnyElement, AnyElement)> {
        let (page, _) = self.settings_page.as_ref()?;
        Some(page.update(cx, |page, cx| {
            (
                page.render_nav(cx).into_any_element(),
                page.render_content(window, cx).into_any_element(),
            )
        }))
    }

    fn render_strip(
        edge: Edge,
        open: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let settings = Settings::get(cx);
        let (id, label) = match edge {
            Edge::Left => ("history-strip", tr("game-history")),
            Edge::Right => ("panel-strip", tr("game-panel")),
        };
        ui::edge_strip(
            id,
            edge,
            open,
            label,
            window,
            &settings,
            cx.listener(move |this, _, _, cx| {
                match edge {
                    Edge::Left => this.show_history = !this.show_history,
                    Edge::Right => this.show_panel = !this.show_panel,
                }
                cx.notify();
            }),
        )
    }
}

/// How wide a piece is drawn for its height. The icons are all square with the drawing
/// filling the height, so that every piece renders through the same box and stands exactly
/// as tall as the others: boxes of different widths come out a pixel apart.
fn role_width(role: Role) -> f32 {
    match role {
        Role::Pawn => 288. / 480.,
        Role::Bishop => 320. / 512.,
        Role::Rook => 416. / 480.,
        Role::Knight => 416. / 512.,
        Role::King => 448. / 512.,
        Role::Queen => 1.,
    }
}

/// A side's row beside the board: what it has taken, and its lead. Always there, so that
/// the board doesn't move when the first piece falls. Sizes and gaps are whole device
/// pixels: otherwise the same piece rasterises differently from one place to the next.
fn render_material(captures: &Captures, color: Color, settings: &Settings, window: &Window) -> Div {
    let height = ui::snap(settings.ui_font_size * MATERIAL_ICON_SCALE, window);
    let scale = window.scale_factor();
    let icons = captures.by(color).iter().map(|role| {
        // What the square leaves empty beside the piece, given back to the row.
        let spare = height * (1. - role_width(*role)) / 2.;
        ui::icon(Icon::of_role(*role), height, settings.colors.muted)
            .mx((spare * scale).floor() / -scale)
    });
    let lead = captures
        .lead_of(color)
        .map(|lead| div().pl_1().child(format!("+{lead}")));
    div()
        .flex()
        .flex_none()
        .items_end()
        .h(MATERIAL_ROW)
        .gap(ui::snap(MATERIAL_ICON_GAP, window))
        .pb(ui::snap(MATERIAL_ROW_INSET, window))
        .text_color(settings.colors.muted)
        .children(icons)
        .children(lead)
}

/// Reloads settings when anything in the config folder changes.
fn watch_settings(cx: &mut Context<Chessdesk>) {
    let Some(folder) = Settings::config_dir() else {
        return;
    };
    let (changes, incoming) = mpsc::channel();
    let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok() {
            let _ = changes.send(());
        }
    });
    let Ok(mut watcher) = watcher else { return };
    if std::fs::create_dir_all(&folder).is_err()
        || watcher.watch(&folder, RecursiveMode::Recursive).is_err()
    {
        return;
    }
    cx.spawn(async move |this, cx| {
        // Lives as long as this task, which lives as long as the window.
        let _watcher = watcher;
        loop {
            cx.background_executor().timer(WATCH_SETTLE).await;
            if incoming.try_iter().count() > 0
                && this.update(cx, Chessdesk::reload_settings).is_err()
            {
                return;
            }
        }
    })
    .detach();
}

impl Chessdesk {
    /// What every action does.
    fn with_actions(root: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        root.on_action(cx.listener(|this, _: &NewGameAction, _, cx| this.show_new_game(cx)))
            .on_action(cx.listener(|_, _: &OpenGame, _, cx| Self::prompt_open(cx)))
            .on_action(cx.listener(|this, _: &SaveGame, _, cx| this.prompt_save(cx)))
            .on_action(cx.listener(|this, _: &ExportGame, _, cx| {
                let session = this.session();
                let (game, path) = (session.game.clone(), session.path.clone());
                this.show_export(game, &path.unwrap_or_default(), cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleLikelihood, _, cx| {
                this.toggle_likelihood(cx);
            }))
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| {
                this.show_settings(SettingsPage::Appearance, cx);
            }))
            .on_action(cx.listener(|this, _: &CloseOverlay, _, cx| {
                let closed = this.dialog.take().is_some()
                    || this.export.take().is_some()
                    || this.confirm_delete.take().is_some()
                    || std::mem::take(&mut this.about_open);
                if !closed {
                    this.settings_page = None;
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleHistory, _, cx| {
                this.show_history = !this.show_history;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &TogglePanel, _, cx| {
                this.show_panel = !this.show_panel;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FlipBoard, _, cx| {
                let id = this.active;
                if let Some(session) = this.session_mut(id) {
                    session.orientation = session.orientation.other();
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &TakeBack, _, cx| this.take_back(cx)))
            .on_action(cx.listener(|this, _: &Resign, _, cx| this.resign(cx)))
            .on_action(cx.listener(|this, _: &Analyze, _, cx| this.analyze(cx)))
            .on_action(cx.listener(|this, _: &PreviousMove, _, cx| {
                this.step_to(this.session().ply.saturating_sub(1), cx);
            }))
            .on_action(cx.listener(|this, _: &NextMove, _, cx| {
                this.step_to(this.session().ply + 1, cx);
            }))
            .on_action(cx.listener(|this, _: &FirstMove, _, cx| this.step_to(0, cx)))
            .on_action(cx.listener(|this, _: &LastMove, _, cx| this.step_to(usize::MAX, cx)))
            .on_action(cx.listener(|_, _: &Quit, _, cx| cx.quit()))
    }
}

impl Render for Chessdesk {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::get(cx);
        match self.session().known_wdl() {
            Some(wdl) => self.likelihood.aim(wdl, Instant::now()),
            None => self.likelihood.clear(Instant::now()),
        }
        self.follow_moves(window);
        let top_bar = self.render_top_bar(window, cx);
        let (side, main) = match self.render_settings(window, cx) {
            Some((nav, page)) => (Some(self.render_side(nav, window, cx)), page),
            None => (
                self.show_history.then(|| self.render_history(window, cx)),
                self.render_board(window, cx),
            ),
        };
        // Settings fill the window between them, as they do in the rest of the layout.
        let plain = self.settings_page.is_none();
        let history_strip =
            plain.then(|| Self::render_strip(Edge::Left, self.show_history, window, cx));
        let panel_strip =
            plain.then(|| Self::render_strip(Edge::Right, self.show_panel, window, cx));
        let panel = (plain && self.show_panel).then(|| self.render_panel(window, cx));
        let promotion = self.render_promotion(window, cx);
        let errors = self.render_errors(window, cx);
        let dialog = self.dialog.as_ref().map(|(dialog, _)| dialog.clone());
        let export = self.export.as_ref().map(|(export, _)| export.clone());
        let context_menu = self.render_context_menu(window, cx);
        let confirm_delete = self.render_confirm_delete(window, cx);
        let about = self.render_about(window, cx);

        // Focus is tracked below the title bar: a press on anything that tracks focus
        // counts as handled, and Windows then never sees the window controls pressed.
        // Overlays stay below it too, so that the window can always be moved and closed.
        let content = div()
            .id("content")
            .track_focus(&self.focus)
            .relative()
            .flex()
            .flex_1()
            .min_h_0()
            .children(history_strip)
            .children(side)
            .child(main)
            .children(panel)
            .children(panel_strip)
            .children(promotion)
            .children(dialog)
            .children(export)
            .children(confirm_delete)
            .children(about)
            .children(context_menu);

        let root = div()
            .id("chessdesk")
            .key_context("Chessdesk")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(settings.colors.background)
            .text_color(settings.colors.text)
            .font(settings.ui_font.clone())
            .text_size(settings.ui_font_size);
        Self::with_actions(root, cx)
            .on_mouse_move(cx.listener(|this, event, window, cx| this.on_drag(event, window, cx)))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, _| this.dragging = None),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    let menu = this.open_menu.take().is_some();
                    if this.context_menu.take().is_some() || menu {
                        cx.notify();
                    }
                }),
            )
            .child(top_bar)
            .child(content)
            .children(errors)
    }
}
