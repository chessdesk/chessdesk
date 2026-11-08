//! Shared building blocks, so that every surface looks and behaves the same.

mod hover;
mod icons;
mod transition;

use std::time::Duration;

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Animation, AnimationElement, AnimationExt, AnyView, App, Axis, Bounds, ClickEvent, Context,
    Corner, CursorStyle, DispatchPhase, Div, ElementId, Hitbox, HitboxBehavior, Hsla, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels, SharedString, Stateful,
    Transformation, Window, anchored, canvas, deferred, div, ease_out_quint, fill, percentage,
    point, prelude::*, px, relative, size, white,
};

use crate::settings::Settings;
pub use hover::hover_fill;
pub use icons::{Assets, Icon, Logo, icon, logo, logotype};
pub use transition::Transition;

const MENU_WIDTH: Pixels = px(260.);
const TOGGLE_HEIGHT: Pixels = px(14.);
const TOGGLE_INSET: Pixels = px(2.);
/// How much wider a switch is than tall.
const TOGGLE_WIDTH: f32 = 1.9;
/// The width of selects, which their lists match.
pub const SELECT_WIDTH: Pixels = px(220.);
/// Between a select and its list: close enough to read as one control.
const SELECT_LIST_GAP: Pixels = px(2.);
const EMPTY_LOGO_SIZE: Pixels = px(34.);
const EMPTY_LOGO_OPACITY: f32 = 0.12;
/// The height of the row across the top of the window, with the menu and window controls.
pub const TITLE_BAR_HEIGHT: Pixels = px(30.);
/// A window control: the full height of the title bar, and a little wider than that.
pub const WINDOW_CONTROL_WIDTH: Pixels = px(42.);
/// The height of buttons and selects.
pub const CONTROL_HEIGHT: Pixels = px(24.);
/// The height of a row holding a label and a control, and of bars across the window.
const ROW_HEIGHT: Pixels = px(34.);
pub const BAR_HEIGHT: Pixels = ROW_HEIGHT;
/// From the left of a bar to its text.
pub const BAR_INSET: Pixels = px(17.);
/// How much larger logos are than the text beside them.
pub const LOGO_SCALE: f32 = 1.25;
/// How much larger a settings row's logo is than its text: about as tall as label and hint.
const SETTING_LOGO_SCALE: f32 = 2.;
/// How round wide, flat shapes are against the corner radius: on them it looks stronger
/// than it does on a button.
pub const FLAT_RADIUS_SCALE: f32 = 0.7;
/// How strongly a card's colour tints its background: results and verdicts.
pub const CARD_TINT: f32 = 0.16;
/// How strongly a card's details show, next to its main line.
pub const CARD_DETAIL: f32 = 0.75;
/// How much of the text colour lifts a notice's button off its card, and again on hover.
const NOTICE_BUTTON_RAISE: f32 = 0.08;
/// A setting's description, against its font size: closer than GPUI's default, so that
/// it reads as one block with its label.
const HINT_LINE_HEIGHT: f32 = 1.3;
/// How much larger titles are than the text around them.
const TITLE_SCALE: f32 = 1.3;
/// One turn of a [`spinner`].
const SPIN: Duration = Duration::from_millis(800);
/// How far beyond its line a divider can be grabbed.
const DIVIDER_REACH: Pixels = px(3.);
/// How wide the strips at the window's edges are, beside the sidebars.
pub const EDGE_STRIP_SIZE: Pixels = px(14.);
const SLIDER_HEIGHT: Pixels = px(14.);
const SLIDER_KNOB: Pixels = px(26.);
const SLIDER_GROOVE_GAP: Pixels = px(3.);
/// How much of the way left the knob covers each frame.
const SLIDER_GLIDE: f32 = 0.3;
/// How close to its stop, in stops, the knob counts as there.
const SLIDER_SETTLED: f32 = 0.01;

