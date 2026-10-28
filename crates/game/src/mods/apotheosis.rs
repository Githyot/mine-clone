//! Apotheosis ideas (our own code): gems and sockets, the salvaging and gem cutting tables,
//! world tiers and elite monsters. The gear's random bonuses and their reforging are the
//! game's own affixes; this adds what sits beside them:
//!
//! - Gear dropped by monsters and found in chests has a loot rarity (Comum .. Mítica) that
//!   gives it sockets. A gem on the cursor right-clicked onto the gear sets it, or the gem
//!   cutting table does (it also cuts three gems into a better one, opens an extra socket
//!   with a Sigil of Socketing and takes gems back out with a Sigil of Withdrawal).
//! - The salvaging table breaks gear into the material of its rarity (and gems into dust).
//! - World tiers (Refúgio .. Pináculo) are raised with sigils made from those materials and
//!   picked with Ctrl+T: monsters get tougher, elites commoner and loot better.
//! - Now and then a monster is born an elite: a coloured name, much more health and harder
//!   hits, and it always drops ranked gear.
//!
//! The numbers (sockets, gems, tiers) are in `voxel::gems`.

use glam::{IVec3, Vec3};
use voxel::gems::{self, Gem, Stat, Stats};
use voxel::item::{Inventory, ItemStack};
use voxel::mob::Kind;
use voxel::rng::Rng;
use winit::keyboard::KeyCode;

use crate::audio::{Material, Sfx};
use crate::survival::{Container, Open, Slot};
use crate::ui::WHITE;
use crate::{Game, Screen};

/// Mob::elite: 0 not looked at yet, 1 an ordinary monster, 2.. an elite of rarity (n - 2),
/// DONE when nothing more is to be done with it (not a monster, or its loot was dropped).
const ELITE_PLAIN: u8 = 1;
const DONE: u8 = 255;

/// Gear materials and the world tier they start dropping at.
const LOOT_MATERIALS: [(&str, usize); 12] = [
    ("wooden", 0),
    ("stone", 0),
    ("leather", 0),
    ("golden", 0),
    ("copper", 0),
    ("iron", 1),
    ("bronze", 1),
    ("lead", 1),
    ("silver", 2),
    ("tungsten", 2),
    ("diamond", 3),
    ("platinum", 3),
];
const LOOT_KINDS: [&str; 8] = ["sword", "axe", "pickaxe", "shovel", "helmet", "chestplate", "leggings", "boots"];

pub struct Apotheosis {
    rng: Rng,
    glow_t: f32,
    /// The tier panel (Ctrl+T) shows until this time.
    panel_until: f32,
}

