//! `chessdesk reset`: for developers, everything Chessdesk keeps on this machine moved to
//! the trash, so that the next start is a first start. Settings, plugins, adapters, engines,
//! fonts and saved games all go, and only after typing `reset` to confirm.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::settings::Settings;

/// What has to be typed to go ahead.
const CONFIRMATION: &str = "reset";

/// Runs the command in the terminal it was started from.
pub fn run() -> ExitCode {
    let folders = folders();
    if folders.is_empty() {
        println!("Nothing to reset: Chessdesk keeps nothing on this machine yet.");
        return ExitCode::SUCCESS;
    }
    println!("This moves everything Chessdesk keeps here to the trash:");
    for folder in &folders {
        println!("\n  {}", folder.display());
        for name in contents(folder) {
            println!("    {name}");
        }
    }
    print!("\nClose Chessdesk first. Type {CONFIRMATION} to go ahead: ");
    let _ = io::stdout().flush();
    // No terminal, as for a release build started on Windows, reads as no answer.
    let mut answer = String::new();
    if io::stdin().lock().read_line(&mut answer).is_err() || answer.trim() != CONFIRMATION {
        println!("\nNothing was changed.");
        return ExitCode::FAILURE;
    }
    match trash::delete_all(&folders) {
        Ok(()) => {
            println!("Done. The next start is a first start.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Could not move everything to the trash: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The config and data folders that exist, once each: they're the same folder when
/// `CHESSDESK_CONFIG_DIR` is set.
fn folders() -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = [Settings::config_dir(), Settings::data_dir()]
        .into_iter()
        .flatten()
        .filter(|folder| folder.is_dir())
        .collect();
    folders.dedup();
    folders
}

/// The names in `folder`, sorted, so that it's clear what goes.
fn contents(folder: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}
