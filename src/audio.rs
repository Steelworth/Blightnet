use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const TAP_N: usize = 512;

/// Last samples from the local deck. The audio thread writes; the UI reads.
/// Nothing here is sent to other seats.
pub struct DeckTap {
    buf: Mutex<[f32; TAP_N]>,
    write: AtomicUsize,
}

impl DeckTap {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            buf: Mutex::new([0.0; TAP_N]),
            write: AtomicUsize::new(0),
        })
    }

    fn push_block(&self, samples: &[f32]) {
        let Ok(mut buf) = self.buf.lock() else {
            return;
        };
        let mut w = self.write.load(Ordering::Relaxed);
        for &s in samples {
            buf[w % TAP_N] = s;
            w = w.wrapping_add(1);
        }
        self.write.store(w, Ordering::Relaxed);
    }

    pub fn latest(&self) -> [f32; 128] {
        let Ok(buf) = self.buf.lock() else {
            return [0.0; 128];
        };
        let w = self.write.load(Ordering::Relaxed);
        let mut out = [0.0; 128];
        for i in 0..128 {
            let idx = w.wrapping_sub(128 - i) % TAP_N;
            out[i] = buf[idx];
        }
        out
    }

    fn clear(&self) {
        if let Ok(mut buf) = self.buf.lock() {
            *buf = [0.0; TAP_N];
        }
        self.write.store(0, Ordering::Relaxed);
    }
}

struct DeckTee<I> {
    inner: I,
    tap: Arc<DeckTap>,
    scratch: [f32; 64],
    filled: usize,
}

impl<I: Iterator<Item = f32>> Iterator for DeckTee<I> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        self.scratch[self.filled] = sample;
        self.filled += 1;
        if self.filled == self.scratch.len() {
            self.tap.push_block(&self.scratch);
            self.filled = 0;
        }
        Some(sample)
    }
}

impl<I: Source<Item = f32>> Source for DeckTee<I> {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

#[derive(Clone, Debug)]
pub struct AudioDev {
    /// What we save and try to open first (hardware description).
    pub id: String,
    /// Shown in the deck combo.
    pub label: String,
    /// Pulse/ALSA node name, used if `id` is not what cpal reports.
    pub alt: String,
}

struct Voice {
    sink: Sink,
    volume: f32,
    presence: f32,
}

pub struct Mixer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    pub master: f32,
    voices: HashMap<String, Voice>,
    root: PathBuf,
    radio_tap: Arc<DeckTap>,
    talk: Option<Sink>,
    clip: Option<Sink>,
    deck: Option<Sink>,
    deck_vol: f32,
    tap: Arc<DeckTap>,
}

fn junk_device(name: &str) -> bool {
    let n = name.to_lowercase();
    const BAD: &[&str] = &[
        "oss",
        "dummy",
        "null",
        "speex",
        "upmix",
        "vdownmix",
        "dmix",
        "dsnoop",
        "usbstream",
        "phoneline",
        "modem",
        "/dev/dsp",
    ];
    if n.contains("jack:") || n.starts_with("jack ") || n.contains("jack audio") {
        return true;
    }
    BAD.iter().any(|b| n.contains(b))
}

fn device_rank(name: &str, is_default: bool) -> i32 {
    if is_default {
        return 0;
    }
    let n = name.to_lowercase();
    if n == "default" || n == "pulse" || n.contains("pipewire") {
        1
    } else if n.contains("pulse") {
        2
    } else if n.starts_with("sysdefault") {
        3
    } else {
        8
    }
}

fn pactl_devs(kind: &str) -> Vec<AudioDev> {
    let Ok(out) = Command::new("pactl").args(["list", kind]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut acc = Vec::new();
    let mut name = String::new();
    let mut desc = String::new();
    let flush = |acc: &mut Vec<AudioDev>, name: &mut String, desc: &mut String| {
        if name.is_empty() {
            return;
        }
        let n = name.clone();
        let d = desc.clone();
        name.clear();
        desc.clear();
        if n.ends_with(".monitor") || d.starts_with("Monitor of") {
            return;
        }
        if junk_device(&n) || junk_device(&d) {
            return;
        }
        let label = if d.is_empty() { n.clone() } else { d };
        if acc.iter().any(|x| x.id == n || x.label == label || x.alt == n) {
            return;
        }
        acc.push(AudioDev {
            id: label.clone(),
            label,
            alt: n,
        });
    };
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("Name:") {
            flush(&mut acc, &mut name, &mut desc);
            name = t.trim_start_matches("Name:").trim().to_string();
        } else if t.starts_with("Description:") {
            desc = t.trim_start_matches("Description:").trim().to_string();
        }
    }
    flush(&mut acc, &mut name, &mut desc);
    acc
}

