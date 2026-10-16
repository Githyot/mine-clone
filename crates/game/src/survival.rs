//! Survival play: the 36-slot inventory, items dropped in the world, experience, hunger,
//! crafting (2×2 in the inventory, 3×3 at a crafting table), furnaces, chests, beds and
//! saplings that grow. The creative hotbar stays as it was; this only runs in survival.

use std::collections::HashMap;
use std::path::Path;

use glam::{IVec3, Vec3};
use serde::{Deserialize, Serialize};
use voxel::block::{Shape, ToolKind};
use voxel::item::{FoodDef, Inventory, ItemId, ItemRegistry, ItemStack, Look, ToolDef};
use voxel::physics::{Body, Input, Terrain};
use voxel::rng::Rng;
use voxel::trees::{self, TreeBlocks, TreeKind, TreeSpace};
use voxel::{BlockId, BlockRegistry, AIR};

use crate::audio::{Material, Sfx};
use crate::item_render::GpuItem;
use crate::ui::{UiBatch, WHITE};
use crate::{Game, Screen};

/// Player inventory: 0..9 is the hotbar, 9..36 the main rows.
pub const INV_SIZE: usize = 36;
const CHEST_SIZE: usize = 27;
const SMELT_TIME: f32 = 10.0;
/// Seconds before a dropped item can be picked up; items the player throws wait longer.
const PICKUP_DELAY: f32 = 0.5;
const THROWN_DELAY: f32 = 2.0;
const DESPAWN: f32 = 300.0;

pub struct Dropped {
    pub stack: ItemStack,
    pub body: Body,
    pub age: f32,
    pub spin: f32,
    /// Fixed turn when lying on the ground.
    pub spin_rest: f32,
}

pub struct Orb {
    pub pos: Vec3,
    pub vel: Vec3,
    pub value: u32,
    pub age: f32,
}

pub struct Furnace {
    /// 0 input, 1 fuel, 2 output.
    pub inv: Inventory,
    pub burn: f32,
    pub burn_max: f32,
    pub cook: f32,
    pub xp: f32,
}

/// The player's body left where they died, holding everything they carried (like the Corpse
/// mod): nothing scatters or despawns, right-click it to take it all back.
pub struct Corpse {
    pub pos: Vec3,
    pub yaw: f32,
    pub items: Vec<ItemStack>,
}

pub enum Container {
    Chest(Inventory),
    Furnace(Furnace),
}

/// The tab open beside the inventory: nothing, every item, or what can be built right now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Closed,
    Items,
    Build,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Open {
    /// The player's own inventory with the 2×2 grid.
    Craft,
    Table,
    Furnace(IVec3),
    Chest(IVec3),
}

pub struct Survival {
    pub inv: Inventory,
    /// Worn armor: head, chest, legs, feet.
    pub armor: [Option<ItemStack>; 4],
    pub cursor: Option<ItemStack>,
    pub craft: [Option<ItemStack>; 9],
    pub open: Open,
    pub hunger: f32,
    pub saturation: f32,
    pub exhaustion: f32,
    /// Total experience points.
    pub xp: u32,
    pub spawn: Option<IVec3>,
    pub containers: HashMap<IVec3, Container>,
    pub drops: Vec<Dropped>,
    pub orbs: Vec<Orb>,
    pub corpses: Vec<Corpse>,
    /// The second hand's slot (holds anything; nothing uses it yet).
    pub offhand: Option<ItemStack>,
    /// Creative inventory: open tab, first row shown, and the search text.
    pub ctab: usize,
    pub cscroll: usize,
    pub csearch: String,
    /// The creative screen is drawn with a pack's art (its slots are part of the picture).
    pub cart: bool,
    /// Recipe book: first row shown, and the item whose recipes are open (with which one).
    pub book_scroll: usize,
    pub book_view: Option<(ItemId, usize)>,
    /// Which list the tabs beside the inventory show.
    pub side: Side,
    /// R was pressed over an item: show its recipes.
    pub want_recipe: bool,
    /// Screen box where the 3D player figure goes this frame (inventory open).
    pub figure_box: Option<(f32, f32, f32, f32)>,
    /// Slot the right-button drag last dropped into.
    drag_last: Option<Slot>,
    /// Saplings planted by the player and seconds until they grow.
    pub saplings: Vec<(IVec3, f32)>,
    /// Leaves of a felled tree, falling a few at a time.
    pub leaf_decay: Vec<IVec3>,
    pub eat: f32,
    /// Seconds since the last swing; a hit only lands at full strength once the weapon is ready.
    pub attack_t: f32,
    eat_sound: f32,
    heal_t: f32,
    starve_t: f32,
    pickup_sound: f32,
    last_pos: Vec3,
    was_ground: bool,
    pub(crate) rng: Rng,
}

impl Survival {
    pub fn new() -> Survival {
        Survival {
            inv: Inventory::new(INV_SIZE),
            armor: [None; 4],
            cursor: None,
            craft: [None; 9],
            open: Open::Craft,
            hunger: 20.0,
            saturation: 5.0,
            exhaustion: 0.0,
            xp: 0,
            spawn: None,
            containers: HashMap::new(),
            drops: Vec::new(),
            orbs: Vec::new(),
            corpses: Vec::new(),
            offhand: None,
            ctab: 0,
            cscroll: 0,
            csearch: String::new(),
            cart: false,
            book_scroll: 0,
            book_view: None,
            side: Side::Closed,
            want_recipe: false,
            figure_box: None,
            drag_last: None,
            saplings: Vec::new(),
            leaf_decay: Vec::new(),
            eat: 0.0,
            attack_t: 10.0,
            eat_sound: 0.0,
            heal_t: 0.0,
            starve_t: 0.0,
            pickup_sound: 0.0,
            last_pos: Vec3::ZERO,
            was_ground: true,
            rng: Rng::new(0x5eed),
        }
    }

    pub fn grid_size(&self) -> usize {
        if self.open == Open::Table {
            3
        } else {
            2
        }
    }

    /// (level, progress 0..1 to the next one) from total points, using the classic curve.
    pub fn level(&self) -> (u32, f32) {
        let mut left = self.xp;
        let mut lvl = 0;
        loop {
            let need = xp_for_level(lvl);
            if left < need {
                return (lvl, left as f32 / need as f32);
            }
            left -= need;
            lvl += 1;
        }
    }
}

fn xp_for_level(l: u32) -> u32 {
    if l < 16 {
        2 * l + 7
    } else if l < 31 {
        5 * l - 38
    } else {
        9 * l - 158
    }
}

// ---------------------------------------------------------------------------------------------
// Saving: by item name, so adding items to the game never breaks old worlds.

type SavedStack = Option<(String, u8, u16)>;

#[derive(Serialize, Deserialize, Default)]
struct SavedContainer {
    pos: [i32; 3],
    furnace: bool,
    slots: Vec<SavedStack>,
    #[serde(default)]
    burn: f32,
    #[serde(default)]
    burn_max: f32,
    #[serde(default)]
    cook: f32,
}

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    inv: Vec<SavedStack>,
    hunger: f32,
    saturation: f32,
    xp: u32,
    spawn: Option<[i32; 3]>,
    containers: Vec<SavedContainer>,
    #[serde(default)]
    saplings: Vec<([i32; 3], f32)>,
    #[serde(default)]
    armor: Vec<SavedStack>,
    #[serde(default)]
    corpses: Vec<([f32; 3], f32, Vec<SavedStack>)>,
    #[serde(default)]
    offhand: SavedStack,
}

fn save_stacks(items: &ItemRegistry, inv: &Inventory) -> Vec<SavedStack> {
    inv.slots.iter().map(|s| s.map(|s| (items.get(s.item).name.clone(), s.count, s.damage))).collect()
}

fn load_stacks(items: &ItemRegistry, v: &[SavedStack], n: usize) -> Inventory {
    let mut inv = Inventory::new(n);
    for (i, s) in v.iter().enumerate().take(n) {
        if let Some((name, count, damage)) = s {
            if let Some(id) = items.id(name) {
                inv.slots[i] = Some(ItemStack { item: id, count: *count, damage: *damage });
            }
        }
    }
    inv
}

impl Survival {
    pub fn save(&self, dir: &Path, items: &ItemRegistry) {
        let mut containers = Vec::new();
        for (p, c) in &self.containers {
            containers.push(match c {
                Container::Chest(inv) => SavedContainer { pos: p.to_array(), furnace: false, slots: save_stacks(items, inv), ..Default::default() },
                Container::Furnace(f) => SavedContainer { pos: p.to_array(), furnace: true, slots: save_stacks(items, &f.inv), burn: f.burn, burn_max: f.burn_max, cook: f.cook },
            });
        }
        let s = Saved {
            inv: save_stacks(items, &self.inv),
            hunger: self.hunger,
            saturation: self.saturation,
            xp: self.xp,
            spawn: self.spawn.map(|p| p.to_array()),
            containers,
            saplings: self.saplings.iter().map(|(p, t)| (p.to_array(), *t)).collect(),
            armor: self.armor.iter().map(|s| s.map(|s| (items.get(s.item).name.clone(), s.count, s.damage))).collect(),
            offhand: self.offhand.map(|s| (items.get(s.item).name.clone(), s.count, s.damage)),
            corpses: self
                .corpses
                .iter()
                .map(|c| (c.pos.to_array(), c.yaw, c.items.iter().map(|s| Some((items.get(s.item).name.clone(), s.count, s.damage))).collect()))
                .collect(),
        };
        if let Ok(text) = ron::ser::to_string_pretty(&s, Default::default()) {
            let _ = std::fs::create_dir_all(dir);
            if let Err(e) = std::fs::write(dir.join("survival.ron"), text) {
                log::error!("could not save survival.ron: {e}");
            }
        }
    }

    pub fn load(dir: &Path, items: &ItemRegistry) -> Survival {
        let mut sv = Survival::new();
        let Some(s) = std::fs::read_to_string(dir.join("survival.ron")).ok().and_then(|t| ron::from_str::<Saved>(&t).ok()) else {
            return sv;
        };
        sv.inv = load_stacks(items, &s.inv, INV_SIZE);
        sv.hunger = s.hunger.clamp(0.0, 20.0);
        sv.saturation = s.saturation.clamp(0.0, 20.0);
        sv.xp = s.xp;
        sv.spawn = s.spawn.map(IVec3::from);
        for c in s.containers {
            let pos = IVec3::from(c.pos);
            let cont = if c.furnace {
                Container::Furnace(Furnace { inv: load_stacks(items, &c.slots, 3), burn: c.burn, burn_max: c.burn_max, cook: c.cook, xp: 0.0 })
            } else {
                Container::Chest(load_stacks(items, &c.slots, CHEST_SIZE))
            };
            sv.containers.insert(pos, cont);
        }
        for (pos, yaw, list) in &s.corpses {
            let inv = load_stacks(items, list, list.len());
            sv.corpses.push(Corpse { pos: Vec3::from(*pos), yaw: *yaw, items: inv.slots.into_iter().flatten().collect() });
        }
        let worn = load_stacks(items, &s.armor, 4);
        for (i, a) in worn.slots.into_iter().enumerate() {
            sv.armor[i] = a;
        }
        sv.saplings = s.saplings.into_iter().map(|(p, t)| (IVec3::from(p), t)).collect();
        sv.offhand = load_stacks(items, std::slice::from_ref(&s.offhand), 1).slots[0];
        sv
    }
}

// ---------------------------------------------------------------------------------------------
// Drawing items in the interface.

/// An item's icon, with its count and wear bar, in a `size` square.
pub fn item_icon(ui: &mut UiBatch, blocks: &BlockRegistry, items: &ItemRegistry, item_base: u16, stack: &ItemStack, x: f32, y: f32, size: f32) {
    let it = items.get(stack.item);
    match it.look {
        Look::Block(b) => ui.block_icon(blocks, b, x, y, size),
        Look::Sprite(l) => {
            let full = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
            ui.quad([[x, y], [x + size, y], [x + size, y + size], [x, y + size]], full, WHITE, (item_base + l) as i32);
        }
    }
    let s = ui.scale;
    let durability = it.tool.as_ref().map(|t| t.durability).or(it.armor.map(|a| a.durability));
    if let Some(dur) = durability {
        if stack.damage > 0 {
            let k = 1.0 - stack.damage as f32 / dur as f32;
            let bw = size - 2.0 * s;
            let by = y + size - 2.0 * s;
            ui.rect(x + s, by, bw, 2.0 * s, [0, 0, 0, 255]);
            let c = [((1.0 - k) * 255.0) as u8, (k * 255.0) as u8, 0, 255];
            ui.rect(x + s, by, (bw * k).max(s), s, c);
        }
    }
    if stack.count > 1 {
        let t = stack.count.to_string();
        let tw = ui.text_width(&t, s);
        ui.text(x + size - tw + 2.0 * s, y + size - 6.0 * s, &t, [62, 62, 62, 255]);
        ui.text(x + size - tw + s, y + size - 7.0 * s, &t, WHITE);
    }
}

/// Fill `r` except for the `hole` rectangle inside it.
pub(crate) fn rect_around(ui: &mut UiBatch, r: (f32, f32, f32, f32), hole: Option<(f32, f32, f32, f32)>, c: [u8; 4]) {
    let (x, y, w, h) = r;
    let Some((hx, hy, hw, hh)) = hole else {
        ui.rect(x, y, w, h, c);
        return;
    };
    ui.rect(x, y, w, hy - y, c);
    ui.rect(x, hy + hh, w, y + h - hy - hh, c);
    ui.rect(x, hy, hx - x, hh, c);
    ui.rect(hx + hw, hy, x + w - hx - hw, hh, c);
}

