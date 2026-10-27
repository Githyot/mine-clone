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
    /// Ladders and vines: a body inside climbs instead of falling.
    fn climbable(&self, _p: IVec3) -> bool {
        false
    }
}

/// How fast a body climbs a ladder, and the most it slides down one (blocks per second).
pub const CLIMB_SPEED: f32 = 2.35;
const CLIMB_SLIDE: f32 = 3.0;

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

/// True if no collision box overlaps the region.
fn clear(world: &impl Terrain, lo: Vec3, hi: Vec3) -> bool {
    !boxes_in(world, lo, hi).iter().any(|b| lo.x < b[3] && hi.x > b[0] && lo.y < b[4] && hi.y > b[1] && lo.z < b[5] && hi.z > b[2])
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
    /// Pushed against a wall while on a ladder last step: climbs up.
    pub climb_push: bool,
    /// Pulling itself up onto a ledge (see `try_mantle`): the body follows this path instead
    /// of walking.
    pub mantle: Option<Mantle>,
    /// Seconds a jump press still counts for catching a ledge after the key is let go.
    pub mantle_buffer: f32,
}

/// Grabbing a ledge up to two blocks high and pulling up onto it, Terraria style: the hands
/// hold the edge for a moment, the body rises along the wall, then swings over onto the top.
#[derive(Clone, Copy, Debug)]
pub struct Mantle {
    pub from: Vec3,
    /// Where the climb rises from: `from`, or a little back from an edge over the head.
    pub back: Vec3,
    pub to: Vec3,
    /// Height of the ledge's top surface (world y).
    pub ledge: f32,
    /// Seconds since the hands caught the edge, and how long the whole climb takes.
    pub t: f32,
    pub dur: f32,
}

/// The highest ledge a body can pull itself onto, above its feet.
pub const MANTLE_REACH: f32 = 2.05;
/// How long a jump press is remembered for catching a ledge.
const MANTLE_BUFFER: f32 = 0.5;
/// Phases of a climb, as fractions of its time: hold the edge, pull up, swing over.
pub const MANTLE_GRAB: f32 = 0.15;
pub const MANTLE_PULL: f32 = 0.7;

impl Mantle {
    /// 0..1 through the climb.
    pub fn progress(&self) -> f32 {
        (self.t / self.dur).clamp(0.0, 1.0)
    }

