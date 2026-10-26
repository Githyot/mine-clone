//! Doors, trapdoors and fence gates that swing open instead of snapping, and a chest lid that
//! lifts while the chest is open (Lote 6 leftover, game feel).
//!
//! The world changes at once, so walking and redstone are as before; only the look lags. The
//! moving block is left out of the chunk mesh (`World::set_hidden`) and drawn by the item
//! renderer as a few boxes turning around a hinge. It comes back into the mesh as the swing
//! ends, and the boxes stay a moment longer so there is no gap while that mesh is built.

use std::f32::consts::FRAC_PI_2;

use glam::{IVec3, Vec3};
use voxel::block::Shape;
use voxel::BlockId;

use crate::item_render::GpuItem;
use crate::survival::Open;
use crate::{Game, Screen};

/// Before moving: the hidden mesh catches up while the boxes cover the still block.
const HOLD: f32 = 0.04;
const MOVE: f32 = 0.18;
const LID_MOVE: f32 = 0.25;
/// After moving: the boxes wait for the block's mesh to come back.
const TAIL: f32 = 0.15;
const LID_ANGLE: f32 = 75.0 * std::f32::consts::PI / 180.0;

pub(crate) struct Swing {
    pos: IVec3,
    /// The block as it is now, once it has swung.
    id: BlockId,
    start: f32,
    /// A chest lid: Some(true) lifting (and staying up), Some(false) closing.
    lid: Option<bool>,
    shown: bool,
}

#[derive(Default)]
pub(crate) struct Swings {
    list: Vec<Swing>,
    /// The chest whose lid is up.
    chest: Option<IVec3>,
    /// Test hook MC_SWING=k: everything near the player swings once and stops k of the way.
    freeze: Option<f32>,
    tested: bool,
}

/// One box in the block's 0..16 frame (as if facing north), turning around `pivot` by
/// yaw (around y) and pitch (around x).
struct Part {
    b: [f32; 6],
    pivot: Vec3,
    yaw: f32,
    pitch: f32,
}

