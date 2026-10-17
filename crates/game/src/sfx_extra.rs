//! More procedural sounds for an immersive world: animal and monster voices, ambience (caves,
//! wind, waves, the Nether, the End, night animals), body sounds (landing, jumping, a heartbeat
//! at low health, gasping for air) and interface sounds. Same rules as sfx.rs: no sound files,
//! nothing shrill. Voices sit low, and every sound comes out a little different each time.

use std::f32::consts::{PI, TAU};

const RATE: f32 = 44100.0;

/// An extra sound (played through `Sfx::Fx`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fx {
    /// A creature's voice: how it sounds, what it is saying, and how deep (1 = normal size).
    Voice(Voice, Call, f32),
    /// A long, low swell deep under ground.
    CaveDrone,
    /// Stones tumbling somewhere far off in the dark.
    CaveRumble,
    /// One drop of water landing.
    Drip,
    /// A gust of wind; the value is how strong (0..1).
    Wind(f32),
    /// Leaves moving in the wind over a forest.
    Canopy,
    /// A wave rolling in on a beach.
    Waves,
    /// Small water slapping against a shore.
    Lapping,
    /// Lava bubbling and popping.
    LavaPop,
    /// A fire crackling.
    FireCrackle,
    /// The Nether's deep, slow moan.
    NetherDrone,
    /// The hollow wind of the soul sand valleys.
    SoulWind,
    /// The End's empty hum.
    EndHum,
    /// An owl at night.
    Owl,
    /// A dove cooing by day.
    Dove,
    /// A frog in a swamp.
    Frog,
    /// "Lub-dub", at low health.
    Heartbeat,
    /// A deep breath after a long time under water.
    Gasp,
    /// Landing after a fall; the value is how hard (0..1).
    Land(f32),
    /// Pushing off the ground.
    Jump,
    /// A weapon or arm cutting the air.
    Swing,
    /// Walking through tall grass and bushes.
    Brush,
    /// Opening the inventory: a soft leather flap.
    InvOpen,
    /// Closing it.
    InvClose,
    /// Worn armour shifting as you walk: true for metal, false for leather.
    Armor(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Voice {
    /// Zombies.
    Groan,
    /// Skeletons: bones knocking together.
    Rattle,
    /// Spiders.
    Hiss,
    /// Cows and other big grazers.
    Moo,
    /// Pigs and boars.
    Grunt,
    /// Sheep.
    Bleat,
    /// Chickens.
    Cluck,
    /// Ducks and penguins.
    Quack,
    /// Small birds, kept to a low coo.
    Coo,
    /// Deer, foxes, rabbits: a soft snort through the nose.
    Snort,
    /// Bears and big cats.
    Growl,
    /// Ghosts, wraiths and void things.
    Wail,
    /// Witches, goblins, imps.
    Cackle,
    /// People: a short "hm" or "huh".
    Murmur,
    /// Giant stone and wood things.
    Rumble,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    Idle,
    Hurt,
    Death,
}

/// Which voice a mob has, from its name (None: it makes no sound of its own).
pub fn voice_of(name: &str, hostile: bool) -> Option<Voice> {
    let has = |s: &str| name.contains(s);
    Some(match name {
        "minecart" => return None,
        _ if has("zombie") => Voice::Groan,
        _ if has("skeleton") => Voice::Rattle,
        "spider" | "stalker" => Voice::Hiss,
        "cow" | "hippo" | "elephant" | "rhino" => Voice::Moo,
        "pig" | "boar" | "crimson_boar" => Voice::Grunt,
        "sheep" => Voice::Bleat,
        "chicken" => Voice::Cluck,
        "duck" | "penguin" => Voice::Quack,
        "bluejay" | "cardinal" => Voice::Coo,
        "deer" | "zebra" | "fox" | "rabbit" | "squirrel" => Voice::Snort,
        "bear" | "polar_bear" | "lion" => Voice::Growl,
        "lich" | "ember_wraith" | "void_blossom" => Voice::Wail,
        _ if has("void_walker") => Voice::Wail,
        "forest_guardian" | "obsidilith" | "void_guardian" => Voice::Rumble,
        "swamp_witch" | "goblin_trader" | "fire_imp" => Voice::Cackle,
        "player" | "brute" | "marauder" | "invoker" => Voice::Murmur,
        _ if has("npc_") => Voice::Murmur,
        _ if hostile => Voice::Groan,
        _ => Voice::Snort,
    })
}

impl Voice {
    /// Average seconds between idle calls.
    pub fn chattiness(self) -> f32 {
        match self {
            Voice::Cluck => 7.0,
            Voice::Hiss | Voice::Groan | Voice::Rumble => 9.0,
            Voice::Rattle | Voice::Grunt | Voice::Quack | Voice::Coo => 10.0,
            Voice::Wail => 11.0,
            Voice::Cackle | Voice::Murmur => 13.0,
            Voice::Bleat => 13.0,
            Voice::Moo => 15.0,
            Voice::Snort => 16.0,
            Voice::Growl => 18.0,
        }
    }
}

pub fn synth(fx: Fx, seed: u32) -> Vec<f32> {
    let mut rng = Rng(seed.wrapping_mul(2654435761).wrapping_add(0x9e37_79b9) | 1);
    let r = &mut rng;
    let out = match fx {
        Fx::Voice(v, c, depth) => voice(r, v, c, depth.clamp(0.5, 1.6)),
        Fx::CaveDrone => cave_drone(r),
        Fx::CaveRumble => cave_rumble(r),
        Fx::Drip => drip(r),
        Fx::Wind(s) => wind(r, s.clamp(0.0, 1.0), false),
        Fx::Canopy => wind(r, 0.5, true),
        Fx::Waves => waves(r),
        Fx::Lapping => lapping(r),
        Fx::LavaPop => lava_pop(r),
        Fx::FireCrackle => crackle(r),
        Fx::NetherDrone => nether_drone(r),
        Fx::SoulWind => soul_wind(r),
        Fx::EndHum => end_hum(r),
        Fx::Owl => owl(r),
        Fx::Dove => dove(r),
        Fx::Frog => frog(r),
        Fx::Heartbeat => heartbeat(),
        Fx::Gasp => gasp(r),
        Fx::Land(s) => land(r, s.clamp(0.0, 1.0)),
        Fx::Jump => whoosh(r, 0.16, 260.0, 520.0, 0.35),
        Fx::Swing => whoosh(r, 0.24, 220.0, 750.0, 0.5),
        Fx::Brush => brush(r),
        Fx::InvOpen => flap(r, true),
        Fx::InvClose => flap(r, false),
        Fx::Armor(metal) => armor(r, metal),
    };
    out.into_iter().map(|s| s.clamp(-1.0, 1.0)).collect()
}

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32
    }
    fn noise(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

/// Two one-pole low-pass filters in a row.
struct LowPass {
    a: f32,
    y1: f32,
    y2: f32,
}

impl LowPass {
    fn new(cutoff: f32) -> LowPass {
        LowPass {
            a: coef(cutoff),
            y1: 0.0,
            y2: 0.0,
        }
    }
    fn set(&mut self, cutoff: f32) {
        self.a = coef(cutoff);
    }
    fn run(&mut self, x: f32) -> f32 {
        self.y1 += self.a * (x - self.y1);
        self.y2 += self.a * (self.y1 - self.y2);
        self.y2
    }
}

fn coef(cutoff: f32) -> f32 {
    1.0 - (-TAU * cutoff.max(1.0) / RATE).exp()
}

/// Band of noise between two corners: a low-pass minus a lower low-pass.
struct Band {
    hi: LowPass,
    lo: LowPass,
}

impl Band {
    fn new(lo: f32, hi: f32) -> Band {
        Band {
            hi: LowPass::new(hi),
            lo: LowPass::new(lo),
        }
    }
    fn set(&mut self, lo: f32, hi: f32) {
        self.hi.set(hi);
        self.lo.set(lo);
    }
    fn run(&mut self, x: f32) -> f32 {
        let h = self.hi.run(x);
        h - self.lo.run(h)
    }
}

/// Two-pole resonator: rings at one frequency, like a vowel formant.
struct Resonator {
    b1: f32,
    b2: f32,
    y1: f32,
    y2: f32,
    gain: f32,
}

impl Resonator {
    fn new(freq: f32, bandwidth: f32) -> Resonator {
        let r = (-PI * bandwidth / RATE).exp();
        let b1 = 2.0 * r * (TAU * freq / RATE).cos();
        Resonator {
            b1,
            b2: -r * r,
            y1: 0.0,
            y2: 0.0,
            gain: 1.0 - r,
        }
    }
    fn run(&mut self, x: f32) -> f32 {
        let y = x * self.gain + self.b1 * self.y1 + self.b2 * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

fn samples(dur: f32) -> usize {
    (dur * RATE) as usize
}

/// Fade both ends so nothing clicks.
fn edges(v: &mut [f32], fade: f32) {
    let n = v.len();
    let f = samples(fade).min(n / 2);
    for k in 0..f {
        let g = k as f32 / f as f32;
        v[k] *= g;
        v[n - 1 - k] *= g;
    }
}

/// How a voice is built: a buzzing throat (pulses) and breath, shaped by mouth resonances.
struct Vox {
    f0: f32,
    f0_end: f32,
    dur: f32,
    /// Formants: (frequency, bandwidth, gain).
    formants: &'static [(f32, f32, f32)],
    breath: f32,
    /// How unsteady the throat is (growl, rasp).
    rough: f32,
    /// Vibrato (rate Hz, depth).
    vibrato: (f32, f32),
    /// Separate bursts in one call ("oink-oink", "bok bok bok").
    syllables: usize,
    /// How hard the start of each syllable punches up in pitch.
    kick: f32,
    voiced: f32,
}

fn vocal(rng: &mut Rng, v: &Vox) -> Vec<f32> {
    let n = samples(v.dur);
    let mut res: Vec<(Resonator, f32)> = v
        .formants
        .iter()
        .map(|&(f, bw, g)| (Resonator::new(f, bw), g))
        .collect();
    let mut breath_lp = LowPass::new(1400.0);
    let mut rough_lp = LowPass::new(30.0);
    let mut phase = 0.0f32;
    let syl = v.syllables.max(1);
    let slot = v.dur / syl as f32;
    // Each burst is most of its slot, the rest a gap.
    let on = if syl == 1 { 1.0 } else { 0.62 };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / RATE;
        let k = t / v.dur;
        let s = ((t / slot) as usize).min(syl - 1);
        let ts = t - s as f32 * slot;
        let u = ts / (slot * on);
        let env = if u < 1.0 {
            (ts / 0.012).min(1.0) * (1.0 - u).powf(0.8) * (u * PI).sin().max(0.0).powf(0.25)
        } else {
            0.0
        };
        let wobble = rough_lp.run(rng.noise()) * v.rough * 18.0;
        let vib = 1.0 + (t * v.vibrato.0 * TAU).sin() * v.vibrato.1;
        let pitch = (v.f0 + (v.f0_end - v.f0) * k)
            * (1.0 + v.kick * (-ts / 0.035).exp())
            * vib
            * (1.0 + wobble);
        phase += pitch.max(20.0) / RATE;
        let p = phase.fract();
        let pulse = ((1.0 - p).powf(6.0) - 0.14) * v.voiced;
        let breath = breath_lp.run(rng.noise()) * v.breath;
        let src = pulse + breath;
        let mut y = 0.0;
        for (r, g) in res.iter_mut() {
            y += r.run(src) * *g;
        }
        // Roughness also shakes the loudness, like a growl.
        let am = 1.0 + wobble.abs() * 2.0;
        out.push(y * env * am * 2.0);
    }
    edges(&mut out, 0.006);
    // Keep every voice at a similar loudness.
    let peak = out.iter().fold(0.0f32, |a, x| a.max(x.abs())).max(1e-4);
    let gain = 0.8 / peak;
    out.iter_mut().for_each(|x| *x *= gain);
    out
}

fn voice(rng: &mut Rng, v: Voice, call: Call, depth: f32) -> Vec<f32> {
    match v {
        Voice::Rattle => return rattle(rng, call),
        Voice::Hiss => return hiss(rng, call),
        Voice::Snort => return snort(rng, call),
        _ => {}
    }
    let jitter = rng.range(0.93, 1.07);
    let mut x = match v {
        Voice::Groan => Vox {
            f0: 92.0,
            f0_end: 78.0,
            dur: 1.1,
            formants: &[
                (480.0, 120.0, 1.3),
                (820.0, 150.0, 0.55),
                (180.0, 80.0, 0.8),
            ],
            breath: 0.3,
            rough: 0.18,
            vibrato: (4.0, 0.03),
            syllables: 1,
            kick: 0.08,
            voiced: 1.0,
        },
        Voice::Moo => Vox {
            f0: 118.0,
            f0_end: 98.0,
            dur: 1.35,
            formants: &[(400.0, 90.0, 1.3), (780.0, 140.0, 0.35), (230.0, 80.0, 0.8)],
            breath: 0.1,
            rough: 0.05,
            vibrato: (3.5, 0.015),
            syllables: 1,
            kick: 0.12,
            voiced: 1.0,
        },
        Voice::Grunt => Vox {
            f0: 135.0,
            f0_end: 112.0,
            dur: 0.42,
            formants: &[
                (620.0, 150.0, 1.2),
                (1050.0, 200.0, 0.35),
                (290.0, 100.0, 0.7),
            ],
            breath: 0.35,
            rough: 0.3,
            vibrato: (0.0, 0.0),
            syllables: 2,
            kick: 0.15,
            voiced: 1.0,
        },
        Voice::Bleat => Vox {
            f0: 168.0,
            f0_end: 150.0,
            dur: 0.85,
            formants: &[
                (680.0, 130.0, 1.2),
                (1100.0, 180.0, 0.4),
                (300.0, 90.0, 0.5),
            ],
            breath: 0.15,
            rough: 0.05,
            vibrato: (7.5, 0.07),
            syllables: 1,
            kick: 0.1,
            voiced: 1.0,
        },
        Voice::Cluck => Vox {
            f0: 220.0,
            f0_end: 200.0,
            dur: 0.42,
            formants: &[(780.0, 200.0, 1.0), (430.0, 120.0, 0.7)],
            breath: 0.4,
            rough: 0.2,
            vibrato: (0.0, 0.0),
            syllables: 3,
            kick: 0.2,
            voiced: 1.0,
        },
        Voice::Quack => Vox {
            f0: 200.0,
            f0_end: 175.0,
            dur: 0.4,
            formants: &[
                (860.0, 260.0, 1.0),
                (1250.0, 300.0, 0.3),
                (420.0, 150.0, 0.5),
            ],
            breath: 0.5,
            rough: 0.35,
            vibrato: (0.0, 0.0),
            syllables: 2,
            kick: 0.1,
            voiced: 1.0,
        },
        Voice::Coo => Vox {
            f0: 420.0,
            f0_end: 380.0,
            dur: 0.9,
            formants: &[(430.0, 70.0, 1.5)],
            breath: 0.06,
            rough: 0.0,
            vibrato: (5.0, 0.01),
            syllables: 3,
            kick: 0.06,
            voiced: 1.0,
        },
        Voice::Growl => Vox {
            f0: 68.0,
            f0_end: 58.0,
            dur: 1.0,
            formants: &[
                (380.0, 150.0, 1.0),
                (680.0, 200.0, 0.45),
                (140.0, 80.0, 1.0),
            ],
            breath: 0.45,
            rough: 0.6,
            vibrato: (0.0, 0.0),
            syllables: 1,
            kick: 0.05,
            voiced: 1.0,
        },
        Voice::Wail => Vox {
            f0: 108.0,
            f0_end: 86.0,
            dur: 1.7,
            formants: &[(560.0, 100.0, 1.0), (300.0, 90.0, 1.0)],
            breath: 0.7,
            rough: 0.05,
            vibrato: (5.0, 0.06),
            syllables: 1,
            kick: 0.0,
            voiced: 0.8,
        },
        Voice::Cackle => Vox {
            f0: 175.0,
            f0_end: 150.0,
            dur: 0.6,
            formants: &[
                (680.0, 150.0, 1.2),
                (1150.0, 200.0, 0.35),
                (320.0, 100.0, 0.5),
            ],
            breath: 0.4,
            rough: 0.15,
            vibrato: (0.0, 0.0),
            syllables: 4,
            kick: 0.15,
            voiced: 1.0,
        },
        Voice::Murmur => Vox {
            f0: 135.0,
            f0_end: 120.0,
            dur: 0.5,
            formants: &[(320.0, 80.0, 1.0), (900.0, 150.0, 0.3), (520.0, 110.0, 0.5)],
            breath: 0.15,
            rough: 0.05,
            vibrato: (0.0, 0.0),
            syllables: 2,
            kick: 0.1,
            voiced: 1.0,
        },
        Voice::Rumble => Vox {
            f0: 46.0,
            f0_end: 40.0,
            dur: 1.8,
            formants: &[(240.0, 100.0, 1.0), (120.0, 60.0, 1.2), (420.0, 160.0, 0.4)],
            breath: 0.25,
            rough: 0.35,
            vibrato: (0.0, 0.0),
            syllables: 1,
            kick: 0.04,
            voiced: 1.0,
        },
        Voice::Rattle | Voice::Hiss | Voice::Snort => unreachable!(),
    };
    // Bigger creatures sound deeper.
    x.f0 *= jitter / depth;
    x.f0_end *= jitter / depth;
    match call {
        Call::Idle => {}
        Call::Hurt => {
            // Short and punchy, pitched up at the start.
            x.dur = (x.dur * 0.45).max(0.2);
            x.syllables = 1;
            x.kick = 0.3;
            x.f0 *= 1.2;
            x.rough += 0.1;
        }
        Call::Death => {
            x.dur *= 1.6;
            x.syllables = 1;
            x.f0_end = x.f0 * 0.55;
            x.breath += 0.15;
        }
    }
    vocal(rng, &x)
}

/// A dry wooden knock, the building block of bone rattles.
fn knock(out: &mut [f32], at: usize, freq: f32, amp: f32, rng: &mut Rng) {
    let mut r = Resonator::new(freq, 160.0);
    let mut lp = LowPass::new(1200.0);
    for k in 0..samples(0.05) {
        let Some(o) = out.get_mut(at + k) else { break };
        let t = k as f32 / RATE;
        let x = if k < 60 { rng.noise() } else { 0.0 };
        *o += lp.run(r.run(x) * 6.0) * amp * (-t / 0.012).exp();
    }
}

fn rattle(rng: &mut Rng, call: Call) -> Vec<f32> {
    let (dur, count) = match call {
        Call::Idle => (0.45, 7),
        Call::Hurt => (0.25, 9),
        Call::Death => (1.0, 22),
    };
    let mut out = vec![0.0; samples(dur + 0.06)];
    for i in 0..count {
        let t = if call == Call::Death {
            (i as f32 / count as f32).powf(1.6) * dur
        } else {
            rng.next() * dur
        };
        let f = rng.range(380.0, 820.0);
        let amp = rng.range(0.4, 0.9)
            * if call == Call::Death {
                1.0 - t / (dur + 0.2)
            } else {
                1.0
            };
        knock(&mut out, samples(t), f, amp, rng);
    }
    edges(&mut out, 0.004);
    out
}

fn hiss(rng: &mut Rng, call: Call) -> Vec<f32> {
    let dur = match call {
        Call::Idle => 0.65,
        Call::Hurt => 0.25,
        Call::Death => 1.1,
    };
    let n = samples(dur);
    let mut band = Band::new(380.0, 1050.0);
    let chitter = rng.range(18.0, 26.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let k = t / dur;
            if call == Call::Death {
                band.set(380.0 - 200.0 * k, 1050.0 - 600.0 * k);
            }
            let trem = 0.55 + 0.45 * (t * chitter * TAU).sin();
            let env = (t / 0.02).min(1.0) * (1.0 - k).powf(1.3);
            band.run(rng.noise()) * trem * env * 2.4
        })
        .collect();
    edges(&mut out, 0.005);
    out
}

fn snort(rng: &mut Rng, call: Call) -> Vec<f32> {
    let (count, len) = match call {
        Call::Idle => (if rng.next() < 0.5 { 1 } else { 2 }, 0.16),
        Call::Hurt => (1, 0.14),
        Call::Death => (3, 0.22),
    };
    let gap = len + 0.06;
    let mut out = vec![0.0; samples(count as f32 * gap + 0.05)];
    for c in 0..count {
        let mut lp = LowPass::new(rng.range(520.0, 700.0));
        let start = samples(c as f32 * gap);
        let fall = if call == Call::Death {
            1.0 - c as f32 * 0.25
        } else {
            1.0
        };
        for k in 0..samples(len) {
            let t = k as f32 / RATE;
            let e = (t / 0.008).min(1.0) * (1.0 - t / len).powf(1.5);
            out[start + k] += lp.run(rng.noise()) * e * 2.4 * fall;
        }
    }
    edges(&mut out, 0.004);
    out
}

fn cave_drone(rng: &mut Rng) -> Vec<f32> {
    let dur = 6.0;
    let n = samples(dur);
    let f = rng.range(38.0, 52.0);
    let mut band = Band::new(70.0, 260.0);
    let mut swell = LowPass::new(0.6);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let k = t / dur;
            let env = (k * PI).sin().powf(1.5);
            let mov = 1.0 + (swell.run(rng.noise()) * 40.0).clamp(-0.3, 0.3);
            let tone = (t * f * TAU).sin() * 0.5
                + (t * f * 1.498 * TAU).sin() * 0.25
                + (t * f * 0.5 * TAU).sin() * 0.3;
            (tone * mov + band.run(rng.noise()) * 1.2) * env * 0.7
        })
        .collect();
    edges(&mut out, 0.05);
    out
}

