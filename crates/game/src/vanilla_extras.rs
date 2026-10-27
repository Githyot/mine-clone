//! Everyday vanilla things that were missing (lote 1 of `plan/o-que-falta.md`): ladders,
//! shields and the totem of undying, shears and shearing sheep, snowballs and eggs, the
//! jukebox and its records (our own tracks), note blocks, cakes, sweet berry bushes, kelp
//! and bamboo that grow, fire that spreads and burns out, wet sponges, carved pumpkins,
//! chickens laying eggs, pigs steered with a carrot on a stick, books you write in, stews
//! that give their bowl back, and the loot that brings chainmail, records and totems.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use glam::{IVec3, Vec3};
use voxel::item::{Inventory, ItemStack};
use voxel::physics::Terrain;
use voxel::{BlockId, BlockRegistry, AIR};

use crate::audio::{Material, Sfx};
use crate::survival::Container;
use crate::Game;

/// The records: (disc item, track id in tracks.json).
pub const DISCS: [(&str, &str); 9] = [
    ("music_disc_first_light", "first_light"),
    ("music_disc_moss", "moss"),
    ("music_disc_still_water", "still_water"),
    ("music_disc_block_by_block", "block_by_block"),
    ("music_disc_embers", "embers"),
    ("music_disc_void", "void"),
    ("music_disc_beginning", "beginning"),
    ("music_disc_wandering", "wandering"),
    ("music_disc_harvest", "harvest"),
];

/// How far a record can be heard (blocks).
const JUKEBOX_RANGE: f32 = 48.0;
/// Lines and characters per line on a book page, and pages per book.
pub const BOOK_LINES: usize = 12;
pub const BOOK_LINE_LEN: usize = 22;
const BOOK_PAGES: usize = 50;

#[derive(Default, serde::Serialize, serde::Deserialize, Clone)]
pub struct Book {
    pub title: String,
    pub pages: Vec<String>,
    pub signed: bool,
}

/// A book open on screen: which one (its number in the library), the page, and whether it can
/// still be written in.
pub struct BookOpen {
    pub id: u16,
    pub page: usize,
    pub write: bool,
}

pub struct Extras {
    pub ladder: BlockId,
    kelp: BlockId,
    bamboo: BlockId,
    berries: [BlockId; 4],
    pub fire: BlockId,
    soul_fire: BlockId,
    water: BlockId,
    sponge: BlockId,
    wet_sponge: BlockId,
    pumpkin: BlockId,
    carved: BlockId,
    cake: BlockId,
    /// Blocks fire can eat (wood, leaves, wool, hay...).
    flammable: Vec<bool>,
    /// Fires burning now and how long each has burned.
    fires: Vec<(IVec3, f32)>,
    fire_t: f32,
    /// Seconds the player keeps burning after touching fire.
    pub burning: f32,
    burn_tick: f32,
    /// The jukebox playing now.
    playing: Option<IVec3>,
    /// The pitch of each note block that was tuned (0..24).
    notes: HashMap<[i32; 3], u8>,
    /// Every book written in this world, by number (1-based in item stacks).
    pub books: Vec<Book>,
    pub book: Option<BookOpen>,
    /// The shield is up this frame.
    pub blocking: bool,
    egg_t: f32,
    climb_t: f32,
    regrow_t: f32,
    /// Sheep sheared lately (so a sheep can't be sheared twice in a moment).
    sheared: HashSet<u32>,
    /// Snowballs and eggs in the air.
    thrown: Vec<Thrown>,
}

struct Thrown {
    pos: Vec3,
    vel: Vec3,
    item: u16,
    life: f32,
}

impl Extras {
    pub fn new(reg: &BlockRegistry) -> Extras {
        let g = |n: &str| reg.id_or_air(n);
        let flammable = reg
            .blocks
            .iter()
            .map(|b| {
                let n = reg.get(b.base).name.as_str();
                (n.ends_with("_planks") || n.ends_with("_log") || n.ends_with("_wood") || n.ends_with("_leaves") || n.ends_with("_wool") || n.ends_with("_carpet") || n == "bookshelf" || n == "hay_block" || n.ends_with("_fence") && !n.contains("nether") || n == "dried_kelp_block" || n == "bamboo" || n == "tnt" || n.ends_with("_stairs") && n.contains("oak") || n.ends_with("_slab") && n.contains("oak") || n == "vine" || n == "short_grass" || n == "tall_grass")
                    && !n.starts_with("crimson")
                    && !n.starts_with("warped")
            })
            .collect();
        Extras {
            ladder: g("ladder"),
            kelp: g("kelp"),
            bamboo: g("bamboo"),
            berries: [0, 1, 2, 3].map(|k| g(&format!("sweet_berry_bush_stage{k}"))),
            fire: g("fire"),
            soul_fire: g("soul_fire"),
            water: g("water"),
            sponge: g("sponge"),
            wet_sponge: g("wet_sponge"),
            pumpkin: g("pumpkin"),
            carved: g("carved_pumpkin"),
            cake: g("cake"),
            flammable,
            fires: Vec::new(),
            fire_t: 0.0,
            burning: 0.0,
            burn_tick: 0.0,
            playing: None,
            notes: HashMap::new(),
            books: Vec::new(),
            book: None,
            blocking: false,
            egg_t: 0.0,
            climb_t: 0.0,
            regrow_t: 0.0,
            sheared: HashSet::new(),
            thrown: Vec::new(),
        }
    }

    /// Note pitches live with each dimension's blocks; books with the world.
    pub fn load(&mut self, dim_dir: &Path, world_dir: &Path) {
        self.notes = std::fs::read_to_string(dim_dir.join("notes.ron")).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default();
        self.books = std::fs::read_to_string(world_dir.join("books.ron")).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default();
        self.fires.clear();
        self.thrown.clear();
        self.playing = None;
        self.book = None;
    }

    pub fn save(&self, dim_dir: &Path, world_dir: &Path) {
        if let Ok(t) = ron::to_string(&self.notes) {
            let _ = std::fs::write(dim_dir.join("notes.ron"), t);
        }
        if !self.books.is_empty() {
            if let Ok(t) = ron::ser::to_string_pretty(&self.books, ron::ser::PrettyConfig::default()) {
                let _ = std::fs::write(world_dir.join("books.ron"), t);
            }
        }
    }

