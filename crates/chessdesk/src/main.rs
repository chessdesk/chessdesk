// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod board;
mod bundled;
mod export;
mod fonts;
mod i18n;
mod installs;
mod library;
mod new_game;
mod onboarding;
mod reset;
mod settings;
mod settings_view;
mod setup;
mod text_area;
mod trace;
mod ui;

use std::borrow::Cow;
use std::process::ExitCode;

use gpui::{
    App, Application, Bounds, SharedString, TitlebarOptions, WindowBounds, WindowOptions, point,
    prelude::*, px, size,
};

use app::Chessdesk;
use settings::Settings;

/// The interface font. Bundled, so that it works without being installed.
const FONT: &[u8] = include_bytes!("../assets/fonts/Geist.ttf");

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("reset") {
        return reset::run();
    }
    Application::new()
        .with_assets(ui::Assets)
        .run(move |cx: &mut App| {
            if let Err(error) = cx.text_system().add_fonts(vec![Cow::Borrowed(FONT)]) {
                eprintln!("could not load the bundled font: {error}");
            }
            let unpacked = Settings::plugins_dir().map(|root| bundled::install(&root));
            // Before the settings, which may name one of them as the interface font.
            let font_errors = fonts::load_fetched(cx);
            let (settings, mut errors) = Settings::load();
            errors.extend(font_errors);
            if let Some(Err(error)) = unpacked {
                errors.push(format!("could not unpack the bundled plugins: {error}"));
            }
            let (width, height) = settings.window_size;
            Settings::set(settings, cx);
            audio::init(cx);
            app::bind_keys(cx);
            text_area::bind_keys(cx);

            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(width, height), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(800.), px(520.))),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::new_static("Chessdesk")),
                    appears_transparent: true,
                    // The macOS window buttons, about 14px tall, centred in our bar.
                    traffic_light_position: Some(point(
                        px(12.),
                        (ui::TITLE_BAR_HEIGHT - px(14.)) / 2.,
                    )),
                }),
                ..Default::default()
            };
            let window = cx.open_window(options, |window, cx| {
                cx.new(|cx| Chessdesk::new(errors, window, cx))
            });
            if let Err(error) = window {
                panic!("could not open the window: {error}");
            }
            cx.activate(true);
        });
    ExitCode::SUCCESS
}
