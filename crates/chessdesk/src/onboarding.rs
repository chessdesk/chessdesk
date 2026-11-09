//! The welcome on first start. First the few choices that make Chessdesk look and sound like
//! one's own, the settings pages' own rows, which apply as they're picked. Then the opponents:
//! the agents one uses and Stockfish, set up before the welcome closes. Last, if an agent was
//! set up, the model each one reviews games and gives hints with.

use std::collections::VecDeque;

use chessdesk_players::acp::AgentKind;
use chessdesk_players::adapters::{self, Readiness};
use chessdesk_players::download;
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, Pixels, Subscription, Window, div, prelude::*,
    px,
};
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess};

use crate::board;
use crate::i18n::{tr, tr_with};
use crate::installs::{Engine, Installs};
use crate::model_picker::{self, ModelPicker, Picked};
use crate::models::Catalog;
use crate::settings::Settings;
use crate::settings_view::SettingsView;
use crate::setup::OpponentKind;
use crate::ui::{self, Logo};

const LOOKS_WIDTH: Pixels = px(720.);
const OPPONENTS_WIDTH: Pixels = px(480.);
const PREVIEW_SIZE: Pixels = px(224.);
/// A few moves into the Italian Game: every kind of piece shows, on both colours of square.
const PREVIEW_FEN: &str = "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4";
/// How faint the switches are while what they chose is being set up.
const SETTLED_OPACITY: f32 = 0.5;

/// The welcome is done with, and everything picked is set up.
pub struct Done {
    /// The first agent picked, to play in the first game.
    pub agent: Option<AgentKind>,
}

/// A choice couldn't be saved.
pub struct Failed(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Looks,
    Opponents,
    /// Fetching what was picked; the dialog stays until it's all here.
    SettingUp,
    /// The model each agent set up reviews and gives hints with. Stockfish may still be on its
    /// way meanwhile.
    Models,
    Finished,
}

pub struct Onboarding {
    choices: Entity<SettingsView>,
    installs: Entity<Installs>,
    step: Step,
    /// The agents picked, in the list's order.
    agents: Vec<AgentKind>,
    /// Adapters still to install: one at a time.
    queue: VecDeque<AgentKind>,
    /// What each agent offers, for the last step.
    catalog: Entity<Catalog>,
    /// The last step's pickers, one for each agent picked that's ready, and what keeps their
    /// picks.
    pickers: Vec<(AgentKind, Entity<ModelPicker>, Subscription)>,
    /// Redraws for the rows of `choices`, which render here, and the next step of the setup
    /// as each install ends.
    _subscriptions: [Subscription; 2],
}

impl EventEmitter<Done> for Onboarding {}
impl EventEmitter<Failed> for Onboarding {}

/// Whether Stockfish is missing and Chessdesk can fetch it.
fn stockfish_wanted(settings: &Settings) -> bool {
    download::can_download_stockfish() && OpponentKind::Stockfish.missing(settings).is_some()
}

impl Onboarding {
    fn finish_setup(&mut self, cx: &mut Context<Self>) {
        self.step = Step::SettingUp;
        let folder = Settings::adapters_dir().unwrap_or_default();
        self.queue = self
            .agents
            .iter()
            .copied()
            .filter(|kind| adapters::readiness(*kind, &folder) == Readiness::NoAdapter)
            .collect();
        // Not a choice: analysis, hints and the likelihood all need it.
        if stockfish_wanted(&Settings::get(cx)) {
            self.installs.update(cx, |installs, cx| {
                installs.fetch_for(OpponentKind::Stockfish, cx);
            });
        }
        self.advance(cx);
    }

    /// Starts the next adapter once the last is in. Then on to the models, if an agent is
    /// ready to use, or else the end once Stockfish is here. A failed install is reported by
    /// the window and doesn't hold up the rest.
    fn advance(&mut self, cx: &mut Context<Self>) {
        if self.step != Step::SettingUp {
            return;
        }
        if self.installs.read(cx).installing().is_some() {
            return;
        }
        if let Some(kind) = self.queue.pop_front() {
            self.installs
                .update(cx, |installs, cx| installs.install_adapter(kind, cx));
        } else if self.show_models(cx) {
            self.step = Step::Models;
        } else {
            self.finish(cx);
        }
        cx.notify();
    }

