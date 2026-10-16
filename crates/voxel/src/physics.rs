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
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// Wanted horizontal direction in world space (not normalised).
    pub wish: Vec3,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
}

#[derive(Clone, Debug)]
pub struct Body {
    /// Feet position (centre of the bottom face of the box).
    pub pos: Vec3,
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
        Body { pos, vel: Vec3::ZERO, on_ground: false, flying: false, in_water: false, head_in_water: false, fall_distance: 0.0 }
    }

    pub fn eye(&self) -> Vec3 {
        self.pos + Vec3::new(0.0, EYE_HEIGHT, 0.0)
    }

    fn aabb(&self) -> (Vec3, Vec3) {
        let h = PLAYER_WIDTH / 2.0;
        (self.pos - Vec3::new(h, 0.0, h), self.pos + Vec3::new(h, PLAYER_HEIGHT, h))
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
            let speed = if input.sneak { SNEAK_SPEED } else if input.sprint { SPRINT_SPEED } else { WALK_SPEED };
            let target = wish * speed;
            // Quick acceleration on the ground, little control in the air.
            let k = if self.on_ground { (dt * 20.0).min(1.0) } else { (dt * 3.0).min(1.0) };
            self.vel.x += (target.x - self.vel.x) * k;
            self.vel.z += (target.z - self.vel.z) * k;
            self.vel.y = (self.vel.y - GRAVITY * dt).max(-TERMINAL_VELOCITY);
            if input.jump && self.on_ground {
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
            let moved = self.move_axis(world, axis, delta[axis]);
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
        let y = (lo.y - 0.05).floor() as i32;
        for x in lo.x.floor() as i32..=(hi.x - 1e-4).floor() as i32 {
            for z in lo.z.floor() as i32..=(hi.z - 1e-4).floor() as i32 {
                if world.solid(IVec3::new(x, y, z)) {
                    return true;
                }
            }
        }
        false
    }

    /// Move along one axis, stopping at the first solid block. Returns the distance actually moved.
    fn move_axis(&mut self, world: &impl Terrain, axis: usize, d: f32) -> f32 {
        if d == 0.0 {
            return 0.0;
        }
        let (lo, hi) = self.aabb();
        let mut allowed = d;
        let eps = 1e-4;
        let range = |a: usize| -> (i32, i32) { ((lo[a] + eps).floor() as i32, (hi[a] - eps).floor() as i32) };
        let (a1, a2) = match axis {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        let (r1, r2) = (range(a1), range(a2));
        let (start, end) = if d > 0.0 {
            ((hi[axis] - eps).floor() as i32 + 1, (hi[axis] + d).floor() as i32)
        } else {
            ((lo[axis] + eps).floor() as i32 - 1, (lo[axis] + d).floor() as i32)
        };
        let dirn = if d > 0.0 { 1 } else { -1 };
        let mut c = start;
        'outer: loop {
            if (dirn > 0 && c > end) || (dirn < 0 && c < end) {
                break;
            }
            for u in r1.0..=r1.1 {
                for v in r2.0..=r2.1 {
                    let mut p = IVec3::ZERO;
                    p[axis] = c;
                    p[a1] = u;
                    p[a2] = v;
                    if world.solid(p) {
                        allowed = if d > 0.0 { c as f32 - hi[axis] } else { (c + 1) as f32 - lo[axis] };
                        // Never move backwards because of float error.
                        if allowed * d < 0.0 {
                            allowed = 0.0;
                        }
                        break 'outer;
                    }
                }
            }
            c += dirn;
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
