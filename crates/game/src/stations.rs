//! Phase 6 work blocks and their screens: the enchanting table (bookshelves around it raise
//! its offers), the anvil (combine, repair, add books; it wears out), the grindstone (takes
//! enchantments off for experience), the smithing table (netherite and trims), the brewing
//! stand (bottles, an ingredient and blaze powder) and the beacon (pick its powers).
//!
//! They use the survival screen: their slots are `Slot::Cont`, held in `Prog::bench` for the
//! tables that give everything back when closed, or in a container at the block for the
//! brewing stand and the beacon, which keep theirs.

use glam::{IVec3, Vec3};
use voxel::item::{Inventory, ItemId, ItemStack};
use voxel::rng::Rng;
use voxel::AIR;

use crate::audio::{Material, Sfx};
use crate::enchanting::{self as ench, WORK};
use crate::survival::{Container, Open, Slot};
use crate::ui::WHITE;
use crate::Game;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Station {
    Enchant(IVec3),
    Anvil(IVec3),
    Grindstone,
    Smithing,
    Brewing(IVec3),
    Beacon(IVec3),
}

impl Station {
    /// The slot that shows a result to take, if the station has one.
    pub fn output(self) -> Option<usize> {
        match self {
            Station::Anvil(_) | Station::Grindstone => Some(2),
            Station::Smithing => Some(3),
            _ => None,
        }
    }

    /// Kept in a container at the block (not given back on close).
    pub fn keeps(self) -> Option<IVec3> {
        match self {
            Station::Brewing(p) | Station::Beacon(p) => Some(p),
            _ => None,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Station::Enchant(_) => "Mesa de Encantamento",
            Station::Anvil(_) => "Bigorna",
            Station::Grindstone => "Rebolo",
            Station::Smithing => "Mesa de Ferraria",
            Station::Brewing(_) => "Suporte de Poções",
            Station::Beacon(_) => "Farol",
        }
    }
}

/// Classic experience curve: points to go from level `l` to `l + 1`.
pub fn xp_for_level(l: u32) -> u32 {
    if l < 16 {
        2 * l + 7
    } else if l < 31 {
        5 * l - 38
    } else {
        9 * l - 158
    }
}

pub fn xp_total(l: u32) -> u32 {
    (0..l).map(xp_for_level).sum()
}

/// What the table offers: (levels needed, enchantments) for each of its three rows.
pub type Offer = Option<(u32, Vec<(u8, u8)>)>;

/// Material that repairs an item on the anvil.
pub fn repair_material(name: &str) -> Option<&'static str> {
    let mat = name.split('_').next()?;
    Some(match mat {
        "wooden" => "oak_planks",
        "stone" => "cobblestone",
        "iron" => "iron_ingot",
        "golden" => "gold_ingot",
        "diamond" => "diamond",
        "netherite" => "netherite_ingot",
        "leather" => "leather",
        "copper" => "copper_ingot",
        "bronze" => "bronze_ingot",
        "silver" => "silver_ingot",
        "lead" => "lead_ingot",
        "tungsten" => "tungsten_ingot",
        "platinum" => "platinum_ingot",
        "turtle" => "turtle_scute",
        "chainmail" => "iron_ingot",
        _ => match name {
            "elytra" => "phantom_membrane",
            "mace" => "breeze_rod",
            "shield" => "oak_planks",
            _ => return None,
        },
    })
}

/// Brewing: the potion a bottle becomes with an ingredient (by potion key), if any.
pub fn brew(potion: &str, ingredient: &str) -> Option<String> {
    let base = match (potion, ingredient) {
        ("water", "nether_wart") => "awkward",
        ("water", "glowstone_dust") => "thick",
        ("water", "fermented_spider_eye") => "weakness",
        ("water", "redstone" | "spider_eye" | "ghast_tear" | "rabbit_foot" | "blaze_powder" | "glistering_melon_slice" | "sugar" | "magma_cream") => "mundane",
        ("awkward", "golden_carrot") => "night_vision",
        ("awkward", "rabbit_foot") => "leaping",
        ("awkward", "magma_cream") => "fire_resistance",
        ("awkward", "sugar") => "swiftness",
        ("awkward", "pufferfish") => "water_breathing",
        ("awkward", "glistering_melon_slice") => "healing",
        ("awkward", "spider_eye") => "poison",
        ("awkward", "ghast_tear") => "regeneration",
        ("awkward", "blaze_powder") => "strength",
        ("awkward", "turtle_scute") => "turtle_master",
        ("awkward", "phantom_membrane") => "slow_falling",
        ("awkward", "breeze_rod") => "wind_charged",
        ("awkward", "cobweb") => "weaving",
        ("awkward", "slime_block") => "oozing",
        ("awkward", "stone") => "infested",
        _ => "",
    };
    if !base.is_empty() {
        return Some(base.to_string());
    }
    // Fermented spider eye turns a potion into its dark twin, keeping long or strong.
    if ingredient == "fermented_spider_eye" {
        let (pre, stem) = split_strength(potion);
        let twin = match stem {
            "night_vision" => "invisibility",
            "leaping" | "swiftness" => "slowness",
            "healing" | "poison" => "harming",
            _ => return None,
        };
        let out = format!("{pre}{twin}");
        return crate::effects::potion(&out).map(|_| out).or_else(|| crate::effects::potion(twin).map(|_| twin.to_string()));
    }
    let (_, stem) = split_strength(potion);
    let out = match ingredient {
        "redstone" => format!("long_{stem}"),
        "glowstone_dust" => format!("strong_{stem}"),
        _ => return None,
    };
    (crate::effects::potion(&out).is_some() && out != potion).then_some(out)
}

