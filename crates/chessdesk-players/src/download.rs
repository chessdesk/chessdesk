//! Fetching engines from their official GitHub releases, so that nobody has to hunt for
//! the right build, fonts too large to bundle, and Chessdesk's own releases. Blocking, like the rest of this crate.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use ureq::http::Response;
use ureq::{Agent, Body};

use crate::notes;

const STOCKFISH_RELEASES: &str =
    "https://api.github.com/repos/official-stockfish/Stockfish/releases/latest";
const LC0_RELEASES: &str = "https://api.github.com/repos/LeelaChessZero/lc0/releases/latest";
const MAIA_WEIGHTS: &str = "https://github.com/CSSLab/maia-chess/releases/download/v1.0";
const SHADOWCHESS_RELEASES: &str =
    "https://api.github.com/repos/chessdesk/shadowchess/releases/latest";
/// The release asset with all the levels' weights.
const SHADOWCHESS_WEIGHTS: &str = "shadowchess.safetensors";
/// The networks there are, by the rating they imitate.
pub const MAIA_LEVELS: [u32; 9] = [1100, 1200, 1300, 1400, 1500, 1600, 1700, 1800, 1900];
/// Beside an engine Chessdesk installed: the release it came from, to tell when there's a newer.
const RELEASE_FILE: &str = "release.txt";
/// GitHub's API turns away requests without one.
const USER_AGENT: &str = "Chessdesk";
const CHUNK: usize = 64 * 1024;
/// How often a request is tried while the server is struggling, and the first pause between
/// tries, which grows with each.
const ATTEMPTS: u32 = 3;
const RETRY_AFTER: Duration = Duration::from_secs(2);
/// How long a server gets to answer at all. Not how long a download may take: large files
/// on a slow line take minutes.
const ANSWER_WITHIN: Duration = Duration::from_secs(20);

/// Gives up on a server that doesn't answer, which would otherwise hang a download forever.
static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Agent::config_builder()
        .timeout_connect(Some(ANSWER_WITHIN))
        .timeout_recv_response(Some(ANSWER_WITHIN))
        .build()
        .into()
});

/// Fetches the font at `url` as `target`. It only takes that name once whole, so that a
/// broken download never passes for the font.
pub fn install_font(url: &str, target: &Path, progress: &mut dyn FnMut(f32)) -> Result<(), String> {
    if let Some(folder) = target.parent() {
        fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    }
    let partial = target.with_extension("part");
    fetch(url, &partial, progress)?;
    fs::rename(&partial, target).map_err(|error| error.to_string())
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    /// The release notes, in Markdown; GitHub sends `null` for none.
    #[serde(default)]
    body: Option<String>,
    assets: Vec<Asset>,
}

/// An engine's newest release: its tag, and what it brings in a sentence or two when its notes
/// open with that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Latest {
    pub tag: String,
    pub summary: Option<String>,
}

impl From<Release> for Latest {
    fn from(release: Release) -> Self {
        Self {
            summary: release.body.as_deref().and_then(notes::summary),
            tag: release.tag_name,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct Asset {
    pub(crate) name: String,
    pub(crate) browser_download_url: String,
}

/// The end of the Stockfish archive for this operating system and CPU.
fn stockfish_suffix(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("windows", "aarch64") => Some("windows-arm64-universal.zip"),
        ("windows", "x86_64") => Some("windows-x86-64-universal.zip"),
        ("linux", "aarch64") => Some("linux-arm64-universal.tar.gz"),
        ("linux", "x86_64") => Some("linux-x86-64-universal.tar.gz"),
        ("macos", _) => Some("macos-universal.tar.gz"),
        _ => None,
    }
}

/// The end of the `lc0` archive for this operating system. Only Windows has builds, and
/// x64 ones, which ARM64 Windows runs too. The CPU build needs no GPU drivers.
fn lc0_suffix(os: &str) -> Option<&'static str> {
    (os == "windows").then_some("windows-cpu-dnnl.zip")
}

pub(crate) fn pick<'a>(assets: &'a [Asset], suffix: &str) -> Option<&'a Asset> {
    assets.iter().find(|asset| asset.name.ends_with(suffix))
}

