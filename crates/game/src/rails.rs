//! Rails and minecarts: laying rails that join up on their own, and carts that roll along
//! them, climb and run down slopes, take a push from powered rails and carry the player.

use glam::{IVec3, Mat4, Vec3};
use voxel::block::{BlockId, AIR};
use voxel::physics::{Body, Terrain};
use voxel::shapes::{rail_on, rail_rises, rail_shape, RAIL_ENDS};

use crate::entity::Bones;
use crate::redstone::Kind;
use crate::Game;

/// Fastest a cart goes on its own, in blocks a second.
const MAX_SPEED: f32 = 8.0;
/// Pull down a slope.
const SLOPE_PULL: f32 = 6.0;
/// What a powered rail adds, per second.
const BOOST: f32 = 14.0;

const N: IVec3 = IVec3::NEG_Z;
const E: IVec3 = IVec3::X;
const S: IVec3 = IVec3::Z;
const W: IVec3 = IVec3::NEG_X;

#[derive(Clone, Debug)]
pub struct Minecart {
    /// Off the rails it falls and slides like anything else.
    pub body: Body,
    pub on_rail: bool,
    /// Turn of the cart (along the rail it is on) and its tilt on a slope.
    pub yaw: f32,
    pub pitch: f32,
    /// Hits taken lately; it breaks after a few.
    pub damage: f32,
    pub hurt: f32,
}

impl Minecart {
    pub fn new(pos: Vec3, yaw: f32) -> Minecart {
        Minecart { body: Body::sized(pos, 0.98, 0.7), on_rail: false, yaw, pitch: 0.0, damage: 0.0, hurt: 1.0 }
    }

    /// The cart's box (for clicking it).
    pub fn aabb(&self) -> (Vec3, Vec3) {
        let p = self.body.pos;
        (p - Vec3::new(0.49, 0.0, 0.49), p + Vec3::new(0.49, 0.7, 0.49))
    }
}

/// Where a cart on the rail at `c` runs: from the middle of one edge to the middle of the other
/// (the first end is the high one on a slope).
fn segment(c: IVec3, v: u8) -> (Vec3, Vec3) {
    let s = rail_shape(v);
    let e = RAIL_ENDS[s];
    let mid = c.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
    let up = if rail_rises(v).is_some() { 1.0 } else { 0.0 };
    (mid + Vec3::new(e[0][0] as f32 * 0.5, up, e[0][1] as f32 * 0.5), mid + Vec3::new(e[1][0] as f32 * 0.5, 0.0, e[1][1] as f32 * 0.5))
}

/// The rail shape that joins these sides (and climbs toward a side marked as higher).
fn shape_for(sides: &[(IVec3, i32)], special: bool, look_x: bool) -> u8 {
    let has = |d: IVec3| sides.iter().any(|s| s.0 == d);
    let up = |d: IVec3| sides.iter().any(|s| s.0 == d && s.1 > 0);
    let rise = |d: IVec3| match d {
        E => 2,
        W => 3,
        N => 4,
        _ => 5,
    };
    // Straight lines first, then a curve, then a lone neighbour sets the line.
    let pair = if has(N) && has(S) {
        Some((N, S))
    } else if has(E) && has(W) {
        Some((E, W))
    } else if !special && sides.len() >= 2 {
        let a = sides[0].0;
        let b = sides.iter().map(|s| s.0).find(|&d| d != a && d != -a);
        b.map(|b| (a, b))
    } else {
        None
    };
    match pair {
        Some((a, b)) if a == -b => {
            if up(a) {
                rise(a)
            } else if up(b) {
                rise(b)
            } else if a.x == 0 {
                0
            } else {
                1
            }
        }
        Some((a, b)) => {
            let (ns, ew) = if a.z != 0 { (a, b) } else { (b, a) };
            match (ns == S, ew == E) {
                (true, true) => 6,
                (true, false) => 7,
                (false, false) => 8,
                (false, true) => 9,
            }
        }
        None => match sides.first() {
            Some(&(d, dy)) => {
                if dy > 0 {
                    rise(d)
                } else if d.x == 0 {
                    0
                } else {
                    1
                }
            }
            None => u8::from(look_x),
        },
    }
}

impl Game {
    fn rail_kind(&self, id: BlockId) -> Option<u8> {
        match self.redstone.kind(id) {
            Kind::Rail(k) => Some(k),
            _ => None,
        }
    }

