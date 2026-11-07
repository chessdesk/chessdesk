//! Plugins, which are nothing but data: a folder with a `plugin.toml` naming piece sets
//! (folders of 12 SVG files, `wK.svg` to `bP.svg`), colour themes (TOML files), boards
//! (an image of all 64 squares, which looks the same under any colour theme), sound packs
//! (folders of OGG or WAV files, one per [`Sound`]) and languages (Fluent files).

use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use shakmaty::{Color, Piece, Role};

use crate::sound::Sound;

/// The file that makes a folder a plugin.
pub const MANIFEST: &str = "plugin.toml";

/// The largest SVG accepted, so that a broken plugin can't stall rendering.
const MAX_SVG_BYTES: u64 = 1 << 20;
/// The largest board image accepted.
const MAX_BOARD_BYTES: u64 = 8 << 20;
/// The largest sound accepted: these are short, and each is read when it plays.
const MAX_SOUND_BYTES: u64 = 4 << 20;
/// The largest translation accepted.
const MAX_LANGUAGE_BYTES: u64 = 1 << 20;
const SOUND_EXTENSIONS: [&str; 2] = ["ogg", "wav"];

const ROLES: [Role; 6] = [
    Role::King,
    Role::Queen,
    Role::Rook,
    Role::Bishop,
    Role::Knight,
    Role::Pawn,
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    name: String,
    /// Required so that plugins stay tellable apart; Chessdesk itself doesn't read it.
    #[expect(dead_code)]
    version: String,
    description: Option<String>,
    /// Of the artwork, which varies from set to set.
    license: String,
    #[serde(default)]
    pieces: Vec<SetEntry>,
    #[serde(default)]
    themes: Vec<ThemeEntry>,
    #[serde(default)]
    boards: Vec<BoardEntry>,
    #[serde(default)]
    sounds: Vec<SoundEntry>,
    #[serde(default)]
    languages: Vec<LanguageEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetEntry {
    name: String,
    dir: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeEntry {
    name: String,
    file: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BoardEntry {
    name: String,
    image: PathBuf,
    light: String,
    dark: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SoundEntry {
    name: String,
    dir: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LanguageEntry {
    name: String,
    locale: String,
    file: PathBuf,
}

/// A board: a picture of the squares, a8 at the top left, in PNG or JPEG.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardTheme {
    pub name: String,
    pub plugin: String,
    pub license: String,
    pub image: PathBuf,
    /// The squares' colours as hex, for the coordinates printed on them.
    pub light: String,
    pub dark: String,
}

/// A sound pack: only the sound of a move is required.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoundPack {
    pub name: String,
    pub plugin: String,
    pub license: String,
    /// Holds the files [`sound_file`] finds.
    pub dir: PathBuf,
}

/// A translation of the interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Language {
    /// In the language itself, as it shows in the list of languages.
    pub name: String,
    pub plugin: String,
    pub license: String,
    /// A BCP 47 tag such as `de` or `pt-BR`, for plurals and matching the system's.
    pub locale: String,
    /// A Project Fluent file.
    pub file: PathBuf,
}

/// A colour theme, selected by `name`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    pub plugin: String,
    pub license: String,
    /// The colours, as TOML with the keys of a theme file.
    pub colors: String,
}

/// Everything the plugins folder holds, and what went wrong reading it.
#[derive(Debug, Default)]
pub struct Plugins {
    pub sets: Vec<PieceSet>,
    pub themes: Vec<Theme>,
    pub boards: Vec<BoardTheme>,
    pub sound_packs: Vec<SoundPack>,
    pub languages: Vec<Language>,
    pub errors: Vec<PluginError>,
}

/// A piece set ready to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PieceSet {
    pub name: String,
    pub plugin: String,
    pub description: Option<String>,
    pub license: String,
    /// Holds the files [`file_name`] names.
    pub dir: PathBuf,
}

/// A plugin that could not be used, for the error ribbon.
#[derive(Debug)]
pub struct PluginError {
    pub folder: PathBuf,
    pub reason: String,
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.folder.display(), self.reason)
    }
}

impl std::error::Error for PluginError {}

/// The file of a piece, named as most piece sets name them: `wK.svg`.
pub fn file_name(piece: Piece) -> String {
    let color = if piece.color == Color::White {
        'w'
    } else {
        'b'
    };
    format!("{color}{}.svg", piece.role.upper_char())
}

fn own_sound_file(dir: &Path, sound: Sound) -> Option<PathBuf> {
    SOUND_EXTENSIONS
        .iter()
        .map(|extension| dir.join(format!("{}.{extension}", sound.file_stem())))
        .find(|path| path.is_file())
}

/// A pack's file for a sound, or for the sound that stands in for it.
pub fn sound_file(dir: &Path, sound: Sound) -> Option<PathBuf> {
    own_sound_file(dir, sound).or_else(|| own_sound_file(dir, sound.fallback()?))
}

