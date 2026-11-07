//! A UCI client: Stockfish, and Maia through `lc0`. Generic over its streams, so that
//! tests can script an engine without starting one.

use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use chessdesk_core::game::fen_of;
use chessdesk_core::{Annotations, Eval, Game, Score, Wdl};
use shakmaty::uci::UciMove;
use shakmaty::{Chess, Color, Move, Position};

use crate::{Decision, Opponent, PlayerError};

/// How long an engine thinks about a move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    Milliseconds(u32),
    /// `go nodes 1` is how Maia plays: its strength is the network's first instinct.
    Nodes(u32),
}

impl Limit {
    fn command(self) -> String {
        match self {
            Self::Milliseconds(time) => format!("go movetime {time}"),
            Self::Nodes(nodes) => format!("go nodes {nodes}"),
        }
    }
}

/// `UCI_Elo` is calibrated at 60 s + 0.6 s a game, about 2 s a move. Less time plays below
/// the number on the label.
const STOCKFISH_MOVE_TIME: Limit = Limit::Milliseconds(2000);

/// How to start an engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineConfig {
    pub program: String,
    pub args: Vec<String>,
    /// Sent as `setoption` after the handshake.
    pub options: Vec<(String, String)>,
    pub limit: Limit,
}

impl EngineConfig {
    /// Stockfish, at full strength or limited to an Elo rating.
    pub fn stockfish(program: &str, elo: Option<u32>) -> Self {
        let mut options = vec![("UCI_ShowWDL".to_owned(), "true".to_owned())];
        if let Some(elo) = elo {
            options.push(("UCI_LimitStrength".to_owned(), "true".to_owned()));
            options.push(("UCI_Elo".to_owned(), elo.to_string()));
        }
        Self {
            program: program.to_owned(),
            args: Vec::new(),
            options,
            limit: STOCKFISH_MOVE_TIME,
        }
    }

    /// Maia: `lc0` with a Maia network such as `maia-1500.pb.gz`.
    pub fn maia(program: &str, weights: &str) -> Self {
        Self {
            program: program.to_owned(),
            args: vec![format!("--weights={weights}")],
            options: Vec::new(),
            limit: Limit::Nodes(1),
        }
    }
}

/// What a search found, from the view of the side to move, as UCI reports it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Search {
    score: Option<Score>,
    wdl: Option<(u16, u16, u16)>,
    best: Option<String>,
}

/// Takes in an `info` line, keeping the latest score and WDL. Only the main line counts.
/// A bound only says the score lies beyond it, and a search that runs out of time often
/// ends on one, so it counts for nothing once there's a true score.
fn read_info(line: &str, search: &mut Search) {
    let is_bound = |word| word == "lowerbound" || word == "upperbound";
    if search.score.is_some() && line.split_whitespace().any(is_bound) {
        return;
    }
    let mut words = line.split_whitespace();
    while let Some(word) = words.next() {
        match word {
            "multipv" if words.next() != Some("1") => return,
            "score" => match (words.next(), words.next().and_then(|n| n.parse().ok())) {
                (Some("cp"), Some(value)) => search.score = Some(Score::Centipawns(value)),
                (Some("mate"), Some(value)) => search.score = Some(Score::Mate(value)),
                _ => {}
            },
            "wdl" => {
                let mut value = || words.next().and_then(|n| n.parse().ok());
                if let (Some(win), Some(draw), Some(loss)) = (value(), value(), value()) {
                    search.wdl = Some((win, draw, loss));
                }
            }
            // Free text and the line itself come last, and could hold anything.
            "pv" | "string" => return,
            _ => {}
        }
    }
}

/// Turns a search's view into White's.
fn white_view(search: &Search, turn: Color) -> Eval {
    let flip = turn == Color::Black;
    Eval {
        score: search.score.map(|score| match score {
            Score::Centipawns(value) if flip => Score::Centipawns(-value),
            Score::Mate(value) if flip => Score::Mate(-value),
            same => same,
        }),
        wdl: search.wdl.map(|(win, draw, loss)| Wdl {
            white: if flip { loss } else { win },
            draw,
            black: if flip { win } else { loss },
        }),
    }
}