/// A full-width row of a list: menus and the library.
pub fn list_row(id: impl Into<ElementId>, selected: bool, settings: &Settings) -> Stateful<Div> {
    let accent = settings.colors.accent;
    div()
        .id(id)
        .flex()
        .items_center()
        .w_full()
        .h(settings.ui_font_size * 2.)
        .px_2()
        .rounded(settings.corner_radius)
        .cursor_pointer()
        .when(selected, |row| row.bg(accent))
        .child(hover_fill(accent, settings.corner_radius))
}

/// A compact control that lights up on hover: menu titles and moves.
pub fn button(id: impl Into<ElementId>, active: bool, settings: &Settings) -> Stateful<Div> {
    let accent = settings.colors.accent;
    let text = settings.colors.text;
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .px_2()
        .py_0p5()
        .rounded(settings.corner_radius)
        .cursor_pointer()
        .text_color(if active { text } else { settings.colors.muted })
        .when(active, |button| button.bg(accent))
        .child(hover_fill(accent, settings.corner_radius))
        .hover(move |style| style.text_color(text))
}

/// An [`action_button`] that can't be pressed yet.
pub fn disabled_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    settings: &Settings,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(CONTROL_HEIGHT)
        .px_2()
        .rounded(settings.corner_radius)
        .text_color(settings.colors.muted)
        .border_1()
        .border_color(settings.colors.popover_border)
        .child(label.into())
}

/// The title of a dialog or a settings page.
pub fn title(text: impl Into<SharedString>, settings: &Settings) -> Div {
    div()
        .pb_2()
        .text_size(settings.ui_font_size * TITLE_SCALE)
        .child(text.into())
}

/// The layer of a dialog: it covers everything below the title bar, so that nothing
/// behind can be pressed, and holds the dialog in its middle.
pub fn modal(id: impl Into<ElementId>, settings: &Settings) -> Stateful<Div> {
    div()
        .id(id)
        .occlude()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(settings.colors.background.opacity(0.7))
}

/// A framed button whose work is under way: a spinner in the place of its label. The
/// label stays, unseen, so that the button keeps its width.
pub fn busy_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    settings: &Settings,
) -> Stateful<Div> {
    let id = id.into();
    let size = settings.ui_font_size;
    div()
        .id(id.clone())
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(CONTROL_HEIGHT)
        .px_2()
        .rounded(settings.corner_radius)
        .border_1()
        .border_color(settings.colors.popover_border)
        .child(div().invisible().child(label.into()))
        .child(
            div()
                .absolute()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(spinner(id, size, settings.colors.muted)),
        )
}

/// A framed square button holding one icon, the height of the controls beside it.
pub fn square_button(id: impl Into<ElementId>, name: Icon, settings: &Settings) -> Stateful<Div> {
    let accent = settings.colors.accent;
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(CONTROL_HEIGHT)
        .rounded(settings.corner_radius)
        .cursor_pointer()
        .border_1()
        .border_color(settings.colors.popover_border)
        .child(hover_fill(accent, settings.corner_radius))
        .child(icon(name, settings.ui_font_size, settings.colors.muted))
}

/// A framed button with a label: what a dialog or the panel offers to do.
pub fn action_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    settings: &Settings,
) -> Stateful<Div> {
    button(id, false, settings)
        .h(CONTROL_HEIGHT)
        .justify_center()
        .text_color(settings.colors.text)
        .border_1()
        .border_color(settings.colors.popover_border)
        .child(label.into())
}

/// An [`action_button`] for the main thing to do: filled, in the colour of a speech bubble.
/// Built afresh rather than from one, since GPUI allows an element only one hover style.
pub fn primary_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    settings: &Settings,
) -> Stateful<Div> {
    let accent = settings.colors.accent;
    let raised = accent.blend(settings.colors.text.opacity(0.08));
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(CONTROL_HEIGHT)
        .px_2()
        .py_0p5()
        .rounded(settings.corner_radius)
        .cursor_pointer()
        .text_color(settings.colors.text)
        .bg(accent)
        .border_1()
        .border_color(accent)
        .child(hover_fill(raised, settings.corner_radius))
        .child(label.into())
}

/// One device pixel. A logical 1px would round to 1 or 2 device pixels
/// by position at fractional display scales, making lines uneven.
pub fn hairline(window: &Window) -> Pixels {
    px(1.) / window.scale_factor()
}

