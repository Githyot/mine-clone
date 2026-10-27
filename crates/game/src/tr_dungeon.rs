//! The big dungeon (plan/terraria-completo.md, what was left after Lote 11).
//!
//! The building itself is a structure of packs/vanilla/tr/structures.ron (`masmorra_blue`,
//! `_green` and `_pink`, written by tools/gen_tr.py): a brick tower over four floors of rooms
//! and halls. Here is what lives in it:
//! - Inside, skeletons, mutant skeletons and invokers keep coming (and the dungeon spirits
//!   once the Vine Queen is beaten); they sometimes drop a golden key.
//! - A golden key opens a locked chest, which holds the dungeon's best things.
//! - Spikes hurt; before the Bone King is beaten, the dungeon's curse withers whoever goes in.

use glam::{IVec3, Vec3};
use voxel::item::{Inventory, ItemStack};

use crate::audio::{Material, Sfx};
use crate::survival::{Container, Open};
use crate::Game;

/// What a locked chest holds: (item, min, max, chance).
const LOCKED_LOOT: &[(&str, u8, u8, f32)] = &[
    ("blue_katana", 1, 1, 0.3),
    ("bastion_shield", 1, 1, 0.3),
    ("bone_flail", 1, 1, 0.3),
    ("water_bolt", 1, 1, 0.25),
    ("spirit_bell", 1, 1, 0.2),
    ("lucky_horseshoe", 1, 1, 0.2),
    ("greater_healing_potion", 1, 3, 0.6),
    ("emerald", 4, 12, 0.8),
    ("diamond", 1, 3, 0.4),
];

#[derive(Default)]
pub struct Dungeon {
    pub inside: bool,
    check_t: f32,
    spawn_t: f32,
    curse_t: f32,
    warned: bool,
    spikes: Option<u16>,
}

impl Game {
    fn is_dungeon_brick(&self, p: IVec3) -> bool {
        self.reg.get(self.world.block_or_air(p)).name.starts_with("dungeon_brick_")
    }

