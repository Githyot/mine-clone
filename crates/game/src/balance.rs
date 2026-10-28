//! The balance sheet of weapons and armour (plan/balanceamento.md): every weapon is placed in a
//! stage of the progression and a class, and the tests below check that at each stage the best
//! weapon of every class, with the best armour set that class can wear by then, lands close to
//! the stage's target damage per second. Nothing here runs in the game: it reads the real
//! tables (items, tr_weapons, magic, rpg, sentries, weapons) so a change to any number there
//! shows up in the sheet.
//!
//! The model, in a few lines:
//! - damage per second on one monster, held down for a 15 s fight;
//! - a 3D factor per kind of weapon: in a world with height, homing and area spells and
//!   flying minions find their mark more often than a sword (the monster can be above you) or
//!   an arrow (it drops and has to be drawn);
//! - casters run on their mana pool and regeneration for the stage, and on mana potions at the
//!   price of mana sickness (a quarter less magic damage) after that;
//! - minions count once per slot a summoner has by then (`SLOTS`), with the whip's mark;
//! - the armour set multiplies damage, crit (a crit is half again) and melee speed.

use crate::mods::combat::weapon_of;
use crate::rpg::{M, MAGIC, MELEE, MINIONS, RANGED, SETS, SUMMON};

pub const STAGES: [&str; 6] = ["Pré-chefe", "Início", "Pré-hardmode", "Hardmode", "Pós-mecânicos", "Fim de jogo"];

/// Target damage per second of each stage (about 1.5 times the stage before). The first one is
/// the iron sword's, which keeps the classic numbers (parity_test.rs).
pub const TARGET: [f32; 6] = [9.0, 13.0, 19.0, 29.0, 45.0, 70.0];

/// How close the best weapon of each class must come to the target (0.85 = 15% under).
pub const BAND: (f32, f32) = (0.85, 1.15);

/// No weapon may pass the target of its own stage by more than this.
pub const CEILING: f32 = 1.2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// Swords, axes, spears, yoyos, boomerangs, flails (melee stats).
    Warrior,
    /// Bows and the crossbow (ranged stats).
    Archer,
    /// Guns (ranged stats).
    Gunner,
    /// Knives, shurikens, javelins, grenades (ranged stats).
    Thrower,
    Mage,
    /// Minions (with the whip of the stage) and sentries.
    Summoner,
}

pub const CLASSES: [Class; 6] = [Class::Warrior, Class::Archer, Class::Gunner, Class::Thrower, Class::Mage, Class::Summoner];

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Warrior => "Guerreiro",
            Class::Archer => "Arqueiro",
            Class::Gunner => "Atirador",
            Class::Thrower => "Arremessador",
            Class::Mage => "Mago",
            Class::Summoner => "Invocador",
        }
    }

    /// The stat class of rpg.rs whose damage and crit it uses.
    fn stat(self) -> usize {
        match self {
            Class::Warrior => MELEE,
            Class::Archer | Class::Gunner | Class::Thrower => RANGED,
            Class::Mage => MAGIC,
            Class::Summoner => SUMMON,
        }
    }
}

use Class::*;

