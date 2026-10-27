//! Town NPCs (Terraria's idea): build a house and people move in. A house is a closed room
//! (walls, floor and roof all around) of 20 to 400 air blocks with a door, a light and a table
//! (crafting table or chest). Each NPC comes once its wish is met: the guide right away, the
//! merchant once you have 50 silver, the nurse after a life crystal, the wizard after your
//! first magic, the smith after the Forest Guardian falls. Right click to talk or trade.

use std::collections::{HashSet, VecDeque};

use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};
use voxel::block::Shape;
use voxel::item::{Inventory, ItemStack};
use voxel::AIR;

use crate::audio::{Material, Sfx};
use crate::money::{g, s};
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
    pub(crate) houses: Vec<(IVec3, Vec3)>,
    check_t: f32,
    /// Seconds until each missing NPC comes back (after dying).
    respawn: Vec<(String, f32)>,
}

impl Town {
    pub fn new(npcs: Vec<NpcSave>) -> Town {
        Town { npcs, check_t: 2.0, ..Default::default() }
    }
}

/// (mob name, what they sell: item, count, price in copper; s() is silver, g() gold)
pub(crate) const SHOPS: &[(&str, &[(&str, u8, u32)])] = &[
    ("npc_merchant", &[("torch", 8, s(4)), ("apple", 3, s(3)), ("cooked_beef", 2, s(4)), ("oak_sapling", 4, s(2)), ("bucket", 1, s(15)), ("fishing_rod", 1, s(12)), ("iron_ingot", 2, s(10)), ("red_bed", 1, s(25)), ("ancient_seed", 1, s(40)), ("lesser_healing_potion", 2, s(6)), ("lesser_mana_potion", 2, s(5)), ("pocket_watch", 1, s(80))]),
    ("npc_wizard", &[("lapis_lazuli", 6, s(10)), ("topaz_wand", 1, s(30)), ("ruby_wand", 1, s(40)), ("sapphire_wand", 1, s(40)), ("amethyst_wand", 1, s(50)), ("mana_crystal", 1, s(60)), ("flame_tome", 1, g(1)), ("thunder_tome", 1, g(1) + s(20)), ("mana_potion", 2, s(15)), ("wisp_staff", 1, s(80)), ("sage_pendant", 1, g(1))]),
    ("goblin_trader", &[("diamond", 1, s(30)), ("redstone", 12, s(5)), ("gold_ingot", 3, s(12)), ("copper_ingot", 10, s(5)), ("honeycomb", 4, s(4)), ("ruby", 1, s(15)), ("topaz", 1, s(15)), ("life_crystal", 1, s(70)), ("minecart", 1, s(20)), ("leather_backpack", 1, s(40)), ("tnt", 2, s(10)), ("magnet_ring", 1, s(60)), ("lucky_coin", 1, s(90))]),
    ("npc_farmer", &[("wheat_seeds", 8, s(1)), ("carrot", 4, s(2)), ("potato", 4, s(2)), ("beetroot_seeds", 6, s(1)), ("bread", 4, s(3)), ("hay_block", 2, s(5)), ("iron_hoe", 1, s(8))]),
    ("npc_smith", &[("bronze_ingot", 4, s(10)), ("silver_ingot", 2, s(12)), ("tungsten_ingot", 2, s(16)), ("platinum_ingot", 2, s(20)), ("iron_pickaxe", 1, s(20)), ("platinum_sword", 1, s(60)), ("diamond", 1, s(35))]),
];

pub(crate) const GUIDE_TIPS: &[&str] = &[
    "Uma casa precisa de porta, luz e uma mesa (bancada ou baú).",
    "Junte 50 de prata e um mercador vai aparecer.",
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
            "npc_merchant" => self.surv.money >= 50 * crate::money::SILVER,
            "npc_nurse" => self.max_health > 20.0,
            "npc_wizard" => self.magic.max_mana > 20.0 || self.surv.inv.slots.iter().flatten().any(|s| crate::magic::is_caster(&self.items.get(s.item).name)),
            "npc_farmer" => ["wheat_seeds", "wheat", "carrot", "potato", "beetroot_seeds"].iter().any(|i| self.carrying(i) > 0),
            "npc_smith" => self.boss.defeated.iter().any(|b| b == "forest_guardian"),
            // A guard comes once a few people live here.
            "npc_guard" => self.town.npcs.len() >= 2,
            _ => self.t5_npc_wish(name),
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
        for name in ["npc_guide", "npc_merchant", "npc_nurse", "npc_wizard", "npc_smith", "npc_farmer", "npc_guard", "npc_tinkerer", "npc_mechanic", "npc_sorceress"] {
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
                let at = self.mobs.list[i].body.pos;
                self.guide_talk(at, &dn);
            }
            "npc_guard" => self.toast(format!("{dn}: Fico de olho nos monstros por aqui. Durma tranquilo!"), 4.0),
            "npc_nurse" => {
                let missing = self.max_health - self.health;
                if missing < 1.0 {
                    self.toast(format!("{dn}: Você está ótimo! Volte quando se machucar."), 4.0);
                } else {
                    // 25 copper a point of health, a silver at least.
                    let price = (missing.ceil() as u64 * 25).max(crate::money::SILVER);
                    let cost = crate::money::text(price);
                    if self.pay(price) {
                        self.health = self.max_health;
                        self.toast(format!("{dn}: Pronto, novinho em folha! ({cost})"), 4.0);
                        self.sfx(Sfx::Xp);
                    } else {
                        self.toast(format!("{dn}: Curar você custa {cost}."), 4.0);
                    }
                }
            }
            _ => {
                let Some(&(_, offers)) = SHOPS.iter().chain(crate::hardmode::SHOPS).find(|s| s.0 == name) else { return true };
                let mut inv = Inventory::new(offers.len());
                for (k, &(item, count, _)) in offers.iter().enumerate() {
                    if let Some(id) = self.items.id(item) {
                        inv.slots[k] = Some(ItemStack::new(id, count));
                    }
                }
                self.surv.shop = inv;
                self.surv.shop_prices = offers.iter().map(|o| o.2).collect();
                self.surv.shop_name = dn;
                self.surv.shop_want = None;
                self.surv.shop_scroll = 0;
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
        self.surv.money += std::env::var("MC_MONEY").ok().and_then(|v| v.parse().ok()).unwrap_or(3 * crate::money::GOLD);
        self.magic.max_mana = 24.0;
        for _ in 0..3 {
            self.town.check_t = 0.0;
            self.update_npcs(0.1);
        }
        if let Ok(shop) = std::env::var("MC_SHOP") {
            // Whoever hasn't moved in yet is called over anyway.
            if !self.mobs.list.iter().any(|m| self.mobs.reg.defs[m.def].name == shop) {
                self.mobs.spawn(&shop, self.body.pos + flat * 2.0);
            }
            if let Some(m) = self.mobs.list.iter_mut().find(|m| self.mobs.reg.defs[m.def].name == shop) {
                // Stand it in front of the eyes so the click finds it.
                m.body.pos = self.body.eye() + flat * 2.0 - Vec3::Y * 1.0;
            }
            self.talk_to_npc();
        }
    }
}
