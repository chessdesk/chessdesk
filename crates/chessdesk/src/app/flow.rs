//! What happens to games: starting them, taking turns, ending them, reviewing them.
//! Everything takes a session's id, since answers may arrive for a game not on show.

use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chessdesk_core::review::{Verdict, loss};
use chessdesk_core::sound::Sound;
use chessdesk_core::{Annotations, Arrow, Eval, Game, Mark};
use chessdesk_players::acp::{AgentConfig, AgentKind, Effort, hint, review_game};
use chessdesk_players::adapters::{self, Readiness};
use chessdesk_players::analysis::evaluate_game;
use chessdesk_players::uci::{Engine, EngineConfig, Limit};
use chessdesk_players::{Decision, Intro, Opponent, PlayerError, Tokens};
use gpui::{AppContext, Context, PathPromptOptions};
use shakmaty::san::SanPlus;
use shakmaty::{Color, KnownOutcome, Move, Outcome, Position, Role, Square};

use super::Chessdesk;
use super::clock::Clock;
use super::session::{HintNote, INTRO_TAG, Live, Pause, Promotion, Session, SessionId, Suggestion};
use crate::audio;
use crate::board::{SLIDE_TIME, Slide, slide_paths, slide_pieces};
use crate::i18n::{self, tr, tr_with};
use crate::library::{self, Meta, Standing};
use crate::settings::Settings;
use crate::setup::{OpponentKind, Setup, effort_label};
use crate::text_area::{Changed, TextArea};

/// How often running clocks are looked at and redrawn.
const CLOCK_TICK: Duration = Duration::from_millis(200);
/// How long Stockfish looks at each position for the live likelihood.
const LIVE_LIMIT: Limit = Limit::Milliseconds(150);
/// How long Stockfish looks for a hint or the optimal move.
const SUGGESTION_LIMIT: Limit = Limit::Milliseconds(500);
/// The tag that keeps a game's token count in its PGN.
const TOKENS_TAG: &str = "ChessdeskTokens";
/// How little time left is worth a warning.
const LOW_TIME: Duration = Duration::from_secs(10);

/// What a running analysis reports.
enum AnalysisUpdate {
    Progress(usize),
    Done(Result<Box<Game>, PlayerError>),
}

/// What the end of a game sounds like for the player, if they were in it.
fn ending_sound(game: &Game) -> Option<Sound> {
    match library::standing(game)? {
        Standing::Won => Some(Sound::Win),
        Standing::Lost => Some(Sound::Lose),
        Standing::Draw => Some(Sound::Draw),
        Standing::Others(_) => None,
    }
}

/// Stops turns waiting for Stockfish's verdicts, which won't come.
fn stockfish_failed(session: &mut Session) {
    if let Some(live) = &mut session.live {
        live.stockfish_failed = true;
    }
}

/// Gives the player's move that led to `ply` its verdict, in tutoring.
fn grade(session: &mut Session, ply: usize) {
    let Some(live) = session
        .live
        .as_ref()
        .filter(|live| live.setup.grades_moves())
    else {
        return;
    };
    let human = live.player;
    let index = ply - 1;
    let before = match index.checked_sub(1) {
        Some(previous) => session.game.moves()[previous].eval.wdl,
        None => session.game.start_wdl(),
    };
    let mover = session.game.position_at(index).turn();
    let engine_choice = session.best_moves.get(&index).copied();
    let played = &mut session.game.moves_mut()[index];
    let (Some(before), Some(after)) = (before, played.eval.wdl) else {
        return;
    };
    if mover == human {
        let verdict = Verdict::of_loss(
            loss(before, after, mover),
            engine_choice == Some(played.played),
        );
        played.nags.extend(verdict.nag());
        played.better = engine_choice.filter(|best| *best != played.played);
    }
}

/// The clock of a game picked up again: what each side had left at its last move.
fn resumed_clock(game: &Game, setup: &Setup) -> Option<Clock> {
    let (time, increment) = setup.clock()?;
    let mut clock = Clock::new(time, increment);
    let mut mover = game.start().turn();
    for played in game.moves() {
        if let Some(left) = played.clock {
            clock.set_left(mover, left);
        }
        mover = mover.other();
    }
    clock.start(mover, Instant::now());
    Some(clock)
}

/// An agent asked for help rather than to play: reviews and hints. Claude gets Sonnet, which
/// explains as well for less; the others their own default.
fn helper_config(kind: AgentKind) -> AgentConfig {
    AgentConfig {
        kind,
        model: (kind == AgentKind::ClaudeCode).then(|| "sonnet".to_owned()),
        effort: Effort::Medium,
        adapters: Settings::adapters_dir().unwrap_or_default(),
        reply_language: i18n::reply_language(),
    }
}

impl Chessdesk {
    fn next_id(&mut self) -> SessionId {
        self.next_session += 1;
        self.next_session
    }

    // Starting and ending games.

