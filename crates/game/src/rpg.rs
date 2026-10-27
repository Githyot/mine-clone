//! The player's stats (Terraria's idea, our own numbers): defense, damage and critical chance per
//! class (melee, ranged, magic, summon), attack and move speed, mana, regeneration, minion slots,
//! dodge, thorns and immunities. Everything that changes them is data in this file: the
//! accessories (70 of them, with what each one does), armour set bonuses, and the buffs and
//! debuffs of effects.rs. The stats are worked out once a frame into `Rpg::stats`, and the rest
//! of the game reads them from there.
//!
//! Also here: healing and mana potions with potion sickness and the quick keys (H heal, N mana,
//! B buffs), extra accessory slots, three equipment loadouts, minions and whips (the summoner
//! class), debuffs on monsters, monsters that leave debuffs on the player, info accessories,
//! and the stats sheet (C, and beside the inventory).

use std::collections::HashMap;

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;

use crate::audio::{Material, Sfx};
use crate::effects::*;
use crate::item_render::GpuItem;
use crate::Game;

pub const MELEE: usize = 0;
pub const RANGED: usize = 1;
pub const MAGIC: usize = 2;
pub const SUMMON: usize = 3;
/// Every class at once (in `M::Dmg` and `M::Crit`).
const ALL_CLASSES: usize = 4;
const CLASS_NAMES: [&str; 4] = ["corpo a corpo", "à distância", "mágico", "de invocação"];

/// Accessory slots: five to start with, two more from rare consumables.
pub const MAX_ACC: usize = 7;
pub const BASE_ACC: usize = 5;

/// Not an effect id: "sets the mob on fire" in `M::Inflict` (mobs burn with `on_fire`).
pub const MOB_BURN: u8 = 200;

/// Info shown on screen by info accessories.
pub const INFO_DEPTH: u32 = 1;
pub const INFO_TIME: u32 = 2;
pub const INFO_RADAR: u32 = 4;
pub const INFO_DPS: u32 = 8;
pub const INFO_ORE: u32 = 16;
pub const INFO_WEATHER: u32 = 32;
pub const INFO_COORDS: u32 = 64;

/// Debuffs the ankh keeps away.
const ALL_DEBUFFS: &[u8] = &[
    POISON, VENOM, BLEEDING, BROKEN_ARMOR, ICHOR, WEAKNESS, SLOWNESS, CHILLED, FROZEN, BLINDNESS, DARKNESS, CONFUSED, NAUSEA, SILENCED, CURSED_FLAMES, FROSTBURN, WITHER, HUNGER,
];

/// One thing an accessory, a set or a buff does to the stats.
#[derive(Clone, Copy)]
pub enum M {
    /// Armour points.
    Def(f32),
    /// Damage of a class (or all of them), as a fraction: 0.1 = +10%.
    Dmg(usize, f32),
    /// Critical chance of a class (or all), 0.1 = +10%.
    Crit(usize, f32),
    MeleeSpeed(f32),
    Move(f32),
    Jump(f32),
    /// Mana (2 = one star).
    Mana(f32),
    /// Less mana per spell (0.1 = 10% cheaper).
    ManaCost(f32),
    /// Mana per second.
    ManaRegen(f32),
    /// Life per second.
    Regen(f32),
    Minions(u32),
    /// Share of a monster's hit sent back to it.
    Thorns(f32),
    /// Extra melee knockback (1.0 = twice as much).
    Knock(f32),
    NoKnock,
    Immune(&'static [u8]),
    /// Melee hits give the monster this debuff for so many seconds.
    Inflict(u8, f32),
    /// Share of all damage taken away.
    Endure(f32),
    /// Chance to dodge a monster's hit entirely.
    Dodge(f32),
    /// Chance not to use up an arrow.
    AmmoSave(f32),
    NoFall,
    /// Lava doesn't hurt at all.
    Lava,
    /// Fire doesn't hurt.
    Fire,
    /// Faster swimming.
    Swim(f32),
    /// Breath lasts this many times longer.
    Breath(f32),
    /// Picks things up from further away (blocks).
    Pickup(f32),
    /// Reaches further to dig and build (blocks).
    Reach(f32),
    /// Extra seconds of safety after being hurt.
    IFrames(f32),
    Luck(f32),
    Info(u32),
}

pub struct AccDef {
    pub name: &'static str,
    /// What it does that the stat lines don't say (movement tricks, on-hurt effects).
    pub desc: &'static str,
    pub mods: &'static [M],
}

const fn acc(name: &'static str, desc: &'static str, mods: &'static [M]) -> AccDef {
    AccDef { name, desc, mods }
}

/// Every accessory. The first 17 came with Etapa 5 (their special moves live in
/// accessories.rs); the rest arrived with this file.
pub const ACCESSORIES: &[AccDef] = &[
    acc("running_boots", "", &[M::Move(0.3)]),
    acc("frost_spark_boots", "", &[M::Move(0.5), M::Immune(&[CHILLED, FROZEN])]),
    acc("terraspark_boots", "Corre muito, anda sobre a água e a lava não queima", &[M::Move(0.5), M::Immune(&[CHILLED, FROZEN]), M::Lava, M::Fire]),
    acc("moon_charm", "À noite: +10% de dano e +10% de velocidade", &[]),
    acc("bastion_shield", "Da masmorra: não é empurrado e +2 de defesa", &[M::NoKnock, M::Def(2.0)]),
    // Only in treasure bags (Especialista and Mestre, tr_modes.rs).
    acc("jelly_heart", "Das sacolas: pulos mais altos e vida que volta", &[M::Jump(0.2), M::Regen(0.2)]),
    acc("eye_of_the_watcher", "Das sacolas: mais críticos e mostra criaturas perto", &[M::Crit(ALL_CLASSES, 0.05), M::Info(INFO_RADAR)]),
    acc("mind_veil", "Das sacolas: às vezes desvia de um golpe", &[M::Dodge(0.08)]),
    acc("hive_heart", "Das sacolas: um lacaio a mais", &[M::Minions(1)]),
    acc("frost_core", "Das sacolas: o frio não te prende", &[M::Immune(&[CHILLED, FROZEN]), M::Def(2.0)]),
    acc("worm_scarf", "Das sacolas: menos dano recebido", &[M::Endure(0.12)]),
    acc("bone_glove", "Das sacolas: corpo a corpo mais forte e rápido", &[M::Dmg(MELEE, 0.08), M::MeleeSpeed(0.08)]),
    acc("breaker_heart", "Das sacolas: defesa e vida que volta", &[M::Def(3.0), M::Regen(0.3)]),
    acc("mech_core", "Das sacolas: todo dano um pouco maior", &[M::Dmg(ALL_CLASSES, 0.06)]),
    acc("vine_seed", "Das sacolas: a vida volta bem mais depressa", &[M::Regen(0.5)]),
    acc("gel_crown", "Das sacolas: pulos altos e sem dano de queda", &[M::Jump(0.25), M::NoFall]),
    acc("sun_core", "Das sacolas: muita defesa", &[M::Def(4.0)]),
    acc("tide_scale", "Das sacolas: nada rápido e respira mais", &[M::Swim(0.5)]),
    acc("prism_heart", "Das sacolas: mais rápido e mais críticos", &[M::Move(0.1), M::Crit(ALL_CLASSES, 0.05)]),
    acc("prophet_eye", "Das sacolas: mais mana e magias mais baratas", &[M::Mana(4.0), M::ManaCost(0.1)]),
    acc("herald_sigil", "Das sacolas: todo dano e defesa maiores", &[M::Dmg(ALL_CLASSES, 0.1), M::Def(4.0)]),
    acc("bundle_of_balloons", "Três pulos a mais no ar e pulos mais altos", &[M::Jump(0.3)]),
    acc("cloud_bottle", "Um segundo pulo no ar", &[]),
    acc("lucky_horseshoe", "", &[M::NoFall]),
    acc("wind_cape", "Toque duas vezes A ou D para avançar", &[]),
    acc("shiny_balloon", "", &[M::Jump(0.3)]),
    acc("feather_wings", "Segure Espaço no ar para voar um pouco e planar", &[]),
    acc("angel_wings", "Voa mais tempo e plana", &[M::NoFall]),
    acc("crystal_wings", "Asas: voam 3 s e planam", &[M::NoFall, M::Move(0.1)]),
    acc("tide_wings", "Asas: voam 3,2 s e planam", &[M::NoFall, M::Move(0.1), M::Dmg(MELEE, 0.05)]),
    acc("empress_wings", "Asas: voam 3,6 s e planam", &[M::NoFall, M::Move(0.12), M::Dmg(ALL_CLASSES, 0.05)]),
    acc("fairy_wings", "Asas: voam 2,4 s e planam", &[M::NoFall, M::Move(0.05)]),
    acc("demon_wings", "Asas: voam 2,4 s e planam", &[M::NoFall, M::Dmg(ALL_CLASSES, 0.03)]),
    acc("leaf_wings", "Asas: voam 2,6 s e planam", &[M::NoFall, M::Move(0.05)]),
    acc("frost_wings", "Asas: voam 2,6 s e planam", &[M::NoFall, M::Move(0.05)]),
    acc("flame_wings", "Asas: voam 2,8 s e planam", &[M::NoFall, M::Move(0.08)]),
    acc("bee_wings", "Asas: voam 2,2 s e planam", &[M::NoFall, M::Move(0.05)]),
    acc("mana_ring", "", &[M::ManaRegen(2.0)]),
    acc("star_amulet", "", &[M::Mana(10.0)]),
    acc("warrior_emblem", "", &[M::Dmg(MELEE, 0.15)]),
    acc("arcane_emblem", "", &[M::Dmg(MAGIC, 0.15)]),
    acc("obsidian_charm", "7 s sem queimar na lava", &[M::NoKnock, M::Fire]),
    acc("regen_band", "", &[M::Regen(0.25)]),
    acc("depth_meter", "", &[M::Info(INFO_DEPTH | INFO_COORDS)]),
    acc("climbing_claws", "Segure Espaço contra a parede para escalar", &[]),
    acc("guardian_shield", "", &[M::Def(4.0), M::NoKnock]),
    // Class emblems.
    acc("ranger_emblem", "", &[M::Dmg(RANGED, 0.15)]),
    acc("summoner_emblem", "", &[M::Dmg(SUMMON, 0.15)]),
    acc("avenger_emblem", "", &[M::Dmg(ALL_CLASSES, 0.12)]),
    acc("champion_emblem", "", &[M::Dmg(ALL_CLASSES, 0.15), M::Crit(ALL_CLASSES, 0.08)]),
    // Melee.
    acc("claw_glove", "", &[M::MeleeSpeed(0.12)]),
    acc("giant_gauntlet", "", &[M::MeleeSpeed(0.12), M::Dmg(MELEE, 0.1), M::Knock(1.0)]),
    acc("ember_stone", "", &[M::Inflict(MOB_BURN, 4.0)]),
    acc("rime_stone", "", &[M::Inflict(FROSTBURN, 4.0)]),
    acc("venom_gland", "", &[M::Inflict(VENOM, 3.0)]),
    acc("fang_necklace", "", &[M::Dmg(MELEE, 0.05), M::Crit(MELEE, 0.05)]),
    // Ranged.
    acc("hunter_quiver", "", &[M::Dmg(RANGED, 0.1), M::AmmoSave(0.2)]),
    acc("marksman_lens", "", &[M::Dmg(RANGED, 0.1), M::Crit(RANGED, 0.1)]),
    // Magic.
    acc("mana_blossom", "Bebe uma poção de mana sozinho quando a mana acaba", &[M::ManaCost(0.08)]),
    acc("arcane_cuffs", "Ganha mana ao ser ferido", &[M::Mana(4.0)]),
    acc("sage_pendant", "", &[M::Crit(MAGIC, 0.1), M::Mana(4.0)]),
    // Summon.
    acc("totem_necklace", "", &[M::Minions(1)]),
    acc("spirit_bell", "", &[M::Minions(1), M::Dmg(SUMMON, 0.1)]),
    // Defence.
    acc("iron_buckler", "", &[M::Def(2.0)]),
    acc("holy_pendant", "", &[M::IFrames(0.5)]),
    acc("star_veil", "Estrelas caem nos monstros quando você é ferido", &[M::IFrames(0.5)]),
    acc("thorned_band", "", &[M::Thorns(0.5)]),
    acc("turtle_shell", "-25% de dano com menos da metade da vida", &[M::Def(2.0)]),
    acc("warding_scarf", "", &[M::Endure(0.12)]),
    acc("mirror_charm", "", &[M::Dodge(0.1)]),
    acc("panic_charm", "Corre mais rápido logo depois de ser ferido", &[]),
    acc("honey_locket", "Regenera vida logo depois de ser ferido", &[]),
    // Immunities.
    acc("antidote_charm", "", &[M::Immune(&[POISON, VENOM])]),
    acc("bandage_wrap", "", &[M::Immune(&[BLEEDING])]),
    acc("steel_polish", "", &[M::Immune(&[BROKEN_ARMOR, ICHOR])]),
    acc("vitamin_pouch", "", &[M::Immune(&[WEAKNESS, HUNGER])]),
    acc("dark_lenses", "", &[M::Immune(&[BLINDNESS, DARKNESS])]),
    acc("pocket_clock", "", &[M::Immune(&[SLOWNESS, CHILLED, FROZEN])]),
    acc("warding_eye", "", &[M::Immune(&[CURSED_FLAMES, WITHER])]),
    acc("brass_horn", "", &[M::Immune(&[SILENCED])]),
    acc("old_map", "", &[M::Immune(&[CONFUSED, NAUSEA])]),
    acc("medicine_kit", "", &[M::Immune(&[POISON, VENOM, BLEEDING])]),
    acc("ankh_charm", "", &[M::Immune(ALL_DEBUFFS)]),
    acc("ankh_shield", "", &[M::Immune(ALL_DEBUFFS), M::Def(4.0), M::NoKnock, M::Fire]),
    // Movement.
    acc("frog_charm", "", &[M::Jump(0.4)]),
    acc("swim_fins", "", &[M::Swim(0.6)]),
    acc("diving_mask", "", &[M::Breath(3.0)]),
    acc("magma_boots", "", &[M::Lava, M::Fire]),
    acc("steam_boots", "Segure Espaço no ar para subir por um instante", &[]),
    acc("shadow_gear", "Avança com dois toques em A ou D e escala paredes", &[M::Dodge(0.1)]),
    // From the deep world (accessories.rs): the jumps stack, one of each per jump.
    acc("leap_boots", "Um pulo a mais no ar", &[M::Move(0.15), M::Jump(0.15)]),
    acc("blizzard_bottle", "Um pulo a mais no ar, bem alto", &[]),
    acc("star_band", "Mais mana", &[M::Mana(4.0)]),
    acc("sandstorm_bottle", "Um pulo a mais que continua subindo enquanto segura Espaço", &[]),
    acc("cloud_balloon", "Um pulo a mais no ar e pulos mais altos", &[M::Jump(0.3), M::NoFall]),
    acc("rocket_boots", "Segure Espaço no ar para subir com foguetes", &[M::Move(0.1)]),
    acc("comet_boots", "Foguetes mais longos e corrida rápida", &[M::Move(0.4), M::Immune(&[CHILLED])]),
    acc("water_walking_boots", "Anda sobre a água (Shift para afundar)", &[]),
    acc("charge_shield", "Dois toques em A ou D: investida que derruba monstros", &[M::Def(2.0), M::NoKnock]),
    // The early bosses of tr_bosses.rs.
    acc("royal_gel", "As geleias não atacam você", &[]),
    acc("watcher_shield", "Dois toques em A ou D: investida que derruba monstros", &[M::Def(2.0)]),
    acc("mind_ward", "Quando você se machuca, confunde os monstros por perto", &[M::Dodge(0.05)]),
    acc("hive_pack", "Quando você se machuca, solta abelhas que picam", &[]),
    acc("fur_cloak", "", &[M::Def(2.0), M::Immune(&[CHILLED, FROZEN, FROSTBURN])]),
    acc("climbing_gear", "Escala paredes, desliza com Shift e salta da parede com Espaço", &[M::NoFall]),
    // Handy.
    acc("magnet_ring", "", &[M::Pickup(3.0)]),
    acc("lucky_coin", "Monstros às vezes deixam uma esmeralda", &[M::Luck(1.0)]),
    acc("sky_stone", "", &[M::Dmg(ALL_CLASSES, 0.08), M::Crit(ALL_CLASSES, 0.04), M::MeleeSpeed(0.08), M::Def(2.0), M::Regen(0.25), M::Move(0.05)]),
    // Info.
    acc("pocket_watch", "", &[M::Info(INFO_TIME)]),
    acc("creature_radar", "", &[M::Info(INFO_RADAR)]),
    acc("battle_tally", "", &[M::Info(INFO_DPS)]),
    acc("ore_finder", "", &[M::Info(INFO_ORE)]),
    acc("weather_vane", "", &[M::Info(INFO_WEATHER)]),
    acc("explorer_kit", "", &[M::Info(INFO_DEPTH | INFO_COORDS | INFO_TIME | INFO_WEATHER)]),
];

pub fn accessory(name: &str) -> Option<&'static AccDef> {
    ACCESSORIES.iter().find(|a| a.name == name)
}

