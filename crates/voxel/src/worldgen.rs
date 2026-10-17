//! World generation. Every chunk is a pure function of (seed, chunk position), so chunks can be
//! generated on any thread in any order. Pipeline: climate → height → surface → caves → ores → plants/trees.

use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos, CHUNK_HEIGHT};
use crate::noise::Perlin;
use crate::rng::Rng;
use crate::trees::{self, TreeBlocks, TreeKind, TreeSpace, Wood};

pub const SEA_LEVEL: i32 = 62;

mod caves;
mod end;
mod nether;
pub use caves::CaveBiome;
pub use end::{end_pillars, EndPillar, END_EXIT_Y, END_PLATFORM};
pub use nether::{NETHER_LAVA, NETHER_ROOF};

/// Which world a generator makes. Each dimension is saved in its own folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Dim {
    #[default]
    Overworld,
    Nether,
    End,
}

impl Dim {
    pub fn from_u8(v: u8) -> Dim {
        match v {
            1 => Dim::Nether,
            2 => Dim::End,
            _ => Dim::Overworld,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Sub-folder of the world's save for this dimension ("" for the overworld).
    pub fn folder(self) -> &'static str {
        match self {
            Dim::Overworld => "",
            Dim::Nether => "nether",
            Dim::End => "end",
        }
    }
}

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
    Swamp = 9,
    Jungle = 10,
    Savanna = 11,
    DarkForest = 12,
    Badlands = 13,
    CherryGrove = 14,
    NetherWastes = 15,
    SoulSandValley = 16,
    CrimsonForest = 17,
    TheEnd = 18,
}