fn cave_rumble(rng: &mut Rng) -> Vec<f32> {
    let dur = 2.4;
    let mut out = vec![0.0; samples(dur + 0.1)];
    let mut low = LowPass::new(160.0);
    for (i, o) in out.iter_mut().enumerate() {
        let t = i as f32 / RATE;
        *o = low.run(rng.noise())
            * 2.5
            * (t / 0.2).min(1.0)
            * (1.0 - t / (dur + 0.1)).max(0.0).powf(2.0);
    }
    // Stones knocking as they roll, fewer and softer as it settles.
    for i in 0..18 {
        let t = (i as f32 / 18.0).powf(1.4) * dur;
        knock(
            &mut out,
            samples(t),
            rng.range(180.0, 420.0),
            0.5 * (1.0 - t / dur),
            rng,
        );
    }
    edges(&mut out, 0.02);
    out
}

/// One bubble-like tone whose pitch rises as it closes.
fn plop(out: &mut [f32], start: usize, f0: f32, amp: f32, len: f32) {
    let mut phase = 0.0f32;
    for k in 0..samples(len) {
        let Some(o) = out.get_mut(start + k) else {
            break;
        };
        let t = k as f32 / RATE;
        phase += f0 * (1.0 + 1.8 * t / len) / RATE;
        *o += (phase * TAU).sin() * amp * (-t / (len * 0.3)).exp() * (t * 2000.0).min(1.0);
    }
}

