//! What an engine thinks of a position, always from White's side.

use std::fmt;

/// An engine's score for White.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Score {
    /// In hundredths of a pawn.
    Centipawns(i32),
    /// Moves until mate; negative when Black mates.
    Mate(i32),
}

impl Score {
    /// Parses the argument of `[%eval ..]`: `0.17`, `-1.50`, `#3` or `#-2`.
    pub fn parse(text: &str) -> Option<Self> {
        if let Some(mate) = text.strip_prefix('#') {
            return mate.parse().ok().map(Self::Mate);
        }
        let pawns: f64 = text.parse().ok()?;
        pawns
            .is_finite()
            .then(|| Self::Centipawns((pawns * 100.).round() as i32))
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Centipawns(centipawns) => write!(f, "{:.2}", f64::from(*centipawns) / 100.),
            Self::Mate(moves) => write!(f, "#{moves}"),
        }
    }
}

/// The likelihood of each result in parts per thousand, as UCI's `wdl` reports it, but
/// from White's side: White wins, a draw, Black wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wdl {
    pub white: u16,
    pub draw: u16,
    pub black: u16,
}

impl Wdl {
    /// Parses the argument of `[%wdl ..]`: `450 300 250`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split_whitespace().map(str::parse);
        let wdl = Self {
            white: parts.next()?.ok()?,
            draw: parts.next()?.ok()?,
            black: parts.next()?.ok()?,
        };
        parts.next().is_none().then_some(wdl)
    }

    /// White's expected score, from 0 to 1: what the graph plots and swings are measured in.
    pub fn expected(self) -> f32 {
        let total = f32::from(self.white) + f32::from(self.draw) + f32::from(self.black);
        if total == 0. {
            return 0.5;
        }
        (f32::from(self.white) + f32::from(self.draw) / 2.) / total
    }
}

impl fmt::Display for Wdl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.white, self.draw, self.black)
    }
}

/// An engine's view of the position after a move.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Eval {
    pub score: Option<Score>,
    pub wdl: Option<Wdl>,
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn reads_scores() {
        assert_eq!(Score::parse("0.17"), Some(Score::Centipawns(17)));
        assert_eq!(Score::parse("-1.50"), Some(Score::Centipawns(-150)));
        assert_eq!(Score::parse("#-2"), Some(Score::Mate(-2)));
        assert_eq!(Score::parse("nan"), None);
    }

    #[test]
    fn expected_score_counts_draws_as_half() {
        let wdl = Wdl {
            white: 500,
            draw: 500,
            black: 0,
        };
        assert!((wdl.expected() - 0.75).abs() < f32::EPSILON);
        let unknown = Wdl {
            white: 0,
            draw: 0,
            black: 0,
        };
        assert!((unknown.expected() - 0.5).abs() < f32::EPSILON);
    }

    proptest! {
        #[test]
        fn scores_round_trip(centipawns in -100_000i32..100_000, mate in -300i32..300) {
            let score = Score::Centipawns(centipawns);
            prop_assert_eq!(Score::parse(&score.to_string()), Some(score));
            let score = Score::Mate(mate);
            prop_assert_eq!(Score::parse(&score.to_string()), Some(score));
        }

        #[test]
        fn wdl_round_trips(white in 0u16..=1000, draw in 0u16..=1000, black in 0u16..=1000) {
            let wdl = Wdl { white, draw, black };
            prop_assert_eq!(Wdl::parse(&wdl.to_string()), Some(wdl));
        }
    }
}
