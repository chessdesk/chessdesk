//! Reading what agents write. Replies are meant to be one move, but models add move
//! numbers, punctuation and the odd sentence, so this looks for the first legal move.

use std::borrow::Cow;

use chessdesk_core::annotation::{self, split_commands};
use chessdesk_core::{Annotations, Game};
use shakmaty::san::San;
use shakmaty::uci::UciMove;
use shakmaty::{Chess, Move, Position};

/// Strips what surrounds a move in prose: `1...`, `**Nf3**`, `e5.`. Castling comes out as
/// `O-O`, the one way the parser reads it, however it was written.
fn clean(word: &str) -> Cow<'_, str> {
    let word =
        word.trim_matches(|c: char| matches!(c, '*' | '`' | '"' | '\'' | ',' | ';' | '(' | ')'));
    let word = without_move_number(word).trim_end_matches(['.', '!', '?', ':']);
    castling(word).map_or(Cow::Borrowed(word), Cow::Owned)
}

/// `e4` from `1.e4` and `e5` from `1...e5`. Digits without dots are left: `0-0` is a move.
fn without_move_number(word: &str) -> &str {
    let rest = word.trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.starts_with('.') {
        rest.trim_start_matches('.')
    } else {
        word
    }
}

/// `0-0`, `o-o-o+` and the like as `O-O` and `O-O-O+`. Models write zeros often, which
/// isn't SAN, and a castling read as no move left the comment's moves to be played instead.
fn castling(word: &str) -> Option<String> {
    let (body, check) = word.split_at(word.trim_end_matches(['+', '#']).len());
    let normal = body.replace(['0', 'o'], "O");
    matches!(normal.as_str(), "O-O" | "O-O-O")
        .then(|| format!("{normal}{check}"))
        .filter(|castle| castle != word)
}

fn parse_move(word: &str, position: &Chess) -> Option<Move> {
    let word = clean(word);
    if let Ok(san) = word.parse::<shakmaty::san::SanPlus>()
        && let Ok(played) = san.san.to_move(position)
    {
        return Some(played);
    }
    word.parse::<UciMove>().ok()?.to_move(position).ok()
}

/// The first legal move in a reply, and the reply's markings. Whatever isn't the move
/// counts as the comment, unless it's in the way of nothing but the move.
pub fn parse(reply: &str, position: &Chess) -> Option<(Move, Annotations)> {
    let (text, _) = split_commands(reply);
    let (played, word) = text
        .split_whitespace()
        .find_map(|word| Some((parse_move(word, position)?, word)))?;
    let mut annotations = annotation::parse(reply);
    annotations.text = comment_text(&text, word);
    Some((played, annotations))
}

/// What the agent said. A comment in braces is all of it: models sometimes carry on
/// thinking aloud after it. Without braces, it's the words other than the move.
fn comment_text(text: &str, moved: &str) -> String {
    let braced = text
        .split_once('{')
        .and_then(|(_, rest)| rest.split_once('}'))
        .map(|(comment, _)| comment.to_owned());
    let rest = braced.unwrap_or_else(|| text.replacen(moved, "", 1));
    let is_number = |word: &&str| word.chars().all(|c| c.is_ascii_digit() || c == '.');
    let words: Vec<_> = rest
        .split_whitespace()
        .filter(|word| !is_number(word))
        .collect();
    words.join(" ")
}

/// The comments of a review, one line per move: `14... Nf6 {why} [%cal Gf6e4]`. Lines
/// that don't name a move of the game are skipped, so a wrong line costs only itself.
pub fn apply_review(reply: &str, game: &mut Game) -> usize {
    let mut applied = 0;
    for line in reply.lines() {
        let Some((index, rest)) = review_line(line, game) else {
            continue;
        };
        let mut annotations = annotation::parse(&rest.replace(['{', '}'], ""));
        if !annotations.is_empty() {
            let shown = &mut game.moves_mut()[index].annotations;
            // A new review's words replace the last one's; marks add up.
            shown.text = std::mem::take(&mut annotations.text);
            shown.merge(annotations);
            applied += 1;
        }
    }
    applied
}

/// Finds which move a line is about, and returns what follows the move.
fn review_line<'a>(line: &'a str, game: &Game) -> Option<(usize, &'a str)> {
    let line = line.trim().trim_start_matches(['-', '*', ' ']);
    let (number, rest) = line.split_once('.')?;
    let number: u32 = number.trim().parse().ok()?;
    let black = rest.starts_with("..");
    let rest = rest.trim_start_matches('.').trim_start();
    let (word, comment) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let san: San = clean(word).parse::<shakmaty::san::SanPlus>().ok()?.san;

    let start = game.start();
    let half_moves = number.checked_sub(start.fullmoves().get())? * 2 + u32::from(black);
    let index = half_moves.checked_sub(u32::from(start.turn().is_black()))? as usize;
    let played = game.moves().get(index)?;
    (played.san.san == san).then_some((index, comment))
}

#[cfg(test)]
mod tests {
    use chessdesk_core::Mark;
    use shakmaty::Square;

    use super::*;

    fn after(moves: &[&str]) -> Game {
        let mut game = Game::default();
        for word in moves {
            let position = game.position();
            let (played, _) = parse(word, &position).unwrap();
            assert!(game.play(played));
        }
        game
    }

