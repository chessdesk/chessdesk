//! The dialog shown before a game: what kind of game, who plays, how strong and how fast,
//! and what an agent may spend.

use std::rc::Rc;

use chessdesk_core::Game;
use chessdesk_players::acp::{AgentKind, Effort};
use chessdesk_players::download::MAIA_LEVELS;
use gpui::{
    AnyElement, App, Axis, Context, Div, Entity, EventEmitter, MouseButton, Pixels, SharedString,
    Subscription, Window, div, prelude::*, px, transparent_black,
};

use crate::combobox::{self, Combobox};
use crate::i18n::{tr, tr_with};
use crate::installs::{self, Installs, NeedsStockfish};
use crate::models::{Catalog, MAX_PLAIN_MODELS, ModelEntry, Offer, model_entries};
use crate::scenario;
use crate::settings::Settings;
use crate::settings_view::SettingsPage;
use crate::setup::{Mode, OpponentKind, Setup, Side, Speed, effort_label, maia_strength};
use crate::text_area::TextArea;
use crate::ui::{self, Icon, Logo};

/// The form's width, beside the modes.
const FORM_WIDTH: Pixels = px(468.);
const SIDEBAR_WIDTH: Pixels = px(228.);
/// `UCI_Elo` runs from 1320 to 3190. Past the last stop it plays at full strength.
const ELO_LEVELS: [u32; 10] = [1320, 1500, 1700, 1900, 2100, 2300, 2500, 2700, 2900, 3100];
/// Minutes each and seconds per move, for the clock's list.
const TIME_CONTROLS: [(u32, u32); 5] = [(0, 0), (5, 0), (10, 0), (15, 10), (30, 0)];
/// The clocks of a competition, fastest first, laid out in rows.
const COMPETITION_CLOCKS: [(u32, u32); 12] = [
    (1, 0),
    (2, 1),
    (3, 0),
    (3, 2),
    (5, 0),
    (5, 3),
    (10, 0),
    (10, 5),
    (15, 10),
    (30, 0),
    (30, 20),
    (90, 30),
];
const CLOCKS_PER_ROW: usize = 4;
const MODES: [Mode; 3] = [Mode::Casual, Mode::Competition, Mode::Scenario];
/// How faded a mode that isn't ready yet is.
const NOT_READY_OPACITY: f32 = 0.5;

/// Whether a mode can be picked yet: scenarios are still to come.
fn ready(mode: Mode) -> bool {
    mode != Mode::Scenario
}
const BUDGETS: [Option<u64>; 4] = [None, Some(50_000), Some(200_000), Some(1_000_000)];

/// What the dialog tells the app.
pub enum Event {
    Start(Setup),
    /// The player wants to fetch what's missing.
    OpenSettings(SettingsPage),
    Cancel,
}

/// One option of a select, and what picking it changes.
struct Choice {
    label: SharedString,
    logo: Option<Logo>,
    /// A word on the choice, quieter, at the row's end.
    note: Option<SharedString>,
    pick: Rc<dyn Fn(&mut Setup)>,
}

fn choice(label: impl Into<SharedString>, pick: impl Fn(&mut Setup) + 'static) -> Choice {
    Choice {
        label: label.into(),
        logo: None,
        note: None,
        pick: Rc::new(pick),
    }
}

/// Which select a row is, and how it's labelled.
struct Field {
    id: &'static str,
    label: SharedString,
    hint: Option<SharedString>,
    logo: Option<Logo>,
}

fn field(id: &'static str, label_id: &str) -> Field {
    Field {
        id,
        label: tr(label_id),
        hint: None,
        logo: None,
    }
}

/// The small mark of an opponent, if it has one.
fn logo_of(kind: OpponentKind) -> Option<Logo> {
    match kind {
        OpponentKind::Stockfish => Some(Logo::Stockfish),
        OpponentKind::Maia => Some(Logo::Maia),
        _ => kind.agent().map(Logo::of_agent),
    }
}

