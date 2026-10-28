//! Life in the air around the player (ideas from Subtle Effects, Particle Rain and Falling
//! Leaves; our own code): leaves letting go of the trees and seesawing down with the wind,
//! fireflies blinking over grass and water on warm nights, butterflies and bees around flowers
//! by day, clouds of gnats over swamps and still water, dust and pollen floating in the
//! sunlight, flocks of birds passing high over, fish leaping out of lakes and rivers, and rings
//! spreading on the water where something touched it.
//!
//! All of it is cheap particles (particles.rs). Places are picked at random near the player,
//! less often the farther away they are, and the "Vida no ambiente" option scales how much of
//! it there is (or turns it off).

use glam::{IVec3, Vec2, Vec3};
use voxel::block::Shape;
use voxel::worldgen::{Biome, Dim};
use voxel::AIR;

use crate::audio::Sfx;
use crate::particles::Kind;
use crate::sfx_extra::Fx;
use crate::Game;

/// The steps of the "Vida no ambiente" option and how much each spawns.
pub const LEVELS: [(&str, f32); 4] = [("Desligada", 0.0), ("Pouca", 0.4), ("Normal", 1.0), ("Muita", 2.0)];

#[derive(Default)]
pub struct Life {
    leaf_t: f32,
    bug_t: f32,
    speck_t: f32,
    ripple_t: f32,
    bird_t: f32,
    fish_t: f32,
    /// The flock now passing over, to hear its wings as it comes closest.
    flock: Option<Flock>,
}

struct Flock {
    pos: Vec3,
    vel: Vec3,
    dist: f32,
    heard: bool,
    life: f32,
}

/// Biomes warm and green enough for fireflies.
fn firefly_land(b: Biome) -> bool {
    matches!(
        b,
        Biome::Plains
            | Biome::Forest
            | Biome::BirchForest
            | Biome::DarkForest
            | Biome::Swamp
            | Biome::MangroveSwamp
            | Biome::Jungle
            | Biome::CherryGrove
            | Biome::River
            | Biome::Meadow
            | Biome::FlowerForest
            | Biome::SunflowerPlains
            | Biome::MataAtlantica
            | Biome::Pantanal
            | Biome::Cerrado
            | Biome::Araucaria
    )
}

fn wet_land(b: Biome) -> bool {
    matches!(b, Biome::Swamp | Biome::MangroveSwamp | Biome::Pantanal | Biome::River)
}

fn cold_land(b: Biome) -> bool {
    matches!(b, Biome::Snowy | Biome::SnowyPeaks | Biome::FrozenOcean | Biome::FrozenRiver | Biome::SnowyBeach | Biome::SnowyTaiga | Biome::SnowySlopes | Biome::IceSpikes | Biome::JaggedPeaks | Biome::Grove)
}

fn flowery(b: Biome) -> bool {
    matches!(b, Biome::Meadow | Biome::FlowerForest | Biome::SunflowerPlains | Biome::Plains | Biome::CherryGrove | Biome::Cerrado)
}

pub(crate) fn is_flower(name: &str) -> bool {
    !(name.contains("grass") || name.contains("fern") || name.contains("sapling") || name.contains("_stage") || name.contains("mushroom") || name.contains("bush") || name.contains("chorus") || name.contains("roots") || name.contains("vine") || name.contains("cane") || name.contains("bamboo") || name.contains("dead") || name.contains("crop") || name.contains("wheat") || name.contains("carrot") || name.contains("potato") || name.contains("beet"))
}

/// Wing colours of butterflies.
const WINGS: [[f32; 3]; 6] = [[1.0, 0.6, 0.15], [0.35, 0.55, 1.0], [1.0, 1.0, 0.92], [1.0, 0.85, 0.2], [0.85, 0.4, 0.9], [0.95, 0.35, 0.2]];

impl Game {
    /// How much ambient life the options ask for (0 = none).
    pub(crate) fn life_amount(&self) -> f32 {
        LEVELS[self.settings.ambient_life.min(3) as usize].1
    }