/// Armour sets: all four pieces of one metal give a bonus.
pub struct SetDef {
    pub prefix: &'static str,
    pub desc: &'static str,
    pub mods: &'static [M],
}

const fn set(prefix: &'static str, desc: &'static str, mods: &'static [M]) -> SetDef {
    SetDef { prefix, desc, mods }
}

pub const SETS: &[SetDef] = &[
    set("copper", "", &[M::Def(1.0), M::Move(0.05)]),
    set("bronze", "", &[M::Def(1.0), M::Dmg(MELEE, 0.05)]),
    set("lead", "", &[M::Def(2.0)]),
    set("silver", "", &[M::Crit(ALL_CLASSES, 0.05)]),
    set("tungsten", "", &[M::Dmg(ALL_CLASSES, 0.05)]),
    set("platinum", "", &[M::Def(1.0), M::Mana(4.0), M::Crit(ALL_CLASSES, 0.03)]),
    set("cobalt", "", &[M::MeleeSpeed(0.12), M::Move(0.1)]),
    set("palladium", "", &[M::Regen(0.35), M::Dmg(ALL_CLASSES, 0.05)]),
    set("mythril", "", &[M::Crit(ALL_CLASSES, 0.1)]),
    set("orichalcum", "Pétalas caem sobre o que você acerta", &[M::Dmg(ALL_CLASSES, 0.05)]),
    set("adamantite", "", &[M::Dmg(MELEE, 0.15), M::MeleeSpeed(0.1)]),
    set("titanium", "", &[M::Endure(0.12), M::Dmg(ALL_CLASSES, 0.05)]),
    set("radiant", "Esquiva um golpe a cada 30 s", &[M::Dmg(ALL_CLASSES, 0.08)]),
    set("verdite", "Um cristal de folha atira nos monstros por perto", &[M::Dmg(ALL_CLASSES, 0.05)]),
    set("magmarite", "Fogo e lava não queimam", &[M::Fire, M::Lava, M::Dmg(MELEE, 0.12), M::Def(2.0)]),
    set("umbral", "", &[M::MeleeSpeed(0.15), M::Move(0.08)]),
    set("sanguinite", "", &[M::Regen(0.5), M::Def(1.0)]),
    set("hive", "", &[M::Minions(1), M::Dmg(SUMMON, 0.1)]),
    set("jungle", "", &[M::Mana(4.0), M::ManaCost(0.16)]),
    set("fossil", "", &[M::Dmg(RANGED, 0.1), M::Crit(RANGED, 0.05)]),
    set("necro", "", &[M::Dmg(RANGED, 0.1), M::Crit(RANGED, 0.05)]),
    set("ninja", "", &[M::Move(0.2), M::Crit(ALL_CLASSES, 0.04)]),
    set("meteor", "O Disparador de Meteoro não gasta mana", &[M::Dmg(MAGIC, 0.09), M::Def(1.0)]),
    set("spectral", "Golpes mágicos curam um pouco", &[M::Dmg(MAGIC, 0.2), M::Mana(6.0), M::Regen(0.4)]),
    set("sporite", "", &[M::Dmg(RANGED, 0.2), M::Crit(RANGED, 0.1)]),
    set("beetle", "", &[M::Dmg(MELEE, 0.15), M::MeleeSpeed(0.15), M::Def(4.0)]),
    set("turtle", "", &[M::Endure(0.15), M::Def(5.0)]),
    set("solar", "", &[M::Dmg(MELEE, 0.25), M::Crit(MELEE, 0.15), M::Move(0.1), M::Def(3.0)]),
    set("vortex", "", &[M::Dmg(RANGED, 0.25), M::Crit(RANGED, 0.15), M::Move(0.1)]),
    set("nebula", "", &[M::Dmg(MAGIC, 0.25), M::Mana(10.0), M::ManaCost(0.15), M::Move(0.1)]),
    set("stardust", "", &[M::Dmg(SUMMON, 0.25), M::Minions(2), M::Move(0.1)]),
    set("lumite", "", &[M::Dmg(ALL_CLASSES, 0.1), M::Crit(ALL_CLASSES, 0.1), M::Minions(1), M::Move(0.1)]),
];

/// Healing and mana potions: (item, life, mana, seconds of sickness).
const RESTORE: &[(&str, f32, f32, f32)] = &[
    ("lesser_healing_potion", 5.0, 0.0, 60.0),
    ("healing_potion", 8.0, 0.0, 60.0),
    ("greater_healing_potion", 13.0, 0.0, 60.0),
    ("super_healing_potion", 20.0, 0.0, 60.0),
    ("restoration_potion", 7.0, 0.0, 45.0),
    ("lesser_mana_potion", 0.0, 8.0, 0.0),
    ("mana_potion", 0.0, 16.0, 0.0),
    ("greater_mana_potion", 0.0, 24.0, 0.0),
];

/// Minions: (staff, picture item, name, damage, shoots from afar, effect it leaves).
pub struct MinionDef {
    pub staff: &'static str,
    pub sprite: &'static str,
    pub name: &'static str,
    pub damage: f32,
    pub ranged: bool,
    pub cooldown: f32,
    pub inflict: u8,
    pub size: f32,
    pub tint: [f32; 3],
}

pub const MINIONS: &[MinionDef] = &[
    MinionDef { staff: "wisp_staff", sprite: "minion_wisp", name: "Fogo-fátuo", damage: 4.0, ranged: true, cooldown: 1.0, inflict: 0, size: 0.5, tint: [0.6, 0.9, 1.6] },
    MinionDef { staff: "wasp_staff", sprite: "minion_wasp", name: "Vespa", damage: 3.0, ranged: false, cooldown: 0.5, inflict: POISON, size: 0.45, tint: [1.5, 1.2, 0.3] },
    MinionDef { staff: "imp_staff", sprite: "minion_imp", name: "Diabrete", damage: 6.0, ranged: true, cooldown: 1.2, inflict: MOB_BURN, size: 0.6, tint: [1.6, 0.5, 0.15] },
    MinionDef { staff: "blade_staff", sprite: "minion_blade", name: "Lâmina Voadora", damage: 9.0, ranged: false, cooldown: 0.4, inflict: 0, size: 0.6, tint: [0.9, 0.9, 1.4] },
    MinionDef { staff: "jelly_staff", sprite: "minion_jelly", name: "Geleinha", damage: 5.0, ranged: false, cooldown: 0.6, inflict: 0, size: 0.5, tint: [0.6, 0.9, 1.5] },
    MinionDef { staff: "star_dragon_staff", sprite: "minion_dragon", name: "Dragão Estelar", damage: 16.0, ranged: false, cooldown: 0.35, inflict: CURSED_FLAMES, size: 1.0, tint: [0.4, 1.4, 1.4] },
];

/// Whips: extra reach, a mark that makes minions go for that monster, and its bonus.
const WHIPS: &[(&str, f32, u8)] = &[("leather_whip", 2.0, 0), ("thorn_whip", 3.0, POISON), ("bone_whip", 4.0, BLEEDING)];

