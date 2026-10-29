//! Storage that opens like the chest: drawers slide out, a shulker box's lid rises and turns,
//! a safe's door swings, a barrel's lid lifts, a decorated pot wobbles. While open each one is
//! hollow and dark inside, with the first few stacks it holds lying on its floor; a decorated
//! pot shows what it holds poking out of its neck all the time.
//!
//! The moving boxes go through swing.rs (the block leaves the chunk mesh and the item renderer
//! draws it); this file says what each kind of storage is made of at a given openness.

use std::f32::consts::{FRAC_PI_2, PI};

use glam::{IVec3, Vec3};
use voxel::block::{Block, Shape};
use voxel::item::{Inventory, Look};

use crate::item_render::GpuItem;
use crate::survival::{Container, Open};
use crate::swing::Part;
use crate::{Game, Screen};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Store {
    #[default]
    Chest,
    Drawer,
    Shulker,
    Safe,
    Barrel,
    Pot,
}

/// The storage kind of a block, if it opens like one.
pub(crate) fn store_of(name: &str) -> Option<Store> {
    Some(match name {
        "chest" => Store::Chest,
        "safe" => Store::Safe,
        "barrel" => Store::Barrel,
        "decorated_pot" => Store::Pot,
        n if n.ends_with("_drawer") => Store::Drawer,
        n if n.ends_with("shulker_box") => Store::Shulker,
        _ => return None,
    })
}

/// How a piece of storage looks at some openness: its boxes in a frame whose front is +z,
/// turned `facing` quarter turns (the way swing.rs turns doors), and where its things lie.
pub(crate) struct StoreLook {
    pub facing: u8,
    pub parts: Vec<Part>,
    /// Texture layers: side, top, bottom, front (0 = no front).
    pub layers: [f32; 4],
    /// The dark inside: floor and top of the hollow (0..1); top 0 for none.
    pub cavity: (f32, f32),
    /// Where its things lie: area x0, z0, x1, z1 and floor height (pixels), and how far the
    /// floor has moved (a drawer sliding out).
    pub area: [f32; 5],
    pub shift: Vec3,
    /// How many stacks show and how high a pile of blocks gets.
    pub stacks: usize,
    pub pile: usize,
}

/// Where the shown stacks lie (0..1 across the floor) and how they are turned: back ones
/// first, where a player standing at the front sees the floor.
/// They stay a flat item's half-length away from the walls.
const SPOTS: [(f32, f32, f32); 6] = [(0.48, 0.28, 0.9), (0.26, 0.3, 2.6), (0.72, 0.3, 0.1), (0.28, 0.7, 0.3), (0.72, 0.72, 1.9), (0.5, 0.52, 2.2)];

fn still(b: [f32; 6]) -> Part {
    Part { b, pivot: Vec3::splat(8.0), yaw: 0.0, pitch: 0.0, shift: Vec3::ZERO, inset: false }
}

/// A floor and four walls one pixel thick, `h` pixels high, from `y0`.
fn hollow(y0: f32, h: f32, v: &mut Vec<Part>) {
    let y1 = y0 + h;
    for b in [
        [1.0, y0, 1.0, 15.0, y0 + 1.0, 15.0],
        [0.0, y0, 0.0, 1.0, y1, 16.0],
        [15.0, y0, 0.0, 16.0, y1, 16.0],
        [1.0, y0, 0.0, 15.0, y1, 1.0],
        [1.0, y0, 15.0, 15.0, y1, 16.0],
    ] {
        v.push(still(b));
    }
}