pub struct NewGame {
    setup: Setup,
    instructions: Entity<TextArea>,
    open: Option<&'static str>,
    error: Option<SharedString>,
    /// What each agent offers, asked for when it's picked.
    catalog: Entity<Catalog>,
    /// For Stockfish, when what's picked needs it and it isn't there.
    installs: Entity<Installs>,
    /// The model list to type in, while it's open.
    combobox: Option<(Entity<Combobox>, Subscription)>,
    _catalog_changes: Subscription,
    _installs_changes: Subscription,
}

impl EventEmitter<Event> for NewGame {}

fn side_label(side: Side) -> SharedString {
    tr(match side {
        Side::White => "side-white",
        Side::Black => "side-black",
        Side::Random => "side-random",
    })
}

fn mode_labels(mode: Mode) -> (SharedString, SharedString, Icon) {
    let (label, hint, icon) = match mode {
        Mode::Casual => ("mode-casual", "mode-casual-hint", Icon::Logo),
        Mode::Competition => ("mode-competition", "mode-competition-hint", Icon::ChessKing),
        Mode::Scenario => ("mode-scenario", "mode-scenario-hint", Icon::ChessRook),
    };
    let hint = if ready(mode) { hint } else { "coming-soon" };
    (tr(label), tr(hint), icon)
}

fn speed_label(speed: Speed) -> SharedString {
    tr(match speed {
        Speed::Bullet => "speed-bullet",
        Speed::Blitz => "speed-blitz",
        Speed::Rapid => "speed-rapid",
        Speed::Classical => "speed-classical",
    })
}

fn time_label(minutes: u32, increment: u32) -> SharedString {
    match (minutes, increment) {
        (0, _) => tr("no-clock"),
        (minutes, 0) => tr_with!("clock-minutes", minutes = minutes),
        (minutes, increment) => {
            tr_with!("clock-increment", minutes = minutes, increment = increment)
        }
    }
}

fn budget_label(budget: Option<u64>) -> SharedString {
    budget.map_or_else(
        || tr("no-limit"),
        |tokens| tr_with!("budget-tokens", tokens = thousands(tokens)),
    )
}

fn thousands(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{}M", value / 1_000_000)
    } else {
        format!("{}k", value / 1000)
    }
}

/// The slider stop for a level: the nearest there is, for one saved by an older version.
fn nearest_stop(levels: &[u32], level: u32) -> usize {
    let distance = |known: &u32| known.abs_diff(level);
    (0..levels.len())
        .min_by_key(|stop| distance(&levels[*stop]))
        .unwrap_or(0)
}

impl NewGame {
    pub fn new(
        mut setup: Setup,
        catalog: Entity<Catalog>,
        installs: Entity<Installs>,
        cx: &mut Context<Self>,
    ) -> Self {
        // A mode remembered from before that can't be picked yet, and the position it
        // brought, which a casual game would otherwise start from.
        if !ready(setup.mode) {
            setup.mode = Mode::Casual;
            setup.fen = None;
        }
        let text = setup.instructions.clone();
        let catalog_changes = cx.observe(&catalog, |_, _, cx| cx.notify());
        let installs_changes = cx.observe(&installs, |_, _, cx| cx.notify());
        let mut dialog = Self {
            setup,
            instructions: cx.new(|cx| TextArea::new(text, tr("instructions-example"), cx)),
            open: None,
            error: None,
            catalog,
            installs,
            combobox: None,
            _catalog_changes: catalog_changes,
            _installs_changes: installs_changes,
        };
        dialog.ask_for_models(cx);
        dialog
    }

    /// Asks the agent picked which models it offers, if that's not known yet.
    fn ask_for_models(&mut self, cx: &mut Context<Self>) {
        if let Some(kind) = self.setup.opponent.agent() {
            self.catalog.update(cx, |catalog, cx| catalog.ask(kind, cx));
        }
    }

