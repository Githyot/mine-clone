//! The vanilla biome variants (Lote 4 of plan/o-que-falta.md): warm, lukewarm, cold and deep
//! oceans, frozen rivers, snowy beaches and stony shores, sunflower plains, flower forests, old
//! growth and snowy taigas, groves, snowy slopes, windswept hills and savannas, jagged peaks,
//! ice spikes, bamboo and sparse jungles, wooded and eroded badlands, and the pale garden.
//!
//! Each variant is chosen inside its parent biome (`Generator::variant_of`) and otherwise
//! behaves as that parent; here it gets its own ground and its own plants. The plants go on a
//! random stream of their own so nothing older moves.

use super::{Biome, Column, Generator, SEA_LEVEL};
use crate::block::{BlockId, AIR};
use crate::chunk::{Chunk, ChunkPos, CHUNK_HEIGHT};
use crate::rng::Rng;

impl Generator {
    /// Top, filler and filler depth of a column whose variant has a ground of its own.
    pub(super) fn variant_top_blocks(&self, col: &Column, wx: i32, wz: i32) -> Option<(BlockId, BlockId, i32)> {
        let p = &self.pal;
        let h = col.height;
        let spot = |k: i32, m: i32| (wx * 31 + wz * 17 + k).rem_euclid(m);
        let patch = self.variant.noise2(wx as f32 / 9.0 + 40.0, wz as f32 / 9.0);
        Some(match col.variant {
            Biome::WarmOcean => (p.sand, p.sand, 4),
            Biome::ColdOcean | Biome::DeepOcean if patch > 0.1 => (p.gravel, p.gravel, 2),
            Biome::StonyShore => {
                if patch > 0.25 {
                    (p.gravel, p.stone, 2)
                } else {
                    (p.stone, p.stone, 1)
                }
            }
            Biome::SnowyBeach => (p.sand, p.sand, 3),
            Biome::WindsweptHills if h > 100 && patch > 0.0 => {
                if patch > 0.3 {
                    (p.gravel, p.stone, 2)
                } else {
                    (p.stone, p.stone, 1)
                }
            }
            Biome::WindsweptHills => (p.grass, p.dirt, 3),
            Biome::JaggedPeaks => {
                if patch > 0.2 && h > 165 {
                    (p.snow_block, p.stone, 2)
                } else if spot(3, 5) == 0 {
                    (p.snow_block, p.stone, 1)
                } else {
                    (p.stone, p.stone, 1)
                }
            }
            Biome::SnowySlopes if p.snow_block != AIR => (p.snow_block, p.stone, 3),
            Biome::Grove => (p.snow_grass, p.dirt, 3),
            Biome::IceSpikes if p.snow_block != AIR => (p.snow_block, p.dirt, 3),
            Biome::OldGrowthTaiga if p.podzol != AIR => {
                if patch > 0.35 && p.coarse_dirt != AIR {
                    (p.coarse_dirt, p.dirt, 3)
                } else if patch > -0.25 {
                    (p.podzol, p.dirt, 3)
                } else {
                    (p.grass, p.dirt, 3)
                }
            }
            // The badlands' plateaus wear grass and coarse dirt where the oaks grow.
            Biome::WoodedBadlands if h > SEA_LEVEL + 22 => {
                if patch > 0.15 && p.coarse_dirt != AIR {
                    (p.coarse_dirt, p.terracotta, 3)
                } else {
                    (p.grass, p.terracotta, 2)
                }
            }
            Biome::WindsweptSavanna if patch > 0.2 && p.coarse_dirt != AIR => (p.coarse_dirt, p.dirt, 3),
            Biome::WindsweptSavanna if patch < -0.45 => (p.stone, p.stone, 1),
            Biome::PaleGarden if patch > 0.3 && p.pale_moss != AIR => (p.pale_moss, p.dirt, 3),
            _ => return None,
        })
    }