/// Whether Stockfish can be downloaded for this machine.
pub fn can_download_stockfish() -> bool {
    stockfish_suffix(std::env::consts::OS, std::env::consts::ARCH).is_some()
}

/// Whether `lc0`, which Maia needs, can be downloaded for this machine.
pub fn can_download_lc0() -> bool {
    lc0_suffix(std::env::consts::OS).is_some()
}

/// A request to `url`, tried again after a pause while the server says it's struggling:
/// GitHub turns requests away for minutes at a time now and then.
pub(crate) fn get(url: &str) -> Result<Response<Body>, String> {
    let mut attempt = 1;
    loop {
        match AGENT.get(url).header("User-Agent", USER_AGENT).call() {
            Ok(response) => return Ok(response),
            Err(error) if attempt < ATTEMPTS && is_passing(&error) => {
                thread::sleep(RETRY_AFTER * attempt);
                attempt += 1;
            }
            Err(error) => return Err(explain(url, &error)),
        }
    }
}

/// Whether trying again soon might go better: the server's own trouble, or no answer.
fn is_passing(error: &ureq::Error) -> bool {
    matches!(
        error,
        ureq::Error::StatusCode(500..=599) | ureq::Error::Timeout(_)
    )
}

/// What went wrong, saying so when it's the server's trouble rather than the app's.
fn explain(url: &str, error: &ureq::Error) -> String {
    let host = url
        .parse::<ureq::http::Uri>()
        .ok()
        .and_then(|uri| uri.host().map(str::to_owned))
        .unwrap_or_else(|| url.to_owned());
    match error {
        ureq::Error::StatusCode(status @ 500..=599) => {
            format!(
                "{host} isn't answering (HTTP {status}). It may be down for a while; try again later"
            )
        }
        ureq::Error::Timeout(_) => {
            format!("{host} isn't answering. It may be down for a while; try again later")
        }
        error => error.to_string(),
    }
}

fn latest(url: &str) -> Result<Release, String> {
    let body = get(url)?
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}

/// The newest Stockfish release, such as `sf_18`.
pub fn latest_stockfish() -> Result<Latest, String> {
    latest(STOCKFISH_RELEASES).map(Latest::from)
}

/// The newest `lc0` release, such as `v0.32.1`.
pub fn latest_lc0() -> Result<Latest, String> {
    latest(LC0_RELEASES).map(Latest::from)
}

/// The newest Shadowchess networks, such as `v1.0.0`.
pub fn latest_shadowchess() -> Result<Latest, String> {
    latest(SHADOWCHESS_RELEASES).map(Latest::from)
}

/// The release the engine in `folder` was installed from, if Chessdesk installed it.
pub fn installed_release(folder: &Path) -> Option<String> {
    let text = fs::read_to_string(folder.join(RELEASE_FILE)).ok()?;
    Some(text.trim().to_owned()).filter(|tag| !tag.is_empty())
}

/// Where the engines are kept under `root`, by the name of their program.
pub fn engine_folder(root: &Path, program: &str) -> PathBuf {
    root.join(program)
}

/// Saves `url` to `target`, reporting the share done from 0 to 1 when the size is known.
pub(crate) fn fetch(url: &str, target: &Path, progress: &mut dyn FnMut(f32)) -> Result<(), String> {
    let response = get(url)?;
    let total = response.body().content_length();
    let mut reader = response.into_body().into_reader();
    let mut file = File::create(target).map_err(|error| error.to_string())?;
    let mut buffer = vec![0; CHUNK];
    let mut done = 0u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            return Ok(());
        }
        file.write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
        done += read as u64;
        if let Some(total) = total.filter(|total| *total > 0) {
            progress(done as f32 / total as f32);
        }
    }
}

/// Unpacks an archive into `folder`. Both libraries refuse paths leading out of it.
pub(crate) fn unpack(archive: &Path, folder: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|error| error.to_string())?;
    if archive
        .extension()
        .is_some_and(|extension| extension == "zip")
    {
        let mut zip = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
        zip.extract(folder).map_err(|error| error.to_string())
    } else {
        let gzip = flate2::read::GzDecoder::new(file);
        tar::Archive::new(gzip)
            .unpack(folder)
            .map_err(|error| error.to_string())
    }
}

