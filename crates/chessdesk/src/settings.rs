//! User settings: `config.toml` over built-in defaults, themes under that, and the
//! plugins that bring themes and piece sets.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use chessdesk_core::Mark;
use chessdesk_core::plugins::{
    self, BoardTheme, Font as LanguageFont, Language, PieceSet, Plugins, SoundPack, Theme,
    all_pieces, file_name,
};
use chessdesk_players::acp::AgentKind;
use gpui::{App, Font, FontFeatures, FontStyle, FontWeight, Global, Hsla, Image, ImageFormat};
use gpui::{Pixels, Rgba, SharedString, px};
use serde::Deserialize;
use shakmaty::{Color, Piece};
use toml::Table;

use crate::bundled;
use crate::i18n::{self, tr};

const DEFAULT_CONFIG: &str = include_str!("../assets/default_config.toml");
/// The table of the model each agent reviews and gives hints with.
const ANALYSIS_MODELS: &str = "analysis_models";
const BUILT_IN_THEMES: [(&str, &str); 2] = [
    ("dark", include_str!("../assets/themes/dark.toml")),
    ("light", include_str!("../assets/themes/light.toml")),
];
/// The size pieces are drawn from. Sets declare anything from 45 pixels to 50 mm, and
/// GPUI rasterises an SVG once at its own size, which would blur on a large board.
const PIECE_SOURCE_SIZE: u32 = 256;

/// `config.toml` as written, merged over [`DEFAULT_CONFIG`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    theme: String,
    piece_set: String,
    board: String,
    language: String,
    sound_pack: String,
    sound_volume: f32,
    ui_font_family: String,
    ui_font_size: f32,
    ui_font_weight: f32,
    corner_radius: f32,
    popover_radius_extra: f32,
    history_width: f32,
    panel_width: f32,
    log_height: f32,
    board_size: f32,
    animate_moves: bool,
    opponent_delay: f32,
    window_width: f32,
    window_height: f32,
    stockfish: String,
    lc0: String,
    maia_weights: String,
    /// By agent id; `""` is the agent's own default.
    #[serde(default)]
    analysis_models: BTreeMap<String, String>,
    #[serde(default)]
    colors: Table,
}

/// The colours Chessdesk uses of a theme file. A theme may hold more: the format is
/// shared with code editors, whose syntax colours don't matter here.
#[derive(Deserialize)]
struct ThemeFile {
    background: Rgba,
    accent: Rgba,
    popover: Rgba,
    popover_border: Rgba,
    text: Rgba,
    muted: Rgba,
    caret: Rgba,
    callout: Rgba,
    ansi: [Rgba; 16],
    /// Without these, the board is made from `accent` and `muted`.
    board_light: Option<Rgba>,
    board_dark: Option<Rgba>,
}

#[derive(Clone)]
pub struct Colors {
    pub background: Hsla,
    pub accent: Hsla,
    pub popover: Hsla,
    pub popover_border: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub caret: Hsla,
    pub callout: Hsla,
    pub red: Hsla,
    pub board_light: Hsla,
    pub board_dark: Hsla,
    green: Hsla,
    yellow: Hsla,
    blue: Hsla,
}

impl Colors {
    /// How a marking on the board looks.
    pub fn mark(&self, mark: Mark) -> Hsla {
        match mark {
            Mark::Green => self.green,
            Mark::Red => self.red,
            Mark::Yellow => self.yellow,
            Mark::Blue => self.blue,
        }
    }
}

impl From<ThemeFile> for Colors {
    fn from(theme: ThemeFile) -> Self {
        let accent: Hsla = theme.accent.into();
        let muted: Hsla = theme.muted.into();
        // A board of the theme's own: its accent, and the accent drawn towards the muted
        // colour. Which of them is lighter depends on the theme: on a light one it's the accent.
        let toward_muted = accent.blend(muted.opacity(0.55));
        let (made_light, made_dark) = if toward_muted.l >= accent.l {
            (toward_muted, accent)
        } else {
            (accent, toward_muted)
        };
        Self {
            background: theme.background.into(),
            accent,
            popover: theme.popover.into(),
            popover_border: theme.popover_border.into(),
            text: theme.text.into(),
            muted,
            caret: theme.caret.into(),
            callout: theme.callout.into(),
            red: theme.ansi[1].into(),
            green: theme.ansi[2].into(),
            yellow: theme.ansi[3].into(),
            blue: theme.ansi[4].into(),
            board_light: theme.board_light.map_or(made_light, Hsla::from),
            board_dark: theme.board_dark.map_or(made_dark, Hsla::from),
        }
    }
}

