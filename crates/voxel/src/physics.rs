//! Player movement: axis-by-axis collision against block boxes, gravity, jumping, flying and swimming.

use glam::{IVec3, Vec3};

pub const PLAYER_WIDTH: f32 = 0.6;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const EYE_HEIGHT: f32 = 1.62;
pub const GRAVITY: f32 = 32.0;
pub const JUMP_SPEED: f32 = 9.0;
pub const WALK_SPEED: f32 = 4.317;
pub const SPRINT_SPEED: f32 = 5.612;
pub const SNEAK_SPEED: f32 = 1.31;
pub const FLY_SPEED: f32 = 10.9;
pub const SWIM_SPEED: f32 = 2.2;
pub const TERMINAL_VELOCITY: f32 = 78.4;

/// What the physics needs to know about the world.
pub trait Terrain {
    /// Entities collide with this block.
    fn solid(&self, p: IVec3) -> bool;
    /// The block is water (or another swimmable liquid).
    fn liquid(&self, p: IVec3) -> bool;
    /// Height of the solid part from the bottom of the cell (slabs and beds are lower).
    fn top(&self, _p: IVec3) -> f32 {
        1.0
    }
    /// Collision boxes of the cell, in block units relative to its corner (stairs and fences
    /// have several, fences reach 1.5 high). By default one box up to `top`.
    fn boxes(&self, p: IVec3, out: &mut Vec<[f32; 6]>) {
        if self.solid(p) {
            out.push([0.0, 0.0, 0.0, 1.0, self.top(p), 1.0]);
        }
    }
}

/// World-space boxes of every cell overlapping the region (cells one lower too, for fences).
fn boxes_in(world: &impl Terrain, lo: Vec3, hi: Vec3) -> Vec<[f32; 6]> {
    let mut out = Vec::new();
    let mut cell = Vec::new();
    let (a, b) = (lo.floor().as_ivec3() - IVec3::Y, hi.floor().as_ivec3());
    for x in a.x..=b.x {
        for y in a.y..=b.y {
            for z in a.z..=b.z {
                let p = IVec3::new(x, y, z);
                cell.clear();
                world.boxes(p, &mut cell);
                let o = p.as_vec3();
                for c in &cell {
                    out.push([c[0] + o.x, c[1] + o.y, c[2] + o.z, c[3] + o.x, c[4] + o.y, c[5] + o.z]);
                }
            }
        }
    }
    out
}

/// How high a walking body climbs onto a low block (slab, bed) without jumping.
const STEP_HEIGHT: f32 = 0.6;

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// Wanted horizontal direction in world space (not normalised).
    pub wish: Vec3,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
    /// Lying down: slow, no jumping, but no edge guard either.
    pub crawl: bool,
}

#[derive(Clone, Debug)]
pub struct Body {
    /// Feet position (centre of the bottom face of the box).
    pub pos: Vec3,
    pub half_width: f32,
    pub height: f32,
    /// Horizontal speed used instead of the player constants when set (mobs).
    pub speed_override: Option<f32>,
    /// Entities that can't jump out of water (and don't jump).
    pub no_jump: bool,
    pub vel: Vec3,
    pub on_ground: bool,
    pub flying: bool,
    pub in_water: bool,
    pub head_in_water: bool,
    /// Distance fallen since last on the ground, for fall damage later.
    pub fall_distance: f32,
}

impl Body {
    pub fn new(pos: Vec3) -> Self {
        Self::sized(pos, PLAYER_WIDTH, PLAYER_HEIGHT)
    }

    pub fn sized(pos: Vec3, width: f32, height: f32) -> Self {
        Body {
            pos,
            half_width: width / 2.0,
            height,
            speed_override: None,
            no_jump: false,
            vel: Vec3::ZERO,
            on_ground: false,
            flying: false,
            in_water: false,
            head_in_water: false,
            fall_distance: 0.0,
        }
    }

    pub fn eye(&self) -> Vec3 {
        self.pos + Vec3::new(0.0, if self.height > 1.7 { EYE_HEIGHT } else { self.height * 0.85 }, 0.0)
    }

