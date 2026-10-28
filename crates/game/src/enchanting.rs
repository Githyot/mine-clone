//! Enchantments (Phase 6): the 43 enchantments, kept on each stack as (id, level) pairs
//! (`voxel::item::Ench`), what they do in play, how they show (tooltip lines and the purple
//! shimmer), and how worlds save them (`stack_key`, a suffix on the item's name).
//!
//! The stations that put them on and take them off (enchanting table, anvil, grindstone) are
//! in `stations.rs`.

use voxel::block::ToolKind;
use voxel::item::{Ench, ItemId, ItemRegistry, ItemStack};
use voxel::rng::Rng;

use crate::ui::UiBatch;

/// What an enchantment can go on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Armor,
    Head,
    Chest,
    Legs,
    Feet,
    /// Swords and axes.
    Melee,
    Sword,
    Digger,
    Bow,
    Crossbow,
    Trident,
    Rod,
    Mace,
    Spear,
    /// Anything that wears out.
    Durable,
    /// Anything worn.
    Wearable,
}

pub struct EnchDef {
    pub id: u8,
    pub key: &'static str,
    pub name: &'static str,
    pub max: u8,
    pub target: Target,
    /// How often the table picks it (vanilla weights: 10 common .. 1 very rare).
    pub weight: u32,
    /// Only from loot, trading and fishing, never the table.
    pub treasure: bool,
    pub curse: bool,
    /// Power needed for level 1, extra per level, and how far above that it still shows.
    pub min: i32,
    pub per: i32,
    pub span: i32,
    /// Enchantments in the same group (non-zero) can't be on one item together.
    pub group: u8,
}

macro_rules! e {
    ($id:expr, $key:expr, $name:expr, $max:expr, $t:ident, $w:expr, $tr:expr, $cu:expr, $min:expr, $per:expr, $span:expr, $g:expr) => {
        EnchDef { id: $id, key: $key, name: $name, max: $max, target: Target::$t, weight: $w, treasure: $tr, curse: $cu, min: $min, per: $per, span: $span, group: $g }
    };
}

pub const PROTECTION: u8 = 1;
pub const FIRE_PROTECTION: u8 = 2;
pub const FEATHER_FALLING: u8 = 3;
pub const BLAST_PROTECTION: u8 = 4;
pub const PROJECTILE_PROTECTION: u8 = 5;
pub const RESPIRATION: u8 = 6;
pub const AQUA_AFFINITY: u8 = 7;
pub const THORNS: u8 = 8;
pub const DEPTH_STRIDER: u8 = 9;
pub const FROST_WALKER: u8 = 10;
pub const SOUL_SPEED: u8 = 11;
pub const SWIFT_SNEAK: u8 = 12;
pub const BINDING: u8 = 13;
pub const SHARPNESS: u8 = 14;
pub const SMITE: u8 = 15;
pub const BANE: u8 = 16;
pub const KNOCKBACK: u8 = 17;
pub const FIRE_ASPECT: u8 = 18;
pub const LOOTING: u8 = 19;
pub const SWEEPING: u8 = 20;
pub const EFFICIENCY: u8 = 21;
pub const SILK_TOUCH: u8 = 22;
pub const FORTUNE: u8 = 23;
pub const UNBREAKING: u8 = 24;
pub const POWER: u8 = 25;
pub const PUNCH: u8 = 26;
pub const FLAME: u8 = 27;
pub const INFINITY: u8 = 28;
pub const LUCK_OF_THE_SEA: u8 = 29;
pub const LURE: u8 = 30;
pub const LOYALTY: u8 = 31;
pub const IMPALING: u8 = 32;
pub const RIPTIDE: u8 = 33;
pub const CHANNELING: u8 = 34;
pub const MULTISHOT: u8 = 35;
pub const QUICK_CHARGE: u8 = 36;
pub const PIERCING: u8 = 37;
pub const DENSITY: u8 = 38;
pub const BREACH: u8 = 39;
pub const WIND_BURST: u8 = 40;
pub const MENDING: u8 = 41;
pub const VANISHING: u8 = 42;
pub const LUNGE: u8 = 43;

