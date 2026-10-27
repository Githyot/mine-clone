//! Villages (vanilla idea, our own folk): the settlers living round a village square and its
//! sentinel. Each settler has a trade: right click to open their shop, or hold out the goods
//! they want (a stack of wheat, coal, paper or wool) to be paid in coins. The sentinel, a
//! big stone-and-iron keeper, fights the monsters that come near. Settlers who die come back
//! after a while, as new folk move in.

use std::collections::HashMap;

use glam::{IVec2, Vec3};
use voxel::block::Shape;
use voxel::item::{Inventory, ItemId, ItemStack};
use voxel::worldgen::Dim;

use crate::audio::Sfx;
use crate::money::s;
use crate::survival::Open;
use crate::Game;

/// Who lives in a village: (mob, shop goods as (item, count, price in copper), what they
/// buy as (item, how many for one `PAY`)).
pub(crate) type Trade = (&'static str, &'static [(&'static str, u8, u32)], (&'static str, u32));
pub(crate) const SETTLERS: [Trade; 4] = [
    ("settler_farmer", &[("bread", 6, s(4)), ("apple", 4, s(3)), ("carrot", 6, s(3)), ("pumpkin", 2, s(3)), ("melon_slice", 8, s(2)), ("wheat_seeds", 8, s(1))], ("wheat", 20)),
    ("settler_smith", &[("iron_pickaxe", 1, s(20)), ("iron_sword", 1, s(20)), ("iron_axe", 1, s(15)), ("iron_chestplate", 1, s(40)), ("iron_helmet", 1, s(25)), ("bucket", 1, s(10)), ("iron_leggings", 1, s(30))], ("coal", 15)),
    ("settler_librarian", &[("bookshelf", 1, s(15)), ("compass", 1, s(20)), ("map", 1, s(15)), ("clock", 1, s(20)), ("lantern", 2, s(5)), ("glass", 8, s(4)), ("name_tag", 1, s(25))], ("paper", 24)),
    ("settler_shepherd", &[("white_wool", 4, s(3)), ("string", 6, s(3)), ("red_wool", 2, s(2)), ("blue_wool", 2, s(2)), ("yellow_wool", 2, s(2)), ("white_carpet", 6, s(3))], ("white_wool", 18)),
];
/// What a settler pays for one lot of the goods they want.
const PAY: u64 = 5 * crate::money::SILVER;
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

    fn count_item_id(&self, id: ItemId) -> u32 {
        self.surv.inv.slots.iter().flatten().filter(|s| s.item == id).map(|s| s.count as u32).sum()
    }

    fn take_item_id(&mut self, id: ItemId, mut n: u32) {
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
        let pay = crate::money::text(PAY);
        // Holding what they buy: paid for every lot of it you carry.
        if self.held_name().as_deref() == Some(want) {
            let Some(wid) = self.items.id(want) else { return true };
            let have = self.count_item_id(wid);
            if have < per {
                self.toast(format!("{dn}: Pago {pay} por {per}. Você só tem {have}."), 3.5);
                return true;
            }
            let n = (have / per).min(16);
            self.take_item_id(wid, n * per);
            self.earn(PAY * n as u64);
            self.toast(format!("{dn}: Negócio fechado! (+{})", crate::money::text(PAY * n as u64)), 3.0);
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
        self.surv.shop_want = self.items.id(want).map(|id| (id, (PAY / per as u64).max(1)));
        self.surv.shop_scroll = 0;
        self.surv.open = Open::Shop;
        self.set_screen(crate::Screen::Inventory);
        let wname = self.items.id(want).map(|id| self.items.get(id).display_name.clone()).unwrap_or_default();
        self.toast(format!("{dn}: Também compro {wname} ({per} por {pay})."), 4.0);
        true
    }
}
