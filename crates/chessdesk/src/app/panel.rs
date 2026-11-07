//! The game sidebar: who you play, the likelihood of each result, what was said about
//! the move on show, and the moves.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use chessdesk_core::review::Verdict;
use chessdesk_core::{Ending, Mark, PlayedMove};
use chessdesk_players::acp::AgentKind;
use chessdesk_players::adapters::{self, Readiness};
use gpui::{
    AnyElement, Axis, Bounds, Context, Corner, Div, Hsla, MouseButton, MouseDownEvent, PathBuilder,
    Pixels, SharedString, Window, anchored, canvas, deferred, div, point, prelude::*, px, rgb,
};
use shakmaty::{Color, Position};

use super::session::Session;
use super::{Chessdesk, Dragging, clock};
use crate::i18n::{tr, tr_with};
use crate::library::{Standing, standing};
use crate::settings::{Settings, side_name};
use crate::ui::{self, Icon, Logo};

const LIKELIHOOD_BAR_HEIGHT: Pixels = px(20.);
/// White's side of the bar, whatever the theme does with light and dark.
const WHITE_SIDE: u32 = 0xf0f0ee;
/// How round wide, flat shapes are against the corner radius: on them it looks stronger
/// than it does on a button.
const FLAT_RADIUS_SCALE: f32 = 0.7;
/// A letter's width against the font size, for telling whether a label fits.
const LABEL_LETTER_WIDTH: f32 = 0.56;
/// From the bar's end to a label.
const LABEL_PADDING: Pixels = px(8.);
/// Past this, White's part reaches the bar's rounded right end.
const ALL_OF_THE_BAR: f32 = 0.98;
const GRAPH_HEIGHT: Pixels = px(72.);
const NUMBER_WIDTH: Pixels = px(30.);
/// How strongly the result's colour tints its background.
const RESULT_TINT: f32 = 0.16;
/// What the panel's padding takes off the width of the Analyze button's choices.
const ANALYZE_MENU_INSET: Pixels = px(24.);
const BUBBLE_TAIL_INSET: Pixels = px(10.);
const BUBBLE_TAIL_WIDTH: Pixels = px(9.);
const BUBBLE_TAIL_HEIGHT: Pixels = px(7.);
/// How far the tail reaches up under the bubble, so that no gap can open between them.
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

/// A section's heading, with anything added as a child at its far end.
fn section_title(title: impl Into<SharedString>, settings: &Settings) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_between()
        .text_color(settings.colors.muted)
        .child(title.into())
}

/// What the agent said: a speech bubble, in the look of an active tab.
fn render_bubble(text: String, settings: &Settings) -> Div {
    let accent = settings.colors.accent;
    let tail = canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let mut path = PathBuilder::fill();
            path.move_to(bounds.origin);
            path.line_to(point(bounds.right(), bounds.top()));
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
                .px_2()
                .py_1()
                .rounded(settings.corner_radius)
                .bg(accent)
                .text_color(settings.colors.text)
                .child(text),
        )
        .child(
            div()
                .flex_none()
                .mt(-BUBBLE_TAIL_OVERLAP)
                .ml(BUBBLE_TAIL_INSET)
                .w(BUBBLE_TAIL_WIDTH)
                .h(BUBBLE_TAIL_HEIGHT)
                .child(tail),
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
            .rounded(settings.corner_radius * FLAT_RADIUS_SCALE)
            .bg(color.opacity(RESULT_TINT))
            .text_color(color)
            .child(format!("{text}{}", ending.unwrap_or_default())),
    )
}

