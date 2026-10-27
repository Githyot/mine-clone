//! Immersive Melodies idea: instruments that play whole melodies. Besides the lute, flute and
//! cello there are a didgeridoo, a bagpipe, a piano, a tiny drum and a handpan, all tuned low
//! and played with real recordings (CC0, see assets/sounds/instruments/CREDITS.md).
//! Sneak + right click picks the next melody, right click plays it. Melodies are written in a
//! small part of ABC notation: the built-in ones are our own tunes, and any `.abc` file the
//! player drops in the `melodies` folder joins the list.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use glam::Vec3;

use crate::audio::Sfx;
use crate::Game;

const RATE: f32 = 44100.0;
const TAU: f32 = std::f32::consts::TAU;
/// Longest a melody may play, in seconds.
const MAX_SECONDS: f32 = 40.0;

/// Instrument items, in the order of `Sfx::Tune`'s first number.
pub const INSTRUMENTS: [&str; 8] = ["lute", "flute", "cello", "didgeridoo", "bagpipe", "piano", "tiny_drum", "handpan"];

#[derive(Clone, Debug)]
pub struct Melody {
    pub title: String,
    /// (MIDI note or None for a rest, length in beats).
    pub notes: Vec<(Option<f32>, f32)>,
    /// Seconds per beat.
    pub beat: f32,
}

/// Our own tunes, in ABC.
const BUILT_IN: &[&str] = &[
    "T:Canção do Vale\nL:1/8\nK:Dm\nD2 F2 A2 d2 | c2 A2 F4 | G2 A2 c2 A2 | D6 z2 | F2 A2 G2 E2 | D8 |",
    "T:Fogueira na Taiga\nL:1/8\nK:Am\nA,2 C2 E2 A2 | G2 E2 D4 | C2 D2 E2 G2 | A,6 z2 | E2 D2 C2 A,2 | A,8 |",
    "T:Barcos no Porto\nL:1/4\nK:G\nG, B, D | G3 | F E D | E3 | C E D | B, A, G, | G,3 |",
    "T:Dança do Moinho\nL:1/8\nK:D\nD F A F D F A F | G B d B A4 | F A d A F A d A | E G F E D4 |",
    "T:Noite na Caverna\nL:1/4\nK:Em\nE, G, B, | A, G, F, | E,2 z | G, B, E | D B, A, | E,3 |",
    "T:Marcha dos Guardas\nL:1/8\nK:C\nC2 C2 G,2 G,2 | C2 E2 G4 | F2 E2 D2 C2 | D2 G,2 C4 |",
];

fn store() -> &'static Mutex<Vec<Melody>> {
    static S: OnceLock<Mutex<Vec<Melody>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(BUILT_IN.iter().filter_map(|t| parse_abc(t, "")).collect()))
}

/// Add the player's `.abc` files (once per run).
pub fn load_custom(dir: &Path) {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        if !dir.exists() {
            let _ = std::fs::create_dir_all(dir);
            let _ = std::fs::write(
                dir.join("LEIA-ME.txt"),
                "Coloque aqui melodias em notação ABC (.abc). Cada arquivo vira uma melodia para os instrumentos:\n\
                 segure Shift e clique com o botão direito para escolher, clique com o direito para tocar.\n",
            );
        }
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut files: Vec<_> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("abc"))).collect();
        files.sort();
        let mut s = store().lock().unwrap();
        for p in files {
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(m) = std::fs::read_to_string(&p).ok().and_then(|t| parse_abc(&t, &stem)) {
                s.push(m);
            }
        }
        log::info!("{} melodies", s.len());
    });
}

pub fn melody_count() -> usize {
    store().lock().map(|s| s.len()).unwrap_or(0)
}

pub fn melody_title(i: usize) -> String {
    store().lock().ok().and_then(|s| s.get(i).map(|m| m.title.clone())).unwrap_or_default()
}