    pub(crate) fn update_tr_dungeon(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let me = self.body.pos.floor().as_ivec3();
        // Spikes under or around the feet.
        let spikes = *self.t5.tr.dungeon.spikes.get_or_insert_with(|| self.reg.id_or_air("dungeon_spikes"));
        if spikes != voxel::AIR && (self.world.block_or_air(me) == spikes || self.world.block_or_air(me + IVec3::Y) == spikes) {
            let at = self.body.pos - Vec3::Y * 0.5;
            self.damage(4.0, Some(at));
        }
        let d = &mut self.t5.tr.dungeon;
        d.check_t -= dt;
        d.spawn_t -= dt;
        d.curse_t -= dt;
        if d.check_t <= 0.0 {
            d.check_t = 1.0;
            // Inside: dungeon brick under the feet and over the head.
            let below = (1..4).any(|k| self.is_dungeon_brick(me - IVec3::Y * k));
            let above = (2..8).any(|k| self.is_dungeon_brick(me + IVec3::Y * k));
            let inside = below && above;
            if inside && !self.t5.tr.dungeon.inside {
                self.toast("A Masmorra", 2.5);
            }
            self.t5.tr.dungeon.inside = inside;
        }
        if !self.t5.tr.dungeon.inside || !self.survival || self.mobs.peaceful {
            return;
        }
        // The curse, until the Bone King falls.
        let free = self.boss.defeated.iter().any(|b| b == "bone_king");
        if !free && self.t5.tr.dungeon.curse_t <= 0.0 {
            self.t5.tr.dungeon.curse_t = 3.0;
            self.add_effect(crate::effects::WITHER, 1, 4.0, false);
            self.add_effect(crate::effects::DARKNESS, 0, 4.0, false);
            if !self.t5.tr.dungeon.warned {
                self.t5.tr.dungeon.warned = true;
                self.toast("Uma maldição pesa aqui. Derrote o Rei dos Ossos (Crânio Amaldiçoado, à noite) para quebrá-la.", 6.0);
            }
        }
        if self.t5.tr.dungeon.spawn_t > 0.0 {
            return;
        }
        self.t5.tr.dungeon.spawn_t = 5.0;
        let here = self.body.pos;
        let near = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].kind == voxel::mob::Kind::Hostile && (m.body.pos - here).length() < 24.0).count();
        if near >= 6 {
            return;
        }
        let spirits = self.t5.hard.on && self.boss.defeated.iter().any(|b| b == "vine_queen");
        let r = self.surv.rng.f32();
        let who = if spirits && r < 0.25 {
            "dungeon_spirit"
        } else if r < 0.6 {
            "skeleton"
        } else if r < 0.85 {
            "invoker"
        } else {
            "mutant_skeleton"
        };
        if let Some(at) = self.spot_near(here, 6.0, 16.0, true) {
            if self.mobs.spawn(who, at + Vec3::Y * 0.1) {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.aggro = true;
                }
            }
        }
    }

    /// Monsters killed in the dungeon sometimes leave a golden key.
    pub(crate) fn dungeon_deaths(&mut self, deaths: &[(usize, Vec3)]) {
        if !self.t5.tr.dungeon.inside {
            return;
        }
        let Some(key) = self.items.id("golden_key") else { return };
        for &(def, pos) in deaths {
            if self.mobs.reg.defs[def].kind == voxel::mob::Kind::Hostile && self.surv.rng.f32() < 1.0 / 10.0 {
                self.spawn_drop(pos + Vec3::Y * 0.5, ItemStack::new(key, 1), Vec3::Y * 2.0);
            }
        }
    }

    pub(crate) fn use_locked_chest(&mut self, p: IVec3) -> bool {
        let key = self.items.id("golden_key");
        let slot = key.and_then(|k| self.surv.inv.slots.iter().position(|s| s.is_some_and(|s| s.item == k)));
        if slot.is_none() && self.survival {
            self.toast("Trancado. Precisa de uma Chave Dourada.", 2.5);
            self.sfx(Sfx::Click);
            return true;
        }
        if let (Some(slot), true) = (slot, self.survival) {
            self.surv.inv.take(slot, 1);
        }
        let mut inv = Inventory::new(27);
        for &(item, lo, hi, chance) in LOCKED_LOOT {
            if self.surv.rng.f32() >= chance {
                continue;
            }
            let Some(id) = self.items.id(item) else { continue };
            let n = lo + self.surv.rng.below((hi - lo + 1) as u32) as u8;
            let at = self.surv.rng.below(27) as usize;
            let _ = inv.add_in(&self.items, ItemStack::new(id, n), (0..27).map(move |i| (i + at) % 27));
        }
        let chest = self.reg.id_or_air("chest");
        self.world.set_block(p, chest);
        self.surv.containers.insert(p, Container::Chest(inv));
        self.surv.open = Open::Chest(p);
        self.set_screen(crate::Screen::Inventory);
        self.sfx(Sfx::Place(Material::Metal));
        self.toast("A chave gira. O baú se abre!", 2.0);
        true
    }

    /// Test scene: MC_TR=dungeon, the blue dungeon's floors built around the player, standing
    /// in a room of the third floor.
    pub(crate) fn test_tr_dungeon(&mut self) {
        let structs = self.world.gen.structures();
        let Some(s) = structs.list.iter().find(|s| s.def.name == "masmorra_blue") else { return };
        let def = s.def.clone();
        let ids: Vec<(char, u16)> = def.palette.iter().map(|(c, n)| (*c, self.reg.id_or_air(n))).collect();
        let me = self.body.pos.floor().as_ivec3();
        let origin = me - IVec3::new(12, 16, 12);
        for (y, layer) in def.layers.iter().enumerate().take(def.sink as usize) {
            for (z, row) in layer.iter().enumerate() {
                for (x, ch) in row.chars().enumerate() {
                    let id = match ch {
                        '.' => voxel::AIR,
                        ' ' => continue,
                        c => match ids.iter().find(|(k, _)| *k == c) {
                            Some(&(_, id)) => id,
                            None => continue,
                        },
                    };
                    self.world.set_block(origin + IVec3::new(x as i32, y as i32, z as i32), id);
                }
            }
        }
        self.body.pos = (origin + IVec3::new(12, 16, 12)).as_vec3() + Vec3::new(0.5, 0.0, 0.5);
        self.t5.tr.dungeon.inside = true;
        for (k, who) in ["skeleton", "invoker", "mutant_skeleton"].iter().enumerate() {
            let at = self.body.pos + Vec3::new(-2.0 + k as f32 * 2.0, 0.0, -4.0);
            let _ = self.mobs.spawn(who, at);
        }
    }
}
