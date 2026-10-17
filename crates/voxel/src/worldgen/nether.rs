//! The Nether: a closed world of hot rock between a bedrock floor and roof, with huge caverns,
//! a lava sea, glowstone hanging from the ceiling and three regions (wastes, soul sand valleys
//! and crimson forests). Our own take on the idea, from the public descriptions of the game.

use super::{Biome, Generator};
use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos};
use crate::rng::Rng;

/// Top of the Nether: bedrock roof, nothing above.
pub const NETHER_ROOF: i32 = 127;
/// Open space at or below this height fills with lava.
pub const NETHER_LAVA: i32 = 31;
const H: usize = NETHER_ROOF as usize + 1;

#[derive(Clone)]
pub(super) struct NetherPalette {
    netherrack: BlockId,
    bedrock: BlockId,
    lava: BlockId,
    quartz_ore: BlockId,
    gold_ore: BlockId,
    soul_sand: BlockId,
    soul_soil: BlockId,
    glowstone: BlockId,
    magma: BlockId,
    gravel: BlockId,
    nylium: BlockId,
    stem: BlockId,
    wart_block: BlockId,
    shroomlight: BlockId,
    basalt: BlockId,
    blackstone: BlockId,
}

impl NetherPalette {
    pub(super) fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        NetherPalette {
            netherrack: g("netherrack"),
            bedrock: g("bedrock"),
            lava: g("lava"),
            quartz_ore: g("nether_quartz_ore"),
            gold_ore: g("nether_gold_ore"),
            soul_sand: g("soul_sand"),
            soul_soil: g("soul_soil"),
            glowstone: g("glowstone"),
            magma: g("magma_block"),
            gravel: g("gravel"),
            nylium: g("crimson_nylium"),
            stem: g("crimson_stem"),
            wart_block: g("nether_wart_block"),
            shroomlight: g("shroomlight"),
            basalt: g("basalt"),
            blackstone: g("blackstone"),
        }
    }
}

impl Generator {
    pub(super) fn nether_biome(&self, wx: i32, wz: i32) -> Biome {
        let (x, z) = (wx as f32, wz as f32);
        let soul = self.variant.fbm2(x / 220.0 + 300.0, z / 220.0, 3);
        let warm = self.temperature.fbm2(x / 260.0 - 500.0, z / 260.0, 2);
        if soul > 0.16 {
            Biome::SoulSandValley
        } else if warm > 0.14 {
            Biome::CrimsonForest
        } else {
            Biome::NetherWastes
        }
    }

    /// Rock or open space: wide caverns in the middle heights, closing in toward the floor
    /// and the roof, with pillars and shelves from a second, finer noise.
    pub(crate) fn nether_solid(&self, wx: i32, y: i32, wz: i32) -> bool {
        if y <= 0 || y >= NETHER_ROOF - 2 {
            return true;
        }
        let (x, yf, z) = (wx as f32, y as f32, wz as f32);
        let big = self.cheese.noise3(x / 90.0, yf / 44.0, z / 90.0);
        let fine = self.cave_a.noise3(x / 28.0, yf / 20.0, z / 28.0);
        let pillars = self.cave_b.noise3(x / 18.0, yf / 90.0, z / 18.0);
        let mid = ((yf - 66.0) / 54.0).powi(2);
        let d = big * 1.7 + fine * 0.6 + mid * 1.05 - 0.36 + (pillars - 0.42).max(0.0) * 4.0;
        d > 0.0
    }

