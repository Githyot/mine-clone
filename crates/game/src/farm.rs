//! Farming: hoes till grass and dirt into farmland, seeds and roots grow there stage by stage
//! on random ticks (faster with water near), and town farmers (Smarter Farmers idea) walk to
//! ripe crops, harvest them, put the harvest in a chest near home and plant again.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::{BlockId, BlockRegistry, AIR};

use crate::audio::{Material, Sfx};
use crate::survival::Container;
use crate::Game;

pub const FARMER: &str = "npc_farmer";

pub struct Farm {
    /// The next growth stage of each block (AIR when it isn't a growing crop).
    next: Vec<BlockId>,
    /// The first stage of the crop each block belongs to (AIR when not a crop).
    first: Vec<BlockId>,
    /// True for the last (ripe) stage.
    ripe: Vec<bool>,
    farmland: BlockId,
    /// The block each crop must stand on (farmland, or soul sand for nether wart).
    soil: Vec<BlockId>,
    /// For a ripe pumpkin or melon stem: the fruit it sets beside itself (else AIR).
    fruit: Vec<BlockId>,
    pub cane: BlockId,
    /// Bee nests and hives that fill with honey (empty ones).
    pub hives: [BlockId; 2],
    /// Composters being filled: how many layers each has (7 makes bone meal).
    compost: std::collections::HashMap<IVec3, u8>,
    t: f32,
}

impl Farm {
    pub fn new(reg: &BlockRegistry) -> Farm {
        let n = reg.len();
        let farmland = reg.id_or_air("farmland");
        let mut f = Farm { next: vec![AIR; n], first: vec![AIR; n], ripe: vec![false; n], farmland, soil: vec![farmland; n], fruit: vec![AIR; n], cane: reg.id_or_air("sugar_cane"), hives: [reg.id_or_air("bee_nest"), reg.id_or_air("beehive")], compost: Default::default(), t: 0.0 };
        let soul = reg.id_or_air("soul_sand");
        for (kind, stages) in [("wheat", 8), ("carrots", 4), ("potatoes", 4), ("beetroots", 4), ("nether_wart", 4), ("pumpkin_stem", 8), ("melon_stem", 8), ("torchflower", 3), ("pitcher", 5)] {
            let ids: Vec<BlockId> = (0..stages).filter_map(|k| reg.id(&format!("{kind}_stage{k}"))).collect();
            if ids.len() != stages {
                continue;
            }
            for (k, &id) in ids.iter().enumerate() {
                f.first[id as usize] = ids[0];
                if kind == "nether_wart" {
                    f.soil[id as usize] = soul;
                }
                if k + 1 < ids.len() {
                    f.next[id as usize] = ids[k + 1];
                } else if let Some(fruit) = kind.strip_suffix("_stem") {
                    // A ripe stem stays and keeps setting fruit; farmers leave it be.
                    f.fruit[id as usize] = reg.id_or_air(fruit);
                } else {
                    f.ripe[id as usize] = true;
                }
            }
        }
        f
    }

    pub fn is_crop(&self, id: BlockId) -> bool {
        self.first.get(id as usize).is_some_and(|&b| b != AIR)
    }

    fn ripe(&self, id: BlockId) -> bool {
        self.ripe.get(id as usize).copied().unwrap_or(false)
    }
}

impl Game {
    /// Right-click with a hoe on grass or dirt with air above: farmland.
    pub(crate) fn till(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        let id = self.world.block_or_air(hit.block);
        let name = self.reg.get(id).name.as_str();
        if !matches!(name, "grass_block" | "dirt" | "snowy_grass_block") || hit.normal != IVec3::Y {
            return false;
        }
        let above = hit.block + IVec3::Y;
        let up = self.world.block_or_air(above);
        if up != AIR && self.reg.get(up).solid {
            return false;
        }
        if up != AIR {
            self.world.set_block(above, AIR);
        }
        self.world.set_block(hit.block, self.farm.farmland);
        self.wear_held(1);
        self.anim.start_swing();
        self.sfx(Sfx::Step(Material::Grass));
        true
    }

    /// Plant a seed or root on the farmland you're looking at.
    pub(crate) fn plant_crop(&mut self, crop: BlockId) -> bool {
        let Some(hit) = self.target else { return false };
        if self.world.block_or_air(hit.block) != self.farm.soil[crop as usize] || hit.normal != IVec3::Y {
            return false;
        }
        let at = hit.block + IVec3::Y;
        if self.world.block_or_air(at) != AIR {
            return false;
        }
        self.world.set_block(at, crop);
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.anim.start_swing();
        self.sfx(Sfx::Place(Material::Grass));
        true
    }

