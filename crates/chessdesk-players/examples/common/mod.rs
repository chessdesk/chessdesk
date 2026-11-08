//! What the headless games share: starting the agent, moving for the player as the app's
//! tutoring would see it, and asking the agent for its move.

use std::time::{Duration, Instant};

use chessdesk_core::review::{Verdict, loss};
use chessdesk_core::{Annotations, Arrow, Game, Mark};
use chessdesk_players::acp::{AgentConfig, AgentKind, AgentOpponent, Effort};
use chessdesk_players::adapters::{Readiness, install, readiness};
use chessdesk_players::uci::{Engine, Limit};
use chessdesk_players::{Opponent, PlayerError, Tokens};
use shakmaty::{Chess, Move, Position};

/// What the app gives Stockfish for the live evaluation that grades the player's moves.
pub const GRADE_LIMIT: Limit = Limit::Milliseconds(150);

/// The agent from the next three arguments: agent, model (`""` for its default) and effort,
/// its adapter installed if need be.
pub fn agent_config(args: &mut impl Iterator<Item = String>) -> AgentConfig {
    let kind = match args.next().as_deref() {
        Some("codex") => AgentKind::Codex,
        Some("copilot") => AgentKind::Copilot,
        Some("opencode") => AgentKind::OpenCode,
        Some("pi") => AgentKind::Pi,
        _ => AgentKind::ClaudeCode,
    };
    let model = args.next().filter(|model| !model.is_empty());
    let effort = match args.next().as_deref() {
        Some("high") => Effort::High,
        Some("medium") => Effort::Medium,
        _ => Effort::Low,
    };
    let adapters = std::env::temp_dir().join("chessdesk-adapters");
    if readiness(kind, &adapters) == Readiness::NoAdapter {
        install(kind, &adapters).expect("the adapter installs");
    }
    AgentConfig {
        adapters,
        kind,
        model,
        effort,
        reply_language: None,
    }
}

/// A new game, its start evaluated as the app does before the first move.
pub fn new_game(grader: &mut Engine) -> Game {
    let mut game = Game::default();
    let (start, _) = grader
        .evaluate(&game.position(), GRADE_LIMIT)
        .expect("Stockfish evaluates");
    if let Some(wdl) = start.wdl {
        game.set_start_wdl(wdl);
    }
    game
}

/// The player's move, graded as the app grades it: the move a green arrow in `lesson`
/// points to, if one does, or else the stand-in engine's. Says whether it was taught.
pub fn players_turn(
    game: &mut Game,
    stand_in: &mut Engine,
    grader: &mut Engine,
    lesson: &[Arrow],
) -> bool {
    // The position before the player's move is the one after the agent's.
    let (eval, best) = grader
        .evaluate(&game.position(), GRADE_LIMIT)
        .expect("Stockfish evaluates");
    if let Some(agents) = game.moves_mut().last_mut() {
        agents.eval = eval;
    }
    let taught = pointed_move(&game.position(), lesson);
    let played = taught.unwrap_or_else(|| {
        stand_in
            .next_move(game, "")
            .expect("Stockfish moves")
            .played
    });
    assert!(game.play(played));
    grade_last(game, grader, best);
    let last = game.moves().last().expect("just played");
    println!(
        "{:>3}. {:<7} {:?}{}",
        game.position_at(game.moves().len() - 1).fullmoves(),
        last.san.to_string(),
        Verdict::of_move(last),
        if taught.is_some() {
            "  (as taught)"
        } else {
            ""
        }
    );
    taught.is_some()
}

/// The move the agent named for the player with `[%next]`, which comes out as a blue arrow:
/// the one a learner plays. The agent's own green arrows are ideas, not instructions.
pub fn pointed_move(position: &Chess, arrows: &[Arrow]) -> Option<Move> {
    let legal = position.legal_moves();
    arrows
        .iter()
        .rev()
        .filter(|arrow| arrow.mark == Mark::Blue)
        .find_map(|arrow| {
            legal.iter().copied().find(|played| {
                let to = played
                    .castling_side()
                    .map_or(played.to(), |side| side.king_to(position.turn()));
                played.from() == Some(arrow.from) && to == arrow.to
            })
        })
}

/// One reply of the agent's: what it said and drew, its tokens and how long it took.
pub struct Said {
    pub annotations: Annotations,
    pub tokens: Option<Tokens>,
    pub took: Duration,
}

/// The agent's move, played, with its comment kept for the PGN.
pub fn agents_turn(
    game: &mut Game,
    agent: &mut AgentOpponent,
    instructions: &str,
) -> Result<Said, PlayerError> {
    let asked = Instant::now();
    let decision = agent.next_move(game, instructions)?;
    let took = asked.elapsed();
    assert!(game.play(decision.played));
    let last = game.moves_mut().last_mut().expect("just played");
    println!(
        "     ... {:<7} {:?} {:.1}s  {}  arrows {:?}",
        last.san.to_string(),
        decision.tokens,
        took.as_secs_f32(),
        decision.annotations.text,
        decision.annotations.arrows
    );
    last.annotations = decision.annotations.clone();
    Ok(Said {
        annotations: decision.annotations,
        tokens: decision.tokens,
        took,
    })
}

/// Grades the move just played as the app does in tutoring: its loss in expected score from
/// the position before, and the engine's choice where it differs.
fn grade_last(game: &mut Game, grader: &mut Engine, best: Option<Move>) {
    let index = game.moves().len() - 1;
    let before = match index.checked_sub(1) {
        Some(previous) => game.moves()[previous].eval.wdl,
        None => game.start_wdl(),
    };
    let (eval, _) = grader
        .evaluate(&game.position(), GRADE_LIMIT)
        .expect("Stockfish evaluates");
    let mover = game.position_at(index).turn();
    let last = &mut game.moves_mut()[index];
    last.eval = eval;
    if let (Some(before), Some(after)) = (before, eval.wdl) {
        let verdict = Verdict::of_loss(loss(before, after, mover), best == Some(last.played));
        last.nags.extend(verdict.nag());
        last.better = best.filter(|best| *best != last.played);
    }
}

pub fn add(total: &mut Tokens, more: Option<Tokens>) {
    if let Some(more) = more {
        *total += more;
    }
}

/// The Stockfish program, from `STOCKFISH`.
pub fn stockfish() -> String {
    std::env::var("STOCKFISH").expect("STOCKFISH names the Stockfish program")
}
