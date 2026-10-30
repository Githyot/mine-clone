//! Particles: bits of broken blocks, water drops from splashes and bubbles, and the small life
//! in the air (leaves, insects, birds, fish, ripples; ambient_life.rs spawns those). Simulated
//! on the CPU (there are only ever a few hundred) and drawn as tiny boxes: cubes cut from block
//! textures, or flat plates that tumble, flap or spread out in a ring.

use std::f32::consts::{PI, TAU};

use glam::{IVec3, Quat, Vec3};
use voxel::physics::Terrain;
use wgpu::util::DeviceExt;

use crate::renderer::Renderer;
use crate::world::World;

const MAX: usize = 1500;
/// Marks a flame as soul fire (its colours are worked out as it burns).
const SOUL: [f32; 3] = [0.0, 0.0, 1.0];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GpuParticle {
    pos: [f32; 4],
    uv: [f32; 4],
    tint: [f32; 4],
    /// Rotation of the box (quaternion xyzw).
    rot: [f32; 4],
    /// xyz: size of the box along each of its axes (times pos.w); w: how it is drawn (0 a piece
    /// of texture, 1 plain colour, 2 a thin ring).
    shape: [f32; 4],
    /// x: opacity 0..1 (dithered); y: size of the texture piece (0.25 = 4×4 pixels).
    extra: [f32; 4],
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum Kind {
    /// Falls and bounces to a stop.
    #[default]
    Debris,
    /// Falls, then vanishes when it lands or touches water.
    Drop,
    /// Rises under water, pops at the surface.
    Bubble,
    /// Magic: drifts without falling and shines at full brightness.
    Glow,
    /// Ambient bits (Subtle Effects idea): ash or dust drifting slowly down, swaying.
    Mote,
    /// A lick of flame: rises faster and faster, white-yellow turning orange then red as it
    /// burns out, at full brightness.
    Flame,
    /// An ember thrown up off a fire: rises, slows, wanders and fades.
    Spark,
    /// A flat leaf (or petal) seesawing down, carried by the wind; it lies on the ground (or
    /// floats on water) for a while before it fades.
    Leaf,
    /// A firefly: wanders slowly, its light swelling and dying away.
    Firefly,
    /// A butterfly flapping about its flower.
    Butterfly,
    /// A bee darting from spot to spot around its flower.
    Bee,
    /// One of a cloud of gnats circling over water.
    Gnat,
    /// A speck of dust or pollen floating in the light.
    Speck,
    /// A bird of a flock passing high over.
    Bird,
    /// A fish leaping out of the water and back in.
    Fish,
    /// A ring spreading on a water surface.
    Ripple,
}

#[derive(Default)]
struct Particle {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    size: f32,
    layer: u32,
    uv: [f32; 2],
    tint: [f32; 3],
    kind: Kind,
    /// Where it hovers around (insects) or its spin angle's axis seed.
    home: Vec3,
    /// Seconds alive.
    age: f32,
    /// Random 0..1, fixed per particle (its own rhythm).
    seed: f32,
    /// Leaves: tumble angle. Butterflies, bees, birds: heading.
    spin: f32,
    /// Leaves: 1 lying on the ground, 2 floating on water. Fish: it has left the water.
    rest: u8,
}

pub struct Particles {
    list: Vec<Particle>,
    seed: u32,
    /// The wind where the player is (wind.rs), m/s. Leaves, specks and gnats drift with it.
    pub wind: Vec3,
    /// Where fish fell back into the water this frame (ambient_life.rs splashes and sounds).
    pub fish_landed: Vec<Vec3>,
    /// The camera, so dust right in front of the eye can be left out.
    pub eye: Vec3,
}

impl Particles {
    pub fn new() -> Particles {
        Particles { list: Vec::new(), seed: 0x9e3779b9, wind: Vec3::ZERO, fish_landed: Vec::new(), eye: Vec3::ZERO }
    }

    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32
    }

    fn push(&mut self, p: Particle) {
        if self.list.len() >= MAX {
            // Make room by dropping the one closest to its end, not whichever sits first.
            let mut worst = 0;
            for (i, q) in self.list.iter().enumerate() {
                if q.life < self.list[worst].life {
                    worst = i;
                }
            }
            self.list.swap_remove(worst);
        }
        self.list.push(p);
    }

    /// How many of this kind are alive.
    pub fn count(&self, kind: Kind) -> usize {
        self.list.iter().filter(|p| p.kind == kind).count()
    }

    /// How many of each living kind there are (test pictures' log).
    pub fn census(&self) -> String {
        [Kind::Leaf, Kind::Firefly, Kind::Butterfly, Kind::Bee, Kind::Gnat, Kind::Speck, Kind::Bird, Kind::Fish, Kind::Ripple]
            .iter()
            .map(|k| format!("{k:?} {}", self.count(*k)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Bits flying out of a block. `count` around 16 for a break, 2–3 for a mining knock.
    pub fn block_bits(&mut self, block: IVec3, layer: u16, tint: [f32; 3], count: usize, burst: f32) {
        for _ in 0..count {
            let off = Vec3::new(self.rand(), self.rand(), self.rand());
            let pos = block.as_vec3() + Vec3::splat(0.15) + off * 0.7;
            let dir = (off - Vec3::splat(0.5)) * 2.0;
            let vel = Vec3::new(dir.x * burst, 1.5 + self.rand() * 2.5 * burst, dir.z * burst);
            let uv = [(self.rand() * 12.0).floor() / 16.0, (self.rand() * 12.0).floor() / 16.0];
            let p = Particle { pos, vel, life: 0.6 + self.rand() * 0.9, size: 0.05 + self.rand() * 0.05, layer: layer as u32, uv, tint, kind: Kind::Debris, ..Default::default() };
            self.push(p);
        }
    }

    /// Drops thrown up around a splash.
    pub fn splash(&mut self, at: Vec3, layer: u16, strength: f32) {
        let n = (10.0 + strength * 30.0) as usize;
        for _ in 0..n {
            let a = self.rand() * TAU;
            let r = 0.3 + self.rand() * 0.9;
            let up = 3.0 + self.rand() * 5.0 * strength;
            let p = Particle {
                pos: at + Vec3::new(a.cos() * r * 0.5, 0.05, a.sin() * r * 0.5),
                vel: Vec3::new(a.cos() * r * 2.0, up, a.sin() * r * 2.0),
                life: 1.5,
                size: 0.05 + self.rand() * 0.05,
                layer: layer as u32,
                uv: [0.4, 0.4],
                tint: [0.75, 0.85, 1.0],
                kind: Kind::Drop,
                ..Default::default()
            };
            self.push(p);
        }
    }

    /// A raindrop hitting the ground: a few tiny droplets bouncing up (Particle Rain idea).
    pub fn rain_drop(&mut self, at: Vec3, layer: u16) {
        for _ in 0..3 {
            let a = self.rand() * TAU;
            let r = 0.4 + self.rand() * 0.8;
            let p = Particle {
                pos: at + Vec3::new(0.0, 0.05, 0.0),
                vel: Vec3::new(a.cos() * r, 1.5 + self.rand() * 1.5, a.sin() * r),
                life: 0.35,
                size: 0.03 + self.rand() * 0.03,
                layer: layer as u32,
                uv: [0.4, 0.4],
                tint: [0.75, 0.85, 1.0],
                kind: Kind::Drop,
                ..Default::default()
            };
            self.push(p);
        }
    }

    /// A glowing mote (spell trails and bursts).
    pub fn glow(&mut self, at: Vec3, vel: Vec3, layer: u16, tint: [f32; 3], size: f32, life: f32) {
        let jitter = Vec3::new(self.rand() - 0.5, self.rand() - 0.5, self.rand() - 0.5);
        let p = Particle { pos: at + jitter * size, vel: vel + jitter * 0.6, life: life * (0.7 + self.rand() * 0.6), size, layer: layer as u32, uv: [0.4, 0.4], tint, kind: Kind::Glow, ..Default::default() };
        self.push(p);
    }

    /// A lick of flame off a torch or fire at `at`, moving along with `carry` (the holder's
    /// speed, so a carried torch's flame stays on it). `red`: redstone dust instead.
    pub fn flame(&mut self, at: Vec3, carry: Vec3, layer: u16, size: f32, life: f32, red: bool) {
        let j = Vec3::new(self.rand() - 0.5, self.rand() * 0.5, self.rand() - 0.5);
        let (kind, tint) = if red { (Kind::Spark, [1.3, 0.12, 0.08]) } else { (Kind::Flame, [1.0; 3]) };
        let vel = carry + Vec3::new(j.x * 0.25, 0.5 + j.y, j.z * 0.25);
        let p = Particle { pos: at + j * Vec3::new(0.1, 0.06, 0.1), vel, life: life * (0.7 + self.rand() * 0.6), size: size * (0.7 + self.rand() * 0.6), layer: layer as u32, uv: [0.4, 0.4], tint, kind, ..Default::default() };
        self.push(p);
    }

    /// A lick of pale blue soul fire (the flame colours run cyan to deep blue).
    pub fn soul_flame(&mut self, at: Vec3, layer: u16, size: f32, life: f32) {
        self.flame(at, Vec3::ZERO, layer, size, life, false);
        if let Some(p) = self.list.last_mut() {
            p.tint = SOUL;
        }
    }

    /// An ember flying up off a fire.
    pub fn spark(&mut self, at: Vec3, carry: Vec3, layer: u16) {
        let a = self.rand() * TAU;
        let vel = carry + Vec3::new(a.cos() * 0.5, 1.6 + self.rand() * 1.2, a.sin() * 0.5);
        let p = Particle { pos: at, vel, life: 0.8 + self.rand() * 0.7, size: 0.018, layer: layer as u32, uv: [0.4, 0.4], tint: [1.5, 0.75, 0.25], kind: Kind::Spark, ..Default::default() };
        self.push(p);
    }

    /// A slow drifting bit lit like the world around it: ash, dust, smoke.
    pub fn mote(&mut self, at: Vec3, layer: u16, tint: [f32; 3], size: f32, fall: f32, life: f32) {
        let a = self.rand() * TAU;
        let uv = [(self.rand() * 12.0).floor() / 16.0, (self.rand() * 12.0).floor() / 16.0];
        let p = Particle { pos: at, vel: Vec3::new(a.cos() * 0.2, -fall, a.sin() * 0.2), life: life * (0.7 + self.rand() * 0.6), size, layer: layer as u32, uv, tint, kind: Kind::Mote, ..Default::default() };
        self.push(p);
    }

    /// A mote blown along `vel` (sandstorm grit).
    pub fn dust(&mut self, at: Vec3, vel: Vec3, layer: u16, tint: [f32; 3], size: f32, life: f32) {
        let uv = [(self.rand() * 12.0).floor() / 16.0, (self.rand() * 12.0).floor() / 16.0];
        let p = Particle { pos: at, vel, life: life * (0.7 + self.rand() * 0.6), size, layer: layer as u32, uv, tint, kind: Kind::Mote, ..Default::default() };
        self.push(p);
    }

    /// A drop of water hanging off a ceiling, then falling.
    pub fn drip(&mut self, at: Vec3, layer: u16) {
        let p = Particle { pos: at, vel: Vec3::new(0.0, -0.4, 0.0), life: 2.5, size: 0.035, layer: layer as u32, uv: [0.4, 0.4], tint: [0.6, 0.75, 1.0], kind: Kind::Drop, ..Default::default() };
        self.push(p);
    }

    pub fn bubbles(&mut self, at: Vec3, layer: u16, count: usize) {
        for _ in 0..count {
            let off = Vec3::new(self.rand() - 0.5, self.rand() * 0.3, self.rand() - 0.5) * 0.6;
            let p = Particle {
                pos: at + off,
                vel: Vec3::new(0.0, 1.2 + self.rand(), 0.0),
                life: 2.5,
                size: 0.04 + self.rand() * 0.04,
                layer: layer as u32,
                uv: [0.3, 0.3],
                tint: [0.85, 0.95, 1.2],
                kind: Kind::Bubble,
                ..Default::default()
            };
            self.push(p);
        }
    }

    /// A leaf letting go of a tree: a flat piece of its texture, `size` across (a petal is
    /// smaller).
    pub fn leaf(&mut self, at: Vec3, layer: u16, tint: [f32; 3], size: f32) {
        let uv = [(self.rand() * 11.0).floor() / 16.0, (self.rand() * 11.0).floor() / 16.0];
        let seed = self.rand();
        let p = Particle { pos: at, vel: Vec3::new(0.0, -0.3, 0.0), life: 40.0, size, layer: layer as u32, uv, tint, kind: Kind::Leaf, seed, spin: seed * TAU, ..Default::default() };
        self.push(p);
    }

    /// A firefly hovering around `at`.
    pub fn firefly(&mut self, at: Vec3) {
        let seed = self.rand();
        let tint = if seed < 0.7 { [0.85, 1.0, 0.3] } else { [1.0, 0.85, 0.3] };
        let vel = Vec3::new(self.rand() - 0.5, 0.0, self.rand() - 0.5) * 0.3;
        let p = Particle { pos: at, home: at, vel, life: 12.0 + self.rand() * 10.0, size: 0.06, tint, kind: Kind::Firefly, seed, ..Default::default() };
        self.push(p);
    }

    /// A butterfly with wings of colour `tint`, around the flower at `at`.
    pub fn butterfly(&mut self, at: Vec3, tint: [f32; 3]) {
        let seed = self.rand();
        let p = Particle { pos: at, home: at, life: 14.0 + self.rand() * 12.0, size: 1.0, tint, kind: Kind::Butterfly, seed, spin: self.rand() * TAU, ..Default::default() };
        self.push(p);
    }

    /// A bee working the flowers around `at`.
    pub fn bee(&mut self, at: Vec3) {
        let seed = self.rand();
        let p = Particle { pos: at, home: at, life: 10.0 + self.rand() * 10.0, size: 1.0, tint: [1.0, 0.78, 0.18], kind: Kind::Bee, seed, spin: self.rand() * TAU, ..Default::default() };
        self.push(p);
    }

    /// A cloud of `n` gnats over `at`.
    pub fn gnats(&mut self, at: Vec3, n: usize) {
        let life = 10.0 + self.rand() * 8.0;
        for _ in 0..n {
            let seed = self.rand();
            let off = Vec3::new(self.rand() - 0.5, self.rand() - 0.5, self.rand() - 0.5) * 0.8;
            let p = Particle { pos: at + off, home: at, life: life * (0.85 + self.rand() * 0.3), size: 0.022, tint: [0.18, 0.17, 0.15], kind: Kind::Gnat, seed, ..Default::default() };
            self.push(p);
        }
    }

    /// A speck floating in the light: dust (grey-white) or pollen (pale yellow).
    pub fn speck(&mut self, at: Vec3, pollen: bool) {
        let seed = self.rand();
        let tint = if pollen { [1.25, 1.15, 0.6] } else { [1.15, 1.12, 1.05] };
        let vel = Vec3::new(self.rand() - 0.5, self.rand() * 0.2 - 0.05, self.rand() - 0.5) * 0.15;
        let p = Particle { pos: at, vel, life: 8.0 + self.rand() * 8.0, size: 0.016 + seed * 0.012, tint, kind: Kind::Speck, seed, ..Default::default() };
        self.push(p);
    }

    /// A flock of `n` birds in a loose V around `at`, all flying along `vel`.
    pub fn flock(&mut self, at: Vec3, vel: Vec3, n: usize, life: f32) {
        let fwd = Vec3::new(vel.x, 0.0, vel.z).normalize_or_zero();
        let side = Vec3::new(-fwd.z, 0.0, fwd.x);
        let heading = fwd.x.atan2(fwd.z);
        let shade = 0.25 + self.rand() * 0.2;
        for i in 0..n {
            // Leader in front, the rest alternating behind it to either side.
            let row = ((i + 1) / 2) as f32;
            let s = if i % 2 == 0 { 1.0 } else { -1.0 };
            let j = Vec3::new(self.rand() - 0.5, self.rand() - 0.5, self.rand() - 0.5) * 0.8;
            let pos = at - fwd * row * 1.6 + side * s * row * 1.4 + j;
            let seed = self.rand();
            let tint = [shade, shade * 0.95, shade * 0.9];
            let p = Particle { pos, vel, life, size: 0.85 + self.rand() * 0.3, tint, kind: Kind::Bird, seed, spin: heading, home: pos, ..Default::default() };
            self.push(p);
        }
    }

    /// A fish leaping up out of the water at `at` (the surface).
    pub fn fish(&mut self, at: Vec3, tint: [f32; 3]) {
        let a = self.rand() * TAU;
        let speed = 1.2 + self.rand() * 1.0;
        let vel = Vec3::new(a.cos() * speed, 3.6 + self.rand() * 1.4, a.sin() * speed);
        let p = Particle { pos: at - Vec3::Y * 0.1, vel, life: 3.0, size: 0.75 + self.rand() * 0.6, tint, kind: Kind::Fish, seed: self.rand(), ..Default::default() };
        self.push(p);
    }

    /// A ring spreading on water from `at` (on the surface), up to `radius` blocks.
    pub fn ripple(&mut self, at: Vec3, radius: f32) {
        let life = 0.9 + radius * 0.8;
        let p = Particle { pos: at, life, size: radius, tint: [0.82, 0.9, 1.0], kind: Kind::Ripple, home: Vec3::new(life, 0.0, 0.0), ..Default::default() };
        self.push(p);
    }

    pub fn update(&mut self, world: &World, dt: f32) {
        let wind = self.wind;
        let mut landed = Vec::new();
        for p in self.list.iter_mut() {
            p.life -= dt;
            p.age += dt;
            match p.kind {
                Kind::Glow => {
                    p.vel *= 1.0 - dt * 2.5;
                    p.pos += p.vel * dt;
                }
                Kind::Flame => {
                    // Hot air: pulled up harder the longer it burns, a little shimmy sideways.
                    let t = p.life * 23.0 + p.pos.x * 7.0;
                    p.vel.y += 3.5 * dt;
                    p.pos += (p.vel + Vec3::new(t.sin() * 0.12, 0.0, t.cos() * 0.12)) * dt;
                }
                Kind::Spark => {
                    let t = p.life * 9.0 + p.pos.z * 5.0;
                    p.vel.y -= 1.2 * dt;
                    p.vel *= 1.0 - dt * 1.2;
                    p.pos += (p.vel + Vec3::new(t.sin() * 0.3, 0.0, t.cos() * 0.3)) * dt;
                }
                Kind::Mote => {
                    // Sway side to side as it sinks; gone once it touches the ground.
                    let t = p.life * 2.3 + p.pos.y;
                    p.pos += (p.vel + Vec3::new(t.sin() * 0.35, 0.0, t.cos() * 0.25)) * dt;
                    if world.solid(p.pos.floor().as_ivec3()) || world.liquid(p.pos.floor().as_ivec3()) {
                        p.life = p.life.min(0.2);
                        p.vel = Vec3::ZERO;
                    }
                }
                Kind::Leaf => update_leaf(p, world, wind, dt),
                Kind::Firefly => {
                    // A slow wander that keeps coming back near where it started.
                    let s = p.seed * 40.0;
                    let t = p.age;
                    let want = Vec3::new((t * 0.31 + s).sin(), (t * 0.23 + s * 1.7).sin() * 0.4, (t * 0.27 + s * 2.3).cos()) * 0.45;
                    p.vel += (want + (p.home - p.pos) * 0.15 - p.vel) * dt * 0.8;
                    p.pos += p.vel * dt;
                    if world.solid(p.pos.floor().as_ivec3()) {
                        p.vel.y = p.vel.y.abs() + 0.2;
                        p.pos.y += dt * 0.5;
                    }
                }
                Kind::Butterfly => {
                    // Fluttering: a heading that keeps changing its mind, pulled back to its
                    // flower; each wing stroke lifts it a little.
                    let s = p.seed * 50.0;
                    let t = p.age;
                    p.spin += ((t * 1.7 + s).sin() * 2.2 + (t * 4.3 + s).sin() * 1.2) * dt;
                    let home = p.home + Vec3::Y * 0.8 - p.pos;
                    let leaving = p.life < 3.0;
                    let back = if leaving { Vec3::new(0.0, 0.4, 0.0) } else { home * 0.35 };
                    let dir = Vec3::new(p.spin.sin(), 0.0, p.spin.cos());
                    let flap = (t * 11.0 + s).sin();
                    let want = dir * 0.9 + back + Vec3::new(0.0, flap * 0.7, 0.0) + wind * 0.15;
                    p.vel += (want - p.vel) * dt * 4.0;
                    let next = p.pos + p.vel * dt;
                    if world.solid(next.floor().as_ivec3()) {
                        p.vel = Vec3::new(-p.vel.x, 0.6, -p.vel.z);
                        p.spin += PI;
                    } else {
                        p.pos = next;
                    }
                }
                Kind::Bee => {
                    // Darts to a new spot near its flower every half second or so, hovers there.
                    let step = (p.age * (1.6 + p.seed)).floor();
                    let h = hash(step + p.seed * 91.0);
                    let leaving = p.life < 2.5;
                    let target = if leaving {
                        p.home + Vec3::new((p.seed - 0.5) * 30.0, 8.0, (h - 0.5) * 30.0)
                    } else {
                        p.home + Vec3::new((h - 0.5) * 2.4, 0.35 + hash(step * 1.3 + 7.0) * 0.9, (hash(step * 2.1 + 3.0) - 0.5) * 2.4)
                    };
                    let to = target - p.pos;
                    let hover = Vec3::new((p.age * 13.0).sin(), (p.age * 17.0).cos(), (p.age * 11.0).cos()) * 0.25;
                    p.vel += (to * 4.0 + hover - p.vel) * dt * 6.0;
                    if to.length_squared() > 0.01 {
                        let want = to.x.atan2(to.z);
                        p.spin += wrap(want - p.spin) * (dt * 8.0).min(1.0);
                    }
                    let next = p.pos + p.vel * dt;
                    if !world.solid(next.floor().as_ivec3()) {
                        p.pos = next;
                    } else {
                        p.vel = Vec3::Y;
                    }
                }
                Kind::Gnat => {
                    // Each circles the cloud's middle on its own tilted orbit; the cloud bobs
                    // and drifts a little with the wind.
                    let s = p.seed * 60.0;
                    let t = p.age * (2.5 + p.seed * 2.0);
                    p.home += (wind * 0.04 + Vec3::new(0.0, (p.age * 0.5 + s).sin() * 0.08, 0.0)) * dt;
                    let r = 0.25 + p.seed * 0.45;
                    let orbit = Vec3::new(t.cos() * r, (t * 1.7 + s).sin() * 0.3, (t + s).sin() * r);
                    p.pos = p.home + orbit + Vec3::new((p.age * 7.0 + s).sin(), (p.age * 9.0).cos(), 0.0) * 0.03;
                }
                Kind::Speck => {
                    // Floating in still air: barely sinking, nudged by the wind and the air's stir.
                    let s = p.seed * 70.0;
                    let stir = Vec3::new((p.age * 0.4 + s).sin(), (p.age * 0.3 + s * 1.3).sin() * 0.6, (p.age * 0.35 + s * 0.7).cos()) * 0.06;
                    p.vel += (wind * 0.2 + stir - Vec3::Y * 0.01 - p.vel) * dt * 0.6;
                    p.pos += p.vel * dt;
                    if world.solid(p.pos.floor().as_ivec3()) {
                        p.life = p.life.min(0.5);
                    }
                }
                Kind::Bird => {
                    // Flying straight on, rising and sinking a little with its wingbeats.
                    let s = p.seed * 30.0;
                    p.pos += (p.vel + Vec3::new(0.0, (p.age * 0.9 + s).sin() * 0.5, 0.0)) * dt;
                }
                Kind::Fish => {
                    p.vel.y -= 14.0 * dt;
                    p.pos += p.vel * dt;
                    let wet = world.liquid(p.pos.floor().as_ivec3());
                    if !wet {
                        p.rest = 1;
                    } else if p.rest == 1 && p.vel.y < 0.0 {
                        landed.push(Vec3::new(p.pos.x, p.pos.y.floor() + 0.9, p.pos.z));
                        p.life = 0.0;
                    }
                }
                Kind::Ripple => {}
                Kind::Bubble => {
                    p.vel.x += ((p.pos.y * 9.0).sin()) * dt * 0.8;
                    p.pos += p.vel * dt;
                    if !world.liquid(p.pos.floor().as_ivec3()) {
                        p.life = 0.0;
                    }
                }
                Kind::Debris | Kind::Drop => {
                    p.vel.y -= 16.0 * dt;
                    p.vel *= 1.0 - dt * 0.8;
                    let next = p.pos + p.vel * dt;
                    let cell = next.floor().as_ivec3();
                    if world.solid(cell) {
                        if p.kind == Kind::Drop {
                            p.life = 0.0;
                        } else {
                            // Land and slide to a stop.
                            p.vel = Vec3::new(p.vel.x * 0.3, 0.0, p.vel.z * 0.3);
                        }
                    } else if p.kind == Kind::Drop && p.vel.y < 0.0 && world.liquid(cell) {
                        p.life = 0.0;
                    } else {
                        p.pos = next;
                    }
                }
            }
        }
        self.list.retain(|p| p.life > 0.0);
        for at in landed {
            self.ripple(at, 1.0);
            self.fish_landed.push(at);
        }
    }

    fn gpu_data(&self, world: &World, cam_block: IVec3) -> Vec<GpuParticle> {
        let c = cam_block.as_vec3();
        let mut out = Vec::with_capacity(self.list.len() + 16);
        for p in &self.list {
            let bright = matches!(p.kind, Kind::Glow | Kind::Flame | Kind::Spark | Kind::Firefly);
            let l = if bright { 0xff } else { world.light(p.pos.floor().as_ivec3()) };
            let rel = p.pos - c;
            let light = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let mut put = |at: Vec3, rot: Quat, scale: Vec3, tint: [f32; 3], mode: f32, alpha: f32, span: f32, uv: [f32; 2], layer: u32| {
                out.push(GpuParticle {
                    pos: [at.x, at.y, at.z, 1.0],
                    uv: [uv[0], uv[1], layer as f32, light.0],
                    tint: [tint[0], tint[1], tint[2], light.1],
                    rot: rot.to_array(),
                    shape: [scale.x, scale.y, scale.z, mode],
                    extra: [alpha, span, 0.0, 0.0],
                });
            };
            // Fading in at birth and out at the end.
            let fade = |inn: f32, out: f32| (p.age / inn).min(1.0) * (p.life / out).clamp(0.0, 1.0);
            match p.kind {
                Kind::Leaf => {
                    let rot = leaf_rot(p);
                    let s = p.size;
                    put(rel, rot, Vec3::new(s, s * 0.06, s), p.tint, 0.0, fade(0.3, 1.5), 0.32, p.uv, p.layer);
                }
                Kind::Firefly => {
                    // Light swells and dies away every few seconds, each at its own rhythm.
                    let period = 2.2 + p.seed * 2.5;
                    let ph = (p.age / period + p.seed).fract();
                    let on = if ph < 0.45 { (ph / 0.45 * PI).sin().powf(1.5) } else { 0.0 };
                    let k = fade(1.0, 1.5);
                    let b = (0.12 + on * 2.8) * k;
                    let s = p.size * (0.5 + on * 0.5);
                    put(rel, Quat::IDENTITY, Vec3::splat(s), [p.tint[0] * b, p.tint[1] * b, p.tint[2] * b], 1.0, k.max(0.2), 0.25, p.uv, p.layer);
                    if on > 0.2 {
                        // A faint halo of light around it while it shines.
                        let h = b * 0.35;
                        put(rel, Quat::IDENTITY, Vec3::splat(p.size * 3.0), [p.tint[0] * h, p.tint[1] * h, p.tint[2] * h], 1.0, on * k * 0.3, 0.25, p.uv, p.layer);
                    }
                }
                Kind::Butterfly => {
                    let a = 0.35 + (p.age * 11.0 + p.seed * 50.0).sin() * 0.95;
                    let body = Quat::from_rotation_y(p.spin);
                    let k = fade(0.5, 1.0);
                    put(rel, body, Vec3::new(0.022, 0.022, 0.08), [0.12, 0.1, 0.08], 1.0, k, 0.25, p.uv, p.layer);
                    let (w, d) = (0.075, 0.085);
                    for side in [1.0f32, -1.0] {
                        let r = body * Quat::from_rotation_z(a * side);
                        put(rel + r * Vec3::new(side * w * 0.5, 0.0, 0.0), r, Vec3::new(w, 0.006, d), p.tint, 1.0, k, 0.25, p.uv, p.layer);
                    }
                }
                Kind::Bee => {
                    let body = Quat::from_rotation_y(p.spin);
                    let k = fade(0.3, 1.0);
                    // Yellow body with a dark tail end, and two pale wings buzzing.
                    put(rel, body, Vec3::new(0.05, 0.045, 0.06), p.tint, 1.0, k, 0.25, p.uv, p.layer);
                    put(rel + body * Vec3::new(0.0, 0.0, -0.045), body, Vec3::new(0.042, 0.04, 0.035), [0.12, 0.1, 0.06], 1.0, k, 0.25, p.uv, p.layer);
                    let a = 0.4 + (p.age * 70.0).sin() * 0.5;
                    for side in [1.0f32, -1.0] {
                        let r = body * Quat::from_rotation_z(a * side);
                        put(rel + Vec3::Y * 0.025 + r * Vec3::new(side * 0.025, 0.0, 0.0), r, Vec3::new(0.05, 0.004, 0.035), [0.9, 0.92, 0.95], 1.0, k * 0.8, 0.25, p.uv, p.layer);
                    }
                }
                Kind::Gnat => {
                    put(rel, Quat::IDENTITY, Vec3::splat(p.size), p.tint, 1.0, fade(1.5, 2.0), 0.25, p.uv, p.layer);
                }
                Kind::Speck => {
                    // A soft twinkle as it turns in the light.
                    let tw = 0.75 + (p.age * 1.3 + p.seed * 40.0).sin() * 0.25;
                    let t = [p.tint[0] * tw, p.tint[1] * tw, p.tint[2] * tw];
                    let near = ((p.pos.distance(self.eye) - 1.5) / 2.0).clamp(0.0, 1.0);
                    put(rel, Quat::IDENTITY, Vec3::splat(p.size), t, 1.0, fade(2.0, 2.0) * 0.8 * near, 0.25, p.uv, p.layer);
                }
                Kind::Bird => {
                    // Beats its wings for a while, then glides with them held out.
                    let cycle = (p.age * 0.35 + p.seed).fract();
                    let a = if cycle < 0.6 { (p.age * 7.5 + p.seed * 9.0).sin() * 0.75 } else { 0.08 };
                    let body = Quat::from_rotation_y(p.spin);
                    let s = p.size;
                    let k = fade(1.5, 3.0);
                    put(rel, body, Vec3::new(0.12, 0.1, 0.38) * s, p.tint, 1.0, k, 0.25, p.uv, p.layer);
                    for side in [1.0f32, -1.0] {
                        let r = body * Quat::from_rotation_z(a * side);
                        put(rel + r * Vec3::new(side * 0.26 * s, 0.0, 0.0), r * Quat::from_rotation_y(-0.25 * side), Vec3::new(0.5, 0.02, 0.18) * s, p.tint, 1.0, k, 0.25, p.uv, p.layer);
                    }
                }
                Kind::Fish => {
                    // Nose along its flight, tail flicking.
                    let v = p.vel;
                    let yaw = v.x.atan2(v.z);
                    let pitch = -v.y.atan2(Vec3::new(v.x, 0.0, v.z).length());
                    let rot = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch) * Quat::from_rotation_y((p.age * 22.0).sin() * 0.25);
                    let s = p.size;
                    put(rel, rot, Vec3::new(0.07, 0.12, 0.3) * s, p.tint, 1.0, 1.0, 0.25, p.uv, p.layer);
                    put(rel + rot * Vec3::new(0.0, 0.0, -0.19 * s), rot, Vec3::new(0.02, 0.11, 0.08) * s, [p.tint[0] * 0.8, p.tint[1] * 0.8, p.tint[2] * 0.8], 1.0, 1.0, 0.25, p.uv, p.layer);
                }
                Kind::Ripple => {
                    // Grows out fast at first, then slows, thinning out as it goes.
                    let life0 = p.home.x.max(0.01);
                    let k = (1.0 - p.life / life0).clamp(0.0, 1.0);
                    let r = p.size * (1.0 - (1.0 - k) * (1.0 - k)) + 0.08;
                    put(rel, Quat::IDENTITY, Vec3::new(r * 2.0, 0.004, r * 2.0), p.tint, 2.0, (1.0 - k) * 0.85, 0.25, p.uv, p.layer);
                }
                _ => {
                    // Shrink away at the end of life.
                    let mut s = p.size * (p.life / 0.25).clamp(0.0, 1.0);
                    let mut tint = p.tint;
                    if p.kind == Kind::Flame {
                        // White-yellow at the root, orange, then a deep red as it dies; it swells
                        // a little on the way up before shrinking away.
                        let k = (p.life / 0.45).clamp(0.0, 1.0);
                        let soul = p.tint == SOUL;
                        let hot = if soul { [0.9, 1.6, 1.7] } else { [1.6, 1.35, 0.75] };
                        let mid = if soul { [0.25, 1.0, 1.4] } else { [1.5, 0.7, 0.18] };
                        let cold = if soul { [0.05, 0.3, 0.8] } else { [0.9, 0.22, 0.06] };
                        let (a, b, f) = if k > 0.5 { (mid, hot, (k - 0.5) * 2.0) } else { (cold, mid, k * 2.0) };
                        tint = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f];
                        s *= 1.0 + (1.0 - k) * 0.6;
                    }
                    put(rel, Quat::IDENTITY, Vec3::splat(s), tint, 0.0, 1.0, 0.25, p.uv, p.layer);
                }
            }
        }
        out
    }
}

/// A leaf in the air seesaws: it swings from side to side across the wind, slowing and lifting
/// a little at the end of each swing, while the wind carries it along. Once down it lies flat on
/// the ground, or floats and drifts on water, until it fades.
fn update_leaf(p: &mut Particle, world: &World, wind: Vec3, dt: f32) {
    let cell = p.pos.floor().as_ivec3();
    match p.rest {
        1 => {}
        2 => {
            p.pos += (wind * 0.08 + Vec3::new((p.age * 0.5).sin(), 0.0, (p.age * 0.4).cos()) * 0.03) * dt;
            if !world.liquid(p.pos.floor().as_ivec3()) {
                p.life = p.life.min(1.0);
            }
        }
        _ => {
            let w = 2.2 + p.seed * 1.4;
            let swing = (p.age * w + p.seed * 20.0).sin();
            let ax = p.seed * TAU;
            let across = Vec3::new(ax.cos(), 0.0, ax.sin());
            let fall = 0.55 + p.seed * 0.35 - swing.abs() * 0.25;
            let want = wind * 0.65 + across * swing * 0.9 - Vec3::Y * fall;
            p.vel += (want - p.vel) * (dt * 3.0).min(1.0);
            p.spin += dt * (1.0 + p.seed * 2.0);
            let next = p.pos + p.vel * dt;
            let nc = next.floor().as_ivec3();
            if world.solid(nc) {
                if nc.y < cell.y {
                    // Settled on top of the block below.
                    p.rest = 1;
                    p.pos.y = cell.y as f32 + 0.02;
                    p.life = p.life.min(4.0 + p.seed * 5.0);
                } else {
                    // Blown against a wall: slide down it.
                    p.vel.x = 0.0;
                    p.vel.z = 0.0;
                }
            } else if world.liquid(nc) {
                p.rest = 2;
                p.pos = Vec3::new(next.x, nc.y as f32 + 0.88, next.z);
                p.life = p.life.min(6.0 + p.seed * 6.0);
            } else {
                p.pos = next;
            }
            if p.age > 30.0 {
                p.life = p.life.min(1.0);
            }
        }
    }
}

fn leaf_rot(p: &Particle) -> Quat {
    let yaw = Quat::from_rotation_y(p.seed * TAU * 3.0);
    if p.rest != 0 {
        return yaw;
    }
    // Tilted into each swing and slowly turning over.
    let w = 2.2 + p.seed * 1.4;
    let swing = (p.age * w + p.seed * 20.0).sin();
    let ax = p.seed * TAU;
    Quat::from_rotation_y(ax) * Quat::from_rotation_z(swing * 0.7) * Quat::from_rotation_x(p.spin.sin() * 1.1) * yaw
}

fn hash(x: f32) -> f32 {
    ((x * 12.9898).sin() * 43758.547).fract().abs()
}

/// An angle brought into -π..π.
fn wrap(a: f32) -> f32 {
    (a + PI).rem_euclid(TAU) - PI
}

impl GpuParticle {
    /// A plain coloured box fixed in the world (sign letters use these): centre relative to
    /// the camera's block, turned by `rot`, `size` along its own axes, lit by (sky, block) 0..1.
    pub(crate) fn plain(at: Vec3, rot: Quat, size: Vec3, tint: [f32; 3], light: (f32, f32)) -> GpuParticle {
        GpuParticle {
            pos: [at.x, at.y, at.z, 1.0],
            uv: [0.0, 0.0, 0.0, light.0],
            tint: [tint[0], tint[1], tint[2], light.1],
            rot: rot.to_array(),
            shape: [size.x, size.y, size.z, 1.0],
            extra: [1.0, 0.25, 0.0, 0.0],
        }
    }
}

pub struct ParticleGpu {
    pipelines: [wgpu::RenderPipeline; 2],
    layout: wgpu::BindGroupLayout,
    buf: wgpu::Buffer,
    bg: wgpu::BindGroup,
    cap: usize,
    count: u32,
}

impl ParticleGpu {
    pub fn new(r: &Renderer) -> ParticleGpu {
        let device = &r.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particles"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle"),
            source: wgpu::ShaderSource::Wgsl(format!("{}\n{}", include_str!("shaders/common.wgsl"), include_str!("shaders/particle.wgsl")).into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle"),
            bind_group_layouts: &[&r.globals_layout, &r.tex_layout, &layout],
            push_constant_ranges: &[],
        });
        let make = |format: wgpu::TextureFormat| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("particle"),
                layout: Some(&pl),
                vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_particle"), buffers: &[], compilation_options: Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_particle"),
                    targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let pipelines = [make(r.config.format), make(crate::post::HDR)];
        let cap = 256;
        let buf = Self::make_buf(device, cap);
        let bg = Self::make_bg(device, &layout, &buf);
        ParticleGpu { pipelines, layout, buf, bg, cap, count: 0 }
    }

    fn make_buf(device: &wgpu::Device, cap: usize) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particles"),
            contents: &vec![0u8; cap * std::mem::size_of::<GpuParticle>()],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn make_bg(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, buf: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("particles"), layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }] })
    }

    /// `extra`: more boxes drawn the same way (the letters on signs).
    pub fn prepare(&mut self, r: &Renderer, particles: &Particles, world: &World, cam_block: IVec3, extra: Vec<GpuParticle>) {
        let mut data = particles.gpu_data(world, cam_block);
        data.extend(extra);
        self.count = data.len() as u32;
        if data.is_empty() {
            return;
        }
        if data.len() > self.cap {
            self.cap = data.len().next_power_of_two();
            self.buf = Self::make_buf(&r.device, self.cap);
            self.bg = Self::make_bg(&r.device, &self.layout, &self.buf);
        }
        r.queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(&data));
    }

    /// Needs the block texture bind group in slot 1.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, hdr: bool, block_tex: &'a wgpu::BindGroup) {
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipelines[hdr as usize]);
        pass.set_bind_group(1, block_tex, &[]);
        pass.set_bind_group(2, &self.bg, &[]);
        pass.draw(0..36, 0..self.count);
    }
}