fn draw_drumstick(ui: &mut UiBatch, x: f32, y: f32, s: f32, fill: f32, empty: bool) {
    // Seven by seven pixels: meat at the top right, bone at the bottom left.
    const ROWS: [&str; 7] = ["...MMM.", "..MMMMM", "..MMMMM", "..MMMM.", ".B.MM..", "B.B....", ".B....."];
    for (r, row) in ROWS.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            // Half a drumstick hides its left side.
            if fill < 1.0 && c < 3 {
                continue;
            }
            let col = match ch {
                'M' if empty => [40, 20, 10, 255],
                'M' => [180, 100, 50, 255],
                'B' if empty => [50, 50, 50, 255],
                'B' => [230, 225, 210, 255],
                _ => continue,
            };
            ui.rect(x + c as f32 * s, y + r as f32 * s, s, s, col);
        }
    }
}

fn draw_shield(ui: &mut UiBatch, x: f32, y: f32, s: f32, fill: f32, empty: bool) {
    // A little chestplate, seven pixels square.
    const ROWS: [&str; 7] = ["AA...AA", "AAAAAAA", ".AAAAA.", ".AAAAA.", ".AAAAA.", ".AAAAA.", "..AAA.."];
    for (r, row) in ROWS.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            if ch != 'A' || (fill < 1.0 && c > 3) {
                continue;
            }
            let col = if empty { [45, 45, 45, 255] } else if r == 1 || c == 1 { [235, 235, 235, 255] } else { [190, 190, 190, 255] };
            ui.rect(x + c as f32 * s, y + r as f32 * s, s, s, col);
        }
    }
}

/// One clickable slot in a container screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Slot {
    Inv(usize),
    Craft(usize),
    CraftOut,
    Cont(usize),
    Armor(usize),
    Offhand,
}

// ---------------------------------------------------------------------------------------------

impl Game {
    pub(crate) fn held_stack(&self) -> Option<ItemStack> {
        self.surv.inv.slots[self.selected]
    }

    pub(crate) fn held_tool(&self) -> Option<ToolDef> {
        self.held_stack().and_then(|s| self.items.get(s.item).tool.clone())
    }

    pub(crate) fn armor_points(&self) -> u32 {
        self.surv.armor.iter().flatten().filter_map(|s| self.items.get(s.item).armor).map(|a| a.points as u32).sum()
    }

    /// Armor takes a share of a hit (4% per point, up to 80%) and wears a little.
    pub(crate) fn armor_absorb(&mut self, amount: f32) -> f32 {
        let points = self.armor_points().min(20);
        if points == 0 {
            return amount;
        }
        for i in 0..4 {
            let Some(st) = self.surv.armor[i] else { continue };
            let Some(a) = self.items.get(st.item).armor else { continue };
            let d = st.damage + (amount / 4.0).max(1.0) as u16;
            self.surv.armor[i] = if d >= a.durability { None } else { Some(ItemStack { damage: d, ..st }) };
            if d >= a.durability {
                self.sfx(Sfx::Break(Material::Stone));
                self.toast("Uma peça da armadura quebrou", 2.0);
            }
        }
        amount * (1.0 - points as f32 * 0.04)
    }

    /// Seconds a weapon needs between full-strength swings (the hand is quick, an axe slow).
    pub(crate) fn attack_cooldown(&self) -> f32 {
        match self.held_tool().map(|t| t.kind) {
            None => 0.25,
            Some(ToolKind::Sword) => 0.625,
            Some(ToolKind::Axe) => 1.0,
            Some(ToolKind::Pickaxe) => 0.83,
            Some(ToolKind::Shovel) => 1.0,
            Some(ToolKind::Hoe) => 0.5,
        }
    }

    /// 0..1, how ready the next swing is.
    pub(crate) fn attack_charge(&self) -> f32 {
        (self.surv.attack_t / self.attack_cooldown()).min(1.0)
    }

    /// Put on the armor in hand, swapping with what was worn.
    fn equip_held(&mut self) -> bool {
        let sel = self.selected;
        let Some(st) = self.surv.inv.slots[sel] else { return false };
        let Some(a) = self.items.get(st.item).armor else { return false };
        let slot = (a.slot as usize).min(3);
        self.surv.inv.slots[sel] = self.surv.armor[slot].take();
        self.surv.armor[slot] = Some(st);
        self.sfx(Sfx::Place(Material::Cloth));
        true
    }

    /// Wear the held tool; it breaks when worn out.
    pub(crate) fn wear_held(&mut self, amount: u16) {
        let sel = self.selected;
        let Some(s) = self.surv.inv.slots[sel] else { return };
        let Some(t) = self.items.get(s.item).tool.clone() else { return };
        let d = s.damage + amount;
        if d >= t.durability {
            self.surv.inv.slots[sel] = None;
            self.sfx(Sfx::Break(Material::Wood));
            self.toast("Sua ferramenta quebrou", 2.0);
        } else {
            self.surv.inv.slots[sel] = Some(ItemStack { damage: d, ..s });
        }
    }

    pub(crate) fn spawn_drop(&mut self, at: Vec3, stack: ItemStack, vel: Vec3) {
        let mut body = Body::sized(at, 0.25, 0.25);
        body.vel = vel;
        body.no_jump = true;
        let spin = self.surv.rng.f32() * 6.28;
        self.surv.drops.push(Dropped { stack, body, age: 0.0, spin, spin_rest: spin });
    }

    fn pop_drop(&mut self, center: Vec3, stack: ItemStack) {
        let r = &mut self.surv.rng;
        let v = Vec3::new(r.f32() - 0.5, 0.0, r.f32() - 0.5) * 2.0 + Vec3::Y * 3.0;
        self.spawn_drop(center, stack, v);
    }

    pub(crate) fn spawn_orbs(&mut self, at: Vec3, mut total: u32) {
        while total > 0 {
            let v = total.min(if total > 16 { 7 } else { 3 });
            total -= v;
            let r = &mut self.surv.rng;
            let vel = Vec3::new(r.f32() - 0.5, 1.0 + r.f32(), r.f32() - 0.5) * 2.5;
            self.surv.orbs.push(Orb { pos: at, vel, value: v, age: 0.0 });
        }
    }

    /// A block was broken by the player in survival: drops, experience, tool wear.
    /// A block blown up: like the classic game, only some of what it would drop survives the
    /// blast (`keep` is that chance); chests and furnaces always spill their contents.
    pub(crate) fn blast_drops(&mut self, pos: IVec3, id: BlockId, keep: f32) {
        let c = pos.as_vec3() + Vec3::splat(0.5);
        if self.surv.rng.f32() < keep {
            let tool = self.items.id("diamond_pickaxe").and_then(|i| self.items.get(i).tool.clone());
            let drops = self.items.drops(&self.reg, id, tool.as_ref(), &mut self.surv.rng);
            for d in drops {
                self.pop_drop(c, d);
            }
        }
        if let Some(cont) = self.surv.containers.remove(&pos) {
            let inv = match cont {
                Container::Chest(i) => i,
                Container::Furnace(f) => f.inv,
            };
            for s in inv.slots.into_iter().flatten() {
                self.pop_drop(c, s);
            }
        }
        self.surv.saplings.retain(|(p, _)| *p != pos);
    }

    pub(crate) fn surv_block_broken(&mut self, pos: IVec3, id: BlockId) {
        let tool = self.held_tool();
        let drops = self.items.drops(&self.reg, id, tool.as_ref(), &mut self.surv.rng);
        let c = pos.as_vec3() + Vec3::splat(0.5);
        let harvested = self.items.can_harvest(&self.reg, id, tool.as_ref());
        for d in drops {
            self.pop_drop(c, d);
        }
        let (lo, hi) = self.reg.get(id).xp;
        if harvested && hi > 0 {
            let n = lo as i32 + self.surv.rng.below((hi - lo + 1) as u32) as i32;
            self.spawn_orbs(c, n.max(0) as u32);
        }
        if let Some(t) = &tool {
            if self.reg.get(id).hardness > 0.0 {
                self.wear_held(if t.kind == ToolKind::Sword { 2 } else { 1 });
            }
        }
        self.surv.exhaustion += 0.005;
        // A chest or furnace spills what it held.
        if let Some(cont) = self.surv.containers.remove(&pos) {
            let inv = match cont {
                Container::Chest(i) => i,
                Container::Furnace(f) => f.inv,
            };
            for s in inv.slots.into_iter().flatten() {
                self.pop_drop(c, s);
            }
        }
        self.surv.saplings.retain(|(p, _)| *p != pos);
        if self.surv.spawn == Some(pos + IVec3::Y) && self.reg.get(id).name.ends_with("_bed") {
            self.surv.spawn = None;
            self.toast("Sua cama foi destruída", 2.5);
        }
    }

    /// A chest the world generator placed comes filled from its building's loot list.
    fn structure_loot(&mut self, p: IVec3) -> Inventory {
        let mut inv = Inventory::new(CHEST_SIZE);
        let Some(loot) = self.world.gen.structure_chest(p).map(|l| l.to_vec()) else { return inv };
        for l in loot {
            if self.surv.rng.f32() >= l.chance {
                continue;
            }
            let Some(id) = self.items.id(&l.item) else { continue };
            let n = l.min + self.surv.rng.below((l.max.saturating_sub(l.min) + 1) as u32) as u8;
            // Scattered over the slots, like a chest someone filled by hand.
            let slot = self.surv.rng.below(CHEST_SIZE as u32) as usize;
            let order = (0..CHEST_SIZE).map(move |i| (i + slot) % CHEST_SIZE);
            let _ = inv.add_in(&self.items, ItemStack::new(id, n.max(1)), order);
        }
        inv
    }

