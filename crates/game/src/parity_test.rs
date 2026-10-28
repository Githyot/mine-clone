//! Parity with Minecraft Java (26.x): the game's own block, item, mob and recipe data checked
//! against the numbers on the public Minecraft Wiki. Run with
//! `cargo test -p game parity -- --nocapture` for the full report (it is also written to
//! `target/parity-report.md`).
//!
//! A row only checks what the game has: a block or item the game does not have yet is listed
//! as "falta" in the report, not as a failure. A difference we keep on purpose goes in
//! `ON_PURPOSE` with the reason, so the test stays green and the choice stays written down.
//!
//! Many vanilla mobs have their own name and design here (the creeper is the "stalker", the
//! enderman the "void_walker"...); `MOBS` names our counterpart.

use voxel::block::ToolKind;
use voxel::item::ItemStack;

use crate::embedded;

/// Tool letters: P pickaxe, A axe, S shovel, H hoe, W sword, - none, ? not checked.
/// Tier: 0 drops by hand, 1 any pickaxe (wood up), 2 stone up, 3 iron up, 4 diamond up.
/// Drop: "" itself, "-" nothing, else an item that must be among the drops.
const BLOCKS: &[(&str, f32, char, u8, &str)] = &[
    ("stone", 1.5, 'P', 1, "cobblestone"),
    ("cobblestone", 2.0, 'P', 1, ""),
    ("mossy_cobblestone", 2.0, 'P', 1, ""),
    ("granite", 1.5, 'P', 1, ""),
    ("diorite", 1.5, 'P', 1, ""),
    ("andesite", 1.5, 'P', 1, ""),
    ("deepslate", 3.0, 'P', 1, "cobbled_deepslate"),
    ("cobbled_deepslate", 3.5, 'P', 1, ""),
    ("tuff", 1.5, 'P', 1, ""),
    ("calcite", 0.75, 'P', 1, ""),
    ("smooth_stone", 2.0, 'P', 1, ""),
    ("grass_block", 0.6, 'S', 0, "dirt"),
    ("dirt", 0.5, 'S', 0, ""),
    ("coarse_dirt", 0.5, 'S', 0, ""),
    ("podzol", 0.5, 'S', 0, "dirt"),
    ("mycelium", 0.6, 'S', 0, "dirt"),
    ("rooted_dirt", 0.5, 'S', 0, ""),
    ("farmland", 0.6, 'S', 0, "dirt"),
    ("dirt_path", 0.65, 'S', 0, "dirt"),
    ("sand", 0.5, 'S', 0, ""),
    ("red_sand", 0.5, 'S', 0, ""),
    ("gravel", 0.6, 'S', 0, "flint"),
    ("clay", 0.6, 'S', 0, "clay_ball"),
    ("mud", 0.5, 'S', 0, ""),
    ("packed_mud", 1.0, 'P', 0, ""),
    ("mud_bricks", 1.5, 'P', 1, ""),
    ("soul_sand", 0.5, 'S', 0, ""),
    ("soul_soil", 0.5, 'S', 0, ""),
    ("snow", 0.2, 'S', 1, "snowball"),
    ("snow_layer", 0.1, 'S', 1, "snowball"),
    ("sandstone", 0.8, 'P', 1, ""),
    ("red_sandstone", 0.8, 'P', 1, ""),
    ("cut_sandstone", 0.8, 'P', 1, ""),
    ("chiseled_sandstone", 0.8, 'P', 1, ""),
    ("bedrock", -1.0, '?', 0, "-"),
    ("obsidian", 50.0, 'P', 4, ""),
    ("crying_obsidian", 50.0, 'P', 4, ""),
    ("oak_log", 2.0, 'A', 0, ""),
    ("birch_log", 2.0, 'A', 0, ""),
    ("spruce_log", 2.0, 'A', 0, ""),
    ("cherry_log", 2.0, 'A', 0, ""),
    ("crimson_stem", 2.0, 'A', 0, ""),
    ("oak_planks", 2.0, 'A', 0, ""),
    ("crimson_planks", 2.0, 'A', 0, ""),
    ("oak_leaves", 0.2, '?', 0, "oak_sapling"),
    ("glass", 0.3, '-', 0, "-"),
    ("glass_pane", 0.3, '-', 0, "-"),
    ("white_stained_glass", 0.3, '-', 0, "-"),
    ("ice", 0.5, 'P', 0, "-"),
    ("packed_ice", 0.5, 'P', 0, "-"),
    ("blue_ice", 2.8, 'P', 0, "-"),
    ("cactus", 0.4, '-', 0, ""),
    ("cobweb", 4.0, '?', 0, "string"),
    ("torch", 0.0, '-', 0, ""),
    ("coal_ore", 3.0, 'P', 1, "coal"),
    ("deepslate_coal_ore", 4.5, 'P', 1, "coal"),
    ("iron_ore", 3.0, 'P', 2, "raw_iron"),
    ("deepslate_iron_ore", 4.5, 'P', 2, "raw_iron"),
    ("copper_ore", 3.0, 'P', 2, "raw_copper"),
    ("gold_ore", 3.0, 'P', 3, "raw_gold"),
    ("deepslate_gold_ore", 4.5, 'P', 3, "raw_gold"),
    ("redstone_ore", 3.0, 'P', 3, "redstone"),
    ("lapis_ore", 3.0, 'P', 2, "lapis_lazuli"),
    ("diamond_ore", 3.0, 'P', 3, "diamond"),
    ("deepslate_diamond_ore", 4.5, 'P', 3, "diamond"),
    ("emerald_ore", 3.0, 'P', 3, "emerald"),
    ("nether_quartz_ore", 3.0, 'P', 1, "quartz"),
    ("nether_gold_ore", 3.0, 'P', 1, "gold_nugget"),
    ("ancient_debris", 30.0, 'P', 4, ""),
    ("coal_block", 5.0, 'P', 1, ""),
    ("iron_block", 5.0, 'P', 2, ""),
    ("gold_block", 3.0, 'P', 3, ""),
    ("diamond_block", 5.0, 'P', 3, ""),
    ("emerald_block", 5.0, 'P', 3, ""),
    ("lapis_block", 3.0, 'P', 2, ""),
    ("redstone_block", 5.0, 'P', 1, ""),
    ("copper_block", 3.0, 'P', 2, ""),
    ("netherite_block", 50.0, 'P', 4, ""),
    ("raw_iron_block", 5.0, 'P', 2, ""),
    ("raw_copper_block", 5.0, 'P', 2, ""),
    ("raw_gold_block", 5.0, 'P', 3, ""),
    ("amethyst_block", 1.5, 'P', 1, ""),
    ("budding_amethyst", 1.5, 'P', 0, "-"),
    ("amethyst_cluster", 1.5, 'P', 0, "amethyst_shard"),
    ("bricks", 2.0, 'P', 1, ""),
    ("stone_bricks", 1.5, 'P', 1, ""),
    ("mossy_stone_bricks", 1.5, 'P', 1, ""),
    ("cracked_stone_bricks", 1.5, 'P', 1, ""),
    ("nether_bricks", 2.0, 'P', 1, ""),
    ("end_stone", 3.0, 'P', 1, ""),
    ("end_stone_bricks", 3.0, 'P', 1, ""),
    ("purpur_block", 1.5, 'P', 1, ""),
    ("purpur_pillar", 1.5, 'P', 1, ""),
    ("quartz_block", 0.8, 'P', 1, ""),
    ("prismarine", 1.5, 'P', 1, ""),
    ("terracotta", 1.25, 'P', 1, ""),
    ("red_terracotta", 1.25, 'P', 1, ""),
    ("white_concrete", 1.8, 'P', 1, ""),
    ("netherrack", 0.4, 'P', 1, ""),
    ("glowstone", 0.3, '-', 0, "glowstone_dust"),
    ("magma_block", 0.5, 'P', 1, ""),
    ("basalt", 1.25, 'P', 1, ""),
    ("polished_basalt", 1.25, 'P', 1, ""),
    ("smooth_basalt", 1.25, 'P', 1, ""),
    ("blackstone", 1.5, 'P', 1, ""),
    ("polished_blackstone", 2.0, 'P', 1, ""),
    ("polished_blackstone_bricks", 1.5, 'P', 1, ""),
    ("gilded_blackstone", 1.5, 'P', 1, "gilded_blackstone"),
    ("crimson_nylium", 0.4, 'P', 1, "netherrack"),
    ("warped_nylium", 0.4, 'P', 1, "netherrack"),
    ("nether_wart_block", 1.0, 'H', 0, ""),
    ("warped_wart_block", 1.0, 'H', 0, ""),
    ("shroomlight", 1.0, 'H', 0, ""),
    ("crafting_table", 2.5, 'A', 0, ""),
    ("furnace", 3.5, 'P', 1, ""),
    ("chest", 2.5, 'A', 0, ""),
    ("bookshelf", 1.5, 'A', 0, "book"),
    ("tnt", 0.0, '-', 0, ""),
    ("pumpkin", 1.0, 'A', 0, ""),
    ("melon", 1.0, 'A', 0, "melon_slice"),
    ("sponge", 0.6, 'H', 0, ""),
    ("hay_block", 0.5, 'H', 0, ""),
    ("white_wool", 0.8, '?', 0, ""),
    ("white_carpet", 0.1, '-', 0, ""),
    ("red_bed", 0.2, '-', 0, ""),
    ("ladder", 0.4, 'A', 0, ""),
    ("oak_door", 3.0, 'A', 0, ""),
    ("iron_door", 5.0, 'P', 1, ""),
    ("oak_trapdoor", 3.0, 'A', 0, ""),
    ("oak_fence", 2.0, 'A', 0, ""),
    ("oak_slab", 2.0, 'A', 0, ""),
    ("oak_stairs", 2.0, 'A', 0, ""),
    ("cobblestone_slab", 2.0, 'P', 1, ""),
    ("cobblestone_stairs", 2.0, 'P', 1, ""),
    ("cobblestone_wall", 2.0, 'P', 1, ""),
    ("stone_slab", 2.0, 'P', 1, ""),
    ("stone_stairs", 1.5, 'P', 1, ""),
    ("stone_brick_slab", 2.0, 'P', 1, ""),
    ("stone_brick_stairs", 1.5, 'P', 1, ""),
    ("brick_slab", 2.0, 'P', 1, ""),
    ("brick_stairs", 2.0, 'P', 1, ""),
    ("sandstone_slab", 2.0, 'P', 1, ""),
    ("sandstone_stairs", 0.8, 'P', 1, ""),
    ("nether_brick_slab", 2.0, 'P', 1, ""),
    ("nether_brick_stairs", 2.0, 'P', 1, ""),
    ("nether_brick_fence", 2.0, 'P', 1, ""),
    ("quartz_slab", 2.0, 'P', 1, ""),
    ("quartz_stairs", 0.8, 'P', 1, ""),
    ("purpur_slab", 2.0, 'P', 1, ""),
    ("purpur_stairs", 1.5, 'P', 1, ""),
    ("end_stone_brick_slab", 3.0, 'P', 1, ""),
    ("end_stone_brick_stairs", 3.0, 'P', 1, ""),
    ("blackstone_slab", 2.0, 'P', 1, ""),
    ("blackstone_stairs", 1.5, 'P', 1, ""),
    ("mossy_cobblestone_slab", 2.0, 'P', 1, ""),
    ("mossy_cobblestone_stairs", 2.0, 'P', 1, ""),
    ("mud_brick_slab", 1.5, 'P', 1, ""),
    ("mud_brick_stairs", 1.5, 'P', 1, ""),
    ("iron_bars", 5.0, 'P', 1, ""),
    ("chain", 5.0, 'P', 1, ""),
    ("lantern", 3.5, 'P', 1, ""),
    ("anvil", 5.0, 'P', 1, ""),
    ("enchanting_table", 5.0, 'P', 1, ""),
    ("hopper", 3.0, 'P', 1, ""),
    ("observer", 3.0, 'P', 1, ""),
    ("dispenser", 3.5, 'P', 1, ""),
    ("dropper", 3.5, 'P', 1, ""),
    ("crafter", 1.5, 'P', 1, ""),
    ("piston", 1.5, 'P', 0, ""),
    ("sticky_piston", 1.5, 'P', 0, ""),
    ("redstone_lamp", 0.3, '-', 0, ""),
    ("rail", 0.7, 'P', 0, ""),
    ("powered_rail", 0.7, 'P', 0, ""),
    ("lever", 0.5, '-', 0, ""),
    ("stone_button", 0.5, 'P', 0, ""),
    ("oak_button", 0.5, 'A', 0, ""),
    ("stone_pressure_plate", 0.5, 'P', 1, ""),
    ("oak_pressure_plate", 0.5, 'A', 0, ""),
    ("redstone_wire", 0.0, '-', 0, "redstone"),
    ("repeater", 0.0, '-', 0, ""),
    ("comparator", 0.0, '-', 0, ""),
    ("note_block", 0.8, 'A', 0, ""),
    ("jukebox", 2.0, 'A', 0, ""),
    ("composter", 0.6, 'A', 0, ""),
    ("end_portal_frame", -1.0, '?', 0, "-"),
    ("end_portal", -1.0, '?', 0, "-"),
    ("end_gateway", -1.0, '?', 0, "-"),
    ("nether_portal", -1.0, '?', 0, "-"),
    ("dragon_egg", 3.0, '-', 0, ""),
    ("respawn_anchor", 50.0, 'P', 4, ""),
    ("end_rod", 0.0, '-', 0, ""),
    ("shulker_box", 2.0, 'P', 0, ""),
    ("moss_block", 0.1, 'H', 0, ""),
    ("dripstone_block", 1.5, 'P', 1, ""),
    ("pointed_dripstone", 1.5, 'P', 0, ""),
    ("honeycomb_block", 0.6, '-', 0, ""),
    ("bee_nest", 0.3, 'A', 0, "-"),
    ("beehive", 0.6, 'A', 0, ""),
    ("red_mushroom_block", 0.2, 'A', 0, "-"),
    ("mushroom_stem", 0.2, 'A', 0, "-"),
    ("chorus_plant", 0.4, 'A', 0, "-"),
    ("chorus_flower", 0.4, 'A', 0, "-"),
    ("cut_copper", 3.0, 'P', 2, ""),
    ("copper_grate", 3.0, 'P', 2, ""),
    ("copper_door", 3.0, 'P', 2, ""),
    ("copper_trapdoor", 3.0, 'P', 2, ""),
    ("copper_bulb", 3.0, 'P', 2, ""),
    ("chiseled_copper", 3.0, 'P', 2, ""),
    ("wither_skeleton_skull", 1.0, '-', 0, ""),
    ("dragon_head", 1.0, '-', 0, ""),
    ("sculk", 0.2, 'H', 0, "-"),
    ("poppy", 0.0, '-', 0, ""),
    ("dandelion", 0.0, '-', 0, ""),
    ("short_grass", 0.0, '-', 0, "-"),
    ("oak_sapling", 0.0, '-', 0, ""),
    ("vine", 0.2, '?', 0, "-"),
    ("lily_pad", 0.0, '-', 0, ""),
    ("sugar_cane", 0.0, '-', 0, ""),
    ("dead_bush", 0.0, '-', 0, "stick"),
];