    /// Blocks that change on random ticks here.
    pub fn ticks(&self, id: BlockId) -> bool {
        id != AIR && (id == self.kelp || id == self.bamboo || self.berries[..3].contains(&id))
    }

    fn is_fire(&self, id: BlockId) -> bool {
        id != AIR && (id == self.fire || id == self.soul_fire)
    }
}

/// The block a note block stands on picks its instrument.
fn instrument(below: &str) -> u8 {
    if below.contains("stone") || below.contains("deepslate") || below.contains("brick") || below == "netherrack" || below.ends_with("_ore") || below == "obsidian" {
        1
    } else if below.ends_with("_planks") || below.ends_with("_log") || below.ends_with("_wood") {
        3
    } else if below == "sand" || below == "gravel" || below.ends_with("_concrete_powder") || below == "red_sand" {
        2
    } else {
        0
    }
}

impl Game {
    pub(crate) fn update_extras(&mut self, dt: f32, use_held: bool) {
        self.update_shield(use_held);
        self.update_fire(dt);
        self.update_jukebox();
        if dt > 0.0 {
            self.update_thrown(dt);
            self.update_vanilla_mobs(dt);
            self.climb_sounds(dt);
            self.update_animals_extras(dt);
        }
    }

    /// Wooden creaks while going up or down a ladder.
    fn climb_sounds(&mut self, dt: f32) {
        let feet = self.body.pos.floor().as_ivec3();
        let on = [feet, feet + IVec3::Y].iter().any(|&p| self.reg.get(self.world.block_or_air(p)).base == self.extras.ladder);
        if !on || self.extras.ladder == AIR || self.body.on_ground || self.body.vel.y.abs() < 0.5 {
            self.extras.climb_t = 0.0;
            return;
        }
        self.extras.climb_t += dt;
        if self.extras.climb_t >= 0.45 {
            self.extras.climb_t = 0.0;
            self.sfx(Sfx::Step(Material::Wood));
        }
    }

    // ---------------------------------------------------------------------------------------
    // Shield and totem.

    fn update_shield(&mut self, use_held: bool) {
        let main = self.held_key();
        let off = self.surv.offhand.map(|s| self.items.get(s.item).name.clone()).unwrap_or_default();
        // The shield goes up with the use button, unless the main hand is busy with something
        // it uses itself (food, a bow...).
        let main_busy = !main.is_empty() && main != "shield" && (self.held_stack().is_some_and(|s| self.items.get(s.item).food.is_some()) || matches!(main.as_str(), "bow" | "crossbow" | "trident" | "fishing_rod"));
        self.extras.blocking = self.survival && use_held && self.screen == crate::Screen::Playing && (main == "shield" || off == "shield" && !main_busy);
    }

    /// A hit from the front while the shield is up does nothing (but wears the shield).
    pub(crate) fn shield_block(&mut self, amount: f32, from: Option<Vec3>) -> f32 {
        let Some(f) = from else { return amount };
        if !self.extras.blocking || amount <= 0.0 {
            return amount;
        }
        let to = f - self.body.eye();
        let look = self.forward();
        if Vec3::new(to.x, 0.0, to.z).normalize_or_zero().dot(Vec3::new(look.x, 0.0, look.z).normalize_or_zero()) < 0.1 {
            return amount;
        }
        let wear = (amount.ceil() as u16).max(1);
        self.wear_shield(wear);
        self.sfx(Sfx::Hit(Material::Wood));
        self.pads.rumble(0.3, 0.3, 0.1);
        0.0
    }

    fn wear_shield(&mut self, wear: u16) {
        let Some(sid) = self.items.id("shield") else { return };
        let max = self.items.get(sid).max_wear().unwrap_or(336);
        let sel = self.selected;
        let slot = if self.surv.inv.slots[sel].is_some_and(|s| s.item == sid) { Some(&mut self.surv.inv.slots[sel]) } else if self.surv.offhand.is_some_and(|s| s.item == sid) { Some(&mut self.surv.offhand) } else { None };
        let Some(slot) = slot else { return };
        let Some(s) = *slot else { return };
        let d = s.wear() + wear;
        if d >= max {
            *slot = None;
            self.sfx(Sfx::Break(Material::Wood));
            self.toast("Seu escudo quebrou", 2.0);
        } else {
            *slot = Some(s.with_wear(d));
        }
    }

    /// About to die with a totem of undying in a hand: it breaks instead, and you live.
    pub(crate) fn use_totem(&mut self) -> bool {
        let Some(tid) = self.items.id("totem_of_undying") else { return false };
        let sel = self.selected;
        if self.surv.inv.slots[sel].is_some_and(|s| s.item == tid) {
            self.surv.inv.slots[sel] = None;
        } else if self.surv.offhand.is_some_and(|s| s.item == tid) {
            self.surv.offhand = None;
        } else {
            return false;
        }
        use crate::effects::*;
        self.health = 1.0;
        self.add_effect(REGENERATION, 1, 45.0, false);
        self.add_effect(ABSORPTION, 1, 5.0, false);
        self.add_effect(FIRE_RESISTANCE, 0, 40.0, false);
        self.extras.burning = 0.0;
        let glow = self.glow_layer();
        let at = self.body.pos + Vec3::Y;
        for k in 0..40 {
            let a = k as f32 * 0.7;
            let v = Vec3::new(a.cos() * 2.5, 1.5 + (k % 5) as f32 * 0.6, a.sin() * 2.5);
            let c = if k % 2 == 0 { [1.2, 1.0, 0.3] } else { [0.4, 1.1, 0.4] };
            self.particles.glow(at, v, glow, c, 0.08, 2.0);
        }
        self.sfx(Sfx::Respawn);
        self.toast("O totem te salvou!", 3.0);
        true
    }

    // ---------------------------------------------------------------------------------------
    // Using items and blocks.

    /// Right click with an item from this file. True when it did something.
    pub(crate) fn extras_item_use(&mut self, name: &str) -> bool {
        match name {
            "snowball" | "egg" => return self.throw_item(name),
            "shield" => return true,
            "writable_book" | "written_book" => return self.open_book(),
            "sweet_berries" => return self.plant_berries(),
            "kelp" => return self.plant_kelp(),
            "bamboo" => return self.plant_bamboo(),
            "shears" => {
                if self.shear_mob() {
                    return true;
                }
                return self.carve_pumpkin();
            }
            "armor_stand" => return self.place_armor_stand(),
            _ => {}
        }
        if name.ends_with("_shovel") && self.make_path() {
            return true;
        }
        if name.ends_with("_hoe") && self.hoe_rough_dirt() {
            return true;
        }
        false
    }

