//! Finished and running games on disk: one PGN file each, so that any chess tool can
//! read them.

use std::fs;
use std::path::{Path, PathBuf};

use chessdesk_core::{Game, pgn};
use gpui::SharedString;
use serde::{Deserialize, Serialize};
use shakmaty::{Color, KnownOutcome, Outcome};

use crate::i18n::{tr, tr_with};
use crate::settings::Settings;
use crate::setup::Setup;

/// Where a game keeps the name the player gave it.
const NAME_TAG: &str = "ChessdeskName";

/// A saved game, as the history lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub title: String,
    pub date: String,
    /// How many moves each side has played.
    pub moves: usize,
    /// `None` while the game is on.
    pub result: Option<Standing>,
}

impl Entry {
    pub fn unfinished(&self) -> bool {
        self.result.is_none()
    }
}

/// How a finished game went for the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    Won,
    Lost,
    Draw,
    /// A game the player wasn't in, from elsewhere: `1-0`.
    Others(String),
}

impl Standing {
    pub fn label(&self) -> SharedString {
        match self {
            Self::Won => tr("standing-won"),
            Self::Lost => tr("standing-lost"),
            Self::Draw => tr("draw"),
            Self::Others(result) => result.clone().into(),
        }
    }
}

/// The player's side, who is `You` in the tags, if they played this game.
pub fn player(game: &Game) -> Option<Color> {
    match (game.tag("White"), game.tag("Black")) {
        (Some("You"), _) => Some(Color::White),
        (_, Some("You")) => Some(Color::Black),
        _ => None,
    }
}

/// How a game stands for the player.
pub fn standing(game: &Game) -> Option<Standing> {
    let Outcome::Known(outcome) = game.outcome() else {
        return None;
    };
    Some(match (outcome, player(game)) {
        (_, None) => Standing::Others(outcome.as_str().to_owned()),
        (KnownOutcome::Draw, _) => Standing::Draw,
        (KnownOutcome::Decisive { winner }, Some(player)) if winner == player => Standing::Won,
        (KnownOutcome::Decisive { .. }, _) => Standing::Lost,
    })
}

/// What it takes to pick a game up again. A PGN doesn't say how the opponent was set up,
/// so this lives beside it, in a TOML file of the same name.
#[derive(Debug, Serialize, Deserialize)]
pub struct Meta {
    pub setup: Setup,
    /// Whether the player has White.
    pub player: bool,
}

fn folder() -> Option<PathBuf> {
    Some(Settings::data_dir()?.join("games"))
}

/// A new file for a game, named so that the newest sort last.
pub fn new_path() -> Option<PathBuf> {
    let stamp = jiff::Zoned::now().strftime("%Y-%m-%d-%H%M%S").to_string();
    Some(folder()?.join(format!("{stamp}.pgn")))
}

pub fn save(game: &Game, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, pgn::write(game)).map_err(|error| error.to_string())
}

pub fn load(path: &Path) -> Result<Game, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    pgn::read(&text).map_err(|error| error.to_string())
}

pub fn save_meta(game: &Path, meta: &Meta) -> Result<(), String> {
    if let Some(parent) = game.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = toml::to_string(meta).map_err(|error| error.to_string())?;
    fs::write(game.with_extension("toml"), text).map_err(|error| error.to_string())
}

/// How a game was set up, if it was started here.
pub fn load_meta(game: &Path) -> Option<Meta> {
    let text = fs::read_to_string(game.with_extension("toml")).ok()?;
    toml::from_str(&text).ok()
}

/// The ways a game can leave Chessdesk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Pgn,
    PgnMovesOnly,
    Fen,
}

impl ExportFormat {
    pub const ALL: [Self; 3] = [Self::Pgn, Self::PgnMovesOnly, Self::Fen];

    pub fn label(self) -> SharedString {
        tr(match self {
            Self::Pgn => "export-pgn",
            Self::PgnMovesOnly => "export-pgn-moves",
            Self::Fen => "export-fen",
        })
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Pgn | Self::PgnMovesOnly => "pgn",
            Self::Fen => "fen",
        }
    }

    fn render(self, game: &Game) -> String {
        match self {
            Self::Pgn => pgn::write(game),
            Self::PgnMovesOnly => pgn::write_moves_only(game),
            Self::Fen => format!("{}\n", game.fen()),
        }
    }
}

pub fn export(game: &Game, format: ExportFormat, path: &Path) -> Result<(), String> {
    fs::write(path, format.render(game)).map_err(|error| error.to_string())
}

