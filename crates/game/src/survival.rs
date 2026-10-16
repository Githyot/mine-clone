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
use voxel::{BlockId, BlockRegistry, AIR};

use crate::audio::{Material, Sfx};
use crate::item_render::GpuItem;
use crate::ui::{UiBatch, WHITE};
use crate::{Game, Screen};

/// Player inventory: 0..9 is the hotbar, 9..36 the main rows.
pub const INV_SIZE: usize = 36;
const CHEST_SIZE: usize = 27;
const SMELT_TIME: f32 = 10.0;
const PICKUP_DELAY: f32 = 0.6;
const DESPAWN: f32 = 300.0;

pub struct Dropped {
    pub stack: ItemStack,
    pub body: Body,
    pub age: f32,
    pub spin: f32,
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

pub enum Container {
    Chest(Inventory),
    Furnace(Furnace),
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
    /// Saplings planted by the player and seconds until they grow.
    pub saplings: Vec<(IVec3, f32)>,
    pub eat: f32,
    eat_sound: f32,
    heal_t: f32,
    starve_t: f32,
    pickup_sound: f32,
    last_pos: Vec3,
    was_ground: bool,
    rng: Rng,
}

impl Survival {
    pub fn new() -> Survival {
        Survival {
            inv: Inventory::new(INV_SIZE),
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
            saplings: Vec::new(),
            eat: 0.0,
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
        sv.saplings = s.saplings.into_iter().map(|(p, t)| (IVec3::from(p), t)).collect();
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
    if let Some(t) = &it.tool {
        if stack.damage > 0 {
            let k = 1.0 - stack.damage as f32 / t.durability as f32;
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

/// One clickable slot in a container screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Slot {
    Inv(usize),
    Craft(usize),
    CraftOut,
    Cont(usize),
}

// ---------------------------------------------------------------------------------------------

impl Game {
    pub(crate) fn held_stack(&self) -> Option<ItemStack> {
        self.surv.inv.slots[self.selected]
    }

    pub(crate) fn held_tool(&self) -> Option<ToolDef> {
        self.held_stack().and_then(|s| self.items.get(s.item).tool.clone())
    }

    /// Wear the held tool; it breaks when worn out.
    fn wear_held(&mut self, amount: u16) {
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
        self.surv.drops.push(Dropped { stack, body, age: 0.0, spin });
    }

    fn pop_drop(&mut self, center: Vec3, stack: ItemStack) {
        let r = &mut self.surv.rng;
        let v = Vec3::new(r.f32() - 0.5, 0.0, r.f32() - 0.5) * 2.0 + Vec3::Y * 3.0;
        self.spawn_drop(center, stack, v);
    }

    fn spawn_orbs(&mut self, at: Vec3, mut total: u32) {
        while total > 0 {
            let v = total.min(if total > 16 { 7 } else { 3 });
            total -= v;
            let r = &mut self.surv.rng;
            let vel = Vec3::new(r.f32() - 0.5, 1.0 + r.f32(), r.f32() - 0.5) * 2.5;
            self.surv.orbs.push(Orb { pos: at, vel, value: v, age: 0.0 });
        }
    }

    /// A block was broken by the player in survival: drops, experience, tool wear.
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
        match self.held_tool() {
            Some(t) => {
                self.wear_held(if t.kind == ToolKind::Sword { 1 } else { 2 });
                t.damage
            }
            None => 2.0,
        }
    }

    pub(crate) fn surv_died(&mut self) {
        let at = self.body.pos + Vec3::Y * 0.8;
        let slots: Vec<ItemStack> = self.surv.inv.slots.iter_mut().filter_map(|s| s.take()).collect();
        for s in slots {
            self.pop_drop(at, s);
        }
        if let Some(c) = self.surv.cursor.take() {
            self.pop_drop(at, c);
        }
        let grid: Vec<ItemStack> = self.surv.craft.iter_mut().filter_map(|s| s.take()).collect();
        for s in grid {
            self.pop_drop(at, s);
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
                        self.surv.containers.entry(hit.block).or_insert_with(|| Container::Chest(Inventory::new(CHEST_SIZE)));
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
        if item.food.is_some() {
            // Eating is handled while the button is held (update).
            return false;
        }
        let Some(block) = item.places else { return false };
        if let Some(pos) = self.place_block_as(block) {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
            if self.reg.get(block).name.ends_with("_sapling") {
                let t = 60.0 + self.surv.rng.f32() * 120.0;
                self.surv.saplings.push((pos, t));
            }
            return true;
        }
        false
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
        }
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
        let alive = self.screen != Screen::Dead && self.survival;
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
            if alive && d.age > PICKUP_DELAY && dist < 2.2 {
                d.body.vel += to.normalize_or_zero() * dt * 30.0;
                if dist < 0.9 {
                    picked.push(i);
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
            let Some(kind) = name.strip_suffix("_sapling") else { continue };
            let log = self.reg.id_or_air(&format!("{kind}_log"));
            let leaves = self.reg.id_or_air(&format!("{kind}_leaves"));
            if log == AIR || leaves == AIR {
                continue;
            }
            // Needs room to grow.
            let h = 4 + (self.surv.rng.below(3) as i32);
            if (1..=h + 1).any(|y| self.world.block_or_air(p + IVec3::Y * y) != AIR) {
                self.surv.saplings.push((p, 30.0));
                continue;
            }
            for y in 0..h {
                self.world.set_block(p + IVec3::Y * y, log);
            }
            let top = p + IVec3::Y * h;
            for dy in -2i32..=1 {
                let r: i32 = if dy >= 0 { 1 } else { 2 };
                for dx in -r..=r {
                    for dz in -r..=r {
                        if dx.abs() == r && dz.abs() == r && (dy == 1 || self.surv.rng.below(2) == 0) {
                            continue;
                        }
                        let q = top + IVec3::new(dx, dy, dz);
                        if self.world.block_or_air(q) == AIR {
                            self.world.set_block(q, leaves);
                        }
                    }
                }
            }
        }
    }

    /// Instances for the item renderer: drops and orbs near the camera.
    pub(crate) fn drop_instances(&self, cam_block: IVec3) -> Vec<GpuItem> {
        let c = cam_block.as_vec3();
        let mut out = Vec::with_capacity(self.surv.drops.len() + self.surv.orbs.len());
        // The orb sprite is loaded right after the item sprites.
        let orb_layer = (self.item_base as usize + self.items.textures.len()) as f32;
        for d in &self.surv.drops {
            let it = self.items.get(d.stack.item);
            let bob = (d.age * 2.5).sin() * 0.06 + 0.18;
            let p = d.body.pos + Vec3::Y * bob - c;
            let l = self.world.light((d.body.pos + Vec3::Y * 0.2).floor().as_ivec3());
            let (sky, blk) = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let (flat, layers, size) = match it.look {
                Look::Block(b) => {
                    let bl = self.reg.get(b);
                    if matches!(bl.shape, Shape::Cross | Shape::Torch) {
                        (1.0, [bl.faces[0] as f32; 3], 0.4)
                    } else {
                        (0.0, [bl.faces[0] as f32, bl.faces[2] as f32, bl.faces[3] as f32], 0.25)
                    }
                }
                Look::Sprite(l) => (1.0, [(self.item_base + l) as f32; 3], 0.4),
            };
            // Stacks show as two or three copies, like the classic game.
            let copies = if d.stack.count > 16 { 3 } else if d.stack.count > 1 { 2 } else { 1 };
            for k in 0..copies {
                let off = Vec3::new(k as f32 * 0.05, k as f32 * 0.04, -(k as f32) * 0.05);
                out.push(GpuItem { pos: [p.x + off.x, p.y + off.y, p.z + off.z, size], info: [d.spin, flat, sky, blk], layers: [layers[0], layers[1], layers[2], 0.0] });
            }
        }
        for o in &self.surv.orbs {
            let p = o.pos - c + Vec3::Y * ((o.age * 4.0).sin() * 0.05);
            let size = 0.12 + (o.value as f32).sqrt() * 0.04;
            out.push(GpuItem { pos: [p.x, p.y, p.z, size], info: [self.yaw, 1.0, 1.0, 1.0], layers: [orb_layer, orb_layer, orb_layer, 0.5] });
        }
        out
    }

    // -----------------------------------------------------------------------------------------
    // Interface.

    /// Hotbar items, hunger and experience over the classic hotbar frame.
    pub(crate) fn draw_survival_hud(&mut self, hx: f32, hy: f32, cell: f32) {
        let s = self.ui.scale;
        for i in 0..9 {
            if let Some(st) = self.surv.inv.slots[i] {
                item_icon(&mut self.ui, &self.reg, &self.items, self.item_base, &st, hx + i as f32 * cell + 3.0 * s, hy + 2.5 * s, 14.0 * s);
            }
        }
        // Hunger: ten drumsticks on the right, above the hotbar.
        let y = hy - 11.0 * s;
        let shake = self.surv.saturation <= 0.0 && self.surv.hunger <= 6.0;
        for i in 0..10 {
            let x = hx + cell * 9.0 - (i + 1) as f32 * 8.0 * s;
            let v = self.surv.hunger - i as f32 * 2.0;
            let dy = if shake && ((self.now * 12.0) as i32 + i) % 3 == 0 { s } else { 0.0 };
            draw_drumstick(&mut self.ui, x, y + dy, s, 1.0, true);
            if v > 0.0 {
                draw_drumstick(&mut self.ui, x, y + dy, s, if v >= 2.0 { 1.0 } else { 0.5 }, false);
            }
        }
        // Experience bar and level.
        let (lvl, k) = self.surv.level();
        let bw = cell * 9.0;
        let by = hy - 3.5 * s;
        self.ui.rect(hx, by, bw, 2.5 * s, [0, 0, 0, 200]);
        self.ui.rect(hx, by, bw * k, 2.5 * s, [128, 255, 32, 255]);
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
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 110]);
        self.ui.rect(x0, y0, pw, ph, [198, 198, 198, 255]);
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
                    // A little figure of the player in the corner, like the classic screen.
                    self.ui.rect(x0 + 26.0 * s, y0 + 8.0 * s, 52.0 * s, 62.0 * s, [0, 0, 0, 255]);
                    self.ui.rect(x0 + 27.0 * s, y0 + 9.0 * s, 50.0 * s, 60.0 * s, [110, 110, 110, 255]);
                    let (cx, cy) = (x0 + 52.0 * s, y0 + 12.0 * s);
                    self.ui.rect(cx - 4.0 * s, cy, 8.0 * s, 8.0 * s, [200, 150, 110, 255]);
                    self.ui.rect(cx - 4.0 * s, cy, 8.0 * s, 3.0 * s, [110, 50, 30, 255]);
                    self.ui.rect(cx - 4.0 * s, cy + 8.0 * s, 8.0 * s, 12.0 * s, [60, 110, 50, 255]);
                    self.ui.rect(cx - 8.0 * s, cy + 8.0 * s, 4.0 * s, 12.0 * s, [60, 110, 50, 255]);
                    self.ui.rect(cx + 4.0 * s, cy + 8.0 * s, 4.0 * s, 12.0 * s, [60, 110, 50, 255]);
                    self.ui.rect(cx - 4.0 * s, cy + 20.0 * s, 8.0 * s, 12.0 * s, [170, 90, 50, 255]);
                    self.ui.rect(cx - 4.0 * s, cy + 32.0 * s, 8.0 * s, 3.0 * s, [50, 35, 25, 255]);
                    let lvl = self.surv.level().0;
                    self.ui.label(x0 + 29.0 * s, y0 + 59.0 * s, &format!("Nível {lvl}"), WHITE, s);
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
        // Clicks.
        let shift = self.keys.contains(&winit::keyboard::KeyCode::ShiftLeft) || self.keys.contains(&winit::keyboard::KeyCode::ShiftRight);
        let inside = mouse.0 >= x0 && mouse.0 < x0 + pw && mouse.1 >= y0 && mouse.1 < y0 + ph;
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
                    }
                }
                None => {}
            }
        }
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
        if self.surv.cursor.is_none() {
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
        self.ui.text_centered(w / 2.0, y0 + ph + 4.0 * s, "Esq.: pegar/pôr  Dir.: metade/um  Shift: mover  1-9: trocar", WHITE);
    }

