//! Accessories (Terraria's idea): five slots beside the inventory for things that change how you
//! move and fight. Boots run faster, a cloud in a bottle gives a second jump, wings fly for a
//! moment and then glide, a cape dashes on a double tap, claws climb walls, the horseshoe takes
//! the sting out of falls; rings, amulets and emblems help mana, magic and melee.

use glam::Vec3;
use voxel::physics::{Terrain, WALK_SPEED};
use winit::keyboard::KeyCode;

use crate::audio::Sfx;
use crate::survival::Slot;
use crate::Game;

pub fn is_accessory(name: &str) -> bool {
    crate::rpg::accessory(name).is_some()
}

/// What an accessory does, in one line (for the toast when it is put on).
pub fn describe(name: &str) -> Option<String> {
    let a = crate::rpg::accessory(name)?;
    if !a.desc.is_empty() {
        return Some(a.desc.to_string());
    }
    a.mods.first().and_then(crate::rpg::mod_line)
}

/// Movement state the accessories need from frame to frame.
#[derive(Default)]
pub struct Moves {
    pub(crate) wing_t: f32,
    jumped_twice: bool,
    space_was: bool,
    was_ground: bool,
    balloon_done: bool,
    a_was: bool,
    d_was: bool,
    last_a: f32,
    last_d: f32,
    dash_cd: f32,
    pub lava_t: f32,
    /// Flying on wings right now (for the HUD).
    pub flying: bool,
}

impl Game {
    /// Is this accessory worn (in an open slot)?
    pub(crate) fn wears(&self, name: &str) -> bool {
        self.t5.acc[..self.acc_slots()].iter().flatten().any(|s| self.items.get(s.item).name == name)
    }

    /// Melee damage multiplier: emblems, sets, buffs (rpg.rs).
    pub(crate) fn melee_bonus(&self) -> f32 {
        self.t5.rpg.stats.dmg[crate::rpg::MELEE]
    }

    pub(crate) fn magic_bonus(&self) -> f32 {
        self.t5.rpg.stats.dmg[crate::rpg::MAGIC]
    }

    /// Falls don't hurt (horseshoe, angel wings, flying on feather wings).
    pub(crate) fn no_fall_damage(&self) -> bool {
        self.t5.rpg.stats.no_fall || self.worn_wings().is_some() && self.t5.moves.flying
    }

    pub(crate) fn no_knockback(&self) -> bool {
        self.t5.rpg.stats.no_knockback
    }

    /// Lava doesn't burn: always with magma boots, for 7 s with the obsidian charm.
    pub(crate) fn lava_safe(&mut self, dt: f32) -> bool {
        if self.t5.rpg.stats.lava_immune {
            return true;
        }
        if !self.wears("obsidian_charm") {
            return false;
        }
        self.t5.moves.lava_t += dt;
        self.t5.moves.lava_t < 7.0
    }

    /// Right click with an accessory: into the first free slot (or swap with the first one).
    pub(crate) fn wear_accessory(&mut self) -> bool {
        let sel = self.selected;
        let Some(st) = self.surv.inv.slots[sel] else { return false };
        let name = self.items.get(st.item).name.clone();
        if self.wears(&name) {
            self.toast("Você já usa esse acessório", 2.0);
            return true;
        }
        let n = self.acc_slots();
        // One pair of wings at a time: new wings take the place of the old.
        let wing_slot = crate::wings::wing_of(&name).and_then(|_| self.t5.acc[..n].iter().position(|a| a.is_some_and(|a| crate::wings::wing_of(&self.items.get(a.item).name).is_some())));
        let i = wing_slot.unwrap_or_else(|| self.t5.acc[..n].iter().position(|a| a.is_none()).unwrap_or(0));
        self.surv.inv.slots[sel] = self.t5.acc[i].take();
        self.t5.acc[i] = Some(st);
        self.sfx(Sfx::Place(crate::audio::Material::Cloth));
        let d = self.items.get(st.item).display_name.clone();
        self.toast(format!("{d}: {}", describe(&name).unwrap_or_default()), 3.0);
        true
    }

