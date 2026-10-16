//! Procedural sound effects. No sound files: every sound is built from filtered noise and a few
//! decaying tones. They are kept low and soft on purpose (thumps, crunches, knocks) like the
//! classic game, because bright, high-pitched effects get tiring to hear thousands of times.

use crate::audio::{Material, Sfx};

const RATE: f32 = 44100.0;
const TAU: f32 = std::f32::consts::TAU;

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

/// Two one-pole low-pass filters in a row: a gentle 12 dB/octave roll-off.
struct LowPass {
    a: f32,
    y1: f32,
    y2: f32,
}

impl LowPass {
    fn new(cutoff: f32) -> LowPass {
        LowPass { a: coef(cutoff), y1: 0.0, y2: 0.0 }
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
    1.0 - (-TAU * cutoff / RATE).exp()
}

/// How a material sounds: noise colour, body resonances, and how gritty it is.
struct Voice {
    cutoff: f32,
    /// Resonant body modes (Hz) and their loudness.
    body: &'static [(f32, f32)],
    body_decay: f32,
    /// Grains per second in the crunchy tail.
    grains: f32,
    grain_len: f32,
    noise_gain: f32,
}

fn voice(m: Material) -> Voice {
    match m {
        // A dull clack with some grit.
        Material::Stone => Voice { cutoff: 1300.0, body: &[(118.0, 0.5), (182.0, 0.35), (265.0, 0.2)], body_decay: 0.045, grains: 70.0, grain_len: 0.006, noise_gain: 0.9 },
        // A hollow knock.
        Material::Wood => Voice { cutoff: 750.0, body: &[(92.0, 0.7), (184.0, 0.45), (276.0, 0.2)], body_decay: 0.08, grains: 25.0, grain_len: 0.01, noise_gain: 0.6 },
        // Soft crunch of dirt and roots.
        Material::Grass => Voice { cutoff: 950.0, body: &[(70.0, 0.35)], body_decay: 0.05, grains: 120.0, grain_len: 0.008, noise_gain: 1.0 },
        // Fine, dense crunch.
        Material::Sand => Voice { cutoff: 1500.0, body: &[(60.0, 0.25)], body_decay: 0.04, grains: 260.0, grain_len: 0.004, noise_gain: 0.85 },
        // Glass keeps a little shine, but low and short.
        Material::Glass => Voice { cutoff: 2100.0, body: &[(105.0, 0.4), (940.0, 0.12), (1230.0, 0.08)], body_decay: 0.07, grains: 60.0, grain_len: 0.005, noise_gain: 0.6 },
        // Muffled.
        Material::Cloth => Voice { cutoff: 380.0, body: &[(65.0, 0.2)], body_decay: 0.05, grains: 40.0, grain_len: 0.02, noise_gain: 1.0 },
    }
}

pub fn synth(sfx: Sfx, seed: u32) -> Vec<f32> {
    let mut rng = Rng(seed.wrapping_mul(747796405).wrapping_add(2891336453) | 1);
    let out = match sfx {
        Sfx::Break(m) => block(m, &mut rng, 0.38, 1.0, 0.22, 0.95),
        Sfx::Place(m) => block(m, &mut rng, 0.16, 1.0, 0.03, 0.8),
        Sfx::Hit(m) => block(m, &mut rng, 0.11, 0.6, 0.04, 0.45),
        Sfx::Step(m) => block(m, &mut rng, 0.13, 0.35, 0.08, 0.3),
        Sfx::Click => click(&mut rng),
        Sfx::Hurt => hurt(&mut rng),
        Sfx::Death => death(&mut rng),
        Sfx::Respawn => respawn(),
        Sfx::MobHit => mob_hit(&mut rng),
        Sfx::Explode => explode(&mut rng),
        Sfx::Splash(s) => splash(&mut rng, s.clamp(0.2, 1.0)),
        Sfx::Swim => swim(&mut rng),
        Sfx::WaterExit => drips(&mut rng, 0.7, 5),
        Sfx::Bubbles => drips(&mut rng, 0.6, 4),
        Sfx::Pickup => pickup(&mut rng),
        Sfx::Xp => xp(&mut rng),
        Sfx::Eat => eat(&mut rng),
    };
    out.into_iter().map(|s| s.clamp(-1.0, 1.0)).collect()
}

/// A block sound: an impact (body modes + noise burst) followed by a crunchy tail of grains.
fn block(m: Material, rng: &mut Rng, dur: f32, impact: f32, tail: f32, gain: f32) -> Vec<f32> {
    let v = voice(m);
    // Every sound is a little different, as if hit in a different spot.
    let pitch = rng.range(0.86, 1.1);
    let n = (dur * RATE) as usize;
    // Grain start times and strengths.
    let count = (v.grains * tail).round() as usize + 1;
    let mut grains: Vec<(usize, f32, usize)> = (0..count)
        .map(|_| {
            let t = rng.next().powf(1.6) * tail;
            let len = (v.grain_len * rng.range(0.6, 1.5) * RATE) as usize + 8;
            ((t * RATE) as usize, rng.range(0.3, 1.0) * (1.0 - t / (tail + 0.05)), len)
        })
        .collect();
    grains.sort_by_key(|g| g.0);
    let mut lp = LowPass::new(v.cutoff * pitch);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / RATE;
        // Impact: a 25 ms burst.
        let mut e = impact * (-t / 0.012).exp();
        for &(start, amp, len) in &grains {
            if start > i {
                break;
            }
            let k = i - start;
            if k < len {
                let x = k as f32 / len as f32;
                e += amp * (x * std::f32::consts::PI).sin() * 0.6;
            }
        }
        let noise = lp.run(rng.noise() * e) * v.noise_gain * 2.2;
        let mut body = 0.0;
        for &(f, a) in v.body {
            body += (t * f * pitch * TAU).sin() * a * (-t / v.body_decay).exp() * impact;
        }
        // Fade the very end so nothing clicks.
        let fade = ((dur - t) / 0.01).clamp(0.0, 1.0);
        out.push((noise + body) * gain * fade);
    }
    out
}