/// A connection to an engine.
pub struct Client<R, W> {
    reader: R,
    writer: W,
}

impl<R: BufRead, W: Write> Client<R, W> {
    fn send(&mut self, command: &str) -> io::Result<()> {
        writeln!(self.writer, "{command}")?;
        self.writer.flush()
    }

    fn read_line(&mut self) -> io::Result<String> {
        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the engine stopped",
            ));
        }
        Ok(line)
    }

    fn wait_for(&mut self, answer: &str) -> io::Result<()> {
        while self.read_line()?.trim() != answer {}
        Ok(())
    }

    /// The handshake, then the options.
    fn start(&mut self, options: &[(String, String)]) -> io::Result<()> {
        self.send("uci")?;
        self.wait_for("uciok")?;
        for (name, value) in options {
            self.send(&format!("setoption name {name} value {value}"))?;
        }
        self.send("isready")?;
        self.wait_for("readyok")
    }

    fn search(&mut self, position: &Chess, limit: Limit) -> io::Result<Search> {
        self.send(&format!("position fen {}", fen_of(position)))?;
        self.send(&limit.command())?;
        let mut search = Search::default();
        loop {
            let line = self.read_line()?;
            let mut words = line.split_whitespace();
            match words.next() {
                Some("info") => read_info(&line, &mut search),
                Some("bestmove") => {
                    search.best = words.next().map(str::to_owned);
                    return Ok(search);
                }
                _ => {}
            }
        }
    }
}

/// Keeps Windows from opening a console for an engine: it's a console program, and a
/// windowed app has none to share.
#[cfg(windows)]
pub(crate) fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(crate) fn hide_console(_: &mut Command) {}

/// A running engine.
pub struct Engine {
    client: Client<BufReader<ChildStdout>, ChildStdin>,
    child: Child,
    limit: Limit,
}

impl Engine {
    pub fn start(config: &EngineConfig) -> Result<Self, PlayerError> {
        let process_error =
            |error| PlayerError::Process(format!("could not run {}: {error}", config.program));
        let mut command = Command::new(&config.program);
        command
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        hide_console(&mut command);
        let mut child = command.spawn().map_err(process_error)?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(PlayerError::Process("the engine has no pipes".to_owned()));
        };
        let mut client = Client {
            reader: BufReader::new(stdout),
            writer: stdin,
        };
        client.start(&config.options).map_err(process_error)?;
        Ok(Self {
            client,
            child,
            limit: config.limit,
        })
    }

    /// Evaluates a position, and names the best move there if there is one.
    pub fn evaluate(
        &mut self,
        position: &Chess,
        limit: Limit,
    ) -> Result<(Eval, Option<Move>), PlayerError> {
        let search = self
            .client
            .search(position, limit)
            .map_err(|error| PlayerError::Process(error.to_string()))?;
        let best = best_move(&search, position);
        Ok((white_view(&search, position.turn()), best))
    }
}

fn best_move(search: &Search, position: &Chess) -> Option<Move> {
    let best: UciMove = search.best.as_deref()?.parse().ok()?;
    best.to_move(position).ok()
}

