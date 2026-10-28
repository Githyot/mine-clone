//! The test world ("Mundo de teste"): flat grass with everything the game has laid out on it.
//! The game works out where each thing goes (it knows the item tabs and the mobs); the
//! generator only stamps those blocks into the chunks they fall in, so chunks stay lazy and as
//! cheap as a superflat world.

use std::collections::HashMap;

use crate::block::BlockId;
use crate::chunk::{Chunk, ChunkPos, PalettedSection, BASE_SECTION};
use crate::structures::Placement;
use crate::worldgen::Generator;

pub struct Showcase {
    /// The grass top of the floor.
    pub floor: i32,
    pub grass: BlockId,
    pub dirt: BlockId,
    pub stone: BlockId,
    /// Blocks set over the floor, by chunk: (x, y, z in the chunk, block).
    cells: HashMap<(i32, i32), Vec<(u8, i32, u8, BlockId)>>,
    /// One of each building, where the game put it.
    pub buildings: Vec<Placement>,
}

impl Showcase {
    pub fn new(floor: i32, grass: BlockId, dirt: BlockId, stone: BlockId) -> Showcase {
        Showcase { floor, grass, dirt, stone, cells: HashMap::new(), buildings: Vec::new() }
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, id: BlockId) {
        let key = (x.div_euclid(16), z.div_euclid(16));
        self.cells.entry(key).or_default().push((x.rem_euclid(16) as u8, y, z.rem_euclid(16) as u8, id));
    }

    pub fn block_count(&self) -> usize {
        self.cells.values().map(|v| v.len()).sum()
    }
}

impl Generator {
    pub(crate) fn generate_showcase(&self, sc: &Showcase, pos: ChunkPos) -> Chunk {
        let mut chunk = Chunk::new(pos);
        // Solid stone under everything, dirt and grass on top.
        let stone = PalettedSection::filled(sc.stone);
        let below = (sc.floor - 3).div_euclid(16);
        for s in 0..(BASE_SECTION as i32 + below).max(0) as usize {
            chunk.sections[s] = stone.clone();
        }
        chunk.deep_lo = BASE_SECTION;
        for lz in 0..16 {
            for lx in 0..16 {
                for y in (below * 16).max(0)..=sc.floor {
                    let id = if y == sc.floor { sc.grass } else if y >= sc.floor - 3 { sc.dirt } else { sc.stone };
                    chunk.set(lx, y as usize, lz, id);
                }
            }
        }
        if let Some(list) = sc.cells.get(&(pos.x, pos.y)) {
            for &(x, y, z, id) in list {
                if y >= 0 {
                    chunk.set(x as usize, y as usize, z as usize, id);
                }
            }
        }
        self.place_structures(&mut chunk);
        chunk
    }
}