fn drip(rng: &mut Rng) -> Vec<f32> {
    let mut out = vec![0.0; samples(0.5)];
    plop(&mut out, 0, rng.range(330.0, 520.0), 0.5, 0.08);
    // A softer echo of it off the cave walls.
    plop(&mut out, samples(0.11), rng.range(300.0, 450.0), 0.12, 0.08);
    let mut lp = LowPass::new(1000.0);
    out.iter_mut().for_each(|x| *x = lp.run(*x));
    out
}

fn wind(rng: &mut Rng, strength: f32, leaves: bool) -> Vec<f32> {
    let dur = rng.range(3.5, 5.0);
    let n = samples(dur);
    let mut band = Band::new(70.0, 400.0);
    let mut gust = LowPass::new(0.8);
    let mut leaf = Band::new(300.0, 1400.0);
    let mut leaf_amp = LowPass::new(8.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let k = t / dur;
            let g = (gust.run(rng.noise()) * 50.0).clamp(-1.0, 1.0) * 0.5 + 0.5;
            let lift = 0.3 + 0.7 * g;
            // Stronger gusts open the wind up a little, but it never whistles.
            band.set(70.0, 260.0 + 360.0 * strength * lift);
            let env = (k * PI).sin().powf(0.8);
            let mut y = band.run(rng.noise()) * (0.6 + 1.6 * strength) * lift;
            if leaves {
                let flutter = (leaf_amp.run(rng.noise()) * 12.0).abs().min(1.0);
                y = y * 0.4 + leaf.run(rng.noise()) * flutter * 1.3 * lift;
            }
            y * env * 1.5
        })
        .collect();
    edges(&mut out, 0.05);
    out
}