fn split_strength(key: &str) -> (&'static str, &str) {
    if let Some(s) = key.strip_prefix("long_") {
        ("long_", s)
    } else if let Some(s) = key.strip_prefix("strong_") {
        ("strong_", s)
    } else {
        ("", key)
    }
}

/// The beacon's powers by pyramid level needed.
pub const BEACON_POWERS: [(u8, u8); 5] = [
    (crate::effects::SPEED, 1),
    (crate::effects::HASTE, 1),
    (crate::effects::RESISTANCE, 2),
    (crate::effects::JUMP_BOOST, 2),
    (crate::effects::STRENGTH, 3),
];

const BEACON_PAY: [&str; 5] = ["iron_ingot", "gold_ingot", "diamond", "emerald", "netherite_ingot"];
const PYRAMID: [&str; 5] = ["iron_block", "gold_block", "diamond_block", "emerald_block", "netherite_block"];

impl Game {
    /// Right click on a Phase 6 block: open its screen. True when it was one.
    pub(crate) fn station_use(&mut self, name: &str, pos: IVec3) -> bool {
        let st = match name {
            "enchanting_table" => Station::Enchant(pos),
            "anvil" | "chipped_anvil" | "damaged_anvil" => Station::Anvil(pos),
            "grindstone" => Station::Grindstone,
            "smithing_table" => Station::Smithing,
            "brewing_stand" => {
                self.surv.containers.entry(pos).or_insert_with(|| Container::Chest(Inventory::new(5)));
                Station::Brewing(pos)
            }
            "beacon" => {
                self.surv.containers.entry(pos).or_insert_with(|| Container::Chest(Inventory::new(1)));
                self.prog.beacons.entry(pos).or_default();
                Station::Beacon(pos)
            }
            _ => return false,
        };
        self.prog.bench = Inventory::new(4);
        self.surv.open = Open::Station(st);
        self.set_screen(crate::Screen::Inventory);
        self.sfx(Sfx::Place(if matches!(st, Station::Anvil(_)) { Material::Metal } else { Material::Wood }));
        true
    }

    pub(crate) fn station_inv(&self, st: Station) -> Option<&Inventory> {
        match st.keeps() {
            Some(p) => match self.surv.containers.get(&p)? {
                Container::Chest(i) => Some(i),
                Container::Furnace(f) => Some(&f.inv),
            },
            None => Some(&self.prog.bench),
        }
    }

    pub(crate) fn station_inv_mut(&mut self, st: Station) -> Option<&mut Inventory> {
        match st.keeps() {
            Some(p) => match self.surv.containers.get_mut(&p)? {
                Container::Chest(i) => Some(i),
                Container::Furnace(f) => Some(&mut f.inv),
            },
            None => Some(&mut self.prog.bench),
        }
    }

    /// Closing a table gives its items back (the result shown is not real yet).
    pub(crate) fn station_closed(&mut self, st: Station) {
        if st.keeps().is_some() {
            return;
        }
        let out = st.output();
        let items: Vec<ItemStack> = self.prog.bench.slots.iter_mut().enumerate().filter(|(i, _)| Some(*i) != out).filter_map(|(_, s)| s.take()).collect();
        self.prog.bench = Inventory::new(4);
        for s in items {
            if let Some(left) = self.surv.inv.add(&self.items, s) {
                self.throw_stack(left);
            }
        }
    }

    pub(crate) fn is_station_output(&self, slot: Slot) -> bool {
        match (self.surv.open, slot) {
            (Open::Station(st), Slot::Cont(i)) => st.output() == Some(i),
            _ => false,
        }
    }

    fn level_now(&self) -> u32 {
        self.surv.level().0
    }

    /// Take `n` levels away, keeping the progress into the current one.
    pub(crate) fn spend_levels(&mut self, n: u32) {
        let (lvl, k) = self.surv.level();
        let to = lvl.saturating_sub(n);
        self.surv.xp = xp_total(to) + (k * xp_for_level(to) as f32) as u32;
    }

    fn item_name(&self, s: &Option<ItemStack>) -> String {
        s.map(|s| self.items.get(s.item).name.clone()).unwrap_or_default()
    }

    // -----------------------------------------------------------------------------------------
    // Enchanting table.

    /// Bookshelves around the table: two blocks out, on the table's level or one up, with air
    /// between (vanilla counts at most 15).
    pub(crate) fn bookshelves(&self, p: IVec3) -> u32 {
        let mut n = 0;
        for dz in -2i32..=2 {
            for dx in -2i32..=2 {
                if dx.abs() != 2 && dz.abs() != 2 {
                    continue;
                }
                for dy in 0..=1 {
                    let q = p + IVec3::new(dx, dy, dz);
                    let name = &self.reg.get(self.world.block_or_air(q)).name;
                    if name != "bookshelf" && name != "chiseled_bookshelf" {
                        continue;
                    }
                    let mid = p + IVec3::new(dx.signum(), dy, dz.signum());
                    if self.world.block_or_air(mid) == AIR {
                        n += 1;
                    }
                }
            }
        }
        n.min(15)
    }

    pub(crate) fn table_offers(&self, p: IVec3) -> [Offer; 3] {
        let Some(st) = self.prog.bench.slots[0] else { return [None, None, None] };
        if !st.ench.pairs().all(|(id, _)| id >= ench::TRIM) || st.count != 1 {
            return [None, None, None];
        }
        let k = ench::kinds(&self.items, st.item);
        if !(k.durable || k.book) || self.items.get(st.item).name == "enchanted_book" {
            return [None, None, None];
        }
        let b = self.bookshelves(p) as i32;
        let mut rng = Rng::new(self.prog.ench_seed as u64 ^ (st.item as u64) << 16);
        let base = rng.range(1, 8) + b / 2 + rng.range(0, b);
        let costs = [(base / 3).max(1), base * 2 / 3 + 1, base.max(b * 2)];
        let mut out: [Offer; 3] = [None, None, None];
        for (i, &c) in costs.iter().enumerate() {
            let mut r = Rng::new(self.prog.ench_seed as u64 + i as u64 * 7919 + st.item as u64);
            let list = ench::roll(&self.items, st.item, c, &mut r);
            if !list.is_empty() {
                out[i] = Some((c as u32, list));
            }
        }
        out
    }

