//! Gems and sockets (ideas from the Apotheosis mod, our own code and numbers). Gear found on
//! elite monsters and in chests comes with a loot rarity (Comum .. Mítica) that gives it
//! sockets; gems set in them add a bonus while the piece is held or worn. Salvaging breaks a
//! ranked piece into the material of its rarity, and world tiers make better ranks likelier.
//!
//! This is only gems, sockets and loot rank: the random bonuses of gear (affixes) and their
//! reforging belong to `affixes.rs` of the game, which keeps them in the stack's wear bits.
//! Everything here lives in one `u32` on the stack (`ItemStack::sockets`):
//!
//! ```text
//! bits  0..3   loot rarity: 0 none, 1 Comum .. 5 Mítica
//! bits  3..5   extra sockets opened with a sigil (0..2)
//! bits  8..20  three sockets of 4 bits: 0 empty, else 1 + gem kind * 3 + cut
//! ```

use crate::rng::Rng;

pub const RARITIES: usize = 5;
pub const RARITY_NAMES: [&str; RARITIES] = ["Comum", "Incomum", "Rara", "Épica", "Mítica"];
pub const RARITY_COLORS: [[u8; 4]; RARITIES] = [[215, 215, 215, 255], [95, 225, 95, 255], [85, 160, 255, 255], [195, 100, 245, 255], [255, 160, 40, 255]];
/// Item names of the salvage material of each rarity.
pub const MATERIALS: [&str; RARITIES] = ["salvage_common", "salvage_uncommon", "salvage_rare", "salvage_epic", "salvage_mythic"];
/// Sockets a piece of each rarity comes with.
const BASE_SOCKETS: [u32; RARITIES] = [0, 1, 1, 2, 3];
pub const MAX_SOCKETS: usize = 3;

/// A bonus a gem gives. Percentages are fractions (0.1 = 10%).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Health,
    Armor,
    Damage,
    AttackSpeed,
    CritChance,
}

pub const STATS: usize = 5;

