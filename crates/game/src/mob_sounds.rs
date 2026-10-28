//! Real recorded creature sounds (free licences, see assets/sounds/mobs/CREDITS.md).
//!
//! Each mob family has its own set of recordings for idle, hurt and death calls. The same clip
//! is played slower for big creatures and faster for small ones, with a little random spread,
//! so a herd never sounds like one sample repeated.

use crate::sfx_extra::{Call, Fx};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex, OnceLock};

const RATE: f32 = 44100.0;

/// Every recording, by name (Ogg Vorbis, mono).
static CLIPS: &[(&str, &[u8])] = &[
    ("bear_attack_1", include_bytes!("../../../assets/sounds/mobs/bear_attack_1.ogg")),
    ("bear_attack_2", include_bytes!("../../../assets/sounds/mobs/bear_attack_2.ogg")),
    ("bear_death_1", include_bytes!("../../../assets/sounds/mobs/bear_death_1.ogg")),
    ("bear_growl_1", include_bytes!("../../../assets/sounds/mobs/bear_growl_1.ogg")),
    ("bear_growl_2", include_bytes!("../../../assets/sounds/mobs/bear_growl_2.ogg")),
    ("bear_growl_3", include_bytes!("../../../assets/sounds/mobs/bear_growl_3.ogg")),
    ("bear_hurt_1", include_bytes!("../../../assets/sounds/mobs/bear_hurt_1.ogg")),
    ("bear_random_1", include_bytes!("../../../assets/sounds/mobs/bear_random_1.ogg")),
    ("bear_random_2", include_bytes!("../../../assets/sounds/mobs/bear_random_2.ogg")),
    ("bear_random_3", include_bytes!("../../../assets/sounds/mobs/bear_random_3.ogg")),
    ("cat_idle_1", include_bytes!("../../../assets/sounds/mobs/cat_idle_1.ogg")),
    ("cat_idle_2", include_bytes!("../../../assets/sounds/mobs/cat_idle_2.ogg")),
    ("chicken_buck_1", include_bytes!("../../../assets/sounds/mobs/chicken_buck_1.ogg")),
    ("chicken_buck_2", include_bytes!("../../../assets/sounds/mobs/chicken_buck_2.ogg")),
    ("chicken_buck_3", include_bytes!("../../../assets/sounds/mobs/chicken_buck_3.ogg")),
    ("chicken_hurt", include_bytes!("../../../assets/sounds/mobs/chicken_hurt.ogg")),
    ("cow", include_bytes!("../../../assets/sounds/mobs/cow.ogg")),
    ("cow_hurt", include_bytes!("../../../assets/sounds/mobs/cow_hurt.ogg")),
    ("donkey_death", include_bytes!("../../../assets/sounds/mobs/donkey_death.ogg")),
    ("donkey_hurt", include_bytes!("../../../assets/sounds/mobs/donkey_hurt.ogg")),
    ("donkey_random_1", include_bytes!("../../../assets/sounds/mobs/donkey_random_1.ogg")),
    ("donkey_random_2", include_bytes!("../../../assets/sounds/mobs/donkey_random_2.ogg")),
    ("enderman_death", include_bytes!("../../../assets/sounds/mobs/enderman_death.ogg")),
    ("enderman_hurt_1", include_bytes!("../../../assets/sounds/mobs/enderman_hurt_1.ogg")),
    ("enderman_hurt_2", include_bytes!("../../../assets/sounds/mobs/enderman_hurt_2.ogg")),
    ("enderman_hurt_3", include_bytes!("../../../assets/sounds/mobs/enderman_hurt_3.ogg")),
    ("enderman_random_1", include_bytes!("../../../assets/sounds/mobs/enderman_random_1.ogg")),
    ("endermite_death_1", include_bytes!("../../../assets/sounds/mobs/endermite_death_1.ogg")),
    ("green_slime_damage", include_bytes!("../../../assets/sounds/mobs/green_slime_damage.ogg")),
    ("green_slime_death", include_bytes!("../../../assets/sounds/mobs/green_slime_death.ogg")),
    ("green_slime_jump", include_bytes!("../../../assets/sounds/mobs/green_slime_jump.ogg")),
    ("green_slime_land", include_bytes!("../../../assets/sounds/mobs/green_slime_land.ogg")),
    ("guardian_death", include_bytes!("../../../assets/sounds/mobs/guardian_death.ogg")),
    ("guardian_random_1", include_bytes!("../../../assets/sounds/mobs/guardian_random_1.ogg")),
    ("guardian_random_2", include_bytes!("../../../assets/sounds/mobs/guardian_random_2.ogg")),
    ("guardian_random_3", include_bytes!("../../../assets/sounds/mobs/guardian_random_3.ogg")),
    ("hoof_1", include_bytes!("../../../assets/sounds/mobs/hoof_1.ogg")),
    ("hoof_2", include_bytes!("../../../assets/sounds/mobs/hoof_2.ogg")),
    ("hoof_3", include_bytes!("../../../assets/sounds/mobs/hoof_3.ogg")),
    ("horse_death", include_bytes!("../../../assets/sounds/mobs/horse_death.ogg")),
    ("horse_hurt", include_bytes!("../../../assets/sounds/mobs/horse_hurt.ogg")),
    ("horse_random_1", include_bytes!("../../../assets/sounds/mobs/horse_random_1.ogg")),
    ("horse_random_2", include_bytes!("../../../assets/sounds/mobs/horse_random_2.ogg")),
    ("llama", include_bytes!("../../../assets/sounds/mobs/llama.ogg")),
    ("magma_cube_big", include_bytes!("../../../assets/sounds/mobs/magma_cube_big.ogg")),
    ("magma_cube_small", include_bytes!("../../../assets/sounds/mobs/magma_cube_small.ogg")),
    ("ocelot_hurt", include_bytes!("../../../assets/sounds/mobs/ocelot_hurt.ogg")),
    ("pig", include_bytes!("../../../assets/sounds/mobs/pig.ogg")),
    ("pig_angry", include_bytes!("../../../assets/sounds/mobs/pig_angry.ogg")),
    ("rabbit_death_1", include_bytes!("../../../assets/sounds/mobs/rabbit_death_1.ogg")),
    ("rabbit_death_2", include_bytes!("../../../assets/sounds/mobs/rabbit_death_2.ogg")),
    ("rabbit_death_3", include_bytes!("../../../assets/sounds/mobs/rabbit_death_3.ogg")),
    ("rabbit_hurt_1", include_bytes!("../../../assets/sounds/mobs/rabbit_hurt_1.ogg")),
    ("rabbit_hurt_2", include_bytes!("../../../assets/sounds/mobs/rabbit_hurt_2.ogg")),
    ("rabbit_hurt_3", include_bytes!("../../../assets/sounds/mobs/rabbit_hurt_3.ogg")),
    ("sheep", include_bytes!("../../../assets/sounds/mobs/sheep.ogg")),
    ("skeleton_death", include_bytes!("../../../assets/sounds/mobs/skeleton_death.ogg")),
    ("skeleton_random_1", include_bytes!("../../../assets/sounds/mobs/skeleton_random_1.ogg")),
    ("skeleton_random_2", include_bytes!("../../../assets/sounds/mobs/skeleton_random_2.ogg")),
    ("spider_attack_1", include_bytes!("../../../assets/sounds/mobs/spider_attack_1.ogg")),
    ("spider_attack_2", include_bytes!("../../../assets/sounds/mobs/spider_attack_2.ogg")),
    ("spider_death", include_bytes!("../../../assets/sounds/mobs/spider_death.ogg")),
    ("spider_hurt_1", include_bytes!("../../../assets/sounds/mobs/spider_hurt_1.ogg")),
    ("spider_random", include_bytes!("../../../assets/sounds/mobs/spider_random.ogg")),
    ("squid_death_1", include_bytes!("../../../assets/sounds/mobs/squid_death_1.ogg")),
    ("squid_hurt_1", include_bytes!("../../../assets/sounds/mobs/squid_hurt_1.ogg")),
    ("squid_hurt_2", include_bytes!("../../../assets/sounds/mobs/squid_hurt_2.ogg")),
    ("vex_death", include_bytes!("../../../assets/sounds/mobs/vex_death.ogg")),
    ("vex_hurt", include_bytes!("../../../assets/sounds/mobs/vex_hurt.ogg")),
    ("villager_1", include_bytes!("../../../assets/sounds/mobs/villager_1.ogg")),
    ("villager_2", include_bytes!("../../../assets/sounds/mobs/villager_2.ogg")),
    ("villager_3", include_bytes!("../../../assets/sounds/mobs/villager_3.ogg")),
    ("villager_4", include_bytes!("../../../assets/sounds/mobs/villager_4.ogg")),
    ("villager_5", include_bytes!("../../../assets/sounds/mobs/villager_5.ogg")),
    ("villager_hurt_1", include_bytes!("../../../assets/sounds/mobs/villager_hurt_1.ogg")),
    ("villager_hurt_2", include_bytes!("../../../assets/sounds/mobs/villager_hurt_2.ogg")),
    ("villager_trade_1", include_bytes!("../../../assets/sounds/mobs/villager_trade_1.ogg")),
    ("villager_trade_2", include_bytes!("../../../assets/sounds/mobs/villager_trade_2.ogg")),
    ("villager_trade_3", include_bytes!("../../../assets/sounds/mobs/villager_trade_3.ogg")),
    ("villager_trade_4", include_bytes!("../../../assets/sounds/mobs/villager_trade_4.ogg")),
    ("vl_elemental_fire_breath", include_bytes!("../../../assets/sounds/mobs/vl_elemental_fire_breath.ogg")),
    ("vl_elemental_fire_died", include_bytes!("../../../assets/sounds/mobs/vl_elemental_fire_died.ogg")),
    ("vl_elemental_fire_hurt", include_bytes!("../../../assets/sounds/mobs/vl_elemental_fire_hurt.ogg")),
    ("vl_witch_death", include_bytes!("../../../assets/sounds/mobs/vl_witch_death.ogg")),
    ("vl_witch_laugh", include_bytes!("../../../assets/sounds/mobs/vl_witch_laugh.ogg")),
    ("vl_witch_laugh_2", include_bytes!("../../../assets/sounds/mobs/vl_witch_laugh_2.ogg")),
    ("wagon_creak_1", include_bytes!("../../../assets/sounds/mobs/wagon_creak_1.ogg")),
    ("wagon_creak_2", include_bytes!("../../../assets/sounds/mobs/wagon_creak_2.ogg")),
    ("wagon_gravel_1", include_bytes!("../../../assets/sounds/mobs/wagon_gravel_1.ogg")),
    ("wagon_gravel_2", include_bytes!("../../../assets/sounds/mobs/wagon_gravel_2.ogg")),
    ("wolf_bark_1", include_bytes!("../../../assets/sounds/mobs/wolf_bark_1.ogg")),
    ("wolf_bark_2", include_bytes!("../../../assets/sounds/mobs/wolf_bark_2.ogg")),
    ("wolf_bark_3", include_bytes!("../../../assets/sounds/mobs/wolf_bark_3.ogg")),
    ("wolf_death", include_bytes!("../../../assets/sounds/mobs/wolf_death.ogg")),
    ("wolf_growl", include_bytes!("../../../assets/sounds/mobs/wolf_growl.ogg")),
    ("wolf_hurt_1", include_bytes!("../../../assets/sounds/mobs/wolf_hurt_1.ogg")),
    ("wolf_hurt_3", include_bytes!("../../../assets/sounds/mobs/wolf_hurt_3.ogg")),
    ("zombie_death", include_bytes!("../../../assets/sounds/mobs/zombie_death.ogg")),
    ("zombie_growl", include_bytes!("../../../assets/sounds/mobs/zombie_growl.ogg")),
    ("zombie_hurt", include_bytes!("../../../assets/sounds/mobs/zombie_hurt.ogg")),
    ("zombiepig_death_1", include_bytes!("../../../assets/sounds/mobs/zombiepig_death_1.ogg")),
    ("zombiepig_death_2", include_bytes!("../../../assets/sounds/mobs/zombiepig_death_2.ogg")),
    ("zombiepig_hurt_1", include_bytes!("../../../assets/sounds/mobs/zombiepig_hurt_1.ogg")),
    ("zombiepig_hurt_2", include_bytes!("../../../assets/sounds/mobs/zombiepig_hurt_2.ogg")),
    ("zombiepig_hurt_3", include_bytes!("../../../assets/sounds/mobs/zombiepig_hurt_3.ogg")),
    ("zombiepig_random_1", include_bytes!("../../../assets/sounds/mobs/zombiepig_random_1.ogg")),
];