/// Every piece, once.
pub fn all_pieces() -> impl Iterator<Item = Piece> {
    Color::ALL
        .into_iter()
        .flat_map(|color| ROLES.map(|role| Piece { color, role }))
}

fn check_set(dir: &Path) -> Result<(), String> {
    for piece in all_pieces() {
        let path = dir.join(file_name(piece));
        let metadata = fs::metadata(&path).map_err(|_| format!("missing {}", path.display()))?;
        if metadata.len() > MAX_SVG_BYTES {
            return Err(format!("{} is larger than 1 MiB", path.display()));
        }
    }
    Ok(())
}

fn check_sounds(dir: &Path) -> Result<(), String> {
    if own_sound_file(dir, Sound::Move).is_none() {
        return Err(format!("{} has no move sound", dir.display()));
    }
    for path in Sound::ALL
        .into_iter()
        .filter_map(|sound| own_sound_file(dir, sound))
    {
        let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
        if metadata.len() > MAX_SOUND_BYTES {
            return Err(format!("{} is larger than 4 MiB", path.display()));
        }
    }
    Ok(())
}

/// A manifest may only reach files inside its own folder.
fn inside(folder: &Path, relative: &Path) -> Result<PathBuf, String> {
    if relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        Ok(folder.join(relative))
    } else {
        Err(format!("{} leaves the plugin", relative.display()))
    }
}

fn load_plugin(folder: &Path) -> Result<Plugins, String> {
    let text = fs::read_to_string(folder.join(MANIFEST)).map_err(|error| error.to_string())?;
    let manifest: Manifest = toml::from_str(&text).map_err(|error| error.to_string())?;
    let mut found = Plugins::default();
    for entry in manifest.pieces {
        let dir = inside(folder, &entry.dir)?;
        check_set(&dir)?;
        found.sets.push(PieceSet {
            name: entry.name,
            plugin: manifest.name.clone(),
            description: manifest.description.clone(),
            license: manifest.license.clone(),
            dir,
        });
    }
    for entry in manifest.themes {
        let path = inside(folder, &entry.file)?;
        let colors =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        found.themes.push(Theme {
            name: entry.name,
            plugin: manifest.name.clone(),
            license: manifest.license.clone(),
            colors,
        });
    }
    for entry in manifest.boards {
        let image = inside(folder, &entry.image)?;
        let metadata = fs::metadata(&image).map_err(|_| format!("missing {}", image.display()))?;
        if metadata.len() > MAX_BOARD_BYTES {
            return Err(format!("{} is larger than 8 MiB", image.display()));
        }
        found.boards.push(BoardTheme {
            name: entry.name,
            plugin: manifest.name.clone(),
            license: manifest.license.clone(),
            image,
            light: entry.light,
            dark: entry.dark,
        });
    }
    for entry in manifest.sounds {
        let dir = inside(folder, &entry.dir)?;
        check_sounds(&dir)?;
        found.sound_packs.push(SoundPack {
            name: entry.name,
            plugin: manifest.name.clone(),
            license: manifest.license.clone(),
            dir,
        });
    }
    for entry in manifest.languages {
        let file = inside(folder, &entry.file)?;
        let metadata = fs::metadata(&file).map_err(|_| format!("missing {}", file.display()))?;
        if metadata.len() > MAX_LANGUAGE_BYTES {
            return Err(format!("{} is larger than 1 MiB", file.display()));
        }
        found.languages.push(Language {
            name: entry.name,
            plugin: manifest.name.clone(),
            license: manifest.license.clone(),
            locale: entry.locale,
            file,
        });
    }
    Ok(found)
}

