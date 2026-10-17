//! Small things in the air around the player (Subtle Effects idea): leaves drifting down
//! under trees, fireflies over grass on warm nights, embers rising off lava, dust in caves,
//! ash in the Nether and violet specks in the End. A few random blocks near the player are
//! looked at each tick; each kind of block has its own small chance to give off something.

use glam::{IVec3, Vec3};
use voxel::block::Shape;
use voxel::worldgen::{Biome, Dim};
use voxel::AIR;

use crate::Game;

/// Blocks looked at per second.
const SAMPLES: f32 = 260.0;
const RANGE: i32 = 14;

impl Game {
    /// Test hook: stand in the nearest open cave of a cave biome (MC_CAVE=lush|frozen|dripstone).
    pub(crate) fn test_cave(&mut self, kind: &str) {
        use voxel::worldgen::CaveBiome;
        let want = match kind {
            "frozen" => CaveBiome::Frozen,
            "dripstone" => CaveBiome::Dripstone,
            _ => CaveBiome::Lush,
        };
        let me = self.body.pos.floor().as_ivec3();
        for r in (0..1200).step_by(8) {
            for k in 0..(r / 4).max(1) {
                let a = k as f32 / (r / 4).max(1) as f32 * std::f32::consts::TAU;
                let (x, z) = (me.x + (a.cos() * r as f32) as i32, me.z + (a.sin() * r as f32) as i32);
                if self.world.gen.cave_biome(x, z) != want {
                    continue;
                }
                let cp = glam::IVec2::new(x.div_euclid(16), z.div_euclid(16));
                let chunk = self.world.gen.generate(cp);
                let (lx, lz) = (x.rem_euclid(16) as usize, z.rem_euclid(16) as usize);
                let at = |y: i32| chunk.get(lx, y as usize, lz);
                for y in (14..50).rev() {
                    let free = (0..3).all(|k| at(y + k) == AIR);
                    if free && at(y - 1) != AIR && self.reg.get(at(y - 1)).solid {
                        self.body.pos = Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5);
                        log::info!("cave {kind} at {x},{y},{z}");
                        return;
                    }
                }
            }
        }
        log::warn!("no {kind} cave found");
    }

    /// Listen to the space around the player: how many directions hit stone close by, how
    /// far, and whether sky light reaches here (Sound Physics idea).
    fn measure_room(&mut self) {
        let eye = self.body.eye();
        let head = eye.floor().as_ivec3();
        let underwater = self.world.is_liquid(self.world.block_or_air(head));
        let mut hits = 0;
        let mut dist = 0.0;
        let dirs = [
            Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z,
            Vec3::new(1.0, 1.0, 1.0), Vec3::new(-1.0, 1.0, 1.0), Vec3::new(1.0, 1.0, -1.0), Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, -0.3, 1.0), Vec3::new(-1.0, -0.3, -1.0),
        ];
        for d in dirs {
            let d = d.normalize();
            let mut t = 1.0;
            while t < 24.0 {
                let p = (eye + d * t).floor().as_ivec3();
                let b = self.world.block_or_air(p);
                if b != AIR && self.reg.get(b).solid {
                    hits += 1;
                    dist += t;
                    break;
                }
                t += 1.0;
            }
        }
        let frac = hits as f32 / dirs.len() as f32;
        let sky = (self.world.light(head) >> 4) as f32 / 15.0;
        let echo = if self.dim == Dim::Overworld { frac * frac * (1.0 - sky * 0.8) } else { frac * frac * 0.8 };
        let delay = if hits > 0 { dist / hits as f32 * 2.0 / 343.0 } else { 0.0 };
        if let Some(a) = self.audio.as_mut() {
            a.room = crate::audio::Room { echo, delay: delay.max(0.04), underwater };
        }
    }

    pub(crate) fn update_ambient(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        self.room_t -= dt;
        self.melody_cd -= dt;
        if self.room_t <= 0.0 {
            self.room_t = 0.4;
            self.measure_room();
        }
        self.ambient_t += dt * SAMPLES;
        let n = self.ambient_t as usize;
        self.ambient_t -= n as f32;
        let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
        let night = (a.sin() * 2.2 + 0.45) < 0.25;
        let me = self.body.pos.floor().as_ivec3();
        let glow = self.glow_layer();
        for _ in 0..n.min(64) {
            let r = &mut self.surv.rng;
            let off = IVec3::new((r.f32() * (2 * RANGE + 1) as f32) as i32 - RANGE, (r.f32() * 13.0) as i32 - 5, (r.f32() * (2 * RANGE + 1) as f32) as i32 - RANGE);
            let roll = r.f32();
            let jit = Vec3::new(r.f32(), r.f32(), r.f32());
            let p = me + off;
            let Some(id) = self.world.block(p) else { continue };
            let below = self.world.block_or_air(p - IVec3::Y);
            let above = self.world.block_or_air(p + IVec3::Y);
            let at = p.as_vec3() + jit;
            if id == AIR {
                match self.dim {
                    Dim::Nether if roll < 0.06 => {
                        // Ash, or pale blue specks over the soul sand valleys.
                        let soul = self.world.gen.biome_at(p.x, p.z) == Biome::SoulSandValley;
                        if soul {
                            self.particles.glow(at, Vec3::new(0.0, -0.15, 0.0), glow, [0.45, 0.85, 0.95], 0.035, 3.0);
                        } else {
                            let ash = self.reg.id_or_air("basalt");
                            let (layer, _) = self.block_look(ash);
                            self.particles.mote(at, layer, [0.5, 0.45, 0.45], 0.03, 0.35, 5.0);
                        }
                    }
                    Dim::End if roll < 0.04 => {
                        self.particles.glow(at, Vec3::new(0.0, 0.1, 0.0), glow, [0.55, 0.35, 0.8], 0.035, 3.5);
                    }
                    Dim::Overworld if roll < 0.02 && (self.world.light(p) >> 4) == 0 && p.y < 60 => {
                        // Cave dust, only where no sky reaches.
                        let stone = self.reg.id_or_air("stone");
                        let (layer, _) = self.block_look(stone);
                        self.particles.mote(at, layer, [0.75, 0.72, 0.68], 0.02, 0.06, 6.0);
                    }
                    _ => {}
                }
                continue;
            }
            let b = self.reg.get(id);
            if b.name.ends_with("_leaves") && below == AIR && roll < 0.12 {
                let (layer, tint) = self.block_look(id);
                self.particles.mote(p.as_vec3() + Vec3::new(jit.x, -0.05, jit.z), layer, tint, 0.05, 0.9, 6.0);
            } else if self.world.fluid(id) == 2 && above == AIR && roll < 0.25 {
                // Embers off lava.
                let v = Vec3::new(jit.x - 0.5, 1.6 + jit.y, jit.z - 0.5);
                self.particles.glow(p.as_vec3() + Vec3::new(jit.x, 1.0, jit.z), v, glow, [1.0, 0.55, 0.15], 0.04, 1.4);
            } else if night && self.dim == Dim::Overworld && (b.shape == Shape::Cross || b.name == "grass_block") && above == AIR && roll < 0.035 {
                let biome = self.world.gen.biome_at(p.x, p.z);
                let warm = matches!(biome, Biome::Plains | Biome::Forest | Biome::Swamp | Biome::DarkForest | Biome::Jungle | Biome::BirchForest | Biome::CherryGrove);
                if warm && !self.weather.state.raining {
                    // A firefly, blinking on for a moment and wandering off.
                    let v = Vec3::new(jit.x - 0.5, jit.y * 0.4, jit.z - 0.5) * 1.6;
                    self.particles.glow(p.as_vec3() + Vec3::new(jit.x, 1.2 + jit.y, jit.z), v, glow, [0.75, 1.0, 0.3], 0.045, 2.2);
                }
            } else if !night && self.dim == Dim::Overworld && b.shape == Shape::Cross && above == AIR && roll < 0.05 && is_flower(&b.name) && !self.weather.state.raining {
                // A butterfly (Naturalist idea) fluttering up off a flower.
                const WINGS: [[f32; 3]; 5] = [[1.0, 0.6, 0.15], [0.35, 0.55, 1.0], [1.0, 1.0, 0.9], [1.0, 0.85, 0.2], [0.85, 0.35, 0.9]];
                let tint = WINGS[((jit.x * 97.0) as usize) % WINGS.len()];
                let (layer, _) = self.block_look(self.reg.id_or_air("white_wool"));
                self.particles.mote(p.as_vec3() + Vec3::new(jit.x, 0.7, jit.z), layer, tint, 0.07, -0.25, 3.5);
            } else if self.reg.light_of(id) >= 13 && b.shape == Shape::Torch && roll < 0.08 {
                // A wisp of smoke off a torch flame.
                let (layer, _) = self.block_look(self.reg.id_or_air("stone"));
                self.particles.mote(p.as_vec3() + Vec3::new(0.5, 0.75, 0.5), layer, [0.35, 0.35, 0.35], 0.035, -0.6, 1.4);
            }
        }
    }
}

fn is_flower(name: &str) -> bool {
    !(name.contains("grass") || name.contains("fern") || name.contains("sapling") || name.contains("_stage") || name.contains("mushroom") || name.contains("bush") || name.contains("chorus"))
}
