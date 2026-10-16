//! World generation. Every chunk is a pure function of (seed, chunk position), so chunks can be
//! generated on any thread in any order. Pipeline: climate → height → surface → caves → ores → plants/trees.

use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos, CHUNK_HEIGHT};
use crate::noise::Perlin;
use crate::rng::Rng;

pub const SEA_LEVEL: i32 = 62;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Biome {
    Plains = 0,
    Forest = 1,
    Desert = 2,
    Snowy = 3,
    Ocean = 4,
    Beach = 5,
    Mountains = 6,
    BirchForest = 7,
    Taiga = 8,
}

impl Biome {
    pub const COUNT: usize = 9;

    pub fn from_u8(v: u8) -> Biome {
        match v {
            1 => Biome::Forest,
            2 => Biome::Desert,
            3 => Biome::Snowy,
            4 => Biome::Ocean,
            5 => Biome::Beach,
            6 => Biome::Mountains,
            7 => Biome::BirchForest,
            8 => Biome::Taiga,
            _ => Biome::Plains,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Biome::Plains => "Plains",
            Biome::Forest => "Forest",
            Biome::Desert => "Desert",
            Biome::Snowy => "Snowy Plains",
            Biome::Ocean => "Ocean",
            Biome::Beach => "Beach",
            Biome::Mountains => "Mountains",
            Biome::BirchForest => "Birch Forest",
            Biome::Taiga => "Taiga",
        }
    }

    /// Grass/leaf tint multiplier (RGB). Textures are already green, so these stay near 1.0.
    pub fn tint(self) -> [f32; 3] {
        match self {
            Biome::Plains => [1.0, 1.0, 1.0],
            Biome::Forest => [0.88, 1.0, 0.85],
            Biome::Desert | Biome::Beach => [1.12, 1.02, 0.72],
            Biome::Snowy => [0.85, 0.95, 1.0],
            Biome::Ocean => [0.95, 1.0, 0.95],
            Biome::Mountains => [0.9, 0.98, 0.95],
            Biome::BirchForest => [1.0, 1.05, 0.85],
            Biome::Taiga => [0.78, 0.92, 0.88],
        }
    }
}

/// Block ids used by the generator, resolved once from the registry.
#[derive(Clone)]
struct Palette {
    stone: BlockId,
    dirt: BlockId,
    grass: BlockId,
    snow_grass: BlockId,
    sand: BlockId,
    sandstone: BlockId,
    gravel: BlockId,
    clay: BlockId,
    water: BlockId,
    ice: BlockId,
    snow_layer: BlockId,
    snow_layer2: BlockId,
    bedrock: BlockId,
    lava: BlockId,
    oak_log: BlockId,
    oak_leaves: BlockId,
    birch_log: BlockId,
    birch_leaves: BlockId,
    spruce_log: BlockId,
    spruce_leaves: BlockId,
    cactus: BlockId,
    short_grass: BlockId,
    poppy: BlockId,
    dandelion: BlockId,
    dead_bush: BlockId,
    pumpkin: BlockId,
    melon: BlockId,
    /// (block, min y, max y, veins per chunk, vein size)
    ores: Vec<(BlockId, i32, i32, u32, u32)>,
    emerald: BlockId,
}

