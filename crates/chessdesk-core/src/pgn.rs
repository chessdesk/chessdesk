//! Games to and from PGN. Everything Chessdesk knows about a move lives in standard
//! comment commands, so saved games open in other chess tools and keep their analysis.

use std::fmt::{self, Write as _};
use std::io;
use std::ops::ControlFlow;
use std::time::Duration;

use pgn_reader::{Nag, Outcome, RawComment, RawTag, Reader, SanPlus, Skip, Visitor};
use shakmaty::{Chess, Position};

use crate::annotation::{split_commands, strip_commands};
use crate::game::fen_of;
use crate::{Annotations, Eval, Game, PlayedMove, Score, Wdl};

/// Tags every PGN starts with, in this order, and their values if unknown. `Result`
/// completes the standard's seven.
const ROSTER: [(&str, &str); 6] = [
    ("Event", "Casual game"),
    ("Site", "Chessdesk"),
    ("Date", "????.??.??"),
    ("Round", "-"),
    ("White", "?"),
    ("Black", "?"),
];
/// Where movetext is broken, as the PGN standard asks.
const LINE_WIDTH: usize = 80;

fn result_text(game: &Game) -> &'static str {
    match game.outcome() {
        Outcome::Known(outcome) => outcome.as_str(),
        Outcome::Unknown => "*",
    }
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn format_clock(clock: Duration) -> String {
    let seconds = clock.as_secs();
    format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

fn parse_clock(text: &str) -> Option<Duration> {
    let mut parts = text.split(':');
    let hours: u64 = parts.next()?.parse().ok()?;
    let minutes: u64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let seconds = Duration::try_from_secs_f64(seconds).ok()?;
    Some(Duration::from_secs(hours * 3600 + minutes * 60) + seconds)
}

/// A move's comment, without braces, if it has anything to say.
fn comment(played: &PlayedMove) -> Option<String> {
    let mut parts = Vec::new();
    // A closing brace ends a comment, and PGN has no way to escape it.
    let text = played.annotations.text.replace(['{', '}'], "");
    if !text.is_empty() {
        parts.push(text);
    }
    parts.extend(played.annotations.commands());
    if let Some(score) = played.eval.score {
        parts.push(format!("[%eval {score}]"));
    }
    if let Some(wdl) = played.eval.wdl {
        parts.push(format!("[%wdl {wdl}]"));
    }
    if let Some(clock) = played.clock {
        parts.push(format!("[%clk {}]", format_clock(clock)));
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

fn write_tags(game: &Game, pgn: &mut String) {
    let mut tag = |name: &str, value: &str| {
        let _ = writeln!(pgn, "[{name} \"{}\"]", escape(value));
    };
    for (name, unknown) in ROSTER {
        tag(name, game.tag(name).unwrap_or(unknown));
    }
    tag("Result", result_text(game));
    if *game.start() != Chess::default() {
        tag("SetUp", "1");
        tag("FEN", &fen_of(game.start()));
    }
    for (name, value) in &game.tags {
        if !ROSTER.iter().any(|(known, _)| known == name) {
            tag(name, value);
        }
    }
}

/// The game's notes as a comment, if there are any.
fn notes(game: &Game) -> Option<String> {
    // As with a move's comment, braces would end it early.
    let notes = game.notes.replace(['{', '}'], "");
    let notes = notes.trim();
    (!notes.is_empty()).then(|| format!("{{{notes}}}"))
}

/// The movetext as units that lines may break between.
fn movetext(game: &Game) -> Vec<String> {
    // The game's own comment comes before its first move.
    let mut tokens: Vec<String> = notes(game).into_iter().collect();
    let mut position = game.start().clone();
    let mut needs_number = true;
    for played in game.moves() {
        let number = position.fullmoves();
        tokens.push(if position.turn().is_white() {
            format!("{number}. {}", played.san)
        } else if needs_number {
            format!("{number}... {}", played.san)
        } else {
            played.san.to_string()
        });
        tokens.extend(played.nags.iter().map(|nag| format!("${nag}")));
        let text = comment(played);
        // After a comment, Black's move needs its number again.
        needs_number = text.is_some();
        tokens.extend(text.map(|text| format!("{{{text}}}")));
        position.play_unchecked(played.played);
    }
    tokens.push(result_text(game).to_owned());
    tokens
}

/// Writes a game as PGN.
pub fn write(game: &Game) -> String {
    let mut pgn = String::new();
    write_tags(game, &mut pgn);
    pgn.push('\n');
    let mut width = 0;
    for token in movetext(game) {
        if width > 0 && width + token.len() + 1 > LINE_WIDTH {
            pgn.push('\n');
            width = 0;
        } else if width > 0 {
            pgn.push(' ');
            width += 1;
        }
        pgn.push_str(&token);
        // Notes over several lines go on from where their last line ends.
        width = match token.rfind('\n') {
            Some(newline) => token.len() - newline - 1,
            None => width + token.len(),
        };
    }
    pgn.push('\n');
    pgn
}

/// Writes a game as PGN with nothing but its moves: no notes, comments, marks or evaluations.
pub fn write_moves_only(game: &Game) -> String {
    let mut plain = game.clone();
    plain.notes.clear();
    for played in plain.moves_mut() {
        played.nags.clear();
        played.annotations = Annotations::default();
        played.eval = Eval::default();
        played.clock = None;
    }
    write(&plain)
}

/// Why a PGN could not be read.
#[derive(Debug)]
pub enum ReadError {
    Io(io::Error),
    Empty,
    BadStart(String),
    IllegalMove { ply: usize, san: String },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "could not read the game: {error}"),
            Self::Empty => f.write_str("there is no game in this file"),
            Self::BadStart(fen) => write!(f, "bad starting position: {fen}"),
            Self::IllegalMove { ply, san } => write!(f, "illegal move {san} at ply {ply}"),
        }
    }
}

