//! Everything agents are told. Each word here is paid for on every game, and the turn
//! texts on every move, so they stay short. Measured sizes are in `docs/design.md`.

use std::fmt::Write as _;

use chessdesk_core::review::{SUMMARY_TAG, Verdict};
use chessdesk_core::{Game, pgn};

use crate::illegal;
use shakmaty::san::San;
use shakmaty::{Chess, Color, Position, Role};

const PLAY: &str = "You play chess. Each turn gives the opponent's move, where every piece \
                    stands, and the FEN. Reply with only your move in SAN.";
const ANNOTATE: &str = " You are also a friendly club player coaching them across the board. \
                        After your move add a {comment} in your own voice: one or two casual \
                        sentences, the way you'd talk over the board. React honestly to their \
                        last move first, then say what you're up to. A turn may say what \
                        Stockfish thinks of their move: go by that, not your own read, but \
                        say it as your own and don't name Stockfish. After a mistake, say \
                        what it missed. Use contractions, have \
                        opinions, skip textbook phrases and vary how you start. Examples: \
                        Nc6 {Nice, no time wasted there. My knight's coming out before that \
                        e-pawn gets lonely.} or Bxf7 {Ooh, I'd have kept that bishop home. \
                        Thanks for the pawn!} Hint at ideas, don't hand them their next move, \
                        unless they ask you to teach them a line, a trap or a mate, or to tell \
                        them what to play. A lesson is theirs to play, not yours against them: \
                        a mate they want to learn is one they give, an opening one they play. \
                        Each turn name only their next move and add [%next SAN] for it, not a \
                        [%cal] arrow: it draws it for them as the move to play. As their \
                        opponent play the replies that let the \
                        lesson work, saying what a sharper opponent would do. Add PGN markup if it \
                        helps: [%cal Gf3e5] for arrows, [%csl Rd5] for squares (G, R, Y, B). \
                        Markup goes after the comment, never in its sentences, which name \
                        moves in words: Nc6 {Now Nf3, so your king can castle.} [%next Nf3]";
const REVIEW: &str = "You're a friendly coach going over a game with the player, who is You \
                      in the PGN. For each listed move write one line: the move as in the \
                      game, a {comment}, then PGN markup: always at least one [%cal Gf3e5] \
                      arrow or [%csl Rd5] square (G, R, Y, B) showing what you mean, such as \
                      the better move or the piece at risk. Talk to them the way you would \
                      at the board afterwards: casual, honest, a sentence or two, no textbook \
                      phrases. Example: 14... Nf6 {This is where it slipped. See the fork on \
                      d5? Happens to everyone.} [%cal Gd5f6] [%csl Rf6]. Cover every listed \
                      move. Then end with one line: Summary {two or three sentences on how \
                      the game went for them and the one thing to work on}. No other text.";

const HINT: &str = "You're a friendly chess coach. The player is stuck and asks for a hint: \
                    the best move for them in the position given. Reply with only that move \
                    in SAN, then a {comment} of one or two casual sentences on why it's good.";

/// The system prompt for a game. It replaces the agent's own, which is what keeps a
/// session this small: Claude's default costs about 28,000 tokens.
pub fn system(annotate: bool, language: Option<&str>) -> String {
    let mut prompt = PLAY.to_owned();
    if annotate {
        prompt.push_str(ANNOTATE);
        prompt.push_str(&comments_in(language));
    }
    prompt
}

pub fn hint_system(language: Option<&str>) -> String {
    format!("{HINT}{}", comments_in(language))
}

/// Asks for a hint for whoever is to move in `position`.
pub fn hint(position: &Chess) -> String {
    let side = if position.turn().is_white() {
        "White"
    } else {
        "Black"
    };
    let fen = shakmaty::fen::Fen::from_position(position, shakmaty::EnPassantMode::Legal);
    format!("I'm {side}.\nFEN {fen}")
}

pub fn review_system(language: Option<&str>) -> String {
    format!("{REVIEW}{}", comments_in(language))
}

/// Keeps comments in the language of the interface. Moves stay in English SAN, the one
/// notation every part of this reads.
fn comments_in(language: Option<&str>) -> String {
    language.map_or_else(String::new, |language| {
        format!(" Write the comments in {language}.")
    })
}