    /// The colour a leaf of block `id` falls with in the place `p`.
    fn leaf_tint(&mut self, id: voxel::BlockId, p: IVec3) -> [f32; 3] {
        let b = self.reg.get(id);
        let mut t = if b.tint { self.world.gen.biome_at(p.x, p.z).tint() } else { [1.0; 3] };
        let r = &mut self.surv.rng;
        let v = 0.82 + r.f32() * 0.28;
        t = [t[0] * v, t[1] * v, t[2] * v];
        if b.tint && r.f32() < 0.18 {
            // An old leaf, gone yellow.
            t = [t[0] * 1.2, t[1] * 1.05, t[2] * 0.6];
        }
        t
    }

    /// A random column near the player and how far it is (0..1 of `range`); None past it.
    fn life_column(&mut self, range: i32) -> Option<(i32, i32, f32)> {
        let me = self.body.pos.floor().as_ivec3();
        let r = &mut self.surv.rng;
        let (dx, dz) = (r.range(-range, range + 1), r.range(-range, range + 1));
        let d = ((dx * dx + dz * dz) as f32).sqrt() / range as f32;
        (d <= 1.0).then_some((me.x + dx, me.z + dz, d))
    }

    /// The ground (or water) of a column, under any tree crowns.
    fn floor_under_trees(&self, x: i32, z: i32) -> Option<i32> {
        let mut y = self.world.surface_y(x, z)?;
        for _ in 0..24 {
            let id = self.world.block_or_air(IVec3::new(x, y, z));
            let b = self.reg.get(id);
            let canopy = id == AIR || b.name.ends_with("_leaves") || b.name.ends_with("_log") || b.name.contains("vine") || !(b.solid || self.world.is_liquid(id));
            if !canopy {
                return Some(y);
            }
            y -= 1;
        }
        None
    }

