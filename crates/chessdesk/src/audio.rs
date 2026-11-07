//! Playing sounds, on a thread of their own: an audio device can take its time to open,
//! and the interface shouldn't wait for it.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;

use chessdesk_core::plugins::sound_file;
use chessdesk_core::sound::Sound;
use gpui::{App, Global};
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Source};

use crate::settings::Settings;

/// A file to play, and how loud from 0 to 1.
type Request = (PathBuf, f32);

struct Player(Sender<Request>);

impl Global for Player {}

/// Reads a whole sound into memory. These are short, and a decoder added to the mixer as
/// it is plays nothing but silence.
fn load(path: &Path) -> Option<SamplesBuffer> {
    let sound = Decoder::try_from(File::open(path).ok()?).ok()?;
    let (channels, rate) = (sound.channels(), sound.sample_rate());
    Some(SamplesBuffer::new(
        channels,
        rate,
        sound.collect::<Vec<_>>(),
    ))
}

/// Plays one file. A sound that can't play isn't worth interrupting a game for, so what
/// goes wrong here stays here.
fn play_file(device: &MixerDeviceSink, path: &Path, volume: f32) {
    if let Some(sound) = load(path) {
        device.mixer().add(sound.amplify(volume));
    }
}

fn open_device() -> Option<MixerDeviceSink> {
    let mut device = DeviceSinkBuilder::open_default_sink().ok()?;
    // It lives as long as the app, so that it closes is no news.
    device.log_on_drop(false);
    Some(device)
}

fn run(requests: mpsc::Receiver<Request>) {
    // Opened on the first sound, and tried again on the next if there's no device yet.
    let mut device = None;
    for (path, volume) in requests {
        if device.is_none() {
            device = open_device();
        }
        if let Some(device) = &device {
            play_file(device, &path, volume);
        }
    }
}

/// Starts the thread. Without this, sounds are simply not played.
pub fn init(cx: &mut App) {
    let (requests, incoming) = mpsc::channel();
    let started = thread::Builder::new()
        .name("audio".to_owned())
        .spawn(move || run(incoming));
    if started.is_ok() {
        cx.set_global(Player(requests));
    }
}

fn send(path: PathBuf, volume: f32, cx: &App) {
    if let Some(player) = cx.try_global::<Player>() {
        let _ = player.0.send((path, volume));
    }
}

/// Plays a sound of the chosen pack, if there is one.
pub fn play(sound: Sound, cx: &App) {
    let settings = Settings::get(cx);
    if let Some(path) = settings
        .sound_pack_dir
        .as_deref()
        .and_then(|dir| sound_file(dir, sound))
    {
        send(path, settings.sound_volume, cx);
    }
}

/// Plays a move in `pack`, to hear what a choice sounds like.
pub fn preview(pack: &str, volume: f32, cx: &App) {
    let settings = Settings::get(cx);
    let chosen = settings.sound_packs.iter().find(|known| known.name == pack);
    if let Some(path) = chosen.and_then(|pack| sound_file(&pack.dir, Sound::Move)) {
        send(path, volume, cx);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::num::NonZero;

    use rodio::mixer::mixer;

    use super::*;

    /// How loud a sound's loudest sample has to be for it to count as more than silence.
    const AUDIBLE: f32 = 0.01;

    /// What a device gets to hear of a file: mixed, and at a rate the file isn't in.
    fn peak_through_a_mixer(path: &Path) -> f32 {
        let channels = NonZero::new(2).unwrap();
        let rate = NonZero::new(48_000).unwrap();
        let (input, output) = mixer(channels, rate);
        input.add(load(path).unwrap());
        output.fold(0_f32, |peak, sample| peak.max(sample.abs()))
    }

    #[test]
    fn every_bundled_sound_reaches_the_device() {
        let plugins = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins");
        let mut checked = 0;
        for pack in ["wood-sounds", "soft-sounds"] {
            for entry in fs::read_dir(plugins.join(pack).join("sounds")).unwrap() {
                let path = entry.unwrap().path();
                let peak = peak_through_a_mixer(&path);
                assert!(peak > AUDIBLE, "{} is silent", path.display());
                assert!(peak <= 1., "{} clips", path.display());
                checked += 1;
            }
        }
        assert_eq!(checked, 18);
    }
}
