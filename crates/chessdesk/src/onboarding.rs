//! The welcome on first start: the few choices that make Chessdesk look and sound like one's
//! own. They're the settings pages' own rows, so they apply as they're picked.

use gpui::{App, Context, Entity, EventEmitter, Pixels, Subscription, Window, div, prelude::*, px};

use crate::i18n::tr;
use crate::settings::Settings;
use crate::settings_view::SettingsView;
use crate::ui;

const DIALOG_WIDTH: Pixels = px(460.);

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
        let start = ui::primary_button("welcome-done", tr("welcome-start"), &settings)
            .on_click(cx.listener(|_, _, _, cx| cx.emit(Done)));
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
                    .children(rows)
                    .child(div().flex().justify_end().pt_2().child(start)),
            )),
        )
    }
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
