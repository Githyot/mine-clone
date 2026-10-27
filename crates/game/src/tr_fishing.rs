//! Lote 8 of the Terraria side (plan/terraria-completo.md): fishing grows up.
//! - Fishing power: the rod, the best bait in your bag and the fishing potion add up. More
//!   power means quicker bites, more crates and the angler's fish.
//! - Baits are used up as you catch (bait in the bag, not on the rod).
//! - Crates: wooden, iron and golden, and one for each kind of place; right click to open.
//! - The angler (Pescador) asks for one odd fish a day and pays well for it.
//! - Lava fishing with the magma rod: fire fish and obsidian crates.
//! - Potions: fishing (power), crates (more crates), sonar (fish stay on the hook longer).

use glam::Vec3;
use voxel::item::ItemStack;
use voxel::worldgen::Biome;

use crate::audio::Sfx;
use crate::tr_world::Land;
use crate::Game;

/// (rod, fishing power)
const RODS: &[(&str, f32)] = &[("fishing_rod", 10.0), (crate::mods::aquaculture::ROD, 15.0), ("reinforced_rod", 20.0), ("magma_rod", 30.0), ("golden_rod", 45.0)];
/// (bait in the bag, power)
const BAITS: &[(&str, f32)] = &[("master_bait", 50.0), ("journeyman_bait", 30.0), ("apprentice_bait", 15.0), ("worm", 10.0)];

/// The angler's fish: (item, where it lives, as he says it)
const QUESTS: &[(&str, &str, &str)] = &[
    ("mud_king", "swamp", "num pântano ou na selva"),
    ("frost_minnow", "snow", "na água gelada da neve"),
    ("sand_eel", "desert", "na água do deserto"),
    ("sky_trout", "sky", "na água lá no alto, acima da altura 120"),
    ("lantern_fish", "cave", "na água das cavernas, bem fundo"),
    ("vile_carp", "evil", "na água da Ruína ou da Chaga"),
    ("prism_guppy", "prism", "na água do Prisma"),
    ("moon_sunfish", "ocean", "no mar, à noite"),
];

/// (crate, what is inside: item, min, max, chance)
const CRATE_LOOT: &[(&str, &[(&str, u8, u8, f32)])] = &[
    ("wooden_crate", &[("iron_ingot", 2, 5, 0.6), ("copper_ingot", 3, 8, 0.6), ("emerald", 1, 3, 0.5), ("apprentice_bait", 2, 4, 0.5), ("lesser_healing_potion", 1, 3, 0.4), ("cloud_bottle", 1, 1, 0.05), ("running_boots", 1, 1, 0.04)]),
    ("iron_crate", &[("iron_ingot", 4, 9, 0.7), ("gold_ingot", 2, 5, 0.5), ("emerald", 2, 5, 0.6), ("journeyman_bait", 2, 4, 0.5), ("healing_potion", 1, 3, 0.4), ("lucky_horseshoe", 1, 1, 0.06), ("shiny_balloon", 1, 1, 0.06)]),
    ("golden_crate", &[("gold_ingot", 4, 10, 0.8), ("diamond", 1, 3, 0.5), ("emerald", 4, 9, 0.7), ("master_bait", 2, 4, 0.5), ("life_crystal", 1, 1, 0.3), ("magnet_ring", 1, 1, 0.1), ("enchanted_golden_apple", 1, 1, 0.05)]),
    ("jungle_crate", &[("jungle_spore", 3, 8, 0.7), ("emerald", 2, 5, 0.6), ("jungle_yoyo", 1, 1, 0.15), ("thorn_chakram", 1, 1, 0.1), ("honey_bottle", 2, 4, 0.5)]),
    ("frozen_crate", &[("frost_shard", 2, 6, 0.7), ("ice_boomerang", 1, 1, 0.15), ("blue_ice", 4, 10, 0.5), ("emerald", 2, 5, 0.5)]),
    ("ocean_crate", &[("prismarine_shard", 4, 10, 0.7), ("nautilus_shell", 1, 2, 0.4), ("heart_of_the_sea", 1, 1, 0.05), ("trident", 1, 1, 0.03), ("emerald", 2, 5, 0.5)]),
    ("sandstone_crate", &[("sturdy_fossil", 2, 5, 0.7), ("sandstorm_bottle", 1, 1, 0.12), ("gold_ingot", 2, 5, 0.5), ("emerald", 2, 5, 0.5)]),
    ("ruin_crate", &[("umbral_ore", 4, 10, 0.5), ("sanguinite_ore", 4, 10, 0.5), ("soul_of_night", 1, 4, 0.5), ("shadow_orb_lamp", 1, 1, 0.08), ("emerald", 2, 5, 0.5)]),
    ("prism_crate", &[("soul_of_light", 2, 5, 0.6), ("pixie_dust", 3, 8, 0.6), ("radiant_ingot", 1, 3, 0.3), ("fairy_bell", 1, 1, 0.06), ("emerald", 3, 6, 0.5)]),
    ("obsidian_crate", &[("obsidian", 4, 10, 0.8), ("magmarite_ingot", 2, 5, 0.6), ("lava_koi", 1, 2, 0.4), ("flame_boomerang", 1, 1, 0.12), ("magma_yoyo", 1, 1, 0.12), ("emerald", 3, 6, 0.5)]),
];