    /// Falling Tree idea: chopping a log of a real tree (one with leaves) with an axe brings the
    /// whole trunk down, and its leaves fall soon after. Sneaking chops just the one block.
    pub(crate) fn fell_tree(&mut self, start: IVec3, id: BlockId) {
        let base = self.reg.get(id).base;
        let is_log = |g: &Game, p: IVec3| g.reg.get(g.world.block_or_air(p)).base == base;
        let is_leaf = |g: &Game, p: IVec3| g.reg.get(g.world.block_or_air(p)).name.ends_with("_leaves");
        let mut logs = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![start];
        seen.insert(start);
        let mut leafy = false;
        while let Some(p) = stack.pop() {
            for dy in 0..=1 {
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        let q = p + IVec3::new(dx, dy, dz);
                        if q.y < start.y || !seen.insert(q) {
                            continue;
                        }
                        if is_log(self, q) && logs.len() < 256 {
                            logs.push(q);
                            stack.push(q);
                        } else if is_leaf(self, q) {
                            leafy = true;
                        }
                    }
                }
            }
        }
        if !leafy || logs.is_empty() {
            return;
        }
        let tool = self.held_tool();
        logs.sort_by_key(|p| p.y);
        let mut wear = 0;
        for &p in &logs {
            let lid = self.world.block_or_air(p);
            self.world.set_block(p, AIR);
            let (layer, tint) = self.block_look(lid);
            self.particles.block_bits(p, layer, tint, 6, 0.8);
            if self.survival {
                let drops = self.items.drops(&self.reg, lid, tool.as_ref(), &mut self.surv.rng);
                for d in drops {
                    self.pop_drop(p.as_vec3() + Vec3::splat(0.5), d);
                }
                wear += 1;
            }
        }
        if wear > 0 {
            self.wear_held(wear);
        }
        self.sfx(Sfx::Break(Material::Wood));
        // Leaves no other trunk holds up.
        let mut lo = start;
        let mut hi = start;
        for p in &logs {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        for y in lo.y..=hi.y + 4 {
            for z in lo.z - 4..=hi.z + 4 {
                for x in lo.x - 4..=hi.x + 4 {
                    let p = IVec3::new(x, y, z);
                    if is_leaf(self, p) && !self.log_near(p) {
                        self.surv.leaf_decay.push(p);
                    }
                }
            }
        }
        // Fall in a random order.
        let n = self.surv.leaf_decay.len();
        for i in (1..n).rev() {
            let j = self.surv.rng.below(i as u32 + 1) as usize;
            self.surv.leaf_decay.swap(i, j);
        }
    }

    fn log_near(&self, p: IVec3) -> bool {
        for z in -4..=4 {
            for y in -4..=4 {
                for x in -4..=4 {
                    if self.reg.get(self.world.block_or_air(p + IVec3::new(x, y, z))).name.ends_with("_log") {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Drop a few of the felled tree's leaves each frame.
    pub(crate) fn update_leaf_decay(&mut self, dt: f32) {
        let n = ((dt * 60.0).ceil() as usize).min(self.surv.leaf_decay.len());
        for p in self.surv.leaf_decay.drain(..n).collect::<Vec<_>>() {
            let id = self.world.block_or_air(p);
            if !self.reg.get(id).name.ends_with("_leaves") {
                continue;
            }
            self.world.set_block(p, AIR);
            let (layer, tint) = self.block_look(id);
            self.particles.block_bits(p, layer, tint, 2, 0.5);
            if self.survival {
                let drops = self.items.drops(&self.reg, id, None, &mut self.surv.rng);
                for d in drops {
                    self.pop_drop(p.as_vec3() + Vec3::splat(0.5), d);
                }
            }
        }
    }

    /// Mobs killed this tick drop their items and experience.
    pub(crate) fn surv_mob_deaths(&mut self, deaths: &[(usize, Vec3)]) {
        for &(def, pos) in deaths {
            let d = &self.mobs.reg.defs[def];
            let list = d.drops.clone();
            let xp = d.xp as u32;
            for dd in list {
                if self.surv.rng.f32() >= dd.chance {
                    continue;
                }
                let Some(i) = self.items.id(&dd.item) else { continue };
                let n = dd.min as i32 + self.surv.rng.below((dd.max.max(dd.min) - dd.min + 1) as u32) as i32;
                if n > 0 {
                    self.pop_drop(pos + Vec3::Y * 0.5, ItemStack::new(i, n as u8));
                }
            }
            self.spawn_orbs(pos + Vec3::Y * 0.5, xp);
        }
    }

    /// Melee damage of what is in hand, and wear on it.
    pub(crate) fn surv_attack_damage(&mut self) -> f32 {
        self.surv.exhaustion += 0.1;
        let k = self.attack_charge();
        self.surv.attack_t = 0.0;
        let base = match self.held_tool() {
            Some(t) => {
                self.wear_held(if t.kind == ToolKind::Sword { 1 } else { 2 });
                t.damage
            }
            None => 1.0,
        };
        base * (0.2 + 0.8 * k * k)
    }

    pub(crate) fn surv_died(&mut self) {
        let at = self.body.pos + Vec3::Y * 0.8;
        // Armor first, so taking it back puts it on again.
        let mut items: Vec<ItemStack> = self.surv.armor.iter_mut().filter_map(|s| s.take()).collect();
        items.extend(self.surv.inv.slots.iter_mut().filter_map(|s| s.take()));
        items.extend(self.surv.cursor.take());
        items.extend(self.surv.craft.iter_mut().filter_map(|s| s.take()));
        if !items.is_empty() {
            // Out of the void or lava, the body rests on the ground above.
            let mut pos = self.body.pos;
            pos.y = pos.y.max(1.0);
            let p = pos.floor().as_ivec3();
            let mut q = p;
            while q.y < p.y + 40 && (Terrain::solid(&self.world, q) || self.world.fluid(self.world.block_or_air(q)) == 2) {
                q.y += 1;
            }
            pos.y = q.y as f32;
            let msg = format!("Seus itens ficaram no seu corpo em {} {} {}", pos.x.floor(), pos.y.floor(), pos.z.floor());
            self.surv.corpses.push(Corpse { pos, yaw: self.yaw, items });
            self.toast(msg, 8.0);
        }
        let (lvl, _) = self.surv.level();
        let keep = (lvl * 7).min(100);
        self.surv.xp = 0;
        self.spawn_orbs(at, keep);
    }

    pub(crate) fn surv_respawned(&mut self) {
        self.surv.hunger = 20.0;
        self.surv.saturation = 5.0;
        self.surv.exhaustion = 0.0;
        if let Some(p) = self.surv.spawn {
            let bed = self.world.block_or_air(p - IVec3::Y);
            if self.reg.get(bed).name.ends_with("_bed") {
                self.body.pos = p.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
            } else {
                self.surv.spawn = None;
                self.toast("Sua cama sumiu ou estava bloqueada", 3.0);
            }
        }
    }

    /// Right click in survival. Returns true when something happened.
    pub(crate) fn surv_use(&mut self) -> bool {
        if self.loot_corpse() {
            return true;
        }
        if let Some(hit) = self.target {
            let id = self.world.block_or_air(hit.block);
            let name = self.reg.get(id).name.clone();
            let sneaking = self.crouch > 0.5;
            if !sneaking || self.held_stack().is_none() {
                match name.as_str() {
                    "crafting_table" => {
                        self.open_container(Open::Table);
                        return true;
                    }
                    "furnace" => {
                        self.surv.containers.entry(hit.block).or_insert_with(|| {
                            Container::Furnace(Furnace { inv: Inventory::new(3), burn: 0.0, burn_max: 0.0, cook: 0.0, xp: 0.0 })
                        });
                        self.open_container(Open::Furnace(hit.block));
                        return true;
                    }
                    "chest" => {
                        if !self.surv.containers.contains_key(&hit.block) {
                            let inv = self.structure_loot(hit.block);
                            self.surv.containers.insert(hit.block, Container::Chest(inv));
                        }
                        self.open_container(Open::Chest(hit.block));
                        self.sfx(Sfx::Place(Material::Wood));
                        return true;
                    }
                    n if n.ends_with("_bed") => {
                        self.use_bed(hit.block);
                        return true;
                    }
                    _ => {}
                }
            }
        }
        let Some(stack) = self.held_stack() else { return false };
        let item = self.items.get(stack.item).clone();
        if item.armor.is_some() {
            return self.equip_held();
        }
        if item.food.is_some() {
            // Eating is handled while the button is held (update).
            return false;
        }
        if item.name == "fishing_rod" {
            return self.use_rod();
        }
        if matches!(item.name.as_str(), "bucket" | "water_bucket" | "lava_bucket") {
            return self.use_bucket(&item.name);
        }
        if let Some(mob) = item.name.strip_suffix("_spawn_egg") {
            let Some(hit) = self.target else { return false };
            let at = (hit.block + hit.normal).as_vec3() + Vec3::new(0.5, 0.0, 0.5);
            if self.mobs.spawn(mob, at) {
                if self.survival {
                    let sel = self.selected;
                    self.surv.inv.take(sel, 1);
                }
                self.anim.start_swing();
                return true;
            }
            return false;
        }
        let Some(block) = item.places else { return false };
        if let Some(pos) = self.place_block_as(block) {
            if self.survival {
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
            }
            if self.reg.get(block).name.ends_with("_sapling") {
                let t = 60.0 + self.surv.rng.f32() * 120.0;
                self.surv.saplings.push((pos, t));
            }
            return true;
        }
        false
    }

    /// Right-click on a corpse: everything comes back, armor onto the body.
    fn loot_corpse(&mut self) -> bool {
        let eye = self.body.eye();
        let f = self.forward();
        let block_dist = self.target.map(|t| t.distance).unwrap_or(f32::MAX);
        let mut best: Option<(usize, f32)> = None;
        for (i, c) in self.surv.corpses.iter().enumerate() {
            let lo = c.pos - Vec3::new(0.9, 0.0, 0.9);
            let hi = c.pos + Vec3::new(0.9, 0.5, 0.9);
            if let Some(t) = crate::mobs::ray_box(eye, f, lo, hi) {
                if t < 4.0 && t <= block_dist + 0.5 && best.is_none_or(|(_, b)| t < b) {
                    best = Some((i, t));
                }
            }
        }
        let Some((i, _)) = best else { return false };
        let c = self.surv.corpses.swap_remove(i);
        for st in c.items {
            if let Some(a) = self.items.get(st.item).armor {
                let slot = (a.slot as usize).min(3);
                if self.surv.armor[slot].is_none() {
                    self.surv.armor[slot] = Some(st);
                    continue;
                }
            }
            if let Some(left) = self.surv.inv.add(&self.items, st) {
                self.pop_drop(c.pos + Vec3::Y * 0.5, left);
            }
        }
        self.sfx(Sfx::Pickup);
        self.toast("Você pegou seus itens de volta", 2.5);
        true
    }

    fn open_container(&mut self, open: Open) {
        self.surv.open = open;
        self.set_screen(Screen::Inventory);
    }

    /// Open the inventory (E) in survival.
    pub(crate) fn surv_open_inventory(&mut self) {
        self.surv.open = Open::Craft;
        self.set_screen(Screen::Inventory);
    }

    /// Closing a screen gives back what was left on the crafting grid and the cursor.
    pub(crate) fn surv_close(&mut self) {
        let mut back: Vec<ItemStack> = self.surv.craft.iter_mut().filter_map(|s| s.take()).collect();
        if let Some(c) = self.surv.cursor.take() {
            back.push(c);
        }
        for s in back {
            if let Some(left) = self.surv.inv.add(&self.items, s) {
                let f = self.forward();
                self.spawn_drop(self.body.eye() - Vec3::Y * 0.3, left, f * 3.0);
            }
        }
    }

    fn use_bed(&mut self, pos: IVec3) {
        let above = pos + IVec3::Y;
        self.surv.spawn = Some(above);
        let t = self.time_of_day;
        if (12500.0..23500.0).contains(&t) {
            self.time_of_day = 0.0;
            self.toast("Você dormiu. Bom dia!", 3.0);
        } else {
            self.toast("Ponto de renascimento definido. Só dá para dormir à noite.", 3.5);
        }
    }

    /// Throw the held item (Q); all of it with Ctrl.
    pub(crate) fn surv_throw(&mut self, all: bool) {
        let sel = self.selected;
        let n = if all { 64 } else { 1 };
        if let Some(s) = self.surv.inv.take(sel, n) {
            let f = self.forward();
            self.spawn_drop(self.body.eye() - Vec3::Y * 0.3, s, f * 5.0 + Vec3::Y);
            self.mark_thrown();
        }
    }

    /// Throw a stack out in front of the player, like dropping it out of the inventory window.
    /// Scoop up a still liquid with an empty bucket, or pour one out where the player looks.
    fn use_bucket(&mut self, name: &str) -> bool {
        let eye = self.body.eye();
        let fwd = self.forward();
        if name == "bucket" {
            let world = &self.world;
            let hit = voxel::raycast::raycast(eye, fwd, 5.0, |p| {
                let id = world.block_or_air(p);
                (world.fluid(id) != 0 && world.reg.get(id).variant == 0) || Terrain::solid(world, p)
            });
            let Some(hit) = hit else { return false };
            let id = self.world.block_or_air(hit.block);
            let f = self.world.fluid(id);
            if f == 0 || self.reg.get(id).variant != 0 {
                return false;
            }
            self.world.set_block(hit.block, AIR);
            self.sfx(if f == 1 { Sfx::Splash(0.3) } else { Sfx::Fizz });
            self.swap_held(if f == 1 { "water_bucket" } else { "lava_bucket" });
            self.anim.start_swing();
            return true;
        }
        let Some(hit) = self.target else { return false };
        let clicked = self.world.block_or_air(hit.block);
        let pos = if self.reg.get(clicked).shape == Shape::Cross { hit.block } else { hit.block + hit.normal };
        let cur = self.world.block_or_air(pos);
        let fits = cur == AIR || self.reg.get(cur).shape == Shape::Cross || (self.world.fluid(cur) != 0 && self.reg.get(cur).variant != 0);
        if !fits {
            return false;
        }
        let f = if name == "water_bucket" { 1 } else { 2 };
        self.world.set_block(pos, self.fluids.id(f, 0));
        self.sfx(if f == 1 { Sfx::Splash(0.4) } else { Sfx::Fizz });
        self.swap_held("bucket");
        self.anim.start_swing();
        true
    }

    /// Turn one of the held item into another (a bucket filled or emptied). Creative keeps it.
    fn swap_held(&mut self, to: &str) {
        if !self.survival {
            return;
        }
        let Some(to) = self.items.id(to) else { return };
        let sel = self.selected;
        match self.surv.inv.slots[sel] {
            Some(s) if s.count <= 1 => self.surv.inv.slots[sel] = Some(ItemStack::new(to, 1)),
            Some(_) => {
                self.surv.inv.take(sel, 1);
                if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(to, 1)) {
                    self.throw_stack(left);
                }
            }
            None => {}
        }
    }

    pub(crate) fn throw_stack(&mut self, st: ItemStack) {
        let f = self.forward();
        self.spawn_drop(self.body.eye() - Vec3::Y * 0.3, st, f * 4.0 + Vec3::Y);
        self.mark_thrown();
    }

    /// The last drop was thrown by the player: it waits before it can be picked up again.
    fn mark_thrown(&mut self) {
        if let Some(d) = self.surv.drops.last_mut() {
            d.age = PICKUP_DELAY - THROWN_DELAY;
        }
    }

    /// Middle click in creative: the block's item into the hand (items are endless here).
    pub(crate) fn creative_pick(&mut self, id: BlockId) {
        let Some(item) = self.items.of_block(id) else { return };
        if let Some(i) = (0..9).find(|&i| self.surv.inv.slots[i].is_some_and(|s| s.item == item)) {
            self.selected = i;
            return;
        }
        self.surv.inv.slots[self.selected] = Some(ItemStack::new(item, 1));
    }

    /// Middle click in survival: pick the block's item from the inventory into the hand.
    pub(crate) fn surv_pick(&mut self, id: BlockId) {
        let Some(item) = self.items.of_block(id) else { return };
        if let Some(i) = (0..9).find(|&i| self.surv.inv.slots[i].is_some_and(|s| s.item == item)) {
            self.selected = i;
            return;
        }
        if let Some(i) = (9..INV_SIZE).find(|&i| self.surv.inv.slots[i].is_some_and(|s| s.item == item)) {
            self.surv.inv.slots.swap(i, self.selected);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Simulation.

    pub(crate) fn surv_update(&mut self, dt: f32, use_held: bool) {
        self.surv.attack_t += dt;
        self.update_drops(dt);
        self.update_furnaces(dt);
        self.update_saplings(dt);
        if !self.survival || self.screen == Screen::Dead {
            self.surv.eat = 0.0;
            return;
        }
        // Exhaustion from moving: sprinting and swimming cost more than walking.
        let moved = (self.body.pos - self.surv.last_pos) * Vec3::new(1.0, 0.0, 1.0);
        let d = moved.length().min(2.0);
        if self.sprinting {
            self.surv.exhaustion += 0.1 * d;
        } else if self.body.in_water {
            self.surv.exhaustion += 0.01 * d;
        }
        if self.surv.was_ground && !self.body.on_ground && self.body.vel.y > 4.0 {
            self.surv.exhaustion += if self.sprinting { 0.2 } else { 0.05 };
        }
        self.surv.was_ground = self.body.on_ground;
        self.surv.last_pos = self.body.pos;
        while self.surv.exhaustion >= 4.0 {
            self.surv.exhaustion -= 4.0;
            if self.surv.saturation > 0.0 {
                self.surv.saturation = (self.surv.saturation - 1.0).max(0.0);
            } else {
                self.surv.hunger = (self.surv.hunger - 1.0).max(0.0);
            }
        }
        // Healing needs a full-ish stomach; an empty one hurts.
        if self.surv.hunger >= 18.0 && self.health < 20.0 {
            self.surv.heal_t += dt;
            let every = if self.surv.hunger >= 20.0 && self.surv.saturation > 0.0 { 0.5 } else { 4.0 };
            if self.surv.heal_t >= every {
                self.surv.heal_t = 0.0;
                self.health = (self.health + 1.0).min(20.0);
                self.surv.exhaustion += 6.0;
            }
        } else {
            self.surv.heal_t = 0.0;
        }
        if self.surv.hunger <= 0.0 {
            self.surv.starve_t += dt;
            if self.surv.starve_t >= 4.0 {
                self.surv.starve_t = 0.0;
                if self.health > 1.0 {
                    self.damage(1.0, None);
                }
            }
        } else {
            self.surv.starve_t = 0.0;
        }
        // Eating: hold the use button with food in hand.
        let food: Option<FoodDef> = self.held_stack().and_then(|s| self.items.get(s.item).food);
        let wants = use_held && self.screen == Screen::Playing && food.is_some() && self.surv.hunger < 20.0;
        if wants {
            self.surv.eat += dt;
            self.surv.eat_sound -= dt;
            if self.surv.eat_sound <= 0.0 {
                self.surv.eat_sound = 0.22;
                self.sfx(Sfx::Eat);
                self.anim.start_swing();
            }
            if self.surv.eat >= 1.6 {
                self.surv.eat = 0.0;
                let f = food.unwrap();
                self.surv.hunger = (self.surv.hunger + f.hunger as f32).min(20.0);
                self.surv.saturation = (self.surv.saturation + f.saturation).min(self.surv.hunger);
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
                self.sfx(Sfx::Pickup);
            }
        } else {
            self.surv.eat = 0.0;
        }
    }

    fn update_drops(&mut self, dt: f32) {
        let still = Input::default();
        let player = self.body.pos + Vec3::Y * 0.9;
        let alive = self.screen != Screen::Dead;
        let mut picked: Vec<usize> = Vec::new();
        for (i, d) in self.surv.drops.iter_mut().enumerate() {
            d.age += dt;
            d.spin += dt * 1.2;
            if self.world.block(d.body.pos.floor().as_ivec3()).is_none() {
                continue;
            }
            // Pulled toward a nearby player.
            let to = player - d.body.pos;
            let dist = to.length();
            // Like the classic game: anything within about a block of the player is taken at once.
            if alive && d.age > PICKUP_DELAY && dist < 2.6 {
                if dist < 1.7 {
                    picked.push(i);
                } else {
                    d.body.vel += to.normalize_or_zero() * dt * 40.0;
                }
            }
            d.body.step(&self.world, &still, dt);
            // Floats up in water.
            if d.body.in_water {
                d.body.vel.y = d.body.vel.y.max(0.6);
            }
        }
        let mut sound = false;
        for &i in picked.iter().rev() {
            let s = self.surv.drops[i].stack;
            match self.surv.inv.add(&self.items, s) {
                None => {
                    self.surv.drops.swap_remove(i);
                    sound = true;
                }
                Some(left) => {
                    if left.count != s.count {
                        sound = true;
                    }
                    self.surv.drops[i].stack = left;
                }
            }
        }
        self.surv.pickup_sound -= dt;
        if sound && self.surv.pickup_sound <= 0.0 {
            self.surv.pickup_sound = 0.08;
            self.sfx(Sfx::Pickup);
        }
        // Equal stacks lying together merge, so a mined tunnel doesn't leave hundreds of items.
        let n = self.surv.drops.len();
        if n > 1 {
            let mut i = 0;
            while i < self.surv.drops.len() {
                let mut j = i + 1;
                while j < self.surv.drops.len() {
                    let (a, b) = (&self.surv.drops[i], &self.surv.drops[j]);
                    let max = self.items.get(a.stack.item).max_stack;
                    if a.stack.stacks_with(&b.stack) && (a.body.pos - b.body.pos).length_squared() < 0.6 && a.stack.count as u16 + b.stack.count as u16 <= max as u16 {
                        let c = b.stack.count;
                        self.surv.drops[i].stack.count += c;
                        self.surv.drops.swap_remove(j);
                    } else {
                        j += 1;
                    }
                }
                i += 1;
            }
        }
        self.surv.drops.retain(|d| d.age < DESPAWN && d.body.pos.y > -64.0);

        // Experience orbs drift to the player.
        let mut gained = 0;
        for o in self.surv.orbs.iter_mut() {
            o.age += dt;
            let to = player - o.pos;
            let dist = to.length();
            if alive && o.age > 0.4 && dist < 7.0 {
                o.vel += to.normalize_or_zero() * dt * (14.0 - dist);
            }
            o.vel.y -= 9.0 * dt;
            o.vel *= 1.0 - (dt * 2.0).min(1.0);
            let next = o.pos + o.vel * dt;
            if Terrain::solid(&self.world, next.floor().as_ivec3()) {
                o.vel = Vec3::new(o.vel.x * 0.5, 0.0, o.vel.z * 0.5);
            } else {
                o.pos = next;
            }
            if alive && o.age > 0.4 && dist < 0.8 {
                gained += o.value;
                o.value = 0;
            }
        }
        self.surv.orbs.retain(|o| o.value > 0 && o.age < DESPAWN);
        if gained > 0 {
            let before = self.surv.level().0;
            self.surv.xp += gained;
            self.sfx(Sfx::Xp);
            if self.surv.level().0 > before {
                let l = self.surv.level().0;
                self.toast(format!("Nível {l}!"), 2.0);
            }
        }
    }

    fn update_furnaces(&mut self, dt: f32) {
        let items = &self.items;
        let recipes = &self.recipes;
        for c in self.surv.containers.values_mut() {
            let Container::Furnace(f) = c else { continue };
            let recipe = f.inv.slots[0].and_then(|s| recipes.smelt(s.item));
            let can = recipe.is_some_and(|r| match f.inv.slots[2] {
                None => true,
                Some(o) => o.item == r.result.item && o.count + r.result.count <= items.get(o.item).max_stack,
            });
            if f.burn <= 0.0 && can {
                if let Some(fuel) = f.inv.slots[1] {
                    let secs = items.get(fuel.item).fuel;
                    if secs > 0.0 {
                        f.inv.take(1, 1);
                        f.burn = secs;
                        f.burn_max = secs;
                    }
                }
            }
            if f.burn > 0.0 {
                f.burn -= dt;
                if can {
                    f.cook += dt;
                    if f.cook >= SMELT_TIME {
                        f.cook = 0.0;
                        let r = recipe.unwrap();
                        f.inv.take(0, 1);
                        match &mut f.inv.slots[2] {
                            Some(o) => o.count += r.result.count,
                            slot => *slot = Some(r.result),
                        }
                        f.xp += r.xp;
                    }
                } else {
                    f.cook = 0.0;
                }
            } else {
                f.burn = 0.0;
                f.cook = (f.cook - dt * 2.0).max(0.0);
            }
        }
    }

    fn update_saplings(&mut self, dt: f32) {
        if self.surv.saplings.is_empty() {
            return;
        }
        let mut grow = Vec::new();
        for (p, t) in self.surv.saplings.iter_mut() {
            if self.world.block(*p).is_none() {
                continue;
            }
            *t -= dt;
            if *t <= 0.0 {
                grow.push(*p);
            }
        }
        self.surv.saplings.retain(|(_, t)| *t > 0.0);
        for p in grow {
            let id = self.world.block_or_air(p);
            let name = self.reg.get(id).name.clone();
            let Some(kind) = TreeKind::from_sapling(&name) else { continue };
            let blocks = TreeBlocks::new(&self.reg);
            // Needs room to grow (the 2x2 trunks also need the three blocks beside the sapling).
            let h = kind.min_height();
            let cols: &[IVec3] = if kind.wide_trunk() { &[IVec3::ZERO, IVec3::X, IVec3::Z, IVec3::new(1, 0, 1)] } else { &[IVec3::ZERO] };
            let blocked = cols.iter().any(|&c| (0..=h).any(|y| (c != IVec3::ZERO || y > 0) && !matches!(self.world.block(p + c + IVec3::Y * y), Some(AIR))));
            if blocked {
                self.surv.saplings.push((p, 30.0));
                continue;
            }
            self.world.set_block(p, AIR);
            let mut rng = Rng::new(self.surv.rng.next_u64());
            let ground = p - IVec3::Y;
            trees::grow(kind, &blocks, &mut WorldTrees(&mut self.world), &mut rng, ground.x, ground.y, ground.z);
            if self.world.block_or_air(p) == AIR {
                // Nothing took its place (a pack without that wood): give the sapling back.
                self.world.set_block(p, id);
            }
        }
    }

    /// Instances for the item renderer: drops and orbs near the camera.
    pub(crate) fn drop_instances(&self, cam_block: IVec3) -> Vec<GpuItem> {
        let c = cam_block.as_vec3();
        let mut out = Vec::with_capacity(self.surv.drops.len() + self.surv.orbs.len());
        // The orb sprite is loaded right after the item sprites.
        let orb_layer = (self.item_base as usize + self.items.textures.len()) as f32;
        let hand = self.hand_light_level();
        let eye = self.body.eye();
        for d in &self.surv.drops {
            let it = self.items.get(d.stack.item);
            // Lying flat once landed (Item Drops style), or the classic floating spin.
            let lying = self.settings.flat_drops && (d.body.on_ground || d.body.vel.length_squared() < 0.01) && !d.body.in_water;
            let bob = if lying { 0.0 } else { (d.age * 2.5).sin() * 0.06 + 0.18 };
            let p = d.body.pos + Vec3::Y * bob - c;
            let l = self.world.light((d.body.pos + Vec3::Y * 0.2).floor().as_ivec3());
            let (sky, blk) = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let blk = blk.max(hand - (d.body.pos - eye).length() / 15.0);
            let (flat, layers, mut size) = match it.look {
                Look::Block(b) => {
                    let bl = self.reg.get(b);
                    if matches!(bl.shape, Shape::Cross | Shape::Torch) {
                        (1.0, [bl.faces[0] as f32; 3], 0.4)
                    } else {
                        (0.0, [bl.faces[0] as f32, bl.faces[2] as f32, bl.faces[3] as f32], 0.22)
                    }
                }
                Look::Sprite(l) => (1.0, [(self.item_base + l) as f32; 3], 0.36),
            };
            // Stacks show as two or three copies, like the classic game.
            let copies = if d.stack.count > 16 { 3 } else if d.stack.count > 1 { 2 } else { 1 };
            if lying && flat > 0.5 {
                size *= 1.1;
            }
            for k in 0..copies {
                let kf = k as f32;
                if lying {
                    // Flat on the ground, a little scattered; blocks sit on their base.
                    let yaw = d.spin_rest + kf * 0.7;
                    let (lift, pitch, spread) = if flat > 0.5 { (0.02 + kf * 0.02, -std::f32::consts::FRAC_PI_2, 0.07) } else { (size * 0.5, 0.0, size * 1.1) };
                    let off = Vec3::new(yaw.cos(), 0.0, yaw.sin()) * kf * spread;
                    out.push(GpuItem { pos: [p.x + off.x, p.y + lift, p.z + off.z, size], info: [yaw, flat, sky, blk], layers: [layers[0], layers[1], layers[2], 0.0], rot: [pitch, 0.0, 0.0, 0.0] });
                } else {
                    let off = Vec3::new(kf * 0.05, kf * 0.04, -kf * 0.05);
                    out.push(GpuItem { pos: [p.x + off.x, p.y + off.y, p.z + off.z, size], info: [d.spin, flat, sky, blk], layers: [layers[0], layers[1], layers[2], 0.0], rot: [0.0; 4] });
                }
            }
        }
        for o in &self.surv.orbs {
            let p = o.pos - c + Vec3::Y * ((o.age * 4.0).sin() * 0.05);
            let size = 0.12 + (o.value as f32).sqrt() * 0.04;
            out.push(GpuItem { pos: [p.x, p.y, p.z, size], info: [self.yaw, 1.0, 1.0, 1.0], layers: [orb_layer, orb_layer, orb_layer, 0.5], rot: [0.0; 4] });
        }
        out
    }

    // -----------------------------------------------------------------------------------------
    // Interface.

    /// Hotbar items, hunger and experience over the classic hotbar frame.
    pub(crate) fn draw_hotbar_items(&mut self, hx: f32, hy: f32, cell: f32) {
        let s = self.ui.scale;
        for i in 0..9 {
            if let Some(st) = self.surv.inv.slots[i] {
                item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &st, hx + i as f32 * cell + 3.0 * s, hy + 2.5 * s, 14.0 * s);
            }
        }
    }

    pub(crate) fn draw_survival_hud(&mut self, hx: f32, hy: f32, cell: f32) {
        let s = self.ui.scale;
        // Hunger: ten drumsticks on the right, above the hotbar.
        let y = hy - 11.0 * s;
        let shake = self.surv.saturation <= 0.0 && self.surv.hunger <= 6.0;
        for i in 0..10 {
            let x = hx + cell * 9.0 - (i + 1) as f32 * 8.0 * s;
            let v = self.surv.hunger - i as f32 * 2.0;
            let dy = if shake && ((self.now * 12.0) as i32 + i) % 3 == 0 { s } else { 0.0 };
            if self.ui.sprite("sprites/hud/food_empty", x, y + dy - s, 9.0 * s, 9.0 * s) {
                if v > 0.0 {
                    let art = if v >= 2.0 { "sprites/hud/food_full" } else { "sprites/hud/food_half" };
                    self.ui.sprite(art, x, y + dy - s, 9.0 * s, 9.0 * s);
                }
                continue;
            }
            draw_drumstick(&mut self.ui, x, y + dy, s, 1.0, true);
            if v > 0.0 {
                draw_drumstick(&mut self.ui, x, y + dy, s, if v >= 2.0 { 1.0 } else { 0.5 }, false);
            }
        }
        // Armor above the hearts.
        let pts = self.armor_points();
        if pts > 0 {
            let y = hy - 21.0 * s;
            for i in 0..10u32 {
                let x = hx + i as f32 * 8.0 * s;
                let v = pts as i32 - i as i32 * 2;
                if self.ui.sprite("sprites/hud/armor_empty", x, y - s, 9.0 * s, 9.0 * s) {
                    if v > 0 {
                        let art = if v >= 2 { "sprites/hud/armor_full" } else { "sprites/hud/armor_half" };
                        self.ui.sprite(art, x, y - s, 9.0 * s, 9.0 * s);
                    }
                    continue;
                }
                draw_shield(&mut self.ui, x, y, s, 1.0, true);
                if v > 0 {
                    draw_shield(&mut self.ui, x, y, s, if v >= 2 { 1.0 } else { 0.5 }, false);
                }
            }
        }
        // Experience bar and level.
        let (lvl, k) = self.surv.level();
        let bw = cell * 9.0;
        let by = hy - 3.5 * s;
        if self.ui.sprite("sprites/hud/experience_bar_background", hx - s, by - 1.5 * s, 182.0 * s, 5.0 * s) {
            self.ui.sprite_part("sprites/hud/experience_bar_progress", [0.0, 0.0, k, 1.0], hx - s, by - 1.5 * s, 182.0 * s * k, 5.0 * s, crate::ui::WHITE);
        } else {
            self.ui.rect(hx, by, bw, 2.5 * s, [0, 0, 0, 200]);
            self.ui.rect(hx, by, bw * k, 2.5 * s, [128, 255, 32, 255]);
        }
        if lvl > 0 {
            let t = lvl.to_string();
            let tw = self.ui.text_width(&t, s);
            self.ui.text(hx + bw / 2.0 - tw / 2.0, by - 8.0 * s, &t, [128, 255, 32, 255]);
        }
        // Eating progress.
        if self.surv.eat > 0.0 {
            let w = 30.0 * s;
            let (cx, cy) = (hx + bw / 2.0 - w / 2.0, hy - 30.0 * s);
            self.ui.rect(cx, cy, w, 2.0 * s, [0, 0, 0, 160]);
            self.ui.rect(cx, cy, w * (self.surv.eat / 1.6).min(1.0), 2.0 * s, [230, 160, 80, 255]);
        }
    }

    /// The inventory and container screens. Clicks are handled here, like the other menus.
    pub(crate) fn draw_survival_inventory(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        let cell = 18.0 * s;
        let (pw, ph) = (176.0 * s, 166.0 * s);
        let x0 = ((w - pw) / 2.0).floor();
        let y0 = ((h - ph) / 2.0).floor();
        let open = self.surv.open;
        // Furnace and chest close when the block is gone.
        if let Open::Furnace(p) | Open::Chest(p) = open {
            if !self.surv.containers.contains_key(&p) {
                self.close_survival_screen();
                return;
            }
        }
        // The player figure is drawn in 3D behind the interface, seen through a hole in it.
        let hole = (open == Open::Craft).then_some((x0 + 27.0 * s, y0 + 9.0 * s, 50.0 * s, 60.0 * s));
        self.surv.figure_box = hole;
        rect_around(&mut self.ui, (0.0, 0.0, w, h), hole, [0, 0, 0, 110]);
        rect_around(&mut self.ui, (x0, y0, pw, ph), hole, [198, 198, 198, 255]);
        self.ui.frame(x0, y0, pw, ph, s, [255, 255, 255, 255]);
        self.ui.rect(x0 + pw - s, y0, s, ph, [85, 85, 85, 255]);
        self.ui.rect(x0, y0 + ph - s, pw, s, [85, 85, 85, 255]);
        let dark = [64, 64, 64, 255];

        let mut slots: Vec<(Slot, f32, f32)> = Vec::new();
        for i in 0..27 {
            slots.push((Slot::Inv(9 + i), x0 + (8 + (i % 9) * 18) as f32 * s, y0 + (84 + (i / 9) * 18) as f32 * s));
        }
        for i in 0..9 {
            slots.push((Slot::Inv(i), x0 + (8 + i * 18) as f32 * s, y0 + 142.0 * s));
        }
        self.ui.label(x0 + 8.0 * s, y0 + 73.0 * s, "Inventário", dark, s);
        let n = self.surv.grid_size();
        match open {
            Open::Craft | Open::Table => {
                let (gx, gy, ox, oy, title) = if n == 3 { (30.0, 17.0, 124.0, 35.0, "Fabricação") } else { (88.0, 26.0, 146.0, 36.0, "Fabricação") };
                self.ui.label(x0 + gx * s, y0 + (gy - 11.0) * s, title, dark, s);
                for i in 0..n * n {
                    slots.push((Slot::Craft(i), x0 + (gx + (i % n) as f32 * 18.0) * s, y0 + (gy + (i / n) as f32 * 18.0) * s));
                }
                slots.push((Slot::CraftOut, x0 + ox * s, y0 + oy * s));
                // Arrow from the grid to the result.
                let ax = x0 + (ox - 22.0) * s;
                let ay = y0 + (oy + 7.0) * s;
                self.ui.rect(ax, ay, 14.0 * s, 3.0 * s, [139, 139, 139, 255]);
                for k in 0..4 {
                    self.ui.rect(ax + (14.0 + k as f32) * s, ay - (3.0 - k as f32) * s, s, (9.0 - 2.0 * k as f32) * s, [139, 139, 139, 255]);
                }
                if n == 2 {
                    for i in 0..4 {
                        slots.push((Slot::Armor(i), x0 + 8.0 * s, y0 + (8 + i * 18) as f32 * s));
                    }
                    // A little figure of the player in the corner, like the classic screen.
                    self.ui.frame(x0 + 26.0 * s, y0 + 8.0 * s, 52.0 * s, 62.0 * s, s, [0, 0, 0, 255]);

                }
            }
            Open::Furnace(p) => {
                self.ui.label(x0 + 60.0 * s, y0 + 6.0 * s, "Fornalha", dark, s);
                slots.push((Slot::Cont(0), x0 + 56.0 * s, y0 + 17.0 * s));
                slots.push((Slot::Cont(1), x0 + 56.0 * s, y0 + 53.0 * s));
                slots.push((Slot::Cont(2), x0 + 116.0 * s, y0 + 35.0 * s));
                let (burn, cook) = match self.surv.containers.get(&p) {
                    Some(Container::Furnace(f)) => (if f.burn_max > 0.0 { (f.burn / f.burn_max).clamp(0.0, 1.0) } else { 0.0 }, f.cook / SMELT_TIME),
                    _ => (0.0, 0.0),
                };
                // Flame (how much fuel is left) and arrow (cooking progress).
                let (fx, fy) = (x0 + 57.0 * s, y0 + 37.0 * s);
                self.ui.rect(fx, fy, 14.0 * s, 14.0 * s, [139, 139, 139, 255]);
                let fh = 14.0 * s * burn;
                self.ui.rect(fx + 2.0 * s, fy + 14.0 * s - fh, 10.0 * s, fh, [230, 120, 30, 255]);
                self.ui.rect(fx + 4.0 * s, fy + 14.0 * s - fh * 0.6, 6.0 * s, fh * 0.6, [250, 200, 60, 255]);
                let (ax, ay) = (x0 + 79.0 * s, y0 + 39.0 * s);
                self.ui.rect(ax, ay, 24.0 * s, 8.0 * s, [139, 139, 139, 255]);
                self.ui.rect(ax, ay, 24.0 * s * cook.clamp(0.0, 1.0), 8.0 * s, [240, 240, 240, 255]);
            }
            Open::Chest(_) => {
                self.ui.label(x0 + 8.0 * s, y0 + 6.0 * s, "Baú", dark, s);
                for i in 0..CHEST_SIZE {
                    slots.push((Slot::Cont(i), x0 + (8 + (i % 9) * 18) as f32 * s, y0 + (18 + (i / 9) * 18) as f32 * s));
                }
            }
        }

        let result = self.craft_result();
        let mut hovered: Option<Slot> = None;
        for &(slot, sx, sy) in &slots {
            let big = slot == Slot::CraftOut || slot == Slot::Cont(2) && matches!(open, Open::Furnace(_));
            let pad = if big { 4.0 * s } else { 0.0 };
            self.ui.rect(sx - s - pad, sy - s - pad, cell + 2.0 * pad, cell + 2.0 * pad, [55, 55, 55, 255]);
            self.ui.rect(sx - pad, sy - pad, cell - s + 2.0 * pad, cell - s + 2.0 * pad, [255, 255, 255, 255]);
            self.ui.rect(sx - pad, sy - pad, cell - 2.0 * s + 2.0 * pad, cell - 2.0 * s + 2.0 * pad, [139, 139, 139, 255]);
            let over = mouse.0 >= sx - pad && mouse.0 < sx + cell - s + pad && mouse.1 >= sy - pad && mouse.1 < sy + cell - s + pad;
            if over {
                hovered = Some(slot);
                self.ui.rect(sx - pad, sy - pad, cell - 2.0 * s + 2.0 * pad, cell - 2.0 * s + 2.0 * pad, [170, 170, 170, 255]);
            }
            let st = if slot == Slot::CraftOut { result } else { self.slot_get(slot) };
            if let Some(st) = st {
                item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &st, sx, sy, 16.0 * s);
            }
        }
        // R over any item opens its recipes in the book.
        if self.surv.want_recipe && hovered.is_some() {
            self.surv.want_recipe = false;
            let st = match hovered {
                Some(Slot::CraftOut) => result,
                Some(sl) => self.slot_get(sl),
                None => None,
            };
            if let Some(st) = st {
                self.surv.book_view = Some((st.item, 0));
            }
        }
        let over_book = self.draw_recipe_book(x0, y0, pw, ph, w, h, mouse);
        self.surv.want_recipe = false;
        // Clicks.
        let shift = self.keys.contains(&winit::keyboard::KeyCode::ShiftLeft) || self.keys.contains(&winit::keyboard::KeyCode::ShiftRight);
        let inside = over_book || (mouse.0 >= x0 && mouse.0 < x0 + pw && mouse.1 >= y0 && mouse.1 < y0 + ph);
        if self.clicked || self.rclicked {
            let right = self.rclicked && !self.clicked;
            match hovered {
                Some(slot) => self.click_slot(slot, right, shift),
                None if !inside => {
                    // Clicking outside the window throws the held stack.
                    if let Some(c) = self.surv.cursor {
                        let n = if right { 1 } else { c.count };
                        let thrown = ItemStack { count: n, ..c };
                        let left = c.count - n;
                        self.surv.cursor = (left > 0).then_some(ItemStack { count: left, ..c });
                        let f = self.forward();
                        self.spawn_drop(self.body.eye() - Vec3::Y * 0.3, thrown, f * 4.0 + Vec3::Y);
                        self.mark_thrown();
                    }
                }
                None => {}
            }
        }
        // Right-drag (like Mouse Tweaks): one item into each slot the mouse passes over.
        if self.rdown && !self.rclicked {
            if let (Some(slot), Some(c)) = (hovered, self.surv.cursor) {
                let fresh = self.surv.drag_last != Some(slot) && !matches!(slot, Slot::CraftOut | Slot::Armor(_)) && !(slot == Slot::Cont(2) && matches!(open, Open::Furnace(_)));
                let fits = match self.slot_get(slot) {
                    None => true,
                    Some(st) => st.stacks_with(&c) && st.count < self.items.get(st.item).max_stack,
                };
                if fresh && fits {
                    self.click_slot(slot, true, false);
                }
            }
        }
        self.surv.drag_last = if self.rdown { hovered.or(self.surv.drag_last) } else { None };
        // Number keys over a slot swap it with that hotbar slot.
        if let (Some(slot), Some(k)) = (hovered, self.inv_number_key.take()) {
            if slot != Slot::CraftOut {
                let a = self.slot_get(slot);
                let b = self.surv.inv.slots[k];
                self.slot_set(slot, b);
                self.surv.inv.slots[k] = a;
            }
        }
        self.inv_number_key = None;
        // Tooltip and the stack on the cursor.
        if self.surv.cursor.is_none() && !over_book {
            let st = match hovered {
                Some(Slot::CraftOut) => result,
                Some(sl) => self.slot_get(sl),
                None => None,
            };
            if let Some(st) = st {
                let it = self.items.get(st.item);
                let mut name = it.display_name.clone();
                if let Some(t) = &it.tool {
                    name = format!("{name} ({}/{})", t.durability - st.damage, t.durability);
                } else if let Some(f) = &it.food {
                    name = format!("{name} (+{} fome)", f.hunger);
                }
                let tw = self.ui.text_width(&name, s);
                let (mx, my) = mouse;
                self.ui.rect(mx + 8.0 * s, my - 14.0 * s, tw + 6.0 * s, 12.0 * s, [16, 0, 16, 235]);
                self.ui.text(mx + 11.0 * s, my - 12.0 * s, &name, WHITE);
            }
        }
        if let Some(c) = self.surv.cursor {
            item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &c, mouse.0 - 8.0 * s, mouse.1 - 8.0 * s, 16.0 * s);
        }
        self.ui.text_centered(w / 2.0, y0 + ph + 4.0 * s, "Esq.: pegar/pôr  Dir.: metade/um  Shift: mover  1-9: trocar  R: receita", WHITE);
    }

    pub(crate) fn close_survival_screen(&mut self) {
        self.set_screen(Screen::Playing);
    }

    fn craft_result(&self) -> Option<ItemStack> {
        let n = self.surv.grid_size();
        let grid: Vec<Option<ItemStack>> = (0..n * n).map(|i| self.surv.craft[i]).collect();
        self.recipes.craft(&grid, n)
    }

    pub(crate) fn slot_get(&self, slot: Slot) -> Option<ItemStack> {
        match slot {
            Slot::Inv(i) => self.surv.inv.slots[i],
            Slot::Craft(i) => self.surv.craft[i],
            Slot::CraftOut => None,
            Slot::Armor(i) => self.surv.armor[i],
            Slot::Offhand => self.surv.offhand,
            Slot::Cont(i) => match self.open_container_inv() {
                Some(inv) => inv.slots.get(i).copied().flatten(),
                None => None,
            },
        }
    }

    pub(crate) fn slot_set(&mut self, slot: Slot, v: Option<ItemStack>) {
        match slot {
            Slot::Inv(i) => self.surv.inv.slots[i] = v,
            Slot::Craft(i) => self.surv.craft[i] = v,
            Slot::CraftOut => {}
            Slot::Armor(i) => self.surv.armor[i] = v,
            Slot::Offhand => self.surv.offhand = v,
            Slot::Cont(i) => {
                if let Some(inv) = self.open_container_inv_mut() {
                    if i < inv.slots.len() {
                        inv.slots[i] = v;
                    }
                }
            }
        }
    }

    fn open_container_inv(&self) -> Option<&Inventory> {
        match self.surv.open {
            Open::Furnace(p) | Open::Chest(p) => match self.surv.containers.get(&p)? {
                Container::Chest(i) => Some(i),
                Container::Furnace(f) => Some(&f.inv),
            },
            _ => None,
        }
    }

    fn open_container_inv_mut(&mut self) -> Option<&mut Inventory> {
        match self.surv.open {
            Open::Furnace(p) | Open::Chest(p) => match self.surv.containers.get_mut(&p)? {
                Container::Chest(i) => Some(i),
                Container::Furnace(f) => Some(&mut f.inv),
            },
            _ => None,
        }
    }

    fn take_furnace_xp(&mut self) {
        if let Open::Furnace(p) = self.surv.open {
            if let Some(Container::Furnace(f)) = self.surv.containers.get_mut(&p) {
                let whole = f.xp.floor();
                f.xp -= whole;
                let at = self.body.pos + Vec3::Y;
                self.spawn_orbs(at, whole as u32);
            }
        }
    }

    pub(crate) fn click_slot(&mut self, slot: Slot, right: bool, shift: bool) {
        let furnace_out = slot == Slot::Cont(2) && matches!(self.surv.open, Open::Furnace(_));
        if slot == Slot::CraftOut {
            self.click_craft_output(shift);
            return;
        }
        if let (Slot::Armor(i), Some(c)) = (slot, self.surv.cursor) {
            if self.items.get(c.item).armor.is_none_or(|a| a.slot as usize != i) {
                return;
            }
        }
        let cur = self.slot_get(slot);
        if shift {
            if let Some(st) = cur {
                let left = self.quick_move(slot, st);
                self.slot_set(slot, left);
                if furnace_out {
                    self.take_furnace_xp();
                }
            }
            return;
        }
        let max = |it: ItemId| self.items.get(it).max_stack;
        let cursor = self.surv.cursor;
        match (cursor, cur, right) {
            (None, Some(st), false) => {
                self.surv.cursor = Some(st);
                self.slot_set(slot, None);
                if furnace_out {
                    self.take_furnace_xp();
                }
            }
            (None, Some(st), true) => {
                let half = st.count.div_ceil(2);
                self.surv.cursor = Some(ItemStack { count: half, ..st });
                let rest = st.count - half;
                self.slot_set(slot, (rest > 0).then_some(ItemStack { count: rest, ..st }));
                if furnace_out {
                    self.take_furnace_xp();
                }
            }
            (Some(c), _, _) if furnace_out => {
                // The furnace output only gives; take more of the same onto the cursor.
                if let Some(st) = cur {
                    if c.stacks_with(&st) && c.count as u16 + st.count as u16 <= max(c.item) as u16 {
                        self.surv.cursor = Some(ItemStack { count: c.count + st.count, ..c });
                        self.slot_set(slot, None);
                        self.take_furnace_xp();
                    }
                }
            }
            (Some(c), None, false) => {
                self.slot_set(slot, Some(c));
                self.surv.cursor = None;
            }
            (Some(c), None, true) => {
                self.slot_set(slot, Some(ItemStack { count: 1, ..c }));
                self.surv.cursor = (c.count > 1).then_some(ItemStack { count: c.count - 1, ..c });
            }
            (Some(c), Some(st), r) if c.stacks_with(&st) => {
                let room = max(st.item).saturating_sub(st.count);
                let n = if r { 1.min(c.count) } else { c.count }.min(room);
                self.slot_set(slot, Some(ItemStack { count: st.count + n, ..st }));
                let left = c.count - n;
                self.surv.cursor = (left > 0).then_some(ItemStack { count: left, ..c });
            }
            (Some(c), Some(st), _) => {
                self.slot_set(slot, Some(c));
                self.surv.cursor = Some(st);
            }
            (None, None, _) => {}
        }
    }

    /// Shift-click: send a stack to the other side. Returns what stays behind.
    fn quick_move(&mut self, from: Slot, st: ItemStack) -> Option<ItemStack> {
        let items = &self.items;
        match from {
            Slot::Inv(_) if self.surv.open == Open::Craft && self.items.get(st.item).armor.is_some_and(|a| self.surv.armor[(a.slot as usize).min(3)].is_none()) => {
                let a = self.items.get(st.item).armor.unwrap();
                self.surv.armor[(a.slot as usize).min(3)] = Some(st);
                None
            }
            Slot::Inv(i) => {
                // Into an open chest; into the furnace input (or fuel); else hotbar <-> main rows.
                match self.surv.open {
                    Open::Chest(p) => {
                        if let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&p) {
                            return inv.add(items, st);
                        }
                        Some(st)
                    }
                    Open::Furnace(p) => {
                        let smeltable = self.recipes.smelt(st.item).is_some();
                        let fuel = items.get(st.item).fuel > 0.0;
                        if let Some(Container::Furnace(f)) = self.surv.containers.get_mut(&p) {
                            if smeltable {
                                return f.inv.add_in(items, st, 0..1);
                            }
                            if fuel {
                                return f.inv.add_in(items, st, 1..2);
                            }
                        }
                        Some(st)
                    }
                    _ => {
                        if i < 9 {
                            self.surv.inv.add_in(items, st, 9..INV_SIZE)
                        } else {
                            self.surv.inv.add_in(items, st, 0..9)
                        }
                    }
                }
            }
            _ => self.surv.inv.add_in(items, st, (0..9).chain(9..INV_SIZE).collect::<Vec<_>>().into_iter()),
        }
    }

    fn click_craft_output(&mut self, shift: bool) {
        let mut crafted = 0;
        loop {
            let Some(r) = self.craft_result() else { break };
            if shift {
                // Craft as many as fit into the inventory.
                let mut test = self.surv.inv.clone();
                if test.add(&self.items, r).is_some() {
                    break;
                }
                self.surv.inv = test;
            } else {
                match self.surv.cursor {
                    None => self.surv.cursor = Some(r),
                    Some(c) if c.stacks_with(&r) && c.count as u16 + r.count as u16 <= self.items.get(c.item).max_stack as u16 => {
                        self.surv.cursor = Some(ItemStack { count: c.count + r.count, ..c });
                    }
                    _ => break,
                }
            }
            let n = self.surv.grid_size();
            for i in 0..n * n {
                if let Some(s) = &mut self.surv.craft[i] {
                    s.count -= 1;
                    if s.count == 0 {
                        self.surv.craft[i] = None;
                    }
                }
            }
            crafted += 1;
            if !shift || crafted > 64 {
                break;
            }
        }
        if crafted > 0 {
            self.sfx(Sfx::Place(Material::Wood));
        }
    }
}

impl Game {
    /// Test picture setup (`--kit`): a filled inventory, items and experience on the ground, and
    /// with `--open` a crafting table, furnace or chest screen.
    pub(crate) fn test_kit(&mut self) {
        let give = |g: &mut Game, slot: usize, name: &str, count: u8, damage: u16| {
            if let Some(id) = g.items.id(name) {
                g.surv.inv.slots[slot] = Some(ItemStack { item: id, count, damage });
            }
        };
        give(self, 0, "iron_pickaxe", 1, 60);
        give(self, 1, "stone_sword", 1, 0);
        give(self, 2, "oak_planks", 37, 0);
        give(self, 3, "cobblestone", 64, 0);
        give(self, 4, "torch", 16, 0);
        give(self, 5, "cooked_beef", 5, 0);
        give(self, 6, "crafting_table", 1, 0);
        give(self, 7, "furnace", 1, 0);
        give(self, 8, "red_bed", 1, 0);
        give(self, 9, "coal", 12, 0);
        give(self, 10, "raw_iron", 7, 0);
        give(self, 11, "stick", 9, 0);
        give(self, 12, "apple", 3, 0);
        give(self, 13, "diamond", 2, 0);
        give(self, 14, "oak_sapling", 4, 0);
        give(self, 15, "chest", 2, 0);
        give(self, 20, "wooden_axe", 1, 40);
        give(self, 22, "string", 6, 0);
        give(self, 27, "feather", 3, 0);
        for (i, n) in ["golden_helmet", "iron_chestplate", "diamond_leggings", "leather_boots"].iter().enumerate() {
            self.surv.armor[i] = self.items.id(n).map(|id| ItemStack::new(id, 1));
        }
        self.surv.hunger = 13.0;
        self.surv.xp = 40;
        self.health = 15.0;
        // On the ground in front of the player.
        let f = self.forward();
        let fwd = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-fwd.z, 0.0, fwd.x);
        let names = ["cobblestone", "iron_pickaxe", "apple", "oak_log", "diamond", "raw_iron", "dirt"];
        for (i, n) in names.iter().enumerate() {
            let Some(id) = self.items.id(n) else { continue };
            let p = self.body.pos + fwd * 2.6 + side * ((i as f32 - 3.0) * 0.55);
            let y = self.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).unwrap_or(p.y as i32) + 1;
            let st = ItemStack::new(id, if i % 3 == 0 { 20 } else { 1 });
            self.spawn_drop(Vec3::new(p.x, y as f32, p.z), st, Vec3::ZERO);
            let d = self.surv.drops.last_mut().unwrap();
            d.age = 1.0 + i as f32 * 0.7;
            d.body.on_ground = true;
        }
        let at = self.body.pos + fwd * 1.8 + Vec3::Y * 0.6;
        for k in 0..3 {
            self.surv.orbs.push(Orb { pos: at + side * (k as f32 * 0.4 - 0.4), vel: Vec3::ZERO, value: 3 + k * 4, age: 10.0 });
        }
        let planks = self.items.id("oak_planks");
        let stick = self.items.id("stick");
        let cobble = self.items.id("cobblestone");
        let one = |i: Option<ItemId>| i.map(|i| ItemStack::new(i, 1));
        match self.args.open.as_deref() {
            Some("table") => {
                self.surv.open = Open::Table;
                // A pickaxe in the making.
                self.surv.craft = [one(cobble), one(cobble), one(cobble), None, one(stick), None, None, one(stick), None];
                self.surv.book_view = self.items.id("iron_chestplate").map(|i| (i, 0));
            }
            Some("furnace") => {
                let p = IVec3::new(0, -100, 0);
                let mut inv = Inventory::new(3);
                inv.slots[0] = self.items.id("raw_iron").map(|i| ItemStack::new(i, 5));
                inv.slots[1] = self.items.id("coal").map(|i| ItemStack::new(i, 3));
                inv.slots[2] = self.items.id("iron_ingot").map(|i| ItemStack::new(i, 2));
                self.surv.containers.insert(p, Container::Furnace(Furnace { inv, burn: 50.0, burn_max: 80.0, cook: 6.0, xp: 0.0 }));
                self.surv.open = Open::Furnace(p);
            }
            Some("corpse") => {
                let at = self.body.pos + fwd * 3.0;
                let y = self.world.surface_y(at.x.floor() as i32, at.z.floor() as i32).unwrap_or(at.y as i32) + 1;
                let items = self.surv.inv.slots.iter().flatten().copied().collect();
                self.surv.corpses.push(Corpse { pos: Vec3::new(at.x, y as f32, at.z), yaw: self.yaw + 2.0, items });
            }
            Some("items") => self.surv.side = Side::Items,
            Some("build") => {
                self.surv.side = Side::Build;
                // A table nearby so the 3×3 recipes show.
                let c = self.body.pos.floor().as_ivec3();
                let table = self.reg.id_or_air("crafting_table");
                self.world.set_block(c + IVec3::new(2, 0, 0), table);
            }
            Some("chest") => {
                let p = IVec3::new(0, -101, 0);
                let mut inv = Inventory::new(CHEST_SIZE);
                for (i, n) in ["bone", "rotten_flesh", "gunpowder", "gold_ingot", "bread", "emerald", "flint"].iter().enumerate() {
                    inv.slots[i * 3 + 1] = self.items.id(n).map(|id| ItemStack::new(id, 1 + i as u8 * 3));
                }
                self.surv.containers.insert(p, Container::Chest(inv));
                self.surv.open = Open::Chest(p);
            }
            _ => {
                self.surv.craft[0] = planks.map(|i| ItemStack::new(i, 2));
                self.surv.craft[1] = planks.map(|i| ItemStack::new(i, 2));
                self.surv.craft[2] = planks.map(|i| ItemStack::new(i, 2));
                self.surv.craft[3] = planks.map(|i| ItemStack::new(i, 2));
            }
        }
    }
}