/// Reads every plugin in `root`. A broken one is reported and the others still load; a
/// missing `root` just means there are none.
pub fn discover(root: &Path) -> Plugins {
    let mut plugins = Plugins::default();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return plugins,
        Err(error) => {
            plugins.errors.push(PluginError {
                folder: root.to_owned(),
                reason: error.to_string(),
            });
            return plugins;
        }
    };
    let mut folders: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(MANIFEST).is_file())
        .collect();
    folders.sort();
    for folder in folders {
        match load_plugin(&folder) {
            Ok(found) => {
                plugins.sets.extend(found.sets);
                plugins.themes.extend(found.themes);
                plugins.boards.extend(found.boards);
                plugins.sound_packs.extend(found.sound_packs);
                plugins.languages.extend(found.languages);
            }
            Err(reason) => plugins.errors.push(PluginError { folder, reason }),
        }
    }
    plugins
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plugins folder under the system's temp folder, removed afterwards.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("chessdesk-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn plugin(&self, name: &str, manifest: &str, files: usize) {
            let dir = self.0.join(name);
            fs::create_dir_all(dir.join("svg")).unwrap();
            fs::write(dir.join(MANIFEST), manifest).unwrap();
            for piece in all_pieces().take(files) {
                fs::write(dir.join("svg").join(file_name(piece)), "<svg/>").unwrap();
            }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const GOOD: &str = r#"
        name = "walnut"
        version = "1.0.0"
        license = "CC0-1.0"
        [[pieces]]
        name = "Walnut"
        dir = "svg"
    "#;

    #[test]
    fn names_files_as_piece_sets_do() {
        let piece = Piece {
            color: Color::Black,
            role: Role::Knight,
        };
        assert_eq!(file_name(piece), "bN.svg");
        assert_eq!(all_pieces().count(), 12);
    }

    #[test]
    fn a_broken_plugin_does_not_stop_the_others() {
        let scratch = Scratch::new("broken");
        scratch.plugin("good", GOOD, 12);
        scratch.plugin("incomplete", GOOD, 11);
        scratch.plugin("typo", &GOOD.replace("license", "licence"), 12);
        let plugins = discover(&scratch.0);
        assert_eq!(plugins.sets.len(), 1);
        assert_eq!(plugins.sets[0].name, "Walnut");
        assert_eq!(plugins.errors.len(), 2);
    }

    #[test]
    fn a_plugin_cannot_reach_outside_its_folder() {
        let scratch = Scratch::new("escape");
        scratch.plugin("sneaky", &GOOD.replace("\"svg\"", "\"../good/svg\""), 12);
        let plugins = discover(&scratch.0);
        assert_eq!(plugins.sets, []);
        assert!(plugins.errors[0].reason.contains("leaves the plugin"));
    }

    const SOUNDS: &str = r#"
        name = "clack"
        version = "1.0.0"
        license = "CC0-1.0"
        [[sounds]]
        name = "Clack"
        dir = "svg"
    "#;

    #[test]
    fn a_sound_pack_needs_a_move_and_nothing_else() {
        let scratch = Scratch::new("sounds");
        scratch.plugin("clack", SOUNDS, 0);
        scratch.plugin("silent", SOUNDS, 0);
        let dir = scratch.0.join("clack/svg");
        fs::write(dir.join("move.ogg"), "ogg").unwrap();
        fs::write(dir.join("win.wav"), "wav").unwrap();
        let plugins = discover(&scratch.0);
        assert_eq!(plugins.sound_packs.len(), 1);
        assert_eq!(plugins.errors.len(), 1);
        assert!(plugins.errors[0].reason.contains("no move sound"));

        let file = |sound| sound_file(&dir, sound).unwrap();
        assert_eq!(file(Sound::Win), dir.join("win.wav"));
        // A capture left out sounds like a move, and an ending left out like nothing.
        assert_eq!(file(Sound::Capture), dir.join("move.ogg"));
        assert_eq!(sound_file(&dir, Sound::Lose), None);
    }

    #[test]
    fn languages_point_at_their_file() {
        let scratch = Scratch::new("languages");
        let manifest = r#"
            name = "lang-fy"
            version = "1.0.0"
            license = "CC0-1.0"
            [[languages]]
            name = "Frysk"
            locale = "fy"
            file = "fy.ftl"
        "#;
        scratch.plugin("fy", manifest, 0);
        scratch.plugin("empty", manifest, 0);
        fs::write(scratch.0.join("fy/fy.ftl"), "quit = Ofslute").unwrap();
        let plugins = discover(&scratch.0);
        assert_eq!(plugins.languages.len(), 1);
        assert_eq!(plugins.languages[0].name, "Frysk");
        assert_eq!(plugins.languages[0].locale, "fy");
        assert!(plugins.errors[0].reason.contains("missing"));
    }

    #[test]
    fn themes_come_with_their_colours() {
        let scratch = Scratch::new("themes");
        let manifest = r#"
            name = "nord"
            version = "1.0.0"
            license = "MIT"
            [[themes]]
            name = "Nord"
            file = "nord.toml"
        "#;
        scratch.plugin("nord", manifest, 0);
        fs::write(scratch.0.join("nord/nord.toml"), "background = \"#2e3440\"").unwrap();
        let plugins = discover(&scratch.0);
        assert_eq!(plugins.themes[0].name, "Nord");
        assert!(plugins.themes[0].colors.contains("#2e3440"));
    }

    #[test]
    fn a_board_needs_its_image() {
        let scratch = Scratch::new("boards");
        let manifest = r##"
            name = "oak"
            version = "1.0.0"
            license = "CC0-1.0"
            [[boards]]
            name = "Oak"
            image = "board.jpg"
            light = "#eeeeee"
            dark = "#333333"
        "##;
        scratch.plugin("oak", manifest, 0);
        assert!(discover(&scratch.0).errors[0].reason.contains("missing"));
        fs::write(scratch.0.join("oak/board.jpg"), "picture").unwrap();
        let plugins = discover(&scratch.0);
        assert_eq!(plugins.boards[0].name, "Oak");
        assert_eq!(plugins.boards[0].dark, "#333333");
    }

    #[test]
    fn no_plugins_folder_is_fine() {
        let plugins = discover(Path::new("does/not/exist"));
        assert!(plugins.sets.is_empty() && plugins.errors.is_empty());
    }
}