/// The name of the board that follows the colour theme.
pub const THEME_BOARD: &str = "Theme";
/// The name of no sound pack at all.
pub const SOUND_OFF: &str = "Off";

fn board_look(board: &BoardTheme) -> Result<BoardLook, String> {
    let bytes = fs::read(&board.image).map_err(|error| error.to_string())?;
    let is_png = board
        .image
        .extension()
        .is_some_and(|extension| extension == "png");
    let format = if is_png {
        ImageFormat::Png
    } else {
        ImageFormat::Jpeg
    };
    let color = |hex: &str| Rgba::try_from(hex).map_err(|error| error.to_string());
    Ok(BoardLook {
        light: color(&board.light)?.into(),
        dark: color(&board.dark)?.into(),
        image: Some(Arc::new(Image::from_bytes(format, bytes))),
    })
}

/// Gives an SVG the size to rasterise at, in place of any it declares. Its `viewBox`
/// keeps the drawing's proportions.
fn with_source_size(svg: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(svg);
    let Some(start) = text.find("<svg") else {
        return svg.to_vec();
    };
    let Some(length) = text[start..].find('>') else {
        return svg.to_vec();
    };
    let end = start + length;
    // Each pair of quotes closes one attribute, and values may hold anything else.
    let pieces: Vec<_> = text[start + 4..end].split_inclusive('"').collect();
    let attributes: String = pieces
        .chunks(2)
        .map(<[&str]>::concat)
        .filter(|attribute| {
            let name = attribute.split('=').next().unwrap_or("").trim();
            name != "width" && name != "height"
        })
        .collect();
    let size = PIECE_SOURCE_SIZE;
    let (before, after) = (&text[..start], &text[end..]);
    format!("{before}<svg width=\"{size}\" height=\"{size}\"{attributes}{after}").into_bytes()
}

fn piece_image(svg: &[u8]) -> Arc<Image> {
    Arc::new(Image::from_bytes(ImageFormat::Svg, with_source_size(svg)))
}

/// How the board looks: in the colour theme's squares, or a board plugin's picture.
#[derive(Clone)]
pub struct BoardLook {
    pub light: Hsla,
    pub dark: Hsla,
    pub image: Option<Arc<Image>>,
}

/// The 12 pieces of a set, ready for GPUI's `img`, which unlike `svg` keeps their colours.
#[derive(Clone)]
pub struct PieceImages(Vec<(Piece, Arc<Image>)>);

impl PieceImages {
    fn fallback() -> Self {
        let image = |piece| piece_image(bundled::fallback_piece(&file_name(piece)).unwrap_or(&[]));
        Self(all_pieces().map(|piece| (piece, image(piece))).collect())
    }

    fn from_set(set: &PieceSet) -> Result<Self, String> {
        let mut images = Vec::new();
        for piece in all_pieces() {
            let path = set.dir.join(file_name(piece));
            let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            images.push((piece, piece_image(&bytes)));
        }
        Ok(Self(images))
    }

    pub fn get(&self, piece: Piece) -> Arc<Image> {
        self.0
            .iter()
            .find(|(known, _)| *known == piece)
            .map(|(_, image)| image.clone())
            .expect("a set holds every piece")
    }
}