/// What the agent is told for its first turn of a session. Sent in full so that a new
/// session can also pick up a game under way.
pub fn opening(game: &Game, side: Color) -> String {
    let side = if side.is_white() { "White" } else { "Black" };
    let mut text = format!("You are {side}.");
    if !game.moves().is_empty() {
        let _ = write!(text, "\nMoves: {}", moves_text(game));
    }
    let _ = write!(text, "\n{}", board(game));
    text
}

/// Where every piece stands, then the FEN. Read from the FEN alone, models lost track of
/// pieces: they missed a rook on an open file, guarded squares and their own hanging knight.
fn board(game: &Game) -> String {
    let position = game.position();
    let side = |color: Color| {
        let board = position.board();
        let mut pieces = Vec::new();
        let mut pawns = Vec::new();
        for role in [
            Role::King,
            Role::Queen,
            Role::Rook,
            Role::Bishop,
            Role::Knight,
            Role::Pawn,
        ] {
            for square in board.by_piece(role.of(color)) {
                if role == Role::Pawn {
                    pawns.push(square.to_string());
                } else {
                    pieces.push(format!("{}{square}", role.upper_char()));
                }
            }
        }
        let name = if color.is_white() { "White" } else { "Black" };
        if pawns.is_empty() {
            format!("{name}: {}", pieces.join(" "))
        } else {
            format!("{name}: {}, pawns {}", pieces.join(" "), pawns.join(" "))
        }
    };
    format!(
        "{}\n{}\nFEN {}",
        side(Color::White),
        side(Color::Black),
        game.fen()
    )
}

/// What the agent is told before the player's first move, so that it can talk them through
/// what's to come. It also opens the session, as [`opening`] would.
pub fn introduction(game: &Game, side: Color) -> String {
    let side = if side.is_white() { "White" } else { "Black" };
    format!(
        "You are {side}. The player moves first. Before they do, reply with only a {{comment}} \
         for them and markup if it helps, no move of your own. If they asked for a lesson, \
         name their first move and add [%next SAN].\n{}",
        board(game)
    )
}

/// What the agent is told on later turns: the reply to its move, and the FEN, which
/// keeps it right after takebacks and costs less than a wrong move. A coaching agent also
/// hears Stockfish's verdict on that move, once it has one: left to its own read, a model
/// praised blunders and warned of threats that weren't there.
pub fn turn(game: &Game, annotate: bool) -> String {
    let Some(last) = game.moves().last() else {
        return board(game);
    };
    let verdict = annotate
        .then(|| verdict(game))
        .flatten()
        .map_or(String::new(), |verdict| format!("\n{verdict}"));
    format!("{}{verdict}\n{}", last.san, board(game))
}

/// Stockfish's verdict on the last move, and the better move where it lost something.
fn verdict(game: &Game) -> Option<String> {
    let last = game.moves().last()?;
    let (said, costly) = match Verdict::of_move(last)? {
        Verdict::Perfect => ("the best move", false),
        Verdict::Good => ("a good move", false),
        Verdict::Inaccuracy => ("an inaccuracy", true),
        Verdict::Mistake => ("a mistake", true),
        Verdict::Blunder => ("a blunder", true),
    };
    let before = game.position_at(game.moves().len() - 1);
    let better = last
        .better
        .filter(|_| costly)
        .map_or(String::new(), |better| {
            format!(", better was {}", San::from_move(&before, better))
        });
    Some(format!("Stockfish: {said}{better}."))
}

/// Puts the player's instructions in front of a turn when they've changed, so that they
/// cost tokens once. `told` is what the session has heard so far.
pub fn with_instructions(text: String, instructions: &str, told: &str) -> String {
    if instructions == told {
        text
    } else if instructions.is_empty() {
        format!("Forget the player's earlier instructions.\n{text}")
    } else {
        format!("The player asks you to play like this: {instructions}\n{text}")
    }
}

/// Sent after a reply without a legal move. Only now is the list worth its tokens. The
/// player never sees the failed reply, which models otherwise apologised for in the next
/// comment, or thanked them for the list. If the reply named a move, it hears why that one
/// can't be played: it came from a misread board, which the reason puts right.
pub fn retry(position: &Chess, attempted: Option<San>) -> String {
    let moves: Vec<_> = position
        .legal_moves()
        .iter()
        .map(|played| San::from_move(position, *played).to_string())
        .collect();
    let reason = attempted
        .and_then(|san| Some(format!(" {san}: {}.", illegal::why(san, position)?)))
        .unwrap_or_default();
    format!("{ILLEGAL}{reason} {AGAIN} Legal moves: {}", moves.join(" "))
}