/// Moves a game and its setup to the Recycle Bin, so that a slip can be undone.
pub fn delete(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|error| error.to_string())?;
    let meta = path.with_extension("toml");
    if meta.exists() {
        trash::delete(meta).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Gives a game a name of the player's own, or with nothing, its usual title back.
pub fn set_name(game: &mut Game, name: &str) {
    game.set_tag(NAME_TAG, name);
}

/// What a game is listed as: the name it was given, or else who it was against, the other
/// side where the player is `You`.
fn title(game: &Game) -> String {
    if let Some(name) = game.tag(NAME_TAG).filter(|name| !name.is_empty()) {
        return name.to_owned();
    }
    let name = |tag| game.tag(tag).unwrap_or("?");
    match (name("White"), name("Black")) {
        ("You", opponent) | (opponent, "You") => opponent.to_owned(),
        (white, black) => tr_with!("versus", white = white, black = black).to_string(),
    }
}

pub fn entry(path: PathBuf, game: &Game) -> Entry {
    Entry {
        title: title(game),
        date: game.tag("Date").unwrap_or("?").to_owned(),
        moves: game.moves().len().div_ceil(2),
        result: standing(game),
        path,
    }
}

/// Every saved game, newest first. Files that don't read as games are left out.
pub fn list() -> Vec<Entry> {
    let Some(entries) = folder().and_then(|folder| fs::read_dir(folder).ok()) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "pgn"))
        .collect();
    paths.sort();
    paths.reverse();
    paths
        .into_iter()
        .filter_map(|path| {
            let game = load(&path).ok()?;
            Some(entry(path, &game))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use shakmaty::Position;

    use super::*;

    #[test]
    fn a_name_replaces_the_title_and_survives_saving() {
        let mut game = Game::default();
        game.set_tag("White", "You");
        game.set_tag("Black", "Maia");
        assert_eq!(title(&game), "Maia");

        set_name(&mut game, r#"The "immortal" game"#);
        let read = pgn::read(&pgn::write(&game)).unwrap();
        assert_eq!(title(&read), r#"The "immortal" game"#);

        set_name(&mut game, "");
        assert_eq!(title(&game), "Maia");
    }

    #[test]
    fn every_format_writes_its_own_kind_of_file() {
        let game = Game::default();
        assert!(ExportFormat::Pgn.render(&game).starts_with("[Event"));
        assert!(ExportFormat::Fen.render(&game).starts_with("rnbqkbnr/"));
        assert_eq!(ExportFormat::PgnMovesOnly.extension(), "pgn");
        assert_eq!(ExportFormat::Fen.extension(), "fen");
    }

    #[test]
    fn a_setup_survives_beside_its_game() {
        let folder = std::env::temp_dir().join(format!("chessdesk-meta-{}", std::process::id()));
        let game = folder.join("game.pgn");
        let meta = Meta {
            setup: Setup::default(),
            player: false,
        };
        save_meta(&game, &meta).unwrap();
        let read = load_meta(&game).unwrap();
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(read.setup, meta.setup);
        assert!(!read.player);
    }

    #[test]
    fn a_result_is_told_from_the_players_side() {
        let mut game = Game::default();
        game.set_tag("White", "Maia 1500");
        game.set_tag("Black", "You");
        game.adjudicate(KnownOutcome::Decisive {
            winner: Color::Black,
        });
        assert_eq!(standing(&game), Some(Standing::Won));
        game.adjudicate(KnownOutcome::Decisive {
            winner: Color::White,
        });
        assert_eq!(standing(&game), Some(Standing::Lost));
        game.adjudicate(KnownOutcome::Draw);
        assert_eq!(standing(&game), Some(Standing::Draw));
        game.set_tag("Black", "Someone");
        assert_eq!(
            standing(&game),
            Some(Standing::Others("1/2-1/2".to_owned()))
        );
    }

    #[test]
    fn an_entry_says_who_played_and_how_it_ended() {
        let mut game = Game::default();
        game.set_tag("White", "You");
        game.set_tag("Black", "Maia 1500");
        game.set_tag("Date", "2026.11.07");
        let first = game.position().legal_moves()[0];
        assert!(game.play(first));
        let listed = entry(PathBuf::from("game.pgn"), &game);
        assert_eq!(listed.title, "Maia 1500");
        assert_eq!(listed.date, "2026.11.07");
        assert_eq!(listed.moves, 1);
        assert!(listed.unfinished());
    }
}
