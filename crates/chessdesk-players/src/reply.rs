//! Reading what agents write. Replies are meant to be one move, but models add move
//! numbers, punctuation and the odd sentence, so this looks for the first legal move.

use std::borrow::Cow;

use chessdesk_core::annotation::{self, split_commands};
use chessdesk_core::{Annotations, Arrow, Game, Mark};
use shakmaty::san::San;
use shakmaty::uci::UciMove;
use shakmaty::{Chess, Color, Move, Position};

/// The command in which an agent names the player's next move, for the app to draw. Models
/// write the move well and its arrow badly: their own move instead, or the whole plan.
const NEXT: &str = "next";

/// Strips what surrounds a move in prose: `1...`, `**Nf3**`, `e5.`, `e5's`. Castling comes
/// out as `O-O`, the one way the parser reads it, however it was written.
fn clean(word: &str) -> Cow<'_, str> {
    let word = word.trim_matches(|c: char| {
        matches!(
            c,
            '*' | '`' | '"' | '\'' | ',' | ';' | '(' | ')' | '{' | '}'
        )
    });
    let word = ["'s", "’s"]
        .iter()
        .find_map(|possessive| word.strip_suffix(possessive))
        .unwrap_or(word);
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
    let reply = &spoken_next(reply);
    let (text, commands) = split_commands(reply);
    let found = text
        .split_whitespace()
        .find_map(|word| Some((parse_move(word, position)?, word)));
    let Some((played, word)) = found else {
        // Its own move put where the player's goes: `{I'll answer in the center.} [%next e5]`.
        let (_, named) = commands.iter().find(|(name, _)| *name == NEXT)?;
        let played = parse_move(named.trim(), position)?;
        let mut annotations = annotation::parse(reply);
        annotations.text = comment_text(&text, "");
        return Some((played, annotations));
    };
    let mut annotations = annotation::parse(reply);
    annotations.text = comment_text(&text, word);
    let mut after = position.clone();
    after.play_unchecked(played);
    add_next(&mut annotations, reply, &after);
    Some((played, annotations))
}

/// Keeps the move of a `[%next SAN]` written inside a comment's sentence: "Your first move:
/// [%next e4]" would read "Your first move:" once the command is taken out. Left alone where
/// the comment names the move already.
fn spoken_next(reply: &str) -> Cow<'_, str> {
    const OPEN: &str = "[%next ";
    if !reply.contains(OPEN) {
        return Cow::Borrowed(reply);
    }
    let mut spoken = String::with_capacity(reply.len() + 8);
    let mut rest = reply;
    while let Some(start) = rest.find(OPEN) {
        let Some(length) = rest[start..].find(']') else {
            break;
        };
        let before = &rest[..start];
        spoken.push_str(before);
        let named = rest[start + OPEN.len()..start + length].trim();
        let opened = spoken.rfind('{');
        let in_comment = opened.is_some_and(|open| !spoken[open..].contains('}'));
        let said = opened.is_some_and(|open| {
            spoken[open..]
                .split(|c: char| c.is_whitespace() || matches!(c, '.' | ',' | '!' | ':' | '*'))
                .any(|word| word == named)
        });
        if in_comment && !said {
            spoken.push_str(named);
            spoken.push(' ');
        }
        spoken.push_str(&rest[start..=start + length]);
        rest = &rest[start + length + 1..];
    }
    spoken.push_str(rest);
    Cow::Owned(spoken)
}

/// Draws the player's next move, if the reply names one that's legal in `position`.
fn add_next(annotations: &mut Annotations, reply: &str, position: &Chess) {
    let (_, commands) = split_commands(reply);
    let arrow = commands
        .iter()
        .find(|(name, _)| *name == NEXT)
        .and_then(|(_, named)| parse_move(named.trim(), position))
        .and_then(|next| arrow_of(next, position.turn()));
    if let Some(arrow) = arrow {
        // Drawn already as one of the agent's own arrows: once is enough, as the next move.
        annotations
            .arrows
            .retain(|drawn| (drawn.from, drawn.to) != (arrow.from, arrow.to));
        annotations.arrows.push(arrow);
    }
}

/// The player's next move as an arrow, castling from the king to where it lands. Blue, as a
/// hint's: green ones are the agent's own ideas, and a lesson's next move drawn green was
/// one arrow among several.
fn arrow_of(played: Move, mover: Color) -> Option<Arrow> {
    let to = played
        .castling_side()
        .map_or(played.to(), |side| side.king_to(mover));
    Some(Arrow {
        from: played.from()?,
        to,
        mark: Mark::Blue,
    })
}

