//! Saving and loading worlds. Layout:
//! `saves/<world>/level.ron` (seed, player, time) and `saves/<world>/chunks/<x>.<z>.bin` for every
//! chunk a player changed. Untouched chunks are regenerated from the seed, so saves stay small.
//! Chunk files store block *names* in a small table, so adding or removing blocks never corrupts a world.

use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos, CHUNK_VOLUME};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"MCRS";
const VERSION: u16 = 1;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LevelData {
    pub name: String,
    pub seed: u64,
    pub player_pos: [f32; 3],
    pub player_yaw: f32,
    pub player_pitch: f32,
    pub flying: bool,
    /// Ticks into the day cycle (0..24000).
    pub time_of_day: f32,
    #[serde(default)]
    pub hotbar: Vec<String>,
    /// Survival mode (health, damage) instead of creative.
    #[serde(default)]
    pub survival: bool,
    #[serde(default = "full_health")]
    pub health: f32,
    /// Weather: raining, storm, and seconds until it changes (0 = pick a fresh spell).
    #[serde(default)]
    pub raining: bool,
    #[serde(default)]
    pub thunder: bool,
    #[serde(default)]
    pub weather_timer: f32,
}

fn full_health() -> f32 {
    20.0
}

pub struct WorldSave {
    pub dir: PathBuf,
}

impl WorldSave {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        WorldSave { dir: dir.into() }
    }

    fn chunk_path(&self, pos: ChunkPos) -> PathBuf {
        self.dir.join("chunks").join(format!("{}.{}.bin", pos.x, pos.y))
    }

    pub fn load_level(&self) -> Option<LevelData> {
        let text = fs::read_to_string(self.dir.join("level.ron")).ok()?;
        ron::from_str(&text).ok()
    }

    pub fn save_level(&self, level: &LevelData) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let text = ron::ser::to_string_pretty(level, ron::ser::PrettyConfig::default()).map_err(io::Error::other)?;
        write_atomic(&self.dir.join("level.ron"), text.as_bytes())
    }

    pub fn save_chunk(&self, reg: &BlockRegistry, chunk: &Chunk) -> io::Result<()> {
        fs::create_dir_all(self.dir.join("chunks"))?;
        write_atomic(&self.chunk_path(chunk.pos), &encode_chunk(reg, chunk))
    }

    pub fn load_chunk(&self, reg: &BlockRegistry, pos: ChunkPos) -> Option<Chunk> {
        let bytes = fs::read(self.chunk_path(pos)).ok()?;
        decode_chunk(reg, pos, &bytes)
    }

    pub fn has_chunk(&self, pos: ChunkPos) -> bool {
        self.chunk_path(pos).exists()
    }
}

fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(tmp, path)
}

pub fn encode_chunk(reg: &BlockRegistry, chunk: &Chunk) -> Vec<u8> {
    let blocks = chunk.all_blocks();
    // Local palette: only names of blocks present in this chunk.
    let mut local: HashMap<BlockId, u16> = HashMap::new();
    let mut names: Vec<&str> = Vec::new();
    let mut ids = Vec::with_capacity(CHUNK_VOLUME * 2);
    for &b in &blocks {
        let next = names.len() as u16;
        let l = *local.entry(b).or_insert_with(|| {
            names.push(&reg.get(b).name);
            next
        });
        ids.extend_from_slice(&l.to_le_bytes());
    }
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(names.len() as u16).to_le_bytes());
    for n in &names {
        out.push(n.len() as u8);
        out.extend_from_slice(n.as_bytes());
    }
    out.extend_from_slice(&chunk.biomes);
    out.extend_from_slice(&lz4_flex::compress_prepend_size(&ids));
    out
}

pub fn decode_chunk(reg: &BlockRegistry, pos: ChunkPos, bytes: &[u8]) -> Option<Chunk> {
    if bytes.len() < 8 || &bytes[0..4] != MAGIC {
        return None;
    }
    let mut i = 6;
    let count = u16::from_le_bytes([bytes[i], bytes[i + 1]]) as usize;
    i += 2;
    let mut map = Vec::with_capacity(count);
    for _ in 0..count {
        let len = *bytes.get(i)? as usize;
        i += 1;
        let name = std::str::from_utf8(bytes.get(i..i + len)?).ok()?;
        i += len;
        // Blocks that no longer exist (removed mod) turn into air.
        map.push(reg.id(name).unwrap_or(AIR));
    }
    let biomes: [u8; 256] = bytes.get(i..i + 256)?.try_into().ok()?;
    i += 256;
    let raw = lz4_flex::decompress_size_prepended(&bytes[i..]).ok()?;
    if raw.len() != CHUNK_VOLUME * 2 {
        return None;
    }
    let blocks: Vec<BlockId> = raw
        .chunks_exact(2)
        .map(|c| *map.get(u16::from_le_bytes([c[0], c[1]]) as usize).unwrap_or(&AIR))
        .collect();
    let mut chunk = Chunk::new(pos);
    chunk.set_all_blocks(&blocks);
    chunk.biomes = biomes;
    chunk.modified = true;
    Some(chunk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_roundtrip_by_name() {
        let reg = BlockRegistry::from_ron(r#"(blocks: [(name: "stone"), (name: "dirt")])"#).unwrap();
        let mut c = Chunk::new(ChunkPos::new(2, -7));
        c.set(1, 2, 3, reg.id("dirt").unwrap());
        c.set(4, 200, 9, reg.id("stone").unwrap());
        c.biomes[10] = 3;
        let bytes = encode_chunk(&reg, &c);
        // A registry with a different order still loads the right blocks.
        let reg2 = BlockRegistry::from_ron(r#"(blocks: [(name: "dirt"), (name: "glass"), (name: "stone")])"#).unwrap();
        let d = decode_chunk(&reg2, c.pos, &bytes).unwrap();
        assert_eq!(d.get(1, 2, 3), reg2.id("dirt").unwrap());
        assert_eq!(d.get(4, 200, 9), reg2.id("stone").unwrap());
        assert_eq!(d.biomes[10], 3);
        assert!(bytes.len() < 4000, "{}", bytes.len());
    }
}
