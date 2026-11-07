//! A stand-in UCI engine for trying Chessdesk without one installed. Plays the first legal
//! move and makes up a likelihood that swings with the position, so that graphs and
//! mistake marks have something to show. Set `stockfish` in `config.toml` to this binary.

use std::io::{self, BufRead, Write};

use chessdesk_core::Game;
use shakmaty::uci::UciMove;
use shakmaty::{CastlingMode, Chess, Position};

fn main() -> io::Result<()> {
    let mut position = Chess::default();
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line?;
        match line.split_whitespace().next() {
            Some("uci") => writeln!(out, "id name Fake\nuciok")?,
            Some("isready") => writeln!(out, "readyok")?,
            Some("position") => {
                if let Some(fen) = line.strip_prefix("position fen ")
                    && let Some(parsed) = Game::parse_fen(fen)
                {
                    position = parsed;
                }
            }
            Some("go") => {
                let moves = position.legal_moves();
                let swing = (moves.len() * 37 % 600) as u32;
                let (win, draw) = (200 + swing, 200);
                let loss = 1000 - win - draw;
                let score = win as i32 - loss as i32;
                writeln!(out, "info depth 1 score cp {score} wdl {win} {draw} {loss}")?;
                match moves.first() {
                    Some(best) => {
                        let best = UciMove::from_move(*best, CastlingMode::Standard);
                        writeln!(out, "bestmove {best}")?;
                    }
                    None => writeln!(out, "bestmove (none)")?,
                }
            }
            Some("quit") => break,
            _ => {}
        }
        out.flush()?;
    }
    Ok(())
}