/// Parse a tune in (a small part of) ABC notation: `T:` title, `L:` unit length, `Q:` tempo,
/// notes A-G/a-g with `^ _ =` accidentals, `'` and `,` octaves, lengths like `2`, `/2`, `3/2`,
/// and `z` rests. Bars, chords' extra notes and decorations are skipped.
pub fn parse_abc(text: &str, fallback_title: &str) -> Option<Melody> {
    let mut title = fallback_title.to_string();
    let mut unit = 1.0 / 8.0;
    let mut beat = 0.32;
    let mut body = String::new();
    for line in text.lines() {
        let l = line.trim();
        if l.len() > 1 && l.as_bytes()[1] == b':' && l.as_bytes()[0].is_ascii_alphabetic() {
            let v = l[2..].trim();
            match &l[..1] {
                "T" if title.is_empty() || title == fallback_title => title = v.to_string(),
                "L" => {
                    if let Some((a, b)) = v.split_once('/') {
                        if let (Ok(a), Ok(b)) = (a.trim().parse::<f32>(), b.trim().parse::<f32>()) {
                            unit = a / b.max(1.0);
                        }
                    }
                }
                "Q" => {
                    let bpm = v.rsplit('=').next().and_then(|n| n.trim().parse::<f32>().ok());
                    if let Some(b) = bpm.filter(|b| *b > 20.0) {
                        beat = (60.0 / b).clamp(0.15, 1.2);
                    }
                }
                _ => {}
            }
            continue;
        }
        if !l.starts_with('%') {
            body.push_str(l);
            body.push(' ');
        }
    }
    let c: Vec<char> = body.chars().collect();
    let mut i = 0;
    let mut notes = Vec::new();
    let mut in_chord = false;
    let mut chord_done = false;
    while i < c.len() {
        let ch = c[i];
        match ch {
            '[' => {
                in_chord = true;
                chord_done = false;
                i += 1;
            }
            ']' => {
                in_chord = false;
                i += 1;
            }
            '"' => {
                // Chord names and annotations.
                i += 1;
                while i < c.len() && c[i] != '"' {
                    i += 1;
                }
                i += 1;
            }
            '^' | '_' | '=' | 'A'..='G' | 'a'..='g' | 'z' | 'Z' => {
                let mut acc = 0.0;
                while i < c.len() && matches!(c[i], '^' | '_' | '=') {
                    acc += match c[i] {
                        '^' => 1.0,
                        '_' => -1.0,
                        _ => 0.0,
                    };
                    i += 1;
                }
                let Some(&n) = c.get(i) else { break };
                i += 1;
                let pitch = match n.to_ascii_uppercase() {
                    'C' => Some(0.0),
                    'D' => Some(2.0),
                    'E' => Some(4.0),
                    'F' => Some(5.0),
                    'G' => Some(7.0),
                    'A' => Some(9.0),
                    'B' => Some(11.0),
                    _ => None,
                };
                let mut midi = pitch.map(|p| 60.0 + p + acc + if n.is_ascii_lowercase() { 12.0 } else { 0.0 });
                while i < c.len() && matches!(c[i], '\'' | ',') {
                    if let Some(m) = midi.as_mut() {
                        *m += if c[i] == '\'' { 12.0 } else { -12.0 };
                    }
                    i += 1;
                }
                // Length: digits, then an optional /digits.
                let mut num = String::new();
                while i < c.len() && c[i].is_ascii_digit() {
                    num.push(c[i]);
                    i += 1;
                }
                let mut len: f32 = num.parse().unwrap_or(1.0);
                if i < c.len() && c[i] == '/' {
                    i += 1;
                    let mut den = String::new();
                    while i < c.len() && c[i].is_ascii_digit() {
                        den.push(c[i]);
                        i += 1;
                    }
                    len /= den.parse::<f32>().unwrap_or(2.0);
                }
                if in_chord && chord_done {
                    continue;
                }
                chord_done = in_chord;
                notes.push((midi, len * unit * 4.0));
            }
            _ => i += 1,
        }
    }
    if notes.iter().all(|n| n.0.is_none()) {
        return None;
    }
    // Keep it low and warm: move whole octaves until the top note is at most E4.
    let hi = notes.iter().filter_map(|n| n.0).fold(f32::MIN, f32::max);
    let lo = notes.iter().filter_map(|n| n.0).fold(f32::MAX, f32::min);
    let mut shift = 0.0;
    while hi + shift > 64.0 {
        shift -= 12.0;
    }
    while lo + shift < 36.0 && hi + shift + 12.0 <= 64.0 {
        shift += 12.0;
    }
    for n in notes.iter_mut() {
        if let Some(m) = n.0.as_mut() {
            *m += shift;
        }
    }
    Some(Melody { title: if title.is_empty() { "Melodia".into() } else { title }, notes, beat })
}

