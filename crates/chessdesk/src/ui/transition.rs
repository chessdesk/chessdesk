//! A value easing between 0 and 1 as a switch flips: for things that come and go smoothly,
//! such as a hover or a folding section.

use std::time::{Duration, Instant};

use gpui::ease_in_out;

#[derive(Clone, Copy, Debug)]
pub struct Transition {
    /// Where it was when the switch last flipped.
    from: f32,
    on: bool,
    since: Instant,
    length: Duration,
}

fn value_of(on: bool) -> f32 {
    if on { 1. } else { 0. }
}

impl Transition {
    /// Resting at on or off, taking `length` for each change to come.
    pub fn settled(on: bool, length: Duration, now: Instant) -> Self {
        Self {
            from: value_of(on),
            on,
            since: now,
            length,
        }
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    /// Sets off towards on or off, from wherever it is right now.
    pub fn set(&mut self, on: bool, now: Instant) {
        if on != self.on {
            self.from = self.value(now);
            self.on = on;
            self.since = now;
        }
    }

    /// From 0 for off to 1 for on.
    pub fn value(&self, now: Instant) -> f32 {
        let share =
            now.saturating_duration_since(self.since).as_secs_f32() / self.length.as_secs_f32();
        let to = value_of(self.on);
        self.from + (to - self.from) * ease_in_out(share.clamp(0., 1.))
    }

    pub fn is_moving(&self, now: Instant) -> bool {
        let to = value_of(self.on);
        (to - self.from).abs() > f32::EPSILON
            && now.saturating_duration_since(self.since) < self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LENGTH: Duration = Duration::from_millis(100);

    #[test]
    fn it_runs_its_course_and_turns_back_from_where_it_is() {
        let start = Instant::now();
        let mut transition = Transition::settled(false, LENGTH, start);
        assert!(transition.value(start).abs() < f32::EPSILON);
        transition.set(true, start);
        assert!(transition.is_moving(start));
        assert!((transition.value(start + LENGTH) - 1.).abs() < f32::EPSILON);
        assert!(!transition.is_moving(start + LENGTH));
        // Halfway, it's switched back: it returns from there, not from all the way on.
        let halfway = start + LENGTH / 2;
        let reached = transition.value(halfway);
        transition.set(false, halfway);
        assert!((transition.value(halfway) - reached).abs() < f32::EPSILON);
        assert!(transition.value(halfway + LENGTH).abs() < f32::EPSILON);
    }

    #[test]
    fn settled_means_still() {
        let now = Instant::now();
        let transition = Transition::settled(true, LENGTH, now);
        assert!((transition.value(now) - 1.).abs() < f32::EPSILON);
        assert!(!transition.is_moving(now));
        assert!(transition.is_on());
    }
}