/// Tools: name, tier, mining speed, attack damage (with the hand's 1), durability.
const TOOLS: &[(&str, u8, f32, f32, u16)] = &[
    ("wooden_pickaxe", 1, 2.0, 2.0, 59),
    ("wooden_axe", 1, 2.0, 7.0, 59),
    ("wooden_shovel", 1, 2.0, 2.5, 59),
    ("wooden_sword", 1, 2.0, 4.0, 59),
    ("wooden_hoe", 1, 2.0, 1.0, 59),
    ("stone_pickaxe", 2, 4.0, 3.0, 131),
    ("stone_axe", 2, 4.0, 9.0, 131),
    ("stone_shovel", 2, 4.0, 3.5, 131),
    ("stone_sword", 2, 4.0, 5.0, 131),
    ("stone_hoe", 2, 4.0, 1.0, 131),
    ("iron_pickaxe", 3, 6.0, 4.0, 250),
    ("iron_axe", 3, 6.0, 9.0, 250),
    ("iron_shovel", 3, 6.0, 4.5, 250),
    ("iron_sword", 3, 6.0, 6.0, 250),
    ("iron_hoe", 3, 6.0, 1.0, 250),
    ("golden_pickaxe", 1, 12.0, 2.0, 32),
    ("golden_axe", 1, 12.0, 7.0, 32),
    ("golden_shovel", 1, 12.0, 2.5, 32),
    ("golden_sword", 1, 12.0, 4.0, 32),
    ("golden_hoe", 1, 12.0, 1.0, 32),
    ("diamond_pickaxe", 4, 8.0, 5.0, 1561),
    ("diamond_axe", 4, 8.0, 9.0, 1561),
    ("diamond_shovel", 4, 8.0, 5.5, 1561),
    ("diamond_sword", 4, 8.0, 7.0, 1561),
    ("diamond_hoe", 4, 8.0, 1.0, 1561),
    ("netherite_pickaxe", 5, 9.0, 6.0, 2031),
    ("netherite_axe", 5, 9.0, 10.0, 2031),
    ("netherite_shovel", 5, 9.0, 6.5, 2031),
    ("netherite_sword", 5, 9.0, 8.0, 2031),
    ("netherite_hoe", 5, 9.0, 1.0, 2031),
];

