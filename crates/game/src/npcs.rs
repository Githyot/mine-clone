//! Town NPCs (Terraria's idea): build a house and people move in. A house is a closed room
//! (walls, floor and roof all around) of 20 to 400 air blocks with a door, a light and a table
//! (crafting table or chest). Each NPC comes once its wish is met: the guide right away, the
//! merchant when you carry emeralds, the nurse after a life crystal, the wizard after your
//! first magic, the smith after the Forest Guardian falls. Right click to talk or trade.

use std::collections::{HashSet, VecDeque};

use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};
use voxel::block::Shape;
use voxel::item::{Inventory, ItemStack};
use voxel::AIR;

use crate::audio::{Material, Sfx};
use crate::survival::Open;
use crate::Game;

/// One NPC living in the world (saved with the level).
#[derive(Clone, Serialize, Deserialize)]
pub struct NpcSave {
    pub name: String,
    pub home: [f32; 3],
}

#[derive(Default)]
pub struct Town {
    pub npcs: Vec<NpcSave>,
    /// Valid rooms found near the player: (lowest cell, a spot to stand in).
    houses: Vec<(IVec3, Vec3)>,
    check_t: f32,
    /// Seconds until each missing NPC comes back (after dying).
    respawn: Vec<(String, f32)>,
    /// Which guide tip comes next.
    tip: usize,
}

impl Town {
    pub fn new(npcs: Vec<NpcSave>) -> Town {
        Town { npcs, check_t: 2.0, ..Default::default() }
    }
}

/// (mob name, what they sell: item, count, price in emeralds)
const SHOPS: &[(&str, &[(&str, u8, u8)])] = &[
    ("npc_merchant", &[("torch", 8, 1), ("apple", 3, 1), ("cooked_beef", 2, 1), ("oak_sapling", 4, 1), ("bucket", 1, 2), ("fishing_rod", 1, 2), ("iron_ingot", 2, 2), ("red_bed", 1, 3), ("ancient_seed", 1, 4)]),
    ("npc_wizard", &[("lapis_lazuli", 6, 2), ("topaz_wand", 1, 5), ("ruby_wand", 1, 6), ("sapphire_wand", 1, 6), ("amethyst_wand", 1, 7), ("mana_crystal", 1, 8), ("flame_tome", 1, 12), ("thunder_tome", 1, 14)]),
    ("goblin_trader", &[("diamond", 1, 5), ("redstone", 12, 1), ("gold_ingot", 3, 2), ("copper_ingot", 10, 1), ("honeycomb", 4, 1), ("ruby", 1, 3), ("topaz", 1, 3), ("life_crystal", 1, 9), ("minecart", 1, 3), ("leather_backpack", 1, 6), ("tnt", 2, 2)]),
    ("npc_farmer", &[("wheat_seeds", 8, 1), ("carrot", 4, 1), ("potato", 4, 1), ("beetroot_seeds", 6, 1), ("bread", 4, 1), ("hay_block", 2, 2), ("iron_hoe", 1, 2)]),
    ("npc_smith", &[("bronze_ingot", 4, 2), ("silver_ingot", 2, 3), ("tungsten_ingot", 2, 4), ("platinum_ingot", 2, 5), ("iron_pickaxe", 1, 4), ("platinum_sword", 1, 8), ("diamond", 1, 6)]),
];

const GUIDE_TIPS: &[&str] = &[
    "Uma casa precisa de porta, luz e uma mesa (bancada ou baú).",
    "Carregue esmeraldas e um mercador vai aparecer.",
    "Cristais de vida crescem no chão de cavernas fundas.",
    "Rubis, safiras, topázios e ametistas viram varinhas mágicas.",
    "A Semente Ancestral, usada à noite, acorda o Guardião da Floresta.",
    "Cada mundo tem seus metais comuns: procure o par raro também.",
];

impl Game {
    fn town_has(&self, name: &str) -> bool {
        self.town.npcs.iter().any(|n| n.name == name)
    }