    pub(crate) fn update_life(&mut self, dt: f32) {
        let eye = self.body.eye();
        let rain = self.weather.rain;
        self.particles.eye = eye;
        self.particles.wind = crate::wind::at(self.now, Vec2::new(eye.x, eye.z), rain, self.weather.state.thunder);
        let water = self.reg.get(self.reg.id_or_air("water")).faces[2];
        for at in std::mem::take(&mut self.particles.fish_landed) {
            self.particles.splash(at, water, 0.0);
            self.sfx_at(Sfx::Fx(Fx::FishSplash), at, 0.22, 20.0);
        }
        self.update_flock(dt, eye);
        let k = self.life_amount();
        if dt <= 0.0 || k <= 0.0 || self.dim != Dim::Overworld {
            return;
        }
        let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
        let sun = (a.sin() * 2.2 + 0.45).clamp(0.0, 1.0);
        let (day, night) = (sun > 0.6, sun < 0.25);
        let clear = rain < 0.2;
        let me = self.body.pos.floor().as_ivec3();
        let sky_here = (self.world.light(eye.floor().as_ivec3()) >> 4) as f32;
        let gust = crate::wind::gust(self.now, Vec2::new(eye.x, eye.z));

        // Leaves: a column near the player is looked up through for the underside of a tree's
        // crown. More let go in gusts and rain. Autumn's are in seasons.rs.
        self.life.leaf_t += dt * 160.0 * k * (0.45 + gust * 1.4 + rain * 0.5);
        let n = self.life.leaf_t as usize;
        self.life.leaf_t -= n as f32;
        let leaf_cap = (320.0 * k) as usize;
        let mut leaves = self.particles.count(Kind::Leaf);
        for _ in 0..n.min(64) {
            let Some((x, z, d)) = self.life_column(16) else { continue };
            if leaves >= leaf_cap || self.surv.rng.f32() > 0.6 * (1.0 - d).sqrt() {
                continue;
            }
            let mut below = self.world.block_or_air(IVec3::new(x, me.y - 5, z));
            for y in me.y - 4..me.y + 19 {
                let p = IVec3::new(x, y, z);
                let id = self.world.block_or_air(p);
                if below == AIR && id != AIR && self.reg.get(id).name.ends_with("_leaves") {
                    let petal = self.reg.get(id).name.contains("cherry");
                    let tint = self.leaf_tint(id, p);
                    let (layer, _) = self.block_look(id);
                    let r = &mut self.surv.rng;
                    let size = if petal { 0.09 + r.f32() * 0.04 } else { 0.15 + r.f32() * 0.07 };
                    let at = p.as_vec3() + Vec3::new(r.f32(), -0.05, r.f32());
                    self.particles.leaf(at, layer, tint, size);
                    leaves += 1;
                    break;
                }
                below = id;
            }
        }

        // Insects on the ground and over water.
        self.life.bug_t += dt * if night { 70.0 } else { 45.0 } * k;
        let n = self.life.bug_t as usize;
        self.life.bug_t -= n as f32;
        let (mut flies, mut butterflies, mut bees, mut gnats) =
            (self.particles.count(Kind::Firefly), self.particles.count(Kind::Butterfly), self.particles.count(Kind::Bee), self.particles.count(Kind::Gnat));
        for _ in 0..n.min(32) {
            if !clear {
                break;
            }
            let Some((x, z, d)) = self.life_column(18) else { continue };
            let Some(y) = self.floor_under_trees(x, z) else { continue };
            if (y - me.y).abs() > 20 {
                continue;
            }
            let near = (1.0 - d).sqrt();
            let biome = self.world.gen.biome_at(x, z);
            let top = self.world.block_or_air(IVec3::new(x, y, z));
            let above = self.world.block_or_air(IVec3::new(x, y + 1, z));
            let wet = self.world.is_liquid(top) && self.world.fluid(top) == 1;
            let roll = self.surv.rng.f32() / near.max(0.05);
            let jit = Vec3::new(self.surv.rng.f32(), self.surv.rng.f32(), self.surv.rng.f32());
            let base = Vec3::new(x as f32, y as f32 + 1.0, z as f32);
            if night && firefly_land(biome) && flies < (70.0 * k) as usize {
                let grassy = self.reg.get(top).name == "grass_block" || self.reg.get(above).shape == Shape::Cross;
                let chance = if wet || wet_land(biome) { 0.5 } else if grassy { 0.3 } else { 0.0 };
                if roll < chance {
                    self.particles.firefly(base + Vec3::new(jit.x, 0.8 + jit.y * 1.6, jit.z));
                    flies += 1;
                }
                continue;
            }
            if wet && !night && above == AIR && gnats < (40.0 * k) as usize && roll < if wet_land(biome) || !day { 0.06 } else { 0.012 } {
                self.particles.gnats(base + Vec3::new(0.5, 0.7 + jit.y * 0.8, 0.5), 10 + (jit.x * 8.0) as usize);
                gnats += 14;
                continue;
            }
            if !day || cold_land(biome) {
                continue;
            }
            let ab = self.reg.get(above);
            if ab.shape == Shape::Cross && is_flower(&ab.name) {
                let at = base + Vec3::new(0.5, 0.5, 0.5);
                if butterflies < (10.0 * k).ceil() as usize && roll < 0.22 {
                    let tint = WINGS[((jit.x * 97.0) as usize) % WINGS.len()];
                    self.particles.butterfly(at, tint);
                    butterflies += 1;
                } else if bees < (8.0 * k).ceil() as usize && roll < 0.32 {
                    self.particles.bee(at);
                    bees += 1;
                }
            }
        }

        // Dust and pollen floating in the sunlight around the player.
        if day && clear && sky_here >= 12.0 {
            self.life.speck_t += dt * 5.0 * k;
            let n = self.life.speck_t as usize;
            self.life.speck_t -= n as f32;
            let mut specks = self.particles.count(Kind::Speck);
            let pollen_land = flowery(self.world.gen.biome_at(me.x, me.z));
            for _ in 0..n.min(8) {
                let r = &mut self.surv.rng;
                let at = eye + Vec3::new(r.f32() - 0.5, r.f32() * 0.6 - 0.35, r.f32() - 0.5) * 10.0;
                if at.distance(eye) < 2.5 {
                    continue;
                }
                let pollen = pollen_land && r.f32() < 0.75;
                let p = at.floor().as_ivec3();
                if specks < (35.0 * k) as usize && self.world.block_or_air(p) == AIR && (self.world.light(p) >> 4) >= 15 {
                    self.particles.speck(at, pollen);
                    specks += 1;
                }
            }
        }

        // Rings on still water: a water strider, a drop off a leaf, a fish nosing the surface.
        if clear && !night {
            self.life.ripple_t += dt * 2.0 * k;
            let n = self.life.ripple_t as usize;
            self.life.ripple_t -= n as f32;
            for _ in 0..n.min(4) {
                let Some((x, z, _)) = self.life_column(14) else { continue };
                let Some(y) = self.world.surface_y(x, z) else { continue };
                let top = self.world.block_or_air(IVec3::new(x, y, z));
                if self.world.fluid(top) == 1 && (y - me.y).abs() < 12 && self.world.block_or_air(IVec3::new(x, y + 1, z)) == AIR {
                    let r = &mut self.surv.rng;
                    let at = Vec3::new(x as f32 + r.f32(), y as f32 + 0.9, z as f32 + r.f32());
                    let size = 0.3 + r.f32() * 0.25;
                    self.particles.ripple(at, size);
                }
            }
        }

        // A fish jumping now and then.
        self.life.fish_t -= dt * k;
        if self.life.fish_t <= 0.0 {
            self.life.fish_t = 3.0 + self.surv.rng.f32() * 7.0;
            self.leap_fish(false);
        }

        // A flock of birds every minute or so, under an open sky.
        self.life.bird_t -= dt * k;
        if self.life.bird_t <= 0.0 {
            self.life.bird_t = 35.0 + self.surv.rng.f32() * 50.0;
            if day && clear && sky_here >= 13.0 {
                self.send_flock(false);
            }
        }
    }