impl Opponent for Engine {
    fn next_move(&mut self, game: &Game, _: &str) -> Result<Decision, PlayerError> {
        let position = game.position();
        let (_, best) = self.evaluate(&position, self.limit)?;
        let played =
            best.ok_or_else(|| PlayerError::Process("the engine named no move".to_owned()))?;
        Ok(Decision {
            played,
            annotations: Annotations::default(),
            tokens: None,
        })
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.client.send("quit");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use shakmaty::Square;

    use super::*;

    fn scripted(lines: &str) -> Client<Cursor<Vec<u8>>, Vec<u8>> {
        Client {
            reader: Cursor::new(lines.as_bytes().to_vec()),
            writer: Vec::new(),
        }
    }

    #[test]
    fn handshake_sends_options_and_waits() {
        let mut client = scripted("id name Fake\nuciok\nreadyok\n");
        let options = [("UCI_ShowWDL".to_owned(), "true".to_owned())];
        client.start(&options).unwrap();
        assert_eq!(
            String::from_utf8(client.writer).unwrap(),
            "uci\nsetoption name UCI_ShowWDL value true\nisready\n"
        );
    }

    #[test]
    fn search_keeps_the_last_main_line_score() {
        let mut client = scripted(
            "info depth 1 score cp 10 wdl 100 800 100 pv e2e4\n\
             info depth 2 multipv 2 score cp -90 wdl 0 100 900 pv a2a3\n\
             info depth 2 multipv 1 score cp 25 wdl 200 700 100 nodes 40 pv e2e4 e7e5\n\
             info string score cp 999\n\
             bestmove e2e4 ponder e7e5\n",
        );
        let search = client.search(&Chess::default(), Limit::Nodes(100)).unwrap();
        assert_eq!(search.score, Some(Score::Centipawns(25)));
        assert_eq!(search.wdl, Some((200, 700, 100)));
        let best = best_move(&search, &Chess::default()).unwrap();
        assert_eq!(best.to(), Square::E4);
        let sent = String::from_utf8(client.writer).unwrap();
        assert!(sent.ends_with("go nodes 100\n"));
    }

    #[test]
    fn a_bound_does_not_replace_a_true_score() {
        let mut client = scripted(
            "info depth 19 score cp 37 wdl 77 919 4 pv e2e4\n\
             info depth 20 score cp 27 upperbound wdl 52 942 6 pv e2e4\n\
             bestmove e2e4\n",
        );
        let search = client.search(&Chess::default(), Limit::Nodes(100)).unwrap();
        assert_eq!(search.score, Some(Score::Centipawns(37)));
        assert_eq!(search.wdl, Some((77, 919, 4)));
    }

    #[test]
    fn a_bound_is_better_than_nothing() {
        let mut client = scripted(
            "info depth 1 score cp 27 lowerbound wdl 52 942 6 pv e2e4\n\
             bestmove e2e4\n",
        );
        let search = client.search(&Chess::default(), Limit::Nodes(100)).unwrap();
        assert_eq!(search.wdl, Some((52, 942, 6)));
    }

    #[test]
    fn evals_are_turned_to_whites_view() {
        let search = Search {
            score: Some(Score::Mate(3)),
            wdl: Some((900, 80, 20)),
            best: None,
        };
        let eval = white_view(&search, Color::Black);
        assert_eq!(eval.score, Some(Score::Mate(-3)));
        assert_eq!(
            eval.wdl,
            Some(Wdl {
                white: 20,
                draw: 80,
                black: 900
            })
        );
        assert_eq!(
            white_view(&search, Color::White).score,
            Some(Score::Mate(3))
        );
    }

    #[test]
    fn a_vanished_engine_is_an_error_not_a_hang() {
        let mut client = scripted("info depth 1 score cp 1\n");
        assert!(client.search(&Chess::default(), Limit::Nodes(1)).is_err());
    }

    #[test]
    fn checkmated_positions_have_no_best_move() {
        let search = Search {
            best: Some("(none)".to_owned()),
            ..Search::default()
        };
        assert_eq!(best_move(&search, &Chess::default()), None);
    }

    #[test]
    fn presets_ask_for_what_they_need() {
        let limited = EngineConfig::stockfish("stockfish", Some(1500));
        assert!(
            limited
                .options
                .iter()
                .any(|(name, value)| name == "UCI_Elo" && value == "1500")
        );
        let maia = EngineConfig::maia("lc0", "maia-1500.pb.gz");
        assert_eq!(maia.limit, Limit::Nodes(1));
        assert_eq!(maia.args, ["--weights=maia-1500.pb.gz"]);
    }
}