    /// Where the feet are at this point of the climb.
    pub fn at(&self) -> Vec3 {
        let u = self.progress();
        let ease = |x: f32| x * x * (3.0 - 2.0 * x);
        let lift = self.to.y + 0.04;
        if u < MANTLE_GRAB {
            // Hands catch the edge: the body sags a little under its own weight (and swings
            // back from an edge it was under).
            let k = u / MANTLE_GRAB;
            self.from + (self.back - self.from) * ease(k) - Vec3::Y * 0.05 * (k * std::f32::consts::PI).sin()
        } else if u < MANTLE_PULL {
            let k = ease((u - MANTLE_GRAB) / (MANTLE_PULL - MANTLE_GRAB));
            Vec3::new(self.back.x, self.back.y + (lift - self.back.y) * k, self.back.z)
        } else {
            let k = ease((u - MANTLE_PULL) / (1.0 - MANTLE_PULL));
            let xz = self.back + (self.to - self.back) * k;
            Vec3::new(xz.x, lift + (self.to.y - lift) * k, xz.z)
        }
    }
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
            climb_push: false,
            mantle: None,
            mantle_buffer: 0.0,
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
        self.fits_at(world, self.pos, height)
    }

    /// True if a box of this body's width and the given height fits with its feet at `pos`.
    pub fn fits_at(&self, world: &impl Terrain, pos: Vec3, height: f32) -> bool {
        let h = self.half_width - 0.001;
        clear(world, pos - Vec3::new(h, -0.001, h), pos + Vec3::new(h, height - 0.001, h))
    }

    /// Climbing onto a ledge: with jump held (or just pressed) next to a ledge at most two
    /// blocks above the feet, the hands catch its edge and the body pulls itself up. The ledge
    /// is looked for toward `wish`, or toward `face` (where the player looks) when standing
    /// still, and may hang right over the head (a shelf bumped into from below): the body
    /// then leans back from it before pulling up. Starts the climb and returns true when
    /// there is such a ledge with room to stand on it.
    pub fn try_mantle(&mut self, world: &impl Terrain, input: &Input, face: Vec3) -> bool {
        if self.mantle.is_some() || !(input.jump || self.mantle_buffer > 0.0) || input.crawl || input.sneak || self.flying || self.in_water || self.no_jump || self.height < PLAYER_HEIGHT - 0.01 {
            return false;
        }
        let feet = self.pos.floor().as_ivec3();
        if world.climbable(feet) || world.climbable(feet + IVec3::Y) {
            return false;
        }
        let wish = Vec3::new(input.wish.x, 0.0, input.wish.z);
        let dir = if wish.length_squared() > 0.01 { wish } else { Vec3::new(face.x, 0.0, face.z) }.normalize_or_zero();
        if dir == Vec3::ZERO {
            return false;
        }
        // Ledges the jump clears on its own are left to it: from the ground that is about
        // 1.27 blocks, in the air whatever height the rise still has left.
        let up_left = if self.on_ground { JUMP_SPEED } else { self.vel.y.max(0.0) };
        let min = (up_left * up_left / (2.0 * GRAVITY) + 0.03).max(0.15);
        let hw = self.half_width - 0.001;
        for top in self.ledges_near(world, dir) {
            let rise = top - self.pos.y;
            if rise <= min || rise > MANTLE_REACH {
                continue;
            }
            // Lean back from an edge hanging over the head until there is room to rise
            // straight up, head included.
            let Some(back) = (0..8).map(|i| i as f32 * 0.1).find(|&b| {
                let from = self.pos - dir * b;
                self.fits_at(world, from, rise + self.height + 0.04) && (b == 0.0 || clear(world, from.min(self.pos) - Vec3::new(hw, -0.001, hw), from.max(self.pos) + Vec3::new(hw, self.height - 0.001, hw)))
            }) else {
                continue;
            };
            let from = self.pos - dir * back;
            // Somewhere on the ledge to stand, as far onto it as fits (a shelf against a wall
            // is narrower than the body: it then stands partly over the edge).
            let up = Vec3::new(from.x, top, from.z);
            for i in 0..32 {
                let d = 0.9 + back - i as f32 * 0.05;
                if d < 0.2 {
                    break;
                }
                let to = up + dir * d;
                if !self.fits_at(world, to, self.height) || !self.supported_at(world, to, top) {
                    continue;
                }
                // Nothing in the way while swinging over.
                let (a, b) = (up.min(to), up.max(to));
                if !clear(world, a - Vec3::new(hw, -0.041, hw), b + Vec3::new(hw, self.height + 0.039, hw)) {
                    continue;
                }
                let dur = 0.32 + 0.16 * rise + back * 0.3;
                self.mantle = Some(Mantle { from: self.pos, back: from, to, ledge: top, t: 0.0, dur });
                self.mantle_buffer = 0.0;
                self.vel = Vec3::ZERO;
                self.fall_distance = 0.0;
                return true;
            }
        }
        false
    }

    /// Tops of the boxes within reach around the front half of the body and half a block
    /// ahead of it, highest first.
    fn ledges_near(&self, world: &impl Terrain, dir: Vec3) -> Vec<f32> {
        let hw = self.half_width;
        let near = self.pos - dir * 0.05;
        let far = self.pos + dir * (hw + 0.55);
        let side = Vec3::new(-dir.z, 0.0, dir.x) * (hw - 0.05);
        let lo = near.min(far).min(near + side).min(near - side).min(far + side).min(far - side);
        let hi = near.max(far).max(near + side).max(near - side).max(far + side).max(far - side);
        let (y0, y1) = (self.pos.y + 0.1, self.pos.y + MANTLE_REACH);
        let mut tops: Vec<f32> = Vec::new();
        for b in boxes_in(world, Vec3::new(lo.x, y0, lo.z), Vec3::new(hi.x, y1, hi.z)) {
            // Fences and walls stand taller than their block: no grabbing their tops.
            let tall = b[4] - b[1].floor() > 1.001;
            if !tall && lo.x < b[3] && hi.x > b[0] && lo.z < b[5] && hi.z > b[2] && b[4] > y0 && b[4] <= y1 && !tops.iter().any(|t| (t - b[4]).abs() < 0.01) {
                tops.push(b[4]);
            }
        }
        tops.sort_by(|a, b| b.total_cmp(a));
        tops
    }

    /// Something to stand on under feet placed at `pos`, with its top at `top`.
    fn supported_at(&self, world: &impl Terrain, pos: Vec3, top: f32) -> bool {
        let h = self.half_width - 0.05;
        let (lo, hi) = (pos - Vec3::new(h, 0.3, h), pos + Vec3::new(h, 0.0, h));
        boxes_in(world, lo, hi).iter().any(|b| lo.x < b[3] && hi.x > b[0] && lo.z < b[5] && hi.z > b[2] && (b[4] - top).abs() < 0.01)
    }

    /// Advance a climb started by `try_mantle`. Returns false when the body isn't climbing.
    pub fn mantle_step(&mut self, dt: f32) -> bool {
        let Some(m) = self.mantle.as_mut() else { return false };
        // Moved by something else (teleported, respawned, mounted): the climb is over.
        if (self.pos - m.at()).length() > 0.25 {
            self.mantle = None;
            return false;
        }
        m.t += dt.min(0.05);
        let m = *m;
        self.pos = m.at();
        self.vel = Vec3::ZERO;
        self.fall_distance = 0.0;
        self.on_ground = false;
        if m.t >= m.dur {
            self.pos = m.to;
            self.on_ground = true;
            self.mantle = None;
        }
        true
    }

    /// True if the player's box overlaps the given block cell.
    pub fn intersects_block(&self, b: IVec3) -> bool {
        let (lo, hi) = self.aabb();
        let bl = b.as_vec3();
        lo.x < bl.x + 1.0 && hi.x > bl.x && lo.y < bl.y + 1.0 && hi.y > bl.y && lo.z < bl.z + 1.0 && hi.z > bl.z
    }

    pub fn step(&mut self, world: &impl Terrain, input: &Input, dt: f32) {
        // A tap of jump counts a little while for catching a ledge at the top of the jump.
        self.mantle_buffer = if input.jump { MANTLE_BUFFER } else { (self.mantle_buffer - dt).max(0.0) };
        // A climb started with `try_mantle` (players only) runs until the body is on top.
        if self.mantle_step(dt) {
            return;
        }
        let dt = dt.min(0.05);
        let feet = self.pos.floor().as_ivec3();
        self.in_water = world.liquid(feet) || world.liquid(feet + IVec3::Y);
        self.head_in_water = world.liquid(self.eye().floor().as_ivec3());
        let mut wish = Vec3::new(input.wish.x, 0.0, input.wish.z);
        if wish.length_squared() > 1.0 {
            wish = wish.normalize();
        }
        let climbing = !self.flying && !self.in_water && (world.climbable(feet) || world.climbable(feet + IVec3::Y));

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
            if climbing {
                // On a ladder: slide down slowly, hold still while sneaking.
                self.vel.y = self.vel.y.max(if input.sneak { 0.0 } else { -CLIMB_SLIDE });
                if input.jump || self.climb_push {
                    self.vel.y = CLIMB_SPEED;
                }
            }
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
        self.climb_push = false;
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
                // Walking into the wall a ladder hangs on climbs it.
                if axis != 1 && climbing {
                    self.climb_push = true;
                }
                if axis == 1 && delta[1] < 0.0 {
                    self.on_ground = true;
                    if self.flying {
                        self.flying = false;
                    }
                }
                self.vel[axis] = 0.0;
            }
        }
        if self.on_ground || self.in_water || climbing {
            self.fall_distance = 0.0;
        } else if delta.y < 0.0 {
            self.fall_distance -= delta.y;
        }
    }

    /// Move by the current velocity without steering it (vehicles): stops against blocks and
    /// steps up onto low ones while on the ground.
    pub fn travel(&mut self, world: &impl Terrain, dt: f32) {
        let dt = dt.min(0.05);
        let delta = self.vel * dt;
        let was_on_ground = self.on_ground;
        self.on_ground = false;
        for axis in [1usize, 0, 2] {
            let mut moved = self.move_axis(world, axis, delta[axis]);
            if axis != 1 && was_on_ground && moved.abs() + 1e-6 < delta[axis].abs() {
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
            if moved.abs() + 1e-6 < delta[axis].abs() {
                if axis == 1 && delta[1] < 0.0 {
                    self.on_ground = true;
                }
                self.vel[axis] = 0.0;
            }
        }
        let feet = self.pos.floor().as_ivec3();
        self.in_water = world.liquid(feet);
    }

    fn has_ground_below(&self, world: &impl Terrain) -> bool {
        let (lo, hi) = self.aabb();
        let probe_lo = Vec3::new(lo.x, lo.y - 0.6, lo.z);
        let probe_hi = Vec3::new(hi.x, lo.y, hi.z);
        boxes_in(world, probe_lo, probe_hi).iter().any(|b| lo.x < b[3] && hi.x > b[0] && lo.z < b[5] && hi.z > b[2] && b[4] >= lo.y - 0.6 && b[4] <= lo.y + 1e-3)
    }

    /// Move along one axis, stopping at the first box in the way. Returns the distance moved.
    /// Free flight for flying creatures: no gravity, moves at `vel`, stops at walls.
    pub fn fly(&mut self, world: &impl Terrain, vel: Vec3, dt: f32) {
        let dt = dt.min(0.05);
        self.vel = vel;
        self.in_water = false;
        self.on_ground = false;
        self.fall_distance = 0.0;
        for axis in [1usize, 0, 2] {
            let d = vel[axis] * dt;
            let moved = self.move_axis(world, axis, d);
            if moved.abs() + 1e-6 < d.abs() {
                self.vel[axis] = 0.0;
                if axis == 1 && d < 0.0 {
                    self.on_ground = true;
                }
            }
        }
    }

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

    /// A wall two blocks high at x = 3 with open ground on top (x 3..6), a three-high wall at
    /// z = 8 and a fence-like box at z = -4.
    struct Ledges;
    impl Terrain for Ledges {
        fn solid(&self, p: IVec3) -> bool {
            p.y < 10 || (p.x >= 3 && p.y < 12) || (p.z == 8 && p.y < 13)
        }
        fn liquid(&self, _: IVec3) -> bool {
            false
        }
    }

    fn climb(world: &impl Terrain, start: Vec3, wish: Vec3) -> Body {
        let mut b = Body::new(start);
        let input = Input { wish, jump: true, ..Default::default() };
        for _ in 0..10 {
            b.step(world, &Input::default(), 0.02);
        }
        for _ in 0..150 {
            b.try_mantle(world, &input, input.wish);
            b.step(world, &input, 0.02);
        }
        b
    }

    #[test]
    fn pulls_up_onto_a_two_block_ledge() {
        let b = climb(&Ledges, Vec3::new(2.0, 10.0, 0.5), Vec3::X);
        assert!(b.pos.y >= 12.0 - 1e-3, "still below: {:?}", b.pos);
        assert!(b.pos.x > 3.0, "{:?}", b.pos);
    }

    #[test]
    fn three_blocks_are_too_high() {
        let b = climb(&Ledges, Vec3::new(0.5, 10.0, 7.0), Vec3::Z);
        assert!(b.pos.y < 11.5, "{:?}", b.pos);
    }

    #[test]
    fn climb_takes_time_and_passes_over_the_edge() {
        let mut b = Body::new(Vec3::new(2.69, 10.0, 0.5));
        b.on_ground = true;
        let input = Input { wish: Vec3::X, jump: true, ..Default::default() };
        assert!(b.try_mantle(&Ledges, &input, Vec3::X));
        b.step(&Ledges, &input, 0.02);
        // Early on the body is still below the top, against the wall.
        assert!(b.pos.y < 11.0 && b.pos.x < 2.75, "{:?}", b.pos);
        for _ in 0..100 {
            if b.mantle.is_none() {
                break;
            }
            b.step(&Ledges, &input, 0.02);
        }
        assert!(b.mantle.is_none() && b.on_ground);
        assert!((b.pos.y - 12.0).abs() < 1e-3 && b.pos.x > 3.3, "{:?}", b.pos);
    }

    #[test]
    fn teleporting_ends_a_climb() {
        let mut b = Body::new(Vec3::new(2.69, 10.0, 0.5));
        b.on_ground = true;
        let input = Input { wish: Vec3::X, jump: true, ..Default::default() };
        assert!(b.try_mantle(&Ledges, &input, Vec3::X));
        b.step(&Ledges, &input, 0.02);
        b.pos = Vec3::new(-5.0, 10.0, 0.5);
        b.step(&Ledges, &Input::default(), 0.02);
        assert!(b.mantle.is_none() && b.pos.x < -4.9);
    }

    /// Shelves on a tall wall at x = 3: thin boards sticking out 7/16 from it, tops at 11.625
    /// and 13.625 (the second one two blocks above the first).
    struct Shelves;
    impl Terrain for Shelves {
        fn solid(&self, p: IVec3) -> bool {
            p.y < 10 || p.x >= 3 || (p.x == 2 && (p.y == 11 || p.y == 13))
        }
        fn liquid(&self, _: IVec3) -> bool {
            false
        }
        fn boxes(&self, p: IVec3, out: &mut Vec<[f32; 6]>) {
            if p.x == 2 && (p.y == 11 || p.y == 13) {
                out.push([0.5625, 0.5, 0.0, 1.0, 0.625, 1.0]);
            } else if self.solid(p) {
                out.push([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
            }
        }
    }

    /// Steps until a climb has started and finished.
    fn run(b: &mut Body, world: &impl Terrain, input: &Input, steps: usize) {
        let mut climbed = false;
        for _ in 0..steps {
            b.try_mantle(world, input, Vec3::X);
            b.step(world, input, 0.02);
            climbed |= b.mantle.is_some();
            if climbed && b.mantle.is_none() {
                return;
            }
        }
    }

    #[test]
    fn climbs_shelves_up_a_wall() {
        let mut b = Body::new(Vec3::new(1.5, 10.0, 0.5));
        let input = Input { wish: Vec3::X, jump: true, ..Default::default() };
        run(&mut b, &Shelves, &input, 80);
        assert!((b.pos.y - 11.625).abs() < 1e-3, "first shelf: {:?}", b.pos);
        // Standing on it the next shelf hangs over the head: lean back, pull up.
        run(&mut b, &Shelves, &input, 80);
        assert!((b.pos.y - 13.625).abs() < 1e-3, "second shelf: {:?}", b.pos);
    }

    #[test]
    fn a_tapped_jump_still_catches_the_edge() {
        let mut b = Body::new(Vec3::new(2.5, 10.0, 0.5));
        b.on_ground = true;
        let tap = Input { wish: Vec3::X, jump: true, ..Default::default() };
        let walk = Input { wish: Vec3::X, ..Default::default() };
        b.step(&Ledges, &tap, 0.02);
        for _ in 0..10 {
            b.try_mantle(&Ledges, &walk, Vec3::X);
            b.step(&Ledges, &walk, 0.02);
        }
        assert!(b.mantle.is_some(), "{:?}", b.pos);
    }

    #[test]
    fn standing_still_uses_the_look() {
        let mut b = Body::new(Vec3::new(2.69, 10.0, 0.5));
        b.on_ground = true;
        let input = Input { jump: true, ..Default::default() };
        assert!(b.try_mantle(&Ledges, &input, Vec3::new(1.0, -0.3, 0.1)));
    }

    #[test]
    fn one_block_steps_are_left_to_the_jump() {
        struct Step;
        impl Terrain for Step {
            fn solid(&self, p: IVec3) -> bool {
                p.y < 10 || (p.x >= 3 && p.y < 11)
            }
            fn liquid(&self, _: IVec3) -> bool {
                false
            }
        }
        let mut b = Body::new(Vec3::new(2.69, 10.0, 0.5));
        b.on_ground = true;
        let input = Input { wish: Vec3::X, jump: true, ..Default::default() };
        assert!(!b.try_mantle(&Step, &input, Vec3::X));
    }

    #[test]
    fn jumping_into_a_shelf_overhead_climbs_it() {
        // One shelf high on the wall (top 12.625, out of reach from the ground), the player
        // right under it: the jump bumps the head, the hands catch it.
        struct High;
        impl Terrain for High {
            fn solid(&self, p: IVec3) -> bool {
                p.y < 10 || p.x >= 3 || (p.x == 2 && p.y == 12)
            }
            fn liquid(&self, _: IVec3) -> bool {
                false
            }
            fn boxes(&self, p: IVec3, out: &mut Vec<[f32; 6]>) {
                if p.x == 2 && p.y == 12 {
                    out.push([0.5625, 0.5, 0.0, 1.0, 0.625, 1.0]);
                } else if self.solid(p) {
                    out.push([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
                }
            }
        }
        let mut b = Body::new(Vec3::new(2.7, 10.0, 0.5));
        let input = Input { wish: Vec3::X, jump: true, ..Default::default() };
        run(&mut b, &High, &input, 120);
        assert!((b.pos.y - 12.625).abs() < 1e-3, "{:?}", b.pos);
    }
}