    /// A fish leaping out of open water near the player (or right in front, for test
    /// pictures). False if no water was found.
    pub(crate) fn leap_fish(&mut self, in_front: bool) -> bool {
        let me = self.body.pos.floor().as_ivec3();
        for _ in 0..16 {
            let (x, z) = if in_front {
                let f = self.forward();
                let d = 5.0 + self.surv.rng.f32() * 4.0;
                let at = self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * d;
                (at.x.floor() as i32, at.z.floor() as i32)
            } else {
                let Some((x, z, d)) = self.life_column(18) else { continue };
                if d < 0.2 {
                    continue;
                }
                (x, z)
            };
            let Some(y) = self.world.surface_y(x, z) else { continue };
            let top = self.world.block_or_air(IVec3::new(x, y, z));
            let deep = self.world.fluid(self.world.block_or_air(IVec3::new(x, y - 1, z))) == 1;
            if self.world.fluid(top) != 1 || !deep || (y - me.y).abs() > 14 || self.world.block_or_air(IVec3::new(x, y + 1, z)) != AIR {
                continue;
            }
            let biome = self.world.gen.biome_at(x, z);
            let tint = match biome {
                Biome::Ocean | Biome::DeepOcean | Biome::ColdOcean | Biome::LukewarmOcean | Biome::Beach => [0.72, 0.78, 0.85],
                Biome::WarmOcean => [0.95, 0.6, 0.3],
                Biome::Swamp | Biome::MangroveSwamp | Biome::Pantanal => [0.5, 0.48, 0.32],
                _ => [0.58, 0.62, 0.5],
            };
            let r = &mut self.surv.rng;
            let at = Vec3::new(x as f32 + r.f32(), y as f32 + 0.9, z as f32 + r.f32());
            self.particles.fish(at, tint);
            self.particles.ripple(at, 0.9);
            let water = self.reg.get(self.reg.id_or_air("water")).faces[2];
            self.particles.splash(at, water, 0.0);
            self.sfx_at(Sfx::Fx(Fx::FishSplash), at, 0.18, 20.0);
            return true;
        }
        false
    }

