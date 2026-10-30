//! Armor stands wear what you give them (Lote 2 leftover). Right click one holding a helmet,
//! chestplate, leggings or boots to put it on (what it wore comes back to you); holding anything
//! else, it takes that in its hand. With an empty hand you take things back, the hand first,
//! then from the head down. Knocking the stand over drops everything; it is saved with the
//! world like the pets.
//!
//! The pieces are drawn with the player's armour models on a still player skeleton standing
//! where the stand is: the stand's model has the player's proportions, so they line up.

use std::collections::HashMap;
use std::f32::consts::PI;

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;

use crate::entity::Bones;
use crate::item_render::GpuItem;
use crate::audio::{Material, Sfx};
use crate::Game;

/// Head, chest, legs, feet (the armour slots), then the hand.
pub(crate) type Gear = [Option<ItemStack>; 5];
const HAND: usize = 4;

#[derive(Default)]
pub(crate) struct Stands {
    /// By the stand's `net_id`: what it wears, and where it stood last frame (for its drops).
    pub gear: HashMap<u32, (Gear, Vec3)>,
    tested: bool,
}

impl Game {
    fn stand_at(&self) -> Option<usize> {
        let (i, t) = self.mobs.pick(self.body.eye(), self.forward(), 4.0)?;
        if self.target.is_some_and(|h| h.distance < t && self.reg.get(self.world.block_or_air(h.block)).solid) {
            return None;
        }
        (self.mobs.reg.defs[self.mobs.list[i].def].name == "armor_stand").then_some(i)
    }

    /// Right click on an armor stand.
    pub(crate) fn stand_use(&mut self) -> bool {
        let Some(i) = self.stand_at() else { return false };
        let (id, pos) = (self.mobs.list[i].net_id, self.mobs.list[i].body.pos);
        let mut gear = self.stands.gear.get(&id).map(|g| g.0).unwrap_or_default();
        let sel = self.selected;
        match self.surv.inv.slots[sel] {
            Some(held) => {
                let slot = self.items.get(held.item).armor.map(|a| (a.slot as usize).min(3)).unwrap_or(HAND);
                let mut one = held;
                one.count = 1;
                let back = gear[slot].replace(one);
                if self.survival || back.is_some() {
                    self.surv.inv.take(sel, 1);
                }
                if let Some(b) = back {
                    // Swapped: the old piece goes where the new one was, or anywhere it fits.
                    if self.surv.inv.slots[sel].is_none() {
                        self.surv.inv.slots[sel] = Some(b);
                    } else if let Some(left) = self.surv.inv.add(&self.items, b) {
                        self.spawn_drop(pos + Vec3::Y, left, Vec3::ZERO);
                    }
                }
            }
            None => {
                let Some(slot) = [HAND, 0, 1, 2, 3].into_iter().find(|&s| gear[s].is_some()) else { return true };
                let st = gear[slot].take().unwrap();
                self.surv.inv.slots[sel] = Some(st);
            }
        }
        self.stands.gear.insert(id, (gear, pos));
        self.sfx(Sfx::Place(Material::Wood));
        self.anim.start_swing();
        true
    }

    /// Each frame: a stand knocked over drops what it wore. One that left with its world
    /// (unloaded, saved as a pet) is just forgotten here.
    pub(crate) fn stands_update(&mut self) {
        self.stand_test_hook();
        if self.stands.gear.is_empty() {
            return;
        }
        let mut seen: HashMap<u32, (Vec3, bool)> = HashMap::new();
        for m in &self.mobs.list {
            if self.stands.gear.contains_key(&m.net_id) {
                seen.insert(m.net_id, (m.body.pos, m.alive()));
            }
        }
        let gone: Vec<u32> = self.stands.gear.keys().copied().filter(|k| !seen.get(k).is_some_and(|s| s.1)).collect();
        for k in gone {
            let (gear, _) = self.stands.gear.remove(&k).unwrap();
            if let Some(&(pos, false)) = seen.get(&k) {
                for st in gear.into_iter().flatten() {
                    self.spawn_drop(pos + Vec3::Y * 0.8, st, Vec3::new(0.0, 2.0, 0.0));
                }
            }
        }
        for (k, (p, _)) in seen {
            if let Some(g) = self.stands.gear.get_mut(&k) {
                g.1 = p;
            }
        }
    }