    /// The agent's models to pick from: a select, or for a long list or none at all, one
    /// to type in.
    fn model_row(&self, kind: AgentKind, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let catalog = self.catalog.read(cx);
        let mut hint = tr("model-hint");
        let models = match catalog.offer(kind) {
            None | Some(Offer::Asking) => {
                let asking =
                    ui::select_button("model", tr("models-asking"), None, false, &settings)
                        .cursor_default();
                return ui::setting_row(tr("model"), Some(hint), &settings)
                    .child(asking)
                    .into_any_element();
            }
            Some(Offer::Failed(error)) => {
                hint = tr_with!("models-unknown", error = error.as_str());
                None
            }
            Some(Offer::Known(models)) => models.clone(),
        };
        let entries = model_entries(models.as_ref(), self.setup.model.as_deref());
        let current = entries
            .iter()
            .position(|(_, id)| *id == self.setup.model)
            .unwrap_or(0);
        if let Some((combobox, _)) = &self.combobox {
            return ui::setting_row(tr("model"), Some(hint), &settings)
                .child(combobox.clone())
                .into_any_element();
        }
        let listed = models.as_ref().map_or(0, |models| models.choices.len());
        let typed = models.is_some() && (listed > MAX_PLAIN_MODELS || listed == 0);
        if typed {
            let label = entries[current].0.clone();
            let button = ui::select_button("model", label, None, false, &settings).on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    this.open_combobox(entries.clone(), current, window, cx);
                }),
            );
            return ui::setting_row(tr("model"), Some(hint), &settings)
                .child(button)
                .into_any_element();
        }
        let label = entries[current].0.clone();
        let choices = entries
            .into_iter()
            .map(|(label, id)| {
                let name = label.to_string();
                choice(label, move |setup| {
                    setup.model.clone_from(&id);
                    setup.model_name = id.is_some().then(|| name.clone());
                })
            })
            .collect();
        let field = Field {
            hint: Some(hint),
            ..field("model", "model")
        };
        self.select(field, label, choices, window, cx)
    }

    fn open_combobox(
        &mut self,
        entries: Vec<ModelEntry>,
        current: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let labels = entries.iter().map(|(label, _)| label.clone()).collect();
        let combobox = cx.new(|cx| Combobox::new(current, labels, window, cx));
        let subscription = cx.subscribe(&combobox, move |this, _, event, cx| {
            match event {
                combobox::Event::Picked(place) => {
                    let (label, id) = entries[*place].clone();
                    this.setup.model_name = id.is_some().then(|| label.to_string());
                    this.setup.model = id;
                }
                combobox::Event::Typed(model) => {
                    this.setup.model = Some(model.clone());
                    this.setup.model_name = Some(model.clone());
                }
                combobox::Event::Dismissed => {}
            }
            this.combobox = None;
            cx.notify();
        });
        self.open = None;
        self.combobox = Some((combobox, subscription));
        cx.notify();
    }

    fn select(
        &self,
        field: Field,
        current: SharedString,
        choices: Vec<Choice>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Field {
            id,
            label,
            hint,
            logo,
        } = field;
        let settings = Settings::get(cx);
        let is_open = self.open == Some(id);
        let button = ui::select_button(id, current, logo, is_open, &settings).on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                this.open = (!is_open).then_some(id);
                // One list open at a time: the field of a list to type in keeps the focus, so
                // nothing else would close it.
                this.combobox = None;
                cx.notify();
            }),
        );
        let list = is_open.then(|| {
            let rows = choices.into_iter().enumerate().map(|(index, choice)| {
                let pick = choice.pick.clone();
                ui::list_row(("choice", index), false, &settings)
                    .gap_1p5()
                    .text_color(settings.colors.text)
                    .children(choice.logo.map(|mark| {
                        ui::logo(
                            mark,
                            settings.ui_font_size * ui::LOGO_SCALE,
                            settings.colors.text,
                        )
                    }))
                    .child(div().flex_1().min_w_0().truncate().child(choice.label))
                    .children(choice.note.map(|note| {
                        div()
                            .flex_none()
                            .text_color(settings.colors.muted)
                            .child(note)
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        pick(&mut this.setup);
                        this.open = None;
                        this.error = None;
                        this.ask_for_models(cx);
                        cx.notify();
                    }))
            });
            ui::select_dropdown(
                ui::popover(window, &settings)
                    .id("list")
                    .occlude()
                    .w(ui::SELECT_WIDTH)
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        this.open = None;
                        cx.notify();
                    }))
                    .children(rows),
            )
        });
        ui::setting_row(label, hint, &settings)
            .child(div().child(button).children(list))
            .into_any_element()
    }

    fn switch(
        id: &'static str,
        label: SharedString,
        hint: SharedString,
        on: bool,
        flip: fn(&mut Setup),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = Settings::get(cx);
        ui::setting_row(label, Some(hint), &settings)
            .child(
                ui::toggle(id, on, window, cx, &settings).on_click(cx.listener(
                    move |this, _, _, cx| {
                        flip(&mut this.setup);
                        cx.notify();
                    },
                )),
            )
            .into_any_element()
    }

    /// A row with a stepped slider, naming the stop it's on.
    fn level_row(
        id: &'static str,
        label: SharedString,
        value: SharedString,
        (count, index): (usize, usize),
        pick: fn(&mut Setup, usize),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = Settings::get(cx);
        let view = cx.weak_entity();
        let slider = ui::slider(
            id,
            count,
            index,
            &settings,
            window,
            cx,
            move |stop, _, cx| {
                let _ = view.update(cx, |this, cx| {
                    pick(&mut this.setup, stop);
                    cx.notify();
                });
            },
        );
        ui::setting_row(label, Some(value), &settings)
            .child(slider)
            .into_any_element()
    }

    fn opponent_choices() -> Vec<Choice> {
        OpponentKind::ALL
            .into_iter()
            .map(|kind| {
                // OpenCode comes with models that cost nothing, and Pi offers OpenRouter's,
                // which is worth knowing here.
                let note = matches!(kind, OpponentKind::OpenCode | OpponentKind::Pi)
                    .then(|| tr("free-options"));
                Choice {
                    logo: logo_of(kind),
                    note,
                    ..choice(kind.name(), move |setup| {
                        setup.opponent = kind;
                        setup.model = None;
                        setup.model_name = None;
                    })
                }
            })
            .collect()
    }

    /// What's needed before this opponent can play, with the way to get it if there is one:
    /// its button starts fetching it, and opens the settings page that shows it coming.
    fn missing_row(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let settings = Settings::get(cx);
        let opponent = self.setup.opponent;
        let missing = opponent.missing(&settings)?;
        let (label, page) = if opponent.agent().is_some() {
            (tr("install-in-settings"), SettingsPage::Agents)
        } else {
            (tr("download-in-settings"), SettingsPage::Engines)
        };
        let action = opponent.downloadable().then(|| {
            ui::notice_button("download", label, false, &settings).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.installs
                        .update(cx, |installs, cx| installs.fetch_for(opponent, cx));
                    cx.emit(Event::OpenSettings(page));
                },
            ))
        });
        Some(
            ui::notice(missing, None, &settings)
                .mb_1()
                .children(action)
                .into_any_element(),
        )
    }

    fn strength_row(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let setup = &self.setup;
        match setup.opponent {
            OpponentKind::Stockfish => {
                let stops = ELO_LEVELS.len() + 1;
                let index = setup
                    .stockfish_elo
                    .map_or(stops - 1, |elo| nearest_stop(&ELO_LEVELS, elo));
                let value = setup.stockfish_elo.map_or_else(
                    || tr("full-strength"),
                    |elo| tr_with!("engine-elo", elo = elo),
                );
                let pick = |setup: &mut Setup, stop: usize| {
                    setup.stockfish_elo = ELO_LEVELS.get(stop).copied();
                };
                Some(Self::level_row(
                    "strength",
                    tr("strength"),
                    value,
                    (stops, index),
                    pick,
                    window,
                    cx,
                ))
            }
            OpponentKind::Maia => {
                let index = nearest_stop(&MAIA_LEVELS, setup.maia_level);
                let value = tr_with!("maia-strength", rating = maia_strength(setup.maia_level));
                let pick = |setup: &mut Setup, stop: usize| {
                    setup.maia_level = MAIA_LEVELS[stop];
                };
                Some(Self::level_row(
                    "strength",
                    tr("strength"),
                    value,
                    (MAIA_LEVELS.len(), index),
                    pick,
                    window,
                    cx,
                ))
            }
            kind => Some(self.model_row(kind.agent()?, window, cx)),
        }
    }

    /// For agents, how hard they think takes the place of how well they play.
    fn effort_row(window: &mut Window, cx: &mut Context<Self>, effort: Effort) -> AnyElement {
        let index = Effort::ALL
            .iter()
            .position(|known| *known == effort)
            .unwrap_or(0);
        let value = tr_with!("effort-value", effort = effort_label(effort).to_string());
        let pick = |setup: &mut Setup, stop: usize| setup.effort = Effort::ALL[stop];
        Self::level_row(
            "effort",
            tr("reasoning-effort"),
            value,
            (Effort::ALL.len(), index),
            pick,
            window,
            cx,
        )
    }

    /// Reads a position from the clipboard, the one way a FEN gets in here.
    fn paste_fen(&mut self, cx: &mut Context<Self>) {
        let text = cx.read_from_clipboard().and_then(|item| item.text());
        match text.map(|text| text.trim().to_owned()) {
            Some(fen) if Game::parse_fen(&fen).is_some() => {
                self.setup.fen = Some(fen);
                self.error = None;
            }
            _ => self.error = Some(tr("invalid-fen")),
        }
        cx.notify();
    }

    /// The kinds of game, down the dialog's side as the settings' pages are: each brings its
    /// own rows to the form beside it.
    fn mode_nav(&self, cx: &mut Context<Self>) -> Div {
        let settings = Settings::get(cx);
        let rows = MODES.into_iter().map(|mode| {
            let (label, hint, icon) = mode_labels(mode);
            let picked = self.setup.mode == mode;
            // Framed when picked; the others' frame is there but unseen, so nothing shifts.
            let edge = if picked {
                ui::card_edge(&settings)
            } else {
                transparent_black()
            };
            let id = ("mode", mode as usize);
            // One that isn't ready yet shows as the others do, faded and not to be pressed.
            let row = if ready(mode) {
                ui::list_row(id, picked, &settings).on_click(cx.listener(move |this, _, _, cx| {
                    this.setup.set_mode(mode);
                    this.open = None;
                    this.combobox = None;
                    this.error = None;
                    cx.notify();
                }))
            } else {
                div()
                    .id(id)
                    .flex()
                    .w_full()
                    .px_2()
                    .rounded(settings.corner_radius)
                    .opacity(NOT_READY_OPACITY)
            };
            row.h_auto()
                .border_1()
                .border_color(edge)
                .items_center()
                .gap_3()
                .py_1p5()
                .text_color(settings.colors.text)
                .child(ui::icon(icon, settings.ui_font_size, settings.colors.text))
                .child(
                    div().flex().flex_col().min_w_0().child(label).child(
                        div()
                            .text_size(settings.ui_font_size * 0.92)
                            .text_color(settings.colors.muted)
                            .child(hint),
                    ),
                )
        });
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(SIDEBAR_WIDTH)
            .p_2()
            .gap_0p5()
            .children(rows)
    }

    /// A competition's clocks as a grid, each with how fast it is beneath it.
    fn clock_grid(setup: &Setup, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let current = (setup.minutes, setup.increment);
        let tile = |index: usize, (minutes, increment): (u32, u32)| {
            let picked = current == (minutes, increment);
            let speed = Speed::of(minutes, increment).map(speed_label);
            ui::button(("clock", index), picked, &settings)
                .flex_1()
                .min_w_0()
                .flex_col()
                .justify_center()
                .py_1()
                .border_1()
                .border_color(if picked {
                    ui::card_edge(&settings)
                } else {
                    settings.colors.popover_border
                })
                .child(
                    div()
                        .text_color(settings.colors.text)
                        .child(format!("{minutes}+{increment}")),
                )
                .children(
                    speed.map(|speed| div().text_size(settings.ui_font_size * 0.92).child(speed)),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.setup.minutes = minutes;
                    this.setup.increment = increment;
                    cx.notify();
                }))
        };
        let rows = COMPETITION_CLOCKS
            .chunks(CLOCKS_PER_ROW)
            .enumerate()
            .map(|(row, clocks)| {
                let tiles = clocks
                    .iter()
                    .enumerate()
                    .map(|(column, clock)| tile(row * CLOCKS_PER_ROW + column, *clock));
                div().flex().gap_1().children(tiles)
            })
            .collect::<Vec<_>>();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .py_1()
            .child(tr("time-control"))
            .children(rows)
            .into_any_element()
    }

    /// The clock as a list, for games where it matters less.
    fn clock_select(&self, setup: &Setup, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let clock = Field {
            hint: Speed::of(setup.minutes, setup.increment).map(speed_label),
            ..field("clock", "time-control")
        };
        self.select(
            clock,
            time_label(setup.minutes, setup.increment),
            TIME_CONTROLS
                .into_iter()
                .map(|(minutes, increment)| {
                    choice(time_label(minutes, increment), move |setup| {
                        setup.minutes = minutes;
                        setup.increment = increment;
                    })
                })
                .collect(),
            window,
            cx,
        )
    }

    /// Which position a scenario starts from, and the way to bring one's own.
    fn scenario_rows(&self, window: &Window, cx: &mut Context<Self>) -> [AnyElement; 2] {
        let settings = Settings::get(cx);
        let current = self.setup.fen.as_deref().and_then(scenario::find);
        let label = current.map_or_else(|| tr("scenario-own"), |scenario| tr(scenario.title));
        let choices = scenario::ALL
            .iter()
            .map(|scenario| Choice {
                note: Some(tr(scenario.kind.label_id())),
                ..choice(tr(scenario.title), |setup| {
                    setup.fen = Some(scenario.fen.to_owned());
                    setup.side = scenario.side;
                })
            })
            .collect();
        let pick = self.select(field("scenario", "scenario"), label, choices, window, cx);
        let paste = ui::setting_row(tr("scenario-own"), Some(tr("scenario-own-hint")), &settings)
            .child(
                ui::action_button("paste", tr("paste-fen"), &settings)
                    .on_click(cx.listener(|this, _, _, cx| this.paste_fen(cx))),
            )
            .into_any_element();
        [pick, paste]
    }

    /// Everything to choose, top to bottom.
    fn rows(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let setup = self.setup.clone();
        let is_agent = setup.opponent.agent().is_some();
        let mut rows = Vec::new();
        if setup.mode == Mode::Scenario {
            rows.extend(self.scenario_rows(window, cx));
        }
        rows.push(self.select(
            Field {
                logo: logo_of(setup.opponent),
                ..field("opponent", "opponent")
            },
            setup.opponent.name().into(),
            Self::opponent_choices(),
            window,
            cx,
        ));
        rows.extend(self.missing_row(cx));
        rows.extend(self.strength_row(window, cx));
        rows.push(self.select(
            field("side", "you-play"),
            side_label(setup.side),
            vec![
                choice(side_label(Side::White), |setup| setup.side = Side::White),
                choice(side_label(Side::Black), |setup| setup.side = Side::Black),
                choice(side_label(Side::Random), |setup| setup.side = Side::Random),
            ],
            window,
            cx,
        ));
        if is_agent {
            rows.push(Self::effort_row(window, cx, setup.effort));
            let budget = Field {
                hint: Some(tr("token-budget-hint")),
                ..field("budget", "token-budget")
            };
            rows.push(
                self.select(
                    budget,
                    budget_label(setup.token_budget),
                    BUDGETS
                        .into_iter()
                        .map(|budget| {
                            choice(budget_label(budget), move |setup| {
                                setup.token_budget = budget;
                            })
                        })
                        .collect(),
                    window,
                    cx,
                ),
            );
            rows.push(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .py_1()
                    .child(tr("agent-instructions"))
                    .child(self.instructions.clone())
                    .into_any_element(),
            );
        }
        rows.push(self.mode_section(window, cx));
        rows
    }

    /// The rows that change with the mode: the clock and the help a game has.
    fn mode_rows(
        &self,
        setup: &Setup,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut rows = Vec::new();
        if setup.assisted() {
            rows.push(self.clock_select(setup, window, cx));
            // Until scenarios are ready, a casual game is where one's own position goes.
            if setup.mode == Mode::Casual {
                rows.push(Self::position_row(setup, cx));
            }
            rows.extend(Self::help_rows(setup, window, cx));
        } else {
            rows.push(Self::clock_grid(setup, cx));
        }
        rows.extend(self.stockfish_row(setup, cx));
        rows
    }

    /// Every mode's own rows in one cell, all but the picked one unseen: the cell is as tall
    /// as the tallest, so the dialog keeps its height as the mode changes, and a shorter mode
    /// leaves room above the buttons. Unseen rows aren't painted and take no clicks.
    fn mode_section(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        // The picked one last, on top, so that it has the hover.
        let mut modes: Vec<Mode> = MODES
            .into_iter()
            .filter(|mode| ready(*mode) && *mode != self.setup.mode)
            .collect();
        modes.push(self.setup.mode);
        let layers: Vec<_> = modes
            .into_iter()
            .map(|mode| {
                let mut setup = self.setup.clone();
                setup.set_mode(mode);
                div()
                    .id(("mode-rows", mode as usize))
                    .row_start(1)
                    .col_start(1)
                    .flex()
                    .flex_col()
                    .when(mode != self.setup.mode, Styled::invisible)
                    .children(self.mode_rows(&setup, window, cx))
            })
            .collect();
        div()
            .grid()
            .grid_cols(1)
            .children(layers)
            .into_any_element()
    }

    /// Win likelihood and tutoring against an engine run on Stockfish: the way to it, when
    /// they're on and it isn't there. Against Stockfish itself, the opponent's card says so.
    fn stockfish_row(&self, setup: &Setup, cx: &mut Context<Self>) -> Option<AnyElement> {
        let settings = Settings::get(cx);
        if setup.opponent == OpponentKind::Stockfish
            || OpponentKind::Stockfish.missing(&settings).is_none()
        {
            return None;
        }
        let need = NeedsStockfish::of(setup.shows_likelihood(), setup.stockfish_coaches())?;
        Some(
            installs::stockfish_notice(need, &self.installs, cx)
                .mt_1()
                .into_any_element(),
        )
    }

    /// The help a game can have, at the end of the dialog.
    fn help_rows(setup: &Setup, window: &mut Window, cx: &mut Context<Self>) -> [AnyElement; 2] {
        let tutoring = tr(if setup.opponent.agent().is_some() {
            "tutoring-hint-agent"
        } else {
            "tutoring-hint-engine"
        });
        [
            Self::switch(
                "tutoring",
                tr("tutoring"),
                tutoring,
                setup.annotate,
                |setup| setup.annotate = !setup.annotate,
                window,
                cx,
            ),
            Self::switch(
                "likelihood",
                tr("show-likelihood"),
                tr("show-likelihood-hint"),
                setup.live_likelihood,
                |setup| setup.live_likelihood = !setup.live_likelihood,
                window,
                cx,
            ),
        ]
    }

    fn position_row(setup: &Setup, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let custom = setup.fen.is_some();
        let hint = tr(if custom {
            "position-custom"
        } else {
            "position-standard"
        });
        ui::setting_row(tr("starting-position"), Some(hint), &settings)
            .child(
                div()
                    .flex()
                    .gap_1()
                    .when(custom, |row| {
                        row.child(
                            ui::action_button("standard", tr("position-standard"), &settings)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.setup.fen = None;
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        ui::action_button("paste", tr("paste-fen"), &settings)
                            .on_click(cx.listener(|this, _, _, cx| this.paste_fen(cx))),
                    ),
            )
            .into_any_element()
    }

    fn buttons(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let settings = Settings::get(cx);
        let ready = self.setup.opponent.missing(&settings).is_none();
        let start = if ready {
            ui::primary_button("start", tr("start-game"), &settings).on_click(cx.listener(
                |this, _, _, cx| {
                    let mut setup = this.setup.clone();
                    this.instructions
                        .read(cx)
                        .text()
                        .trim()
                        .clone_into(&mut setup.instructions);
                    // The default's name too, so that the game says who plays.
                    if let Some(kind) = setup.opponent.agent()
                        && let Some(name) = this
                            .catalog
                            .read(cx)
                            .model_name(kind, setup.model.as_deref())
                    {
                        setup.model_name = Some(name);
                    }
                    cx.emit(Event::Start(setup));
                },
            ))
        } else {
            ui::disabled_button("start", tr("start-game"), &settings)
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .pt_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(settings.colors.red)
                    .children(self.error.clone()),
            )
            .child(
                ui::action_button("cancel", tr("cancel"), &settings)
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Event::Cancel))),
            )
            .child(start)
    }
}