/// The recordings one creature family uses.
struct Set {
    idle: &'static [&'static str],
    hurt: &'static [&'static str],
    death: &'static [&'static str],
}

const COW: Set = Set { idle: &["cow"], hurt: &["cow_hurt"], death: &["cow_hurt"] };
const PIG: Set = Set { idle: &["pig"], hurt: &["pig_angry"], death: &["pig_angry"] };
const SHEEP: Set = Set { idle: &["sheep"], hurt: &["sheep"], death: &["sheep"] };
const CHICKEN: Set = Set {
    idle: &["chicken_buck_1", "chicken_buck_2", "chicken_buck_3"],
    hurt: &["chicken_hurt"],
    death: &["chicken_hurt"],
};
const SQUID: Set = Set { idle: &[], hurt: &["squid_hurt_1", "squid_hurt_2"], death: &["squid_death_1"] };
const GUARDIAN: Set = Set {
    idle: &["guardian_random_1", "guardian_random_2", "guardian_random_3"],
    hurt: &["guardian_random_2"],
    death: &["guardian_death"],
};
const VILLAGER: Set = Set {
    idle: &[
        "villager_1", "villager_2", "villager_3", "villager_4", "villager_5", "villager_trade_1",
        "villager_trade_2", "villager_trade_3", "villager_trade_4",
    ],
    hurt: &["villager_hurt_1", "villager_hurt_2"],
    death: &["villager_hurt_1", "villager_hurt_2"],
};
const ZOMBIE_PIG: Set = Set {
    idle: &["zombiepig_random_1"],
    hurt: &["zombiepig_hurt_1", "zombiepig_hurt_2", "zombiepig_hurt_3"],
    death: &["zombiepig_death_1", "zombiepig_death_2"],
};
const ZOMBIE: Set = Set { idle: &["zombie_growl"], hurt: &["zombie_hurt"], death: &["zombie_death"] };
const SKELETON: Set = Set {
    idle: &["skeleton_random_1", "skeleton_random_2"],
    hurt: &["skeleton_death"],
    death: &["skeleton_death"],
};
const SPIDER: Set = Set {
    idle: &["spider_random", "spider_attack_1", "spider_attack_2"],
    hurt: &["spider_hurt_1"],
    death: &["spider_death"],
};
const ENDER: Set = Set {
    idle: &["enderman_random_1"],
    hurt: &["enderman_hurt_1", "enderman_hurt_2", "enderman_hurt_3"],
    death: &["enderman_death"],
};
const LICH: Set = Set { idle: &["enderman_random_1"], hurt: &["vex_hurt"], death: &["vex_death"] };
const FIRE: Set = Set {
    idle: &["vl_elemental_fire_breath"],
    hurt: &["vl_elemental_fire_hurt"],
    death: &["vl_elemental_fire_died"],
};
const MAGMA: Set = Set { idle: &["magma_cube_big"], hurt: &["green_slime_damage"], death: &["green_slime_death"] };
const MAGMA_SMALL: Set = Set { idle: &["magma_cube_small"], hurt: &["green_slime_damage"], death: &["green_slime_death"] };
const SLIME: Set = Set {
    idle: &["green_slime_jump", "green_slime_land"],
    hurt: &["green_slime_damage"],
    death: &["green_slime_death"],
};
const MITE: Set = Set { idle: &[], hurt: &["endermite_death_1"], death: &["endermite_death_1"] };
const WITCH: Set = Set {
    idle: &["vl_witch_laugh", "vl_witch_laugh_2"],
    hurt: &["villager_hurt_1", "villager_hurt_2"],
    death: &["vl_witch_death"],
};
const HORSE: Set = Set { idle: &["horse_random_1", "horse_random_2"], hurt: &["horse_hurt"], death: &["horse_death"] };
const DONKEY: Set = Set { idle: &["donkey_random_1", "donkey_random_2"], hurt: &["donkey_hurt"], death: &["donkey_death"] };
const LLAMA: Set = Set { idle: &["llama"], hurt: &["llama"], death: &["llama"] };
const WOLF: Set = Set {
    idle: &["wolf_bark_1", "wolf_bark_2", "wolf_bark_3", "wolf_growl"],
    hurt: &["wolf_hurt_1", "wolf_hurt_3"],
    death: &["wolf_death"],
};
const CAT: Set = Set { idle: &["cat_idle_1", "cat_idle_2"], hurt: &["ocelot_hurt"], death: &["ocelot_hurt"] };
// Rabbits keep quiet until hurt (their squeaks were too shrill).
const RABBIT: Set = Set {
    idle: &[],
    hurt: &["rabbit_hurt_1", "rabbit_hurt_2", "rabbit_hurt_3"],
    death: &["rabbit_death_1", "rabbit_death_2", "rabbit_death_3"],
};
const BEAR: Set = Set {
    idle: &["bear_random_1", "bear_random_2", "bear_random_3", "bear_growl_1", "bear_growl_2", "bear_growl_3"],
    hurt: &["bear_hurt_1", "bear_attack_1", "bear_attack_2"],
    death: &["bear_death_1"],
};