/// Armor: name, slot (0 head .. 3 feet), armor points, durability.
const ARMOR: &[(&str, u8, u8, u16)] = &[
    ("leather_helmet", 0, 1, 55),
    ("leather_chestplate", 1, 3, 80),
    ("leather_leggings", 2, 2, 75),
    ("leather_boots", 3, 1, 65),
    ("chainmail_helmet", 0, 2, 165),
    ("chainmail_chestplate", 1, 5, 240),
    ("chainmail_leggings", 2, 4, 225),
    ("chainmail_boots", 3, 1, 195),
    ("iron_helmet", 0, 2, 165),
    ("iron_chestplate", 1, 6, 240),
    ("iron_leggings", 2, 5, 225),
    ("iron_boots", 3, 2, 195),
    ("golden_helmet", 0, 2, 77),
    ("golden_chestplate", 1, 5, 112),
    ("golden_leggings", 2, 3, 105),
    ("golden_boots", 3, 1, 91),
    ("diamond_helmet", 0, 3, 363),
    ("diamond_chestplate", 1, 8, 528),
    ("diamond_leggings", 2, 6, 495),
    ("diamond_boots", 3, 3, 429),
    ("netherite_helmet", 0, 3, 407),
    ("netherite_chestplate", 1, 8, 592),
    ("netherite_leggings", 2, 6, 555),
    ("netherite_boots", 3, 3, 481),
    ("turtle_helmet", 0, 2, 275),
];