/// Menu click: a soft, low wooden tick.
fn click(rng: &mut Rng) -> Vec<f32> {
    let n = (0.05 * RATE) as usize;
    let mut lp = LowPass::new(900.0);
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let e = (-t / 0.008).exp();
            ((t * 430.0 * TAU).sin() * 0.5 + lp.run(rng.noise()) * 0.8) * e * 0.45
        })
        .collect()
}

/// A short falling "oof".
fn hurt(rng: &mut Rng) -> Vec<f32> {
    let dur = 0.26;
    let n = (dur * RATE) as usize;
    let base = rng.range(150.0, 175.0);
    let mut phase = 0.0f32;
    let mut lp = LowPass::new(900.0);
    (0..n)
        .map(|i| {
            let k = i as f32 / n as f32;
            phase += base * (1.0 - 0.4 * k) / RATE;
            let saw = (phase.fract() * 2.0 - 1.0) * 0.6 + (phase * TAU).sin() * 0.5;
            lp.run(saw) * (1.0 - k).powf(1.5) * (k * 40.0).min(1.0) * 1.3
        })
        .collect()
}

/// Dying: a long falling groan, then the body hitting the ground.
fn death(rng: &mut Rng) -> Vec<f32> {
    let dur = 1.5;
    let n = (dur * RATE) as usize;
    let mut phase = 0.0f32;
    let mut voice = LowPass::new(650.0);
    let mut thud = LowPass::new(160.0);
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            // The groan: 140 Hz sliding down to 55 Hz over the first second.
            let k = (t / 1.0).min(1.0);
            phase += (140.0 - 85.0 * k.powf(0.7)) / RATE;
            let saw = (phase.fract() * 2.0 - 1.0) * 0.5 + (phase * TAU).sin() * 0.6;
            let groan = voice.run(saw) * (t * 20.0).min(1.0) * (1.0 - k).powf(1.2) * 1.2;
            // The fall at 0.75 s: a deep thump with a little dirt noise.
            let dt = t - 0.75;
            let body = if dt >= 0.0 { thud.run(rng.noise()) * 5.0 * (-dt / 0.12).exp() + (dt * 55.0 * TAU).sin() * 0.7 * (-dt / 0.18).exp() } else { thud.run(0.0) };
            groan + body
        })
        .collect()
}

