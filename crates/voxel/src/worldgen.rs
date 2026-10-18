//! World generation. Every chunk is a pure function of (seed, chunk position), so chunks can be
//! generated on any thread in any order. Pipeline: climate → height → surface → caves → ores → plants/trees.

use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos, CHUNK_HEIGHT};
use crate::noise::Perlin;
use crate::rng::Rng;
use crate::trees::{self, TreeBlocks, TreeKind, TreeSpace, Wood};

pub const SEA_LEVEL: i32 = 62;

mod aether;
mod caves;
mod distant_lands;
mod ecologics;
mod end;
mod nether;
mod otherside;
mod variants;
pub use caves::CaveBiome;
pub use distant_lands::is_distant;
pub use end::{end_pillars, EndPillar, END_EXIT_Y, END_PLATFORM};
pub use aether::AETHER_ARRIVAL_Y;
pub use otherside::OTHERSIDE_ROOF;
pub use nether::{NETHER_LAVA, NETHER_ROOF};

/// Which world a generator makes. Each dimension is saved in its own folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Dim {
    #[default]
    Overworld,
    Nether,
    End,
    /// The mod pack's sky islands (worldgen/aether.rs).
    Aether,
    /// The mod pack's dark world under the Deep Dark (worldgen/otherside.rs).
    Otherside,
}

impl Dim {
    pub fn from_u8(v: u8) -> Dim {
        match v {
            1 => Dim::Nether,
            2 => Dim::End,
            3 => Dim::Aether,
            4 => Dim::Otherside,
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
            Dim::Aether => "aether",
            Dim::Otherside => "otherside",
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
    WarpedForest = 19,
    River = 20,
    MushroomFields = 21,
    Meadow = 22,
    SnowyPeaks = 23,
    FrozenOcean = 24,
    BasaltDeltas = 25,
    EndHighlands = 26,
    MangroveSwamp = 27,
    // Variants of the biomes above (Lote 4): each looks and grows its own way but otherwise
    // behaves as its `base()` (mobs, weather, sounds, structures).
    WarmOcean = 28,
    LukewarmOcean = 29,
    ColdOcean = 30,
    DeepOcean = 31,
    FrozenRiver = 32,
    SnowyBeach = 33,
    StonyShore = 34,
    SunflowerPlains = 35,
    FlowerForest = 36,
    OldGrowthTaiga = 37,
    SnowyTaiga = 38,
    WindsweptHills = 39,
    // Biomes of the mod content pack (crates/voxel/src/worldgen/moddims.rs and Distant Lands).
    Aether = 40,
    GoldenGrove = 41,
    Deeplands = 42,
    EchoingForest = 43,
    OvercastColumns = 44,
    BloomingCaverns = 45,
    MataAtlantica = 46,
    Cerrado = 47,
    Pantanal = 48,
    Araucaria = 49,
    Caatinga = 50,
    JaggedPeaks = 51,
    Grove = 52,
    SnowySlopes = 53,
    IceSpikes = 54,
    BambooJungle = 55,
    SparseJungle = 56,
    WoodedBadlands = 57,
    ErodedBadlands = 58,
    WindsweptSavanna = 59,
    PaleGarden = 60,
}

impl Biome {
    pub const COUNT: usize = 61;

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
            19 => Biome::WarpedForest,
            20 => Biome::River,
            21 => Biome::MushroomFields,
            22 => Biome::Meadow,
            23 => Biome::SnowyPeaks,
            24 => Biome::FrozenOcean,
            25 => Biome::BasaltDeltas,
            26 => Biome::EndHighlands,
            27 => Biome::MangroveSwamp,
            28 => Biome::WarmOcean,
            29 => Biome::LukewarmOcean,
            30 => Biome::ColdOcean,
            31 => Biome::DeepOcean,
            32 => Biome::FrozenRiver,
            33 => Biome::SnowyBeach,
            34 => Biome::StonyShore,
            35 => Biome::SunflowerPlains,
            36 => Biome::FlowerForest,
            37 => Biome::OldGrowthTaiga,
            38 => Biome::SnowyTaiga,
            39 => Biome::WindsweptHills,
            40 => Biome::Aether,
            41 => Biome::GoldenGrove,
            42 => Biome::Deeplands,
            43 => Biome::EchoingForest,
            44 => Biome::OvercastColumns,
            45 => Biome::BloomingCaverns,
            46 => Biome::MataAtlantica,
            47 => Biome::Cerrado,
            48 => Biome::Pantanal,
            49 => Biome::Araucaria,
            50 => Biome::Caatinga,
            51 => Biome::JaggedPeaks,
            52 => Biome::Grove,
            53 => Biome::SnowySlopes,
            54 => Biome::IceSpikes,
            55 => Biome::BambooJungle,
            56 => Biome::SparseJungle,
            57 => Biome::WoodedBadlands,
            58 => Biome::ErodedBadlands,
            59 => Biome::WindsweptSavanna,
            60 => Biome::PaleGarden,
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
            Biome::WarpedForest => "Warped Forest",
            Biome::River => "River",
            Biome::MushroomFields => "Mushroom Fields",
            Biome::Meadow => "Meadow",
            Biome::SnowyPeaks => "Snowy Peaks",
            Biome::FrozenOcean => "Frozen Ocean",
            Biome::MangroveSwamp => "Mangrove Swamp",
            Biome::BasaltDeltas => "Basalt Deltas",
            Biome::EndHighlands => "End Highlands",
            Biome::Aether => "Aether",
            Biome::GoldenGrove => "Golden Grove",
            Biome::Deeplands => "Deeplands",
            Biome::EchoingForest => "Echoing Forest",
            Biome::OvercastColumns => "Overcast Columns",
            Biome::BloomingCaverns => "Blooming Caverns",
            Biome::MataAtlantica => "Mata Atlântica",
            Biome::Cerrado => "Cerrado",
            Biome::Pantanal => "Pantanal",
            Biome::Araucaria => "Mata de Araucárias",
            Biome::Caatinga => "Caatinga",
            Biome::WarmOcean => "Warm Ocean",
            Biome::LukewarmOcean => "Lukewarm Ocean",
            Biome::ColdOcean => "Cold Ocean",
            Biome::DeepOcean => "Deep Ocean",
            Biome::FrozenRiver => "Frozen River",
            Biome::SnowyBeach => "Snowy Beach",
            Biome::StonyShore => "Stony Shore",
            Biome::SunflowerPlains => "Sunflower Plains",
            Biome::FlowerForest => "Flower Forest",
            Biome::OldGrowthTaiga => "Old Growth Taiga",
            Biome::SnowyTaiga => "Snowy Taiga",
            Biome::WindsweptHills => "Windswept Hills",
            Biome::JaggedPeaks => "Jagged Peaks",
            Biome::Grove => "Grove",
            Biome::SnowySlopes => "Snowy Slopes",
            Biome::IceSpikes => "Ice Spikes",
            Biome::BambooJungle => "Bamboo Jungle",
            Biome::SparseJungle => "Sparse Jungle",
            Biome::WoodedBadlands => "Wooded Badlands",
            Biome::ErodedBadlands => "Eroded Badlands",
            Biome::WindsweptSavanna => "Windswept Savanna",
            Biome::PaleGarden => "Pale Garden",
        }
    }