    #[test]
    fn reads_the_ways_models_write_moves() {
        let position = Chess::default();
        for reply in [
            "e4",
            "e2e4",
            "1. e4",
            "**e4**",
            "`e4`",
            "e4.",
            "1.e4",
            "I play e4!",
        ] {
            let (played, _) = parse(reply, &position).unwrap();
            assert_eq!(played.to(), Square::E4, "{reply}");
        }
        let (played, _) = parse("1...e5", &after(&["e4"]).position()).unwrap();
        assert_eq!(played.to(), Square::E5);
    }

    #[test]
    fn castling_reads_however_it_is_written() {
        let position = after(&["e4", "e5", "Nf3", "Nc6", "Bc4", "Bc5"]).position();
        for reply in [
            "O-O", "0-0", "o-o", "4. 0-0", "4.O-O", "**0-0**", "O-O+", "e1g1",
        ] {
            let (played, _) = parse(reply, &position).unwrap();
            assert!(played.is_castle(), "{reply}");
        }
        // The comment names another legal move, which must not be played instead.
        let (played, annotations) = parse("0-0 {Castling, then h3}", &position).unwrap();
        assert!(played.is_castle());
        assert_eq!(annotations.text, "Castling, then h3");
    }

    #[test]
    fn queenside_castling_reads_with_zeros() {
        let position = Game::parse_fen("r3k3/8/8/8/8/8/8/4K3 b q - 0 1").unwrap();
        let (played, _) = parse("0-0-0", &position).unwrap();
        assert!(played.is_castle());
        assert_eq!(clean("0-0-0#"), "O-O-O#");
        assert_eq!(clean("10."), "");
        assert_eq!(clean("1...e5"), "e5");
    }

    #[test]
    fn an_illegal_move_is_no_move() {
        // Black's pawn can't reach e4, and the words around it are no moves either.
        assert!(parse("e4", &after(&["e4"]).position()).is_none());
        assert!(parse("I resign", &Chess::default()).is_none());
    }

    #[test]
    fn keeps_markings_and_comment() {
        let reply = "Nf3 {Pressures e5} [%cal Gf3e5] [%csl Ye5]";
        let (played, annotations) = parse(reply, &Chess::default()).unwrap();
        assert_eq!(played.to(), Square::F3);
        assert_eq!(annotations.text, "Pressures e5");
        assert_eq!(annotations.arrows[0].mark, Mark::Green);
        assert_eq!(annotations.highlights.len(), 1);
    }

    #[test]
    fn thinking_aloud_after_the_comment_is_left_out() {
        let reply = "Nf3 {Your e4 is sound. I develop.} Wait, I need to reply with one move.";
        let (_, annotations) = parse(reply, &after(&["e4", "e5"]).position()).unwrap();
        assert_eq!(annotations.text, "Your e4 is sound. I develop.");
    }

    #[test]
    fn the_move_number_is_no_comment() {
        let position = after(&["e4"]).position();
        let (_, annotations) = parse("1... e5 {Classical}", &position).unwrap();
        assert_eq!(annotations.text, "Classical");
    }

    #[test]
    fn bad_markings_never_cost_the_move() {
        let (played, annotations) = parse("e4 [%cal nonsense]", &Chess::default()).unwrap();
        assert_eq!(played.to(), Square::E4);
        assert_eq!(annotations.arrows, []);
    }

    #[test]
    fn applies_a_review_to_the_right_moves() {
        let mut game = after(&["e4", "e5", "Nf3", "Nc6"]);
        let reply = "Here is my review:\n\
                     1. e4 {Solid} [%csl Ge4]\n\
                     - 2... Nc6 {Natural} [%cal Gc6e5]\n\
                     2. Bb5 {Not played}\n\
                     9. e4 {Past the end}\n";
        assert_eq!(apply_review(reply, &mut game), 2);
        assert_eq!(game.moves()[0].annotations.text, "Solid");
        assert_eq!(game.moves()[0].annotations.highlights.len(), 1);
        assert_eq!(game.moves()[3].annotations.arrows.len(), 1);
        assert!(game.moves()[2].annotations.is_empty());
    }

    #[test]
    fn a_second_review_replaces_the_words_of_the_first() {
        let mut game = after(&["e4"]);
        apply_review("1. e4 {First}", &mut game);
        apply_review("1. e4 {Second}", &mut game);
        assert_eq!(game.moves()[0].annotations.text, "Second");
    }

    #[test]
    fn counts_moves_from_a_custom_start() {
        // Black to move on move 10: the first move of the game is 10...
        let start = Game::parse_fen("4k3/4p3/8/8/8/8/4P3/4K3 b - - 0 10").unwrap();
        let mut game = Game::from_position(start);
        let (played, _) = parse("e5", &game.position()).unwrap();
        assert!(game.play(played));
        let (played, _) = parse("e4", &game.position()).unwrap();
        assert!(game.play(played));
        assert_eq!(apply_review("10... e5 {One}\n11. e4 {Two}", &mut game), 2);
        assert_eq!(game.moves()[1].annotations.text, "Two");
    }
}
