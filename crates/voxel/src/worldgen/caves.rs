//! Cave biomes (YUNG's Cave Biomes idea): below the surface, large regions of the cave system
//! take on a look of their own. Lush caves grow moss on the floors and glowing vines from the
//! ceilings; frozen caves are lined with packed ice and snow; dripstone caves bristle with
//! stone spikes from the floor and the roof. Frozen caves also get frozen pools, blue ice and
//! icicles; lush caves get clay-bottomed pools and moss carpet; dripstone caves get columns
//! joining floor and roof, and shallow pools.

use super::{Column, Generator};
use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos, CHUNK_HEIGHT};
use crate::rng::Rng;

#[derive(Clone)]
pub(super) struct CavePalette {
    stone: BlockId,
    moss: BlockId,
    vines: BlockId,
    packed_ice: BlockId,
    snow_layer: BlockId,
    dripstone: BlockId,
    spike_up: BlockId,
    spike_down: BlockId,
    grass: BlockId,
    ice: BlockId,
    blue_ice: BlockId,
    icicle: BlockId,
    water: BlockId,
    clay: BlockId,
    carpet: BlockId,
}

impl CavePalette {
    pub(super) fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        CavePalette {
            stone: g("stone"),
            moss: g("moss_block"),
            vines: g("cave_vines"),
            packed_ice: g("packed_ice"),
            snow_layer: g("snow_layer"),
            dripstone: g("dripstone_block"),
            spike_up: g("pointed_dripstone"),
            spike_down: g("pointed_dripstone_down"),
            grass: g("short_grass"),
            ice: g("ice"),
            blue_ice: g("blue_ice"),
            icicle: g("icicle"),
            water: g("water"),
            clay: g("clay"),
            carpet: g("moss_carpet"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaveBiome {
    Plain,
    Lush,
    Frozen,
    Dripstone,
}

/// The highest a cave biome reaches: it stays well under the surface.
const TOP: usize = 56;

impl Generator {
    /// Which cave biome is under this column.
    pub fn cave_biome(&self, wx: i32, wz: i32) -> CaveBiome {
        let n = self.detail.fbm2(wx as f32 / 190.0 + 311.0, wz as f32 / 190.0 - 77.0, 3);
        let m = self.detail.fbm2(wx as f32 / 230.0 - 905.0, wz as f32 / 230.0 + 41.0, 2);
        if n > 0.28 {
            CaveBiome::Lush
        } else if n < -0.3 {
            CaveBiome::Frozen
        } else if m > 0.3 {
            CaveBiome::Dripstone
        } else {
            CaveBiome::Plain
        }
    }

    pub(super) fn decorate_caves(&self, chunk: &mut Chunk, rng: &mut Rng, cols: &[Column], pos: ChunkPos) {
        let p = &self.caves;
        if p.moss == AIR {
            return;
        }
        let (bx, bz) = (pos.x * 16, pos.y * 16);
        for z in 0..16usize {
            for x in 0..16usize {
                let kind = self.cave_biome(bx + x as i32, bz + z as i32);
                if kind == CaveBiome::Plain {
                    continue;
                }
                let surface = cols[z * 16 + x].height.max(0) as usize;
                let top = TOP.min(surface.saturating_sub(8)).min(CHUNK_HEIGHT - 2);
                for y in 12..top {
                    let here = chunk.get(x, y, z);
                    if here != p.stone {
                        continue;
                    }
                    let up = chunk.get(x, y + 1, z);
                    let down = chunk.get(x, y - 1, z);
                    let floor = up == AIR;
                    let ceiling = down == AIR;
                    if !floor && !ceiling {
                        continue;
                    }
                    // Pools: a floor cell walled in on all four sides (so the water stays put),
                    // in patches picked by noise.
                    let (wx, wz) = (bx + x as i32, bz + z as i32);
                    let walled = (1..15).contains(&x)
                        && (1..15).contains(&z)
                        && [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)].iter().all(|&(a, b)| chunk.get(a, y, b) != AIR);
                    let patch = self.detail.fbm2(wx as f32 / 7.0 + 51.0, wz as f32 / 7.0 - 13.0, 2);
                    let pool = floor && !ceiling && walled && patch > 0.12 && p.water != AIR;
                    match kind {
                        CaveBiome::Lush => {
                            if pool {
                                chunk.set(x, y, z, p.water);
                                if p.clay != AIR && chunk.get(x, y - 1, z) == p.stone {
                                    chunk.set(x, y - 1, z, p.clay);
                                }
                                continue;
                            }
                            chunk.set(x, y, z, p.moss);
                            if floor && rng.chance(0.22) && p.grass != AIR {
                                chunk.set(x, y + 1, z, p.grass);
                            } else if floor && rng.chance(0.2) && p.carpet != AIR {
                                chunk.set(x, y + 1, z, p.carpet);
                            }
                            if ceiling && rng.chance(0.12) && p.vines != AIR {
                                let len = 1 + rng.below(4) as usize;
                                for k in 1..=len {
                                    if y < k + 1 || chunk.get(x, y - k, z) != AIR || chunk.get(x, y - k - 1, z) != AIR {
                                        break;
                                    }
                                    chunk.set(x, y - k, z, p.vines);
                                }
                            }
                        }
                        CaveBiome::Frozen => {
                            // Frozen pools: smooth ice sheets on the floor, without snow.
                            if floor && patch > 0.05 && p.ice != AIR {
                                chunk.set(x, y, z, p.ice);
                                continue;
                            }
                            let blue = p.blue_ice != AIR && rng.chance(0.07);
                            chunk.set(x, y, z, if blue { p.blue_ice } else { p.packed_ice });
                            if floor && rng.chance(0.6) && p.snow_layer != AIR {
                                chunk.set(x, y + 1, z, p.snow_layer);
                            }
                            if ceiling && p.icicle != AIR && y > 2 && chunk.get(x, y - 2, z) == AIR && rng.chance(0.14) {
                                chunk.set(x, y - 1, z, p.icicle);
                            }
                        }
                        CaveBiome::Dripstone => {
                            if pool && patch > 0.25 {
                                chunk.set(x, y, z, p.water);
                                chunk.set(x, y - 1, z, p.dripstone);
                                continue;
                            }
                            if rng.chance(0.7) {
                                chunk.set(x, y, z, p.dripstone);
                            }
                            // Columns: where the roof is low, floor and roof grow together.
                            let gap = if floor { (1..8).find(|&k| y + k < CHUNK_HEIGHT && chunk.get(x, y + k, z) != AIR) } else { None };
                            if let Some(g) = gap.filter(|&g| g >= 3 && rng.chance(0.06)) {
                                for k in 1..g {
                                    chunk.set(x, y + k, z, p.dripstone);
                                }
                                continue;
                            }
                            if floor && rng.chance(0.1) {
                                let h = 1 + rng.below(3) as usize;
                                for k in 1..=h {
                                    if chunk.get(x, y + k, z) != AIR || chunk.get(x, y + k + 1, z) != AIR {
                                        break;
                                    }
                                    chunk.set(x, y + k, z, p.spike_up);
                                }
                            } else if ceiling && rng.chance(0.12) {
                                let h = 1 + rng.below(3) as usize;
                                for k in 1..=h {
                                    if y < k + 1 || chunk.get(x, y - k, z) != AIR || chunk.get(x, y - k - 1, z) != AIR {
                                        break;
                                    }
                                    chunk.set(x, y - k, z, p.spike_down);
                                }
                            }
                        }
                        CaveBiome::Plain => {}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::ChunkPos;

    fn registry() -> BlockRegistry {
        BlockRegistry::from_ron_many(&[include_str!("../../../../packs/vanilla/blocks.ron"), include_str!("../../../../packs/vanilla/mods/blocks.ron")]).unwrap()
    }

    #[test]
    fn cave_biomes_get_their_features() {
        let reg = registry();
        let g = Generator::new(1234, &reg);
        let want = ["blue_ice", "icicle", "moss_carpet", "clay", "water"].map(|n| reg.id(n).unwrap());
        let mut seen = [0u32; 5];
        let mut kinds = [0u32; 4];
        for cz in -40..40 {
            for cx in -40..40 {
                let k = g.cave_biome(cx * 16 + 8, cz * 16 + 8);
                if k == CaveBiome::Plain || kinds[k as usize] >= 12 {
                    continue;
                }
                kinds[k as usize] += 1;
                let c = g.generate(ChunkPos::new(cx, cz));
                for y in 12..TOP {
                    for z in 0..16 {
                        for x in 0..16 {
                            let id = c.get(x, y, z);
                            if let Some(i) = want.iter().position(|&w| w == id) {
                                seen[i] += 1;
                            }
                        }
                    }
                }
            }
        }
        println!("blue_ice, icicle, carpet, clay, water: {seen:?}");
        assert!(seen.iter().all(|&n| n > 0), "{seen:?}");
    }
}
