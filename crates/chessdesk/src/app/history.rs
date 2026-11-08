//! The sidebar of games: every saved game, newest first. Clicking one shows it without
//! touching whatever else is running; a dot marks the ones still going.

use std::path::PathBuf;

use chessdesk_core::Mark;
use gpui::{
    AnyElement, Axis, Context, Div, Entity, Focusable, MouseButton, MouseDownEvent, Subscription,
    Window, anchored, deferred, div, prelude::*, px,
};

use super::{Chessdesk, Dragging};
use crate::i18n::{tr, tr_with};
use crate::library::{Entry, Standing};
use crate::settings::Settings;
use crate::text_area::{Finished, TextArea};
use crate::ui;

/// The dot beside a game that isn't over.
const DOT_SIZE: gpui::Pixels = px(5.);
const CONFIRM_WIDTH: gpui::Pixels = px(360.);

/// A game's title being edited, in its row.
pub(super) struct Renaming {
    path: PathBuf,
    title: String,
    field: Entity<TextArea>,
    _finished: Subscription,
    _left: Subscription,
}

/// The line under a game's title: when, how it went, and how long it took.
fn render_detail(entry: &Entry, settings: &Settings) -> Div {
    let colors = &settings.colors;
    let result = entry.result.as_ref().map(|result| {
        let color = match result {
            Standing::Won => colors.mark(Mark::Green),
            Standing::Lost => colors.mark(Mark::Red),
            Standing::Draw | Standing::Others(_) => colors.muted,
        };
        div().flex_none().text_color(color).child(result.label())
    });
    let moves = tr_with!("move-count", count = entry.moves);
    div()
        .flex()
        .gap_1p5()
        .min_w_0()
        .text_size(settings.ui_font_size * 0.92)
        .text_color(colors.muted)
        .child(div().flex_none().child(entry.date.clone()))
        .children(result)
        .child(div().truncate().child(moves))
}

impl Chessdesk {
    /// Turns a game's title into a field, all of it selected for typing over.
    fn start_rename(&mut self, entry: &Entry, window: &mut Window, cx: &mut Context<Self>) {
        let title = entry.title.clone();
        let field = cx.new(|cx| TextArea::one_line(title, cx));
        let finished = cx.subscribe_in(&field, window, |this, _, finished, window, cx| {
            this.finish_rename(matches!(finished, Finished::Accepted), cx);
            this.focus.focus(window);
        });
        let focus = field.focus_handle(cx);
        // Clicking elsewhere keeps what was typed, as a file manager does.
        let left = cx.on_blur(&focus, window, |this, _, cx| this.finish_rename(true, cx));
        focus.focus(window);
        self.renaming = Some(Renaming {
            path: entry.path.clone(),
            title: entry.title.clone(),
            field,
            _finished: finished,
            _left: left,
        });
        cx.notify();
    }

    fn finish_rename(&mut self, keep: bool, cx: &mut Context<Self>) {
        let Some(renaming) = self.renaming.take() else {
            return;
        };
        let text = renaming.field.read(cx).text();
        // A pasted line break has no place in a title.
        let name = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if keep && name != renaming.title {
            self.rename_saved(&renaming.path, &name, cx);
        }
        cx.notify();
    }

