//! The game clock: time left for each side, and whose is running.

use std::time::{Duration, Instant};

use shakmaty::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    white: Duration,
    black: Duration,
    increment: Duration,
    /// Who is on the move, and since when.
    running: Option<(Color, Instant)>,
}

impl Clock {
    pub fn new(time: Duration, increment: Duration) -> Self {
        Self {
            white: time,
            black: time,
            increment,
            running: None,
        }
    }

    fn stored(&self, color: Color) -> Duration {
        if color.is_white() {
            self.white
        } else {
            self.black
        }
    }

    /// What `color` has left at `now`.
    pub fn left(&self, color: Color, now: Instant) -> Duration {
        let stored = self.stored(color);
        match self.running {
            Some((running, since)) if running == color => {
                stored.saturating_sub(now.saturating_duration_since(since))
            }
            _ => stored,
        }
    }

    /// Starts `color`'s time.
    pub fn start(&mut self, color: Color, now: Instant) {
        self.running = Some((color, now));
    }

    /// Stops the running side's time, without any increment.
    pub fn stop(&mut self, now: Instant) {
        if let Some((color, _)) = self.running {
            let left = self.left(color, now);
            self.set_left(color, left);
            self.running = None;
        }
    }

    /// Ends a move: books the time used, adds the increment and starts the other side.
    pub fn press(&mut self, now: Instant) {
        if let Some((color, _)) = self.running {
            self.stop(now);
            self.set_left(color, self.stored(color) + self.increment);
            self.start(color.other(), now);
        }
    }

    /// The side that has run out of time, if any.
    pub fn flagged(&self, now: Instant) -> Option<Color> {
        let (color, _) = self.running?;
        self.left(color, now).is_zero().then_some(color)
    }

    /// Sets what `color` has left, as when picking up a saved game.
    pub fn set_left(&mut self, color: Color, left: Duration) {
        if color.is_white() {
            self.white = left;
        } else {
            self.black = left;
        }
    }
}

/// Time as `m:ss`, or `h:mm:ss` from an hour up.
pub fn format(left: Duration) -> String {
    let seconds = left.as_secs();
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: Duration = Duration::from_secs(60);
    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn only_the_running_side_loses_time() {
        let start = Instant::now();
        let mut clock = Clock::new(MINUTE, Duration::ZERO);
        clock.start(Color::White, start);
        let later = start + 10 * SECOND;
        assert_eq!(clock.left(Color::White, later), 50 * SECOND);
        assert_eq!(clock.left(Color::Black, later), MINUTE);
    }

    #[test]
    fn pressing_adds_the_increment_and_switches() {
        let start = Instant::now();
        let mut clock = Clock::new(MINUTE, 5 * SECOND);
        clock.start(Color::White, start);
        let later = start + 10 * SECOND;
        clock.press(later);
        assert_eq!(clock.left(Color::White, later + MINUTE), 55 * SECOND);
        assert_eq!(clock.left(Color::Black, later + SECOND), 59 * SECOND);
    }

    #[test]
    fn a_stopped_clock_stands_still_and_never_goes_negative() {
        let start = Instant::now();
        let mut clock = Clock::new(MINUTE, Duration::ZERO);
        clock.start(Color::Black, start);
        assert_eq!(clock.flagged(start + 2 * MINUTE), Some(Color::Black));
        clock.stop(start + 2 * MINUTE);
        assert_eq!(clock.left(Color::Black, start + 3 * MINUTE), Duration::ZERO);
        assert_eq!(clock.flagged(start + 3 * MINUTE), None);
    }

    #[test]
    fn formats_time() {
        assert_eq!(format(65 * SECOND), "1:05");
        assert_eq!(format(3725 * SECOND), "1:02:05");
    }
}