/// Which recordings a mob uses and how fast it plays them (below 1: deeper). None: it stays
/// quiet rather than borrow a sound that doesn't fit it (insects, fish, birds, machines).
fn set_of(name: &str) -> Option<(&'static Set, f32)> {
    let has = |s: &str| name.contains(s);
    Some(match name {
        "cow" | "flying_cow" => (&COW, 1.0),
        "mooshroom" => (&COW, 0.95),
        "ravager" => (&COW, 0.55),
        "zoglin" => (&PIG, 0.75),
        "skeleton_horse" => (&HORSE, 0.85),
        "mule" => (&DONKEY, 0.9),
        "cave_spider" => (&SPIDER, 0.95),
        "silverfish" => (&MITE, 0.85),
        "glow_squid" | "tropical_fish" | "tadpole" => (&SQUID, 1.0),
        "elder_guardian" => (&GUARDIAN, 0.55),
        "bogged" => (&SKELETON, 0.9),
        "creaking" => (&GUARDIAN, 0.6),
        "armadillo" | "bat" => (&RABBIT, 0.8),
        "allay" | "vex" | "snow_golem" => (&VILLAGER, 0.95),
        "moose" | "bison" => (&COW, 0.8),
        "ox" => (&COW, 0.72),
        "hippo" => (&COW, 0.8),
        "elephant" => (&COW, 0.7),
        "rhino" => (&COW, 0.85),
        "camel" => (&COW, 0.9),
        "sniffer" => (&COW, 0.75),
        "pig" | "boar" | "phyg" => (&PIG, 1.0),
        "tusklin" => (&PIG, 0.9),
        "crimson_boar" | "hoglin" => (&PIG, 0.8),
        "sheep" | "sheepuff" => (&SHEEP, 1.0),
        "goat" => (&SHEEP, 0.9),
        "chicken" => (&CHICKEN, 1.0),
        "duck" => (&CHICKEN, 0.85),
        "penguin" => (&CHICKEN, 0.75),
        "emu" | "cockatrice" => (&CHICKEN, 0.75),
        "moa" => (&CHICKEN, 0.65),
        "squid" | "turtle" | "tortoise" | "snapping_turtle" => (&SQUID, 1.0),
        "giant_squid" => (&SQUID, 0.7),
        "dolphin" => (&SQUID, 1.1),
        "axolotl" => (&SQUID, 1.25),
        "deep_watcher" | "sentry" | "mech_eye" => (&GUARDIAN, 0.8),
        "sentinel" => (&GUARDIAN, 0.75),
        "forest_guardian" | "void_guardian" => (&GUARDIAN, 0.7),
        "obsidilith" => (&GUARDIAN, 0.65),
        "warden" => (&GUARDIAN, 0.6),
        "glare" => (&GUARDIAN, 0.9),
        "player" => (&VILLAGER, 0.9),
        "valkyrie" => (&VILLAGER, 1.05),
        "valkyrie_queen" => (&VILLAGER, 0.95),
        "brute" | "marauder" | "invoker" | "necromancer" => (&VILLAGER, 0.8),
        "archmage" | "pyromancer" | "cryomancer" => (&VILLAGER, 0.85),
        _ if has("npc_") || has("settler_") => (&VILLAGER, 1.0),
        _ if has("goblin_") => (&VILLAGER, 0.85),
        "piglin" | "nether_zombie" => (&ZOMBIE_PIG, 1.0),
        "piglin_brute" => (&ZOMBIE_PIG, 0.75),
        "mutant_zombie" => (&ZOMBIE, 0.75),
        _ if has("zombie") => (&ZOMBIE, 1.0),
        "mummy" | "shattered" => (&ZOMBIE, 0.85),
        "sunken" => (&ZOMBIE, 0.9),
        "wither_skeleton" => (&SKELETON, 0.8),
        "mutant_skeleton" => (&SKELETON, 0.75),
        "bone_king" => (&SKELETON, 0.7),
        _ if has("skeleton") => (&SKELETON, 1.0),
        "frost_archer" => (&SKELETON, 1.0),
        "spider" => (&SPIDER, 0.85),
        "tarantula_hawk" => (&SPIDER, 0.95),
        "stalker" => (&SPIDER, 0.75),
        "otherside_stalker" => (&SPIDER, 0.7),
        "mutant_stalker" => (&SPIDER, 0.6),
        "mutant_void_walker" => (&ENDER, 0.75),
        _ if has("void_walker") => (&ENDER, 1.0),
        "ember_wraith" => (&ENDER, 0.9),
        "void_blossom" | "shulker" => (&ENDER, 1.0),
        "ghast" => (&ENDER, 0.7),
        "wither" => (&ENDER, 0.6),
        "ender_dragon" => (&ENDER, 0.5),
        "lich" => (&LICH, 0.9),
        "blaze" => (&FIRE, 1.0),
        "fire_imp" => (&FIRE, 1.1),
        "fire_minion" => (&FIRE, 1.2),
        "magma_cube" => (&MAGMA, 1.0),
        "magma_cube_small" => (&MAGMA_SMALL, 1.0),
        "lava_leviathan" => (&MAGMA, 0.6),
        "ooze" => (&SLIME, 1.0),
        "sludge" | "mimic_cube" => (&SLIME, 0.8),
        "swet" => (&SLIME, 1.2),
        "endermite" | "sculk_leech" => (&MITE, 0.9),
        "cockroach" => (&MITE, 1.0),
        "swamp_witch" => (&WITCH, 1.0),
        "horse" => (&HORSE, 1.0),
        "deer" => (&HORSE, 1.15),
        "gazelle" => (&HORSE, 1.2),
        "zebra" | "donkey" => (&DONKEY, 1.0),
        "llama" => (&LLAMA, 1.0),
        "wolf" | "wolf_tame" => (&WOLF, 1.0),
        "spectral_wolf" => (&WOLF, 0.9),
        "fox" => (&WOLF, 1.2),
        "lobo_guara" => (&WOLF, 1.1),
        "hyena" => (&WOLF, 1.05),
        "cat" | "cat_tame" => (&CAT, 0.9),
        "rabbit" => (&RABBIT, 0.85),
        "aerbunny" | "squirrel" | "quati" | "skunk" => (&RABBIT, 0.9),
        "hedgehog" => (&RABBIT, 0.95),
        "jerboa" => (&RABBIT, 1.0),
        "raccoon" => (&RABBIT, 0.85),
        "bear" => (&BEAR, 1.0),
        "polar_bear" => (&BEAR, 0.9),
        "grizzly_bear" => (&BEAR, 0.85),
        "dropbear" | "panda" => (&BEAR, 1.1),
        "gorilla" => (&BEAR, 0.9),
        "lion" => (&BEAR, 0.85),
        "lioness" | "onca_pintada" => (&BEAR, 0.9),
        _ => return None,
    })
}