const ILLEGAL: &str = "Illegal.";
/// Told why, models wrote about the try in the next comment ("Wrong pawn file, my mistake"),
/// which the player never saw: hence the plain ban.
const AGAIN: &str = "The player never saw that move. Play another, and leave the try out \
                     of your comment: don't mention it.";

fn moves_text(game: &Game) -> String {
    game.moves()
        .iter()
        .map(|played| played.san.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Which moves are the player's: those of the side named You, or all in someone else's
/// game.
fn player_moves(game: &Game) -> Vec<usize> {
    let side = match (game.tag("White"), game.tag("Black")) {
        (Some("You"), _) => Some(Color::White),
        (_, Some("You")) => Some(Color::Black),
        _ => None,
    };
    let first = game.start().turn();
    (0..game.moves().len())
        .filter(|index| {
            let mover = if index % 2 == 0 { first } else { first.other() };
            side.is_none_or(|side| side == mover)
        })
        .collect()
}

/// The whole review in one message: the game as PGN, evaluations included if there are any,
/// and the moves to comment on. The player's notes are their own and stay out of it, and a
/// summary from an earlier review would only be written again.
pub fn review(game: &Game) -> String {
    let mut shared = game.clone();
    shared.notes.clear();
    shared.tags.retain(|(name, _)| name != SUMMARY_TAG);
    let mut text = pgn::write(&shared);
    text.push_str("\nComment on:");
    for index in player_moves(game) {
        let _ = write!(text, " {}", numbered(game, index));
    }
    text
}

/// A move with its number: `14... Nf6`.
fn numbered(game: &Game, index: usize) -> String {
    let position = game.position_at(index);
    let dots = if position.turn().is_white() {
        "."
    } else {
        "..."
    };
    format!("{}{dots} {}", position.fullmoves(), game.moves()[index].san)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reply::parse;

    fn after(moves: &[&str]) -> Game {
        let mut game = Game::default();
        for word in moves {
            let (played, _) = parse(word, &game.position()).unwrap();
            assert!(game.play(played));
        }
        game
    }

    #[test]
    fn comments_follow_the_interface_language() {
        assert!(system(true, Some("Deutsch")).ends_with(" Write the comments in Deutsch."));
        assert!(review_system(Some("Deutsch")).ends_with(" Write the comments in Deutsch."));
        // Without comments or in English, it costs nothing.
        assert_eq!(system(false, Some("Deutsch")), PLAY);
        assert_eq!(review_system(None), REVIEW);
    }

    #[test]
    fn an_introduction_names_the_side_and_asks_for_no_move() {
        let text = introduction(&Game::default(), Color::Black);
        assert!(text.starts_with("You are Black. The player moves first."));
        assert!(text.contains("{comment}"));
        assert!(text.ends_with(&format!("FEN {}", Game::default().fen())));
    }

    #[test]
    fn a_hint_is_asked_for_the_side_to_move() {
        let position = after(&["e4"]).position();
        assert_eq!(
            hint(&position),
            "I'm Black.\nFEN rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
        );
        assert!(hint_system(Some("Deutsch")).ends_with(" Write the comments in Deutsch."));
    }

    #[test]
    fn a_turn_is_the_last_move_and_the_position() {
        assert_eq!(
            turn(&after(&["e4", "e5", "Nf3"]), true),
            "Nf3\n\
             White: Ke1 Qd1 Ra1 Rh1 Bc1 Bf1 Nb1 Nf3, pawns a2 b2 c2 d2 f2 g2 h2 e4\n\
             Black: Ke8 Qd8 Ra8 Rh8 Bc8 Bf8 Nb8 Ng8, pawns e5 a7 b7 c7 d7 f7 g7 h7\n\
             FEN rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2"
        );
    }

    #[test]
    fn a_side_without_pawns_is_listed_without_them() {
        let game = Game::from_position(Game::parse_fen("4k3/8/8/8/8/8/8/R3K3 w - - 0 1").unwrap());
        assert!(board(&game).starts_with("White: Ke1 Ra1\nBlack: Ke8\nFEN "));
    }

    #[test]
    fn a_lesson_is_the_players_to_play() {
        let prompt = system(true, None);
        assert!(prompt.contains("A lesson is theirs to play"));
        assert!(introduction(&Game::default(), Color::Black).contains("[%next SAN]"));
    }

    /// The last move graded by Stockfish: `nag` as the app gives it, `better` where it would
    /// have played something else.
    fn graded(moves: &[&str], nag: Option<u8>, better: Option<&str>) -> Game {
        let mut game = after(moves);
        let before = game.position_at(moves.len() - 1);
        let last = game.moves_mut().last_mut().unwrap();
        last.eval.wdl = Some(chessdesk_core::Wdl {
            white: 0,
            draw: 1000,
            black: 0,
        });
        last.nags.extend(nag);
        last.better = better.map(|word| parse(word, &before).unwrap().0);
        game
    }

    #[test]
    fn a_coaching_turn_carries_the_verdict_on_the_players_move() {
        let blunder = graded(&["e4", "e5", "Qh5", "Nc6", "Qxf7+"], Some(4), Some("Bc4"));
        let text = turn(&blunder, true);
        assert!(text.starts_with("Qxf7+\nStockfish: a blunder, better was Bc4.\nWhite: "));
        // Without coaching, the agent only plays: the verdict would be tokens for nothing.
        assert!(!turn(&blunder, false).contains("Stockfish"));

        let good = graded(&["e4", "e5", "Nf3"], None, Some("d4"));
        assert!(turn(&good, true).starts_with("Nf3\nStockfish: a good move.\nWhite: "));
    }

    #[test]
    fn a_move_not_yet_graded_goes_without_a_verdict() {
        assert!(!turn(&after(&["e4"]), true).contains("Stockfish"));
    }

    #[test]
    fn the_opening_message_catches_up_a_new_session() {
        let text = opening(&after(&["e4"]), Color::Black);
        assert!(text.starts_with("You are Black.\nMoves: e4\nWhite: "));
    }

    #[test]
    fn tutoring_is_opt_in_and_gives_feedback() {
        assert!(!system(false, None).contains("%cal"));
        assert!(system(true, None).contains("%cal"));
        assert!(system(true, None).contains("React honestly to their last move"));
    }

    #[test]
    fn instructions_are_told_once_and_can_be_withdrawn() {
        let turn = || "e4".to_owned();
        assert_eq!(with_instructions(turn(), "", ""), "e4");
        assert_eq!(with_instructions(turn(), "Be bold", "Be bold"), "e4");
        assert_eq!(
            with_instructions(turn(), "Be bold", ""),
            "The player asks you to play like this: Be bold\ne4"
        );
        assert!(with_instructions(turn(), "", "Be bold").starts_with("Forget"));
    }

    #[test]
    fn retry_lists_every_legal_move() {
        let text = retry(&Chess::default(), None);
        let (said, moves) = text.split_once(" Legal moves: ").unwrap();
        assert_eq!(said, format!("{ILLEGAL} {AGAIN}"));
        assert_eq!(moves.split_whitespace().count(), 20);
        assert!(moves.contains("Nf3"));
    }

    #[test]
    fn retry_says_why_the_move_was_illegal() {
        let e5 = "e5".parse().unwrap();
        let text = retry(&Chess::default(), Some(e5));
        assert!(text.starts_with(&format!("{ILLEGAL} e5: no pawn of yours reaches e5 (")));
        assert!(text.contains(&format!(". {AGAIN} Legal moves: ")));
    }

    #[test]
    fn review_covers_every_move_of_the_player() {
        let mut game = after(&["e4", "e5", "Nf3", "Nc6"]);
        game.set_tag("White", "Maia 1500");
        game.set_tag("Black", "You");
        assert!(review(&game).ends_with("Comment on: 1... e5 2... Nc6"));
    }

    #[test]
    fn review_of_someone_elses_game_covers_every_move() {
        assert!(review(&after(&["e4", "e5"])).ends_with("Comment on: 1. e4 1... e5"));
    }

    #[test]
    fn review_leaves_out_the_players_notes() {
        let mut game = after(&["e4", "e5"]);
        game.notes = "My secret plan".to_owned();
        assert!(!review(&game).contains("secret"));
    }

    #[test]
    fn review_leaves_out_the_last_summary() {
        let mut game = after(&["e4", "e5"]);
        game.set_tag(SUMMARY_TAG, "Well played");
        assert!(!review(&game).contains("Well played"));
    }
}