/// Extras kept in the same pairs: an armor trim (pattern id, material id) and the anvil's
/// work count (each use makes the next one dearer).
pub const TRIM: u8 = 200;
pub const WORK: u8 = 250;

pub static ENCHANTS: &[EnchDef] = &[
    e!(PROTECTION, "protection", "Proteção", 4, Armor, 10, false, false, 1, 11, 11, 1),
    e!(FIRE_PROTECTION, "fire_protection", "Proteção contra Fogo", 4, Armor, 5, false, false, 10, 8, 8, 1),
    e!(FEATHER_FALLING, "feather_falling", "Peso-pena", 4, Feet, 5, false, false, 5, 6, 6, 0),
    e!(BLAST_PROTECTION, "blast_protection", "Proteção contra Explosões", 4, Armor, 2, false, false, 5, 8, 8, 1),
    e!(PROJECTILE_PROTECTION, "projectile_protection", "Proteção contra Projéteis", 4, Armor, 5, false, false, 3, 6, 6, 1),
    e!(RESPIRATION, "respiration", "Respiração", 3, Head, 2, false, false, 10, 10, 30, 0),
    e!(AQUA_AFFINITY, "aqua_affinity", "Afinidade Aquática", 1, Head, 2, false, false, 1, 0, 40, 0),
    e!(THORNS, "thorns", "Espinhos", 3, Chest, 1, false, false, 10, 20, 50, 0),
    e!(DEPTH_STRIDER, "depth_strider", "Passos Profundos", 3, Feet, 2, false, false, 10, 10, 15, 4),
    e!(FROST_WALKER, "frost_walker", "Passos Gelados", 2, Feet, 2, true, false, 10, 10, 15, 4),
    e!(SOUL_SPEED, "soul_speed", "Velocidade das Almas", 3, Feet, 1, true, false, 10, 10, 15, 0),
    e!(SWIFT_SNEAK, "swift_sneak", "Furtividade Ágil", 3, Legs, 1, true, false, 25, 25, 50, 0),
    e!(BINDING, "binding_curse", "Maldição do Vínculo", 1, Wearable, 1, true, true, 25, 0, 25, 0),
    e!(SHARPNESS, "sharpness", "Afiação", 5, Melee, 10, false, false, 1, 11, 20, 2),
    e!(SMITE, "smite", "Julgamento", 5, Melee, 5, false, false, 5, 8, 20, 2),
    e!(BANE, "bane_of_arthropods", "Ruína dos Artrópodes", 5, Melee, 5, false, false, 5, 8, 20, 2),
    e!(KNOCKBACK, "knockback", "Repulsão", 2, Sword, 5, false, false, 5, 20, 50, 0),
    e!(FIRE_ASPECT, "fire_aspect", "Aspecto Flamejante", 2, Sword, 2, false, false, 10, 20, 50, 0),
    e!(LOOTING, "looting", "Pilhagem", 3, Sword, 2, false, false, 15, 9, 50, 0),
    e!(SWEEPING, "sweeping_edge", "Lâmina Ampla", 3, Sword, 2, false, false, 5, 9, 15, 0),
    e!(EFFICIENCY, "efficiency", "Eficiência", 5, Digger, 10, false, false, 1, 10, 50, 0),
    e!(SILK_TOUCH, "silk_touch", "Toque Suave", 1, Digger, 1, false, false, 15, 0, 50, 3),
    e!(FORTUNE, "fortune", "Fortuna", 3, Digger, 2, false, false, 15, 9, 50, 3),
    e!(UNBREAKING, "unbreaking", "Inquebrável", 3, Durable, 5, false, false, 5, 8, 50, 0),
    e!(POWER, "power", "Força", 5, Bow, 10, false, false, 1, 10, 15, 0),
    e!(PUNCH, "punch", "Impacto", 2, Bow, 2, false, false, 12, 20, 25, 0),
    e!(FLAME, "flame", "Chama", 1, Bow, 2, false, false, 20, 0, 30, 0),
    e!(INFINITY, "infinity", "Infinidade", 1, Bow, 1, false, false, 20, 0, 30, 5),
    e!(LUCK_OF_THE_SEA, "luck_of_the_sea", "Sorte do Mar", 3, Rod, 2, false, false, 15, 9, 50, 0),
    e!(LURE, "lure", "Isca", 3, Rod, 2, false, false, 15, 9, 50, 0),
    e!(LOYALTY, "loyalty", "Lealdade", 3, Trident, 5, false, false, 12, 7, 38, 7),
    e!(IMPALING, "impaling", "Empalamento", 5, Trident, 2, false, false, 1, 8, 20, 0),
    e!(RIPTIDE, "riptide", "Correnteza", 3, Trident, 2, false, false, 17, 7, 33, 8),
    e!(CHANNELING, "channeling", "Condutividade", 1, Trident, 1, false, false, 25, 0, 25, 7),
    e!(MULTISHOT, "multishot", "Tiro Múltiplo", 1, Crossbow, 2, false, false, 20, 0, 30, 6),
    e!(QUICK_CHARGE, "quick_charge", "Recarga Rápida", 3, Crossbow, 5, false, false, 12, 20, 38, 0),
    e!(PIERCING, "piercing", "Perfuração", 4, Crossbow, 10, false, false, 1, 10, 49, 6),
    e!(DENSITY, "density", "Densidade", 5, Mace, 5, false, false, 5, 8, 20, 9),
    e!(BREACH, "breach", "Ruptura", 4, Mace, 2, false, false, 15, 9, 50, 9),
    e!(WIND_BURST, "wind_burst", "Explosão de Vento", 3, Mace, 2, true, false, 15, 9, 50, 0),
    e!(MENDING, "mending", "Remendo", 1, Durable, 2, true, false, 25, 0, 50, 5),
    e!(VANISHING, "vanishing_curse", "Maldição do Desaparecimento", 1, Durable, 1, true, true, 25, 0, 25, 0),
    e!(LUNGE, "lunge", "Investida", 3, Spear, 5, false, false, 5, 8, 20, 0),
];

