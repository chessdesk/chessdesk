//! Why a move an agent wrote can't be played, said in the words of the board. An illegal move
//! comes from a misread board: a pawn it didn't see in the way, a piece it no longer has.
//! Naming the misread lets it correct itself without being told anything it didn't ask, so
//! it plays no stronger than its own reading.

use shakmaty::attacks::{attacks, between, pawn_attacks};
use shakmaty::san::San;
use shakmaty::{Bitboard, CastlingSide, Chess, Color, EnPassantMode, Position, Rank, Role, Square};

/// The reason `san` is illegal in `position`, if it reads as a move at all.
pub fn why(san: San, position: &Chess) -> Option<String> {
    match san {
        San::Normal {
            role,
            file,
            rank,
            capture,
            to,
            ..
        } => {
            let mine: Bitboard = position
                .our(role)
                .into_iter()
                .filter(|from| file.is_none_or(|file| from.file() == file))
                .filter(|from| rank.is_none_or(|rank| from.rank() == rank))
                .collect();
            Some(
                ambiguous(position, role, mine, to)
                    .unwrap_or_else(|| normal(position, role, mine, capture, to)),
            )
        }
        San::Castle(side) => Some(castle(position, side)),
        San::Put { .. } | San::Null => None,
    }
}

/// Where more than one of `mine` can make the move: the SAN didn't say which. Without this,
/// the reason fell through to a pin that wasn't there.
fn ambiguous(position: &Chess, role: Role, mine: Bitboard, to: Square) -> Option<String> {
    let moves: Vec<_> = position
        .legal_moves()
        .into_iter()
        .filter(|played| played.role() == role && played.to() == to && !played.is_castle())
        .filter(|played| played.from().is_some_and(|from| mine.contains(from)))
        .collect();
    if moves.len() < 2 {
        return None;
    }
    let froms: Vec<_> = moves
        .iter()
        .filter_map(|played| played.from().map(|from| from.to_string()))
        .collect();
    let written: Vec<_> = moves
        .iter()
        .map(|played| San::from_move(position, *played).to_string())
        .collect();
    let both = if moves.len() == 2 { "both" } else { "all" };
    Some(format!(
        "your {}s on {} {both} reach {to}: write {}",
        role_name(role),
        listed(&froms, "and"),
        listed(&written, "or")
    ))
}

/// `a and b`, or `a, b and c`, with `last` as the last joining word.
fn listed(items: &[String], last: &str) -> String {
    match items {
        [rest @ .., final_item] if !rest.is_empty() => {
            format!("{} {last} {final_item}", rest.join(", "))
        }
        _ => items.concat(),
    }
}

fn normal(position: &Chess, role: Role, mine: Bitboard, capture: bool, to: Square) -> String {
    let board = position.board();
    let us = position.turn();
    let name = role_name(role);
    if position.our(role).is_empty() {
        return format!("you have no {name} left");
    }
    if let Some(piece) = board.piece_at(to).filter(|piece| piece.color == us) {
        return format!("your own {} stands on {to}", role_name(piece.role));
    }
    let en_passant = role == Role::Pawn && position.ep_square(EnPassantMode::Legal) == Some(to);
    if capture && board.piece_at(to).is_none() && !en_passant {
        return format!("there is nothing to capture on {to}");
    }
    for from in mine {
        if !reaches(role, us, from, to, capture) {
            continue;
        }
        if role == Role::Pawn && !capture && board.piece_at(to).is_some() {
            return format!("{to} is taken, and a pawn can't capture straight ahead");
        }
        if let Some(block) = (between(from, to) & board.occupied()).first() {
            let blocker = board
                .piece_at(block)
                .expect("an occupied square has a piece");
            return format!(
                "{} {} on {block} blocks your {name} on {from}",
                owner(blocker.color, us),
                role_name(blocker.role)
            );
        }
        if role == Role::King {
            return format!("{to} is attacked, so your king can't go there");
        }
        if let Some(checker) = position.checkers().first() {
            let piece = board.piece_at(checker).expect("a checker is a piece");
            return format!(
                "you're in check from {} {} on {checker}",
                owner(piece.color, us),
                role_name(piece.role)
            );
        }
        return format!("your {name} on {from} is pinned: moving it would expose your king");
    }
    let squares: Vec<_> = position
        .our(role)
        .into_iter()
        .map(|at| at.to_string())
        .collect();
    let plural = if squares.len() == 1 { "" } else { "s" };
    format!(
        "no {name} of yours reaches {to} (your {name}{plural}: {})",
        squares.join(", ")
    )
}

