//! The deep dark (Phase 6): sculk that soaks up the deaths around a catalyst, sensors that
//! hear vibrations (steps, blocks broken, mobs moving) and give a redstone signal, shriekers
//! that warn and, at the fourth warning, call up the warden: blind, slow to anger, with a
//! sonic boom that goes through armor and the darkness it spreads around it.
//!
//! Sneaking makes no vibrations. Wool blocks between a source and a sensor muffle it.

use glam::{IVec3, Vec3};
use voxel::block::AIR;
use voxel::physics::Terrain;

use crate::audio::Sfx;
use crate::effects::DARKNESS;
use crate::progression::{DamageKind, Event};

/// Vibration kinds (a calibrated sensor listens for one; the comparator reads it).
pub const STEP: u8 = 1;
pub const JUMP: u8 = 2;
pub const BLOCK: u8 = 13;
pub const MOB: u8 = 3;

impl crate::Game {
    /// A vibration at `at`: sensors in range pick it up after a moment (sound travels).
    pub(crate) fn vibrate(&mut self, at: Vec3, kind: u8) {
        if self.prog.near.is_empty() {
            return;
        }
        self.prog.vibrations.push((at, kind, 0.0));
    }

    pub(crate) fn update_deepdark(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let sneaking = self.crouch > 0.5;
        // The player's own steps and jumps.
        let p = self.body.pos;
        if self.body.on_ground && !self.body.flying && self.survival {
            let moved = Vec3::new(p.x - self.prog.dd_last.x, 0.0, p.z - self.prog.dd_last.z).length();
            if moved < 5.0 {
                self.prog.dd_walk += moved;
            }
            if self.prog.dd_walk > 1.4 {
                self.prog.dd_walk = 0.0;
                if sneaking {
                    if !self.near_of(&["sculk_sensor", "calibrated_sculk_sensor", "sculk_shrieker"]).iter().all(|s| (s.as_vec3() - p).length() > 8.0) {
                        self.prog.event(Event::Token("sneak_sensor".into()));
                    }
                } else {
                    self.vibrate(p, STEP);
                }
            }
            if !self.prog.prev_ground_dd && !sneaking {
                self.vibrate(p, JUMP);
            }
            // Stepping on a shrieker sets it off.
            let under = (p - Vec3::Y * 0.1).floor().as_ivec3();
            if self.reg.get(self.world.block_or_air(under)).name == "sculk_shrieker" && !sneaking {
                self.shriek(under);
            }
        }
        self.prog.prev_ground_dd = self.body.on_ground;
        self.prog.dd_last = p;
        // Mobs moving make vibrations too (the warden doesn't count itself).
        self.prog.mob_vib_t -= dt;
        if self.prog.mob_vib_t <= 0.0 {
            self.prog.mob_vib_t = 1.0;
            let warden = self.mobs.reg.id("warden");
            let moving: Vec<Vec3> = self.mobs.list.iter().filter(|m| m.alive() && Some(m.def) != warden && Vec3::new(m.body.vel.x, 0.0, m.body.vel.z).length() > 0.5 && (m.body.pos - p).length() < 24.0).map(|m| m.body.pos).take(8).collect();
            for at in moving {
                self.vibrate(at, MOB);
            }
        }
        self.update_sensors(dt);
        self.update_wardens(dt);
        // Warnings wear off: one level every ten minutes.
        self.prog.shriek_cd = (self.prog.shriek_cd - dt).max(0.0);
        if self.prog.warning > 0 {
            self.prog.warning_t += dt;
            if self.prog.warning_t > 600.0 {
                self.prog.warning_t = 0.0;
                self.prog.warning -= 1;
            }
        }
    }

    /// Is a wool block on the straight line between two points? (It muffles vibrations.)
    fn muffled(&self, a: Vec3, b: Vec3) -> bool {
        let d = b - a;
        let n = (d.length() * 2.0) as i32;
        (1..n).any(|k| {
            let q = (a + d * (k as f32 / n as f32)).floor().as_ivec3();
            self.reg.get(self.world.block_or_air(q)).name.ends_with("_wool")
        })
    }