fn waves(rng: &mut Rng) -> Vec<f32> {
    let dur = rng.range(4.5, 6.0);
    let n = samples(dur);
    let crest = rng.range(0.35, 0.45);
    let mut lp = LowPass::new(300.0);
    let mut floor = LowPass::new(60.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let k = t / dur;
            // Builds as it rolls in, breaks, then washes out with a brighter fizz.
            let env = if k < crest {
                (k / crest).powf(2.0)
            } else {
                (1.0 - (k - crest) / (1.0 - crest)).powf(1.2)
            };
            let cut = if k < crest {
                220.0 + 300.0 * k / crest
            } else {
                520.0 + 380.0 * (1.0 - (k - crest) / (1.0 - crest)).powf(3.0)
            };
            lp.set(cut);
            let x = lp.run(rng.noise());
            (x - floor.run(x) * 0.6) * env * 2.2
        })
        .collect();
    edges(&mut out, 0.05);
    out
}

fn lapping(rng: &mut Rng) -> Vec<f32> {
    let dur = 1.6;
    let mut out = vec![0.0; samples(dur)];
    for _ in 0..rng.range(3.0, 5.0) as usize {
        let at = samples(rng.range(0.0, dur - 0.35));
        let mut lp = LowPass::new(rng.range(450.0, 700.0));
        let len = rng.range(0.15, 0.3);
        for k in 0..samples(len) {
            let t = k as f32 / RATE;
            let e = (t / 0.02).min(1.0) * (1.0 - t / len).powf(2.0);
            out[at + k] += lp.run(rng.noise()) * e * 1.5;
        }
        plop(
            &mut out,
            at + samples(0.03),
            rng.range(220.0, 340.0),
            0.08,
            0.07,
        );
    }
    edges(&mut out, 0.02);
    out
}

