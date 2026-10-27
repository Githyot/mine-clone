//! Chunk storage. A chunk is a 16×16 column, 1024 blocks tall (world y -768 up to 255, the deep
//! Terraria-style underground lives below y 0), split into 64 sections of 16³.
//! Each section stores its blocks with a palette: an all-air section costs a few bytes,
//! a section with up to 16 kinds of block costs 2 KB instead of 8 KB.
//!
//! `get`/`set` take a world y from 0 up (the old surface world), `get_w`/`set_w` any world y,
//! `*_local` the index from the bottom of the column (what lighting and meshing walk).
//! Light is only kept for a band of sections around the player (`light_lo..light_hi`).

use crate::block::{BlockId, AIR};
use glam::IVec2;

pub const CHUNK_SIZE: usize = 16;
/// The top of the world (exclusive, world y).
pub const CHUNK_HEIGHT: usize = 256;
/// The bottom of the world (world y): the floor of the Underworld.
pub const MIN_Y: i32 = -768;
/// Blocks from the bottom of the column up to the top.
pub const WORLD_HEIGHT: usize = CHUNK_HEIGHT + DEPTH;
/// Blocks below y 0.
pub const DEPTH: usize = (-MIN_Y) as usize;
pub const SECTIONS: usize = WORLD_HEIGHT / 16;
/// Local section holding world y 0.
pub const BASE_SECTION: usize = DEPTH / 16;
pub const SECTION_VOLUME: usize = 16 * 16 * 16;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT;

pub type ChunkPos = IVec2;

/// Index inside a section: x + z*16 + y*256.
#[inline(always)]
pub fn section_index(x: usize, y: usize, z: usize) -> usize {
    x | (z << 4) | (y << 8)
}

/// Index inside a run of sections: x + z*16 + y*256 (y counted from the run's bottom).
#[inline(always)]
pub fn column_index(x: usize, y: usize, z: usize) -> usize {
    x | (z << 4) | (y << 8)
}

/// Local y (from the bottom of the column) of a world y.
#[inline(always)]
pub fn local_y(y: i32) -> usize {
    (y - MIN_Y) as usize
}

#[derive(Clone, Debug)]
pub struct PalettedSection {
    palette: Vec<BlockId>,
    /// Bits per entry: 0 (single block), 4, 8 or 16 (direct ids).
    bits: u32,
    data: Vec<u64>,
    /// Number of non-air blocks, so empty sections are skipped quickly.
    non_air: u16,
}

impl Default for PalettedSection {
    fn default() -> Self {
        Self::filled(AIR)
    }
}

impl PalettedSection {
    pub fn filled(id: BlockId) -> Self {
        PalettedSection {
            palette: vec![id],
            bits: 0,
            data: Vec::new(),
            non_air: if id == AIR { 0 } else { SECTION_VOLUME as u16 },
        }
    }

    /// Pack 4096 blocks (`section_index` order) at once, much faster than setting them one by one.
    pub fn from_slice(blocks: &[BlockId]) -> Self {
        let first = blocks[0];
        if blocks.iter().all(|&b| b == first) {
            return Self::filled(first);
        }
        let mut palette: Vec<BlockId> = Vec::with_capacity(16);
        let mut idx = vec![0u16; SECTION_VOLUME];
        let (mut last, mut last_i) = (BlockId::MAX, 0u16);
        let mut non_air = 0u16;
        for (i, &b) in blocks.iter().enumerate() {
            if b != AIR {
                non_air += 1;
            }
            if b != last {
                last = b;
                last_i = match palette.iter().position(|&p| p == b) {
                    Some(p) => p as u16,
                    None => {
                        palette.push(b);
                        (palette.len() - 1) as u16
                    }
                };
            }
            idx[i] = last_i;
        }
        let bits: u32 = if palette.len() <= 16 { 4 } else if palette.len() <= 256 { 8 } else { 16 };
        let mut s = PalettedSection { palette, bits, data: vec![0u64; SECTION_VOLUME.div_ceil(64 / bits as usize)], non_air };
        for (i, &p) in idx.iter().enumerate() {
            let v = if bits == 16 { blocks[i] as u32 } else { p as u32 };
            s.set_raw(i, v);
        }
        s
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.non_air == 0
    }

    #[inline]
    pub fn non_air(&self) -> u16 {
        self.non_air
    }

    /// Whether some block for which `f` holds may be here (false means surely not), from the
    /// palette alone, so a whole section can be skipped without reading its blocks.
    pub fn may_contain(&self, f: impl Fn(BlockId) -> bool) -> bool {
        self.bits == 16 || self.palette.iter().any(|&b| f(b))
    }

    #[inline(always)]
    fn raw(&self, i: usize) -> u32 {
        match self.bits {
            0 => 0,
            16 => ((self.data[i >> 2] >> ((i & 3) * 16)) & 0xFFFF) as u32,
            b => {
                let per = 64 / b as usize;
                let word = self.data[i / per];
                ((word >> ((i % per) * b as usize)) & ((1u64 << b) - 1)) as u32
            }
        }
    }

