//! Accessories (Terraria's idea): five slots beside the inventory for things that change how you
//! move and fight. Boots run faster, a cloud in a bottle gives a second jump, wings fly for a
//! moment and then glide, a cape dashes on a double tap, claws climb walls, the horseshoe takes
//! the sting out of falls; rings, amulets and emblems help mana, magic and melee. From the deep
//! world: more jumps in a bottle (each one once per jump, and they stack), rocket boots, boots
//! that walk on water, a shield that charges through monsters, and gear that wall-jumps.

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
    /// Extra jumps already used since leaving the ground (bit per JUMPS entry).
    jumps_used: u8,
    /// Seconds of the sandstorm's floaty rise left.
    sand_t: f32,
    /// Seconds of the shield charge left (it hurts what it runs through).
    charge_t: f32,
    charge_dir: Vec3,
    pub lava_t: f32,
    /// Flying on wings right now (for the HUD).
    pub flying: bool,
}

/// Accessories that are several in one (Lote 9): wearing one counts as wearing its parts.
const COMBOS: &[(&str, &[&str])] = &[
    ("terraspark_boots", &["frost_spark_boots", "water_walking_boots", "obsidian_charm"]),
    ("bundle_of_balloons", &["cloud_bottle", "blizzard_bottle", "sandstorm_bottle", "shiny_balloon"]),
];

/// Extra jumps in the air, used in this order: (accessory, upward speed).
const JUMPS: &[(&str, f32)] = &[
    ("cloud_bottle", 9.5),
    ("cloud_balloon", 9.5),
    ("blizzard_bottle", 12.0),
    ("sandstorm_bottle", 7.5),
    ("leap_boots", 10.5),
];

impl Game {
    /// Seconds of lift in the air: wings, then rocket boots, then steam boots.
    fn flight_max(&self) -> f32 {
        if let Some(k) = self.worn_wings() {
            crate::wings::WINGS[k].flight
        } else if let Some(t) = self.tr_wing().map(|w| w.1) {
            t
        } else if self.wears("comet_boots") {
            2.6
        } else if self.wears("rocket_boots") {
            1.8
        } else {
            0.9
        }
    }

    /// Is this accessory worn (in an open slot)?
    pub(crate) fn wears(&self, name: &str) -> bool {
        self.t5.acc[..self.acc_slots()].iter().flatten().any(|s| {
            let worn = &self.items.get(s.item).name;
            worn == name || COMBOS.iter().any(|(c, parts)| c == worn && parts.contains(&name))
        })
    }

