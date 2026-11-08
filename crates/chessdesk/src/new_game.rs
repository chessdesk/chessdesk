//! The dialog shown before a game: who plays, how strong and how fast, and what an
//! agent may spend.

use std::rc::Rc;

use chessdesk_core::Game;
use chessdesk_players::acp::Effort;
use chessdesk_players::download::MAIA_LEVELS;
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, MouseButton, Pixels, SharedString, Window, div,
    prelude::*, px,
};

use crate::i18n::{tr, tr_with};
use crate::settings::Settings;
use crate::settings_view::SettingsPage;
use crate::setup::{OpponentKind, Setup, Side, maia_strength, title_case};
use crate::text_area::TextArea;
use crate::ui::{self, Logo};

const DIALOG_WIDTH: Pixels = px(460.);
/// `UCI_Elo` runs from 1320 to 3190. Past the last stop it plays at full strength.
const ELO_LEVELS: [u32; 10] = [1320, 1500, 1700, 1900, 2100, 2300, 2500, 2700, 2900, 3100];
/// Minutes each and seconds per move, for the clock's list.
const TIME_CONTROLS: [(u32, u32); 5] = [(0, 0), (5, 0), (10, 0), (15, 10), (30, 0)];
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
    pick: Rc<dyn Fn(&mut Setup)>,
}

fn choice(label: impl Into<SharedString>, pick: impl Fn(&mut Setup) + 'static) -> Choice {
    Choice {
        label: label.into(),
        logo: None,
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
        OpponentKind::ClaudeCode => Some(Logo::ClaudeCode),
        OpponentKind::Codex => Some(Logo::Codex),
        _ => None,
    }
}

pub struct NewGame {
    setup: Setup,
    instructions: Entity<TextArea>,
    open: Option<&'static str>,
    error: Option<SharedString>,
}

impl EventEmitter<Event> for NewGame {}

fn side_label(side: Side) -> SharedString {
    tr(match side {
        Side::White => "side-white",
        Side::Black => "side-black",
        Side::Random => "side-random",
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

fn effort_label(effort: Effort) -> SharedString {
    tr(match effort {
        Effort::Low => "effort-low",
        Effort::Medium => "effort-medium",
        Effort::High => "effort-high",
    })
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
    pub fn new(setup: Setup, cx: &mut Context<Self>) -> Self {
        let text = setup.instructions.clone();
        Self {
            setup,
            instructions: cx.new(|cx| TextArea::new(text, tr("instructions-example"), cx)),
            open: None,
            error: None,
        }
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
                cx.notify();
            }),
        );
        let list = is_open.then(|| {
            let rows = choices.into_iter().enumerate().map(|(index, choice)| {
                let pick = choice.pick.clone();
                ui::list_row(("choice", index), false, &settings)
                    .gap_1p5()
                    .text_color(settings.colors.text)
                    .children(
                        choice
                            .logo
                            .map(|mark| ui::logo(mark, settings.ui_font_size * ui::LOGO_SCALE)),
                    )
                    .child(div().truncate().child(choice.label))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        pick(&mut this.setup);
                        this.open = None;
                        this.error = None;
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
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = Settings::get(cx);
        ui::setting_row(label, Some(hint), &settings)
            .child(ui::toggle(id, on, window, &settings).on_click(cx.listener(
                move |this, _, _, cx| {
                    flip(&mut this.setup);
                    cx.notify();
                },
            )))
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

    fn opponent_choices(settings: &Settings) -> Vec<Choice> {
        OpponentKind::ALL
            .into_iter()
            .map(|kind| {
                let label = if kind.missing(settings).is_some() {
                    tr_with!("not-installed", name = kind.name())
                } else {
                    kind.name().into()
                };
                Choice {
                    logo: logo_of(kind),
                    ..choice(label, move |setup| {
                        setup.opponent = kind;
                        setup.model = kind.models().first().map(ToString::to_string);
                    })
                }
            })
            .collect()
    }

    /// What's needed before this opponent can play, with the way to get it if there is one.
    fn missing_row(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let settings = Settings::get(cx);
        let opponent = self.setup.opponent;
        let missing = opponent.missing(&settings)?;
        let (label, page) = if opponent.agent().is_some() {
            (tr("install-in-settings"), SettingsPage::Agents)
        } else {
            (tr("download-in-settings"), SettingsPage::Engines)
        };
        let link = opponent.downloadable().then(|| {
            ui::text_link("download", label, &settings)
                .on_click(cx.listener(move |_, _, _, cx| cx.emit(Event::OpenSettings(page))))
        });
        Some(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_4()
                .pb_1()
                .child(div().text_color(settings.colors.callout).child(missing))
                .children(link)
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
            kind => {
                let models = kind.models();
                if models.is_empty() {
                    return None;
                }
                let choices = models
                    .iter()
                    .map(|model| {
                        choice(title_case(model), move |setup| {
                            setup.model = Some((*model).to_owned());
                        })
                    })
                    .collect();
                let current = setup.model.as_deref().unwrap_or(models[0]);
                let model = Field {
                    hint: Some(tr("model-hint")),
                    ..field("model", "model")
                };
                Some(self.select(model, title_case(current).into(), choices, window, cx))
            }
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

    /// Everything to choose, top to bottom.
    fn rows(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        let setup = self.setup.clone();
        let is_agent = setup.opponent.agent().is_some();
        let mut rows = vec![self.select(
            Field {
                logo: logo_of(setup.opponent),
                ..field("opponent", "opponent")
            },
            setup.opponent.name().into(),
            Self::opponent_choices(&settings),
            window,
            cx,
        )];
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
        rows.push(
            self.select(
                field("clock", "time-control"),
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
            ),
        );
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
        rows.extend(self.closing_rows(window, cx));
        rows
    }

    /// The rows every opponent has, at the end of the dialog.
    fn closing_rows(&self, window: &Window, cx: &mut Context<Self>) -> [AnyElement; 3] {
        let setup = &self.setup;
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
            self.position_row(cx),
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

    fn position_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let custom = self.setup.fen.is_some();
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
        let rows = self.rows(window, cx);
        let buttons = self.buttons(cx);

        ui::fade_in(
            "new-game-shade",
            ui::modal("new-game", &settings).child(ui::rise_in(
                "new-game-in",
                ui::dialog(window, &settings)
                    .w(DIALOG_WIDTH)
                    .p_4()
                    .text_color(settings.colors.text)
                    .child(ui::title(tr("new-game"), &settings))
                    .children(rows)
                    .child(buttons),
            )),
        )
    }
}

/// Opens the dialog as a view.
pub fn dialog(setup: Setup, cx: &mut App) -> Entity<NewGame> {
    cx.new(|cx| NewGame::new(setup, cx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_read_naturally() {
        assert_eq!(time_label(0, 0), "No clock");
        assert_eq!(time_label(15, 10), "15 minutes + 10 s");
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