pub fn def(id: u8) -> Option<&'static EnchDef> {
    ENCHANTS.iter().find(|d| d.id == id)
}

pub fn by_key(key: &str) -> Option<&'static EnchDef> {
    ENCHANTS.iter().find(|d| d.key == key)
}

/// Riptide shuts out loyalty and channeling; the rest of a group shuts out each other.
pub fn compatible(a: u8, b: u8) -> bool {
    if a == b {
        return true;
    }
    let (Some(x), Some(y)) = (def(a), def(b)) else { return true };
    let pair = |p: u8, q: u8| (x.id == p && y.id == q) || (x.id == q && y.id == p);
    if pair(RIPTIDE, LOYALTY) || pair(RIPTIDE, CHANNELING) {
        return false;
    }
    if (x.id == LOYALTY && y.id == CHANNELING) || (x.id == CHANNELING && y.id == LOYALTY) {
        return true;
    }
    x.group == 0 || x.group != y.group
}

pub fn roman(n: u8) -> &'static str {
    ["", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"].get(n as usize).copied().unwrap_or("X+")
}

/// Kinds of item, from its name and its tool or armor facts.
#[derive(Clone, Copy, Default, Debug)]
pub struct Kinds {
    pub armor: Option<u8>,
    pub sword: bool,
    pub axe: bool,
    pub digger: bool,
    pub bow: bool,
    pub crossbow: bool,
    pub trident: bool,
    pub rod: bool,
    pub mace: bool,
    pub spear: bool,
    pub durable: bool,
    pub book: bool,
    pub wearable: bool,
}

