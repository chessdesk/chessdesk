//! The game sidebar: who you play, the likelihood of each result, what was said about
//! the move on show, and the moves.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use chessdesk_core::review::Verdict;
use chessdesk_core::{Ending, Mark, PlayedMove};
use chessdesk_players::acp::AgentKind;
use chessdesk_players::adapters::{self, Readiness};
use gpui::{
    AnyElement, Axis, Bounds, Context, Corner, Div, Hsla, MouseButton, MouseDownEvent, PathBuilder,
    Pixels, SharedString, Transformation, Window, anchored, canvas, deferred, div, percentage,
    point, prelude::*, px,
};
use shakmaty::{Color, Position};

use super::session::{Feedback, Session};
use super::{Chessdesk, Dragging, clock};
use crate::i18n::{tr, tr_with};
use crate::installs::{self, NeedsStockfish};
use crate::library::{Standing, standing};
use crate::settings::{Settings, side_name};
use crate::setup::OpponentKind;
use crate::ui::{self, Icon, Logo, Transition};

const LIKELIHOOD_BAR_HEIGHT: Pixels = px(20.);
/// White's side of the bar, whatever the theme does with light and dark.
/// A letter's width against the font size, for telling whether a label fits.
const LABEL_LETTER_WIDTH: f32 = 0.56;
/// From the bar's end to a label.
const LABEL_PADDING: Pixels = px(8.);
/// Past this, White's part reaches the bar's rounded right end.
const ALL_OF_THE_BAR: f32 = 0.98;
const GRAPH_HEIGHT: Pixels = px(72.);
const NUMBER_WIDTH: Pixels = px(30.);
/// What the panel's padding takes off the width of the Analyze button's choices.
const ANALYZE_MENU_INSET: Pixels = px(24.);
/// How faint the logo of an agent that isn't set up is, beside its greyed-out name.
const UNAVAILABLE_OPACITY: f32 = 0.4;
/// How long a section of the game panel takes to fold or open.
const FOLD: Duration = Duration::from_millis(200);
/// How much of the room the opponent's section and the moves share the moves may take, at
/// most: in a small window, both give way, so that what was said stays in view.
pub(super) const LOG_SHARE: f32 = 2. / 3.;
/// What every share of the panel's room is multiplied by: see [`grow`].
const GROW_SCALE: f32 = 1000.;
const BUBBLE_TAIL_INSET: Pixels = px(10.);
const BUBBLE_TAIL_WIDTH: Pixels = px(9.);
const BUBBLE_TAIL_HEIGHT: Pixels = px(7.);
/// How far the tail reaches down under the bubble, so that no gap can open between them.
const BUBBLE_TAIL_OVERLAP: Pixels = px(1.);

fn ending_text(ending: Ending) -> SharedString {
    tr(match ending {
        Ending::Checkmate => "ending-checkmate",
        Ending::Stalemate => "ending-stalemate",
        Ending::InsufficientMaterial => "ending-insufficient-material",
        Ending::Time => "ending-time",
        Ending::Resignation => "ending-resignation",
    })
}

fn verdict_text(verdict: Verdict) -> SharedString {
    tr(match verdict {
        Verdict::Perfect => "verdict-perfect",
        Verdict::Good => "verdict-good",
        Verdict::Inaccuracy => "verdict-inaccuracy",
        Verdict::Mistake => "verdict-mistake",
        Verdict::Blunder => "verdict-blunder",
    })
}

/// `?`, `??` and friends, for the glyphs a move carries.
fn glyphs(played: &PlayedMove) -> &'static str {
    played
        .nags
        .iter()
        .find_map(|nag| match nag {
            1 => Some("!"),
            2 => Some("?"),
            3 => Some("!!"),
            4 => Some("??"),
            5 => Some("!?"),
            6 => Some("?!"),
            _ => None,
        })
        .unwrap_or("")
}

/// A number the way the panel shows tokens: `1,234`.
fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut text = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

/// Which ply a press at `x` on the graph means, for a game of `plies`.
fn ply_at(x: f32, width: f32, plies: usize) -> usize {
    if plies == 0 || width <= 0. {
        return 0;
    }
    let share = (x / width).clamp(0., 1.);
    (share * plies as f32).round() as usize
}

