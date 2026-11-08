//! Reviewing a finished game: an engine evaluates every position, then an agent, if
//! asked, comments on the moments that mattered in a single call.

use chessdesk_core::review::{glyph, losses};
use chessdesk_core::{Annotations, Arrow, Eval, Game, Highlight, Mark, Wdl};
use shakmaty::{Chess, Color, KnownOutcome, Move, Outcome, Position};

use crate::PlayerError;
use crate::uci::{Engine, Limit};

/// How long the engine spends on each position of a review.
pub const REVIEW_LIMIT: Limit = Limit::Milliseconds(300);

/// What a finished position is worth: no engine needed.
fn final_eval(position: &Chess) -> Option<Eval> {
    let Outcome::Known(outcome) = position.outcome() else {
        return None;
    };
    let wdl = match outcome {
        KnownOutcome::Decisive {
            winner: Color::White,
        } => (1000, 0, 0),
        KnownOutcome::Decisive {
            winner: Color::Black,
        } => (0, 0, 1000),
        KnownOutcome::Draw => (0, 1000, 0),
    };
    Some(Eval {
        score: None,
        wdl: Some(Wdl {
            white: wdl.0,
            draw: wdl.1,
            black: wdl.2,
        }),
    })
}

/// Fills in every move's evaluation, calling `progress` with moves done. It then grades
/// the moves and marks what the engine would have played instead of each mistake.
pub fn evaluate_game(
    engine: &mut Engine,
    game: &mut Game,
    mut progress: impl FnMut(usize),
) -> Result<(), PlayerError> {
    let mut best_before = Vec::with_capacity(game.moves().len());
    let (start, mut best) = engine.evaluate(game.start(), REVIEW_LIMIT)?;
    if let Some(wdl) = start.wdl {
        game.set_start_wdl(wdl);
    }
    for index in 0..game.moves().len() {
        best_before.push(best);
        let position = game.position_at(index + 1);
        let (eval, next_best) = match final_eval(&position) {
            Some(eval) => (eval, None),
            None => engine.evaluate(&position, REVIEW_LIMIT)?,
        };
        game.moves_mut()[index].eval = eval;
        best = next_best;
        progress(index + 1);
    }
    mark_mistakes(game, &best_before);
    for (played, best) in game.moves_mut().iter_mut().zip(&best_before) {
        played.better = best.filter(|best| *best != played.played);
    }
    Ok(())
}

/// Grades each move, and on a mistake marks the move and the better one.
fn mark_mistakes(game: &mut Game, best_before: &[Option<Move>]) {
    for (index, loss) in losses(game) {
        let Some(nag) = glyph(loss) else { continue };
        let played = &mut game.moves_mut()[index];
        if !played.nags.contains(&nag) {
            played.nags.push(nag);
        }
        played
            .annotations
            .merge(mistake_marks(played.played, best_before[index]));
    }
}

fn mistake_marks(played: Move, best: Option<Move>) -> Annotations {
    let mut marks = Annotations {
        highlights: vec![Highlight {
            square: played.to(),
            mark: Mark::Red,
        }],
        ..Annotations::default()
    };
    if let Some(best) = best.filter(|best| *best != played)
        && let Some(from) = best.from()
    {
        marks.arrows.push(Arrow {
            from,
            to: best.to(),
            mark: Mark::Green,
        });
    }
    marks
}

#[cfg(test)]
mod tests {
    use shakmaty::Square;

    use super::*;
    use crate::reply::parse;

    #[test]
    fn checkmate_needs_no_engine() {
        let mut game = Game::default();
        for word in ["f3", "e5", "g4", "Qh4#"] {
            let (played, _) = parse(word, &game.position()).unwrap();
            assert!(game.play(played));
        }
        let eval = final_eval(&game.position()).unwrap();
        assert_eq!(eval.wdl.unwrap().black, 1000);
        assert_eq!(final_eval(&Chess::default()), None);
    }

    #[test]
    fn a_mistake_gets_a_glyph_a_mark_and_the_better_move() {
        let mut game = Game::default();
        let (played, _) = parse("a3", &game.position()).unwrap();
        let (better, _) = parse("e4", &game.position()).unwrap();
        assert!(game.play(played));
        game.moves_mut()[0].eval.wdl = Some(Wdl {
            white: 0,
            draw: 200,
            black: 800,
        });
        mark_mistakes(&mut game, &[Some(better)]);
        let marked = &game.moves()[0];
        assert_eq!(marked.nags, [4]);
        assert_eq!(marked.annotations.highlights[0].square, Square::A3);
        assert_eq!(marked.annotations.arrows[0].to, Square::E4);
    }
}