/// How the player's latest move rated.
fn render_verdict(played: &PlayedMove, verdict: Verdict, settings: &Settings) -> Div {
    let colors = &settings.colors;
    let color = match verdict {
        Verdict::Perfect => colors.mark(Mark::Green),
        Verdict::Good => colors.text,
        Verdict::Inaccuracy => colors.mark(Mark::Yellow),
        Verdict::Mistake => colors.callout,
        Verdict::Blunder => colors.mark(Mark::Red),
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap_1p5()
        .child(div().text_color(color).child(verdict_text(verdict)))
        .child(div().text_color(colors.muted).child(played.san.to_string()))
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

/// Black's side of the bar: a dark colour of the theme that stands out from the panel, which
/// is the accent on dark themes, while on light ones it's the text.
fn black_side(settings: &Settings) -> Hsla {
    let colors = &settings.colors;
    if colors.background.l < 0.5 {
        colors.accent
    } else {
        colors.text
    }
}

/// One bar for both sides, as on a chess site: White's part grows from the left, and it
/// rests in the middle. With `None`, nothing is known and only the frame shows. The parts
/// are round themselves, as a parent's rounding doesn't clip what's inside it.
fn likelihood_bar(white: Option<f32>, width: Pixels, settings: &Settings) -> Div {
    let radius = settings.corner_radius * FLAT_RADIUS_SCALE;
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
    let light: Hsla = rgb(WHITE_SIDE).into();
    let dark = black_side(settings);
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
            bar.child(bar_label(white_label, dark).left(LABEL_PADDING))
        })
        .when(shows_black, |bar| {
            bar.child(bar_label(black_label, light).right(LABEL_PADDING))
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
        let session = self.session();
        let height = self.log_height;
        let log_divider = Self::render_divider(
            "log-divider",
            Axis::Vertical,
            move |event: &MouseDownEvent, this: &Self| Dragging::Log {
                from: event.position.y,
                height: this.log_height,
            },
            window,
            cx,
        );
        let likelihood = self.render_likelihood(window, cx);
        let line_below = likelihood
            .is_some()
            .then(|| ui::horizontal_line(window, &settings));
        // With no game there's nothing to act on, and the new game button is on the board's place.
        let actions = (!session.is_blank()).then(|| {
            div()
                .flex()
                .flex_col()
                .flex_none()
                .child(ui::horizontal_line(window, &settings))
                .child(self.render_actions(window, cx))
        });
        div()
            .flex()
            .flex_none()
            .h_full()
            .child(Self::render_divider(
                "panel-divider",
                Axis::Horizontal,
                |_, _| Dragging::Panel,
                window,
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .h_full()
                    .w(self.panel_width)
                    .child(Self::render_opponent(session, &settings))
                    .children(line_below)
                    .children(likelihood)
                    .child(log_divider)
                    .child(self.render_moves(height, cx))
                    .children(actions),
            )
            .into_any_element()
    }

    /// Who you're playing, whether they're thinking, and what they said about the move.
    /// Takes the room left over.
    fn render_opponent(session: &Session, settings: &Settings) -> impl IntoElement + use<> {
        if session.is_blank() {
            return ui::empty_panel(settings);
        }
        let size = settings.ui_font_size;
        let muted = settings.colors.muted;
        let thinking = session.live.as_ref().is_some_and(|live| live.thinking);
        let busy = thinking || session.agent_review || session.analysis.is_some();
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
                    .px_3()
                    .pb_2()
                    .child(div().text_color(muted).child(tr("agent-instructions")))
                    .child(area.clone())
            });
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .id("opponent-section")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .overflow_y_scroll()
                    .text_color(settings.colors.text)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(name)
                            .when(busy, |row| row.child(ui::spinner("busy", size, muted)))
                            .children(progress),
                    )
                    .children(clocks)
                    .children(render_result(session, settings))
                    .children(
                        session
                            .latest_verdict()
                            .map(|(played, verdict)| render_verdict(played, verdict, settings)),
                    )
                    .children(comment.map(|text| render_bubble(text, settings)))
                    .children(
                        paused
                            .map(|reason| div().text_color(settings.colors.callout).child(reason)),
                    ),
            )
            .children(instructions)
    }

    fn render_likelihood(&self, window: &Window, cx: &mut Context<Self>) -> Option<Div> {
        let settings = Settings::get(cx);
        let session = self.session();
        if !session.shows_likelihood() {
            return None;
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

        Some(
            section(&settings)
                .gap_3()
                .child(
                    section_title(tr("win-likelihood"), &settings).child(ui::logo(
                        Logo::Stockfish,
                        settings.ui_font_size * ui::LOGO_SCALE,
                    )),
                )
                .child(summary)
                .child(
                    div()
                        .id("graph")
                        .h(GRAPH_HEIGHT)
                        .w_full()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, press)
                        .child(graph),
                ),
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

    fn render_moves(&self, height: Pixels, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let settings = Settings::get(cx);
        let session = self.session();
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
        section(&settings)
            .h(height)
            .child(section_title(tr("moves"), &settings).child(steps))
            .child(
                div()
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
                    .children(empty),
            )
    }

    /// The Analyze button: its choices open upwards, from the bottom of the panel.
    fn render_analyze(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let settings = Settings::get(cx);
        let session = self.session();
        if session.analysis.is_some() || session.agent_review {
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
                ))
                .child("Stockfish")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.analyze_menu = false;
                    this.analyze(cx);
                }));
            let folder = Settings::adapters_dir().unwrap_or_default();
            let agents = AgentKind::ALL
                .into_iter()
                .filter(|kind| adapters::readiness(*kind, &folder) == Readiness::Ready)
                .enumerate()
                .map(|(index, kind)| {
                    let mark = match kind {
                        AgentKind::ClaudeCode => Some(Logo::ClaudeCode),
                        AgentKind::Codex => Some(Logo::Codex),
                        _ => None,
                    };
                    ui::list_row(("analyze-agent", index), false, &settings)
                        .gap_1p5()
                        .text_color(settings.colors.text)
                        .children(
                            mark.map(|mark| ui::logo(mark, settings.ui_font_size * ui::LOGO_SCALE)),
                        )
                        .child(kind.name())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.analyze_menu = false;
                            this.review_with(kind, cx);
                        }))
                });
            deferred(
                anchored().anchor(Corner::BottomLeft).child(
                    div().pb_1().child(
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
                    ),
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
            buttons = buttons
                .children(optimal)
                .child(
                    wide("new", "new-game")
                        .on_click(cx.listener(|this, _, _, cx| this.show_new_game(cx))),
                )
                .children(analyze);
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
