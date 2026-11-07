//! The likelihood bar's movement: from wherever it is to the latest numbers.

use std::time::{Duration, Instant};

use chessdesk_core::Wdl;
use gpui::ease_in_out;

/// How long the bar takes to settle.
const TIME: Duration = Duration::from_millis(450);
/// Where the bar rests with nothing to go by.
const MIDDLE: f32 = 0.5;

/// White's share of the bar, from 0 to 1, on its way to the latest numbers.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    from: f32,
    to: f32,
    /// What `to` was made from, to tell a new target from the same one.
    target: Option<Wdl>,
    since: Instant,
}

impl Motion {
    pub fn new(now: Instant) -> Self {
        Self {
            from: MIDDLE,
            to: MIDDLE,
            target: None,
            since: now,
        }
    }

    /// Back to the middle: nothing is known, and the last game's numbers aren't this one's.
    pub fn clear(&mut self, now: Instant) {
        if self.target.is_some() {
            *self = Self::new(now);
        }
    }

    /// Sets off towards `wdl`, from wherever the bar is right now.
    pub fn aim(&mut self, wdl: Wdl, now: Instant) {
        if self.target != Some(wdl) {
            self.from = self.value(now);
            self.to = wdl.expected();
            self.target = Some(wdl);
            self.since = now;
        }
    }

    pub fn value(&self, now: Instant) -> f32 {
        let share = now.saturating_duration_since(self.since).as_secs_f32() / TIME.as_secs_f32();
        self.from + (self.to - self.from) * ease_in_out(share.clamp(0., 1.))
    }

    pub fn is_moving(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.since) < TIME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE_WINS: Wdl = Wdl {
        white: 1000,
        draw: 0,
        black: 0,
    };
    const BLACK_WINS: Wdl = Wdl {
        white: 0,
        draw: 0,
        black: 1000,
    };

    #[test]
    fn it_starts_in_the_middle() {
        let start = Instant::now();
        assert!((Motion::new(start).value(start) - MIDDLE).abs() < f32::EPSILON);
    }

    #[test]
    fn it_ends_exactly_on_the_target() {
        let start = Instant::now();
        let mut motion = Motion::new(start);
        motion.aim(WHITE_WINS, start);
        assert!(motion.is_moving(start));
        let done = start + TIME;
        assert!((motion.value(done) - 1.).abs() < f32::EPSILON);
        assert!(!motion.is_moving(done));
    }

    #[test]
    fn a_draw_stays_in_the_middle() {
        let start = Instant::now();
        let mut motion = Motion::new(start);
        let draw = Wdl {
            white: 0,
            draw: 1000,
            black: 0,
        };
        motion.aim(draw, start);
        assert!((motion.value(start + TIME) - MIDDLE).abs() < f32::EPSILON);
    }

    #[test]
    fn a_new_target_sets_off_from_where_the_bar_is() {
        let start = Instant::now();
        let mut motion = Motion::new(start);
        motion.aim(WHITE_WINS, start);
        let halfway = start + TIME / 2;
        let there = motion.value(halfway);
        motion.aim(BLACK_WINS, halfway);
        assert!((motion.value(halfway) - there).abs() < f32::EPSILON);
    }

    #[test]
    fn the_same_target_changes_nothing() {
        let start = Instant::now();
        let mut motion = Motion::new(start);
        motion.aim(WHITE_WINS, start);
        let before = motion.since;
        motion.aim(WHITE_WINS, start + TIME / 2);
        assert_eq!(motion.since, before);
    }

    #[test]
    fn clearing_forgets_the_last_game() {
        let start = Instant::now();
        let mut motion = Motion::new(start);
        motion.aim(WHITE_WINS, start);
        let later = start + TIME;
        motion.clear(later);
        assert!((motion.value(later) - MIDDLE).abs() < f32::EPSILON);
        // Called on every frame, so it must not keep the bar moving.
        let since = motion.since;
        motion.clear(later + TIME);
        assert_eq!(motion.since, since);
    }
}
