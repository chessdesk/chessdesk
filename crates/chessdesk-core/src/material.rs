//! What each side has taken, and who is ahead. Captures come from the moves, so they're
//! exact whatever the start; the lead comes from the board, so promotions count.

use shakmaty::{Board, Color, Piece, Position, Role};

use crate::Game;

/// Most valuable first, the order they're shown in.
const ROLES: [(Role, i32); 5] = [
    (Role::Queen, 9),
    (Role::Rook, 5),
    (Role::Bishop, 3),
    (Role::Knight, 3),
    (Role::Pawn, 1),
];

/// The pieces each side has taken, and the balance on the board.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Captures {
    pub by_white: Vec<Role>,
    pub by_black: Vec<Role>,
    /// In pawns, positive when White leads.
    pub lead: i32,
}

impl Captures {
    /// What `color` has taken.
    pub fn by(&self, color: Color) -> &[Role] {
        if color.is_white() {
            &self.by_white
        } else {
            &self.by_black
        }
    }

    /// How far ahead `color` is, if it's ahead at all.
    pub fn lead_of(&self, color: Color) -> Option<i32> {
        let lead = if color.is_white() {
            self.lead
        } else {
            -self.lead
        };
        (lead > 0).then_some(lead)
    }
}

fn worth(role: Role) -> i32 {
    ROLES
        .iter()
        .find(|(known, _)| *known == role)
        .map_or(0, |(_, value)| *value)
}

/// White's lead in pawns.
fn lead(board: &Board) -> i32 {
    ROLES
        .iter()
        .map(|(role, value)| {
            let count = |color| board.by_piece(Piece { color, role: *role }).count() as i32;
            (count(Color::White) - count(Color::Black)) * value
        })
        .sum()
}

/// The state of play after `ply` moves.
pub fn captures(game: &Game, ply: usize) -> Captures {
    let mut found = Captures {
        lead: lead(game.position_at(ply).board()),
        ..Captures::default()
    };
    let mut mover = game.start().turn();
    for played in game.moves().iter().take(ply) {
        if let Some(taken) = played.played.capture() {
            let side = if mover.is_white() {
                &mut found.by_white
            } else {
                &mut found.by_black
            };
            side.push(taken);
        }
        mover = mover.other();
    }
    found.by_white.sort_by_key(|role| -worth(*role));
    found.by_black.sort_by_key(|role| -worth(*role));
    found
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use shakmaty::san::San;

    use super::*;

    fn game_of(moves: &[&str]) -> Game {
        let mut game = Game::default();
        for word in moves {
            let san: San = word.parse().unwrap();
            let played = san.to_move(&game.position()).unwrap();
            assert!(game.play(played));
        }
        game
    }

    #[test]
    fn nothing_is_taken_at_the_start() {
        assert_eq!(captures(&Game::default(), 0), Captures::default());
    }

    #[test]
    fn an_even_trade_shows_on_both_sides_without_a_lead() {
        let game = game_of(&["e4", "d5", "exd5", "Qxd5"]);
        let found = captures(&game, 4);
        assert_eq!(found.by_white, [Role::Pawn]);
        assert_eq!(found.by_black, [Role::Pawn]);
        assert_eq!(found.lead, 0);
        assert_eq!(found.lead_of(Color::White), None);
    }

    #[test]
    fn counts_up_to_the_position_on_show() {
        let game = game_of(&["e4", "d5", "exd5", "Qxd5"]);
        let found = captures(&game, 3);
        assert_eq!(found.by_white, [Role::Pawn]);
        assert_eq!(found.by_black, Vec::<Role>::new());
        assert_eq!(found.lead_of(Color::White), Some(1));
    }

    #[test]
    fn the_biggest_catch_comes_first() {
        let game = game_of(&[
            "e4", "d5", "exd5", "Qxd5", "Nc3", "Qxg2", "Bxg2", "Nf6", "Bxb7", "Bxb7",
        ]);
        let found = captures(&game, 10);
        assert_eq!(found.by_white, [Role::Queen, Role::Pawn, Role::Pawn]);
        assert_eq!(found.by_black, [Role::Bishop, Role::Pawn, Role::Pawn]);
        assert_eq!(found.lead_of(Color::White), Some(6));
    }

    #[test]
    fn en_passant_takes_a_pawn() {
        let game = game_of(&["e4", "a6", "e5", "d5", "exd6"]);
        assert_eq!(captures(&game, 5).by_white, [Role::Pawn]);
    }

    proptest! {
        #[test]
        fn taken_pieces_explain_the_lead_without_promotions(
            choices in prop::collection::vec(any::<prop::sample::Index>(), 0..60),
        ) {
            let mut game = Game::default();
            for choice in choices {
                let moves = game.position().legal_moves();
                let plain: Vec<_> = moves.iter().filter(|m| m.promotion().is_none()).collect();
                if plain.is_empty() {
                    break;
                }
                prop_assert!(game.play(*plain[choice.index(plain.len())]));
            }
            let found = captures(&game, game.moves().len());
            let total = |roles: &[Role]| roles.iter().map(|role| worth(*role)).sum::<i32>();
            prop_assert_eq!(found.lead, total(&found.by_white) - total(&found.by_black));
        }
    }
}