fn lava_pop(rng: &mut Rng) -> Vec<f32> {
    let mut out = vec![0.0; samples(0.6)];
    // A thick bubble bursting: a low plop and a short sputter.
    plop(&mut out, 0, rng.range(90.0, 140.0), 0.8, 0.14);
    let mut lp = LowPass::new(900.0);
    let at = samples(0.05);
    for k in 0..samples(0.3) {
        let t = k as f32 / RATE;
        let x = if rng.next() < 0.04 {
            rng.noise() * 3.0
        } else {
            0.0
        };
        out[at + k] += lp.run(x) * (-t / 0.1).exp() * 0.8;
    }
    edges(&mut out, 0.004);
    out
}

fn crackle(rng: &mut Rng) -> Vec<f32> {
    let dur = 1.6;
    let n = samples(dur);
    let mut roar = Band::new(60.0, 300.0);
    let mut out: Vec<f32> = (0..n).map(|_| roar.run(rng.noise()) * 0.5).collect();
    for _ in 0..30 {
        let at = samples(rng.range(0.0, dur - 0.05));
        knock(
            &mut out,
            at,
            rng.range(500.0, 1100.0),
            rng.range(0.15, 0.45),
            rng,
        );
    }
    edges(&mut out, 0.05);
    out
}

fn nether_drone(rng: &mut Rng) -> Vec<f32> {
    let dur = 7.0;
    let n = samples(dur);
    let f = rng.range(34.0, 46.0);
    let mut moan = Resonator::new(rng.range(260.0, 360.0), 60.0);
    let mut breath = LowPass::new(500.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let k = t / dur;
            let env = (k * PI).sin().powf(2.0);
            let bend = 1.0 + 0.05 * (t * 0.35 * TAU).sin();
            let tone = (t * f * bend * TAU).sin() * 0.6 + (t * f * 2.01 * TAU).sin() * 0.2;
            let m = moan.run(breath.run(rng.noise())) * 9.0;
            (tone + m) * env * 0.7
        })
        .collect();
    edges(&mut out, 0.05);
    out
}