    /// Makes a picker for each agent picked that's ready to use. Returns whether there's one.
    fn show_models(&mut self, cx: &mut Context<Self>) -> bool {
        let folder = Settings::adapters_dir().unwrap_or_default();
        let ready: Vec<_> = self
            .agents
            .iter()
            .copied()
            .filter(|kind| adapters::readiness(*kind, &folder) == Readiness::Ready)
            .collect();
        for kind in ready {
            let chosen = Settings::get(cx).analysis_model(kind);
            let catalog = self.catalog.clone();
            let picker = cx.new(|cx| ModelPicker::new(kind, chosen, catalog, cx));
            let store = cx.subscribe(&picker, move |_, _, Picked(model): &Picked, cx| {
                if let Err(error) = Settings::store_analysis_model(kind, model.as_deref()) {
                    cx.emit(Failed(
                        tr_with!("error-save-setting", error = error).to_string(),
                    ));
                }
            });
            self.pickers.push((kind, picker, store));
        }
        !self.pickers.is_empty()
    }

    /// Ends the welcome, once Stockfish is here: the dialog waits for it.
    fn finish(&mut self, cx: &mut Context<Self>) {
        if self.installs.read(cx).downloading().is_some() {
            return;
        }
        self.step = Step::Finished;
        cx.emit(Done {
            agent: self.agents.first().copied(),
        });
    }

    fn models(&self, cx: &mut Context<Self>) -> AnyElement {
        let rows = self
            .pickers
            .iter()
            .map(|(kind, picker, _)| model_picker::row(*kind, picker, cx));
        div().flex().flex_col().children(rows).into_any_element()
    }

    fn looks(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let rows = self
            .choices
            .update(cx, |choices, cx| choices.render_welcome(window, cx));
        div()
            .flex()
            .gap_6()
            .child(div().flex().flex_col().flex_1().min_w_0().children(rows))
            .child(board::preview(&preview_position(), PREVIEW_SIZE, cx))
            .into_any_element()
    }

    /// A row with a switch, which only switches before the setup starts, and only if `flip`
    /// says how.
    fn switch_row(
        &self,
        row: gpui::Div,
        id: impl Into<gpui::ElementId>,
        on: bool,
        flip: Option<impl Fn(&mut Self) + 'static>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let settings = Settings::get(cx);
        let toggle = ui::toggle(id, on, window, cx, &settings);
        let flip = flip.filter(|_| self.step == Step::Opponents);
        let toggle = if let Some(flip) = flip {
            toggle
                .on_click(cx.listener(move |this, _, _, cx| {
                    flip(this);
                    cx.notify();
                }))
                .into_any_element()
        } else {
            toggle
                .cursor_default()
                .opacity(SETTLED_OPACITY)
                .into_any_element()
        };
        row.child(toggle).into_any_element()
    }