impl Biome {
    pub const COUNT: usize = 19;

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
            9 => Biome::Swamp,
            10 => Biome::Jungle,
            11 => Biome::Savanna,
            12 => Biome::DarkForest,
            13 => Biome::Badlands,
            14 => Biome::CherryGrove,
            15 => Biome::NetherWastes,
            16 => Biome::SoulSandValley,
            17 => Biome::CrimsonForest,
            18 => Biome::TheEnd,
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
            Biome::Swamp => "Swamp",
            Biome::Jungle => "Jungle",
            Biome::Savanna => "Savanna",
            Biome::DarkForest => "Dark Forest",
            Biome::Badlands => "Badlands",
            Biome::CherryGrove => "Cherry Grove",
            Biome::NetherWastes => "Nether Wastes",
            Biome::SoulSandValley => "Soul Sand Valley",
            Biome::CrimsonForest => "Crimson Forest",
            Biome::TheEnd => "The End",
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
            Biome::Swamp => [0.74, 0.8, 0.58],
            Biome::Jungle => [0.82, 1.12, 0.66],
            Biome::Savanna => [1.16, 1.0, 0.6],
            Biome::DarkForest => [0.72, 0.86, 0.72],
            Biome::Badlands => [1.14, 0.94, 0.62],
            Biome::CherryGrove => [1.02, 1.08, 0.9],
            Biome::NetherWastes | Biome::SoulSandValley | Biome::CrimsonForest | Biome::TheEnd => [1.0, 1.0, 1.0],
        }
    }

    /// Water colour multiplier (RGB): murky green in swamps, plain elsewhere.
    pub fn water_tint(self) -> [f32; 3] {
        match self {
            Biome::Swamp => [0.52, 0.74, 0.4],
            Biome::Jungle => [0.88, 1.04, 0.96],
            _ => [1.0, 1.0, 1.0],
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
    oak_leaves: BlockId,
    birch_leaves: BlockId,
    spruce_leaves: BlockId,
    cactus: BlockId,
    short_grass: BlockId,
    poppy: BlockId,
    twig: BlockId,
    pebble: BlockId,
    dandelion: BlockId,
    dead_bush: BlockId,
    pumpkin: BlockId,
    melon: BlockId,
    red_sand: BlockId,
    terracotta: BlockId,
    lily_pad: BlockId,
    red_mushroom: BlockId,
    brown_mushroom: BlockId,
    cobblestone: BlockId,
    mossy_cobblestone: BlockId,
    /// (block, min y, max y, veins per chunk, vein size)
    ores: Vec<(BlockId, i32, i32, u32, u32)>,
    emerald: BlockId,
    life_crystal: BlockId,
}

/// The metal pairs of the ore ladder (Terraria's idea): each world makes one of each pair
/// common and the other rare, so worlds differ but every metal can still be found.
const ORE_PAIRS: [(&str, &str, i32, i32, u32, u32); 4] = [
    ("copper_ore", "tin_ore", 0, 96, 10, 9),
    ("iron_ore", "lead_ore", 0, 64, 14, 8),
    ("silver_ore", "tungsten_ore", 0, 40, 5, 7),
    ("gold_ore", "platinum_ore", 0, 32, 3, 7),
];

/// Which metal of pair `i` is the common one in this world.
pub fn common_metal(seed: u64, i: usize) -> bool {
    (seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (20 + i * 7)) & 1 == 0
}

impl Palette {
    fn new(r: &BlockRegistry, seed: u64) -> Self {
        let g = |n: &str| r.id_or_air(n);
        let mut ores = vec![(g("coal_ore"), 5, 128, 20, 12)];
        for (i, &(a, b, lo, hi, veins, size)) in ORE_PAIRS.iter().enumerate() {
            let (common, rare) = if common_metal(seed, i) { (a, b) } else { (b, a) };
            ores.push((g(common), lo, hi, veins, size));
            ores.push((g(rare), lo, hi, (veins / 4).max(1), size.saturating_sub(2).max(3)));
        }
        ores.extend([
            (g("redstone_ore"), 0, 16, 7, 7),
            (g("lapis_ore"), 0, 30, 2, 6),
            (g("diamond_ore"), 0, 16, 2, 6),
            // Gems: small and deep.
            (g("ruby_ore"), 0, 24, 1, 4),
            (g("sapphire_ore"), 0, 24, 1, 4),
            (g("topaz_ore"), 0, 32, 1, 4),
            (g("amethyst_ore"), 0, 32, 1, 4),
            (g("gravel"), 0, 128, 6, 24),
            (g("dirt"), 0, 128, 6, 24),
        ]);
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
            oak_leaves: g("oak_leaves"),
            birch_leaves: g("birch_leaves"),
            spruce_leaves: g("spruce_leaves"),
            cactus: g("cactus"),
            short_grass: g("short_grass"),
            poppy: g("poppy"),
            twig: g("twig"),
            pebble: g("pebble"),
            dandelion: g("dandelion"),
            dead_bush: g("dead_bush"),
            pumpkin: g("pumpkin"),
            melon: g("melon"),
            red_sand: g("red_sand"),
            terracotta: g("terracotta"),
            lily_pad: g("lily_pad"),
            red_mushroom: g("red_mushroom"),
            brown_mushroom: g("brown_mushroom"),
            cobblestone: g("cobblestone"),
            mossy_cobblestone: g("mossy_cobblestone"),
            ores,
            emerald: g("emerald_ore"),
            life_crystal: g("life_crystal"),
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
    /// Picks the rarer variant of a climate (badlands in deserts, dark forests, cherry groves).
    variant: Perlin,
    pal: Palette,
    structures: std::sync::Arc<crate::structures::Structures>,
    /// Percent of each building's spacing (Sparse Structures idea: more than 100 spreads them out).
    pub(crate) spread: i32,
    trees: TreeBlocks,
    /// Coloured terracotta layers of the badlands, repeating every `BANDS` blocks.
    bands: [BlockId; BANDS],
    pub dim: Dim,
    nether: nether::NetherPalette,
    end: end::EndPalette,
    caves: caves::CavePalette,
}

const BANDS: usize = 48;

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A chunk seen as a place to grow trees: writes outside it are dropped.
struct ChunkSpace<'a>(&'a mut Chunk);

impl TreeSpace for ChunkSpace<'_> {
    fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        if (0..16).contains(&x) && (0..16).contains(&z) && (0..CHUNK_HEIGHT as i32).contains(&y) {
            self.0.get(x as usize, y as usize, z as usize)
        } else {
            BlockId::MAX
        }
    }

    fn set(&mut self, x: i32, y: i32, z: i32, id: BlockId) {
        if (0..16).contains(&x) && (0..16).contains(&z) && (0..CHUNK_HEIGHT as i32).contains(&y) {
            self.0.set(x as usize, y as usize, z as usize, id);
        }
    }
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
        // The badlands' layers: mostly plain terracotta with runs of colour, fixed per seed.
        let g = |n: &str| reg.id_or_air(n);
        let colours = [
            g("terracotta"),
            g("orange_terracotta"),
            g("yellow_terracotta"),
            g("red_terracotta"),
            g("brown_terracotta"),
            g("white_terracotta"),
            g("light_gray_terracotta"),
        ];
        let mut bands = [colours[0]; BANDS];
        let mut r = Rng::new(seed ^ 0xBAD1_A4D5);
        let mut i = 0;
        while i < BANDS {
            let c = if r.chance(0.45) { colours[0] } else { colours[1 + r.below(6) as usize] };
            let run = r.range(1, 3) as usize;
            for b in bands.iter_mut().skip(i).take(run) {
                *b = if c == AIR { colours[0] } else { c };
            }
            i += run;
        }
        Generator {
            structures: std::sync::Arc::new(crate::structures::Structures::empty()),
            spread: 100,
            seed,
            continent: Perlin::new(seed ^ 0x01),
            mountain: Perlin::new(seed ^ 0x02),
            detail: Perlin::new(seed ^ 0x03),
            temperature: Perlin::new(seed ^ 0x04),
            humidity: Perlin::new(seed ^ 0x05),
            cave_a: Perlin::new(seed ^ 0x06),
            cave_b: Perlin::new(seed ^ 0x07),
            cheese: Perlin::new(seed ^ 0x08),
            variant: Perlin::new(seed ^ 0x09),
            pal: Palette::new(reg, seed),
            trees: TreeBlocks::new(reg),
            bands,
            dim: Dim::Overworld,
            nether: nether::NetherPalette::new(reg),
            end: end::EndPalette::new(reg),
            caves: caves::CavePalette::new(reg),
        }
    }

    fn column(&self, wx: i32, wz: i32) -> Column {
        if self.dim == Dim::Nether {
            return Column { height: NETHER_ROOF, biome: self.nether_biome(wx, wz) };
        }
        if self.dim == Dim::End {
            return Column { height: self.end_land(wx, wz).map(|l| l.0).unwrap_or(0), biome: Biome::TheEnd };
        }
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

        let climate = self.temperature.fbm2(x / 1100.0, z / 1100.0, 3);
        let hum = self.humidity.fbm2(x / 900.0 + 300.0, z / 900.0, 3);
        let var = self.variant.fbm2(x / 520.0 - 70.0, z / 520.0, 2);
        let sea = SEA_LEVEL as f32;
        // Badlands: the driest corner of the deserts rises in flat-topped terraces.
        let bad = smoothstep(0.27, 0.35, climate) * smoothstep(0.06, -0.04, hum) * smoothstep(0.0, 0.12, var);
        if bad > 0.0 && h > sea + 1.0 {
            let land = ((h - sea - 1.0) / 6.0).min(1.0);
            let m = self.detail.fbm2(x / 110.0 + 40.0, z / 110.0 - 40.0, 2);
            let raise = bad * land * (6.0 + 34.0 * m.max(0.0));
            // Flat-topped terraces with steep risers, so the cliffs show their layers.
            let t = h + raise;
            let k = (t / 7.0).floor();
            let terraced = (k + (t / 7.0 - k).powi(16)) * 7.0;
            h = (t + (terraced - t) * smoothstep(0.2, 0.6, bad)).max(sea + 2.0);
        }
        // Swamps: wet, mild lowlands flatten out around sea level into pools and mud flats.
        let swamp = smoothstep(0.26, 0.36, hum) * smoothstep(-0.2, -0.1, climate) * smoothstep(0.3, 0.2, climate);
        if swamp > 0.0 {
            let (lo, hi) = (sea - 8.0, sea + 16.0);
            if h > lo && h < hi {
                let edge = ((h - lo) / 4.0).min((hi - h) / 6.0).min(1.0);
                let target = sea - 0.3 + detail * 3.0;
                h += (target - h) * swamp * edge;
            }
        }
        let height = (h as i32).clamp(4, CHUNK_HEIGHT as i32 - 20);

        let temp = climate - (height - 90).max(0) as f32 / 120.0;
        let biome = if swamp > 0.5 && (SEA_LEVEL - 5..=SEA_LEVEL + 4).contains(&height) {
            Biome::Swamp
        } else if height < SEA_LEVEL - 2 {
            Biome::Ocean
        } else if height <= SEA_LEVEL + 1 && temp > -0.25 && swamp < 0.25 {
            Biome::Beach
        } else if bad > 0.5 {
            Biome::Badlands
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
        } else if temp > 0.22 && (hum > 0.2 || temp > 0.26) {
            if hum > 0.2 {
                Biome::Jungle
            } else {
                Biome::Savanna
            }
        } else if hum > 0.22 {
            if var > 0.12 {
                Biome::DarkForest
            } else {
                Biome::Forest
            }
        } else if var < -0.36 && temp > -0.2 {
            Biome::CherryGrove
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
            // Red sand over deep layers of terracotta (the layers are picked in `generate`).
            Biome::Badlands if p.red_sand != AIR => (p.red_sand, p.terracotta, (h - SEA_LEVEL + 14).max(3)),
            Biome::Badlands => (p.sand, p.sandstone, 4),
            _ => (p.grass, p.dirt, 3),
        }
    }

    /// The surface of one column, cheaply: no caves, ores or chunk. Matches `generate` except
    /// for trees, which are only a density here.
    pub fn surface(&self, wx: i32, wz: i32) -> Surface {
        let p = &self.pal;
        let col = self.column(wx, wz);
        if self.dim == Dim::Nether {
            return Surface { height: NETHER_ROOF, biome: col.biome, top: p.bedrock, liquid: None, trees: None };
        }
        if self.dim == Dim::End {
            return Surface { height: col.height, biome: col.biome, top: self.end.end_stone, liquid: None, trees: None };
        }
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
                Biome::Jungle => Some((self.trees.leaves(Wood::Jungle), 1.0)),
                Biome::DarkForest => Some((self.trees.leaves(Wood::DarkOak), 1.0)),
                Biome::Savanna => Some((self.trees.leaves(Wood::Acacia), 0.25)),
                Biome::CherryGrove => Some((self.trees.leaves(Wood::Cherry), 0.5)),
                Biome::Swamp => Some((p.oak_leaves, 0.4)),
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
        if self.dim == Dim::Nether {
            return self.generate_nether(pos);
        }
        if self.dim == Dim::End {
            return self.generate_end(pos);
        }
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
                let banded = filler == p.terracotta && filler != AIR;
                // Badlands layers wobble a little so they don't look ruled.
                let wob = if banded { (self.mountain.noise2(wx as f32 / 60.0, wz as f32 / 60.0) * 3.0) as i32 } else { 0 };
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
                        if banded {
                            self.bands[(y + wob).rem_euclid(BANDS as i32) as usize]
                        } else {
                            filler
                        }
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
        self.decorate_caves(&mut chunk, &mut rng, &cols, pos);
        self.decorate(&mut chunk, &mut rng, &cols, pos);
        self.place_structures(&mut chunk);
        chunk
    }

    /// Buildings from data (see `structures`).
    pub fn with_structures(mut self, s: std::sync::Arc<crate::structures::Structures>) -> Self {
        self.structures = s;
        self
    }

    pub fn with_spread(mut self, percent: i32) -> Self {
        self.spread = percent.max(50);
        self
    }

    pub fn with_dim(mut self, dim: Dim) -> Self {
        self.dim = dim;
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
        // Life crystals: now and then on a cave floor deep down.
        if p.life_crystal != AIR && rng.below(5) == 0 {
            let (x, z) = (rng.range(0, 15) as usize, rng.range(0, 15) as usize);
            for y in 6..40 {
                if chunk.get(x, y, z) == AIR && chunk.get(x, y - 1, z) == p.stone && chunk.get(x, y + 1, z) == AIR {
                    chunk.set(x, y, z, p.life_crystal);
                    break;
                }
            }
        }
    }

    fn decorate(&self, chunk: &mut Chunk, rng: &mut Rng, cols: &[Column], pos: ChunkPos) {
        let p = &self.pal;
        // Trees stay inside the chunk (see `TreeKind::reach`) so generation never writes to neighbors.
        let tree_attempts = match cols[8 + 8 * 16].biome {
            Biome::Forest | Biome::BirchForest | Biome::Taiga => 9,
            Biome::Plains | Biome::Snowy => 1,
            Biome::Mountains => 1,
            Biome::Desert => 3,
            Biome::Jungle => 14,
            Biome::DarkForest => 16,
            Biome::Swamp => 4,
            Biome::Savanna => 2,
            Biome::CherryGrove => 5,
            Biome::Badlands => 2,
            _ => 0,
        };
        for _ in 0..tree_attempts {
            let x = rng.range(2, 13) as usize;
            let z = rng.range(2, 13) as usize;
            let col = &cols[x + z * 16];
            let y = col.height;
            // Swamp trees may stand in a block of water.
            let wading = col.biome == Biome::Swamp && y == SEA_LEVEL - 1;
            if (y < SEA_LEVEL && !wading) || y > 200 {
                continue;
            }
            let ground = chunk.get(x, y as usize, z);
            if matches!(col.biome, Biome::Desert | Biome::Badlands) {
                if (ground == p.sand || ground == p.red_sand) && ground != AIR && rng.chance(0.6) {
                    let hgt = rng.range(1, 3);
                    for i in 1..=hgt {
                        chunk.set(x, (y + i) as usize, z, p.cactus);
                    }
                }
                continue;
            }
            if ground != p.grass && ground != p.snow_grass && !(wading && ground == p.dirt) {
                continue;
            }
            let kind = match col.biome {
                Biome::Taiga | Biome::Snowy | Biome::Mountains => TreeKind::Spruce,
                Biome::BirchForest => {
                    if rng.chance(0.8) {
                        TreeKind::Round(Wood::Birch)
                    } else {
                        TreeKind::Round(Wood::Oak)
                    }
                }
                Biome::Forest => {
                    if rng.chance(0.2) {
                        TreeKind::Round(Wood::Birch)
                    } else {
                        TreeKind::Round(Wood::Oak)
                    }
                }
                Biome::Jungle => {
                    let r = rng.f32();
                    if r < 0.18 {
                        TreeKind::MegaJungle
                    } else if r < 0.6 {
                        TreeKind::Jungle
                    } else {
                        TreeKind::Bush(Wood::Jungle)
                    }
                }
                Biome::DarkForest => {
                    if rng.chance(0.8) {
                        TreeKind::DarkOak
                    } else {
                        TreeKind::Round(Wood::Oak)
                    }
                }
                Biome::Swamp => TreeKind::SwampOak,
                Biome::Savanna => {
                    if rng.chance(0.8) {
                        TreeKind::Acacia
                    } else {
                        TreeKind::Round(Wood::Oak)
                    }
                }
                Biome::CherryGrove => {
                    if rng.chance(0.85) {
                        TreeKind::Cherry
                    } else {
                        TreeKind::Bush(Wood::Oak)
                    }
                }
                _ => TreeKind::Round(Wood::Oak),
            };
            // Wide trees are pulled in from the chunk edge; the old trees never move (reach 2).
            let m = kind.reach();
            let far = 15 - m - kind.wide_trunk() as i32;
            let (tx, tz) = ((x as i32).clamp(m, far), (z as i32).clamp(m, far));
            if (tx, tz) != (x as i32, z as i32) {
                let c2 = &cols[tx as usize + tz as usize * 16];
                let g2 = chunk.get(tx as usize, c2.height as usize, tz as usize);
                if c2.height != y || (g2 != p.grass && !(wading && g2 == p.dirt)) {
                    continue;
                }
            }
            trees::grow(kind, &self.trees, &mut ChunkSpace(chunk), rng, tx, y, tz);
        }

        self.scatter(chunk, cols, pos);

        // Ground plants.
        for z in 0..16usize {
            for x in 0..16usize {
                let col = &cols[x + z * 16];
                let y = col.height;
                if col.biome == Biome::Swamp && y < SEA_LEVEL {
                    // Lily pads on the shallow pools.
                    let s = SEA_LEVEL as usize;
                    if y >= SEA_LEVEL - 3 && p.lily_pad != AIR && chunk.get(x, s, z) == p.water && chunk.get(x, s + 1, z) == AIR && rng.chance(0.06) {
                        chunk.set(x, s + 1, z, p.lily_pad);
                    }
                    continue;
                }
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
                        Biome::Savanna => 0.42,
                        Biome::Jungle => 0.32,
                        Biome::CherryGrove => 0.2,
                        Biome::Swamp => 0.14,
                        Biome::DarkForest => 0.08,
                        _ => 0.06,
                    };
                    let flower_p = if col.biome == Biome::CherryGrove { 0.05 } else { 0.012 };
                    if r < flower_p {
                        chunk.set(x, above, z, if rng.chance(0.5) { p.poppy } else { p.dandelion });
                    } else if r < flower_p + grass_p {
                        chunk.set(x, above, z, p.short_grass);
                    } else if r < 0.0125 + grass_p && col.biome == Biome::Plains {
                        chunk.set(x, above, z, if rng.chance(0.5) { p.pumpkin } else { p.melon });
                    } else if r > 0.995 && col.biome == Biome::Jungle {
                        chunk.set(x, above, z, p.melon);
                    } else if r > 0.993 && matches!(col.biome, Biome::Forest | Biome::BirchForest | Biome::DarkForest | Biome::Taiga | Biome::Plains) {
                        // Twigs and pebbles lying about (Twigs idea).
                        let b = if r > 0.9965 { p.pebble } else { p.twig };
                        if b != AIR {
                            chunk.set(x, above, z, b);
                        }
                    } else if r > 0.97 && matches!(col.biome, Biome::DarkForest | Biome::Swamp) {
                        let m = if rng.chance(0.5) { p.red_mushroom } else { p.brown_mushroom };
                        if m != AIR {
                            chunk.set(x, above, z, m);
                        }
                    }
                } else if (ground == p.sand && col.biome == Biome::Desert && rng.chance(0.01)) || (ground == p.red_sand && ground != AIR && rng.chance(0.012)) {
                    chunk.set(x, above, z, p.dead_bush);
                } else if ground == p.snow_grass && chunk.get(x, above, z) == AIR && p.snow_layer != AIR {
                    // A blanket of snow, a little deeper here and there.
                    let deep = rng.chance(0.15);
                    chunk.set(x, above, z, if deep { p.snow_layer2 } else { p.snow_layer });
                }
            }
        }
    }

    /// Small extras on their own random stream, so the older trees and plants keep their places:
    /// mossy boulders and bushes (ideas from Distant Lands and Ecologics).
    fn scatter(&self, chunk: &mut Chunk, cols: &[Column], pos: ChunkPos) {
        let p = &self.pal;
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 2);
        let biome = cols[8 + 8 * 16].biome;
        let (boulder_p, mossy) = match biome {
            Biome::Taiga => (0.3, 0.6),
            Biome::Mountains => (0.25, 0.15),
            Biome::Snowy => (0.1, 0.0),
            Biome::Forest | Biome::BirchForest => (0.1, 0.7),
            Biome::DarkForest => (0.2, 0.9),
            Biome::Plains | Biome::Savanna => (0.05, 0.2),
            Biome::Swamp | Biome::Jungle => (0.05, 0.95),
            _ => (0.0, 0.0),
        };
        if p.cobblestone != AIR && rng.chance(boulder_p) {
            let (x, z) = (rng.range(2, 13), rng.range(2, 13));
            self.boulder(chunk, &mut rng, cols, x, z, mossy);
        }
        let (bushes, wood) = match biome {
            Biome::Plains => (if rng.chance(0.4) { 1 } else { 0 }, Wood::Oak),
            Biome::Forest => (2, Wood::Oak),
            Biome::BirchForest => (1, Wood::Birch),
            Biome::Taiga => (1, Wood::Spruce),
            Biome::Savanna => (2, Wood::Acacia),
            Biome::CherryGrove => (1, Wood::Oak),
            Biome::DarkForest => (1, Wood::DarkOak),
            _ => (0, Wood::Oak),
        };
        if !self.trees.has(wood) {
            return;
        }
        for _ in 0..bushes {
            let (x, z) = (rng.range(2, 13), rng.range(2, 13));
            let col = &cols[(x + z * 16) as usize];
            let y = col.height;
            if !(SEA_LEVEL..=200).contains(&y) {
                continue;
            }
            let ground = chunk.get(x as usize, y as usize, z as usize);
            if (ground == p.grass || ground == p.snow_grass) && chunk.get(x as usize, y as usize + 1, z as usize) == AIR {
                trees::grow(TreeKind::Bush(wood), &self.trees, &mut ChunkSpace(chunk), &mut rng, x, y, z);
            }
        }
    }

    /// A lump of (mossy) cobblestone that hugs the ground.
    fn boulder(&self, chunk: &mut Chunk, rng: &mut Rng, cols: &[Column], x: i32, z: i32, mossy: f32) {
        let p = &self.pal;
        let base = cols[(x + z * 16) as usize].height;
        if !(SEA_LEVEL..=200).contains(&base) {
            return;
        }
        let r = 1.3 + rng.f32() * 0.9;
        for dz in -2..=2i32 {
            for dx in -2..=2i32 {
                let (px, pz) = (x + dx, z + dz);
                let d2 = (dx * dx + dz * dz) as f32 + rng.f32() * 0.8;
                if d2 > r * r {
                    continue;
                }
                let top = base + ((r * r - d2).sqrt() * 1.1).round() as i32;
                let colh = cols[(px + pz * 16) as usize].height;
                if colh < SEA_LEVEL {
                    continue;
                }
                for y in (colh + 1)..=top {
                    let (ux, uy, uz) = (px as usize, y as usize, pz as usize);
                    if chunk.get(ux, uy, uz) != AIR {
                        continue;
                    }
                    let b = if p.mossy_cobblestone != AIR && rng.chance(mossy) { p.mossy_cobblestone } else if rng.chance(0.25) { p.stone } else { p.cobblestone };
                    chunk.set(ux, uy, uz, b);
                }
            }
        }
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

    #[test]
    fn nether_is_closed_with_a_lava_sea() {
        let reg = registry();
        let g = Generator::new(99, &reg).with_dim(Dim::Nether);
        let (bedrock, lava) = (reg.id("bedrock").unwrap(), reg.id("lava").unwrap());
        let (mut seas, mut open) = (0, 0);
        for cx in 0..4 {
            let c = g.generate(ChunkPos::new(cx, 0));
            for x in 0..16 {
                assert_eq!(c.get(x, 0, 3), bedrock);
                assert_eq!(c.get(x, NETHER_ROOF as usize, 3), bedrock);
                assert_eq!(c.get(x, NETHER_ROOF as usize + 1, 3), AIR, "nothing above the roof");
                seas += (c.get(x, NETHER_LAVA as usize, 3) == lava) as usize;
                open += (40..100).filter(|&y| c.get(x, y, 3) == AIR).count();
            }
        }
        assert!(seas > 0, "no lava sea");
        assert!(open > 64 * 10, "no caverns");
    }

    #[test]
    fn end_has_an_island_pillars_and_a_void() {
        let reg = registry();
        let g = Generator::new(5, &reg).with_dim(Dim::End);
        let (stone, obsidian, crystal) = (reg.id("end_stone").unwrap(), reg.id("obsidian").unwrap(), reg.id("end_crystal").unwrap());
        let c = g.generate(ChunkPos::new(1, 1));
        assert!((0..16).any(|x| (40..80).any(|y| c.get(x, y, 8) == stone)), "no island near the middle");
        let p = end_pillars(5)[0];
        let pc = g.generate(ChunkPos::new(p.x.div_euclid(16), p.z.div_euclid(16)));
        let (lx, lz) = (p.x.rem_euclid(16) as usize, p.z.rem_euclid(16) as usize);
        assert_eq!(pc.get(lx, p.top as usize, lz), obsidian);
        assert_eq!(pc.get(lx, p.top as usize + 2, lz), crystal);
        let void = g.generate(ChunkPos::new(20, 0));
        assert!((0..256).all(|y| void.get(3, y, 3) == AIR), "the gap past the main island is empty");
    }

    #[test]
    fn every_biome_appears() {
        let reg = registry();
        let g = Generator::new(1234, &reg);
        let mut count = [0usize; Biome::COUNT];
        for z in (-12000..12000).step_by(64) {
            for x in (-12000..12000).step_by(64) {
                count[g.biome_at(x, z) as usize] += 1;
            }
        }
        // The Nether's regions come from the Nether's own generator.
        let nether = Generator::new(1234, &reg).with_dim(Dim::Nether);
        for z in (-3000..3000).step_by(64) {
            for x in (-3000..3000).step_by(64) {
                count[nether.biome_at(x, z) as usize] += 1;
            }
        }
        count[Generator::new(1234, &reg).with_dim(Dim::End).biome_at(0, 0) as usize] += 1;
        let total: usize = count.iter().sum();
        for (i, &c) in count.iter().enumerate() {
            println!("{:>14}: {:.1}%", Biome::from_u8(i as u8).name(), c as f32 * 100.0 / total as f32);
            assert!(c > 0, "{} never appears", Biome::from_u8(i as u8).name());
        }
    }

    #[test]
    fn new_biomes_grow_their_trees() {
        let reg = registry();
        let g = Generator::new(77, &reg);
        for (biome, log) in [(Biome::Jungle, "jungle_log"), (Biome::Savanna, "acacia_log"), (Biome::DarkForest, "dark_oak_log"), (Biome::CherryGrove, "cherry_log")] {
            let log = reg.id(log).unwrap();
            let mut found = false;
            'search: for cz in -100..100 {
                for cx in -100..100 {
                    let (x, z) = (cx * 112 + 8, cz * 112 + 8);
                    if g.biome_at(x, z) != biome {
                        continue;
                    }
                    let c = g.generate(ChunkPos::new(x >> 4, z >> 4));
                    if c.all_blocks().contains(&log) {
                        found = true;
                        break 'search;
                    }
                }
            }
            assert!(found, "no {} trees", biome.name());
        }
    }

    /// Prints where each biome can be found (for screenshots):
    /// `SEED=1 cargo test -p voxel --release -- --ignored --nocapture print_biome`.
    #[test]
    #[ignore]
    fn print_biome_locations() {
        let reg = registry();
        let seed = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1234);
        let g = Generator::new(seed, &reg);
        let mut best: Vec<Option<(i32, i32, i32)>> = vec![None; Biome::COUNT];
        for r in (0..8000).step_by(48) {
            for (x, z) in [(r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, r), (r, -r), (-r, -r)] {
                let b = g.biome_at(x, z) as usize;
                // Prefer the middle of a patch: the same biome 40 blocks around.
                let solid = [(40, 0), (-40, 0), (0, 40), (0, -40)].iter().all(|(dx, dz)| g.biome_at(x + dx, z + dz) as usize == b);
                if best[b].is_none() && solid {
                    best[b] = Some((x, g.height_at(x, z), z));
                }
            }
        }
        for (i, b) in best.iter().enumerate() {
            println!("{:>14}: {:?}", Biome::from_u8(i as u8).name(), b);
        }
    }
}