    pub(super) fn start_game(&mut self, setup: Setup, cx: &mut Context<Self>) {
        if let Err(error) = setup.save() {
            self.report(tr_with!("error-remember-choices", error = error), cx);
        }
        let Some(position) = setup.start_position() else {
            self.report(tr("invalid-position"), cx);
            return;
        };
        let settings = Settings::get(cx);
        let player = setup.player_color(jiff::Timestamp::now().subsec_nanosecond() % 2 == 0);
        let opponent_name = setup.opponent_name();

        let mut game = Game::from_position(position);
        let (white, black) = if player.is_white() {
            ("You", opponent_name.as_str())
        } else {
            (opponent_name.as_str(), "You")
        };
        game.set_tag("White", white);
        game.set_tag("Black", black);
        game.set_tag("Date", jiff::Zoned::now().strftime("%Y.%m.%d").to_string());

        let mut clock = setup
            .clock()
            .map(|(time, increment)| Clock::new(time, increment));
        if let Some(clock) = &mut clock {
            clock.start(game.position().turn(), Instant::now());
        }
        let id = self.next_id();
        let mut session = Session::new(id, game);
        session.orientation = player;
        session.path = library::new_path();
        if let Some(path) = &session.path {
            let meta = Meta {
                setup: setup.clone(),
                player: player.is_white(),
            };
            if let Err(error) = library::save_meta(path, &meta) {
                self.report(tr_with!("error-save-setup", error = error), cx);
            }
        }
        session.live = Some(Live {
            plan: setup.plan(&settings, player.other()),
            player,
            opponent: None,
            plan_version: 0,
            thinking: false,
            paused: None,
            clock,
            low_time_warned: false,
            evaluator: None,
            evaluating: false,
            instructions: None,
            introduced: false,
            stockfish_failed: false,
            setup,
        });
        // The empty board from before any game has served its purpose.
        self.sessions.retain(|session| session.path.is_some());
        self.sessions.push(session);
        self.active = id;
        self.settings_page = None;
        self.attach_instructions(id, cx);
        self.attach_notes(id, cx);
        self.run_clocks(cx);
        self.after_change(id, cx);
    }

