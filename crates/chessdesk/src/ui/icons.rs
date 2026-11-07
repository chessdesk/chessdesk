//! The interface's icons: Lucide (ISC, see `assets/icons/LICENSE`) and Chessdesk's logo,
//! tinted by the theme and served to GPUI's `svg` element.

use std::borrow::Cow;
use std::sync::{Arc, LazyLock};

use gpui::{
    AssetSource, Hsla, Image, ImageFormat, Img, Pixels, Result, SharedString, Svg, img, prelude::*,
    svg,
};
use shakmaty::Role;

/// The logotype's width for its height.
const LOGOTYPE_WIDTH: f32 = 211. / 43.;

macro_rules! icons {
    ($($name:ident => $file:literal),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Icon {
            $($name),*
        }

        impl Icon {
            const ALL: &[(Self, &str, &[u8])] = &[
                $((
                    Self::$name,
                    concat!("icons/", $file, ".svg"),
                    include_bytes!(concat!("../../assets/icons/", $file, ".svg")),
                )),*
            ];
        }
    };
}

icons! {
    ChevronDown => "chevron-down",
    ChevronLeft => "chevron-left",
    ChevronRight => "chevron-right",
    ChessBishop => "chess-bishop",
    ChessKing => "chess-king",
    ChessKnight => "chess-knight",
    ChessPawn => "chess-pawn",
    ChessQueen => "chess-queen",
    ChessRook => "chess-rook",
    Close => "x",
    Logo => "logo",
    Logotype => "logotype",
    Spinner => "loader-circle",
    Warning => "triangle-alert",
    // The window controls' own, drawn lighter than the rest.
    WindowMinimize => "window-minimize",
    WindowMaximize => "window-maximize",
    WindowRestore => "window-restore",
    WindowClose => "window-close",
}

impl Icon {
    /// The icon of a piece type.
    pub fn of_role(role: Role) -> Self {
        match role {
            Role::Pawn => Self::ChessPawn,
            Role::Knight => Self::ChessKnight,
            Role::Bishop => Self::ChessBishop,
            Role::Rook => Self::ChessRook,
            Role::Queen => Self::ChessQueen,
            Role::King => Self::ChessKing,
        }
    }

    fn path(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(icon, ..)| *icon == self)
            .map_or("", |(_, path, _)| path)
    }
}

/// Serves the interface icons to GPUI's `svg` element.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(Icon::ALL
            .iter()
            .find(|(_, known, _)| *known == path)
            .map(|(.., bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, _: &str) -> Result<Vec<SharedString>> {
        Ok(Icon::ALL
            .iter()
            .map(|(_, path, _)| SharedString::new_static(path))
            .collect())
    }
}

/// A project's own logo, in its own colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Logo {
    Stockfish,
    ClaudeCode,
    Codex,
}

fn logo_image(format: ImageFormat, bytes: &[u8]) -> Arc<Image> {
    Arc::new(Image::from_bytes(format, bytes.to_vec()))
}

static STOCKFISH: LazyLock<Arc<Image>> = LazyLock::new(|| {
    logo_image(
        ImageFormat::Png,
        include_bytes!("../../assets/stockfish.png"),
    )
});
static CLAUDE_CODE: LazyLock<Arc<Image>> = LazyLock::new(|| {
    logo_image(
        ImageFormat::Png,
        include_bytes!("../../assets/claude-code.png"),
    )
});
static CODEX: LazyLock<Arc<Image>> =
    LazyLock::new(|| logo_image(ImageFormat::Png, include_bytes!("../../assets/codex.png")));

pub fn logo(logo: Logo, size: Pixels) -> Img {
    let image = match logo {
        Logo::Stockfish => STOCKFISH.clone(),
        Logo::ClaudeCode => CLAUDE_CODE.clone(),
        Logo::Codex => CODEX.clone(),
    };
    img(image).flex_none().size(size)
}

/// The name as drawn, crown and all, as wide as its proportions make it.
pub fn logotype(height: Pixels, color: Hsla) -> Svg {
    icon(Icon::Logotype, height, color).w(height * LOGOTYPE_WIDTH)
}

pub fn icon(icon: Icon, size: Pixels, color: Hsla) -> Svg {
    svg()
        .path(icon.path())
        .flex_none()
        .size(size)
        .text_color(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_served() {
        for (icon, path, bytes) in Icon::ALL {
            assert_eq!(icon.path(), *path);
            assert!(Assets.load(path).unwrap().is_some());
            assert!(bytes.starts_with(b"<svg"));
        }
    }

    #[test]
    fn unknown_assets_are_absent() {
        assert!(Assets.load("icons/nope.svg").unwrap().is_none());
    }
}