    /// A random tick on a crop: it grows a stage (more often with water close by), or breaks
    /// off if its farmland is gone.
    pub(crate) fn crop_tick(&mut self, p: IVec3, id: BlockId, r: u32) {
        let soil = self.farm.soil[id as usize];
        if self.world.block_or_air(p - IVec3::Y) != soil {
            self.world.set_block(p, AIR);
            return;
        }
        let fruit = self.farm.fruit[id as usize];
        if fruit != AIR {
            self.set_fruit(p, fruit, r);
            return;
        }
        let next = self.farm.next[id as usize];
        if next == AIR {
            return;
        }
        let wet = soil != self.farm.farmland || (-4..=4).any(|dx| (-4..=4).any(|dz| self.world.fluid(self.world.block_or_air(p + IVec3::new(dx, -1, dz))) == 1));
        // About one stage every 2 to 4 minutes.
        if r % if wet { 3 } else { 6 } == 0 {
            self.world.set_block(p, next);
        }
    }

    /// A ripe stem: now and then a pumpkin or melon on a free side, on dirt, grass or
    /// farmland, if it hasn't one already.
    fn set_fruit(&mut self, p: IVec3, fruit: BlockId, r: u32) {
        let sides = [IVec3::X, -IVec3::X, IVec3::Z, -IVec3::Z];
        if sides.iter().any(|&d| self.world.block_or_air(p + d) == fruit) || r % 4 != 0 {
            return;
        }
        let at = p + sides[(r / 4 % 4) as usize];
        let below = self.reg.get(self.world.block_or_air(at - IVec3::Y)).name.as_str();
        if self.world.block_or_air(at) == AIR && matches!(below, "dirt" | "grass_block" | "farmland" | "coarse_dirt" | "rocky_dirt") {
            self.world.set_block(at, fruit);
        }
    }

    /// Can sugar cane go at `p`? On more cane, or on sand, dirt or grass next to water.
    fn cane_fits(&self, p: IVec3) -> bool {
        let below = self.world.block_or_air(p - IVec3::Y);
        if below == self.farm.cane {
            return true;
        }
        let name = self.reg.get(below).name.as_str();
        matches!(name, "sand" | "red_sand" | "dirt" | "grass_block" | "coarse_dirt" | "podzol" | "farmland")
            && [IVec3::X, -IVec3::X, IVec3::Z, -IVec3::Z].iter().any(|&d| self.world.fluid(self.world.block_or_air(p - IVec3::Y + d)) == 1)
    }

    /// Plant sugar cane on the block you look at.
    pub(crate) fn plant_cane(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        let at = hit.block + hit.normal;
        if self.world.block_or_air(at) != AIR || !self.cane_fits(at) {
            return false;
        }
        self.world.set_block(at, self.farm.cane);
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.anim.start_swing();
        self.sfx(Sfx::Place(Material::Grass));
        true
    }

    /// A random tick on sugar cane: it grows up to three tall, or drops off if its ground is
    /// gone.
    pub(crate) fn cane_tick(&mut self, p: IVec3, r: u32) {
        if !self.cane_fits(p) {
            self.world.set_block(p, AIR);
            if let Some(it) = self.items.id("sugar_cane") {
                self.pop_drop(p.as_vec3() + Vec3::splat(0.5), ItemStack::new(it, 1));
            }
            return;
        }
        let tall = (1..3).take_while(|&k| self.world.block_or_air(p - IVec3::Y * k) == self.farm.cane).count();
        if tall < 2 && r % 3 == 0 && self.world.block_or_air(p + IVec3::Y) == AIR {
            self.world.set_block(p + IVec3::Y, self.farm.cane);
        }
    }