    /// A rail next to `p` on side `d`: level with it, a block up, or a block down.
    fn rail_beside(&self, p: IVec3, d: IVec3) -> Option<IVec3> {
        [p + d, p + d + IVec3::Y, p + d - IVec3::Y].into_iter().find(|&q| self.rail_kind(self.world.block_or_air(q)).is_some())
    }

    /// Whether the rail at `q` runs into `p` (one of its ends points there).
    fn rail_joins(&self, q: IVec3, p: IVec3) -> bool {
        let id = self.world.block_or_air(q);
        self.rail_kind(id).is_some() && self.redstone.rail_links(&self.world, q, id).iter().any(|l| l.x == p.x && l.z == p.z && (l.y - p.y).abs() <= 1)
    }

    /// Sides of the rail at `q` that run into a rail which runs back into it.
    fn rail_joined_sides(&self, q: IVec3) -> Vec<(IVec3, i32)> {
        let id = self.world.block_or_air(q);
        let v = self.reg.get(id).variant;
        let s = rail_shape(v);
        let mut out = Vec::new();
        for (k, e) in RAIL_ENDS[s].iter().enumerate() {
            let d = IVec3::new(e[0], 0, e[1]);
            if let Some(n) = self.rail_beside(q, d) {
                if self.rail_joins(n, q) {
                    let dy = if k == 0 && rail_rises(v).is_some() { 1 } else { 0 };
                    out.push((d, dy));
                }
            }
        }
        out
    }

    /// The shape a rail placed at `p` takes: it joins the rails around it that are still free.
    pub(crate) fn rail_variant(&self, p: IVec3, id: BlockId, look: Vec3) -> u8 {
        let special = self.reg.get(id).name != "rail";
        let mut sides = Vec::new();
        for d in [N, E, S, W] {
            if let Some(q) = self.rail_beside(p, d) {
                if self.rail_joins(q, p) || self.rail_joined_sides(q).len() < 2 {
                    sides.push((d, q.y - p.y));
                }
            }
        }
        shape_for(&sides, special, look.x.abs() > look.z.abs())
    }

    /// After a rail is laid at `p`, the rails it points into turn to meet it.
    pub(crate) fn connect_rail(&mut self, p: IVec3) {
        let id = self.world.block_or_air(p);
        for q in self.redstone.rail_links(&self.world, p, id) {
            if self.rail_joins(q, p) {
                continue;
            }
            let mut sides = self.rail_joined_sides(q);
            if sides.len() >= 2 {
                continue;
            }
            let d = IVec3::new(p.x - q.x, 0, p.z - q.z);
            sides.push((d, p.y - q.y));
            let qid = self.world.block_or_air(q);
            let qb = self.reg.get(qid);
            let special = self.reg.get(qb.base).name != "rail";
            let mut v = shape_for(&sides, special, d.x != 0);
            if special && rail_on(qb.variant) {
                v += 10;
            }
            let new = self.reg.variant(qb.base, v);
            self.world.set_block(q, new);
        }
    }

    /// The rail under a cart at `pos`: (cell, variant, kind).
    fn rail_under(&self, pos: Vec3) -> Option<(IVec3, u8, u8)> {
        let c = (pos + Vec3::Y * 0.1).floor().as_ivec3();
        for q in [c, c - IVec3::Y] {
            let id = self.world.block_or_air(q);
            if let Some(k) = self.rail_kind(id) {
                return Some((q, self.reg.get(id).variant, k));
            }
        }
        None
    }

    /// Right click: get into a cart you look at, or put one on a rail.
    pub(crate) fn cart_use(&mut self) -> bool {
        if let Some(i) = self.cart_in_sight() {
            if self.riding.is_none() && self.crouch <= 0.5 {
                self.riding = Some(i);
                self.body.vel = Vec3::ZERO;
                return true;
            }
        }
        let held = self.held_stack().map(|s| self.items.get(s.item).name.clone());
        if held.as_deref() != Some("minecart") {
            return false;
        }
        let Some(hit) = self.target else { return false };
        let id = self.world.block_or_air(hit.block);
        if self.rail_kind(id).is_none() {
            return false;
        }
        let v = self.reg.get(id).variant;
        let (a, b) = segment(hit.block, v);
        let d = b - a;
        let mut cart = Minecart::new((a + b) * 0.5, d.x.atan2(d.z));
        cart.on_rail = true;
        self.surv.carts.push(cart);
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.anim.start_swing();
        self.sfx(crate::audio::Sfx::Place(crate::audio::Material::Stone));
        true
    }