    /// Right click on a block from this file (before the held item gets its turn).
    pub(crate) fn extras_block_use(&mut self, name: &str, p: IVec3) -> bool {
        match name {
            "jukebox" => self.jukebox_use(p),
            "note_block" => {
                let k = self.extras.notes.entry(p.to_array()).or_insert(0);
                *k = (*k + 1) % 25;
                self.play_note_block(p);
                self.anim.start_swing();
                true
            }
            "cake" => self.eat_cake(p),
            n if n.starts_with("sweet_berry_bush_stage") && (n.ends_with('2') || n.ends_with('3')) => {
                let ripe = n.ends_with('3');
                let id = self.extras.berries[1];
                self.world.set_block(p, id);
                if let Some(b) = self.items.id("sweet_berries") {
                    let n = if ripe { 2 + self.surv.rng.below(2) } else { 1 + self.surv.rng.below(2) };
                    self.pop_drop(p.as_vec3() + Vec3::splat(0.5), ItemStack::new(b, n as u8));
                }
                self.sfx(Sfx::Place(Material::Leaves));
                self.anim.start_swing();
                true
            }
            n if n.ends_with("_bed") && n != "red_bed" => {
                // Every colour of bed sleeps the same.
                false
            }
            _ => false,
        }
    }

    /// Right click on a creature with the held item (shears on a sheep).
    pub(crate) fn extras_mob_use(&mut self) -> bool {
        if self.vanilla_mob_use() {
            return true;
        }
        let held = self.held_key();
        if held != "shears" {
            return false;
        }
        self.shear_mob()
    }

    fn pick_mob(&self, reach: f32) -> Option<usize> {
        let (i, t) = self.mobs.pick(self.body.eye(), self.forward(), reach)?;
        if self.target.is_some_and(|h| h.distance < t && self.reg.get(self.world.block_or_air(h.block)).solid) {
            return None;
        }
        Some(i)
    }

    fn shear_mob(&mut self) -> bool {
        let Some(i) = self.pick_mob(4.0) else { return false };
        let name = self.mobs.reg.defs[self.mobs.list[i].def].name.clone();
        if name != "sheep" || !self.mobs.list[i].alive() {
            return false;
        }
        let Some(d) = self.mobs.reg.id("sheep_sheared") else { return false };
        let at = self.mobs.list[i].body.pos + Vec3::Y;
        let m = &mut self.mobs.list[i];
        m.def = d;
        if let Some(w) = self.items.id("white_wool") {
            let n = 1 + self.surv.rng.below(3) as u8;
            self.pop_drop(at, ItemStack::new(w, n));
        }
        self.sfx(Sfx::Place(Material::Cloth));
        self.anim.start_swing();
        self.wear_held(1);
        true
    }

    fn carve_pumpkin(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        if self.world.block_or_air(hit.block) != self.extras.pumpkin || self.extras.carved == AIR || hit.normal.y != 0 {
            return false;
        }
        // The face looks out of the side clicked.
        let facing = match (hit.normal.x, hit.normal.z) {
            (0, -1) => 0,
            (1, 0) => 1,
            (0, 1) => 2,
            _ => 3,
        };
        let id = self.variant_id(self.extras.carved, facing);
        self.world.set_block(hit.block, id);
        if let Some(s) = self.items.id("pumpkin_seeds") {
            self.pop_drop((hit.block + hit.normal).as_vec3() + Vec3::splat(0.5), ItemStack::new(s, 4));
        }
        self.sfx(Sfx::Break(Material::Leaves));
        self.anim.start_swing();
        self.wear_held(1);
        true
    }

    /// A shovel on grass makes a path.
    fn make_path(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        let name = self.reg.get(self.world.block_or_air(hit.block)).name.clone();
        if !matches!(name.as_str(), "grass_block" | "dirt" | "coarse_dirt" | "podzol" | "mycelium" | "rooted_dirt") || hit.normal != IVec3::Y {
            return false;
        }
        if self.world.block_or_air(hit.block + IVec3::Y) != AIR {
            return false;
        }
        let path = self.reg.id_or_air("dirt_path");
        if path == AIR {
            return false;
        }
        self.world.set_block(hit.block, path);
        self.sfx(Sfx::Step(Material::Dirt));
        self.anim.start_swing();
        self.wear_held(1);
        true
    }

    /// A hoe on coarse or rooted dirt loosens it into plain dirt.
    fn hoe_rough_dirt(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        let name = self.reg.get(self.world.block_or_air(hit.block)).name.clone();
        if !matches!(name.as_str(), "coarse_dirt" | "rooted_dirt") {
            return false;
        }
        let dirt = self.reg.id_or_air("dirt");
        self.world.set_block(hit.block, dirt);
        if name == "rooted_dirt" {
            if let Some(r) = self.items.id("hanging_roots") {
                self.pop_drop((hit.block + hit.normal).as_vec3() + Vec3::splat(0.5), ItemStack::new(r, 1));
            }
        }
        self.sfx(Sfx::Step(Material::Dirt));
        self.anim.start_swing();
        self.wear_held(1);
        true
    }

    fn take_one_held(&mut self) {
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
    }

    /// Put a plant on the ground clicked: `on` says what it may stand on.
    fn plant_on(&mut self, block: BlockId, on: impl Fn(&str) -> bool) -> bool {
        let Some(hit) = self.target else { return false };
        if block == AIR || hit.normal != IVec3::Y {
            return false;
        }
        let below = self.reg.get(self.world.block_or_air(hit.block)).name.clone();
        let at = hit.block + IVec3::Y;
        if !on(&below) || self.world.block_or_air(at) != AIR {
            return false;
        }
        self.world.set_block(at, block);
        self.take_one_held();
        self.anim.start_swing();
        self.sfx(Sfx::Place(Material::Grass));
        true
    }

    fn plant_berries(&mut self) -> bool {
        let b = self.extras.berries[0];
        self.plant_on(b, |n| matches!(n, "grass_block" | "dirt" | "podzol" | "coarse_dirt" | "rooted_dirt" | "farmland" | "snowy_grass_block"))
    }

