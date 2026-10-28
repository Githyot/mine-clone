//! Beds: placed two blocks long, slept in lying down (the model is `voxel::bed`).
//!
//! Test hook for screenshots: MC_BED=1 sets every colour of bed out in rows in front of the
//! player, MC_BED=one a single bed side-on; MC_BED=sleep puts the player to sleep in a bed just ahead (pair with --camera 1 or 2
//! to see the body lying on it).

use std::f32::consts::{FRAC_PI_2, PI};

use glam::{IVec3, Vec3};
use voxel::bed;
use voxel::block::{BlockId, Shape, AIR};

use crate::Game;

/// Seconds lying down before the night is skipped.
const FALL_ASLEEP: f32 = 2.5;

/// Lying in a bed.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sleep {
    /// The head part.
    pub head: IVec3,
    pub facing: u8,
    pub t: f32,
    /// Asleep at night: the morning comes when `t` reaches `FALL_ASLEEP`.
    pub night: bool,
}

impl Sleep {
    /// The way the head points, as a unit vector.
    fn dir(&self) -> Vec3 {
        let [x, z] = bed::dir(self.facing);
        Vec3::new(x as f32, 0.0, z as f32)
    }

    /// Where the feet are: on the blanket near the foot end.
    pub fn feet(&self) -> Vec3 {
        self.head.as_vec3() + Vec3::new(0.5, bed::TOP as f32 / 16.0, 0.5) - self.dir() * 1.45
    }

    /// Over the middle of the bed: what the camera circles in third person.
    pub fn middle(&self) -> Vec3 {
        self.head.as_vec3() + Vec3::new(0.5, 1.2, 0.5) - self.dir() * 0.5
    }

    /// The eyes: the head on the pillow, looking up.
    pub fn eye(&self) -> Vec3 {
        self.head.as_vec3() + Vec3::new(0.5, 0.95, 0.5) - self.dir() * 0.12
    }
}

/// The body's turn for a bed facing `facing` (the lying model's head end is its -z).
pub(crate) fn body_yaw(facing: u8) -> f32 {
    -(facing as f32) * FRAC_PI_2
}

impl Game {
    pub(crate) fn is_bed(&self, id: BlockId) -> bool {
        self.reg.get(id).shape == Shape::Bed
    }

    /// The head part of the bed at `p`, and its facing.
    pub(crate) fn bed_head(&self, p: IVec3) -> Option<(IVec3, u8)> {
        let b = self.reg.get(self.world.block_or_air(p));
        if b.shape != Shape::Bed {
            return None;
        }
        let head = if bed::is_head(b.variant) {
            p
        } else {
            let [x, z] = bed::dir(b.variant);
            p + IVec3::new(x, 0, z)
        };
        Some((head, b.variant & 3))
    }

    /// Put a bed down with its foot at `pos`, the head one block further the way `facing` points.
    pub(crate) fn place_bed(&mut self, pos: IVec3, id: BlockId, facing: u8) -> Option<IVec3> {
        let [x, z] = bed::dir(facing);
        let head = pos + IVec3::new(x, 0, z);
        let cur = self.world.block_or_air(head);
        let free = cur == AIR || self.reg.get(cur).shape == Shape::Cross;
        // Both halves need the ground under them.
        let firm = |p: IVec3| self.reg.get(self.world.block_or_air(p - IVec3::Y)).solid;
        if !free || !firm(pos) || !firm(head) || self.body.intersects_block(head) {
            return None;
        }
        if self.world.set_block(pos, self.reg.variant(id, facing)) {
            self.world.set_block(head, self.reg.variant(id, facing | bed::HEAD));
            self.world.mesh_now(head);
            return self.placed(pos, id);
        }
        None
    }

    /// Right click on a bed: the respawn point moves here, and at night the player lies down
    /// to sleep the night away.
    pub(crate) fn use_bed(&mut self, pos: IVec3) {
        let Some((head, facing)) = self.bed_head(pos) else { return };
        if self.sleep.is_some() || self.seat.is_some() || self.riding.is_some() {
            return;
        }
        self.surv.spawn = Some(head + IVec3::Y);
        let night = (12500.0..23500.0).contains(&self.time_of_day);
        if night && self.monsters_near(head.as_vec3()) {
            self.toast("Você não pode dormir agora: há monstros por perto", 3.0);
            return;
        }
        self.sleep = Some(Sleep { head, facing, t: 0.0, night });
        self.body.vel = Vec3::ZERO;
        // Lying on the back, looking up past the headboard.
        self.yaw = body_yaw(facing) + PI;
        self.pitch = 1.1;
        if night {
            self.toast("Boa noite...", 2.0);
        } else {
            self.toast("Ponto de renascimento definido. Só dá para dormir à noite. (Shift levanta)", 3.5);
        }
    }

