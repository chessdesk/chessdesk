//! Drawing the board: squares, pieces, what's marked on them, and arrows above it all.
//! What happens on a press is the app's business; this only reports it.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use chessdesk_core::{Annotations, Arrow};
use gpui::{
    Animation, AnimationExt, AnyElement, App, Bounds, Context, Div, Hsla, Image, MouseButton,
    MouseDownEvent, ObjectFit, PathBuilder, Pixels, Point, Window, canvas, div, ease_in_out, img,
    point, prelude::*, px,
};
use shakmaty::{CastlingSide, Chess, Color, Move, Piece, Position, Role, Square};

use crate::settings::Settings;

/// How long a piece takes to glide to its square.
pub const SLIDE_TIME: Duration = Duration::from_millis(180);
const LIGHT_MARK: f32 = 0.45;
const ARROW_OPACITY: f32 = 0.8;

/// A piece on its way between squares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlidePiece {
    pub piece: Piece,
    pub from: Square,
    pub to: Square,
}

/// The pieces of a move gliding into place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slide {
    /// New for each slide, so that each animation starts over.
    pub id: u64,
    pub pieces: Vec<SlidePiece>,
}

/// What a move carries across the board: the piece, and on castling the rook after the
/// king. The first entry is what a player would call the move.
pub fn slide_paths(played: Move) -> Vec<(Square, Square)> {
    match played {
        Move::Normal { from, to, .. } | Move::EnPassant { from, to } => vec![(from, to)],
        Move::Castle { king, rook } => {
            let side = CastlingSide::from_queen_side(rook.file() < king.file());
            let rank = king.rank();
            vec![
                (king, Square::from_coords(side.king_to_file(), rank)),
                (rook, Square::from_coords(side.rook_to_file(), rank)),
            ]
        }
        Move::Put { .. } => Vec::new(),
    }
}

/// The pieces to glide when `shown` comes up by playing a move, or by taking it back.
pub fn slide_pieces(played: Move, shown: &Chess, forward: bool) -> Vec<SlidePiece> {
    slide_paths(played)
        .into_iter()
        .map(|(from, to)| if forward { (from, to) } else { (to, from) })
        .filter_map(|(from, to)| {
            let piece = shown.board().piece_at(to)?;
            Some(SlidePiece { piece, from, to })
        })
        .collect()
}

/// What the board should show right now.
pub struct View {
    pub position: Chess,
    /// Whose side faces the player.
    pub orientation: Color,
    pub selected: Option<Square>,
    /// Where the selected piece can go.
    pub targets: Vec<Square>,
    pub last_move: Option<(Square, Square)>,
    pub annotations: Annotations,
    pub size: Pixels,
    pub slides: Vec<Slide>,
}

/// Dragged from a square: which one, and how it looks under the mouse.
#[derive(Clone)]
pub struct DraggedPiece {
    pub from: Square,
    image: Arc<Image>,
    size: Pixels,
}

impl Render for DraggedPiece {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        piece_image(self.image.clone(), self.size)
    }
}

fn piece_image(image: Arc<Image>, size: Pixels) -> gpui::Img {
    img(image).size(size).object_fit(ObjectFit::Contain)
}

/// Where a square is drawn, counted in squares from the top left.
fn grid_position(square: Square, orientation: Color) -> (u32, u32) {
    let (file, rank) = (u32::from(square.file()), u32::from(square.rank()));
    if orientation.is_white() {
        (file, 7 - rank)
    } else {
        (7 - file, rank)
    }
}

/// The centre of a square within the board.
fn center(square: Square, orientation: Color, size: Pixels) -> Point<Pixels> {
    let (column, row) = grid_position(square, orientation);
    let cell = size / 8.;
    point(cell * (column as f32 + 0.5), cell * (row as f32 + 0.5))
}

type PressHandler = Rc<dyn Fn(Square, &mut Window, &mut App)>;
type DropHandler = Rc<dyn Fn(Square, Square, &mut Window, &mut App)>;

/// Which handlers the board calls.
pub struct Handlers {
    pub press: PressHandler,
    pub drop: DropHandler,
}