impl Palette {
    fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        let ores = vec![
            (g("coal_ore"), 5, 128, 20, 12),
            (g("copper_ore"), 0, 96, 10, 9),
            (g("tin_ore"), 0, 80, 10, 8),
            (g("iron_ore"), 0, 64, 14, 8),
            (g("silver_ore"), 0, 40, 5, 7),
            (g("gold_ore"), 0, 32, 3, 7),
            (g("redstone_ore"), 0, 16, 7, 7),
            (g("lapis_ore"), 0, 30, 2, 6),
            (g("diamond_ore"), 0, 16, 2, 6),
            (g("gravel"), 0, 128, 6, 24),
            (g("dirt"), 0, 128, 6, 24),
        ];
        Palette {
            stone: g("stone"),
            dirt: g("dirt"),
            grass: g("grass_block"),
            snow_grass: g("snowy_grass_block"),
            sand: g("sand"),
            sandstone: g("sandstone"),
            gravel: g("gravel"),
            clay: g("clay"),
            water: g("water"),
            ice: g("ice"),
            snow_layer: g("snow_layer"),
            snow_layer2: g("snow_layer:1"),
            bedrock: g("bedrock"),
            lava: g("lava"),
            oak_log: g("oak_log"),
            oak_leaves: g("oak_leaves"),
            birch_log: g("birch_log"),
            birch_leaves: g("birch_leaves"),
            spruce_log: g("spruce_log"),
            spruce_leaves: g("spruce_leaves"),
            cactus: g("cactus"),
            short_grass: g("short_grass"),
            poppy: g("poppy"),
            dandelion: g("dandelion"),
            dead_bush: g("dead_bush"),
            pumpkin: g("pumpkin"),
            melon: g("melon"),
            ores,
            emerald: g("emerald_ore"),
        }
    }
}

pub struct Generator {
    pub seed: u64,
    continent: Perlin,
    mountain: Perlin,
    detail: Perlin,
    temperature: Perlin,
    humidity: Perlin,
    cave_a: Perlin,
    cave_b: Perlin,
    cheese: Perlin,
    pal: Palette,
    structures: std::sync::Arc<crate::structures::Structures>,
}

/// What the top of one column looks like, without generating the chunk (distant terrain).
#[derive(Clone, Copy, Debug)]
pub struct Surface {
    /// Highest solid block.
    pub height: i32,
    pub biome: Biome,
    /// Block on top of the ground (before trees and plants).
    pub top: BlockId,
    /// Water or ice above the ground, up to `SEA_LEVEL`.
    pub liquid: Option<BlockId>,
    /// Leaves of the trees that grow here, and roughly how much of the ground they cover (0..1).
    pub trees: Option<(BlockId, f32)>,
}

/// What the generator decided for one column.
struct Column {
    height: i32,
    biome: Biome,
}

impl Generator {
    pub fn new(seed: u64, reg: &BlockRegistry) -> Self {
        Generator {
            structures: std::sync::Arc::new(crate::structures::Structures::empty()),
            seed,
            continent: Perlin::new(seed ^ 0x01),
            mountain: Perlin::new(seed ^ 0x02),
            detail: Perlin::new(seed ^ 0x03),
            temperature: Perlin::new(seed ^ 0x04),
            humidity: Perlin::new(seed ^ 0x05),
            cave_a: Perlin::new(seed ^ 0x06),
            cave_b: Perlin::new(seed ^ 0x07),
            cheese: Perlin::new(seed ^ 0x08),
            pal: Palette::new(reg),
        }
    }

    fn column(&self, wx: i32, wz: i32) -> Column {
        let x = wx as f32;
        let z = wz as f32;
        let cont = self.continent.fbm2(x / 700.0, z / 700.0, 4); // -1..1
        let mount_mask = ((self.mountain.fbm2(x / 900.0 + 50.0, z / 900.0, 2) + 0.05) * 3.0).clamp(0.0, 1.0);
        let ridges = self.mountain.ridged2(x / 260.0, z / 260.0, 4);
        let detail = self.detail.fbm2(x / 48.0, z / 48.0, 3);
        let mut h = 64.0 + cont * 34.0 + detail * 5.0;
        // Land rises gently from the coast.
        if cont > 0.0 {
            h += cont * 10.0;
        }
        h += mount_mask * ridges * ridges * 95.0 * cont.max(0.05).sqrt();
        let height = (h as i32).clamp(4, CHUNK_HEIGHT as i32 - 20);

        let temp = self.temperature.fbm2(x / 1100.0, z / 1100.0, 3) - (height - 90).max(0) as f32 / 120.0;
        let hum = self.humidity.fbm2(x / 900.0 + 300.0, z / 900.0, 3);
        let biome = if height < SEA_LEVEL - 2 {
            Biome::Ocean
        } else if height <= SEA_LEVEL + 1 && temp > -0.25 {
            Biome::Beach
        } else if height > 118 {
            Biome::Mountains
        } else if temp < -0.28 {
            if hum > 0.0 {
                Biome::Taiga
            } else {
                Biome::Snowy
            }
        } else if temp > 0.32 && hum < 0.05 {
            Biome::Desert
        } else if hum > 0.22 {
            Biome::Forest
        } else if hum > 0.08 && temp < 0.1 {
            Biome::BirchForest
        } else {
            Biome::Plains
        };
        Column { height, biome }
    }