pub fn kinds(items: &ItemRegistry, id: ItemId) -> Kinds {
    let it = items.get(id);
    let n = it.name.as_str();
    let mut k = Kinds { armor: it.armor.map(|a| a.slot), ..Default::default() };
    if let Some(t) = &it.tool {
        k.durable = true;
        match t.kind {
            ToolKind::Sword => k.sword = true,
            ToolKind::Axe => {
                k.axe = true;
                k.digger = true;
            }
            _ => k.digger = true,
        }
    }
    k.bow = n == "bow";
    k.crossbow = n == "crossbow";
    k.trident = n == "trident";
    k.rod = n == "fishing_rod";
    k.mace = n == "mace";
    k.spear = n.ends_with("_spear");
    k.book = n == "book" || n == "enchanted_book";
    k.wearable = k.armor.is_some() || n == "elytra" || n == "carved_pumpkin";
    if k.armor.is_some() || k.bow || k.crossbow || k.trident || k.rod || k.mace || k.spear || matches!(n, "shears" | "flint_and_steel" | "elytra" | "shield" | "brush" | "carrot_on_a_stick") {
        k.durable = true;
    }
    if k.spear || k.mace || k.trident {
        // Their own wear lives in their tool facts when they have them; the table treats them
        // as weapons only for their own enchantments.
        k.sword = false;
    }
    k
}

pub fn fits(target: Target, k: &Kinds) -> bool {
    if k.book {
        return true;
    }
    match target {
        Target::Armor => k.armor.is_some(),
        Target::Head => k.armor == Some(0),
        Target::Chest => k.armor == Some(1),
        Target::Legs => k.armor == Some(2),
        Target::Feet => k.armor == Some(3),
        Target::Melee => k.sword || k.axe,
        Target::Sword => k.sword,
        Target::Digger => k.digger,
        Target::Bow => k.bow,
        Target::Crossbow => k.crossbow,
        Target::Trident => k.trident,
        Target::Rod => k.rod,
        Target::Mace => k.mace,
        Target::Spear => k.spear,
        Target::Durable => k.durable,
        Target::Wearable => k.wearable,
    }
}

/// How well a material takes enchantments (vanilla "enchantability").
pub fn enchantability(items: &ItemRegistry, id: ItemId) -> i32 {
    let n = items.get(id).name.as_str();
    let armor = items.get(id).armor.is_some();
    let mat = n.split('_').next().unwrap_or("");
    match (mat, armor) {
        ("leather", _) => 15,
        ("chainmail", _) => 12,
        ("iron", true) => 9,
        ("iron", false) => 14,
        ("golden", true) => 25,
        ("golden", false) => 22,
        ("diamond", _) => 10,
        ("netherite", _) => 15,
        ("turtle", _) => 9,
        ("wooden", _) => 15,
        ("stone", _) => 5,
        ("copper", true) => 8,
        ("copper", false) => 13,
        ("bronze", _) => 12,
        ("silver", _) => 18,
        ("platinum", _) => 16,
        ("tungsten", _) => 8,
        ("lead", _) => 6,
        _ => {
            if n == "book" || n == "bow" || n == "crossbow" || n == "trident" || n == "fishing_rod" || n == "mace" {
                if n == "book" { 1 } else if n == "mace" { 15 } else { 1 }
            } else {
                1
            }
        }
    }
}