pub fn render(view: &View, handlers: &Handlers, cx: &App) -> Div {
    let settings = Settings::get(cx);
    let cell = view.size / 8.;
    let squares = Square::ALL
        .into_iter()
        .map(|square| render_square(square, view, handlers, cell, &settings, cx));

    let picture = settings
        .board
        .image
        .clone()
        .map(|image| img(image).absolute().size(cell * 8.));
    let mut slides = Vec::new();
    for slide in &view.slides {
        for (index, moving) in slide.pieces.iter().enumerate() {
            let id = slide.id * 2 + index as u64;
            slides.push(render_slide(id, *moving, view.orientation, cell, &settings));
        }
    }

    let arrows = view.annotations.arrows.clone();
    let orientation = view.orientation;
    let size = view.size;
    let colors = settings.colors.clone();
    let overlay = canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            for arrow in &arrows {
                let color = colors.mark(arrow.mark).opacity(ARROW_OPACITY);
                paint_arrow(*arrow, bounds, orientation, size, color, window);
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();

    div()
        .relative()
        .flex_none()
        .size(cell * 8.)
        .overflow_hidden()
        .rounded(settings.corner_radius)
        .children(picture)
        .children(squares)
        .children(slides)
        .child(overlay)
}

/// A still picture of `position` in the board and pieces set, for picking a look: nothing on
/// it marked, and nothing to press.
pub fn preview(position: &Chess, size: Pixels, cx: &App) -> Div {
    let settings = Settings::get(cx);
    let look = &settings.board;
    let cell = size / 8.;
    let squares = Square::ALL.into_iter().map(|square| {
        let (column, row) = grid_position(square, Color::White);
        let base = if square.is_light() {
            look.light
        } else {
            look.dark
        };
        // No coordinates: on squares this small they'd crowd the pieces.
        div()
            .absolute()
            .left(cell * column as f32)
            .top(cell * row as f32)
            .size(cell)
            .when(look.image.is_none(), |square| square.bg(base))
            .children(
                position
                    .board()
                    .piece_at(square)
                    .map(|piece| piece_image(settings.pieces.get(piece), cell)),
            )
    });
    let picture = look
        .image
        .clone()
        .map(|image| img(image).absolute().size(size));
    div()
        .relative()
        .flex_none()
        .size(size)
        .overflow_hidden()
        .rounded(settings.corner_radius)
        .children(picture)
        .children(squares)
}

/// A piece gliding above the squares, while its own square holds it back.
fn render_slide(
    id: u64,
    moving: SlidePiece,
    orientation: Color,
    cell: Pixels,
    settings: &Settings,
) -> AnyElement {
    let (from_column, from_row) = grid_position(moving.from, orientation);
    let (to_column, to_row) = grid_position(moving.to, orientation);
    let along = move |from: u32, to: u32, delta: f32| {
        cell * (from as f32 + (to as f32 - from as f32) * delta)
    };
    div()
        .absolute()
        .size(cell)
        .child(piece_image(settings.pieces.get(moving.piece), cell))
        .with_animation(
            ("slide", id),
            Animation::new(SLIDE_TIME).with_easing(ease_in_out),
            move |piece, delta| {
                piece
                    .left(along(from_column, to_column, delta))
                    .top(along(from_row, to_row, delta))
            },
        )
        .into_any_element()
}

fn render_square(
    square: Square,
    view: &View,
    handlers: &Handlers,
    cell: Pixels,
    settings: &Settings,
    cx: &App,
) -> AnyElement {
    let colors = &settings.colors;
    let (column, row) = grid_position(square, view.orientation);
    let look = &settings.board;
    let (base, contrast) = if square.is_light() {
        (look.light, look.dark)
    } else {
        (look.dark, look.light)
    };

    let mut tint = None;
    if let Some((from, to)) = view.last_move
        && (square == from || square == to)
    {
        tint = Some(colors.callout.opacity(0.32));
    }
    if view.selected == Some(square) {
        tint = Some(colors.caret.opacity(0.4));
    }
    let marked = view.annotations.highlights.iter().rev();
    if let Some(found) = marked.into_iter().find(|h| h.square == square) {
        tint = Some(colors.mark(found.mark).opacity(LIGHT_MARK));
    }
    let piece = view.position.board().piece_at(square);
    let gliding_here = view
        .slides
        .iter()
        .flat_map(|slide| &slide.pieces)
        .any(|moving| moving.to == square);
    let in_check = view.position.is_check()
        && piece
            == Some(Piece {
                color: view.position.turn(),
                role: Role::King,
            });

    let press = handlers.press.clone();
    let drop = handlers.drop.clone();
    div()
        .id(("square", u32::from(square)))
        .absolute()
        .left(cell * column as f32)
        .top(cell * row as f32)
        .size(cell)
        .when(look.image.is_none(), |square| square.bg(base))
        .on_mouse_down(MouseButton::Left, move |_: &MouseDownEvent, window, cx| {
            press(square, window, cx);
        })
        .on_drop(move |dragged: &DraggedPiece, window, cx| {
            drop(dragged.from, square, window, cx);
        })
        .children(tint.map(|tint| div().absolute().size_full().bg(tint)))
        .when(in_check, |square| {
            square.child(div().absolute().size_full().bg(colors.red.opacity(0.5)))
        })
        .children(coordinates(square, row, column, contrast, settings))
        .children(
            piece
                .filter(|_| !gliding_here)
                .map(|piece| render_piece(square, piece, view, cell, settings, cx)),
        )
        .when(view.targets.contains(&square), |element| {
            element.child(target_marker(piece.is_some(), cell, colors.caret))
        })
        .into_any_element()
}

fn render_piece(
    square: Square,
    piece: Piece,
    view: &View,
    cell: Pixels,
    settings: &Settings,
    cx: &App,
) -> impl IntoElement {
    let image = settings.pieces.get(piece);
    let dragged = DraggedPiece {
        from: square,
        image: image.clone(),
        size: cell,
    };
    let lifted = view.selected == Some(square) && cx.has_active_drag();
    div()
        .id(("piece", u32::from(square)))
        .absolute()
        .size(cell)
        .cursor_pointer()
        .when(lifted, |piece| piece.opacity(0.3))
        .on_drag(dragged, |dragged, _, _, cx| cx.new(|_| dragged.clone()))
        .child(piece_image(image, cell))
}

/// A dot on an empty square, a ring around a capture.
fn target_marker(capture: bool, cell: Pixels, color: Hsla) -> Div {
    let holder = div()
        .absolute()
        .size_full()
        .flex()
        .items_center()
        .justify_center();
    if capture {
        holder.child(
            div()
                .size(cell * 0.92)
                .rounded_full()
                .border(cell * 0.07)
                .border_color(color.opacity(0.75)),
        )
    } else {
        holder.child(
            div()
                .size(cell * 0.26)
                .rounded_full()
                .bg(color.opacity(0.75)),
        )
    }
}

/// The labels of a square: its file on the bottom row, its rank on the left column.
fn coordinates(
    square: Square,
    row: u32,
    column: u32,
    contrast: Hsla,
    settings: &Settings,
) -> Vec<Div> {
    let label = |text: char| {
        div()
            .absolute()
            .text_size(settings.ui_font_size * 0.8)
            .text_color(contrast)
            .child(text.to_string())
    };
    let mut labels = Vec::new();
    if row == 7 {
        labels.push(label(square.file().char()).bottom(px(1.)).right(px(3.)));
    }
    if column == 0 {
        labels.push(label(square.rank().char()).top(px(1.)).left(px(3.)));
    }
    labels
}

fn paint_arrow(
    arrow: Arrow,
    bounds: Bounds<Pixels>,
    orientation: Color,
    size: Pixels,
    color: Hsla,
    window: &mut Window,
) {
    if arrow.from == arrow.to {
        return;
    }
    let cell = f32::from(size / 8.);
    let origin = bounds.origin;
    let start = center(arrow.from, orientation, size) + origin;
    let end = center(arrow.to, orientation, size) + origin;
    let (dx, dy) = (f32::from(end.x - start.x), f32::from(end.y - start.y));
    let length = dx.hypot(dy);
    let (ux, uy) = (dx / length, dy / length);
    let (nx, ny) = (-uy, ux);
    let shaft = cell * 0.09;
    let head = cell * 0.26;
    let tip_gap = cell * 0.12;
    let at = |along: f32, across: f32| {
        point(
            start.x + px(ux * along + nx * across),
            start.y + px(uy * along + ny * across),
        )
    };
    let (begin, tip) = (cell * 0.22, length - tip_gap);
    let neck = tip - head * 1.25;
    let mut path = PathBuilder::fill();
    path.move_to(at(begin, shaft));
    path.line_to(at(neck, shaft));
    path.line_to(at(neck, head));
    path.line_to(at(tip, 0.));
    path.line_to(at(neck, -head));
    path.line_to(at(neck, -shaft));
    path.line_to(at(begin, -shaft));
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after(moves: &[&str]) -> (Chess, Move) {
        let mut position = Chess::default();
        let mut last = None;
        for word in moves {
            let san: shakmaty::san::San = word.parse().unwrap();
            let played = san.to_move(&position).unwrap();
            position.play_unchecked(played);
            last = Some(played);
        }
        (position, last.unwrap())
    }

    #[test]
    fn a_move_glides_its_piece_forward_and_back() {
        let (position, played) = after(&["e4"]);
        let forward = slide_pieces(played, &position, true);
        assert_eq!((forward[0].from, forward[0].to), (Square::E2, Square::E4));
        let back = slide_pieces(played, &Chess::default(), false);
        assert_eq!((back[0].from, back[0].to), (Square::E4, Square::E2));
        assert_eq!(back[0].piece.role, Role::Pawn);
    }

    #[test]
    fn castling_glides_king_and_rook() {
        let (position, played) = after(&["e4", "e5", "Nf3", "Nc6", "Bc4", "Bc5", "O-O"]);
        let pieces = slide_pieces(played, &position, true);
        assert_eq!(pieces.len(), 2);
        assert_eq!((pieces[0].from, pieces[0].to), (Square::E1, Square::G1));
        assert_eq!((pieces[1].from, pieces[1].to), (Square::H1, Square::F1));
    }

    #[test]
    fn white_sees_a1_at_the_bottom_left() {
        assert_eq!(grid_position(Square::A1, Color::White), (0, 7));
        assert_eq!(grid_position(Square::H8, Color::White), (7, 0));
    }

    #[test]
    fn black_sees_the_board_turned_around() {
        assert_eq!(grid_position(Square::A1, Color::Black), (7, 0));
        assert_eq!(grid_position(Square::H8, Color::Black), (0, 7));
    }
}
