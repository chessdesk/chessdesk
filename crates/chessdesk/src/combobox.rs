//! A select to type in, for long lists: the text narrows the list, and with no match stands
//! for itself. While it exists it is open; its owner shows a plain select otherwise.

use std::ops::Range;

use gpui::{
    AnyElement, Context, Entity, EventEmitter, Focusable, ScrollStrategy, SharedString,
    Subscription, UniformListScrollHandle, Window, div, prelude::*, uniform_list,
};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::settings::Settings;
use crate::text_area::{Changed, Finished, Step, TextArea};
use crate::ui;

/// The most rows the list shows before it scrolls.
const MAX_VISIBLE_ROWS: usize = 10;

pub enum Event {
    /// One of the choices, by its place in the list given.
    Picked(usize),
    /// Text that matches no choice, standing for itself.
    Typed(String),
    Dismissed,
}

pub struct Combobox {
    input: Entity<TextArea>,
    labels: Vec<SharedString>,
    /// The places in `labels` of the choices that match what's typed, best first.
    matches: Vec<usize>,
    /// Which of `matches` Enter picks.
    selected: usize,
    matcher: Matcher,
    scroll: UniformListScrollHandle,
    /// Whether the pointer is over the list, which lies outside the field's box: a press
    /// there is no press outside.
    over_list: bool,
    _subscriptions: [Subscription; 4],
}

impl EventEmitter<Event> for Combobox {}

/// The places of the `labels` that match `query`, best first; all of them, in order, for an
/// empty one.
fn filter(labels: &[SharedString], query: &str, matcher: &mut Matcher) -> Vec<usize> {
    if query.is_empty() {
        return (0..labels.len()).collect();
    }
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut buffer = Vec::new();
    let mut scored: Vec<(usize, u32)> = labels
        .iter()
        .enumerate()
        .filter_map(|(place, label)| {
            let score = pattern.score(Utf32Str::new(label, &mut buffer), matcher)?;
            Some((place, score))
        })
        .collect();
    // Stable, so that equal matches keep the list's order.
    scored.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    scored.into_iter().map(|(place, _)| place).collect()
}

impl Combobox {
    /// Opens on the whole list, `current` selected in it and in the field: typing replaces it.
    pub fn new(
        current: usize,
        labels: Vec<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let text = labels
            .get(current)
            .map(ToString::to_string)
            .unwrap_or_default();
        let input = cx.new(|cx| TextArea::one_line(text, cx).control_height());
        let subscriptions = [
            cx.subscribe(&input, |this, _, _: &Changed, cx| this.refilter(cx)),
            cx.subscribe(&input, |this, _, step: &Step, cx| this.step(step.0, cx)),
            cx.subscribe(&input, |this, _, finished: &Finished, cx| match finished {
                Finished::Accepted => this.confirm(cx),
                Finished::Cancelled => cx.emit(Event::Dismissed),
            }),
            // Clicks on the list leave the focus in the field, so this is a click elsewhere.
            cx.on_blur(&input.focus_handle(cx), window, |_, _, cx| {
                cx.emit(Event::Dismissed);
            }),
        ];
        input.focus_handle(cx).focus(window);
        let selected = current.min(labels.len().saturating_sub(1));
        let scroll = UniformListScrollHandle::new();
        scroll.scroll_to_item(selected, ScrollStrategy::Center);
        Self {
            input,
            matches: (0..labels.len()).collect(),
            labels,
            selected,
            matcher: Matcher::new(Config::DEFAULT),
            scroll,
            over_list: false,
            _subscriptions: subscriptions,
        }
    }

    fn refilter(&mut self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).text().trim().to_owned();
        self.matches = filter(&self.labels, &query, &mut self.matcher);
        self.selected = 0;
        self.scroll.scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }

    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let count = self.matches.len() as isize;
        if count > 0 {
            self.selected = (self.selected as isize + by).rem_euclid(count) as usize;
            self.scroll
                .scroll_to_item(self.selected, ScrollStrategy::Top);
            cx.notify();
        }
    }

    fn confirm(&mut self, cx: &mut Context<Self>) {
        let typed = self.input.read(cx).text().trim().to_owned();
        let event = match self.matches.get(self.selected) {
            Some(place) => Event::Picked(*place),
            None if !typed.is_empty() => Event::Typed(typed),
            None => Event::Dismissed,
        };
        cx.emit(event);
    }

    fn render_rows(&self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        range
            .map(|row| {
                let place = self.matches[row];
                ui::list_row(("option", row), row == self.selected, &settings)
                    .text_color(settings.colors.text)
                    .child(div().truncate().child(self.labels[place].clone()))
                    .on_click(cx.listener(move |_, _, _, cx| cx.emit(Event::Picked(place))))
                    .into_any_element()
            })
            .collect()
    }
}

impl Render for Combobox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::get(cx);
        let rows = self.matches.len().min(MAX_VISIBLE_ROWS) as f32;
        let list = (!self.matches.is_empty()).then(|| {
            ui::select_dropdown(
                ui::popover(window, &settings)
                    .id("combobox")
                    .occlude()
                    .w(ui::SELECT_WIDTH)
                    .on_hover(cx.listener(|this, over: &bool, _, _| this.over_list = *over))
                    .child(
                        uniform_list(
                            "options",
                            self.matches.len(),
                            cx.processor(|this, range, _, cx| this.render_rows(range, cx)),
                        )
                        .track_scroll(self.scroll.clone())
                        .h(settings.ui_font_size * 2. * rows),
                    ),
            )
        });
        // Not a flex container: there the list, positioned on its own, would start at the
        // top, over the field, rather than below it.
        div()
            .id("combobox-field")
            .w(ui::SELECT_WIDTH)
            // A press on what can't take the focus leaves it in the field, so the blur
            // above doesn't see it.
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if !this.over_list {
                    cx.emit(Event::Dismissed);
                }
            }))
            .child(self.input.clone())
            .children(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn models() -> Vec<SharedString> {
        ["Opus 5.5", "Sonnet 5.5", "Haiku 5.5", "Haiku 4.5"]
            .map(SharedString::new_static)
            .to_vec()
    }

    #[test]
    fn an_empty_query_keeps_every_choice_in_order() {
        let mut matcher = Matcher::new(Config::DEFAULT);
        assert_eq!(filter(&models(), "", &mut matcher), [0, 1, 2, 3]);
    }

    #[test]
    fn typing_narrows_the_list() {
        let mut matcher = Matcher::new(Config::DEFAULT);
        assert_eq!(filter(&models(), "haiku", &mut matcher), [2, 3]);
        assert_eq!(filter(&models(), "HAI 4", &mut matcher), [3]);
        assert_eq!(filter(&models(), "zzz", &mut matcher), Vec::<usize>::new());
    }
}