/// The enchantments a table roll at `power` gives this item (vanilla's selection: one by
/// weight among those whose power range holds it, then maybe more, each time less likely).
pub fn roll(items: &ItemRegistry, id: ItemId, level: i32, rng: &mut Rng) -> Vec<(u8, u8)> {
    let k = kinds(items, id);
    let ench = enchantability(items, id).max(1);
    let mut p = level + 1 + rng.below((ench / 4 + 1) as u32) as i32 + rng.below((ench / 4 + 1) as u32) as i32;
    let bonus = 1.0 + (rng.f32() + rng.f32() - 1.0) * 0.15;
    p = ((p as f32 * bonus).round() as i32).max(1);
    let avail = |p: i32| -> Vec<(u8, u8, u32)> {
        let mut v = Vec::new();
        for d in ENCHANTS {
            if d.treasure || d.curse || !fits(d.target, &k) {
                continue;
            }
            for lvl in (1..=d.max).rev() {
                let lo = d.min + d.per * (lvl as i32 - 1);
                if p >= lo && p <= lo + d.span {
                    v.push((d.id, lvl, d.weight));
                    break;
                }
            }
        }
        v
    };
    let pick = |list: &[(u8, u8, u32)], rng: &mut Rng| -> Option<(u8, u8)> {
        let total: u32 = list.iter().map(|e| e.2).sum();
        if total == 0 {
            return None;
        }
        let mut r = rng.below(total);
        for e in list {
            if r < e.2 {
                return Some((e.0, e.1));
            }
            r -= e.2;
        }
        None
    };
    let mut out = Vec::new();
    let mut list = avail(p);
    if let Some(first) = pick(&list, rng) {
        out.push(first);
        loop {
            if rng.f32() >= (p + 1) as f32 / 50.0 {
                break;
            }
            list.retain(|e| out.iter().all(|o| compatible(o.0, e.0)));
            match pick(&list, rng) {
                Some(e) => out.push(e),
                None => break,
            }
            p /= 2;
        }
    }
    if k.book && out.len() > 1 && rng.f32() < 0.5 {
        // Books carry one fewer, like vanilla.
        out.pop();
    }
    out
}

/// A treasure roll for loot chests and fishing: any enchantment that fits, treasure ones too.
pub fn random_any(items: &ItemRegistry, id: ItemId, rng: &mut Rng) -> Ench {
    let k = kinds(items, id);
    let list: Vec<&EnchDef> = ENCHANTS.iter().filter(|d| !d.curse && fits(d.target, &k)).collect();
    let mut e = Ench::EMPTY;
    if list.is_empty() {
        return e;
    }
    let d = list[rng.below(list.len() as u32) as usize];
    e.set(d.id, 1 + rng.below(d.max as u32) as u8);
    e
}

// ---------------------------------------------------------------------------------------------
// Saving: "diamond_sword#sharpness=3,unbreaking=2" keeps old saves and other versions readable.

pub fn stack_key(items: &ItemRegistry, s: &ItemStack) -> String {
    let name = items.get(s.item).name.clone();
    if s.ench.is_empty() {
        return name;
    }
    let parts: Vec<String> = s
        .ench
        .pairs()
        .map(|(id, l)| match def(id) {
            Some(d) => format!("{}={l}", d.key),
            None => format!("{id}={l}"),
        })
        .collect();
    format!("{name}#{}", parts.join(","))
}

pub fn parse_key(items: &ItemRegistry, key: &str) -> Option<(ItemId, Ench)> {
    let (name, rest) = key.split_once('#').unwrap_or((key, ""));
    let id = items.id(name)?;
    let mut e = Ench::EMPTY;
    for part in rest.split(',').filter(|p| !p.is_empty()) {
        let Some((k, v)) = part.split_once('=') else { continue };
        let Ok(l) = v.parse::<u8>() else { continue };
        let eid = by_key(k).map(|d| d.id).or_else(|| k.parse::<u8>().ok());
        if let Some(eid) = eid {
            e.set(eid, l);
        }
    }
    Some((id, e))
}

/// True when the stack shows the enchanted shimmer.
pub fn shimmers(items: &ItemRegistry, s: &ItemStack) -> bool {
    s.ench.pairs().any(|(id, _)| id < TRIM) || matches!(items.get(s.item).name.as_str(), "enchanted_book" | "enchanted_golden_apple" | "nether_star" | "experience_bottle")
}