impl Render for NewGame {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::get(cx);
        let nav = self.mode_nav(cx);
        let rows = self.rows(window, cx);
        let buttons = self.buttons(cx);
        let form = div()
            .flex()
            .flex_col()
            .w(FORM_WIDTH)
            .p_5()
            .child(ui::title(tr("new-game"), &settings))
            .children(rows)
            .child(buttons);

        ui::fade_in(
            "new-game-shade",
            ui::modal("new-game", &settings).child(ui::rise_in(
                "new-game-in",
                ui::dialog(window, &settings)
                    .p_0()
                    .flex()
                    .flex_row()
                    .text_color(settings.colors.text)
                    .child(nav)
                    .child(ui::divider_line(Axis::Horizontal, window, &settings))
                    .child(form),
            )),
        )
    }
}

/// Opens the dialog as a view.
pub fn dialog(
    setup: Setup,
    catalog: Entity<Catalog>,
    installs: Entity<Installs>,
    cx: &mut App,
) -> Entity<NewGame> {
    cx.new(|cx| NewGame::new(setup, catalog, installs, cx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_read_naturally() {
        assert_eq!(time_label(0, 0), "No clock");
        assert_eq!(time_label(15, 10), "15 minutes + 10 s");
        assert_eq!(speed_label(Speed::Blitz), "Blitz");
        assert_eq!(budget_label(None), "No limit");
        assert_eq!(budget_label(Some(200_000)), "200k tokens");
        assert_eq!(budget_label(Some(1_000_000)), "1M tokens");
    }

    #[test]
    fn a_saved_level_finds_the_nearest_stop() {
        assert_eq!(nearest_stop(&ELO_LEVELS, 1320), 0);
        assert_eq!(nearest_stop(&ELO_LEVELS, 1800), 2);
        assert_eq!(nearest_stop(&ELO_LEVELS, 5000), ELO_LEVELS.len() - 1);
    }
}