fn soul_wind(rng: &mut Rng) -> Vec<f32> {
    let dur = 5.0;
    let n = samples(dur);
    let mut r1 = Resonator::new(rng.range(180.0, 230.0), 25.0);
    let mut r2 = Resonator::new(rng.range(280.0, 330.0), 30.0);
    let mut lp = LowPass::new(700.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let k = t / dur;
            let env = (k * PI).sin().powf(1.5) * (0.7 + 0.3 * (t * 0.7 * TAU).sin());
            let x = lp.run(rng.noise());
            (r1.run(x) * 14.0 + r2.run(x) * 9.0) * env
        })
        .collect();
    edges(&mut out, 0.05);
    out
}

fn end_hum(rng: &mut Rng) -> Vec<f32> {
    let dur = 6.0;
    let n = samples(dur);
    let f = rng.range(55.0, 70.0);
    let mut air = Band::new(80.0, 240.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let k = t / dur;
            let env = (k * PI).sin().powf(1.8);
            // Two tones a hair apart beat slowly against each other.
            let tone = (t * f * TAU).sin() * 0.4
                + (t * (f + 0.7) * TAU).sin() * 0.4
                + (t * f * 1.5 * TAU).sin() * 0.12;
            (tone + air.run(rng.noise()) * 0.9) * env * 0.7
        })
        .collect();
    edges(&mut out, 0.05);
    out
}

/// A soft hooted note: a sine with a breathy start.
fn hoot(out: &mut [f32], at: usize, f: f32, len: f32, amp: f32, rng: &mut Rng) {
    let mut lp = LowPass::new(900.0);
    for k in 0..samples(len) {
        let Some(o) = out.get_mut(at + k) else { break };
        let t = k as f32 / RATE;
        let e = (t / 0.04).min(1.0) * (1.0 - t / len).powf(1.4);
        let bend = 1.0 + 0.04 * (-t / 0.05).exp() - 0.03 * t / len;
        *o += lp.run((t * f * bend * TAU).sin() + rng.noise() * 0.15 * (-t / 0.05).exp()) * e * amp;
    }
}

fn owl(rng: &mut Rng) -> Vec<f32> {
    let f = rng.range(330.0, 390.0);
    let mut out = vec![0.0; samples(1.6)];
    // "Hoo... hoo-hoo".
    hoot(&mut out, 0, f, 0.45, 0.7, rng);
    hoot(&mut out, samples(0.7), f * 0.9, 0.22, 0.55, rng);
    hoot(&mut out, samples(0.98), f * 0.9, 0.5, 0.65, rng);
    edges(&mut out, 0.01);
    out
}

