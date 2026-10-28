//! Boats (vanilla idea, our own model): put one on water with a right click, climb in, steer
//! it with the movement keys toward where you look, and get out with sneak. They float on
//! water, crawl on land and break after a few hits, dropping the boat back.

use glam::{IVec3, Mat4, Vec3};
use voxel::item::ItemStack;
use voxel::physics::Terrain;

use crate::Game;

/// Paddling: how hard the oars push, and top speed on water.
const THRUST: f32 = 9.0;
const TOP_SPEED: f32 = 7.0;

#[derive(Clone, Debug)]
pub struct Boat {
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    /// Which wood (the item it drops back).
    pub item: String,
    pub damage: f32,
    pub hurt: f32,
}

impl Boat {
    pub fn new(pos: Vec3, yaw: f32, item: &str) -> Boat {
        Boat { pos, vel: Vec3::ZERO, yaw, item: item.to_string(), damage: 0.0, hurt: 1.0 }
    }

    fn aabb(&self) -> (Vec3, Vec3) {
        (self.pos - Vec3::new(0.7, 0.0, 0.7), self.pos + Vec3::new(0.7, 0.6, 0.7))
    }
}

impl Game {
    fn water_at(&self, p: IVec3) -> bool {
        let id = self.world.block_or_air(p);
        self.reg.id("water").is_some_and(|w| self.reg.get(id).base == w)
    }

    /// Room for a boat's hull at `p` (its middle), solid blocks only.
    fn boat_fits(&self, p: Vec3) -> bool {
        for (dx, dz) in [(-0.6, -0.6), (0.6, -0.6), (-0.6, 0.6), (0.6, 0.6), (0.0, 0.0)] {
            for dy in [0.1, 0.5] {
                if self.world.solid((p + Vec3::new(dx, dy, dz)).floor().as_ivec3()) {
                    return false;
                }
            }
        }
        true
    }