    pub fn center(&self) -> Vec3 {
        self.pos + Vec3::new(0.0, self.height / 2.0, 0.0)
    }

    pub fn aabb(&self) -> (Vec3, Vec3) {
        let h = self.half_width;
        (self.pos - Vec3::new(h, 0.0, h), self.pos + Vec3::new(h, self.height, h))
    }

    /// Push the body away (hit by something).
    pub fn knockback(&mut self, dir: Vec3, strength: f32) {
        let d = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
        self.vel += d * strength + Vec3::new(0.0, strength * 0.6, 0.0);
    }

    /// True if a box of this body's width and the given height fits where the body stands.
    pub fn fits(&self, world: &impl Terrain, height: f32) -> bool {
        let h = self.half_width - 0.001;
        let lo = self.pos - Vec3::new(h, -0.001, h);
        let hi = self.pos + Vec3::new(h, height - 0.001, h);
        !boxes_in(world, lo, hi).iter().any(|b| lo.x < b[3] && hi.x > b[0] && lo.y < b[4] && hi.y > b[1] && lo.z < b[5] && hi.z > b[2])
    }

    /// True if the player's box overlaps the given block cell.
    pub fn intersects_block(&self, b: IVec3) -> bool {
        let (lo, hi) = self.aabb();
        let bl = b.as_vec3();
        lo.x < bl.x + 1.0 && hi.x > bl.x && lo.y < bl.y + 1.0 && hi.y > bl.y && lo.z < bl.z + 1.0 && hi.z > bl.z
    }

    pub fn step(&mut self, world: &impl Terrain, input: &Input, dt: f32) {
        let dt = dt.min(0.05);
        let feet = self.pos.floor().as_ivec3();
        self.in_water = world.liquid(feet) || world.liquid(feet + IVec3::Y);
        self.head_in_water = world.liquid(self.eye().floor().as_ivec3());
        let mut wish = Vec3::new(input.wish.x, 0.0, input.wish.z);
        if wish.length_squared() > 1.0 {
            wish = wish.normalize();
        }

        if self.flying {
            let speed = if input.sprint { FLY_SPEED * 2.0 } else { FLY_SPEED };
            let target = wish * speed;
            let k = (dt * 10.0).min(1.0);
            self.vel.x += (target.x - self.vel.x) * k;
            self.vel.z += (target.z - self.vel.z) * k;
            let vy = if input.jump { speed * 0.75 } else if input.sneak { -speed * 0.75 } else { 0.0 };
            self.vel.y += (vy - self.vel.y) * k;
        } else if self.in_water {
            let target = wish * SWIM_SPEED * if input.sprint { 1.5 } else { 1.0 };
            let k = (dt * 6.0).min(1.0);
            self.vel.x += (target.x - self.vel.x) * k;
            self.vel.z += (target.z - self.vel.z) * k;
            self.vel.y -= GRAVITY * 0.15 * dt;
            if input.jump {
                self.vel.y = (self.vel.y + 20.0 * dt).min(3.5);
            }
            self.vel.y *= 1.0 - (dt * 2.0).min(1.0);
            self.fall_distance = 0.0;
        } else {
            let base = self.speed_override.unwrap_or(WALK_SPEED);
            let speed = if input.crawl { base * 0.28 } else if input.sneak { base * 0.3 } else if input.sprint { base * 1.3 } else { base };
            let target = wish * speed;
            // Quick acceleration on the ground, little control in the air.
            let k = if self.on_ground { (dt * 20.0).min(1.0) } else { (dt * 3.0).min(1.0) };
            self.vel.x += (target.x - self.vel.x) * k;
            self.vel.z += (target.z - self.vel.z) * k;
            self.vel.y = (self.vel.y - GRAVITY * dt).max(-TERMINAL_VELOCITY);
            if input.jump && self.on_ground && !self.no_jump && !input.crawl {
                self.vel.y = JUMP_SPEED;
                if input.sprint {
                    self.vel += wish * 1.0;
                }
            }
        }

        let delta = self.vel * dt;
        let was_on_ground = self.on_ground;
        self.on_ground = false;
        // Sneaking keeps you from walking off edges.
        let sneak_guard = input.sneak && was_on_ground && !self.flying;
        for axis in [1usize, 0, 2] {
            let before = self.pos;
            let mut moved = self.move_axis(world, axis, delta[axis]);
            // Walk up onto low blocks: lift, move, settle back down.
            if axis != 1 && was_on_ground && !self.flying && moved.abs() + 1e-6 < delta[axis].abs() {
                let blocked = self.pos;
                let up = self.move_axis(world, 1, STEP_HEIGHT);
                let side = self.move_axis(world, axis, delta[axis] - moved);
                let down = self.move_axis(world, 1, -up);
                if side.abs() > 1e-4 && up + down > 1e-4 {
                    moved += side;
                } else {
                    self.pos = blocked;
                }
            }
            if sneak_guard && axis != 1 && !self.has_ground_below(world) {
                self.pos = before;
                self.vel[axis] = 0.0;
            } else if moved.abs() + 1e-6 < delta[axis].abs() {
                if axis == 1 && delta[1] < 0.0 {
                    self.on_ground = true;
                    if self.flying {
                        self.flying = false;
                    }
                }
                self.vel[axis] = 0.0;
            }
        }
        if self.on_ground || self.in_water {
            self.fall_distance = 0.0;
        } else if delta.y < 0.0 {
            self.fall_distance -= delta.y;
        }
    }

