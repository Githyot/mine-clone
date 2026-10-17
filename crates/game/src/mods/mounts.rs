//! Riding creatures (the Aether's moas, phygs and flying cows): right click one with a saddle
//! to climb on, sneak to get off. The creature carries the player along and lends its gift: a
//! moa jumps again in mid-air (three times), a phyg or flying cow flies while jump is held.

use glam::Vec3;
use winit::keyboard::KeyCode;

use crate::audio::Sfx;
use crate::controller::Action;
use crate::Game;

/// Creatures that can be ridden: (name, air jumps, flies, speed boost).
const MOUNTS: &[(&str, u8, bool, f32)] = &[("moa", 3, false, 1.5), ("phyg", 0, true, 1.2), ("flying_cow", 0, true, 1.1)];

#[derive(Default)]
pub struct Mount {
    /// The ridden creature's index in the mob list and its kind in `MOUNTS`.
    pub ridden: Option<(usize, usize)>,
    air_jumps: u8,
    jump_was: bool,
}

impl Game {
    fn jump_held(&self) -> bool {
        self.keys.contains(&KeyCode::Space) || self.pad_held.contains(&Action::Jump)
    }

    /// Right click on a creature that can carry the player, holding a saddle.
    pub(crate) fn try_mount(&mut self, item: &str) -> bool {
        if item != "saddle" || self.mods.mount.ridden.is_some() {
            return false;
        }
        let eye = self.body.eye();
        let fwd = self.forward();
        let mut best: Option<(f32, usize, usize)> = None;
        for (i, m) in self.mobs.list.iter().enumerate() {
            let name = &self.mobs.reg.defs[m.def].name;
            let Some(k) = MOUNTS.iter().position(|t| t.0 == name) else { continue };
            let to = m.body.pos + Vec3::Y * 0.6 - eye;
            let d = to.length();
            if d < 4.0 && to.normalize_or_zero().dot(fwd) > 0.8 && best.is_none_or(|b| d < b.0) {
                best = Some((d, i, k));
            }
        }
        let Some((_, i, k)) = best else { return false };
        self.mods.mount.ridden = Some((i, k));
        self.body.pos = self.mobs.list[i].body.pos + Vec3::Y * 0.4;
        self.toast(format!("Montado: {} (agache para descer)", self.mobs.reg.defs[self.mobs.list[i].def].display_name), 3.0);
        self.sfx(Sfx::Pickup);
        true
    }

    pub(crate) fn update_mount(&mut self, dt: f32) {
        let Some((i, k)) = self.mods.mount.ridden else { return };
        let (name, jumps, flies, boost) = MOUNTS[k];
        let ok = self.mobs.list.get(i).is_some_and(|m| m.alive() && self.mobs.reg.defs[m.def].name == name);
        if !ok || self.crouch > 0.5 || self.dim_changed_since_mount() {
            self.mods.mount.ridden = None;
            return;
        }
        let jump = self.jump_held();
        let pressed = jump && !self.mods.mount.jump_was;
        self.mods.mount.jump_was = jump;
        if self.body.on_ground {
            self.mods.mount.air_jumps = jumps;
        } else if pressed && self.mods.mount.air_jumps > 0 {
            // The moa flaps up again.
            self.mods.mount.air_jumps -= 1;
            self.body.vel.y = 9.0;
            self.fall_peak = self.body.pos.y;
            self.sfx(Sfx::Step(crate::audio::Material::Cloth));
        }
        if flies {
            if jump {
                self.body.vel.y = (self.body.vel.y + 30.0 * dt).min(6.0);
            } else {
                self.body.vel.y = self.body.vel.y.max(-3.0);
            }
            self.fall_peak = self.body.pos.y;
        }
        // Faster on the ground than on foot.
        let h = Vec3::new(self.body.vel.x, 0.0, self.body.vel.z);
        if h.length() > 0.5 {
            self.body.vel.x *= 1.0 + (boost - 1.0) * (dt * 8.0).min(1.0);
            self.body.vel.z *= 1.0 + (boost - 1.0) * (dt * 8.0).min(1.0);
        }
        let (pos, vel, yaw) = (self.body.pos, self.body.vel, self.yaw);
        let m = &mut self.mobs.list[i];
        m.body.pos = pos;
        m.body.vel = vel;
        m.yaw = yaw + std::f32::consts::PI;
        m.aggro = false;
        if h.length() > 0.5 {
            m.walk += dt * h.length();
        }
    }

    /// The mob list is emptied on every trip between dimensions.
    fn dim_changed_since_mount(&self) -> bool {
        self.mobs.list.is_empty()
    }
}