    /// The plants and shapes of the variants, after the classic decoration.
    pub(super) fn variant_scatter(&self, chunk: &mut Chunk, cols: &[Column], pos: ChunkPos) {
        let p = &self.pal;
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 4);
        let variant = cols[8 + 8 * 16].variant;
        let top = |x: i32, z: i32| cols[(x + z * 16) as usize].height;
        let free = |chunk: &Chunk, x: usize, y: usize, z: usize| y < CHUNK_HEIGHT && (chunk.get(x, y, z) == AIR || chunk.get(x, y, z) == p.short_grass);
        match variant {
            Biome::SunflowerPlains if p.sunflower != AIR => {
                // Sunflowers in drifts.
                let (cx, cz) = (rng.range(2, 13), rng.range(2, 13));
                for _ in 0..rng.range(10, 22) {
                    let (x, z) = ((cx + rng.range(-5, 5)).clamp(0, 15), (cz + rng.range(-5, 5)).clamp(0, 15));
                    let y = top(x, z);
                    let (ux, uz) = (x as usize, z as usize);
                    if chunk.get(ux, y as usize, uz) == p.grass && free(chunk, ux, y as usize + 1, uz) {
                        chunk.set(ux, y as usize + 1, uz, p.sunflower);
                    }
                }
            }
            Biome::FlowerForest if !p.meadow_flowers.is_empty() => {
                // A carpet of flowers in bands of one kind.
                for z in 0..16i32 {
                    for x in 0..16i32 {
                        let y = top(x, z);
                        let (ux, uy, uz) = (x as usize, y as usize, z as usize);
                        if chunk.get(ux, uy, uz) != p.grass || !free(chunk, ux, uy + 1, uz) || !rng.chance(0.35) {
                            continue;
                        }
                        let (wx, wz) = ((pos.x * 16 + x) as f32, (pos.y * 16 + z) as f32);
                        let k = (self.variant.noise2(wx / 18.0 - 300.0, wz / 18.0) * 0.5 + 0.5) * p.meadow_flowers.len() as f32;
                        let f = if rng.chance(0.85) { p.meadow_flowers[(k as usize).min(p.meadow_flowers.len() - 1)] } else { p.poppy };
                        chunk.set(ux, uy + 1, uz, f);
                    }
                }
            }
            Biome::IceSpikes if p.packed_ice != AIR && rng.chance(0.45) => {
                // A spike of packed ice: a fat base tapering to a point, sometimes a tall one.
                let (cx, cz) = (rng.range(3, 12), rng.range(3, 12));
                let y0 = top(cx, cz);
                let tall = rng.chance(0.2);
                let h = if tall { rng.range(18, 34) } else { rng.range(5, 12) };
                let r0 = if tall { 2.6 } else { 1.6 };
                for k in -2..h {
                    let r = r0 * (1.0 - k.max(0) as f32 / h as f32) + 0.3;
                    let ri = r.ceil() as i32;
                    for dz in -ri..=ri {
                        for dx in -ri..=ri {
                            let (x, z) = (cx + dx, cz + dz);
                            let y = y0 + k;
                            if !(0..16).contains(&x) || !(0..16).contains(&z) || y < 1 || y as usize >= CHUNK_HEIGHT {
                                continue;
                            }
                            if ((dx * dx + dz * dz) as f32).sqrt() <= r {
                                let b = if p.blue_ice != AIR && k > 2 && rng.chance(0.06) { p.blue_ice } else { p.packed_ice };
                                chunk.set(x as usize, y as usize, z as usize, b);
                            }
                        }
                    }
                }
            }
            Biome::ErodedBadlands if p.terracotta != AIR => {
                // Hoodoos: thin spires of the banded terracotta standing up out of the red sand.
                for _ in 0..rng.range(0, 3) {
                    let (cx, cz) = (rng.range(2, 13), rng.range(2, 13));
                    let y0 = top(cx, cz);
                    if y0 <= SEA_LEVEL {
                        continue;
                    }
                    let h = rng.range(6, 20);
                    for k in 1..=h {
                        let r: i32 = if k < h / 3 { 1 } else { 0 };
                        for dz in -r..=r {
                            for dx in -r..=r {
                                let (x, z, y) = ((cx + dx) as usize, (cz + dz) as usize, (y0 + k) as usize);
                                if y < CHUNK_HEIGHT {
                                    chunk.set(x, y, z, p.terracotta);
                                }
                            }
                        }
                    }
                }
            }
            Biome::BambooJungle if p.bamboo != AIR => {
                // Bamboo everywhere, on podzol.
                for _ in 0..rng.range(30, 60) {
                    let (x, z) = (rng.range(0, 15), rng.range(0, 15));
                    let y = top(x, z);
                    let (ux, uz) = (x as usize, z as usize);
                    if !(SEA_LEVEL..=200).contains(&y) || chunk.get(ux, y as usize, uz) != p.grass || !free(chunk, ux, y as usize + 1, uz) {
                        continue;
                    }
                    if p.podzol != AIR {
                        chunk.set(ux, y as usize, uz, p.podzol);
                    }
                    for k in 1..=rng.range(5, 14) {
                        let uy = (y + k) as usize;
                        if !free(chunk, ux, uy, uz) {
                            break;
                        }
                        chunk.set(ux, uy, uz, p.bamboo);
                    }
                }
            }
            Biome::PaleGarden => {
                for z in 0..16i32 {
                    for x in 0..16i32 {
                        let y = top(x, z);
                        let (ux, uy, uz) = (x as usize, y as usize, z as usize);
                        // Pale moss carpets on the ground.
                        if p.pale_moss_carpet != AIR && chunk.get(ux, uy, uz) == p.grass && free(chunk, ux, uy + 1, uz) && rng.chance(0.18) {
                            chunk.set(ux, uy + 1, uz, p.pale_moss_carpet);
                        }
                        // Moss hanging under the pale crowns, one to three blocks.
                        if p.pale_hanging_moss == AIR {
                            continue;
                        }
                        for sy in (uy + 3..(uy + 16).min(CHUNK_HEIGHT - 1)).rev() {
                            if self.trees.is_leaves(chunk.get(ux, sy, uz)) && chunk.get(ux, sy - 1, uz) == AIR {
                                if rng.chance(0.35) {
                                    for k in 1..=rng.range(1, 3) as usize {
                                        if sy - k <= uy + 1 || chunk.get(ux, sy - k, uz) != AIR {
                                            break;
                                        }
                                        chunk.set(ux, sy - k, uz, p.pale_hanging_moss);
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
            Biome::WarmOcean | Biome::LukewarmOcean | Biome::Ocean | Biome::ColdOcean | Biome::DeepOcean => {
                // Sea grass on the sea floor; coral reefs in the warm seas.
                let warm = variant == Biome::WarmOcean && p.corals[0] != AIR;
                let reef = warm && rng.chance(0.7);
                let (cx, cz) = (rng.range(2, 13) as f32, rng.range(2, 13) as f32);
                for z in 0..16i32 {
                    for x in 0..16i32 {
                        let y = top(x, z);
                        let (ux, uy, uz) = (x as usize, y as usize, z as usize);
                        if y > SEA_LEVEL - 3 || chunk.get(ux, uy + 1, uz) != p.water {
                            continue;
                        }
                        let d = ((x as f32 - cx).powi(2) + (z as f32 - cz).powi(2)).sqrt();
                        if reef && d < 6.0 + rng.f32() * 2.0 {
                            let c = rng.below(5) as usize;
                            let h = if d < 3.0 { rng.range(1, 3) } else { 1 };
                            for k in 1..=h {
                                if chunk.get(ux, uy + k as usize, uz) != p.water {
                                    break;
                                }
                                chunk.set(ux, uy + k as usize, uz, p.coral_blocks[c]);
                            }
                            let ty = uy + h as usize + 1;
                            if ty < SEA_LEVEL as usize && chunk.get(ux, ty, uz) == p.water && rng.chance(0.6) {
                                chunk.set(ux, ty, uz, p.corals[rng.below(5) as usize]);
                            }
                        } else if p.seagrass != AIR && chunk.get(ux, uy + 1, uz) == p.water && rng.chance(if warm { 0.25 } else { 0.12 }) {
                            chunk.set(ux, uy + 1, uz, p.seagrass);
                        } else if warm && rng.chance(0.03) {
                            chunk.set(ux, uy + 1, uz, p.corals[rng.below(5) as usize]);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