/// The look of `store` (block `bl`) `o` of the way open (0..1, already eased).
pub(crate) fn look(store: Store, bl: &Block, o: f32) -> StoreLook {
    let f = |i: usize| bl.faces[i] as f32;
    let mut v = Vec::new();
    let mut look = StoreLook {
        facing: 0,
        parts: Vec::new(),
        layers: [f(0), f(2), f(3), 0.0],
        cavity: (1.0 / 16.0, 10.0 / 16.0),
        area: [2.0, 2.0, 14.0, 14.0, 1.0],
        shift: Vec3::ZERO,
        stacks: 6,
        pile: 3,
    };
    match store {
        Store::Chest => {
            hollow(0.0, 10.0, &mut v);
            v.push(Part { pivot: Vec3::new(8.0, 10.0, 0.0), pitch: -o * 75f32.to_radians(), ..still([0.0, 10.0, 0.0, 16.0, 16.0, 16.0]) });
            look.layers[3] = f(4);
        }
        Store::Barrel => {
            // Deep: what it holds sits on a false floor halfway down, where it can be seen.
            hollow(0.0, 15.0, &mut v);
            v.push(still([1.0, 7.0, 1.0, 15.0, 8.0, 15.0]));
            v.push(Part { pivot: Vec3::new(8.0, 15.0, 0.0), pitch: -o * 105f32.to_radians(), ..still([0.0, 15.0, 0.0, 16.0, 16.0, 16.0]) });
            look.cavity = (8.0 / 16.0, 15.0 / 16.0);
            look.area[4] = 8.0;
        }
        Store::Shulker => {
            // The lid rises and turns, like the shell of its namesake.
            hollow(0.0, 8.0, &mut v);
            v.push(Part { pivot: Vec3::new(8.0, 12.0, 8.0), yaw: o * PI * 0.75, shift: Vec3::new(0.0, o * 9.0, 0.0), ..still([0.0, 8.0, 0.0, 16.0, 16.0, 16.0]) });
            look.cavity = (1.0 / 16.0, 8.0 / 16.0);
            look.pile = 2;
        }
        Store::Safe => {
            // Thick walls, and the door swinging out on its left hinge.
            for b in [
                [0.0, 0.0, 0.0, 16.0, 2.0, 16.0],
                [0.0, 14.0, 0.0, 16.0, 16.0, 16.0],
                [0.0, 2.0, 0.0, 2.0, 14.0, 16.0],
                [14.0, 2.0, 0.0, 16.0, 14.0, 16.0],
                [2.0, 2.0, 0.0, 14.0, 14.0, 2.0],
            ] {
                v.push(still(b));
            }
            v.push(Part { pivot: Vec3::new(2.0, 8.0, 15.5), yaw: -o * 100f32.to_radians(), ..still([2.0, 2.0, 15.0, 14.0, 14.0, 16.0]) });
            look.layers[3] = f(4);
            look.cavity = (2.0 / 16.0, 14.0 / 16.0);
            look.area = [3.0, 3.0, 13.0, 12.0, 2.0];
            look.stacks = 4;
        }
        Store::Drawer => {
            // Two drawers; the top one slides out of the frame.
            let dir = (bl.variant & 7).min(3);
            look.facing = (dir + 2) % 4;
            // An Oriented block shows its front texture on the face it points to (`top` in data).
            look.layers = [f(0), f(0), f(0), f(2)];
            for b in [
                [0.0, 0.0, 0.0, 16.0, 8.0, 16.0],
                [0.0, 15.0, 0.0, 16.0, 16.0, 16.0],
                [0.0, 8.0, 0.0, 1.0, 15.0, 16.0],
                [15.0, 8.0, 0.0, 16.0, 15.0, 16.0],
                [1.0, 8.0, 0.0, 15.0, 15.0, 1.0],
            ] {
                v.push(still(b));
            }
            let out = Vec3::new(0.0, 0.0, o * 10.0);
            for b in [
                [1.0, 8.0, 15.0, 15.0, 15.0, 16.0],
                [2.0, 8.0, 2.0, 14.0, 9.0, 15.0],
                [1.0, 8.0, 1.0, 2.0, 13.0, 15.0],
                [14.0, 8.0, 1.0, 15.0, 13.0, 15.0],
                [2.0, 8.0, 1.0, 14.0, 13.0, 2.0],
            ] {
                v.push(Part { shift: out, inset: true, ..still(b) });
            }
            look.cavity = (9.0 / 16.0, 13.0 / 16.0);
            look.area = [3.0, 3.0, 13.0, 14.0, 9.0];
            look.shift = out;
            look.stacks = 4;
            look.pile = 2;
        }
        Store::Pot => {
            // No lid: it rocks on its base as it is opened and closed.
            let w = (o * 3.0 * PI).sin() * (1.0 - o * 0.6) * 0.14;
            for b in [[5.0, 0.0, 5.0, 11.0, 7.0, 11.0], [6.0, 7.0, 6.0, 10.0, 9.0, 10.0]] {
                v.push(Part { pivot: Vec3::new(8.0, 0.0, 8.0), pitch: w, yaw: w * 0.5, ..still(b) });
            }
            look.cavity = (0.0, 0.0);
            look.stacks = 0;
        }
    }
    look.parts = v;
    look
}