impl Game {
    /// The item in hand, drawn in front of the camera in first person. It is placed in the
    /// fist of the first-person arm, so it follows walking, swings and attacks with it: tools
    /// and weapons are gripped by the handle with the blade up and forward, blocks rest on the
    /// palm, and other items are held up flat.
    pub(crate) fn held_item_instance(&self, eye: Vec3, cam_block: IVec3) -> Option<GpuItem> {
        let hand = self.anim.first_person_hand(eye, self.yaw, self.pitch);
        let l = self.world.light(eye.floor().as_ivec3());
        let eat = if self.surv.eat > 0.0 { (self.surv.eat * 14.0).sin().abs() * 0.04 } else { 0.0 };
        // In this frame -Z is the top of the forearm and +Y runs back to the shoulder.
        let face = (Vec3::Y * 0.85 - Vec3::X * 0.5).normalize();
        // Tools lean away from the eye and to the left, like a sword held ready.
        let tip = (Vec3::NEG_Z * 0.8 + Vec3::NEG_Y * 0.55 + Vec3::NEG_X * 0.35).normalize();
        self.item_in_hand(hand, Vec3::NEG_Z, face, 1.0, l, Vec3::Y * eat - cam_block.as_vec3(), tip, 0.82)
    }

    /// Skeletons' bows and the arrows in flight or stuck in blocks.
    pub(crate) fn arrow_and_bow_instances(&self, mobs: &[(usize, crate::entity::Bones)], cam_block: IVec3) -> Vec<GpuItem> {
        use glam::{EulerRot, Mat3, Quat};
        let mut out = Vec::new();
        let sprite = |name: &str| self.items.id(name).and_then(|i| match self.items.get(i).look {
            Look::Sprite(l) => Some((self.item_base + l) as f32),
            _ => None,
        });
        if let Some(bow) = sprite("bow") {
            for (d, b) in mobs {
                let def = &self.mobs.reg.defs[*d];
                if !def.ranged || def.parts.len() < 3 {
                    continue;
                }
                let p = &def.parts[2];
                let m = glam::Mat4::from_cols_array_2d(&b.m[2]);
                let fist = m.transform_point3(Vec3::new(p.pivot.0 as f32, p.pivot.1 as f32 - 10.0, p.pivot.2 as f32 + 1.0));
                let (_, rot, _) = m.to_scale_rotation_translation();
                // Held upright in the outstretched arm, string towards the body, seen side-on.
                let tip = Vec3::Z;
                let face = (Vec3::X - Vec3::Y * 0.7).normalize();
                let side = face.cross(tip);
                let x = (tip - side) * std::f32::consts::FRAC_1_SQRT_2;
                let y = (tip + side) * std::f32::consts::FRAC_1_SQRT_2;
                let q = rot * Quat::from_mat3(&Mat3::from_cols(x, y, face));
                let (yaw, pitch, roll) = q.to_euler(EulerRot::YXZ);
                out.push(GpuItem { pos: [fist.x, fist.y, fist.z, 0.62], info: [yaw, 1.0, b.light[0], b.light[1]], layers: [bow; 4], rot: [pitch, roll, 0.0, 0.0] });
            }
        }
        out.extend(self.bobber_instance(cam_block));
        if let Some(arrow) = sprite("arrow") {
            let c = cam_block.as_vec3();
            for a in &self.mobs.arrows {
                let d = a.vel.normalize_or_zero();
                if d == Vec3::ZERO && !a.stuck {
                    continue;
                }
                let flat = (d.x * d.x + d.z * d.z).sqrt();
                // The picture's diagonal lies along the flight, in two crossed planes.
                let yaw = (-d.z).atan2(d.x);
                let roll = -std::f32::consts::FRAC_PI_4 + d.y.atan2(flat);
                let l = self.world.light(a.pos.floor().as_ivec3());
                let (sky, blk) = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
                let p = a.pos - c;
                for pitch in [0.0, std::f32::consts::FRAC_PI_2] {
                    out.push(GpuItem { pos: [p.x, p.y, p.z, 0.5], info: [yaw, 1.0, sky, blk], layers: [arrow; 4], rot: [pitch, roll, 0.0, 0.0] });
                }
            }
        }
        out
    }

