//! Villages (vanilla idea, our own folk): the settlers living round a village square and its
//! sentinel. Each settler has a trade: right click to open their shop, or hold out the goods
//! they want (a stack of wheat, coal, paper or wool) to be paid in emeralds. The sentinel, a
//! big stone-and-iron keeper, fights the monsters that come near. Settlers who die come back
//! after a while, as new folk move in.

use std::collections::HashMap;

use glam::{IVec2, Vec3};
use voxel::block::Shape;
use voxel::item::{Inventory, ItemId, ItemStack};
use voxel::worldgen::Dim;

use crate::audio::Sfx;
use crate::survival::Open;
use crate::Game;

/// Who lives in a village: (mob, shop goods as (item, count, price in emeralds), what they
/// buy as (item, how many for one emerald)).
type Trade = (&'static str, &'static [(&'static str, u8, u8)], (&'static str, u32));
const SETTLERS: [Trade; 4] = [
    ("settler_farmer", &[("bread", 6, 1), ("apple", 4, 1), ("carrot", 6, 1), ("pumpkin", 2, 1), ("melon_slice", 8, 1), ("wheat_seeds", 8, 1)], ("wheat", 20)),
    ("settler_smith", &[("iron_pickaxe", 1, 4), ("iron_sword", 1, 4), ("iron_axe", 1, 3), ("iron_chestplate", 1, 8), ("iron_helmet", 1, 5), ("bucket", 1, 2), ("iron_leggings", 1, 6)], ("coal", 15)),
    ("settler_librarian", &[("bookshelf", 1, 3), ("compass", 1, 4), ("map", 1, 3), ("clock", 1, 4), ("lantern", 2, 1), ("glass", 8, 1), ("name_tag", 1, 5)], ("paper", 24)),
    ("settler_shepherd", &[("white_wool", 4, 1), ("string", 6, 1), ("red_wool", 2, 1), ("blue_wool", 2, 1), ("yellow_wool", 2, 1), ("white_carpet", 6, 1)], ("white_wool", 18)),
];
const SENTINEL: &str = "sentinel";
/// Seconds before a village fills a gap left by a settler or sentinel who died.
const REFILL: f32 = 180.0;

#[derive(Default)]
pub struct Villages {
    check_t: f32,
    /// When each village (by square) was last peopled.
    peopled: HashMap<IVec2, f32>,
    clock: f32,
}

impl Game {
    pub(crate) fn update_villages(&mut self, dt: f32) {
        if dt == 0.0 || self.dim != Dim::Overworld {
            return;
        }
        self.villages.clock += dt;
        self.villages.check_t -= dt;
        if self.villages.check_t > 0.0 {
            return;
        }
        self.villages.check_t = 4.0;
        let p = self.body.pos;
        let near = self.world.gen.villages_near(p.x as i32, p.z as i32, 80);
        for c in near {
            let cv = c.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
            // Close enough that the square is loaded, but not so close the folk pop in under your nose.
            let d = Vec3::new(cv.x - p.x, 0.0, cv.z - p.z).length();
            if d > 64.0 {
                continue;
            }
            let key = IVec2::new(c.x, c.z);
            let now = self.villages.clock;
            if self.villages.peopled.get(&key).is_some_and(|&t| now - t < REFILL) && d < 40.0 {
                continue;
            }
            let have = |g: &Game, name: &str| {
                g.mobs.list.iter().any(|m| m.alive() && g.mobs.reg.defs[m.def].name == name && m.home.is_some_and(|h| (h - cv).length() < 40.0))
            };
            let mut wanted: Vec<&str> = SETTLERS.iter().map(|s| s.0).filter(|n| !have(self, n)).collect();
            if !have(self, SENTINEL) {
                wanted.push(SENTINEL);
            }
            if wanted.is_empty() {
                self.villages.peopled.insert(key, now);
                continue;
            }
            if self.villages.peopled.get(&key).is_some_and(|&t| now - t < REFILL) {
                continue;
            }
            self.villages.peopled.insert(key, now);
            for (k, name) in wanted.into_iter().enumerate() {
                let a = self.surv.rng.f32() * std::f32::consts::TAU;
                let r = if name == SENTINEL { 7.0 } else { 6.0 + k as f32 * 2.0 };
                let (x, z) = ((cv.x + a.cos() * r).floor() as i32, (cv.z + a.sin() * r).floor() as i32);
                let y = self.world.gen.height_at(x, z) + 1;
                let at = Vec3::new(x as f32 + 0.5, y as f32 + 0.1, z as f32 + 0.5);
                if self.mobs.spawn(name, at) {
                    if let Some(m) = self.mobs.list.last_mut() {
                        m.home = Some(at);
                    }
                }
            }
        }
    }

    fn count_item(&self, id: ItemId) -> u32 {
        self.surv.inv.slots.iter().flatten().filter(|s| s.item == id).map(|s| s.count as u32).sum()
    }

    fn take_item(&mut self, id: ItemId, mut n: u32) {
        for k in 0..self.surv.inv.slots.len() {
            if n == 0 {
                break;
            }
            if let Some(st) = self.surv.inv.slots[k].filter(|s| s.item == id) {
                let take = (st.count as u32).min(n);
                self.surv.inv.take(k, take as u8);
                n -= take;
            }
        }
    }

    /// Right click on a settler: sell them what they want, or open their shop.
    pub(crate) fn village_use(&mut self) -> bool {
        let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), 4.0) else { return false };
        let block_dist = self.target.filter(|h| self.reg.get(self.world.block_or_air(h.block)).shape != Shape::Cross).map(|h| h.distance).unwrap_or(f32::MAX);
        if t > block_dist {
            return false;
        }
        let def = &self.mobs.reg.defs[self.mobs.list[i].def];
        let (name, dn) = (def.name.clone(), def.display_name.clone());
        if name == SENTINEL {
            self.toast(format!("{dn}: ..."), 2.0);
            return true;
        }
        let Some(&(_, offers, (want, per))) = SETTLERS.iter().find(|s| s.0 == name) else { return false };
        let to = self.body.pos - self.mobs.list[i].body.pos;
        self.mobs.list[i].yaw = to.x.atan2(to.z);
        // Holding what they buy: a stack's worth for an emerald.
        if self.held_name().as_deref() == Some(want) {
            let (Some(wid), Some(em)) = (self.items.id(want), self.items.id("emerald")) else { return true };
            let have = self.count_item(wid);
            if have < per {
                self.toast(format!("{dn}: Pago 1 esmeralda por {per}. Você só tem {have}."), 3.5);
                return true;
            }
            let n = (have / per).min(16);
            self.take_item(wid, n * per);
            if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(em, n as u8)) {
                self.pop_drop(self.body.pos + Vec3::Y, left);
            }
            self.toast(format!("{dn}: Negócio fechado! (+{n} esmeralda(s))"), 3.0);
            self.sfx(Sfx::Xp);
            return true;
        }
        let mut inv = Inventory::new(offers.len());
        let mut prices = Vec::new();
        let mut k = 0;
        for &(item, count, price) in offers {
            if let Some(id) = self.items.id(item) {
                inv.slots[k] = Some(ItemStack::new(id, count));
                prices.push(price);
                k += 1;
            }
        }
        self.surv.shop = inv;
        self.surv.shop_prices = prices;
        self.surv.shop_name = dn.clone();
        self.surv.open = Open::Shop;
        self.set_screen(crate::Screen::Inventory);
        let wname = self.items.id(want).map(|id| self.items.get(id).display_name.clone()).unwrap_or_default();
        self.toast(format!("{dn}: Também compro {wname} ({per} por 1 esmeralda)."), 4.0);
        true
    }
}