    pub(crate) fn update_accessories(&mut self, dt: f32) {
        // Mana and life regeneration come from the stats (rpg.rs).
        if !self.survival || self.screen == crate::Screen::Dead || dt == 0.0 {
            self.body.speed_override = None;
            return;
        }
        // Out of lava, the charm cools down again.
        let feet = self.body.pos.floor().as_ivec3();
        if self.world.fluid(self.world.block_or_air(feet)) != 2 {
            self.t5.moves.lava_t = (self.t5.moves.lava_t - dt * 2.0).max(0.0);
        }
        // Speed: boots, sets, buffs and the "Ágil" modifier (rpg.rs); the boots don't stack.
        let speed = self.t5.rpg.stats.move_speed - if self.wears("frost_spark_boots") && self.wears("running_boots") { 0.3 } else { 0.0 };
        self.body.speed_override = if (speed - 1.0).abs() > 0.001 && !self.body.in_water { Some(WALK_SPEED * speed.max(0.2)) } else { None };
        // Fins: faster swimming the way you look.
        let swim = self.t5.rpg.stats.swim;
        if swim > 0.0 && self.body.in_water && self.screen == crate::Screen::Playing && self.keys.contains(&KeyCode::KeyW) {
            let f = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
            let cap = 4.0 * (1.0 + swim);
            let flat = Vec3::new(self.body.vel.x, 0.0, self.body.vel.z);
            if flat.length() < cap {
                self.body.vel += f * swim * 12.0 * dt;
            }
        }

        let playing = self.screen == crate::Screen::Playing;
        let space = playing && self.keys.contains(&KeyCode::Space);
        let pressed = space && !self.t5.moves.space_was;
        self.t5.moves.space_was = space;
        let ground = self.body.on_ground || self.body.in_water;
        if ground {
            self.t5.moves.jumped_twice = false;
            // Each pair flies for its own time; steam boots give a short lift, no glide.
            self.t5.moves.wing_t = self.worn_wings().map_or(0.9, |k| crate::wings::WINGS[k].flight);
            self.t5.moves.flying = false;
            self.t5.moves.balloon_done = false;
        }
        // Balloon, frog charm: the jump that just left the ground goes higher.
        let jump = self.t5.rpg.stats.jump;
        if self.t5.moves.was_ground && !self.body.on_ground && self.body.vel.y > 8.0 && !self.t5.moves.balloon_done && jump > 0.0 {
            self.body.vel.y *= 1.0 + jump.min(0.8);
            self.t5.moves.balloon_done = true;
        }
        self.t5.moves.was_ground = self.body.on_ground;
        let glow = self.glow_layer();
        let hooked = self.t5.hook.attached();
        if !ground && !hooked && self.seat.is_none() && self.riding.is_none() {
            let worn = self.worn_wings();
            let wings = worn.is_some();
            let steam = !wings && self.wears("steam_boots");
            if pressed && self.wears("cloud_bottle") && !self.t5.moves.jumped_twice {
                // A puff of cloud and a second jump.
                self.t5.moves.jumped_twice = true;
                self.body.vel.y = 9.5;
                self.fall_peak = self.body.pos.y;
                for _ in 0..14 {
                    let v = Vec3::new(self.surv.rng.f32() - 0.5, -0.4, self.surv.rng.f32() - 0.5) * 3.0;
                    self.particles.glow(self.body.pos, v, glow, [1.3, 1.3, 1.35], 0.18, 0.6);
                }
                self.sfx(Sfx::Place(crate::audio::Material::Cloth));
            } else if space && steam && self.t5.moves.wing_t > 0.0 {
                // Steam boots: puff upward for a moment.
                self.t5.moves.wing_t -= dt;
                self.body.vel.y = (self.body.vel.y + 40.0 * dt).min(6.0);
                self.t5.moves.flying = true;
                self.fall_peak = self.body.pos.y;
                if self.surv.rng.f32() < dt * 25.0 {
                    self.particles.glow(self.body.pos, Vec3::new(0.0, -2.0, 0.0), glow, [1.1, 1.1, 1.1], 0.1, 0.4);
                }
            } else if let (true, Some(k)) = (space, worn) {
                let def = &crate::wings::WINGS[k];
                if self.t5.moves.wing_t > 0.0 {
                    self.t5.moves.wing_t -= dt;
                    self.body.vel.y = (self.body.vel.y + 45.0 * dt).min(def.lift);
                    self.t5.moves.flying = true;
                    // Faster through the air than on foot.
                    let base = self.body.speed_override.unwrap_or(WALK_SPEED);
                    self.body.speed_override = Some(base * def.speed);
                } else if self.body.vel.y < -2.5 {
                    // Out of flight time: glide down.
                    self.body.vel.y = -2.5;
                }
                self.fall_peak = self.body.pos.y;
                if self.surv.rng.f32() < dt * 20.0 {
                    let c = crate::wings::WINGS[k].colors[2];
                    let tint = [c[0] as f32 / 200.0, c[1] as f32 / 200.0, c[2] as f32 / 200.0];
                    self.particles.glow(self.body.pos + Vec3::Y * 1.2, Vec3::new(0.0, -1.5, 0.0), glow, tint, 0.08, 0.5);
                }
            }
            // Claws: hold Space against a wall to climb it.
            if space && (self.wears("climbing_claws") || self.wears("shadow_gear")) && self.against_wall() {
                self.body.vel.y = self.body.vel.y.max(3.5);
                self.fall_peak = self.body.pos.y;
            }
        }
        // The cape: tap A or D twice quickly to dash that way.
        let a = playing && self.keys.contains(&KeyCode::KeyA);
        let d = playing && self.keys.contains(&KeyCode::KeyD);
        self.t5.moves.dash_cd -= dt;
        let mut dash = 0.0;
        if a && !self.t5.moves.a_was {
            if self.now - self.t5.moves.last_a < 0.25 {
                dash = -1.0;
            }
            self.t5.moves.last_a = self.now;
        }
        if d && !self.t5.moves.d_was {
            if self.now - self.t5.moves.last_d < 0.25 {
                dash = 1.0;
            }
            self.t5.moves.last_d = self.now;
        }
        self.t5.moves.a_was = a;
        self.t5.moves.d_was = d;
        if dash != 0.0 && self.t5.moves.dash_cd <= 0.0 && (self.wears("wind_cape") || self.wears("shadow_gear")) {
            self.t5.moves.dash_cd = 1.0;
            let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
            self.body.vel += right * dash * 16.0;
            for _ in 0..12 {
                self.particles.glow(self.body.pos + Vec3::Y * 0.8, -right * dash * 2.0, glow, [0.9, 1.1, 1.2], 0.1, 0.4);
            }
            self.sfx(Sfx::Place(crate::audio::Material::Cloth));
        }
    }