impl Default for Apotheosis {
    fn default() -> Self {
        Apotheosis { rng: Rng::new(0xa907), glow_t: 0.0, panel_until: 0.0 }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Table {
    Salvaging,
    GemCutting,
}

impl Game {
    pub(crate) fn world_tier(&self) -> usize {
        (self.mods.save.world_tier as usize).min(gems::TIERS - 1)
    }

    /// Gem bonuses of the held tool or weapon and the worn armour.
    pub(crate) fn gem_stats(&self) -> Stats {
        let mut st = Stats::default();
        if let Some(h) = self.held_stack().filter(|h| self.items.get(h.item).tool.is_some()) {
            st.add(h.sockets);
        }
        for a in self.surv.armor.iter().flatten() {
            st.add(a.sockets);
        }
        st
    }

    /// Weapons swing this much faster.
    pub(crate) fn gem_attack_speed(&self) -> f32 {
        1.0 + self.gem_stats().get(Stat::AttackSpeed)
    }

    pub(crate) fn gem_armor(&self) -> u32 {
        self.gem_stats().get(Stat::Armor) as u32
    }

    /// A melee hit with gems: more damage, and a chance of a critical hit.
    pub(crate) fn gem_attack(&mut self, dmg: f32) -> f32 {
        let st = self.gem_stats();
        let mut d = dmg + st.get(Stat::Damage);
        if st.get(Stat::CritChance) > 0.0 && self.mods.apoth.rng.chance(st.get(Stat::CritChance)) {
            d *= 1.5;
            let layer = self.glow_layer();
            let at = self.body.eye() + self.forward() * 2.0;
            for k in 0..8 {
                let a = k as f32 * 0.785;
                self.particles.glow(at, Vec3::new(a.cos(), 0.6, a.sin()) * 1.5, layer, [0.8, 0.5, 1.3], 0.05, 0.5);
            }
        }
        d
    }

    // -----------------------------------------------------------------------------------------
    // Every frame.

    pub(crate) fn update_apotheosis(&mut self, dt: f32) {
        self.sync_gem_health();
        if dt <= 0.0 {
            return;
        }
        self.roll_new_mobs();
        if self.survival {
            self.drop_mob_loot();
        }
        self.mods.apoth.glow_t -= dt;
        if self.mods.apoth.glow_t <= 0.0 {
            self.mods.apoth.glow_t = 0.2;
            let layer = self.glow_layer();
            let mut sparks = Vec::new();
            for m in self.mobs.list.iter().filter(|m| m.alive() && (2..DONE).contains(&m.elite)) {
                let c = gems::RARITY_COLORS[(m.elite - 2) as usize];
                let h = self.mobs.reg.defs[m.def].height;
                sparks.push((m.body.pos + Vec3::Y * h * 0.5, [c[0] as f32 / 160.0, c[1] as f32 / 160.0, c[2] as f32 / 160.0]));
            }
            for (at, tint) in sparks {
                let r = &mut self.mods.apoth.rng;
                let off = Vec3::new(r.f32() - 0.5, r.f32() - 0.5, r.f32() - 0.5) * 0.9;
                self.particles.glow(at + off, Vec3::Y * 0.6, layer, tint, 0.06, 1.0);
            }
        }
    }

    /// Vital gems add health on top of the hearts the player has; what was added is kept in
    /// mods.ron so it is never counted twice.
    fn sync_gem_health(&mut self) {
        let applied = self.mods.save.gem_health;
        let base = self.max_health - applied;
        let want = if self.survival { self.gem_stats().get(Stat::Health).min(40.0 - base).max(0.0) } else { 0.0 };
        if want != applied {
            self.max_health = base + want;
            self.health = self.health.min(self.max_health);
            self.mods.save.gem_health = want;
        }
    }

    /// New monsters: tougher at higher tiers, and now and then an elite.
    fn roll_new_mobs(&mut self) {
        let tier = self.world_tier();
        let Game { mobs, mods, .. } = self;
        let rng = &mut mods.apoth.rng;
        for m in mobs.list.iter_mut().filter(|m| m.elite == 0) {
            let def = &mobs.reg.defs[m.def];
            if def.kind != Kind::Hostile || def.boss || def.town {
                m.elite = DONE;
                continue;
            }
            let mut hp = gems::tier_health(tier);
            m.power = gems::tier_damage(tier);
            m.elite = ELITE_PLAIN;
            if rng.chance(gems::elite_chance(tier)) {
                let r = gems::roll_rarity(tier, rng, 1);
                m.elite = 2 + r as u8;
                hp *= gems::elite_health(r);
                m.power *= gems::elite_damage(r);
                if let Some(s) = m.body.speed_override.as_mut() {
                    *s *= 1.15;
                }
            }
            m.max_health *= hp;
            m.health *= hp;
        }
    }

    /// Dead monsters: an elite always leaves ranked gear, an ordinary one sometimes.
    fn drop_mob_loot(&mut self) {
        let tier = self.world_tier();
        let mut loot = Vec::new();
        for m in self.mobs.list.iter_mut().filter(|m| !m.alive() && (ELITE_PLAIN..DONE).contains(&m.elite)) {
            loot.push((m.elite, m.body.pos + Vec3::Y * 0.5));
            m.elite = DONE;
        }
        for (e, at) in loot {
            if e == ELITE_PLAIN {
                if self.mods.apoth.rng.chance(gems::drop_chance(tier)) {
                    let st = self.ranked_gear(0);
                    self.pop_drop(at, st);
                }
                if self.mods.apoth.rng.chance(0.02 + 0.01 * tier as f32) {
                    let g = self.rough_gem();
                    self.pop_drop(at, g);
                }
                continue;
            }
            let r = (e - 2) as usize;
            let st = self.ranked_gear(r);
            self.pop_drop(at, st);
            if self.mods.apoth.rng.chance(0.5 + 0.1 * r as f32) {
                let g = self.rough_gem();
                self.pop_drop(at, g);
            }
            if let Some(id) = self.items.id(gems::MATERIALS[r]) {
                let n = 1 + self.mods.apoth.rng.below(2) as u8;
                self.pop_drop(at, ItemStack::new(id, n));
            }
            self.spawn_orbs(at, 10 + 6 * r as u32);
            self.toast(format!("Elite {} derrotado!", gems::RARITY_NAMES[r]), 3.0);
        }
    }

    /// A random piece of gear for this world tier, of rarity at least `min`.
    pub(crate) fn ranked_gear(&mut self, min: usize) -> ItemStack {
        let tier = self.world_tier();
        let r = gems::roll_rarity(tier, &mut self.mods.apoth.rng, min);
        let mats: Vec<&str> = LOOT_MATERIALS.iter().filter(|(_, t)| *t <= tier).map(|(m, _)| *m).collect();
        for _ in 0..20 {
            let rng = &mut self.mods.apoth.rng;
            // Newer materials are a little likelier, so higher tiers feel it.
            let i = (rng.below(mats.len() as u32) as usize).max(rng.below(mats.len() as u32) as usize);
            let k = LOOT_KINDS[rng.below(LOOT_KINDS.len() as u32) as usize];
            if let Some(id) = self.items.id(&format!("{}_{k}", mats[i])) {
                return ItemStack { sockets: gems::ranked(r), ..ItemStack::new(id, 1) };
            }
        }
        let id = self.items.id("iron_sword").unwrap_or(1);
        ItemStack { sockets: gems::ranked(r), ..ItemStack::new(id, 1) }
    }

    fn rough_gem(&mut self) -> ItemStack {
        let kind = self.mods.apoth.rng.below(5) as u8;
        let id = self.items.id(&Gem { kind, cut: 0 }.item_name()).unwrap_or(1);
        ItemStack::new(id, 1)
    }

    /// A chest the world made: gear in it may come ranked, and now and then a ranked piece
    /// or a gem is added.
    pub(crate) fn apoth_chest_loot(&mut self, inv: &mut Inventory) {
        let tier = self.world_tier();
        for i in 0..inv.slots.len() {
            let Some(st) = inv.slots[i] else { continue };
            let it = self.items.get(st.item);
            if (it.tool.is_some() || it.armor.is_some()) && st.sockets == 0 && self.mods.apoth.rng.chance(0.5) {
                let r = gems::roll_rarity(tier, &mut self.mods.apoth.rng, 0);
                inv.slots[i] = Some(ItemStack { sockets: gems::ranked(r), ..st });
            }
        }
        let mut extra = Vec::new();
        if self.mods.apoth.rng.chance(0.25 + 0.05 * tier as f32) {
            extra.push(self.ranked_gear(0));
        }
        if self.mods.apoth.rng.chance(0.2) {
            extra.push(self.rough_gem());
        }
        for st in extra {
            let free: Vec<usize> = (0..inv.slots.len()).filter(|&i| inv.slots[i].is_none()).collect();
            if !free.is_empty() {
                let k = free[self.mods.apoth.rng.below(free.len() as u32) as usize];
                inv.slots[k] = Some(st);
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Using things.

    pub(crate) fn apoth_table(&self, p: IVec3) -> Option<Table> {
        match self.reg.get(self.world.block(p)?).name.as_str() {
            "salvaging_table" => Some(Table::Salvaging),
            "gem_cutting_table" => Some(Table::GemCutting),
            _ => None,
        }
    }

    /// Right click: a table opens, a sigil raises the world tier.
    pub(crate) fn apoth_use(&mut self) -> bool {
        if let Some(hit) = self.target {
            if let Some(t) = self.apoth_table(hit.block) {
                if self.crouch <= 0.5 || self.held_stack().is_none() {
                    if self.survival {
                        let n = if t == Table::Salvaging { 1 } else { 3 };
                        self.surv.containers.entry(hit.block).or_insert_with(|| Container::Chest(Inventory::new(n)));
                        self.surv.open = Open::Chest(hit.block);
                        self.set_screen(Screen::Inventory);
                        self.sfx(Sfx::Place(Material::Stone));
                    }
                    return true;
                }
            }
        }
        let Some(st) = self.held_stack() else { return false };
        let name = self.items.get(st.item).name.clone();
        let Some(t) = gems::TIER_SIGILS.iter().position(|s| !s.is_empty() && *s == name) else { return false };
        let unlocked = self.mods.save.tier_unlocked as usize;
        if t <= unlocked {
            self.mods.save.world_tier = t as u8;
            self.toast(format!("Nível do mundo: {} (já alcançado)", gems::TIER_NAMES[t]), 3.0);
            return true;
        }
        if t > unlocked + 1 {
            self.toast(format!("Primeiro alcance o nível {}", gems::TIER_NAMES[t - 1]), 3.0);
            return true;
        }
        self.mods.save.tier_unlocked = t as u8;
        self.mods.save.world_tier = t as u8;
        let sel = self.selected;
        self.surv.inv.take(sel, 1);
        self.sfx(Sfx::Portal);
        self.anim.start_swing();
        self.mods.apoth.panel_until = self.now + 5.0;
        self.toast(format!("O mundo subiu ao nível {}!", gems::TIER_NAMES[t]), 4.0);
        true
    }

    /// Ctrl+T: switch between the world tiers reached so far.
    pub(crate) fn apoth_key(&mut self, code: KeyCode) -> bool {
        let ctrl = self.keys.contains(&KeyCode::ControlLeft) || self.keys.contains(&KeyCode::ControlRight);
        if code != KeyCode::KeyT || !ctrl || self.screen != Screen::Playing || !self.world_open {
            return false;
        }
        let n = self.mods.save.tier_unlocked as usize + 1;
        if self.now < self.mods.apoth.panel_until {
            self.mods.save.world_tier = ((self.world_tier() + 1) % n) as u8;
        }
        self.mods.apoth.panel_until = self.now + 4.0;
        true
    }

    /// Right click with a gem on the cursor over socketed gear: set the gem.
    pub(crate) fn apoth_socket_click(&mut self, slot: Slot, right: bool) -> bool {
        let (Some(c), Some(gear)) = (self.surv.cursor, self.slot_get(slot)) else { return false };
        if !right || gear.count != 1 {
            return false;
        }
        let Some(g) = Gem::from_name(&self.items.get(c.item).name) else { return false };
        let Some(i) = gems::free_socket(gear.sockets) else { return false };
        self.slot_set(slot, Some(ItemStack { sockets: gems::set_gem(gear.sockets, i, Some(g)), ..gear }));
        self.surv.cursor = (c.count > 1).then_some(ItemStack { count: c.count - 1, ..c });
        self.sfx(Sfx::Place(Material::Glass));
        true
    }

    // -----------------------------------------------------------------------------------------
    // Tooltips.

    /// Name colour: the rarity's for ranked gear, else white.
    pub(crate) fn gem_name_color(&self, st: &ItemStack) -> [u8; 4] {
        gems::rarity(st.sockets).map_or(WHITE, |r| gems::RARITY_COLORS[r])
    }

    pub(crate) fn gem_tooltip(&self, st: &ItemStack) -> Vec<(String, [u8; 4])> {
        let mut out = gems::tooltip(st.sockets);
        if let Some(g) = Gem::from_name(&self.items.get(st.item).name) {
            let (s, v) = g.bonus();
            out.push((format!("Engastada: {}", s.line(v)), gems::GEM_COLORS[g.kind as usize]));
            out.push(("Botão dir. sobre um item com engaste".to_string(), [150, 150, 150, 255]));
        }
        out
    }

    pub(crate) fn apoth_info_line(&self) -> String {
        format!("Nível do mundo: {} ({}/{})", gems::TIER_NAMES[self.world_tier()], self.world_tier() + 1, gems::TIERS)
    }

    // -----------------------------------------------------------------------------------------
    // The tables' screens: slots in the top half of the inventory panel and one button.

    pub(crate) fn draw_apoth_table(&mut self, p: IVec3, x0: f32, y0: f32, mouse: (f32, f32)) -> Vec<(Slot, f32, f32)> {
        let s = self.ui.scale;
        let dark = [64, 64, 64, 255];
        let Some(t) = self.apoth_table(p) else { return Vec::new() };
        let get = |g: &Game, i: usize| g.slot_get(Slot::Cont(i));
        let (title, slots, action, hint) = match t {
            Table::Salvaging => {
                let (label, hint) = match get(self, 0) {
                    None => (None, "Ponha um equipamento ou gemas".to_string()),
                    Some(st) => self.salvage_preview(&st),
                };
                ("Mesa de Desmonte", vec![(Slot::Cont(0), 48.0, 30.0)], label, hint)
            }
            Table::GemCutting => {
                let (label, hint) = self.cutting_preview();
                ("Mesa de Lapidação", vec![(Slot::Cont(0), 12.0, 30.0), (Slot::Cont(1), 48.0, 30.0), (Slot::Cont(2), 84.0, 30.0)], label, hint)
            }
        };
        self.ui.label(x0 + 8.0 * s, y0 + 6.0 * s, title, dark, s);
        if t == Table::GemCutting {
            for (x, t) in [(12.0, "Item"), (48.0, "Gema"), (84.0, "Pó")] {
                let tw = self.ui.text_width(t, s);
                self.ui.label(x0 + (x + 8.0) * s - tw / 2.0, y0 + 19.0 * s, t, dark, s);
            }
        }
        let (bx, by, bw, bh) = (x0 + 110.0 * s, y0 + 28.0 * s, 58.0 * s, 20.0 * s);
        match action {
            Some(label) => {
                let over = self.ui.button(bx, by, bw, bh, label, mouse);
                if over && self.clicked {
                    match t {
                        Table::Salvaging => self.salvage(p),
                        Table::GemCutting => self.cut_gems(),
                    }
                }
            }
            None => {
                self.ui.rect(bx, by, bw, bh, [90, 90, 90, 255]);
                self.ui.text_centered(bx + bw / 2.0, by + 6.0 * s, "-", [150, 150, 150, 255]);
            }
        }
        // The hint, wrapped to the panel's width.
        let hs = s;
        let mut line = String::new();
        let mut y = y0 + 54.0 * s;
        for word in hint.split(' ') {
            let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if self.ui.text_width(&next, hs) > 160.0 * s && !line.is_empty() {
                self.ui.label(x0 + 8.0 * s, y, &line, dark, hs);
                y += 9.0 * hs;
                line = word.to_string();
            } else {
                line = next;
            }
        }
        self.ui.label(x0 + 8.0 * s, y, &line, dark, hs);
        slots.into_iter().map(|(sl, x, y)| (sl, x0 + x * s, y0 + y * s)).collect()
    }

    /// The salvaging button's label and what it would give.
    fn salvage_preview(&self, st: &ItemStack) -> (Option<&'static str>, String) {
        let it = self.items.get(st.item);
        if let Some(g) = Gem::from_name(&it.name) {
            return (Some("Moer"), format!("Rende {}x Pó de Gema", dust_of(g) * st.count as u32));
        }
        if it.tool.is_none() && it.armor.is_none() {
            return (None, "Isso não pode ser desmontado".to_string());
        }
        let mat = |r: usize| self.items.id(gems::MATERIALS[r]).map(|i| self.items.get(i).display_name.clone()).unwrap_or_default();
        let mut text = match gems::rarity(st.sockets) {
            Some(r) => format!("Rende 1-3x {}", mat(r)),
            None => format!("Item comum: 50% de 1x {}", mat(0)),
        };
        let n = gems::gems(st.sockets).len();
        if n > 0 {
            text.push_str(&format!(" e {n} gema(s)"));
        }
        (Some("Desmontar"), text)
    }

    fn salvage(&mut self, p: IVec3) {
        let Some(st) = self.slot_get(Slot::Cont(0)) else { return };
        let name = self.items.get(st.item).name.clone();
        let mut out = Vec::new();
        if let Some(g) = Gem::from_name(&name) {
            if let Some(id) = self.items.id("gem_dust") {
                out.push(ItemStack::new(id, (dust_of(g) * st.count as u32).min(64) as u8));
            }
        } else {
            if let Some((r, n)) = gems::salvage_yield(st.sockets, &mut self.mods.apoth.rng) {
                if let Some(id) = self.items.id(gems::MATERIALS[r]) {
                    out.push(ItemStack::new(id, n));
                }
            }
            for g in gems::gems(st.sockets) {
                if let Some(id) = self.items.id(&g.item_name()) {
                    out.push(ItemStack::new(id, 1));
                }
            }
        }
        self.slot_set(Slot::Cont(0), None);
        let got = out.iter().map(|o| format!("{}x {}", o.count, self.items.get(o.item).display_name)).collect::<Vec<_>>().join(", ");
        self.give_all(out);
        self.sfx(Sfx::Break(Material::Metal));
        let at = p.as_vec3() + Vec3::new(0.5, 1.1, 0.5);
        let (layer, tint) = self.block_look(self.reg.id_or_air("iron_block"));
        self.particles.block_bits(at.floor().as_ivec3(), layer, tint, 8, 0.5);
        self.toast(if got.is_empty() { "Não sobrou nada".to_string() } else { format!("Desmontado: {got}") }, 3.0);
    }

    /// What the gem cutting table would do with what is in it.
    fn cutting_preview(&self) -> (Option<&'static str>, String) {
        let gear = self.slot_get(Slot::Cont(0));
        let mid = self.slot_get(Slot::Cont(1));
        let dust = self.slot_get(Slot::Cont(2)).filter(|d| self.items.get(d.item).name == "gem_dust");
        let mid_name = mid.map(|m| self.items.get(m.item).name.clone()).unwrap_or_default();
        let gem = Gem::from_name(&mid_name);
        if let Some(gear) = gear {
            let it = self.items.get(gear.item);
            if it.tool.is_none() && it.armor.is_none() {
                return (None, "O primeiro espaço é para equipamento".to_string());
            }
            if gem.is_some() {
                return match gems::free_socket(gear.sockets) {
                    Some(_) => (Some("Engastar"), "Engasta a gema no item".to_string()),
                    None if gems::sockets(gear.sockets) == 0 => (None, "Este item não tem engastes".to_string()),
                    None => (None, "Os engastes estão cheios".to_string()),
                };
            }
            if mid_name == "sigil_socketing" {
                return match gems::with_extra_socket(gear.sockets) {
                    Some(_) => (Some("Abrir"), "Abre mais um engaste".to_string()),
                    None => (None, "Não cabe mais nenhum engaste".to_string()),
                };
            }
            if mid_name == "sigil_withdrawal" {
                return if gems::gems(gear.sockets).is_empty() {
                    (None, "Não há gemas para tirar".to_string())
                } else {
                    (Some("Extrair"), "Tira as gemas inteiras".to_string())
                };
            }
            return (None, "Ponha uma gema ou um sigilo".to_string());
        }
        if let (Some(g), Some(m)) = (gem, mid) {
            if g.cut >= 2 {
                return (None, "Esta gema já é perfeita".to_string());
            }
            if m.count < 3 || dust.is_none() {
                return (None, "Lapidar: 3 gemas iguais + 1 Pó de Gema".to_string());
            }
            return (Some("Lapidar"), format!("Faz 1 {}", Gem { cut: g.cut + 1, ..g }.display_name()));
        }
        (None, "Item + gema, ou 3 gemas + pó".to_string())
    }

    fn cut_gems(&mut self) {
        let (Some(label), _) = self.cutting_preview() else { return };
        let mid = self.slot_get(Slot::Cont(1)).unwrap();
        let take_mid = |g: &mut Game, n: u8| g.slot_set(Slot::Cont(1), (mid.count > n).then_some(ItemStack { count: mid.count - n, ..mid }));
        let mid_name = self.items.get(mid.item).name.clone();
        match label {
            "Lapidar" => {
                let g = Gem::from_name(&mid_name).unwrap();
                take_mid(self, 3);
                if let Some(d) = self.slot_get(Slot::Cont(2)) {
                    self.slot_set(Slot::Cont(2), (d.count > 1).then_some(ItemStack { count: d.count - 1, ..d }));
                }
                if let Some(id) = self.items.id(&Gem { cut: g.cut + 1, ..g }.item_name()) {
                    self.give_all(vec![ItemStack::new(id, 1)]);
                }
            }
            _ => {
                let gear = self.slot_get(Slot::Cont(0)).unwrap();
                let mut s = gear.sockets;
                match label {
                    "Engastar" => {
                        let g = Gem::from_name(&mid_name).unwrap();
                        s = gems::set_gem(s, gems::free_socket(s).unwrap(), Some(g));
                    }
                    "Abrir" => s = gems::with_extra_socket(s).unwrap(),
                    _ => {
                        let back = gems::gems(s).into_iter().filter_map(|g| self.items.id(&g.item_name())).map(|id| ItemStack::new(id, 1)).collect();
                        self.give_all(back);
                        s = gems::without_gems(s);
                    }
                }
                take_mid(self, 1);
                self.slot_set(Slot::Cont(0), Some(ItemStack { sockets: s, ..gear }));
            }
        }
        self.sfx(Sfx::Place(Material::Glass));
    }

    /// Into the player's inventory, or on the ground when it is full.
    fn give_all(&mut self, list: Vec<ItemStack>) {
        for st in list {
            if let Some(left) = self.surv.inv.add(&self.items, st) {
                let at = self.body.eye();
                self.spawn_drop(at, left, Vec3::Y);
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // HUD: elite names over their heads, and the tier panel.

    pub(crate) fn draw_apoth_hud(&mut self, w: f32, h: f32) {
        if let Some((vp, cam_block)) = self.ui_view_proj {
            let s = self.ui.scale;
            let eye = self.body.eye();
            let mut labels = Vec::new();
            for m in self.mobs.list.iter().filter(|m| m.alive() && (2..DONE).contains(&m.elite)) {
                let def = &self.mobs.reg.defs[m.def];
                // Above the health bar.
                let top = m.body.pos + Vec3::Y * (def.height + 0.95);
                let dist = (top - eye).length();
                if dist > 32.0 || !self.in_sight(&[m.body.pos + Vec3::Y * def.height * 0.9, m.body.center()]) {
                    continue;
                }
                let c = vp * (top - cam_block.as_vec3()).extend(1.0);
                if c.w < 0.1 || (c.x / c.w).abs() > 1.1 || (c.y / c.w).abs() > 1.1 {
                    continue;
                }
                let r = (m.elite - 2) as usize;
                let (x, y) = ((c.x / c.w * 0.5 + 0.5) * w, (0.5 - c.y / c.w * 0.5) * h);
                // Smaller further away, like the health bars.
                let sc = (s * 7.0 / dist.max(1.0)).round().clamp(1.0, s);
                labels.push((x, y, sc, format!("{} Elite", def.display_name), gems::RARITY_COLORS[r]));
            }
            for (x, y, sc, t, c) in labels {
                let tw = self.ui.text_width(&t, sc);
                self.ui.rect(x - tw / 2.0 - 2.0 * sc, y - 2.0 * sc, tw + 4.0 * sc, 11.0 * sc, [0, 0, 0, 120]);
                self.ui.text_scaled(x - tw / 2.0, y, &t, c, sc);
            }
        }
        if self.now < self.mods.apoth.panel_until && self.screen == Screen::Playing {
            let s = self.ui.scale;
            let widest = gems::TIER_NAMES.iter().map(|n| self.ui.text_width(&format!("> {n} (bloqueado)"), s)).fold(0.0, f32::max);
            let (pw, ph) = (widest + 20.0 * s, (24.0 + 11.0 * gems::TIERS as f32) * s);
            let (x, y) = ((w - pw) / 2.0, 40.0 * s);
            self.ui.rect(x, y, pw, ph, [16, 8, 24, 220]);
            self.ui.frame(x, y, pw, ph, s, [120, 80, 170, 255]);
            self.ui.text_centered(w / 2.0, y + 4.0 * s, "Nível do mundo", WHITE);
            let unlocked = self.mods.save.tier_unlocked as usize;
            for t in 0..gems::TIERS {
                let col = if t == self.world_tier() {
                    gems::RARITY_COLORS[t]
                } else if t <= unlocked {
                    [200, 200, 200, 255]
                } else {
                    [100, 100, 100, 255]
                };
                let mark = if t == self.world_tier() { "> " } else { "  " };
                let lock = if t > unlocked { " (bloqueado)" } else { "" };
                self.ui.text(x + 10.0 * s, y + (16.0 + 11.0 * t as f32) * s, &format!("{mark}{}{lock}", gems::TIER_NAMES[t]), col);
            }
            self.ui.text_centered(w / 2.0, y + ph + 2.0 * s, "Ctrl+T: trocar", [200, 200, 200, 255]);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Test pictures (MC_APOTH=tooltip|salvage|cutting|elite|tier).

    pub(crate) fn apoth_test_hook(&mut self) {
        let Ok(what) = std::env::var("MC_APOTH") else { return };
        let id = |g: &Game, n: &str| g.items.id(n).unwrap_or(1);
        // A Mythic diamond sword with two gems, ranked armour and some gems in the bag.
        let mut sword = ItemStack { sockets: gems::ranked(4), ..ItemStack::new(id(self, "diamond_sword"), 1) };
        sword.sockets = gems::set_gem(sword.sockets, 0, Some(Gem { kind: 2, cut: 2 }));
        sword.sockets = gems::set_gem(sword.sockets, 1, Some(Gem { kind: 4, cut: 1 }));
        // Gear in the top row (room for its tooltip below), gems and the rest in the hotbar.
        self.surv.inv.slots[9] = Some(sword);
        self.surv.inv.slots[10] = Some(ItemStack { sockets: gems::ranked(2), ..ItemStack::new(id(self, "iron_pickaxe"), 1) });
        self.surv.inv.slots[11] = Some(ItemStack { sockets: gems::ranked(3), ..ItemStack::new(id(self, "iron_chestplate"), 1) });
        self.surv.inv.slots[12] = Some(ItemStack { sockets: gems::ranked(1), ..ItemStack::new(id(self, "golden_axe"), 1) });
        let rest = [
            ("gem_vital_rough", 4), ("gem_bastion_cut", 2), ("gem_swift_flawless", 1), ("gem_fury_rough", 3), ("gem_shadow_cut", 1),
            ("gem_dust", 6), ("salvage_common", 7), ("salvage_uncommon", 4), ("salvage_rare", 3), ("salvage_epic", 2), ("salvage_mythic", 1),
            ("sigil_frontier", 1), ("sigil_ascent", 1), ("sigil_socketing", 1), ("sigil_withdrawal", 1),
        ];
        for (i, (n, c)) in rest.iter().enumerate() {
            let slot = if i < 9 { i } else { 18 + i };
            self.surv.inv.slots[slot] = Some(ItemStack::new(id(self, n), *c));
        }
        match what.as_str() {
            "salvage" | "cutting" => {
                let p = self.body.pos.floor().as_ivec3() - IVec3::Y * 3;
                let table = if what == "salvage" { "salvaging_table" } else { "gem_cutting_table" };
                // The table's screen needs the block: one goes in the ground below.
                self.world.set_block(p, self.reg.id_or_air(table));
                let mut inv = Inventory::new(if what == "salvage" { 1 } else { 3 });
                if what == "salvage" {
                    inv.slots[0] = Some(ItemStack { sockets: gems::set_gem(gems::ranked(3), 0, Some(Gem { kind: 0, cut: 1 })), ..ItemStack::new(id(self, "golden_helmet"), 1) });
                } else {
                    inv.slots[0] = Some(ItemStack { sockets: gems::set_gem(gems::ranked(4), 0, Some(Gem { kind: 1, cut: 0 })), ..ItemStack::new(id(self, "diamond_chestplate"), 1) });
                    inv.slots[1] = Some(ItemStack::new(id(self, "gem_fury_cut"), 3));
                    inv.slots[2] = Some(ItemStack::new(id(self, "gem_dust"), 2));
                }
                self.surv.containers.insert(p, Container::Chest(inv));
                self.surv.open = Open::Chest(p);
            }
            "elite" => {
                self.mods.save.world_tier = 3;
                self.mods.save.tier_unlocked = 3;
                let f = self.forward();
                let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
                let side = Vec3::new(-flat.z, 0.0, flat.x);
                for (k, (mob, r)) in [("zombie", 4u8), ("skeleton", 2), ("spider", 3)].iter().enumerate() {
                    let at = self.body.pos + flat * 8.0 + side * (k as f32 - 1.0) * 5.0;
                    // On the ground the player stands on (the surface may be a tree top).
                    let y = self.body.pos.y + 0.5;
                    if self.mobs.spawn(mob, Vec3::new(at.x, y, at.z)) {
                        let m = self.mobs.list.last_mut().unwrap();
                        m.elite = 2 + r;
                        m.yaw = (-flat.x).atan2(-flat.z);
                    }
                }
                self.mods.apoth.glow_t = 0.0;
                for _ in 0..6 {
                    self.mods.apoth.glow_t = 0.0;
                    self.update_apotheosis(0.05);
                    self.particles.update(&self.world, 0.05);
                }
            }
            "tier" => {
                self.mods.save.world_tier = 2;
                self.mods.save.tier_unlocked = 3;
                self.mods.apoth.panel_until = self.now + 100.0;
            }
            _ => {}
        }
    }
}

/// Gem dust from grinding a gem: more for better cuts.
fn dust_of(g: Gem) -> u32 {
    [1, 2, 4][g.cut as usize]
}
