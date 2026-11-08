//! Chessdesk's own updates: its latest release on GitHub, put in the place of the running
//! program. Chessdesk is portable, so it's replaced wherever it is, and the new version runs
//! from the next start. Blocking, like the rest of this crate.

use std::fs;
use std::path::{Path, PathBuf};

use semver::Version;
use serde::Deserialize;

use crate::download::{Asset, fetch, find_program, get, unpack};

const RELEASES: &str = "https://api.github.com/repos/chessdesk/chessdesk/releases/latest";
/// Where a download is unpacked: beside the program, so that moving it in is a rename on the
/// same drive, and a folder that can't be written to shows before anything is downloaded.
const STAGING: &str = ".chessdesk-update";
/// The app's folder inside the macOS archive.
const BUNDLE: &str = "Chessdesk.app";

/// A newer Chessdesk, with a build for this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// The release's page, which says what's new.
    pub page: String,
    asset: Asset,
}

#[derive(Debug, Deserialize)]
struct Latest {
    tag_name: String,
    html_url: String,
    assets: Vec<Asset>,
}

/// The archive the release workflow builds for this operating system and CPU. Releases name
/// CPUs as people know them, `arm64` and `x64`, not as Rust does.
fn asset_name(os: &str, arch: &str) -> Option<String> {
    let cpu = match arch {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        _ => return None,
    };
    match os {
        "windows" => Some(format!("chessdesk-windows-{cpu}.zip")),
        "linux" => Some(format!("chessdesk-linux-{cpu}.tar.gz")),
        "macos" => Some("chessdesk-macos.zip".to_owned()),
        _ => None,
    }
}

/// The latest release, if it's newer than `current` and built for this machine.
pub fn check(current: &str) -> Result<Option<Release>, String> {
    let body = get(RELEASES)?
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    newer(&body, current, std::env::consts::OS, std::env::consts::ARCH)
}

fn newer(body: &str, current: &str, os: &str, arch: &str) -> Result<Option<Release>, String> {
    let latest: Latest = serde_json::from_str(body).map_err(|error| error.to_string())?;
    let current = Version::parse(current).map_err(|error| error.to_string())?;
    let tag = latest.tag_name.trim_start_matches('v');
    let version = Version::parse(tag).map_err(|error| format!("release {tag}: {error}"))?;
    if version <= current {
        return Ok(None);
    }
    let Some(name) = asset_name(os, arch) else {
        return Ok(None);
    };
    let asset = latest.assets.into_iter().find(|asset| asset.name == name);
    Ok(asset.map(|asset| Release {
        version,
        page: latest.html_url,
        asset,
    }))
}

/// Downloads `release` and puts it in the place of the running program, which carries on as
/// it is. Returns the program that starts the new version.
pub fn install(release: &Release, progress: &mut dyn FnMut(f32)) -> Result<PathBuf, String> {
    let program = std::env::current_exe().map_err(|error| error.to_string())?;
    // On macOS the whole app goes: its Info.plist and signature belong to the program.
    let bundle = if cfg!(target_os = "macos") {
        app_bundle(&program)
    } else {
        None
    };
    let replaced = bundle.as_deref().unwrap_or(&program);
    let folder = replaced.parent().ok_or("the program has no folder")?;
    let staging = folder.join(STAGING);
    // What an update that broke off left behind.
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)
        .map_err(|error| format!("can't write to {}: {error}", folder.display()))?;
    let swapped = download_and_swap(release, &staging, bundle.as_deref(), progress);
    let _ = fs::remove_dir_all(&staging);
    swapped.map(|()| program)
}

fn download_and_swap(
    release: &Release,
    staging: &Path,
    bundle: Option<&Path>,
    progress: &mut dyn FnMut(f32),
) -> Result<(), String> {
    let archive = staging.join(&release.asset.name);
    fetch(&release.asset.browser_download_url, &archive, progress)?;
    unpack(&archive, staging)?;
    if let Some(bundle) = bundle {
        return replace_bundle(bundle, &staging.join(BUNDLE));
    }
    let new = find_program(staging, "chessdesk")
        .map_err(|error| error.to_string())?
        .ok_or("no Chessdesk in the download")?;
    self_replace::self_replace(new).map_err(|error| error.to_string())
}

