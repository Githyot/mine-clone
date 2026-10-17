//! Tame animals and mounts. A saddle on a horse, donkey, llama or camel lets you ride it:
//! right click to climb on, steer with the movement keys, jump with space and sneak to get
//! off. A bone may tame a wolf and raw fish may tame a cat; tamed ones follow you around,
//! fight monsters (wolves) or chase small animals (cats), and are saved with the world.

use std::path::Path;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use voxel::physics::Input;

use crate::Game;

const MOUNTS: [&str; 4] = ["horse", "donkey", "llama", "camel"];
/// (wild, tamed, the food that tames it, chance per try)
const TAMES: [(&str, &str, &[&str], f32); 2] = [("wolf", "wolf_tame", &["bone"], 0.33), ("cat", "cat_tame", &["cod", "salmon", "raw_cod", "raw_salmon"], 0.33)];

#[derive(Clone, Serialize, Deserialize)]
pub struct PetSave {
    pub name: String,
    pub pos: [f32; 3],
    pub saddled: bool,
}

pub fn load(dir: &Path) -> Vec<PetSave> {
    std::fs::read_to_string(dir.join("pets.ron")).ok().and_then(|t| ron::from_str(&t).ok()).unwrap_or_default()
}

impl Game {
    fn is_pet(&self, i: usize) -> bool {
        let m = &self.mobs.list[i];
        let n = self.mobs.reg.defs[m.def].name.as_str();
        n.ends_with("_tame") || m.saddled
    }

    pub(crate) fn save_pets(&self) {
        let list: Vec<PetSave> = (0..self.mobs.list.len())
            .filter(|&i| self.mobs.list[i].alive() && self.is_pet(i))
            .map(|i| {
                let m = &self.mobs.list[i];
                PetSave { name: self.mobs.reg.defs[m.def].name.clone(), pos: m.body.pos.to_array(), saddled: m.saddled }
            })
            .chain(self.pets_waiting.iter().cloned())
            .collect();
        if let Ok(t) = ron::ser::to_string_pretty(&list, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(self.world.save.dir.join("pets.ron"), t);
        }
    }

    /// Right click on an animal: saddle it, ride it, or feed it to tame it.
    pub(crate) fn pet_use(&mut self) -> bool {
        let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), 4.0) else { return false };
        if self.target.is_some_and(|h| h.distance < t && self.reg.get(self.world.block_or_air(h.block)).solid) {
            return false;
        }
        let name = self.mobs.reg.defs[self.mobs.list[i].def].name.clone();
        let held = self.held_stack().map(|s| self.items.get(s.item).name.clone()).unwrap_or_default();
        if MOUNTS.contains(&name.as_str()) {
            if !self.mobs.list[i].saddled {
                if held != "saddle" {
                    self.toast("Use uma sela para poder montar", 2.5);
                    return true;
                }
                self.mobs.list[i].saddled = true;
                if self.survival {
                    let sel = self.selected;
                    self.surv.inv.take(sel, 1);
                }
                self.toast("Selado! Clique com o direito para montar", 3.0);
                return true;
            }
            if self.crouch > 0.5 || self.mobs.list.iter().any(|m| m.ridden) {
                return false;
            }
            self.mobs.list[i].ridden = true;
            self.toast("Agache (Shift) para descer", 2.5);
            return true;
        }
        if let Some(&(_, tame, food, chance)) = TAMES.iter().find(|t| t.0 == name) {
            if !food.contains(&held.as_str()) {
                return false;
            }
            if self.survival {
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
            }
            let glow = self.glow_layer();
            let at = self.mobs.list[i].body.pos + Vec3::Y;
            if self.surv.rng.f32() < chance {
                if let Some(d) = self.mobs.reg.id(tame) {
                    let m = &mut self.mobs.list[i];
                    m.def = d;
                    m.health = self.mobs.reg.defs[d].health;
                    m.max_health = m.health;
                }
                for k in 0..10 {
                    let a = k as f32 * 0.63;
                    self.particles.glow(at + Vec3::new(a.cos() * 0.4, 0.2, a.sin() * 0.4), Vec3::Y * 0.8, glow, [1.0, 0.35, 0.45], 0.06, 1.2);
                }
                self.toast("Domado! Agora ele te segue", 3.0);
            } else {
                self.particles.glow(at, Vec3::Y * 0.6, glow, [0.5, 0.5, 0.5], 0.05, 1.0);
            }
            return true;
        }
        false
    }

    /// The player on a mount: it runs where they steer. True while riding.
    pub(crate) fn mount_step(&mut self, wish: Vec3, jump: bool, sneak: bool, dt: f32) -> bool {
        let Some(i) = self.mobs.list.iter().position(|m| m.ridden) else { return false };
        let alive = self.mobs.list[i].alive();
        let f = self.forward();
        let m = &mut self.mobs.list[i];
        if sneak || !alive {
            m.ridden = false;
            let side = Vec3::new(m.yaw.cos(), 0.0, -m.yaw.sin());
            self.body.pos = m.body.pos + side * 1.2 + Vec3::Y * 0.2;
            self.body.vel = Vec3::ZERO;
            return false;
        }
        let def = &self.mobs.reg.defs[m.def];
        m.yaw = f.x.atan2(f.z);
        m.body.speed_override = Some(def.speed * 2.0);
        let before = m.body.pos;
        m.body.step(&self.world, &Input { wish, jump, sneak: false, sprint: false, crawl: false }, dt);
        let speed = ((m.body.pos - before) * Vec3::new(1.0, 0.0, 1.0)).length() / dt.max(1e-4);
        m.walk_speed += (speed - m.walk_speed) * (dt * 10.0).min(1.0);
        m.walk += m.walk_speed * dt * 2.2;
        self.body.pos = m.body.pos + Vec3::Y * (def.height * def.scale.max(0.5) * 0.55);
        self.body.vel = m.body.vel;
        self.body.on_ground = true;
        self.body.fall_distance = 0.0;
        true
    }

    /// Tame animals follow the player; saved ones come back when the player is near.
    pub(crate) fn update_pets(&mut self, dt: f32) {
        if dt == 0.0 {
            return;
        }
        let me = self.body.pos;
        let mut k = 0;
        while k < self.pets_waiting.len() {
            let p = Vec3::from(self.pets_waiting[k].pos);
            if p.distance(me) < 64.0 && self.world.block(p.floor().as_ivec3()).is_some() {
                let pet = self.pets_waiting.remove(k);
                if self.mobs.spawn(&pet.name, p) {
                    if let Some(m) = self.mobs.list.last_mut() {
                        m.saddled = pet.saddled;
                    }
                }
            } else {
                k += 1;
            }
        }
        for m in self.mobs.list.iter_mut() {
            if !m.alive() || m.ridden || !self.mobs.reg.defs[m.def].name.ends_with("_tame") {
                continue;
            }
            let d = m.body.pos.distance(me);
            if d > 28.0 {
                m.body.pos = me - self.body.vel.normalize_or_zero() * 2.0 + Vec3::Y * 0.5;
            } else if d > 6.0 && m.prey.is_none() {
                m.go_to(me, 0.5);
            }
        }
    }
}