    fn carrying(&self, item: &str) -> u32 {
        let Some(id) = self.items.id(item) else { return 0 };
        self.surv.inv.slots.iter().flatten().filter(|s| s.item == id).map(|s| s.count as u32).sum()
    }

    /// Who wants to move in now?
    fn npc_wish_met(&self, name: &str) -> bool {
        match name {
            "npc_guide" => true,
            "npc_merchant" => self.carrying("emerald") >= 5,
            "npc_nurse" => self.max_health > 20.0,
            "npc_wizard" => self.magic.max_mana > 20.0 || self.surv.inv.slots.iter().flatten().any(|s| crate::magic::is_caster(&self.items.get(s.item).name)),
            "npc_farmer" => ["wheat_seeds", "wheat", "carrot", "potato", "beetroot_seeds"].iter().any(|i| self.carrying(i) > 0),
            "npc_smith" => self.boss.defeated.iter().any(|b| b == "forest_guardian"),
            // A guard comes once a few people live here.
            "npc_guard" => self.town.npcs.len() >= 2,
            _ => false,
        }
    }

    pub(crate) fn update_npcs(&mut self, dt: f32) {
        if dt == 0.0 {
            return;
        }
        // Keep everyone who lives here in the world (back after a while if they died).
        for k in 0..self.town.npcs.len() {
            let n = self.town.npcs[k].clone();
            let home = Vec3::from(n.home);
            let present = self.mobs.list.iter().any(|m| self.mobs.reg.defs[m.def].name == n.name);
            if present || home.distance(self.body.pos) > 96.0 {
                continue;
            }
            if let Some(r) = self.town.respawn.iter_mut().find(|r| r.0 == n.name) {
                r.1 -= dt;
                if r.1 > 0.0 {
                    continue;
                }
            }
            self.town.respawn.retain(|r| r.0 != n.name);
            if self.mobs.spawn(&n.name, home) {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.home = Some(home);
                    m.fed = 0.0;
                }
            }
        }
        // A town NPC who died waits a minute before coming home.
        for m in &self.mobs.list {
            let d = &self.mobs.reg.defs[m.def];
            if d.town && !m.alive() && !self.town.respawn.iter().any(|r| r.0 == d.name) {
                self.town.respawn.push((d.name.clone(), 60.0));
            }
        }
        self.town.check_t -= dt;
        if self.town.check_t > 0.0 {
            return;
        }
        self.town.check_t = 6.0;
        self.find_houses();
        // One newcomer per check, into a house nobody lives in.
        for name in ["npc_guide", "npc_merchant", "npc_nurse", "npc_wizard", "npc_smith", "npc_farmer", "npc_guard"] {
            if self.town_has(name) || !self.npc_wish_met(name) {
                continue;
            }
            let free = self.town.houses.iter().find(|(_, spot)| !self.town.npcs.iter().any(|n| Vec3::from(n.home).distance(*spot) < 1.5)).copied();
            let Some((_, spot)) = free else { break };
            self.town.npcs.push(NpcSave { name: name.to_string(), home: spot.to_array() });
            if self.mobs.spawn(name, spot) {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.home = Some(spot);
                    m.fed = 0.0;
                }
            }
            let dn = self.mobs.reg.id(name).map(|i| self.mobs.reg.defs[i].display_name.clone()).unwrap_or_default();
            self.toast(format!("{dn} se mudou para uma das suas casas!"), 4.0);
            self.sfx(Sfx::Place(Material::Wood));
            break;
        }
    }

    /// Look for valid rooms behind the doors near the player.
    fn find_houses(&mut self) {
        let c = self.body.pos.floor().as_ivec3();
        let mut doors = Vec::new();
        for y in -8..=8 {
            for z in -20..=20 {
                for x in -20..=20 {
                    let p = c + IVec3::new(x, y, z);
                    let id = self.world.block_or_air(p);
                    if id == AIR {
                        continue;
                    }
                    let b = self.reg.get(id);
                    // Lower halves only (the upper half's variant has bit 8).
                    if b.shape == Shape::Door && b.variant & 8 == 0 {
                        doors.push(p);
                    }
                }
            }
        }
        let mut found: Vec<(IVec3, Vec3)> = Vec::new();
        for d in doors {
            for dir in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                if let Some(room) = self.room_from(d + dir) {
                    if !found.iter().any(|r| r.0 == room.0) {
                        found.push(room);
                    }
                }
            }
        }
        self.town.houses = found;
    }

    /// Air cells that are not walls (torches, plants and carpets don't block).
    fn open_cell(&self, p: IVec3) -> bool {
        let id = self.world.block_or_air(p);
        if id == AIR {
            return true;
        }
        let b = self.reg.get(id);
        !b.solid && b.shape != Shape::Door && !self.world.is_liquid(id)
    }

    /// Flood the room from `start`; a valid house gives (lowest cell, standing spot).
    fn room_from(&self, start: IVec3) -> Option<(IVec3, Vec3)> {
        if !self.open_cell(start) {
            return None;
        }
        let mut seen = HashSet::new();
        let mut queue = VecDeque::from([start]);
        seen.insert(start);
        let (mut light, mut table) = (false, false);
        let mut spot = None;
        let mut low = start;
        while let Some(p) = queue.pop_front() {
            if seen.len() > 400 || (p - start).abs().max_element() > 12 {
                return None; // outdoors, or too big to be a room
            }
            if self.reg.light_of(self.world.block_or_air(p)) > 0 {
                light = true;
            }
            if (p.y, p.x, p.z) < (low.y, low.x, low.z) {
                low = p;
            }
            if spot.is_none() && !self.open_cell(p - IVec3::Y) && self.open_cell(p + IVec3::Y) {
                spot = Some(p.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
            }
            for dir in [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z] {
                let q = p + dir;
                if self.open_cell(q) {
                    if seen.insert(q) {
                        queue.push_back(q);
                    }
                } else {
                    let id = self.world.block_or_air(q);
                    let b = self.reg.get(id);
                    if b.light > 0 {
                        light = true;
                    }
                    if matches!(self.reg.get(b.base).name.as_str(), "crafting_table" | "chest") {
                        table = true;
                    }
                }
            }
        }
        (seen.len() >= 20 && light && table).then_some((low, spot?))
    }

    /// Right click on a town NPC: talk, heal or trade.
    pub(crate) fn talk_to_npc(&mut self) -> bool {
        let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), 4.0) else { return false };
        // Grass and flowers in the way don't count.
        let block_dist = self.target.filter(|h| self.reg.get(self.world.block_or_air(h.block)).shape != Shape::Cross).map(|h| h.distance).unwrap_or(f32::MAX);
        let def = &self.mobs.reg.defs[self.mobs.list[i].def];
        if !def.town || t > block_dist || def.name.ends_with("_tame") {
            return false;
        }
        let (name, dn) = (def.name.clone(), def.display_name.clone());
        // They turn to look at you.
        let to = self.body.pos - self.mobs.list[i].body.pos;
        self.mobs.list[i].yaw = to.x.atan2(to.z);
        match name.as_str() {
            "npc_guide" => {
                let tip = GUIDE_TIPS[self.town.tip % GUIDE_TIPS.len()];
                self.town.tip += 1;
                self.toast(format!("{dn}: {tip}"), 6.0);
            }
            "npc_guard" => self.toast(format!("{dn}: Fico de olho nos monstros por aqui. Durma tranquilo!"), 4.0),
            "npc_nurse" => {
                let missing = self.max_health - self.health;
                if missing < 1.0 {
                    self.toast(format!("{dn}: Você está ótimo! Volte quando se machucar."), 4.0);
                } else {
                    let price = ((missing / 8.0).ceil() as u32).max(1);
                    if !self.survival || self.carrying("emerald") >= price {
                        if self.survival {
                            self.pay_emeralds(price);
                        }
                        self.health = self.max_health;
                        self.toast(format!("{dn}: Pronto, novinho em folha! ({price} esmeralda(s))"), 4.0);
                        self.sfx(Sfx::Xp);
                    } else {
                        self.toast(format!("{dn}: Curar você custa {price} esmeralda(s)."), 4.0);
                    }
                }
            }
            _ => {
                let Some(&(_, offers)) = SHOPS.iter().find(|s| s.0 == name) else { return true };
                let mut inv = Inventory::new(offers.len());
                for (k, &(item, count, _)) in offers.iter().enumerate() {
                    if let Some(id) = self.items.id(item) {
                        inv.slots[k] = Some(ItemStack::new(id, count));
                    }
                }
                self.surv.shop = inv;
                self.surv.shop_prices = offers.iter().map(|o| o.2).collect();
                self.surv.shop_name = dn;
                self.surv.open = Open::Shop;
                self.set_screen(crate::Screen::Inventory);
            }
        }
        true
    }

    /// Test hook: build two plank cottages ahead and let the guide and the wizard move in.
    pub(crate) fn test_houses(&mut self) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let base = (self.body.pos + flat * 7.0).floor().as_ivec3();
        let g = |n: &str| self.reg.id_or_air(n);
        let (plank, glass, door, door_up, torch, table) = (g("oak_planks"), g("glass"), g("oak_door:2"), g("oak_door:10"), g("torch"), g("crafting_table"));
        for (k, off) in [(0, -4), (1, 4)] {
            let o = base + IVec3::new(off, 0, 0) - IVec3::new(2, 0, 2);
            let y0 = self.world.surface_y(o.x + 2, o.z + 2).unwrap_or(o.y);
            for x in 0..5 {
                for z in 0..5 {
                    for y in 0..6 {
                        let p = IVec3::new(o.x + x, y0 + y, o.z + z);
                        let wall = x == 0 || x == 4 || z == 0 || z == 4 || y == 0 || y == 5;
                        let id = if !wall { AIR } else if y == 2 && (x == 2 || z == 2) && (x == 0 || x == 4) { glass } else { plank };
                        self.world.set_block(p, id);
                    }
                }
            }
            let front = IVec3::new(o.x + 2, y0 + 1, o.z + 4);
            self.world.set_block(front, door);
            self.world.set_block(front + IVec3::Y, door_up);
            self.world.set_block(IVec3::new(o.x + 1, y0 + 1, o.z + 1), table);
            self.world.set_block(IVec3::new(o.x + 3, y0 + 1, o.z + 1), torch);
            let _ = k;
        }
        if let Some(e) = self.items.id("emerald") {
            self.surv.inv.add(&self.items, ItemStack::new(e, 20));
        }
        self.magic.max_mana = 24.0;
        for _ in 0..3 {
            self.town.check_t = 0.0;
            self.update_npcs(0.1);
        }
        if let Ok(shop) = std::env::var("MC_SHOP") {
            if let Some(m) = self.mobs.list.iter_mut().find(|m| self.mobs.reg.defs[m.def].name == shop) {
                // Stand it in front of the eyes so the click finds it.
                m.body.pos = self.body.eye() + flat * 2.0 - Vec3::Y * 1.0;
            }
            self.talk_to_npc();
        }
    }

    fn pay_emeralds(&mut self, mut n: u32) {
        let Some(id) = self.items.id("emerald") else { return };
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

    /// Clicking a good in a shop buys it (onto the cursor, or into the inventory).
    pub(crate) fn buy(&mut self, i: usize) {
        let (Some(good), Some(&price)) = (self.surv.shop.slots.get(i).copied().flatten(), self.surv.shop_prices.get(i)) else { return };
        if self.surv.cursor.is_some_and(|c| !c.stacks_with(&good) || c.count as u32 + good.count as u32 > self.items.get(good.item).max_stack as u32) {
            return;
        }
        if self.survival && self.carrying("emerald") < price as u32 {
            self.toast(format!("Faltam esmeraldas: custa {price}"), 2.0);
            return;
        }
        if self.survival {
            self.pay_emeralds(price as u32);
        }
        self.surv.cursor = Some(match self.surv.cursor {
            Some(c) => ItemStack { count: c.count + good.count, ..c },
            None => good,
        });
        self.sfx(Sfx::Pickup);
    }
}