    fn against_wall(&self) -> bool {
        let f = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let p = self.body.pos + f * 0.5 + Vec3::Y * 0.5;
        self.world.solid(p.floor().as_ivec3()) || self.world.solid((p + Vec3::Y).floor().as_ivec3())
    }

    /// Slots of the accessory column, left of the inventory panel at (x0, y0), with the loadout
    /// buttons over it and the stats sheet beside it.
    pub(crate) fn accessory_slots(&mut self, x0: f32, y0: f32, mouse: (f32, f32)) -> Vec<(Slot, f32, f32)> {
        let s = self.ui.scale;
        let n = self.acc_slots();
        let (px, py) = (x0 - 26.0 * s, y0 + 6.0 * s);
        let ph = (n as f32 * 18.0 + 16.0) * s;
        self.ui.rect(px - 2.0 * s, py - 10.0 * s, 24.0 * s, ph + 10.0 * s, [198, 198, 198, 255]);
        self.ui.frame(px - 2.0 * s, py - 10.0 * s, 24.0 * s, ph + 10.0 * s, s, [255, 255, 255, 255]);
        self.draw_loadout_buttons(px - 0.5 * s, py - 8.5 * s, mouse);
        self.ui.label(px - 1.0 * s, py + 2.0 * s, "Ac.", [64, 64, 64, 255], s);
        if self.t5.rpg.sheet {
            self.draw_stats_sheet(px - 4.0 * s, py - 10.0 * s);
        } else {
            let tw = self.ui.text_width("C: status", self.ui.scale);
            self.ui.text(px - 4.0 * s - tw, py + ph - 10.0 * s, "C: status", [230, 230, 230, 255]);
        }
        let mut slots: Vec<(Slot, f32, f32)> = (0..n).map(|i| (Slot::Acc(i), px + 1.0 * s, py + (12.0 + i as f32 * 18.0) * s)).collect();
        if !self.t5.rpg.sheet {
            slots.extend(self.visual_slots(px - 2.0 * s, py));
        }
        slots
    }

    pub(crate) fn draw_accessory_hud(&mut self, _w: f32, h: f32) {
        if !self.survival {
            return;
        }
        let s = self.ui.scale;
        // The depth meter and the other info accessories draw in rpg.rs.
        if self.t5.moves.flying && self.t5.moves.wing_t > 0.0 {
            let max = self.worn_wings().map_or(0.9, |k| crate::wings::WINGS[k].flight);
            let k = (self.t5.moves.wing_t / max).clamp(0.0, 1.0);
            self.ui.rect(4.0 * s, h - 34.0 * s, 40.0 * s, 3.0 * s, [30, 30, 30, 200]);
            self.ui.rect(4.0 * s, h - 34.0 * s, 40.0 * s * k, 3.0 * s, [240, 230, 160, 230]);
        }
        // A flying mount's flight left.
        if let Some(k) = self.mount_flight_left() {
            self.ui.rect(4.0 * s, h - 34.0 * s, 40.0 * s, 3.0 * s, [30, 30, 30, 200]);
            self.ui.rect(4.0 * s, h - 34.0 * s, 40.0 * s * k, 3.0 * s, [250, 210, 90, 230]);
        }
    }
}