fn section(settings: &Settings) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_none()
        .gap_1()
        .px_3()
        .py_2()
        .text_color(settings.colors.text)
}

/// The parts of the game panel that fold into their titles.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    Opponent,
    Likelihood,
    Moves,
    Notes,
}

impl Section {
    const ALL: [Self; 4] = [Self::Opponent, Self::Likelihood, Self::Moves, Self::Notes];
}

/// How far each section of the game panel is open, on its way to open or folded.
pub(super) struct Folds {
    open: [Transition; 4],
    /// How tall what each section holds stands, measured whenever it shows. Only the
    /// sections as tall as what they hold use it: the others share the panel's room.
    heights: [Rc<Cell<Pixels>>; 4],
}

impl Folds {
    /// Everything open but the notes, which are there for when they're wanted.
    pub(super) fn new(now: Instant) -> Self {
        Self {
            open: Section::ALL
                .map(|section| Transition::settled(section != Section::Notes, FOLD, now)),
            heights: Default::default(),
        }
    }

    fn get(&self, section: Section) -> &Transition {
        &self.open[section as usize]
    }

    fn toggle(&mut self, section: Section, now: Instant) {
        let fold = &mut self.open[section as usize];
        fold.set(!fold.is_on(), now);
    }

    /// From 0 for folded to 1 for open.
    fn openness(&self, section: Section, now: Instant) -> f32 {
        self.get(section).value(now)
    }

    /// Folded all the way, and staying so: nothing below its title to draw.
    fn is_folded(&self, section: Section, now: Instant) -> bool {
        let fold = self.get(section);
        !fold.is_on() && !fold.is_moving(now)
    }

    fn is_moving(&self, now: Instant) -> bool {
        self.open.iter().any(|fold| fold.is_moving(now))
    }
}

/// How tall the moves' log stands right now, and how much more the opponent's section above
/// could give it: what dragging the divider between them goes by, rather than the height
/// asked for, which a small window may not have room for.
#[derive(Default)]
pub(super) struct LogSpace {
    shown: Rc<Cell<Pixels>>,
    spare: Rc<Cell<Pixels>>,
}

impl LogSpace {
    pub(super) fn shown(&self) -> Pixels {
        self.shown.get()
    }
}

/// Keeps `into` up to date with the height its parent is drawn at, drawing once more when it
/// changes, so that whatever goes by it never lags a frame behind.
fn measure(into: &Rc<Cell<Pixels>>) -> impl IntoElement {
    let into = into.clone();
    canvas(
        move |bounds, window, _| {
            if into.replace(bounds.size.height) != bounds.size.height {
                window.request_animation_frame();
            }
        },
        |_, (), _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The part of a section below its title that shares the panel's room with the others,
/// taking `weight` of it, down to nothing in a small window however much it holds.
fn shared_body(weight: f32, holds: impl IntoElement) -> Div {
    grow(
        div()
            .flex()
            .flex_col()
            .flex_basis(px(0.))
            .min_h_0()
            .overflow_hidden(),
        weight,
    )
    .child(
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .px_3()
            .pb_2()
            .child(holds),
    )
}

/// Sets how much of the room left a part takes, next to the others asking for some.
fn grow(mut part: Div, weight: f32) -> Div {
    // Weights adding up to less than one hand out only that much of the room and leave the
    // rest empty, as in CSS. Scaled up, only how they compare counts.
    part.style().flex_grow = Some(weight * GROW_SCALE);
    part
}

/// What the agent said: a speech bubble, in the look of an active tab. Its tail points up, to
/// the opponent's name.
fn render_bubble(text: String, settings: &Settings) -> Div {
    let accent = settings.colors.accent;
    let tail = canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let mut path = PathBuilder::fill();
            path.move_to(bounds.origin);
            path.line_to(point(bounds.right(), bounds.bottom()));
            path.line_to(point(bounds.left(), bounds.bottom()));
            path.close();
            if let Ok(path) = path.build() {
                window.paint_path(path, accent);
            }
        },
    )
    .size_full();
    div()
        .flex()
        .flex_col()
        .flex_none()
        .child(
            div()
                .flex_none()
                .mb(-BUBBLE_TAIL_OVERLAP)
                .ml(BUBBLE_TAIL_INSET)
                .w(BUBBLE_TAIL_WIDTH)
                .h(BUBBLE_TAIL_HEIGHT)
                .child(tail),
        )
        .child(
            div()
                .px_2()
                .py_1()
                .rounded(settings.corner_radius)
                .bg(accent)
                .text_color(settings.colors.text)
                .child(text),
        )
}