    pub(super) fn render_history(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let active = self.session().path.clone();
        let rows = self.history.iter().enumerate().map(|(index, entry)| {
            let path = entry.path.clone();
            let listed = entry.clone();
            let selected = active.as_ref() == Some(&entry.path) && self.settings_page.is_none();
            let renaming = self
                .renaming
                .as_ref()
                .filter(|renaming| renaming.path == entry.path);
            let is_renaming = renaming.is_some();
            let title = match renaming {
                Some(renaming) => renaming.field.clone().into_any_element(),
                None => div()
                    .truncate()
                    .child(entry.title.clone())
                    .into_any_element(),
            };
            ui::list_row(("game", index), selected, &settings)
                .h_auto()
                .flex_none()
                .gap_2()
                .py_1()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(title)
                        .child(render_detail(entry, &settings)),
                )
                .when(entry.unfinished(), |row| {
                    row.child(
                        div()
                            .flex_none()
                            .size(DOT_SIZE)
                            .rounded_full()
                            .bg(settings.colors.caret),
                    )
                })
                .when(!is_renaming, |row| {
                    row.on_click(cx.listener(move |this, _, _, cx| this.open(&path, cx)))
                })
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        this.context_menu = Some((listed.clone(), event.position));
                        cx.notify();
                    }),
                )
        });
        let empty = self.history.is_empty().then(|| {
            ui::empty_panel(&settings)
                .p_2()
                .text_color(settings.colors.muted)
                .child(tr("history-empty"))
        });
        let list = div()
            .id("history")
            .flex()
            .flex_col()
            .size_full()
            .p_1()
            .gap_0p5()
            .overflow_y_scroll()
            .children(rows)
            .children(empty);
        self.render_side(list.into_any_element(), window, cx)
    }

    /// The left sidebar's frame: what the history and the settings' pages go into.
    pub(super) fn render_side(
        &self,
        content: AnyElement,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex()
            .flex_none()
            .h_full()
            .w(self.history_width)
            .child(div().flex_1().min_w_0().h_full().child(content))
            .child(Self::render_divider(
                "history-divider",
                Axis::Horizontal,
                |_, _| Dragging::History,
                window,
                cx,
            ))
            .into_any_element()
    }

    /// Asks before a game goes. A running game stops with it, so this is worth a question.
    pub(super) fn render_confirm_delete(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let entry = self.confirm_delete.as_ref()?;
        let settings = Settings::get(cx);
        let hint = tr(if entry.unfinished() {
            "delete-hint-running"
        } else {
            "delete-hint"
        });
        let buttons = div()
            .flex()
            .justify_end()
            .gap_2()
            .pt_2()
            .child(
                ui::action_button("keep", tr("cancel"), &settings).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.confirm_delete = None;
                        cx.notify();
                    },
                )),
            )
            .child(
                ui::action_button("delete", tr("delete"), &settings).on_click(cx.listener(
                    |this, _, _, cx| {
                        if let Some(entry) = this.confirm_delete.take() {
                            this.delete_saved(&entry.path, cx);
                        }
                    },
                )),
            );
        Some(
            ui::fade_in(
                "confirm-delete-shade",
                ui::modal("confirm-delete", &settings).child(ui::rise_in(
                    "confirm-delete-in",
                    ui::dialog(window, &settings)
                        .w(CONFIRM_WIDTH)
                        .p_4()
                        .text_color(settings.colors.text)
                        .child(ui::title(
                            tr_with!("delete-title", game = entry.title.as_str()),
                            &settings,
                        ))
                        .child(div().text_color(settings.colors.muted).child(hint))
                        .child(buttons),
                )),
            )
            .into_any_element(),
        )
    }

    /// What can be done with the game whose row was right-clicked.
    pub(super) fn render_context_menu(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let (entry, position) = self.context_menu.clone()?;
        let settings = Settings::get(cx);
        let listed = entry.clone();
        let rename = ui::menu_item("menu-rename", tr("rename"), None, &settings).on_click(
            cx.listener(move |this, _, window, cx| {
                this.context_menu = None;
                this.start_rename(&listed, window, cx);
            }),
        );
        let resign = entry.unfinished().then(|| {
            let path = entry.path.clone();
            ui::menu_item("menu-resign", tr("resign"), None, &settings).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.context_menu = None;
                    this.resign_saved(&path, cx);
                },
            ))
        });
        let path = entry.path.clone();
        let export = ui::menu_item("menu-export", tr("export"), None, &settings).on_click(
            cx.listener(move |this, _, _, cx| {
                this.context_menu = None;
                this.export_saved(&path, cx);
            }),
        );
        let delete = ui::menu_item("menu-delete", tr("delete"), None, &settings).on_click(
            cx.listener(move |this, _, _, cx| {
                this.confirm_delete = this.context_menu.take().map(|(entry, _)| entry);
                cx.notify();
            }),
        );
        Some(
            deferred(
                anchored().position(position).child(ui::drop_in(
                    "context-menu-in",
                    ui::menu(window, &settings)
                        .child(rename)
                        .children(resign)
                        .child(export)
                        .child(delete),
                )),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }
}