/// Every weapon: its class and the stage it can first be had in (by the cheapest way: a
/// recipe, a shop, a chest or a boss).
pub const GEAR: &[(&str, Class, u8)] = &[
    // Pré-chefe: wood to gold and platinum, gems, the first shops.
    ("wooden_sword", Warrior, 0),
    ("stone_sword", Warrior, 0),
    ("copper_sword", Warrior, 0),
    ("bronze_sword", Warrior, 0),
    ("iron_sword", Warrior, 0),
    ("lead_sword", Warrior, 0),
    ("silver_sword", Warrior, 0),
    ("tungsten_sword", Warrior, 0),
    ("golden_sword", Warrior, 0),
    ("platinum_sword", Warrior, 0),
    ("stone_greatsword", Warrior, 0),
    ("iron_greatsword", Warrior, 0),
    ("golden_greatsword", Warrior, 0),
    ("stone_halberd", Warrior, 0),
    ("iron_halberd", Warrior, 0),
    ("golden_halberd", Warrior, 0),
    ("wooden_spear", Warrior, 0),
    ("stone_spear", Warrior, 0),
    ("copper_spear", Warrior, 0),
    ("iron_spear", Warrior, 0),
    ("golden_spear", Warrior, 0),
    ("skyroot_sword", Warrior, 0),
    ("holystone_sword", Warrior, 0),
    ("wood_yoyo", Warrior, 0),
    ("wood_boomerang", Warrior, 0),
    ("iron_flail", Warrior, 0),
    ("enchanted_boomerang", Warrior, 0),
    ("enchanted_sword", Warrior, 0),
    ("sky_blade", Warrior, 0),
    ("star_saber", Warrior, 0),
    ("bow", Archer, 0),
    ("crossbow", Archer, 0),
    ("copper_bow", Archer, 0),
    ("silver_bow", Archer, 0),
    ("platinum_bow", Archer, 0),
    ("flintlock_pistol", Gunner, 0),
    ("shuriken", Thrower, 0),
    ("throwing_knife", Thrower, 0),
    ("javelin", Thrower, 0),
    ("bone_javelin", Thrower, 0),
    ("grenade", Thrower, 0),
    ("blowpipe", Gunner, 0),
    ("topaz_wand", Mage, 0),
    ("ruby_wand", Mage, 0),
    ("sapphire_wand", Mage, 0),
    ("amethyst_wand", Mage, 0),
    ("emerald_wand", Mage, 0),
    ("fire_staff", Mage, 0),
    ("frost_staff", Mage, 0),
    ("thunder_tome", Mage, 0),
    ("flame_tome", Mage, 0),
    ("chain_tome", Mage, 0),
    ("wisp_staff", Summoner, 0),
    ("jelly_staff", Summoner, 0),
    ("leather_whip", Summoner, 0),
    ("storm_totem_staff", Summoner, 0),
    ("frost_crystal_staff", Summoner, 0),
    ("ballista_rod", Summoner, 0),
    // Início: the first bosses, diamond, the meteor, the evil biomes.
    ("diamond_sword", Warrior, 1),
    ("diamond_greatsword", Warrior, 1),
    ("diamond_halberd", Warrior, 1),
    ("diamond_spear", Warrior, 1),
    ("zanite_sword", Warrior, 1),
    ("umbral_sword", Warrior, 1),
    ("sanguinite_sword", Warrior, 1),
    ("umbral_yoyo", Warrior, 1),
    ("sanguinite_yoyo", Warrior, 1),
    ("umbral_flail", Warrior, 1),
    ("sanguinite_flail", Warrior, 1),
    ("jungle_yoyo", Warrior, 1),
    ("thorn_chakram", Warrior, 1),
    ("ice_boomerang", Warrior, 1),
    ("umbral_bow", Archer, 1),
    ("sanguinite_bow", Archer, 1),
    ("musket", Gunner, 1),
    ("star_cannon", Gunner, 1),
    ("minishark", Gunner, 1),
    ("boomstick", Gunner, 1),
    ("guardian_staff", Mage, 1),
    ("meteor_blaster", Mage, 1),
    ("ruin_thorn", Mage, 1),
    ("blight_rod", Mage, 1),
    ("poison_knife", Thrower, 1),
    ("worm_staff", Mage, 1),
    ("wasp_staff", Summoner, 1),
    ("thorn_whip", Summoner, 1),
    // Pré-hardmode: the hive, the dungeon, the Nether, the Underworld's metal.
    ("magmarite_sword", Warrior, 2),
    ("netherite_sword", Warrior, 2),
    ("netherite_spear", Warrior, 2),
    ("gravitite_sword", Warrior, 2),
    ("valkyrie_lance", Warrior, 2),
    ("resonarium_sword", Warrior, 2),
    ("hive_blade", Warrior, 2),
    ("blue_katana", Warrior, 2),
    ("night_edge", Warrior, 2),
    ("mace", Warrior, 2),
    ("trident", Warrior, 2),
    ("bone_flail", Warrior, 2),
    ("sun_flail", Warrior, 2),
    ("magma_yoyo", Warrior, 2),
    ("flame_boomerang", Warrior, 2),
    ("rapid_gun", Gunner, 2),
    ("magmarite_bow", Archer, 2),
    ("magmarite_knife", Thrower, 2),
    ("water_bolt", Mage, 2),
    ("demon_scythe", Mage, 2),
    ("hive_staff", Mage, 2),
    ("colossus_staff", Mage, 2),
    ("void_tome", Mage, 2),
    ("imp_staff", Summoner, 2),
    ("bone_whip", Summoner, 2),
    ("ember_tower_staff", Summoner, 2),
    // Hardmode: the six new metals, the first invasions, the queen of the jellies.
    ("cobalt_sword", Warrior, 3),
    ("palladium_sword", Warrior, 3),
    ("mythril_sword", Warrior, 3),
    ("orichalcum_sword", Warrior, 3),
    ("adamantite_sword", Warrior, 3),
    ("titanium_sword", Warrior, 3),
    ("eclipse_scythe", Warrior, 3),
    ("crystal_yoyo", Warrior, 3),
    ("cutlass", Warrior, 3),
    ("cobalt_bow", Archer, 3),
    ("mythril_bow", Archer, 3),
    ("adamantite_bow", Archer, 3),
    ("adamantite_rifle", Gunner, 3),
    ("mythril_javelin", Thrower, 3),
    ("ice_spike_staff", Mage, 3),
    ("blizzard_staff", Mage, 3),
    ("explosive_trap_rod", Summoner, 3),
    ("lightning_aura_rod", Summoner, 3),
    ("flame_sentry_rod", Summoner, 3),
    ("crystal_jelly_staff", Summoner, 3),
    ("megashark", Gunner, 3),
    ("flamethrower", Gunner, 3),
    ("ogre_club", Warrior, 3),
    // Pós-mecânicos: radiant and verdite, the vine queen, the idol, the tide duke, the empress.
    ("radiant_sword", Warrior, 4),
    ("verdite_sword", Warrior, 4),
    ("radiant_blade", Warrior, 4),
    ("true_night_edge", Warrior, 4),
    ("true_radiant_blade", Warrior, 4),
    ("terra_blade", Warrior, 4),
    ("vine_blade", Warrior, 4),
    ("horseman_blade", Warrior, 4),
    ("stone_fist", Warrior, 4),
    ("tide_boomerang", Warrior, 4),
    ("prism_saber", Warrior, 4),
    ("bubble_rifle", Gunner, 4),
    ("laser_rifle", Gunner, 4),
    ("radiant_bow", Archer, 4),
    ("verdite_bow", Archer, 4),
    ("verdite_knife", Thrower, 4),
    ("meteor_staff", Mage, 4),
    ("spirit_staff", Mage, 4),
    ("sun_stone_staff", Mage, 4),
    ("blade_staff", Summoner, 4),
    ("ballista_cane", Summoner, 4),
    ("sniper_rifle", Gunner, 4),
    ("coin_gun", Gunner, 4),
    ("elf_gun", Gunner, 4),
    ("rocket_launcher", Gunner, 4),
    ("grenade_launcher", Gunner, 4),
    ("paladin_hammer", Warrior, 4),
    ("betsy_wrath", Warrior, 4),
    ("razorpine", Warrior, 4),
    ("spooky_staff", Summoner, 4),
    // Fim de jogo: the pillars, lumite and the Celestial Herald.
    ("lumite_sword", Warrior, 5),
    ("celestial_blade", Warrior, 5),
    ("celestial_wrath", Warrior, 5),
    ("apex_blade", Warrior, 5),
    ("solar_eruption", Warrior, 5),
    ("nebula_blade", Warrior, 5),
    ("stardust_yoyo", Warrior, 5),
    ("vortex_beater", Gunner, 5),
    ("lumite_bow", Archer, 5),
    ("sun_javelin", Thrower, 5),
    ("star_tome", Mage, 5),
    ("star_dragon_staff", Summoner, 5),
];