    /// The nearest cart the crosshair is on, nearer than the block it points at.
    fn cart_in_sight(&self) -> Option<usize> {
        let eye = self.body.eye();
        let f = self.forward();
        let block = self.target.map(|t| t.distance).unwrap_or(f32::MAX);
        let mut best = None;
        for (i, c) in self.surv.carts.iter().enumerate() {
            if self.riding == Some(i) {
                continue;
            }
            let (lo, hi) = c.aabb();
            if let Some(t) = crate::mobs::ray_box(eye, f, lo, hi) {
                if t < self.reach() && t < block + 0.3 && best.map_or(true, |(_, bt)| t < bt) {
                    best = Some((i, t));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// Left click on a cart: a few hits break it (one in creative).
    pub(crate) fn cart_attack(&mut self) -> bool {
        let Some(i) = self.cart_in_sight() else { return false };
        let c = &mut self.surv.carts[i];
        c.damage += if self.survival { 4.0 } else { 40.0 };
        c.hurt = 0.0;
        let eye = self.body.eye();
        let push = (c.body.pos - eye) * Vec3::new(1.0, 0.0, 1.0);
        c.body.vel += push.normalize_or_zero() * 1.5;
        let (broken, at) = (c.damage >= 10.0, c.body.pos + Vec3::Y * 0.3);
        self.anim.start_swing();
        self.sfx(crate::audio::Sfx::Hit(crate::audio::Material::Stone));
        if broken {
            self.remove_cart(i);
            if self.survival {
                if let Some(it) = self.items.id("minecart") {
                    self.spawn_drop(at, voxel::item::ItemStack::new(it, 1), Vec3::Y * 2.0);
                }
            }
        }
        true
    }

    fn remove_cart(&mut self, i: usize) {
        self.surv.carts.remove(i);
        self.riding = match self.riding {
            Some(r) if r == i => None,
            Some(r) if r > i => Some(r - 1),
            r => r,
        };
    }

    /// Climb out of the cart beside it (or on top when the sides are walled in).
    pub(crate) fn leave_cart(&mut self) {
        let Some(i) = self.riding.take() else { return };
        let c = self.surv.carts[i].body.pos;
        for d in [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z] {
            let mut b = self.body.clone();
            b.pos = c + d + Vec3::Y * 0.1;
            if b.fits(&self.world, voxel::physics::PLAYER_HEIGHT) && self.world.solid((b.pos - Vec3::Y * 0.5).floor().as_ivec3()) {
                self.body.pos = b.pos;
                return;
            }
        }
        self.body.pos = c + Vec3::Y * 0.8;
    }

    /// The player in a cart: they sit in it, steer it a little with the movement keys and
    /// climb out with sneak. True while riding.
    pub(crate) fn ride_step(&mut self, wish: Vec3, sneak: bool) -> bool {
        let Some(i) = self.riding else { return false };
        if i >= self.surv.carts.len() {
            self.riding = None;
            return false;
        }
        if sneak {
            self.leave_cart();
            return false;
        }
        let c = &mut self.surv.carts[i];
        if wish.length_squared() > 0.01 {
            c.body.vel += wish.normalize() * 0.15;
        }
        self.body.pos = c.body.pos + Vec3::Y * 0.1;
        self.body.vel = c.body.vel;
        self.body.on_ground = true;
        self.body.fall_distance = 0.0;
        true
    }

    /// Roll every cart along.
    pub(crate) fn update_carts(&mut self, dt: f32) {
        if self.surv.carts.is_empty() {
            return;
        }
        let mut carts = std::mem::take(&mut self.surv.carts);
        let me = self.body.pos;
        let mut eject = false;
        for (i, c) in carts.iter_mut().enumerate() {
            let rider = self.riding == Some(i);
            c.hurt += dt;
            c.damage = (c.damage - dt * 2.0).max(0.0);
            if self.world.block(c.body.pos.floor().as_ivec3()).is_none() {
                continue;
            }
            // Walking into a cart pushes it.
            let off = (c.body.pos - me) * Vec3::new(1.0, 0.0, 1.0);
            if !rider && off.length() < 0.8 && (c.body.pos.y - me.y).abs() < 1.0 {
                c.body.vel += off.normalize_or_zero() * 8.0 * dt;
            }
            let steps = ((c.body.vel.length() * dt / 0.3).ceil() as usize).clamp(1, 8);
            let h = dt / steps as f32;
            for _ in 0..steps {
                if self.cart_step(c, h, rider) {
                    eject = eject || rider;
                }
            }
        }
        // Carts bump into each other.
        for a in 0..carts.len() {
            for b in a + 1..carts.len() {
                let d = carts[b].body.pos - carts[a].body.pos;
                if d.length() < 1.0 && d.length() > 1e-4 {
                    let n = d.normalize();
                    let closing = (carts[a].body.vel - carts[b].body.vel).dot(n);
                    if closing > 0.0 {
                        carts[a].body.vel -= n * closing;
                        carts[b].body.vel += n * closing;
                    }
                }
            }
        }
        self.surv.carts = carts;
        if eject {
            self.leave_cart();
        }
    }

    /// One short step of a cart. True when an activator rail threw its rider out.
    fn cart_step(&self, c: &mut Minecart, dt: f32, rider: bool) -> bool {
        let Some((cell, v, kind)) = self.rail_under(c.body.pos) else {
            c.on_rail = false;
            let still = voxel::physics::Input::default();
            c.body.step(&self.world, &still, dt);
            c.pitch *= 0.8;
            return false;
        };
        c.on_rail = true;
        let (a, b) = segment(cell, v);
        let len = (b - a).length();
        let dir = (b - a) / len;
        let mut t = (c.body.pos - a).dot(dir).clamp(0.0, len);
        let vel = c.body.vel;
        let along = vel.dot(dir);
        // Coming round a bend keeps the speed; a push across the rail mostly doesn't.
        let mut sp = if along.abs() > vel.length() * 0.3 { vel.length() * along.signum() } else { along };
        sp -= SLOPE_PULL * dir.y * dt;
        let on = rail_on(v);
        let mut thrown = false;
        match kind {
            1 if on => {
                if sp.abs() < 0.5 {
                    // From standstill it pushes away from a wall at one end.
                    let ea = (a - cell.as_vec3() - Vec3::new(0.5, 0.0, 0.5)) * Vec3::new(2.0, 0.0, 2.0);
                    let wall_a = self.world.solid(cell + ea.round().as_ivec3());
                    let wall_b = self.world.solid(cell - ea.round().as_ivec3());
                    if wall_a && !wall_b {
                        sp = 2.0;
                    } else if wall_b && !wall_a {
                        sp = -2.0;
                    }
                } else if sp.abs() < MAX_SPEED * self.tr3_cart_factor() {
                    sp += BOOST * sp.signum() * dt;
                }
            }
            1 => {
                sp *= (-12.0 * dt).exp();
                if sp.abs() < 0.3 {
                    sp = 0.0;
                }
            }
            3 if on && rider => thrown = true,
            _ => {}
        }
        sp *= (-(if rider { 0.08 } else { 0.5 }) * dt).exp();
        let max = MAX_SPEED * if rider { self.tr3_cart_factor() } else { 1.0 };
        sp = sp.clamp(-max, max);
        t += sp * dt;
        let mut pos = a + dir * t.clamp(0.0, len);
        if t < 0.0 || t > len {
            // Off one end: into the next rail, against a wall, or off into the air.
            let (end, out) = if t > len { (b, dir) } else { (a, -dir) };
            let over = if t > len { t - len } else { -t };
            let next = end + out * over;
            let flat = Vec3::new(out.x, 0.0, out.z).normalize_or_zero();
            let ahead = (end + flat * 0.3 + Vec3::Y * 0.5).floor().as_ivec3();
            if self.rail_under(next).is_some() {
                pos = next;
            } else if self.world.solid(ahead) {
                pos = end - out * 0.01;
                sp = 0.0;
            } else {
                pos = next;
                c.on_rail = false;
            }
        }
        c.body.pos = pos;
        c.body.vel = dir * sp;
        c.body.on_ground = true;
        c.yaw = dir.x.atan2(dir.z);
        c.pitch = dir.y.asin();
        thrown
    }

    /// Cart models for the entity renderer.
    pub(crate) fn cart_instances(&self, cam_block: IVec3) -> Vec<(usize, Bones)> {
        let Some(def) = self.mobs.reg.id("minecart") else { return Vec::new() };
        self.surv
            .carts
            .iter()
            .map(|c| {
                let l = self.world.light((c.body.pos + Vec3::Y * 0.5).floor().as_ivec3());
                let mut b = Bones { light: [(l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0, if c.hurt < 0.25 { 1.0 } else { 0.0 }, 0.0], ..Default::default() };
                let m = Mat4::from_translation(c.body.pos - cam_block.as_vec3())
                    * Mat4::from_rotation_y(c.yaw)
                    * Mat4::from_rotation_x(-c.pitch)
                    * Mat4::from_scale(Vec3::splat(1.0 / 16.0));
                for mm in b.m.iter_mut() {
                    *mm = m.to_cols_array_2d();
                }
                (def, b)
            })
            .collect()
    }

    /// Carts on detector rails, for the redstone engine.
    pub(crate) fn cart_cells(&self) -> Vec<IVec3> {
        self.surv.carts.iter().filter(|c| c.on_rail).filter_map(|c| self.rail_under(c.body.pos).map(|r| r.0)).collect()
    }

    /// Test scene (--fase4): a loop of rails with carts on it, a ramp, a detector lighting a
    /// lamp, the copper family in its four stages and a crafter.
    pub(crate) fn build_phase4(&mut self) {
        let p = self.body.pos.floor().as_ivec3();
        let floor = self.reg.id_or_air("stone");
        for x in -8..=8 {
            for z in 2..=19 {
                self.world.set_block(IVec3::new(p.x + x, p.y - 1, p.z - z), floor);
                for y in 0..5 {
                    self.world.set_block(IVec3::new(p.x + x, p.y + y, p.z - z), AIR);
                }
            }
        }
        let reg = self.reg.clone();
        let at = |x: i32, y: i32, z: i32| IVec3::new(p.x + x, p.y + y, p.z - z);
        let put = |w: &mut crate::world::World, x: i32, y: i32, z: i32, name: &str, v: u8| {
            let id = reg.variant(reg.id_or_air(name), v);
            w.set_block(at(x, y, z), id);
        };
        let w = &mut self.world;
        // The loop: straight sides, four curves, powered rails on a redstone bed at the front.
        for x in -5..=5 {
            let powered = (-2..=2).contains(&x);
            if powered {
                put(w, x, -1, 4, "redstone_block", 0);
            }
            put(w, x, 0, 4, if powered { "powered_rail" } else { "rail" }, if powered { 11 } else { 1 });
            put(w, x, 0, 12, if x == 0 { "detector_rail" } else { "rail" }, 1);
        }
        for z in 5..=11 {
            put(w, -6, 0, z, "rail", 0);
            put(w, 6, 0, z, "rail", 0);
        }
        put(w, -6, 0, 4, "rail", 9);
        put(w, 6, 0, 4, "rail", 8);
        put(w, -6, 0, 12, "rail", 6);
        put(w, 6, 0, 12, "rail", 7);
        put(w, 0, 0, 13, "redstone_lamp", 0);
        // A ramp over a step of stone.
        put(w, -7, 0, 15, "rail", 1);
        put(w, -6, 0, 15, "rail", 2);
        put(w, -5, 0, 15, "stone", 0);
        put(w, -4, 0, 15, "stone", 0);
        put(w, -5, 1, 15, "rail", 1);
        put(w, -4, 1, 15, "rail", 1);
        put(w, -3, 0, 15, "rail", 3);
        put(w, -2, 0, 15, "rail", 1);
        // Copper, from new to green: blocks, cut, chiseled; then grates, lit bulbs, cut stairs.
        for (k, s) in ["", "exposed_", "weathered_", "oxidized_"].iter().enumerate() {
            let x = 1 + k as i32;
            let block = if k == 0 { "copper_block".to_string() } else { format!("{s}copper") };
            put(w, x, 0, 18, &block, 0);
            put(w, x, 1, 18, &format!("{s}cut_copper"), 0);
            put(w, x, 2, 18, &format!("{s}chiseled_copper"), 0);
            put(w, x + 4, 0, 18, &format!("{s}copper_grate"), 0);
            put(w, x + 4, 1, 18, &format!("{s}copper_bulb_lit"), 0);
            put(w, x + 4, 2, 18, &format!("{s}copper_bulb"), 0);
            put(w, x, 0, 17, &format!("{s}cut_copper_stairs"), 2);
        }
        put(w, 1, 0, 15, "copper_door", 2);
        put(w, 1, 1, 15, "copper_door", 2 | 8);
        put(w, 3, 0, 15, "weathered_copper_trapdoor", 0);
        // Furniture inside the loop: a long oak table with chairs round it, a small cherry set.
        for x in -1..=1 {
            put(w, x, 0, 8, "oak_table", 0);
            put(w, x, 0, 9, "oak_chair", 2);
            put(w, x, 0, 7, "spruce_chair", 0);
        }
        put(w, -2, 0, 8, "birch_chair", 1);
        put(w, 2, 0, 8, "dark_oak_chair", 3);
        put(w, 4, 0, 9, "cherry_table", 0);
        put(w, 4, 0, 10, "cherry_chair", 2);
        put(w, -4, 0, 9, "acacia_table", 0);
        put(w, -4, 0, 10, "jungle_chair", 2);
        put(w, 0, 1, 8, "lantern", 0);
        put(w, 4, 1, 9, "copper_lantern", 0);
        put(w, -4, 1, 9, "paper_lamp", 0);
        // A wall of windows behind the table, a lantern hanging under a beam.
        for (k, wood) in ["oak", "birch", "spruce", "jungle", "acacia", "dark_oak", "cherry", "mangrove"].iter().enumerate() {
            put(w, k as i32 - 3, 0, 11, &format!("{wood}_window"), 0);
            put(w, k as i32 - 3, 1, 11, &format!("{wood}_window"), 0);
        }
        put(w, 7, 3, 8, "oak_planks", 0);
        put(w, 7, 2, 8, "chain", 0);
        put(w, 7, 1, 8, "lantern", 1);
        put(w, 7, 0, 6, "red_paper_lamp", 0);
        // A rope bridge over a dip, and a row of iron bars.
        for z in 3..=10 {
            put(w, -8, 0, z, "rope_bridge", 0);
            put(w, -8, -1, z, "air", 0);
        }
        for z in 13..=16 {
            put(w, 7, 0, z, "iron_bars", 0);
            put(w, 7, 1, z, "iron_bars", 0);
        }
        // A crafter facing you with planks in it, and a lever to work it.
        put(w, -1, 0, 18, "crafter", 2);
        put(w, -2, 0, 18, "lever", 3);
        let stack = |g: &crate::Game, n: &str, c: u8| g.items.id(n).map(|i| voxel::item::ItemStack::new(i, c));
        let mut inv = voxel::item::Inventory::new(9);
        inv.slots[0] = stack(self, "oak_planks", 8);
        inv.slots[3] = stack(self, "oak_planks", 8);
        self.surv.containers.insert(at(-1, 0, 18), crate::survival::Container::Chest(inv));
        // Carts: one running laps (the powered rails keep it going), one rolling along the far
        // side and one heading over the ramp.
        for (x, z, vx) in [(-1, 4, 2.0), (3, 12, -6.0), (-2, 15, -5.0)] {
            let mut c = Minecart::new(at(x, 0, z).as_vec3() + Vec3::new(0.5, 0.0, 0.5), std::f32::consts::FRAC_PI_2);
            c.on_rail = true;
            c.body.vel = Vec3::new(vx, 0.0, 0.0);
            self.surv.carts.push(c);
        }
        if std::env::var("MC_SIT").is_ok() {
            self.seat = Some(at(-2, 0, 8));
        } else if self.args.screenshot.is_some() {
            self.body.pos += Vec3::new(0.0, 6.0, 2.0);
            self.body.flying = true;
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rails_join_the_way_they_should() {
        assert_eq!(shape_for(&[(N, 0), (S, 0)], false, false), 0);
        assert_eq!(shape_for(&[(E, 0)], false, false), 1);
        assert_eq!(shape_for(&[(E, 1), (W, 0)], false, false), 2);
        assert_eq!(shape_for(&[(S, 0), (E, 0)], false, false), 6);
        assert_eq!(shape_for(&[(N, 0), (W, 0)], false, false), 8);
        // Powered rails don't bend.
        assert_eq!(shape_for(&[(N, 0), (W, 0)], true, false), 0);
        assert_eq!(shape_for(&[], false, true), 1);
    }

    #[test]
    fn segments_meet_at_block_edges() {
        let (a, b) = segment(IVec3::ZERO, 2);
        assert_eq!(a, Vec3::new(1.0, 1.0, 0.5));
        assert_eq!(b, Vec3::new(0.0, 0.0, 0.5));
        let (a, b) = segment(IVec3::ZERO, 6);
        assert_eq!(a, Vec3::new(0.5, 0.0, 1.0));
        assert_eq!(b, Vec3::new(1.0, 0.0, 0.5));
    }
}
