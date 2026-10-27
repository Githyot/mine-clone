//! The deep world on the game side (the generator is voxel's worldgen/deep.rs): which layer and
//! underground biome the player is in, for the depth meter, the music and the monsters, and the
//! test hook that drops the player somewhere down there.

use glam::Vec3;
use voxel::chunk::{ChunkPos, MIN_Y};
use voxel::worldgen::deep::{Layer, UBiome, LAVA_SEA};
use voxel::worldgen::Dim;
use voxel::AIR;

use crate::Game;

impl Game {
    /// The sky seen from deep down: it fades to the dark of the rock below y -24 (gone by -96),
    /// and in the Underworld it is a red glow (the last value: drawn like the Nether).
    pub(crate) fn deep_sky(&self, y: f32, top: [f32; 3], horizon: [f32; 3], daylight: f32) -> ([f32; 3], [f32; 3], f32, bool) {
        if self.dim != Dim::Overworld || y > -24.0 {
            return (top, horizon, daylight, false);
        }
        if Layer::at(y.floor() as i32) == Layer::Underworld {
            return ([0.25, 0.06, 0.03], [0.3, 0.08, 0.04], 0.0, true);
        }
        let t = ((-24.0 - y) / 72.0).clamp(0.0, 1.0);
        let dark = [0.012, 0.012, 0.018];
        let mix = |c: [f32; 3]| [c[0] + (dark[0] - c[0]) * t, c[1] + (dark[1] - c[1]) * t, c[2] + (dark[2] - c[2]) * t];
        (mix(top), mix(horizon), daylight * (1.0 - t), false)
    }

    /// The layer of the world the player is in.
    pub(crate) fn layer(&self) -> Layer {
        if self.dim != Dim::Overworld {
            return Layer::Surface;
        }
        Layer::at(self.body.pos.y.floor() as i32)
    }

    /// The underground biome around the player (Plain above ground and in other dimensions).
    pub(crate) fn ubiome(&self) -> UBiome {
        let p = self.body.pos.floor().as_ivec3();
        self.world.gen.ubiome_at(p.x, p.y, p.z)
    }

    /// Test hook MC_DEEP: stand somewhere down there. `uw` (the Underworld's floor), `tunnel`
    /// (a long tunnel), a building (`cabana_subterranea`, `mina_abandonada`, `torre_em_ruinas`...),
    /// an underground biome (`jungle`, `ice`, `desert`, `mushroom`, `marble`, `granite`, `spider`,
    /// `gems`) or a height.
    pub(crate) fn test_deep(&mut self, what: &str) {
        let gen = self.world.gen.clone();
        let (x0, z0) = (self.body.pos.x as i32, self.body.pos.z as i32);
        let want = match what {
            "jungle" => Some(UBiome::Jungle),
            "ice" => Some(UBiome::Ice),
            "desert" => Some(UBiome::Desert),
            "mushroom" => Some(UBiome::Mushroom),
            "marble" => Some(UBiome::Marble),
            "granite" => Some(UBiome::Granite),
            "spider" => Some(UBiome::Spider),
            "gems" => Some(UBiome::Gems),
            "uw" => Some(UBiome::Underworld),
            _ => None,
        };
        let mut spot: Option<(Vec3, f32)> = None;
        if let Ok(y) = what.parse::<i32>() {
            spot = Some((Vec3::new(x0 as f32 + 0.5, y as f32, z0 as f32 + 0.5), 0.0));
        } else if let Some(c) = gen.find_deep(what, glam::IVec3::new(x0, -300, z0), 40) {
            spot = self.open_spot_near(c.x, c.z, c.y - 12, c.y + 12).map(|p| (p, 0.0));
        } else if what == "tunnel" {
            'search: for r in (0..3000).step_by(24) {
                for k in 0..(r / 12).max(1) {
                    let a = k as f32 / (r / 12).max(1) as f32 * std::f32::consts::TAU;
                    let (x, z) = (x0 + (a.cos() * r as f32) as i32, z0 + (a.sin() * r as f32) as i32);
                    if let Some(p) = self.open_spot_near(x, z, -230, -180) {
                        spot = Some((p, 0.0));
                        break 'search;
                    }
                }
            }
        } else if let Some(want) = want {
            let (lo, hi) = match want {
                UBiome::Underworld => (LAVA_SEA + 1, -640),
                UBiome::Gems => (-560, -300),
                _ => (-380, -40),
            };
            'search: for r in (0..4000).step_by(32) {
                for k in 0..(r / 16).max(1) {
                    let a = k as f32 / (r / 16).max(1) as f32 * std::f32::consts::TAU;
                    let (x, z) = (x0 + (a.cos() * r as f32) as i32, z0 + (a.sin() * r as f32) as i32);
                    let y = (lo + hi) / 2;
                    if gen.ubiome_at(x, y, z) != want {
                        continue;
                    }
                    if let Some(p) = self.open_spot_near(x, z, lo, hi) {
                        if gen.ubiome_at(p.x as i32, p.y as i32, p.z as i32) == want {
                            spot = Some((p, 0.0));
                            break 'search;
                        }
                    }
                }
            }
        }
        if let Some((p, _)) = spot {
            log::info!("deep test spot {what} at {p}");
            self.arrival = None;
            self.needs_spawn = false;
            self.body.pos = p;
            self.body.vel = Vec3::ZERO;
            self.fall_peak = p.y;
            self.world.set_focus(p.y);
        } else {
            log::warn!("no deep test spot for {what}");
        }
    }

    /// A floor with room to stand in a column range near (x, z), from a freshly made chunk.
    fn open_spot_near(&self, x: i32, z: i32, lo: i32, hi: i32) -> Option<Vec3> {
        let cp = ChunkPos::new(x.div_euclid(16), z.div_euclid(16));
        let c = self.world.gen.generate_full(cp);
        let solid = |b| b != AIR && self.reg.is_opaque(b);
        for (dx, dz) in [(8, 8), (4, 4), (12, 12), (4, 12), (12, 4), (8, 2), (2, 8), (14, 8), (8, 14)] {
            for y in (lo.max(MIN_Y + 6)..hi.min(-2)).rev() {
                let (a, b, f) = (c.get_w(dx, y, dz), c.get_w(dx, y + 1, dz), c.get_w(dx, y - 1, dz));
                if a == AIR && b == AIR && solid(f) {
                    return Some(Vec3::new((cp.x * 16 + dx as i32) as f32 + 0.5, y as f32, (cp.y * 16 + dz as i32) as f32 + 0.5));
                }
            }
        }
        None
    }
}