    fn plant_bamboo(&mut self) -> bool {
        let b = self.extras.bamboo;
        self.plant_on(b, |n| matches!(n, "grass_block" | "dirt" | "podzol" | "coarse_dirt" | "sand" | "red_sand" | "gravel" | "mud" | "bamboo" | "rooted_dirt"))
    }

    /// Kelp goes into water, on the bottom or on more kelp.
    fn plant_kelp(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        let at = hit.block + hit.normal;
        let here = self.world.block_or_air(at);
        if self.world.fluid(here) != 1 || self.extras.kelp == AIR {
            return false;
        }
        let below = self.world.block_or_air(at - IVec3::Y);
        if !(below == self.extras.kelp || self.reg.get(below).solid && self.reg.is_opaque(below)) {
            return false;
        }
        self.world.set_block(at, self.extras.kelp);
        self.take_one_held();
        self.anim.start_swing();
        self.sfx(Sfx::Splash(0.2));
        true
    }

    fn eat_cake(&mut self, p: IVec3) -> bool {
        if !self.survival || self.surv.hunger >= 20.0 {
            return false;
        }
        let b = self.reg.get(self.world.block_or_air(p));
        if b.base != self.extras.cake {
            return false;
        }
        let bites = b.variant + 1;
        if bites > 6 {
            self.world.set_block(p, AIR);
        } else {
            let id = self.variant_id(self.extras.cake, bites);
            self.world.set_block(p, id);
        }
        self.surv.hunger = (self.surv.hunger + 2.0).min(20.0);
        self.surv.saturation = (self.surv.saturation + 0.4).min(self.surv.hunger);
        self.sfx(Sfx::Eat);
        self.anim.start_swing();
        true
    }

    // ---------------------------------------------------------------------------------------
    // Snowballs and eggs.

    fn throw_item(&mut self, name: &str) -> bool {
        let Some(id) = self.items.id(name) else { return false };
        let f = self.forward();
        self.extras.thrown.push(Thrown { pos: self.body.eye() + f * 0.4, vel: f * 22.0 + self.body.vel * 0.5, item: id, life: 10.0 });
        self.take_one_held();
        self.anim.start_swing();
        self.sfx(Sfx::Place(Material::Snow));
        true
    }

    /// Something thrown by a creature (snow golems' snowballs).
    pub(crate) fn throw_from(&mut self, item: u16, from: Vec3, vel: Vec3) {
        self.extras.thrown.push(Thrown { pos: from, vel, item, life: 6.0 });
    }

    fn update_thrown(&mut self, dt: f32) {
        let mut thrown = std::mem::take(&mut self.extras.thrown);
        let glow = self.glow_layer();
        let mut hits = Vec::new();
        thrown.retain_mut(|t| {
            t.life -= dt;
            if t.life <= 0.0 {
                return false;
            }
            t.vel.y -= 20.0 * dt;
            let step = t.vel * dt;
            let n = (step.length() / 0.25).ceil().max(1.0) as usize;
            for _ in 0..n {
                t.pos += step / n as f32;
                if let Some(i) = self.ne_mob_at(t.pos, false) {
                    hits.push((t.item, t.pos, Some(i), t.vel));
                    return false;
                }
                if self.world.solid(t.pos.floor().as_ivec3()) {
                    hits.push((t.item, t.pos - step / n as f32, None, t.vel));
                    return false;
                }
            }
            let snow = self.items.get(t.item).name == "snowball";
            let tint = if snow { [0.95, 0.97, 1.0] } else { [0.85, 0.72, 0.55] };
            self.particles.glow(t.pos, Vec3::ZERO, glow, tint, 0.12, 0.04);
            true
        });
        self.extras.thrown = thrown;
        for (item, at, mob, vel) in hits {
            self.thrown_hits(item, at, mob, vel);
        }
    }