    #[inline(always)]
    fn set_raw(&mut self, i: usize, v: u32) {
        let b = self.bits as usize;
        let per = 64 / b;
        let shift = (i % per) * b;
        let mask = ((1u64 << b) - 1) << shift;
        let w = &mut self.data[i / per];
        *w = (*w & !mask) | ((v as u64) << shift);
    }

    #[inline(always)]
    pub fn get(&self, i: usize) -> BlockId {
        if self.bits == 16 {
            self.raw(i) as BlockId
        } else {
            self.palette[self.raw(i) as usize]
        }
    }

    pub fn set(&mut self, i: usize, id: BlockId) {
        let old = self.get(i);
        if old == id {
            return;
        }
        if old == AIR {
            self.non_air += 1;
        } else if id == AIR {
            self.non_air -= 1;
        }
        if self.bits == 16 {
            self.set_raw(i, id as u32);
            return;
        }
        let idx = match self.palette.iter().position(|&p| p == id) {
            Some(p) => p,
            None => {
                self.palette.push(id);
                let needed = self.palette.len();
                let cap = if self.bits == 0 { 1 } else { 1usize << self.bits };
                if needed > cap {
                    self.grow(needed);
                }
                if self.bits == 16 {
                    self.set_raw(i, id as u32);
                    return;
                }
                needed - 1
            }
        };
        self.set_raw(i, idx as u32);
    }

    fn grow(&mut self, needed: usize) {
        let new_bits = if needed <= 16 { 4 } else if needed <= 256 { 8 } else { 16 };
        let old: Vec<BlockId> = (0..SECTION_VOLUME)
            .map(|i| if self.bits == 0 { self.palette[0] } else { self.get(i) })
            .collect();
        self.bits = new_bits;
        let per = 64 / new_bits as usize;
        self.data = vec![0u64; SECTION_VOLUME.div_ceil(per)];
        if new_bits == 16 {
            for (i, id) in old.into_iter().enumerate() {
                self.set_raw(i, id as u32);
            }
        } else {
            for (i, id) in old.into_iter().enumerate() {
                let p = self.palette.iter().position(|&x| x == id).unwrap();
                self.set_raw(i, p as u32);
            }
        }
    }

    /// Memory used by the block data, for the debug screen.
    pub fn bytes(&self) -> usize {
        self.data.len() * 8 + self.palette.len() * 2
    }
}

#[derive(Clone)]
pub struct Chunk {
    pub pos: ChunkPos,
    pub sections: [PalettedSection; SECTIONS],
    /// Packed light per block of sections `light_lo..light_hi` (local): high nibble = sky light,
    /// low nibble = block light. Empty until lit.
    pub light: Vec<u8>,
    pub light_lo: usize,
    pub light_hi: usize,
    /// Biome id per column (x + z*16), used for grass/leaf tint.
    pub biomes: [u8; 256],
    /// Changed by a player since generation, so it must be saved.
    pub modified: bool,
    /// Loaded from a save made before the deep world: only y 0 and up came from the file.
    pub legacy: bool,
    /// Sections below this one (local) are not generated yet: plain stone standing in for the
    /// depths until someone goes down there (`Generator::generate_deep`).
    pub deep_lo: usize,
}

impl Chunk {
    pub fn new(pos: ChunkPos) -> Self {
        Chunk {
            pos,
            sections: std::array::from_fn(|_| PalettedSection::default()),
            light: Vec::new(),
            light_lo: 0,
            light_hi: 0,
            biomes: [0; 256],
            modified: false,
            legacy: false,
            deep_lo: 0,
        }
    }

    /// World y from 0 up.
    #[inline(always)]
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId {
        self.get_local(x, y + DEPTH, z)
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, id: BlockId) {
        self.set_local(x, y + DEPTH, z, id)
    }

    #[inline(always)]
    pub fn get_local(&self, x: usize, y: usize, z: usize) -> BlockId {
        self.sections[y >> 4].get(section_index(x, y & 15, z))
    }

    #[inline]
    pub fn set_local(&mut self, x: usize, y: usize, z: usize, id: BlockId) {
        self.sections[y >> 4].set(section_index(x, y & 15, z), id)
    }

    /// Any world y inside the world (MIN_Y..CHUNK_HEIGHT).
    #[inline(always)]
    pub fn get_w(&self, x: usize, y: i32, z: usize) -> BlockId {
        self.get_local(x, local_y(y), z)
    }

    #[inline]
    pub fn set_w(&mut self, x: usize, y: i32, z: usize, id: BlockId) {
        self.set_local(x, local_y(y), z, id)
    }

