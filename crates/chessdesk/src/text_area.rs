//! A small multi-line text field: typing, selection, the clipboard and wrapping. Offsets
//! are bytes into the text, always on a character boundary.

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, Hsla,
    InspectorElementId, IntoElement, KeyBinding, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, NoAction, Pixels, Point, SharedString, Style, TextAlign, TextRun,
    UTF16Selection, Window, WrappedLine, actions, div, fill, point, prelude::*, px, relative, size,
};

use crate::settings::Settings;

actions!(
    text_area,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        Home,
        End,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        Newline,
        Copy,
        Cut,
        Paste,
        Cancel,
    ]
);

const CONTEXT: &str = "TextArea";
/// A field of one line: Escape means something to it, and elsewhere closes what's around.
const ONE_LINE_CONTEXT: &str = "OneLine";
const CARET_WIDTH: Pixels = px(1.5);
/// How many lines a field offers even when it's empty.
const MIN_LINES: f32 = 3.;
const MAX_LINES: f32 = 6.;

pub fn bind_keys(cx: &mut App) {
    let context = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, context),
        KeyBinding::new("delete", Delete, context),
        KeyBinding::new("left", Left, context),
        KeyBinding::new("right", Right, context),
        KeyBinding::new("up", Up, context),
        KeyBinding::new("down", Down, context),
        KeyBinding::new("home", Home, context),
        KeyBinding::new("end", End, context),
        KeyBinding::new("shift-left", SelectLeft, context),
        KeyBinding::new("shift-right", SelectRight, context),
        KeyBinding::new("shift-up", SelectUp, context),
        KeyBinding::new("shift-down", SelectDown, context),
        KeyBinding::new("ctrl-a", SelectAll, context),
        KeyBinding::new("enter", Newline, context),
        KeyBinding::new("escape", Cancel, Some(ONE_LINE_CONTEXT)),
        KeyBinding::new("ctrl-c", Copy, context),
        KeyBinding::new("ctrl-x", Cut, context),
        KeyBinding::new("ctrl-v", Paste, context),
        // While typing, these mean nothing to the game behind.
        KeyBinding::new("ctrl-z", NoAction, context),
        KeyBinding::new("ctrl-f", NoAction, context),
        KeyBinding::new("ctrl-s", NoAction, context),
    ]);
}

/// The text changed.
pub struct Changed;

/// How editing a one-line field ended.
pub enum Finished {
    Accepted,
    Cancelled,
}

/// A line of the text as laid out, which may wrap into several rows.
struct Paragraph {
    /// Where it starts in the text.
    start: usize,
    line: WrappedLine,
    top: Pixels,
    height: Pixels,
}

pub struct TextArea {
    focus: FocusHandle,
    text: String,
    selection: Range<usize>,
    /// Whether the moving end of the selection is its start.
    reversed: bool,
    placeholder: SharedString,
    /// Enter ends the editing instead of starting a line.
    one_line: bool,
    dragging: bool,
    paragraphs: Vec<Paragraph>,
    bounds: Bounds<Pixels>,
    line_height: Pixels,
}

impl EventEmitter<Changed> for TextArea {}
impl EventEmitter<Finished> for TextArea {}

impl Focusable for TextArea {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// The previous character boundary before `offset`.
fn previous_boundary(text: &str, offset: usize) -> usize {
    text[..offset]
        .char_indices()
        .next_back()
        .map_or(0, |(at, _)| at)
}

/// The next character boundary after `offset`.
fn next_boundary(text: &str, offset: usize) -> usize {
    text[offset..]
        .chars()
        .next()
        .map_or(offset, |next| offset + next.len_utf8())
}

fn utf16_to_byte(text: &str, target: usize) -> usize {
    let mut units = 0;
    for (at, character) in text.char_indices() {
        if units >= target {
            return at;
        }
        units += character.len_utf16();
    }
    text.len()
}

fn byte_to_utf16(text: &str, offset: usize) -> usize {
    text[..offset].chars().map(char::len_utf16).sum()
}

impl TextArea {
    pub fn new(text: String, placeholder: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        let end = text.len();
        Self {
            focus: cx.focus_handle(),
            text,
            selection: end..end,
            reversed: false,
            placeholder: placeholder.into(),
            one_line: false,
            dragging: false,
            paragraphs: Vec::new(),
            bounds: Bounds::default(),
            line_height: px(16.),
        }
    }