/// Food: name, hunger points, saturation.
const FOOD: &[(&str, u8, f32)] = &[
    ("apple", 4, 2.4),
    ("baked_potato", 5, 6.0),
    ("beef", 3, 1.8),
    ("cooked_beef", 8, 12.8),
    ("bread", 5, 6.0),
    ("carrot", 3, 3.6),
    ("chicken", 2, 1.2),
    ("cooked_chicken", 6, 7.2),
    ("porkchop", 3, 1.8),
    ("cooked_porkchop", 8, 12.8),
    ("mutton", 2, 1.2),
    ("cooked_mutton", 6, 9.6),
    ("cod", 2, 0.4),
    ("cooked_cod", 5, 6.0),
    ("salmon", 2, 0.4),
    ("cooked_salmon", 6, 9.6),
    ("rabbit", 3, 1.8),
    ("cooked_rabbit", 5, 6.0),
    ("golden_apple", 4, 9.6),
    ("enchanted_golden_apple", 4, 9.6),
    ("golden_carrot", 6, 14.4),
    ("melon_slice", 2, 1.2),
    ("cookie", 2, 0.4),
    ("pumpkin_pie", 8, 4.8),
    ("rotten_flesh", 4, 0.8),
    ("potato", 1, 0.6),
    ("poisonous_potato", 2, 1.2),
    ("sweet_berries", 2, 0.4),
    ("glow_berries", 2, 0.4),
    ("mushroom_stew", 6, 7.2),
    ("beetroot", 1, 1.2),
    ("beetroot_soup", 6, 7.2),
    ("rabbit_stew", 10, 12.0),
    ("dried_kelp", 1, 0.6),
    ("spider_eye", 2, 3.2),
    ("chorus_fruit", 4, 2.4),
    ("tropical_fish", 1, 0.2),
    ("pufferfish", 1, 0.2),
    ("suspicious_stew", 6, 7.2),
];

/// Stack sizes that are not 64 (tools and armor are checked by `TOOLS`/`ARMOR`).
const STACKS: &[(&str, u8)] = &[
    ("ender_pearl", 16),
    ("snowball", 16),
    ("egg", 16),
    ("bucket", 16),
    ("oak_sign", 16),
    ("honey_bottle", 16),
    ("white_banner", 16),
    ("armor_stand", 16),
    ("water_bucket", 1),
    ("lava_bucket", 1),
    ("milk_bucket", 1),
    ("mushroom_stew", 1),
    ("rabbit_stew", 1),
    ("beetroot_soup", 1),
    ("suspicious_stew", 1),
    ("cake", 1),
    ("red_bed", 1),
    ("saddle", 1),
    ("minecart", 1),
    ("oak_boat", 1),
    // Our potions are named by what is in them; the plain one is the water bottle.
    ("potion_water", 1),
    ("enchanted_book", 1),
    ("totem_of_undying", 1),
    ("bow", 1),
    ("crossbow", 1),
    ("trident", 1),
    ("shield", 1),
    ("fishing_rod", 1),
    ("flint_and_steel", 1),
    ("shears", 1),
    ("elytra", 1),
    ("writable_book", 1),
    ("written_book", 1),
    // Our discs carry our own music, so they are named after our tracks.
    ("music_disc_first_light", 1),
    ("white_shulker_box", 1),
    ("shulker_box", 1),
    ("arrow", 64),
    ("torch", 64),
    ("stick", 64),
    ("coal", 64),
    ("diamond", 64),
    ("bone", 64),
    ("experience_bottle", 64),
];

/// Durability of items that wear but are not tools or armor.
const WEAR: &[(&str, u16)] = &[
    ("bow", 384),
    ("crossbow", 465),
    ("trident", 250),
    ("shield", 336),
    ("fishing_rod", 64),
    ("flint_and_steel", 64),
    ("shears", 238),
    ("elytra", 432),
    ("carrot_on_a_stick", 25),
    ("brush", 64),
    ("mace", 500),
];

/// Furnace fuel in seconds of burning (one item smelts in 10 s).
const FUEL: &[(&str, f32)] = &[
    ("coal", 80.0),
    ("charcoal", 80.0),
    ("coal_block", 800.0),
    ("lava_bucket", 1000.0),
    ("blaze_rod", 120.0),
    ("oak_planks", 15.0),
    ("oak_log", 15.0),
    ("stick", 5.0),
    ("wooden_pickaxe", 10.0),
    ("oak_slab", 7.5),
    ("crafting_table", 15.0),
    ("chest", 15.0),
    ("bookshelf", 15.0),
    ("dried_kelp_block", 200.0),
    ("bamboo", 2.5),
    ("oak_sapling", 5.0),
    ("white_wool", 5.0),
    ("white_carpet", 3.35),
    ("bow", 15.0),
    ("ladder", 15.0),
];