    pub(super) fn generate_nether(&self, pos: ChunkPos) -> Chunk {
        let p = &self.nether;
        let mut chunk = Chunk::new(pos);
        let (bx, bz) = (pos.x * 16, pos.y * 16);
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 0x4E45);
        let mut solid = vec![false; 16 * 16 * H];
        let at = |x: usize, y: usize, z: usize| (y * 16 + z) * 16 + x;
        for z in 0..16 {
            for x in 0..16 {
                for y in 0..H {
                    solid[at(x, y, z)] = self.nether_solid(bx + x as i32, y as i32, bz + z as i32);
                }
            }
        }
        for z in 0..16usize {
            for x in 0..16usize {
                let (wx, wz) = (bx + x as i32, bz + z as i32);
                let biome = self.nether_biome(wx, wz);
                for y in 0..H {
                    let yi = y as i32;
                    let id = if solid[at(x, y, z)] {
                        let bedrock = yi == 0 || yi == NETHER_ROOF || (yi < 4 && rng.chance(0.5)) || (yi > NETHER_ROOF - 4 && rng.chance(0.5));
                        let open_above = y + 1 < H && !solid[at(x, y + 1, z)];
                        let near_floor = (1..=3).any(|k| y + k < H && !solid[at(x, y + k, z)]);
                        if bedrock {
                            p.bedrock
                        } else {
                            match biome {
                                Biome::SoulSandValley if near_floor => {
                                    if open_above && rng.chance(0.6) {
                                        p.soul_sand
                                    } else {
                                        p.soul_soil
                                    }
                                }
                                Biome::CrimsonForest if open_above && yi > NETHER_LAVA => p.nylium,
                                Biome::NetherWastes if open_above && (NETHER_LAVA - 2..=NETHER_LAVA + 3).contains(&yi) && rng.chance(0.3) => p.gravel,
                                _ => p.netherrack,
                            }
                        }
                    } else if yi <= NETHER_LAVA {
                        p.lava
                    } else {
                        AIR
                    };
                    if id != AIR {
                        chunk.set(x, y, z, id);
                    }
                }
            }
        }
        self.decorate_nether(&mut chunk, &mut rng);
        self.place_structures(&mut chunk);
        chunk
    }

    fn decorate_nether(&self, chunk: &mut Chunk, rng: &mut Rng) {
        let p = &self.nether;
        let (bx, bz) = (chunk.pos.x * 16, chunk.pos.y * 16);
        // Ore and magma blobs inside the rock.
        let blobs = [(p.quartz_ore, 10, 117, 14, 6), (p.gold_ore, 10, 117, 8, 5), (p.magma, NETHER_LAVA - 4, NETHER_LAVA + 6, 6, 7), (p.blackstone, 5, 40, 3, 14)];
        for (block, lo, hi, tries, size) in blobs {
            if block == AIR {
                continue;
            }
            for _ in 0..tries {
                let (mut x, mut y, mut z) = (rng.range(0, 15), rng.range(lo, hi), rng.range(0, 15));
                for _ in 0..size {
                    if (0..16).contains(&x) && (1..NETHER_ROOF - 3).contains(&y) && (0..16).contains(&z) && chunk.get(x as usize, y as usize, z as usize) == p.netherrack {
                        chunk.set(x as usize, y as usize, z as usize, block);
                    }
                    x += rng.range(-1, 1);
                    y += rng.range(-1, 1);
                    z += rng.range(-1, 1);
                }
            }
        }
        // Glowstone hanging from the ceilings: each piece hangs from rock or from another piece.
        for _ in 0..4 {
            let (x, z) = (rng.range(1, 15), rng.range(1, 15));
            let Some(y) = (NETHER_LAVA + 8..NETHER_ROOF - 3).rev().find(|&y| chunk.get(x as usize, y as usize, z as usize) != AIR && chunk.get(x as usize, (y - 1) as usize, z as usize) == AIR) else {
                continue;
            };
            if !rng.chance(0.45) {
                continue;
            }
            chunk.set(x as usize, (y - 1) as usize, z as usize, p.glowstone);
            let mut placed = vec![(x, y - 1, z)];
            for _ in 0..rng.range(14, 40) {
                let (px, py, pz) = placed[rng.range(0, placed.len() as i32 - 1) as usize];
                let (cx, cy, cz) = (px + rng.range(-1, 1), py - rng.range(0, 1), pz + rng.range(-1, 1));
                if !(0..16).contains(&cx) || !(0..16).contains(&cz) || cy <= NETHER_LAVA {
                    continue;
                }
                let (ux, uy, uz) = (cx as usize, cy as usize, cz as usize);
                if chunk.get(ux, uy, uz) == AIR && chunk.get(ux, uy + 1, uz) != AIR {
                    chunk.set(ux, uy, uz, p.glowstone);
                    placed.push((cx, cy, cz));
                }
            }
        }
        // Crimson fungi on nylium, and basalt columns in the soul sand valleys.
        for _ in 0..10 {
            let (x, z) = (rng.range(2, 14), rng.range(2, 14));
            let biome = self.nether_biome(bx + x, bz + z);
            let floor = (NETHER_LAVA + 1..NETHER_ROOF - 12).find(|&y| {
                let b = chunk.get(x as usize, y as usize, z as usize);
                b != AIR && b != p.lava && chunk.get(x as usize, (y + 1) as usize, z as usize) == AIR && chunk.get(x as usize, (y + 2) as usize, z as usize) == AIR
            });
            let Some(fy) = floor else { continue };
            let start = rng.range(fy, NETHER_ROOF - 12);
            // Pick a floor at a random height so fungi grow on upper ledges too.
            let fy = (start..NETHER_ROOF - 12)
                .rev()
                .find(|&y| {
                    let b = chunk.get(x as usize, y as usize, z as usize);
                    b != AIR && b != p.lava && chunk.get(x as usize, (y + 1) as usize, z as usize) == AIR
                })
                .unwrap_or(fy);
            let ground = chunk.get(x as usize, fy as usize, z as usize);
            match biome {
                Biome::CrimsonForest if ground == p.nylium => {
                    let h = rng.range(4, 8);
                    if (1..=h + 2).any(|k| chunk.get(x as usize, (fy + k) as usize, z as usize) != AIR) {
                        continue;
                    }
                    for k in 1..=h {
                        chunk.set(x as usize, (fy + k) as usize, z as usize, p.stem);
                    }
                    let top = fy + h;
                    for dy in -1..=1 {
                        let r: i32 = if dy == 1 { 1 } else { 2 };
                        for dz in -r..=r {
                            for dx in -r..=r {
                                if dx.abs() == r && dz.abs() == r && r > 1 {
                                    continue;
                                }
                                let (cx, cy, cz) = (x + dx, top + dy, z + dz);
                                if chunk.get(cx as usize, cy as usize, cz as usize) == AIR {
                                    let b = if rng.chance(0.1) { p.shroomlight } else { p.wart_block };
                                    chunk.set(cx as usize, cy as usize, cz as usize, b);
                                }
                            }
                        }
                    }
                }
                Biome::SoulSandValley if rng.chance(0.5) => {
                    let h = rng.range(3, 12);
                    for k in 1..=h {
                        let y = fy + k;
                        if y >= NETHER_ROOF - 3 || chunk.get(x as usize, y as usize, z as usize) != AIR {
                            break;
                        }
                        chunk.set(x as usize, y as usize, z as usize, p.basalt);
                    }
                }
                _ => {}
            }
        }
    }

    /// A safe place to stand near (x, z) in the Nether: floor under two free cells, out of the
    /// lava, nearest to height `y`.
    pub fn nether_floor_near(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        (NETHER_LAVA + 1..NETHER_ROOF - 4)
            .filter(|&h| self.nether_solid(x, h, z) && !self.nether_solid(x, h + 1, z) && !self.nether_solid(x, h + 2, z))
            .min_by_key(|&h| (h - y).abs())
    }
}