/// A length in whole device pixels. Edges are rounded to those one by one, so that parts
/// with fractional sizes come out differently from one place on screen to the next.
pub fn snap(length: Pixels, window: &Window) -> Pixels {
    let scale = window.scale_factor();
    (length * scale).round() / scale
}

/// The floating surface of menus and dialogs.
/// Give it a fixed width, so that its rows all span the same width.
pub fn popover(window: &Window, settings: &Settings) -> Div {
    div()
        .p_1()
        .flex()
        .flex_col()
        .bg(settings.colors.popover)
        .border(hairline(window))
        .border_color(settings.colors.popover_border)
        .rounded(settings.popover_radius())
}

/// The surface of a dialog: a popover, rounder to look as round as the controls in it.
pub fn dialog(window: &Window, settings: &Settings) -> Div {
    popover(window, settings).rounded(settings.dialog_radius())
}

/// How long a dialog, menu or tooltip takes to come in. Going is at once: it's gone by then.
const APPEAR: Duration = Duration::from_millis(150);
/// How far a dialog or menu moves as it comes in.
const APPEAR_TRAVEL: Pixels = px(6.);

/// Fades `element` in as it moves the last of `travel` into place: down from above for a
/// positive one, up from below for a negative one.
fn appear<E: Styled + IntoElement + 'static>(
    id: impl Into<ElementId>,
    element: E,
    travel: Pixels,
) -> AnimationElement<E> {
    element.with_animation(
        id,
        Animation::new(APPEAR).with_easing(ease_out_quint()),
        move |element, done| element.opacity(done).top(-travel * (1. - done)),
    )
}

/// A menu coming in under what opened it.
pub fn drop_in<E: Styled + IntoElement + 'static>(
    id: impl Into<ElementId>,
    element: E,
) -> AnimationElement<E> {
    appear(id, element, APPEAR_TRAVEL)
}

/// A dialog, or a menu that opens upwards, coming in.
pub fn rise_in<E: Styled + IntoElement + 'static>(
    id: impl Into<ElementId>,
    element: E,
) -> AnimationElement<E> {
    appear(id, element, -APPEAR_TRAVEL)
}

/// What comes in where it stands: a dialog's shade, a tooltip.
pub fn fade_in<E: Styled + IntoElement + 'static>(
    id: impl Into<ElementId>,
    element: E,
) -> AnimationElement<E> {
    appear(id, element, px(0.))
}

/// A popover for a list of commands.
pub fn menu(window: &Window, settings: &Settings) -> Stateful<Div> {
    popover(window, settings)
        .id("menu")
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .w(MENU_WIDTH)
}

/// A row of a menu: a title, and the key that does the same, if any.
pub fn menu_item(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    key: Option<&'static str>,
    settings: &Settings,
) -> Stateful<Div> {
    list_row(id, false, settings)
        .justify_between()
        .text_color(settings.colors.text)
        .child(title.into())
        .child(div().text_color(settings.colors.muted).children(key))
}

/// A label with an optional hint, for a control added as a child.
pub fn setting_row(
    label: impl Into<SharedString>,
    hint: Option<SharedString>,
    settings: &Settings,
) -> Div {
    labelled_row(setting_label(label, hint, settings))
}

/// A [`setting_row`] with a logo before its label and hint, centred beside both.
pub fn logo_setting_row(
    mark: Logo,
    label: impl Into<SharedString>,
    hint: Option<SharedString>,
    settings: &Settings,
) -> Div {
    let size = settings.ui_font_size * SETTING_LOGO_SCALE;
    labelled_row(
        div()
            .flex()
            .items_center()
            .gap_2p5()
            .min_w_0()
            .child(logo(mark, size, settings.colors.text))
            .child(setting_label(label, hint, settings)),
    )
}