    fn monsters_near(&self, at: Vec3) -> bool {
        self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].kind == voxel::mob::Kind::Hostile && m.body.pos.distance(at) < 8.0)
    }

    /// While lying in bed: the body stays on it, the night passes. True while in bed.
    pub(crate) fn sleep_step(&mut self, sneak: bool, dt: f32) -> bool {
        let Some(mut s) = self.sleep else { return false };
        let still_there = self.bed_head(s.head).is_some_and(|(h, _)| h == s.head);
        s.t += dt;
        let morning = s.night && s.t >= FALL_ASLEEP;
        if !still_there || (sneak && s.t > 0.3) || morning {
            self.wake_up(&s);
            if morning {
                self.time_of_day = 0.0;
                self.toast("Você dormiu. Bom dia!", 3.0);
                self.prog.event(crate::progression::Event::Token("slept".into()));
            }
            return false;
        }
        self.sleep = Some(s);
        self.body.pos = s.feet();
        self.body.vel = Vec3::ZERO;
        self.body.on_ground = true;
        self.body.fall_distance = 0.0;
        true
    }

    /// Get up, standing on the bed.
    fn wake_up(&mut self, s: &Sleep) {
        self.sleep = None;
        self.body.pos = s.head.as_vec3() + Vec3::new(0.5, bed::TOP as f32 / 16.0 + 0.01, 0.5);
        self.body.vel = Vec3::ZERO;
        self.pitch = 0.0;
    }

    /// The screen darkening while falling asleep (0..1).
    pub(crate) fn sleep_fade(&self) -> f32 {
        match self.sleep {
            Some(s) if s.night => (s.t / FALL_ASLEEP).clamp(0.0, 1.0).powf(1.5),
            _ => 0.0,
        }
    }

    /// The bed a body at `feet` lies in (for other players' bodies): its facing.
    pub(crate) fn bed_under(&self, feet: Vec3) -> Option<u8> {
        let p = (feet - Vec3::Y * 0.2).floor().as_ivec3();
        self.bed_head(p).map(|(_, f)| f)
    }

    /// Test hook MC_BED (see the module docs).
    pub(crate) fn bed_test(&mut self, what: &str) {
        let f = self.forward();
        let fwd = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let facing = if fwd.x.abs() > fwd.z.abs() { if fwd.x > 0.0 { 1 } else { 3 } } else if fwd.z > 0.0 { 2 } else { 0 };
        let [dx, dz] = bed::dir(facing);
        let ahead = IVec3::new(dx, 0, dz);
        let side = IVec3::new(-dz, 0, dx);
        let base = self.body.pos.floor().as_ivec3();
        let stone = self.reg.id_or_air("oak_planks");
        let put = |g: &mut Game, p: IVec3, id: BlockId, facing: u8| {
            g.world.set_block(p, g.reg.variant(id, facing));
            g.world.set_block(p + IVec3::new(bed::dir(facing)[0], 0, bed::dir(facing)[1]), g.reg.variant(id, facing | bed::HEAD));
        };
        // A wooden floor and clear air over it.
        for a in -1..=12 {
            for b in -9..=9 {
                let p = base + ahead * a + side * b;
                self.world.set_block(p - IVec3::Y, stone);
                for y in 0..4 {
                    self.world.set_block(p + IVec3::Y * y, AIR);
                }
            }
        }
        if what == "one" {
            // One bed side-on, a little to the right, for a close look.
            let red = self.reg.id_or_air("red_bed");
            put(self, base + ahead * 3 - side, red, (facing + 1) & 3);
            self.hide_hud = true;
            return;
        }
        if what == "sleep" {
            let red = self.reg.id_or_air("red_bed");
            // Side-on to the camera, a little ahead.
            let foot = base + ahead * 2 + side;
            let turned = (facing + 3) & 3;
            put(self, foot, red, turned);
            self.time_of_day = 18000.0;
            let look = (self.yaw, self.pitch);
            self.use_bed(foot);
            // Keep the camera where --look aimed it.
            (self.yaw, self.pitch) = look;
            if let Some(s) = self.sleep.as_mut() {
                // Hold the picture before the screen goes dark.
                s.night = false;
            }
            self.sleep_step(false, 0.0);
            self.hide_hud = true;
            return;
        }
        const COLORS: [&str; 16] = ["white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black"];
        // Two rows of eight, heads away from the camera, the near row a little closer.
        for (i, c) in COLORS.iter().enumerate() {
            let Some(id) = self.reg.id(&format!("{c}_bed")) else { continue };
            let (row, col) = (i / 8, i % 8);
            let foot = base + ahead * (5 + row as i32 * 4) + side * (col as i32 * 2 - 7);
            put(self, foot, id, facing);
        }
        self.hide_hud = true;
    }
}