    /// A field for one line, with all of its text selected: typing replaces it.
    pub fn one_line(text: String, cx: &mut Context<Self>) -> Self {
        let selection = 0..text.len();
        Self {
            selection,
            one_line: true,
            ..Self::new(text, SharedString::default(), cx)
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    fn cursor(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selection = offset..offset;
        self.reversed = false;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.reversed {
            self.selection.start = offset;
        } else {
            self.selection.end = offset;
        }
        if self.selection.end < self.selection.start {
            self.reversed = !self.reversed;
            self.selection = self.selection.end..self.selection.start;
        }
        cx.notify();
    }

    fn replace(&mut self, range: Range<usize>, new_text: &str, cx: &mut Context<Self>) {
        self.text.replace_range(range.clone(), new_text);
        let end = range.start + new_text.len();
        self.selection = end..end;
        self.reversed = false;
        cx.emit(Changed);
        cx.notify();
    }

    fn insert(&mut self, new_text: &str, cx: &mut Context<Self>) {
        self.replace(self.selection.clone(), new_text, cx);
    }

    /// Where an offset sits, relative to the text's top left.
    fn position_of(&self, offset: usize) -> Point<Pixels> {
        for paragraph in &self.paragraphs {
            let end = paragraph.start + paragraph.line.len();
            if offset <= end {
                let inner = offset.saturating_sub(paragraph.start);
                let at = paragraph.line.position_for_index(inner, self.line_height);
                return at.unwrap_or_default() + point(px(0.), paragraph.top);
            }
        }
        Point::default()
    }

    /// The offset nearest a point relative to the text's top left.
    fn offset_at(&self, position: Point<Pixels>) -> usize {
        let paragraph = self
            .paragraphs
            .iter()
            .rev()
            .find(|paragraph| paragraph.top <= position.y)
            .or(self.paragraphs.first());
        let Some(paragraph) = paragraph else {
            return 0;
        };
        // Below the text means its end, wherever along the row the press was.
        if position.y >= paragraph.top + paragraph.height {
            return paragraph.start + paragraph.line.len();
        }
        let inside = point(position.x, position.y - paragraph.top);
        let found = paragraph
            .line
            .closest_index_for_position(inside, self.line_height);
        paragraph.start + found.unwrap_or_else(|index| index)
    }

    /// The offset a row above or below the cursor, as far along as the row allows.
    fn vertical(&self, rows: f32) -> usize {
        let at = self.position_of(self.cursor());
        let target = at.y + self.line_height * rows;
        if target < px(0.) {
            return 0;
        }
        self.offset_at(point(at.x, target))
    }

    /// Where the row holding the cursor starts or ends.
    fn line_edge(&self, end: bool) -> usize {
        let y = self.position_of(self.cursor()).y;
        let x = if end { self.bounds.size.width } else { px(0.) };
        self.offset_at(point(x, y))
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        if self.selection.is_empty() {
            let start = previous_boundary(&self.text, self.selection.start);
            self.selection.start = start;
        }
        self.insert("", cx);
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        if self.selection.is_empty() {
            let end = next_boundary(&self.text, self.selection.end);
            self.selection.end = end;
        }
        self.insert("", cx);
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let to = if self.selection.is_empty() {
            previous_boundary(&self.text, self.cursor())
        } else {
            self.selection.start
        };
        self.move_to(to, cx);
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let to = if self.selection.is_empty() {
            next_boundary(&self.text, self.cursor())
        } else {
            self.selection.end
        };
        self.move_to(to, cx);
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.vertical(-1.), cx);
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.vertical(1.), cx);
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.line_edge(false), cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.line_edge(true), cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(previous_boundary(&self.text, self.cursor()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(next_boundary(&self.text, self.cursor()), cx);
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.vertical(-1.), cx);
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.vertical(1.), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selection = 0..self.text.len();
        self.reversed = false;
        cx.notify();
    }

    fn newline(&mut self, _: &Newline, _: &mut Window, cx: &mut Context<Self>) {
        if self.one_line {
            cx.emit(Finished::Accepted);
        } else {
            self.insert("\n", cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selection.is_empty() {
            let selected = self.text[self.selection.clone()].to_owned();
            cx.write_to_clipboard(ClipboardItem::new_string(selected));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.copy(&Copy, window, cx);
        self.insert("", cx);
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.insert(&text, cx);
        }
    }

    fn inside(&self, position: Point<Pixels>) -> Point<Pixels> {
        position - self.bounds.origin
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus);
        self.dragging = true;
        let offset = self.offset_at(self.inside(event.position));
        if event.modifiers.shift {
            self.select_to(offset, cx);
        } else {
            self.move_to(offset, cx);
        }
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.dragging {
            self.select_to(self.offset_at(self.inside(event.position)), cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.dragging = false;
    }
}

impl EntityInputHandler for TextArea {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let bytes = utf16_to_byte(&self.text, range.start)..utf16_to_byte(&self.text, range.end);
        *adjusted =
            Some(byte_to_utf16(&self.text, bytes.start)..byte_to_utf16(&self.text, bytes.end));
        Some(self.text[bytes].to_owned())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: byte_to_utf16(&self.text, self.selection.start)
                ..byte_to_utf16(&self.text, self.selection.end),
            reversed: self.reversed,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let bytes = range.map_or_else(
            || self.selection.clone(),
            |range| utf16_to_byte(&self.text, range.start)..utf16_to_byte(&self.text, range.end),
        );
        self.replace(bytes, text, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(range, text, window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let offset = utf16_to_byte(&self.text, range.start);
        let at = self.position_of(offset);
        Some(Bounds::new(
            element_bounds.origin + at,
            size(CARET_WIDTH, self.line_height),
        ))
    }

    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let offset = self.offset_at(self.inside(position));
        Some(byte_to_utf16(&self.text, offset))
    }
}

/// Lays out `text` to `width`, returning its paragraphs and their height.
fn shape(text: &str, width: Pixels, color: Hsla, window: &Window) -> (Vec<Paragraph>, Pixels) {
    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let line_height = window.line_height();
    let run = TextRun {
        len: text.len(),
        font: style.font(),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let lines = window
        .text_system()
        .shape_text(text.to_owned().into(), font_size, &[run], Some(width), None)
        .unwrap_or_default();
    let mut paragraphs = Vec::new();
    let mut start = 0;
    let mut top = px(0.);
    for line in lines {
        let height = line.size(line_height).height.max(line_height);
        let length = line.len();
        paragraphs.push(Paragraph {
            start,
            line,
            top,
            height,
        });
        // The break between lines is one byte the layout leaves out.
        start += length + 1;
        top += height;
    }
    (paragraphs, top.max(line_height))
}

/// Draws the text and takes keyboard input.
struct TextElement {
    area: Entity<TextArea>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let area = self.area.clone();
        let layout = window.request_measured_layout(style, move |known, available, window, cx| {
            let width = known.width.unwrap_or(match available.width {
                gpui::AvailableSpace::Definite(width) => width,
                _ => px(200.),
            });
            let text = area.read(cx).text.clone();
            let color = Settings::get(cx).colors.text;
            let (_, height) = shape(&text, width, color, window);
            size(width, height)
        });
        let _ = cx;
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let text = self.area.read(cx).text.clone();
        let color = Settings::get(cx).colors.text;
        let (paragraphs, _) = shape(&text, bounds.size.width, color, window);
        let line_height = window.line_height();
        self.area.update(cx, |area, _| {
            area.paragraphs = paragraphs;
            area.bounds = bounds;
            area.line_height = line_height;
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let settings = Settings::get(cx);
        let area = self.area.read(cx);
        let focus = area.focus.clone();
        let height = area.line_height;
        let selection = area.selection.clone();
        let (start, end) = (
            area.position_of(selection.start),
            area.position_of(selection.end),
        );
        let caret = area.position_of(area.cursor());
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.area.clone()),
            cx,
        );

        let origin = bounds.origin;
        let tint = settings.colors.caret.opacity(0.3);
        if !selection.is_empty() {
            for row in selection_rows(start, end, height, bounds.size.width) {
                window.paint_quad(fill(Bounds::new(origin + row.origin, row.size), tint));
            }
        }
        let area = self.area.read(cx);
        let lines: Vec<_> = area
            .paragraphs
            .iter()
            .map(|paragraph| (paragraph.line.clone(), paragraph.top))
            .collect();
        for (line, top) in lines {
            let at = origin + point(px(0.), top);
            let _ = line.paint(at, height, TextAlign::Left, None, window, cx);
        }
        if focus.is_focused(window) && selection.is_empty() {
            let bar = Bounds::new(origin + caret, size(CARET_WIDTH, height));
            window.paint_quad(fill(bar, settings.colors.caret));
        }
    }
}

/// The rectangles a selection covers, from row to row.
fn selection_rows(
    start: Point<Pixels>,
    end: Point<Pixels>,
    height: Pixels,
    width: Pixels,
) -> Vec<Bounds<Pixels>> {
    if start.y == end.y {
        return vec![Bounds::new(start, size(end.x - start.x, height))];
    }
    let mut rows = vec![Bounds::new(start, size(width - start.x, height))];
    let mut y = start.y + height;
    while y < end.y {
        rows.push(Bounds::new(point(px(0.), y), size(width, height)));
        y += height;
    }
    rows.push(Bounds::new(point(px(0.), end.y), size(end.x, height)));
    rows
}

impl Render for TextArea {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::get(cx);
        let line = window.line_height();
        let focused = self.focus.is_focused(window);
        let placeholder = self.text.is_empty().then(|| {
            div()
                .absolute()
                .left_2()
                .right_2()
                .top_1()
                .text_color(settings.colors.muted)
                .child(self.placeholder.clone())
        });
        div()
            .id("text-area")
            .key_context(if self.one_line {
                "TextArea OneLine"
            } else {
                CONTEXT
            })
            .track_focus(&self.focus)
            .relative()
            .w_full()
            .map(|field| {
                if self.one_line {
                    field.h(line + px(2.)).px_1()
                } else {
                    field
                        .min_h(line * MIN_LINES + px(10.))
                        .max_h(line * MAX_LINES + px(10.))
                        .px_2()
                        .py_1()
                }
            })
            .overflow_y_scroll()
            .rounded(settings.corner_radius)
            .border_1()
            .border_color(if focused {
                settings.colors.caret.opacity(0.5)
            } else {
                settings.colors.popover_border
            })
            .text_color(settings.colors.text)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(|_, _: &Cancel, _, cx| cx.emit(Finished::Cancelled)))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .children(placeholder)
            .child(TextElement { area: cx.entity() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundaries_respect_wide_characters() {
        let text = "aé€";
        assert_eq!(next_boundary(text, 0), 1);
        assert_eq!(next_boundary(text, 1), 3);
        assert_eq!(previous_boundary(text, 6), 3);
        assert_eq!(previous_boundary(text, 3), 1);
        assert_eq!(previous_boundary(text, 0), 0);
        assert_eq!(next_boundary(text, text.len()), text.len());
    }

    #[test]
    fn utf16_offsets_convert_both_ways() {
        let text = "a😀b";
        assert_eq!(byte_to_utf16(text, 5), 3);
        assert_eq!(utf16_to_byte(text, 3), 5);
        assert_eq!(utf16_to_byte(text, 99), text.len());
    }

    #[test]
    fn a_selection_covers_each_row_it_crosses() {
        let height = px(10.);
        let start = point(px(30.), px(0.));
        let end = point(px(20.), px(20.));
        let rows = selection_rows(start, end, height, px(100.));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].size.width, px(70.));
        assert_eq!(rows[1].size.width, px(100.));
        assert_eq!(rows[2].size.width, px(20.));
    }
}
