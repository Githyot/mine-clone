//! The Aether (the Aether mod's idea): a bright sky dimension of floating islands of holystone
//! under blue-green grass, two layers of them, with skyroot and golden oak trees, drifting
//! aercloud banks and the void below. Our own take on the idea, from public descriptions; the
//! blocks and their art are ours (tools/moddata/aether.py, tools/modart/aether.py).

use super::{Biome, Generator};
use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos};
use crate::modtrees::{self, ModTree, Shape};
use crate::rng::Rng;

/// Heights of the two island layers.
const LAYERS: [i32; 2] = [72, 132];
/// Island noise above this makes land.
const EDGE: f32 = 0.1;
/// Travellers from the Overworld arrive at this height if no island is under them.
pub const AETHER_ARRIVAL_Y: i32 = 100;

#[derive(Clone)]
pub(super) struct AetherPalette {
    grass: BlockId,
    dirt: BlockId,
    holystone: BlockId,
    mossy: BlockId,
    quicksoil: BlockId,
    icestone: BlockId,
    ambrosium: BlockId,
    zanite: BlockId,
    gravitite: BlockId,
    cloud: BlockId,
    blue_cloud: BlockId,
    gold_cloud: BlockId,
    skyroot: ModTree,
    golden_oak: ModTree,
    tall_grass: BlockId,
    purple_flower: BlockId,
    white_flower: BlockId,
    berry_bush: BlockId,
}

impl AetherPalette {
    pub(super) fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        AetherPalette {
            grass: g("aether_grass_block"),
            dirt: g("aether_dirt"),
            holystone: g("holystone"),
            mossy: g("mossy_holystone"),
            quicksoil: g("quicksoil"),
            icestone: g("icestone"),
            ambrosium: g("ambrosium_ore"),
            zanite: g("zanite_ore"),
            gravitite: g("gravitite_ore"),
            cloud: g("cold_aercloud"),
            blue_cloud: g("blue_aercloud"),
            gold_cloud: g("golden_aercloud"),
            skyroot: ModTree { log: g("skyroot_log"), leaves: g("skyroot_leaves"), shape: Shape::Round, fruit: AIR },
            golden_oak: ModTree { log: g("golden_oak_log"), leaves: g("golden_oak_leaves"), shape: Shape::Spread, fruit: AIR },
            tall_grass: g("aether_tall_grass"),
            purple_flower: g("purple_flower"),
            white_flower: g("white_flower"),
            berry_bush: g("blueberry_bush"),
        }
    }
}

impl AetherPalette {
    pub(super) fn grass_id(&self) -> BlockId {
        self.grass
    }
}

impl Generator {
    /// The land of one layer in a column: top, bottom and how far inside the island it is (0..).
    fn aether_layer(&self, wx: i32, wz: i32, k: usize) -> Option<(i32, i32, f32)> {
        let (x, z) = (wx as f32, wz as f32);
        let off = k as f32 * 731.0;
        let m = self.continent.fbm2(x / 120.0 + off, z / 120.0 - off, 3) + 0.25 * self.detail.fbm2(x / 30.0 - off, z / 30.0, 2);
        if m <= EDGE {
            return None;
        }
        let e = m - EDGE;
        let d = self.detail.fbm2(x / 40.0 + 300.0 + off, z / 40.0, 2);
        let top = LAYERS[k] + (e * 30.0).min(9.0) as i32 + (d * 4.0) as i32;
        // Deep, ragged undersides that hang down under the middle of each island.
        let hang = self.cave_b.fbm2(x / 14.0 + off, z / 14.0, 2).abs();
        let depth = 2 + (e * 70.0).min(34.0) as i32 + (hang * e * 30.0) as i32;
        Some((top, top - depth, e))
    }

    /// Top and bottom of the highest island in a column.
    pub(crate) fn aether_land(&self, wx: i32, wz: i32) -> Option<(i32, i32)> {
        (0..2).rev().find_map(|k| self.aether_layer(wx, wz, k)).map(|(t, b, _)| (t, b))
    }

    pub(super) fn aether_biome(&self, wx: i32, wz: i32) -> Biome {
        let v = self.variant.fbm2(wx as f32 / 380.0 + 77.0, wz as f32 / 380.0, 2);
        if v > 0.22 {
            Biome::GoldenGrove
        } else {
            Biome::Aether
        }
    }

