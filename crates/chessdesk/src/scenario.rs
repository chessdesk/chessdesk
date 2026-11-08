//! Positions to start a game from: endgames to practise and openings to play out.

use crate::setup::Side;

pub struct Scenario {
    /// The message that names it.
    pub title: &'static str,
    pub kind: Kind,
    pub fen: &'static str,
    /// The side that has the lesson to learn.
    pub side: Side,
}

#[derive(Clone, Copy)]
pub enum Kind {
    Endgame,
    Opening,
}

impl Kind {
    pub fn label_id(self) -> &'static str {
        match self {
            Self::Endgame => "scenario-endgame",
            Self::Opening => "scenario-opening",
        }
    }
}

pub const ALL: [Scenario; 9] = [
    Scenario {
        title: "scenario-queen-mate",
        kind: Kind::Endgame,
        fen: "8/8/8/4k3/8/8/8/3QK3 w - - 0 1",
        side: Side::White,
    },
    Scenario {
        title: "scenario-rook-mate",
        kind: Kind::Endgame,
        fen: "8/8/8/4k3/8/8/8/R3K3 w - - 0 1",
        side: Side::White,
    },
    Scenario {
        title: "scenario-king-pawn",
        kind: Kind::Endgame,
        fen: "4k3/8/4K3/4P3/8/8/8/8 w - - 0 1",
        side: Side::White,
    },
    Scenario {
        title: "scenario-lucena",
        kind: Kind::Endgame,
        fen: "1K6/1P2k3/8/8/8/8/r7/3R4 w - - 0 1",
        side: Side::White,
    },
    Scenario {
        title: "scenario-philidor",
        kind: Kind::Endgame,
        fen: "4k3/R7/7r/3KP3/8/8/8/8 b - - 0 1",
        side: Side::Black,
    },
    Scenario {
        title: "scenario-italian",
        kind: Kind::Opening,
        fen: "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
        side: Side::White,
    },
    Scenario {
        title: "scenario-sicilian",
        kind: Kind::Opening,
        fen: "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
        side: Side::Black,
    },
    Scenario {
        title: "scenario-queens-gambit",
        kind: Kind::Opening,
        fen: "rnbqkbnr/ppp1pppp/8/3p4/2PP4/8/PP2PPPP/RNBQKBNR b KQkq - 0 2",
        side: Side::White,
    },
    Scenario {
        title: "scenario-french",
        kind: Kind::Opening,
        fen: "rnbqkbnr/ppp2ppp/4p3/3p4/3PP3/8/PPP2PPP/RNBQKBNR w KQkq - 0 3",
        side: Side::Black,
    },
];

/// The scenario that starts from `fen`, if it's one of them rather than the player's own.
pub fn find(fen: &str) -> Option<&'static Scenario> {
    ALL.iter().find(|scenario| scenario.fen == fen)
}

#[cfg(test)]
mod tests {
    use chessdesk_core::Game;
    use shakmaty::{Color, Position};

    use super::*;

    #[test]
    fn every_scenario_is_a_legal_position() {
        for scenario in &ALL {
            assert!(
                Game::parse_fen(scenario.fen).is_some(),
                "{}",
                scenario.title
            );
        }
    }

    #[test]
    fn endgames_start_with_the_learner_to_move() {
        for scenario in ALL.iter().filter(|s| matches!(s.kind, Kind::Endgame)) {
            let turn = Game::parse_fen(scenario.fen).unwrap().turn();
            let learner = match scenario.side {
                Side::Black => Color::Black,
                _ => Color::White,
            };
            assert_eq!(turn, learner, "{}", scenario.title);
        }
    }

    #[test]
    fn a_pasted_position_is_no_scenario() {
        assert!(find(ALL[3].fen).is_some());
        assert!(find("8/8/8/8/8/8/8/K6k w - - 0 1").is_none());
    }
}