pub fn is_whip(name: &str) -> bool {
    WHIPS.iter().any(|w| w.0 == name)
}

/// What monsters' hits leave on the player: (mob, effect or MOB_BURN, chance, seconds).
const MOB_DEBUFFS: &[(&str, u8, f32, f32)] = &[
    ("blaze", MOB_BURN, 0.5, 5.0),
    ("fire_imp", MOB_BURN, 0.5, 4.0),
    ("ember_wraith", MOB_BURN, 0.4, 4.0),
    ("magma_cube", MOB_BURN, 0.4, 4.0),
    ("magma_cube_small", MOB_BURN, 0.3, 3.0),
    ("nether_zombie", MOB_BURN, 0.2, 3.0),
    ("frost_archer", CHILLED, 0.5, 5.0),
    ("polar_bear", CHILLED, 0.25, 4.0),
    ("blood_zombie", BLEEDING, 0.4, 8.0),
    ("mutant_zombie", BLEEDING, 0.5, 6.0),
    ("crimson_boar", BLEEDING, 0.3, 6.0),
    ("marauder", BLEEDING, 0.3, 6.0),
    ("bone_king", BLEEDING, 0.3, 8.0),
    ("deep_worm", BLEEDING, 0.3, 6.0),
    ("deep_worm_body", BLEEDING, 0.2, 6.0),
    ("goblin_warrior", BROKEN_ARMOR, 0.25, 8.0),
    ("brute", BROKEN_ARMOR, 0.25, 8.0),
    ("mutant_skeleton", BROKEN_ARMOR, 0.3, 8.0),
    ("obsidilith", BROKEN_ARMOR, 0.4, 10.0),
    ("world_breaker", BROKEN_ARMOR, 0.5, 10.0),
    ("mech_eye", CURSED_FLAMES, 0.3, 4.0),
    ("mech_worm", ICHOR, 0.3, 8.0),
    ("mech_worm_body", ICHOR, 0.2, 6.0),
    ("mech_skull", CURSED_FLAMES, 0.3, 4.0),
    ("vine_queen", VENOM, 0.4, 4.0),
    ("forest_guardian", POISON, 0.3, 4.0),
    ("void_blossom", VENOM, 0.3, 4.0),
    ("celestial_herald", CONFUSED, 0.3, 3.0),
    ("lich", SILENCED, 0.3, 4.0),
    ("void_walker", CONFUSED, 0.15, 3.0),
    ("deep_watcher", SILENCED, 0.2, 4.0),
    ("stalker", BLEEDING, 0.25, 6.0),
    ("jelly_ice", CHILLED, 0.4, 4.0),
    ("jelly_lava", MOB_BURN, 0.4, 4.0),
    ("great_eye", BLEEDING, 0.2, 5.0),
    ("blight_mind", CONFUSED, 0.25, 2.0),
    ("blight_creeper", ICHOR, 0.2, 5.0),
    ("hive_queen", POISON, 0.4, 5.0),
    ("hive_drone", POISON, 0.3, 4.0),
    ("frost_colossus", FROSTBURN, 0.4, 4.0),
];

/// Rare drops (Terraria's idea of accessories from monsters): (mob, item, chance).
const DROPS: &[(&str, &str, f32)] = &[
    ("zombie", "bandage_wrap", 0.015),
    ("zombie", "lesser_healing_potion", 0.04),
    ("skeleton", "steel_polish", 0.015),
    ("skeleton", "holy_pendant", 0.005),
    ("creeper", "vitamin_pouch", 0.015),
    ("spider", "venom_gland", 0.02),
    ("cave_spider", "antidote_charm", 0.03),
    ("cave_spider", "venom_gland", 0.02),
    ("blaze", "ember_stone", 0.04),
    ("magma_cube", "magma_boots", 0.02),
    ("frost_archer", "rime_stone", 0.04),
    ("swamp_witch", "brass_horn", 0.05),
    ("swamp_witch", "lesser_mana_potion", 0.2),
    ("sunken", "swim_fins", 0.04),
    ("sunken", "diving_mask", 0.03),
    ("void_walker", "old_map", 0.03),
    ("demon_eye", "dark_lenses", 0.04),
    ("demon_eye", "warding_eye", 0.02),
    ("blood_zombie", "fang_necklace", 0.04),
    ("blood_zombie", "panic_charm", 0.03),
    ("goblin_warrior", "claw_glove", 0.03),
    ("goblin_archer", "marksman_lens", 0.04),
    ("goblin_sorcerer", "sage_pendant", 0.04),
    ("mutant_zombie", "giant_gauntlet", 0.12),
    ("mutant_skeleton", "hunter_quiver", 0.12),
    ("wither_skeleton", "warding_eye", 0.03),
    ("polar_bear", "pocket_clock", 0.03),
    ("bee", "honey_locket", 0.02),
    ("hive_drone", "honey_locket", 0.02),
    ("jelly_blue", "lucky_horseshoe", 0.005),
    ("jelly_ice", "rime_stone", 0.02),
    ("servant_eye", "dark_lenses", 0.02),
    ("lich", "holy_pendant", 0.15),
    ("stalker", "mirror_charm", 0.05),
    ("forest_guardian", "totem_necklace", 0.5),
    ("forest_guardian", "ranger_emblem", 0.33),
    ("deep_worm", "magnet_ring", 0.5),
    ("bone_king", "spirit_bell", 0.5),
    ("bone_king", "bone_whip", 0.33),
    ("world_breaker", "ancestral_heart", 1.0),
    ("world_breaker", "summoner_emblem", 0.25),
    ("world_breaker", "warrior_emblem", 0.25),
    ("world_breaker", "ranger_emblem", 0.25),
    ("world_breaker", "arcane_emblem", 0.25),
    ("vine_queen", "turtle_shell", 0.33),
    ("vine_queen", "thorn_whip", 0.5),
    ("mech_eye", "blade_staff", 0.2),
    ("mech_skull", "blade_staff", 0.2),
    ("mech_worm", "blade_staff", 0.2),
    ("celestial_herald", "celestial_socket", 1.0),
    ("celestial_herald", "sky_stone", 0.5),
    ("celestial_herald", "star_dragon_staff", 0.25),
];

/// Seconds-per-point of damage over time on monsters (mob debuffs).
fn mob_dot(id: u8) -> f32 {
    match id {
        POISON => 1.0,
        VENOM => 3.0,
        FROSTBURN => 2.0,
        CURSED_FLAMES => 3.0,
        BLEEDING => 1.0,
        _ => 0.0,
    }
}

fn mob_debuff_tint(id: u8) -> [f32; 3] {
    let c = info(id).1;
    [c[0] as f32 / 160.0, c[1] as f32 / 160.0, c[2] as f32 / 160.0]
}

#[derive(Clone, Debug)]
pub struct Stats {
    pub defense: f32,
    pub endurance: f32,
    pub dmg: [f32; 4],
    pub crit: [f32; 4],
    pub melee_speed: f32,
    pub move_speed: f32,
    pub jump: f32,
    pub max_mana: f32,
    pub mana_cost: f32,
    pub mana_regen: f32,
    pub regen: f32,
    pub minions: u32,
    pub thorns: f32,
    pub knockback: f32,
    pub no_knockback: bool,
    pub immune: u128,
    pub inflict: Vec<(u8, f32)>,
    pub dodge: f32,
    pub ammo_save: f32,
    pub no_fall: bool,
    pub lava_immune: bool,
    pub fire_immune: bool,
    pub swim: f32,
    pub breath: f32,
    pub pickup: f32,
    pub reach: f32,
    pub iframes: f32,
    pub luck: f32,
    pub info: u32,
    /// The armour set worn in full, if any.
    pub set: Option<usize>,
}

impl Default for Stats {
    fn default() -> Stats {
        Stats {
            defense: 0.0,
            endurance: 0.0,
            dmg: [1.0; 4],
            // Ranged and magic hits have a small chance to be critical, like melee jump hits.
            crit: [0.0, 0.04, 0.04, 0.0],
            melee_speed: 1.0,
            move_speed: 1.0,
            jump: 0.0,
            max_mana: 0.0,
            mana_cost: 1.0,
            mana_regen: 0.0,
            regen: 0.0,
            minions: 1,
            thorns: 0.0,
            knockback: 1.0,
            no_knockback: false,
            immune: 0,
            inflict: Vec::new(),
            dodge: 0.0,
            ammo_save: 0.0,
            no_fall: false,
            lava_immune: false,
            fire_immune: false,
            swim: 0.0,
            breath: 1.0,
            pickup: 0.0,
            reach: 0.0,
            iframes: 0.0,
            luck: 0.0,
            info: 0,
            set: None,
        }
    }
}

impl Stats {
    pub(crate) fn apply(&mut self, m: &M) {
        match *m {
            M::Def(v) => self.defense += v,
            M::Dmg(c, v) if c == ALL_CLASSES => self.dmg.iter_mut().for_each(|d| *d += v),
            M::Dmg(c, v) => self.dmg[c] += v,
            M::Crit(c, v) if c == ALL_CLASSES => self.crit.iter_mut().for_each(|d| *d += v),
            M::Crit(c, v) => self.crit[c] += v,
            M::MeleeSpeed(v) => self.melee_speed += v,
            M::Move(v) => self.move_speed += v,
            M::Jump(v) => self.jump += v,
            M::Mana(v) => self.max_mana += v,
            M::ManaCost(v) => self.mana_cost -= v,
            M::ManaRegen(v) => self.mana_regen += v,
            M::Regen(v) => self.regen += v,
            M::Minions(n) => self.minions += n,
            M::Thorns(v) => self.thorns += v,
            M::Knock(v) => self.knockback += v,
            M::NoKnock => self.no_knockback = true,
            M::Immune(list) => {
                for &id in list {
                    self.immune |= 1u128 << id;
                }
            }
            M::Inflict(id, t) => self.inflict.push((id, t)),
            M::Endure(v) => self.endurance += v,
            M::Dodge(v) => self.dodge += v,
            M::AmmoSave(v) => self.ammo_save += v,
            M::NoFall => self.no_fall = true,
            M::Lava => self.lava_immune = true,
            M::Fire => self.fire_immune = true,
            M::Swim(v) => self.swim += v,
            M::Breath(v) => self.breath = self.breath.max(v),
            M::Pickup(v) => self.pickup += v,
            M::Reach(v) => self.reach += v,
            M::IFrames(v) => self.iframes += v,
            M::Luck(v) => self.luck += v,
            M::Info(b) => self.info |= b,
        }
    }

    pub fn immune(&self, id: u8) -> bool {
        id < 128 && self.immune & (1u128 << id) != 0
    }
}