    fn update_sensors(&mut self, dt: f32) {
        // Vibrations reach sensors after distance / 20 seconds, like the 1-tick-per-block travel.
        let sensors = self.near_of(&["sculk_sensor", "calibrated_sculk_sensor"]);
        let vibs = std::mem::take(&mut self.prog.vibrations);
        let mut keep = Vec::new();
        for (at, kind, mut t) in vibs {
            t += dt;
            let mut waiting = false;
            for &s in &sensors {
                let calibrated = self.reg.get(self.world.block_or_air(s)).name.starts_with("calibrated");
                let range = if calibrated { 16.0 } else { 8.0 };
                let c = s.as_vec3() + Vec3::splat(0.5);
                let d = (c - at).length();
                if d > range || self.prog.sensor_on.iter().any(|(q, _)| *q == s) {
                    continue;
                }
                if t < d / 20.0 {
                    waiting = true;
                    continue;
                }
                if self.muffled(at + Vec3::Y * 0.5, c) {
                    continue;
                }
                self.sensor_fire(s, calibrated, kind, at);
            }
            if waiting && t < 1.0 {
                keep.push((at, kind, t));
            }
        }
        self.prog.vibrations = keep;
        // Active sensors go quiet again.
        let mut i = 0;
        while i < self.prog.sensor_on.len() {
            self.prog.sensor_on[i].1 -= dt;
            if self.prog.sensor_on[i].1 <= 0.0 {
                let (s, _) = self.prog.sensor_on.swap_remove(i);
                let name = self.reg.get(self.world.block_or_air(s)).name.clone();
                if let Some(base) = name.strip_suffix("_active") {
                    let id = self.reg.id_or_air(base);
                    if id != AIR {
                        self.world.set_block(s, id);
                        self.redstone.touch(s);
                    }
                }
                continue;
            }
            i += 1;
        }
    }

    fn sensor_fire(&mut self, s: IVec3, calibrated: bool, kind: u8, from: Vec3) {
        let base = if calibrated { "calibrated_sculk_sensor" } else { "sculk_sensor" };
        let on = self.reg.id_or_air(&format!("{base}_active"));
        if on != AIR {
            self.world.set_block(s, on);
            self.redstone.touch(s);
        }
        self.prog.sensor_on.push((s, if calibrated { 0.5 } else { 1.5 }));
        self.prog.sensor_kind.insert(s, kind);
        let glow = self.glow_layer();
        let c = s.as_vec3() + Vec3::new(0.5, 0.6, 0.5);
        // A little stream of blue from the source to the sensor.
        let d = c - from;
        for k in 0..6 {
            self.particles.glow(from + d * (k as f32 / 6.0) + Vec3::Y * 0.4, Vec3::Y * 0.2, glow, [0.2, 0.9, 1.1], 0.06, 0.5);
        }
        if (c - self.body.pos).length() < 16.0 {
            self.sfx(Sfx::Hit(crate::audio::Material::Cloth));
        }
        // A sensor that heard the player wakes the shriekers around it.
        let by_player = (from - self.body.pos).length() < 2.0;
        if by_player {
            let shriekers: Vec<IVec3> = self.near_of(&["sculk_shrieker"]).into_iter().filter(|q| (q.as_vec3() - c).length() <= 8.0).collect();
            if let Some(&q) = shriekers.first() {
                self.shriek(q);
            }
        }
    }

    /// A shrieker goes off: darkness, one more warning, and at four the warden comes.
    pub(crate) fn shriek(&mut self, at: IVec3) {
        if self.prog.shriek_cd > 0.0 || !self.survival {
            return;
        }
        self.prog.shriek_cd = 10.0;
        self.prog.warning = (self.prog.warning + 1).min(4);
        self.prog.warning_t = 0.0;
        self.prog.event(Event::Shrieked);
        let glow = self.glow_layer();
        let c = at.as_vec3() + Vec3::new(0.5, 1.0, 0.5);
        for k in 0..8 {
            self.particles.glow(c, Vec3::Y * (1.0 + k as f32 * 0.4), glow, [0.3, 0.8, 0.9], 0.15, 1.2);
        }
        // A low groan, not a scream.
        self.sfx(Sfx::Cast(3));
        self.add_effect(DARKNESS, 0, 12.0, false);
        if self.prog.warning >= 4 {
            let wardens = self.mobs.list.iter().filter(|m| m.alive() && self.mobs.reg.defs[m.def].name == "warden" && (m.body.pos - c).length() < 48.0).count();
            if wardens == 0 {
                if let Some(spot) = self.warden_spot(at) {
                    if self.mobs.spawn("warden", spot) {
                        self.prog.event(Event::WardenSpawned);
                        self.prog.warning = 0;
                        self.prog.warden_away = 0.0;
                        self.toast("O Warden emerge...", 3.0);
                        if let Some(m) = self.mobs.list.last_mut() {
                            m.aggro = true;
                        }
                    }
                }
            }
        }
    }

    fn warden_spot(&mut self, at: IVec3) -> Option<Vec3> {
        for _ in 0..20 {
            let dx = self.prog.rng.range(-5, 6);
            let dz = self.prog.rng.range(-5, 6);
            for dy in -3..=3 {
                let q = at + IVec3::new(dx, dy, dz);
                let free = (0..3).all(|k| !Terrain::solid(&self.world, q + IVec3::Y * k));
                if free && Terrain::solid(&self.world, q - IVec3::Y) {
                    return Some(q.as_vec3() + Vec3::new(0.5, 0.0, 0.5));
                }
            }
        }
        None
    }