fn pactl_default(kind: &str) -> Option<String> {
    let arg = if kind == "sources" {
        "get-default-source"
    } else {
        "get-default-sink"
    };
    let out = Command::new("pactl").arg(arg).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() || s.ends_with(".monitor") {
        None
    } else {
        Some(s)
    }
}

fn cpal_fallback(input: bool) -> Vec<AudioDev> {
    let host = rodio::cpal::default_host();
    let listed = if input {
        host.input_devices()
    } else {
        host.output_devices()
    };
    let mut names = Vec::new();
    if let Ok(devs) = listed {
        for d in devs {
            if let Ok(n) = d.name() {
                if junk_device(&n) {
                    continue;
                }
                let low = n.to_lowercase();
                let soft = low == "pulse"
                    || low.contains("pipewire")
                    || low.starts_with("sysdefault")
                    || (!cfg!(windows) && (low == "default" || low == "pulse"));
                if soft {
                    continue;
                }
                let label = pretty_alsa(&n);
                if names.iter().any(|x: &AudioDev| x.id == n || x.label == label) {
                    continue;
                }
                names.push(AudioDev {
                    id: label.clone(),
                    label,
                    alt: n,
                });
            }
        }
    }
    names
}

fn pretty_alsa(id: &str) -> String {
    let mut s = id.to_string();
    for p in [
        "alsa_input.",
        "alsa_output.",
        "alsa_input:",
        "alsa_output:",
    ] {
        if let Some(r) = s.strip_prefix(p) {
            s = r.to_string();
            break;
        }
    }
    s = s.replace('_', " ");
    s = s.replace('-', " ");
    s.split('.').next().unwrap_or(&s).trim().to_string()
}

fn enumerate(input: bool) -> Vec<AudioDev> {
    let kind = if input { "sources" } else { "sinks" };
    let mut names = pactl_devs(kind);
    if names.is_empty() {
        names = cpal_fallback(input);
    }
    let def = pactl_default(kind).or_else(|| {
        let host = rodio::cpal::default_host();
        if input {
            host.default_input_device()
        } else {
            host.default_output_device()
        }
        .and_then(|d| d.name().ok())
        .filter(|n| !junk_device(n) && !n.ends_with(".monitor"))
    });
    names.sort_by_key(|n| device_rank(&n.label, def.as_ref().is_some_and(|d| d == &n.id || d == &n.alt)));
    if let Some(d) = def {
        if let Some(i) = names.iter().position(|x| x.id == d || x.alt == d) {
            names.swap(0, i);
        }
    }
    names
}

pub fn list_inputs() -> Vec<AudioDev> {
    enumerate(true)
}

pub fn list_outputs() -> Vec<AudioDev> {
    enumerate(false)
}

pub fn default_input_name() -> Option<String> {
    list_inputs().into_iter().next().map(|d| d.id)
}

pub fn default_output_name() -> Option<String> {
    list_outputs().into_iter().next().map(|d| d.id)
}

pub fn pick_listed(list: &[AudioDev], current: &str, fallback: Option<String>) -> String {
    if !current.is_empty() {
        if let Some(d) = list
            .iter()
            .find(|d| d.id == current || d.label == current || d.alt == current)
        {
            return d.id.clone();
        }
    }
    if let Some(fb) = fallback {
        if let Some(d) = list
            .iter()
            .find(|d| d.id == fb || d.label == fb || d.alt == fb)
        {
            return d.id.clone();
        }
    }
    list.first().map(|d| d.id.clone()).unwrap_or_default()
}

fn try_named_output(want: &str) -> Option<(OutputStream, OutputStreamHandle)> {
    let host = rodio::cpal::default_host();
    let Ok(devs) = host.output_devices() else {
        return None;
    };
    let mut aliases = vec![want.to_string()];
    for mapped in pactl_devs("sinks") {
        if mapped.id == want || mapped.label == want || mapped.alt == want {
            aliases.push(mapped.id);
            aliases.push(mapped.label);
            aliases.push(mapped.alt);
        }
    }
    for d in devs {
        if let Ok(n) = d.name() {
            if aliases.iter().any(|a| a == &n) {
                return OutputStream::try_from_device(&d).ok();
            }
        }
    }
    None
}

