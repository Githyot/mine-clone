//! Plants of each kind of water, so a lake, a river, a swamp, a warm sea and an underground lake
//! each look like themselves: cattails and flowering lilies on the lakes, pondweed waving in the
//! rivers, duckweed carpets on the swamps, sea pickles and anemones on the warm sea floor, and
//! glowing algae in the dark lakes of the caves and on the deep sea floor.
//!
//! Runs after everything else on its own random stream, so nothing older moves. The kind of
//! water follows the same rules as the creatures' (game/src/mobs/aquatic.rs).

use super::{Biome, Column, Generator, SEA_LEVEL};
use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos, CHUNK_HEIGHT};
use crate::rng::Rng;

#[derive(Clone)]
pub(super) struct WaterPalette {
    water: BlockId,
    lava: BlockId,
    cattail: BlockId,
    cattail_top: BlockId,
    duckweed: BlockId,
    pondweed: BlockId,
    water_lily: BlockId,
    lily_pad: BlockId,
    sea_pickle: BlockId,
    sea_anemone: BlockId,
    glow_algae: BlockId,
    seagrass: BlockId,
}

impl WaterPalette {
    pub(super) fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        WaterPalette {
            water: g("water"),
            lava: g("lava"),
            cattail: g("cattail"),
            cattail_top: g("cattail_top"),
            duckweed: g("duckweed"),
            pondweed: g("pondweed"),
            water_lily: g("water_lily"),
            lily_pad: g("lily_pad"),
            sea_pickle: g("sea_pickle"),
            sea_anemone: g("sea_anemone"),
            glow_algae: g("glow_algae"),
            seagrass: g("seagrass"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Lake,
    Swamp,
    River,
    JungleRiver,
    WarmSea,
    Sea,
    ColdSea,
}

fn kind_of(b: Biome, jungle_near: bool) -> Kind {
    match b {
        Biome::WarmOcean | Biome::LukewarmOcean => Kind::WarmSea,
        Biome::ColdOcean | Biome::FrozenOcean | Biome::SnowyBeach => Kind::ColdSea,
        Biome::Ocean | Biome::DeepOcean | Biome::Beach | Biome::StonyShore => Kind::Sea,
        Biome::River if jungle_near => Kind::JungleRiver,
        Biome::River | Biome::FrozenRiver => Kind::River,
        Biome::Swamp | Biome::MangroveSwamp | Biome::Pantanal => Kind::Swamp,
        _ => Kind::Lake,
    }
}

impl Generator {
    pub(super) fn decorate_waters(&self, chunk: &mut Chunk, cols: &[Column], pos: ChunkPos) {
        let p = &self.waters;
        if p.cattail == AIR || p.water == AIR {
            return;
        }
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 11);
        let (cx, cz) = (pos.x * 16 + 8, pos.y * 16 + 8);
        let jungle_near = [(24, 0), (-24, 0), (0, 24), (0, -24)]
            .iter()
            .any(|&(dx, dz)| matches!(self.biome_at(cx + dx, cz + dz).base(), Biome::Jungle | Biome::MataAtlantica | Biome::Pantanal));
        // Duckweed grows in drifting patches, not evenly.
        let patch = |x: i32, z: i32| self.variant.noise2((pos.x * 16 + x) as f32 / 7.0 + 300.0, (pos.y * 16 + z) as f32 / 7.0 - 90.0);
        let sea = SEA_LEVEL as usize;
        for z in 0..16usize {
            for x in 0..16usize {
                let col = &cols[x + z * 16];
                let y = col.height;
                if y >= SEA_LEVEL || y < 1 || chunk.get(x, y as usize + 1, z) != p.water {
                    self.cave_water(chunk, &mut rng, x, z, col.height);
                    continue;
                }
                let depth = SEA_LEVEL - y;
                let kind = kind_of(col.variant, jungle_near);
                let bed = y as usize + 1;
                let free = |c: &Chunk, uy: usize| c.get(x, uy, z) == p.water;
                let surface_free = chunk.get(x, sea, z) == p.water && chunk.get(x, sea + 1, z) == AIR;
                match kind {
                    Kind::Lake | Kind::Swamp | Kind::River | Kind::JungleRiver => {
                        // Cattails in the shallows, their heads out of the water.
                        let reeds = match kind {
                            Kind::Swamp => 0.2,
                            Kind::Lake => 0.12,
                            _ => 0.1,
                        };
                        if depth == 2 && surface_free && rng.chance(reeds) {
                            // The stems stop under the top water block, so the water's surface
                            // stays whole around them; the heads stand on it.
                            for uy in bed..sea {
                                chunk.set(x, uy, z, p.cattail);
                            }
                            chunk.set(x, sea + 1, z, p.cattail_top);
                            continue;
                        }
                        // On the surface: duckweed in the swamps, lilies on the still waters.
                        if surface_free {
                            let r = rng.f32();
                            let pad = match kind {
                                Kind::Swamp if patch(x as i32, z as i32) > 0.15 && r < 0.55 => Some(p.duckweed),
                                Kind::Swamp if r < 0.03 => Some(p.water_lily),
                                Kind::Lake if r < 0.02 => Some(p.water_lily),
                                Kind::Lake if r < 0.06 && depth <= 4 => Some(p.lily_pad),
                                Kind::JungleRiver if r < 0.05 => Some(p.lily_pad),
                                Kind::JungleRiver if r < 0.07 => Some(p.water_lily),
                                _ => None,
                            };
                            if let Some(b) = pad.filter(|&b| b != AIR) {
                                chunk.set(x, sea + 1, z, b);
                            }
                        }
                        // Under the water: pondweed waving, a little sea grass.
                        if depth >= 2 && free(chunk, bed) {
                            let r = rng.f32();
                            let weed = match kind {
                                Kind::River | Kind::JungleRiver => 0.16,
                                Kind::Lake => 0.14,
                                _ => 0.12,
                            };
                            if r < weed {
                                let h = 1 + rng.below(2.min(depth as u32 - 1).max(1)) as usize;
                                for k in 0..h {
                                    if free(chunk, bed + k) && bed + k < sea {
                                        chunk.set(x, bed + k, z, p.pondweed);
                                    }
                                }
                            } else if r < weed + 0.08 && p.seagrass != AIR {
                                chunk.set(x, bed, z, p.seagrass);
                            }
                        }
                    }
                    Kind::WarmSea | Kind::Sea | Kind::ColdSea => {
                        if !free(chunk, bed) {
                            continue;
                        }
                        let r = rng.f32();
                        match kind {
                            Kind::WarmSea if r < 0.025 => chunk.set(x, bed, z, p.sea_pickle),
                            Kind::WarmSea if r < 0.045 => chunk.set(x, bed, z, p.sea_anemone),
                            Kind::Sea if r < 0.012 && depth >= 4 => chunk.set(x, bed, z, p.sea_anemone),
                            // Far down, where no sunlight reaches, the floor glows here and there.
                            _ if depth >= 14 && r < 0.02 => chunk.set(x, bed, z, p.glow_algae),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// Glowing algae on the floor of the lakes in the caves (and the upper deep layers).
    fn cave_water(&self, chunk: &mut Chunk, rng: &mut Rng, x: usize, z: usize, top: i32) {
        let p = &self.waters;
        let hi = (top - 6).min(CHUNK_HEIGHT as i32 - 2).min(SEA_LEVEL - 4);
        let mut y = CAVE_LAKES_FROM;
        while y < hi {
            if chunk.get_w(x, y, z) == p.water {
                let below = chunk.get_w(x, y - 1, z);
                if below != AIR && below != p.water && below != p.lava && rng.chance(0.09) {
                    chunk.set_w(x, y, z, if rng.chance(0.75) { p.glow_algae } else { p.pondweed });
                }
                // Skip the rest of this pool.
                while y < hi && chunk.get_w(x, y, z) == p.water {
                    y += 1;
                }
            }
            y += 1;
        }
    }
}

/// How deep the cave lakes get their algae (the deep world's upper layers too).
pub const CAVE_LAKES_FROM: i32 = -160;