    /// The armour the player wears, drawn over the player model with the same bones.
    pub(crate) fn armor_instances(&self, bones: &crate::entity::Bones) -> Vec<(usize, crate::entity::Bones)> {
        (0..4)
            .filter_map(|piece| {
                let st = self.surv.armor[piece]?;
                let mesh = self.entities.armor_mesh(&self.items.get(st.item).name, piece)?;
                Some((mesh, *bones))
            })
            .collect()
    }

    /// The held item in the right hand of the player model, for the views from outside.
    pub(crate) fn held_item_third_person(&self, arm: [[f32; 4]; 4], pivot: Vec3) -> Option<GpuItem> {
        // The bone maps model pixels to the world (relative to `cam_block`); the fist is near
        // the bottom of the 12-pixel arm. The hanging arm's front (+Z) points the way a held
        // sword does, and its face looks out to the side.
        let m = glam::Mat4::from_cols_array_2d(&arm);
        let fist = m.transform_point3(Vec3::new(pivot.x, pivot.y - 10.0, pivot.z + 1.5));
        let (_, rot, _) = m.to_scale_rotation_translation();
        let hand = glam::Mat4::from_rotation_translation(rot, fist);
        let l = self.world.light((self.body.pos + Vec3::Y).floor().as_ivec3());
        // Tools point forward and a little up out of the fist, at full item size like other
        // players see them; blocks are about a third of a block.
        // (Found by eye against the classic look: out to the side of the body, up and forward.)
        let tip = Vec3::new(-1.0, 0.4, -0.4).normalize();
        self.item_in_hand(hand, Vec3::Z, Vec3::NEG_X, 2.0, l, Vec3::ZERO, tip, 1.0)
    }