fn output_stream(name: Option<&str>) -> Result<(OutputStream, OutputStreamHandle), String> {
    if let Some(want) = name {
        if !want.is_empty() {
            if let Some(s) = try_named_output(want) {
                return Ok(s);
            }
        }
    }
    if !cfg!(windows) {
        for n in ["pulse", "pipewire", "default"] {
            if let Some(s) = try_named_output(n) {
                return Ok(s);
            }
        }
    }
    if let Some(n) = default_output_name() {
        if let Some(s) = try_named_output(&n) {
            return Ok(s);
        }
    }
    OutputStream::try_default().map_err(|e| format!("audio device: {e}"))
}

impl Mixer {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        Self::with_output(root, None)
    }

    pub fn with_output(root: PathBuf, speaker: Option<&str>) -> Result<Self, String> {
        let (stream, handle) = output_stream(speaker)?;
        Ok(Self {
            _stream: stream,
            handle,
            master: 0.85,
            voices: HashMap::new(),
            root,
            talk: None,
            clip: None,
            deck: None,
            deck_vol: 0.7,
            tap: DeckTap::new(),
            radio_tap: DeckTap::new(),
        })
    }

    pub fn deck_wave(&self) -> [f32; 128] {
        if self.deck_live() {
            self.tap.latest()
        } else {
            [0.0; 128]
        }
    }

    /// Bars for the player. A deck file wins. A station uses the radio tap.
    pub fn viz_wave(&self) -> [f32; 128] {
        if self.deck_live() {
            return self.deck_wave();
        } else if self.is_on("__radio") {
            self.radio_tap.latest()
        } else {
            [0.0; 128]
        }
    }

    fn append_voice<S>(&self, sink: &Sink, id: &str, source: S)
    where
        S: Source<Item = f32> + Send + 'static,
    {
        if id == "__radio" {
            sink.append(DeckTee {
                inner: source,
                tap: Arc::clone(&self.radio_tap),
                scratch: [0.0; 64],
                filled: 0,
            });
        } else {
            sink.append(source);
        }
    }

    pub fn play_pcm(&mut self, samples: Vec<f32>, rate: u32) {
        if samples.is_empty() {
            return;
        }
        if let Some(sink) = self.talk.as_ref() {
            if sink.len() >= 6 {
                return;
            }
            let buf = SamplesBuffer::new(1, rate, samples);
            sink.append(buf);
            if sink.is_paused() && sink.len() >= 2 {
                sink.play();
            }
            return;
        }
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.set_volume(self.master.clamp(0.2, 1.0));
            let primed = samples.len() >= 480;
            sink.pause();
            sink.append(SamplesBuffer::new(1, rate, samples));
            if primed {
                sink.play();
            }
            self.talk = Some(sink);
        }
    }

    pub fn play_bytes(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        use std::io::Cursor;
        let Ok(dec) = Decoder::new(Cursor::new(bytes.to_vec())) else {
            return;
        };
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.set_volume(self.master.clamp(0.2, 1.0));
            sink.append(dec);
            sink.play();
            self.clip = Some(sink);
        }
    }

    pub fn playing(&self) -> impl Iterator<Item = (&String, f32)> {
        self.voices.iter().map(|(id, v)| (id, v.volume))
    }

    pub fn is_on(&self, id: &str) -> bool {
        self.voices.contains_key(id)
    }

    pub fn volume_of(&self, id: &str) -> f32 {
        self.voices.get(id).map(|v| v.volume).unwrap_or(0.45)
    }

    pub fn set_master(&mut self, v: f32) {
        self.master = v.clamp(0.0, 1.0);
        self.reapply();
    }

    fn reapply(&mut self) {
        let m = self.master;
        for v in self.voices.values() {
            v.sink
                .set_volume((v.volume * v.presence * m).clamp(0.0, 1.0));
        }
        if let Some(s) = self.deck.as_ref() {
            s.set_volume((self.deck_vol * m).clamp(0.0, 1.0));
        }
    }

    pub fn deck_play_path(&mut self, path: &Path) -> Result<(), String> {
        self.deck_stop();
        let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let dec = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("decode {}: {e}", path.display()))?
            .convert_samples::<f32>();
        let sink = Sink::try_new(&self.handle).map_err(|e| e.to_string())?;
        sink.set_volume((self.deck_vol * self.master).clamp(0.0, 1.0));
        sink.append(DeckTee {
            inner: dec,
            tap: Arc::clone(&self.tap),
            scratch: [0.0; 64],
            filled: 0,
        });
        sink.play();
        self.deck = Some(sink);
        Ok(())
    }

    pub fn deck_stop(&mut self) {
        if let Some(s) = self.deck.take() {
            s.stop();
        }
        self.tap.clear();
    }

    pub fn deck_pause(&mut self) {
        if let Some(s) = self.deck.as_ref() {
            s.pause();
        }
    }

    pub fn deck_resume(&mut self) {
        if let Some(s) = self.deck.as_ref() {
            s.play();
        }
    }

    pub fn deck_live(&self) -> bool {
        self.deck
            .as_ref()
            .map(|s| !s.empty() && !s.is_paused())
            .unwrap_or(false)
    }

    pub fn deck_done(&self) -> bool {
        match &self.deck {
            Some(s) => s.empty() && !s.is_paused(),
            None => true,
        }
    }

    pub fn set_deck_vol(&mut self, v: f32) {
        self.deck_vol = v.clamp(0.0, 1.0);
        if let Some(s) = self.deck.as_ref() {
            s.set_volume((self.deck_vol * self.master).clamp(0.0, 1.0));
        }
    }

    fn apply_voice(v: &Voice, master: f32) {
        v.sink
            .set_volume((v.volume * v.presence * master).clamp(0.0, 1.0));
    }

    pub fn stop(&mut self, id: &str) {
        if let Some(v) = self.voices.remove(id) {
            v.sink.stop();
        }
        if id == "__radio" {
            self.radio_tap.clear();
        }
    }

    pub fn silence(&mut self) {
        for (_, v) in self.voices.drain() {
            v.sink.stop();
        }
    }

    pub fn set_volume(&mut self, id: &str, vol: f32) {
        let vol = vol.clamp(0.0, 1.0);
        let master = self.master;
        if let Some(v) = self.voices.get_mut(id) {
            v.volume = vol;
            Self::apply_voice(v, master);
        }
    }

    pub fn set_presence(&mut self, id: &str, p: f32) {
        let master = self.master;
        if let Some(v) = self.voices.get_mut(id) {
            v.presence = p.clamp(0.0, 1.5);
            Self::apply_voice(v, master);
        }
    }

    pub fn snapshot(&self) -> HashMap<String, f32> {
        self.voices
            .iter()
            .filter(|(id, _)| !id.starts_with("__"))
            .map(|(id, v)| (id.clone(), v.volume))
            .collect()
    }

    pub fn play_once(&mut self, id: &str, path: &std::path::Path, vol: f32) -> Result<(), String> {
        self.stop(id);
        let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let dec = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("decode: {e}"))?
            .convert_samples::<f32>();
        let sink = Sink::try_new(&self.handle).map_err(|e| e.to_string())?;
        let vol = vol.clamp(0.0, 1.0);
        sink.set_volume((vol * self.master).clamp(0.0, 1.0));
        self.append_voice(&sink, id, dec);
        sink.play();
        self.voices.insert(
            id.to_string(),
            Voice {
                sink,
                volume: vol,
                presence: 1.0,
            },
        );
        Ok(())
    }

    pub fn voice_done(&self, id: &str) -> bool {
        self.voices.get(id).map(|v| v.sink.empty()).unwrap_or(true)
    }

    pub fn play(&mut self, id: &str, rel: &str, vol: f32) -> Result<(), String> {
        if self.voices.contains_key(id) {
            self.set_volume(id, vol);
            return Ok(());
        }
        let path: PathBuf = self.root.join(rel);
        let file = File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let dec = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("decode {rel}: {e}"))?
            .convert_samples::<f32>()
            .repeat_infinite();
        let sink = Sink::try_new(&self.handle).map_err(|e| e.to_string())?;
        let vol = vol.clamp(0.0, 1.0);
        sink.set_volume((vol * self.master).clamp(0.0, 1.0));
        self.append_voice(&sink, id, dec);
        sink.play();
        self.voices.insert(
            id.to_string(),
            Voice {
                sink,
                volume: vol,
                presence: 1.0,
            },
        );
        Ok(())
    }

    pub fn apply_scene(
        &mut self,
        layers: &HashMap<String, f32>,
        files: &HashMap<String, String>,
    ) -> Result<(), String> {
        let ids: Vec<String> = self.voices.keys().cloned().collect();
        for id in ids {
            if id.starts_with("__") {
                continue;
            }
            if !layers.contains_key(&id) {
                self.stop(&id);
            }
        }
        let mut first_err = None;
        for (id, vol) in layers {
            if let Some(file) = files.get(id) {
                if let Err(e) = self.play(id, file, *vol) {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                }
            } else if first_err.is_none() {
                first_err = Some(format!("no file for layer {id}"));
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

#[allow(dead_code)]
pub fn exists(root: &Path, rel: &str) -> bool {
    root.join(rel).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn sample_tracks_decode() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        probe_ogg(&root, "audio/music/tavern_jig.ogg").unwrap();
        probe_ogg(&root, "audio/weather/drizzle.ogg").unwrap();
        probe_ogg(&root, "audio/ambience/barn.ogg").unwrap();
    }

    #[test]
    fn blight_radio_mood_tracks_decode() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for rel in [
            "audio/blight/music/br_burn_the_world_waltz.ogg",
            "audio/blight/music/metalmania.ogg",
            "audio/blight/music/newer_wave.ogg",
            "audio/blight/music/chill_wave.ogg",
            "audio/blight/music/edm_detect.ogg",
        ] {
            if root.join(rel).is_file() {
                probe_ogg(&root, rel).unwrap_or_else(|e| panic!("{rel}: {e}"));
            }
        }
    }

    #[test]
    fn device_lists_do_not_panic() {
        let _ = list_inputs();
        let _ = list_outputs();
        let _ = default_input_name();
        let _ = default_output_name();
    }

    #[test]
    fn pick_listed_keeps_current_and_falls_back() {
        let list = vec![
            AudioDev {
                id: "Starship HD Audio".into(),
                label: "Starship HD Audio".into(),
                alt: "alsa_input.pci-hw".into(),
            },
            AudioDev {
                id: "USB 2.0 Camera".into(),
                label: "USB 2.0 Camera".into(),
                alt: "alsa_input.usb-cam".into(),
            },
        ];
        assert_eq!(
            pick_listed(&list, "alsa_input.usb-cam", None),
            "USB 2.0 Camera"
        );
        assert_eq!(
            pick_listed(&list, "Starship HD Audio", None),
            "Starship HD Audio"
        );
        assert_eq!(
            pick_listed(&list, "gone", Some("alsa_input.pci-hw".into())),
            "Starship HD Audio"
        );
        assert_eq!(pick_listed(&[], "x", None), "");
    }

    #[test]
    fn music_ext_accepts_common_files() {
        assert!(is_music(Path::new("/home/x/song.ogg")));
        assert!(is_music(Path::new("mix.MP3")));
        assert!(is_music(Path::new("a.flac")));
        assert!(!is_music(Path::new("notes.txt")));
        assert!(!is_music(Path::new("clip.mp4")));
    }

    #[test]
    fn wav_roundtrip_header() {
        let wav = encode_wav(&[0.0, 0.5, -0.5], 16000);
        assert!(wav.starts_with(b"RIFF"));
        assert!(wav.len() > 44);
    }

    #[test]
    fn hardware_labels_skip_software_aliases() {
        let ins = list_inputs();
        let outs = list_outputs();
        for d in ins.iter().chain(outs.iter()) {
            let n = d.label.to_lowercase();
            assert!(!n.eq("pulse") && !n.eq("pipewire"));
            if !cfg!(windows) {
                assert!(!n.eq("default"));
            }
            assert!(!d.id.ends_with(".monitor"));
            assert!(!d.label.starts_with("Monitor of"));
        }
    }
}

pub fn encode_wav(samples: &[f32], rate: u32) -> Vec<u8> {
    let pcm: Vec<i16> = samples
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect();
    let data_len = (pcm.len() * 2) as u32;
    let mut o = Vec::with_capacity(44 + data_len as usize);
    o.extend_from_slice(b"RIFF");
    o.extend_from_slice(&(36 + data_len).to_le_bytes());
    o.extend_from_slice(b"WAVEfmt ");
    o.extend_from_slice(&16u32.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&rate.to_le_bytes());
    o.extend_from_slice(&(rate * 2).to_le_bytes());
    o.extend_from_slice(&2u16.to_le_bytes());
    o.extend_from_slice(&16u16.to_le_bytes());
    o.extend_from_slice(b"data");
    o.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        o.extend_from_slice(&s.to_le_bytes());
    }
    o
}

pub fn is_music(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str(),
        "ogg" | "mp3" | "wav" | "flac" | "opus" | "m4a" | "aac"
    )
}

pub fn probe_ogg(root: &Path, rel: &str) -> Result<(), String> {
    let path = root.join(rel);
    let file = File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let _ = Decoder::new(BufReader::new(file)).map_err(|e| format!("decode {rel}: {e}"))?;
    Ok(())
}
