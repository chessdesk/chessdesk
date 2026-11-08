//! The welcome on first start: the few choices that make Chessdesk look and sound like one's
//! own. They're the settings pages' own rows, so they apply as they're picked.

use gpui::{App, Context, Entity, EventEmitter, Pixels, Subscription, Window, div, prelude::*, px};
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess};

use crate::board;
use crate::i18n::tr;
use crate::settings::Settings;
use crate::settings_view::SettingsView;
use crate::ui;

const DIALOG_WIDTH: Pixels = px(720.);
const PREVIEW_SIZE: Pixels = px(224.);
/// A few moves into the Italian Game: every kind of piece shows, on both colours of square.
const PREVIEW_FEN: &str = "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4";

/// The welcome is done with.
pub struct Done;

pub struct Onboarding {
    choices: Entity<SettingsView>,
    /// Its rows render here, so its changes have to redraw this view.
    _redraw: Subscription,
}

impl EventEmitter<Done> for Onboarding {}

impl Render for Onboarding {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::get(cx);
        let rows = self
            .choices
            .update(cx, |choices, cx| choices.render_welcome(window, cx));
        // Not before the language's font is here: without it, its letters may not show.
        let start = if settings.missing_font.is_some() {
            ui::disabled_button("welcome-done", tr("welcome-start"), &settings)
        } else {
            ui::primary_button("welcome-done", tr("welcome-start"), &settings)
                .on_click(cx.listener(|_, _, _, cx| cx.emit(Done)))
        };
        ui::fade_in(
            "welcome-shade",
            ui::modal("welcome", &settings).child(ui::rise_in(
                "welcome-in",
                ui::dialog(window, &settings)
                    .w(DIALOG_WIDTH)
                    .p_4()
                    .text_color(settings.colors.text)
                    .child(ui::title(tr("welcome-title"), &settings))
                    .child(
                        div()
                            .pb_2()
                            .text_color(settings.colors.muted)
                            .child(tr("welcome-intro")),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_6()
                            .child(div().flex().flex_col().flex_1().min_w_0().children(rows))
                            .child(board::preview(&preview_position(), PREVIEW_SIZE, cx)),
                    )
                    .child(div().flex().justify_end().pt_4().child(start)),
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

/// Opens the welcome, offering what `choices` holds.
pub fn dialog(choices: Entity<SettingsView>, cx: &mut App) -> Entity<Onboarding> {
    cx.new(|cx| {
        let redraw = cx.observe(&choices, |_, _, cx| cx.notify());
        Onboarding {
            choices,
            _redraw: redraw,
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