    /// Gives a game its field for the player's notes.
    fn attach_notes(&mut self, id: SessionId, cx: &mut Context<Self>) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        let text = session.game.notes.clone();
        let area = cx.new(|cx| TextArea::new(text, tr("notes-placeholder"), cx));
        let changes = cx.subscribe(&area, move |this, area, _: &Changed, cx| {
            let text = area.read(cx).text().to_owned();
            this.on_notes(id, text, cx);
        });
        session.notes = Some((area, changes));
    }

    /// Keeps the notes in the game's file, as they're typed.
    fn on_notes(&mut self, id: SessionId, text: String, cx: &mut Context<Self>) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        session.game.notes = text;
        self.autosave(id, cx);
    }

    /// Gives a game against an agent its field for how to play.
    fn attach_instructions(&mut self, id: SessionId, cx: &mut Context<Self>) {
        let Some(live) = self
            .session_mut(id)
            .and_then(|session| session.live.as_mut())
        else {
            return;
        };
        if live.setup.opponent.agent().is_none() {
            return;
        }
        let text = live.setup.instructions.clone();
        let area = cx.new(|cx| TextArea::new(text, tr("instructions-example"), cx));
        let changes = cx.subscribe(&area, move |this, area, _: &Changed, cx| {
            let text = area.read(cx).text().to_owned();
            this.on_instructions(id, text, cx);
        });
        live.instructions = Some((area, changes));
    }

    /// Keeps new instructions for the agent's next turn, and for picking the game up later.
    fn on_instructions(&mut self, id: SessionId, text: String, cx: &mut Context<Self>) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        let path = session.path.clone();
        let Some(live) = &mut session.live else {
            return;
        };
        live.setup.instructions = text;
        let meta = Meta {
            setup: live.setup.clone(),
            player: live.player.is_white(),
        };
        if let Some(path) = path
            && let Err(error) = library::save_meta(&path, &meta)
        {
            self.report(tr_with!("error-save-instructions", error = error), cx);
        }
    }

    /// Ends a game off the board.
    fn finish(&mut self, id: SessionId, outcome: KnownOutcome, cx: &mut Context<Self>) {
        if let Some(session) = self.session_mut(id) {
            session.game.adjudicate(outcome);
            session.epoch += 1;
        }
        self.after_change(id, cx);
    }

    /// Saves the game and hands the turn on: to the opponent, or to nobody at the end.
    pub(super) fn after_change(&mut self, id: SessionId, cx: &mut Context<Self>) {
        self.autosave(id, cx);
        let on_show = id == self.active;
        let Some(session) = self.session_mut(id) else {
            return;
        };
        session.selected = None;
        if session.game.outcome().is_known() {
            if on_show
                && session.live.is_some()
                && let Some(sound) = ending_sound(&session.game)
            {
                audio::play(sound, cx);
            }
            session.stop_clock();
            session.live = None;
        } else {
            self.evaluate_live(id, cx);
            self.ask_opponent(id, cx);
        }
        cx.notify();
    }

    fn autosave(&mut self, id: SessionId, cx: &mut Context<Self>) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        if let Some(tokens) = session.tokens {
            session.game.set_tag(TOKENS_TAG, tokens.total.to_string());
        }
        let Some(path) = session.path.clone() else {
            return;
        };
        let game = session.game.clone();
        if let Err(error) = library::save(&game, &path) {
            self.report(tr_with!("error-save-game", error = error), cx);
        }
        self.update_history(library::entry(path, &game));
    }

    // The opponent's turn.

    fn ask_opponent(&mut self, id: SessionId, cx: &mut Context<Self>) {
        let stockfish = OpponentKind::Stockfish
            .missing(&Settings::get(cx))
            .is_none();
        let Some(session) = self.session_mut(id) else {
            return;
        };
        // Asked again once Stockfish has given it, from `on_live_eval`.
        if session.awaits_verdict(stockfish) {
            return;
        }
        let position = session.game.position();
        let tokens = session.tokens;
        let game = session.game.clone();
        let epoch = session.epoch;
        let Some(live) = &mut session.live else {
            return;
        };
        if live.thinking || live.paused.is_some() {
            return;
        }
        if position.turn() == live.player {
            // Before the player's first move, a coaching agent gets its say: an opponent
            // with Black would otherwise be silent until the player had moved.
            let introduces = !live.introduced
                && game.moves().is_empty()
                && game.tag(INTRO_TAG).is_none()
                && live.setup.opponent.agent().is_some()
                && live.setup.tutoring();
            if introduces {
                self.ask_intro(id, cx);
            }
            return;
        }
        if let (Some(budget), Some(tokens)) = (live.setup.token_budget, tokens)
            && tokens.total >= budget
        {
            live.paused = Some(Pause::Notice(tr_with!("budget-used", budget = budget)));
            cx.notify();
            return;
        }
        live.thinking = true;
        let mut opponent = live.opponent.take();
        let plan = live.plan.clone();
        let version = live.plan_version;
        let instructions = live.setup.instructions.clone();
        let delay = Settings::get(cx).opponent_delay;
        cx.spawn(async move |this, cx| {
            let asked = Instant::now();
            let (opponent, decision) = cx
                .background_spawn(async move {
                    let started = match opponent.take() {
                        Some(opponent) => Ok(opponent),
                        None => plan.start(),
                    };
                    match started {
                        Ok(mut opponent) => {
                            let decision = opponent.next_move(&game, &instructions);
                            (Some(opponent), decision)
                        }
                        Err(error) => (None, Err(error)),
                    }
                })
                .await;
            // A move that comes at once is easy to miss, and feels like no thought at all.
            if let Some(rest) = delay.checked_sub(asked.elapsed()) {
                cx.background_executor().timer(rest).await;
            }
            this.update(cx, |this, cx| {
                this.on_decision(id, (epoch, version), opponent, decision, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Has the opponent speak before the player's first move. The player waits for it as
    /// for a move: it's about the position they're to move in.
    fn ask_intro(&mut self, id: SessionId, cx: &mut Context<Self>) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        let game = session.game.clone();
        let Some(live) = &mut session.live else {
            return;
        };
        live.introduced = true;
        live.thinking = true;
        let mut opponent = live.opponent.take();
        let plan = live.plan.clone();
        let version = live.plan_version;
        let instructions = live.setup.instructions.clone();
        cx.spawn(async move |this, cx| {
            let (opponent, intro) = cx
                .background_spawn(async move {
                    let started = match opponent.take() {
                        Some(opponent) => Ok(opponent),
                        None => plan.start(),
                    };
                    match started {
                        Ok(mut opponent) => {
                            let intro = opponent.introduce(&game, &instructions);
                            (Some(opponent), intro)
                        }
                        Err(error) => (None, Err(error)),
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                this.on_intro(id, version, opponent, intro, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Keeps what the opponent said before the first move, in the game so that it's saved.
    /// On a failure the game goes on without it: it's no reason to stop.
    fn on_intro(
        &mut self,
        id: SessionId,
        version: u64,
        opponent: Option<Box<dyn Opponent>>,
        intro: Result<Option<Intro>, PlayerError>,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        let Some(live) = &mut session.live else {
            return;
        };
        live.thinking = false;
        live.opponent = opponent.filter(|_| version == live.plan_version);
        match intro {
            Ok(Some(intro)) => {
                if let Some(tokens) = intro.tokens {
                    *session.tokens.get_or_insert_default() += tokens;
                }
                // A tag is one line.
                let comment = intro.annotations.to_comment();
                let comment = comment.split_whitespace().collect::<Vec<_>>().join(" ");
                session.game.set_tag(INTRO_TAG, comment);
            }
            Ok(None) => {}
            Err(error) => {
                // Still busy with what it was asked: its first move starts a new session.
                live.opponent = None;
                let name = live.setup.opponent.name();
                self.report(format!("{name}: {error}"), cx);
            }
        }
        self.after_change(id, cx);
    }

    /// What the opponent decided, asked at the game's `epoch` with the plan's `version`.
    fn on_decision(
        &mut self,
        id: SessionId,
        (epoch, version): (u64, u64),
        opponent: Option<Box<dyn Opponent>>,
        decision: Result<Decision, PlayerError>,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        let Some(live) = &mut session.live else {
            return;
        };
        live.thinking = false;
        // One started on an older plan, such as another effort, makes way for a new one.
        live.opponent = opponent.filter(|_| version == live.plan_version);
        if epoch != session.epoch {
            // The position changed under it. Ask about the one on the board now.
            self.ask_opponent(id, cx);
            return;
        }
        match decision {
            Ok(decision) => {
                if let Some(tokens) = decision.tokens {
                    *session.tokens.get_or_insert_default() += tokens;
                }
                self.play(id, decision.played, decision.annotations, true, cx);
            }
            Err(PlayerError::TimedOut(limit)) => {
                // Still busy with the move it was asked for: the next ask starts afresh.
                live.opponent = None;
                let reason = tr_with!(
                    "error-timed-out",
                    agent = live.setup.opponent.name(),
                    minutes = limit.as_secs() / 60,
                    effort = effort_label(live.setup.effort).to_string()
                );
                live.paused = Some(Pause::Failed);
                self.report(reason, cx);
            }
            Err(error) => {
                // Such as the provider turning the agent away: an error like any other.
                live.paused = Some(Pause::Failed);
                let agent = live.setup.opponent.name();
                self.report(format!("{agent}: {error}"), cx);
            }
        }
    }

    /// Has the agent of the game on show think harder or less from its next move on. Its
    /// effort is set when a session starts, so the next move starts a new one, which is told
    /// the game so far.
    pub(super) fn set_effort(&mut self, effort: Effort, cx: &mut Context<Self>) {
        let id = self.active;
        let settings = Settings::get(cx);
        let Some(session) = self.session_mut(id) else {
            return;
        };
        let path = session.path.clone();
        let Some(live) = &mut session.live else {
            return;
        };
        if live.setup.effort == effort || live.setup.opponent.agent().is_none() {
            return;
        }
        live.setup.effort = effort;
        live.plan = live.setup.plan(&settings, live.player.other());
        live.plan_version += 1;
        live.opponent = None;
        let meta = Meta {
            setup: live.setup.clone(),
            player: live.player.is_white(),
        };
        if let Some(path) = path
            && let Err(error) = library::save_meta(&path, &meta)
        {
            self.report(tr_with!("error-save-setup", error = error), cx);
        }
        cx.notify();
    }

    /// Goes on after a pause: asks the opponent again.
    pub(super) fn resume(&mut self, cx: &mut Context<Self>) {
        let id = self.active;
        if let Some(live) = self
            .session_mut(id)
            .and_then(|session| session.live.as_mut())
        {
            live.paused = None;
            live.setup.token_budget = None;
        }
        self.ask_opponent(id, cx);
        cx.notify();
    }

    fn play(
        &mut self,
        id: SessionId,
        played: Move,
        annotations: Annotations,
        animate: bool,
        cx: &mut Context<Self>,
    ) {
        let on_show = id == self.active;
        let Some(session) = self.session_mut(id) else {
            return;
        };
        let followed = session.at_latest();
        session.suggestion = None;
        if !session.game.play(played) {
            self.report(tr("illegal-move"), cx);
            return;
        }
        if let Some(last) = session.game.moves_mut().last_mut() {
            last.annotations = annotations;
        }
        // A move that ends the game is drowned out by the ending's own sound.
        if on_show
            && !session.game.outcome().is_known()
            && let Some(last) = session.game.moves().last()
        {
            audio::play(Sound::of_move(last), cx);
        }
        if let Some(clock) = session.live.as_mut().and_then(|live| live.clock.as_mut()) {
            let now = Instant::now();
            clock.press(now);
            let mover = session.game.position().turn().other();
            if let Some(last) = session.game.moves_mut().last_mut() {
                last.clock = Some(clock.left(mover, now));
            }
        }
        if followed {
            let latest = session.game.moves().len();
            self.show_ply(id, latest, animate, cx);
        }
        self.after_change(id, cx);
    }

    /// Shows a position, gliding the pieces there if it's one move away.
    pub(super) fn show_ply(
        &mut self,
        id: SessionId,
        ply: usize,
        animate: bool,
        cx: &mut Context<Self>,
    ) {
        let enabled = Settings::get(cx).animate_moves;
        self.next_slide += 1;
        let slide_id = self.next_slide;
        let Some(session) = self.session_mut(id) else {
            return;
        };
        let ply = ply.min(session.game.moves().len());
        let forward = ply == session.ply + 1;
        let backward = ply + 1 == session.ply;
        let step = if forward { ply } else { session.ply };
        let played = step
            .checked_sub(1)
            .and_then(|at| session.game.moves().get(at))
            .map(|played| played.played);
        session.ply = ply;
        session.selected = None;
        let Some(played) = played.filter(|_| animate && enabled && (forward || backward)) else {
            session.slides.clear();
            cx.notify();
            return;
        };
        let shown = session.game.position_at(ply);
        session.slides.push(Slide {
            id: slide_id,
            pieces: slide_pieces(played, &shown, forward),
        });
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SLIDE_TIME).await;
            this.update(cx, |this, cx| {
                if let Some(session) = this.session_mut(id) {
                    session.slides.retain(|slide| slide.id != slide_id);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Looks at every running clock, as long as there is one.
    fn run_clocks(&mut self, cx: &mut Context<Self>) {
        if self.clocks_running {
            return;
        }
        self.clocks_running = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(CLOCK_TICK).await;
                let running = this.update(cx, Self::tick);
                if !matches!(running, Ok(true)) {
                    return;
                }
            }
        })
        .detach();
    }

    /// Sounds once when the player's clock on the game on show nears the end.
    fn warn_of_low_time(&mut self, now: Instant, cx: &mut Context<Self>) {
        let active = self.active;
        let Some(live) = self.session_mut(active).and_then(|s| s.live.as_mut()) else {
            return;
        };
        let Some(left) = live
            .clock
            .as_ref()
            .map(|clock| clock.left(live.player, now))
        else {
            return;
        };
        if left < LOW_TIME && !live.low_time_warned {
            live.low_time_warned = true;
            audio::play(Sound::LowTime, cx);
        }
    }

    /// Redraws clocks and calls flags. Returns whether any clock still runs.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        let now = Instant::now();
        self.warn_of_low_time(now, cx);
        let flagged: Vec<_> = self
            .sessions
            .iter()
            .filter_map(|session| {
                let clock = session.live.as_ref()?.clock.as_ref()?;
                Some((session.id, clock.flagged(now)?))
            })
            .collect();
        for (id, loser) in flagged {
            let winner = loser.other();
            if let Some(session) = self.session_mut(id) {
                session.game.set_tag("Termination", "Time forfeit");
            }
            self.finish(id, KnownOutcome::Decisive { winner }, cx);
        }
        let running = self.sessions.iter().any(|session| {
            session
                .live
                .as_ref()
                .is_some_and(|live| live.clock.is_some())
        });
        self.clocks_running = running;
        if running {
            cx.notify();
        }
        running
    }

    /// Has Stockfish look at the next position waiting, if this game wants that. One at a
    /// time and in order, however fast the moves come: a verdict needs both sides of a move.
    /// Evaluates the games that waited for Stockfish, which may just have arrived.
    pub(super) fn evaluate_waiting(&mut self, cx: &mut Context<Self>) {
        let waiting: Vec<_> = self
            .sessions
            .iter()
            .filter(|session| session.wants_evaluations())
            .map(|session| session.id)
            .collect();
        for id in waiting {
            self.evaluate_live(id, cx);
        }
    }

    fn evaluate_live(&mut self, id: SessionId, cx: &mut Context<Self>) {
        let settings = Settings::get(cx);
        if OpponentKind::Stockfish.missing(&settings).is_some() {
            return;
        }
        let Some(session) = self.session_mut(id) else {
            return;
        };
        if !session.wants_evaluations() {
            return;
        }
        let Some(ply) = session.next_to_evaluate() else {
            return;
        };
        let position = session.game.position_at(ply);
        let Some(live) = &mut session.live else {
            return;
        };
        if live.evaluating {
            return;
        }
        live.evaluating = true;
        let evaluator = live.evaluator.take();
        let config = EngineConfig::stockfish(&settings.stockfish, None);
        cx.spawn(async move |this, cx| {
            let (evaluator, eval) = cx
                .background_spawn(async move {
                    let engine = evaluator.map_or_else(|| Engine::start(&config), Ok);
                    match engine {
                        Ok(mut engine) => {
                            let eval = engine.evaluate(&position, LIVE_LIMIT);
                            (Some(engine), eval)
                        }
                        Err(error) => (None, Err(error)),
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                this.on_live_eval(id, ply, evaluator, eval, cx);
            })
            .ok();
        })
        .detach();
    }

    fn on_live_eval(
        &mut self,
        id: SessionId,
        ply: usize,
        evaluator: Option<Engine>,
        evaluated: Result<(Eval, Option<Move>), PlayerError>,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        if let Some(live) = &mut session.live {
            live.evaluating = false;
            live.evaluator = evaluator;
        }
        match evaluated {
            Ok((eval, best)) => {
                // Taken back meanwhile: this says nothing about the game as it stands.
                if ply <= session.game.moves().len() {
                    if let Some(best) = best {
                        session.best_moves.insert(ply, best);
                    }
                    if ply == 0 {
                        if let Some(wdl) = eval.wdl {
                            session.game.set_start_wdl(wdl);
                        }
                    } else {
                        session.game.moves_mut()[ply - 1].eval = eval;
                        grade(session, ply);
                    }
                }
                // Without a likelihood the position would come up again, for ever.
                if eval.wdl.is_some() {
                    self.autosave(id, cx);
                    self.evaluate_live(id, cx);
                } else {
                    stockfish_failed(session);
                }
            }
            Err(error) => {
                if let Some(session) = self.session_mut(id) {
                    stockfish_failed(session);
                }
                self.report(format!("Stockfish: {error}"), cx);
            }
        }
        // A coaching agent may have been waiting for this verdict, or for Stockfish to fail.
        let latest = self
            .session_mut(id)
            .is_some_and(|session| ply > 0 && ply == session.game.moves().len());
        if latest {
            self.ask_opponent(id, cx);
        }
        cx.notify();
    }

    // The player's actions, always on the game on show.

    pub(super) fn press_square(&mut self, square: Square, cx: &mut Context<Self>) {
        self.open_menu = None;
        let id = self.active;
        let Some(session) = self.session_mut(id) else {
            return;
        };
        if let Some(from) = session.selected.take()
            && from != square
            && self.try_move(from, square, true, cx)
        {
            return;
        }
        let Some(session) = self.session_mut(id) else {
            return;
        };
        if session.player_to_move() {
            let position = session.game.position();
            let own = position.board().color_at(square) == Some(position.turn());
            session.selected = own.then_some(square);
        }
        cx.notify();
    }

    pub(super) fn drop_piece(&mut self, from: Square, to: Square, cx: &mut Context<Self>) {
        let id = self.active;
        if let Some(session) = self.session_mut(id) {
            session.selected = None;
        }
        // The hand has already carried the piece there.
        if from != to {
            self.try_move(from, to, false, cx);
        }
        cx.notify();
    }

    /// Plays from `from` to `to` if that is a move. Returns whether it was.
    fn try_move(
        &mut self,
        from: Square,
        to: Square,
        animate: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let id = self.active;
        let Some(session) = self.session_mut(id) else {
            return false;
        };
        if !session.player_to_move() {
            return false;
        }
        match session.moves_between(from, to).as_slice() {
            [] => false,
            [only] => {
                let only = *only;
                self.play(id, only, Annotations::default(), animate, cx);
                true
            }
            _ => {
                session.promotion = Some(Promotion { from, to });
                cx.notify();
                true
            }
        }
    }

    pub(super) fn promote(&mut self, role: Role, cx: &mut Context<Self>) {
        let id = self.active;
        let Some(session) = self.session_mut(id) else {
            return;
        };
        let Some(Promotion { from, to }) = session.promotion.take() else {
            return;
        };
        let moves = session.moves_between(from, to);
        if let Some(played) = moves.iter().find(|played| played.promotion() == Some(role)) {
            self.play(id, *played, Annotations::default(), true, cx);
        }
        cx.notify();
    }

    pub(super) fn take_back(&mut self, cx: &mut Context<Self>) {
        let id = self.active;
        let Some(session) = self.session_mut(id) else {
            return;
        };
        let Some(live) = &session.live else {
            return;
        };
        if live.thinking || session.suggesting || !live.setup.assisted() {
            return;
        }
        // Back to the player's own turn.
        let count = if session.game.position().turn() == live.player {
            2
        } else {
            1
        };
        session.game.take_back(count);
        session.suggestion = None;
        let moves = session.game.moves().len();
        session.best_moves.retain(|ply, _| *ply <= moves);
        session.epoch += 1;
        session.ply = session.game.moves().len();
        self.after_change(id, cx);
    }

    pub(super) fn resign(&mut self, cx: &mut Context<Self>) {
        self.resign_session(self.active, cx);
    }

    fn resign_session(&mut self, id: SessionId, cx: &mut Context<Self>) {
        let winner = self
            .session_mut(id)
            .and_then(|session| Some(session.live.as_ref()?.player.other()));
        if let Some(winner) = winner {
            self.finish(id, KnownOutcome::Decisive { winner }, cx);
        }
    }

    /// Has Stockfish find the best move in the position on show, and draws it: a hint
    /// during a game, the optimal move in a replay. It's local, so it costs no tokens.
    pub(super) fn suggest(&mut self, cx: &mut Context<Self>) {
        let id = self.active;
        let settings = Settings::get(cx);
        let config = EngineConfig::stockfish(&settings.stockfish, None);
        if OpponentKind::Stockfish.missing(&settings).is_some() {
            return self.report(tr("needs-stockfish"), cx);
        }
        let Some(session) = self.session_mut(id) else {
            return;
        };
        if session.suggesting {
            return;
        }
        session.suggesting = true;
        let ply = session.ply;
        let position = session.game.position_at(ply);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let best = cx
                .background_spawn(async move {
                    let mut engine = Engine::start(&config)?;
                    Ok::<_, PlayerError>(engine.evaluate(&position, SUGGESTION_LIMIT)?.1)
                })
                .await;
            let best = best.map(|best| {
                best.map(|played| Decision {
                    played,
                    annotations: Annotations::default(),
                    tokens: None,
                })
            });
            this.update(cx, |this, cx| {
                this.on_suggestion(id, ply, "Stockfish", best, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Has an agent suggest a move for the position on show, and say why: a hint that costs
    /// tokens, in a session of its own.
    pub(super) fn suggest_with(&mut self, kind: AgentKind, cx: &mut Context<Self>) {
        let id = self.active;
        let adapters_dir = Settings::adapters_dir().unwrap_or_default();
        if adapters::readiness(kind, &adapters_dir) != Readiness::Ready {
            return self.report(tr_with!("agent-not-set-up", agent = kind.name()), cx);
        }
        let Some(session) = self.session_mut(id) else {
            return;
        };
        if session.suggesting {
            return;
        }
        session.suggesting = true;
        let ply = session.ply;
        let position = session.game.position_at(ply);
        let config = helper_config(kind);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let hint = cx
                .background_spawn(async move { hint(&config, &position).map(Some) })
                .await;
            this.update(cx, |this, cx| {
                this.on_suggestion(id, ply, kind.name(), hint, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Draws the move `by` came up with, and keeps what it said about it.
    fn on_suggestion(
        &mut self,
        id: SessionId,
        ply: usize,
        by: &'static str,
        best: Result<Option<Decision>, PlayerError>,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        session.suggesting = false;
        match best {
            Ok(best) => {
                if let Some(tokens) = best.as_ref().and_then(|best| best.tokens) {
                    *session.tokens.get_or_insert_default() += tokens;
                }
                let position = session.game.position_at(ply);
                session.suggestion = best.and_then(|best| {
                    let (from, to) = slide_paths(best.played).first().copied()?;
                    let text = best.annotations.text;
                    let note = (!text.is_empty()).then(|| HintNote {
                        by,
                        san: SanPlus::from_move(position, best.played).to_string(),
                        text,
                    });
                    Some(Suggestion {
                        ply,
                        arrow: Arrow {
                            from,
                            to,
                            mark: Mark::Blue,
                        },
                        note,
                    })
                });
            }
            Err(error) => self.report(format!("{by}: {error}"), cx),
        }
        cx.notify();
    }

    /// Shows or hides the likelihood of the running game, whose evaluations start then.
    pub(super) fn toggle_likelihood(&mut self, cx: &mut Context<Self>) {
        let id = self.active;
        if let Some(live) = self
            .session_mut(id)
            .and_then(|session| session.live.as_mut())
            .filter(|live| live.setup.assisted())
        {
            live.setup.live_likelihood = !live.setup.live_likelihood;
            self.evaluate_live(id, cx);
        }
        cx.notify();
    }

    // Saved games.

    /// Resigns a game from the history, open or not.
    pub(super) fn resign_saved(&mut self, path: &Path, cx: &mut Context<Self>) {
        let open = self
            .sessions
            .iter()
            .find(|session| session.path.as_deref() == Some(path))
            .map(|session| session.id);
        if let Some(id) = open {
            self.resign_session(id, cx);
            return;
        }
        let Some(meta) = library::load_meta(path) else {
            return self.report(tr("only-own-games-resign"), cx);
        };
        let saved = library::load(path).and_then(|mut game| {
            let winner = if meta.player {
                Color::Black
            } else {
                Color::White
            };
            game.adjudicate(KnownOutcome::Decisive { winner });
            library::save(&game, path)?;
            Ok(game)
        });
        match saved {
            Ok(game) => self.update_history(library::entry(path.to_owned(), &game)),
            Err(error) => self.report(error, cx),
        }
        cx.notify();
    }

    /// Names a game from the history, open or not. An open one is saved from its session,
    /// which would otherwise write its old tags back.
    pub(super) fn rename_saved(&mut self, path: &Path, name: &str, cx: &mut Context<Self>) {
        let open = self
            .sessions
            .iter_mut()
            .find(|session| session.path.as_deref() == Some(path));
        if let Some(session) = open {
            library::set_name(&mut session.game, name);
            let id = session.id;
            self.autosave(id, cx);
            return;
        }
        let saved = library::load(path).and_then(|mut game| {
            library::set_name(&mut game, name);
            library::save(&game, path)?;
            Ok(game)
        });
        match saved {
            Ok(game) => self.update_history(library::entry(path.to_owned(), &game)),
            Err(error) => self.report(error, cx),
        }
    }

    /// Opens the export dialog for a game from the history, open or not.
    pub(super) fn export_saved(&mut self, path: &Path, cx: &mut Context<Self>) {
        let open = self
            .sessions
            .iter()
            .find(|session| session.path.as_deref() == Some(path))
            .map(|session| session.game.clone());
        match open.map_or_else(|| library::load(path), Ok) {
            Ok(game) => self.show_export(game, path, cx),
            Err(error) => self.report(error, cx),
        }
    }

    /// Moves a game to the Recycle Bin, stopping it if it runs.
    pub(super) fn delete_saved(&mut self, path: &Path, cx: &mut Context<Self>) {
        if let Err(error) = library::delete(path) {
            return self.report(tr_with!("error-delete-game", error = error), cx);
        }
        self.history.retain(|entry| entry.path != path);
        self.sessions
            .retain(|session| session.path.as_deref() != Some(path));
        if self.sessions.is_empty() {
            let id = self.next_id();
            self.sessions.push(Session::new(id, Game::default()));
        }
        if !self
            .sessions
            .iter()
            .any(|session| session.id == self.active)
        {
            self.active = self.sessions[0].id;
        }
        cx.notify();
    }

    /// Shows a saved game. One already open comes to the front as it is, running or not;
    /// an unfinished one from an earlier run is picked up where it stopped.
    pub(super) fn open(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.settings_page = None;
        if let Some(session) = self
            .sessions
            .iter()
            .find(|session| session.path.as_deref() == Some(path))
        {
            self.active = session.id;
            cx.notify();
            return;
        }
        let game = match library::load(path) {
            Ok(game) => game,
            Err(error) => return self.report(error, cx),
        };
        let id = self.next_id();
        let mut session = Session::new(id, game);
        session.ply = session.game.moves().len();
        session.path = Some(path.to_owned());
        session.tokens = session
            .game
            .tag(TOKENS_TAG)
            .and_then(|tokens| tokens.parse().ok())
            .map(|total| Tokens { total, cached: 0 });
        let meta = library::load_meta(path).filter(|_| session.game.outcome() == Outcome::Unknown);
        let resumed = meta.is_some();
        if let Some(meta) = meta {
            let settings = Settings::get(cx);
            let player = if meta.player {
                Color::White
            } else {
                Color::Black
            };
            session.orientation = player;
            session.live = Some(Live {
                plan: meta.setup.plan(&settings, player.other()),
                player,
                opponent: None,
                plan_version: 0,
                thinking: false,
                paused: None,
                clock: resumed_clock(&session.game, &meta.setup),
                low_time_warned: false,
                evaluator: None,
                evaluating: false,
                instructions: None,
                introduced: false,
                stockfish_failed: false,
                setup: meta.setup,
            });
        }
        self.sessions.retain(|session| session.path.is_some());
        self.sessions.push(session);
        self.active = id;
        self.attach_notes(id, cx);
        if resumed {
            self.attach_instructions(id, cx);
            self.run_clocks(cx);
            self.after_change(id, cx);
        }
        cx.notify();
    }

    pub(super) fn prompt_open(cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(tr("open")),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await
                && let Some(path) = paths.first()
            {
                this.update(cx, |this, cx| this.open(path, cx)).ok();
            }
        })
        .detach();
    }

    pub(super) fn prompt_save(&mut self, cx: &mut Context<Self>) {
        let home = dirs::home_dir().unwrap_or_default();
        let path = cx.prompt_for_new_path(&home, Some("game.pgn"));
        let game = self.session().game.clone();
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = path.await
                && let Err(error) = library::save(&game, &path)
            {
                this.update(cx, |this, cx| this.report(error, cx)).ok();
            }
        })
        .detach();
    }

    // Review.

    /// Runs Stockfish over the game on show, reporting progress as it goes.
    pub(super) fn analyze(&mut self, cx: &mut Context<Self>) {
        let id = self.active;
        let settings = Settings::get(cx);
        let Some(session) = self.session_mut(id) else {
            return;
        };
        if session.live.is_some() || session.analysis.is_some() || session.game.moves().is_empty() {
            return;
        }
        let config = EngineConfig::stockfish(&settings.stockfish, None);
        let mut game = session.game.clone();
        session.analysis = Some((0, game.moves().len()));
        cx.notify();

        let (updates, incoming) = mpsc::channel();
        cx.background_spawn(async move {
            let progress = updates.clone();
            let analysed = Engine::start(&config).and_then(|mut engine| {
                evaluate_game(&mut engine, &mut game, |done| {
                    let _ = progress.send(AnalysisUpdate::Progress(done));
                })
            });
            let _ = updates.send(AnalysisUpdate::Done(analysed.map(|()| Box::new(game))));
        })
        .detach();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(CLOCK_TICK).await;
                for update in incoming.try_iter() {
                    let finished = matches!(update, AnalysisUpdate::Done(_));
                    this.update(cx, |this, cx| this.on_analysis(id, update, cx))
                        .ok();
                    if finished {
                        return;
                    }
                }
            }
        })
        .detach();
    }

    fn on_analysis(&mut self, id: SessionId, update: AnalysisUpdate, cx: &mut Context<Self>) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        match update {
            AnalysisUpdate::Progress(done) => {
                if let Some((progress, _)) = &mut session.analysis {
                    *progress = done;
                }
            }
            AnalysisUpdate::Done(analysed) => {
                session.analysis = None;
                match analysed {
                    Ok(mut game) => {
                        // Notes typed while the analysis ran are newer than its copy.
                        game.notes = std::mem::take(&mut session.game.notes);
                        session.game = *game;
                        self.autosave(id, cx);
                    }
                    Err(error) => self.report(format!("Stockfish: {error}"), cx),
                }
            }
        }
        cx.notify();
    }

    /// Has an agent comment on the game on show, in one call.
    pub(super) fn review_with(&mut self, kind: AgentKind, cx: &mut Context<Self>) {
        let id = self.active;
        let Some(session) = self.session_mut(id) else {
            return;
        };
        if session.live.is_some() || session.reviewer.is_some() {
            return;
        }
        if session.game.moves().is_empty() {
            return;
        }
        let adapters_dir = Settings::adapters_dir().unwrap_or_default();
        if adapters::readiness(kind, &adapters_dir) != Readiness::Ready {
            return self.report(tr_with!("agent-not-set-up", agent = kind.name()), cx);
        }
        let Some(session) = self.session_mut(id) else {
            return;
        };
        session.reviewer = Some(kind);
        let mut game = session.game.clone();
        cx.notify();
        let config = helper_config(kind);
        cx.spawn(async move |this, cx| {
            let reviewed = cx
                .background_spawn(async move {
                    review_game(&config, &mut game).map(|(added, tokens)| (game, added, tokens))
                })
                .await;
            this.update(cx, |this, cx| this.on_reviewed(id, reviewed, cx))
                .ok();
        })
        .detach();
    }

    fn on_reviewed(
        &mut self,
        id: SessionId,
        reviewed: Result<(Game, usize, Option<Tokens>), PlayerError>,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.session_mut(id) else {
            return;
        };
        session.reviewer = None;
        match reviewed {
            Ok((_, 0, _)) => self.report(tr("review-empty"), cx),
            Ok((mut game, _, tokens)) => {
                // Notes typed while the review ran are newer than its copy.
                game.notes = std::mem::take(&mut session.game.notes);
                let first = game
                    .moves()
                    .iter()
                    .position(|played| !played.annotations.text.is_empty());
                session.game = game;
                if let Some(tokens) = tokens {
                    *session.tokens.get_or_insert_default() += tokens;
                }
                self.autosave(id, cx);
                // Straight to the first comment, so that the review can't go unnoticed.
                if let Some(index) = first {
                    self.show_ply(id, index + 1, false, cx);
                }
            }
            Err(error) => self.report(tr_with!("error-review", error = error.to_string()), cx),
        }
        cx.notify();
    }
}