    pub(crate) fn close_survival_screen(&mut self) {
        self.set_screen(Screen::Playing);
    }

    fn craft_result(&self) -> Option<ItemStack> {
        let n = self.surv.grid_size();
        let grid: Vec<Option<ItemStack>> = (0..n * n).map(|i| self.surv.craft[i]).collect();
        self.recipes.craft(&grid, n)
    }

    fn slot_get(&self, slot: Slot) -> Option<ItemStack> {
        match slot {
            Slot::Inv(i) => self.surv.inv.slots[i],
            Slot::Craft(i) => self.surv.craft[i],
            Slot::CraftOut => None,
            Slot::Cont(i) => match self.open_container_inv() {
                Some(inv) => inv.slots.get(i).copied().flatten(),
                None => None,
            },
        }
    }

    fn slot_set(&mut self, slot: Slot, v: Option<ItemStack>) {
        match slot {
            Slot::Inv(i) => self.surv.inv.slots[i] = v,
            Slot::Craft(i) => self.surv.craft[i] = v,
            Slot::CraftOut => {}
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

    fn click_slot(&mut self, slot: Slot, right: bool, shift: bool) {
        let furnace_out = slot == Slot::Cont(2) && matches!(self.surv.open, Open::Furnace(_));
        if slot == Slot::CraftOut {
            self.click_craft_output(shift);
            return;
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
            self.surv.drops.last_mut().unwrap().age = 1.0 + i as f32 * 0.7;
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
