//! Music and sound effects.
//!
//! Music works like the classic game: a calm track now and then, picked at random for the current
//! situation, with long silences in between (10 to 20 minutes by default). Your own songs can
//! replace or join the built-in ones: put them in the `music` folder (mp3, ogg, wav or flac),
//! optionally in sub-folders named after a situation.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use serde::Deserialize;

use crate::embedded;
use crate::settings::{MusicMode, Settings};

/// Nether and End music is ready for when those dimensions arrive.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    Title,
    Overworld,
    Creative,
    Underwater,
    Nether,
    End,
}

impl Situation {
    fn folder(self) -> &'static str {
        match self {
            Situation::Title => "title",
            Situation::Overworld => "overworld",
            Situation::Creative => "creative",
            Situation::Underwater => "underwater",
            Situation::Nether => "nether",
            Situation::End => "end",
        }
    }
    /// Which track situations may play here. Creative also plays overworld music, like the original.
    fn accepts(self, track: &str) -> bool {
        match self {
            Situation::Creative => track == "creative" || track == "overworld",
            s => track == s.folder(),
        }
    }
}

#[derive(Deserialize)]
struct TrackInfo {
    id: String,
    title: String,
    situation: String,
}

#[derive(Clone)]
enum TrackSource {
    Builtin(&'static [u8]),
    File(PathBuf),
}

#[derive(Clone)]
struct Track {
    title: String,
    /// Situation folder name, or "any" for loose files in the music folder.
    situation: String,
    source: TrackSource,
    custom: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Sfx {
    Break(Material),
    Place(Material),
    Step(Material),
    Click,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Stone,
    Wood,
    Grass,
    Sand,
    Glass,
    Cloth,
}

impl Material {
    pub fn of(name: &str) -> Material {
        if name.contains("glass") || name == "ice" || name == "glowstone" {
            Material::Glass
        } else if name.contains("log") || name.contains("planks") || name.contains("crafting") || name == "bookshelf" {
            Material::Wood
        } else if name.contains("wool") || name == "sponge" {
            Material::Cloth
        } else if name == "sand" || name == "gravel" || name.contains("snow") || name == "clay" {
            Material::Sand
        } else if name.contains("grass") || name == "dirt" || name.contains("leaves") || name == "poppy" || name == "dandelion"
            || name == "dead_bush" || name == "melon" || name == "pumpkin" || name == "cactus" || name == "tnt"
        {
            Material::Grass
        } else {
            Material::Stone
        }
    }
}

pub struct Audio {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    music: Sink,
    tracks: Vec<Track>,
    /// Seconds until the next song may start.
    pub next_in: f32,
    pub now_playing: Option<String>,
    rng: u64,
    music_folder: PathBuf,
}

impl Audio {
    pub fn new(settings: &Settings, base: &Path) -> Option<Audio> {
        let (stream, handle) = match OutputStream::try_default() {
            Ok(v) => v,
            Err(e) => {
                log::warn!("no audio device: {e}");
                return None;
            }
        };
        let music = Sink::try_new(&handle).ok()?;
        let mut a = Audio {
            _stream: stream,
            handle,
            music,
            tracks: Vec::new(),
            next_in: 0.0,
            now_playing: None,
            rng: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1,
            music_folder: base.join(&settings.music_folder),
        };
        a.reload_tracks();
        // The first song comes fairly soon after joining, the rest after long silences.
        a.next_in = 20.0 + a.rand() * 40.0;
        Some(a)
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn reload_tracks(&mut self) {
        let infos: Vec<TrackInfo> = serde_json_lite(embedded::TRACKS_JSON);
        self.tracks.clear();
        for (id, bytes) in embedded::MUSIC {
            let info = infos.iter().find(|t| t.id == *id);
            self.tracks.push(Track {
                title: info.map(|i| i.title.clone()).unwrap_or_else(|| id.to_string()),
                situation: info.map(|i| i.situation.clone()).unwrap_or_else(|| "overworld".into()),
                source: TrackSource::Builtin(bytes),
                custom: false,
            });
        }
        let mut custom = Vec::new();
        collect_music(&self.music_folder, "any", &mut custom, 0);
        log::info!("music: {} built-in tracks, {} of your own in {}", self.tracks.len(), custom.len(), self.music_folder.display());
        self.tracks.extend(custom);
    }

    pub fn custom_track_count(&self) -> usize {
        self.tracks.iter().filter(|t| t.custom).count()
    }

    /// Call every frame.
    pub fn update(&mut self, dt: f32, situation: Situation, settings: &Settings) {
        self.music.set_volume(settings.music_volume);
        if settings.music_mode == MusicMode::Off {
            if !self.music.empty() {
                self.music.stop();
            }
            self.now_playing = None;
            return;
        }
        if !self.music.empty() {
            return;
        }
        if self.now_playing.take().is_some() {
            // A song just ended: wait the classic long silence.
            let (lo, hi) = (settings.music_delay_min, settings.music_delay_max.max(settings.music_delay_min));
            self.next_in = lo + self.rand() * (hi - lo);
        }
        self.next_in -= dt;
        if self.next_in <= 0.0 {
            self.play_random(situation, settings.music_mode);
        }
    }

    /// Start a song right away (the "next song" key).
    pub fn skip(&mut self, situation: Situation, settings: &Settings) {
        self.music.stop();
        self.now_playing = None;
        if settings.music_mode != MusicMode::Off {
            self.play_random(situation, settings.music_mode);
        }
    }

    fn play_random(&mut self, situation: Situation, mode: MusicMode) {
        let pick: Vec<Track> = self
            .tracks
            .iter()
            .filter(|t| match mode {
                MusicMode::Classic => !t.custom,
                MusicMode::Custom => t.custom,
                MusicMode::Both => true,
                MusicMode::Off => false,
            })
            .filter(|t| t.situation == "any" || situation.accepts(&t.situation))
            .cloned()
            .collect();
        if pick.is_empty() {
            self.next_in = 30.0;
            return;
        }
        let t = &pick[((self.rand() * pick.len() as f32) as usize).min(pick.len() - 1)];
        let ok = match &t.source {
            TrackSource::Builtin(b) => Decoder::new(Cursor::new(*b)).map(|d| self.music.append(d)).is_ok(),
            TrackSource::File(p) => File::open(p)
                .ok()
                .and_then(|f| Decoder::new(BufReader::new(f)).ok())
                .map(|d| self.music.append(d))
                .is_some(),
        };
        if ok {
            log::info!("now playing: {}", t.title);
            self.now_playing = Some(t.title.clone());
            self.music.play();
        } else {
            self.next_in = 5.0;
        }
    }

    pub fn play(&mut self, sfx: Sfx, volume: f32) {
        if volume <= 0.0 {
            return;
        }
        let seed = (self.rand() * 1e6) as u32;
        let samples = synth(sfx, seed);
        let src = rodio::buffer::SamplesBuffer::new(1, 44100, samples).amplify(volume);
        let _ = self.handle.play_raw(src);
    }
}

fn collect_music(dir: &Path, situation: &str, out: &mut Vec<Track>, depth: u32) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() && depth < 3 {
            let name = p.file_name().unwrap().to_string_lossy().to_lowercase();
            let known = ["title", "overworld", "creative", "underwater", "nether", "end"];
            let sit = if known.contains(&name.as_str()) { name } else { situation.to_string() };
            collect_music(&p, &sit, out, depth + 1);
        } else if p.extension().is_some_and(|e| {
            let e = e.to_string_lossy().to_lowercase();
            ["mp3", "ogg", "wav", "flac"].contains(&e.as_str())
        }) {
            out.push(Track {
                title: p.file_stem().unwrap().to_string_lossy().to_string(),
                situation: situation.to_string(),
                source: TrackSource::File(p),
                custom: true,
            });
        }
    }
}

/// tracks.json is a flat array of objects with string/number values; parse it without a JSON crate
/// by converting it to RON (a JSON object `{"a": 1}` is a RON map).
fn serde_json_lite(json: &str) -> Vec<TrackInfo> {
    let Ok(list) = ron::from_str::<Vec<std::collections::HashMap<String, ron::Value>>>(json) else { return Vec::new() };
    list.into_iter()
        .filter_map(|m| {
            let s = |k: &str| match m.get(k) {
                Some(ron::Value::String(s)) => Some(s.clone()),
                _ => None,
            };
            Some(TrackInfo { id: s("id")?, title: s("title")?, situation: s("situation")? })
        })
        .collect()
}

/// Small procedural sound effects (no sound files needed).
fn synth(sfx: Sfx, seed: u32) -> Vec<f32> {
    let rate = 44100.0;
    let mut state = seed.wrapping_mul(747796405).wrapping_add(2891336453) | 1;
    let mut noise = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    let (mat, dur, gain) = match sfx {
        Sfx::Break(m) => (m, 0.22, 0.9),
        Sfx::Place(m) => (m, 0.12, 0.7),
        Sfx::Step(m) => (m, 0.09, 0.25),
        Sfx::Click => (Material::Glass, 0.04, 0.4),
    };
    // Low-pass amount and a resonant tone per material.
    let (lp, tone, tone_mix): (f32, f32, f32) = match mat {
        Material::Stone => (0.55, 180.0, 0.15),
        Material::Wood => (0.25, 140.0, 0.45),
        Material::Grass => (0.18, 0.0, 0.0),
        Material::Sand => (0.4, 0.0, 0.0),
        Material::Glass => (0.85, 1800.0, 0.35),
        Material::Cloth => (0.08, 0.0, 0.0),
    };
    let n = (dur * rate) as usize;
    let mut out = Vec::with_capacity(n);
    let mut y = 0.0f32;
    let crunch = matches!(mat, Material::Sand | Material::Grass);
    for i in 0..n {
        let t = i as f32 / rate;
        let env = (1.0 - t / dur).max(0.0).powf(if matches!(sfx, Sfx::Step(_)) { 3.0 } else { 2.0 });
        let mut x = noise();
        if crunch && (i / 300) % 3 == 0 {
            x *= 0.3;
        }
        y += lp * (x - y);
        let s = y * (1.0 - tone_mix) + (t * tone * std::f32::consts::TAU).sin() * tone_mix * (1.0 - t / dur);
        out.push(s * env * gain);
    }
    out
}
