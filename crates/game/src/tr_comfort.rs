//! Lote 12 of the Terraria side, second round (plan/terraria-completo.md): comfort at home.
//!
//! - Near a Heart Lantern life comes back faster, near a Star in a Bottle mana does.
//! - The Water Candle and the Battle Potion bring more monsters; the Peace Candle and the
//!   Calming Potion fewer.
//! - Stations give a buff for ten minutes on right click: the Crystal Ball (magic), the
//!   Sharpening Stone (melee), the Ammo Box (saves ammo), the War Table and the Bewitching
//!   Table (one more minion each).
//! - The Enchanted Sundial runs the day fast up to the next morning, once a day.
//! - Every 50 of a kind of monster killed leaves a Hunting Banner; with any banner hung within
//!   48 blocks, those kinds take half again as much damage.
//! - Dying leaves a tombstone where it happened.
//! - In the inventory, O sorts the backpack and X puts what's in it into chests nearby that
//!   already hold that item.

use glam::{IVec3, Vec3};
use voxel::item::{Inventory, ItemStack};

use crate::audio::{Material, Sfx};
use crate::effects::{AMMO_BOX, BATTLE, BEWITCHED, CALM, CLAIRVOYANCE, HEART_LAMP, HUNTING, SHARPENED, STAR_BOTTLE, STRATEGIST};
use crate::rpg::{Stats, MAGIC, MELEE};
use crate::survival::{Container, INV_SIZE};
use crate::Game;

/// Blocks that work by being near: (name, reach, effect).
const NEAR: &[(&str, f32, u8)] = &[("heart_lantern", 12.0, HEART_LAMP), ("star_bottle", 12.0, STAR_BOTTLE), ("water_candle", 24.0, BATTLE), ("peace_candle", 24.0, CALM), ("hunt_banner", 48.0, HUNTING)];

/// Stations clicked for a buff: (name, effect, what it says).
const STATIONS: &[(&str, u8, &str)] = &[
    ("crystal_ball", CLAIRVOYANCE, "Clarividência: magia mais forte e mais mana"),
    ("sharpening_station", SHARPENED, "Afiado: armas de perto batem mais forte"),
    ("ammo_box", AMMO_BOX, "Caixa de Munição: 20% de chance de não gastar munição"),
    ("war_table", STRATEGIST, "Estrategista: um lacaio a mais"),
    ("bewitching_table", BEWITCHED, "Enfeitiçado: um lacaio a mais"),
];
const STATION_SECS: f32 = 600.0;

/// Kills of one kind for each Hunting Banner.
pub const BANNER_KILLS: u32 = 50;

#[derive(Default)]
pub struct Comfort {
    /// The near-blocks placed in this world: where, and which (index in `NEAR`).
    pub placed: Vec<(IVec3, u8)>,
    /// Kinds of monster (by def) that take more damage while a banner is near.
    pub hunt: Vec<bool>,
    tick: f32,
    /// The sundial is running the day fast.
    sundial: bool,
    sundial_cd: f32,
    /// Where the player last stood alive, and whether the death was seen.
    alive_at: Vec3,
    dead: bool,
    grave: Option<IVec3>,
}

pub(crate) fn is_station(name: &str) -> bool {
    name == "enchanted_sundial" || STATIONS.iter().any(|s| s.0 == name)
}

impl Game {
    pub(crate) fn update_tr_comfort(&mut self, dt: f32) {
        self.comfort_grave();
        if self.t5.tr.comfort.sundial {
            // About 25 seconds for a whole night.
            let t = self.time_of_day;
            let left = (24000.0 - t) % 24000.0;
            let step = (dt * 600.0).min(left);
            self.time_of_day = (t + step) % 24000.0;
            if left - step < 1.0 {
                self.t5.tr.comfort.sundial = false;
                self.toast("Bom dia!", 2.0);
            }
        }
        let c = &mut self.t5.tr.comfort;
        c.sundial_cd = (c.sundial_cd - dt).max(0.0);
        c.tick -= dt;
        if c.tick > 0.0 || dt <= 0.0 {
            return;
        }
        c.tick = 1.0;
        let me = self.body.pos;
        let ids: Vec<u16> = NEAR.iter().map(|n| self.reg.id_or_air(n.0)).collect();
        // Forget the ones broken (only near the player, where the chunks are in).
        let world = &self.world;
        self.t5.tr.comfort.placed.retain(|&(p, k)| p.as_vec3().distance(me) > 64.0 || world.block_or_air(p) == ids[k as usize]);
        let mut near = [false; 5];
        for &(p, k) in &self.t5.tr.comfort.placed {
            let (_, reach, _) = NEAR[k as usize];
            if (p.as_vec3() + Vec3::splat(0.5)).distance(me) <= reach {
                near[k as usize] = true;
            }
        }
        let mut hunt = Vec::new();
        if near[4] {
            let kills = &self.t5.tr.modes.kills;
            hunt = self.mobs.reg.defs.iter().map(|d| kills.get(&d.name).is_some_and(|&n| n >= BANNER_KILLS)).collect();
            if !hunt.iter().any(|&h| h) {
                near[4] = false;
            }
        }
        self.t5.tr.comfort.hunt = if near[4] { hunt } else { Vec::new() };
        for (k, &on) in near.iter().enumerate() {
            if on {
                self.add_effect(NEAR[k].2, 0, 2.5, true);
            }
        }
        let mut mult = 1.0;
        if self.has_effect(BATTLE) {
            mult *= 2.0;
        }
        if self.has_effect(CALM) {
            mult *= 0.4;
        }
        self.mobs.spawn_mult = mult;
    }

