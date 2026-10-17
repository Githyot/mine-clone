//! Smarter Farmers idea, on top of the town farmers in `farm.rs` (which harvest and replant
//! ripe crops): farmers now also
//!
//! * sow bare farmland near home with seeds from their chest,
//! * pick pumpkins and melons (leaving the stems) and store them,
//! * keep working hours: at night they go home and rest instead of wandering the fields.

use glam::{IVec3, Vec3};
use voxel::item::{ItemId, ItemStack};
use voxel::{BlockId, AIR};

use crate::audio::{Material, Sfx};
use crate::farm::FARMER;
use crate::survival::Container;
use crate::Game;

#[derive(Default)]
pub struct Farmers {
    t: f32,
}

/// Blocks around home a farmer looks after.
const RANGE: i32 = 10;

impl Game {
    /// At night farmers stay home; true while they rest (the harvest in `farm.rs` waits).
    pub(crate) fn farmers_resting(&mut self) -> bool {
        if !self.is_night() {
            return false;
        }
        for m in self.mobs.list.iter_mut() {
            if m.alive() && self.mobs.reg.defs[m.def].name == FARMER {
                if let Some(h) = m.home {
                    if (m.body.pos - h).length() > 2.0 {
                        m.go_to(h, 4.0);
                    }
                }
            }
        }
        true
    }

    fn chest_near(&self, home: Vec3) -> Option<IVec3> {
        let h = home.floor().as_ivec3();
        self.surv
            .containers
            .iter()
            .filter(|(c, k)| matches!(k, Container::Chest(_)) && (**c - h).abs().max_element() <= 8)
            .map(|(c, _)| *c)
            .min_by_key(|c| (*c - h).length_squared())
    }

    /// Take one seed that plants on farmland out of a chest: (seed item, crop block).
    fn take_seed(&mut self, chest: IVec3) -> Option<(ItemId, BlockId)> {
        let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&chest) else { return None };
        for slot in inv.slots.iter_mut() {
            let Some(st) = *slot else { continue };
            let Some(b) = self.items.get(st.item).places else { continue };
            if !self.farm.sows_on_farmland(b) {
                continue;
            }
            *slot = if st.count > 1 { Some(ItemStack { count: st.count - 1, ..st }) } else { None };
            return Some((st.item, b));
        }
        None
    }

    pub(crate) fn update_smarter_farmers(&mut self, dt: f32) {
        self.mods.farmers.t -= dt;
        if self.mods.farmers.t > 0.0 || self.is_night() {
            return;
        }
        self.mods.farmers.t = 1.5;
        let Some(farmland) = self.reg.id("farmland") else { return };
        let (pumpkin, melon) = (self.reg.id_or_air("pumpkin"), self.reg.id_or_air("melon"));
        let farmers: Vec<(usize, Vec3, Vec3)> = self
            .mobs
            .list
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive() && self.mobs.reg.defs[m.def].name == FARMER)
            .map(|(i, m)| (i, m.body.pos, m.home.unwrap_or(m.body.pos)))
            .collect();
        for (i, pos, home) in farmers {
            if (pos - self.body.pos).length() > 96.0 {
                continue;
            }
            // Something to do near home: bare farmland or a pumpkin or melon to pick.
            let h = home.floor().as_ivec3();
            let mut job: Option<(i32, IVec3, bool)> = None;
            for dy in -3..=3 {
                for dz in -RANGE..=RANGE {
                    for dx in -RANGE..=RANGE {
                        let p = h + IVec3::new(dx, dy, dz);
                        let id = self.world.block_or_air(p);
                        let fruit = id != AIR && (id == pumpkin || id == melon) && self.stem_next_to(p);
                        let bare = id == farmland && self.world.block_or_air(p + IVec3::Y) == AIR;
                        if fruit || bare {
                            let d = dx * dx + dz * dz + dy * dy;
                            if job.is_none_or(|(b, _, _)| d < b) {
                                job = Some((d, if bare { p + IVec3::Y } else { p }, fruit));
                            }
                        }
                    }
                }
            }
            let Some((_, spot, fruit)) = job else { continue };
            let at = spot.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
            if Vec3::new(at.x - pos.x, 0.0, at.z - pos.z).length() > 1.8 {
                self.mobs.list[i].go_to(at, 2.0);
                continue;
            }
            let chest = self.chest_near(home);
            if fruit {
                let id = self.world.block_or_air(spot);
                let drops = self.items.drops(&self.reg, id, None, &mut self.surv.rng);
                self.world.set_block(spot, AIR);
                let (layer, tint) = self.block_look(id);
                self.particles.block_bits(spot, layer, tint, 6, 0.6);
                for d in drops {
                    let left = match chest.and_then(|c| self.surv.containers.get_mut(&c)) {
                        Some(Container::Chest(inv)) => inv.add(&self.items, d),
                        _ => Some(d),
                    };
                    if let Some(l) = left {
                        self.pop_drop(spot.as_vec3() + Vec3::splat(0.5), l);
                    }
                }
                if self.body.pos.distance(at) < 16.0 {
                    self.sfx(Sfx::Break(Material::Wood));
                }
            } else if let Some((_, crop)) = chest.and_then(|c| self.take_seed(c)) {
                self.world.set_block(spot, crop);
                if self.body.pos.distance(at) < 16.0 {
                    self.sfx(Sfx::Place(Material::Grass));
                }
            }
            self.mobs.list[i].go_to(at, 1.0);
        }
    }

    /// A pumpkin or melon grown from a stem (not one found wild): a stem block beside it.
    fn stem_next_to(&self, p: IVec3) -> bool {
        [IVec3::X, -IVec3::X, IVec3::Z, -IVec3::Z].iter().any(|&o| self.reg.get(self.world.block_or_air(p + o)).name.contains("stem"))
            || self.reg.id("pumpkin_stem").is_none()
    }
}