/// The angler's thanks, cycling: (item, count).
const REWARDS: &[(&str, u8)] = &[
    ("reinforced_rod", 1),
    ("crate_potion", 2),
    ("journeyman_bait", 5),
    ("sonar_potion", 2),
    ("golden_rod", 1),
    ("fishing_potion", 3),
    ("master_bait", 5),
    ("golden_crate", 1),
];

#[derive(Default)]
pub struct Fishing {
    /// The fish asked for today, and whether it was brought in.
    quest: Option<usize>,
    done: bool,
    pub(crate) quests_done: u32,
    /// Seconds left on the fishing, crate and sonar potions.
    potion: f32,
    crates: f32,
    sonar: f32,
    /// The float sits in lava.
    pub(crate) lava: bool,
}

pub fn rod_power(name: &str) -> Option<f32> {
    RODS.iter().find(|r| r.0 == name).map(|r| r.1)
}

impl Game {
    pub(crate) fn fishing_power(&self) -> f32 {
        let rod = self.held_name().and_then(|n| rod_power(&n)).unwrap_or(10.0);
        let bait = BAITS.iter().find(|b| self.inv_has(b.0)).map_or(0.0, |b| b.1);
        let potion = if self.t5.tr.fish.potion > 0.0 { 15.0 } else { 0.0 };
        rod + bait + potion
    }

    fn inv_has(&self, name: &str) -> bool {
        let Some(id) = self.items.id(name) else { return false };
        self.surv.inv.slots.iter().flatten().any(|s| s.item == id)
    }

    fn take_one(&mut self, name: &str) {
        let Some(id) = self.items.id(name) else { return };
        if let Some(k) = self.surv.inv.slots.iter().position(|s| s.is_some_and(|s| s.item == id)) {
            self.surv.inv.take(k, 1);
        }
    }

    /// Bites come sooner with more power.
    pub(crate) fn tr_bite_factor(&self) -> f32 {
        40.0 / (30.0 + self.fishing_power())
    }

    /// The sonar potion keeps the fish on the hook longer.
    pub(crate) fn tr_bite_window(&self) -> f32 {
        if self.t5.tr.fish.sonar > 0.0 {
            1.6
        } else {
            1.0
        }
    }

    /// Can the float rest on lava?
    pub(crate) fn lava_rod(&self) -> bool {
        self.held_name().as_deref() == Some("magma_rod")
    }