    /// Melee damage multiplier: emblems, sets, buffs (rpg.rs).
    pub(crate) fn melee_bonus(&self) -> f32 {
        self.t5.rpg.stats.dmg[crate::rpg::MELEE] * self.skill_melee()
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
            self.t5.moves.jumps_used = 0;
            self.t5.moves.wing_t = self.flight_max();
            self.t5.moves.flying = false;
            self.t5.moves.balloon_done = false;
        }
        // Water walking boots: the surface holds you up unless you sneak.
        let sneak = playing && (self.keys.contains(&KeyCode::ShiftLeft) || self.keys.contains(&KeyCode::ShiftRight));
        if self.wears("water_walking_boots") && !sneak && self.body.vel.y <= 0.5 {
            let p = self.body.pos;
            let below = (p - Vec3::Y * 0.45).floor().as_ivec3();
            let top = below.y as f32 + 1.0;
            let water = self.world.fluid(self.world.block_or_air(below)) == 1;
            let open = self.world.fluid(self.world.block_or_air(below + glam::IVec3::Y)) == 0;
            if water && open && p.y > top - 0.45 {
                self.body.pos.y = top + 0.02;
                self.body.vel.y = 0.0;
                self.body.in_water = false;
                self.fall_peak = top;
                self.t5.moves.jumps_used = 0;
                self.t5.moves.wing_t = self.flight_max();
                if pressed {
                    self.body.vel.y = 8.5;
                }
            }
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
            // The painted pairs (wings.rs) first, then the Terraria side's own (tr_pets.rs).
            let worn = self.worn_wings();
            let tr_wing = if worn.is_none() { self.tr_wing() } else { None };
            let wings = worn.is_some() || tr_wing.is_some();
            let rocket = !wings && (self.wears("rocket_boots") || self.wears("comet_boots"));
            let steam = !wings && !rocket && self.wears("steam_boots");
            let next = if pressed {
                JUMPS.iter().enumerate().find(|(k, j)| self.t5.moves.jumps_used & (1 << k) == 0 && self.wears(j.0))
            } else {
                None
            };
            if let Some((k, &(name, up))) = next {
                // A puff of cloud, snow or sand, and one more jump.
                self.t5.moves.jumps_used |= 1 << k;
                self.t5.moves.jumped_twice = true;
                self.body.vel.y = up;
                self.fall_peak = self.body.pos.y;
                self.t5.moves.sand_t = if name == "sandstorm_bottle" { 0.7 } else { 0.0 };
                let tint = match name {
                    "blizzard_bottle" => [1.0, 1.2, 1.5],
                    "sandstorm_bottle" => [1.4, 1.15, 0.7],
                    "leap_boots" => [0.8, 1.3, 0.8],
                    _ => [1.3, 1.3, 1.35],
                };
                for _ in 0..14 {
                    let v = Vec3::new(self.surv.rng.f32() - 0.5, -0.4, self.surv.rng.f32() - 0.5) * 3.0;
                    self.particles.glow(self.body.pos, v, glow, tint, 0.18, 0.6);
                }
                self.sfx(Sfx::Place(crate::audio::Material::Cloth));
            } else if space && self.t5.moves.sand_t > 0.0 {
                // The sandstorm keeps lifting you for a moment while Space is held.
                self.t5.moves.sand_t -= dt;
                self.body.vel.y = self.body.vel.y.max(5.0);
                self.fall_peak = self.body.pos.y;
                if self.surv.rng.f32() < dt * 30.0 {
                    self.particles.glow(self.body.pos, Vec3::new(0.0, -1.0, 0.0), glow, [1.4, 1.15, 0.7], 0.12, 0.5);
                }
            } else if space && rocket && self.t5.moves.wing_t > 0.0 && (self.body.vel.y < 3.0 || self.t5.moves.flying) {
                // Rocket boots: once the jump runs out, hold Space to burn upward.
                self.t5.moves.wing_t -= dt;
                self.body.vel.y = (self.body.vel.y + 50.0 * dt).min(8.0);
                self.t5.moves.flying = true;
                self.fall_peak = self.body.pos.y;
                if self.surv.rng.f32() < dt * 40.0 {
                    let v = Vec3::new(self.surv.rng.f32() - 0.5, -6.0, self.surv.rng.f32() - 0.5);
                    let tint = if self.wears("comet_boots") { [1.2, 0.9, 1.6] } else { [1.6, 0.8, 0.3] };
                    self.particles.glow(self.body.pos, v, glow, tint, 0.1, 0.35);
                }
            } else if space && steam && self.t5.moves.wing_t > 0.0 {
                // Steam boots: puff upward for a moment.
                self.t5.moves.wing_t -= dt;
                self.body.vel.y = (self.body.vel.y + 40.0 * dt).min(6.0);
                self.t5.moves.flying = true;
                self.fall_peak = self.body.pos.y;
                if self.surv.rng.f32() < dt * 25.0 {
                    self.particles.glow(self.body.pos, Vec3::new(0.0, -2.0, 0.0), glow, [1.1, 1.1, 1.1], 0.1, 0.4);
                }
            } else if space && wings {
                let (lift, speed) = worn.map_or((7.5, 1.35), |k| (crate::wings::WINGS[k].lift, crate::wings::WINGS[k].speed));
                if self.t5.moves.wing_t > 0.0 {
                    self.t5.moves.wing_t -= dt;
                    self.body.vel.y = (self.body.vel.y + 45.0 * dt).min(lift);
                    self.t5.moves.flying = true;
                    // Faster through the air than on foot.
                    let base = self.body.speed_override.unwrap_or(WALK_SPEED);
                    self.body.speed_override = Some(base * speed);
                } else if self.body.vel.y < -2.5 {
                    // Out of flight time: glide down.
                    self.body.vel.y = -2.5;
                }
                self.fall_peak = self.body.pos.y;
                if self.surv.rng.f32() < dt * 20.0 {
                    let tint = match (worn, tr_wing) {
                        (Some(k), _) => {
                            let c = crate::wings::WINGS[k].colors[2];
                            [c[0] as f32 / 200.0, c[1] as f32 / 200.0, c[2] as f32 / 200.0]
                        }
                        (None, Some(w)) => w.2,
                        _ => [1.2, 1.2, 1.2],
                    };
                    self.particles.glow(self.body.pos + Vec3::Y * 1.2, Vec3::new(0.0, -1.5, 0.0), glow, tint, 0.08, 0.5);
                }
            }
            // Claws: hold Space against a wall to climb it.
            let gear = self.wears("climbing_gear");
            if space && (gear || self.wears("climbing_claws") || self.wears("shadow_gear")) && self.against_wall() {
                self.body.vel.y = self.body.vel.y.max(if gear { 4.5 } else { 3.5 });
                self.fall_peak = self.body.pos.y;
            }
            // Climbing gear: hold Shift against a wall to slide slowly, tap Space to kick off it.
            if gear && self.against_wall() {
                if sneak && self.body.vel.y < -1.5 {
                    self.body.vel.y = -1.5;
                    self.fall_peak = self.body.pos.y;
                }
                if pressed && self.body.vel.y < 1.0 {
                    let f = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
                    self.body.vel = -f * 7.0 + Vec3::Y * 9.0;
                    self.t5.moves.jumps_used = 0;
                    self.sfx(Sfx::Place(crate::audio::Material::Cloth));
                }
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
        let shield = self.wears("charge_shield") || self.wears("watcher_shield");
        if dash != 0.0 && self.t5.moves.dash_cd <= 0.0 && (shield || self.wears("wind_cape") || self.wears("shadow_gear")) {
            self.t5.moves.dash_cd = 1.0;
            let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
            self.body.vel += right * dash * if shield { 19.0 } else { 16.0 };
            if shield {
                self.t5.moves.charge_t = 0.35;
                self.t5.moves.charge_dir = right * dash;
            }
            for _ in 0..12 {
                self.particles.glow(self.body.pos + Vec3::Y * 0.8, -right * dash * 2.0, glow, [0.9, 1.1, 1.2], 0.1, 0.4);
            }
            self.sfx(Sfx::Place(crate::audio::Material::Cloth));
        }
        self.shield_charge(dt);
    }

    /// The shield charge: monsters it runs into are hit and thrown aside, once each.
    fn shield_charge(&mut self, dt: f32) {
        if self.t5.moves.charge_t <= 0.0 {
            return;
        }
        self.t5.moves.charge_t -= dt;
        let at = self.body.pos + Vec3::Y * 0.9;
        let dir = self.t5.moves.charge_dir;
        let hit: Vec<usize> = self
            .mobs
            .list
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive() && m.body.center().distance(at) < 1.6)
            .map(|(k, _)| k)
            .collect();
        for k in hit {
            let dmg = 6.0 * self.melee_bonus();
            let m = &mut self.mobs.list[k];
            m.hurt(dmg, at - dir);
            m.body.vel += dir * 9.0 + Vec3::Y * 4.0;
            let def = &self.mobs.reg.defs[m.def];
            self.last_hit = Some((def.display_name.clone(), m.health.max(0.0), m.max_health, self.now + 3.0));
            self.sfx(Sfx::Hit(crate::audio::Material::Metal));
            // One bump per charge.
            self.t5.moves.charge_t = 0.0;
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
            let max = self.flight_max();
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