/// Respawning: a soft, warm chord that swells in and fades.
fn respawn() -> Vec<f32> {
    let dur = 1.8;
    let n = (dur * RATE) as usize;
    let notes = [(98.0, 0.5), (147.0, 0.4), (196.0, 0.3), (247.0, 0.15)];
    let mut lp = LowPass::new(800.0);
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let vib = 1.0 + (t * 5.0 * TAU).sin() * 0.003;
            let mut x = 0.0;
            for (j, (f, a)) in notes.iter().enumerate() {
                // The notes come in one after another, from the bottom.
                let start = j as f32 * 0.12;
                if t > start {
                    let e = ((t - start) / 0.35).min(1.0) * (1.0 - (t - start) / (dur - start)).max(0.0).powf(1.4);
                    x += (f * vib * t * TAU).sin() * a * e;
                }
            }
            lp.run(x) * 0.9
        })
        .collect()
}

/// A soft, low pop for picking something up.
fn pickup(rng: &mut Rng) -> Vec<f32> {
    let dur = 0.12;
    let n = (dur * RATE) as usize;
    let f0 = rng.range(150.0, 190.0);
    let mut phase = 0.0f32;
    let mut lp = LowPass::new(600.0);
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            // A quick rise in pitch, like a cork.
            phase += (f0 * (1.0 + t * 6.0)) / RATE;
            let e = (t / 0.006).min(1.0) * (-t / 0.035).exp();
            lp.run((phase * TAU).sin() * e) * 0.8
        })
        .collect()
}

/// Experience: a short, warm two-note chime, kept low.
fn xp(rng: &mut Rng) -> Vec<f32> {
    let dur = 0.45;
    let n = (dur * RATE) as usize;
    let k = rng.range(0.95, 1.08);
    let notes = [(220.0 * k, 0.45, 0.0), (330.0 * k, 0.3, 0.06)];
    let mut lp = LowPass::new(700.0);
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let mut x = 0.0;
            for (f, a, start) in notes {
                if t > start {
                    let u = t - start;
                    x += (f * u * TAU).sin() * a * (u / 0.01).min(1.0) * (-u / 0.12).exp();
                }
            }
            lp.run(x) * 0.8
        })
        .collect()
}

/// A bite: a short, dull crunch.
fn eat(rng: &mut Rng) -> Vec<f32> {
    let dur = 0.16;
    let n = (dur * RATE) as usize;
    let mut lp = LowPass::new(rng.range(650.0, 850.0));
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            // Two or three crunches in quick succession.
            let e = (-t / 0.03).exp() + 0.6 * (-((t - 0.06).max(0.0)) / 0.025).exp() * if t > 0.06 { 1.0 } else { 0.0 };
            lp.run(rng.noise() * e) * 1.4
        })
        .collect()
}

/// A punch landing on a mob.
fn mob_hit(rng: &mut Rng) -> Vec<f32> {
    let dur = 0.16;
    let n = (dur * RATE) as usize;
    let mut lp = LowPass::new(700.0);
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let k = i as f32 / n as f32;
            phase += (110.0 - 50.0 * k) / RATE;
            (lp.run(rng.noise()) * 1.4 + (phase * TAU).sin() * 0.6) * (1.0 - k).powi(3) * 0.8
        })
        .collect()
}

/// Deep rumbling noise with a crack at the start.
fn explode(rng: &mut Rng) -> Vec<f32> {
    let dur = 1.5;
    let n = (dur * RATE) as usize;
    let mut low = LowPass::new(140.0);
    let mut mid = LowPass::new(1200.0);
    (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let k = t / dur;
            let x = rng.noise();
            let crack = if t < 0.06 { mid.run(x) * (1.0 - t / 0.06) * 1.2 } else { 0.0 };
            (low.run(x) * 7.0 + crack) * (1.0 - k).powf(2.2)
        })
        .collect()
}