    /// The buffs' share of the stats (called from `compute_stats`).
    pub(crate) fn comfort_stats(&self, s: &mut Stats) {
        if self.has_effect(HEART_LAMP) {
            s.regen += 0.2;
        }
        if self.has_effect(STAR_BOTTLE) {
            s.mana_regen += 2.0;
        }
        if self.has_effect(CLAIRVOYANCE) {
            s.dmg[MAGIC] += 0.05;
            s.apply(&crate::rpg::M::Mana(2.0));
        }
        if self.has_effect(SHARPENED) {
            s.dmg[MELEE] += 0.1;
        }
        if self.has_effect(AMMO_BOX) {
            s.ammo_save += 0.2;
        }
        if self.has_effect(STRATEGIST) {
            s.minions += 1;
        }
        if self.has_effect(BEWITCHED) {
            s.minions += 1;
        }
    }

    /// A block went down: remember the ones that work by being near.
    pub(crate) fn comfort_placed(&mut self, p: IVec3, name: &str) {
        if let Some(k) = NEAR.iter().position(|n| n.0 == name) {
            let placed = &mut self.t5.tr.comfort.placed;
            placed.retain(|q| q.0 != p);
            placed.push((p, k as u8));
            self.t5.tr.comfort.tick = 0.0;
            if name == "hunt_banner" && !self.t5.tr.modes.kills.values().any(|&n| n >= BANNER_KILLS) {
                self.toast("Nenhuma caça completa ainda: abata 50 de um mesmo tipo de monstro", 3.0);
            }
        }
    }

    /// Right click on a buff station or the sundial.
    pub(crate) fn use_comfort_station(&mut self, name: &str) -> bool {
        if name == "enchanted_sundial" {
            let c = &mut self.t5.tr.comfort;
            if c.sundial {
                return true;
            }
            if c.sundial_cd > 0.0 {
                let min = (c.sundial_cd / 60.0).ceil() as u32;
                self.toast(format!("O relógio de sol ainda está recarregando ({min} min)"), 2.5);
                return true;
            }
            c.sundial = true;
            c.sundial_cd = 1200.0;
            self.toast("O relógio de sol gira: o tempo corre até a manhã", 3.0);
            self.sfx(Sfx::Cast(3));
            return true;
        }
        let Some(&(_, effect, text)) = STATIONS.iter().find(|s| s.0 == name) else { return false };
        self.add_effect(effect, 0, STATION_SECS, false);
        self.toast(format!("{text} (10 min)"), 3.0);
        self.sfx(Sfx::Place(if name == "crystal_ball" { Material::Glass } else { Material::Wood }));
        true
    }

    pub(crate) fn tr_comfort_use(&mut self, name: &str) -> Option<bool> {
        let (effect, secs, text) = match name {
            "battle_potion" => (BATTLE, 420.0, "Batalha: muito mais monstros por 7 minutos"),
            "calming_potion" => (CALM, 300.0, "Calmaria: menos monstros por 5 minutos"),
            _ => return None,
        };
        self.add_effect(effect, 0, secs, false);
        self.toast(text, 3.0);
        self.sfx(Sfx::Eat);
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        Some(true)
    }

    /// Every 50th of a kind killed leaves a Hunting Banner (after `modes_deaths` counted them).
    pub(crate) fn comfort_deaths(&mut self, deaths: &[(usize, Vec3)]) {
        let Some(banner) = self.items.id("hunt_banner") else { return };
        let mut seen: Vec<usize> = Vec::new();
        for &(def, pos) in deaths {
            if seen.contains(&def) {
                continue;
            }
            seen.push(def);
            let d = &self.mobs.reg.defs[def];
            let Some(&now) = self.t5.tr.modes.kills.get(&d.name) else { continue };
            let n = deaths.iter().filter(|x| x.0 == def).count() as u32;
            if now / BANNER_KILLS > now.saturating_sub(n) / BANNER_KILLS {
                let text = format!("Estandarte de Caça: {} ({} abatidos)", d.display_name, now / BANNER_KILLS * BANNER_KILLS);
                self.spawn_drop(pos + Vec3::Y * 0.5, ItemStack::new(banner, 1), Vec3::Y * 2.0);
                self.toast(text, 3.5);
                self.sfx(Sfx::Xp);
            }
        }
    }