/// How the game ended, in the colour of how it went for the player.
fn render_result(session: &Session, settings: &Settings) -> Option<Div> {
    let standing = standing(&session.game)?;
    let colors = &settings.colors;
    let (text, color) = match &standing {
        Standing::Won => (tr("result-won"), colors.mark(Mark::Green)),
        Standing::Lost => (tr("result-lost"), colors.mark(Mark::Red)),
        Standing::Draw => (tr("draw"), colors.muted),
        Standing::Others(result) => (result.clone().into(), colors.muted),
    };
    let ending = session
        .game
        .ending()
        .map(|ending| format!(" {}", ending_text(ending)));
    Some(
        div()
            .flex_none()
            .px_2()
            .py_1()
            .rounded(settings.corner_radius * ui::FLAT_RADIUS_SCALE)
            .bg(color.opacity(ui::CARD_TINT))
            .text_color(color)
            .child(format!("{text}{}", ending.unwrap_or_default())),
    )
}

/// How the player's latest move rated: a card like the result's, in the verdict's colour,
/// with what the move did to the likelihood and what would have been better.
fn render_feedback(feedback: &Feedback, settings: &Settings) -> Div {
    let colors = &settings.colors;
    let color = match feedback.verdict {
        Verdict::Perfect => colors.mark(Mark::Green),
        Verdict::Good => colors.text,
        Verdict::Inaccuracy => colors.mark(Mark::Yellow),
        Verdict::Mistake => colors.callout,
        Verdict::Blunder => colors.mark(Mark::Red),
    };
    // The rest in the card's own colour, quieter than the verdict.
    let detail = color.opacity(ui::CARD_DETAIL);
    let percent = |share: f32| (share * 100.).round() as u32;
    let likelihood = feedback.likelihood.map(|(before, after)| {
        tr_with!(
            "verdict-likelihood",
            before = percent(before),
            after = percent(after)
        )
    });
    let best = feedback
        .best
        .as_ref()
        .map(|best| tr_with!("verdict-best", best = best.as_str()));
    div()
        .flex()
        .flex_col()
        .flex_none()
        .gap_0p5()
        .px_2()
        .py_1()
        .rounded(settings.corner_radius * ui::FLAT_RADIUS_SCALE)
        .bg(color.opacity(ui::CARD_TINT))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_color(color)
                        .child(verdict_text(feedback.verdict)),
                )
                .child(div().text_color(detail).child(feedback.numbered.clone())),
        )
        .children(likelihood.map(|text| div().text_color(detail).child(text)))
        .children(best.map(|text| div().text_color(detail).child(text)))
}

/// Whether a side's name fits in its part of the bar. Going by a typical letter's width,
/// which is close enough to decide when to drop it.
fn label_fits(label: &str, room: Pixels, settings: &Settings) -> bool {
    let letters = label.chars().count() as f32;
    room >= settings.ui_font_size * LABEL_LETTER_WIDTH * letters + LABEL_PADDING * 2.
}

/// A side's name, at its end of the bar.
fn bar_label(label: SharedString, color: Hsla) -> Div {
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .flex()
        .items_center()
        .text_color(color)
        .child(label)
}

/// One bar for both sides, as on a chess site: White's part grows from the left, and it
/// rests in the middle. With `None`, nothing is known and only the frame shows. The parts
/// are round themselves, as a parent's rounding doesn't clip what's inside it.
fn likelihood_bar(white: Option<f32>, width: Pixels, settings: &Settings) -> Div {
    let radius = settings.corner_radius * ui::FLAT_RADIUS_SCALE;
    let bar = div()
        .relative()
        .flex_none()
        .w(width)
        .h(LIKELIHOOD_BAR_HEIGHT)
        .rounded(radius)
        .text_size(settings.ui_font_size * 0.92);
    let Some(white) = white else {
        return bar.bg(settings.colors.accent);
    };
    // The theme's own board colours: White and Black that belong to the theme, and labels that
    // read on both without a stark black and white.
    let (light, dark) = (settings.colors.board_light, settings.colors.board_dark);
    let label = settings.colors.text;
    let (white_label, black_label) = (tr("side-white"), tr("side-black"));
    let shows_white = label_fits(&white_label, width * white, settings);
    let shows_black = label_fits(&black_label, width * (1. - white), settings);
    bar.bg(dark)
        .child(
            div()
                .h_full()
                .w(gpui::relative(white))
                .rounded_l(radius)
                .when(white > ALL_OF_THE_BAR, |part| part.rounded_r(radius))
                .bg(light),
        )
        .when(shows_white, |bar| {
            bar.child(bar_label(white_label, label).left(LABEL_PADDING))
        })
        .when(shows_black, |bar| {
            bar.child(bar_label(black_label, label).right(LABEL_PADDING))
        })
}