    pub fn biome_at(&self, wx: i32, wz: i32) -> Biome {
        self.column(wx, wz).biome
    }

    pub fn height_at(&self, wx: i32, wz: i32) -> i32 {
        self.column(wx, wz).height
    }

    /// Top, filler and filler depth for a column.
    fn top_blocks(&self, col: &Column, wx: i32, wz: i32) -> (BlockId, BlockId, i32) {
        let p = &self.pal;
        let h = col.height;
        match col.biome {
            Biome::Desert => (p.sand, p.sandstone, 4),
            Biome::Beach => (p.sand, p.sand, 3),
            Biome::Ocean => {
                if (wx ^ wz) & 7 == 0 && h > SEA_LEVEL - 12 {
                    (p.clay, p.sand, 2)
                } else if h < SEA_LEVEL - 14 {
                    (p.gravel, p.gravel, 2)
                } else {
                    (p.sand, p.sand, 3)
                }
            }
            Biome::Mountains => {
                if h > 150 {
                    (p.snow_grass, p.stone, 1)
                } else if h > 128 {
                    (p.stone, p.stone, 1)
                } else {
                    (p.grass, p.dirt, 3)
                }
            }
            Biome::Snowy | Biome::Taiga => (p.snow_grass, p.dirt, 3),
            _ => (p.grass, p.dirt, 3),
        }
    }

    /// The surface of one column, cheaply: no caves, ores or chunk. Matches `generate` except
    /// for trees, which are only a density here.
    pub fn surface(&self, wx: i32, wz: i32) -> Surface {
        let p = &self.pal;
        let col = self.column(wx, wz);
        let (top, _, _) = self.top_blocks(&col, wx, wz);
        let h = col.height;
        let top = if h < SEA_LEVEL && (top == p.grass || top == p.snow_grass) { p.dirt } else { top };
        let liquid = (h < SEA_LEVEL).then_some(if matches!(col.biome, Biome::Snowy | Biome::Taiga) { p.ice } else { p.water });
        // Tree attempts per chunk in `decorate`, out of the forest's 9.
        let trees = if !(SEA_LEVEL..=200).contains(&h) {
            None
        } else {
            match col.biome {
                Biome::Forest => Some((p.oak_leaves, 1.0)),
                Biome::BirchForest => Some((p.birch_leaves, 1.0)),
                Biome::Taiga => Some((p.spruce_leaves, 1.0)),
                Biome::Plains => Some((p.oak_leaves, 0.11)),
                Biome::Snowy | Biome::Mountains if h <= 128 => Some((p.spruce_leaves, 0.11)),
                _ => None,
            }
        };
        Surface { height: h, biome: col.biome, top, liquid, trees }
    }

    #[inline]
    fn is_cave(&self, wx: i32, y: i32, wz: i32, surface: i32) -> bool {
        if y < 6 || y > surface - 3 {
            return false;
        }
        let (x, yf, z) = (wx as f32, y as f32, wz as f32);
        // Spaghetti caves: where two noise fields both cross zero we get long winding tunnels.
        let a = self.cave_a.noise3(x / 48.0, yf / 32.0, z / 48.0);
        let b = self.cave_b.noise3(x / 48.0, yf / 32.0, z / 48.0);
        if a.abs() < 0.06 && b.abs() < 0.06 {
            return true;
        }
        // Cheese caves: big open rooms deeper down.
        if y < 48 {
            let c = self.cheese.noise3(x / 70.0, yf / 40.0, z / 70.0);
            let depth_bias = (48 - y) as f32 / 120.0;
            if c > 0.62 - depth_bias * 0.4 {
                return true;
            }
        }
        false
    }