/// One stat line for a tooltip ("+10% de dano mágico").
pub fn mod_line(m: &M) -> Option<String> {
    let pct = |v: f32| format!("{:+.0}%", v * 100.0);
    let class = |c: usize| if c == ALL_CLASSES { String::new() } else { format!(" {}", CLASS_NAMES[c]) };
    Some(match *m {
        M::Def(v) => format!("{:+.0} de armadura", v),
        M::Dmg(c, v) => format!("{} de dano{}", pct(v), class(c)),
        M::Crit(c, v) => format!("{} de chance de crítico{}", pct(v), class(c)),
        M::MeleeSpeed(v) => format!("{} de velocidade de ataque", pct(v)),
        M::Move(v) => format!("{} de velocidade ao andar", pct(v)),
        M::Jump(v) => format!("{} na altura do pulo", pct(v)),
        M::Mana(v) => format!("+{:.0} estrela(s) de mana", v / 2.0),
        M::ManaCost(v) => format!("-{:.0}% de custo de mana", v * 100.0),
        M::ManaRegen(_) => "Mana volta mais rápido".to_string(),
        M::Regen(_) => "Vida volta aos poucos".to_string(),
        M::Minions(n) => format!("+{n} lacaio(s)"),
        M::Thorns(v) => format!("Devolve {:.0}% do dano aos monstros", v * 100.0),
        M::Knock(_) => "Empurra mais longe".to_string(),
        M::NoKnock => "Sem recuo ao ser atingido".to_string(),
        M::Immune(list) if list.len() > 6 => "Imune à maioria dos efeitos ruins".to_string(),
        M::Immune(list) => format!("Imune a: {}", list.iter().map(|&e| info(e).0).collect::<Vec<_>>().join(", ")),
        M::Inflict(MOB_BURN, _) => "Ataques corpo a corpo põem fogo".to_string(),
        M::Inflict(id, _) => format!("Ataques corpo a corpo causam {}", info(id).0.to_lowercase()),
        M::Endure(v) => format!("-{:.0}% de dano recebido", v * 100.0),
        M::Dodge(v) => format!("{:.0}% de chance de esquivar", v * 100.0),
        M::AmmoSave(v) => format!("{:.0}% de chance de não gastar flecha", v * 100.0),
        M::NoFall => "Sem dano de queda".to_string(),
        M::Lava => "Imune à lava".to_string(),
        M::Fire => "Imune ao fogo".to_string(),
        M::Swim(_) => "Nada mais rápido".to_string(),
        M::Breath(v) => format!("Fôlego {v:.0}x maior debaixo d'água"),
        M::Pickup(v) => format!("Pega itens a +{v:.0} blocos"),
        M::Reach(v) => format!("+{v:.1} blocos de alcance"),
        M::IFrames(_) => "Fica protegido por mais tempo depois de um golpe".to_string(),
        M::Luck(_) => "Mais sorte".to_string(),
        M::Info(b) => {
            let mut parts = Vec::new();
            if b & INFO_DEPTH != 0 {
                parts.push("profundidade");
            }
            if b & INFO_COORDS != 0 {
                parts.push("posição");
            }
            if b & INFO_TIME != 0 {
                parts.push("hora");
            }
            if b & INFO_WEATHER != 0 {
                parts.push("clima");
            }
            if b & INFO_RADAR != 0 {
                parts.push("monstros por perto");
            }
            if b & INFO_DPS != 0 {
                parts.push("dano por segundo");
            }
            if b & INFO_ORE != 0 {
                parts.push("minério mais valioso por perto");
            }
            format!("Mostra: {}", parts.join(", "))
        }
    })
}

/// Armour and accessories of one loadout.
#[derive(Clone, Default)]
pub struct Gear {
    pub armor: [Option<ItemStack>; 4],
    pub acc: [Option<ItemStack>; MAX_ACC],
}

pub struct Minion {
    pub kind: usize,
    pub pos: Vec3,
    vel: Vec3,
    cd: f32,
    phase: f32,
}

/// The RPG state of the world that is open (inside `Terraria`).
#[derive(Default)]
pub struct Rpg {
    pub stats: Stats,
    /// Accessory slots unlocked beyond the five (0..2).
    pub extra_slots: u8,
    /// Which loadout is worn, and what the others hold.
    pub loadout: usize,
    pub loadouts: [Gear; 3],
    pub minions: Vec<Minion>,
    /// The monster the last whip hit (net id), and for how long minions go for it.
    pub tagged: Option<(u32, f32, f32)>,
    /// The stats sheet over the game (C).
    pub sheet: bool,
    regen_t: f32,
    mob_t: f32,
    holy_cd: f32,
    leaf_cd: f32,
    scan_t: f32,
    ores: Vec<IVec3>,
    best_ore: Option<(String, f32)>,
    aura_t: f32,
    dps_last: HashMap<u32, f32>,
    dps: Vec<(f32, f32)>,
    radar: usize,
    ore_ids: Option<Vec<(voxel::BlockId, u8)>>,
}

fn class_of_minion(name: &str) -> Option<usize> {
    MINIONS.iter().position(|m| m.staff == name)
}

impl Game {
    // -----------------------------------------------------------------------------------------
    // Working out the stats.

    pub(crate) fn acc_slots(&self) -> usize {
        BASE_ACC + self.t5.rpg.extra_slots.min(2) as usize
    }

    /// The armour set worn in full.
    fn full_set(&self) -> Option<usize> {
        let first = self.surv.armor[0]?;
        let n = &self.items.get(first.item).name;
        let i = SETS.iter().position(|s| n.strip_prefix(s.prefix).is_some_and(|r| r.starts_with('_')))?;
        let p = SETS[i].prefix;
        self.surv.armor.iter().all(|a| a.is_some_and(|a| self.items.get(a.item).name.strip_prefix(p).is_some_and(|r| r.starts_with('_')))).then_some(i)
    }

    pub(crate) fn compute_stats(&self) -> Stats {
        let mut s = Stats::default();
        s.defense = self.armor_points() as f32;
        let n = self.acc_slots();
        for st in self.t5.acc[..n].iter().flatten() {
            let name = &self.items.get(st.item).name;
            if let Some(a) = accessory(name) {
                for m in a.mods {
                    s.apply(m);
                }
            }
            if let Some(a) = crate::affixes::of(st) {
                s.defense += a.armor as f32;
                s.move_speed *= a.move_speed;
            }
        }
        // Info items in the backpack and the Shimmer's upgrades (tr_utility.rs).
        s.info |= self.t5.tr.util.inv_info;
        let perma = self.t5.tr.util.perma;
        if perma & crate::tr_utility::PERMA_VITAL != 0 {
            s.regen += 0.25;
        }
        if perma & crate::tr_utility::PERMA_ARCANE != 0 {
            s.apply(&M::Mana(4.0));
            s.apply(&M::ManaRegen(1.0));
        }
        if perma & crate::tr_utility::PERMA_AEGIS != 0 {
            s.defense += 4.0;
        }
        s.set = self.full_set();
        if let Some(i) = s.set {
            for m in SETS[i].mods {
                s.apply(m);
            }
        }
        // Buffs.
        let lvl = |id| self.effect_level(id) as f32;
        s.defense += 4.0 * lvl(IRONSKIN);
        s.mana_regen += 3.0 * lvl(MANA_REGEN);
        s.dmg[MAGIC] += 0.2 * lvl(MAGIC_POWER);
        s.dmg[RANGED] += 0.2 * lvl(ARCHERY);
        for d in s.dmg.iter_mut() {
            *d += 0.1 * lvl(WRATH);
        }
        for c in s.crit.iter_mut() {
            *c += 0.1 * lvl(RAGE);
        }
        s.endurance += 0.1 * lvl(ENDURANCE);
        if self.has_effect(THORNS) {
            s.thorns += 0.33;
        }
        s.knockback += 0.5 * lvl(TITAN);
        s.minions += lvl(SUMMONING) as u32;
        if self.has_effect(BUILDER) {
            s.apply(&M::Reach(1.5));
        }
        if self.has_effect(WELL_FED) {
            s.defense += 1.0;
            s.dmg.iter_mut().for_each(|d| *d += 0.05);
            s.crit.iter_mut().for_each(|c| *c += 0.02);
            s.move_speed += 0.05;
            s.regen += 0.1;
        }
        if self.has_effect(COZY_FIRE) {
            s.regen += 0.25;
        }
        self.comfort_stats(&mut s);
        if self.has_effect(FIRE_RESISTANCE) {
            s.fire_immune = true;
        }
        if self.has_effect(MANA_SICKNESS) {
            s.dmg[MAGIC] *= 0.75;
        }
        // Debuffs.
        if self.has_effect(BROKEN_ARMOR) {
            s.defense *= 0.5;
        }
        s.defense = (s.defense - 4.0 * lvl(ICHOR)).max(0.0);
        if self.wears("turtle_shell") && self.health < self.max_health * 0.5 {
            s.endurance += 0.25;
        }
        s.endurance = s.endurance.min(0.6);
        s.dodge = s.dodge.min(0.5);
        s.mana_cost = s.mana_cost.max(0.3);
        s
    }

    pub(crate) fn immune_to(&self, id: u8) -> bool {
        self.t5.rpg.stats.immune(id)
    }

    /// Armour points that soften a hit, after debuffs (whole points).
    pub(crate) fn defense_points(&self) -> u32 {
        self.t5.rpg.stats.defense.round().max(0.0) as u32
    }

    // -----------------------------------------------------------------------------------------
    // Each frame.

    pub(crate) fn update_rpg(&mut self, dt: f32) {
        self.t5.rpg.stats = self.compute_stats();
        if dt <= 0.0 {
            return;
        }
        let st = self.t5.rpg.stats.clone();
        // Mana: stars from accessories, faster refill.
        self.magic.extra_max = st.max_mana;
        let cap = self.magic.max_mana + self.magic.extra_max;
        if st.mana_regen > 0.0 {
            self.magic.mana = (self.magic.mana + st.mana_regen * dt).min(cap);
        }
        self.magic.mana = self.magic.mana.min(cap);
        if self.survival && self.screen != crate::Screen::Dead {
            // Life regeneration from gear and buffs; poison and bleeding stop it.
            let blocked = self.has_effect(BLEEDING) || self.has_effect(POISON) || self.has_effect(VENOM);
            if st.regen > 0.0 && !blocked && self.health < self.max_health {
                self.t5.rpg.regen_t += st.regen * dt;
                if self.t5.rpg.regen_t >= 1.0 {
                    self.t5.rpg.regen_t -= 1.0;
                    self.health = (self.health + 1.0).min(self.max_health);
                }
            }
            // Fire immunity puts the flames out.
            if st.fire_immune {
                self.extras.burning = 0.0;
            }
            self.t5.rpg.holy_cd -= dt;
            self.update_auras(dt);
        }
        self.update_mob_debuffs(dt);
        self.update_minions(dt);
        if let Some(t) = self.t5.rpg.tagged.as_mut() {
            t.1 -= dt;
        }
        self.t5.rpg.tagged = self.t5.rpg.tagged.filter(|t| t.1 > 0.0);
        self.update_info(dt);
    }