impl Stat {
    /// Words shown in the tooltip.
    pub fn line(self, v: f32) -> String {
        let p = (v * 100.0).round() as i32;
        match self {
            Stat::Health => format!("+{} Vida", v as i32),
            Stat::Armor => format!("+{} Armadura", v as i32),
            Stat::Damage => format!("+{} Dano", v as i32),
            Stat::AttackSpeed => format!("+{p}% Velocidade de ataque"),
            Stat::CritChance => format!("+{p}% Chance de crítico"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The number on the stack.

/// Loot rarity index (0 Comum .. 4 Mítica), or None for plain gear.
pub fn rarity(s: u32) -> Option<usize> {
    let r = (s & 7) as usize;
    (1..=RARITIES).contains(&r).then(|| r - 1)
}

/// The number of a freshly found piece of a rarity: its sockets are empty.
pub fn ranked(rarity: usize) -> u32 {
    rarity.min(RARITIES - 1) as u32 + 1
}

pub fn extra_sockets(s: u32) -> u32 {
    (s >> 3) & 3
}

/// One more socket (Sigil of Socketing), at most two extra and three in all. Plain gear
/// opened this way counts as Comum.
pub fn with_extra_socket(s: u32) -> Option<u32> {
    if sockets(s) >= MAX_SOCKETS || extra_sockets(s) >= 2 {
        return None;
    }
    let s = if rarity(s).is_none() { s | 1 } else { s };
    Some((s & !(3 << 3)) | ((extra_sockets(s) + 1) << 3))
}

/// How many gems the piece can hold.
pub fn sockets(s: u32) -> usize {
    match rarity(s) {
        Some(r) => ((BASE_SOCKETS[r] + extra_sockets(s)) as usize).min(MAX_SOCKETS),
        None => 0,
    }
}

/// A gem: one of five kinds, in three cuts (rough, cut, flawless).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gem {
    pub kind: u8,
    pub cut: u8,
}

pub const GEM_KINDS: [&str; 5] = ["vital", "bastion", "fury", "swift", "shadow"];
const GEM_NAMES: [&str; 5] = ["Vital", "Bastião", "da Fúria", "Ligeira", "Sombria"];
const GEM_CUTS: [&str; 3] = ["rough", "cut", "flawless"];
const GEM_CUT_NAMES: [&str; 3] = ["Bruta", "Lapidada", "Perfeita"];
const GEM_STATS: [(Stat, [f32; 3]); 5] = [
    (Stat::Health, [2.0, 4.0, 6.0]),
    (Stat::Armor, [1.0, 2.0, 3.0]),
    (Stat::Damage, [1.0, 2.0, 3.0]),
    (Stat::AttackSpeed, [0.08, 0.15, 0.25]),
    (Stat::CritChance, [0.05, 0.10, 0.15]),
];
pub const GEM_COLORS: [[u8; 4]; 5] = [[235, 70, 80, 255], [90, 150, 245, 255], [245, 140, 40, 255], [110, 235, 160, 255], [175, 110, 230, 255]];

impl Gem {
    pub fn item_name(self) -> String {
        format!("gem_{}_{}", GEM_KINDS[self.kind as usize], GEM_CUTS[self.cut as usize])
    }

    pub fn from_name(name: &str) -> Option<Gem> {
        let rest = name.strip_prefix("gem_")?;
        let (kind, cut) = rest.rsplit_once('_')?;
        let kind = GEM_KINDS.iter().position(|k| *k == kind)? as u8;
        let cut = GEM_CUTS.iter().position(|c| *c == cut)? as u8;
        Some(Gem { kind, cut })
    }

    pub fn display_name(self) -> String {
        format!("Gema {} {}", GEM_NAMES[self.kind as usize], GEM_CUT_NAMES[self.cut as usize])
    }

    pub fn bonus(self) -> (Stat, f32) {
        let (s, v) = GEM_STATS[self.kind as usize];
        (s, v[self.cut as usize])
    }

    fn code(self) -> u32 {
        1 + self.kind as u32 * 3 + self.cut as u32
    }

    fn from_code(c: u32) -> Option<Gem> {
        (1..=15).contains(&c).then(|| Gem { kind: ((c - 1) / 3) as u8, cut: ((c - 1) % 3) as u8 })
    }
}

pub fn gem(s: u32, i: usize) -> Option<Gem> {
    if i >= MAX_SOCKETS {
        return None;
    }
    Gem::from_code((s >> (8 + 4 * i)) & 15)
}

pub fn set_gem(s: u32, i: usize, g: Option<Gem>) -> u32 {
    let sh = 8 + 4 * i as u32;
    (s & !(15 << sh)) | (g.map_or(0, Gem::code) << sh)
}

pub fn gems(s: u32) -> Vec<Gem> {
    (0..MAX_SOCKETS).filter_map(|i| gem(s, i)).collect()
}

/// The same piece with its gems taken out.
pub fn without_gems(s: u32) -> u32 {
    s & !(0xfff << 8)
}

/// The first empty socket, if any.
pub fn free_socket(s: u32) -> Option<usize> {
    (0..sockets(s)).find(|&i| gem(s, i).is_none())
}

/// Totals of the gem bonuses over the gear in use.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats(pub [f32; STATS]);

impl Stats {
    pub fn add(&mut self, s: u32) {
        for g in gems(s) {
            let (stat, v) = g.bonus();
            self.0[stat as usize] += v;
        }
    }

    pub fn get(&self, s: Stat) -> f32 {
        self.0[s as usize]
    }
}

/// Tooltip lines below the name: rarity and sockets.
pub fn tooltip(s: u32) -> Vec<(String, [u8; 4])> {
    const SOCKET: [u8; 4] = [150, 150, 150, 255];
    let Some(r) = rarity(s) else { return Vec::new() };
    let mut out = vec![(format!("Raridade: {}", RARITY_NAMES[r]), RARITY_COLORS[r])];
    for i in 0..sockets(s) {
        match gem(s, i) {
            Some(g) => {
                let (stat, v) = g.bonus();
                out.push((format!("[*] {}: {}", g.display_name(), stat.line(v)), GEM_COLORS[g.kind as usize]));
            }
            None => out.push(("[ ] Engaste vazio".to_string(), SOCKET)),
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// World tiers and elites.

pub const TIERS: usize = 5;
pub const TIER_NAMES: [&str; TIERS] = ["Refúgio", "Fronteira", "Ascensão", "Cume", "Pináculo"];
/// Items that raise the world to each tier (none for the first).
pub const TIER_SIGILS: [&str; TIERS] = ["", "sigil_frontier", "sigil_ascent", "sigil_summit", "sigil_pinnacle"];
const TIER_WEIGHTS: [[u32; RARITIES]; TIERS] = [[60, 30, 9, 1, 0], [40, 35, 18, 6, 1], [25, 35, 25, 12, 3], [10, 30, 32, 21, 7], [5, 20, 35, 28, 12]];

/// A loot rarity at a world tier, at least `min`.
pub fn roll_rarity(tier: usize, rng: &mut Rng, min: usize) -> usize {
    let w = TIER_WEIGHTS[tier.min(TIERS - 1)];
    let total: u32 = w.iter().sum();
    let mut roll = rng.below(total);
    for (i, &k) in w.iter().enumerate() {
        if roll < k {
            return i.max(min);
        }
        roll -= k;
    }
    min
}

/// Chance a monster is born an elite.
pub fn elite_chance(tier: usize) -> f32 {
    [0.025, 0.04, 0.06, 0.09, 0.13][tier.min(TIERS - 1)]
}

/// Chance an ordinary monster drops a ranked piece.
pub fn drop_chance(tier: usize) -> f32 {
    [0.02, 0.035, 0.05, 0.07, 0.09][tier.min(TIERS - 1)]
}

/// Health and damage of every monster at a tier.
pub fn tier_health(tier: usize) -> f32 {
    [1.0, 1.2, 1.45, 1.75, 2.1][tier.min(TIERS - 1)]
}

pub fn tier_damage(tier: usize) -> f32 {
    [1.0, 1.1, 1.25, 1.4, 1.6][tier.min(TIERS - 1)]
}

/// An elite of rarity `r` is this much tougher and hits this much harder.
pub fn elite_health(r: usize) -> f32 {
    2.0 + 0.5 * r as f32
}

pub fn elite_damage(r: usize) -> f32 {
    1.3 + 0.15 * r as f32
}

/// What salvaging a piece gives: (rarity of the material, how many). Plain gear gives a
/// common scrap now and then.
pub fn salvage_yield(s: u32, rng: &mut Rng) -> Option<(usize, u8)> {
    match rarity(s) {
        Some(r) => Some((r, 1 + rng.below(3) as u8)),
        None => rng.chance(0.5).then_some((0, 1)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sockets_follow_rarity() {
        assert_eq!(sockets(0), 0);
        assert_eq!(rarity(0), None);
        let counts: Vec<usize> = (0..RARITIES).map(|r| sockets(ranked(r))).collect();
        assert_eq!(counts, vec![0, 1, 1, 2, 3]);
        for r in 0..RARITIES {
            assert_eq!(rarity(ranked(r)), Some(r));
        }
        // A sigil opens a socket even on plain gear, at most two extra and three in all.
        let s = with_extra_socket(0).unwrap();
        assert_eq!((rarity(s), sockets(s)), (Some(0), 1));
        let s = with_extra_socket(s).unwrap();
        assert_eq!(sockets(s), 2);
        assert_eq!(with_extra_socket(s), None);
        assert_eq!(with_extra_socket(ranked(4)), None);
    }

    #[test]
    fn gems_set_and_come_out() {
        let mut s = ranked(3); // Épica: two sockets
        assert_eq!(free_socket(s), Some(0));
        let g = Gem { kind: 2, cut: 1 };
        s = set_gem(s, 0, Some(g));
        assert_eq!(gem(s, 0), Some(g));
        assert_eq!(free_socket(s), Some(1));
        s = set_gem(s, 1, Some(Gem { kind: 4, cut: 2 }));
        assert_eq!(free_socket(s), None);
        assert_eq!(gems(s).len(), 2);
        // The bits of one socket never touch the others or the rarity.
        assert_eq!(rarity(s), Some(3));
        let out = without_gems(s);
        assert!(gems(out).is_empty());
        assert_eq!(sockets(out), 2);
        let mut st = Stats::default();
        st.add(s);
        assert_eq!(st.get(Stat::Damage), 2.0);
        assert_eq!(st.get(Stat::CritChance), 0.15);
        // Every gem decodes to itself and its item name goes both ways.
        for k in 0..5 {
            for c in 0..3 {
                let g = Gem { kind: k, cut: c };
                assert_eq!(gem(set_gem(ranked(4), 2, Some(g)), 2), Some(g));
                assert_eq!(Gem::from_name(&g.item_name()), Some(g));
            }
        }
        assert_eq!(Gem::from_name("gem_nothing_rough"), None);
        assert_eq!(Gem::from_name("gem_dust"), None);
    }

    #[test]
    fn rarity_rolls_follow_tiers() {
        let mut rng = Rng::new(3);
        let avg = |tier: usize, rng: &mut Rng| (0..4000).map(|_| roll_rarity(tier, rng, 0)).sum::<usize>() as f32 / 4000.0;
        let low = avg(0, &mut rng);
        let high = avg(4, &mut rng);
        assert!(high > low + 1.0, "{low} {high}");
        assert!((0..100).all(|_| roll_rarity(0, &mut rng, 2) >= 2));
    }

    #[test]
    fn tooltip_lists_sockets() {
        let s = set_gem(ranked(4), 1, Some(Gem { kind: 0, cut: 0 }));
        let t = tooltip(s);
        assert_eq!(t[0].0, "Raridade: Mítica");
        assert_eq!(t.len(), 1 + 3);
        assert!(t[2].0.contains("Gema Vital Bruta"));
        assert!(tooltip(0).is_empty());
    }
}