impl std::error::Error for ReadError {}

/// Builds a [`Game`] from what the reader finds.
struct Builder;

impl Visitor for Builder {
    type Tags = Vec<(String, String)>;
    type Movetext = Game;
    type Output = Result<Game, ReadError>;

    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> {
        ControlFlow::Continue(Vec::new())
    }

    fn tag(
        &mut self,
        tags: &mut Self::Tags,
        name: &[u8],
        value: RawTag<'_>,
    ) -> ControlFlow<Self::Output> {
        tags.push((
            String::from_utf8_lossy(name).into_owned(),
            value.decode_utf8_lossy().into_owned(),
        ));
        ControlFlow::Continue(())
    }

    fn begin_movetext(&mut self, tags: Self::Tags) -> ControlFlow<Self::Output, Self::Movetext> {
        let start = match tags.iter().find(|(name, _)| name == "FEN") {
            Some((_, fen)) => match Game::parse_fen(fen) {
                Some(start) => start,
                None => return ControlFlow::Break(Err(ReadError::BadStart(fen.clone()))),
            },
            None => Chess::default(),
        };
        let mut game = Game::from_position(start);
        game.tags = tags
            .into_iter()
            .filter(|(name, _)| !matches!(name.as_str(), "FEN" | "SetUp" | "Result"))
            .collect();
        ControlFlow::Continue(game)
    }

    fn begin_variation(&mut self, _: &mut Self::Movetext) -> ControlFlow<Self::Output, Skip> {
        // Chessdesk's games have one line.
        ControlFlow::Continue(Skip(true))
    }

    fn san(&mut self, game: &mut Self::Movetext, san: SanPlus) -> ControlFlow<Self::Output> {
        let legal = san.san.to_move(&game.position()).ok();
        if legal.is_some_and(|played| game.play(played)) {
            return ControlFlow::Continue(());
        }
        ControlFlow::Break(Err(ReadError::IllegalMove {
            ply: game.moves().len() + 1,
            san: san.to_string(),
        }))
    }

    fn nag(&mut self, game: &mut Self::Movetext, nag: Nag) -> ControlFlow<Self::Output> {
        if let Some(last) = game.moves_mut().last_mut() {
            last.nags.push(nag.0);
        }
        ControlFlow::Continue(())
    }

    fn comment(
        &mut self,
        game: &mut Self::Movetext,
        comment: RawComment<'_>,
    ) -> ControlFlow<Self::Output> {
        let comment = String::from_utf8_lossy(comment.as_bytes());
        if let Some(last) = game.moves_mut().last_mut() {
            apply_comment(last, &comment);
            return ControlFlow::Continue(());
        }
        // Before the first move, it's about the game as a whole.
        let notes = strip_commands(&comment);
        if !notes.is_empty() {
            if !game.notes.is_empty() {
                game.notes.push('\n');
            }
            game.notes.push_str(&notes);
        }
        ControlFlow::Continue(())
    }

    fn outcome(
        &mut self,
        game: &mut Self::Movetext,
        outcome: Outcome,
    ) -> ControlFlow<Self::Output> {
        if let Outcome::Known(known) = outcome
            && game.position().outcome() != outcome
        {
            game.adjudicate(known);
        }
        ControlFlow::Continue(())
    }

    fn end_game(&mut self, game: Self::Movetext) -> Self::Output {
        Ok(game)
    }
}

fn apply_comment(played: &mut PlayedMove, comment: &str) {
    let (text, commands) = split_commands(comment);
    let mut annotations = Annotations {
        text,
        ..Annotations::default()
    };
    for (name, args) in commands {
        match name {
            "eval" => played.eval.score = Score::parse(args.trim()),
            "wdl" => played.eval.wdl = Wdl::parse(args),
            "clk" => played.clock = parse_clock(args.trim()),
            _ => annotations.add_command(name, args),
        }
    }
    played.annotations.merge(annotations);
}

