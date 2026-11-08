//! What can be drawn on the board, in the PGN comment commands that chess tools share:
//! `[%csl Gd4,Ye5]` highlights squares, `[%cal Ge2e4]` draws arrows. Agents reply in the
//! same syntax, so it costs few tokens and needs no format of its own.

use shakmaty::Square;

/// A marking's colour, by the letter PGN uses. The theme decides how it looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Green,
    Red,
    Yellow,
    Blue,
}

impl Mark {
    fn letter(self) -> char {
        match self {
            Self::Green => 'G',
            Self::Red => 'R',
            Self::Yellow => 'Y',
            Self::Blue => 'B',
        }
    }

    fn from_letter(letter: char) -> Option<Self> {
        match letter {
            'G' => Some(Self::Green),
            'R' => Some(Self::Red),
            'Y' => Some(Self::Yellow),
            'B' => Some(Self::Blue),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Highlight {
    pub square: Square,
    pub mark: Mark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arrow {
    pub from: Square,
    pub to: Square,
    pub mark: Mark,
}

/// The markings and words attached to a move.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Annotations {
    pub highlights: Vec<Highlight>,
    pub arrows: Vec<Arrow>,
    pub text: String,
}

impl Annotations {
    pub fn is_empty(&self) -> bool {
        self.highlights.is_empty() && self.arrows.is_empty() && self.text.is_empty()
    }

    /// Adds what `other` brings and this doesn't have yet, so that analysing a game
    /// twice doesn't mark it twice.
    pub fn merge(&mut self, other: Self) {
        for highlight in other.highlights {
            if !self.highlights.contains(&highlight) {
                self.highlights.push(highlight);
            }
        }
        for arrow in other.arrows {
            if !self.arrows.contains(&arrow) {
                self.arrows.push(arrow);
            }
        }
        if !other.text.is_empty() && !self.text.contains(&other.text) {
            if !self.text.is_empty() {
                self.text.push(' ');
            }
            self.text.push_str(&other.text);
        }
    }

    /// Takes in a `[%csl ..]` or `[%cal ..]` command; any other is ignored. Entries that
    /// don't parse are dropped: a bad marking must never cost a move.
    pub(crate) fn add_command(&mut self, name: &str, args: &str) {
        let entries = args.split(',').map(str::trim);
        match name {
            "csl" => self.highlights.extend(entries.filter_map(parse_highlight)),
            "cal" => self.arrows.extend(entries.filter_map(parse_arrow)),
            _ => {}
        }
    }

    /// The `[%csl ..]` and `[%cal ..]` commands, without the text.
    pub(crate) fn commands(&self) -> Vec<String> {
        let mut commands = Vec::new();
        if !self.highlights.is_empty() {
            let entries: Vec<_> = self
                .highlights
                .iter()
                .map(|h| format!("{}{}", h.mark.letter(), h.square))
                .collect();
            commands.push(format!("[%csl {}]", entries.join(",")));
        }
        if !self.arrows.is_empty() {
            let entries: Vec<_> = self
                .arrows
                .iter()
                .map(|a| format!("{}{}{}", a.mark.letter(), a.from, a.to))
                .collect();
            commands.push(format!("[%cal {}]", entries.join(",")));
        }
        commands
    }
}

fn parse_highlight(entry: &str) -> Option<Highlight> {
    let mark = Mark::from_letter(entry.chars().next()?)?;
    let square = entry.get(1..)?.parse().ok()?;
    Some(Highlight { square, mark })
}

fn parse_arrow(entry: &str) -> Option<Arrow> {
    let mark = Mark::from_letter(entry.chars().next()?)?;
    let from = entry.get(1..3)?.parse().ok()?;
    let to = entry.get(3..)?.parse().ok()?;
    Some(Arrow { from, to, mark })
}

/// Splits a comment into its plain text, on one line, and its `[%name args]` commands.
pub fn split_commands(comment: &str) -> (String, Vec<(&str, &str)>) {
    let (text, commands) = take_commands(comment);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (text, commands)
}

/// A comment's plain text with its line breaks kept, its commands left out.
pub fn strip_commands(comment: &str) -> String {
    take_commands(comment).0.trim().to_owned()
}

/// Takes the `[%name args]` commands out of a comment, leaving its text spaced as written.
fn take_commands(comment: &str) -> (String, Vec<(&str, &str)>) {
    let mut text = String::new();
    let mut commands = Vec::new();
    let mut rest = comment;
    while let Some(start) = rest.find("[%") {
        let Some(length) = rest[start..].find(']') else {
            break;
        };
        text.push_str(&rest[..start]);
        let command = rest[start + 2..start + length].trim();
        commands.push(command.split_once(' ').unwrap_or((command, "")));
        rest = &rest[start + length + 1..];
    }
    text.push_str(rest);
    (text, commands)
}

/// Reads the markings and text of a comment, ignoring any other commands.
pub fn parse(comment: &str) -> Annotations {
    let (text, commands) = split_commands(comment);
    let mut annotations = Annotations {
        text,
        ..Annotations::default()
    };
    for (name, args) in commands {
        annotations.add_command(name, args);
    }
    annotations
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use shakmaty::Square::{D4, E2, E4, E5};

    use super::*;

    #[test]
    fn reads_markings_and_text() {
        let parsed = parse("Pressures e5 [%cal Ge2e4] [%csl Rd4,Ye5]");
        assert_eq!(parsed.text, "Pressures e5");
        assert_eq!(
            parsed.arrows,
            [Arrow {
                from: E2,
                to: E4,
                mark: Mark::Green
            }]
        );
        assert_eq!(
            parsed.highlights,
            [
                Highlight {
                    square: D4,
                    mark: Mark::Red
                },
                Highlight {
                    square: E5,
                    mark: Mark::Yellow
                },
            ]
        );
    }

    #[test]
    fn drops_bad_entries_and_keeps_good_ones() {
        let parsed = parse("[%csl Xd4,Gz9,Ge5,G] [%cal Ge2,Ge2e4,é]");
        assert_eq!(parsed.highlights.len(), 1);
        assert_eq!(parsed.arrows.len(), 1);
    }

    #[test]
    fn merging_twice_changes_nothing() {
        let marks = parse("Nice [%csl Gd4] [%cal Ge2e4]");
        let mut merged = marks.clone();
        merged.merge(marks.clone());
        assert_eq!(merged, marks);
    }

    #[test]
    fn leaves_an_unclosed_command_as_text() {
        assert_eq!(parse("nice [%csl Gd4").text, "nice [%csl Gd4");
    }

    fn mark() -> impl Strategy<Value = Mark> {
        prop_oneof![
            Just(Mark::Green),
            Just(Mark::Red),
            Just(Mark::Yellow),
            Just(Mark::Blue)
        ]
    }

    fn square() -> impl Strategy<Value = Square> {
        (0u32..64).prop_map(Square::new)
    }

    proptest! {
        #[test]
        fn commands_round_trip(
            highlights in prop::collection::vec((square(), mark()), 0..6),
            arrows in prop::collection::vec((square(), square(), mark()), 0..6),
        ) {
            let annotations = Annotations {
                highlights: highlights
                    .into_iter()
                    .map(|(square, mark)| Highlight { square, mark })
                    .collect(),
                arrows: arrows
                    .into_iter()
                    .map(|(from, to, mark)| Arrow { from, to, mark })
                    .collect(),
                text: String::new(),
            };
            prop_assert_eq!(parse(&annotations.commands().join(" ")), annotations);
        }

        #[test]
        fn any_comment_parses(comment in any::<String>()) {
            parse(&comment);
        }
    }
}
