// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod board;
mod bundled;
mod combobox;
mod export;
mod fonts;
mod i18n;
mod installs;
mod library;
mod models;
mod new_game;
mod onboarding;
mod reset;
mod scenario;
mod screenshot;
mod script;
mod settings;
mod settings_view;
mod setup;
mod text_area;
mod trace;
mod ui;

use std::borrow::Cow;
use std::path::Path;
use std::process::ExitCode;

use gpui::{
    App, Application, Bounds, SharedString, TitlebarOptions, WindowBounds, WindowOptions, point,
    prelude::*, px, size,
};

use app::Chessdesk;
use settings::Settings;

/// The interface font. Bundled, so that it works without being installed.
const FONT: &[u8] = include_bytes!("../assets/fonts/Geist.ttf");

/// Reads the script `chessdesk script <file>` names.
fn read_script(path: Option<String>) -> Result<Vec<script::Step>, String> {
    let path = path.ok_or("usage: chessdesk script <file>")?;
    let path = Path::new(&path);
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    script::parse(&text, path.parent().unwrap_or(Path::new("")))
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let script = match args.next().as_deref() {
        Some("reset") => return reset::run(),
        Some("script") => match read_script(args.next()) {
            Ok(steps) => Some(steps),
            Err(error) => {
                eprintln!("script: {error}");
                return ExitCode::from(2);
            }
        },
        _ => None,
    };
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
            let scripted = script.is_some();
            let window = cx.open_window(options, |window, cx| {
                cx.new(|cx| Chessdesk::new(errors, scripted, window, cx))
            });
            let window = match window {
                Ok(window) => window,
                Err(error) => panic!("could not open the window: {error}"),
            };
            if let Some(steps) = script {
                let started = window.update(cx, |_, window, cx| {
                    Chessdesk::run_script(steps, window, cx);
                });
                if let Err(error) = started {
                    panic!("could not start the script: {error}");
                }
            }
            cx.activate(true);
        });
    ExitCode::SUCCESS
}