/// Mobs: vanilla name, our counterpart, health, attack at Normal (0 = not checked), width, height.
const MOBS: &[(&str, &str, f32, f32, f32, f32)] = &[
    ("zombie", "zombie", 20.0, 3.0, 0.6, 1.95),
    ("husk", "mummy", 20.0, 3.0, 0.6, 1.95),
    ("drowned", "sunken", 20.0, 3.0, 0.6, 1.95),
    ("skeleton", "skeleton", 20.0, 0.0, 0.6, 1.99),
    ("stray", "frost_archer", 20.0, 0.0, 0.6, 1.99),
    ("creeper", "stalker", 20.0, 0.0, 0.6, 1.7),
    ("spider", "spider", 16.0, 2.0, 1.4, 0.9),
    ("enderman", "void_walker", 40.0, 7.0, 0.6, 2.9),
    ("witch", "swamp_witch", 26.0, 0.0, 0.6, 1.95),
    ("iron_golem", "sentinel", 100.0, 0.0, 1.4, 2.7),
    ("villager", "settler_farmer", 20.0, 0.0, 0.6, 1.95),
    ("phantom", "nightwing", 20.0, 6.0, 0.9, 0.5),
    ("guardian", "deep_watcher", 30.0, 6.0, 0.85, 0.85),
    ("pillager", "marauder", 24.0, 0.0, 0.6, 1.95),
    ("evoker", "invoker", 24.0, 0.0, 0.6, 1.95),
    ("cave_spider", "cave_spider", 12.0, 2.0, 0.7, 0.5),
    ("mooshroom", "mooshroom", 10.0, 0.0, 0.9, 1.4),
    ("glow_squid", "glow_squid", 10.0, 0.0, 0.8, 0.8),
    ("mule", "mule", 0.0, 0.0, 1.3965, 1.6),
    ("skeleton_horse", "skeleton_horse", 15.0, 0.0, 1.3965, 1.6),
    ("zoglin", "zoglin", 40.0, 0.0, 1.3965, 1.4),
    ("bogged", "bogged", 16.0, 0.0, 0.6, 1.99),
    ("zombie_villager", "zombie_villager", 20.0, 3.0, 0.6, 1.95),
    ("elder_guardian", "elder_guardian", 80.0, 8.0, 1.9975, 1.9975),
    ("silverfish", "silverfish", 8.0, 1.0, 0.4, 0.3),
    ("bat", "bat", 6.0, 0.0, 0.5, 0.9),
    ("snow_golem", "snow_golem", 4.0, 0.0, 0.7, 1.9),
    ("tropical_fish", "tropical_fish", 3.0, 0.0, 0.5, 0.4),
    ("tadpole", "tadpole", 6.0, 0.0, 0.4, 0.3),
    ("armadillo", "armadillo", 12.0, 0.0, 0.7, 0.65),
    ("allay", "allay", 20.0, 0.0, 0.35, 0.6),
    ("vex", "vex", 14.0, 9.0, 0.4, 0.8),
    ("ravager", "ravager", 100.0, 12.0, 1.95, 2.2),
    ("creaking", "creaking", 0.0, 3.0, 0.9, 2.7),
    ("armor_stand", "armor_stand", 0.0, 0.0, 0.5, 1.975),
    ("pig", "pig", 10.0, 0.0, 0.9, 0.9),
    ("cow", "cow", 10.0, 0.0, 0.9, 1.4),
    ("sheep", "sheep", 8.0, 0.0, 0.9, 1.3),
    ("chicken", "chicken", 4.0, 0.0, 0.4, 0.7),
    ("rabbit", "rabbit", 3.0, 0.0, 0.4, 0.5),
    ("fox", "fox", 10.0, 0.0, 0.6, 0.7),
    ("wolf", "wolf", 8.0, 4.0, 0.6, 0.85),
    ("wolf (domado)", "wolf_tame", 40.0, 4.0, 0.6, 0.85),
    ("cat", "cat", 10.0, 0.0, 0.6, 0.7),
    ("horse", "horse", 0.0, 0.0, 1.3965, 1.6),
    ("donkey", "donkey", 0.0, 0.0, 1.3965, 1.5),
    ("llama", "llama", 0.0, 0.0, 0.9, 1.87),
    ("goat", "goat", 10.0, 2.0, 0.9, 1.3),
    ("polar_bear", "polar_bear", 30.0, 6.0, 1.4, 1.4),
    ("bee", "bee", 10.0, 2.0, 0.7, 0.6),
    ("turtle", "turtle", 30.0, 0.0, 1.2, 0.4),
    ("dolphin", "dolphin", 10.0, 3.0, 0.9, 0.6),
    ("squid", "squid", 10.0, 0.0, 0.8, 0.8),
    ("axolotl", "axolotl", 14.0, 0.0, 0.75, 0.42),
    ("panda", "panda", 20.0, 0.0, 1.3, 1.25),
    ("parrot", "parrot", 6.0, 0.0, 0.5, 0.9),
    ("frog", "frog", 10.0, 0.0, 0.5, 0.5),
    ("camel", "camel", 32.0, 0.0, 1.7, 2.375),
    ("sniffer", "sniffer", 14.0, 0.0, 1.9, 1.75),
    ("strider", "strider", 20.0, 0.0, 0.9, 1.7),
    ("blaze", "blaze", 20.0, 0.0, 0.6, 1.8),
    ("ghast", "ghast", 10.0, 0.0, 4.0, 4.0),
    ("magma_cube (grande)", "magma_cube", 16.0, 0.0, 2.04, 2.04),
    ("piglin", "piglin", 16.0, 8.0, 0.6, 1.95),
    ("piglin_brute", "piglin_brute", 50.0, 13.0, 0.6, 1.95),
    ("zombified_piglin", "nether_zombie", 20.0, 8.0, 0.6, 1.95),
    ("hoglin", "hoglin", 40.0, 0.0, 1.3965, 1.4),
    ("wither_skeleton", "wither_skeleton", 20.0, 8.0, 0.7, 2.4),
    ("wither", "wither", 300.0, 0.0, 0.9, 3.5),
    ("shulker", "shulker", 30.0, 0.0, 1.0, 1.0),
    ("endermite", "endermite", 8.0, 2.0, 0.4, 0.3),
    ("ender_dragon", "ender_dragon", 200.0, 0.0, 16.0, 8.0),
    ("warden", "warden", 500.0, 30.0, 0.9, 2.9),
    ("breeze", "breeze", 30.0, 0.0, 0.6, 1.77),
];

