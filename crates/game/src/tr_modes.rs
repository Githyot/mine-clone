//! Lote 10 of the Terraria side (plan/terraria-completo.md): the world's difficulty and the
//! bestiary.
//!
//! - Normal, Especialista and Mestre, chosen when the world is made (or later with the Pedra
//!   da Ousadia). Monsters have 2x / 3x health and hit 1.5x / 2x harder.
//! - In Especialista and Mestre a boss leaves a treasure bag: its loot rolled again, emeralds,
//!   and for most bosses an accessory found nowhere else. In Mestre it also leaves its relic.
//! - The bestiary counts every creature killed; the book shows them, and finding a quarter,
//!   half, three quarters and all of them gives a present.

use std::collections::HashMap;

use glam::Vec3;
use voxel::item::ItemStack;

use crate::audio::Sfx;
use crate::Game;

pub const MODE_NAMES: [&str; 3] = ["Normal", "Especialista", "Mestre"];

/// Boss -> the accessory only its bag holds.
const EXPERT_ITEMS: &[(&str, &str)] = &[
    ("king_jelly", "jelly_heart"),
    ("great_eye", "eye_of_the_watcher"),
    ("blight_mind", "mind_veil"),
    ("hive_queen", "hive_heart"),
    ("frost_colossus", "frost_core"),
    ("deep_worm", "worm_scarf"),
    ("bone_king", "bone_glove"),
    ("world_breaker", "breaker_heart"),
    ("mech_eye", "mech_core"),
    ("mech_worm", "mech_core"),
    ("mech_skull", "mech_core"),
    ("vine_queen", "vine_seed"),
    ("jelly_queen", "gel_crown"),
    ("scale_idol", "sun_core"),
    ("tide_duke", "tide_scale"),
    ("prism_empress", "prism_heart"),
    ("lunar_prophet", "prophet_eye"),
    ("celestial_herald", "herald_sigil"),
];

/// Bestiary presents: (share found, item, how many).
const REWARDS: [(f32, &str, u8); 4] = [(0.25, "emerald", 16), (0.5, "creature_radar", 1), (0.75, "lucky_coin", 1), (1.0, "aegis_fruit", 1)];

#[derive(Default)]
pub struct Modes {
    pub mode: u8,
    pub kills: HashMap<String, u32>,
    /// Bestiary presents given (bits).
    pub rewards: u8,
}

impl Game {
    /// How much harder monsters hit.
    pub(crate) fn mode_damage(&self) -> f32 {
        [1.0, 1.5, 2.0][self.t5.tr.modes.mode.min(2) as usize]
    }

    pub(crate) fn update_tr_modes(&mut self) {
        self.mobs.hp_scale = [1.0, 2.0, 3.0][self.t5.tr.modes.mode.min(2) as usize];
    }

    /// A boss fell: its bag (Especialista and up) and relic (Mestre) near the player.
    pub(crate) fn modes_boss_defeated(&mut self, name: &str) {
        let mode = self.t5.tr.modes.mode;
        if mode == 0 {
            return;
        }
        let at = self.body.pos + Vec3::Y * 1.5;
        let mut give = vec![format!("bag_{name}")];
        if mode >= 2 {
            give.push(format!("relic_{name}"));
        }
        for n in give {
            if let Some(id) = self.items.id(&n) {
                self.spawn_drop(at, ItemStack::new(id, 1), Vec3::Y * 3.0);
            }
        }
    }

    pub(crate) fn tr_modes_use(&mut self, name: &str) -> Option<bool> {
        if let Some(boss) = name.strip_prefix("bag_") {
            self.open_boss_bag(boss);
            return Some(true);
        }
        match name {
            "daring_stone" => {
                let m = &mut self.t5.tr.modes.mode;
                *m = (*m + 1) % 3;
                let m = *m;
                self.update_tr_modes();
                let text = match m {
                    0 => "Mundo Normal: monstros como sempre",
                    1 => "Mundo Especialista: monstros com o dobro de vida e mais fortes; chefes deixam sacolas",
                    _ => "Mundo Mestre: monstros com o triplo de vida e o dobro de força; chefes deixam sacolas e relíquias",
                };
                self.toast(text, 4.0);
                self.sfx(Sfx::Thunder(0.2));
                Some(true)
            }
            "bestiary" => Some(self.open_bestiary()),
            _ => None,
        }
    }

    fn open_boss_bag(&mut self, boss: &str) {
        let at = self.body.eye() + self.forward() * 0.8;
        let mut stacks = Vec::new();
        if let Some(def) = self.mobs.reg.id(boss) {
            for d in self.mobs.reg.defs[def].drops.clone() {
                if self.surv.rng.f32() >= d.chance.max(0.35) {
                    continue;
                }
                let Some(id) = self.items.id(&d.item) else { continue };
                let n = d.min + (self.surv.rng.f32() * (d.max.saturating_sub(d.min) + 1) as f32) as u8;
                stacks.push(ItemStack::new(id, u16::from(n.clamp(1, d.max.max(1)))));
            }
        }
        if let Some(&(_, item)) = EXPERT_ITEMS.iter().find(|e| e.0 == boss) {
            if let Some(id) = self.items.id(item) {
                stacks.push(ItemStack::new(id, 1));
            }
        }
        if let Some(id) = self.items.id("emerald") {
            stacks.push(ItemStack::new(id, u16::from(3 + (self.surv.rng.f32() * 6.0) as u8)));
        }
        for st in stacks {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, 2.5, self.surv.rng.f32() - 0.5);
            self.spawn_drop(at, st, v);
        }
        self.sfx(Sfx::Place(crate::audio::Material::Cloth));
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Bestiary.