    /// A snowball or egg lands on a creature: a little push (snowballs hurt blazes), and now and
    /// then a chick hatches from an egg.
    pub(crate) fn thrown_hits(&mut self, item: u16, at: Vec3, mob: Option<usize>, vel: Vec3) {
        let name = self.items.get(item).name.clone();
        if let Some(k) = mob {
            let mname = self.mobs.reg.defs[self.mobs.list[k].def].name.clone();
            let eye = self.body.eye();
            let m = &mut self.mobs.list[k];
            let dmg = if name == "snowball" && matches!(mname.as_str(), "blaze" | "fire_imp" | "ember_wraith") { 3.0 } else { 0.01 };
            m.hurt(dmg, eye);
            m.body.knockback(vel, 1.5);
            self.sfx(Sfx::MobHit);
        }
        let (layer, tint) = match name.as_str() {
            "snowball" => self.block_look(self.reg.id_or_air("snow")),
            _ => self.block_look(self.reg.id_or_air("white_wool")),
        };
        self.particles.block_bits(at.floor().as_ivec3(), layer, tint, 8, 0.6);
        if name == "egg" && self.surv.rng.f32() < 0.125 {
            let n = if self.surv.rng.f32() < 1.0 / 32.0 { 4 } else { 1 };
            for _ in 0..n {
                self.mobs.spawn("chicken", at + Vec3::Y * 0.2);
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Jukebox and note blocks.

    fn jukebox_use(&mut self, p: IVec3) -> bool {
        let inside = match self.surv.containers.get(&p) {
            Some(Container::Chest(inv)) => inv.slots.first().copied().flatten(),
            _ => None,
        };
        if let Some(disc) = inside {
            // Out pops the record.
            self.surv.containers.remove(&p);
            self.pop_drop(p.as_vec3() + Vec3::new(0.5, 1.2, 0.5), disc);
            if self.extras.playing == Some(p) {
                self.extras.playing = None;
                if let Some(a) = self.audio.as_mut() {
                    a.stop_disc();
                }
            }
            return true;
        }
        let Some(st) = self.held_stack() else { return false };
        let name = self.items.get(st.item).name.clone();
        let Some(&(_, track)) = DISCS.iter().find(|(d, _)| *d == name) else { return false };
        let mut inv = Inventory::new(1);
        inv.slots[0] = Some(ItemStack::new(st.item, 1));
        self.surv.containers.insert(p, Container::Chest(inv));
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        let ok = self.audio.as_mut().is_some_and(|a| a.play_disc(track));
        if ok {
            self.extras.playing = Some(p);
            let title = self.items.get(st.item).display_name.trim_start_matches("Disco: ").to_string();
            self.toast(format!("Tocando agora: {title}"), 3.0);
        }
        self.prog.event(crate::progression::Event::Token("jukebox".into()));
        self.anim.start_swing();
        true
    }

    fn update_jukebox(&mut self) {
        let Some(p) = self.extras.playing else { return };
        let still_there = self.reg.get(self.world.block_or_air(p)).name == "jukebox" && self.surv.containers.contains_key(&p);
        let Some(a) = self.audio.as_mut() else { return };
        if !still_there || !a.disc_playing() {
            a.stop_disc();
            self.extras.playing = None;
            return;
        }
        let d = (p.as_vec3() + Vec3::splat(0.5)).distance(self.body.eye());
        let k = (1.0 - d / JUKEBOX_RANGE).clamp(0.0, 1.0);
        a.set_disc_volume(self.settings.music_volume.max(0.2) * k * k);
        // A few drifting notes over the box.
        if self.surv.rng.f32() < 0.02 {
            let glow = self.glow_layer();
            self.particles.glow(p.as_vec3() + Vec3::new(0.5, 1.2, 0.5), Vec3::Y * 0.6, glow, [0.5, 1.0, 0.6], 0.06, 1.5);
        }
    }

    pub(crate) fn play_note_block(&mut self, p: IVec3) {
        if self.world.block_or_air(p + IVec3::Y) != AIR {
            return;
        }
        let pitch = self.extras.notes.get(&p.to_array()).copied().unwrap_or(0);
        let below = self.reg.get(self.world.block_or_air(p - IVec3::Y)).name.clone();
        let inst = instrument(&below);
        let d = (p.as_vec3() - self.body.pos).length();
        if d < 32.0 {
            let vol = self.settings.sound_volume * (1.0 - d / 32.0);
            if let Some(a) = self.audio.as_mut() {
                a.play(Sfx::Note(pitch, inst), vol);
            }
        }
        let glow = self.glow_layer();
        let hue = pitch as f32 / 24.0;
        let c = [0.4 + hue, 1.2 - hue * 0.8, 0.5 + (1.0 - hue) * 0.5];
        self.particles.glow(p.as_vec3() + Vec3::new(0.5, 1.2, 0.5), Vec3::Y * 0.8, glow, c, 0.07, 1.2);
    }

    // ---------------------------------------------------------------------------------------
    // Fire.

    /// Flint and steel on a block that is not a portal frame or TNT: a fire on top.
    pub(crate) fn light_fire(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        let at = hit.block + hit.normal;
        if self.world.block_or_air(at) != AIR || self.extras.fire == AIR {
            return false;
        }
        let under = self.reg.get(self.world.block_or_air(at - IVec3::Y)).name.clone();
        let soul = matches!(under.as_str(), "soul_sand" | "soul_soil");
        let id = if soul { self.extras.soul_fire } else { self.extras.fire };
        self.world.set_block(at, id);
        self.extras.fires.push((at, 0.0));
        self.sfx(Sfx::Fizz);
        self.anim.start_swing();
        self.wear_held(1);
        true
    }

    /// Light a fire somewhere (lightning, fire charges).
    #[allow(dead_code)]
    pub(crate) fn set_fire(&mut self, at: IVec3) {
        if self.world.block_or_air(at) == AIR && self.extras.fire != AIR {
            self.world.set_block(at, self.extras.fire);
            self.extras.fires.push((at, 0.0));
        }
    }

    fn update_fire(&mut self, dt: f32) {
        // Standing in fire sets you burning; water puts it out.
        let feet = self.body.pos.floor().as_ivec3();
        let touching = [feet, feet + IVec3::Y].iter().any(|&p| self.extras.is_fire(self.world.block_or_air(p)));
        if touching {
            self.extras.burning = self.extras.burning.max(8.0);
        }
        if self.body.in_water || (self.weather.rain > 0.3 && self.world.surface_y(feet.x, feet.z).is_some_and(|y| y < feet.y)) {
            self.extras.burning = 0.0;
        }
        if self.extras.burning > 0.0 {
            self.extras.burning -= dt;
            self.extras.burn_tick -= dt;
            if self.extras.burn_tick <= 0.0 {
                self.extras.burn_tick = 1.0;
                if self.survival && !self.has_effect(crate::effects::FIRE_RESISTANCE) {
                    self.prog.next_kind = crate::progression::DamageKind::Fire;
                    self.damage(if touching { 1.0 } else { 1.0 }, None);
                }
            }
        }
        // Mobs in fire catch fire.
        if !self.extras.fires.is_empty() {
            for m in self.mobs.list.iter_mut() {
                let p = m.body.pos.floor().as_ivec3();
                if self.extras.fires.iter().any(|(f, _)| *f == p || *f == p + IVec3::Y) {
                    m.on_fire = m.on_fire.max(8.0);
                }
            }
        }
        // Fires burn, spread and go out about once a second each.
        self.extras.fire_t += dt;
        if self.extras.fire_t < 1.0 {
            return;
        }
        self.extras.fire_t = 0.0;
        let rain = self.weather.rain > 0.3;
        let fires = std::mem::take(&mut self.extras.fires);
        let mut keep = Vec::new();
        let mut new = Vec::new();
        for (p, age) in fires {
            let id = self.world.block_or_air(p);
            if !self.extras.is_fire(id) {
                continue;
            }
            let age = age + 1.0;
            let under = self.reg.get(self.world.block_or_air(p - IVec3::Y)).name.clone();
            let eternal = matches!(under.as_str(), "netherrack" | "magma_block" | "soul_sand" | "soul_soil") ;
            let open_sky = self.world.surface_y(p.x, p.z).is_some_and(|y| y < p.y);
            if (rain && open_sky) || self.world.block_or_air(p - IVec3::Y) == AIR {
                self.world.set_block(p, AIR);
                continue;
            }
            if eternal {
                keep.push((p, 0.0));
                continue;
            }
            // What around it can burn.
            let mut fuel = Vec::new();
            for d in [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z] {
                let q = p + d;
                let b = self.world.block_or_air(q);
                if b != AIR && self.extras.flammable[b as usize] {
                    fuel.push(q);
                }
            }
            if fuel.is_empty() && age > 3.0 + self.surv.rng.f32() * 4.0 || age > 30.0 {
                self.world.set_block(p, AIR);
                continue;
            }
            for q in fuel {
                if self.surv.rng.f32() < 0.12 {
                    // The block burns away; the fire takes its place (TNT goes off).
                    let b = self.world.block_or_air(q);
                    if self.reg.get(b).name == "tnt" {
                        self.world.set_block(q, AIR);
                        self.prime_tnt(q, 4.0);
                    } else {
                        self.world.set_block(q, self.extras.fire);
                        new.push((q, 0.0));
                    }
                } else if self.surv.rng.f32() < 0.08 {
                    // Or the flames climb onto it from a free side.
                    for d in [IVec3::Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                        let r = q + d;
                        if self.world.block_or_air(r) == AIR && self.reg.get(self.world.block_or_air(r - IVec3::Y)).solid && new.len() < 64 {
                            self.world.set_block(r, self.extras.fire);
                            new.push((r, 0.0));
                            break;
                        }
                    }
                }
            }
            keep.push((p, age));
        }
        keep.extend(new);
        keep.truncate(512);
        self.extras.fires = keep;
        if !self.extras.fires.is_empty() {
            let near = self.extras.fires.iter().any(|(p, _)| p.as_vec3().distance(self.body.pos) < 12.0);
            if near && self.surv.rng.f32() < 0.5 {
                self.sfx(Sfx::Fizz);
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Plants, sponges and what breaking a block does to its neighbours.

    /// A random tick on kelp, bamboo or a berry bush: it grows.
    pub(crate) fn extras_tick(&mut self, p: IVec3, id: BlockId, r: u32) {
        let e = &self.extras;
        if id == e.kelp || id == e.bamboo {
            // Columns grow at the top, kelp while there is water above, bamboo up to 12 tall.
            let above = p + IVec3::Y;
            let up = self.world.block_or_air(above);
            let ok = if id == e.kelp { self.world.fluid(up) == 1 && self.world.fluid(self.world.block_or_air(above + IVec3::Y)) == 1 } else { up == AIR };
            if !ok || r % 4 != 0 {
                return;
            }
            let mut h = 1;
            while h < 26 && self.world.block_or_air(p - IVec3::Y * h) == id {
                h += 1;
            }
            if h < if id == e.kelp { 25 } else { 12 } {
                self.world.set_block(above, id);
            }
        } else if let Some(k) = e.berries[..3].iter().position(|&b| b == id) {
            if r % 5 == 0 {
                let next = e.berries[k + 1];
                self.world.set_block(p, next);
            }
        }
    }

    /// After any block is broken (in every mode): plants left floating come down with it, kelp
    /// leaves water, a sponge set down soaks up the water around it.
    pub(crate) fn after_break(&mut self, p: IVec3, was: BlockId) {
        self.plugins_block(p, was, false);
        let name = self.reg.get(was).name.clone();
        self.infested_broken(p, &name);
        self.pale_broken(p, was);
        if self.world.is_wet(was) && self.extras.water != AIR && self.world.block_or_air(p) == AIR {
            self.world.set_block(p, self.extras.water);
        }
        let mut q = p + IVec3::Y;
        for _ in 0..32 {
            let id = self.world.block_or_air(q);
            if id == AIR {
                break;
            }
            let b = self.reg.get(id);
            let column = id == self.extras.kelp || id == self.extras.bamboo || id == self.farm.cane;
            let plant = !b.solid && matches!(b.shape, voxel::block::Shape::Cross) && !self.world.is_liquid(id);
            if !(plant || column) {
                break;
            }
            let wet = self.world.is_wet(id);
            self.world.set_block(q, if wet { self.extras.water } else { AIR });
            if self.survival {
                let drops = self.items.drops(&self.reg, id, None, &mut self.surv.rng);
                for d in drops {
                    self.pop_drop(q.as_vec3() + Vec3::splat(0.5), d);
                }
            }
            if !column {
                break;
            }
            q += IVec3::Y;
        }
    }

    /// A block was just placed: a sponge drinks the water around it.
    pub(crate) fn extras_after_place(&mut self, p: IVec3, id: BlockId) {
        self.try_build_snow_golem(p, id);
        if id == self.extras.sponge && id != AIR {
            let mut todo = vec![(p, 0)];
            let mut seen = HashSet::new();
            let mut n = 0;
            while let Some((q, d)) = todo.pop() {
                for dir in [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z] {
                    let r = q + dir;
                    if d >= 6 || n >= 65 || !seen.insert(r) {
                        continue;
                    }
                    let b = self.world.block_or_air(r);
                    if self.world.fluid(b) == 1 || self.world.is_wet(b) {
                        self.world.set_block(r, AIR);
                        n += 1;
                        todo.push((r, d + 1));
                    }
                }
            }
            if n > 0 {
                self.world.set_block(p, self.extras.wet_sponge);
                self.sfx(Sfx::Splash(0.3));
            }
        }
        if id == self.extras.wet_sponge && self.dim == voxel::worldgen::Dim::Nether {
            // The Nether dries it at once.
            self.world.set_block(p, self.extras.sponge);
            self.sfx(Sfx::Fizz);
        }
    }

    /// Shears keep leaves, vines, cobwebs and grass whole; they wear on each.
    pub(crate) fn extras_drops(&mut self, id: BlockId, mut drops: Vec<ItemStack>) -> Vec<ItemStack> {
        let n = self.reg.get(self.reg.get(id).base).name.clone();
        // Now and then a potato comes up green, and jungle leaves hide cocoa beans.
        let bonus = if n.starts_with("potatoes") && !drops.is_empty() && self.surv.rng.f32() < 0.02 {
            "poisonous_potato"
        } else if n == "jungle_leaves" && self.surv.rng.f32() < 0.03 {
            "cocoa_beans"
        } else {
            ""
        };
        if let Some(i) = self.items.id(bonus) {
            drops.push(ItemStack::new(i, 1));
        }
        if self.held_key() != "shears" {
            return drops;
        }
        let keep = n.ends_with("_leaves") || matches!(n.as_str(), "vine" | "cobweb" | "short_grass" | "tall_grass" | "fern" | "large_fern" | "dead_bush" | "seagrass" | "hanging_roots" | "glow_lichen");
        if n.ends_with("_wool") || keep {
            self.wear_held(1);
        }
        if !keep {
            return drops;
        }
        match self.items.of_block(self.reg.get(id).base) {
            Some(i) => vec![ItemStack::new(i, 1)],
            None => drops,
        }
    }

    /// Shears cut leaves, cobwebs and wool fast.
    pub(crate) fn shears_speed(&self, id: BlockId) -> f32 {
        if self.held_key() != "shears" {
            return 1.0;
        }
        let n = &self.reg.get(self.reg.get(id).base).name;
        if n.ends_with("_leaves") || n == "cobweb" {
            15.0
        } else if n.ends_with("_wool") {
            5.0
        } else if n == "vine" || n == "glow_lichen" {
            2.0
        } else {
            1.0
        }
    }

    // ---------------------------------------------------------------------------------------
    // Animals: eggs, wool growing back, pigs on a stick.

    fn update_animals_extras(&mut self, dt: f32) {
        self.extras.egg_t += dt;
        self.extras.regrow_t += dt;
        if self.extras.egg_t >= 1.0 {
            self.extras.egg_t = 0.0;
            // A hen lays every five to ten minutes.
            let egg = self.items.id("egg");
            let mut lay = Vec::new();
            for m in &self.mobs.list {
                if m.alive() && self.mobs.reg.defs[m.def].name == "chicken" && self.surv.rng.f32() < 1.0 / 450.0 {
                    lay.push(m.body.pos + Vec3::Y * 0.3);
                }
            }
            if let Some(e) = egg {
                for at in lay {
                    self.spawn_drop(at, ItemStack::new(e, 1), Vec3::Y * 2.0);
                }
            }
        }
        if self.extras.regrow_t >= 2.0 {
            self.extras.regrow_t = 0.0;
            // Sheared sheep eat grass and their wool grows back.
            let (Some(sheep), Some(bare)) = (self.mobs.reg.id("sheep"), self.mobs.reg.id("sheep_sheared")) else { return };
            let grass = self.reg.id_or_air("grass_block");
            let dirt = self.reg.id_or_air("dirt");
            for k in 0..self.mobs.list.len() {
                if self.mobs.list[k].def != bare || self.surv.rng.f32() > 1.0 / 40.0 {
                    continue;
                }
                let below = self.mobs.list[k].body.pos.floor().as_ivec3() - IVec3::Y;
                if self.world.block_or_air(below) == grass {
                    self.world.set_block(below, dirt);
                    self.mobs.list[k].def = sheep;
                }
            }
        }
        self.extras.sheared.clear();
    }

    /// Riding a pig: it only goes where a carrot on a stick points it.
    pub(crate) fn pig_steer(&mut self, wish: Vec3) -> Vec3 {
        let Some(i) = self.mobs.list.iter().position(|m| m.ridden) else { return wish };
        if self.mobs.reg.defs[self.mobs.list[i].def].name != "pig" {
            return wish;
        }
        if self.held_key() != "carrot_on_a_stick" {
            return Vec3::ZERO;
        }
        if wish.length_squared() > 0.01 && self.surv.rng.f32() < 0.002 {
            self.wear_held(1);
        }
        let f = self.forward();
        Vec3::new(f.x, 0.0, f.z).normalize_or_zero()
    }

    // ---------------------------------------------------------------------------------------
    // Armor stands.

    fn place_armor_stand(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        if hit.normal != IVec3::Y {
            return false;
        }
        let at = (hit.block + IVec3::Y).as_vec3() + Vec3::new(0.5, 0.0, 0.5);
        if !self.mobs.spawn("armor_stand", at) {
            return false;
        }
        let f = self.forward();
        if let Some(m) = self.mobs.list.last_mut() {
            m.yaw = (-f.x).atan2(-f.z);
        }
        self.take_one_held();
        self.sfx(Sfx::Place(Material::Wood));
        self.anim.start_swing();
        true
    }

    // ---------------------------------------------------------------------------------------
    // Eating and loot.

    /// After eating: stews give their bowl back, some foods have a catch.
    pub(crate) fn extras_ate(&mut self, name: &str) {
        use crate::effects::*;
        match name {
            "mushroom_stew" | "beetroot_soup" | "rabbit_stew" | "suspicious_stew" => {
                if let Some(b) = self.items.id("bowl") {
                    let sel = self.selected;
                    if self.surv.inv.slots[sel].is_none() {
                        self.surv.inv.slots[sel] = Some(ItemStack::new(b, 1));
                    } else if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(b, 1)) {
                        self.throw_stack(left);
                    }
                }
                if name == "suspicious_stew" {
                    // Which flower went in is a surprise.
                    let pick = [(REGENERATION, 8.0), (NIGHT_VISION, 5.0), (FIRE_RESISTANCE, 4.0), (JUMP_BOOST, 6.0), (SATURATION, 0.35), (WEAKNESS, 9.0), (BLINDNESS, 8.0), (POISON, 12.0)];
                    let (e, t) = pick[self.surv.rng.below(pick.len() as u32) as usize];
                    self.add_effect(e, 0, t, false);
                }
            }
            "poisonous_potato" if self.surv.rng.f32() < 0.6 => self.add_effect(POISON, 0, 5.0, false),
            _ => {}
        }
    }

    /// Extra drops of creatures from this file: records from stalkers, the totem from the
    /// invoker, eggs and feathers.
    pub(crate) fn extras_mob_drop(&mut self, name: &str, pos: Vec3) {
        let give = |g: &mut Game, item: &str, n: u8| {
            if let Some(i) = g.items.id(item) {
                g.pop_drop(pos + Vec3::Y * 0.5, ItemStack::new(i, n));
            }
        };
        match name {
            "invoker" => give(self, "totem_of_undying", 1),
            "stalker" if self.surv.rng.f32() < 0.05 => {
                let d = DISCS[self.surv.rng.below(DISCS.len() as u32) as usize].0;
                give(self, d, 1);
            }
            "sheep_sheared" => {}
            _ => {}
        }
    }

    /// Chests the world placed: now and then a record, chainmail or an iron nugget pile.
    pub(crate) fn extras_chest_loot(&mut self, inv: &mut Inventory) {
        let roll = self.surv.rng.f32();
        let pick: Option<(&str, u8)> = if roll < 0.06 {
            Some((DISCS[self.surv.rng.below(DISCS.len() as u32) as usize].0, 1))
        } else if roll < 0.12 {
            let c = ["chainmail_helmet", "chainmail_chestplate", "chainmail_leggings", "chainmail_boots"];
            Some((c[self.surv.rng.below(4) as usize], 1))
        } else if roll < 0.25 {
            Some(("iron_nugget", 2 + self.surv.rng.below(8) as u8))
        } else if roll < 0.3 {
            Some(("bowl", 1))
        } else {
            None
        };
        if let Some((n, c)) = pick {
            if let Some(i) = self.items.id(n) {
                let _ = inv.add(&self.items, ItemStack::new(i, c));
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Books.

    fn open_book(&mut self) -> bool {
        let Some(st) = self.held_stack() else { return false };
        let write = self.items.get(st.item).name == "writable_book";
        let mut id = st.damage;
        if id == 0 || id as usize > self.extras.books.len() {
            if !write {
                return false;
            }
            // A fresh book gets its own page in the world's library.
            self.extras.books.push(Book { title: String::new(), pages: vec![String::new()], signed: false });
            id = self.extras.books.len() as u16;
            let sel = self.selected;
            if let Some(s) = &mut self.surv.inv.slots[sel] {
                s.damage = id;
            }
        }
        self.extras.book = Some(BookOpen { id, page: 0, write });
        self.set_screen(crate::Screen::Sign);
        true
    }

    fn book_mut(&mut self) -> Option<(&mut Book, &mut BookOpen)> {
        let open = self.extras.book.as_mut()?;
        let b = self.extras.books.get_mut(open.id as usize - 1)?;
        Some((b, open))
    }

    pub(crate) fn book_key(&mut self, code: winit::keyboard::KeyCode) {
        use winit::keyboard::KeyCode;
        let ctrl = self.keys.contains(&KeyCode::ControlLeft) || self.keys.contains(&KeyCode::ControlRight);
        let Some((b, open)) = self.book_mut() else { return };
        match code {
            KeyCode::Escape => {
                self.extras.book = None;
                self.set_screen(crate::Screen::Playing);
            }
            KeyCode::PageDown | KeyCode::ArrowRight => {
                if open.page + 1 < b.pages.len() {
                    open.page += 1;
                } else if open.write && b.pages.len() < BOOK_PAGES {
                    b.pages.push(String::new());
                    open.page += 1;
                }
            }
            KeyCode::PageUp | KeyCode::ArrowLeft => open.page = open.page.saturating_sub(1),
            _ if !open.write => {}
            KeyCode::Backspace => {
                b.pages[open.page].pop();
            }
            KeyCode::Enter | KeyCode::NumpadEnter if ctrl => {
                // Sign it: the first line is the title, and the book is closed for writing.
                let title = b.pages.first().and_then(|p| p.lines().next()).unwrap_or("").trim().to_string();
                b.title = if title.is_empty() { "Sem título".into() } else { title };
                b.signed = true;
                self.extras.book = None;
                if let Some(w) = self.items.id("written_book") {
                    let sel = self.selected;
                    if let Some(s) = &mut self.surv.inv.slots[sel] {
                        s.item = w;
                    }
                }
                self.toast("Livro assinado", 2.0);
                self.set_screen(crate::Screen::Playing);
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let page = &mut b.pages[open.page];
                if page.split('\n').count() < BOOK_LINES {
                    page.push('\n');
                }
            }
            _ => {}
        }
    }

    pub(crate) fn book_type(&mut self, text: &str) {
        let Some((b, open)) = self.book_mut() else { return };
        if !open.write {
            return;
        }
        let page = &mut b.pages[open.page];
        for c in text.chars() {
            if c.is_control() {
                continue;
            }
            let line_len = page.rsplit('\n').next().unwrap_or("").chars().count();
            if line_len >= BOOK_LINE_LEN {
                if page.split('\n').count() >= BOOK_LINES {
                    break;
                }
                page.push('\n');
            }
            page.push(c);
        }
    }

    pub(crate) fn draw_book(&mut self, w: f32, h: f32) {
        let Some(open) = self.extras.book.as_ref() else { return };
        let Some(b) = self.extras.books.get(open.id as usize - 1).cloned() else { return };
        let (page, write) = (open.page, open.write);
        let s = self.ui.scale;
        let (bw, bh) = (150.0 * s, 180.0 * s);
        let (x, y) = ((w - bw) / 2.0, (h - bh) / 2.0 - 10.0 * s);
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 120]);
        self.ui.rect(x - 3.0 * s, y - 3.0 * s, bw + 6.0 * s, bh + 6.0 * s, [92, 58, 30, 255]);
        self.ui.rect(x, y, bw, bh, [236, 226, 196, 255]);
        let ink = [40, 30, 20, 255];
        let head = if b.signed { format!("{} · página {} de {}", b.title, page + 1, b.pages.len()) } else { format!("Página {} de {}", page + 1, b.pages.len()) };
        self.ui.text_centered(w / 2.0, y + 5.0 * s, &head, [110, 80, 50, 255]);
        let text = b.pages.get(page).cloned().unwrap_or_default();
        let blink = write && (self.now * 2.0) as i64 % 2 == 0;
        let lines: Vec<&str> = text.split('\n').collect();
        for (i, l) in lines.iter().enumerate() {
            let shown = if i + 1 == lines.len() && blink { format!("{l}_") } else { l.to_string() };
            self.ui.text(x + 8.0 * s, y + 20.0 * s + i as f32 * 12.5 * s, &shown, ink);
        }
        let help = if write { "Digite. Setas: páginas. Ctrl+Enter: assinar. Esc: fecha" } else { "Setas: páginas. Esc: fecha" };
        self.ui.text_centered(w / 2.0, y + bh + 10.0 * s, help, [255, 255, 255, 255]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruments_follow_the_block_below() {
        assert_eq!(instrument("stone"), 1);
        assert_eq!(instrument("oak_planks"), 3);
        assert_eq!(instrument("sand"), 2);
        assert_eq!(instrument("grass_block"), 0);
    }

    #[test]
    fn discs_are_our_tracks() {
        let tracks = include_str!("../../../assets/music/tracks.json");
        for (_, t) in DISCS {
            assert!(tracks.contains(&format!("\"{t}\"")), "{t}");
        }
    }
}