/// Crafting: grid rows (3 wide, space = empty), letter → item, result, count.
const CRAFTS: &[(&[&str], &[(char, &str)], &str, u8)] = &[
    (&["P  ", "P  "], &[('P', "oak_planks")], "stick", 4),
    (&["L  "], &[('L', "oak_log")], "oak_planks", 4),
    (&["PP ", "PP "], &[('P', "oak_planks")], "crafting_table", 1),
    (&["CCC", "C C", "CCC"], &[('C', "cobblestone")], "furnace", 1),
    (&["PPP", "P P", "PPP"], &[('P', "oak_planks")], "chest", 1),
    (&["C  ", "S  "], &[('C', "coal"), ('S', "stick")], "torch", 4),
    (&["C  ", "S  "], &[('C', "charcoal"), ('S', "stick")], "torch", 4),
    (&["III", " S ", " S "], &[('I', "iron_ingot"), ('S', "stick")], "iron_pickaxe", 1),
    (&["DD ", "DS ", " S "], &[('D', "diamond"), ('S', "stick")], "diamond_axe", 1),
    (&["I  ", "I  ", "S  "], &[('I', "iron_ingot"), ('S', "stick")], "iron_sword", 1),
    (&["I I", " I "], &[('I', "iron_ingot")], "bucket", 1),
    (&[" I", "I "], &[('I', "iron_ingot")], "shears", 1),
    (&["WWW"], &[('W', "wheat")], "bread", 1),
    (&["S S", "SSS", "S S"], &[('S', "stick")], "ladder", 3),
    (&["PSP", "PSP"], &[('P', "oak_planks"), ('S', "stick")], "oak_fence", 3),
    (&["PPP", "PPP", " S "], &[('P', "oak_planks"), ('S', "stick")], "oak_sign", 3),
    (&["PP ", "PP ", "PP "], &[('P', "oak_planks")], "oak_door", 3),
    (&["PPP", "PPP"], &[('P', "oak_planks")], "oak_trapdoor", 2),
    (&["PPP"], &[('P', "oak_planks")], "oak_slab", 6),
    (&["P  ", "PP ", "PPP"], &[('P', "oak_planks")], "oak_stairs", 4),
    (&["GGG", "GGG"], &[('G', "glass")], "glass_pane", 16),
    (&["III", "III"], &[('I', "iron_ingot")], "iron_bars", 16),
    (&["PPP", "BBB", "PPP"], &[('P', "oak_planks"), ('B', "book")], "bookshelf", 1),
    (&["GSG", "SGS", "GSG"], &[('G', "gunpowder"), ('S', "sand")], "tnt", 1),
    (&["RRR"], &[('R', "sugar_cane")], "paper", 3),
    (&[" ST", "S T", " ST"], &[('S', "stick"), ('T', "string")], "bow", 1),
    (&["F  ", "S  ", "E  "], &[('F', "flint"), ('S', "stick"), ('E', "feather")], "arrow", 4),
    (&["I I", "ISI", "I I"], &[('I', "iron_ingot"), ('S', "stick")], "rail", 16),
    (&["CCC", "CCC", "CCC"], &[('C', "coal")], "coal_block", 1),
    (&["III", "III", "III"], &[('I', "iron_ingot")], "iron_block", 1),
    (&["B  "], &[('B', "iron_block")], "iron_ingot", 9),
    (&["N  "], &[('N', "iron_ingot")], "iron_nugget", 9),
    (&["SS ", "SS "], &[('S', "stone")], "stone_bricks", 4),
    (&["SS ", "SS "], &[('S', "sand")], "sandstone", 1),
    (&["BB ", "BB "], &[('B', "brick")], "bricks", 1),
    (&["CC ", "CC "], &[('C', "clay_ball")], "clay", 1),
    (&["WWW", "PPP"], &[('W', "white_wool"), ('P', "oak_planks")], "white_bed", 1),
    (&["WW "], &[('W', "white_wool")], "white_carpet", 3),
    (&["SS ", "SS "], &[('S', "string")], "white_wool", 1),
    (&[" I ", "IRI", " I "], &[('I', "iron_ingot"), ('R', "redstone")], "compass", 1),
    (&[" G ", "GRG", " G "], &[('G', "gold_ingot"), ('R', "redstone")], "clock", 1),
    (&["PPP", "CIC", "CRC"], &[('P', "oak_planks"), ('C', "cobblestone"), ('I', "iron_ingot"), ('R', "redstone")], "piston", 1),
    (&["R  ", "S  "], &[('R', "redstone"), ('S', "stick")], "redstone_torch", 1),
    (&["S  ", "C  "], &[('S', "stick"), ('C', "cobblestone")], "lever", 1),
    (&["I I", "III"], &[('I', "iron_ingot")], "minecart", 1),
    (&["P P", "PPP"], &[('P', "oak_planks")], "oak_boat", 1),
    (&["CCC", "CBC", "CRC"], &[('C', "cobblestone"), ('B', "bow"), ('R', "redstone")], "dispenser", 1),
    (&[" B ", "DOD", "OOO"], &[('B', "book"), ('D', "diamond"), ('O', "obsidian")], "enchanting_table", 1),
    (&["BBB", " I ", "III"], &[('B', "iron_block"), ('I', "iron_ingot")], "anvil", 1),
    (&["I I", "ICI", " I "], &[('I', "iron_ingot"), ('C', "chest")], "hopper", 1),
    (&["GGG", "GGG", "GGG"], &[('G', "gold_nugget")], "gold_ingot", 1),
    (&["GGG", "GAG", "GGG"], &[('G', "gold_ingot"), ('A', "apple")], "golden_apple", 1),
    (&["SSS", "SSS"], &[('S', "cobblestone")], "cobblestone_wall", 6),
    (&["MMM", "SES", "WWW"], &[('M', "milk_bucket"), ('S', "sugar"), ('E', "egg"), ('W', "wheat")], "cake", 1),
];