/// The quarter turns of a frame (as swing.rs turns door boxes) applied to an offset.
pub(crate) fn turn_vec(v: Vec3, turn: f32) -> Vec3 {
    let (c, s) = (turn.cos(), turn.sin());
    Vec3::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
}

/// Texture layers, flatness and size of an item shown inside storage.
fn item_look(g: &Game, st: &voxel::item::ItemStack) -> (bool, [f32; 3], f32) {
    match g.items.get(st.item).look {
        Look::Block(b) => {
            let bl = g.reg.get(b);
            if matches!(bl.shape, Shape::Cross | Shape::Torch | Shape::Rail | Shape::Model) {
                (true, [bl.faces[0] as f32; 3], 0.34)
            } else {
                (false, [bl.faces[0] as f32, bl.faces[2] as f32, bl.faces[3] as f32], 0.2)
            }
        }
        Look::Sprite(l) => (true, [(g.item_base + l) as f32; 3], 0.34),
    }
}

impl Game {
    /// The storage block this player has open (the real safe for the Terraria safe, whose
    /// contents live at a fixed spot), for the lid and for other players.
    pub(crate) fn open_store_block(&self) -> Option<IVec3> {
        match self.surv.open {
            Open::Chest(p) if self.screen == Screen::Inventory => {
                if p == crate::tr_utility::bank_pos(1) {
                    self.swings.safe_at
                } else {
                    Some(p)
                }
            }
            _ => None,
        }
    }

    /// Remember which safe was clicked: its contents are kept elsewhere.
    pub(crate) fn note_safe(&mut self) {
        let open = matches!(self.surv.open, Open::Chest(p) if p == crate::tr_utility::bank_pos(1)) && self.screen == Screen::Inventory;
        if !open {
            self.swings.safe_at = None;
        } else if self.swings.safe_at.is_none() {
            self.swings.safe_at = self.target.map(|h| h.block).filter(|b| self.reg.get(self.world.block_or_air(*b)).name == "safe");
        }
    }

    /// What storage at `pos` holds, if this player can see it.
    fn store_inv(&self, store: Store, pos: IVec3) -> Option<&Inventory> {
        let at = if store == Store::Safe && !self.surv.containers.contains_key(&pos) {
            // Every safe opens the same personal safe: only ours shows it. (A safe with its
            // own contents is the test hook's.)
            if self.open_store_block() != Some(pos) {
                return None;
            }
            crate::tr_utility::bank_pos(1)
        } else {
            pos
        };
        match self.surv.containers.get(&at)? {
            Container::Chest(i) => Some(i),
            Container::Furnace(_) => None,
        }
    }