/// Weapons of the tables that are tools first (they blow up the ground) and don't count.
const NOT_WEAPONS: &[&str] = &["bomb", "dynamite"];

/// Armour sets: the stage each can first be worn in.
pub const SET_STAGE: &[(&str, u8)] = &[
    ("copper", 0),
    ("bronze", 0),
    ("lead", 0),
    ("silver", 0),
    ("tungsten", 0),
    ("platinum", 0),
    ("meteor", 1),
    ("umbral", 1),
    ("sanguinite", 1),
    ("jungle", 1),
    ("fossil", 1),
    ("ninja", 1),
    ("gladiator", 1),
    ("hive", 2),
    ("necro", 2),
    ("magmarite", 2),
    ("cobalt", 3),
    ("palladium", 3),
    ("mythril", 3),
    ("orichalcum", 3),
    ("adamantite", 3),
    ("titanium", 3),
    ("radiant", 4),
    ("verdite", 4),
    ("spectral", 4),
    ("sporite", 4),
    ("beetle", 4),
    ("turtle", 4),
    ("solar", 5),
    ("vortex", 5),
    ("nebula", 5),
    ("stardust", 5),
    ("lumite", 5),
];

/// Armour without a set bonus, for the defense a stage offers.
pub const PLAIN_ARMOUR: &[(&str, u8)] = &[("iron", 0), ("chainmail", 0), ("golden", 0), ("diamond", 1), ("zanite", 1), ("netherite", 2), ("gravitite", 2), ("resonarium", 2)];