/// Furnace: input, result.
const SMELTS: &[(&str, &str)] = &[
    ("raw_iron", "iron_ingot"),
    ("raw_gold", "gold_ingot"),
    ("raw_copper", "copper_ingot"),
    ("sand", "glass"),
    ("cobblestone", "stone"),
    ("stone", "smooth_stone"),
    ("oak_log", "charcoal"),
    ("clay_ball", "brick"),
    ("clay", "terracotta"),
    ("netherrack", "nether_brick"),
    ("beef", "cooked_beef"),
    ("porkchop", "cooked_porkchop"),
    ("chicken", "cooked_chicken"),
    ("mutton", "cooked_mutton"),
    ("cod", "cooked_cod"),
    ("salmon", "cooked_salmon"),
    ("potato", "baked_potato"),
    ("cactus", "green_dye"),
    ("ancient_debris", "netherite_scrap"),
    ("iron_ore", "iron_ingot"),
    ("gold_ore", "gold_ingot"),
    ("diamond_ore", "diamond"),
    ("kelp", "dried_kelp"),
    ("wet_sponge", "sponge"),
    ("cobbled_deepslate", "deepslate"),
    ("sandstone", "smooth_sandstone"),
    ("quartz_block", "smooth_quartz"),
];

/// Differences kept on purpose: (row label as the report writes it, reason).
const ON_PURPOSE: &[(&str, &str)] = &[
    ("bloco amethyst_block: dureza", "a ametista é a mesma gema do lado Terraria; o bloco guarda 9 gemas"),
    ("bloco amethyst_block: nível", "a ametista é a mesma gema do lado Terraria; o bloco guarda 9 gemas"),
    ("bloco budding_amethyst: nível", "a ametista é a mesma gema do lado Terraria"),
    ("bloco amethyst_cluster: nível", "a ametista é a mesma gema do lado Terraria"),
    ("bloco amethyst_cluster: drop", "dá a gema ametista, que serve para as duas coisas"),
    ("mob ender_dragon: tamanho", "no vanilla a caixa é feita de várias partes; a nossa é a do corpo"),
];

