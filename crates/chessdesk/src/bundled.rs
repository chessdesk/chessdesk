//! The plugins Chessdesk comes with, inside the binary so that it needs no installing.
//! They're unpacked into the plugins folder and from there load like any other plugin.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

macro_rules! bundle {
    ($($path:literal,)*) => {
        &[$(($path, include_bytes!(concat!("../../../plugins/", $path)))),*]
    };
}

const FILES: &[(&str, &[u8])] = bundle!(
    "cburnett/plugin.toml",
    "cburnett/svg/wK.svg",
    "cburnett/svg/wQ.svg",
    "cburnett/svg/wR.svg",
    "cburnett/svg/wB.svg",
    "cburnett/svg/wN.svg",
    "cburnett/svg/wP.svg",
    "cburnett/svg/bK.svg",
    "cburnett/svg/bQ.svg",
    "cburnett/svg/bR.svg",
    "cburnett/svg/bB.svg",
    "cburnett/svg/bN.svg",
    "cburnett/svg/bP.svg",
    "merida/plugin.toml",
    "merida/svg/wK.svg",
    "merida/svg/wQ.svg",
    "merida/svg/wR.svg",
    "merida/svg/wB.svg",
    "merida/svg/wN.svg",
    "merida/svg/wP.svg",
    "merida/svg/bK.svg",
    "merida/svg/bQ.svg",
    "merida/svg/bR.svg",
    "merida/svg/bB.svg",
    "merida/svg/bN.svg",
    "merida/svg/bP.svg",
    "wood-board/plugin.toml",
    "wood-board/board.jpg",
    "green-board/plugin.toml",
    "green-board/board.jpg",
    "marble-board/plugin.toml",
    "marble-board/board.jpg",
    "wood-sounds/plugin.toml",
    "wood-sounds/sounds/move.ogg",
    "wood-sounds/sounds/capture.ogg",
    "wood-sounds/sounds/castle.ogg",
    "wood-sounds/sounds/check.ogg",
    "wood-sounds/sounds/game-start.ogg",
    "wood-sounds/sounds/win.ogg",
    "wood-sounds/sounds/lose.ogg",
    "wood-sounds/sounds/draw.ogg",
    "wood-sounds/sounds/low-time.ogg",
    "soft-sounds/plugin.toml",
    "soft-sounds/sounds/move.ogg",
    "soft-sounds/sounds/capture.ogg",
    "soft-sounds/sounds/castle.ogg",
    "soft-sounds/sounds/check.ogg",
    "soft-sounds/sounds/game-start.ogg",
    "soft-sounds/sounds/win.ogg",
    "soft-sounds/sounds/lose.ogg",
    "soft-sounds/sounds/draw.ogg",
    "soft-sounds/sounds/low-time.ogg",
    "lang-de/plugin.toml",
    "lang-de/de.ftl",
    "lang-es/plugin.toml",
    "lang-es/es.ftl",
    "lang-fr/plugin.toml",
    "lang-fr/fr.ftl",
    "celeste/plugin.toml",
    "celeste/celeste.toml",
    "mariana/plugin.toml",
    "mariana/mariana.toml",
    "monokai/plugin.toml",
    "monokai/monokai.toml",
);

/// The set used if the chosen one can't be found, so that a board always has pieces.
const FALLBACK_SET: &str = "cburnett";

fn plugin_of(path: &str) -> &str {
    path.split('/').next().unwrap_or(path)
}

/// The version a manifest declares.
fn version_of(manifest: &str) -> Option<&str> {
    let line = manifest.lines().find(|line| line.starts_with("version"))?;
    line.split('"').nth(1)
}

/// Whether a bundled plugin should be written to `root`: when it isn't there yet, or there
/// in another version. One edited by hand at the same version stays as it is.
fn needs_writing(plugin: &str, root: &Path) -> bool {
    let folder = root.join(plugin);
    if !folder.exists() {
        return true;
    }
    let path = format!("{plugin}/plugin.toml");
    let bundled = FILES.iter().find(|(known, _)| *known == path);
    let bundled = bundled.map(|(_, bytes)| String::from_utf8_lossy(bytes));
    let installed = fs::read_to_string(folder.join("plugin.toml")).unwrap_or_default();
    match (
        bundled.as_deref().and_then(version_of),
        version_of(&installed),
    ) {
        (Some(bundled), Some(installed)) => bundled != installed,
        _ => false,
    }
}