/// Resolved settings, ready for rendering.
#[derive(Clone)]
pub struct Settings {
    pub colors: Colors,
    pub ui_font: Font,
    pub ui_font_size: Pixels,
    pub corner_radius: Pixels,
    popover_radius_extra: Pixels,
    pub history_width: Pixels,
    pub panel_width: Pixels,
    pub log_height: Pixels,
    pub board_size: f32,
    pub animate_moves: bool,
    /// The least time an opponent's move takes, so that a quick one doesn't land at once.
    pub opponent_delay: Duration,
    pub window_size: (Pixels, Pixels),
    pub stockfish: String,
    pub lc0: String,
    pub maia_weights: PathBuf,
    /// The model each agent reviews and gives hints with, by agent id; `""` is its default.
    analysis_models: BTreeMap<String, String>,
    pub pieces: PieceImages,
    pub board: BoardLook,
    pub board_name: String,
    pub theme: String,
    pub piece_set: String,
    pub language: String,
    /// The font the language asks for, while it hasn't been fetched.
    pub missing_font: Option<LanguageFont>,
    pub sound_pack: String,
    /// Where the chosen pack's sounds are, if it's on.
    pub sound_pack_dir: Option<PathBuf>,
    pub sound_volume: f32,
    pub ui_font_px: f32,
    pub corner_radius_px: f32,
    /// Names for the settings page.
    pub theme_names: Vec<String>,
    pub piece_set_names: Vec<String>,
    pub board_names: Vec<String>,
    pub language_names: Vec<String>,
    pub sound_pack_names: Vec<String>,
    /// What the plugins bring, for the settings page.
    pub piece_sets: Vec<PieceSet>,
    pub themes: Vec<Theme>,
    pub board_themes: Vec<BoardTheme>,
    pub sound_packs: Vec<SoundPack>,
    pub languages: Vec<Language>,
}

struct Current(Rc<Settings>);

impl Global for Current {}

fn merge(base: &mut Table, over: Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(base)), toml::Value::Table(over)) => merge(base, over),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

fn parse_theme(text: &str, overrides: &Table) -> Result<Colors, String> {
    let mut table: Table = toml::from_str(text).map_err(|error| error.to_string())?;
    merge(&mut table, overrides.clone());
    let theme: ThemeFile = table.try_into().map_err(|error| error.to_string())?;
    Ok(theme.into())
}

/// The theme's TOML: built in, from a plugin, or from `<config dir>/themes`.
fn theme_text(name: &str, plugins: &Plugins) -> Option<String> {
    let built_in = BUILT_IN_THEMES.iter().find(|(known, _)| *known == name);
    if let Some((_, text)) = built_in {
        return Some((*text).to_owned());
    }
    let from_plugin = plugins.themes.iter().find(|theme| theme.name == name);
    from_plugin.map(|theme| theme.colors.clone()).or_else(|| {
        let path = Settings::config_dir()?
            .join("themes")
            .join(format!("{name}.toml"));
        fs::read_to_string(path).ok()
    })
}

impl Settings {
    pub fn get(cx: &App) -> Rc<Self> {
        cx.global::<Current>().0.clone()
    }

    pub fn set(settings: Self, cx: &mut App) {
        cx.set_global(Current(Rc::new(settings)));
    }

    /// Large surfaces look less rounded than small ones at the same radius.
    pub fn popover_radius(&self) -> Pixels {
        self.corner_radius + self.popover_radius_extra
    }

    /// Dialogs are larger still, so they take the extra twice.
    pub fn dialog_radius(&self) -> Pixels {
        self.popover_radius() + self.popover_radius_extra
    }

    /// Where plugins live: copying a folder here installs one.
    pub fn plugins_dir() -> Option<PathBuf> {
        Some(Self::config_dir()?.join("plugins"))
    }

