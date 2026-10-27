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
    /// One beat of an aircraft's engine (Immersive Aircraft idea): 0 piston engine, 1 rotor,
    /// 2 airship's slow chug. The second value is the throttle in quarters (0..4).
    Engine(u8, u8),
    /// A whole melody on an instrument (mods/melodies.rs): instrument, melody number.
    Tune(u8, u16),
    /// A note block: pitch 0..24 (semitones up from F#2) and instrument (0 plucked string,
    /// 1 low drum on stone, 2 wooden knock on wood, 3 double bass).
    Note(u8, u8),
}

/// Which volume slider of the Áudio options a sound follows (`Settings::volumes`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundCat {
    Records,
    Weather,
    Blocks,
    Creatures,
    Players,
    Ambient,
    Interface,
}

impl SoundCat {
    pub const ALL: [SoundCat; 7] =
        [SoundCat::Blocks, SoundCat::Creatures, SoundCat::Players, SoundCat::Ambient, SoundCat::Weather, SoundCat::Records, SoundCat::Interface];

    pub fn label(self) -> &'static str {
        match self {
            SoundCat::Records => "Discos e instrumentos",
            SoundCat::Weather => "Clima",
            SoundCat::Blocks => "Blocos",
            SoundCat::Creatures => "Criaturas",
            SoundCat::Players => "Jogador",
            SoundCat::Ambient => "Ambiente",
            SoundCat::Interface => "Interface",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            SoundCat::Records => "Toca-discos, blocos musicais e instrumentos.",
            SoundCat::Weather => "Chuva e trovões.",
            SoundCat::Blocks => "Quebrar, colocar e minerar blocos, pistões, explosões.",
            SoundCat::Creatures => "Animais, monstros e pessoas.",
            SoundCat::Players => "Passos, pulos, golpes, dano, comer, nadar, magias.",
            SoundCat::Ambient => "Vento, cavernas, ondas, lava, fogo, portais.",
            SoundCat::Interface => "Cliques nos menus e o inventário.",
        }
    }
}

impl Sfx {
    pub fn category(self) -> SoundCat {
        use crate::sfx_extra::Fx;
        match self {
            Sfx::Break(_) | Sfx::Place(_) | Sfx::Hit(_) | Sfx::Piston(_) | Sfx::Explode | Sfx::Fizz => SoundCat::Blocks,
            Sfx::Click | Sfx::Fx(Fx::InvOpen | Fx::InvClose) => SoundCat::Interface,
            Sfx::MobHit | Sfx::Fx(Fx::Clip(..)) => SoundCat::Creatures,
            Sfx::Thunder(_) | Sfx::Rain => SoundCat::Weather,
            Sfx::Melody(..) | Sfx::Tune(..) | Sfx::Note(..) => SoundCat::Records,
            Sfx::Portal
            | Sfx::Bubbles
            | Sfx::Fx(
                Fx::CaveDrone
                | Fx::CaveRumble
                | Fx::Drip
                | Fx::Wind(_)
                | Fx::Canopy
                | Fx::Waves
                | Fx::Lapping
                | Fx::LavaPop
                | Fx::FireCrackle
                | Fx::NetherDrone
                | Fx::SoulWind
                | Fx::EndHum,
            ) => SoundCat::Ambient,
            _ => SoundCat::Players,
        }
    }
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
        } else if has("leaves") || has("sapling") || has("propagule") || has("mushroom") || name == "vine" || name == "lily_pad" {
            Material::Leaves
        } else if has("log") || has("planks") || has("mangrove_") || has("crafting") || name == "bookshelf" || has("oak_") || has("chest") || has("door") || has("fence") || has("ladder") {
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
        } else if name == "dirt" || name == "farmland" || name == "coarse_dirt" || name == "mud" || name == "packed_mud" {
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
    /// A jukebox's record: one of the built-in tracks, played on its own while the music waits.
    disc: Sink,
    /// How the record sounds right now: what it stands on, the walls between it and the ear.
    pub disc_tone: std::sync::Arc<DiscTone>,
    tracks: Vec<Track>,
    /// Seconds until the next song may start.
    pub next_in: f32,
    pub now_playing: Option<String>,
    rng: u64,
    music_folder: PathBuf,
    /// How the space around the player colours sound effects (Sound Physics idea).
    pub room: Room,
    /// Recorded sounds from resource packs, by key (see `pack_key`); they replace ours.
    pack: std::collections::HashMap<&'static str, Vec<std::sync::Arc<[u8]>>>,
    /// Volume of each sound group, in `SoundCat::ALL` order (from the settings, every frame).
    gains: [f32; 7],
}