/// Reads the first game of a PGN.
pub fn read(pgn: &str) -> Result<Game, ReadError> {
    Reader::new(io::Cursor::new(pgn.as_bytes()))
        .read_game(&mut Builder)
        .map_err(ReadError::Io)?
        .ok_or(ReadError::Empty)?
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use shakmaty::{Color, KnownOutcome, Square};

    use super::*;
    use crate::{Arrow, Highlight, Mark};

    #[test]
    fn writes_the_roster_and_movetext() {
        let mut game = Game::default();
        game.set_tag("White", "You");
        game.set_tag("Black", "Claude Code");
        let moves = game.position().legal_moves();
        let e4 = moves.iter().find(|m| m.to() == Square::E4).unwrap();
        assert!(game.play(*e4));
        let pgn = write(&game);
        assert!(pgn.starts_with("[Event \"Casual game\"]\n[Site \"Chessdesk\"]\n"));
        assert!(pgn.contains("[White \"You\"]\n[Black \"Claude Code\"]\n[Result \"*\"]\n"));
        assert!(pgn.ends_with("\n1. e4 *\n"));
    }

    #[test]
    fn keeps_analysis_through_a_round_trip() {
        let mut game = Game::default();
        let first = game.position().legal_moves()[0];
        assert!(game.play(first));
        let played = &mut game.moves_mut()[0];
        played.nags.push(2);
        played.clock = Some(Duration::from_secs(3725));
        played.eval = Eval {
            score: Some(Score::Mate(-3)),
            wdl: Some(Wdl {
                white: 10,
                draw: 40,
                black: 950,
            }),
        };
        played.annotations = Annotations {
            highlights: vec![Highlight {
                square: Square::D4,
                mark: Mark::Red,
            }],
            arrows: vec![Arrow {
                from: Square::E2,
                to: Square::E4,
                mark: Mark::Blue,
            }],
            text: "A quiet move".to_owned(),
        };
        assert_eq!(read(&write(&game)).unwrap().moves(), game.moves());
    }

    #[test]
    fn moves_only_leaves_out_everything_else() {
        let mut game = Game::default();
        let first = game.position().legal_moves()[0];
        assert!(game.play(first));
        let played = &mut game.moves_mut()[0];
        played.nags.push(4);
        played.annotations.text = "Oops".to_owned();
        played.eval.score = Some(Score::Centipawns(10));
        let pgn = write_moves_only(&game);
        assert!(!pgn.contains("Oops") && !pgn.contains("%eval") && !pgn.contains('$'));
        assert_eq!(read(&pgn).unwrap().moves().len(), 1);
    }

    #[test]
    fn keeps_notes_with_their_line_breaks_before_the_first_move() {
        let mut game = Game::default();
        let first = game.position().legal_moves()[0];
        assert!(game.play(first));
        game.notes = "Try the London next time.\nWatch the f7 square.".to_owned();
        let pgn = write(&game);
        // The moves go on after the notes' last line, not on a line of their own.
        assert!(pgn.contains("{Try the London next time.\nWatch the f7 square.} 1. "));
        let read = read(&pgn).unwrap();
        assert_eq!(read.notes, game.notes);
        assert_eq!(read.moves()[0].annotations.text, "");
    }

    #[test]
    fn reads_another_tools_game_comment_as_notes() {
        let read = read("{Club night, round 3 [%evp 0,20]} 1. e4 {Best by test} e5 *").unwrap();
        assert_eq!(read.notes, "Club night, round 3");
        assert_eq!(read.moves()[0].annotations.text, "Best by test");
    }

    #[test]
    fn moves_only_leaves_out_the_notes() {
        let mut game = Game::default();
        game.notes = "Private".to_owned();
        assert!(!write_moves_only(&game).contains("Private"));
    }

    #[test]
    fn keeps_a_custom_start_and_a_resignation() {
        let fen = "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1";
        let mut game = Game::from_position(Game::parse_fen(fen).unwrap());
        game.adjudicate(KnownOutcome::Decisive {
            winner: Color::White,
        });
        let read = read(&write(&game)).unwrap();
        assert_eq!(read.fen(), fen);
        assert_eq!(read.outcome(), game.outcome());
    }

    #[test]
    fn skips_variations_and_reports_illegal_moves() {
        assert_eq!(read("1. e4 (1. d4 d5) e5 *").unwrap().moves().len(), 2);
        assert!(matches!(
            read("1. e4 e4 *"),
            Err(ReadError::IllegalMove { ply: 2, .. })
        ));
    }

    proptest! {
        #[test]
        fn random_games_round_trip(
            choices in prop::collection::vec(any::<prop::sample::Index>(), 0..120),
            // Notes as typed: words and line breaks, with nothing around them.
            notes in "([a-zA-Z0-9.,!?]{1,8}( |\n)){0,6}[a-zA-Z0-9.,!?]{0,8}",
        ) {
            let mut game = Game::default();
            game.notes = notes.trim().to_owned();
            for choice in choices {
                let moves = game.position().legal_moves();
                if moves.is_empty() {
                    break;
                }
                prop_assert!(game.play(moves[choice.index(moves.len())]));
            }
            let read = read(&write(&game)).unwrap();
            prop_assert_eq!(read.moves(), game.moves());
            prop_assert_eq!(read.outcome(), game.outcome());
            prop_assert_eq!(read.notes, game.notes);
        }
    }
}