    /// `CHESSDESK_CONFIG_DIR`, or else the platform's per-user config folder.
    pub fn config_dir() -> Option<PathBuf> {
        std::env::var_os("CHESSDESK_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|| Some(dirs::config_dir()?.join("chessdesk")))
    }

    pub fn config_path() -> Option<PathBuf> {
        Some(Self::config_dir()?.join("config.toml"))
    }

    /// Whether Chessdesk runs for the first time here: nothing has been set, not even once.
    pub fn is_first_run() -> bool {
        Self::config_path().is_some_and(|path| !path.exists())
    }

    /// The config file, created from the defaults if there is none yet, for opening.
    pub fn config_file() -> Result<PathBuf, String> {
        let path = Self::config_path().ok_or("no config folder on this system")?;
        if !path.exists() {
            if let Some(folder) = path.parent() {
                fs::create_dir_all(folder).map_err(|error| error.to_string())?;
            }
            fs::write(&path, DEFAULT_CONFIG).map_err(|error| error.to_string())?;
        }
        Ok(path)
    }

    /// Where installed agent adapters go.
    pub fn adapters_dir() -> Option<PathBuf> {
        Some(Self::data_dir()?.join("adapters"))
    }

    /// Where downloaded fonts go.
    pub fn fonts_dir() -> Option<PathBuf> {
        Some(Self::data_dir()?.join("fonts"))
    }

    /// The interface font: the one the language asks for once it's here, else the one set.
    /// With it, the font still to fetch for the language, if any.
    fn resolve_font(set: &str, wanted: Option<LanguageFont>) -> (String, Option<LanguageFont>) {
        match wanted {
            Some(font) if !Self::has_font(&font) => (set.to_owned(), Some(font)),
            Some(font) => (font.family, None),
            None => (set.to_owned(), None),
        }
    }

    /// Whether `font` has been fetched.
    pub fn has_font(font: &LanguageFont) -> bool {
        Self::fonts_dir().is_some_and(|folder| folder.join(font.file_name()).is_file())
    }

    /// Where downloaded engines go.
    pub fn engines_dir() -> Option<PathBuf> {
        Some(Self::data_dir()?.join("engines"))
    }

    /// Where games and remembered choices live.
    pub fn data_dir() -> Option<PathBuf> {
        std::env::var_os("CHESSDESK_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|| Some(dirs::data_local_dir()?.join("chessdesk")))
    }

    /// Reads everything, falling back to defaults. Returns what was wrong, for the ribbon.
    pub fn load() -> (Self, Vec<String>) {
        let mut errors = Vec::new();
        let mut table: Table = toml::from_str(DEFAULT_CONFIG).expect("the defaults parse");
        let defaults = table.clone();
        if let Some(path) = Self::config_path()
            && let Ok(text) = fs::read_to_string(&path)
        {
            match toml::from_str(&text) {
                Ok(user) => merge(&mut table, user),
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            }
        }
        let config: Config = table.try_into().unwrap_or_else(|error| {
            errors.push(format!("config.toml: {error}"));
            defaults.try_into().expect("the defaults are complete")
        });

        let plugins = Self::plugins_dir()
            .map(|dir| plugins::discover(&dir))
            .unwrap_or_default();
        errors.extend(plugins.errors.iter().map(ToString::to_string));
        let colors = Self::resolve_theme(&config, &plugins, &mut errors);
        let pieces = Self::resolve_pieces(&config, &plugins, &mut errors);
        let sound_pack_dir = Self::resolve_sounds(&config, &plugins, &mut errors);
        if let Err(error) = i18n::use_language(&config.language, &plugins.languages) {
            errors.push(format!("language {}: {error}", config.language));
        }
        let wanted_font = i18n::chosen(&config.language, &plugins.languages)
            .ok()
            .flatten()
            .and_then(|language| language.font.clone());
        let (font_family, missing_font) = Self::resolve_font(&config.ui_font_family, wanted_font);

        let mut theme_names: Vec<_> = BUILT_IN_THEMES
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect();
        theme_names.extend(plugins.themes.iter().map(|theme| theme.name.clone()));
        let piece_set_names = plugins.sets.iter().map(|set| set.name.clone()).collect();

        let board = Self::resolve_board(&config, &plugins, &colors, &mut errors);
        let mut board_names = vec![THEME_BOARD.to_owned()];
        board_names.extend(plugins.boards.iter().map(|board| board.name.clone()));
        let mut language_names = vec![i18n::SYSTEM.to_owned(), i18n::ENGLISH.to_owned()];
        language_names.extend(plugins.languages.iter().map(|known| known.name.clone()));
        let mut sound_pack_names = vec![SOUND_OFF.to_owned()];
        sound_pack_names.extend(plugins.sound_packs.iter().map(|pack| pack.name.clone()));

        let settings = Self {
            colors,
            ui_font: Font {
                family: font_family.into(),
                features: FontFeatures::default(),
                fallbacks: None,
                weight: FontWeight(config.ui_font_weight),
                style: FontStyle::Normal,
            },
            ui_font_size: px(config.ui_font_size),
            corner_radius: px(config.corner_radius),
            popover_radius_extra: px(config.popover_radius_extra),
            history_width: px(config.history_width),
            panel_width: px(config.panel_width),
            log_height: px(config.log_height),
            board_size: config.board_size.clamp(0.5, 1.),
            animate_moves: config.animate_moves,
            // `nan` is valid TOML; it comes out as no delay.
            opponent_delay: Duration::try_from_secs_f32(config.opponent_delay.clamp(0., 5.))
                .unwrap_or_default(),
            window_size: (px(config.window_width), px(config.window_height)),
            stockfish: config.stockfish,
            lc0: config.lc0,
            maia_weights: PathBuf::from(config.maia_weights),
            analysis_models: config.analysis_models,
            pieces,
            board,
            board_name: config.board,
            theme: config.theme,
            piece_set: config.piece_set,
            language: config.language,
            missing_font,
            sound_pack: config.sound_pack,
            sound_pack_dir,
            sound_volume: config.sound_volume.clamp(0., 1.),
            ui_font_px: config.ui_font_size,
            corner_radius_px: config.corner_radius,
            theme_names,
            piece_set_names,
            board_names,
            language_names,
            sound_pack_names,
            piece_sets: plugins.sets,
            themes: plugins.themes,
            board_themes: plugins.boards,
            sound_packs: plugins.sound_packs,
            languages: plugins.languages,
        };
        (settings, errors)
    }

    fn resolve_theme(config: &Config, plugins: &Plugins, errors: &mut Vec<String>) -> Colors {
        let parsed = theme_text(&config.theme, plugins)
            .ok_or_else(|| "no such theme".to_owned())
            .and_then(|text| parse_theme(&text, &config.colors));
        parsed.unwrap_or_else(|error| {
            errors.push(format!("theme {}: {error}", config.theme));
            parse_theme(BUILT_IN_THEMES[0].1, &Table::new()).expect("the built-in theme parses")
        })
    }

    fn resolve_board(
        config: &Config,
        plugins: &Plugins,
        colors: &Colors,
        errors: &mut Vec<String>,
    ) -> BoardLook {
        let plain = BoardLook {
            light: colors.board_light,
            dark: colors.board_dark,
            image: None,
        };
        if config.board == THEME_BOARD {
            return plain;
        }
        let chosen = plugins
            .boards
            .iter()
            .find(|board| board.name == config.board);
        let look = chosen
            .ok_or_else(|| "no such board".to_owned())
            .and_then(board_look);
        look.unwrap_or_else(|error| {
            errors.push(format!("board {}: {error}", config.board));
            plain
        })
    }

    fn resolve_sounds(
        config: &Config,
        plugins: &Plugins,
        errors: &mut Vec<String>,
    ) -> Option<PathBuf> {
        if config.sound_pack == SOUND_OFF {
            return None;
        }
        let packs = &plugins.sound_packs;
        let chosen = packs.iter().find(|pack| pack.name == config.sound_pack);
        if chosen.is_none() {
            errors.push(format!("sound pack {}: no such pack", config.sound_pack));
        }
        chosen.map(|pack| pack.dir.clone())
    }

    fn resolve_pieces(config: &Config, plugins: &Plugins, errors: &mut Vec<String>) -> PieceImages {
        let set = plugins.sets.iter().find(|set| set.name == config.piece_set);
        let images = set
            .ok_or_else(|| "no such set".to_owned())
            .and_then(PieceImages::from_set);
        images.unwrap_or_else(|error| {
            errors.push(format!("piece set {}: {error}", config.piece_set));
            PieceImages::fallback()
        })
    }

    /// Writes one setting to `config.toml`, keeping the rest of the file as it is.
    pub fn store(key: &str, value: impl Into<toml_edit::Value>) -> Result<(), String> {
        Self::edit(|document| document[key] = toml_edit::value(value.into()))
    }

    /// The model `kind` reviews and gives hints with, or `None` for the agent's own default.
    pub fn analysis_model(&self, kind: AgentKind) -> Option<String> {
        self.analysis_models
            .get(kind.id())
            .filter(|model| !model.is_empty())
            .cloned()
    }

    /// Keeps the model `kind` reviews and gives hints with: `None` is the agent's own default,
    /// written as `""` so that the shipped default doesn't come back in its place.
    pub fn store_analysis_model(kind: AgentKind, model: Option<&str>) -> Result<(), String> {
        Self::edit(|document| {
            let table = document
                .entry(ANALYSIS_MODELS)
                .or_insert_with(toml_edit::table)
                .as_table_like_mut();
            if let Some(table) = table {
                table.insert(kind.id(), toml_edit::value(model.unwrap_or_default()));
            }
        })
    }

    /// Changes `config.toml` as `change` says, keeping the rest of the file as it was written.
    fn edit(change: impl FnOnce(&mut toml_edit::DocumentMut)) -> Result<(), String> {
        let path = Self::config_path().ok_or("no config folder on this system")?;
        let text = fs::read_to_string(&path).unwrap_or_default();
        let mut document: toml_edit::DocumentMut = text
            .parse()
            .map_err(|error: toml_edit::TomlError| error.to_string())?;
        change(&mut document);
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder).map_err(|error| error.to_string())?;
        }
        fs::write(&path, document.to_string()).map_err(|error| error.to_string())
    }
}