    /// Test hook MC_STAND: two dressed stands in front of the player.
    fn stand_test_hook(&mut self) {
        if self.stands.tested || !self.world_open || self.now < 1.0 {
            return;
        }
        self.stands.tested = true;
        if std::env::var("MC_STAND").is_err() {
            return;
        }
        let f = self.forward();
        let f = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-f.z, 0.0, f.x);
        let mut sets: Vec<[String; 5]> = vec![
            ["diamond_helmet", "diamond_chestplate", "diamond_leggings", "diamond_boots", "diamond_sword"].map(String::from),
            ["golden_helmet", "iron_chestplate", "chainmail_leggings", "leather_boots", "shield"].map(String::from),
        ];
        // MC_STAND=iron,golden,... puts up one stand per material, in a row, in its full set.
        let list = std::env::var("MC_STAND").unwrap_or_default();
        if list.contains(',') || list.chars().any(|c| c.is_alphabetic()) {
            sets = list.split(',').map(|m| ["helmet", "chestplate", "leggings", "boots", "sword"].map(|p| format!("{m}_{p}"))).collect();
        }
        let (n, gap) = (sets.len() as f32, if sets.len() > 2 { 1.35 } else { 1.6 });
        // MC_STAND_DIST: how far ahead the row stands; MC_STAND_TURN: degrees each stand is turned
        // from facing the player (for three-quarter and side views in screenshots).
        let dist = std::env::var("MC_STAND_DIST").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(2.6 + n * 0.55);
        let turn = std::env::var("MC_STAND_TURN").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0).to_radians();
        for (k, set) in sets.iter().enumerate() {
            let mut at = self.body.pos + f * dist + side * ((k as f32 - (n - 1.0) / 2.0) * gap);
            at.y = at.y.floor();
            while at.y > 1.0 && !self.reg.get(self.world.block_or_air((at - Vec3::Y * 0.5).floor().as_ivec3())).solid {
                at.y -= 1.0;
            }
            if !self.mobs.spawn("armor_stand", at) {
                continue;
            }
            let m = self.mobs.list.last_mut().unwrap();
            m.yaw = (-f.x).atan2(-f.z) + turn;
            let mut gear: Gear = Default::default();
            // MC_STAND_EMPTY=1: nothing in the hand, so the armour shows whole.
            let hands = std::env::var("MC_STAND_EMPTY").is_err();
            for (i, n) in set.iter().enumerate().filter(|(i, _)| *i < 4 || hands) {
                gear[i] = self.items.id(n).map(|id| ItemStack::new(id, 1));
            }
            let (id, pos) = (m.net_id, m.body.pos);
            self.stands.gear.insert(id, (gear, pos));
        }
    }

    /// The worn pieces and the held item of every stand in view.
    pub(crate) fn stand_instances(&mut self, block: IVec3, range: f32) -> (Vec<(usize, Bones)>, Vec<GpuItem>) {
        let (mut models, mut held) = (Vec::new(), Vec::new());
        let Some(pd) = self.mobs.reg.id("player") else { return (models, held) };
        if self.stands.gear.is_empty() {
            return (models, held);
        }
        let eye = self.body.eye();
        let mut list = Vec::new();
        for m in &self.mobs.list {
            if let Some((gear, _)) = self.stands.gear.get(&m.net_id) {
                if m.alive() && (m.body.pos - eye).length() < range {
                    list.push((m.body.pos, m.yaw, *gear));
                }
            }
        }
        for (pos, yaw, gear) in list {
            let def = &self.mobs.reg.defs[pd];
            let pose = crate::player_model::Pose { pos, look_yaw: yaw - PI, on_ground: true, holding: gear[HAND].is_some(), ..Default::default() };
            let l = self.world.light((pos + Vec3::Y).floor().as_ivec3());
            let bones = crate::player_model::Animator::lying(yaw).bones(def, &pose, block, (l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            for (piece, st) in gear[..4].iter().enumerate() {
                let Some(st) = st else { continue };
                if let Some(mesh) = self.entities.armor_mesh(&self.items.get(st.item).name, piece) {
                    models.push((mesh, bones));
                    if let Some(t) = crate::smithing::trim_of(st).and_then(|(_, m)| self.entities.trim_mesh(m, piece)) {
                        models.push((t, bones));
                    }
                }
            }
            if let Some(st) = gear[HAND] {
                // Put in our own hand for a moment to reuse the same placement.
                let pv = def.parts[2].pivot;
                let sel = self.selected;
                let mine = self.surv.inv.slots[sel].replace(st);
                held.extend(self.held_item_third_person(bones.forearm(2), Vec3::new(pv.0 as f32, pv.1 as f32, pv.2 as f32)));
                self.surv.inv.slots[sel] = mine;
            }
        }
        (models, held)
    }
}