fn setting_label(
    label: impl Into<SharedString>,
    hint: Option<SharedString>,
    settings: &Settings,
) -> Div {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .child(label.into())
        .children(hint.map(|hint| {
            div()
                .text_size(settings.ui_font_size * 0.92)
                .line_height(relative(HINT_LINE_HEIGHT))
                .text_color(settings.colors.muted)
                .child(hint)
        }))
}

fn labelled_row(label: Div) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .min_h(ROW_HEIGHT)
        .py_1()
        .child(label)
}

/// Something to see to before going on, such as an engine to fetch: a card in the theme's
/// own colours, its message and a quieter detail beside room for an action, added as a child.
pub fn notice(
    message: impl Into<SharedString>,
    detail: Option<SharedString>,
    settings: &Settings,
) -> Div {
    let text = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .child(div().text_color(settings.colors.text).child(message.into()))
        .children(detail.map(|detail| {
            div()
                .text_size(settings.ui_font_size * 0.92)
                .text_color(settings.colors.muted)
                .child(detail)
        }));
    div()
        .flex()
        .items_center()
        .gap_3()
        .px_2p5()
        .py_2()
        .rounded(settings.corner_radius * FLAT_RADIUS_SCALE)
        .bg(settings.colors.accent)
        .child(text)
}

/// The button of a [`notice`]: raised a little from the card, with no frame, its label in
/// the text colour. While `busy` a spinner takes the label's place and the button keeps its
/// width; add `on_click` only when it isn't.
pub fn notice_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    busy: bool,
    settings: &Settings,
) -> Stateful<Div> {
    let id = id.into();
    let text = settings.colors.text;
    let raised = settings
        .colors
        .accent
        .blend(text.opacity(NOTICE_BUTTON_RAISE));
    let hovered = raised.blend(text.opacity(NOTICE_BUTTON_RAISE));
    let radius = settings.corner_radius;
    let label = div().when(busy, Styled::invisible).child(label.into());
    div()
        .id(id.clone())
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(CONTROL_HEIGHT)
        .px_2()
        .rounded(radius)
        .text_color(text)
        .bg(raised)
        .when(!busy, |button| {
            button.cursor_pointer().child(hover_fill(hovered, radius))
        })
        .child(label)
        .when(busy, |button| {
            button.child(
                div()
                    .absolute()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(spinner(id, settings.ui_font_size, settings.colors.muted)),
            )
        })
}

/// A switch. Add `on_click`.
pub fn toggle(
    id: impl Into<ElementId>,
    on: bool,
    window: &Window,
    settings: &Settings,
) -> Stateful<Div> {
    let height = snap(TOGGLE_HEIGHT, window);
    let inset = snap(TOGGLE_INSET, window);
    let knob = height - inset * 2.;
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .w(snap(height * TOGGLE_WIDTH, window))
        .h(height)
        .px(inset)
        .rounded_full()
        .cursor_pointer()
        .bg(if on {
            settings.colors.caret
        } else {
            settings.colors.accent
        })
        .when(on, gpui::Styled::justify_end)
        .child(div().size(knob).rounded_full().bg(if on {
            settings.colors.background
        } else {
            settings.colors.muted
        }))
}

/// What a panel with nothing in it shows: the logo, faint, in the middle of its room.
pub fn empty_panel(settings: &Settings) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .items_center()
        .justify_center()
        .gap_2()
        .child(icon(
            Icon::Logo,
            EMPTY_LOGO_SIZE,
            settings.colors.muted.opacity(EMPTY_LOGO_OPACITY),
        ))
}

/// The closed state of a select. Add `on_mouse_down`: its list closes on any press outside
/// it, this button included, so a click would open the list again.
pub fn select_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    mark: Option<Logo>,
    open: bool,
    settings: &Settings,
) -> Stateful<Div> {
    let accent = settings.colors.accent;
    let size = settings.ui_font_size;
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_between()
        .gap_2()
        .w(SELECT_WIDTH)
        .h(CONTROL_HEIGHT)
        .px_2()
        .rounded(settings.corner_radius)
        .cursor_pointer()
        .border_1()
        .border_color(settings.colors.popover_border)
        .when(open, |button| button.bg(accent))
        .child(hover_fill(accent, settings.corner_radius))
        .child(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap_1p5()
                .children(mark.map(|mark| logo(mark, size * LOGO_SCALE, settings.colors.text)))
                .child(div().truncate().child(label.into())),
        )
        .child(icon(Icon::ChevronDown, size, settings.colors.muted))
}