/// The `.app` holding `program`, at `Chessdesk.app/Contents/MacOS/chessdesk`.
fn app_bundle(program: &Path) -> Option<PathBuf> {
    let bundle = program.parent()?.parent()?.parent()?;
    bundle
        .extension()
        .is_some_and(|extension| extension == "app")
        .then(|| bundle.to_owned())
}

/// Puts the app at `new` in the place of the one at `old`. macOS lets a running app's folder
/// move and go: the program runs on from what's loaded.
fn replace_bundle(old: &Path, new: &Path) -> Result<(), String> {
    if !new.is_dir() {
        return Err(format!("no {BUNDLE} in the download"));
    }
    let aside = old.with_extension("app.old");
    let _ = fs::remove_dir_all(&aside);
    fs::rename(old, &aside).map_err(|error| error.to_string())?;
    if let Err(error) = fs::rename(new, old) {
        let _ = fs::rename(&aside, old);
        return Err(error.to_string());
    }
    let _ = fs::remove_dir_all(&aside);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LATEST: &str = r#"{"tag_name":"v1.9.0",
        "html_url":"https://github.com/chessdesk/chessdesk/releases/tag/v1.9.0",
        "assets":[
            {"name":"chessdesk-linux-x64.tar.gz","browser_download_url":"https://example.com/l"},
            {"name":"chessdesk-macos.zip","browser_download_url":"https://example.com/m"},
            {"name":"chessdesk-windows-arm64.zip","browser_download_url":"https://example.com/w"}
        ]}"#;

    #[test]
    fn finds_a_newer_release_built_for_this_machine() {
        let release = newer(LATEST, "1.8.0", "windows", "aarch64")
            .unwrap()
            .unwrap();
        assert_eq!(release.version, Version::new(1, 9, 0));
        assert_eq!(release.asset.name, "chessdesk-windows-arm64.zip");
        assert!(release.page.ends_with("/v1.9.0"));
        let mac = newer(LATEST, "1.8.0", "macos", "x86_64").unwrap().unwrap();
        assert_eq!(mac.asset.name, "chessdesk-macos.zip");
    }

    #[test]
    fn the_same_or_an_older_release_is_no_update() {
        assert_eq!(newer(LATEST, "1.9.0", "linux", "x86_64"), Ok(None));
        assert_eq!(newer(LATEST, "1.10.0", "linux", "x86_64"), Ok(None));
    }

    #[test]
    fn a_release_without_this_machines_build_is_no_update() {
        assert_eq!(newer(LATEST, "1.8.0", "windows", "x86_64"), Ok(None));
        assert_eq!(newer(LATEST, "1.8.0", "freebsd", "x86_64"), Ok(None));
    }

    #[test]
    fn a_tag_that_is_no_version_says_so() {
        let odd = LATEST.replace("v1.9.0\"", "nightly\"");
        assert!(newer(&odd, "1.8.0", "linux", "x86_64").is_err());
    }

    #[test]
    fn the_bundle_is_found_from_its_program() {
        let program = Path::new("/Applications/Chessdesk.app/Contents/MacOS/chessdesk");
        assert_eq!(
            app_bundle(program),
            Some(PathBuf::from("/Applications/Chessdesk.app"))
        );
        assert_eq!(app_bundle(Path::new("/usr/local/bin/chessdesk")), None);
    }

    #[test]
    fn a_new_bundle_takes_the_old_ones_place() {
        let root = std::env::temp_dir().join(format!("chessdesk-bundle-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (old, new) = (
            root.join("Chessdesk.app"),
            root.join("staging/Chessdesk.app"),
        );
        fs::create_dir_all(&old).unwrap();
        fs::create_dir_all(&new).unwrap();
        fs::write(old.join("version"), "old").unwrap();
        fs::write(new.join("version"), "new").unwrap();

        replace_bundle(&old, &new).unwrap();
        let version = fs::read_to_string(old.join("version")).unwrap();
        let aside = root.join("Chessdesk.app.old").exists();
        let missing = replace_bundle(&old, &root.join("staging/Gone.app")).is_err();
        let kept = old.join("version").exists();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(version, "new");
        assert!(!aside);
        assert!(missing && kept);
    }
}