fn tool_letter(t: Option<ToolKind>) -> char {
    match t {
        Some(ToolKind::Pickaxe) => 'P',
        Some(ToolKind::Axe) => 'A',
        Some(ToolKind::Shovel) => 'S',
        Some(ToolKind::Hoe) => 'H',
        Some(ToolKind::Sword) => 'W',
        None => '-',
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.051 * b.abs().max(1.0)
}

struct Report {
    fails: Vec<String>,
    kept: Vec<String>,
    missing: Vec<String>,
    checked: usize,
}

impl Report {
    fn check(&mut self, label: String, ok: bool, detail: String) {
        self.checked += 1;
        if ok {
            return;
        }
        match ON_PURPOSE.iter().find(|(l, _)| *l == label) {
            Some((_, why)) => self.kept.push(format!("- {label}: {detail} (de propósito: {why})")),
            None => self.fails.push(format!("- {label}: {detail}")),
        }
    }
}

#[test]
fn parity_with_vanilla() {
    let (reg, items, recipes) = crate::game_data(embedded::BLOCKS_RON, embedded::ITEMS_RON, embedded::RECIPES_RON).unwrap();
    let mobs_text = crate::terraria::with_extra(&crate::merge_list(embedded::MOBS_RON, embedded::FASE6_MOBS_RON), embedded::T5_MOBS_RON);
    let mobs_text = crate::terraria::with_extra(&crate::terraria::with_extra(&crate::terraria::with_extra(&mobs_text, embedded::TR_MOBS_RON), embedded::AGUAS_MOBS_RON), embedded::VULCAO_MOBS_RON);
    let mob_reg = voxel::mob::MobRegistry::from_ron_many(&[&mobs_text, embedded::MODS_MOBS_RON, embedded::MODS_DIMS_MOBS_RON]).unwrap();
    let mut r = Report { fails: vec![], kept: vec![], missing: vec![], checked: 0 };

    for &(name, hardness, tool, tier, drop) in BLOCKS {
        let Some(id) = reg.id(name) else {
            r.missing.push(format!("bloco {name}"));
            continue;
        };
        let b = reg.get(id);
        r.check(format!("bloco {name}: dureza"), close(b.hardness, hardness), format!("nosso {} · vanilla {hardness}", b.hardness));
        if tool != '?' {
            let ours = tool_letter(b.tool);
            // A block that needs no tool may leave the tool out; what matters is the speed-up.
            let ok = ours == tool || (tool == '-' && tier == 0);
            r.check(format!("bloco {name}: ferramenta"), ok, format!("nossa {ours} · vanilla {tool}"));
        }
        r.check(format!("bloco {name}: nível"), b.tier == tier, format!("nosso {} · vanilla {tier}", b.tier));
        if hardness < 0.0 {
            // Unbreakable: what it would drop never matters.
        } else if drop != "" {
            let drops: Vec<String> = match &b.drops {
                None => vec![name.to_string()],
                Some(d) => d.iter().map(|d| d.item.clone()).collect(),
            };
            let ok = if drop == "-" { drops.is_empty() || drops.iter().all(|d| d != name) } else { drops.iter().any(|d| d == drop) };
            r.check(format!("bloco {name}: drop"), ok, format!("nosso {drops:?} · vanilla {drop}"));
        } else if let Some(d) = &b.drops {
            let ok = d.iter().any(|d| d.item == name && d.chance >= 0.99);
            r.check(format!("bloco {name}: drop"), ok, format!("nosso {:?} · vanilla ele mesmo", d.iter().map(|d| &d.item).collect::<Vec<_>>()));
        }
    }

    let item = |name: &str| items.id(name).map(|i| items.get(i));
    for &(name, tier, speed, damage, dur) in TOOLS {
        let Some(it) = item(name) else {
            r.missing.push(format!("item {name}"));
            continue;
        };
        let Some(t) = &it.tool else {
            r.fails.push(format!("- item {name}: não é ferramenta"));
            continue;
        };
        r.check(format!("item {name}: nível"), t.tier == tier, format!("nosso {} · vanilla {tier}", t.tier));
        r.check(format!("item {name}: velocidade"), close(t.speed, speed), format!("nossa {} · vanilla {speed}", t.speed));
        r.check(format!("item {name}: dano"), close(t.damage, damage), format!("nosso {} · vanilla {damage}", t.damage));
        r.check(format!("item {name}: durabilidade"), t.durability == dur, format!("nossa {} · vanilla {dur}", t.durability));
        r.check(format!("item {name}: pilha"), it.max_stack == 1, format!("nossa {} · vanilla 1", it.max_stack));
    }
    for &(name, slot, points, dur) in ARMOR {
        let Some(it) = item(name) else {
            r.missing.push(format!("item {name}"));
            continue;
        };
        let Some(a) = &it.armor else {
            r.fails.push(format!("- item {name}: não é armadura"));
            continue;
        };
        r.check(format!("item {name}: encaixe"), a.slot == slot, format!("nosso {} · vanilla {slot}", a.slot));
        r.check(format!("item {name}: pontos"), a.points == points, format!("nossos {} · vanilla {points}", a.points));
        r.check(format!("item {name}: durabilidade"), a.durability == dur, format!("nossa {} · vanilla {dur}", a.durability));
    }
    for &(name, hunger, sat) in FOOD {
        let Some(it) = item(name) else {
            r.missing.push(format!("comida {name}"));
            continue;
        };
        let Some(f) = &it.food else {
            r.fails.push(format!("- comida {name}: não é comestível"));
            continue;
        };
        r.check(format!("comida {name}: fome"), f.hunger == hunger, format!("nossa {} · vanilla {hunger}", f.hunger));
        r.check(format!("comida {name}: saturação"), close(f.saturation, sat), format!("nossa {} · vanilla {sat}", f.saturation));
    }
    for &(name, n) in STACKS {
        let Some(it) = item(name) else {
            r.missing.push(format!("item {name}"));
            continue;
        };
        r.check(format!("item {name}: pilha"), it.max_stack == n, format!("nossa {} · vanilla {n}", it.max_stack));
    }
    for &(name, dur) in WEAR {
        let Some(it) = item(name) else {
            r.missing.push(format!("item {name}"));
            continue;
        };
        let ours = it.max_wear();
        if ours.is_none() {
            r.missing.push(format!("desgaste de {name} (não gasta)"));
            continue;
        }
        r.check(format!("item {name}: durabilidade"), ours == Some(dur), format!("nossa {ours:?} · vanilla {dur}"));
    }
    for &(name, secs) in FUEL {
        let Some(it) = item(name) else {
            r.missing.push(format!("combustível {name}"));
            continue;
        };
        r.check(format!("combustível {name}"), close(it.fuel, secs), format!("nosso {} s · vanilla {secs} s", it.fuel));
    }

    for &(vanilla, ours, health, attack, w, h) in MOBS {
        let Some(m) = mob_reg.id(ours).map(|i| &mob_reg.defs[i]) else {
            r.missing.push(format!("mob {vanilla} ({ours})"));
            continue;
        };
        if health > 0.0 {
            r.check(format!("mob {vanilla}: vida"), close(m.health, health), format!("{ours} {} · vanilla {health}", m.health));
        }
        if attack > 0.0 {
            r.check(format!("mob {vanilla}: ataque"), close(m.attack, attack), format!("{ours} {} · vanilla {attack}", m.attack));
        }
        let size_ok = (m.width - w).abs() <= 0.15 * w.max(0.5) && (m.height - h).abs() <= 0.15 * h.max(0.5);
        r.check(format!("mob {vanilla}: tamanho"), size_ok, format!("{ours} {}×{} · vanilla {w}×{h}", m.width, m.height));
    }

    for &(rows, key, result, count) in CRAFTS {
        let Some(want) = items.id(result) else {
            r.missing.push(format!("receita de {result} (item falta)"));
            continue;
        };
        let mut grid = vec![None; 9];
        let mut absent = None;
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                if c == ' ' {
                    continue;
                }
                let name = key.iter().find(|(k, _)| *k == c).map(|(_, n)| *n).unwrap();
                match items.id(name) {
                    Some(i) => grid[y * 3 + x] = Some(ItemStack::new(i, 1)),
                    None => absent = Some(name),
                }
            }
        }
        if let Some(name) = absent {
            r.missing.push(format!("receita de {result} (ingrediente {name} falta)"));
            continue;
        }
        let got = recipes.craft(&grid, 3);
        let ok = got.is_some_and(|s| s.item == want && (count == 0 || s.count == count));
        let ours = got.map(|s| format!("{} ×{}", items.get(s.item).name, s.count)).unwrap_or_else(|| "nada".into());
        r.check(format!("receita {result}"), ok, format!("nossa {ours} · vanilla {result} ×{count}"));
    }
    for &(input, result) in SMELTS {
        let (Some(i), Some(want)) = (items.id(input), items.id(result)) else {
            r.missing.push(format!("fornalha {input} → {result}"));
            continue;
        };
        let got = recipes.smelt(i).map(|s| s.result.item);
        let ours = got.map(|g| items.get(g).name.clone()).unwrap_or_else(|| "nada".into());
        r.check(format!("fornalha {input}"), got == Some(want), format!("nossa {ours} · vanilla {result}"));
    }

    let mut out = format!(
        "# Paridade com o Minecraft Java\n\n{} verificações, {} diferenças, {} mantidas de propósito, {} coisas que faltam.\n\n## Diferenças\n\n{}\n\n## Mantidas de propósito\n\n{}\n\n## Faltam no jogo\n\n{}\n",
        r.checked,
        r.fails.len(),
        r.kept.len(),
        r.missing.len(),
        r.fails.join("\n"),
        r.kept.join("\n"),
        r.missing.iter().map(|m| format!("- {m}")).collect::<Vec<_>>().join("\n"),
    );
    out = out.replace("\n\n\n", "\n\n");
    println!("{out}");
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/parity-report.md");
    let _ = std::fs::write(path, &out);
    assert!(r.fails.is_empty(), "{} diferenças com o vanilla (veja target/parity-report.md)", r.fails.len());
}
