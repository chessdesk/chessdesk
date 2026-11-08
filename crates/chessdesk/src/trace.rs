//! What went wrong, written up to copy or to file as a GitHub issue: enough to act on, and
//! nothing that says who the player is.

use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use url::Url;

const ISSUES: &str = "https://github.com/chessdesk/chessdesk/issues/new";
/// The most errors kept for the log, the latest ones.
const LOG_LENGTH: usize = 50;
/// The most of a report that goes into an issue link: browsers and GitHub turn away much
/// longer addresses, and the full report can still be pasted.
const ISSUE_BODY_LENGTH: usize = 6000;

/// The errors and notices of this run, and when each came.
pub struct Trace {
    started: Instant,
    log: Vec<(Duration, String)>,
}

impl Trace {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            log: Vec::new(),
        }
    }

    pub fn record(&mut self, error: &str) {
        if self.log.len() == LOG_LENGTH {
            self.log.remove(0);
        }
        self.log.push((self.started.elapsed(), error.to_owned()));
    }

    /// `error` in full, what it happened in, and the log of this run, with the home folder
    /// written as `~`: paths under it hold the user's name.
    pub fn report(&self, error: &str, game: &str) -> String {
        let mut report = format!(
            "{error}\n\nChessdesk {} on {} {}\nGame: {game}\n\nErrors and notices this run:\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
        for (at, logged) in &self.log {
            let seconds = at.as_secs();
            let _ = writeln!(report, "[{:02}:{:02}] {logged}", seconds / 60, seconds % 60);
        }
        redact(&report, dirs::home_dir().as_deref())
    }
}

/// A new GitHub issue about `report`, filled in.
pub fn issue_url(report: &str) -> String {
    let title = report.lines().next().unwrap_or_default();
    let body = format!("```\n{}\n```\n", shortened(report, ISSUE_BODY_LENGTH));
    Url::parse_with_params(ISSUES, [("title", title), ("body", &body)])
        .map_or_else(|_| ISSUES.to_owned(), String::from)
}

/// At most `most` bytes of `text`, cut between characters.
fn shortened(text: &str, most: usize) -> &str {
    let mut end = most.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// `text` with the home folder written as `~`, however its separators are written.
fn redact(text: &str, home: Option<&Path>) -> String {
    let Some(home) = home.map(|home| home.to_string_lossy().into_owned()) else {
        return text.to_owned();
    };
    if home.len() < 2 {
        return text.to_owned();
    }
    let mut redacted = text.to_owned();
    for spelling in [
        home.clone(),
        home.replace('\\', "/"),
        home.replace('/', "\\"),
    ] {
        redacted = redacted.replace(&spelling, "~");
    }
    redacted
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn the_home_folder_is_left_out_in_any_spelling() {
        let home = Path::new(r"C:\Users\ada");
        let text = r"no C:\Users\ada\x.exe nor C:/Users/ada/y here";
        assert_eq!(redact(text, Some(home)), r"no ~\x.exe nor ~/y here");
    }

    #[test]
    fn without_a_home_folder_nothing_changes() {
        assert_eq!(redact("as is", None), "as is");
    }

    #[test]
    fn a_report_holds_the_error_and_the_log() {
        let mut trace = Trace::new();
        trace.record("first");
        trace.record("second");
        let report = trace.report("second\nin detail", "Stockfish, 12 moves");
        assert!(report.starts_with("second\nin detail\n"));
        assert!(report.contains("Game: Stockfish, 12 moves"));
        assert!(report.contains("] first\n"));
        assert!(report.contains("] second\n"));
    }

    #[test]
    fn the_log_keeps_the_latest_errors() {
        let mut trace = Trace::new();
        for index in 0..=LOG_LENGTH {
            trace.record(&index.to_string());
        }
        assert_eq!(trace.log.len(), LOG_LENGTH);
        assert_eq!(trace.log[0].1, "1");
    }

    #[test]
    fn an_issue_is_titled_with_the_first_line() {
        let url = Url::parse(&issue_url("It broke\ndetails")).unwrap();
        let title = url.query_pairs().find(|(key, _)| key == "title").unwrap().1;
        assert_eq!(title, "It broke");
    }

    proptest! {
        #[test]
        fn shortening_never_splits_a_char(text in "\\PC{0,40}", most in 0usize..200) {
            let short = shortened(&text, most);
            prop_assert!(text.starts_with(short));
            prop_assert!(short.len() <= most);
        }
    }
}