/// The shimmer over an enchanted item's icon: diagonal bands of violet light sliding across.
pub fn draw_glint(ui: &mut UiBatch, x: f32, y: f32, size: f32, t: f32) {
    let n = 6;
    let band = size / n as f32;
    let shift = (t * 0.6).fract();
    for i in 0..n {
        let k = ((i as f32 / n as f32 + shift).fract() * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        let a = (25.0 + 55.0 * k) as u8;
        let y0 = y + i as f32 * band;
        // Slanted: each band leans by its own height.
        ui.quad(
            [[x, y0 + band], [x + size, y0], [x + size, y0 + band], [x, (y0 + 2.0 * band).min(y + size)]],
            [[0.0; 2]; 4],
            [150, 80, 255, a],
            -2,
        );
    }
}

/// Tooltip lines for a stack's enchantments (gray, curses red) and its trim.
pub fn lines(s: &ItemStack) -> Vec<(String, [u8; 4])> {
    let mut out = Vec::new();
    for (id, l) in s.ench.pairs() {
        if id == WORK {
            continue;
        }
        if id == TRIM {
            if let Some(t) = crate::smithing::trim_line(l) {
                out.push((t, [200, 170, 255, 255]));
            }
            continue;
        }
        let Some(d) = def(id) else { continue };
        let text = if d.max == 1 { d.name.to_string() } else { format!("{} {}", d.name, roman(l)) };
        out.push((text, if d.curse { [255, 85, 85, 255] } else { [170, 170, 170, 255] }));
    }
    out
}

/// What the anvil adds to the price for each enchantment level brought in (vanilla multipliers,
/// halved for books).
pub fn anvil_cost(id: u8, book: bool) -> u32 {
    let Some(d) = def(id) else { return 1 };
    let m = match d.weight {
        10 => 1,
        5 => 2,
        2 => 4,
        _ => 8,
    };
    if book {
        (m / 2).max(1)
    } else {
        m
    }
}

/// Enchantment levels on a stack, for the grindstone's experience and the anvil's price.
pub fn total_power(s: &ItemStack) -> u32 {
    s.ench.pairs().filter(|(id, _)| *id < TRIM).filter_map(|(id, l)| def(id).map(|d| (d.min + d.per * (l as i32 - 1)).max(1) as u32)).sum()
}

/// Level of an enchantment on a stack.
pub fn level(s: &Option<ItemStack>, id: u8) -> u8 {
    s.map(|s| s.ench.get(id)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forty_three_enchantments_with_unique_ids_and_keys() {
        assert_eq!(ENCHANTS.len(), 43);
        for (i, a) in ENCHANTS.iter().enumerate() {
            for b in &ENCHANTS[i + 1..] {
                assert_ne!(a.id, b.id);
                assert_ne!(a.key, b.key);
            }
            assert!(a.id < TRIM);
        }
    }

    #[test]
    fn groups_shut_each_other_out() {
        assert!(!compatible(SHARPNESS, SMITE));
        assert!(!compatible(SILK_TOUCH, FORTUNE));
        assert!(!compatible(RIPTIDE, LOYALTY));
        assert!(compatible(LOYALTY, CHANNELING));
        assert!(compatible(SHARPNESS, LOOTING));
        assert!(!compatible(PROTECTION, FIRE_PROTECTION));
    }

    #[test]
    fn ench_pairs_set_and_clear() {
        let mut e = Ench::EMPTY;
        assert!(e.set(SHARPNESS, 3));
        assert!(e.set(UNBREAKING, 2));
        assert_eq!(e.get(SHARPNESS), 3);
        e.set(SHARPNESS, 0);
        assert_eq!(e.get(SHARPNESS), 0);
        assert_eq!(e.pairs().count(), 1);
    }
}

/// Whether a stack carries a real enchantment (not just anvil wear or an armour trim).
pub fn is_enchanted(s: &ItemStack) -> bool {
    s.ench.pairs().any(|(id, _)| id != WORK && id != TRIM && def(id).is_some())
}