fn index_of(name: &str) -> Option<u16> {
    CLIPS.iter().position(|(n, _)| *n == name).map(|i| i as u16)
}

/// A recording that is not a creature's call (hooves, a wagon's wood creaking), played at
/// `rate` (below 1: slower and deeper).
pub fn clip(name: &str, rate: f32) -> Option<Fx> {
    Some(Fx::Clip(index_of(name)?, rate.clamp(0.4, 1.1)))
}

/// A recorded call for this mob, or None if it has none. `depth` is 1 for a normal-sized
/// creature and more for bigger ones; r1 and r2 are random numbers in 0..1.
pub fn pick(name: &str, call: Call, depth: f32, r1: f32, r2: f32) -> Option<Fx> {
    let (set, rate) = set_of(name)?;
    let (list, extra) = match call {
        Call::Idle => (set.idle, 1.0),
        Call::Hurt => (if set.hurt.is_empty() { set.idle } else { set.hurt }, 1.04),
        Call::Death => (if set.death.is_empty() { set.hurt } else { set.death }, 0.9),
    };
    if list.is_empty() {
        return None;
    }
    let clip = list[((r1 * list.len() as f32) as usize).min(list.len() - 1)];
    let rate = rate * extra / depth.clamp(0.6, 1.8).powf(0.3) * (0.94 + 0.12 * r2);
    // Small creatures may sound smaller, but never sped up into a squeak.
    Some(Fx::Clip(index_of(clip)?, rate.clamp(0.4, 1.1)))
}