    /// Like `get_w` but returns air outside the vertical range.
    #[inline]
    pub fn get_i(&self, x: usize, y: i32, z: usize) -> BlockId {
        if !(MIN_Y..CHUNK_HEIGHT as i32).contains(&y) {
            AIR
        } else {
            self.get_w(x, y, z)
        }
    }

    /// Light at a world y from 0 up.
    #[inline(always)]
    pub fn light_at(&self, x: usize, y: usize, z: usize) -> u8 {
        self.light_local(x, y + DEPTH, z)
    }

    #[inline(always)]
    pub fn light_w(&self, x: usize, y: i32, z: usize) -> u8 {
        self.light_local(x, local_y(y), z)
    }

    /// Outside the lit band: open sky above it, dark rock below it.
    #[inline(always)]
    pub fn light_local(&self, x: usize, y: usize, z: usize) -> u8 {
        if self.light.is_empty() {
            0xF0
        } else if y < self.light_lo * 16 {
            0
        } else if y >= self.light_hi * 16 {
            0xF0
        } else {
            self.light[column_index(x, y - self.light_lo * 16, z)]
        }
    }

    /// Highest non-air block at or above y 0 in a column, or -1.
    pub fn top_y(&self, x: usize, z: usize) -> i32 {
        for s in (BASE_SECTION..SECTIONS).rev() {
            if self.sections[s].is_empty() {
                continue;
            }
            for yy in (0..16).rev() {
                let y = s * 16 + yy;
                if self.get_local(x, y, z) != AIR {
                    return y as i32 + MIN_Y;
                }
            }
        }
        -1
    }

    pub fn all_blocks(&self) -> Vec<BlockId> {
        let mut v = vec![AIR; CHUNK_VOLUME];
        for (s, sec) in self.sections.iter().enumerate() {
            if sec.is_empty() {
                continue;
            }
            for i in 0..SECTION_VOLUME {
                v[s * SECTION_VOLUME + i] = sec.get(i);
            }
        }
        v
    }

    pub fn set_all_blocks(&mut self, blocks: &[BlockId]) {
        for s in 0..SECTIONS {
            let slice = &blocks[s * SECTION_VOLUME..(s + 1) * SECTION_VOLUME];
            let first = slice[0];
            let mut sec = PalettedSection::filled(first);
            if slice.iter().any(|&b| b != first) {
                for (i, &b) in slice.iter().enumerate() {
                    sec.set(i, b);
                }
            }
            self.sections[s] = sec;
        }
    }

    pub fn memory_bytes(&self) -> usize {
        self.sections.iter().map(|s| s.bytes()).sum::<usize>() + self.light.len() + 256
    }
}

/// Split a world block coordinate into (chunk position, local x, local z).
#[inline]
pub fn split_xz(x: i32, z: i32) -> (ChunkPos, usize, usize) {
    (IVec2::new(x >> 4, z >> 4), (x & 15) as usize, (z & 15) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_grows_and_keeps_values() {
        let mut s = PalettedSection::default();
        assert!(s.is_empty());
        for i in 0..SECTION_VOLUME {
            s.set(i, (i % 300) as BlockId);
        }
        for i in 0..SECTION_VOLUME {
            assert_eq!(s.get(i), (i % 300) as BlockId);
        }
        assert_eq!(s.non_air() as usize, SECTION_VOLUME - SECTION_VOLUME.div_ceil(300));
    }

    #[test]
    fn from_slice_matches_set() {
        let v: Vec<BlockId> = (0..SECTION_VOLUME).map(|i| ((i / 7) % 5) as BlockId).collect();
        let s = PalettedSection::from_slice(&v);
        assert!((0..SECTION_VOLUME).all(|i| s.get(i) == v[i]));
        assert_eq!(s.non_air() as usize, v.iter().filter(|&&b| b != AIR).count());
        let w: Vec<BlockId> = (0..SECTION_VOLUME).map(|i| (i % 300) as BlockId).collect();
        let s = PalettedSection::from_slice(&w);
        assert!((0..SECTION_VOLUME).all(|i| s.get(i) == w[i]));
    }

    #[test]
    fn small_palette_is_small() {
        let mut s = PalettedSection::filled(1);
        s.set(5, 2);
        s.set(6, 3);
        assert_eq!(s.get(5), 2);
        assert_eq!(s.get(6), 3);
        assert_eq!(s.get(7), 1);
        assert!(s.bytes() <= 2100);
    }

    #[test]
    fn chunk_roundtrip() {
        let mut c = Chunk::new(IVec2::ZERO);
        c.set(3, 100, 7, 5);
        c.set(15, 255, 15, 9);
        let all = c.all_blocks();
        let mut d = Chunk::new(IVec2::ZERO);
        d.set_all_blocks(&all);
        assert_eq!(d.get(3, 100, 7), 5);
        assert_eq!(d.get(15, 255, 15), 9);
        assert_eq!(d.top_y(15, 15), 255);
    }
}