/// Whether a piece on `from` moves to `to` on an empty board, as it would capture or not.
fn reaches(role: Role, us: Color, from: Square, to: Square, capture: bool) -> bool {
    if role != Role::Pawn {
        return attacks(from, role.of(us), Bitboard::EMPTY).contains(to);
    }
    if capture {
        return pawn_attacks(us, from).contains(to);
    }
    let (step, start) = if us.is_white() {
        (1, Rank::Second)
    } else {
        (-1, Rank::Seventh)
    };
    let one = from.offset(8 * step);
    let two = (from.rank() == start)
        .then(|| from.offset(16 * step))
        .flatten();
    one == Some(to) || two == Some(to)
}

fn castle(position: &Chess, side: CastlingSide) -> String {
    let us = position.turn();
    let board = position.board();
    let castles = position.castles();
    let (Some(rook), Some(king)) = (castles.rook(us, side), board.king_of(us)) else {
        return "you can no longer castle on that side".to_owned();
    };
    if !castles.has(us, side) {
        return "you can no longer castle on that side".to_owned();
    }
    if !position.checkers().is_empty() {
        return "you can't castle out of check".to_owned();
    }
    if let Some(block) = (between(king, rook) & board.occupied()).first() {
        let blocker = board
            .piece_at(block)
            .expect("an occupied square has a piece");
        return format!(
            "{} {} on {block} stands between your king and rook",
            owner(blocker.color, us),
            role_name(blocker.role)
        );
    }
    "your king would pass through or land on an attacked square".to_owned()
}

/// [`why`] for a move the agent named for the player to play: the same reason, about them.
pub fn why_for_the_player(san: San, position: &Chess) -> Option<String> {
    Some(about_them(&why(san, position)?))
}

/// `reason` with the player in place of the agent: `your knight` as `their knight`.
fn about_them(reason: &str) -> String {
    let mut said = String::with_capacity(reason.len());
    let mut word = String::new();
    let flush = |word: &mut String, said: &mut String| {
        said.push_str(match word.as_str() {
            "you" => "they",
            "your" => "their",
            "yours" => "theirs",
            "you're" => "they're",
            other => other,
        });
        word.clear();
    };
    for next in reason.chars() {
        if next.is_alphabetic() || next == '\'' {
            word.push(next);
        } else {
            flush(&mut word, &mut said);
            said.push(next);
        }
    }
    flush(&mut word, &mut said);
    said
}

fn owner(color: Color, us: Color) -> &'static str {
    match (color == us, color) {
        (true, _) => "your",
        (false, Color::White) => "White's",
        (false, Color::Black) => "Black's",
    }
}

pub(crate) fn role_name(role: Role) -> &'static str {
    match role {
        Role::Pawn => "pawn",
        Role::Knight => "knight",
        Role::Bishop => "bishop",
        Role::Rook => "rook",
        Role::Queen => "queen",
        Role::King => "king",
    }
}

#[cfg(test)]
mod tests {
    use chessdesk_core::Game;

    use super::*;

    fn reason(fen: &str, san: &str) -> String {
        let position = Game::parse_fen(fen).unwrap();
        let san: San = san.parse().unwrap();
        assert!(san.to_move(&position).is_err(), "{san} is legal");
        why(san, &position).unwrap()
    }

    /// Black to move after 16. Bc4 in the game where GLM 5.3 Flash lost its queen.
    const AFTER_BC4: &str = "4rrk1/pp1b2pp/1npqp3/4Rp2/2BP4/5N2/PPP1QPPP/R5K1 b - - 5 16";
    /// Black to move after 17. dxe5: the dark-squared bishop is long gone.
    const AFTER_DXE5: &str = "4rrk1/pp1b2pp/1np1p3/4Pp2/2B5/5N2/PPP1QPPP/R5K1 b - - 0 17";

    #[test]
    fn names_the_piece_in_the_way() {
        assert_eq!(
            reason(AFTER_BC4, "Rxe5"),
            "your pawn on e6 blocks your rook on e8"
        );
    }

    #[test]
    fn says_where_the_pieces_really_are() {
        assert_eq!(
            reason(AFTER_DXE5, "Bd6"),
            "no bishop of yours reaches d6 (your bishop: d7)"
        );
        assert_eq!(reason(AFTER_DXE5, "Qd6"), "you have no queen left");
        // White after 9... b5 in the Sicilian lesson, sure its b3 knight takes on b5.
        let after_b5 = "r1bq1rk1/p3bppp/2np1n2/1p2p3/4P3/1NN1B3/PPP1BPPP/R2Q1RK1 w - - 0 10";
        assert_eq!(
            reason(after_b5, "Nb3xb5"),
            "no knight of yours reaches b5 (your knights: b3, c3)"
        );
    }

    #[test]
    fn sees_own_pieces_and_empty_captures() {
        assert_eq!(reason(AFTER_BC4, "Nd7"), "your own bishop stands on d7");
        assert_eq!(reason(AFTER_BC4, "Bxe6"), "your own pawn stands on e6");
        assert_eq!(
            reason(AFTER_BC4, "Qxa3"),
            "there is nothing to capture on a3"
        );
    }