    /// The biome a variant behaves as everywhere but its own ground, plants and colours.
    pub fn base(self) -> Biome {
        match self {
            Biome::WarmOcean | Biome::LukewarmOcean | Biome::ColdOcean | Biome::DeepOcean => Biome::Ocean,
            Biome::FrozenRiver => Biome::River,
            Biome::SnowyBeach | Biome::StonyShore => Biome::Beach,
            Biome::SunflowerPlains => Biome::Plains,
            Biome::FlowerForest => Biome::Forest,
            Biome::OldGrowthTaiga | Biome::SnowyTaiga | Biome::Grove => Biome::Taiga,
            Biome::WindsweptHills => Biome::Mountains,
            Biome::JaggedPeaks => Biome::SnowyPeaks,
            Biome::SnowySlopes | Biome::IceSpikes => Biome::Snowy,
            Biome::BambooJungle | Biome::SparseJungle => Biome::Jungle,
            Biome::WoodedBadlands | Biome::ErodedBadlands => Biome::Badlands,
            Biome::WindsweptSavanna => Biome::Savanna,
            Biome::PaleGarden => Biome::DarkForest,
            b => b,
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
            Biome::NetherWastes | Biome::SoulSandValley | Biome::CrimsonForest | Biome::WarpedForest | Biome::BasaltDeltas | Biome::TheEnd | Biome::EndHighlands => [1.0, 1.0, 1.0],
            Biome::River => [0.92, 1.0, 0.9],
            Biome::MushroomFields => [0.95, 1.05, 0.85],
            Biome::Meadow => [0.9, 1.06, 0.82],
            Biome::SnowyPeaks | Biome::FrozenOcean => [0.85, 0.95, 1.0],
            Biome::MangroveSwamp => [0.8, 0.95, 0.62],
            Biome::Aether => [0.9, 1.08, 0.95],
            Biome::GoldenGrove => [1.18, 1.08, 0.62],
            Biome::Deeplands => [0.62, 0.7, 0.78],
            Biome::EchoingForest => [0.55, 0.85, 0.9],
            Biome::OvercastColumns => [0.75, 0.78, 0.82],
            Biome::BloomingCaverns => [0.85, 0.7, 1.0],
            Biome::MataAtlantica => [0.74, 1.1, 0.62],
            Biome::Cerrado => [1.12, 1.0, 0.62],
            Biome::Pantanal => [0.8, 0.98, 0.66],
            Biome::Araucaria => [0.78, 0.94, 0.8],
            Biome::Caatinga => [1.14, 0.96, 0.66],
            Biome::WarmOcean => [0.95, 1.0, 0.95],
            Biome::LukewarmOcean => [0.95, 1.0, 0.95],
            Biome::ColdOcean => [0.9, 0.98, 1.0],
            Biome::DeepOcean => [0.95, 1.0, 0.95],
            Biome::FrozenRiver => [0.85, 0.95, 1.0],
            Biome::SnowyBeach => [0.85, 0.95, 1.0],
            Biome::StonyShore => [0.9, 0.98, 0.95],
            Biome::SunflowerPlains => [1.02, 1.03, 0.92],
            Biome::FlowerForest => [0.92, 1.04, 0.86],
            Biome::OldGrowthTaiga => [0.82, 0.92, 0.78],
            Biome::SnowyTaiga => [0.8, 0.92, 0.95],
            Biome::WindsweptHills => [0.84, 0.94, 0.9],
            Biome::JaggedPeaks => [0.85, 0.95, 1.0],
            Biome::Grove => [0.82, 0.94, 0.92],
            Biome::SnowySlopes => [0.85, 0.95, 1.0],
            Biome::IceSpikes => [0.85, 0.95, 1.0],
            Biome::BambooJungle => [0.8, 1.14, 0.62],
            Biome::SparseJungle => [0.9, 1.08, 0.7],
            Biome::WoodedBadlands => [1.06, 0.98, 0.66],
            Biome::ErodedBadlands => [1.14, 0.94, 0.62],
            Biome::WindsweptSavanna => [1.12, 1.0, 0.64],
            Biome::PaleGarden => [0.84, 0.86, 0.82],
        }
    }