    /// The first stacks of open storage, lying on its floor in the dark: flat items laid
    /// down, blocks as little cubes, a fuller stack as a small pile.
    pub(crate) fn store_contents(&self, store: Store, pos: IVec3, look: &StoreLook, centre: Vec3, light: (f32, f32), out: &mut Vec<GpuItem>) {
        if look.stacks == 0 {
            return;
        }
        let Some(inv) = self.store_inv(store, pos) else {
            return;
        };
        // Down in the box, away from the open top.
        let (sky, blk) = (light.0 * 0.7, light.1 * 0.75);
        let turn = -(look.facing as f32) * FRAC_PI_2;
        let [x0, z0, x1, z1, floor] = look.area;
        let h = (pos.x.wrapping_mul(73_856_093) ^ pos.y.wrapping_mul(19_349_663) ^ pos.z.wrapping_mul(83_492_791)) as u32;
        for (n, st) in inv.slots.iter().flatten().take(look.stacks.min(SPOTS.len())).enumerate() {
            let (flat, layers, size) = item_look(self, st);
            let (sx, sz, yaw) = SPOTS[n];
            // Each one lays its things out a little differently.
            let yaw = yaw + ((h >> (n * 3)) & 7) as f32 * 0.21;
            let copies = if st.count > 16 { 3 } else if st.count > 1 { 2 } else { 1 };
            let copies = if flat { copies } else { copies.min(look.pile) };
            for k in 0..copies {
                let kf = k as f32;
                // Flat things pile up; blocks sit side by side, the third on top of the two.
                let (lift, pitch) = if flat { (0.012 + kf * 0.018, -FRAC_PI_2) } else { (size * if k == 2 { 1.5 } else { 0.5 }, 0.0) };
                let tw = if flat { yaw + kf * 0.5 } else { yaw };
                let off = if flat {
                    Vec3::new(tw.cos(), 0.0, tw.sin()) * kf * 0.03
                } else {
                    Vec3::new(yaw.cos(), 0.0, yaw.sin()) * size * [0.0, 1.05, 0.5][k]
                };
                let px = x0 + sx * (x1 - x0);
                let pz = z0 + sz * (z1 - z0);
                let local = Vec3::new(px / 16.0 - 0.5, floor / 16.0 - 0.5 + lift, pz / 16.0 - 0.5) + off + look.shift / 16.0;
                let p = centre + turn_vec(local, turn);
                out.push(GpuItem {
                    pos: [p.x, p.y, p.z, size],
                    info: [tw + turn, if flat { 1.0 } else { 0.0 }, sky, blk],
                    layers: [layers[0], layers[1], layers[2], 0.0],
                    rot: [pitch, 0.0, 0.0, 0.0],
                    ..bytemuck::Zeroable::zeroed()
                });
            }
        }
    }

    /// What each decorated pot nearby holds, poking out of its neck.
    pub(crate) fn pot_instances(&self, cam_block: IVec3, out: &mut Vec<GpuItem>) {
        let Some(pot) = self.reg.id("decorated_pot") else {
            return;
        };
        let eye = self.body.eye();
        for (p, c) in &self.surv.containers {
            let Container::Chest(inv) = c else { continue };
            let Some(st) = inv.slots.first().copied().flatten() else { continue };
            if (p.as_vec3() - eye).length_squared() > 32.0 * 32.0 || self.world.block_or_air(*p) != pot {
                continue;
            }
            let l = self.world.light(*p);
            let (sky, blk) = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let (flat, layers, size) = item_look(self, &st);
            // Standing up in the neck, half of it inside; a block sits in the mouth.
            let (y, size) = if flat { (9.0 / 16.0, size * 0.7) } else { (9.6 / 16.0, 0.16) };
            let yaw = ((p.x * 31 + p.z * 17) & 7) as f32 * 0.4;
            let c = p.as_vec3() + Vec3::new(0.5, y, 0.5) - cam_block.as_vec3();
            out.push(GpuItem {
                pos: [c.x, c.y, c.z, size],
                info: [yaw, if flat { 1.0 } else { 0.0 }, sky, blk],
                layers: [layers[0], layers[1], layers[2], 0.0],
                rot: [0.0; 4],
                ..bytemuck::Zeroable::zeroed()
            });
        }
    }

    /// Light on storage: from above, or from in front of it when that is brighter.
    pub(crate) fn store_light(&self, pos: IVec3, bl: &Block, facing: u8) -> (f32, f32) {
        let front = turn_vec(Vec3::Z, -(facing as f32) * FRAC_PI_2).round().as_ivec3();
        let mut best = (0u8, 0u8);
        let at: &[IVec3] = if bl.opaque { &[IVec3::Y, front] } else { &[IVec3::ZERO] };
        for d in at {
            let l = self.world.light(pos + *d);
            best = (best.0.max(l >> 4), best.1.max(l & 15));
        }
        (best.0 as f32 / 15.0, best.1 as f32 / 15.0)
    }
}