/// The share of a stage's best defense the set each class wears for damage must keep.
pub const ARMOUR_FLOOR: f32 = 0.55;

/// Defense of the four pieces of a set and its own bonus.
pub fn set_defense(items: &voxel::item::ItemRegistry, prefix: &str) -> f32 {
    let pieces: f32 = ["helmet", "chestplate", "leggings", "boots"]
        .iter()
        .filter_map(|k| items.id(&format!("{prefix}_{k}")).and_then(|id| items.get(id).armor))
        .map(|a| a.points as f32)
        .sum();
    let bonus: f32 = SETS.iter().filter(|s| s.prefix == prefix).flat_map(|s| s.mods.iter()).map(|m| if let M::Def(d) = m { *d } else { 0.0 }).sum();
    pieces + bonus
}

/// The most defense one can wear by `stage`.
pub fn best_defense(items: &voxel::item::ItemRegistry, stage: u8) -> (f32, &'static str) {
    SET_STAGE.iter().chain(PLAIN_ARMOUR).filter(|s| s.1 <= stage).map(|s| (set_defense(items, s.0), s.0)).fold((0.0, ""), |a, b| if b.0 > a.0 { b } else { a })
}

/// Minion slots a summoner has by each stage: one, the summoning potion, the totem necklace
/// (Forest Guardian), the hive set and then the orichalcum one, and the stardust set at the end.
pub const SLOTS: [f32; 6] = [2.0, 3.0, 4.0, 4.0, 4.0, 5.0];

/// Mana a caster can spend each second in a 15 s fight without potions: the pool (mana crystals
/// and accessories) over 15 s, plus regeneration while casting.
pub const MANA_BUDGET: [f32; 6] = [2.0, 2.5, 3.8, 4.5, 5.3, 6.3];

/// What a caster's damage keeps under mana sickness.
const SICK: f32 = 0.75;

/// The 3D factor of a weapon (see the module comment).
fn factor_3d(name: &str, class: Class) -> f32 {
    match class {
        Warrior if crate::tr_weapons::sustained_dps(name).is_some() => {
            if name.ends_with("_yoyo") {
                1.0
            } else {
                0.95
            }
        }
        Warrior => 0.9,
        Archer => 0.9,
        Gunner => 1.0,
        Thrower => 0.85,
        Mage => {
            if matches!(name, "amethyst_wand" | "hive_staff" | "star_tome" | "spirit_staff" | "chain_tome" | "thunder_tome" | "blizzard_staff") {
                1.1
            } else if matches!(name, "fire_staff" | "meteor_staff" | "sun_stone_staff" | "blight_rod" | "void_tome" | "worm_staff") {
                1.05
            } else {
                1.0
            }
        }
        Summoner => 1.0,
    }
}

/// Damage per second of a weapon by itself, before the armour set and the 3D factor.
pub fn raw_dps(items: &voxel::item::ItemRegistry, name: &str, class: Class, stage: u8) -> Option<f32> {
    match class {
        Warrior => {
            let tool = items.id(name).and_then(|id| items.get(id).tool.clone());
            let swing = tool.as_ref().and_then(|t| weapon_of(name, Some(t.kind)).map(|w| w.swing_dps(t.damage))).unwrap_or(0.0);
            let flying = crate::tr_weapons::sustained_dps(name).map_or(0.0, |d| d.1);
            // A yoyo, boomerang or flail is thrown, not swung (its item has no blade).
            let swing = if tool.is_some_and(|t| t.damage > 1.0) { swing } else { 0.0 };
            Some(swing + flying).filter(|d| *d > 0.0)
        }
        Archer => crate::weapons::bow_dps(name),
        Gunner | Thrower => crate::tr_weapons::sustained_dps(name).map(|d| d.1),
        Mage => {
            let (dps, mana) = crate::magic::sustained_dps(name)?;
            let full = (MANA_BUDGET[stage as usize] / mana.max(1e-3)).min(1.0);
            Some(dps * (full + (1.0 - full) * SICK))
        }
        Summoner => {
            if let Some(m) = MINIONS.iter().find(|m| m.staff == name) {
                return Some(m.damage * (1.0 + best_whip(stage)) / m.cooldown * SLOTS[stage as usize]);
            }
            if crate::rpg::whip_bonus(name).is_some() {
                // A whip by itself: its swing (its mark counts with the minions).
                let tool = items.id(name).and_then(|id| items.get(id).tool.clone())?;
                return weapon_of(name, Some(tool.kind)).map(|w| w.swing_dps(tool.damage));
            }
            crate::sentries::sentry_dps(name).or_else(|| crate::tr_weapons::sustained_dps(name).map(|d| d.1))
        }
    }
}