/// Hangs a list from the select it belongs to, added as a child below it. Where the list
/// doesn't fit under the select it opens above instead, and the bottom padding then clears
/// the select itself.
pub fn select_dropdown(list: impl IntoElement) -> impl IntoElement {
    deferred(
        anchored().anchor(Corner::TopLeft).child(drop_in(
            "select-list",
            div()
                .pt(SELECT_LIST_GAP)
                .pb(CONTROL_HEIGHT + SELECT_LIST_GAP)
                .child(list),
        )),
    )
    .with_priority(1)
}

/// What a control shows while hovered: what it does.
struct Tooltip {
    text: SharedString,
}

impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = Settings::get(cx);
        fade_in(
            "tooltip",
            popover(window, &settings)
                .px_2()
                .py_1()
                .font(settings.ui_font.clone())
                .text_size(settings.ui_font_size * 0.92)
                .text_color(settings.colors.text)
                .child(self.text.clone()),
        )
    }
}

/// A tooltip for `.tooltip(..)` on any hoverable element.
pub fn tooltip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| Tooltip { text: text.clone() }).into()
}

/// A notch going round, for something under way. `id` keeps its place in the turn
/// across frames, so it must stay the same while it shows.
pub fn spinner(id: impl Into<ElementId>, size: Pixels, color: Hsla) -> impl IntoElement {
    icon(Icon::Spinner, size, color).with_animation(
        id,
        Animation::new(SPIN).repeat(),
        |icon, turned| icon.with_transformation(Transformation::rotate(percentage(turned))),
    )
}

/// From one stripe of a [`loading_bar`] to the next, half of it stripe and half gap.
const STRIPE_PERIOD: Pixels = px(24.);
/// How long the stripes take to move on by one period, where the pattern repeats.
const STRIPE_SLIDE: Duration = Duration::from_millis(900);
/// How strong the gaps between stripes are, next to the stripes' full accent.
const STRIPE_GAP_OPACITY: f32 = 0.45;

/// Slanted stripes sliding along behind `status`, for work that takes a while. It fills
/// what holds it. `id` keeps the stripes' place across frames, as for a [`spinner`].
pub fn loading_bar(
    id: impl Into<ElementId>,
    status: impl Into<SharedString>,
    settings: &Settings,
) -> Div {
    let accent = settings.colors.accent;
    let stripes = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .with_animation(
            id,
            Animation::new(STRIPE_SLIDE).repeat(),
            move |stripes, moved| {
                stripes.child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, (), window, _| paint_stripes(bounds, moved, accent, window),
                    )
                    .size_full(),
                )
            },
        );
    // Cut off without an ellipsis. Never `line_clamp` with `text_ellipsis` to get one: text
    // measured twice keeps the first cut's length, and gpui panics if that splits a char.
    let label = div()
        .flex_1()
        .min_w_0()
        .truncate()
        .text_center()
        .child(status.into());
    div()
        .relative()
        .flex()
        .items_center()
        .size_full()
        .px_2()
        .overflow_hidden()
        .text_color(settings.colors.text)
        .bg(accent.opacity(STRIPE_GAP_OPACITY))
        .child(stripes)
        .child(label)
}