    /// Water colour multiplier (RGB): murky green in swamps, plain elsewhere.
    pub fn water_tint(self) -> [f32; 3] {
        match self {
            Biome::Swamp => [0.52, 0.74, 0.4],
            Biome::MangroveSwamp => [0.5, 0.78, 0.62],
            Biome::Jungle | Biome::BambooJungle | Biome::SparseJungle => [0.88, 1.04, 0.96],
            Biome::WarmOcean => [0.62, 1.1, 1.06],
            Biome::LukewarmOcean => [0.8, 1.06, 1.04],
            Biome::ColdOcean | Biome::FrozenRiver => [0.84, 0.88, 1.06],
            Biome::DeepOcean => [0.86, 0.92, 1.0],
            Biome::PaleGarden => [0.78, 0.8, 0.8],
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
    meadow_flowers: Vec<BlockId>,
    mycelium: BlockId,
    snow_block: BlockId,
    packed_ice: BlockId,
    mushroom_stem: BlockId,
    red_mushroom_block: BlockId,
    brown_mushroom_block: BlockId,
    twig: BlockId,
    pebble: BlockId,
    dandelion: BlockId,
    dead_bush: BlockId,
    pumpkin: BlockId,
    melon: BlockId,
    red_sand: BlockId,
    terracotta: BlockId,
    lily_pad: BlockId,
    mud: BlockId,
    sugar_cane: BlockId,
    red_mushroom: BlockId,
    brown_mushroom: BlockId,
    cobblestone: BlockId,
    mossy_cobblestone: BlockId,
    /// (block, min y, max y, veins per chunk, vein size)
    ores: Vec<(BlockId, i32, i32, u32, u32)>,
    emerald: BlockId,
    life_crystal: BlockId,
    /// Geode shells from outside in: basalt, calcite, amethyst; then budding amethyst and its clusters.
    geode: [BlockId; 5],
    granite: BlockId,
    tuff: BlockId,
    bee_nest: BlockId,
    /// Big ore veins: (ore, raw block) for copper and iron.
    vein_copper: (BlockId, BlockId),
    vein_iron: (BlockId, BlockId),
    /// The deepslate floor of the world, and the ores that change with it.
    deepslate: BlockId,
    deep_ores: Vec<(BlockId, BlockId)>,
    /// Vanilla plants and grounds added later (kelp, bamboo, berry bushes, podzol...).
    kelp: BlockId,
    bamboo: BlockId,
    berry_bush: BlockId,
    podzol: BlockId,
    coarse_dirt: BlockId,
    red_sandstone: BlockId,
    /// Lote 4's biome variants: sunflowers, the pale garden, sea grass and coral reefs.
    sunflower: BlockId,
    pale_moss: BlockId,
    pale_moss_carpet: BlockId,
    pale_hanging_moss: BlockId,
    seagrass: BlockId,
    corals: [BlockId; 5],
    coral_blocks: [BlockId; 5],
    blue_ice: BlockId,
}

/// The five corals of the warm reefs.
const CORALS: [&str; 5] = ["tube", "brain", "bubble", "fire", "horn"];

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
            // Stones of their own colour in blobs.
            (g("granite"), 0, 128, 3, 32),
            (g("diorite"), 0, 128, 3, 32),
            (g("andesite"), 0, 128, 3, 32),
            (g("tuff"), 1, 24, 2, 32),
            (g("gravel"), 0, 128, 6, 24),
            (g("dirt"), 0, 128, 6, 24),
            // Silverfish hiding in the stone (vanilla puts them under mountains).
            (g("infested_stone"), 0, 64, 2, 6),
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
            meadow_flowers: ["cornflower", "allium", "azure_bluet", "oxeye_daisy", "red_tulip", "orange_tulip", "white_tulip", "pink_tulip", "lily_of_the_valley", "poppy", "dandelion"]
                .iter()
                .map(|n| g(n))
                .filter(|&b| b != AIR)
                .collect(),
            mycelium: g("mycelium"),
            snow_block: g("snow"),
            packed_ice: g("packed_ice"),
            mushroom_stem: g("mushroom_stem"),
            red_mushroom_block: g("red_mushroom_block"),
            brown_mushroom_block: g("brown_mushroom_block"),
            twig: g("twig"),
            pebble: g("pebble"),
            dandelion: g("dandelion"),
            dead_bush: g("dead_bush"),
            pumpkin: g("pumpkin"),
            melon: g("melon"),
            red_sand: g("red_sand"),
            terracotta: g("terracotta"),
            lily_pad: g("lily_pad"),
            mud: g("mud"),
            sugar_cane: g("sugar_cane"),
            red_mushroom: g("red_mushroom"),
            brown_mushroom: g("brown_mushroom"),
            cobblestone: g("cobblestone"),
            mossy_cobblestone: g("mossy_cobblestone"),
            ores,
            deepslate: g("deepslate"),
            kelp: g("kelp"),
            bamboo: g("bamboo"),
            berry_bush: g("sweet_berry_bush_stage3"),
            podzol: g("podzol"),
            coarse_dirt: g("coarse_dirt"),
            red_sandstone: g("red_sandstone"),
            sunflower: g("sunflower"),
            pale_moss: g("pale_moss_block"),
            pale_moss_carpet: g("pale_moss_carpet"),
            pale_hanging_moss: g("pale_hanging_moss"),
            seagrass: g("seagrass"),
            corals: CORALS.map(|c| g(&format!("{c}_coral"))),
            coral_blocks: CORALS.map(|c| g(&format!("{c}_coral_block"))),
            blue_ice: g("blue_ice"),
            deep_ores: ["coal", "iron", "gold", "diamond"]
                .iter()
                .map(|n| (g(&format!("{n}_ore")), g(&format!("deepslate_{n}_ore"))))
                .filter(|&(a, b)| a != AIR && b != AIR)
                .collect(),
            emerald: g("emerald_ore"),
            life_crystal: g("life_crystal"),
            geode: [
                Some(g("smooth_basalt")).filter(|&b| b != AIR).unwrap_or(g("basalt")),
                g("calcite"),
                g("amethyst_block"),
                g("budding_amethyst"),
                g("amethyst_cluster"),
            ],
            granite: g("granite"),
            bee_nest: g("bee_nest"),
            tuff: g("tuff"),
            vein_copper: (g("copper_ore"), g("raw_copper_block")),
            vein_iron: (g("iron_ore"), g("raw_iron_block")),
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
    ecologics: ecologics::EcoPalette,
    aether: aether::AetherPalette,
    otherside: otherside::OthersidePalette,
    distant: distant_lands::DistantPalette,
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
    /// The base biome: what the generator, mobs and weather go by.
    biome: Biome,
    /// The biome as named on the F3 screen: the base or one of its variants (`Biome::base`).
    variant: Biome,
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
            ecologics: ecologics::EcoPalette::new(reg),
            aether: aether::AetherPalette::new(reg),
            otherside: otherside::OthersidePalette::new(reg),
            distant: distant_lands::DistantPalette::new(reg),
        }
    }

    fn column(&self, wx: i32, wz: i32) -> Column {
        if self.dim == Dim::Nether {
            let b = self.nether_biome(wx, wz);
            return Column { height: NETHER_ROOF, biome: b, variant: b };
        }
        if self.dim == Dim::End {
            let b = end::end_biome(wx, wz);
            return Column { height: self.end_land(wx, wz).map(|l| l.0).unwrap_or(0), biome: b, variant: b };
        }
        if self.dim == Dim::Otherside {
            let b = self.otherside_biome(wx, wz);
            return Column { height: OTHERSIDE_ROOF, biome: b, variant: b };
        }
        if self.dim == Dim::Aether {
            let b = self.aether_biome(wx, wz);
            return Column { height: self.aether_land(wx, wz).map(|l| l.0).unwrap_or(0), biome: b, variant: b };
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
        // Mangrove swamps: the same, in the warm wet lowlands; lower, so mud lies under
        // a block or two of water.
        let mangrove = smoothstep(0.24, 0.34, hum) * smoothstep(0.16, 0.24, climate) * smoothstep(0.5, 0.42, climate);
        for (wet, target) in [(swamp, sea - 0.3 + detail * 3.0), (mangrove, sea - 1.2 + detail * 2.5)] {
            let (lo, hi) = (sea - 8.0, sea + 16.0);
            if wet > 0.0 && h > lo && h < hi {
                let edge = ((h - lo) / 4.0).min((hi - h) / 6.0).min(1.0);
                h += (target - h) * wet * edge;
            }
        }
        let wetland = swamp.max(mangrove);
        // Mushroom islands: rare low humps far out in the deep ocean.
        let mush = smoothstep(-0.38, -0.46, cont) * smoothstep(0.38, 0.46, self.humidity.fbm2(x / 260.0 - 900.0, z / 260.0 + 400.0, 2));
        if mush > 0.0 {
            let target = sea + 2.0 + detail * 4.0;
            h += (target - h).max(0.0) * mush;
        }
        // Rivers: long winding channels across the land, cut down to just under sea level.
        let rv = self.variant.fbm2(x / 640.0 + 900.0, z / 640.0 - 900.0, 3).abs();
        let river = smoothstep(0.05, 0.015, rv) * smoothstep(sea + 45.0, sea + 25.0, h) * (1.0 - wetland) * (1.0 - mush);
        if river > 0.0 && h > sea - 4.0 {
            let target = sea - 3.0 + detail;
            h += (target - h) * river.min(1.0).powf(0.7);
        }
        let pantanal = self.pantanal_shape(x, z, climate, hum, detail, &mut h);
        let height = (h as i32).clamp(4, CHUNK_HEIGHT as i32 - 20);

        let temp = climate - (height - 90).max(0) as f32 / 120.0;
        let biome = if mangrove > 0.5 && (SEA_LEVEL - 4..=SEA_LEVEL + 3).contains(&height) {
            Biome::MangroveSwamp
        } else if swamp > 0.5 && (SEA_LEVEL - 5..=SEA_LEVEL + 4).contains(&height) {
            Biome::Swamp
        } else if mush > 0.5 && height >= SEA_LEVEL {
            Biome::MushroomFields
        } else if river > 0.5 && height < SEA_LEVEL {
            Biome::River
        } else if height < SEA_LEVEL - 2 {
            if temp < -0.3 {
                Biome::FrozenOcean
            } else {
                Biome::Ocean
            }
        } else if height <= SEA_LEVEL + 1 && temp > -0.25 && wetland < 0.25 {
            Biome::Beach
        } else if bad > 0.5 {
            Biome::Badlands
        } else if height > 150 {
            Biome::SnowyPeaks
        } else if height > 118 {
            Biome::Mountains
        } else if height > 92 && temp > -0.2 && temp < 0.2 && hum < 0.15 && var > 0.05 {
            Biome::Meadow
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
        let biome = self.distant_biome(biome, x, z, temp, height, pantanal);
        let variant = self.variant_of(biome, x, z, temp, hum, height, mount_mask);
        // The high, sharp mountain tops count as peaks.
        Column { height, biome: variant.base(), variant }
    }

    /// The biome here as named on the F3 screen: a variant when there is one (`Biome::base`).
    pub fn biome_at(&self, wx: i32, wz: i32) -> Biome {
        self.column(wx, wz).variant
    }

    /// The biome here as the game goes by it (mobs, weather, sounds): the variant's `base()`.
    pub fn base_biome_at(&self, wx: i32, wz: i32) -> Biome {
        self.column(wx, wz).biome
    }

    /// Which variant of its biome a column is (Lote 4 of plan/o-que-falta.md): picked by the
    /// same climate the biome came from plus a patchwork noise of its own, so each variant
    /// grows in regions inside its parent.
    #[allow(clippy::too_many_arguments)]
    fn variant_of(&self, b: Biome, x: f32, z: f32, temp: f32, hum: f32, height: i32, mount_mask: f32) -> Biome {
        let v = self.variant.fbm2(x / 380.0 + 500.0, z / 380.0 - 200.0, 2);
        match b {
            Biome::Ocean if height < SEA_LEVEL - 24 && temp < 0.3 => Biome::DeepOcean,
            Biome::Ocean if temp > 0.3 => Biome::WarmOcean,
            Biome::Ocean if temp > 0.14 => Biome::LukewarmOcean,
            Biome::Ocean if temp < -0.12 => Biome::ColdOcean,
            Biome::River if temp < -0.28 => Biome::FrozenRiver,
            Biome::Beach if mount_mask > 0.55 || v > 0.42 => Biome::StonyShore,
            Biome::Beach if temp < -0.12 => Biome::SnowyBeach,
            Biome::Plains if v > 0.28 => Biome::SunflowerPlains,
            Biome::Forest if v < -0.25 => Biome::FlowerForest,
            Biome::Taiga if temp < -0.42 => Biome::SnowyTaiga,
            Biome::Taiga if v > 0.25 => Biome::OldGrowthTaiga,
            Biome::Mountains if height > 136 && v > 0.15 => Biome::JaggedPeaks,
            Biome::Mountains if temp < -0.3 && hum > 0.0 => Biome::Grove,
            Biome::Mountains if temp < -0.3 => Biome::SnowySlopes,
            Biome::Mountains if v < -0.18 => Biome::WindsweptHills,
            Biome::SnowyPeaks if v > -0.15 => Biome::JaggedPeaks,
            Biome::Snowy if v > 0.3 => Biome::IceSpikes,
            Biome::Jungle if v > 0.22 => Biome::BambooJungle,
            Biome::Jungle if v < -0.25 => Biome::SparseJungle,
            Biome::Badlands if v > 0.25 => Biome::WoodedBadlands,
            Biome::Badlands if v < -0.25 => Biome::ErodedBadlands,
            Biome::Savanna if height > 96 || v > 0.38 => Biome::WindsweptSavanna,
            Biome::DarkForest if v > 0.22 => Biome::PaleGarden,
            b => b,
        }
    }

    pub fn height_at(&self, wx: i32, wz: i32) -> i32 {
        self.column(wx, wz).height
    }

    /// Top, filler and filler depth for a column.
    fn top_blocks(&self, col: &Column, wx: i32, wz: i32) -> (BlockId, BlockId, i32) {
        let p = &self.pal;
        let h = col.height;
        if let Some(t) = self.variant_top_blocks(col, wx, wz) {
            return t;
        }
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
            Biome::SnowyPeaks if p.snow_block != AIR => {
                if (wx * 7 + wz * 13).rem_euclid(17) == 0 && p.packed_ice != AIR {
                    (p.packed_ice, p.stone, 2)
                } else {
                    (p.snow_block, p.stone, 2)
                }
            }
            Biome::SnowyPeaks => (p.snow_grass, p.stone, 1),
            Biome::River | Biome::FrozenOcean => {
                if (wx ^ wz) & 7 == 0 {
                    (p.clay, p.sand, 2)
                } else if h < SEA_LEVEL - 10 {
                    (p.gravel, p.gravel, 2)
                } else {
                    (p.sand, p.sand, 3)
                }
            }
            Biome::MushroomFields if p.mycelium != AIR => (p.mycelium, p.dirt, 3),
            // Mud flats; the odd grassy hummock above the water.
            Biome::MangroveSwamp if p.mud != AIR => {
                if h > SEA_LEVEL + 1 && (wx * 31 + wz * 17).rem_euclid(5) != 0 {
                    (p.grass, p.mud, 3)
                } else {
                    (p.mud, p.mud, 4)
                }
            }
            // Red sand over deep layers of terracotta (the layers are picked in `generate`).
            Biome::Badlands if p.red_sand != AIR => (p.red_sand, p.terracotta, (h - SEA_LEVEL + 14).max(3)),
            Biome::Badlands => (p.sand, p.sandstone, 4),
            b if is_distant(b) => self.distant_top_blocks(col, wx, wz),
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
        if self.dim == Dim::Otherside {
            return Surface { height: OTHERSIDE_ROOF, biome: col.biome, top: p.bedrock, liquid: None, trees: None };
        }
        if self.dim == Dim::Aether {
            return Surface { height: col.height, biome: col.biome, top: self.aether.grass_id(), liquid: None, trees: None };
        }
        let (top, _, _) = self.top_blocks(&col, wx, wz);
        let h = col.height;
        let top = if h < SEA_LEVEL && (top == p.grass || top == p.snow_grass) { p.dirt } else { top };
        let liquid = (h < SEA_LEVEL).then_some(if matches!(col.biome, Biome::Snowy | Biome::Taiga | Biome::FrozenOcean | Biome::SnowyPeaks) { p.ice } else { p.water });
        // Tree attempts per chunk in `decorate`, out of the forest's 9.
        let trees = if !(SEA_LEVEL..=200).contains(&h) && !(col.biome == Biome::MangroveSwamp && h >= SEA_LEVEL - 2) {
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
                Biome::MangroveSwamp => Some((self.trees.leaves(Wood::Mangrove), 0.7)),
                Biome::Meadow => Some((p.birch_leaves, 0.05)),
                b => self.distant_tree_cover(b),
            }
        };
        Surface { height: h, biome: col.variant, top, liquid, trees }
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
        if self.dim == Dim::Otherside {
            return self.generate_otherside(pos);
        }
        if self.dim == Dim::Aether {
            return self.generate_aether(pos);
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
                chunk.biomes[lx + lz * 16] = col.variant as u8;
                let h = col.height;
                let (top, filler, filler_depth) = self.top_blocks(col, wx, wz);
                let banded = filler == p.terracotta && filler != AIR;
                // Badlands layers wobble a little so they don't look ruled.
                let wob = if banded { (self.mountain.noise2(wx as f32 / 60.0, wz as f32 / 60.0) * 3.0) as i32 } else { 0 };
                for y in 0..=h.max(SEA_LEVEL) {
                    let id = if y == 0 || (y <= 4 && rng.below(5) < (5 - y) as u32) {
                        p.bedrock
                    } else if y > h {
                        if y == SEA_LEVEL && (matches!(col.biome, Biome::Snowy | Biome::Taiga | Biome::FrozenOcean) || (col.biome == Biome::River && self.temperature.fbm2(wx as f32 / 1100.0, wz as f32 / 1100.0, 3) < -0.28)) {
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
        self.geodes_and_veins(&mut chunk, pos);
        self.decorate_caves(&mut chunk, &mut rng, &cols, pos);
        self.decorate(&mut chunk, &mut rng, &cols, pos);
        self.decorate_ecologics(&mut chunk, &cols, pos);
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

    pub fn structures(&self) -> &crate::structures::Structures {
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
        // The bottom few blocks above bedrock turn to deepslate, ores and all.
        if p.deepslate != AIR {
            for z in 0..16 {
                for x in 0..16 {
                    let top = 5 + rng.below(4) as usize;
                    for y in 1..top {
                        let b = chunk.get(x, y, z);
                        if b == p.stone {
                            chunk.set(x, y, z, p.deepslate);
                        } else if let Some(&(_, d)) = p.deep_ores.iter().find(|&&(o, _)| o == b) {
                            chunk.set(x, y, z, d);
                        }
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

    /// Now and then deep down: an amethyst geode (a hollow ball lined with calcite and
    /// amethyst, crystals growing inside) or a big ore vein (a long ribbon of copper ore in
    /// granite, or iron ore in tuff, with lumps of raw ore here and there).
    fn geodes_and_veins(&self, chunk: &mut Chunk, pos: ChunkPos) {
        let p = &self.pal;
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 7300);
        if p.geode.iter().all(|&b| b != AIR) && rng.below(28) == 0 {
            let r = 4.0 + rng.f32() * 1.2;
            let (cx, cy, cz) = (rng.range(6, 10), rng.range(12, 40), rng.range(6, 10));
            let mut lining = Vec::new();
            for y in cy - 6..=cy + 6 {
                for z in (cz - 6).max(0)..=(cz + 6).min(15) {
                    for x in (cx - 6).max(0)..=(cx + 6).min(15) {
                        let d = (((x - cx) * (x - cx) + (y - cy) * (y - cy) + (z - cz) * (z - cz)) as f32).sqrt();
                        if d > r || y < 2 {
                            continue;
                        }
                        let (ux, uy, uz) = (x as usize, y as usize, z as usize);
                        let id = if d > r - 1.0 {
                            p.geode[0]
                        } else if d > r - 2.0 {
                            p.geode[1]
                        } else if d > r - 3.0 {
                            lining.push((ux, uy, uz));
                            if rng.below(10) == 0 { p.geode[3] } else { p.geode[2] }
                        } else {
                            AIR
                        };
                        chunk.set(ux, uy, uz, id);
                    }
                }
            }
            // Crystals on the inside of each budding block.
            for (x, y, z) in lining {
                if chunk.get(x, y, z) != p.geode[3] {
                    continue;
                }
                let toward = [((cx - x as i32).signum(), 0, 0), (0, (cy - y as i32).signum(), 0), (0, 0, (cz - z as i32).signum())];
                for (dx, dy, dz) in toward {
                    let (nx, ny, nz) = (x as i32 + dx, y as i32 + dy, z as i32 + dz);
                    if (dx, dy, dz) != (0, 0, 0) && (0..16).contains(&nx) && (0..16).contains(&nz) && chunk.get(nx as usize, ny as usize, nz as usize) == AIR {
                        chunk.set(nx as usize, ny as usize, nz as usize, p.geode[4]);
                        break;
                    }
                }
            }
        }
        if rng.below(14) == 0 {
            let iron = rng.below(2) == 0;
            let (filler, (ore, raw)) = if iron { (p.tuff, p.vein_iron) } else { (p.granite, p.vein_copper) };
            if filler == AIR || ore == AIR {
                return;
            }
            let (mut x, mut y, mut z) = (rng.f32() * 16.0, if iron { rng.range(6, 24) } else { rng.range(20, 50) } as f32, rng.f32() * 16.0);
            let a = rng.f32() * std::f32::consts::TAU;
            let (dx, dz) = (a.cos(), a.sin());
            for _ in 0..40 {
                for oy in -1..=1 {
                    for oz in -1..=1 {
                        for ox in -1..=1 {
                            let (bx, by, bz) = (x as i32 + ox, y as i32 + oy, z as i32 + oz);
                            if !(0..16).contains(&bx) || !(0..16).contains(&bz) || by < 2 {
                                continue;
                            }
                            let (ux, uy, uz) = (bx as usize, by as usize, bz as usize);
                            let here = chunk.get(ux, uy, uz);
                            if here != p.stone && here != filler {
                                continue;
                            }
                            let roll = rng.below(100);
                            let id = if roll < 2 && raw != AIR { raw } else if roll < 26 { ore } else { filler };
                            chunk.set(ux, uy, uz, id);
                        }
                    }
                }
                x += dx * 0.6;
                z += dz * 0.6;
                y += (rng.f32() - 0.5) * 0.8;
            }
        }
    }

    /// A huge mushroom: a pale stem under a red dome or a wide flat brown cap. `x` and `z`
    /// keep three blocks from the chunk edge.
    fn huge_mushroom(&self, chunk: &mut Chunk, rng: &mut Rng, x: usize, y: i32, z: usize) {
        let p = &self.pal;
        if p.mushroom_stem == AIR {
            return;
        }
        let h = rng.range(4, 7);
        if y + h + 3 >= CHUNK_HEIGHT as i32 || (1..=h + 1).any(|k| chunk.get(x, (y + k) as usize, z) != AIR) {
            return;
        }
        for k in 1..=h {
            chunk.set(x, (y + k) as usize, z, p.mushroom_stem);
        }
        let top = y + h;
        let set = |chunk: &mut Chunk, dx: i32, dy: i32, dz: i32, b: BlockId| {
            let (cx, cy, cz) = ((x as i32 + dx) as usize, (top + dy) as usize, (z as i32 + dz) as usize);
            if chunk.get(cx, cy, cz) == AIR {
                chunk.set(cx, cy, cz, b);
            }
        };
        if rng.chance(0.5) {
            // Red: a dome, sides hanging down three blocks.
            for dy in -2..=1 {
                let r: i32 = if dy == 1 { 1 } else { 2 };
                for dz in -r..=r {
                    for dx in -r..=r {
                        let rim = dx.abs() == r || dz.abs() == r;
                        if (dy < 1 && !rim) || (dx.abs() == r && dz.abs() == r) {
                            continue;
                        }
                        set(chunk, dx, dy, dz, p.red_mushroom_block);
                    }
                }
            }
        } else {
            // Brown: one wide flat cap.
            for dz in -3i32..=3 {
                for dx in -3i32..=3 {
                    if dx.abs() == 3 && dz.abs() == 3 {
                        continue;
                    }
                    set(chunk, dx, 1, dz, p.brown_mushroom_block);
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
            Biome::MangroveSwamp => 5,
            Biome::Savanna => 2,
            Biome::CherryGrove => 5,
            Biome::Badlands => 2,
            Biome::Meadow => 1,
            Biome::MushroomFields => 3,
            _ => 0,
        };
        let tree_attempts = match cols[8 + 8 * 16].variant {
            Biome::SparseJungle => 3,
            Biome::BambooJungle => 5,
            Biome::OldGrowthTaiga => 12,
            Biome::FlowerForest => 6,
            Biome::Grove => 5,
            Biome::WoodedBadlands => 7,
            Biome::WindsweptHills => 2,
            Biome::PaleGarden => 12,
            Biome::SnowySlopes | Biome::IceSpikes | Biome::JaggedPeaks => 0,
            _ => tree_attempts,
        };
        for _ in 0..tree_attempts {
            let x = rng.range(2, 13) as usize;
            let z = rng.range(2, 13) as usize;
            let col = &cols[x + z * 16];
            let y = col.height;
            // Swamp trees may stand in a block of water, mangroves in two.
            let wading = (col.biome == Biome::Swamp && y == SEA_LEVEL - 1) || (col.biome == Biome::MangroveSwamp && y >= SEA_LEVEL - 2 && y < SEA_LEVEL);
            if (y < SEA_LEVEL && !wading) || y > 200 {
                continue;
            }
            let ground = chunk.get(x, y as usize, z);
            if matches!(col.biome, Biome::Desert | Biome::Badlands) && !(col.variant == Biome::WoodedBadlands && ground == p.grass) {
                if (ground == p.sand || ground == p.red_sand) && ground != AIR && rng.chance(0.6) {
                    let hgt = rng.range(1, 3);
                    for i in 1..=hgt {
                        chunk.set(x, (y + i) as usize, z, p.cactus);
                    }
                }
                continue;
            }
            if col.biome == Biome::MushroomFields {
                if ground == p.mycelium && ground != AIR {
                    self.huge_mushroom(chunk, rng, x.clamp(3, 12), y, z.clamp(3, 12));
                }
                continue;
            }
            let muddy = ground == p.mud && ground != AIR;
            if ground != p.grass && ground != p.snow_grass && !(wading && ground == p.dirt) && !muddy {
                continue;
            }
            let kind = match (col.variant, col.biome) {
                (Biome::PaleGarden, _) if self.trees.has(Wood::PaleOak) => {
                    if rng.chance(0.85) {
                        TreeKind::PaleOak
                    } else {
                        TreeKind::DarkOak
                    }
                }
                (Biome::WoodedBadlands, _) => TreeKind::Round(Wood::Oak),
                (Biome::SparseJungle, _) if rng.chance(0.5) => TreeKind::Bush(Wood::Jungle),
                (Biome::FlowerForest, _) if rng.chance(0.3) => TreeKind::Round(Wood::Birch),
                (_, b) => match b {
                Biome::Meadow => TreeKind::Round(Wood::Birch),
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
                Biome::MangroveSwamp if self.trees.has(Wood::Mangrove) => TreeKind::Mangrove,
                Biome::MangroveSwamp => TreeKind::SwampOak,
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
                },
            };
            // Wide trees are pulled in from the chunk edge; the old trees never move (reach 2).
            let m = kind.reach();
            let far = 15 - m - kind.wide_trunk() as i32;
            let (tx, tz) = ((x as i32).clamp(m, far), (z as i32).clamp(m, far));
            if (tx, tz) != (x as i32, z as i32) {
                let c2 = &cols[tx as usize + tz as usize * 16];
                let g2 = chunk.get(tx as usize, c2.height as usize, tz as usize);
                if c2.height != y || (g2 != p.grass && !(wading && g2 == p.dirt) && !(muddy && g2 == p.mud)) {
                    continue;
                }
            }
            trees::grow(kind, &self.trees, &mut ChunkSpace(chunk), rng, tx, y, tz);
            // Now and then a bees' nest hangs on the trunk of a broadleaf tree in open country.
            let open = matches!(col.biome, Biome::Plains | Biome::Meadow | Biome::Forest | Biome::BirchForest | Biome::CherryGrove);
            if open && p.bee_nest != AIR && matches!(kind, TreeKind::Round(_) | TreeKind::Cherry) {
                let mut nr = Rng::for_pos(self.seed, pos.x * 16 + tx, pos.y * 16 + tz, 7400);
                if nr.below(if col.biome == Biome::Meadow { 6 } else { 18 }) == 0 {
                    let (nx, ny, nz) = (tx as usize, (y + 2) as usize, (tz + 1) as usize);
                    if nz < 16 && chunk.get(nx, ny, nz) == AIR && chunk.get(tx as usize, ny, tz as usize) != AIR {
                        chunk.set(nx, ny, nz, p.bee_nest);
                    }
                }
            }
        }

        self.decorate_distant(chunk, cols, pos);
        self.scatter(chunk, cols, pos);
        self.vanilla_scatter(chunk, cols, pos);

        // Ground plants.
        for z in 0..16usize {
            for x in 0..16usize {
                let col = &cols[x + z * 16];
                let y = col.height;
                if matches!(col.biome, Biome::Swamp | Biome::MangroveSwamp) && y < SEA_LEVEL {
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
                // Sugar cane on the banks, one to three tall.
                let wet = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dz)| {
                    let (nx, nz) = (x as i32 + dx, z as i32 + dz);
                    (0..16).contains(&nx) && (0..16).contains(&nz) && chunk.get(nx as usize, uy, nz as usize) == p.water
                });
                if wet && p.sugar_cane != AIR && (ground == p.grass || ground == p.sand) && rng.chance(0.18) {
                    let h = 1 + (rng.f32() * 3.0) as usize;
                    for k in 0..h.min(CHUNK_HEIGHT - above) {
                        chunk.set(x, above + k, z, p.sugar_cane);
                    }
                    continue;
                }
                if ground == p.grass {
                    let r = rng.f32();
                    let grass_p = match col.biome {
                        Biome::Plains => 0.22,
                        Biome::Forest | Biome::BirchForest => 0.12,
                        Biome::Savanna => 0.42,
                        Biome::Jungle => 0.32,
                        Biome::CherryGrove => 0.2,
                        Biome::Swamp | Biome::MangroveSwamp => 0.14,
                        Biome::DarkForest => 0.08,
                        Biome::Meadow => 0.4,
                        _ => 0.06,
                    };
                    let flower_p = match col.biome {
                        Biome::CherryGrove => 0.05,
                        Biome::Meadow => 0.14,
                        Biome::Plains | Biome::Forest | Biome::BirchForest => 0.02,
                        _ => 0.012,
                    };
                    if r < flower_p {
                        // Meadows and plains get every kind; elsewhere mostly poppies and dandelions.
                        let many = matches!(col.biome, Biome::Meadow | Biome::Plains | Biome::Forest | Biome::BirchForest | Biome::CherryGrove) && !p.meadow_flowers.is_empty();
                        let f = if many && rng.chance(0.7) {
                            // Patches of one kind, picked by place.
                            let k = (self.variant.noise2((pos.x * 16 + x as i32) as f32 / 24.0, (pos.y * 16 + z as i32) as f32 / 24.0) * 0.5 + 0.5) * p.meadow_flowers.len() as f32;
                            p.meadow_flowers[(k as usize).min(p.meadow_flowers.len() - 1)]
                        } else if rng.chance(0.5) {
                            p.poppy
                        } else {
                            p.dandelion
                        };
                        chunk.set(x, above, z, f);
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
                    } else if r > 0.97 && matches!(col.biome, Biome::DarkForest | Biome::Swamp | Biome::MangroveSwamp) {
                        let m = if rng.chance(0.5) { p.red_mushroom } else { p.brown_mushroom };
                        if m != AIR {
                            chunk.set(x, above, z, m);
                        }
                    }
                } else if ground == p.mycelium && ground != AIR && rng.chance(0.03) {
                    chunk.set(x, above, z, if rng.chance(0.5) { p.red_mushroom } else { p.brown_mushroom });
                } else if (ground == p.sand && col.biome == Biome::Desert && rng.chance(0.01)) || (ground == p.red_sand && ground != AIR && rng.chance(0.012)) {
                    chunk.set(x, above, z, p.dead_bush);
                } else if ground == p.snow_grass && chunk.get(x, above, z) == AIR && p.snow_layer != AIR {
                    // A blanket of snow, a little deeper here and there.
                    let deep = rng.chance(0.15);
                    chunk.set(x, above, z, if deep { p.snow_layer2 } else { p.snow_layer });
                }
            }
        }
        self.variant_scatter(chunk, cols, pos);
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

    /// Vanilla things added later, on a random stream of their own so nothing older moves: kelp
    /// in the oceans, bamboo groves in the jungle, podzol and berry bushes in the taiga, coarse
    /// dirt in savannas and on mountains, red sandstone under the badlands' red sand.
    fn vanilla_scatter(&self, chunk: &mut Chunk, cols: &[Column], pos: ChunkPos) {
        let p = &self.pal;
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 3);
        let biome = cols[8 + 8 * 16].biome;
        let top = |x: i32, z: i32| cols[(x + z * 16) as usize].height;
        match biome {
            Biome::Ocean if p.kelp != AIR => {
                for _ in 0..rng.range(0, 14) {
                    let (x, z) = (rng.range(0, 15), rng.range(0, 15));
                    let y = top(x, z);
                    let (ux, uz) = (x as usize, z as usize);
                    let ground = chunk.get(ux, y as usize, uz);
                    if y > SEA_LEVEL - 4 || y < 2 || !(ground == p.sand || ground == p.gravel || ground == p.clay) {
                        continue;
                    }
                    let room = SEA_LEVEL - 1 - y;
                    let h = 1 + rng.below(room.max(1) as u32) as i32;
                    for k in 1..=h {
                        let uy = (y + k) as usize;
                        if chunk.get(ux, uy, uz) != p.water {
                            break;
                        }
                        chunk.set(ux, uy, uz, p.kelp);
                    }
                }
            }
            Biome::Jungle if p.bamboo != AIR && rng.chance(0.45) => {
                let (cx, cz) = (rng.range(3, 12), rng.range(3, 12));
                for _ in 0..rng.range(6, 16) {
                    let (x, z) = ((cx + rng.range(-3, 3)).clamp(0, 15), (cz + rng.range(-3, 3)).clamp(0, 15));
                    let y = top(x, z);
                    let (ux, uz) = (x as usize, z as usize);
                    if !(SEA_LEVEL..=200).contains(&y) || chunk.get(ux, y as usize, uz) != p.grass {
                        continue;
                    }
                    if p.podzol != AIR {
                        chunk.set(ux, y as usize, uz, p.podzol);
                    }
                    let h = rng.range(4, 12);
                    for k in 1..=h {
                        let uy = (y + k) as usize;
                        let b = chunk.get(ux, uy, uz);
                        if b != AIR && b != p.short_grass {
                            break;
                        }
                        chunk.set(ux, uy, uz, p.bamboo);
                    }
                }
            }
            Biome::Taiga => {
                // Podzol in round patches under the spruces.
                if p.podzol != AIR && rng.chance(0.6) {
                    let (cx, cz, r) = (rng.range(2, 13), rng.range(2, 13), 1.5 + rng.f32() * 1.5);
                    for z in (cz - 3).max(0)..=(cz + 3).min(15) {
                        for x in (cx - 3).max(0)..=(cx + 3).min(15) {
                            let d = (((x - cx) * (x - cx) + (z - cz) * (z - cz)) as f32).sqrt();
                            let y = top(x, z) as usize;
                            if d + rng.f32() * 0.6 <= r && chunk.get(x as usize, y, z as usize) == p.snow_grass {
                                chunk.set(x as usize, y, z as usize, p.podzol);
                            }
                        }
                    }
                }
                // A few sweet berry bushes, ripe.
                if p.berry_bush != AIR && rng.chance(0.3) {
                    for _ in 0..rng.range(1, 3) {
                        let (x, z) = (rng.range(0, 15), rng.range(0, 15));
                        let y = top(x, z);
                        let (ux, uy, uz) = (x as usize, y as usize, z as usize);
                        let g = chunk.get(ux, uy, uz);
                        if !(SEA_LEVEL..=200).contains(&y) || !(g == p.snow_grass || g == p.podzol || g == p.grass) {
                            continue;
                        }
                        let above = chunk.get(ux, uy + 1, uz);
                        if above == AIR || above == p.snow_layer || above == p.snow_layer2 || above == p.short_grass {
                            chunk.set(ux, uy + 1, uz, p.berry_bush);
                        }
                    }
                }
            }
            Biome::Savanna | Biome::Mountains if p.coarse_dirt != AIR && rng.chance(0.35) => {
                let (cx, cz) = (rng.range(2, 13), rng.range(2, 13));
                for _ in 0..rng.range(6, 14) {
                    let (x, z) = ((cx + rng.range(-2, 2)).clamp(0, 15), (cz + rng.range(-2, 2)).clamp(0, 15));
                    let y = top(x, z) as usize;
                    if chunk.get(x as usize, y, z as usize) == p.grass {
                        chunk.set(x as usize, y, z as usize, p.coarse_dirt);
                        if chunk.get(x as usize, y + 1, z as usize) == p.short_grass {
                            chunk.set(x as usize, y + 1, z as usize, AIR);
                        }
                    }
                }
            }
            Biome::Badlands if p.red_sandstone != AIR && p.red_sand != AIR => {
                for z in 0..16 {
                    for x in 0..16 {
                        let y = top(x, z) as usize;
                        if y > 3 && chunk.get(x as usize, y, z as usize) == p.red_sand {
                            for d in 1..=2 {
                                if chunk.get(x as usize, y - d, z as usize) != AIR {
                                    chunk.set(x as usize, y - d, z as usize, p.red_sandstone);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
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
        count[Generator::new(1234, &reg).with_dim(Dim::End).biome_at(2000, 0) as usize] += 1;
        let total: usize = count.iter().sum();
        let mut missing = vec![];
        for (i, &c) in count.iter().enumerate() {
            // Unused numbers, and the mod pack's dimensions' biomes (tested with their dimensions);
            // Distant Lands' biomes are in the Overworld, so they must turn up here.
            if Biome::from_u8(i as u8) as usize != i || (40..46).contains(&i) {
                continue;
            }
            println!("{:>14}: {:.1}%", Biome::from_u8(i as u8).name(), c as f32 * 100.0 / total as f32);
            if c == 0 {
                missing.push(Biome::from_u8(i as u8).name());
            }
        }
        assert!(missing.is_empty(), "never appear: {missing:?}");
    }

    #[test]
    fn aether_has_islands_in_both_biomes() {
        let reg = registry();
        let g = Generator::new(42, &reg).with_dim(Dim::Aether);
        let (mut land, mut golden, mut n) = (0, 0, 0);
        for z in (-3000..3000).step_by(48) {
            for x in (-3000..3000).step_by(48) {
                n += 1;
                land += usize::from(g.aether_land(x, z).is_some());
                golden += usize::from(g.biome_at(x, z) == Biome::GoldenGrove);
            }
        }
        assert!(land * 10 > n && land * 10 < n * 8, "islands cover {land} of {n} columns");
        assert!(golden > 0, "no golden groves");
        let c = g.generate(ChunkPos::new(0, 0));
        assert!((0..256).all(|y| c.get(5, y, 5) != reg.id_or_air("bedrock")), "no bedrock in the sky");
    }

    #[test]
    fn new_biomes_grow_their_trees() {
        let reg = registry();
        let g = Generator::new(77, &reg);
        for (biome, log) in [(Biome::Jungle, "jungle_log"), (Biome::Savanna, "acacia_log"), (Biome::DarkForest, "dark_oak_log"), (Biome::CherryGrove, "cherry_log"), (Biome::MangroveSwamp, "mangrove_log")] {
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

#[cfg(test)]
mod geode_tests {
    use super::*;

    #[test]
    fn geodes_and_veins_appear() {
        let reg = BlockRegistry::from_ron(include_str!("../../../packs/vanilla/blocks.ron")).unwrap();
        let gen = Generator::new(1234, &reg);
        let (bud, raw) = (reg.id("budding_amethyst").unwrap(), reg.id("raw_copper_block").unwrap());
        let (mut geodes, mut veins) = (0, 0);
        for cz in -6..6 {
            for cx in -6..6 {
                let c = gen.generate(ChunkPos::new(cx, cz));
                let mut found = (false, false);
                for y in 1..60 {
                    for z in 0..16 {
                        for x in 0..16 {
                            let b = c.get(x, y, z);
                            if b == bud && !found.0 {
                                found.0 = true;
                                if std::env::var("MC_PRINT").is_ok() {
                                    println!("geode near {} {} {}", cx * 16 + x as i32, y, cz * 16 + z as i32);
                                    for yy in (y.saturating_sub(5)..y + 6).rev() {
                                        let row: String = (0..16)
                                            .map(|xx| match reg.get(c.get(xx, yy, z)).name.as_str() {
                                                "air" => '.',
                                                "calcite" => 'c',
                                                "amethyst_block" => 'A',
                                                "budding_amethyst" => 'B',
                                                "amethyst_cluster" => '*',
                                                "basalt" | "smooth_basalt" => '#',
                                                _ => ' ',
                                            })
                                            .collect();
                                        println!("  {row}");
                                    }
                                }
                            }
                            found.1 |= b == raw;
                        }
                    }
                }
                geodes += found.0 as u32;
                veins += found.1 as u32;
            }
        }
        assert!(geodes > 0 && veins > 0, "geodes {geodes}, veins {veins}");
    }
}