    /// Bone meal on what you look at: a crop jumps ahead, a sapling grows soon, grass sprouts
    /// tall grass and flowers around.
    pub(crate) fn use_bone_meal(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        let p = hit.block;
        let id = self.world.block_or_air(p);
        let name = self.reg.get(id).name.clone();
        let mut used = false;
        if self.farm.is_crop(id) {
            let mut cur = id;
            for _ in 0..1 + (self.surv.rng.f32() * 3.0) as u32 {
                let n = self.farm.next[cur as usize];
                if n != AIR {
                    cur = n;
                }
            }
            if cur != id {
                self.world.set_block(p, cur);
                used = true;
            } else if self.farm.fruit[id as usize] != AIR {
                let r = (self.surv.rng.f32() * 64.0) as u32 * 4;
                self.set_fruit(p, self.farm.fruit[id as usize], r);
                used = true;
            }
        } else if voxel::trees::is_sapling(&name) {
            if self.surv.rng.f32() < 0.45 {
                for s in self.surv.saplings.iter_mut().filter(|(q, _)| *q == p) {
                    s.1 = 0.0;
                }
                if !self.surv.saplings.iter().any(|(q, _)| *q == p) {
                    self.surv.saplings.push((p, 0.0));
                }
            }
            used = true;
        } else if name == "grass_block" {
            let grass = self.reg.id_or_air("short_grass");
            let flowers = [self.reg.id_or_air("poppy"), self.reg.id_or_air("dandelion")];
            for _ in 0..24 {
                let (dx, dz) = ((self.surv.rng.f32() * 7.0) as i32 - 3, (self.surv.rng.f32() * 7.0) as i32 - 3);
                for dy in -1..=1 {
                    let g = p + IVec3::new(dx, dy, dz);
                    if self.reg.get(self.world.block_or_air(g)).name == "grass_block" && self.world.block_or_air(g + IVec3::Y) == AIR {
                        let roll = self.surv.rng.f32();
                        let b = if roll < 0.12 { flowers[(roll * 20.0) as usize % 2] } else { grass };
                        if b != AIR {
                            self.world.set_block(g + IVec3::Y, b);
                        }
                        break;
                    }
                }
            }
            used = true;
        }
        if !used {
            return false;
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        let glow = self.glow_layer();
        for k in 0..8 {
            let a = k as f32 * 0.8;
            self.particles.glow(p.as_vec3() + Vec3::new(0.5 + a.cos() * 0.4, 1.1, 0.5 + a.sin() * 0.4), Vec3::Y * 0.6, glow, [0.6, 1.0, 0.4], 0.05, 1.0);
        }
        self.anim.start_swing();
        true
    }

    /// Right click on a composter: plant matter fills it a layer at a time (some of it
    /// rots away), and a full one gives bone meal.
    pub(crate) fn use_composter(&mut self, p: IVec3, ready: bool) -> bool {
        if ready {
            self.world.set_block(p, self.reg.id_or_air("composter"));
            self.farm.compost.remove(&p);
            if let Some(it) = self.items.id("bone_meal") {
                self.pop_drop(p.as_vec3() + Vec3::new(0.5, 1.1, 0.5), ItemStack::new(it, 1));
            }
            self.sfx(Sfx::Break(Material::Grass));
            return true;
        }
        let Some(stack) = self.held_stack() else { return false };
        let name = self.items.get(stack.item).name.clone();
        let chance = if name.ends_with("_seeds") || name.ends_with("_leaves") || name == "short_grass" || name == "sugar_cane" || name == "kelp" {
            0.3
        } else if voxel::trees::is_sapling(&name) || name == "melon_slice" || name == "cactus" || name == "vine" || name == "glow_berries" {
            0.5
        } else if matches!(name.as_str(), "wheat" | "carrot" | "potato" | "beetroot" | "apple" | "melon" | "pumpkin" | "nether_wart" | "poppy" | "dandelion" | "hay_block" | "bread" | "baked_potato") {
            0.65
        } else {
            return false;
        };
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.anim.start_swing();
        self.sfx(Sfx::Place(Material::Grass));
        if self.surv.rng.f32() < chance {
            let n = self.farm.compost.entry(p).or_default();
            *n += 1;
            if *n >= 7 {
                self.world.set_block(p, self.reg.id_or_air("composter_ready"));
            }
        }
        true
    }

    /// Farmers: walk to the nearest ripe crop near home, harvest it, plant it again and put
    /// what they picked in a chest near home (or leave it on the ground).
    pub(crate) fn update_farmers(&mut self, dt: f32) {
        self.farm.t -= dt;
        if self.farm.t > 0.0 {
            return;
        }
        self.farm.t = 1.0;
        let farmers: Vec<usize> =
            self.mobs.list.iter().enumerate().filter(|(_, m)| m.alive() && self.mobs.reg.defs[m.def].name == FARMER).map(|(i, _)| i).collect();
        for i in farmers {
            let (pos, home) = {
                let m = &self.mobs.list[i];
                (m.body.pos, m.home.unwrap_or(m.body.pos))
            };
            let Some(crop) = self.nearest_ripe(home, 10) else { continue };
            let spot = crop.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
            if Vec3::new(spot.x - pos.x, 0.0, spot.z - pos.z).length() > 1.6 {
                self.mobs.list[i].go_to(spot, 2.0);
                continue;
            }
            self.farmer_harvest(crop, home);
            self.mobs.list[i].go_to(spot, 1.0);
        }
    }

    fn nearest_ripe(&self, c: Vec3, r: i32) -> Option<IVec3> {
        let c = c.floor().as_ivec3();
        let mut best: Option<(i32, IVec3)> = None;
        for dy in -3..=3 {
            for dz in -r..=r {
                for dx in -r..=r {
                    let p = c + IVec3::new(dx, dy, dz);
                    if self.farm.ripe(self.world.block_or_air(p)) {
                        let d = dx * dx + dz * dz + dy * dy;
                        if best.is_none_or(|(b, _)| d < b) {
                            best = Some((d, p));
                        }
                    }
                }
            }
        }
        best.map(|(_, p)| p)
    }

    fn farmer_harvest(&mut self, p: IVec3, home: Vec3) {
        let id = self.world.block_or_air(p);
        let drops = self.items.drops(&self.reg, id, None, &mut self.surv.rng);
        let first = self.farm.first[id as usize];
        self.world.set_block(p, first);
        // The seed for the new plant comes out of the harvest.
        let mut drops: Vec<ItemStack> = drops;
        if let Some(seed) = self.items.items.iter().position(|it| it.places == Some(first)).map(|i| i as voxel::item::ItemId) {
            if let Some(d) = drops.iter_mut().find(|d| d.item == seed && d.count > 0) {
                d.count -= 1;
            }
        }
        drops.retain(|d| d.count > 0);
        let (layer, tint) = self.block_look(id);
        self.particles.block_bits(p, layer, tint, 6, 0.6);
        if self.body.pos.distance(p.as_vec3()) < 16.0 {
            self.sfx(Sfx::Break(Material::Grass));
        }
        // Into the nearest chest within 8 blocks of home.
        let h = home.floor().as_ivec3();
        let chest = self.surv.containers.iter().filter(|(c, k)| matches!(k, Container::Chest(_)) && (**c - h).abs().max_element() <= 8).map(|(c, _)| *c).min_by_key(|c| (*c - h).length_squared());
        for d in drops {
            let left = match chest.and_then(|c| self.surv.containers.get_mut(&c)) {
                Some(Container::Chest(inv)) => inv.add(&self.items, d),
                _ => Some(d),
            };
            if let Some(l) = left {
                self.pop_drop(p.as_vec3() + Vec3::splat(0.5), l);
            }
        }
    }

    /// Test scene: a small wheat and carrot field by a pond, some of it ripe.
    pub(crate) fn test_farm(&mut self) {
        let f = self.forward();
        let c = (self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 6.0).floor().as_ivec3();
        let y = c.y - 1;
        let water = self.reg.id_or_air("water");
        let names = ["wheat_stage7", "wheat_stage4", "carrots_stage3", "potatoes_stage2", "beetroots_stage3"];
        for dz in -2..=2 {
            for dx in -3..=3 {
                let p = IVec3::new(c.x + dx, y, c.z + dz);
                for k in 1..6 {
                    self.world.set_block(p + IVec3::Y * k, AIR);
                }
                if dx == 0 && dz == 0 {
                    self.world.set_block(p, water);
                    continue;
                }
                self.world.set_block(p, self.farm.farmland);
                let crop = self.reg.id_or_air(names[((dx + 3) as usize + dz.unsigned_abs() as usize) % names.len()]);
                self.world.set_block(p + IVec3::Y, crop);
            }
        }
        // Stems with their fruit, cane by the water, a composter.
        let cane = self.farm.cane;
        for k in 1..3 {
            self.world.set_block(IVec3::new(c.x + 1, y + k, c.z), cane);
        }
        self.world.set_block(IVec3::new(c.x - 1, y + 1, c.z), cane);
        for (dz, stem, fruit) in [(-1, "pumpkin_stem_stage7", "pumpkin"), (1, "melon_stem_stage7", "melon")] {
            self.world.set_block(IVec3::new(c.x - 3, y + 1, c.z + dz), self.reg.id_or_air(stem));
            self.world.set_block(IVec3::new(c.x - 4, y, c.z + dz), self.reg.id_or_air("grass_block"));
            self.world.set_block(IVec3::new(c.x - 4, y + 1, c.z + dz), self.reg.id_or_air(fruit));
        }
        self.world.set_block(IVec3::new(c.x + 4, y + 1, c.z + 2), self.reg.id_or_air("composter"));
        self.world.set_block(IVec3::new(c.x + 4, y + 1, c.z - 2), self.reg.id_or_air("composter_ready"));
        let hay = self.reg.id_or_air("hay_block");
        self.world.set_block(IVec3::new(c.x + 4, y + 1, c.z), hay);
    }
}