    /// Place the held item in a fist. `hand` is the fist's frame (its arm runs along +Y back to
    /// the shoulder), `top` the side the item stands out from and `face` roughly where a tool's
    /// flat side should look.
    #[allow(clippy::too_many_arguments)]
    fn item_in_hand(&self, hand: glam::Mat4, top: Vec3, face: Vec3, scale: f32, l: u8, shift: Vec3, tip: Vec3, tool_scale: f32) -> Option<GpuItem> {
        use glam::{EulerRot, Mat3, Quat};
        let st = self.held_stack()?;
        let it = self.items.get(st.item);
        let (_, arm_rot, fist) = hand.to_scale_rotation_translation();
        let along = Vec3::NEG_Y;
        let upright = Quat::from_mat3(&Mat3::from_cols(Vec3::X, top, Vec3::X.cross(top)));
        let tool = it.tool.is_some() || it.name.ends_with("_sword") || it.name == "stick" || it.name.ends_with("_rod");
        let (flat, layers, size, local, offset) = match it.look {
            Look::Block(b) => {
                let bl = self.reg.get(b);
                if matches!(bl.shape, Shape::Cross | Shape::Torch) {
                    // Torches and plants stand up out of the fist.
                    let size = if bl.shape == Shape::Torch { 0.42 } else { 0.32 };
                    (1.0, [bl.faces[0] as f32; 3], size, upright, top * (size * 0.35))
                } else {
                    // A small block on top of the fist, turned a little to show two sides.
                    (0.0, [bl.faces[0] as f32, bl.faces[2] as f32, bl.faces[3] as f32], 0.18, upright * Quat::from_rotation_y(0.8), top * 0.15 + along * 0.02)
                }
            }
            Look::Sprite(l) => {
                let layer = (self.item_base + l) as f32;
                if tool {
                    // The picture's diagonal (handle bottom-left, tip top-right) points out of
                    // the fist and a little forward along the arm.
                    let face = (face - tip * face.dot(tip)).normalize();
                    let side = face.cross(tip);
                    let x = (tip - side) * std::f32::consts::FRAC_1_SQRT_2;
                    let y = (tip + side) * std::f32::consts::FRAC_1_SQRT_2;
                    let rot = Quat::from_mat3(&Mat3::from_cols(x, y, face));
                    let size = 0.42 * tool_scale;
                    // Move the picture so its handle (a quarter in from the corner) sits in the fist.
                    let grip = rot * Vec3::new(-0.3 * size, -0.3 * size, 0.0);
                    (1.0, [layer; 3], size, rot, -grip + top * 0.05 + tip * 0.05)
                } else {
                    // Food and other items: held up, facing the eye.
                    (1.0, [layer; 3], 0.3, upright * Quat::from_rotation_z(0.3), top * 0.16)
                }
            }
        };
        let world_rot = arm_rot * local;
        let p = fist + arm_rot * (offset * scale) + shift;
        let (yaw, pitch, roll) = world_rot.to_euler(EulerRot::YXZ);
        let blk = ((l & 15) as f32 / 15.0).max(self.hand_light_level());
        Some(GpuItem {
            pos: [p.x, p.y, p.z, size * scale],
            info: [yaw, flat, (l >> 4) as f32 / 15.0, blk],
            layers: [layers[0], layers[1], layers[2], 0.0],
            rot: [pitch, roll, 0.0, 0.0],
        })
    }