/// The best whip mark a summoner can have by `stage`.
fn best_whip(stage: u8) -> f32 {
    GEAR.iter().filter(|g| g.1 == Summoner && g.2 <= stage).filter_map(|g| crate::rpg::whip_bonus(g.0)).fold(0.0, f32::max)
}

/// How much the best armour set a class can wear by `stage` multiplies its damage per second.
pub fn set_factor(class: Class, stage: u8) -> (f32, &'static str) {
    let k = class.stat();
    let base_crit = crate::rpg::Stats::default().crit[k];
    let mut best = (1.0 + 0.5 * base_crit, "");
    for (prefix, s) in SET_STAGE {
        if *s > stage {
            continue;
        }
        let Some(set) = SETS.iter().find(|x| x.prefix == *prefix) else { continue };
        // (A set's minion slots are already in `SLOTS`.)
        let (mut dmg, mut crit, mut speed) = (1.0, base_crit, 1.0);
        for m in set.mods {
            match *m {
                M::Dmg(c, v) if c == k || c == 4 => dmg += v,
                M::Crit(c, v) if (c == k || c == 4) && class != Summoner => crit += v,
                M::MeleeSpeed(v) if class == Warrior => speed += v,
                _ => {}
            }
        }
        let f = dmg * (1.0 + 0.5 * crit) * speed;
        if f > best.0 {
            best = (f, prefix);
        }
    }
    best
}

/// A weapon's power at its own stage: what the sheet compares.
pub fn power(items: &voxel::item::ItemRegistry, name: &str, class: Class, stage: u8) -> Option<f32> {
    let raw = raw_dps(items, name, class, stage)?;
    Some(raw * factor_3d(name, class) * set_factor(class, stage).0)
}