fn freq(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

struct Lp {
    a: f32,
    y1: f32,
    y2: f32,
}

impl Lp {
    fn new(cut: f32) -> Lp {
        Lp { a: 1.0 - (-TAU * cut / RATE).exp(), y1: 0.0, y2: 0.0 }
    }
    fn run(&mut self, x: f32) -> f32 {
        self.y1 += self.a * (x - self.y1);
        self.y2 += self.a * (self.y1 - self.y2);
        self.y2
    }
}

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

/// Real recordings the instruments play with (CC0, VS Chamber Orchestra Community Edition,
/// see assets/sounds/instruments/CREDITS.md): (set, MIDI note it sounds, Ogg Vorbis bytes).
/// A note is played with the nearest recording of its set, retuned by changing its speed.
static CLIPS: &[(&str, f32, &[u8])] = &[
    ("lute", 45.0, include_bytes!("../../../../assets/sounds/instruments/lute_0.ogg")),
    ("lute", 48.0, include_bytes!("../../../../assets/sounds/instruments/lute_1.ogg")),
    ("lute", 52.0, include_bytes!("../../../../assets/sounds/instruments/lute_2.ogg")),
    ("lute", 55.0, include_bytes!("../../../../assets/sounds/instruments/lute_3.ogg")),
    ("lute", 59.0, include_bytes!("../../../../assets/sounds/instruments/lute_4.ogg")),
    ("lute", 62.0, include_bytes!("../../../../assets/sounds/instruments/lute_5.ogg")),
    ("cello", 36.0, include_bytes!("../../../../assets/sounds/instruments/cello_0.ogg")),
    ("cello", 43.0, include_bytes!("../../../../assets/sounds/instruments/cello_1.ogg")),
    ("cello", 50.0, include_bytes!("../../../../assets/sounds/instruments/cello_2.ogg")),
    ("cello", 57.0, include_bytes!("../../../../assets/sounds/instruments/cello_3.ogg")),
    ("cello", 64.0, include_bytes!("../../../../assets/sounds/instruments/cello_4.ogg")),
    ("flute", 60.0, include_bytes!("../../../../assets/sounds/instruments/flute_0.ogg")),
    ("flute", 64.0, include_bytes!("../../../../assets/sounds/instruments/flute_1.ogg")),
    ("flute", 69.0, include_bytes!("../../../../assets/sounds/instruments/flute_2.ogg")),
    ("chanter", 50.0, include_bytes!("../../../../assets/sounds/instruments/chanter_0.ogg")),
    ("chanter", 53.0, include_bytes!("../../../../assets/sounds/instruments/chanter_1.ogg")),
    ("chanter", 58.0, include_bytes!("../../../../assets/sounds/instruments/chanter_2.ogg")),
    ("chanter", 62.0, include_bytes!("../../../../assets/sounds/instruments/chanter_3.ogg")),
    ("chanter", 65.0, include_bytes!("../../../../assets/sounds/instruments/chanter_4.ogg")),
    ("reed_low", 46.0, include_bytes!("../../../../assets/sounds/instruments/reed_low_0.ogg")),
    ("reed_low", 48.0, include_bytes!("../../../../assets/sounds/instruments/reed_low_1.ogg")),
    ("reed_low", 55.0, include_bytes!("../../../../assets/sounds/instruments/reed_low_2.ogg")),
    ("reed_low", 60.0, include_bytes!("../../../../assets/sounds/instruments/reed_low_3.ogg")),
    ("piano", 36.0, include_bytes!("../../../../assets/sounds/instruments/piano_0.ogg")),
    ("piano", 43.0, include_bytes!("../../../../assets/sounds/instruments/piano_1.ogg")),
    ("piano", 48.0, include_bytes!("../../../../assets/sounds/instruments/piano_2.ogg")),
    ("piano", 55.0, include_bytes!("../../../../assets/sounds/instruments/piano_3.ogg")),
    ("piano", 60.0, include_bytes!("../../../../assets/sounds/instruments/piano_4.ogg")),
    ("piano", 67.0, include_bytes!("../../../../assets/sounds/instruments/piano_5.ogg")),
    ("drum", 48.0, include_bytes!("../../../../assets/sounds/instruments/drum_0.ogg")),
    ("drum", 48.0, include_bytes!("../../../../assets/sounds/instruments/drum_1.ogg")),
    ("drum", 36.0, include_bytes!("../../../../assets/sounds/instruments/drum_2.ogg")),
    ("marimba", 59.0, include_bytes!("../../../../assets/sounds/instruments/marimba_0.ogg")),
    ("marimba", 65.0, include_bytes!("../../../../assets/sounds/instruments/marimba_1.ogg")),
    ("marimba", 72.0, include_bytes!("../../../../assets/sounds/instruments/marimba_2.ogg")),
    ("marimba", 79.0, include_bytes!("../../../../assets/sounds/instruments/marimba_3.ogg")),
];

/// Every clip decoded to mono samples (once), with its sample rate.
fn decoded() -> &'static Vec<(Vec<f32>, f32)> {
    static D: OnceLock<Vec<(Vec<f32>, f32)>> = OnceLock::new();
    D.get_or_init(|| {
        use rodio::Source;
        CLIPS
            .iter()
            .map(|(_, _, bytes)| match rodio::Decoder::new(std::io::Cursor::new(*bytes)) {
                Ok(src) => {
                    let (ch, sr) = (src.channels().max(1) as usize, src.sample_rate() as f32);
                    let all: Vec<f32> = src.convert_samples::<f32>().collect();
                    (all.chunks(ch).map(|c| c.iter().sum::<f32>() / ch as f32).collect(), sr)
                }
                Err(_) => (Vec::new(), RATE),
            })
            .collect()
    })
}