    pub fn generate(&self, pos: ChunkPos) -> Chunk {
        let p = &self.pal;
        let mut chunk = Chunk::new(pos);
        let base_x = pos.x * 16;
        let base_z = pos.y * 16;
        let mut cols: Vec<Column> = Vec::with_capacity(256);
        for lz in 0..16 {
            for lx in 0..16 {
                cols.push(self.column(base_x + lx, base_z + lz));
            }
        }
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 1);

        for lz in 0..16usize {
            for lx in 0..16usize {
                let col = &cols[lx + lz * 16];
                let wx = base_x + lx as i32;
                let wz = base_z + lz as i32;
                chunk.biomes[lx + lz * 16] = col.biome as u8;
                let h = col.height;
                let (top, filler, filler_depth) = self.top_blocks(col, wx, wz);
                for y in 0..=h.max(SEA_LEVEL) {
                    let id = if y == 0 || (y <= 4 && rng.below(5) < (5 - y) as u32) {
                        p.bedrock
                    } else if y > h {
                        if y == SEA_LEVEL && matches!(col.biome, Biome::Snowy | Biome::Taiga) {
                            p.ice
                        } else {
                            p.water
                        }
                    } else if self.is_cave(wx, y, wz, h) {
                        if y < 11 {
                            p.lava
                        } else {
                            AIR
                        }
                    } else if y == h {
                        if h < SEA_LEVEL && top == p.grass {
                            p.dirt
                        } else if h < SEA_LEVEL && top == p.snow_grass {
                            p.dirt
                        } else {
                            top
                        }
                    } else if y > h - filler_depth {
                        filler
                    } else {
                        p.stone
                    };
                    if id != AIR {
                        chunk.set(lx, y as usize, lz, id);
                    }
                }
            }
        }