/// What's plotted, and where the cursor is, if it shows.
struct Graph {
    points: Vec<(usize, f32)>,
    plies: usize,
    cursor: Option<usize>,
    line: Hsla,
    marker: Hsla,
}

impl Graph {
    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let at = |ply: usize, value: f32| {
            let x = ply as f32 / self.plies.max(1) as f32 * width;
            point(
                bounds.left() + px(x),
                bounds.top() + px((1. - value) * height),
            )
        };
        let middle = [at(0, 0.5), at(self.plies, 0.5)];
        draw_line(px(1.), &middle, self.marker.opacity(0.4), window);
        if let Some(ply) = self.cursor {
            draw_line(px(1.), &[at(ply, 0.), at(ply, 1.)], self.marker, window);
        }
        let curve: Vec<_> = self
            .points
            .iter()
            .map(|(ply, value)| at(*ply, *value))
            .collect();
        draw_line(px(1.5), &curve, self.line, window);
    }
}

fn draw_line(width: Pixels, points: &[gpui::Point<Pixels>], color: Hsla, window: &mut Window) {
    let Some((first, rest)) = points.split_first() else {
        return;
    };
    let mut path = PathBuilder::stroke(width);
    path.move_to(*first);
    for point in rest {
        path.line_to(*point);
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

impl Chessdesk {
    pub(super) fn render_panel(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let now = Instant::now();
        if self.folds.is_moving(now) {
            window.request_animation_frame();
        }
        let blank = self.session().is_blank();
        let line = || ui::horizontal_line(window, &settings);
        // The opponent's section takes the room the others leave, the moves what it leaves
        // as it folds, and with both folded the room stays empty above the buttons.
        let opponent = if blank {
            1.
        } else {
            self.folds.openness(Section::Opponent, now)
        };
        let moves = self.folds.openness(Section::Moves, now);
        let opponent_open = blank || self.folds.get(Section::Opponent).is_on();
        // Sharing the room by dragging only means something while both of them take some.
        let above_moves = if opponent_open && self.folds.get(Section::Moves).is_on() {
            self.render_divider(
                "log-divider",
                Axis::Vertical,
                move |event: &MouseDownEvent, this: &Self| {
                    let height = this.log_space.shown.get();
                    Dragging::Log {
                        from: event.position.y,
                        height,
                        most: (height + this.log_space.spare.get()) * LOG_SHARE,
                    }
                },
                window,
                cx,
            )
        } else {
            line()
        };

        let mut column = div().flex().flex_col().h_full().w(self.panel_width);
        let (opponent_title, opponent_body) = self.render_opponent(opponent, cx);
        column = column.child(opponent_title).children(opponent_body);
        if let Some(likelihood) = self.render_likelihood(window, cx) {
            column = column.child(line()).child(likelihood);
        }
        let (moves_title, moves_body) = self.render_moves(opponent, moves, cx);
        column = column
            .child(above_moves)
            .child(moves_title)
            .children(moves_body);
        // With no game there's nothing to note or act on, and the new game button is on the
        // board's place.
        if !blank {
            column = column
                .child(line())
                .child(self.render_notes(cx))
                .child(line());
            let empty = (1. - opponent) * (1. - moves);
            if empty > 0. {
                column = column.child(
                    grow(div().flex_basis(px(0.)), empty)
                        .border_b(ui::hairline(window))
                        .border_color(settings.colors.popover_border),
                );
            }
            // Whatever room is left over goes above them: the buttons keep to the bottom.
            column = column.child(self.render_actions(window, cx).mt_auto());
        }
        div()
            .flex()
            .flex_none()
            .h_full()
            .child(self.render_divider(
                "panel-divider",
                Axis::Horizontal,
                |_, _| Dragging::Panel,
                window,
                cx,
            ))
            .child(column)
            .into_any_element()
    }

    /// A section's title row. Pressing the title folds the section into the row, or opens it
    /// again, the chevron turning with it; anything added as a child sits at the row's far end.
    fn section_title(
        &self,
        section: Section,
        title: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> Div {
        let settings = Settings::get(cx);
        let text = settings.colors.text;
        let open = self.folds.openness(section, Instant::now());
        let chevron = ui::icon(
            Icon::ChevronRight,
            settings.ui_font_size,
            settings.colors.muted,
        )
        .with_transformation(Transformation::rotate(percentage(open / 4.)));
        let fold = div()
            .id(("fold", section as usize))
            .flex()
            .flex_1()
            .min_w_0()
            .items_center()
            .gap_1()
            .cursor_pointer()
            .hover(move |style| style.text_color(text))
            .child(chevron)
            .child(title)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.folds.toggle(section, Instant::now());
                cx.notify();
            }));
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .gap_2()
            .text_color(settings.colors.muted)
            .child(fold)
    }

    /// What a section as tall as what it holds shows below its title: all of it while open,
    /// and while it folds or opens, as much of its height as it's open. `None` once folded.
    fn fold_body(&self, section: Section, body: Div) -> Option<Div> {
        let now = Instant::now();
        if self.folds.is_folded(section, now) {
            return None;
        }
        let body = body
            .flex_none()
            .child(measure(&self.folds.heights[section as usize]));
        let fold = self.folds.get(section);
        if fold.is_on() && !fold.is_moving(now) {
            return Some(body);
        }
        let shown = self.folds.heights[section as usize].get() * fold.value(now);
        Some(div().flex_none().h(shown).overflow_hidden().child(body))
    }

    /// The opponent's name, with the tokens used so far on hover, and whether they're busy.
    fn render_opponent_title(&self, cx: &mut Context<Self>) -> Div {
        let settings = Settings::get(cx);
        let session = self.session();
        let size = settings.ui_font_size;
        let muted = settings.colors.muted;
        let thinking = session.live.as_ref().is_some_and(|live| live.thinking);
        let busy = thinking || session.reviewer.is_some() || session.analysis.is_some();
        let tokens = session.tokens.map(|tokens| {
            ui::tooltip(tr_with!(
                "tokens-used",
                total = grouped(tokens.total),
                cached = grouped(tokens.cached)
            ))
        });
        let name = div()
            .id("opponent")
            .truncate()
            .child(session.opponent_name())
            .when_some(tokens, gpui::StatefulInteractiveElement::tooltip);
        let progress = session.analysis.map(|(done, all)| {
            let progress = tr_with!("progress", done = done, all = all);
            div().text_color(muted).child(progress)
        });
        self.section_title(
            Section::Opponent,
            div()
                .flex()
                .min_w_0()
                .items_center()
                .gap_1p5()
                .text_color(settings.colors.text)
                .child(name)
                .when(busy, |row| row.child(ui::spinner("busy", size, muted)))
                .children(progress),
            cx,
        )
    }

    /// Who you're playing, whether they're thinking, and what they said about the move: the
    /// title row, and below it what takes as much of the room left over as it's `open`.
    fn render_opponent(&self, open: f32, cx: &mut Context<Self>) -> (AnyElement, Option<Div>) {
        let settings = Settings::get(cx);
        let settings = &settings;
        let session = self.session();
        if session.is_blank() {
            return (ui::empty_panel(settings).into_any_element(), None);
        }
        let title = section(settings).child(self.render_opponent_title(cx));
        if self.folds.is_folded(Section::Opponent, Instant::now()) {
            return (title.into_any_element(), None);
        }
        let muted = settings.colors.muted;
        let time = |color: Color| {
            let clock = session.live.as_ref()?.clock.as_ref()?;
            let left = clock::format(clock.left(color, Instant::now()));
            Some(format!("{} {left}", side_name(color)))
        };
        let clocks = time(Color::White)
            .zip(time(Color::Black))
            .map(|(white, black)| {
                div()
                    .flex()
                    .justify_between()
                    .text_color(muted)
                    .child(white)
                    .child(black)
            });
        let comment = session.shown_comment();
        let paused = session.live.as_ref().and_then(|live| live.paused.clone());
        let instructions = session
            .live
            .as_ref()
            .and_then(|live| live.instructions.as_ref())
            .map(|(area, _)| {
                div()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .gap_1()
                    .pt_2()
                    .child(div().text_color(muted).child(tr("agent-instructions")))
                    .child(area.clone())
            });
        let said = div()
            .id("opponent-section")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_2()
            .overflow_y_scroll()
            .children(clocks)
            .children(render_result(session, settings))
            .children(
                session
                    .latest_feedback()
                    .map(|feedback| render_feedback(&feedback, settings)),
            )
            .children(comment.map(|text| render_bubble(text, settings)))
            .children(paused.map(|reason| div().text_color(settings.colors.callout).child(reason)));
        let holds = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(said)
            .children(instructions);
        let body =
            shared_body(open * (1. - LOG_SHARE), holds).child(measure(&self.log_space.spare));
        (title.into_any_element(), Some(body))
    }

    fn render_likelihood(&self, window: &Window, cx: &mut Context<Self>) -> Option<Div> {
        let settings = Settings::get(cx);
        let session = self.session();
        if !session.shows_likelihood() {
            return None;
        }
        let title = self
            .section_title(Section::Likelihood, tr("win-likelihood"), cx)
            .child(
                div()
                    .id("likelihood-source")
                    .child(ui::logo(
                        Logo::Stockfish,
                        settings.ui_font_size * ui::LOGO_SCALE,
                        settings.colors.text,
                    ))
                    .tooltip(ui::tooltip(tr("likelihood-source"))),
            );
        if self.folds.is_folded(Section::Likelihood, Instant::now()) {
            return Some(section(&settings).child(title));
        }
        // Switched on in a game without Stockfish: the way to it, not a graph that stays empty.
        if session.live.is_some() && OpponentKind::Stockfish.missing(&settings).is_some() {
            let notice =
                installs::stockfish_notice(NeedsStockfish::Likelihood, &self.installs.0, cx);
            let body = div().pt_3().child(notice);
            return Some(
                section(&settings)
                    .gap_0()
                    .child(title)
                    .children(self.fold_body(Section::Likelihood, body)),
            );
        }
        let start = session.start_wdl().map(|wdl| (0, wdl.expected()));
        let moves = session.game.moves().iter().enumerate();
        let later =
            moves.filter_map(|(index, played)| Some((index + 1, played.eval.wdl?.expected())));
        let points: Vec<_> = start.into_iter().chain(later).collect();

        // Always there, so that the section keeps its shape between evaluations.
        let now = Instant::now();
        if self.likelihood.is_moving(now) {
            window.request_animation_frame();
        }
        let white = session.known_wdl().map(|_| self.likelihood.value(now));
        let summary = likelihood_bar(white, self.panel_width - ANALYZE_MENU_INSET, &settings);

        let plies = session.game.moves().len();
        let plot = Graph {
            points,
            plies,
            // Only a replay has a position to point at: in a game, it's always the last.
            cursor: session.live.is_none().then_some(session.ply),
            line: settings.colors.caret,
            marker: settings.colors.muted,
        };
        let width = Rc::new(Cell::new((px(0.), px(1.))));
        let measured = width.clone();
        let graph = canvas(
            move |bounds, _, _| measured.set((bounds.left(), bounds.size.width)),
            move |bounds, (), window, _| plot.paint(bounds, window),
        )
        .size_full();
        let press = cx.listener(move |this, event: &MouseDownEvent, _, cx| {
            let (left, wide) = width.get();
            let x = f32::from(event.position.x - left);
            this.step_to(ply_at(x, f32::from(wide), plies), cx);
        });

        let body = div().flex().flex_col().gap_3().pt_3().child(summary).child(
            div()
                .id("graph")
                .h(GRAPH_HEIGHT)
                .w_full()
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, press)
                .child(graph),
        );
        Some(
            section(&settings)
                .gap_0()
                .child(title)
                .children(self.fold_body(Section::Likelihood, body)),
        )
    }

    fn render_move(&self, index: usize, cx: &mut Context<Self>) -> Option<AnyElement> {
        let settings = Settings::get(cx);
        let session = self.session();
        let played = session.game.moves().get(index)?;
        let text = format!("{}{}", played.san, glyphs(played));
        let ply = index + 1;
        Some(
            ui::button(("move", index), session.ply == ply, &settings)
                .w_full()
                .text_color(settings.colors.text)
                .child(text)
                .on_click(cx.listener(move |this, _, _, cx| this.step_to(ply, cx)))
                .into_any_element(),
        )
    }

    /// The moves, in a log as tall as the divider above it sets, or its share of the room in a
    /// window too small for that, while the opponent's section is `opponent` open; taking the
    /// room it leaves as it folds; as much of either as the log itself is `open`.
    fn render_moves(&self, opponent: f32, open: f32, cx: &mut Context<Self>) -> (Div, Option<Div>) {
        let settings = Settings::get(cx);
        let session = self.session();
        let ply = session.ply;
        let steps = div()
            .flex()
            .flex_none()
            .gap_1()
            .child(
                ui::square_button("previous-move", Icon::ChevronLeft, &settings)
                    .tooltip(ui::tooltip(tr("previous-move")))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.step_to(ply.saturating_sub(1), cx);
                    })),
            )
            .child(
                ui::square_button("next-move", Icon::ChevronRight, &settings)
                    .tooltip(ui::tooltip(tr("next-move")))
                    .on_click(cx.listener(move |this, _, _, cx| this.step_to(ply + 1, cx))),
            );
        // The steps stay when it's folded: going through the game needs no log.
        let title = section(&settings).child(
            self.section_title(Section::Moves, tr("moves"), cx)
                .child(steps),
        );
        if self.folds.is_folded(Section::Moves, Instant::now()) {
            return (title, None);
        }

        let start = session.game.start();
        let first_number = start.fullmoves().get() as usize;
        let offset = usize::from(start.turn() == Color::Black);
        let rows = (session.game.moves().len() + offset).div_ceil(2);
        let lines: Vec<_> = (0..rows)
            .map(|row| {
                let white = (row * 2).checked_sub(offset);
                let black = row * 2 + 1 - offset;
                // Two equal columns fill what the number leaves, up to the panel's edge.
                let column = || div().flex_1().flex_basis(px(0.)).min_w_0();
                div()
                    .flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .w(NUMBER_WIDTH)
                            .flex_none()
                            .text_color(settings.colors.muted)
                            .child(format!("{}.", first_number + row)),
                    )
                    .child(column().children(white.and_then(|index| self.render_move(index, cx))))
                    .child(column().children(self.render_move(black, cx)))
            })
            .collect();
        let empty = lines.is_empty().then(|| ui::empty_panel(&settings));
        let log = div()
            .id("moves")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .w_full()
            .gap_0p5()
            .overflow_y_scroll()
            .track_scroll(&self.moves_scroll)
            .children(lines)
            .children(empty);
        // Its share of the room, up to the height asked for while the opponent's section is
        // open, and without bound as that folds away.
        let most = self.log_height * open / opponent.max(f32::EPSILON);
        let body = shared_body(open * LOG_SHARE, log)
            .max_h(most)
            .child(measure(&self.log_space.shown));
        (title, Some(body))
    }

    /// The player's own notes on the game, kept in its file.
    fn render_notes(&self, cx: &mut Context<Self>) -> Div {
        let settings = Settings::get(cx);
        let title = self.section_title(Section::Notes, tr("notes"), cx);
        let body = self
            .session()
            .notes
            .as_ref()
            .map(|(area, _)| div().flex().flex_col().pt_1().child(area.clone()))
            .and_then(|body| self.fold_body(Section::Notes, body));
        section(&settings).gap_0().child(title).children(body)
    }

    /// The Analyze button: its choices open upwards, from the bottom of the panel.
    fn render_analyze(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let session = self.session();
        if session.analysis.is_some() || session.reviewer.is_some() {
            return ui::busy_button("analyze", tr("analyze"), &settings)
                .w_full()
                .into_any_element();
        }
        let open = self.analyze_menu;
        let button = ui::action_button("analyze", tr("analyze"), &settings)
            .w_full()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.analyze_menu = !open;
                    cx.notify();
                }),
            );
        let list = open.then(|| {
            let stockfish = ui::list_row("analyze-stockfish", false, &settings)
                .gap_1p5()
                .text_color(settings.colors.text)
                .child(ui::logo(
                    Logo::Stockfish,
                    settings.ui_font_size * ui::LOGO_SCALE,
                    settings.colors.text,
                ))
                .child("Stockfish")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.analyze_menu = false;
                    this.analyze(cx);
                }));
            let folder = Settings::adapters_dir().unwrap_or_default();
            // Every agent, greyed out until it's set up, so that they can all be found.
            let agents = AgentKind::ALL.into_iter().enumerate().map(|(index, kind)| {
                let ready = adapters::readiness(kind, &folder) == Readiness::Ready;
                let row = ui::list_row(("analyze-agent", index), false, &settings)
                    .gap_1p5()
                    .child(
                        div()
                            .when(!ready, |mark| mark.opacity(UNAVAILABLE_OPACITY))
                            .child(ui::logo(
                                Logo::of_agent(kind),
                                settings.ui_font_size * ui::LOGO_SCALE,
                                settings.colors.text,
                            )),
                    )
                    .child(kind.name());
                if ready {
                    row.text_color(settings.colors.text).on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.analyze_menu = false;
                            this.review_with(kind, cx);
                        },
                    ))
                } else {
                    row.cursor_default().text_color(settings.colors.muted)
                }
            });
            deferred(
                anchored().anchor(Corner::BottomLeft).child(
                    div().pb_1().child(ui::rise_in(
                        "analyze-menu-in",
                        ui::popover(window, &settings)
                            .id("analyze-menu")
                            .occlude()
                            .w(self.panel_width - ANALYZE_MENU_INSET)
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                this.analyze_menu = false;
                                cx.notify();
                            }))
                            .child(stockfish)
                            .children(agents),
                    )),
                ),
            )
            .with_priority(1)
        });
        div()
            .relative()
            .w_full()
            .children(list)
            .child(button)
            .into_any_element()
    }

    /// Full-width buttons, one above the other.
    fn render_actions(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let settings = Settings::get(cx);
        let session = self.session();
        let wide = |id: &'static str, label: &'static str| {
            ui::action_button(id, tr(label), &settings).w_full()
        };
        let mut buttons = section(&settings);
        // What the engine would play, for the position on show.
        let suggest = |label: &'static str, cx: &mut Context<Self>| {
            if session.suggesting {
                ui::busy_button("suggest", label, &settings)
                    .w_full()
                    .into_any_element()
            } else {
                wide("suggest", label)
                    .on_click(cx.listener(|this, _, _, cx| this.suggest(cx)))
                    .into_any_element()
            }
        };
        if let Some(live) = &session.live {
            let first = if live.paused.is_some() {
                wide("resume", "continue").on_click(cx.listener(|this, _, _, cx| this.resume(cx)))
            } else {
                wide("take-back", "take-back")
                    .on_click(cx.listener(|this, _, _, cx| this.take_back(cx)))
            };
            // Greyed out rather than gone while it's not your move, so the buttons below stay put.
            let hint = if session.player_to_move() {
                suggest("hint", cx)
            } else {
                ui::disabled_button("suggest", tr("hint"), &settings)
                    .w_full()
                    .into_any_element()
            };
            buttons = buttons.child(hint).child(first).child(
                wide("resign", "resign").on_click(cx.listener(|this, _, _, cx| this.resign(cx))),
            );
        } else {
            let played = !session.game.moves().is_empty();
            let optimal = played.then(|| suggest("optimal-move", cx));
            let analyze = played.then(|| self.render_analyze(window, cx));
            // What's next comes last, filled, as the way on from a game that's done.
            buttons = buttons.children(optimal).children(analyze).child(
                ui::primary_button("new", tr("new-game"), &settings)
                    .w_full()
                    .on_click(cx.listener(|this, _, _, cx| this.show_new_game(cx))),
            );
        }
        buttons
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_thousands() {
        assert_eq!(grouped(868), "868");
        assert_eq!(grouped(1234), "1,234");
        assert_eq!(grouped(1234567), "1,234,567");
    }

    #[test]
    fn presses_on_the_graph_land_on_a_move() {
        assert_eq!(ply_at(0., 100., 40), 0);
        assert_eq!(ply_at(50., 100., 40), 20);
        assert_eq!(ply_at(150., 100., 40), 40);
        assert_eq!(ply_at(10., 100., 0), 0);
    }
}