    /// Creatures that count for the bestiary (monsters and bosses).
    fn bestiary_defs(&self) -> Vec<usize> {
        let defs = &self.mobs.reg.defs;
        (0..defs.len()).filter(|&i| defs[i].kind == voxel::mob::Kind::Hostile || defs[i].boss).collect()
    }

    pub(crate) fn modes_deaths(&mut self, deaths: &[(usize, Vec3)]) {
        let mut new = Vec::new();
        for &(def, _) in deaths {
            let d = &self.mobs.reg.defs[def];
            if !(d.kind == voxel::mob::Kind::Hostile || d.boss) {
                continue;
            }
            let k = self.t5.tr.modes.kills.entry(d.name.clone()).or_insert(0);
            *k += 1;
            if *k == 1 {
                new.push(d.display_name.clone());
            }
        }
        if new.is_empty() {
            return;
        }
        self.toast(format!("Novo no bestiário: {}", new.join(", ")), 3.0);
        let total = self.bestiary_defs().len().max(1);
        let found = self.bestiary_defs().iter().filter(|&&i| self.t5.tr.modes.kills.contains_key(&self.mobs.reg.defs[i].name)).count();
        let share = found as f32 / total as f32;
        for (k, &(need, item, n)) in REWARDS.iter().enumerate() {
            if share + 1e-4 >= need && self.t5.tr.modes.rewards & (1 << k) == 0 {
                self.t5.tr.modes.rewards |= 1 << k;
                if let Some(id) = self.items.id(item) {
                    let at = self.body.pos + Vec3::Y;
                    self.spawn_drop(at, ItemStack::new(id, u16::from(n)), Vec3::Y * 2.0);
                }
                self.toast(format!("Bestiário {}% completo: um presente para você!", (need * 100.0) as u32), 4.0);
                self.sfx(Sfx::Xp);
            }
        }
    }

    /// The bestiary as a book: a summary page, then the creatures found, 10 to a page.
    fn bestiary_pages(&self) -> Vec<String> {
        let defs = self.bestiary_defs();
        let kills = &self.t5.tr.modes.kills;
        let mut found: Vec<(String, u32, bool)> = defs
            .iter()
            .filter_map(|&i| {
                let d = &self.mobs.reg.defs[i];
                kills.get(&d.name).map(|&n| (d.display_name.clone(), n, d.boss))
            })
            .collect();
        found.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        let bosses = defs.iter().filter(|&&i| self.mobs.reg.defs[i].boss).count();
        let bosses_found = found.iter().filter(|f| f.2).count();
        let all: u32 = kills.values().sum();
        let pct = found.len() * 100 / defs.len().max(1);
        let mut pages = vec![format!(
            "Criaturas achadas:\n  {} de {} ({pct}%)\nChefes: {} de {}\nAbatidos: {all}\n\nMundo: {}\n\nPresentes em 25%,\n50%, 75% e 100%.",
            found.len(),
            defs.len(),
            bosses_found,
            bosses,
            MODE_NAMES[self.t5.tr.modes.mode.min(2) as usize]
        )];
        for chunk in found.chunks(10) {
            let lines: Vec<String> = chunk
                .iter()
                .map(|(n, k, boss)| {
                    let mut n: String = n.chars().take(15).collect();
                    if *boss {
                        n = format!("*{n}");
                    }
                    format!("{n} {k}")
                })
                .collect();
            pages.push(lines.join("\n"));
        }
        pages
    }

    fn open_bestiary(&mut self) -> bool {
        let pages = self.bestiary_pages();
        let books = &mut self.extras.books;
        let id = match books.iter().position(|b| b.signed && b.title == "Bestiário") {
            Some(i) => i,
            None => {
                books.push(crate::vanilla_extras::Book { title: "Bestiário".into(), pages: Vec::new(), signed: true });
                books.len() - 1
            }
        };
        books[id].pages = pages;
        self.extras.book = Some(crate::vanilla_extras::BookOpen { id: id as u16 + 1, page: 0, write: false });
        self.set_screen(crate::Screen::Sign);
        self.sfx(Sfx::Place(crate::audio::Material::Cloth));
        true
    }
}

impl Game {
    /// Test scenes: MC_TR=relics (a row of relics and bags) and bestiary (the book, with a
    /// few kills counted).
    pub(crate) fn test_tr_modes(&mut self, scene: &str) {
        if scene == "bestiary" {
            for (k, d) in self.mobs.reg.defs.iter().enumerate().filter(|(_, d)| d.kind == voxel::mob::Kind::Hostile || d.boss).step_by(3) {
                self.t5.tr.modes.kills.insert(d.name.clone(), 1 + (k as u32 * 7) % 40);
            }
            self.t5.tr.modes.mode = 2;
            self.open_bestiary();
            log::info!("bestiary pages: {:?}", self.bestiary_pages());
            if let Some(b) = self.extras.book.as_mut() {
                b.page = 1;
            }
            return;
        }
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-flat.z, 0.0, flat.x);
        let base = self.body.pos + flat * 5.0;
        let bosses = ["king_jelly", "great_eye", "hive_queen", "frost_colossus", "prism_empress", "lunar_prophet", "tide_duke"];
        for (k, b) in bosses.iter().enumerate() {
            let p = (base + side * ((k as f32 - 3.0) * 1.5)).floor().as_ivec3();
            let id = self.reg.id_or_air(&format!("relic_{b}"));
            self.world.set_block(p, id);
        }
        for (k, b) in bosses.iter().enumerate() {
            if let Some(id) = self.items.id(&format!("bag_{b}")) {
                self.surv.inv.slots[k] = Some(ItemStack::new(id, 1));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn expert_items_are_accessories() {
        for (boss, item) in super::EXPERT_ITEMS {
            assert!(crate::rpg::accessory(item).is_some(), "{boss}: {item}");
        }
    }
}