/// The clip of `set` closest to `midi` (round-robin among equals with `k`).
fn nearest(set: &str, midi: f32, k: usize) -> Option<usize> {
    let best = CLIPS.iter().filter(|c| c.0 == set).map(|c| (c.1 - midi).abs()).fold(f32::MAX, f32::min);
    let near: Vec<usize> = (0..CLIPS.len()).filter(|&i| CLIPS[i].0 == set && (CLIPS[i].1 - midi).abs() <= best + 0.01).collect();
    near.get(k % near.len().max(1)).copied()
}

/// Add clip `i` into `out` from `start`, retuned to `midi`, for `len` output samples. A held
/// note longer than the recording keeps going by crossfading back into its steady middle.
fn add_clip(out: &mut [f32], start: usize, i: usize, midi: f32, len: usize, gain: f32) {
    let (src, sr) = &decoded()[i];
    if src.len() < 64 {
        return;
    }
    let step = sr / RATE * 2f32.powf((midi - CLIPS[i].1) / 12.0);
    let xfade = (0.2 * RATE) as usize;
    let release = (0.15 * RATE) as usize;
    // Output samples one pass of the recording lasts (its end already fades out).
    let pass = ((src.len() as f32 - 2.0) / step) as usize;
    let loop_from = (0.5 * sr) as f32;
    let mut seg_start = 0usize;
    let mut offset = 0.0f32;
    let mut first = true;
    while seg_start < len {
        let seg_len = (((src.len() as f32 - 2.0 - offset) / step) as usize).min(pass);
        if seg_len <= xfade * 2 {
            break;
        }
        for k in 0..seg_len {
            let at = seg_start + k;
            if at >= len {
                break;
            }
            let p = offset + k as f32 * step;
            let j = p as usize;
            let f = p - j as f32;
            let v = src[j] * (1.0 - f) + src[j + 1] * f;
            let fade_in = if first { 1.0 } else { (k as f32 / xfade as f32).min(1.0) };
            let end = (len - at).min(seg_len - k);
            let fade_out = if seg_start + seg_len >= len || end > xfade { (((len - at) as f32) / release as f32).min(1.0) } else { end as f32 / xfade as f32 };
            if let Some(o) = out.get_mut(start + at) {
                *o += v * gain * fade_in * fade_out;
            }
        }
        if seg_start + seg_len >= len || loop_from >= src.len() as f32 * 0.6 {
            break;
        }
        seg_start += seg_len - xfade;
        offset = loop_from;
        first = false;
    }
}

