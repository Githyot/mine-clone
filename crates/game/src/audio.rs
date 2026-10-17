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
    /// The player gets hurt.
    Hurt,
    /// The player dies.
    Death,
    /// Back to life at the spawn point.
    Respawn,
    /// A mob gets hit.
    MobHit,
    Explode,
    /// Mining tick while a block is being broken.
    Hit(Material),
    /// Falling into water; the value is how hard (0..1).
    Splash(f32),
    /// A stroke while swimming at the surface.
    Swim,
    /// Climbing out of water.
    WaterExit,
    /// Air bubbles under water.
    Bubbles,
    /// An item picked up from the ground.
    Pickup,
    /// Experience collected.
    Xp,
    /// A bite of food.
    Eat,
    /// Lava meeting water.
    Fizz,
    /// Far thunder; the value is how close (0..1).
    Thunder(f32),
    /// Two seconds of rain on the ground, played back to back while it rains.
    Rain,
    /// A spell leaving a wand, staff or book; the value picks the school's voice.
    Cast(u8),
    /// A spell hitting something.
    SpellHit(u8),
    /// A piston pushing out (true) or pulling back.
    Piston(bool),
    /// A nether portal lighting or pulling the player in: a deep wavering hum.
    Portal,
    /// A tune on an instrument (Immersive Melodies idea): instrument 0 lute, 1 flute, 2 cello.
    Melody(u8, u8),
    /// The sounds of the immersion stage: voices, ambience, body and interface (sfx_extra.rs).
    Fx(crate::sfx_extra::Fx),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Stone,
    Wood,
    Grass,
    Sand,
    Glass,
    Cloth,
    Dirt,
    Gravel,
    Leaves,
    Snow,
    Metal,
    Ore,
}

impl Material {
    /// How a block sounds when walked on, hit, placed or broken, from its name.
    pub fn of(name: &str) -> Material {
        let has = |s: &str| name.contains(s);
        if has("glass") || name == "ice" || name == "glowstone" {
            Material::Glass
        } else if has("_ore") {
            Material::Ore
        } else if name.ends_with("iron_block") || name.ends_with("gold_block") || name.ends_with("diamond_block") || has("anvil") || has("iron_door") {
            Material::Metal
        } else if has("leaves") || has("sapling") || has("mushroom") || name == "vine" || name == "lily_pad" {
            Material::Leaves
        } else if has("log") || has("planks") || has("crafting") || name == "bookshelf" || has("oak_") || has("chest") || has("door") || has("fence") || has("ladder") {
            Material::Wood
        } else if has("wool") || name == "sponge" || has("_bed") || has("carpet") {
            Material::Cloth
        } else if has("snow") {
            Material::Snow
        } else if name == "gravel" {
            Material::Gravel
        } else if name == "sand" || name == "red_sand" || name == "clay" {
            Material::Sand
        } else if has("leaves") || name == "poppy" || name == "dandelion" || name == "dead_bush" || has("sapling") || has("short_grass") || name == "melon" || name == "pumpkin" || name == "cactus" {
            Material::Leaves
        } else if name == "dirt" || name == "farmland" || name == "coarse_dirt" || name == "mud" {
            Material::Dirt
        } else if has("grass") || name == "tnt" {
            Material::Grass
        } else {
            Material::Stone
        }
    }

    #[allow(dead_code)]
    pub const ALL: [Material; 12] = [
        Material::Stone,
        Material::Wood,
        Material::Grass,
        Material::Sand,
        Material::Glass,
        Material::Cloth,
        Material::Dirt,
        Material::Gravel,
        Material::Leaves,
        Material::Snow,
        Material::Metal,
        Material::Ore,
    ];
}

pub struct Audio {
    _stream: OutputStream,
    pub(crate) handle: OutputStreamHandle,
    music: Sink,
    tracks: Vec<Track>,
    /// Seconds until the next song may start.
    pub next_in: f32,
    pub now_playing: Option<String>,
    rng: u64,
    music_folder: PathBuf,
    /// How the space around the player colours sound effects (Sound Physics idea).
    pub room: Room,
}

/// The space around the listener: how much echo comes back, after how long, and whether
/// the ears are under water.
#[derive(Clone, Copy, Default, Debug)]
pub struct Room {
    pub echo: f32,
    pub delay: f32,
    pub underwater: bool,
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
            room: Room::default(),
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
        let samples = room_sound(crate::sfx::synth(sfx, seed), self.room);
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

/// Echo off the walls of a cave or a closed room, and the dull muffle of being under water.
/// The echoes are filtered darker than the sound itself, so nothing gets brighter.
pub(crate) fn room_sound(mut s: Vec<f32>, room: Room) -> Vec<f32> {
    const RATE: f32 = 44100.0;
    if room.underwater {
        let mut lp = 0.0;
        for v in s.iter_mut() {
            lp += (*v - lp) * 0.06;
            *v = lp * 1.4;
        }
    }
    if room.echo < 0.05 || s.is_empty() {
        return s;
    }
    let d = ((room.delay.clamp(0.03, 0.3)) * RATE) as usize;
    let fb = (room.echo * 0.55).min(0.6);
    let tail = d * 6;
    s.resize(s.len() + tail, 0.0);
    let mut lp = 0.0;
    for i in d..s.len() {
        lp += (s[i - d] - lp) * 0.35;
        s[i] += lp * fb;
    }
    // Fade the very end so the tail never clicks off.
    let n = s.len();
    let fade = (RATE * 0.05) as usize;
    for k in 0..fade.min(n) {
        s[n - 1 - k] *= k as f32 / fade as f32;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cave_echo_adds_a_tail_and_stays_quiet() {
        let dry: Vec<f32> = (0..4410).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
        let wet = room_sound(dry.clone(), Room { echo: 1.0, delay: 0.1, underwater: false });
        assert!(wet.len() > dry.len());
        assert!(wet.iter().all(|v| v.abs() < 1.5));
        assert_eq!(room_sound(dry.clone(), Room::default()).len(), dry.len());
    }
}
