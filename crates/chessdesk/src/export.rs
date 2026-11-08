//! The export dialog: which format a game leaves in, and where it goes.

use std::path::PathBuf;

use chessdesk_core::Game;
use gpui::{App, Context, Entity, EventEmitter, Pixels, SharedString, Window, div, prelude::*, px};

use crate::i18n::{tr, tr_with};
use crate::library::{self, ExportFormat};
use crate::settings::Settings;
use crate::ui;

const DIALOG_WIDTH: Pixels = px(460.);

/// What the dialog tells the app.
pub enum Event {
    Done,
    Failed(String),
}

pub struct ExportDialog {
    game: Game,
    format: ExportFormat,
    path: PathBuf,
}

impl EventEmitter<Event> for ExportDialog {}

impl ExportDialog {
    fn pick_format(&mut self, format: ExportFormat, cx: &mut Context<Self>) {
        self.format = format;
        self.path.set_extension(format.extension());
        cx.notify();
    }

    fn choose_location(&mut self, cx: &mut Context<Self>) {
        let folder = self.path.parent().map(PathBuf::from).unwrap_or_default();
        let name = self
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        let chosen = cx.prompt_for_new_path(&folder, name.as_deref());
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = chosen.await {
                this.update(cx, |this, cx| {
                    this.path = path;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn export(&mut self, cx: &mut Context<Self>) {
        match library::export(&self.game, self.format, &self.path) {
            Ok(()) => cx.emit(Event::Done),
            Err(error) => cx.emit(Event::Failed(
                tr_with!("error-export-game", error = error).to_string(),
            )),
        }
    }
}

impl Render for ExportDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::get(cx);
        let formats = ExportFormat::ALL
            .into_iter()
            .enumerate()
            .map(|(index, format)| {
                ui::list_row(("format", index), format == self.format, &settings)
                    .text_color(settings.colors.text)
                    .child(format.label())
                    .on_click(cx.listener(move |this, _, _, cx| this.pick_format(format, cx)))
            });
        let location: SharedString = self.path.display().to_string().into();
        ui::fade_in(
            "export-shade",
            ui::modal("export", &settings)
                .on_mouse_down(gpui::MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .child(ui::rise_in(
                    "export-in",
                    ui::dialog(window, &settings)
                        .w(DIALOG_WIDTH)
                        .p_4()
                        .gap_2()
                        .text_color(settings.colors.text)
                        .child(ui::title(tr("export-game"), &settings))
                        .child(div().flex().flex_col().gap_0p5().children(formats))
                        .child(
                            ui::setting_row(tr("export-location"), Some(location), &settings)
                                .child(
                                    ui::action_button("choose", tr("choose"), &settings).on_click(
                                        cx.listener(|this, _, _, cx| this.choose_location(cx)),
                                    ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap_2()
                                .pt_2()
                                .child(
                                    ui::action_button("cancel", tr("cancel"), &settings)
                                        .on_click(cx.listener(|_, _, _, cx| cx.emit(Event::Done))),
                                )
                                .child(
                                    ui::action_button("export-now", tr("export"), &settings)
                                        .on_click(cx.listener(|this, _, _, cx| this.export(cx))),
                                ),
                        ),
                )),
        )
    }
}

/// Opens the dialog for a game, suggesting `name` in the documents folder.
pub fn dialog(game: Game, name: &str, cx: &mut App) -> Entity<ExportDialog> {
    let format = ExportFormat::Pgn;
    let folder = dirs::document_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_default();
    let path = folder.join(name).with_extension(format.extension());
    cx.new(|_| ExportDialog { game, format, path })
}
