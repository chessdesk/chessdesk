//! What an update brings, in a few lines to show beside its button: Chessdesk's from its
//! changelog, an engine's from the opening of its release notes.

use semver::Version;

/// The longest an engine's summary may run before it's cut at a sentence.
const SUMMARY_LENGTH: usize = 240;

/// What a version brought, as the changelog tells players.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notes {
    pub version: Version,
    pub entries: Vec<String>,
}

/// Each version in `changelog` newer than `current`, newest first, with its entries. Reads
/// Keep a Changelog's `## 1.2.0 - date` sections: each `- ` entry is joined onto one line and
/// its Markdown dropped, and the `###` headings between them are skipped.
pub fn since(changelog: &str, current: &Version) -> Vec<Notes> {
    let mut notes: Vec<Notes> = Vec::new();
    let mut taking = false;
    for line in changelog.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let version = heading
                .split_whitespace()
                .next()
                .and_then(|word| Version::parse(word.trim_start_matches('v')).ok());
            taking = version.as_ref().is_some_and(|version| version > current);
            if let (true, Some(version)) = (taking, version) {
                notes.push(Notes {
                    version,
                    entries: Vec::new(),
                });
            }
            continue;
        }
        let Some(entries) = notes.last_mut().filter(|_| taking).map(|n| &mut n.entries) else {
            continue;
        };
        if let Some(entry) = line.strip_prefix("- ") {
            entries.push(entry.trim().to_owned());
        } else if line.starts_with("  ")
            && let Some(entry) = entries.last_mut()
        {
            entry.push(' ');
            entry.push_str(line.trim());
        }
    }
    for note in &mut notes {
        for entry in &mut note.entries {
            *entry = plain(entry);
        }
    }
    notes
}

/// The opening paragraph of an engine's release notes, cut after the last sentence that fits,
/// when it says something by itself: not a heading, nor a line leading into a list.
pub fn summary(body: &str) -> Option<String> {
    let body = body.replace("\r\n", "\n");
    let paragraph = body.split("\n\n").map(str::trim).find(|p| !p.is_empty())?;
    // A list may follow its lead-in line without a blank line between them.
    let leads_in = paragraph.lines().any(|line| line.trim_end().ends_with(':'));
    if paragraph.starts_with('#') || leads_in {
        return None;
    }
    let text = plain(&paragraph.split_whitespace().collect::<Vec<_>>().join(" "));
    if text.len() <= SUMMARY_LENGTH {
        return Some(text);
    }
    let end = text.floor_char_boundary(SUMMARY_LENGTH);
    let cut = text[..end].rfind(". ")?;
    Some(text[..=cut].to_owned())
}

/// `text` without its Markdown: links keep their text, `[text](url)` and `[text][1]` alike,
/// and code and bold marks go.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let inner = &rest[open + 1..];
        let Some(close) = inner.find(']') else {
            out.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let after = &inner[close + 1..];
        let target = match after.chars().next() {
            Some('(') => after.find(')'),
            Some('[') => after.find(']'),
            _ => None,
        };
        if let Some(end) = target {
            out.push_str(&inner[..close]);
            rest = &after[end + 1..];
        } else {
            // Brackets that aren't a link stay as they are.
            out.push('[');
            rest = inner;
        }
    }
    out.push_str(rest);
    out.replace('`', "").replace("**", "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fmt::Write as _;

    const CHANGELOG: &str = "# Changelog

Notable changes per version.

## Unreleased

- Not out yet

## 2.10.0 - 2026-11-13

### Added

- Shadowchess plays at every rating from 500 to 1800, from
  [chessdesk/shadowchess](https://github.com/chessdesk/shadowchess)
- Updates for the `engines`

### Fixed

- A **bold** fix

## 2.9.0 - 2026-11-13

- Shadowchess

## 2.8.1 - 2026-11-10

- Older
";

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn takes_the_versions_newer_than_the_running_one() {
        let notes = since(CHANGELOG, &version("2.8.1"));
        assert_eq!(
            notes,
            [
                Notes {
                    version: version("2.10.0"),
                    entries: vec![
                        "Shadowchess plays at every rating from 500 to 1800, from \
                         chessdesk/shadowchess"
                            .to_owned(),
                        "Updates for the engines".to_owned(),
                        "A bold fix".to_owned(),
                    ],
                },
                Notes {
                    version: version("2.9.0"),
                    entries: vec!["Shadowchess".to_owned()],
                },
            ]
        );
        assert_eq!(since(CHANGELOG, &version("2.10.0")), []);
    }

    #[test]
    fn an_engine_summary_is_its_opening_sentences() {
        let stockfish = "Today, we have the pleasure of announcing **Stockfish 19**, a new \
            major release. As always, you can freely download it at \
            [stockfishchess.org/download][1] and use it as a drop-in replacement in the [GUI \
            of your choice][2] to benefit from stronger play and more accurate analysis, in \
            whatever you play with today.\r\n\r\nWhether you can spare hours...";
        assert_eq!(
            summary(stockfish).as_deref(),
            Some("Today, we have the pleasure of announcing Stockfish 19, a new major release.")
        );
        assert_eq!(
            summary("Levels 500 to 1800.\n\nMore.").as_deref(),
            Some("Levels 500 to 1800.")
        );
        // lc0 leads into a list, which says nothing by itself.
        assert_eq!(summary("In this version:\n* Faster"), None);
        assert_eq!(summary("## Changes\n\n- a"), None);
        assert_eq!(summary(""), None);
    }

    #[test]
    fn markdown_goes_but_text_stays() {
        assert_eq!(
            plain("see [the page](https://x) and [docs][3]"),
            "see the page and docs"
        );
        assert_eq!(plain("a [bracket] and [half"), "a [bracket] and [half");
        assert_eq!(plain("`code` and **bold**"), "code and bold");
    }

    proptest! {
        /// A changelog written from entries reads back as those entries, for the versions
        /// newer than the running one.
        #[test]
        fn reads_back_what_was_written(
            versions in prop::collection::btree_set((0u64..4, 0u64..12, 0u64..4), 1..6),
            entries in prop::collection::vec(
                prop::collection::vec("[A-Za-z][A-Za-z0-9 ,.]{0,40}[a-z]", 1..4), 6),
            running in (0u64..4, 0u64..12, 0u64..4),
        ) {
            let versions: Vec<Version> =
                versions.into_iter().rev().map(|(a, b, c)| Version::new(a, b, c)).collect();
            let running = Version::new(running.0, running.1, running.2);
            let mut changelog = String::from("# Changelog\n\n## Unreleased\n\n- Soon\n");
            for (version, entries) in versions.iter().zip(&entries) {
                writeln!(changelog, "\n## {version} - 2026-01-01\n\n### Added\n").unwrap();
                for entry in entries {
                    writeln!(changelog, "- {entry}").unwrap();
                }
            }
            let expected: Vec<Notes> = versions
                .iter()
                .zip(&entries)
                .filter(|(version, _)| **version > running)
                .map(|(version, entries)| Notes {
                    version: version.clone(),
                    entries: entries.iter().map(|entry| entry.trim().to_owned()).collect(),
                })
                .collect();
            prop_assert_eq!(since(&changelog, &running), expected);
        }
    }
}