    pub(super) fn generate_aether(&self, pos: ChunkPos) -> Chunk {
        let p = &self.aether;
        let mut chunk = Chunk::new(pos);
        let (bx, bz) = (pos.x * 16, pos.y * 16);
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 0xAE7);
        // The tops of the islands in this chunk, for plants and trees.
        let mut tops: Vec<(usize, usize, i32)> = Vec::new();
        for z in 0..16usize {
            for x in 0..16usize {
                let (wx, wz) = (bx + x as i32, bz + z as i32);
                for k in 0..2 {
                    let Some((top, bottom, e)) = self.aether_layer(wx, wz, k) else { continue };
                    // Quicksoil on the thin rims of the islands, like a beach.
                    let rim = e < 0.02 && self.cave_a.noise2(wx as f32 / 9.0, wz as f32 / 9.0) > 0.15;
                    for y in bottom.max(1)..=top {
                        let id = if y == top {
                            if rim {
                                p.quicksoil
                            } else {
                                p.grass
                            }
                        } else if y >= top - 2 {
                            if rim {
                                p.quicksoil
                            } else {
                                p.dirt
                            }
                        } else {
                            let r = rng.f32();
                            let deep = (top - y) as f32 / (top - bottom).max(1) as f32;
                            if r < 0.012 {
                                p.ambrosium
                            } else if r < 0.018 {
                                p.zanite
                            } else if r < 0.0195 && deep > 0.6 {
                                p.gravitite
                            } else if r < 0.024 {
                                p.icestone
                            } else if r < 0.06 && y == bottom {
                                p.mossy
                            } else {
                                p.holystone
                            }
                        };
                        chunk.set(x, y as usize, z, id);
                    }
                    if !rim {
                        tops.push((x, z, top));
                    }
                }
                // Aercloud banks drifting between and under the layers.
                for (lo, hi) in [(36, 56), (98, 112), (170, 182)] {
                    for y in lo..hi {
                        if chunk.get(x, y as usize, z) != AIR {
                            continue;
                        }
                        let n = self.cave_a.noise3(wx as f32 / 28.0, y as f32 / 6.0, wz as f32 / 28.0);
                        if n > 0.48 {
                            let kind = self.cave_b.noise3(wx as f32 / 60.0, y as f32 / 20.0, wz as f32 / 60.0);
                            let id = if kind > 0.45 {
                                p.gold_cloud
                            } else if kind > 0.3 {
                                p.blue_cloud
                            } else {
                                p.cloud
                            };
                            chunk.set(x, y as usize, z, id);
                        }
                    }
                }
            }
        }
        // Trees, kept inside the chunk like the Overworld's.
        let golden = self.aether_biome(bx + 8, bz + 8) == Biome::GoldenGrove;
        let attempts = if golden { 5 } else { 4 };
        for _ in 0..attempts {
            if tops.is_empty() {
                break;
            }
            let (x, z, y) = tops[rng.below(tops.len() as u32) as usize];
            let t = if golden && rng.chance(0.6) { p.golden_oak } else { p.skyroot };
            let reach = t.shape.reach();
            if !(reach..16 - reach).contains(&(x as i32)) || !(reach..16 - reach).contains(&(z as i32)) {
                continue;
            }
            if chunk.get(x, y as usize, z) != p.grass || chunk.get(x, y as usize + 1, z) != AIR {
                continue;
            }
            chunk.set(x, y as usize, z, p.dirt);
            let mut space = super::ChunkSpace(&mut chunk);
            modtrees::grow(t, &mut space, &mut rng, x as i32, y, z as i32);
        }
        // Grass, flowers and blueberry bushes.
        for _ in 0..14 {
            if tops.is_empty() {
                break;
            }
            let (x, z, y) = tops[rng.below(tops.len() as u32) as usize];
            if chunk.get(x, y as usize, z) != p.grass || chunk.get(x, y as usize + 1, z) != AIR {
                continue;
            }
            let r = rng.f32();
            let id = if r < 0.6 {
                p.tall_grass
            } else if r < 0.75 {
                p.purple_flower
            } else if r < 0.9 {
                p.white_flower
            } else {
                p.berry_bush
            };
            chunk.set(x, y as usize + 1, z, id);
        }
        // Dungeons and ruins of the mod pack's buildings.
        self.place_structures(&mut chunk);
        chunk
    }
}