/// Every sound key a resource pack may fill, in Minecraft's sound folder layout.
pub const PACK_KEYS: [&str; 34] = [
    "dig/stone", "dig/wood", "dig/grass", "dig/sand", "dig/glass", "dig/cloth", "dig/gravel", "dig/snow",
    "step/stone", "step/wood", "step/grass", "step/sand", "step/cloth", "step/gravel", "step/snow", "step/ladder",
    "random/glass", "random/click", "random/explode", "random/pop", "random/orb", "random/eat", "random/fizz",
    "damage/hit", "ambient/weather/thunder", "ambient/weather/rain", "liquid/splash", "liquid/swim",
    "tile/piston/out", "tile/piston/in", "portal/portal", "random/levelup", "random/bow", "mob/zombie/hurt",
];

/// The pack sound that stands for one of ours, if a pack may replace it.
fn pack_key(s: Sfx) -> Option<&'static str> {
    let dig = |m: Material| match m {
        Material::Stone | Material::Ore | Material::Metal => "dig/stone",
        Material::Wood => "dig/wood",
        Material::Grass | Material::Leaves | Material::Dirt => "dig/grass",
        Material::Sand => "dig/sand",
        Material::Glass => "random/glass",
        Material::Cloth => "dig/cloth",
        Material::Gravel => "dig/gravel",
        Material::Snow => "dig/snow",
    };
    let step = |m: Material| match m {
        Material::Stone | Material::Ore | Material::Metal | Material::Glass => "step/stone",
        Material::Wood => "step/wood",
        Material::Grass | Material::Leaves | Material::Dirt => "step/grass",
        Material::Sand => "step/sand",
        Material::Cloth => "step/cloth",
        Material::Gravel => "step/gravel",
        Material::Snow => "step/snow",
    };
    Some(match s {
        Sfx::Break(m) => dig(m),
        Sfx::Place(m) => match m {
            Material::Glass => "dig/stone",
            m => dig(m),
        },
        Sfx::Step(m) | Sfx::Hit(m) => step(m),
        Sfx::Click => "random/click",
        Sfx::Hurt => "damage/hit",
        Sfx::Explode => "random/explode",
        Sfx::Pickup => "random/pop",
        Sfx::Xp => "random/orb",
        Sfx::Eat => "random/eat",
        Sfx::Fizz => "random/fizz",
        Sfx::Thunder(_) => "ambient/weather/thunder",
        Sfx::Rain => "ambient/weather/rain",
        Sfx::Splash(_) => "liquid/splash",
        Sfx::Swim => "liquid/swim",
        Sfx::Piston(true) => "tile/piston/out",
        Sfx::Piston(false) => "tile/piston/in",
        Sfx::Portal => "portal/portal",
        _ => return None,
    })
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
        let disc = Sink::try_new(&handle).ok()?;
        let mut a = Audio {
            _stream: stream,
            handle,
            music,
            disc,
            disc_tone: Default::default(),
            tracks: Vec::new(),
            next_in: 0.0,
            now_playing: None,
            rng: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1,
            music_folder: base.join(&settings.music_folder),
            room: Room::default(),
            pack: Default::default(),
            gains: [1.0; 7],
        };
        a.reload_tracks();
        // The first song comes fairly soon after joining, the rest after long silences.
        a.next_in = 20.0 + a.rand() * 40.0;
        Some(a)
    }

    /// Sounds from the enabled resource packs (none = all our own).
    pub fn set_pack_sounds(&mut self, pack: std::collections::HashMap<&'static str, Vec<std::sync::Arc<[u8]>>>) {
        if !pack.is_empty() {
            log::info!("{} sound effects from resource packs", pack.len());
        }
        self.pack = pack;
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
        // The master volume (sound_volume) scales the music and every sound group.
        self.music.set_volume(settings.music_volume * settings.sound_volume);
        for (g, c) in self.gains.iter_mut().zip(SoundCat::ALL) {
            *g = settings.volumes.of(c);
        }
        if !self.disc.empty() {
            // A record is playing: the background music stays quiet until it ends.
            if !self.music.empty() {
                self.music.stop();
                self.now_playing = None;
            }
            self.next_in = self.next_in.max(30.0);
            return;
        }
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

    /// Put a record on: the built-in track `id` (as in tracks.json), at once.
    pub fn play_disc(&mut self, id: &str) -> bool {
        self.disc.stop();
        let Some((_, bytes)) = embedded::MUSIC.iter().find(|(t, _)| *t == id) else { return false };
        match Decoder::new(Cursor::new(*bytes)) {
            Ok(d) => {
                self.disc.append(Toned::new(d.convert_samples::<f32>(), self.disc_tone.clone()));
                self.disc.play();
                true
            }
            Err(_) => false,
        }
    }

    /// A music box (tr_style.rs): the song with this title, on the record's sink.
    pub fn play_title_on_disc(&mut self, title: &str) -> bool {
        self.disc.stop();
        let Some(t) = self.tracks.iter().find(|t| t.title == title) else { return false };
        let ok = match &t.source {
            TrackSource::Builtin(b) => Decoder::new(Cursor::new(*b)).map(|d| self.disc.append(d)).is_ok(),
            TrackSource::File(p) => File::open(p).ok().and_then(|f| Decoder::new(BufReader::new(f)).ok()).map(|d| self.disc.append(d)).is_some(),
        };
        if ok {
            self.disc.play();
        }
        ok
    }

    pub fn stop_disc(&mut self) {
        self.disc.stop();
    }

    /// The record's loudness (it fades with the distance to its jukebox).
    pub fn set_disc_volume(&mut self, v: f32) {
        self.disc.set_volume(v);
    }

    pub fn disc_playing(&self) -> bool {
        !self.disc.empty()
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
        let cat = SoundCat::ALL.iter().position(|&c| c == sfx.category()).unwrap_or(0);
        let volume = volume * self.gains[cat];
        if volume <= 0.0 {
            return;
        }
        let r = self.rand();
        if let Some(list) = pack_key(sfx).and_then(|k| self.pack.get(k)).filter(|l| !l.is_empty()) {
            let bytes = list[(r * list.len() as f32) as usize % list.len()].clone();
            // A little pitch change each time, like the classic game, so repeats don't drone.
            let pitch = 0.92 + self.rand() * 0.16;
            if let Ok(d) = Decoder::new(Cursor::new(bytes)) {
                let _ = self.handle.play_raw(d.convert_samples::<f32>().speed(pitch).amplify(volume));
                return;
            }
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

/// The colour of a playing record, set by the game each frame and read by the audio thread.
/// `cut` is a low-pass amount (1 = as recorded, near 0 = heavily muffled), `echo` how much
/// comes back off hard surfaces and `delay` after how many seconds. Nothing here ever makes
/// the sound brighter, only duller or more distant.
pub struct DiscTone {
    cut: std::sync::atomic::AtomicU32,
    echo: std::sync::atomic::AtomicU32,
    delay: std::sync::atomic::AtomicU32,
}

impl Default for DiscTone {
    fn default() -> Self {
        let f = |v: f32| std::sync::atomic::AtomicU32::new(v.to_bits());
        DiscTone { cut: f(1.0), echo: f(0.0), delay: f(0.08) }
    }
}

impl DiscTone {
    pub fn set(&self, cut: f32, echo: f32, delay: f32) {
        use std::sync::atomic::Ordering::Relaxed;
        self.cut.store(cut.clamp(0.02, 1.0).to_bits(), Relaxed);
        self.echo.store(echo.clamp(0.0, 0.6).to_bits(), Relaxed);
        self.delay.store(delay.clamp(0.02, TONE_MAX_DELAY).to_bits(), Relaxed);
    }

    fn get(&self) -> (f32, f32, f32) {
        use std::sync::atomic::Ordering::Relaxed;
        let f = |a: &std::sync::atomic::AtomicU32| f32::from_bits(a.load(Relaxed));
        (f(&self.cut), f(&self.echo), f(&self.delay))
    }
}

const TONE_MAX_DELAY: f32 = 0.3;

/// A record streamed through the jukebox's tone: a one-pole low-pass per channel and a
/// darkened echo line. Settings glide over ~50 ms so moving or covering the box never clicks.
pub struct Toned<S> {
    inner: S,
    tone: std::sync::Arc<DiscTone>,
    channels: usize,
    rate: f32,
    lp: [f32; 2],
    elp: [f32; 2],
    ring: Vec<f32>,
    w: usize,
    n: usize,
    cur: (f32, f32, f32),
    target: (f32, f32, f32),
}

impl<S: Source<Item = f32>> Toned<S> {
    pub fn new(inner: S, tone: std::sync::Arc<DiscTone>) -> Self {
        let channels = (inner.channels() as usize).clamp(1, 2);
        let rate = inner.sample_rate() as f32;
        let len = ((TONE_MAX_DELAY * rate) as usize + 1) * channels;
        let now = tone.get();
        Toned { inner, tone, channels, rate, lp: [0.0; 2], elp: [0.0; 2], ring: vec![0.0; len], w: 0, n: 0, cur: now, target: now }
    }
}

impl<S: Source<Item = f32>> Iterator for Toned<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let x = self.inner.next()?;
        if self.n % 512 == 0 {
            self.target = self.tone.get();
        }
        let g = 0.0005;
        self.cur.0 += (self.target.0 - self.cur.0) * g;
        self.cur.1 += (self.target.1 - self.cur.1) * g;
        self.cur.2 += (self.target.2 - self.cur.2) * g;
        let (cut, echo, delay) = self.cur;
        let c = self.n % self.channels;
        self.n = self.n.wrapping_add(1);
        // Muffling loses some loudness; give a little of it back.
        self.lp[c] += (x - self.lp[c]) * cut;
        let y = self.lp[c] * (1.0 + (1.0 - cut) * 0.4);
        let len = self.ring.len();
        let d = ((delay * self.rate) as usize).max(1) * self.channels;
        let back = self.ring[(self.w + len - d.min(len - self.channels)) % len];
        self.elp[c] += (back - self.elp[c]) * 0.3;
        let out = y + self.elp[c] * echo;
        self.ring[self.w] = y + self.elp[c] * echo * 0.5;
        self.w = (self.w + 1) % len;
        Some(out.clamp(-1.0, 1.0))
    }
}

impl<S: Source<Item = f32>> Source for Toned<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.channels as u16
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
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

    fn tone_energy(cut: f32, echo: f32) -> (f32, f32) {
        // A low hum plus a fast wiggle; the wiggle is what muffling should take away.
        let rate = 44100;
        let src: Vec<f32> = (0..rate).map(|i| { let t = i as f32 / rate as f32; (t * 110.0 * std::f32::consts::TAU).sin() * 0.3 + (t * 5000.0 * std::f32::consts::TAU).sin() * 0.3 }).collect();
        let tone = std::sync::Arc::new(DiscTone::default());
        tone.set(cut, echo, 0.08);
        let out: Vec<f32> = Toned::new(rodio::buffer::SamplesBuffer::new(1, rate, src), tone).collect();
        let tail = &out[rate as usize / 2..];
        let hi: f32 = tail.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / tail.len() as f32;
        let lo: f32 = tail.iter().map(|v| v.abs()).sum::<f32>() / tail.len() as f32;
        (hi, lo)
    }

    #[test]
    fn wool_muffles_the_record_and_nothing_gets_brighter() {
        let (dry_hi, dry_lo) = tone_energy(1.0, 0.0);
        let (wool_hi, wool_lo) = tone_energy(0.08, 0.0);
        assert!(wool_hi < dry_hi * 0.4, "muffled {wool_hi} vs dry {dry_hi}");
        assert!(wool_lo > 0.05, "the low part still plays");
        // Stone sends an echo back: louder overall, but no brighter for it.
        let (stone_hi, stone_lo) = tone_energy(0.9, 0.4);
        assert!(stone_hi / stone_lo <= dry_hi / dry_lo * 1.05);
    }
}