    /// Where the float is, as the angler and the crates see it.
    fn water_kind(&self, at: Vec3) -> &'static str {
        if self.t5.tr.fish.lava {
            return "lava";
        }
        if at.y > 120.0 {
            return "sky";
        }
        match self.t5.tr.lands.here {
            Some(Land::Prism) => return "prism",
            Some(_) => return "evil",
            None => {}
        }
        if at.y < 30.0 {
            return "cave";
        }
        match self.world.gen.base_biome_at(at.x.floor() as i32, at.z.floor() as i32) {
            Biome::Jungle | Biome::Swamp | Biome::MangroveSwamp => "swamp",
            Biome::Snowy | Biome::Taiga | Biome::SnowyPeaks | Biome::FrozenOcean => "snow",
            Biome::Desert | Biome::Badlands => "desert",
            Biome::Ocean | Biome::Beach => "ocean",
            _ => "forest",
        }
    }

    /// A catch decided here instead of the usual table: crates, the angler's fish, lava fish.
    pub(crate) fn tr_special_catch(&mut self, at: Vec3) -> Option<&'static str> {
        let power = self.fishing_power();
        // Baits are used up as you catch.
        if let Some(b) = BAITS.iter().find(|b| self.inv_has(b.0)) {
            if self.survival && self.surv.rng.f32() < 0.6 {
                self.take_one(b.0);
            }
        }
        let kind = self.water_kind(at);
        let roll = self.surv.rng.f32();
        let mut crate_p = 0.08 + power / 500.0;
        if self.t5.tr.fish.crates > 0.0 {
            crate_p *= 2.0;
        }
        if kind == "lava" {
            return Some(if roll < crate_p { "obsidian_crate" } else if roll < 0.6 { "lava_koi" } else { "obsidian_fish" });
        }
        if roll < crate_p {
            let r = self.surv.rng.f32();
            return Some(if r < 0.06 + power / 1000.0 {
                "golden_crate"
            } else if r < 0.3 {
                "iron_crate"
            } else if r < 0.65 {
                match kind {
                    "swamp" => "jungle_crate",
                    "snow" => "frozen_crate",
                    "ocean" => "ocean_crate",
                    "desert" => "sandstone_crate",
                    "evil" => "ruin_crate",
                    "prism" => "prism_crate",
                    _ => "wooden_crate",
                }
            } else {
                "wooden_crate"
            });
        }
        let f = &self.t5.tr.fish;
        if let Some(q) = f.quest.filter(|_| !f.done) {
            let (fish, place, _) = QUESTS[q];
            let fits = place == kind && (fish != "moon_sunfish" || self.is_night());
            if fits && self.surv.rng.f32() < 0.15 + power / 300.0 {
                return Some(fish);
            }
        }
        None
    }

    pub(crate) fn angler_new_day(&mut self) {
        let f = &mut self.t5.tr.fish;
        f.quest = None;
        f.done = false;
    }

    /// Talking to the angler: bring today's fish for a reward, or hear what he wants.
    pub(crate) fn angler_talk(&mut self) {
        if self.t5.tr.fish.quest.is_none() {
            let q = (self.surv.rng.f32() * QUESTS.len() as f32) as usize % QUESTS.len();
            self.t5.tr.fish.quest = Some(q);
        }
        let q = self.t5.tr.fish.quest.unwrap();
        let (fish, _, place) = QUESTS[q];
        if self.t5.tr.fish.done {
            self.toast("Pescador: Já pescou o de hoje! Volte amanhã.", 4.0);
            return;
        }
        if self.inv_has(fish) {
            self.take_one(fish);
            let n = self.t5.tr.fish.quests_done as usize;
            let (item, count) = REWARDS[n % REWARDS.len()];
            self.t5.tr.fish.done = true;
            self.t5.tr.fish.quests_done += 1;
            for (it, c) in [(item, count), ("emerald", 3 + (n as u8 % 4))] {
                if let Some(id) = self.items.id(it) {
                    let at = self.body.pos + Vec3::Y;
                    self.spawn_drop(at, ItemStack::new(id, c), Vec3::Y * 2.0);
                }
            }
            self.toast(format!("Pescador: Isso! Tome, pelo seu trabalho. ({} missões feitas)", n + 1), 4.0);
            self.sfx(Sfx::Xp);
            return;
        }
        let dn = self.items.id(fish).map(|i| self.items.get(i).display_name.clone()).unwrap_or_default();
        self.toast(format!("Pescador: Hoje eu quero um {dn}. Ele vive {place}."), 5.0);
    }

    /// Crates and the fishing potions.
    pub(crate) fn tr_fishing_use(&mut self, name: &str) -> Option<bool> {
        let timer = match name {
            "fishing_potion" => Some(0),
            "crate_potion" => Some(1),
            "sonar_potion" => Some(2),
            _ => None,
        };
        if let Some(k) = timer {
            let f = &mut self.t5.tr.fish;
            *[&mut f.potion, &mut f.crates, &mut f.sonar][k] = 240.0;
            self.toast(["Poção de Pesca: mais poder de pesca por 4 minutos", "Poção de Caixas: mais caixas por 4 minutos", "Poção de Sonar: o peixe fica mais tempo no anzol"][k], 3.0);
            self.sfx(Sfx::Eat);
            if self.survival {
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
            }
            return Some(true);
        }
        let &(_, loot) = CRATE_LOOT.iter().find(|c| c.0 == name)?;
        let at = self.body.eye() + self.forward() * 0.8;
        let mut got = 0;
        for &(item, lo, hi, chance) in loot {
            if self.surv.rng.f32() >= chance {
                continue;
            }
            let Some(id) = self.items.id(item) else { continue };
            let n = lo + (self.surv.rng.f32() * (hi - lo + 1) as f32) as u8;
            self.spawn_drop(at, ItemStack::new(id, n.min(hi)), Vec3::Y * 2.5);
            got += 1;
        }
        if got == 0 {
            if let Some(id) = self.items.id("emerald") {
                self.spawn_drop(at, ItemStack::new(id, 2), Vec3::Y * 2.5);
            }
        }
        self.sfx(Sfx::Place(crate::audio::Material::Wood));
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        Some(true)
    }

    pub(crate) fn update_tr_fishing(&mut self, dt: f32) {
        let f = &mut self.t5.tr.fish;
        f.potion = (f.potion - dt).max(0.0);
        f.crates = (f.crates - dt).max(0.0);
        f.sonar = (f.sonar - dt).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn crate_goods_exist() {
        fn read(dir: &std::path::Path, out: &mut String) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    read(&p, out);
                } else if p.extension().is_some_and(|x| x == "ron") {
                    out.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
                }
            }
        }
        let mut items = String::new();
        read(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../packs/vanilla")), &mut items);
        let mut missing = Vec::new();
        for (c, loot) in super::CRATE_LOOT {
            for name in std::iter::once(*c).chain(loot.iter().map(|l| l.0)) {
                if !items.contains(&format!("\"{name}\"")) {
                    missing.push(name);
                }
            }
        }
        let rewards = super::REWARDS.iter().map(|r| r.0);
        for name in rewards.chain(super::BAITS.iter().map(|b| b.0)) {
            if !items.contains(&format!("\"{name}\"")) {
                missing.push(name);
            }
        }
        for (name, _, _) in super::QUESTS {
            if !items.contains(&format!("\"{name}\"")) {
                missing.push(name);
            }
        }
        assert!(missing.is_empty(), "{missing:?}");
    }
}