    /// The warden: darkness around it, a sonic boom at range, and it digs back down when the
    /// player has been gone a while.
    fn update_wardens(&mut self, dt: f32) {
        let Some(wi) = self.mobs.reg.id("warden") else { return };
        let me = self.body.pos;
        let eye = self.body.eye();
        self.prog.boom_t = (self.prog.boom_t - dt).max(0.0);
        self.prog.dark_t -= dt;
        let mut nearest: Option<(usize, f32)> = None;
        for (i, m) in self.mobs.list.iter().enumerate() {
            if m.def == wi && m.alive() {
                let d = (m.body.pos - me).length();
                if nearest.is_none_or(|n| d < n.1) {
                    nearest = Some((i, d));
                }
            }
        }
        let Some((i, d)) = nearest else { return };
        if d < 20.0 && self.prog.dark_t <= 0.0 {
            self.prog.dark_t = 6.0;
            self.add_effect(DARKNESS, 0, 13.0, false);
        }
        // The heartbeat: a deep thump that quickens as it closes in.
        self.prog.heartbeat_t -= dt;
        if d < 24.0 && self.prog.heartbeat_t <= 0.0 {
            self.prog.heartbeat_t = 0.6 + d / 12.0;
            self.sfx(Sfx::Hit(crate::audio::Material::Wood));
        }
        let creative = !self.survival;
        if !creative && d > 4.0 && d < 15.0 && self.prog.boom_t <= 0.0 {
            self.prog.boom_t = 5.0;
            let from = self.mobs.list[i].body.pos + Vec3::Y * 1.8;
            let dir = eye - from;
            let glow = self.glow_layer();
            let n = (dir.length() * 1.5) as i32;
            for k in 0..n {
                self.particles.glow(from + dir * (k as f32 / n as f32), Vec3::ZERO, glow, [0.3, 0.9, 1.0], 0.25, 0.4);
            }
            self.sfx(Sfx::Explode);
            // Straight through armor.
            self.prog.next_kind = DamageKind::Magic;
            self.damage(10.0, None);
            self.body.vel += dir.normalize_or_zero() * 6.0 + Vec3::Y * 3.0;
        }
        if d > 40.0 {
            self.prog.warden_away += dt;
            if self.prog.warden_away > 60.0 {
                self.mobs.list.swap_remove(i);
                self.prog.warden_away = 0.0;
            }
        } else {
            self.prog.warden_away = 0.0;
        }
    }

    /// A mob died: a sculk catalyst within 8 blocks spreads sculk where it fell.
    pub(crate) fn catalyst_feed(&mut self, at: Vec3, xp: u8) {
        let cats = self.near_of(&["sculk_catalyst"]);
        if !cats.iter().any(|c| (c.as_vec3() - at).length() <= 8.0) {
            return;
        }
        let sculk = self.reg.id_or_air("sculk");
        if sculk == AIR {
            return;
        }
        self.prog.event(Event::Token("catalyst_kill".into()));
        let sensor = self.reg.id_or_air("sculk_sensor");
        let shrieker = self.reg.id_or_air("sculk_shrieker");
        let charge = (xp as i32).max(1) * 2;
        let base = at.floor().as_ivec3();
        let glow = self.glow_layer();
        for _ in 0..charge {
            let q = base + IVec3::new(self.prog.rng.range(-3, 4), self.prog.rng.range(-2, 1), self.prog.rng.range(-3, 4));
            let b = self.world.block_or_air(q);
            let up = self.world.block_or_air(q + IVec3::Y);
            if b == AIR || b == sculk || up != AIR || self.world.is_liquid(b) || !Terrain::solid(&self.world, q) {
                continue;
            }
            let hard = self.reg.get(b).hardness;
            if hard < 0.0 || hard > 10.0 {
                continue;
            }
            self.world.set_block(q, sculk);
            let r = self.prog.rng.f32();
            if r < 0.06 && sensor != AIR {
                self.world.set_block(q + IVec3::Y, sensor);
            } else if r < 0.075 && shrieker != AIR {
                self.world.set_block(q + IVec3::Y, shrieker);
            }
            self.particles.glow(q.as_vec3() + Vec3::new(0.5, 1.05, 0.5), Vec3::Y * 0.3, glow, [0.1, 0.6, 0.8], 0.08, 1.0);
        }
        for c in cats {
            if (c.as_vec3() - at).length() <= 8.0 {
                self.particles.glow(c.as_vec3() + Vec3::new(0.5, 1.1, 0.5), Vec3::Y * 0.8, glow, [0.3, 1.0, 1.0], 0.2, 1.0);
            }
        }
    }
}