        self.place_ores(&mut chunk, &mut rng, &cols);
        self.decorate(&mut chunk, &mut rng, &cols);
        self.place_structures(&mut chunk);
        chunk
    }

    /// Buildings from data (see `structures`).
    pub fn with_structures(mut self, s: std::sync::Arc<crate::structures::Structures>) -> Self {
        self.structures = s;
        self
    }

    pub(crate) fn structures(&self) -> &crate::structures::Structures {
        &self.structures
    }

    fn place_ores(&self, chunk: &mut Chunk, rng: &mut Rng, cols: &[Column]) {
        let p = &self.pal;
        for &(ore, lo, hi, veins, size) in &p.ores {
            if ore == AIR {
                continue;
            }
            for _ in 0..veins {
                let mut x = rng.range(1, 14);
                let mut y = rng.range(lo.max(1), hi);
                let mut z = rng.range(1, 14);
                let n = size / 2 + rng.below(size / 2 + 1);
                for _ in 0..n {
                    if (0..16).contains(&x) && (1..CHUNK_HEIGHT as i32).contains(&y) && (0..16).contains(&z) {
                        let (ux, uy, uz) = (x as usize, y as usize, z as usize);
                        if chunk.get(ux, uy, uz) == p.stone {
                            chunk.set(ux, uy, uz, ore);
                        }
                    }
                    match rng.below(6) {
                        0 => x += 1,
                        1 => x -= 1,
                        2 => y += 1,
                        3 => y -= 1,
                        4 => z += 1,
                        _ => z -= 1,
                    }
                }
            }
        }
        // Emeralds: single blocks, mountains only.
        if p.emerald != AIR {
            for _ in 0..4 {
                let (x, z) = (rng.range(0, 15) as usize, rng.range(0, 15) as usize);
                if cols[x + z * 16].biome == Biome::Mountains {
                    let y = rng.range(4, 30) as usize;
                    if chunk.get(x, y, z) == p.stone {
                        chunk.set(x, y, z, p.emerald);
                    }
                }
            }
        }
    }

    fn decorate(&self, chunk: &mut Chunk, rng: &mut Rng, cols: &[Column]) {
        let p = &self.pal;
        // Trees stay inside the chunk (2 blocks from the edge) so generation never writes to neighbors.
        let tree_attempts = match cols[8 + 8 * 16].biome {
            Biome::Forest | Biome::BirchForest | Biome::Taiga => 9,
            Biome::Plains | Biome::Snowy => 1,
            Biome::Mountains => 1,
            Biome::Desert => 3,
            _ => 0,
        };
        for _ in 0..tree_attempts {
            let x = rng.range(2, 13) as usize;
            let z = rng.range(2, 13) as usize;
            let col = &cols[x + z * 16];
            let y = col.height;
            if y < SEA_LEVEL || y > 200 {
                continue;
            }
            let ground = chunk.get(x, y as usize, z);
            if col.biome == Biome::Desert {
                if ground == p.sand && rng.chance(0.6) {
                    let hgt = rng.range(1, 3);
                    for i in 1..=hgt {
                        chunk.set(x, (y + i) as usize, z, p.cactus);
                    }
                }
                continue;
            }
            if ground != p.grass && ground != p.snow_grass {
                continue;
            }
            match col.biome {
                Biome::Taiga | Biome::Snowy | Biome::Mountains => self.spruce(chunk, rng, x, y, z),
                Biome::BirchForest => {
                    if rng.chance(0.8) {
                        self.round_tree(chunk, rng, x, y, z, p.birch_log, p.birch_leaves)
                    } else {
                        self.round_tree(chunk, rng, x, y, z, p.oak_log, p.oak_leaves)
                    }
                }
                Biome::Forest => {
                    if rng.chance(0.2) {
                        self.round_tree(chunk, rng, x, y, z, p.birch_log, p.birch_leaves)
                    } else {
                        self.round_tree(chunk, rng, x, y, z, p.oak_log, p.oak_leaves)
                    }
                }
                _ => self.round_tree(chunk, rng, x, y, z, p.oak_log, p.oak_leaves),
            }
        }

        // Ground plants.
        for z in 0..16usize {
            for x in 0..16usize {
                let col = &cols[x + z * 16];
                let y = col.height;
                if y < SEA_LEVEL || y + 1 >= CHUNK_HEIGHT as i32 {
                    continue;
                }
                let (uy, above) = (y as usize, y as usize + 1);
                if chunk.get(x, above, z) != AIR {
                    continue;
                }
                let ground = chunk.get(x, uy, z);
                if ground == p.grass {
                    let r = rng.f32();
                    let grass_p = match col.biome {
                        Biome::Plains => 0.22,
                        Biome::Forest | Biome::BirchForest => 0.12,
                        _ => 0.06,
                    };
                    if r < 0.012 {
                        chunk.set(x, above, z, if rng.chance(0.5) { p.poppy } else { p.dandelion });
                    } else if r < 0.012 + grass_p {
                        chunk.set(x, above, z, p.short_grass);
                    } else if r < 0.0125 + grass_p && col.biome == Biome::Plains {
                        chunk.set(x, above, z, if rng.chance(0.5) { p.pumpkin } else { p.melon });
                    }
                } else if ground == p.sand && col.biome == Biome::Desert && rng.chance(0.01) {
                    chunk.set(x, above, z, p.dead_bush);
                } else if ground == p.snow_grass && chunk.get(x, above, z) == AIR && p.snow_layer != AIR {
                    // A blanket of snow, a little deeper here and there.
                    let deep = rng.chance(0.15);
                    chunk.set(x, above, z, if deep { p.snow_layer2 } else { p.snow_layer });
                }
            }
        }
    }

    fn round_tree(&self, chunk: &mut Chunk, rng: &mut Rng, x: usize, y: i32, z: usize, log: BlockId, leaves: BlockId) {
        let h = rng.range(4, 6);
        let top = y + h;
        if top + 2 >= CHUNK_HEIGHT as i32 {
            return;
        }
        for ly in (top - 2)..=(top + 1) {
            let r: i32 = if ly >= top { 1 } else { 2 };
            for dx in -r..=r {
                for dz in -r..=r {
                    // Trim corners randomly for the classic round-ish look.
                    if dx.abs() == r && dz.abs() == r && (ly >= top || rng.chance(0.5)) {
                        continue;
                    }
                    let (px, pz) = (x as i32 + dx, z as i32 + dz);
                    if (0..16).contains(&px) && (0..16).contains(&pz) {
                        let (px, py, pz) = (px as usize, ly as usize, pz as usize);
                        if chunk.get(px, py, pz) == AIR {
                            chunk.set(px, py, pz, leaves);
                        }
                    }
                }
            }
        }
        for ly in (y + 1)..top {
            chunk.set(x, ly as usize, z, log);
        }
        chunk.set(x, y as usize, z, self.pal.dirt);
    }

    fn spruce(&self, chunk: &mut Chunk, rng: &mut Rng, x: usize, y: i32, z: usize) {
        let p = &self.pal;
        let h = rng.range(6, 9);
        let top = y + h;
        if top + 2 >= CHUNK_HEIGHT as i32 {
            return;
        }
        let mut r = 0i32;
        let mut ly = top + 1;
        while ly > y + 2 {
            for dx in -r..=r {
                for dz in -r..=r {
                    if r > 0 && dx.abs() == r && dz.abs() == r {
                        continue;
                    }
                    let (px, pz) = (x as i32 + dx, z as i32 + dz);
                    if (0..16).contains(&px) && (0..16).contains(&pz) && chunk.get(px as usize, ly as usize, pz as usize) == AIR {
                        chunk.set(px as usize, ly as usize, pz as usize, p.spruce_leaves);
                    }
                }
            }
            r = if r >= 2 { 1 } else { r + 1 };
            ly -= 1;
        }
        for ly in (y + 1)..=top {
            chunk.set(x, ly as usize, z, p.spruce_log);
        }
        chunk.set(x, y as usize, z, p.dirt);
    }

    /// A good spawn point: a land column near the origin.
    pub fn find_spawn(&self) -> (i32, i32, i32) {
        for r in 0..200 {
            for (dx, dz) in [(r, 0), (-r, 0), (0, r), (0, -r)] {
                let x = dx * 8;
                let z = dz * 8;
                let c = self.column(x, z);
                if c.height > SEA_LEVEL + 1 && c.biome != Biome::Mountains {
                    return (x, c.height + 2, z);
                }
            }
        }
        (0, 100, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> BlockRegistry {
        BlockRegistry::from_ron(include_str!("../../../packs/vanilla/blocks.ron")).unwrap()
    }

    #[test]
    fn generation_is_deterministic() {
        let reg = registry();
        let g = Generator::new(1234, &reg);
        let a = g.generate(ChunkPos::new(3, -5));
        let b = g.generate(ChunkPos::new(3, -5));
        assert_eq!(a.all_blocks(), b.all_blocks());
    }

    #[test]
    fn generates_land_and_bedrock() {
        let reg = registry();
        let g = Generator::new(99, &reg);
        let (sx, _, sz) = g.find_spawn();
        let c = g.generate(ChunkPos::new(sx >> 4, sz >> 4));
        assert_eq!(c.get(0, 0, 0), reg.id("bedrock").unwrap());
        assert!(c.top_y((sx & 15) as usize, (sz & 15) as usize) >= SEA_LEVEL);
    }

    #[test]
    fn surface_matches_generated_chunks() {
        let reg = registry();
        let g = Generator::new(4321, &reg);
        for (cx, cz) in [(0, 0), (40, -12), (-75, 33), (120, 120)] {
            let c = g.generate(ChunkPos::new(cx, cz));
            for (lx, lz) in [(0usize, 0usize), (5, 9), (15, 15)] {
                let s = g.surface(cx * 16 + lx as i32, cz * 16 + lz as i32);
                assert_eq!(c.get(lx, s.height as usize, lz), s.top, "top at chunk {cx},{cz}");
                if let Some(l) = s.liquid {
                    assert_eq!(c.get(lx, SEA_LEVEL as usize, lz), l);
                }
            }
        }
    }
}