    /// The nearest boat the crosshair is on, nearer than the block it points at.
    fn boat_in_sight(&self) -> Option<usize> {
        let eye = self.body.eye();
        let f = self.forward();
        let block = self.target.map(|t| t.distance).unwrap_or(f32::MAX);
        let mut best: Option<(usize, f32)> = None;
        for (i, b) in self.surv.boats.iter().enumerate() {
            if self.boating == Some(i) {
                continue;
            }
            let (lo, hi) = b.aabb();
            if let Some(t) = crate::mobs::ray_box(eye, f, lo, hi) {
                if t < self.reach() && t < block + 0.3 && best.map_or(true, |(_, bt)| t < bt) {
                    best = Some((i, t));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// Right click: get into a boat, or put one down on the water (or ground) you look at.
    pub(crate) fn boat_use(&mut self) -> bool {
        if let Some(i) = self.boat_in_sight() {
            if self.boating.is_none() && self.riding.is_none() && self.crouch <= 0.5 {
                self.boating = Some(i);
                self.body.vel = Vec3::ZERO;
                return true;
            }
        }
        let Some(held) = self.held_name().filter(|n| n.ends_with("_boat")) else { return false };
        // Along the look ray, the first water surface or block top.
        let eye = self.body.eye();
        let f = self.forward();
        let mut at = None;
        for k in 0..40 {
            let p = eye + f * (k as f32 * 0.125);
            let c = p.floor().as_ivec3();
            if self.water_at(c) {
                at = Some(Vec3::new(p.x, c.y as f32 + 0.95, p.z));
                break;
            }
            if self.world.solid(c) {
                if let Some(t) = self.target.filter(|t| t.normal == IVec3::Y) {
                    at = Some(t.block.as_vec3() + Vec3::new(0.5, 1.0, 0.5));
                }
                break;
            }
        }
        let Some(at) = at else { return false };
        self.surv.boats.push(Boat::new(at, f.x.atan2(f.z), &held));
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.anim.start_swing();
        self.sfx(crate::audio::Sfx::Place(crate::audio::Material::Wood));
        true
    }

    /// Left click on a boat: a few hits break it (one in creative).
    pub(crate) fn boat_attack(&mut self) -> bool {
        let Some(i) = self.boat_in_sight() else { return false };
        let b = &mut self.surv.boats[i];
        b.damage += if self.survival { 4.0 } else { 40.0 };
        b.hurt = 0.0;
        let push = (b.pos - self.body.eye()) * Vec3::new(1.0, 0.0, 1.0);
        b.vel += push.normalize_or_zero() * 1.5;
        let (broken, at, item) = (b.damage >= 10.0, b.pos + Vec3::Y * 0.3, b.item.clone());
        self.anim.start_swing();
        self.sfx(crate::audio::Sfx::Hit(crate::audio::Material::Wood));
        if broken {
            self.surv.boats.remove(i);
            self.boating = match self.boating {
                Some(r) if r == i => None,
                Some(r) if r > i => Some(r - 1),
                r => r,
            };
            if self.survival {
                if let Some(it) = self.items.id(&item) {
                    self.spawn_drop(at, ItemStack::new(it, 1), Vec3::Y * 2.0);
                }
            }
        }
        true
    }

    /// The player in a boat: paddles toward where they look; sneak climbs out onto the
    /// nearest dry block. True while boating.
    pub(crate) fn boat_step(&mut self, wish: Vec3, sneak: bool, dt: f32) -> bool {
        let Some(i) = self.boating else { return false };
        if i >= self.surv.boats.len() {
            self.boating = None;
            return false;
        }
        let p = self.surv.boats[i].pos;
        if sneak {
            self.boating = None;
            for d in [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z, Vec3::X * 2.0, Vec3::NEG_X * 2.0, Vec3::Z * 2.0, Vec3::NEG_Z * 2.0] {
                let q = (p + d).floor().as_ivec3();
                for dy in 0..2 {
                    let feet = q + IVec3::Y * dy;
                    if self.world.solid(feet - IVec3::Y) && !self.world.solid(feet) && !self.world.solid(feet + IVec3::Y) {
                        self.body.pos = feet.as_vec3() + Vec3::new(0.5, 0.05, 0.5);
                        self.body.vel = Vec3::ZERO;
                        return false;
                    }
                }
            }
            self.body.pos = p + Vec3::Y * 0.8;
            return false;
        }
        let f = self.forward();
        let b = &mut self.surv.boats[i];
        // The bow swings round toward the view, the oars push along it.
        let want = f.x.atan2(f.z);
        let mut d = want - b.yaw;
        while d > std::f32::consts::PI {
            d -= std::f32::consts::TAU;
        }
        while d < -std::f32::consts::PI {
            d += std::f32::consts::TAU;
        }
        b.yaw += d * (dt * 4.0).min(1.0);
        if wish.length_squared() > 0.01 {
            b.vel += wish.normalize() * THRUST * dt;
        }
        self.body.pos = b.pos + Vec3::Y * 0.15;
        self.body.vel = b.vel;
        self.body.on_ground = true;
        self.body.fall_distance = 0.0;
        true
    }

    /// Float, drift and crawl every boat along.
    pub(crate) fn update_boats(&mut self, dt: f32) {
        // Test hook: a few boats on the water in front, the player in the first.
        if std::env::var("MC_BOAT").is_ok() && self.surv.boats.is_empty() {
            let me = self.body.pos;
            for k in 0..3 {
                let p = me + Vec3::new(k as f32 * 2.0 - 2.0, 0.0, -4.0 - k as f32);
                let c = p.floor().as_ivec3();
                let y = (0..40).map(|d| c.y - d).find(|&y| self.water_at(IVec3::new(c.x, y, c.z))).unwrap_or(c.y);
                self.surv.boats.push(Boat::new(Vec3::new(p.x, y as f32 + 0.95, p.z), 0.6 * k as f32, ["oak_boat", "birch_boat", "cherry_boat"][k]));
            }
            self.boating = Some(0);
        }
        if self.surv.boats.is_empty() || dt == 0.0 {
            return;
        }
        let mut boats = std::mem::take(&mut self.surv.boats);
        for (i, b) in boats.iter_mut().enumerate() {
            b.hurt += dt;
            b.damage = (b.damage - dt * 2.0).max(0.0);
            let c = b.pos.floor().as_ivec3();
            if self.world.block(c).is_none() {
                continue;
            }
            let rider = self.boating == Some(i);
            let (here, below) = (self.water_at(c), self.water_at(c - IVec3::Y));
            let floating = here || (below && b.pos.y - c.y as f32 <= 0.3);
            if floating {
                // Ride on the surface: the top of the highest water block in this column.
                let mut top = if here { c } else { c - IVec3::Y };
                while self.water_at(top + IVec3::Y) {
                    top += IVec3::Y;
                }
                let surface = top.y as f32 + 0.95;
                b.vel.y = (surface - b.pos.y) * 6.0;
                let drag = (1.0 - dt * if rider { 0.9 } else { 1.6 }).max(0.0);
                b.vel.x *= drag;
                b.vel.z *= drag;
                let flat = Vec3::new(b.vel.x, 0.0, b.vel.z);
                if flat.length() > TOP_SPEED {
                    let v = flat.normalize() * TOP_SPEED;
                    b.vel.x = v.x;
                    b.vel.z = v.z;
                }
            } else {
                b.vel.y -= 20.0 * dt;
                // On land it drags along the ground, and glides fast over ice like the classic
                // ice-boat roads.
                let g = self.world.ground((b.pos - Vec3::Y * 0.05).floor().as_ivec3());
                let rate = if g.slippery() { g.grip * 0.15 } else { 8.0 / g.speed };
                let drag = (1.0 - dt * rate).max(0.0);
                b.vel.x *= drag;
                b.vel.z *= drag;
                let flat = Vec3::new(b.vel.x, 0.0, b.vel.z);
                let top = TOP_SPEED * 2.5 * g.speed;
                if flat.length() > top {
                    let v = flat.normalize() * top;
                    b.vel.x = v.x;
                    b.vel.z = v.z;
                }
            }
            // Move one axis at a time so it slides along banks.
            for axis in 0..3 {
                let mut step = Vec3::ZERO;
                step[axis] = b.vel[axis] * dt;
                let next = b.pos + step;
                if self.boat_fits(next) {
                    b.pos = next;
                } else {
                    b.vel[axis] = 0.0;
                }
            }
        }
        // Boats bump into each other.
        for a in 0..boats.len() {
            for c in a + 1..boats.len() {
                let d = (boats[c].pos - boats[a].pos) * Vec3::new(1.0, 0.0, 1.0);
                if d.length() < 1.4 && d.length() > 1e-4 {
                    let n = d.normalize();
                    boats[a].pos -= n * 0.02;
                    boats[c].pos += n * 0.02;
                }
            }
        }
        self.surv.boats = boats;
    }

    /// Boat models for the entity renderer.
    pub(crate) fn boat_instances(&self, cam_block: IVec3) -> Vec<(usize, crate::entity::Bones)> {
        let Some(def) = self.mobs.reg.id("boat") else { return Vec::new() };
        self.surv
            .boats
            .iter()
            .map(|b| {
                let l = self.world.light((b.pos + Vec3::Y * 0.5).floor().as_ivec3());
                let mut bones = crate::entity::Bones { light: [(l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0, if b.hurt < 0.25 { 1.0 } else { 0.0 }, 0.0], ..Default::default() };
                let m = Mat4::from_translation(b.pos - Vec3::Y * 0.2 - cam_block.as_vec3()) * Mat4::from_rotation_y(b.yaw) * Mat4::from_scale(Vec3::splat(1.0 / 16.0));
                for mm in bones.m.iter_mut() {
                    *mm = m.to_cols_array_2d();
                }
                (def, bones)
            })
            .collect()
    }
}