/// One bubble: a damped tone whose pitch rises as it closes.
fn bubble(out: &mut [f32], start: usize, f0: f32, amp: f32, len: f32) {
    let n = (len * RATE) as usize;
    let mut phase = 0.0f32;
    for k in 0..n {
        let i = start + k;
        if i >= out.len() {
            break;
        }
        let t = k as f32 / RATE;
        phase += f0 * (1.0 + 2.5 * t / len) / RATE;
        out[i] += (phase * TAU).sin() * amp * (-t / (len * 0.35)).exp() * (t * 2000.0).min(1.0);
    }
}

/// Falling into water: a body of low noise, then bubbles.
fn splash(rng: &mut Rng, strength: f32) -> Vec<f32> {
    let dur = 0.5 + strength * 0.5;
    let n = (dur * RATE) as usize;
    let mut lp = LowPass::new(1400.0);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / RATE;
        // The splash darkens as the water closes over.
        lp.set(1100.0 - 650.0 * (t / dur));
        let e = (t * 300.0).min(1.0) * (-t / (0.08 + 0.12 * strength)).exp() + 0.15 * (-t / 0.35).exp();
        out.push(lp.run(rng.noise()) * e * 2.6 * strength.sqrt());
    }
    let count = (6.0 + strength * 10.0) as usize;
    for _ in 0..count {
        let start = (rng.range(0.03, dur * 0.8) * RATE) as usize;
        bubble(&mut out, start, rng.range(260.0, 620.0), rng.range(0.05, 0.14), rng.range(0.03, 0.07));
    }
    out
}

/// A swimming stroke: a soft slosh with a couple of bubbles.
fn swim(rng: &mut Rng) -> Vec<f32> {
    let dur = 0.4;
    let n = (dur * RATE) as usize;
    let mut lp = LowPass::new(800.0);
    let mut out: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let e = (t / dur * std::f32::consts::PI).sin().powi(2);
            lp.run(rng.noise()) * e * 0.9
        })
        .collect();
    for _ in 0..3 {
        let start = (rng.range(0.05, 0.3) * RATE) as usize;
        bubble(&mut out, start, rng.range(300.0, 520.0), 0.06, 0.05);
    }
    out
}

/// Drips or rising bubbles.
fn drips(rng: &mut Rng, dur: f32, count: usize) -> Vec<f32> {
    let n = (dur * RATE) as usize;
    let mut out = vec![0.0; n];
    let mut lp = LowPass::new(500.0);
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / RATE;
        *s = lp.run(rng.noise()) * 0.25 * (-t / 0.2).exp();
    }
    for _ in 0..count {
        let start = (rng.range(0.0, dur * 0.75) * RATE) as usize;
        bubble(&mut out, start, rng.range(280.0, 560.0), rng.range(0.08, 0.15), rng.range(0.035, 0.06));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Zero crossings per second / 2 is a rough pitch estimate: block and water sounds must stay low.
    fn brightness(v: &[f32]) -> f32 {
        let zc = v.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count();
        zc as f32 / (v.len() as f32 / RATE) / 2.0
    }

    #[test]
    fn effects_are_low_and_clean() {
        let mats = [Material::Stone, Material::Wood, Material::Grass, Material::Sand, Material::Glass, Material::Cloth];
        let mut all = Vec::new();
        for m in mats {
            all.extend([Sfx::Break(m), Sfx::Place(m), Sfx::Hit(m), Sfx::Step(m)]);
        }
        all.extend([Sfx::Click, Sfx::Hurt, Sfx::Death, Sfx::Respawn, Sfx::MobHit, Sfx::Explode, Sfx::Splash(1.0), Sfx::Swim, Sfx::WaterExit, Sfx::Bubbles, Sfx::Pickup, Sfx::Xp, Sfx::Eat]);
        for sfx in all {
            let v = synth(sfx, 7);
            assert!(!v.is_empty() && v.iter().all(|x| x.is_finite()), "{sfx:?}");
            let peak = v.iter().fold(0.0f32, |a, x| a.max(x.abs()));
            assert!(peak > 0.02, "{sfx:?} is silent");
            let b = brightness(&v);
            assert!(b < 1300.0, "{sfx:?} too bright: {b} Hz");
            println!("{sfx:?}: peak {peak:.2}, ~{b:.0} Hz");
        }
    }
}