    /// Dying leaves a tombstone; after coming back, where it is.
    fn comfort_grave(&mut self) {
        let dead = self.screen == crate::Screen::Dead || self.health <= 0.0;
        let c = &mut self.t5.tr.comfort;
        if !dead {
            if c.dead {
                c.dead = false;
                if let Some(g) = c.grave.take() {
                    self.toast(format!("Sua lápide ficou em {} {} {}", g.x, g.y, g.z), 5.0);
                }
            }
            self.t5.tr.comfort.alive_at = self.body.pos;
            return;
        }
        if c.dead || !self.survival {
            return;
        }
        c.dead = true;
        let at = c.alive_at.floor().as_ivec3();
        let stone = self.reg.id_or_air("tombstone");
        if stone == voxel::AIR {
            return;
        }
        for dy in 0..4 {
            let p = at + IVec3::Y * dy;
            if self.world.block_or_air(p) == voxel::AIR && self.world.block_or_air(p - IVec3::Y) != voxel::AIR {
                self.world.set_block(p, stone);
                self.t5.tr.comfort.grave = Some(p);
                break;
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // The inventory: sort, and put things away in chests nearby.

    /// O in the inventory: the backpack (not the hotbar) merged and sorted.
    pub(crate) fn sort_backpack(&mut self) {
        let mut all: Vec<ItemStack> = self.surv.inv.slots[9..INV_SIZE].iter_mut().filter_map(|s| s.take()).collect();
        let mut merged = Inventory::new(INV_SIZE - 9);
        let mut left = Vec::new();
        for st in all.drain(..) {
            if let Some(rest) = merged.add(&self.items, st) {
                left.push(rest);
            }
        }
        let items = &self.items;
        let key = |s: &ItemStack| {
            let it = items.get(s.item);
            let group = if it.tool.is_some() {
                0
            } else if it.armor.is_some() {
                1
            } else if it.places.is_some() {
                3
            } else {
                2
            };
            (group, it.name.clone(), std::cmp::Reverse(s.count))
        };
        let mut stacks: Vec<ItemStack> = merged.slots.into_iter().flatten().collect();
        stacks.sort_by_key(|s| key(s));
        for (k, st) in stacks.into_iter().chain(left).enumerate() {
            self.surv.inv.slots[9 + k] = Some(st);
        }
        self.sfx(Sfx::Click);
        self.toast("Mochila organizada", 1.5);
    }

    /// X in the inventory: whatever in the backpack a chest within 8 blocks already holds goes
    /// into that chest.
    pub(crate) fn quick_stack(&mut self) {
        let me = self.body.pos;
        let chests: Vec<IVec3> = self.surv.containers.iter().filter(|(p, c)| matches!(c, Container::Chest(_)) && (p.as_vec3() + Vec3::splat(0.5)).distance(me) <= 8.0).map(|(p, _)| *p).collect();
        let mut moved = 0;
        for p in chests {
            let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&p) else { continue };
            for k in 9..INV_SIZE {
                let Some(st) = self.surv.inv.slots[k] else { continue };
                if !inv.slots.iter().flatten().any(|s| s.stacks_with(&st)) {
                    continue;
                }
                let n = inv.slots.len();
                let rest = inv.add_in(&self.items, st, 0..n);
                if rest.map(|r| r.count) != Some(st.count) {
                    moved += 1;
                }
                self.surv.inv.slots[k] = rest;
            }
        }
        if moved > 0 {
            self.toast(format!("{moved} pilha(s) guardadas nos baús perto"), 1.5);
            self.sfx(Sfx::Place(Material::Wood));
        } else {
            self.toast("Nenhum baú perto tem esses itens", 1.5);
        }
    }

    /// Test scene: MC_TR=comfort, the new blocks around the player, a banner earned.
    pub(crate) fn test_tr_comfort(&mut self) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-flat.z, 0.0, flat.x);
        let names = ["heart_lantern", "crystal_ball", "sharpening_station", "ammo_box", "war_table", "bewitching_table", "enchanted_sundial", "star_bottle", "water_candle", "hunt_banner"];
        let base = self.body.pos + flat * 4.0;
        for (k, n) in names.iter().enumerate() {
            let p = (base + side * ((k as f32 - 4.5) * 1.2)).floor().as_ivec3();
            let p = IVec3::new(p.x, self.body.pos.y.floor() as i32, p.z);
            let id = self.reg.id_or_air(n);
            self.world.set_block(p, id);
            self.comfort_placed(p, n);
        }
        if let Some(d) = self.mobs.reg.defs.iter().find(|d| d.name == "zombie") {
            self.t5.tr.modes.kills.insert(d.name.clone(), BANNER_KILLS);
        }
        for (n, _, _) in STATIONS {
            self.use_comfort_station(n);
        }
        self.t5.tr.comfort.tick = 0.0;
        self.update_tr_comfort(0.1);
    }
}
