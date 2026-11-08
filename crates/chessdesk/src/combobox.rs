//! A select to type in, for long lists: the text narrows the list, and with no match stands
//! for itself. While it exists it is open; its owner shows a plain select otherwise.

use gpui::{
    AnyElement, Context, Entity, EventEmitter, Focusable, ScrollHandle, SharedString, Subscription,
    Window, div, prelude::*,
};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::i18n::tr;
use crate::settings::Settings;
use crate::text_area::{Changed, Finished, Step, TextArea};
use crate::ui;

/// About how many rows the list shows before it scrolls.
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
    /// The places in `labels` of the choices that match what's typed, best first, the free
    /// ones ahead of the rest.
    matches: Vec<usize>,
    /// How many of `matches` are free: a section of their own, under a heading.
    free: usize,
    /// Which of `matches` Enter picks.
    selected: usize,
    matcher: Matcher,
    scroll: ScrollHandle,
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

/// Whether a model costs nothing, going by its name, as `OpenCode`'s free ones say so.
fn is_free(label: &str) -> bool {
    label.to_lowercase().contains("free")
}

/// Puts the free choices first, each part in the order it had, and says how many there are.
fn free_first(labels: &[SharedString], matches: Vec<usize>) -> (Vec<usize>, usize) {
    let (mut free, rest): (Vec<_>, Vec<_>) = matches
        .into_iter()
        .partition(|place| is_free(&labels[*place]));
    let count = free.len();
    free.extend(rest);
    (free, count)
}

/// A row of the list: a section's heading, or a choice by its place in the matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Heading(Section),
    Choice(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Free,
    /// The rest, below the free ones: it's headed only when there are free ones.
    Premium,
}

/// The rows for `count` matches, the first `free` of them free.
fn rows(count: usize, free: usize) -> Vec<Row> {
    let mut rows = Vec::with_capacity(count + 2);
    if free > 0 {
        rows.push(Row::Heading(Section::Free));
    }
    for index in 0..count {
        if index == free && free > 0 {
            rows.push(Row::Heading(Section::Premium));
        }
        rows.push(Row::Choice(index));
    }
    rows
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
        let (matches, free) = free_first(&labels, (0..labels.len()).collect());
        let selected = matches
            .iter()
            .position(|place| *place == current)
            .unwrap_or(0);
        let mut combobox = Self {
            input,
            matches,
            free,
            labels,
            selected,
            matcher: Matcher::new(Config::DEFAULT),
            scroll: ScrollHandle::new(),
            over_list: false,
            _subscriptions: subscriptions,
        };
        combobox.scroll_to_selected();
        combobox
    }

    /// The list's rows, top to bottom.
    fn rows(&self) -> Vec<Row> {
        rows(self.matches.len(), self.free)
    }

    fn scroll_to_selected(&mut self) {
        let rows = self.rows();
        let mut row = rows
            .iter()
            .position(|row| *row == Row::Choice(self.selected))
            .unwrap_or(0);
        // A choice first in its section brings the heading above it into view too.
        if row > 0 && matches!(rows[row - 1], Row::Heading(_)) {
            row -= 1;
        }
        self.scroll.scroll_to_item(row);
    }

    fn refilter(&mut self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).text().trim().to_owned();
        let matches = filter(&self.labels, &query, &mut self.matcher);
        (self.matches, self.free) = free_first(&self.labels, matches);
        self.selected = 0;
        self.scroll_to_selected();
        cx.notify();
    }

    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let count = self.matches.len() as isize;
        if count > 0 {
            self.selected = (self.selected as isize + by).rem_euclid(count) as usize;
            self.scroll_to_selected();
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

    fn render_rows(&self, window: &Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = Settings::get(cx);
        self.rows()
            .into_iter()
            .map(|row| match row {
                Row::Heading(section) => {
                    let title = div()
                        .px_2()
                        .pt_0p5()
                        .pb_1()
                        .text_size(settings.ui_font_size * 0.92)
                        .text_color(settings.colors.muted)
                        .child(tr(match section {
                            Section::Free => "free-models",
                            Section::Premium => "premium-models",
                        }));
                    div()
                        .flex()
                        .flex_col()
                        .flex_none()
                        // Edge to edge of the list, like a menu's separators.
                        .when(section == Section::Premium, |heading| {
                            heading.child(ui::separator(window, &settings).my_1p5())
                        })
                        .child(title)
                        .into_any_element()
                }
                Row::Choice(index) => {
                    let place = self.matches[index];
                    ui::list_row(("option", index), index == self.selected, &settings)
                        .flex_none()
                        .text_color(settings.colors.text)
                        .child(div().truncate().child(self.labels[place].clone()))
                        .on_click(cx.listener(move |_, _, _, cx| cx.emit(Event::Picked(place))))
                        .into_any_element()
                }
            })
            .collect()
    }
}

impl Render for Combobox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::get(cx);
        let list = (!self.matches.is_empty()).then(|| {
            let rows = self.render_rows(window, cx);
            // The padding is the scrolling part's own, so that separators reach its edges.
            ui::select_dropdown(
                ui::popover(window, &settings)
                    .id("combobox")
                    .occlude()
                    .p_0()
                    .w(ui::SELECT_WIDTH)
                    .on_hover(cx.listener(|this, over: &bool, _, _| this.over_list = *over))
                    .child(
                        div()
                            .id("options")
                            .flex()
                            .flex_col()
                            .p_1()
                            .max_h(settings.ui_font_size * 2. * MAX_VISIBLE_ROWS as f32)
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .children(rows),
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
    fn free_models_come_first_in_their_own_order() {
        let labels = ["Default", "GPT 5", "Big Pickle (free)", "Grok Fast Free"]
            .map(SharedString::new_static)
            .to_vec();
        assert_eq!(free_first(&labels, vec![0, 1, 2, 3]), (vec![2, 3, 0, 1], 2));
        assert_eq!(free_first(&labels, vec![3, 1]), (vec![3, 1], 1));
        assert_eq!(free_first(&labels, vec![1, 0]), (vec![1, 0], 0));
    }

    #[test]
    fn free_and_premium_get_headings_only_side_by_side() {
        use Row::{Choice, Heading};
        assert_eq!(
            rows(3, 1),
            [
                Heading(Section::Free),
                Choice(0),
                Heading(Section::Premium),
                Choice(1),
                Choice(2)
            ]
        );
        assert_eq!(rows(2, 0), [Choice(0), Choice(1)]);
        assert_eq!(rows(1, 1), [Heading(Section::Free), Choice(0)]);
    }

    #[test]
    fn typing_narrows_the_list() {
        let mut matcher = Matcher::new(Config::DEFAULT);
        assert_eq!(filter(&models(), "haiku", &mut matcher), [2, 3]);
        assert_eq!(filter(&models(), "HAI 4", &mut matcher), [3]);
        assert_eq!(filter(&models(), "zzz", &mut matcher), Vec::<usize>::new());
    }
}