    fn opponents(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let folder = Settings::adapters_dir().unwrap_or_default();
        let installing = self.installs.read(cx).installing();
        let mut rows: Vec<AnyElement> = AgentKind::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| {
                let hint = match adapters::readiness(kind, &folder) {
                    _ if installing == Some(kind) => tr("installing-adapter"),
                    Readiness::Ready => tr("installed"),
                    Readiness::NoAdapter => tr("adapter-to-install"),
                    Readiness::NoNode => tr("needs-node"),
                    Readiness::NoProgram(program) => {
                        tr_with!("program-not-found", program = program)
                    }
                };
                let row =
                    ui::logo_setting_row(Logo::of_agent(kind), kind.name(), Some(hint), &settings);
                let on = self.agents.contains(&kind);
                self.switch_row(
                    row,
                    ("agent", index),
                    on,
                    Some(move |this: &mut Self| {
                        if on {
                            this.agents.retain(|picked| *picked != kind);
                        } else {
                            // In the list's order, so the first is the same however picked.
                            this.agents.push(kind);
                            this.agents.sort_by_key(|picked| {
                                AgentKind::ALL.iter().position(|known| known == picked)
                            });
                        }
                    }),
                    window,
                    cx,
                )
            })
            .collect();
        rows.push(ui::separator(window, &settings).my_2().into_any_element());
        rows.push(self.stockfish_row(window, cx));
        div().flex().flex_col().children(rows).into_any_element()
    }

    fn stockfish_row(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let fetching = self.installs.read(cx).downloading();
        let hint = match fetching {
            Some((Engine::Stockfish, done)) => {
                tr_with!("downloading", percent = (done * 100.).round() as u32)
            }
            _ if OpponentKind::Stockfish.missing(&settings).is_none() => tr("installed"),
            _ if download::can_download_stockfish() => tr("stockfish-offer"),
            _ => tr("stockfish-missing"),
        };
        let row = ui::logo_setting_row(Logo::Stockfish, "Stockfish", Some(hint), &settings);
        if !stockfish_wanted(&settings) && fetching.is_none() {
            return row.into_any_element();
        }
        // On and fixed: it's fetched with the rest.
        self.switch_row(row, "stockfish", true, None::<fn(&mut Self)>, window, cx)
    }

    fn buttons(&self, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let buttons = div().flex().justify_end().gap_2().pt_4();
        match self.step {
            // Not before the language's font is here: without it, its letters may not show.
            Step::Looks if settings.missing_font.is_some() => buttons.child(ui::disabled_button(
                "welcome-next",
                tr("continue"),
                &settings,
            )),
            Step::Looks => buttons.child(
                ui::primary_button("welcome-next", tr("continue"), &settings).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.step = Step::Opponents;
                        cx.notify();
                    }),
                ),
            ),
            Step::Opponents => buttons
                .child(
                    ui::action_button("welcome-back", tr("back"), &settings).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.step = Step::Looks;
                            cx.notify();
                        },
                    )),
                )
                .child(
                    ui::primary_button("welcome-done", tr("welcome-finish"), &settings)
                        .on_click(cx.listener(|this, _, _, cx| this.finish_setup(cx))),
                ),
            // Stockfish may still be on its way: the welcome closes once it's here.
            Step::Models if self.installs.read(cx).downloading().is_none() => buttons.child(
                ui::primary_button("welcome-done", tr("welcome-finish"), &settings)
                    .on_click(cx.listener(|this, _, _, cx| this.finish(cx))),
            ),
            Step::SettingUp | Step::Models | Step::Finished => buttons.child(ui::busy_button(
                "welcome-done",
                tr("welcome-finish"),
                &settings,
            )),
        }
        .into_any_element()
    }
}

impl Render for Onboarding {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::get(cx);
        let (title, intro, width, content) = if self.step == Step::Looks {
            (
                tr("welcome-title"),
                tr("welcome-intro"),
                LOOKS_WIDTH,
                self.looks(window, cx),
            )
        } else if self.step == Step::Models {
            (
                tr("welcome-models"),
                tr("welcome-models-intro"),
                OPPONENTS_WIDTH,
                self.models(cx),
            )
        } else {
            (
                tr("welcome-opponents"),
                tr("welcome-opponents-intro"),
                OPPONENTS_WIDTH,
                self.opponents(window, cx),
            )
        };
        let buttons = self.buttons(cx);
        ui::fade_in(
            "welcome-shade",
            ui::modal("welcome", &settings).child(ui::rise_in(
                "welcome-in",
                ui::dialog(window, &settings)
                    .w(width)
                    .p_4()
                    .text_color(settings.colors.text)
                    .child(ui::title(title, &settings))
                    .child(div().pb_2().text_color(settings.colors.muted).child(intro))
                    .child(content)
                    .child(buttons),
            )),
        )
    }
}

/// The position the board and pieces are shown in.
fn preview_position() -> Chess {
    PREVIEW_FEN
        .parse::<Fen>()
        .ok()
        .and_then(|fen| fen.into_position(CastlingMode::Standard).ok())
        .unwrap_or_default()
}

/// Opens the welcome, offering what `choices` holds, setting up opponents with `installs`
/// and offering the models `catalog` knows of.
pub fn dialog(
    choices: Entity<SettingsView>,
    installs: Entity<Installs>,
    catalog: Entity<Catalog>,
    cx: &mut App,
) -> Entity<Onboarding> {
    cx.new(|cx| {
        let subscriptions = [
            cx.observe(&choices, |_: &mut Onboarding, _, cx| cx.notify()),
            cx.observe(&installs, |this, _, cx| {
                this.advance(cx);
                cx.notify();
            }),
        ];
        Onboarding {
            choices,
            installs,
            step: Step::Looks,
            agents: Vec::new(),
            queue: VecDeque::new(),
            catalog,
            pickers: Vec::new(),
            _subscriptions: subscriptions,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_position_reads() {
        assert_ne!(preview_position(), Chess::default());
    }
}