    /// 0..1 light given off by what the player holds (a torch, glowstone...).
    pub(crate) fn hand_light_level(&self) -> f32 {
        let id = self.held_stack().and_then(|s| self.items.get(s.item).places).unwrap_or(AIR);
        self.reg.light_of(id) as f32 / 15.0
    }
}

/// One way to make an item, for the recipe book.
enum BookRecipe {
    Grid { w: usize, cells: Vec<Option<Vec<ItemId>>>, result: ItemStack },
    Smelt { input: Vec<ItemId>, result: ItemStack },
}

impl Game {
    fn recipes_for(&self, item: ItemId) -> Vec<BookRecipe> {
        let mut out = Vec::new();
        for r in &self.recipes.shaped {
            if r.result.item == item {
                out.push(BookRecipe::Grid { w: r.w, cells: r.cells.clone(), result: r.result });
            }
        }
        for r in &self.recipes.shapeless {
            if r.result.item == item {
                let w = if r.ingredients.len() > 4 { 3 } else { 2 };
                out.push(BookRecipe::Grid { w, cells: r.ingredients.iter().map(|i| Some(i.clone())).collect(), result: r.result });
            }
        }
        for r in &self.recipes.smelting {
            if r.result.item == item {
                out.push(BookRecipe::Smelt { input: r.input.clone(), result: r.result });
            }
        }
        out
    }

