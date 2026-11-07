//! What the game sounds like: which sound belongs to which moment. Playing them is the
//! app's business, and what they sound like a plugin's.

use crate::PlayedMove;

/// A moment with a sound. Each is a file in a sound pack, named by [`Sound::file_stem`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Move,
    Capture,
    Check,
    Castle,
    GameStart,
    Win,
    Lose,
    Draw,
    LowTime,
}

impl Sound {
    pub const ALL: [Self; 9] = [
        Self::Move,
        Self::Capture,
        Self::Check,
        Self::Castle,
        Self::GameStart,
        Self::Win,
        Self::Lose,
        Self::Draw,
        Self::LowTime,
    ];

    /// The file's name without its extension.
    pub fn file_stem(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Capture => "capture",
            Self::Check => "check",
            Self::Castle => "castle",
            Self::GameStart => "game-start",
            Self::Win => "win",
            Self::Lose => "lose",
            Self::Draw => "draw",
            Self::LowTime => "low-time",
        }
    }

    /// What plays if a pack leaves this one out: every move still sounds like a move.
    /// Moments outside of moves just stay silent.
    pub fn fallback(self) -> Option<Self> {
        matches!(self, Self::Capture | Self::Check | Self::Castle).then_some(Self::Move)
    }

    /// The sound of a move: a check matters most, whatever else the move does.
    pub fn of_move(played: &PlayedMove) -> Self {
        let san = played.san.to_string();
        if san.ends_with(['+', '#']) {
            Self::Check
        } else if played.played.is_capture() {
            Self::Capture
        } else if played.played.is_castle() {
            Self::Castle
        } else {
            Self::Move
        }
    }
}

#[cfg(test)]
mod tests {
    use shakmaty::uci::UciMove;

    use super::*;
    use crate::Game;

    fn sound_of_last(moves: &[&str]) -> Sound {
        let mut game = Game::default();
        for text in moves {
            let parsed: UciMove = text.parse().unwrap();
            assert!(game.play(parsed.to_move(&game.position()).unwrap()));
        }
        Sound::of_move(game.moves().last().unwrap())
    }

    #[test]
    fn each_kind_of_move_has_its_sound() {
        assert_eq!(sound_of_last(&["e2e4"]), Sound::Move);
        assert_eq!(sound_of_last(&["e2e4", "d7d5", "e4d5"]), Sound::Capture);
        assert_eq!(
            sound_of_last(&["e2e4", "e7e5", "g1f3", "g8f6", "f1c4", "f8c5", "e1h1"]),
            Sound::Castle
        );
        assert_eq!(sound_of_last(&["e2e4", "f7f6", "d1h5"]), Sound::Check);
    }

    #[test]
    fn a_capture_with_check_sounds_like_a_check() {
        let moves = ["e2e4", "e7e5", "f1c4", "a7a6", "c4f7"];
        assert_eq!(sound_of_last(&moves), Sound::Check);
    }

    #[test]
    fn mate_counts_as_a_check() {
        let moves = ["f2f3", "e7e5", "g2g4", "d8h4"];
        assert_eq!(sound_of_last(&moves), Sound::Check);
    }

    #[test]
    fn only_moves_fall_back() {
        assert_eq!(Sound::Capture.fallback(), Some(Sound::Move));
        assert_eq!(Sound::Move.fallback(), None);
        assert_eq!(Sound::Win.fallback(), None);
    }
}
