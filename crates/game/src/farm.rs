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
    t: f32,
}

impl Farm {
    pub fn new(reg: &BlockRegistry) -> Farm {
        let n = reg.len();
        let farmland = reg.id_or_air("farmland");
        let mut f = Farm { next: vec![AIR; n], first: vec![AIR; n], ripe: vec![false; n], farmland, soil: vec![farmland; n], t: 0.0 };
        let soul = reg.id_or_air("soul_sand");
        for (kind, stages) in [("wheat", 8), ("carrots", 4), ("potatoes", 4), ("beetroots", 4), ("nether_wart", 4), ("torchflower", 3), ("pitcher", 5)] {
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
        let hay = self.reg.id_or_air("hay_block");
        self.world.set_block(IVec3::new(c.x + 4, y + 1, c.z), hay);
    }
}