/// Finds the program in an unpacked archive, wherever it put it: the first file whose
/// name starts with `prefix`, and on Windows ends in `.exe`.
pub(crate) fn find_program(folder: &Path, prefix: &str) -> io::Result<Option<PathBuf>> {
    let mut folders = vec![folder.to_owned()];
    while let Some(current) = folders.pop() {
        let mut entries: Vec<_> = fs::read_dir(current)?.flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                folders.push(path);
            } else if is_program(&path, prefix) {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

fn is_program(path: &Path, prefix: &str) -> bool {
    let named = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(prefix));
    let extension = path.extension().and_then(|extension| extension.to_str());
    named
        && if cfg!(windows) {
            extension == Some("exe")
        } else {
            extension.is_none()
        }
}

/// Downloads the release's archive ending in `suffix`, unpacks it into `folder` and notes
/// the release there, returning the program in it.
fn install_archive(
    release: &Release,
    suffix: &str,
    folder: &Path,
    program: &str,
    progress: &mut dyn FnMut(f32),
) -> Result<PathBuf, String> {
    let asset = pick(&release.assets, suffix).ok_or("the release has no build for this machine")?;
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let archive = folder.join(&asset.name);
    fetch(&asset.browser_download_url, &archive, progress)?;
    unpack(&archive, folder)?;
    fs::remove_file(&archive).map_err(|error| error.to_string())?;
    let found = find_program(folder, program)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("no {program} in {}", asset.name))?;
    fs::write(folder.join(RELEASE_FILE), &release.tag_name).map_err(|error| error.to_string())?;
    Ok(found)
}

/// Installs the latest Stockfish under `root`, returning the program.
pub fn install_stockfish(root: &Path, progress: &mut dyn FnMut(f32)) -> Result<PathBuf, String> {
    let suffix = stockfish_suffix(std::env::consts::OS, std::env::consts::ARCH)
        .ok_or("no Stockfish build for this machine")?;
    let release = latest(STOCKFISH_RELEASES)?;
    install_archive(
        &release,
        suffix,
        &engine_folder(root, "stockfish"),
        "stockfish",
        progress,
    )
}

/// Installs the latest Shadowchess networks under `root`, returning the weights file. They
/// run in Chessdesk itself, so there's no program, and every machine can have them.
pub fn install_shadowchess(root: &Path, progress: &mut dyn FnMut(f32)) -> Result<PathBuf, String> {
    let release = latest(SHADOWCHESS_RELEASES)?;
    let asset = pick(&release.assets, SHADOWCHESS_WEIGHTS)
        .ok_or_else(|| format!("no {SHADOWCHESS_WEIGHTS} in the release"))?;
    let folder = engine_folder(root, "shadowchess");
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let target = folder.join(SHADOWCHESS_WEIGHTS);
    // Under its own name only once whole, so that a broken download never passes for it.
    let partial = target.with_extension("part");
    fetch(&asset.browser_download_url, &partial, progress)?;
    fs::rename(&partial, &target).map_err(|error| error.to_string())?;
    fs::write(folder.join(RELEASE_FILE), &release.tag_name).map_err(|error| error.to_string())?;
    Ok(target)
}

/// Installs `lc0` and the Maia networks under `root`, returning the program and the
/// networks' folder.
pub fn install_maia(
    root: &Path,
    progress: &mut dyn FnMut(f32),
) -> Result<(PathBuf, PathBuf), String> {
    let suffix = lc0_suffix(std::env::consts::OS).ok_or("no lc0 build for this system")?;
    let release = latest(LC0_RELEASES)?;
    // The program is the larger part, so it gets half of the bar.
    let lc0 = install_archive(
        &release,
        suffix,
        &engine_folder(root, "lc0"),
        "lc0",
        &mut |done| progress(done / 2.),
    )?;

    let weights = root.join("maia");
    fs::create_dir_all(&weights).map_err(|error| error.to_string())?;
    for (index, level) in MAIA_LEVELS.into_iter().enumerate() {
        let name = format!("maia-{level}.pb.gz");
        fetch(
            &format!("{MAIA_WEIGHTS}/{name}"),
            &weights.join(&name),
            &mut |_| {},
        )?;
        progress(0.5 + (index + 1) as f32 / MAIA_LEVELS.len() as f32 / 2.);
    }
    Ok((lc0, weights))
}