fn dove(rng: &mut Rng) -> Vec<f32> {
    let f = rng.range(380.0, 440.0);
    let mut out = vec![0.0; samples(1.5)];
    // "coo-COO-coo".
    hoot(&mut out, 0, f, 0.25, 0.45, rng);
    hoot(&mut out, samples(0.32), f * 1.12, 0.42, 0.65, rng);
    hoot(&mut out, samples(0.85), f * 0.95, 0.35, 0.45, rng);
    edges(&mut out, 0.01);
    out
}

fn frog(rng: &mut Rng) -> Vec<f32> {
    let croaks = if rng.next() < 0.5 { 1 } else { 2 };
    let mut out = vec![0.0; samples(croaks as f32 * 0.55 + 0.1)];
    for c in 0..croaks {
        let at = samples(c as f32 * 0.55);
        let len = rng.range(0.3, 0.42);
        let mut res = Resonator::new(rng.range(260.0, 340.0), 90.0);
        let mut body = Resonator::new(110.0, 60.0);
        // A croak is a fast train of pulses, around 25 a second.
        let rate = rng.range(22.0, 30.0);
        for k in 0..samples(len) {
            let t = k as f32 / RATE;
            let p = (t * rate).fract();
            let x = if p < 0.12 { (p / 0.12 * PI).sin() } else { 0.0 } * (t * 120.0 * TAU).sin();
            let e = (t / 0.02).min(1.0) * (1.0 - t / len).powf(0.7);
            out[at + k] += (res.run(x) * 10.0 + body.run(x) * 8.0) * e;
        }
    }
    edges(&mut out, 0.005);
    out
}

fn heartbeat() -> Vec<f32> {
    let mut out = vec![0.0; samples(0.6)];
    for (start, amp, f) in [(0.0, 1.0, 55.0), (0.24, 0.7, 50.0)] {
        let at = samples(start);
        for k in 0..samples(0.18) {
            let t = k as f32 / RATE;
            out[at + k] +=
                (t * f * (1.0 - t) * TAU).sin() * amp * (t / 0.005).min(1.0) * (-t / 0.05).exp();
        }
    }
    edges(&mut out, 0.004);
    out
}

fn gasp(rng: &mut Rng) -> Vec<f32> {
    let dur = 0.75;
    let n = samples(dur);
    let mut band = Band::new(300.0, 1200.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let k = t / dur;
            // A sharp pull of air, then a slower breath out.
            let env = if k < 0.35 {
                (k / 0.35).powf(0.6)
            } else {
                (1.0 - (k - 0.35) / 0.65).powf(1.5) * 0.7
            };
            band.set(300.0, if k < 0.35 { 1200.0 } else { 800.0 });
            band.run(rng.noise()) * env * 1.8
        })
        .collect();
    edges(&mut out, 0.01);
    out
}

fn land(rng: &mut Rng, strength: f32) -> Vec<f32> {
    let dur = 0.25 + strength * 0.2;
    let n = samples(dur);
    let mut lp = LowPass::new(260.0 + strength * 200.0);
    let f = rng.range(48.0, 60.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let thump = (t * f * TAU).sin() * (-t / (0.05 + strength * 0.05)).exp();
            let dirt = lp.run(rng.noise()) * (-t / 0.04).exp() * 2.0;
            (thump + dirt) * (0.5 + strength * 0.5)
        })
        .collect();
    edges(&mut out, 0.004);
    out
}

/// Air moving: a band of noise sweeping up and back down.
fn whoosh(rng: &mut Rng, dur: f32, lo: f32, hi: f32, gain: f32) -> Vec<f32> {
    let n = samples(dur);
    let mut band = Band::new(lo * 0.5, lo);
    let peak = rng.range(0.4, 0.55);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let k = i as f32 / n as f32;
            let s = if k < peak {
                k / peak
            } else {
                1.0 - (k - peak) / (1.0 - peak)
            };
            band.set(lo * 0.4, lo + (hi - lo) * s);
            band.run(rng.noise()) * s.powf(1.5) * gain * 3.0
        })
        .collect();
    edges(&mut out, 0.004);
    out
}

fn brush(rng: &mut Rng) -> Vec<f32> {
    let dur = rng.range(0.28, 0.38);
    let n = samples(dur);
    let mut band = Band::new(220.0, 950.0);
    let mut flutter = LowPass::new(25.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let k = i as f32 / n as f32;
            let f = (flutter.run(rng.noise()) * 8.0).abs().min(1.0);
            band.run(rng.noise()) * (k * PI).sin() * (0.4 + f) * 1.4
        })
        .collect();
    edges(&mut out, 0.005);
    out
}

fn flap(rng: &mut Rng, open: bool) -> Vec<f32> {
    let dur = 0.16;
    let n = samples(dur);
    let mut lp = LowPass::new(if open { 750.0 } else { 600.0 });
    let f = if open { 140.0 } else { 115.0 };
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let e = (t / 0.004).min(1.0) * (-t / 0.03).exp();
            (lp.run(rng.noise()) * 1.4 + (t * f * TAU).sin() * 0.5) * e * 0.7
        })
        .collect();
    edges(&mut out, 0.003);
    out
}