/// What the agent said without a move: the comment of an introduction in `position`, and
/// its markings.
pub fn remark(reply: &str, position: &Chess) -> Annotations {
    let reply = &spoken_next(reply);
    let (text, _) = split_commands(reply);
    let mut annotations = annotation::parse(reply);
    annotations.text = comment_text(&text, "");
    add_next(&mut annotations, reply, position);
    annotations
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
    for line in one_move_per_line(reply).lines() {
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

/// Starts a new line at each move number outside a comment. Models sometimes write several
/// moves on one line, `1. f3 {…} 2. Kf2 {…}`, which would give the first all the comments.
/// Numbers inside a comment, as in "after 1. f3", are words, and stay.
fn one_move_per_line(reply: &str) -> String {
    let mut split = String::with_capacity(reply.len());
    let mut depth = 0usize;
    let mut previous = '\n';
    let mut chars = reply.chars().peekable();
    while let Some(next) = chars.next() {
        match next {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        let starts_number = next.is_ascii_digit() && !previous.is_ascii_digit();
        if depth == 0 && starts_number && previous == ' ' && is_move_number(chars.clone()) {
            split.push('\n');
        }
        split.push(next);
        previous = next;
    }
    split
}

/// Whether what follows a digit continues a move number: more digits, then a dot.
fn is_move_number(mut rest: impl Iterator<Item = char>) -> bool {
    rest.find(|next| !next.is_ascii_digit()) == Some('.')
}

/// The review's closing line, `Summary {how it went}`, as one line of plain text.
pub fn review_summary(reply: &str) -> Option<String> {
    let line = reply.lines().rev().map(str::trim).find(|line| {
        let line = line.trim_start_matches(['-', '*', ' ']);
        line.get(..7)
            .is_some_and(|word| word.eq_ignore_ascii_case("summary"))
    })?;
    let rest = line.trim_start_matches(['-', '*', ' '])[7..].trim_start_matches([':', '*', ' ']);
    let text = annotation::parse(&rest.replace(['{', '}'], "")).text;
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
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
    fn a_coaching_reply_keeps_the_pointer_to_the_next_move() {
        let position = after(&["e4", "e5", "Nf3"]).position();
        let reply = "Nc6 {Nice. Your bishop's still home, though.} [%cal Gf1c4]";
        let (played, said) = parse(reply, &position).unwrap();
        assert_eq!(San::from_move(&position, played).to_string(), "Nc6");
        assert_eq!(said.text, "Nice. Your bishop's still home, though.");
        assert_eq!(said.arrows.len(), 1);
    }

    #[test]
    fn an_introduction_is_its_comment_and_markings() {
        let said = remark(
            "{The Sicilian starts with c5, fighting for d4.} [%cal Gc7c5] [%csl Yd4] Ready?",
            &Chess::default(),
        );
        assert_eq!(said.text, "The Sicilian starts with c5, fighting for d4.");
        assert_eq!(said.arrows.len(), 1);
        assert_eq!(said.highlights.len(), 1);
        assert_eq!(remark("Let's go!", &Chess::default()).text, "Let's go!");
    }

    #[test]
    fn a_move_put_where_the_players_goes_is_still_played() {
        // As GLM 5.3 Flash wrote it: no move but its own, in `[%next]`.
        let position = after(&["e4"]).position();
        let reply = "{Good, clean start. I'll answer in the center.} [%next e5]";
        let (played, said) = parse(reply, &position).unwrap();
        assert_eq!(played.to(), Square::E5);
        assert_eq!(said.text, "Good, clean start. I'll answer in the center.");
        assert_eq!(said.arrows.len(), 0);
        // A possessive is no reason to miss it.
        let (played, _) = parse("{e5's legal, I'm playing it.}", &position).unwrap();
        assert_eq!(played.to(), Square::E5);
    }

    #[test]
    fn the_next_move_is_blue_and_drawn_once() {
        let reply = "Nc6 {Bring the bishop out.} [%cal Gf1c4,Gc4f7] [%next Bc4]";
        let (_, said) = parse(reply, &after(&["e4", "e5", "Qh5"]).position()).unwrap();
        let drawn: Vec<_> = said
            .arrows
            .iter()
            .map(|arrow| (arrow.from.to_string(), arrow.to.to_string(), arrow.mark))
            .collect();
        assert_eq!(
            drawn,
            [
                ("c4".to_owned(), "f7".to_owned(), Mark::Green),
                ("f1".to_owned(), "c4".to_owned(), Mark::Blue),
            ]
        );
    }

    #[test]
    fn the_players_next_move_is_drawn_when_named_and_legal() {
        let squares = |said: &Annotations| -> Vec<String> {
            said.arrows
                .iter()
                .map(|arrow| format!("{}{}", arrow.from, arrow.to))
                .collect()
        };
        // The agent has White and plays Nf3; the player's next move is ...d6.
        let reply = "Nf3 {Now ...d6, keeping e5 covered.} [%next d6]";
        let (_, said) = parse(reply, &after(&["e4", "c5"]).position()).unwrap();
        assert_eq!(said.text, "Now ...d6, keeping e5 covered.");
        assert_eq!(squares(&said), ["d7d6"]);
        // Before the first move, the start is the player's to move.
        let intro = remark("{Start with e4.} [%next e4]", &Chess::default());
        assert_eq!(squares(&intro), ["e2e4"]);
        // Not a move the player can make: nothing drawn.
        assert_eq!(
            squares(&remark("{Hm.} [%next e5]", &Chess::default())),
            [""; 0]
        );
    }

    #[test]
    fn a_next_move_written_into_the_sentence_stays_in_it() {
        // As GLM 5.3 Flash wrote them.
        let intro = remark(
            "{Your first move: [%next e4] [%csl Ge5f7]}",
            &Chess::default(),
        );
        assert_eq!(intro.text, "Your first move: e4");
        assert_eq!(intro.arrows.len(), 1);
        let position = after(&["e4", "e5", "Bc4"]).position();
        let (_, said) = parse(
            "Nc6 {Here's your chance: [%next Qh5] puts f7 in your sights.}",
            &position,
        )
        .unwrap();
        assert_eq!(said.text, "Here's your chance: Qh5 puts f7 in your sights.");
        // Named already, or outside the comment: nothing added.
        let named = remark("{Play e4. [%next e4]}", &Chess::default());
        assert_eq!(named.text, "Play e4.");
        let earlier = remark(
            "{First move: e4, grabbing the center. [%next e4]}",
            &Chess::default(),
        );
        assert_eq!(earlier.text, "First move: e4, grabbing the center.");
        let after_it = remark("{Push the king's pawn.} [%next e4]", &Chess::default());
        assert_eq!(after_it.text, "Push the king's pawn.");
    }

    #[test]
    fn castling_is_drawn_to_where_the_king_lands() {
        let position = after(&["e4", "e5", "Nf3", "Nc6", "Bc4", "Bc5"]).position();
        let said = remark("{Castle now.} [%next O-O]", &position);
        assert_eq!(said.arrows.len(), 1);
        assert_eq!(said.arrows[0].to.to_string(), "g1");
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
    fn moves_on_one_line_each_get_their_own_comment() {
        // As GLM 5.3 Flash wrote it, with a move number in the prose that isn't a move.
        let mut game = after(&["f3", "Nf6", "Kf2"]);
        let reply = "1. f3 {Weakens the king, after 1. f3 the e1-h4 line opens.} \
                     [%csl Rf3] 2. Kf2 {King on f2 already.} [%cal Gd2d4]\n\
                     Summary {Develop first.}";
        assert_eq!(apply_review(reply, &mut game), 2);
        assert_eq!(
            game.moves()[0].annotations.text,
            "Weakens the king, after 1. f3 the e1-h4 line opens."
        );
        assert_eq!(game.moves()[2].annotations.text, "King on f2 already.");
        assert_eq!(game.moves()[2].annotations.arrows.len(), 1);
    }

    #[test]
    fn reads_the_summary_off_the_last_line() {
        let reply = "1. e4 {Solid}
Summary {Good start. Watch your king.} [%csl Rg8]";
        assert_eq!(
            review_summary(reply).as_deref(),
            Some("Good start. Watch your king.")
        );
        assert_eq!(
            review_summary(
                "**Summary:** Calm
  game."
            )
            .as_deref(),
            Some("Calm")
        );
        assert_eq!(review_summary("1. e4 {Solid}"), None);
        assert_eq!(review_summary("Summary {}"), None);
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