    #[test]
    fn a_move_for_the_player_is_explained_about_them() {
        let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let position = Game::parse_fen(start).unwrap();
        let reason = |san: &str| why_for_the_player(san.parse().unwrap(), &position).unwrap();
        assert_eq!(reason("Nd2"), "their own pawn stands on d2");
        assert_eq!(
            reason("Nc5"),
            "no knight of theirs reaches c5 (their knights: b1, g1)"
        );
        let checked = "4k3/8/8/8/4q3/8/3P4/4K3 w - - 0 1";
        let position = Game::parse_fen(checked).unwrap();
        assert_eq!(
            why_for_the_player("d3".parse().unwrap(), &position).unwrap(),
            "they're in check from Black's queen on e4"
        );
    }

    #[test]
    fn a_move_two_pieces_can_make_names_both() {
        // Rd8 with rooks on a8 and e8, as GLM 5.3 Flash wrote it twice in one game: it was
        // told its rook on a8 was pinned, which it wasn't.
        let two_rooks = "r3r1k1/5ppp/8/8/8/8/5PPP/6K1 b - - 0 1";
        assert_eq!(
            reason(two_rooks, "Rd8"),
            "your rooks on a8 and e8 both reach d8: write Rad8 or Red8"
        );
        let three_knights = "6k1/8/8/1N6/8/1N3N2/8/6K1 w - - 0 1";
        assert_eq!(
            reason(three_knights, "Nd4"),
            "your knights on b3, f3 and b5 all reach d4: write Nb3d4, Nfd4 or N5d4"
        );
    }

    #[test]
    fn explains_pawns_checks_and_pins() {
        let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        assert_eq!(
            reason(start, "e5"),
            "no pawn of yours reaches e5 (your pawns: a2, b2, c2, d2, e2, f2, g2, h2)"
        );
        // Black's queen checks along e-file; the d2 pawn's move doesn't help.
        let check = "4k3/8/8/8/4q3/8/3P4/4K3 w - - 0 1";
        assert_eq!(
            reason(check, "d3"),
            "you're in check from Black's queen on e4"
        );
        let pinned = "4k3/4q3/8/8/8/8/4N3/4K3 w - - 0 1";
        assert_eq!(
            reason(pinned, "Nf4"),
            "your knight on e2 is pinned: moving it would expose your king"
        );
        assert_eq!(
            reason(pinned, "Kf3"),
            "no king of yours reaches f3 (your king: e1)"
        );
        let checked = "4k3/8/8/8/8/8/5q2/4K3 w - - 0 1";
        assert_eq!(
            reason(checked, "Kd2"),
            "d2 is attacked, so your king can't go there"
        );
    }

    proptest::proptest! {
        /// Whatever the game and whatever move it names, an illegal move gets a reason and
        /// nothing panics.
        #[test]
        fn every_illegal_move_gets_a_reason(
            picks in proptest::collection::vec(0usize..256, 0..60),
            role in 1usize..=6,
            to in 0u32..64,
            capture: bool,
            castle_short: bool,
        ) {
            let mut position = Chess::default();
            for pick in picks {
                let legal = position.legal_moves();
                if legal.is_empty() {
                    break;
                }
                position.play_unchecked(legal[pick % legal.len()]);
            }
            let role = Role::ALL[role - 1];
            let normal = San::Normal {
                role,
                file: None,
                rank: None,
                capture,
                to: Square::new(to),
                promotion: None,
            };
            let side = if castle_short { CastlingSide::KingSide } else { CastlingSide::QueenSide };
            for san in [normal, San::Castle(side)] {
                match san.to_move(&position) {
                    Ok(_) => {}
                    // A move more than one piece can make is never blamed on something else.
                    Err(shakmaty::san::SanError::AmbiguousSan) => {
                        let reason = why(san, &position).unwrap_or_default();
                        proptest::prop_assert!(reason.contains(" reach "), "{reason}");
                        proptest::prop_assert!(reason.contains(": write "), "{reason}");
                    }
                    Err(_) => proptest::prop_assert!(why(san, &position).is_some()),
                }
            }
        }
    }

    #[test]
    fn explains_castling() {
        let blocked = "4k3/8/8/8/8/8/8/R3KB1R w KQ - 0 1";
        assert_eq!(
            reason(blocked, "O-O"),
            "your bishop on f1 stands between your king and rook"
        );
        let no_rights = "4k3/8/8/8/8/8/8/R3K2R w - - 0 1";
        assert_eq!(
            reason(no_rights, "O-O"),
            "you can no longer castle on that side"
        );
    }
}