    fn has_ground_below(&self, world: &impl Terrain) -> bool {
        let (lo, hi) = self.aabb();
        let probe_lo = Vec3::new(lo.x, lo.y - 0.6, lo.z);
        let probe_hi = Vec3::new(hi.x, lo.y, hi.z);
        boxes_in(world, probe_lo, probe_hi).iter().any(|b| lo.x < b[3] && hi.x > b[0] && lo.z < b[5] && hi.z > b[2] && b[4] >= lo.y - 0.6 && b[4] <= lo.y + 1e-3)
    }

    /// Move along one axis, stopping at the first box in the way. Returns the distance moved.
    fn move_axis(&mut self, world: &impl Terrain, axis: usize, d: f32) -> f32 {
        if d == 0.0 {
            return 0.0;
        }
        let (lo, hi) = self.aabb();
        let eps = 1e-4;
        let mut slo = lo;
        let mut shi = hi;
        if d > 0.0 {
            shi[axis] += d;
        } else {
            slo[axis] += d;
        }
        let mut allowed = d;
        for b in boxes_in(world, slo, shi) {
            // Overlap on the other two axes.
            let other = (0..3).filter(|&a| a != axis).all(|a| lo[a] < b[a + 3] - eps && hi[a] > b[a] + eps);
            if !other {
                continue;
            }
            if d > 0.0 && b[axis] >= hi[axis] - eps {
                allowed = allowed.min(b[axis] - hi[axis]);
            } else if d < 0.0 && b[axis + 3] <= lo[axis] + eps {
                allowed = allowed.max(b[axis + 3] - lo[axis]);
            }
        }
        // Never move backwards because of float error.
        if allowed * d < 0.0 {
            allowed = 0.0;
        }
        self.pos[axis] += allowed;
        allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Flat;
    impl Terrain for Flat {
        fn solid(&self, p: IVec3) -> bool {
            p.y < 10 || (p.x == 3 && p.y < 12)
        }
        fn liquid(&self, _: IVec3) -> bool {
            false
        }
    }

    #[test]
    fn falls_and_lands() {
        let mut b = Body::new(Vec3::new(0.5, 20.0, 0.5));
        for _ in 0..200 {
            b.step(&Flat, &Input::default(), 0.02);
        }
        assert!(b.on_ground);
        assert!((b.pos.y - 10.0).abs() < 1e-3, "{}", b.pos.y);
    }

    #[test]
    fn walls_stop_movement() {
        let mut b = Body::new(Vec3::new(0.5, 10.0, 0.5));
        let input = Input { wish: Vec3::X, ..Default::default() };
        for _ in 0..200 {
            b.step(&Flat, &input, 0.02);
        }
        assert!((b.pos.x - (3.0 - PLAYER_WIDTH / 2.0)).abs() < 1e-3, "{}", b.pos.x);
    }
}