/// Leans at 45 degrees, starting far enough left that no corner of `bounds` is ever bare.
/// `moved` is how far along the period the stripes are, from 0 to 1.
fn paint_stripes(bounds: Bounds<Pixels>, moved: f32, color: Hsla, window: &mut Window) {
    let (top, bottom) = (bounds.top(), bounds.bottom());
    let slant = bounds.size.height;
    let width = STRIPE_PERIOD / 2.;
    let mut path = PathBuilder::fill();
    let mut left = bounds.left() - slant - STRIPE_PERIOD * (1. - moved);
    while left < bounds.right() {
        path.move_to(point(left, bottom));
        path.line_to(point(left + slant, top));
        path.line_to(point(left + slant + width, top));
        path.line_to(point(left + width, bottom));
        path.close();
        left += STRIPE_PERIOD;
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

/// A line between two panes. It takes up one pixel, so that lines meet exactly, and is
/// grabbed by a wider area painted over its neighbours.
/// `Axis::Horizontal` means it moves left and right.
pub fn divider(
    id: impl Into<ElementId>,
    axis: Axis,
    window: &Window,
    settings: &Settings,
    on_press: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let accent = settings.colors.accent;
    let thin = hairline(window);
    let handle = div()
        .id(id)
        .absolute()
        .child(hover_fill(accent, px(0.)))
        .on_mouse_down(MouseButton::Left, on_press);
    let (line, handle) = match axis {
        Axis::Horizontal => (
            div().w(thin).h_full(),
            handle
                .top_0()
                .bottom_0()
                .left(-DIVIDER_REACH)
                .w(DIVIDER_REACH * 2. + thin)
                .cursor(CursorStyle::ResizeLeftRight),
        ),
        Axis::Vertical => (
            div().h(thin).w_full(),
            handle
                .left_0()
                .right_0()
                .top(-DIVIDER_REACH)
                .h(DIVIDER_REACH * 2. + thin)
                .cursor(CursorStyle::ResizeUpDown),
        ),
    };
    line.relative()
        .flex_none()
        .bg(settings.colors.popover_border)
        .child(deferred(handle))
}

/// Which edge of the window an [`edge_strip`] sits at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
}

/// The strip at the window's edge beside a sidebar, open or not: a line towards the
/// window's centre, and an area to click with a chevron pointing the way the click moves
/// things: away from the edge to open, back to it to close.
pub fn edge_strip(
    id: impl Into<ElementId>,
    edge: Edge,
    open: bool,
    label: impl Into<SharedString>,
    window: &Window,
    settings: &Settings,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let accent = settings.colors.accent;
    let chevron = if (edge == Edge::Left) == open {
        Icon::ChevronLeft
    } else {
        Icon::ChevronRight
    };
    let area = div()
        .id(id)
        .flex()
        .flex_1()
        .h_full()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .child(hover_fill(accent, px(0.)))
        .tooltip(tooltip(label))
        .on_click(on_click)
        .child(icon(chevron, settings.ui_font_size, settings.colors.muted));
    div()
        .flex()
        .flex_none()
        .h_full()
        .w(EDGE_STRIP_SIZE)
        .when(edge == Edge::Right, gpui::Styled::flex_row_reverse)
        .child(area)
        .child(
            div()
                .flex_none()
                .h_full()
                .w(hairline(window))
                .bg(settings.colors.popover_border),
        )
}

/// Which of `count` stops a pointer at `x` means, on a slider `width` wide.
fn slider_stop(x: f32, width: f32, count: usize) -> usize {
    if count < 2 || width <= 0. {
        return 0;
    }
    let share = (x / width).clamp(0., 1.);
    (share * (count - 1) as f32).round() as usize
}

/// What a slider remembers across frames.
#[derive(Default)]
struct SliderState {
    pressed: Cell<bool>,
    /// Where the knob is on its way to its stop, in stops.
    shown: Cell<Option<f32>>,
}

/// A framed track with `count` stops, and a grooved knob gliding to the one at `index`.
/// Press or drag anywhere on it; `on_pick` hears each new stop.
pub fn slider(
    id: impl Into<ElementId>,
    count: usize,
    index: usize,
    settings: &Settings,
    window: &mut Window,
    cx: &mut App,
    on_pick: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id = id.into();
    let state = window
        .use_keyed_state(id.clone(), cx, |_, _| Rc::new(SliderState::default()))
        .read(cx)
        .clone();
    let on_pick = Rc::new(on_pick);
    let colors = settings.colors.clone();
    let radius = settings.corner_radius;
    let paint = move |bounds: Bounds<Pixels>, hitbox: Hitbox, window: &mut Window, _: &mut App| {
        // The knob's left edge travels what the track has beyond the knob's own width.
        let travel = bounds.size.width - SLIDER_KNOB;
        let target = index as f32;
        let shown = state.shown.get().unwrap_or(target);
        let shown = if (target - shown).abs() < SLIDER_SETTLED {
            target
        } else {
            window.request_animation_frame();
            shown + (target - shown) * SLIDER_GLIDE
        };
        state.shown.set(Some(shown));
        let left = bounds.left() + travel * (shown / (count.max(2) - 1) as f32);

        // Clear in the border's own colour: a clear black would shade the border's inner
        // edge, which shows as a dark line on light themes.
        window.paint_quad(
            fill(bounds, colors.popover_border.opacity(0.))
                .corner_radii(radius)
                .border_widths(px(1.))
                .border_color(colors.popover_border),
        );
        let knob = Bounds::new(
            point(left, bounds.top()),
            size(SLIDER_KNOB, bounds.size.height),
        );
        // A grey knob is the darkest thing on a light page, so there it's white and framed
        // like the track, its grooves grey.
        let light = colors.background.l > 0.5;
        let groove_color = if light {
            window.paint_quad(
                fill(knob, white())
                    .corner_radii(radius)
                    .border_widths(px(1.))
                    .border_color(colors.popover_border),
            );
            colors.muted
        } else {
            window.paint_quad(fill(knob, colors.muted).corner_radii(radius));
            colors.background.opacity(0.55)
        };
        let middle = left + SLIDER_KNOB / 2.;
        for offset in [-SLIDER_GROOVE_GAP, px(0.), SLIDER_GROOVE_GAP] {
            let groove = Bounds::new(
                point(
                    middle + offset - px(0.5),
                    bounds.top() + bounds.size.height * 0.28,
                ),
                size(px(1.), bounds.size.height * 0.44),
            );
            window.paint_quad(fill(groove, groove_color));
        }

        let pick = {
            let on_pick = on_pick.clone();
            move |x: Pixels, window: &mut Window, cx: &mut App| {
                let from_left = f32::from(x - bounds.left() - SLIDER_KNOB / 2.);
                let stop = slider_stop(from_left, f32::from(travel), count);
                if stop != index {
                    on_pick(stop, window, cx);
                }
            }
        };
        let down = state.clone();
        let press = pick.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            // The hitbox, not the bounds: a list open over the slider takes its clicks.
            if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                down.pressed.set(true);
                press(event.position.x, window, cx);
            }
        });
        let held = state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble && held.pressed.get() {
                if event.dragging() {
                    pick(event.position.x, window, cx);
                } else {
                    held.pressed.set(false);
                }
            }
        });
        let up = state.clone();
        window.on_mouse_event(move |_: &MouseUpEvent, _, _, _| up.pressed.set(false));
    };
    div()
        .id(id)
        .flex_none()
        .w(SELECT_WIDTH)
        .h(SLIDER_HEIGHT)
        .cursor_pointer()
        .child(
            canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                paint,
            )
            .size_full(),
        )
}

/// A full-width line across a window-level bar.
pub fn horizontal_line(window: &Window, settings: &Settings) -> Div {
    div()
        .flex_none()
        .w_full()
        .h(hairline(window))
        .bg(settings.colors.popover_border)
}

/// A popover divider from edge to edge: the negative margin cancels the padding.
pub fn separator(window: &Window, settings: &Settings) -> Div {
    div()
        .flex_none()
        .mx_neg_1()
        .h(hairline(window))
        .my_1()
        .bg(settings.colors.popover_border)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slider_snaps_to_the_nearest_stop() {
        assert_eq!(slider_stop(0., 100., 5), 0);
        assert_eq!(slider_stop(12., 100., 5), 0);
        assert_eq!(slider_stop(13., 100., 5), 1);
        assert_eq!(slider_stop(100., 100., 5), 4);
        assert_eq!(slider_stop(-20., 100., 5), 0);
        assert_eq!(slider_stop(180., 100., 5), 4);
        assert_eq!(slider_stop(50., 100., 1), 0);
    }
}
