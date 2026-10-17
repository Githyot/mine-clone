//! Cave biomes (YUNG's Cave Biomes idea): below the surface, large regions of the cave system
//! take on a look of their own. Lush caves grow moss on the floors and glowing vines from the
//! ceilings; frozen caves are lined with packed ice and snow; dripstone caves bristle with
//! stone spikes from the floor and the roof.

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
                    match kind {
                        CaveBiome::Lush => {
                            chunk.set(x, y, z, p.moss);
                            if floor && rng.chance(0.25) && p.grass != AIR {
                                chunk.set(x, y + 1, z, p.grass);
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
                            chunk.set(x, y, z, p.packed_ice);
                            if floor && rng.chance(0.6) && p.snow_layer != AIR {
                                chunk.set(x, y + 1, z, p.snow_layer);
                            }
                        }
                        CaveBiome::Dripstone => {
                            if rng.chance(0.7) {
                                chunk.set(x, y, z, p.dripstone);
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