    /// Cozy fire near a campfire, the spelunker's glowing ores, the inferno ring, the leaf
    /// crystal of the verdite set.
    fn update_auras(&mut self, dt: f32) {
        self.t5.rpg.aura_t -= dt;
        let tick = self.t5.rpg.aura_t <= 0.0;
        if tick {
            self.t5.rpg.aura_t = 1.0;
        }
        let me = self.body.pos;
        let glow = self.glow_layer();
        if tick {
            let p = me.floor().as_ivec3();
            let mut fire = false;
            'scan: for dy in -2..=2 {
                for dz in -6..=6 {
                    for dx in -6..=6 {
                        let id = self.world.block_or_air(p + IVec3::new(dx, dy, dz));
                        if id != voxel::block::AIR && self.reg.get(self.reg.get(id).base).name.contains("campfire") {
                            fire = true;
                            break 'scan;
                        }
                    }
                }
            }
            if fire {
                self.add_effect(COZY_FIRE, 0, 3.0, true);
            }
            if self.has_effect(INFERNO) {
                for k in 0..self.mobs.list.len() {
                    let m = &self.mobs.list[k];
                    if !m.alive() || !self.is_hostile(k) || m.body.pos.distance(me) > 3.5 {
                        continue;
                    }
                    let m = &mut self.mobs.list[k];
                    m.on_fire = m.on_fire.max(3.0);
                    m.health -= 1.0;
                    m.hurt_age = 0.0;
                }
                for i in 0..16 {
                    let a = i as f32 / 16.0 * std::f32::consts::TAU;
                    self.particles.glow(me + Vec3::new(a.cos() * 3.0, 0.3, a.sin() * 3.0), Vec3::Y, glow, [1.6, 0.5, 0.1], 0.09, 0.5);
                }
            }
        }
        // Verdite: a leaf crystal over the shoulder shoots at monsters.
        if self.t5.rpg.stats.set.is_some_and(|i| SETS[i].prefix == "verdite") {
            let at = me + Vec3::new((self.now * 1.5).cos() * 0.8, 2.3, (self.now * 1.5).sin() * 0.8);
            self.particles.glow(at, Vec3::ZERO, glow, [0.5, 1.6, 0.4], 0.12, 0.06);
            self.t5.rpg.leaf_cd -= dt;
            if self.t5.rpg.leaf_cd <= 0.0 {
                if let Some(k) = self.nearest_hostile(at, 12.0) {
                    self.t5.rpg.leaf_cd = 0.8;
                    let c = self.mobs.list[k].body.center();
                    let seg = c - at;
                    for j in 0..10 {
                        self.particles.glow(at + seg * (j as f32 / 10.0), Vec3::ZERO, glow, [0.5, 1.6, 0.4], 0.07, 0.25);
                    }
                    let d = 5.0 * self.t5.rpg.stats.dmg[MAGIC];
                    self.mobs.list[k].hurt(d, at);
                }
            }
        }
        // Spelunker: ores around glow through the rock.
        if self.has_effect(SPELUNKER) {
            if tick || self.t5.rpg.ores.is_empty() && self.t5.rpg.scan_t <= 0.0 {
                self.t5.rpg.ores = self.scan_ores(10).into_iter().map(|(p, _)| p).collect();
                self.t5.rpg.scan_t = 1.0;
            }
            self.t5.rpg.scan_t -= dt;
            let n = self.t5.rpg.ores.len();
            for _ in 0..(n.min(40) as f32 * dt * 6.0).ceil() as usize {
                if n == 0 {
                    break;
                }
                let p = self.t5.rpg.ores[self.surv.rng.below(n as u32) as usize];
                let at = p.as_vec3() + Vec3::new(self.surv.rng.f32(), self.surv.rng.f32(), self.surv.rng.f32());
                self.particles.glow(at, Vec3::ZERO, glow, [1.6, 1.3, 0.4], 0.07, 0.6);
            }
        } else {
            self.t5.rpg.ores.clear();
        }
    }

    /// Ore blocks within `r`, with how valuable each is (higher is better).
    fn scan_ores(&mut self, r: i32) -> Vec<(IVec3, u8)> {
        if self.t5.rpg.ore_ids.is_none() {
            let mut ids = Vec::new();
            for (i, b) in self.reg.blocks.iter().enumerate() {
                let n = b.name.as_str();
                if !(n.ends_with("_ore") || n == "ancient_debris") {
                    continue;
                }
                let rank = ORE_RANK.iter().position(|o| n.contains(o)).map(|k| k as u8 + 1).unwrap_or(1);
                ids.push((i as voxel::BlockId, rank));
            }
            self.t5.rpg.ore_ids = Some(ids);
        }
        let ids = self.t5.rpg.ore_ids.clone().unwrap_or_default();
        let p = self.body.pos.floor().as_ivec3();
        let mut out = Vec::new();
        for dy in -r..=r {
            for dz in -r..=r {
                for dx in -r..=r {
                    let q = p + IVec3::new(dx, dy, dz);
                    let id = self.world.block_or_air(q);
                    if id == voxel::block::AIR {
                        continue;
                    }
                    if let Some(&(_, rank)) = ids.iter().find(|o| o.0 == id) {
                        out.push((q, rank));
                    }
                }
            }
        }
        out
    }

    pub(crate) fn is_hostile(&self, k: usize) -> bool {
        let d = &self.mobs.reg.defs[self.mobs.list[k].def];
        d.kind == voxel::mob::Kind::Hostile && !d.town
    }

    pub(crate) fn nearest_hostile(&self, at: Vec3, range: f32) -> Option<usize> {
        (0..self.mobs.list.len())
            .filter(|&k| self.mobs.list[k].alive() && self.is_hostile(k))
            .map(|k| (k, self.mobs.list[k].body.center().distance(at)))
            .filter(|&(_, d)| d < range)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(k, _)| k)
    }

    // -----------------------------------------------------------------------------------------
    // Debuffs on monsters.

    /// Put a debuff (or fire) on a monster.
    pub(crate) fn add_mob_debuff(&mut self, k: usize, id: u8, secs: f32) {
        let m = &mut self.mobs.list[k];
        if id == MOB_BURN {
            m.on_fire = m.on_fire.max(secs);
            return;
        }
        if id == CHILLED || id == SLOWNESS {
            m.chill = m.chill.max(secs);
        }
        match m.debuffs.iter_mut().find(|d| d.0 == id) {
            Some(d) => d.1 = d.1.max(secs),
            None => m.debuffs.push((id, secs)),
        }
    }

    fn update_mob_debuffs(&mut self, dt: f32) {
        self.t5.rpg.mob_t -= dt;
        let tick = self.t5.rpg.mob_t <= 0.0;
        if tick {
            self.t5.rpg.mob_t = 0.5;
        }
        let glow = self.glow_layer();
        let mut sparks = Vec::new();
        // Hunting banners near (tr_comfort.rs): those kinds take more damage.
        let hunt = &self.t5.tr.comfort.hunt;
        for m in self.mobs.list.iter_mut() {
            let base = if hunt.get(m.def).copied().unwrap_or(false) { 0.5 } else { 0.0 };
            if m.debuffs.is_empty() {
                m.vuln = base;
                continue;
            }
            let mut vuln = base;
            let mut dot = 0.0;
            for d in m.debuffs.iter_mut() {
                d.1 -= dt;
                if d.0 == ICHOR {
                    vuln += 0.25;
                }
                if d.0 == BROKEN_ARMOR {
                    vuln += 0.15;
                }
                if tick {
                    dot += mob_dot(d.0);
                }
            }
            if dot > 0.0 && m.alive() {
                m.health -= dot * 0.5;
                m.hurt_age = m.hurt_age.min(0.2);
            }
            m.vuln = vuln;
            m.debuffs.retain(|d| d.1 > 0.0);
            if m.alive() && !m.debuffs.is_empty() && fastrand_like(m.net_id, self.now) < dt * 8.0 {
                sparks.push((m.body.center(), mob_debuff_tint(m.debuffs[0].0)));
            }
        }
        for (c, tint) in sparks {
            self.particles.glow(c + Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() * 0.6, self.surv.rng.f32() - 0.5), Vec3::Y * 0.8, glow, tint, 0.07, 0.5);
        }
    }

    /// After a melee hit landed on monster `i`: debuffs from gear, titan knockback, whip marks,
    /// orichalcum petals.
    pub(crate) fn rpg_after_melee(&mut self, i: usize, dmg: f32) {
        let st = self.t5.rpg.stats.clone();
        for &(id, t) in &st.inflict {
            self.add_mob_debuff(i, id, t);
        }
        if st.knockback > 1.01 {
            let eye = self.body.eye();
            let m = &mut self.mobs.list[i];
            m.body.knockback(m.body.pos - eye, 3.0 * (st.knockback - 1.0));
        }
        let held = self.held_key();
        if let Some(&(_, _, eff)) = WHIPS.iter().find(|w| w.0 == held) {
            let id = self.mobs.list[i].net_id;
            let bonus = WHIPS.iter().find(|w| w.0 == held).map(|w| w.1).unwrap_or(0.0);
            self.t5.rpg.tagged = Some((id, 5.0, bonus));
            if eff != 0 {
                self.add_mob_debuff(i, eff, 4.0);
            }
        }
        self.orichalcum_petals(i, dmg);
    }

    /// Any hit by the player (melee, arrows, spells): the orichalcum set sometimes drops petals.
    pub(crate) fn orichalcum_petals(&mut self, i: usize, dmg: f32) {
        if !self.t5.rpg.stats.set.is_some_and(|s| SETS[s].prefix == "orichalcum") || self.surv.rng.f32() > 0.2 || i >= self.mobs.list.len() {
            return;
        }
        let c = self.mobs.list[i].body.center();
        let glow = self.glow_layer();
        for _ in 0..8 {
            let at = c + Vec3::new(self.surv.rng.f32() - 0.5, 2.0 + self.surv.rng.f32(), self.surv.rng.f32() - 0.5);
            self.particles.glow(at, Vec3::new(0.0, -4.0, 0.0), glow, [1.6, 0.5, 1.3], 0.08, 0.5);
        }
        self.mobs.list[i].hurt((dmg * 0.3).max(2.0), c + Vec3::Y * 2.0);
    }

    /// A monster died: the rare drops above (luck helps), and an emerald from the lucky coin.
    pub(crate) fn rpg_drops(&mut self, def: usize, at: Vec3) {
        if !self.survival {
            return;
        }
        let name = self.mobs.reg.defs[def].name.clone();
        let hostile = self.mobs.reg.defs[def].kind == voxel::mob::Kind::Hostile;
        let luck = 1.0 + 0.5 * self.t5.rpg.stats.luck + 0.25 * self.effect_level(LUCK) as f32;
        let mut out: Vec<&str> = DROPS.iter().filter(|d| d.0 == name && self.surv.rng.f32() < d.2 * luck).map(|d| d.1).collect();
        if hostile && self.wears("lucky_coin") && self.surv.rng.f32() < 0.05 {
            out.push("emerald");
        }
        for item in out {
            if let Some(id) = self.items.id(item) {
                let v = Vec3::new(self.surv.rng.f32() - 0.5, 3.0, self.surv.rng.f32() - 0.5);
                self.spawn_drop(at + Vec3::Y * 0.5, ItemStack::new(id, 1), v);
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Hits on the player.

    /// A monster's hit: maybe dodged (mirror, radiant set). True when it missed.
    pub(crate) fn rpg_dodge(&mut self, from: Option<Vec3>) -> bool {
        if from.is_none() || !self.survival {
            return false;
        }
        let radiant = self.t5.rpg.stats.set.is_some_and(|s| SETS[s].prefix == "radiant") && self.t5.rpg.holy_cd <= 0.0;
        let dodge = self.t5.rpg.stats.dodge;
        if !(radiant || dodge > 0.0 && self.surv.rng.f32() < dodge) {
            return false;
        }
        if radiant {
            self.t5.rpg.holy_cd = 30.0;
        }
        self.hurt_age = 0.0;
        let glow = self.glow_layer();
        for _ in 0..14 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() * 0.5, self.surv.rng.f32() - 0.5) * 4.0;
            self.particles.glow(self.body.pos + Vec3::Y, v, glow, [1.4, 1.4, 1.6], 0.08, 0.4);
        }
        self.toast("Esquivou!", 1.0);
        self.sfx(Sfx::Place(Material::Cloth));
        true
    }

    /// Damage after defense above the classic 20 points and endurance.
    pub(crate) fn rpg_reduce(&self, amount: f32, from: Option<Vec3>) -> f32 {
        let st = &self.t5.rpg.stats;
        let mut a = amount;
        if from.is_some() {
            // The classic armour formula covers 20 points; each point above softens 2% more.
            let extra = (st.defense - 20.0).max(0.0);
            a *= (1.0 - extra * 0.02).max(0.5);
        }
        a * (1.0 - st.endurance)
    }

    /// After the player was hurt: thorns, arcane cuffs, panic charm, honey locket, star veil.
    pub(crate) fn rpg_after_hurt(&mut self, amount: f32, from: Option<Vec3>) {
        let Some(f) = from else { return };
        self.tr_after_hurt(amount, f);
        let thorns = self.t5.rpg.stats.thorns;
        if thorns > 0.0 {
            if let Some(k) = self.nearest_hostile(f, 3.0) {
                let me = self.body.pos;
                self.mobs.list[k].hurt((amount * thorns).max(1.0), me);
            }
        }
        if self.wears("arcane_cuffs") {
            let cap = self.magic.max_mana + self.magic.extra_max;
            self.magic.mana = (self.magic.mana + amount * 2.0).min(cap);
        }
        if self.wears("panic_charm") {
            self.add_effect(SPEED, 1, 4.0, false);
        }
        if self.wears("honey_locket") {
            self.add_effect(REGENERATION, 0, 5.0, false);
        }
        if self.wears("star_veil") {
            let glow = self.glow_layer();
            let me = self.body.pos;
            for _ in 0..3 {
                let Some(k) = self.nearest_hostile(me, 8.0) else { break };
                let c = self.mobs.list[k].body.center();
                for j in 0..12 {
                    let at = c + Vec3::new(1.5, 8.0, 0.5) * (1.0 - j as f32 / 12.0);
                    self.particles.glow(at, Vec3::ZERO, glow, [1.6, 1.4, 0.6], 0.1, 0.35);
                }
                self.mobs.list[k].hurt(6.0 * self.t5.rpg.stats.dmg[MAGIC], c + Vec3::Y * 8.0);
            }
        }
    }

    /// A monster's hit landed: its debuff on the player.
    pub(crate) fn rpg_mob_hit(&mut self, from: Option<Vec3>) {
        let Some(f) = from else { return };
        if !self.survival {
            return;
        }
        let near = self.mobs.list.iter().enumerate().filter(|(_, m)| m.alive()).map(|(i, m)| (i, m.body.center().distance(f))).min_by(|a, b| a.1.total_cmp(&b.1));
        let Some((i, d)) = near else { return };
        let def = &self.mobs.reg.defs[self.mobs.list[i].def];
        if d > 3.5 && !def.ranged {
            return;
        }
        let Some(&(_, id, chance, secs)) = MOB_DEBUFFS.iter().find(|m| m.0 == def.name) else { return };
        if self.surv.rng.f32() > chance {
            return;
        }
        if id == MOB_BURN {
            if !self.t5.rpg.stats.fire_immune {
                self.extras.burning = self.extras.burning.max(secs);
            }
        } else {
            self.add_effect(id, 0, secs, false);
        }
    }

    /// Movement keys after debuffs: confusion turns them around, frozen stops them.
    pub(crate) fn rpg_wish(&self, wish: Vec3) -> Vec3 {
        if self.has_effect(FROZEN) {
            Vec3::ZERO
        } else if self.has_effect(CONFUSED) {
            -wish
        } else {
            wish
        }
    }

    // -----------------------------------------------------------------------------------------
    // Potions, consumables, staffs.

    /// Right click with an item of this file; true when it did something.
    pub(crate) fn rpg_use(&mut self, name: &str) -> Option<bool> {
        if RESTORE.iter().any(|r| r.0 == name) {
            let sel = self.selected;
            return Some(self.drink_restore(sel));
        }
        if let Some(k) = class_of_minion(name) {
            return Some(self.summon_minion(k));
        }
        match name {
            "ancestral_heart" | "celestial_socket" => {
                let need = if name == "ancestral_heart" { 0 } else { 1 };
                if self.t5.rpg.extra_slots != need {
                    let msg = if self.t5.rpg.extra_slots > need { "Você já usou isso" } else { "Use primeiro o Coração Ancestral" };
                    self.toast(msg, 2.5);
                    return Some(true);
                }
                self.t5.rpg.extra_slots += 1;
                self.consume_held();
                self.sfx(Sfx::Xp);
                self.toast(format!("Novo espaço de acessório! Agora são {}", self.acc_slots()), 3.0);
                Some(true)
            }
            "life_fruit" => {
                if self.max_health < 40.0 {
                    self.toast("Use cristais de vida até ter 20 corações", 2.5);
                    return Some(true);
                }
                if self.max_health >= 50.0 {
                    self.toast("Sua vida já está no máximo", 2.0);
                    return Some(true);
                }
                self.max_health = (self.max_health + 2.0).min(50.0);
                self.health = (self.health + 2.0).min(self.max_health);
                self.consume_held();
                self.sfx(Sfx::Xp);
                self.toast(format!("Vida máxima: {} corações", (self.max_health / 2.0) as i32), 3.0);
                Some(true)
            }
            _ => None,
        }
    }

    fn consume_held(&mut self) {
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
    }

    /// Drink the healing or mana potion in inventory slot `slot`.
    fn drink_restore(&mut self, slot: usize) -> bool {
        let Some(st) = self.surv.inv.slots[slot] else { return false };
        let name = self.items.get(st.item).name.clone();
        let Some(&(_, life, mana, sick)) = RESTORE.iter().find(|r| r.0 == name) else { return false };
        if life > 0.0 && self.has_effect(POTION_SICKNESS) {
            self.toast("Enjoo de poção: espere um pouco", 1.5);
            return true;
        }
        if life > 0.0 {
            // The alchemist stone idea lives in the sickness time: shorter with restoration.
            self.health = (self.health + life).min(self.max_health);
            self.add_effect(POTION_SICKNESS, 0, sick, false);
        }
        let mana = if name == "restoration_potion" { 10.0 } else { mana };
        if mana > 0.0 {
            let cap = self.magic.max_mana + self.magic.extra_max;
            self.magic.mana = (self.magic.mana + mana).min(cap);
            self.add_effect(MANA_SICKNESS, 0, 5.0, false);
        }
        if self.survival {
            self.surv.inv.take(slot, 1);
            if let Some(b) = self.items.id("glass_bottle") {
                if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(b, 1)) {
                    self.throw_stack(left);
                }
            }
        }
        self.sfx(Sfx::Bubbles);
        self.prog.stats.add("potions_drunk", 1);
        true
    }

    /// The best potion in the inventory for what is missing: the smallest that fills the gap,
    /// else the biggest.
    fn best_restore(&self, life: bool) -> Option<usize> {
        let missing = if life { self.max_health - self.health } else { self.magic.max_mana + self.magic.extra_max - self.magic.mana };
        let mut cands: Vec<(usize, f32)> = Vec::new();
        for (i, s) in self.surv.inv.slots.iter().enumerate() {
            let Some(s) = s else { continue };
            let n = &self.items.get(s.item).name;
            if let Some(r) = RESTORE.iter().find(|r| r.0 == n) {
                let v = if life { r.1 } else if r.0 == "restoration_potion" { 10.0 } else { r.2 };
                if v > 0.0 {
                    cands.push((i, v));
                }
            }
        }
        let fill = cands.iter().filter(|c| c.1 >= missing).min_by(|a, b| a.1.total_cmp(&b.1));
        fill.or_else(|| cands.iter().max_by(|a, b| a.1.total_cmp(&b.1))).map(|c| c.0)
    }

    pub(crate) fn quick_heal(&mut self) {
        if !self.survival || self.health >= self.max_health {
            return;
        }
        match self.best_restore(true) {
            Some(i) => {
                self.drink_restore(i);
            }
            None => self.toast("Nenhuma poção de vida no inventário", 1.5),
        }
    }

    pub(crate) fn quick_mana(&mut self) -> bool {
        if self.magic.mana >= self.magic.max_mana + self.magic.extra_max - 0.01 {
            return false;
        }
        match self.best_restore(false) {
            Some(i) => self.drink_restore(i),
            None => false,
        }
    }

    /// B: drink every buff potion in the inventory whose effect isn't on yet.
    pub(crate) fn quick_buff(&mut self) {
        let mut drank = 0;
        for i in 0..self.surv.inv.slots.len() {
            let Some(s) = self.surv.inv.slots[i] else { continue };
            let name = self.items.get(s.item).name.clone();
            let Some((form, p)) = potion_of(&name) else { continue };
            if form != Form::Drink || p.effects.is_empty() {
                continue;
            }
            let good = p.effects.iter().all(|&(e, _, t)| t > 0.0 && !info(e).2);
            if !good || p.effects.iter().all(|&(e, _, _)| self.has_effect(e)) {
                continue;
            }
            for &(e, a, t) in p.effects {
                self.add_effect(e, a, t, false);
            }
            drank += 1;
            if self.survival {
                self.surv.inv.take(i, 1);
                if let Some(b) = self.items.id("glass_bottle") {
                    if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(b, 1)) {
                        self.throw_stack(left);
                    }
                }
            }
        }
        if drank > 0 {
            self.sfx(Sfx::Bubbles);
            self.toast(format!("{drank} poção(ões) bebida(s)"), 1.5);
        } else {
            self.toast("Nenhuma poção de efeito nova no inventário", 1.5);
        }
    }

    /// Eating a real meal makes you well fed for a while.
    pub(crate) fn rpg_ate(&mut self, hunger: u32, saturation: f32) {
        if hunger >= 4 {
            let secs = 30.0 * (hunger as f32 + saturation);
            self.add_effect(WELL_FED, 0, secs.min(900.0), false);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Minions.

    fn summon_minion(&mut self, kind: usize) -> bool {
        let cost = 10.0 * self.t5.rpg.stats.mana_cost;
        if self.survival {
            if self.magic.mana < cost && !(self.wears("mana_blossom") && self.quick_mana()) {
                self.toast("Mana insuficiente", 1.0);
                return true;
            }
            self.magic.mana = (self.magic.mana - cost).max(0.0);
        }
        let slots = self.t5.rpg.stats.minions as usize;
        while self.t5.rpg.minions.len() >= slots && !self.t5.rpg.minions.is_empty() {
            self.t5.rpg.minions.remove(0);
        }
        let at = self.body.pos + Vec3::Y * 1.8;
        let phase = self.surv.rng.f32() * std::f32::consts::TAU;
        self.t5.rpg.minions.push(Minion { kind, pos: at, vel: Vec3::ZERO, cd: 0.5, phase });
        let glow = self.glow_layer();
        for _ in 0..16 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5) * 3.0;
            self.particles.glow(at, v, glow, MINIONS[kind].tint, 0.1, 0.5);
        }
        self.sfx(Sfx::Cast(3));
        self.anim.start_swing();
        self.toast(format!("{} ({}/{})", MINIONS[kind].name, self.t5.rpg.minions.len(), slots), 1.5);
        true
    }

    fn update_minions(&mut self, dt: f32) {
        if self.t5.rpg.minions.is_empty() {
            return;
        }
        if self.screen == crate::Screen::Dead {
            self.t5.rpg.minions.clear();
            return;
        }
        // Fewer slots than minions (a buff ran out): the oldest go.
        let slots = self.t5.rpg.stats.minions as usize;
        while self.t5.rpg.minions.len() > slots {
            self.t5.rpg.minions.remove(0);
        }
        let me = self.body.pos + Vec3::Y * 1.6;
        let glow = self.glow_layer();
        let tagged = self.t5.rpg.tagged.and_then(|(id, _, bonus)| self.mobs.list.iter().position(|m| m.net_id == id && m.alive()).map(|k| (k, bonus)));
        let power = self.t5.rpg.stats.dmg[SUMMON];
        let n = self.t5.rpg.minions.len();
        let mut minions = std::mem::take(&mut self.t5.rpg.minions);
        for (idx, mi) in minions.iter_mut().enumerate() {
            let def = &MINIONS[mi.kind];
            mi.cd -= dt;
            mi.phase += dt * 2.0;
            if mi.pos.distance(me) > 40.0 {
                mi.pos = me;
                mi.vel = Vec3::ZERO;
            }
            let target = tagged.filter(|&(k, _)| self.mobs.list[k].body.center().distance(me) < 30.0).or_else(|| self.nearest_hostile(me, 20.0).map(|k| (k, 0.0)));
            let goal = match target {
                Some((k, _)) => {
                    let c = self.mobs.list[k].body.center();
                    if def.ranged {
                        // Keep a few blocks away, above the monster.
                        let off = (mi.pos - c).normalize_or_zero();
                        c + Vec3::new(off.x, 0.0, off.z).normalize_or_zero() * 4.0 + Vec3::Y * 2.0
                    } else {
                        c
                    }
                }
                None => {
                    // Hover in a ring behind the player.
                    let a = idx as f32 / n.max(1) as f32 * std::f32::consts::TAU + self.now * 0.4;
                    me + Vec3::new(a.cos() * 1.6, 0.5 + (mi.phase).sin() * 0.25, a.sin() * 1.6)
                }
            };
            let to = goal - mi.pos;
            let speed: f32 = if target.is_some() && !def.ranged { 14.0 } else { 9.0 };
            let want = to.normalize_or_zero() * speed.min(to.length() * 4.0);
            mi.vel += (want - mi.vel) * (dt * 5.0).min(1.0);
            mi.pos += mi.vel * dt;
            if let Some((k, bonus)) = target {
                let c = self.mobs.list[k].body.center();
                let reach = if def.ranged { 12.0 } else { 1.3 + self.mobs.list[k].body.half_width };
                if mi.cd <= 0.0 && mi.pos.distance(c) < reach {
                    mi.cd = def.cooldown;
                    if def.ranged {
                        let seg = c - mi.pos;
                        for j in 0..10 {
                            self.particles.glow(mi.pos + seg * (j as f32 / 10.0), seg.normalize_or_zero() * 2.0, glow, def.tint, 0.07, 0.25);
                        }
                    }
                    let from = mi.pos;
                    let hit = (def.damage + bonus) * power;
                    self.mobs.list[k].hurt(hit, from);
                    if def.inflict != 0 {
                        self.add_mob_debuff(k, def.inflict, 3.0);
                    }
                    let d = &self.mobs.reg.defs[self.mobs.list[k].def];
                    self.last_hit = Some((d.display_name.clone(), self.mobs.list[k].health.max(0.0), self.mobs.list[k].max_health, self.now + 3.0));
                }
            }
            if self.surv.rng.f32() < dt * 10.0 {
                self.particles.glow(mi.pos, -mi.vel * 0.1, glow, def.tint, 0.06, 0.4);
            }
        }
        self.t5.rpg.minions = minions;
    }

    /// Minions drawn as little sprites of their pictures.
    pub(crate) fn minion_instances(&self, cam_block: IVec3) -> Vec<GpuItem> {
        let c = cam_block.as_vec3();
        let mut out = Vec::new();
        for mi in &self.t5.rpg.minions {
            let def = &MINIONS[mi.kind];
            let Some(id) = self.items.id(def.sprite) else { continue };
            let voxel::item::Look::Sprite(l) = self.items.get(id).look else { continue };
            let layer = (self.item_base + l) as f32;
            let p = mi.pos + Vec3::Y * (mi.phase * 2.0).sin() * 0.06 - c;
            // Turned to face where it flies, or the player when it waits.
            let dir = if mi.vel.length() > 0.5 { mi.vel } else { self.body.pos - mi.pos };
            let yaw = dir.x.atan2(dir.z) + std::f32::consts::FRAC_PI_2;
            out.push(GpuItem { pos: [p.x, p.y, p.z, def.size], info: [yaw, 1.0, 1.0, 0.8], layers: [layer, layer, layer, 0.0], rot: [0.0; 4], ..bytemuck::Zeroable::zeroed() });
        }
        out
    }

    // -----------------------------------------------------------------------------------------
    // Accessory slots, loadouts.

    /// Switch to loadout `k`: what is worn goes into the current one, `k`'s gear comes out.
    pub(crate) fn switch_loadout(&mut self, k: usize) {
        let cur = self.t5.rpg.loadout;
        if k == cur || k >= 3 {
            return;
        }
        let worn = Gear { armor: self.surv.armor, acc: self.t5.acc };
        let next = std::mem::take(&mut self.t5.rpg.loadouts[k]);
        self.t5.rpg.loadouts[cur] = worn;
        self.surv.armor = next.armor;
        self.t5.acc = next.acc;
        self.t5.rpg.loadout = k;
        self.sfx(Sfx::Place(Material::Cloth));
        self.toast(format!("Equipamento {}", k + 1), 1.5);
    }

    /// The three loadout buttons over the accessory column; a click switches.
    pub(crate) fn draw_loadout_buttons(&mut self, x: f32, y: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        for k in 0..3 {
            let bx = x + k as f32 * 8.0 * s;
            let over = mouse.0 >= bx && mouse.0 < bx + 7.0 * s && mouse.1 >= y && mouse.1 < y + 7.0 * s;
            let on = k == self.t5.rpg.loadout;
            let col = if on { [90, 160, 90, 255] } else if over { [170, 170, 170, 255] } else { [120, 120, 120, 255] };
            self.ui.rect(bx, y, 7.0 * s, 7.0 * s, [40, 40, 40, 255]);
            self.ui.rect(bx + s * 0.5, y + s * 0.5, 6.0 * s, 6.0 * s, col);
            self.ui.text(bx + 1.5 * s, y + 0.2 * s, &(k + 1).to_string(), [255, 255, 255, 255]);
            if over && self.clicked && self.surv.cursor.is_none() {
                self.clicked = false;
                self.switch_loadout(k);
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Tooltips, the stats sheet, info accessories.

    /// Stat lines for an accessory or a piece of a set.
    pub(crate) fn rpg_details(&self, name: &str, out: &mut Vec<(String, [u8; 4])>) {
        const BLUE: [u8; 4] = [170, 200, 255, 255];
        const GREEN: [u8; 4] = [120, 220, 120, 255];
        if let Some(a) = accessory(name) {
            if !a.desc.is_empty() {
                out.push((a.desc.to_string(), BLUE));
            }
            for m in a.mods {
                if let Some(l) = mod_line(m) {
                    out.push((l, BLUE));
                }
            }
            out.push(("Acessório".to_string(), [150, 150, 150, 255]));
        }
        if let Some(r) = RESTORE.iter().find(|r| r.0 == name) {
            if r.1 > 0.0 {
                out.push((format!("Cura {:.0} de vida (H)", r.1), GREEN));
            }
            let mana = if r.0 == "restoration_potion" { 10.0 } else { r.2 };
            if mana > 0.0 {
                out.push((format!("Devolve {:.0} de mana (N)", mana), BLUE));
            }
        }
        if let Some(m) = MINIONS.iter().find(|m| m.staff == name) {
            out.push((format!("Invoca: {} · Dano {:.0} · Mana 10", m.name, m.damage), BLUE));
            out.push(("Usa um espaço de lacaio".to_string(), [150, 150, 150, 255]));
        }
        if is_whip(name) {
            out.push(("Marca o monstro: seus lacaios vão atrás dele".to_string(), BLUE));
        }
        match name {
            "ancestral_heart" => out.push(("Abre o 6º espaço de acessório".to_string(), GREEN)),
            "celestial_socket" => out.push(("Abre o 7º espaço de acessório".to_string(), GREEN)),
            "life_fruit" => out.push(("+1 coração (depois dos 20), até 25".to_string(), GREEN)),
            _ => {}
        }
        if let Some(set) = SETS.iter().find(|s| name.strip_prefix(s.prefix).is_some_and(|r| r.starts_with('_')) && (name.ends_with("_helmet") || name.ends_with("_chestplate") || name.ends_with("_leggings") || name.ends_with("_boots"))) {
            let worn = self.t5.rpg.stats.set.is_some_and(|i| SETS[i].prefix == set.prefix);
            let col = if worn { GREEN } else { [140, 140, 140, 255] };
            out.push(("Bônus do conjunto completo:".to_string(), col));
            if !set.desc.is_empty() {
                out.push((format!(" {}", set.desc), col));
            }
            for m in set.mods {
                if let Some(l) = mod_line(m) {
                    out.push((format!(" {l}"), col));
                }
            }
        }
    }

    fn sheet_lines(&self) -> Vec<(String, [u8; 4])> {
        const W: [u8; 4] = [240, 240, 240, 255];
        const G: [u8; 4] = [170, 170, 170, 255];
        let st = &self.t5.rpg.stats;
        let pct = |v: f32| format!("{:+.0}%", (v - 1.0) * 100.0);
        let mut out = vec![
            (format!("Defesa: {:.0}", st.defense), W),
            (format!("Dano recebido: -{:.0}%", (1.0 - (1.0 - (st.defense.min(20.0) * 0.04)) * (1.0 - ((st.defense - 20.0).max(0.0) * 0.02).min(0.5)) * (1.0 - st.endurance)) * 100.0), G),
            (format!("Vida: {:.0}/{:.0}  Mana: {:.0}/{:.0}", self.health.max(0.0), self.max_health, self.magic.mana, self.magic.max_mana + self.magic.extra_max), W),
        ];
        let names = ["Corpo a corpo", "Distância", "Magia", "Invocação"];
        for c in 0..4 {
            out.push((format!("{}: {} dano, {:.0}% crít.", names[c], pct(st.dmg[c]), st.crit[c] * 100.0), W));
        }
        out.push((format!("Vel. de ataque: {}  Andar: {}", pct(st.melee_speed), pct(st.move_speed)), W));
        out.push((format!("Lacaios: {}/{}", self.t5.rpg.minions.len(), st.minions), W));
        if st.regen > 0.0 {
            out.push((format!("Regeneração: {:.2} vida/s", st.regen), G));
        }
        if st.mana_cost < 0.999 {
            out.push((format!("Custo de mana: -{:.0}%", (1.0 - st.mana_cost) * 100.0), G));
        }
        if st.dodge > 0.0 {
            out.push((format!("Esquiva: {:.0}%", st.dodge * 100.0), G));
        }
        if st.thorns > 0.0 {
            out.push((format!("Espinhos: {:.0}%", st.thorns * 100.0), G));
        }
        let imm = (1..=ALL).filter(|&e| st.immune(e)).count();
        if imm > 0 {
            out.push((format!("Imunidades: {imm}"), G));
        }
        if let Some(i) = st.set {
            out.push((format!("Conjunto: {}", SET_NAMES.iter().find(|n| n.0 == SETS[i].prefix).map(|n| n.1).unwrap_or(SETS[i].prefix)), [120, 220, 120, 255]));
        }
        out.push((format!("Acessórios: {} espaços", self.acc_slots()), G));
        out
    }

    /// The stats panel: beside the accessory column in the inventory, or over the game (C).
    pub(crate) fn draw_stats_sheet(&mut self, right: f32, y: f32) {
        let s = self.ui.scale;
        let lines = self.sheet_lines();
        let tw = lines.iter().map(|(t, _)| self.ui.text_width(t, s)).fold(0.0, f32::max);
        let (pw, ph) = (tw + 8.0 * s, (lines.len() as f32 * 9.0 + 16.0) * s);
        let x = (right - pw).max(2.0 * s);
        self.ui.rect(x, y, pw, ph, [20, 16, 28, 215]);
        self.ui.frame(x, y, pw, ph, s, [110, 90, 150, 255]);
        self.ui.text(x + 4.0 * s, y + 3.0 * s, "Status", [255, 230, 140, 255]);
        for (k, (t, c)) in lines.iter().enumerate() {
            self.ui.text(x + 4.0 * s, y + (13.0 + k as f32 * 9.0) * s, t, *c);
        }
    }

    fn update_info(&mut self, dt: f32) {
        let info = self.t5.rpg.stats.info;
        if info & INFO_DPS != 0 {
            let me = self.body.pos;
            let mut dealt = 0.0;
            let mut seen = HashMap::new();
            for m in &self.mobs.list {
                if m.body.pos.distance(me) > 32.0 {
                    continue;
                }
                let h = m.health.max(0.0);
                if let Some(&last) = self.t5.rpg.dps_last.get(&m.net_id) {
                    if last > h && m.hurt_age < 0.6 {
                        dealt += last - h;
                    }
                }
                seen.insert(m.net_id, h);
            }
            self.t5.rpg.dps_last = seen;
            let now = self.now;
            if dealt > 0.0 {
                self.t5.rpg.dps.push((now, dealt));
            }
            self.t5.rpg.dps.retain(|d| now - d.0 < 3.0);
        }
        if info & (INFO_RADAR | INFO_ORE) != 0 {
            self.t5.rpg.scan_t -= dt;
            if self.t5.rpg.scan_t <= 0.0 {
                self.t5.rpg.scan_t = 1.0;
                let me = self.body.pos;
                self.t5.rpg.radar = (0..self.mobs.list.len()).filter(|&k| self.mobs.list[k].alive() && self.is_hostile(k) && self.mobs.list[k].body.pos.distance(me) < 48.0).count();
                if info & INFO_ORE != 0 {
                    let ores = self.scan_ores(12);
                    self.t5.rpg.best_ore = ores.iter().max_by_key(|o| o.1).map(|&(p, _)| {
                        let id = self.world.block_or_air(p);
                        (self.reg.get(id).display_name.clone(), (p.as_vec3() + Vec3::splat(0.5)).distance(me))
                    });
                }
            }
        }
    }

    /// Lines of the info accessories, bottom left.
    pub(crate) fn draw_info_hud(&mut self, h: f32) {
        let info = self.t5.rpg.stats.info;
        if info == 0 || !self.survival {
            return;
        }
        let s = self.ui.scale;
        let mut lines: Vec<String> = Vec::new();
        let p = self.body.pos;
        if info & INFO_DEPTH != 0 {
            let depth = 64.0 - p.y;
            lines.push(if depth > 0.0 { format!("Profundidade: {:.0} abaixo do mar", depth) } else { format!("Altura: {:.0} acima do mar", -depth) });
            // The layer, and the underground biome once below the surface (deep.rs).
            let layer = self.layer();
            let ub = self.ubiome();
            lines.push(if p.y < 0.0 && ub != voxel::worldgen::deep::UBiome::Plain { format!("{} · {}", layer.name(), ub.name()) } else { layer.name().to_string() });
        }
        if info & INFO_COORDS != 0 {
            lines.push(format!("Posição: {:.0}, {:.0}", p.x, p.z));
        }
        if info & INFO_TIME != 0 {
            let t = (self.time_of_day / 1000.0 + 6.0) % 24.0;
            lines.push(format!("Hora: {:02}:{:02}", t as u32, ((t.fract()) * 60.0) as u32));
        }
        if info & INFO_WEATHER != 0 {
            let w = if self.weather.state.thunder {
                "tempestade"
            } else if self.weather.state.raining {
                "chuva"
            } else {
                "céu limpo"
            };
            let blood = if self.t5.hard.blood_moon() { " · lua de sangue" } else { "" };
            lines.push(format!("Clima: {w}{blood}"));
        }
        if info & INFO_RADAR != 0 {
            lines.push(format!("Monstros por perto: {}", self.t5.rpg.radar));
        }
        if info & INFO_DPS != 0 {
            let sum: f32 = self.t5.rpg.dps.iter().map(|d| d.1).sum();
            lines.push(format!("Dano por segundo: {:.1}", sum / 3.0));
        }
        if info & INFO_ORE != 0 {
            lines.push(match &self.t5.rpg.best_ore {
                Some((n, d)) => format!("Minério: {n} a {d:.0} blocos"),
                None => "Nenhum minério por perto".to_string(),
            });
        }
        let y0 = h - (24.0 + 10.0 * lines.len() as f32) * s;
        for (k, l) in lines.iter().enumerate() {
            self.ui.text(4.0 * s, y0 + k as f32 * 10.0 * s, l, [220, 220, 160, 255]);
        }
    }

    /// The hunter potion: hostile monsters marked through walls.
    pub(crate) fn draw_hunter_marks(&mut self, w: f32, h: f32) {
        if !self.has_effect(HUNTER) {
            return;
        }
        let Some((vp, cam_block)) = self.ui_view_proj else { return };
        let s = self.ui.scale;
        let eye = self.body.eye();
        let marks: Vec<Vec3> = (0..self.mobs.list.len()).filter(|&k| self.mobs.list[k].alive() && self.is_hostile(k)).map(|k| self.mobs.list[k].body.center()).filter(|c| c.distance(eye) < 48.0).collect();
        for c in marks {
            let v = vp * (c - cam_block.as_vec3()).extend(1.0);
            if v.w < 0.1 {
                continue;
            }
            let (nx, ny) = (v.x / v.w, v.y / v.w);
            if nx.abs() > 1.0 || ny.abs() > 1.0 {
                continue;
            }
            let (x, y) = ((nx * 0.5 + 0.5) * w, (0.5 - ny * 0.5) * h);
            self.ui.frame(x - 4.0 * s, y - 4.0 * s, 8.0 * s, 8.0 * s, s, [255, 90, 60, 220]);
        }
    }

    /// Test scenes (MC_RPG=name): the stats sheet, minions fighting, debuffs.
    pub(crate) fn test_rpg(&mut self, scene: &str) {
        if !self.t5.loaded {
            self.t5_load();
        }
        let give = |g: &mut Game, name: &str, slot: usize, n: u8| {
            if let Some(id) = g.items.id(name) {
                g.surv.inv.slots[slot] = Some(ItemStack::new(id, n));
            }
        };
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        match scene {
            "sheet" | "inventory" => {
                self.survival = true;
                self.t5.rpg.extra_slots = 2;
                let accs = ["champion_emblem", "ankh_shield", "sky_stone", "spirit_bell", "mirror_charm", "explorer_kit", "frost_spark_boots"];
                for (k, n) in accs.iter().enumerate() {
                    if let Some(id) = self.items.id(n) {
                        self.t5.acc[k] = Some(ItemStack::new(id, 1));
                    }
                }
                for (k, n) in ["mythril_helmet", "mythril_chestplate", "mythril_leggings", "mythril_boots"].iter().enumerate() {
                    if let Some(id) = self.items.id(n) {
                        self.surv.armor[k] = Some(ItemStack::new(id, 1));
                    }
                }
                let items = [
                    "lesser_healing_potion", "healing_potion", "greater_healing_potion", "super_healing_potion", "restoration_potion", "lesser_mana_potion", "mana_potion", "greater_mana_potion", "life_fruit",
                    "wisp_staff", "wasp_staff", "imp_staff", "blade_staff", "star_dragon_staff", "leather_whip", "thorn_whip", "ancestral_heart", "celestial_socket",
                    "potion_ironskin", "potion_wrath", "potion_rage", "potion_endurance", "potion_lifeforce", "potion_summoning", "potion_spelunker", "potion_hunter", "potion_inferno",
                    "claw_glove", "giant_gauntlet", "ember_stone", "hunter_quiver", "mana_blossom", "warding_scarf", "turtle_shell", "steam_boots", "ore_finder",
                ];
                for (k, n) in items.iter().enumerate() {
                    give(self, n, k, 1);
                }
                for (e, t) in [(IRONSKIN, 300.0), (WRATH, 200.0), (WELL_FED, 600.0), (SUMMONING, 300.0)] {
                    self.add_effect(e, 0, t, false);
                }
                self.t5.rpg.stats = self.compute_stats();
                if scene == "inventory" {
                    self.args.show_inventory = true;
                } else {
                    self.t5.rpg.sheet = true;
                }
            }
            "minions" => {
                self.survival = true;
                self.selected = 0;
                give(self, "star_dragon_staff", 0, 1);
                self.add_effect(SUMMONING, 0, 300.0, false);
                self.t5.rpg.stats = self.compute_stats();
                for k in [3, 4, 0] {
                    self.summon_minion(k);
                }
                let at = self.body.pos + flat * 7.0;
                let y = ground(self, at);
                for n in ["zombie", "skeleton"] {
                    self.mobs.spawn(n, Vec3::new(at.x, y, at.z));
                    if let Some(m) = self.mobs.list.last_mut() {
                        m.aggro = true;
                    }
                }
                for _ in 0..40 {
                    self.update_minions(0.03);
                }
                // In view for the picture: spread before the monsters.
                let side = flat.cross(Vec3::Y);
                for (i, mi) in self.t5.rpg.minions.iter_mut().enumerate() {
                    mi.pos = self.body.eye() + flat * 3.5 + side * (i as f32 - 1.0) * 1.2;
                    mi.vel = flat;
                }
            }
            "debuffs" => {
                self.survival = true;
                for (e, t) in [(BLEEDING, 20.0), (FROSTBURN, 10.0), (ICHOR, 20.0), (BROKEN_ARMOR, 20.0), (WELL_FED, 300.0), (IRONSKIN, 300.0), (MAGIC_POWER, 200.0), (HUNTER, 200.0)] {
                    self.add_effect(e, 0, t, false);
                }
                let at = self.body.pos + flat * 6.0;
                let y = ground(self, at);
                self.mobs.spawn("zombie", Vec3::new(at.x, y, at.z));
                let k = self.mobs.list.len() - 1;
                self.add_mob_debuff(k, CURSED_FLAMES, 30.0);
                self.add_mob_debuff(k, ICHOR, 30.0);
                self.t5.rpg.sheet = true;
            }
            _ => log::warn!("unknown MC_RPG scene {scene}"),
        }
    }
}

/// Display names of the sets for the sheet.
const SET_NAMES: &[(&str, &str)] = &[
    ("copper", "Cobre"),
    ("bronze", "Bronze"),
    ("lead", "Chumbo"),
    ("silver", "Prata"),
    ("tungsten", "Tungstênio"),
    ("platinum", "Platina"),
    ("cobalt", "Cobalto"),
    ("palladium", "Paládio"),
    ("mythril", "Mitril"),
    ("orichalcum", "Oricalco"),
    ("adamantite", "Adamantita"),
    ("titanium", "Titânio"),
    ("radiant", "Radiante"),
    ("verdite", "Verdita"),
    ("lumite", "Lumita"),
    ("meteor", "Meteoro"),
    ("umbral", "Sombraço"),
    ("sanguinite", "Sanguinita"),
    ("hive", "Abelha"),
    ("jungle", "Selva"),
    ("fossil", "Fóssil"),
    ("necro", "Necro"),
    ("ninja", "Ninja"),
    ("spectral", "Espectral"),
    ("sporite", "Esporita"),
    ("beetle", "Escamada"),
    ("turtle", "Tartaruga"),
    ("solar", "Solar"),
    ("vortex", "Vórtice"),
    ("nebula", "Nebulosa"),
    ("stardust", "Pó Estelar"),
];

/// Ores from least to most valuable, for the ore finder (by part of the block name).
const ORE_RANK: &[&str] = &[
    "coal", "copper", "tin", "iron", "lead", "redstone", "lapis", "silver", "tungsten", "gold", "platinum", "emerald", "diamond", "meteor", "ancient_debris", "cobalt", "palladium", "mythril",
    "orichalcum", "adamantite", "titanium", "verdite",
];

/// A cheap repeatable "random" for particle timing per mob.
fn fastrand_like(id: u32, now: f32) -> f32 {
    let x = (id as f32 * 12.9898 + now * 78.233).sin() * 43758.547;
    x - x.floor()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessories_are_unique_and_described() {
        for (i, a) in ACCESSORIES.iter().enumerate() {
            assert!(ACCESSORIES[i + 1..].iter().all(|b| b.name != a.name), "{} twice", a.name);
            assert!(!a.desc.is_empty() || !a.mods.is_empty(), "{} does nothing", a.name);
            for m in a.mods {
                assert!(mod_line(m).is_some());
            }
        }
        assert!(ACCESSORIES.len() >= 70);
    }

    #[test]
    fn drops_are_accessories_or_known() {
        for d in DROPS {
            assert!(d.2 > 0.0 && d.2 <= 1.0, "{} {}", d.0, d.1);
        }
    }

    #[test]
    fn every_named_item_exists() {
        let (_, items, _) = crate::game_data(crate::embedded::BLOCKS_RON, crate::embedded::ITEMS_RON, crate::embedded::RECIPES_RON).unwrap();
        let mut names: Vec<&str> = ACCESSORIES.iter().map(|a| a.name).collect();
        names.extend(MINIONS.iter().flat_map(|m| [m.staff, m.sprite]));
        names.extend(RESTORE.iter().map(|r| r.0));
        names.extend(DROPS.iter().map(|d| d.1));
        names.extend(WHIPS.iter().map(|w| w.0));
        names.extend(["ancestral_heart", "celestial_socket", "life_fruit", "glass_bottle"]);
        for p in crate::effects::POTIONS {
            names.push(Box::leak(format!("potion_{}", p.key).into_boxed_str()));
        }
        for shop in crate::npcs::SHOPS.iter().chain(crate::hardmode::SHOPS.iter()) {
            names.extend(shop.1.iter().map(|g| g.0));
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/vanilla/textures/item");
        for n in names {
            assert!(items.id(n).is_some(), "no item {n}");
        }
        for a in ACCESSORIES {
            assert!(root.join(format!("{}.png", a.name)).exists(), "no picture for {}", a.name);
        }
        // Every brewing ingredient of the buff potions is a real item.
        for ing in ["iron_ingot", "lapis_lazuli", "amethyst", "feather", "ruby", "topaz", "armadillo_scute", "glow_berries", "cactus", "obsidian", "glow_ink_sac", "gold_nugget", "bone", "glowstone", "brick", "fire_charge", "magma_block"] {
            assert!(items.id(ing).is_some(), "no ingredient {ing}");
            assert!(crate::stations::brew("awkward", ing).is_some(), "{ing} brews nothing");
        }
    }

    #[test]
    fn stats_add_up() {
        let mut s = Stats::default();
        for m in accessory("champion_emblem").unwrap().mods {
            s.apply(m);
        }
        assert!((s.dmg[MAGIC] - 1.15).abs() < 1e-4);
        assert!((s.crit[RANGED] - 0.12).abs() < 1e-4);
        for m in accessory("ankh_charm").unwrap().mods {
            s.apply(m);
        }
        assert!(s.immune(POISON) && s.immune(BLEEDING) && !s.immune(SPEED));
    }
}