    /// The recipe book beside the inventory (like JEI): every item on the right, and the recipes
    /// of the chosen one on the left. Returns whether the mouse is over it.
    #[allow(clippy::too_many_arguments)]
    fn draw_recipe_book(&mut self, x0: f32, y0: f32, pw: f32, ph: f32, w: f32, _h: f32, mouse: (f32, f32)) -> bool {
        let s = self.ui.scale;
        let cell = 18.0 * s;
        let mut over = false;
        let click = self.clicked;
        // Two tabs stuck to the right edge of the inventory: every item, and building.
        let tx = x0 + pw - s;
        let tabs = [(Side::Items, "Todos os itens", "chest"), (Side::Build, "Construir", "crafting_table")];
        let mut tab_tip: Option<&str> = None;
        for (k, (side, label, icon)) in tabs.into_iter().enumerate() {
            let ty = y0 + 4.0 * s + k as f32 * 24.0 * s;
            let on = self.surv.side == side;
            let tw = if on { 25.0 * s } else { 22.0 * s };
            let hov = mouse.0 >= tx && mouse.0 < tx + tw && mouse.1 >= ty && mouse.1 < ty + 22.0 * s;
            self.ui.rect(tx, ty, tw, 22.0 * s, if on { [198, 198, 198, 255] } else if hov { [170, 170, 170, 255] } else { [139, 139, 139, 255] });
            self.ui.rect(tx, ty, tw, s, [255, 255, 255, 255]);
            self.ui.rect(tx + tw - s, ty, s, 22.0 * s, [85, 85, 85, 255]);
            self.ui.rect(tx, ty + 22.0 * s - s, tw, s, [85, 85, 85, 255]);
            let id = self.items.id(icon).or_else(|| self.items.id("crafting_table"));
            if let Some(id) = id {
                item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(id, 1), tx + 3.0 * s, ty + 3.0 * s, 16.0 * s);
            }
            if hov {
                over = true;
                tab_tip = Some(label);
                if click {
                    self.surv.side = if on { Side::Closed } else { side };
                    self.surv.book_scroll = 0;
                }
            }
        }
        let bx = x0 + pw + 27.0 * s;
        let cols = (((w - bx - 4.0 * s) / cell).floor() as usize).min(9);
        if self.surv.side != Side::Closed && cols >= 2 {
            let rows = ((ph - 14.0 * s) / cell).floor() as usize;
            let build = self.surv.side == Side::Build;
            let (list, station) = if build { self.buildable() } else { ((1..self.items.items.len() as ItemId).filter(|&i| self.items.get(i).name != "fishing_bobber").map(|i| (Vec::new(), ItemStack::new(i, 1))).collect(), "") };
            let total_rows = list.len().div_ceil(cols).max(1);
            let max_scroll = total_rows.saturating_sub(rows);
            self.surv.book_scroll = self.surv.book_scroll.min(max_scroll);
            let bw = cols as f32 * cell + 4.0 * s;
            self.ui.rect(bx, y0, bw, ph, [0, 0, 0, 150]);
            let page = if max_scroll > 0 { format!(" {}/{}", self.surv.book_scroll + 1, max_scroll + 1) } else { String::new() };
            let title = if build { format!("Construir{}{}", station, page) } else { format!("Itens{}", page) };
            self.ui.text(bx + 3.0 * s, y0 + 3.0 * s, &title, [200, 200, 200, 255]);
            if mouse.0 >= bx && mouse.0 < bx + bw && mouse.1 >= y0 && mouse.1 < y0 + ph {
                over = true;
            }
            if build && list.is_empty() {
                self.ui.text(bx + 3.0 * s, y0 + 16.0 * s, "Nada a fazer", [200, 200, 200, 255]);
                self.ui.text(bx + 3.0 * s, y0 + 26.0 * s, "com o que tem.", [200, 200, 200, 255]);
            }
            let shift = self.keys.contains(&winit::keyboard::KeyCode::ShiftLeft) || self.keys.contains(&winit::keyboard::KeyCode::ShiftRight);
            let mut tip: Option<usize> = None;
            for r in 0..rows {
                for c in 0..cols {
                    let i = (r + self.surv.book_scroll) * cols + c;
                    let Some((_, result)) = list.get(i) else { continue };
                    let (sx, sy) = (bx + 2.0 * s + c as f32 * cell, y0 + 13.0 * s + r as f32 * cell);
                    let hov = mouse.0 >= sx && mouse.0 < sx + cell && mouse.1 >= sy && mouse.1 < sy + cell;
                    if hov {
                        self.ui.rect(sx, sy, cell, cell, [255, 255, 255, 60]);
                        tip = Some(i);
                        if std::mem::take(&mut self.surv.want_recipe) || (click && !build) {
                            self.surv.book_view = Some((result.item, 0));
                        }
                    }
                    item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, result, sx + s, sy + s, 16.0 * s);
                }
            }
            if let Some(i) = tip {
                let (needs, result) = list[i].clone();
                if build && click {
                    self.build_craft(&needs, result, shift);
                }
                let mut lines = vec![self.items.get(result.item).display_name.clone()];
                if build {
                    // What it uses, summed by item.
                    let mut sum: Vec<(ItemId, u32)> = Vec::new();
                    for n in &needs {
                        let id = n.iter().copied().find(|&id| self.surv.inv.count_of(id) > 0).unwrap_or(n[0]);
                        match sum.iter_mut().find(|e| e.0 == id) {
                            Some(e) => e.1 += 1,
                            None => sum.push((id, 1)),
                        }
                    }
                    for (id, k) in sum {
                        lines.push(format!("{}x {}", k, self.items.get(id).display_name));
                    }
                }
                let tw = lines.iter().map(|l| self.ui.text_width(l, s)).fold(0.0, f32::max);
                let lh = 10.0 * s;
                let th = lines.len() as f32 * lh + 2.0 * s;
                let tx = (mouse.0 + 8.0 * s).min(w - tw - 8.0 * s);
                let ty = mouse.1 - 14.0 * s;
                self.ui.rect(tx, ty, tw + 6.0 * s, th, [16, 0, 16, 235]);
                for (k, l) in lines.iter().enumerate() {
                    self.ui.text(tx + 3.0 * s, ty + 2.0 * s + k as f32 * lh, l, if k == 0 { WHITE } else { [190, 190, 190, 255] });
                }
            }
        }
        if let Some(label) = tab_tip {
            let tw = self.ui.text_width(label, s);
            let tx = (mouse.0 + 8.0 * s).min(w - tw - 8.0 * s);
            self.ui.rect(tx, mouse.1 - 14.0 * s, tw + 6.0 * s, 12.0 * s, [16, 0, 16, 235]);
            self.ui.text(tx + 3.0 * s, mouse.1 - 12.0 * s, label, WHITE);
        }
        // Recipes of the chosen item on the left.
        let Some((item, idx)) = self.surv.book_view else { return over };
        let list = self.recipes_for(item);
        let pw2 = 124.0 * s;
        let px = x0 - pw2 - 4.0 * s;
        if px < 0.0 {
            return over;
        }
        let ph2 = 104.0 * s;
        self.ui.rect(px, y0, pw2, ph2, [0, 0, 0, 170]);
        let in_panel = mouse.0 >= px && mouse.0 < px + pw2 && mouse.1 >= y0 && mouse.1 < y0 + ph2;
        over |= in_panel;
        let mut name = self.items.get(item).display_name.clone();
        while name.chars().count() > 3 && self.ui.text_width(&name, s) > pw2 - 16.0 * s {
            name.pop();
        }
        self.ui.text(px + 3.0 * s, y0 + 3.0 * s, &name, YELLOWISH);
        // Close button.
        let (cx, cy) = (px + pw2 - 9.0 * s, y0 + 2.0 * s);
        self.ui.text(cx, cy, "x", WHITE);
        if click && mouse.0 >= cx - 2.0 * s && mouse.0 < cx + 8.0 * s && mouse.1 >= cy && mouse.1 < cy + 9.0 * s {
            self.surv.book_view = None;
            return over;
        }
        if list.is_empty() {
            self.ui.text(px + 3.0 * s, y0 + 20.0 * s, "Sem receita:", [200, 200, 200, 255]);
            self.ui.text(px + 3.0 * s, y0 + 30.0 * s, "ache no mundo", [200, 200, 200, 255]);
            return over;
        }
        let idx = idx % list.len();
        // Ingredient choices (any plank...) take turns once a second.
        let tick = self.now as usize;
        let pick = |opts: &Vec<ItemId>| opts[tick % opts.len().max(1)];
        let gy = y0 + 16.0 * s;
        let gx = px + 4.0 * s;
        let slot_bg = |ui: &mut UiBatch, x: f32, y: f32| {
            ui.rect(x, y, cell - s, cell - s, [139, 139, 139, 255]);
        };
        let mut fill: Option<(usize, Vec<Option<Vec<ItemId>>>)> = None;
        match &list[idx] {
            BookRecipe::Grid { w: rw, cells, result } => {
                for r in 0..3 {
                    for c in 0..3 {
                        slot_bg(&mut self.ui, gx + c as f32 * cell, gy + r as f32 * cell);
                    }
                }
                for (i, cellv) in cells.iter().enumerate() {
                    if let Some(opts) = cellv {
                        if opts.is_empty() {
                            continue;
                        }
                        let (c, r) = (i % rw, i / rw);
                        item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(pick(opts), 1), gx + c as f32 * cell, gy + r as f32 * cell, 16.0 * s);
                    }
                }
                let rx = gx + 3.0 * cell + 6.0 * s;
                self.ui.text(rx, gy + cell + 4.0 * s, ">", WHITE);
                slot_bg(&mut self.ui, rx + 8.0 * s, gy + cell);
                item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, result, rx + 8.0 * s, gy + cell, 16.0 * s);
                let rows = cells.len().div_ceil(*rw);
                let need = (*rw).max(rows);
                let label = if need <= 2 { "Grade 2x2" } else { "Mesa 3x3" };
                self.ui.text(gx, gy + 3.0 * cell + 3.0 * s, label, [200, 200, 200, 255]);
                fill = Some((*rw, cells.clone()));
            }
            BookRecipe::Smelt { input, result } => {
                slot_bg(&mut self.ui, gx, gy + cell);
                if !input.is_empty() {
                    item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &ItemStack::new(pick(input), 1), gx, gy + cell, 16.0 * s);
                }
                self.ui.text(gx + cell + 4.0 * s, gy + cell + 4.0 * s, ">", WHITE);
                slot_bg(&mut self.ui, gx + cell + 14.0 * s, gy + cell);
                item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, result, gx + cell + 14.0 * s, gy + cell, 16.0 * s);
                self.ui.text(gx, gy + 3.0 * cell + 3.0 * s, "Fornalha", [200, 200, 200, 255]);
            }
        }
        // Next recipe, and fill the crafting grid from the inventory.
        let by = y0 + ph2 - 13.0 * s;
        if list.len() > 1 {
            let t = format!("{}/{} >", idx + 1, list.len());
            if self.ui.button(px + 3.0 * s, by, 44.0 * s, 11.0 * s, &t, mouse) && click {
                self.surv.book_view = Some((item, idx + 1));
            }
        }
        let grid_open = matches!(self.surv.open, Open::Craft | Open::Table);
        if let (Some((rw, cells)), true) = (fill, grid_open) {
            let n = self.surv.grid_size();
            let rows = cells.len().div_ceil(rw);
            if rw <= n && rows <= n && self.ui.button(px + 50.0 * s, by, 70.0 * s, 11.0 * s, "Preencher", mouse) && click {
                self.fill_grid(rw, &cells);
            }
        }
        over
    }

    /// Is there a block with this name within a few blocks of the player?
    fn near_block(&self, name: &str) -> bool {
        let Some(id) = self.reg.id(name) else { return false };
        let c = self.body.pos.floor().as_ivec3();
        for dy in -2..=3 {
            for dz in -4..=4 {
                for dx in -4..=4 {
                    if self.world.block_or_air(c + IVec3::new(dx, dy, dz)) == id {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Inventory slots that would pay for these ingredients (one item each), if there are enough.
    fn pay_for(&self, needs: &[Vec<ItemId>]) -> Option<Vec<usize>> {
        let mut left: Vec<u8> = self.surv.inv.slots.iter().map(|s| s.map_or(0, |s| if s.damage == 0 { s.count } else { 0 })).collect();
        let mut out = Vec::with_capacity(needs.len());
        for n in needs {
            let k = (0..left.len()).find(|&k| left[k] > 0 && self.surv.inv.slots[k].is_some_and(|st| n.contains(&st.item)))?;
            left[k] -= 1;
            out.push(k);
        }
        Some(out)
    }

    /// Every recipe the player can make right now from the inventory (like Terraria's crafting
    /// list): 2×2 anywhere, 3×3 next to a crafting table, smelting next to a furnace.
    fn buildable(&self) -> (Vec<(Vec<Vec<ItemId>>, ItemStack)>, &'static str) {
        let table = self.near_block("crafting_table");
        let furnace = self.near_block("furnace");
        let mut out: Vec<(Vec<Vec<ItemId>>, ItemStack)> = Vec::new();
        for r in &self.recipes.shaped {
            if (r.w > 2 || r.h > 2) && !table {
                continue;
            }
            out.push((r.cells.iter().flatten().filter(|c| !c.is_empty()).cloned().collect(), r.result));
        }
        for r in &self.recipes.shapeless {
            if r.ingredients.len() > 4 && !table {
                continue;
            }
            out.push((r.ingredients.clone(), r.result));
        }
        if furnace {
            for r in &self.recipes.smelting {
                out.push((vec![r.input.clone()], r.result));
            }
        }
        out.retain(|(needs, _)| !needs.is_empty() && self.pay_for(needs).is_some());
        // One entry per result item.
        let mut seen = std::collections::HashSet::new();
        out.retain(|(_, r)| seen.insert(r.item));
        let station = match (table, furnace) {
            (true, true) => " (mesa+forno)",
            (true, false) => " (mesa)",
            (false, true) => " (forno)",
            _ => "",
        };
        (out, station)
    }

    /// Make one (or, with shift, a stack's worth) from the build list.
    fn build_craft(&mut self, needs: &[Vec<ItemId>], result: ItemStack, all: bool) {
        let max = self.items.get(result.item).max_stack.max(1) as u32;
        let times = if all { (max / result.count.max(1) as u32).max(1) } else { 1 };
        let mut made = 0;
        for _ in 0..times {
            let Some(pay) = self.pay_for(needs) else { break };
            if !all {
                // Into the hand, like taking from the crafting output.
                match self.surv.cursor {
                    None => {}
                    Some(c) if c.stacks_with(&result) && c.count as u32 + result.count as u32 <= max => {}
                    _ => break,
                }
            }
            for k in pay {
                self.surv.inv.take(k, 1);
            }
            if all {
                if let Some(left) = self.surv.inv.add(&self.items, result) {
                    let f = self.forward();
                    self.spawn_drop(self.body.eye(), left, f * 2.0);
                }
            } else {
                self.surv.cursor = Some(match self.surv.cursor {
                    Some(c) => ItemStack { count: c.count + result.count, ..c },
                    None => result,
                });
            }
            made += 1;
        }
        if made > 0 {
            self.sfx(Sfx::Pickup);
        }
    }

    /// Move one of each ingredient from the inventory into the crafting grid.
    fn fill_grid(&mut self, rw: usize, cells: &[Option<Vec<ItemId>>]) {
        let n = self.surv.grid_size();
        // Clear the grid back into the inventory first.
        for i in 0..9 {
            if let Some(st) = self.surv.craft[i].take() {
                if let Some(left) = self.surv.inv.add(&self.items, st) {
                    self.surv.craft[i] = Some(left);
                    return;
                }
            }
        }
        let mut taken: Vec<(usize, ItemStack)> = Vec::new();
        for (i, c) in cells.iter().enumerate() {
            let Some(opts) = c else { continue };
            let slot = (0..INV_SIZE).find(|&k| self.surv.inv.slots[k].is_some_and(|st| opts.contains(&st.item) && st.damage == 0));
            let Some(k) = slot else {
                // Missing something: put back what was moved.
                for (_, st) in taken {
                    let _ = self.surv.inv.add(&self.items, st);
                }
                self.toast("Faltam ingredientes", 2.0);
                return;
            };
            let st = self.surv.inv.take(k, 1).unwrap();
            let (cx, cy) = (i % rw, i / rw);
            taken.push((cy * n + cx, st));
        }
        for (g, st) in taken {
            self.surv.craft[g] = Some(st);
        }
    }
}

const YELLOWISH: [u8; 4] = [255, 255, 160, 255];

/// The live world as a place for a sapling to grow into a tree.
struct WorldTrees<'a>(&'a mut crate::world::World);

impl TreeSpace for WorldTrees<'_> {
    fn get(&self, x: i32, y: i32, z: i32) -> voxel::BlockId {
        // Unloaded places read as solid so nothing is written there.
        self.0.block(IVec3::new(x, y, z)).unwrap_or(voxel::BlockId::MAX)
    }

    fn set(&mut self, x: i32, y: i32, z: i32, id: voxel::BlockId) {
        self.0.set_block(IVec3::new(x, y, z), id);
    }
}