type Decoded = Arc<(Vec<f32>, u32)>;

/// A clip decoded to mono (cached: each one is decoded once).
fn decoded(i: u16) -> Option<Decoded> {
    static CACHE: OnceLock<Mutex<HashMap<u16, Decoded>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(d) = cache.lock().ok()?.get(&i) {
        return Some(d.clone());
    }
    use rodio::Source;
    let bytes: &'static [u8] = CLIPS.get(i as usize)?.1;
    let src = rodio::Decoder::new(Cursor::new(bytes)).ok()?;
    let (ch, sr) = (src.channels().max(1) as usize, src.sample_rate());
    let all: Vec<f32> = src.convert_samples::<f32>().collect();
    let mono = all.chunks(ch).map(|c| c.iter().sum::<f32>() / ch as f32).collect();
    let d = Arc::new((mono, sr));
    cache.lock().ok()?.insert(i, d.clone());
    Some(d)
}

/// Plays clip `i` at `rate` (1 = as recorded) as mono samples at 44.1 kHz.
pub fn render(i: u16, rate: f32) -> Vec<f32> {
    let Some(d) = decoded(i) else {
        return Vec::new();
    };
    let (src, sr) = (&d.0, d.1 as f32);
    if src.len() < 2 {
        return Vec::new();
    }
    let step = sr / RATE * rate.clamp(0.25, 4.0);
    let n = ((src.len() - 1) as f32 / step) as usize;
    let mut out: Vec<f32> = (0..n)
        .map(|k| {
            let p = k as f32 * step;
            let j = p as usize;
            let f = p - j as f32;
            src[j] * (1.0 - f) + src[(j + 1).min(src.len() - 1)] * f
        })
        .collect();
    // Two gentle low-pass stages (about 3 kHz): keeps the body of the voice and takes off the
    // hiss and squeak, so nothing sounds shrill.
    let k = 1.0 - (-std::f32::consts::TAU * 3000.0 / RATE).exp();
    let (mut a, mut b) = (0.0f32, 0.0f32);
    for s in out.iter_mut() {
        a += (*s - a) * k;
        b += (a - b) * k;
        *s = b;
    }
    // Short fades so no clip starts or ends on a click; peak at a steady level.
    let fade = (RATE * 0.006) as usize;
    let len = out.len();
    for k in 0..fade.min(len / 2) {
        let g = k as f32 / fade as f32;
        out[k] *= g;
        out[len - 1 - k] *= g;
    }
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 1e-4 {
        let g = 0.8 / peak;
        out.iter_mut().for_each(|s| *s *= g);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETS: &[&Set] = &[
        &COW, &PIG, &SHEEP, &CHICKEN, &SQUID, &GUARDIAN, &VILLAGER, &ZOMBIE_PIG, &ZOMBIE,
        &SKELETON, &SPIDER, &ENDER, &LICH, &FIRE, &MAGMA, &MAGMA_SMALL, &SLIME, &MITE, &WITCH,
        &HORSE, &DONKEY, &LLAMA, &WOLF, &CAT, &RABBIT, &BEAR,
    ];

    #[test]
    fn every_set_names_real_clips() {
        for s in SETS {
            for n in s.idle.iter().chain(s.hurt).chain(s.death) {
                assert!(index_of(n).is_some(), "missing clip {n}");
            }
        }
    }

    #[test]
    fn every_clip_decodes_and_sounds() {
        for (i, (name, _)) in CLIPS.iter().enumerate() {
            let v = render(i as u16, 1.0);
            assert!(v.len() > 2000, "{name} too short: {}", v.len());
            assert!(v.iter().any(|s| s.abs() > 0.5), "{name} is silent");
            assert!(v.last().unwrap().abs() < 0.01, "{name} ends on a click");
            // Effective pitch from how fast the wave changes: low and full, never shrill.
            let e: f32 = v.iter().map(|s| s * s).sum();
            let d: f32 = v.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
            let hz = RATE / std::f32::consts::PI * ((d / e).sqrt() / 2.0).min(1.0).asin();
            assert!(hz < 2500.0, "{name} too bright: {hz:.0} Hz");
        }
    }

    #[test]
    fn big_mobs_sound_deeper_and_quiet_ones_stay_quiet() {
        let Some(Fx::Clip(_, cow)) = pick("cow", Call::Idle, 1.0, 0.0, 0.5) else { panic!() };
        let Some(Fx::Clip(_, elephant)) = pick("elephant", Call::Idle, 1.6, 0.0, 0.5) else { panic!() };
        assert!(elephant < cow);
        assert!(pick("butterfly", Call::Idle, 1.0, 0.0, 0.0).is_none());
        assert!(pick("squid", Call::Idle, 1.0, 0.0, 0.0).is_none());
        assert!(pick("squid", Call::Hurt, 1.0, 0.0, 0.0).is_some());
        assert!(pick("nightwing", Call::Hurt, 1.0, 0.0, 0.0).is_none());
    }
}

#[cfg(test)]
mod export {
    use super::*;

    /// Writes a few calls of each mob family as WAV files into $MC_SOUNDS_OUT (listening test).
    #[test]
    #[ignore]
    fn export_mob_sounds() {
        let Ok(dir) = std::env::var("MC_SOUNDS_OUT") else { return };
        let mobs = [
            ("vaca", "cow", 1.0), ("elefante", "elephant", 1.6), ("porco", "pig", 0.8), ("ovelha", "sheep", 1.0),
            ("galinha", "chicken", 0.7), ("cavalo", "horse", 1.2), ("burro", "donkey", 1.1), ("lhama", "llama", 1.3),
            ("lobo", "wolf", 0.8), ("raposa", "fox", 0.7), ("gato", "cat", 0.6), ("coelho", "rabbit", 0.6),
            ("urso", "bear", 1.2), ("leao", "lion", 1.1), ("zumbi", "zombie", 1.4), ("esqueleto", "skeleton", 1.4),
            ("aranha", "spider", 0.8), ("aldeao", "npc_guide", 1.4), ("piglin", "piglin", 1.4),
            ("void_walker", "void_walker", 2.0), ("blaze", "blaze", 1.3), ("slime", "ooze", 1.0),
            ("magma", "magma_cube", 1.0), ("bruxa", "swamp_witch", 1.4), ("guardiao", "forest_guardian", 2.2),
            ("lula", "squid", 1.0), ("endermite", "endermite", 0.5),
        ];
        for (pt, name, height) in mobs {
            let depth = (height / 1.4f32).clamp(0.5, 2.6).sqrt();
            let mut all = Vec::new();
            for (k, call) in [Call::Idle, Call::Idle, Call::Hurt, Call::Death].into_iter().enumerate() {
                if let Some(Fx::Clip(i, r)) = pick(name, call, depth, k as f32 * 0.37 % 1.0, 0.5) {
                    all.extend(render(i, r).iter().map(|s| s * 0.7));
                    all.extend(std::iter::repeat_n(0.0, 14000));
                }
            }
            let zc = all.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count();
            let loud = all.iter().filter(|s| s.abs() > 0.02).count().max(1);
            println!("{pt}: {:.0} Hz", zc as f32 / (loud as f32 / RATE) / 2.0);
            let data: Vec<u8> = all.iter().flat_map(|s| ((s.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes()).collect();
            let mut b = Vec::new();
            b.extend_from_slice(b"RIFF");
            b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
            b.extend_from_slice(b"WAVEfmt ");
            for v in [16u32, 1 | (1 << 16), 44100, 88200, 2 | (16 << 16)] {
                b.extend_from_slice(&v.to_le_bytes());
            }
            b.extend_from_slice(b"data");
            b.extend_from_slice(&(data.len() as u32).to_le_bytes());
            b.extend_from_slice(&data);
            std::fs::write(format!("{dir}/{pt}.wav"), b).unwrap();
        }
    }
}
