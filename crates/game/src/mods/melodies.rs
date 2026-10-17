//! Immersive Melodies idea: instruments that play whole melodies. Besides the lute, flute and
//! cello there are a didgeridoo, a bagpipe, a piano, a tiny drum and a handpan, all tuned low.
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

/// The samples of melody `tune` on instrument `inst`.
pub fn synth_tune(inst: u8, tune: u16, seed: u32) -> Vec<f32> {
    let Some(m) = store().lock().ok().and_then(|s| s.get(tune as usize).cloned()) else { return vec![0.0; 100] };
    let beat = m.beat * if inst == 2 { 1.25 } else { 1.0 };
    let total: f32 = (m.notes.iter().map(|n| n.1).sum::<f32>() * beat + 1.5).min(MAX_SECONDS + 1.5);
    let mut out = vec![0.0f32; (total * RATE) as usize];
    let mut rng = Noise(seed | 1);
    let mut t0 = 0.0;
    // Drones under the didgeridoo and bagpipe.
    if inst == 3 || inst == 4 {
        let f = if inst == 3 { 73.4 } else { 110.0 };
        let mut lp = Lp::new(if inst == 3 { 420.0 } else { 600.0 });
        let n = out.len();
        for (k, o) in out.iter_mut().enumerate() {
            let t = k as f32 / RATE;
            let env = (t / 0.3).min(1.0) * ((n - k) as f32 / RATE / 0.6).min(1.0);
            let wob = 1.0 + 0.5 * (t * 2.3 * TAU).sin();
            let x = (t * f * TAU).sin() + 0.6 * (t * f * 2.0 * TAU).sin() * wob + 0.3 * ((t * f).fract() * 2.0 - 1.0);
            *o += lp.run(x) * env * 0.18;
        }
    }
    for &(note, beats) in &m.notes {
        let dur = beats * beat;
        if t0 > MAX_SECONDS {
            break;
        }
        if let Some(midi) = note {
            let f = freq(midi - if inst == 2 { 12.0 } else { 0.0 });
            let start = (t0 * RATE) as usize;
            let tail = match inst {
                0 | 5 | 7 => 1.2,
                6 => 0.35,
                _ => 0.12,
            };
            let len = ((dur + tail) * RATE) as usize;
            let mut lp = Lp::new(match inst {
                0 => 1100.0,
                1 => 900.0,
                4 => 950.0,
                5 => 1200.0,
                6 => 300.0,
                7 => 1000.0,
                _ => 800.0,
            });
            if inst == 0 {
                // Plucked string.
                let n = (RATE / f) as usize;
                let mut buf: Vec<f32> = (0..n.max(2)).map(|_| rng.next() * 0.6).collect();
                for k in 0..len {
                    let i = k % buf.len();
                    let j = (k + 1) % buf.len();
                    let v = (buf[i] + buf[j]) * 0.4985;
                    buf[i] = v;
                    if let Some(o) = out.get_mut(start + k) {
                        *o += lp.run(v) * 0.9;
                    }
                }
            } else {
                let mut ph = 0.0f32;
                for k in 0..len {
                    let t = k as f32 / RATE;
                    let x = match inst {
                        // Flute: breathy sine.
                        1 => {
                            ph += f * (1.0 + (t * 5.2 * TAU).sin() * 0.004) / RATE;
                            let env = (t / 0.08).min(1.0) * (((dur + 0.12) - t) / 0.12).clamp(0.0, 1.0);
                            ((ph.fract() * TAU).sin() + 0.15 * (ph.fract() * 2.0 * TAU).sin() + rng.next() * 0.04) * env * 0.4
                        }
                        // Cello: bowed saw.
                        2 => {
                            ph += f * (1.0 + (t * 5.0 * TAU).sin() * 0.005) / RATE;
                            let env = (t / 0.1).min(1.0) * (((dur + 0.12) - t) / 0.12).clamp(0.0, 1.0);
                            ((ph.fract() * 2.0 - 1.0) * 0.6 + (ph.fract() * TAU).sin() * 0.5) * env * 0.35
                        }
                        // Didgeridoo: a growl on top of the drone, an octave down.
                        3 => {
                            ph += f * 0.5 / RATE;
                            let env = (t / 0.05).min(1.0) * (-t / (dur + 0.1)).exp() * (((dur + 0.12) - t) / 0.12).clamp(0.0, 1.0);
                            ((ph.fract() * TAU).sin() + 0.5 * (ph.fract() * 3.0 * TAU).sin()) * env * 0.35
                        }
                        // Bagpipe chanter: a reedy pulse.
                        4 => {
                            ph += f / RATE;
                            let env = (t / 0.03).min(1.0) * (((dur + 0.12) - t) / 0.12).clamp(0.0, 1.0);
                            (if ph.fract() < 0.3 { 1.0 } else { -0.43 }) * env * 0.25
                        }
                        // Piano: a struck string, slightly stretched partials, fading.
                        5 => {
                            let env = (t / 0.004).min(1.0) * (-t / 0.9).exp();
                            ((t * f * TAU).sin() + 0.45 * (t * f * 2.003 * TAU).sin() + 0.2 * (t * f * 3.01 * TAU).sin()) * env * 0.35
                        }
                        // Tiny drum: a low thump whose pitch drops.
                        6 => {
                            let df = 55.0 + (f - 55.0).max(0.0) * 0.25;
                            ph += df * (1.0 + 0.6 * (-t / 0.03).exp()) / RATE;
                            let env = (-t / 0.18).exp();
                            ((ph.fract() * TAU).sin() + rng.next() * 0.25 * (-t / 0.02).exp()) * env * 0.6
                        }
                        // Handpan: a soft bell with its octave and a fifth above.
                        _ => {
                            let env = (t / 0.01).min(1.0) * (-t / 1.1).exp();
                            ((t * f * TAU).sin() + 0.35 * (t * f * 2.0 * TAU).sin() + 0.12 * (t * f * 2.98 * TAU).sin()) * env * 0.35
                        }
                    };
                    if let Some(o) = out.get_mut(start + k) {
                        *o += lp.run(x);
                    }
                }
            }
        }
        t0 += dur;
    }
    out
}

#[derive(Default)]
pub struct Melodies {
    /// The melody picked for the instruments.
    pub pick: usize,
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
        let secs = store().lock().ok().and_then(|s| s.get(pick).map(|m| m.notes.iter().map(|n| n.1).sum::<f32>() * m.beat)).unwrap_or(6.0);
        self.melody_cd = secs.clamp(2.0, MAX_SECONDS);
        self.anim.start_swing();
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
        }
    }
}