/// The colour pieces of this side are named by, for labels.
pub fn side_name(color: Color) -> SharedString {
    tr(if color.is_white() {
        "side-white"
    } else {
        "side-black"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_themes_parse() {
        for (_, text) in BUILT_IN_THEMES {
            parse_theme(text, &Table::new()).unwrap();
        }
    }

    #[test]
    fn the_defaults_are_complete() {
        let table: Table = toml::from_str(DEFAULT_CONFIG).unwrap();
        let _: Config = table.try_into().unwrap();
    }

    #[test]
    fn the_analysis_models_name_real_agents_and_a_pick_overrides_them() {
        let mut table: Table = toml::from_str(DEFAULT_CONFIG).unwrap();
        let config: Config = table.clone().try_into().unwrap();
        assert_eq!(config.analysis_models["claude-code"], "sonnet");
        assert!(
            config
                .analysis_models
                .keys()
                .all(|id| AgentKind::from_id(id).is_some())
        );
        // The agent's own default, picked over the shipped one, is kept as "".
        let user: Table = toml::from_str("[analysis_models]\nclaude-code = \"\"").unwrap();
        merge(&mut table, user);
        let config: Config = table.try_into().unwrap();
        assert_eq!(config.analysis_models["claude-code"], "");
    }

    #[test]
    fn a_themes_own_board_has_its_light_squares_lighter() {
        for (name, text) in BUILT_IN_THEMES {
            let colors = parse_theme(text, &Table::new()).unwrap();
            assert!(colors.board_light.l > colors.board_dark.l, "{name}");
        }
    }

    #[test]
    fn colour_overrides_win() {
        let overrides: Table = toml::from_str("board_dark = \"#112233\"").unwrap();
        let plain = parse_theme(BUILT_IN_THEMES[0].1, &Table::new()).unwrap();
        let changed = parse_theme(BUILT_IN_THEMES[0].1, &overrides).unwrap();
        assert_ne!(plain.board_dark, changed.board_dark);
    }

    #[test]
    fn pieces_get_one_size_whatever_they_declare() {
        let declared = br#"<svg width="50mm" xmlns="x" height="50mm" viewBox="0 0 50 50"><g width="9"/></svg>"#;
        assert_eq!(
            String::from_utf8(with_source_size(declared)).unwrap(),
            r#"<svg width="256" height="256" xmlns="x" viewBox="0 0 50 50"><g width="9"/></svg>"#
        );
        let bare = with_source_size(br#"<svg viewBox="0 0 45 45"/>"#);
        assert!(
            String::from_utf8(bare)
                .unwrap()
                .starts_with(r#"<svg width="256""#)
        );
    }
}
