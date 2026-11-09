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
            let mine = position
                .our(role)
                .into_iter()
                .filter(|from| file.is_none_or(|file| from.file() == file))
                .filter(|from| rank.is_none_or(|rank| from.rank() == rank))
                .collect();
            Some(normal(position, role, mine, capture, to))
        }
        San::Castle(side) => Some(castle(position, side)),
        San::Put { .. } | San::Null => None,
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

fn owner(color: Color, us: Color) -> &'static str {
    match (color == us, color) {
        (true, _) => "your",
        (false, Color::White) => "White's",
        (false, Color::Black) => "Black's",
    }
}

fn role_name(role: Role) -> &'static str {
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
                if san.to_move(&position).is_err() {
                    proptest::prop_assert!(why(san, &position).is_some());
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