fn armor(rng: &mut Rng, metal: bool) -> Vec<f32> {
    let dur = 0.22;
    let mut out = vec![0.0; samples(dur)];
    if metal {
        // A few soft, low clinks of plates touching.
        for _ in 0..3 {
            let at = samples(rng.range(0.0, 0.08));
            let f = rng.range(320.0, 520.0);
            for k in 0..samples(0.12) {
                let Some(o) = out.get_mut(at + k) else { break };
                let t = k as f32 / RATE;
                *o += ((t * f * TAU).sin() * 0.25 + (t * f * 1.41 * TAU).sin() * 0.1)
                    * (-t / 0.03).exp();
            }
        }
    } else {
        let mut lp = LowPass::new(650.0);
        for (i, o) in out.iter_mut().enumerate() {
            let t = i as f32 / RATE;
            *o = lp.run(rng.noise()) * (t / dur * PI).sin() * 0.8;
        }
    }
    let mut lp = LowPass::new(1100.0);
    out.iter_mut().for_each(|x| *x = lp.run(*x));
    edges(&mut out, 0.004);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brightness(v: &[f32]) -> f32 {
        let zc = v
            .windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();
        zc as f32 / (v.len() as f32 / RATE) / 2.0
    }

    pub fn every_fx() -> Vec<Fx> {
        use Voice::*;
        let mut all = vec![
            Fx::CaveDrone,
            Fx::CaveRumble,
            Fx::Drip,
            Fx::Wind(0.1),
            Fx::Wind(1.0),
            Fx::Canopy,
            Fx::Waves,
            Fx::Lapping,
            Fx::LavaPop,
            Fx::FireCrackle,
            Fx::NetherDrone,
            Fx::SoulWind,
            Fx::EndHum,
            Fx::Owl,
            Fx::Dove,
            Fx::Frog,
            Fx::Heartbeat,
            Fx::Gasp,
            Fx::Land(0.0),
            Fx::Land(1.0),
            Fx::Jump,
            Fx::Swing,
            Fx::Brush,
            Fx::InvOpen,
            Fx::InvClose,
            Fx::Armor(true),
            Fx::Armor(false),
        ];
        for v in [
            Groan, Rattle, Hiss, Moo, Grunt, Bleat, Cluck, Quack, Coo, Snort, Growl, Wail, Cackle,
            Murmur, Rumble,
        ] {
            for c in [Call::Idle, Call::Hurt, Call::Death] {
                for d in [0.6, 1.0, 1.5] {
                    all.push(Fx::Voice(v, c, d));
                }
            }
        }
        all
    }

    #[test]
    fn extra_sounds_are_low_and_clean() {
        for fx in every_fx() {
            for seed in [1, 7, 99] {
                let v = synth(fx, seed);
                assert!(!v.is_empty() && v.iter().all(|x| x.is_finite()), "{fx:?}");
                let peak = v.iter().fold(0.0f32, |a, x| a.max(x.abs()));
                assert!(peak > 0.02, "{fx:?} is silent ({peak})");
                let b = brightness(&v);
                assert!(b < 1300.0, "{fx:?} too bright: {b} Hz");
                // Nothing should end on a click.
                assert!(v.last().unwrap().abs() < 0.05, "{fx:?} ends abruptly");
            }
        }
    }

    /// Writes every sound to WAV files for listening: MC_SOUNDS_OUT=dir cargo test -- --ignored
    #[test]
    #[ignore]
    fn export_sounds() {
        let Ok(dir) = std::env::var("MC_SOUNDS_OUT") else {
            return;
        };
        std::fs::create_dir_all(&dir).unwrap();
        for fx in every_fx() {
            let name = format!("{fx:?}")
                .replace(['(', ')', ',', ' ', '.'], "_")
                .replace("__", "_");
            let v = synth(fx, 7);
            let mut b = Vec::new();
            let len = (v.len() * 2) as u32;
            b.extend_from_slice(b"RIFF");
            b.extend_from_slice(&(36 + len).to_le_bytes());
            b.extend_from_slice(b"WAVEfmt ");
            b.extend_from_slice(&16u32.to_le_bytes());
            b.extend_from_slice(&1u16.to_le_bytes());
            b.extend_from_slice(&1u16.to_le_bytes());
            b.extend_from_slice(&44100u32.to_le_bytes());
            b.extend_from_slice(&88200u32.to_le_bytes());
            b.extend_from_slice(&2u16.to_le_bytes());
            b.extend_from_slice(&16u16.to_le_bytes());
            b.extend_from_slice(b"data");
            b.extend_from_slice(&len.to_le_bytes());
            for x in v {
                b.extend_from_slice(&((x * 32000.0) as i16).to_le_bytes());
            }
            std::fs::write(format!("{dir}/{}.wav", name.trim_end_matches('_')), b).unwrap();
        }
    }

    #[test]
    fn every_mob_name_gets_a_sensible_voice() {
        assert_eq!(voice_of("zombie", true), Some(Voice::Groan));
        assert_eq!(voice_of("ash_skeleton", true), Some(Voice::Rattle));
        assert_eq!(voice_of("void_walker_soul", true), Some(Voice::Wail));
        assert_eq!(voice_of("npc_guide", false), Some(Voice::Murmur));
        assert_eq!(voice_of("minecart", false), None);
    }
}
