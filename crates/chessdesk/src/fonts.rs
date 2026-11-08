//! Fonts fetched for languages, handed to GPUI so that the interface can use them.

use std::borrow::Cow;
use std::fs;
use std::path::Path;

use gpui::App;

use crate::settings::Settings;

/// Adds every fetched font. Returns what couldn't be read, for the error bar.
pub fn load_fetched(cx: &App) -> Vec<String> {
    let Some(entries) = Settings::fonts_dir().and_then(|folder| fs::read_dir(folder).ok()) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "ttf" || extension == "otf")
        })
        .filter_map(|path| load(&path, cx).err())
        .collect()
}

/// Adds the font in `path`.
pub fn load(path: &Path, cx: &App) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    cx.text_system()
        .add_fonts(vec![Cow::Owned(bytes)])
        .map_err(|error| format!("{}: {error}", path.display()))
}