fn smooth(k: f32) -> f32 {
    let k = k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

impl Game {
    pub(crate) fn swing_update(&mut self) {
        self.swing_test_hook();
        let new = std::mem::take(&mut self.world.swings);
        let eye = self.body.eye();
        for (pos, _from, id) in new {
            if self.settings.reduce_motion || (pos.as_vec3() - eye).length_squared() > 48.0 * 48.0 {
                continue;
            }
            self.swings.list.retain(|s| s.pos != pos);
            self.world.set_hidden(pos, true);
            self.swings.list.push(Swing { pos, id, start: self.now, lid: None, shown: false });
        }
        // The chest being looked into lifts its lid; the last one closes.
        let chest = match self.surv.open {
            Open::Chest(p) if self.screen == Screen::Inventory && self.reg.get(self.world.block_or_air(p)).name == "chest" && !self.settings.reduce_motion => Some(p),
            _ => None,
        };
        if chest != self.swings.chest {
            if let Some(old) = self.swings.chest {
                // Close from wherever the lid got to.
                let up = self.swings.list.iter().find(|s| s.pos == old && s.lid == Some(true)).map(|s| smooth((self.now - s.start - HOLD) / LID_MOVE)).unwrap_or(0.0);
                self.swings.list.retain(|s| s.pos != old);
                let id = self.world.block_or_air(old);
                self.swings.list.push(Swing { pos: old, id, start: self.now - HOLD - (1.0 - up) * LID_MOVE, lid: Some(false), shown: false });
            }
            if let Some(p) = chest {
                self.swings.list.retain(|s| s.pos != p);
                self.world.set_hidden(p, true);
                let id = self.world.block_or_air(p);
                self.swings.list.push(Swing { pos: p, id, start: self.now, lid: Some(true), shown: false });
            }
            self.swings.chest = chest;
        }
        if self.swings.freeze.is_some() {
            return;
        }
        let now = self.now;
        let mut done = Vec::new();
        for s in self.swings.list.iter_mut() {
            let t = now - s.start;
            let len = if s.lid.is_some() { LID_MOVE } else { MOVE };
            if self.world.block_or_air(s.pos) != s.id {
                done.push(s.pos);
                s.start = f32::NEG_INFINITY;
            } else if s.lid != Some(true) && t > HOLD + len && !s.shown {
                s.shown = true;
                self.world.set_hidden(s.pos, false);
            }
        }
        for p in done {
            self.world.set_hidden(p, false);
        }
        self.swings.list.retain(|s| s.start.is_finite() && (s.lid == Some(true) || now - s.start <= HOLD + if s.lid.is_some() { LID_MOVE } else { MOVE } + TAIL));
    }

    /// The boxes of every swinging block, for the item renderer.
    pub(crate) fn swing_instances(&self, cam_block: IVec3) -> Vec<GpuItem> {
        let mut out = Vec::new();
        for s in &self.swings.list {
            let bl = self.reg.get(s.id);
            let len = if s.lid.is_some() { LID_MOVE } else { MOVE };
            let t = self.swings.freeze.map(|k| k * len).unwrap_or(self.now - s.start - HOLD);
            let (facing, parts) = match s.lid {
                Some(up) => {
                    let k = smooth(t / LID_MOVE);
                    let a = if up { k } else { 1.0 - k } * LID_ANGLE;
                    let body = Part { b: [0.0, 0.0, 0.0, 16.0, 10.0, 16.0], pivot: Vec3::splat(8.0), yaw: 0.0, pitch: 0.0 };
                    let lid = Part { b: [0.0, 10.0, 0.0, 16.0, 16.0, 16.0], pivot: Vec3::new(8.0, 10.0, 0.0), yaw: 0.0, pitch: -a };
                    (0, vec![body, lid])
                }
                None => {
                    let k = smooth(t / MOVE);
                    let open = bl.variant & 4 != 0;
                    let a = if open { k } else { 1.0 - k } * FRAC_PI_2;
                    (voxel::shapes::facing_of(bl.variant), parts_of(bl.shape, bl.variant, a))
                }
            };
            let l = if bl.opaque { self.world.light(s.pos + IVec3::Y) } else { self.world.light(s.pos) };
            let (sky, blk) = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let front = if s.lid.is_some() { bl.faces[4] as f32 } else { 0.0 };
            let turn = -(facing as f32) * FRAC_PI_2;
            let centre = s.pos.as_vec3() + Vec3::splat(0.5) - cam_block.as_vec3();
            for p in parts {
                let piv = p.pivot / 16.0;
                let off = piv - Vec3::splat(0.5);
                // The block's own turn (as the terrain mesher turns it), around its centre.
                let (c, sn) = (turn.cos(), turn.sin());
                let off = Vec3::new(off.x * c + off.z * sn, off.y, -off.x * sn + off.z * c);
                let at = centre + off;
                out.push(GpuItem {
                    pos: [at.x, at.y, at.z, 1.0],
                    info: [turn + p.yaw, 0.0, sky, blk],
                    layers: [bl.faces[0] as f32, bl.faces[2] as f32, bl.faces[3] as f32, 0.0],
                    rot: [p.pitch, 0.0, facing as f32, 0.0],
                    bmin: [p.b[0] / 16.0, p.b[1] / 16.0, p.b[2] / 16.0, 0.0],
                    bmax: [p.b[3] / 16.0, p.b[4] / 16.0, p.b[5] / 16.0, front],
                    piv: [piv.x, piv.y, piv.z, 1.0],
                });
            }
        }
        out
    }
}

impl Game {
    fn swing_test_hook(&mut self) {
        if self.swings.tested || !self.world_open || self.screen != Screen::Playing || self.now < 1.0 {
            return;
        }
        let Some(k) = std::env::var("MC_SWING").ok().and_then(|v| v.parse::<f32>().ok()) else {
            self.swings.tested = true;
            return;
        };
        self.swings.tested = true;
        let c = self.body.pos.floor().as_ivec3();
        // A gate and a chest beside the first door (the shapes scene's).
        let mut door = None;
        for x in -8..=8 {
            for y in -8..=3 {
                for z in -14..=2 {
                    let p = c + IVec3::new(x, y, z);
                    let b = self.reg.get(self.world.block_or_air(p));
                    if door.is_none() && b.shape == Shape::Door && b.variant & 8 == 0 {
                        door = Some(p);
                    }
                }
            }
        }
        let d = door.unwrap_or(c);
        let gate = self.reg.id_or_air("oak_fence_gate");
        self.world.set_block(d + IVec3::new(-2, 0, 0), gate);
        let chest = self.reg.id_or_air("chest");
        let cp = d + IVec3::new(0, 0, 2);
        self.world.set_block(cp, chest);
        for x in -8..=8 {
            for y in -8..=3 {
                for z in -14..=2 {
                    let p = c + IVec3::new(x, y, z);
                    let b = self.reg.get(self.world.block_or_air(p));
                    if matches!(b.shape, Shape::Door | Shape::Trapdoor | Shape::FenceGate) {
                        let id = self.world.block_or_air(p);
                        let flipped = self.reg.blocks.iter().position(|o| o.base == b.base && o.variant == b.variant ^ 4).map(|i| i as BlockId).unwrap_or(id);
                        self.world.set_block(p, flipped);
                    }
                }
            }
        }
        self.world.set_hidden(cp, true);
        self.swings.list.push(Swing { pos: cp, id: chest, start: self.now, lid: Some(true), shown: false });
        // Let the new swings start, then hold them.
        self.swings.freeze = None;
        let new = std::mem::take(&mut self.world.swings);
        for (pos, _, id) in new {
            self.world.set_hidden(pos, true);
            self.swings.list.push(Swing { pos, id, start: self.now, lid: None, shown: false });
        }
        self.swings.freeze = Some(k);
    }
}

/// The moving boxes of a door, trapdoor or gate, `a` radians open, facing north.
fn parts_of(shape: Shape, variant: u8, a: f32) -> Vec<Part> {
    let still = |b: [f32; 6]| Part { b, pivot: Vec3::splat(8.0), yaw: 0.0, pitch: 0.0 };
    match shape {
        // Hinged at the east edge of its north side.
        Shape::Door => vec![Part { b: [0.0, 0.0, 0.0, 16.0, 16.0, 3.0], pivot: Vec3::new(14.5, 8.0, 1.5), yaw: a, pitch: 0.0 }],
        // Hinged along the south edge, swinging up (or down from the top half).
        Shape::Trapdoor => {
            if variant & 8 != 0 {
                vec![Part { b: [0.0, 13.0, 0.0, 16.0, 16.0, 16.0], pivot: Vec3::new(8.0, 14.5, 14.5), yaw: 0.0, pitch: -a }]
            } else {
                vec![Part { b: [0.0, 0.0, 0.0, 16.0, 3.0, 16.0], pivot: Vec3::new(8.0, 1.5, 14.5), yaw: 0.0, pitch: a }]
            }
        }
        // Two leaves folding back against their posts.
        Shape::FenceGate => {
            let mut v = vec![still([0.0, 5.0, 7.0, 2.0, 16.0, 9.0]), still([14.0, 5.0, 7.0, 16.0, 16.0, 9.0])];
            for b in [[2.0, 6.0, 7.0, 8.0, 9.0, 9.0], [2.0, 12.0, 7.0, 8.0, 15.0, 9.0], [6.0, 9.0, 7.0, 8.0, 12.0, 9.0]] {
                v.push(Part { b, pivot: Vec3::new(1.0, 8.0, 8.0), yaw: -a, pitch: 0.0 });
            }
            for b in [[8.0, 6.0, 7.0, 14.0, 9.0, 9.0], [8.0, 12.0, 7.0, 14.0, 15.0, 9.0], [8.0, 9.0, 7.0, 10.0, 12.0, 9.0]] {
                v.push(Part { b, pivot: Vec3::new(15.0, 8.0, 8.0), yaw: a, pitch: 0.0 });
            }
            v
        }
        _ => Vec::new(),
    }
}