/// The samples of melody `tune` on instrument `inst`, played with real recordings: lute on
/// harp strings, flute, a cello section, a didgeridoo made of a low bassoon drone with breathy
/// low notes, a bagpipe of clarinet over a bassoon drone, an upright piano, a hand drum and
/// a marimba for the handpan. All kept low and warm.
pub fn synth_tune(inst: u8, tune: u16, seed: u32) -> Vec<f32> {
    let Some(m) = store().lock().ok().and_then(|s| s.get(tune as usize).cloned()) else { return vec![0.0; 100] };
    let beat = m.beat * if inst == 2 { 1.25 } else { 1.0 };
    let total: f32 = (m.notes.iter().map(|n| n.1).sum::<f32>() * beat + 1.5).min(MAX_SECONDS + 1.5);
    let mut out = vec![0.0f32; (total * RATE) as usize];
    let n_out = out.len();
    // Drones under the didgeridoo and the bagpipe, for the whole tune.
    match inst {
        3 => {
            if let Some(i) = nearest("reed_low", 38.0, 0) {
                add_clip(&mut out, 0, i, 38.0, n_out, 0.35);
            }
        }
        4 => {
            if let Some(i) = nearest("reed_low", 45.0, 0) {
                add_clip(&mut out, 0, i, 45.0, n_out, 0.22);
                add_clip(&mut out, 0, i, 33.0, n_out, 0.18);
            }
        }
        _ => {}
    }
    let (set, ring, gain, shift) = match inst {
        0 => ("lute", 1.0, 0.6, 0.0),
        1 => ("flute", 0.0, 0.5, 0.0),
        2 => ("cello", 0.0, 0.55, -12.0),
        3 => ("reed_low", 0.0, 0.3, -12.0),
        4 => ("chanter", 0.0, 0.45, 0.0),
        5 => ("piano", 1.1, 0.6, 0.0),
        6 => ("drum", 0.6, 0.7, 0.0),
        _ => ("marimba", 1.2, 0.6, 0.0),
    };
    let mut t0 = 0.0;
    for (k, &(note, beats)) in m.notes.iter().enumerate() {
        let dur = beats * beat;
        if t0 > MAX_SECONDS {
            break;
        }
        if let Some(midi) = note {
            let mut midi = midi + shift;
            // Recordings stretched too far sound wrong: keep the flute near its own range.
            if inst == 1 && midi < 53.0 {
                midi += 12.0;
            }
            let start = (t0 * RATE) as usize;
            // Plucked and struck notes ring on; held ones stop with the note.
            let len = ((if ring > 0.0 { dur + ring } else { dur * if inst == 3 { 0.8 } else { 1.0 } + 0.08 }) * RATE) as usize;
            // The drum keeps its own pitch, only nudged by the note, and alternates hands.
            let (pick, tuned) = if inst == 6 {
                let i = if midi < 45.0 { nearest("drum", 36.0, 0) } else { nearest("drum", 48.0, k + seed as usize) };
                (i, i.map(|i| CLIPS[i].1 + (midi - 48.0) * 0.25).unwrap_or(48.0))
            } else {
                (nearest(set, midi, k + seed as usize), midi)
            };
            if let Some(i) = pick {
                add_clip(&mut out, start, i, tuned, len.min(n_out.saturating_sub(start)), gain);
            }
        }
        t0 += dur;
    }
    // Every instrument equally loud; a gentle low-pass keeps the top soft.
    let mut lp = Lp::new(4200.0);
    let peak = out.iter().fold(0.0f32, |a, x| a.max(x.abs()));
    let k = 0.75 / peak.max(1e-4);
    for o in out.iter_mut() {
        *o = lp.run(*o * k);
    }
    out
}

/// How long melody `tune` lasts on instrument `inst`, in seconds.
pub fn melody_seconds(tune: usize, inst: u8) -> f32 {
    store().lock().ok().and_then(|s| s.get(tune).map(|m| m.notes.iter().map(|n| n.1).sum::<f32>() * m.beat * if inst == 2 { 1.25 } else { 1.0 })).unwrap_or(6.0)
}

/// The note sounding `t` seconds into melody `tune` on instrument `inst`, for the hands.
pub fn note_at(tune: usize, inst: u8, t: f32) -> crate::instruments::Play {
    let s = store().lock().ok();
    let Some(m) = s.as_ref().and_then(|s| s.get(tune)) else { return crate::instruments::Play::steady(t) };
    let beat = m.beat * if inst == 2 { 1.25 } else { 1.0 };
    let mut t0 = 0.0;
    let mut last = crate::instruments::Play { since: t, dur: 0.5, pitch: 0.4, note: 0, t };
    for (k, &(note, beats)) in m.notes.iter().enumerate() {
        let dur = beats * beat;
        if let Some(midi) = note {
            if t0 > t {
                break;
            }
            last = crate::instruments::Play { since: t - t0, dur, pitch: ((midi - 40.0) / 24.0).clamp(0.0, 1.0), note: k, t };
        }
        t0 += dur;
    }
    last
}