/// The strongest weapon of a class one can have by `stage` and its power then.
/// A whip's own swing and a sentry don't count: they help the minions.
pub fn best(items: &voxel::item::ItemRegistry, class: Class, stage: u8) -> Option<(&'static str, f32)> {
    let helper = |n: &str| class == Summoner && !MINIONS.iter().any(|m| m.staff == n);
    GEAR.iter()
        .filter(|g| g.1 == class && g.2 <= stage && !helper(g.0))
        .filter_map(|g| power(items, g.0, class, stage).map(|p| (g.0, p)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> voxel::item::ItemRegistry {
        crate::game_data(crate::embedded::BLOCKS_RON, crate::embedded::ITEMS_RON, crate::embedded::RECIPES_RON).unwrap().1
    }

    /// Prints the whole sheet (cargo test -p game balance -- --nocapture).
    #[test]
    fn print_sheet() {
        let items = items();
        for (s, name) in STAGES.iter().enumerate() {
            let (top, top_set) = best_defense(&items, s as u8);
            println!("== {name} (meta {:.0}, defesa máxima {top:.0} {top_set})", TARGET[s]);
            for c in CLASSES {
                let (set_k, set) = set_factor(c, s as u8);
                let b = best(&items, c, s as u8);
                println!("  {:<13} {:>6.1} {:<22} conjunto {set} x{set_k:.2} defesa {:.0}", c.name(), b.map_or(0.0, |b| b.1), b.map_or("-", |b| b.0), set_defense(&items, set));
            }
            for g in GEAR.iter().filter(|g| g.2 == s as u8) {
                let raw = raw_dps(&items, g.0, g.1, g.2).unwrap_or(-1.0);
                let p = power(&items, g.0, g.1, g.2).unwrap_or(-1.0);
                println!("    {:<22} {:<12} cru {:>6.1}  efetivo {:>6.1}  ({:>3.0}%)", g.0, g.1.name(), raw, p, 100.0 * p / TARGET[s]);
            }
        }
    }

    #[test]
    fn every_weapon_has_a_stage_and_numbers() {
        let items = items();
        for g in GEAR {
            assert!(items.id(g.0).is_some(), "no item {}", g.0);
            assert!(raw_dps(&items, g.0, g.1, g.2).is_some_and(|d| d > 0.0), "{} has no damage", g.0);
            assert_eq!(GEAR.iter().filter(|h| h.0 == g.0).count(), 1, "{} twice", g.0);
        }
        let listed = |n: &str| GEAR.iter().any(|g| g.0 == n) || NOT_WEAPONS.contains(&n);
        for n in crate::tr_weapons::all_weapons().chain(crate::magic::all_casters()) {
            if crate::magic::sustained_dps(n).is_none() && crate::tr_weapons::sustained_dps(n).is_none() {
                continue;
            }
            assert!(listed(n), "{n} is not in the balance sheet (balance.rs GEAR)");
        }
        for m in MINIONS {
            assert!(listed(m.staff), "{} is not in the balance sheet", m.staff);
        }
        for s in crate::sentries::SENTRIES {
            assert!(listed(s.staff), "{} is not in the balance sheet", s.staff);
        }
        // Every blade of the item files, except the modular tools (they copy the plain ones).
        for it in &items.items {
            let blade = it.tool.as_ref().is_some_and(|t| t.kind == voxel::block::ToolKind::Sword && t.damage > 1.0);
            if blade && !it.name.starts_with("gear_") && crate::rpg::whip_bonus(&it.name).is_none() {
                assert!(listed(&it.name), "{} is not in the balance sheet", it.name);
            }
        }
        for (p, _) in SET_STAGE {
            assert!(SETS.iter().any(|s| s.prefix == *p), "no set {p}");
        }
        for s in SETS {
            assert!(SET_STAGE.iter().any(|x| x.0 == s.prefix), "set {} has no stage", s.prefix);
        }
    }

    /// The point of the sheet: at every stage every class has something close to the target.
    #[test]
    fn classes_are_close_at_every_stage() {
        let items = items();
        let mut bad = Vec::new();
        for s in 0..STAGES.len() {
            for c in CLASSES {
                let Some((name, p)) = best(&items, c, s as u8) else {
                    bad.push(format!("{}: {} sem arma", STAGES[s], c.name()));
                    continue;
                };
                let r = p / TARGET[s];
                if r < BAND.0 || r > BAND.1 {
                    bad.push(format!("{}: {} {name} {p:.1} ({:.0}% da meta {:.0})", STAGES[s], c.name(), 100.0 * r, TARGET[s]));
                }
            }
        }
        assert!(bad.is_empty(), "fora da faixa:\n{}", bad.join("\n"));
    }

    /// No single weapon runs away from its own stage.
    #[test]
    fn no_weapon_beats_its_stage() {
        let items = items();
        let mut bad = Vec::new();
        for g in GEAR {
            let p = power(&items, g.0, g.1, g.2).unwrap();
            if p > TARGET[g.2 as usize] * CEILING {
                bad.push(format!("{} ({}, {}): {p:.1} > {:.1}", g.0, g.1.name(), STAGES[g.2 as usize], TARGET[g.2 as usize] * CEILING));
            }
        }
        assert!(bad.is_empty(), "fortes demais:\n{}", bad.join("\n"));
    }

    /// Every class's damage set still protects: no class has to go about in paper.
    #[test]
    fn armour_keeps_up() {
        let items = items();
        let mut last = 0.0;
        for s in 0..STAGES.len() {
            let (top, _) = best_defense(&items, s as u8);
            assert!(top >= last, "a defesa cai em {}", STAGES[s]);
            last = top;
            for c in CLASSES {
                let (_, set) = set_factor(c, s as u8);
                if set.is_empty() {
                    continue;
                }
                let d = set_defense(&items, set);
                assert!(d >= ARMOUR_FLOOR * top, "{} em {}: conjunto {set} tem {d} de {top}", c.name(), STAGES[s]);
            }
        }
    }

    /// The best weapon of every class keeps getting better from stage to stage.
    #[test]
    fn every_class_grows() {
        let items = items();
        for c in CLASSES {
            let mut last = 0.0;
            for s in 0..STAGES.len() {
                let p = best(&items, c, s as u8).map_or(0.0, |b| b.1);
                assert!(p > last, "{} não cresce em {}", c.name(), STAGES[s]);
                last = p;
            }
        }
    }
}
