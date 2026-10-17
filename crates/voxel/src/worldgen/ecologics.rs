//! Ecologics idea in the Overworld: coconut palms with hanging coconuts on the beaches, walnut
//! trees here and there on the plains, maples glowing orange in the forests, and patches of
//! thin ice on the frozen lakes and shores of the cold biomes.
//!
//! Runs after the classic decoration on its own random stream, so the older trees and plants
//! keep their places. Trees stay inside the chunk like every other tree.

use super::{Biome, ChunkSpace, Column, Generator, SEA_LEVEL};
use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos};
use crate::modtrees::{self, ModTree, Shape};
use crate::rng::Rng;

#[derive(Clone)]
pub(super) struct EcoPalette {
    palm: ModTree,
    walnut: ModTree,
    maple: ModTree,
    sand: BlockId,
    grass: BlockId,
    dirt: BlockId,
    ice: BlockId,
    thin_ice: BlockId,
}

impl EcoPalette {
    pub(super) fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        // The coconut hangs (its lantern shape's variant 1).
        let coconut = r.id("coconut").map(|c| r.variant(c, 1)).unwrap_or(AIR);
        EcoPalette {
            palm: ModTree { log: g("coconut_log"), leaves: g("coconut_leaves"), shape: Shape::Palm, fruit: coconut },
            walnut: ModTree { log: g("walnut_log"), leaves: g("walnut_leaves"), shape: Shape::Round, fruit: AIR },
            maple: ModTree { log: g("maple_log"), leaves: g("maple_leaves"), shape: Shape::Spread, fruit: AIR },
            sand: g("sand"),
            grass: g("grass_block"),
            dirt: g("dirt"),
            ice: g("ice"),
            thin_ice: g("thin_ice"),
        }
    }
}

impl Generator {
    pub(super) fn decorate_ecologics(&self, chunk: &mut Chunk, cols: &[Column], pos: ChunkPos) {
        let p = &self.ecologics;
        if p.palm.log == AIR {
            return;
        }
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 7);
        let biome = cols[8 + 8 * 16].biome;
        let (tree, attempts, ground) = match biome {
            Biome::Beach => (p.palm, 3, p.sand),
            Biome::Plains => (p.walnut, if rng.chance(0.3) { 1 } else { 0 }, p.grass),
            Biome::Forest | Biome::BirchForest => (p.maple, if rng.chance(0.45) { 2 } else { 0 }, p.grass),
            Biome::Taiga => (p.maple, if rng.chance(0.15) { 1 } else { 0 }, p.grass),
            _ => (p.palm, 0, AIR),
        };
        let reach = tree.shape.reach();
        for _ in 0..attempts {
            let x = rng.range(reach, 15 - reach);
            let z = rng.range(reach, 15 - reach);
            let col = &cols[x as usize + z as usize * 16];
            let y = col.height;
            if col.biome != biome || y < SEA_LEVEL || y > 200 {
                continue;
            }
            if chunk.get(x as usize, y as usize, z as usize) != ground || chunk.get(x as usize, y as usize + 1, z as usize) != AIR {
                continue;
            }
            // Room to grow: no other tree's trunk right here.
            if chunk.get(x as usize, y as usize + 2, z as usize) != AIR {
                continue;
            }
            if ground == p.grass {
                chunk.set(x as usize, y as usize, z as usize, p.dirt);
            }
            modtrees::grow(tree, &mut ChunkSpace(chunk), &mut rng, x, y, z);
        }
        // Thin ice over the frozen water, in wandering patches.
        if p.thin_ice != AIR && matches!(biome, Biome::Snowy | Biome::Taiga) {
            let s = SEA_LEVEL as usize;
            for z in 0..16usize {
                for x in 0..16usize {
                    if chunk.get(x, s, z) != p.ice {
                        continue;
                    }
                    let (wx, wz) = (pos.x * 16 + x as i32, pos.y * 16 + z as i32);
                    let n = self.detail.fbm2(wx as f32 / 23.0 + 517.0, wz as f32 / 23.0 - 93.0, 2);
                    if n > 0.15 {
                        chunk.set(x, s, z, p.thin_ice);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> BlockRegistry {
        BlockRegistry::from_ron_many(&[include_str!("../../../../packs/vanilla/blocks.ron"), include_str!("../../../../packs/vanilla/mods/blocks.ron")]).unwrap()
    }

    /// Beaches grow palms with coconuts and forests grow maples somewhere near the origin.
    #[test]
    fn palms_and_maples_grow() {
        let reg = registry();
        let g = Generator::new(1234, &reg);
        let palm = reg.id("coconut_log").unwrap();
        let maple = reg.id("maple_log").unwrap();
        let coconut = reg.id("coconut").unwrap();
        let (mut palms, mut maples, mut nuts) = (0, 0, 0);
        let print = std::env::var("ECO_PRINT").is_ok();
        for cz in -24..24 {
            for cx in -24..24 {
                let b = g.biome_at(cx * 16 + 8, cz * 16 + 8);
                if !matches!(b, Biome::Beach | Biome::Forest | Biome::BirchForest) {
                    continue;
                }
                let c = g.generate(ChunkPos::new(cx, cz));
                let mut found = (false, false);
                for id in c.all_blocks() {
                    if id == palm {
                        found.0 = true;
                    } else if id == maple {
                        found.1 = true;
                    } else if reg.get(id).base == coconut {
                        nuts += 1;
                    }
                }
                palms += found.0 as u32;
                maples += found.1 as u32;
                if print && (found.0 || found.1) {
                    println!("{} chunk at {} {}", if found.0 { "palm" } else { "maple" }, cx * 16 + 8, cz * 16 + 8);
                }
            }
        }
        assert!(palms > 0 && maples > 0 && nuts > 0, "palms {palms}, maples {maples}, coconuts {nuts}");
    }
}