#[cfg(test)]
mod tests {
    use zip::write::SimpleFileOptions;

    use super::*;

    #[test]
    fn server_trouble_is_tried_again_and_said_to_be_theirs() {
        let busy = ureq::Error::StatusCode(504);
        assert!(is_passing(&busy));
        assert!(!is_passing(&ureq::Error::StatusCode(404)));
        let said = explain("https://api.github.com/repos/x/y", &busy);
        assert!(said.starts_with("api.github.com isn't answering (HTTP 504)"));
        assert_eq!(
            explain("https://example.com/a", &ureq::Error::StatusCode(404)),
            "http status: 404"
        );
    }

    fn asset(name: &str) -> Asset {
        Asset {
            name: name.to_owned(),
            browser_download_url: format!("https://example.com/{name}"),
        }
    }

    #[test]
    fn picks_the_build_for_each_machine() {
        // The names of Stockfish 19's assets.
        let assets: Vec<_> = [
            "stockfish-android-arm64-universal.tar.gz",
            "stockfish-linux-arm64-universal.tar.gz",
            "stockfish-linux-x86-64-universal.tar.gz",
            "stockfish-macos-universal.tar.gz",
            "stockfish-windows-arm64-universal.zip",
            "stockfish-windows-x86-64-universal.zip",
        ]
        .map(asset)
        .into();
        let name = |os, arch| {
            let suffix = stockfish_suffix(os, arch).unwrap();
            pick(&assets, suffix).unwrap().name.as_str()
        };
        assert_eq!(
            name("windows", "aarch64"),
            "stockfish-windows-arm64-universal.zip"
        );
        assert_eq!(
            name("windows", "x86_64"),
            "stockfish-windows-x86-64-universal.zip"
        );
        assert_eq!(
            name("linux", "x86_64"),
            "stockfish-linux-x86-64-universal.tar.gz"
        );
        assert_eq!(name("macos", "aarch64"), "stockfish-macos-universal.tar.gz");
        assert_eq!(stockfish_suffix("freebsd", "x86_64"), None);
    }

    #[test]
    fn lc0_comes_only_for_windows_and_without_gpu_needs() {
        let assets = [
            asset("lc0-v0.32.1-windows-gpu-nvidia-cuda12.zip"),
            asset("lc0-v0.32.1-windows-cpu-dnnl.zip"),
        ];
        let picked = pick(&assets, lc0_suffix("windows").unwrap()).unwrap();
        assert_eq!(picked.name, "lc0-v0.32.1-windows-cpu-dnnl.zip");
        assert_eq!(lc0_suffix("macos"), None);
    }

    #[test]
    fn reads_a_release() {
        let body = r#"{"tag_name":"sf_19","assets":[{"name":"a.zip",
            "browser_download_url":"https://example.com/a.zip","size":1}]}"#;
        let release: Release = serde_json::from_str(body).unwrap();
        assert_eq!(release.tag_name, "sf_19");
        assert_eq!(release.assets, [asset("a.zip")]);
    }

    #[test]
    fn an_engine_knows_the_release_it_came_from() {
        let folder = std::env::temp_dir().join(format!("chessdesk-release-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        assert_eq!(installed_release(&folder), None);
        fs::write(folder.join(RELEASE_FILE), "sf_19\n").unwrap();
        let found = installed_release(&folder);
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(found.as_deref(), Some("sf_19"));
    }

    #[test]
    fn finds_the_program_in_an_unpacked_zip() {
        let folder = std::env::temp_dir().join(format!("chessdesk-zip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        let archive = folder.join("engine.zip");
        let mut zip = zip::ZipWriter::new(File::create(&archive).unwrap());
        let name = if cfg!(windows) {
            "stockfish/stockfish-windows.exe"
        } else {
            "stockfish/stockfish-linux"
        };
        zip.start_file("stockfish/README.md", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"read me").unwrap();
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(b"binary").unwrap();
        zip.finish().unwrap();

        unpack(&archive, &folder).unwrap();
        let found = find_program(&folder, "stockfish").unwrap().unwrap();
        let contents = fs::read(&found).unwrap();
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(contents, b"binary");
    }
}