    /// A flock of birds flying across the sky over the player (or ahead of the camera and
    /// already in view, for test pictures).
    pub(crate) fn send_flock(&mut self, in_view: bool) {
        let eye = self.body.eye();
        let r = &mut self.surv.rng;
        let (fwd, start) = if in_view {
            let f = self.forward();
            let ahead = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
            let side = Vec3::new(-ahead.z, 0.0, ahead.x);
            // Crossing the view from left to right, a little ahead and above.
            (side, eye + ahead * 26.0 - side * 6.0 + Vec3::Y * 9.0)
        } else {
            let a = r.f32() * std::f32::consts::TAU;
            let fwd = Vec3::new(a.cos(), 0.0, a.sin());
            let side = Vec3::new(-fwd.z, 0.0, fwd.x);
            let ground = self.world.surface_y(eye.x.floor() as i32, eye.z.floor() as i32).unwrap_or(eye.y as i32) as f32;
            let h = ground.max(eye.y) + 22.0 + r.f32() * 14.0;
            let off = (r.f32() - 0.5) * 40.0;
            let at = Vec3::new(eye.x, h, eye.z) - fwd * 70.0 + side * off;
            (fwd, at)
        };
        let r = &mut self.surv.rng;
        let speed = 6.5 + r.f32() * 2.5;
        let n = 3 + (r.f32() * 6.0) as usize;
        let life = 140.0 / speed;
        self.particles.flock(start, fwd * speed, n, life);
        self.life.flock = Some(Flock { pos: start, vel: fwd * speed, dist: f32::MAX, heard: false, life });
    }

    /// The flock's wings are heard once, as it passes closest.
    fn update_flock(&mut self, dt: f32, eye: Vec3) {
        let Some(f) = self.life.flock.as_mut() else { return };
        f.pos += f.vel * dt;
        f.life -= dt;
        let d = f.pos.distance(eye);
        let passing = d > f.dist && f.dist < 40.0 && !f.heard;
        f.dist = d;
        let (pos, done) = (f.pos, f.life <= 0.0);
        if passing {
            f.heard = true;
            self.sfx_at(Sfx::Fx(Fx::Wings), pos, 0.35, 50.0);
        }
        if done {
            self.life.flock = None;
        }
    }

    /// Test hook (MC_LIFE=...): put life in front of the camera for pictures, then let it run
    /// a few seconds. `birds`, `fish`, `bugs` (butterflies and bees on flowers ahead), or
    /// `all`.
    pub(crate) fn test_life(&mut self, what: &str) {
        let all = what == "all";
        if all || what.contains("birds") {
            self.send_flock(true);
        }
        if all || what.contains("leaves") {
            // A few leaves already on their way down, just ahead.
            let f = self.forward();
            let ahead = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
            let side = Vec3::new(-ahead.z, 0.0, ahead.x);
            let id = self.reg.id_or_air("oak_leaves");
            let (layer, _) = self.block_look(id);
            for i in 0..10 {
                let at = self.body.eye() + ahead * (1.8 + (i % 4) as f32 * 0.6) + side * ((i as f32 - 4.5) * 0.35) + Vec3::Y * (0.6 + (i % 3) as f32 * 0.4);
                let tint = self.leaf_tint(id, at.floor().as_ivec3());
                self.particles.leaf(at, layer, tint, 0.18);
            }
        }
        if all || what.contains("bugs") {
            let f = self.forward();
            let ahead = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
            let side = Vec3::new(-ahead.z, 0.0, ahead.x);
            for i in 0..5 {
                let at = self.body.eye() + ahead * (2.5 + i as f32 * 0.7) + side * ((i as f32 - 2.0) * 0.8) - Vec3::Y * 0.6;
                if i % 2 == 0 {
                    self.particles.butterfly(at, WINGS[i % WINGS.len()]);
                } else {
                    self.particles.bee(at);
                }
            }
        }
        for _ in 0..40 {
            self.update_life(0.05);
            self.particles.update(&self.world, 0.05);
        }
        if all || what.contains("fish") {
            // Caught at the top of its leap.
            if self.leap_fish(true) {
                for _ in 0..5 {
                    self.particles.update(&self.world, 0.05);
                }
            }
        }
    }
}