/// A melody being played by the player.
pub struct Playing {
    pub inst: usize,
    pub tune: usize,
    /// Its length in seconds (the cooldown it started with).
    pub len: f32,
}

#[derive(Default)]
pub struct Melodies {
    /// The melody picked for the instruments.
    pub pick: usize,
    pub playing: Option<Playing>,
}

impl Melodies {
    pub fn play_at(&self, inst: usize, tune: usize, t: f32) -> crate::instruments::Play {
        note_at(tune, inst as u8, t)
    }
}

impl Game {
    /// Right click with an instrument: play the picked melody; sneaking picks the next one.
    pub(crate) fn melodies_use(&mut self) -> bool {
        let Some(st) = self.held_stack() else { return false };
        let name = self.items.get(st.item).name.clone();
        let Some(inst) = INSTRUMENTS.iter().position(|n| *n == name) else { return false };
        load_custom(&self.base.join("melodies"));
        let n = melody_count().max(1);
        if self.crouch > 0.5 {
            self.mods.melodies.pick = (self.mods.melodies.pick + 1) % n;
            self.toast(format!("Melodia: {} ({}/{})", melody_title(self.mods.melodies.pick), self.mods.melodies.pick + 1, n), 2.5);
            return true;
        }
        if self.melody_cd > 0.0 {
            return true;
        }
        let pick = self.mods.melodies.pick % n;
        self.sfx(Sfx::Tune(inst as u8, pick as u16));
        let secs = melody_seconds(pick, inst as u8);
        self.melody_cd = secs.clamp(2.0, MAX_SECONDS);
        self.mods.melodies.playing = Some(Playing { inst, tune: pick, len: self.melody_cd });
        let glow = self.glow_layer();
        let at = self.body.eye() + self.forward() * 0.6;
        for k in 0..8 {
            let v = Vec3::new((k as f32 * 1.7).cos() * 0.4, 0.8 + k as f32 * 0.1, (k as f32 * 1.7).sin() * 0.4);
            self.particles.glow(at, v, glow, [0.5, 1.0, 0.6], 0.06, 2.0);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abc_parses_and_stays_low() {
        let m = parse_abc("T:Teste\nL:1/8\nK:C\nc2 d/2 e3/2 z2 | [ceg]4 ^f' |", "x").unwrap();
        assert_eq!(m.title, "Teste");
        assert_eq!(m.notes.len(), 6);
        assert!(m.notes[3].0.is_none());
        assert!((m.notes[1].1 - 0.25).abs() < 1e-6, "d/2 is a sixteenth: a quarter beat");
        assert!(m.notes.iter().filter_map(|n| n.0).all(|n| n <= 64.0));
        for t in BUILT_IN {
            assert!(parse_abc(t, "").is_some(), "{t}");
        }
    }

    #[test]
    fn every_instrument_sounds_low() {
        for inst in 0..INSTRUMENTS.len() as u8 {
            let v = synth_tune(inst, 0, 7);
            let peak = v.iter().fold(0.0f32, |a, x| a.max(x.abs()));
            assert!(peak > 0.02 && v.iter().all(|x| x.is_finite()), "instrument {inst}");
            let zc = v.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count();
            let bright = zc as f32 / (v.len() as f32 / RATE) / 2.0;
            assert!(bright < 900.0, "instrument {inst} too bright: {bright}");
            // MC_DUMP_TUNES=folder writes each one as a .wav to listen to.
            if let Ok(dir) = std::env::var("MC_DUMP_TUNES") {
                let mut w = Vec::new();
                let n = v.len() as u32 * 2;
                w.extend_from_slice(b"RIFF");
                w.extend_from_slice(&(36 + n).to_le_bytes());
                w.extend_from_slice(b"WAVEfmt ");
                w.extend_from_slice(&[16, 0, 0, 0, 1, 0, 1, 0]);
                w.extend_from_slice(&44100u32.to_le_bytes());
                w.extend_from_slice(&88200u32.to_le_bytes());
                w.extend_from_slice(&[2, 0, 16, 0]);
                w.extend_from_slice(b"data");
                w.extend_from_slice(&n.to_le_bytes());
                for x in &v {
                    w.extend_from_slice(&((x.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes());
                }
                let _ = std::fs::write(format!("{dir}/{}.wav", INSTRUMENTS[inst as usize]), w);
            }
        }
    }
}