/// The translations that come with Chessdesk, by path, for checking them against English.
#[cfg(test)]
pub fn translations() -> Vec<(&'static str, &'static str)> {
    FILES
        .iter()
        .filter(|(path, _)| {
            let extension = Path::new(path).extension();
            extension.is_some_and(|extension| extension.eq_ignore_ascii_case("ftl"))
        })
        .map(|(path, bytes)| (*path, std::str::from_utf8(bytes).unwrap()))
        .collect()
}

/// Unpacks the bundled plugins that are missing from `root` or out of date.
pub fn install(root: &Path) -> io::Result<()> {
    let plugins: BTreeSet<_> = FILES.iter().map(|(path, _)| plugin_of(path)).collect();
    let missing: BTreeSet<_> = plugins
        .into_iter()
        .filter(|plugin| needs_writing(plugin, root))
        .collect();
    for (path, bytes) in FILES {
        if missing.contains(plugin_of(path)) {
            let target = root.join(path);
            if let Some(folder) = target.parent() {
                fs::create_dir_all(folder)?;
            }
            fs::write(target, bytes)?;
        }
    }
    Ok(())
}

/// A file of the fallback set, by its name such as `wK.svg`.
pub fn fallback_piece(name: &str) -> Option<&'static [u8]> {
    let path = format!("{FALLBACK_SET}/svg/{name}");
    FILES
        .iter()
        .find(|(known, _)| *known == path)
        .map(|(_, bytes)| *bytes)
}

#[cfg(test)]
mod tests {
    use chessdesk_core::plugins::{all_pieces, discover, file_name};

    use super::*;

    #[test]
    fn unpacked_plugins_load_like_any_other() {
        let root = std::env::temp_dir().join(format!("chessdesk-bundled-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        install(&root).unwrap();
        let plugins = discover(&root);
        fs::remove_dir_all(&root).unwrap();
        assert!(plugins.errors.is_empty(), "{:?}", plugins.errors);
        let sets: Vec<_> = plugins.sets.iter().map(|set| set.name.as_str()).collect();
        assert_eq!(sets, ["Cburnett", "Merida"]);
        assert_eq!(plugins.themes.len(), 3);
        let boards: Vec<_> = plugins
            .boards
            .iter()
            .map(|board| board.name.as_str())
            .collect();
        assert_eq!(boards, ["Green", "Marble", "Wood"]);
        let packs: Vec<_> = plugins.sound_packs.iter().map(|pack| &pack.name).collect();
        assert_eq!(packs, ["Soft", "Wood"]);
        let languages: Vec<_> = plugins.languages.iter().map(|known| &known.name).collect();
        assert_eq!(languages, ["Deutsch", "Español", "Français"]);
    }

    #[test]
    fn an_edited_plugin_is_left_alone() {
        let root = std::env::temp_dir().join(format!("chessdesk-edited-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("merida")).unwrap();
        fs::write(root.join("merida/plugin.toml"), "mine").unwrap();
        install(&root).unwrap();
        let kept = fs::read_to_string(root.join("merida/plugin.toml")).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(kept, "mine");
    }

    #[test]
    fn an_older_bundled_plugin_is_replaced() {
        let root = std::env::temp_dir().join(format!("chessdesk-update-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        install(&root).unwrap();
        let manifest = root.join("wood-board/plugin.toml");
        let current = fs::read_to_string(&manifest).unwrap();
        let older = current.replace(version_of(&current).unwrap(), "0.1.0");
        fs::write(&manifest, older).unwrap();
        install(&root).unwrap();
        let updated = fs::read_to_string(&manifest).unwrap();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(updated, current);
    }

    #[test]
    fn the_fallback_has_every_piece() {
        for piece in all_pieces() {
            assert!(fallback_piece(&file_name(piece)).is_some());
        }
    }
}