    fn enchant_pick(&mut self, p: IVec3, row: usize) {
        let offers = self.table_offers(p);
        let Some((cost, list)) = offers[row].clone() else { return };
        let lapis = self.prog.bench.slots[1].filter(|s| self.items.get(s.item).name == "lapis_lazuli").map(|s| s.count).unwrap_or(0);
        let need = row as u32 + 1;
        if self.survival && (self.level_now() < cost || (lapis as u32) < need) {
            self.toast(if (lapis as u32) < need { "Falta lápis-lazúli" } else { "Nível de experiência insuficiente" }, 2.0);
            return;
        }
        let Some(mut st) = self.prog.bench.slots[0] else { return };
        if self.items.get(st.item).name == "book" {
            if let Some(b) = self.items.id("enchanted_book") {
                st.item = b;
            }
        }
        for (id, l) in list {
            st.ench.set(id, l);
        }
        self.prog.bench.slots[0] = Some(st);
        if self.survival {
            self.prog.bench.take(1, need as u8);
            self.spend_levels(need);
        }
        self.prog.ench_seed = self.prog.ench_seed.wrapping_mul(1_103_515_245).wrapping_add(12345 + self.surv.xp);
        self.prog.stats.add("items_enchanted", 1);
        self.prog.event(crate::progression::Event::Enchanted);
        self.sfx(Sfx::Cast(2));
        let glow = self.glow_layer();
        for k in 0..16 {
            let a = k as f32 * 0.4;
            self.particles.glow(p.as_vec3() + Vec3::new(0.5 + a.cos() * 1.6, 1.4, 0.5 + a.sin() * 1.6), Vec3::new(-a.cos(), -0.6, -a.sin()) * 1.2, glow, [0.7, 0.5, 1.4], 0.06, 1.0);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Anvil, grindstone, smithing: the result each shows.

    /// The anvil's result and its price in levels.
    pub(crate) fn anvil_result(&self) -> Option<(ItemStack, u32)> {
        let a = self.prog.bench.slots[0]?;
        let b = self.prog.bench.slots[1]?;
        let an = self.items.get(a.item).name.clone();
        let bn = self.items.get(b.item).name.clone();
        let ka = ench::kinds(&self.items, a.item);
        let mut out = ItemStack { count: 1, ..a };
        let mut cost: u32 = 0;
        let dur = self.items.get(a.item).tool.as_ref().map(|t| t.durability).or(self.items.get(a.item).armor.map(|x| x.durability)).unwrap_or(0);
        let book_b = bn == "enchanted_book";
        if repair_material(&an) == Some(bn.as_str()) && a.damage > 0 && dur > 0 {
            // Each material mends a quarter of the item.
            let per = (dur / 4).max(1);
            let units = (a.damage.div_ceil(per)).min(b.count as u16);
            out.damage = a.damage.saturating_sub(per * units);
            cost += units as u32;
        } else if book_b || (b.item == a.item && a.count == 1) {
            if b.item == a.item && dur > 0 && a.damage > 0 {
                let left = (dur - a.damage) as u32 + (dur - b.damage.min(dur)) as u32 + dur as u32 * 12 / 100;
                out.damage = dur.saturating_sub(left.min(dur as u32) as u16);
                cost += 2;
            }
            for (id, l) in b.ench.pairs() {
                if id >= ench::TRIM {
                    continue;
                }
                let Some(d) = ench::def(id) else { continue };
                let fits = an == "enchanted_book" || ench::fits(d.target, &ka);
                let clash = a.ench.pairs().any(|(o, _)| o < ench::TRIM && !ench::compatible(o, id));
                if !fits || clash {
                    cost += 1;
                    continue;
                }
                let have = a.ench.get(id);
                let new = if have == l { (l + 1).min(d.max) } else { have.max(l) };
                out.ench.set(id, new);
                cost += ench::anvil_cost(id, book_b) * new as u32;
            }
        } else {
            return None;
        }
        let (wa, wb) = (a.ench.get(WORK) as u32, b.ench.get(WORK) as u32);
        cost += (1 << wa.min(6)) - 1 + (1 << wb.min(6)) - 1;
        out.ench.set(WORK, (wa.max(wb) + 1).min(30) as u8);
        if out.ench.pairs().filter(|(i, _)| *i != WORK).eq(a.ench.pairs().filter(|(i, _)| *i != WORK)) && out.damage == a.damage {
            return None;
        }
        Some((out, cost.max(1)))
    }

    /// The grindstone's result and the experience it gives back.
    pub(crate) fn grindstone_result(&self) -> Option<(ItemStack, u32)> {
        let (a, b) = (self.prog.bench.slots[0], self.prog.bench.slots[1]);
        let base = a.or(b)?;
        if let (Some(a), Some(b)) = (a, b) {
            if a.item != b.item {
                return None;
            }
        }
        let mut out = ItemStack { count: 1, ..base };
        let mut xp = 0;
        for s in [a, b].into_iter().flatten() {
            xp += ench::total_power(&s);
        }
        if let (Some(a), Some(b)) = (a, b) {
            let dur = self.items.get(a.item).tool.as_ref().map(|t| t.durability).or(self.items.get(a.item).armor.map(|x| x.durability)).unwrap_or(0);
            if dur == 0 {
                return None;
            }
            let left = (dur - a.damage.min(dur)) as u32 + (dur - b.damage.min(dur)) as u32 + dur as u32 * 5 / 100;
            out.damage = dur.saturating_sub(left.min(dur as u32) as u16);
        }
        let mut e = voxel::item::Ench::EMPTY;
        for (id, l) in base.ench.pairs() {
            if id == ench::TRIM || ench::def(id).is_some_and(|d| d.curse) {
                e.set(id, l);
            }
        }
        out.ench = e;
        if self.items.get(out.item).name == "enchanted_book" && out.ench.is_empty() {
            out.item = self.items.id("book")?;
        }
        if out == base && b.is_none() {
            return None;
        }
        Some((out, xp))
    }

    pub(crate) fn station_refresh(&mut self, st: Station) {
        let out = match st {
            Station::Anvil(_) => self.anvil_result().map(|r| r.0),
            Station::Grindstone => self.grindstone_result().map(|r| r.0),
            Station::Smithing => crate::smithing::result(&self.items, self.prog.bench.slots[0], self.prog.bench.slots[1], self.prog.bench.slots[2]),
            _ => return,
        };
        if let Some(i) = st.output() {
            self.prog.bench.slots[i] = out;
        }
    }

    /// Clicks on a station's own slots that the plain container rules can't handle: taking a
    /// result. True when handled.
    pub(crate) fn station_click(&mut self, slot: Slot, shift: bool) -> bool {
        let Open::Station(st) = self.surv.open else { return false };
        let Slot::Cont(i) = slot else { return false };
        if st.output() != Some(i) {
            return false;
        }
        let Some(res) = self.prog.bench.slots[i] else { return true };
        let cost = match st {
            Station::Anvil(_) => self.anvil_result().map(|r| r.1).unwrap_or(0),
            _ => 0,
        };
        if self.survival && cost >= 40 {
            self.toast("Caro demais!", 2.0);
            return true;
        }
        if self.survival && cost > self.level_now() {
            self.toast(format!("Precisa de {cost} níveis"), 2.0);
            return true;
        }
        if shift {
            if self.surv.inv.clone().add(&self.items, res).is_some() {
                return true;
            }
            self.surv.inv.add(&self.items, res);
        } else if self.surv.cursor.is_none() {
            self.surv.cursor = Some(res);
        } else {
            return true;
        }
        match st {
            Station::Anvil(p) => {
                if self.survival {
                    self.spend_levels(cost);
                }
                let bn = self.item_name(&self.prog.bench.slots[1]);
                let an = self.item_name(&self.prog.bench.slots[0]);
                let used = if repair_material(&an) == Some(bn.as_str()) {
                    let a = self.prog.bench.slots[0].unwrap();
                    let dur = self.items.get(a.item).tool.as_ref().map(|t| t.durability).or(self.items.get(a.item).armor.map(|x| x.durability)).unwrap_or(1);
                    a.damage.div_ceil((dur / 4).max(1)).min(64) as u8
                } else {
                    1
                };
                self.prog.bench.slots[0] = None;
                self.prog.bench.take(1, used);
                self.sfx(Sfx::Place(Material::Metal));
                self.prog.event(crate::progression::Event::Repaired);
                // The anvil wears: chipped, then damaged, then it breaks.
                if self.survival && self.prog.rng.f32() < 0.12 {
                    let now = self.reg.get(self.world.block_or_air(p)).name.clone();
                    let next = match now.as_str() {
                        "anvil" => "chipped_anvil",
                        "chipped_anvil" => "damaged_anvil",
                        _ => "",
                    };
                    if next.is_empty() {
                        self.world.set_block(p, AIR);
                        self.sfx(Sfx::Break(Material::Metal));
                        self.close_survival_screen();
                    } else {
                        let id = self.reg.id_or_air(next);
                        self.world.set_block(p, id);
                    }
                }
            }
            Station::Grindstone => {
                let xp = self.grindstone_result().map(|r| r.1).unwrap_or(0);
                self.prog.bench.take(0, 1);
                self.prog.bench.take(1, 1);
                if xp > 0 {
                    let n = xp.div_ceil(2) + self.prog.rng.below(xp.div_ceil(2) + 1);
                    let at = self.body.pos + Vec3::Y;
                    self.spawn_orbs(at, n);
                }
                self.sfx(Sfx::Hit(Material::Stone));
                self.prog.event(crate::progression::Event::Ground);
            }
            Station::Smithing => {
                let base = self.item_name(&self.prog.bench.slots[1]);
                self.prog.bench.take(0, 1);
                self.prog.bench.take(1, 1);
                self.prog.bench.take(2, 1);
                self.sfx(Sfx::Place(Material::Metal));
                let rn = self.items.get(res.item).name.clone();
                if rn.starts_with("netherite_") && base.starts_with("diamond_") {
                    self.prog.event(crate::progression::Event::Smithed(rn));
                } else {
                    self.prog.event(crate::progression::Event::Trimmed);
                }
            }
            _ => {}
        }
        self.prog.bench.slots[i] = None;
        true
    }

    /// Shift-click from the inventory into a station: the slot each item belongs in.
    pub(crate) fn station_quick_move(&mut self, st: Station, s: ItemStack) -> Option<ItemStack> {
        let name = self.items.get(s.item).name.clone();
        let range = match st {
            Station::Enchant(_) => {
                if name == "lapis_lazuli" {
                    1..2
                } else {
                    0..1
                }
            }
            Station::Anvil(_) | Station::Grindstone => {
                if self.prog.bench.slots[0].is_none() {
                    0..1
                } else {
                    1..2
                }
            }
            Station::Smithing => {
                if name.ends_with("_smithing_template") {
                    0..1
                } else if crate::smithing::MATERIALS.iter().any(|m| m.0 == name) && self.prog.bench.slots[1].is_some() {
                    2..3
                } else {
                    1..2
                }
            }
            Station::Brewing(_) => {
                if name == "blaze_powder" {
                    4..5
                } else if crate::effects::potion_of(&name).is_some() || name == "glass_bottle" {
                    0..3
                } else {
                    3..4
                }
            }
            Station::Beacon(_) => 0..1,
        };
        let items = &self.items;
        match st.keeps() {
            Some(p) => match self.surv.containers.get_mut(&p) {
                Some(Container::Chest(inv)) => inv.add_in(items, s, range),
                _ => Some(s),
            },
            None => self.prog.bench.add_in(items, s, range),
        }
    }

    /// The station's half of the survival screen: its slots and controls.
    pub(crate) fn draw_station(&mut self, st: Station, x0: f32, y0: f32, slots: &mut Vec<(Slot, f32, f32)>, mouse: (f32, f32)) {
        let s = self.ui.scale;
        let dark = [64, 64, 64, 255];
        self.station_refresh(st);
        self.ui.label(x0 + 8.0 * s, y0 + 6.0 * s, st.title(), dark, s);
        let at = |a: f32, b: f32| (x0 + a * s, y0 + b * s);
        match st {
            Station::Enchant(p) => {
                let (a, b) = at(15.0, 47.0);
                slots.push((Slot::Cont(0), a, b));
                let (a, b) = at(35.0, 47.0);
                slots.push((Slot::Cont(1), a, b));
                let offers = self.table_offers(p);
                let lvl = self.level_now();
                let lapis = self.prog.bench.slots[1].filter(|s| self.items.get(s.item).name == "lapis_lazuli").map(|s| s.count as u32).unwrap_or(0);
                let shelves = self.bookshelves(p);
                for (i, o) in offers.iter().enumerate() {
                    let (bx, by) = at(60.0, 14.0 + i as f32 * 19.0);
                    let (bw, bh) = (108.0 * s, 19.0 * s);
                    let over = mouse.0 >= bx && mouse.0 < bx + bw && mouse.1 >= by && mouse.1 < by + bh;
                    let ok = o.as_ref().is_some_and(|(c, _)| !self.survival || (lvl >= *c && lapis > i as u32));
                    let bg = match (o.is_some(), ok, over) {
                        (false, _, _) => [140, 120, 110, 255],
                        (true, true, true) => [160, 110, 200, 255],
                        (true, true, false) => [120, 85, 160, 255],
                        (true, false, _) => [100, 80, 90, 255],
                    };
                    self.ui.rect(bx, by, bw, bh - s, bg);
                    self.ui.frame(bx, by, bw, bh - s, s, [40, 30, 40, 255]);
                    if let Some((cost, list)) = o {
                        // The first enchantment is the clue; the rest is a surprise.
                        let d = ench::def(list[0].0).unwrap();
                        let clue = format!("{} {}...?", d.name, ench::roman(list[0].1));
                        let col = if ok { [230, 220, 255, 255] } else { [150, 140, 150, 255] };
                        self.ui.text_scaled(bx + 14.0 * s, by + 3.0 * s, &clue, col, s * 0.75);
                        let c = cost.to_string();
                        let cw = self.ui.text_width(&c, s);
                        self.ui.text(bx + bw - cw - 3.0 * s, by + 8.0 * s, &c, if ok { [128, 255, 32, 255] } else { [100, 140, 60, 255] });
                        // Lapis dots: how much this row takes.
                        for k in 0..=i {
                            self.ui.rect(bx + 3.0 * s, by + (3.0 + k as f32 * 4.0) * s, 3.0 * s, 3.0 * s, [40, 80, 200, 255]);
                        }
                        if over && self.clicked && ok {
                            self.clicked = false;
                            self.enchant_pick(p, i);
                        }
                    }
                }
                let t = format!("Estantes: {shelves}/15");
                self.ui.text_scaled(x0 + 8.0 * s, y0 + 31.0 * s, &t, dark, s * 0.6);
                // A small open book over the table.
                let (bx, by) = at(14.0, 18.0);
                let flap = (self.now * 2.0).sin() * 2.0 * s;
                self.ui.rect(bx, by + flap.abs(), 14.0 * s, 10.0 * s, [120, 60, 30, 255]);
                self.ui.rect(bx + 14.0 * s, by + flap.abs(), 14.0 * s, 10.0 * s, [120, 60, 30, 255]);
                self.ui.rect(bx + s, by + s + flap.abs(), 12.0 * s, 8.0 * s, [235, 225, 200, 255]);
                self.ui.rect(bx + 15.0 * s, by + s + flap.abs(), 12.0 * s, 8.0 * s, [235, 225, 200, 255]);
            }
            Station::Anvil(_) | Station::Grindstone => {
                let anvil = matches!(st, Station::Anvil(_));
                let (pa, pb, po) = if anvil { ((27.0, 47.0), (76.0, 47.0), (134.0, 47.0)) } else { ((49.0, 19.0), (49.0, 40.0), (129.0, 34.0)) };
                for (k, (a, b)) in [pa, pb, po].into_iter().enumerate() {
                    let (x, y) = at(a, b);
                    slots.push((Slot::Cont(k), x, y));
                }
                if anvil {
                    self.ui.text(x0 + 57.0 * s, y0 + 51.0 * s, "+", dark);
                    if let Some((_, cost)) = self.anvil_result() {
                        let too = self.survival && cost >= 40;
                        let t = if too { "Caro demais!".to_string() } else { format!("Custo: {cost} níveis") };
                        let col = if too || (self.survival && cost > self.level_now()) { [255, 85, 85, 255] } else { [80, 200, 40, 255] };
                        self.ui.text_scaled(x0 + 60.0 * s, y0 + 24.0 * s, &t, col, s * 0.85);
                    }
                } else if let Some((_, xp)) = self.grindstone_result() {
                    if xp > 0 {
                        self.ui.label(x0 + 100.0 * s, y0 + 60.0 * s, &format!("+{xp} xp"), [80, 200, 40, 255], s);
                    }
                }
                let (ax, ay) = at(if anvil { 102.0 } else { 98.0 }, if anvil { 50.0 } else { 37.0 });
                self.ui.rect(ax, ay, 22.0 * s, 4.0 * s, [139, 139, 139, 255]);
            }
            Station::Smithing => {
                for (k, (a, b)) in [(8.0, 48.0), (26.0, 48.0), (44.0, 48.0), (98.0, 48.0)].into_iter().enumerate() {
                    let (x, y) = at(a, b);
                    slots.push((Slot::Cont(k), x, y));
                }
                self.ui.text_scaled(x0 + 8.0 * s, y0 + 20.0 * s, "Molde + Peça + Material", dark, s * 0.75);
                let (ax, ay) = at(68.0, 52.0);
                self.ui.rect(ax, ay, 22.0 * s, 4.0 * s, [139, 139, 139, 255]);
            }
            Station::Brewing(p) => {
                for (k, (a, b)) in [(56.0, 51.0), (79.0, 58.0), (102.0, 51.0), (79.0, 17.0), (17.0, 17.0)].into_iter().enumerate() {
                    let (x, y) = at(a, b);
                    slots.push((Slot::Cont(k), x, y));
                }
                let brew = self.prog.brews.get(&p).copied().unwrap_or_default();
                // Bubbles rising while it brews, and the fuel bar.
                let (bx, by) = at(64.0, 18.0);
                let k = if brew.time > 0.0 { 1.0 - brew.time / 20.0 } else { 0.0 };
                self.ui.rect(bx, by, 4.0 * s, 26.0 * s, [139, 139, 139, 255]);
                self.ui.rect(bx, by + 26.0 * s * (1.0 - k), 4.0 * s, 26.0 * s * k, [230, 200, 255, 255]);
                let (fx, fy) = at(18.0, 40.0);
                self.ui.rect(fx, fy, 18.0 * s, 4.0 * s, [60, 40, 40, 255]);
                self.ui.rect(fx, fy, 18.0 * s * brew.fuel as f32 / 20.0, 4.0 * s, [230, 140, 40, 255]);
            }
            Station::Beacon(p) => {
                let (x, y) = at(150.0, 50.0);
                slots.push((Slot::Cont(0), x, y));
                let levels = self.pyramid_levels(p);
                let state = self.prog.beacons.get(&p).copied().unwrap_or_default();
                self.ui.text_scaled(x0 + 8.0 * s, y0 + 18.0 * s, &format!("Pirâmide: {levels} níveis"), dark, s * 0.75);
                let mut pick: Option<(u8, bool)> = None;
                for (k, &(e, need)) in BEACON_POWERS.iter().enumerate() {
                    let (bx, by) = at(8.0 + (k % 3) as f32 * 24.0, 28.0 + (k / 3) as f32 * 22.0);
                    let on = state.primary == e;
                    let can = levels >= need;
                    let over = mouse.0 >= bx && mouse.0 < bx + 22.0 * s && mouse.1 >= by && mouse.1 < by + 20.0 * s;
                    self.ui.rect(bx, by, 22.0 * s, 20.0 * s, if on { [90, 160, 90, 255] } else if can && over { [150, 150, 170, 255] } else if can { [110, 110, 120, 255] } else { [70, 70, 70, 255] });
                    let c = crate::effects::info(e).1;
                    crate::effects::draw_effect_icon(&mut self.ui, e, c, bx + 3.0 * s, by + 2.0 * s, s);
                    if over && can && self.clicked {
                        pick = Some((e, false));
                    }
                }
                // The second power at four levels: regeneration, or the first one at level II.
                for (k, e) in [crate::effects::REGENERATION, state.primary].into_iter().enumerate() {
                    if e == 0 {
                        continue;
                    }
                    let (bx, by) = at(88.0 + k as f32 * 24.0, 28.0);
                    let can = levels >= 4;
                    let on = state.secondary == e;
                    let over = mouse.0 >= bx && mouse.0 < bx + 22.0 * s && mouse.1 >= by && mouse.1 < by + 20.0 * s;
                    self.ui.rect(bx, by, 22.0 * s, 20.0 * s, if on { [90, 160, 90, 255] } else if can && over { [150, 150, 170, 255] } else if can { [110, 110, 120, 255] } else { [70, 70, 70, 255] });
                    let c = crate::effects::info(e).1;
                    crate::effects::draw_effect_icon(&mut self.ui, e, c, bx + 3.0 * s, by + 2.0 * s, s);
                    if k == 1 {
                        self.ui.text_scaled(bx + 14.0 * s, by + 12.0 * s, "II", WHITE, s * 0.75);
                    }
                    if over && can && self.clicked {
                        pick = Some((e, true));
                    }
                }
                self.ui.text_scaled(x0 + 88.0 * s, y0 + 55.0 * s, "Pagamento:", dark, s * 0.75);
                if let Some((e, second)) = pick {
                    self.clicked = false;
                    let paid = self.station_inv(st).and_then(|i| i.slots[0]).is_some_and(|s| BEACON_PAY.contains(&self.items.get(s.item).name.as_str()));
                    if !paid && self.survival {
                        self.toast("Ponha um lingote de ferro, ouro, netherita, um diamante ou uma esmeralda", 3.0);
                    } else {
                        if let Some(inv) = self.station_inv_mut(st) {
                            inv.take(0, 1);
                        }
                        let b = self.prog.beacons.entry(p).or_default();
                        if second {
                            b.secondary = e;
                        } else {
                            b.primary = e;
                        }
                        self.sfx(Sfx::Portal);
                        self.prog.event(crate::progression::Event::BeaconOn(levels));
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Brewing stands, beacons and conduits at work.

    pub(crate) fn update_stations(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let stands: Vec<IVec3> = self
            .surv
            .containers
            .iter()
            .filter(|(p, c)| matches!(c, Container::Chest(i) if i.slots.len() == 5) && self.reg.get(self.world.block_or_air(**p)).name == "brewing_stand")
            .map(|(p, _)| *p)
            .collect();
        for p in stands {
            self.brew_tick(p, dt);
        }
        self.prog.beacon_t -= dt;
        if self.prog.beacon_t <= 0.0 {
            self.prog.beacon_t = 4.0;
            self.beacon_pulse();
        }
        self.prog.conduit_t -= dt;
        if self.prog.conduit_t <= 0.0 {
            self.prog.conduit_t = 2.0;
            self.conduit_pulse();
        }
        // Beams: a column of light over each working beacon near the player.
        let glow = self.glow_layer();
        let beams: Vec<IVec3> = self.prog.beacons.iter().filter(|(_, b)| b.primary != 0 && b.levels > 0).map(|(p, _)| *p).collect();
        for p in beams {
            if p.as_vec3().distance(self.body.pos) > 96.0 {
                continue;
            }
            for k in 0..3 {
                let y = (self.now * 9.0 + k as f32 * 13.0) % 60.0;
                self.particles.glow(p.as_vec3() + Vec3::new(0.5, 1.0 + y, 0.5), Vec3::Y * 6.0, glow, [0.8, 0.9, 1.4], 0.22, 0.4);
            }
        }
        // Frost walker ice melts back to water.
        let water = self.reg.id_or_air("water");
        let frosted = self.reg.id_or_air("frosted_ice");
        let mut melt = Vec::new();
        for f in self.prog.frost.iter_mut() {
            f.1 -= dt;
            if f.1 <= 0.0 {
                melt.push(f.0);
            }
        }
        self.prog.frost.retain(|f| f.1 > 0.0);
        for p in melt {
            if self.world.block_or_air(p) == frosted {
                self.world.set_block(p, water);
            }
        }
    }

    fn brew_tick(&mut self, p: IVec3, dt: f32) {
        let Some(Container::Chest(inv)) = self.surv.containers.get(&p) else { return };
        let inv = inv.clone();
        let ingredient = inv.slots[3].map(|s| self.items.get(s.item).name.clone()).unwrap_or_default();
        let bottles: Vec<(usize, String)> = (0..3).filter_map(|i| inv.slots[i].map(|s| (i, self.items.get(s.item).name.clone()))).collect();
        let results: Vec<(usize, String)> = bottles
            .iter()
            .filter_map(|(i, n)| {
                let (form, pot) = crate::effects::potion_of(n)?;
                let next = if ingredient == "gunpowder" && form == crate::effects::Form::Drink {
                    format!("splash_potion_{}", pot.key)
                } else if ingredient == "dragon_breath" && form == crate::effects::Form::Splash {
                    format!("lingering_potion_{}", pot.key)
                } else {
                    let key = brew(pot.key, &ingredient)?;
                    match form {
                        crate::effects::Form::Drink => format!("potion_{key}"),
                        crate::effects::Form::Splash => format!("splash_potion_{key}"),
                        crate::effects::Form::Lingering => format!("lingering_potion_{key}"),
                        crate::effects::Form::Arrow => return None,
                    }
                };
                self.items.id(&next).map(|_| (*i, next))
            })
            .collect();
        let mut b = self.prog.brews.get(&p).copied().unwrap_or_default();
        if results.is_empty() {
            b.time = 0.0;
            self.prog.brews.insert(p, b);
            return;
        }
        if b.time <= 0.0 {
            if b.fuel == 0 {
                let has_powder = inv.slots[4].is_some_and(|s| self.items.get(s.item).name == "blaze_powder");
                if !has_powder {
                    return;
                }
                if let Some(Container::Chest(i)) = self.surv.containers.get_mut(&p) {
                    i.take(4, 1);
                }
                b.fuel = 20;
            }
            b.fuel -= 1;
            b.time = 20.0;
        }
        b.time -= dt;
        if b.time <= 0.0 {
            b.time = 0.0;
            if let Some(Container::Chest(i)) = self.surv.containers.get_mut(&p) {
                for (k, name) in &results {
                    if let Some(id) = self.items.id(name) {
                        i.slots[*k] = Some(ItemStack::new(id, 1));
                    }
                }
                i.take(3, 1);
                // Dragon's breath leaves its bottle behind.
                if ingredient == "dragon_breath" {
                    if let Some(gb) = self.items.id("glass_bottle") {
                        if i.slots[3].is_none() {
                            i.slots[3] = Some(ItemStack::new(gb, 1));
                        }
                    }
                }
            }
            if p.as_vec3().distance(self.body.pos) < 16.0 {
                self.sfx(Sfx::Bubbles);
            }
            self.prog.stats.add("potions_brewed", results.len() as u64);
            self.prog.event(crate::progression::Event::Brewed);
        } else if p.as_vec3().distance(self.body.pos) < 12.0 && self.prog.rng.f32() < dt * 2.0 {
            let glow = self.glow_layer();
            self.particles.glow(p.as_vec3() + Vec3::new(0.5, 0.9, 0.5), Vec3::Y * 0.5, glow, [0.9, 0.7, 1.2], 0.05, 0.8);
        }
        self.prog.brews.insert(p, b);
    }

    /// Height of the full layers of metal blocks under a beacon (0..4).
    pub(crate) fn pyramid_levels(&self, p: IVec3) -> u8 {
        let ids: Vec<voxel::BlockId> = PYRAMID.iter().map(|n| self.reg.id_or_air(n)).filter(|&i| i != AIR).collect();
        let mut levels = 0;
        for l in 1..=4 {
            let y = p.y - l;
            for dz in -l..=l {
                for dx in -l..=l {
                    if !ids.contains(&self.world.block_or_air(IVec3::new(p.x + dx, y, p.z + dz))) {
                        return levels;
                    }
                }
            }
            levels = l as u8;
        }
        levels
    }

    fn beacon_pulse(&mut self) {
        let list: Vec<(IVec3, crate::progression::BeaconState)> = self.prog.beacons.iter().map(|(p, b)| (*p, *b)).collect();
        for (p, mut b) in list {
            if self.reg.get(self.world.block_or_air(p)).name != "beacon" {
                if self.world.block(p).is_some() {
                    self.prog.beacons.remove(&p);
                }
                continue;
            }
            b.levels = self.pyramid_levels(p);
            self.prog.beacons.insert(p, b);
            if b.levels == 0 || b.primary == 0 {
                continue;
            }
            let range = 10.0 + 10.0 * b.levels as f32;
            let d = self.body.pos - p.as_vec3();
            if d.x.abs() > range || d.z.abs() > range {
                continue;
            }
            let strong = b.levels >= 4 && b.secondary == b.primary;
            self.add_effect(b.primary, if strong { 1 } else { 0 }, 13.0, true);
            if b.levels >= 4 && b.secondary != 0 && b.secondary != b.primary {
                self.add_effect(b.secondary, 0, 13.0, true);
            }
        }
    }

    /// Conduits: a frame of prismarine and sea lanterns around one, under water, gives
    /// conduit power to the player in water nearby (16 blocks for each 7 frame blocks).
    fn conduit_pulse(&mut self) {
        let conduit = self.reg.id_or_air("conduit");
        if conduit == AIR {
            return;
        }
        let frame: Vec<voxel::BlockId> = ["prismarine", "prismarine_bricks", "dark_prismarine", "sea_lantern"].iter().map(|n| self.reg.id_or_air(n)).filter(|&i| i != AIR).collect();
        let c = self.body.pos.floor().as_ivec3();
        let mut best = 0;
        for dy in -12..=12 {
            for dz in -12..=12 {
                for dx in -12..=12 {
                    let p = c + IVec3::new(dx, dy, dz);
                    if self.world.block_or_air(p) != conduit {
                        continue;
                    }
                    let mut n = 0;
                    for fy in -2..=2i32 {
                        for fz in -2..=2i32 {
                            for fx in -2..=2i32 {
                                // The frame is three rings 5 wide, through the conduit's centre.
                                let on_ring = [fx == 0, fy == 0, fz == 0].iter().filter(|&&b| b).count() >= 1 && (fx.abs() == 2 || fy.abs() == 2 || fz.abs() == 2);
                                if on_ring && frame.contains(&self.world.block_or_air(p + IVec3::new(fx, fy, fz))) {
                                    n += 1;
                                }
                            }
                        }
                    }
                    let water = self.world.is_liquid(self.world.block_or_air(p + IVec3::Y));
                    if n >= 16 && water {
                        let range = (n / 7 * 16) as f32;
                        if (p.as_vec3() - self.body.pos).length() < range {
                            best = best.max(n);
                        }
                    }
                }
            }
        }
        if best > 0 && (self.body.in_water || self.weather.rain > 0.3) {
            self.add_effect(crate::effects::CONDUIT_POWER, 0, 13.0, true);
            self.prog.event(crate::progression::Event::ConduitOn);
        }
    }

    pub(crate) fn _station_item(&self, name: &str) -> Option<ItemId> {
        self.items.id(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brewing_chains() {
        assert_eq!(brew("water", "nether_wart").as_deref(), Some("awkward"));
        assert_eq!(brew("awkward", "sugar").as_deref(), Some("swiftness"));
        assert_eq!(brew("swiftness", "redstone").as_deref(), Some("long_swiftness"));
        assert_eq!(brew("swiftness", "glowstone_dust").as_deref(), Some("strong_swiftness"));
        assert_eq!(brew("long_swiftness", "fermented_spider_eye").as_deref(), Some("long_slowness"));
        assert_eq!(brew("strong_healing", "fermented_spider_eye").as_deref(), Some("strong_harming"));
        assert_eq!(brew("healing", "redstone"), None);
        assert_eq!(brew("awkward", "dirt"), None);
        // Every brew lands on a real potion.
        for p in crate::effects::POTIONS {
            for ing in ["nether_wart", "redstone", "glowstone_dust", "fermented_spider_eye", "sugar", "rabbit_foot", "spider_eye"] {
                if let Some(out) = brew(p.key, ing) {
                    assert!(crate::effects::potion(&out).is_some(), "{} + {ing} -> {out}", p.key);
                }
            }
        }
    }

    #[test]
    fn levels_round_trip() {
        for l in 0..40 {
            assert_eq!(xp_total(l + 1) - xp_total(l), xp_for_level(l));
        }
    }
}
