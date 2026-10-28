//! Doors, trapdoors and fence gates that swing open instead of snapping, and storage that opens
//! while it is in use: a chest's lid lifts, a drawer slides out (storage.rs has every kind).
//!
//! The world changes at once, so walking and redstone are as before; only the look lags. The
//! moving block is left out of the chunk mesh (`World::set_hidden`) and drawn by the item
//! renderer as a few boxes turning around a hinge. It comes back into the mesh as the swing
//! ends, and the boxes stay a moment longer so there is no gap while that mesh is built.
//!
//! Open storage is hollow: thin walls whose inner faces darken towards the bottom, a lid with
//! a dark underside, and the first few stacks it holds lying on its floor (from the same
//! container copy the screen shows, so a guest sees what the host sends). Storage another
//! player has open opens too (`PlayerState::chest`).

use std::f32::consts::FRAC_PI_2;

use glam::{IVec3, Vec3};
use voxel::block::Shape;
use voxel::BlockId;

use crate::item_render::GpuItem;
use crate::storage::{store_of, Store};
use crate::survival::Container;
use crate::{Game, Screen};

/// Before moving: the hidden mesh catches up while the boxes cover the still block.
const HOLD: f32 = 0.04;
const MOVE: f32 = 0.18;
const LID_MOVE: f32 = 0.25;
/// After moving: the boxes wait for the block's mesh to come back.
const TAIL: f32 = 0.15;

pub(crate) struct Swing {
    pos: IVec3,
    /// The block as it is now, once it has swung.
    id: BlockId,
    start: f32,
    /// Storage: Some(true) opening (and staying open), Some(false) closing.
    lid: Option<bool>,
    store: Store,
    shown: bool,
}

#[derive(Default)]
pub(crate) struct Swings {
    list: Vec<Swing>,
    /// The storage that is open: ours and other players'.
    chests: Vec<IVec3>,
    /// The safe we clicked (the Terraria safe keeps its contents elsewhere).
    pub(crate) safe_at: Option<IVec3>,
    /// Test hook MC_SWING=k: everything near the player swings once and stops k of the way.
    freeze: Option<f32>,
    tested: bool,
}

/// One box in the block's 0..16 frame (as if facing north), turning around `pivot` by
/// yaw (around y) and pitch (around x), then moved by `shift` (pixels). `inset`: a box inside
/// a box (a drawer), whose own sides are not inner faces.
pub(crate) struct Part {
    pub b: [f32; 6],
    pub pivot: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub shift: Vec3,
    pub inset: bool,
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
            self.swings.list.push(Swing { pos, id, start: self.now, lid: None, store: Store::Chest, shown: false });
        }
        // The storage being looked into opens; the rest closes.
        self.note_safe();
        let mut chests = Vec::new();
        if !self.settings.reduce_motion {
            chests.extend(self.open_store_block());
            let dim = self.dim.as_u8();
            for r in self.net.remotes.values() {
                if let Some(p) = r.state.chest {
                    if r.state.dim == dim {
                        chests.push(IVec3::from(p));
                    }
                }
            }
            chests.retain(|p| store_of(&self.reg.get(self.world.block_or_air(*p)).name).is_some());
            chests.sort_by_key(|p| p.to_array());
            chests.dedup();
        }
        if chests != self.swings.chests {
            for old in std::mem::take(&mut self.swings.chests) {
                if chests.contains(&old) {
                    continue;
                }
                // Close from wherever the lid got to.
                let up = self.swings.list.iter().find(|s| s.pos == old && s.lid == Some(true)).map(|s| smooth((self.now - s.start - HOLD) / LID_MOVE)).unwrap_or(0.0);
                self.swings.list.retain(|s| s.pos != old);
                let id = self.world.block_or_air(old);
                let store = store_of(&self.reg.get(id).name).unwrap_or_default();
                self.swings.list.push(Swing { pos: old, id, start: self.now - HOLD - (1.0 - up) * LID_MOVE, lid: Some(false), store, shown: false });
            }
            for &p in &chests {
                if self.swings.list.iter().any(|s| s.pos == p && s.lid == Some(true)) {
                    continue;
                }
                // Lift from wherever a closing lid got to.
                let down = self.swings.list.iter().find(|s| s.pos == p && s.lid == Some(false)).map(|s| 1.0 - smooth((self.now - s.start - HOLD) / LID_MOVE)).unwrap_or(0.0);
                self.swings.list.retain(|s| s.pos != p);
                self.world.set_hidden(p, true);
                let id = self.world.block_or_air(p);
                let start = if down > 0.0 { self.now - HOLD - down * LID_MOVE } else { self.now };
                let store = store_of(&self.reg.get(id).name).unwrap_or_default();
                self.swings.list.push(Swing { pos: p, id, start, lid: Some(true), store, shown: false });
            }
            self.swings.chests = chests;
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
            let centre = s.pos.as_vec3() + Vec3::splat(0.5) - cam_block.as_vec3();
            if let Some(up) = s.lid {
                let k = smooth(t / LID_MOVE);
                let look = crate::storage::look(s.store, bl, if up { k } else { 1.0 - k });
                let light = self.store_light(s.pos, bl, look.facing);
                push_parts(&mut out, &look.parts, centre, look.facing, light, look.layers, look.cavity, false);
                self.store_contents(s.store, s.pos, &look, centre, light, &mut out);
                continue;
            }
            let k = smooth(t / MOVE);
            let open = bl.variant & 4 != 0;
            let a = if open { k } else { 1.0 - k } * FRAC_PI_2;
            let facing = voxel::shapes::facing_of(bl.variant);
            let l = if bl.opaque { self.world.light(s.pos + IVec3::Y) } else { self.world.light(s.pos) };
            let light = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let layers = [bl.faces[0] as f32, bl.faces[2] as f32, bl.faces[3] as f32, 0.0];
            push_parts(&mut out, &parts_of(bl.shape, bl.variant, a), centre, facing, light, layers, (0.0, 0.0), true);
        }
        self.pot_instances(cam_block, &mut out);
        out
    }
}

/// The boxes of a moving block for the item renderer, turned `facing` quarter turns around its
/// centre. `turn_tex`: the texture turns with it, as the terrain turns doors; storage keeps
/// the faces the terrain gives it. `cavity`: floor and top of a dark inside (top 0: none).
#[allow(clippy::too_many_arguments)]
fn push_parts(out: &mut Vec<GpuItem>, parts: &[Part], centre: Vec3, facing: u8, light: (f32, f32), layers: [f32; 4], cavity: (f32, f32), turn_tex: bool) {
    let turn = -(facing as f32) * FRAC_PI_2;
    for p in parts {
        let piv = p.pivot / 16.0;
        // The block's own turn (as the terrain mesher turns it), around its centre.
        let at = centre + crate::storage::turn_vec(piv - Vec3::splat(0.5) + p.shift / 16.0, turn);
        out.push(GpuItem {
            pos: [at.x, at.y, at.z, 1.0],
            info: [turn + p.yaw, 0.0, light.0, light.1],
            layers: [layers[0], layers[1], layers[2], 0.0],
            // rot.z: texture quarter turns; rot.w: top of the dark inside (bmin.w its floor).
            rot: [p.pitch, 0.0, if turn_tex { facing as f32 } else { 0.0 }, cavity.1],
            bmin: [p.b[0] / 16.0, p.b[1] / 16.0, p.b[2] / 16.0, cavity.0],
            bmax: [p.b[3] / 16.0, p.b[4] / 16.0, p.b[5] / 16.0, layers[3]],
            // piv.w: 1 a block part, 2 a box inside a box.
            piv: [piv.x, piv.y, piv.z, if p.inset { 2.0 } else { 1.0 }],
        });
    }
}

impl Game {
    fn swing_test_hook(&mut self) {
        if self.swings.tested || !self.world_open || self.screen != Screen::Playing || self.now < 1.0 {
            return;
        }
        if let Ok(v) = std::env::var("MC_CHEST") {
            self.swings.tested = true;
            self.chest_test(&v);
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
        self.swings.list.push(Swing { pos: cp, id: chest, start: self.now, lid: Some(true), store: Store::Chest, shown: false });
        // Let the new swings start, then hold them.
        self.swings.freeze = None;
        let new = std::mem::take(&mut self.world.swings);
        for (pos, _, id) in new {
            self.world.set_hidden(pos, true);
            self.swings.list.push(Swing { pos, id, start: self.now, lid: None, store: Store::Chest, shown: false });
        }
        self.swings.freeze = Some(k);
    }
}

impl Game {
    /// Test hook MC_CHEST=items: a chest on the ground a little ahead holding those items
    /// (comma-separated names, "name*count" for a stack), its lid held open. MC_STORE=block
    /// puts other storage there instead (oak_drawer, shulker_box, barrel, decorated_pot...),
    /// MC_STORE_OPEN=k holds it k of the way open.
    fn chest_test(&mut self, list: &str) {
        let f = self.forward();
        let ahead = self.body.pos + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * (1.0 / (-self.pitch).tan().max(0.3)).clamp(1.2, 2.3);
        // At the player's feet, on something.
        let p = IVec3::new(ahead.x.floor() as i32, (self.body.pos.y + 0.01).floor() as i32, ahead.z.floor() as i32);
        if !self.reg.get(self.world.block_or_air(p - IVec3::Y)).solid {
            let stone = self.reg.id_or_air("stone");
            self.world.set_block(p - IVec3::Y, stone);
        }
        let name = std::env::var("MC_STORE").unwrap_or_else(|_| "chest".into());
        // A drawer faces the player (its variant: the way it points, 0 north .. 3 west).
        let base = self.reg.id_or_air(&name);
        let chest = if name.ends_with("_drawer") {
            let d = self.body.pos - p.as_vec3() - Vec3::splat(0.5);
            let dir = if d.x.abs() > d.z.abs() { if d.x > 0.0 { 1 } else { 3 } } else if d.z > 0.0 { 2 } else { 0 };
            let b = self.reg.get(base).base;
            self.reg.blocks.iter().position(|o| o.base == b && o.variant == dir).map(|i| i as BlockId).unwrap_or(base)
        } else {
            base
        };
        self.world.set_block(p, chest);
        let store = store_of(&name).unwrap_or_default();
        let mut inv = voxel::item::Inventory::new(if store == Store::Pot { 1 } else { 27 });
        for (i, e) in list.split(',').filter(|e| !e.is_empty()).enumerate() {
            let (name, n) = e.split_once('*').map(|(a, b)| (a, b.parse().unwrap_or(1))).unwrap_or((e, 1));
            if let (Some(id), Some(slot)) = (self.items.id(name.trim()), inv.slots.get_mut(i)) {
                *slot = Some(voxel::item::ItemStack::new(id, n));
            }
        }
        self.surv.containers.insert(p, Container::Chest(inv));
        self.world.set_hidden(p, true);
        if store != Store::Pot {
            self.swings.list.push(Swing { pos: p, id: chest, start: self.now, lid: Some(true), store, shown: false });
        } else {
            self.world.set_hidden(p, false);
        }
        self.swings.freeze = Some(std::env::var("MC_STORE_OPEN").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0));
        self.hide_hud = true;
    }
}

/// The moving boxes of a door, trapdoor or gate, `a` radians open, facing north.
fn parts_of(shape: Shape, variant: u8, a: f32) -> Vec<Part> {
    let still = |b: [f32; 6]| Part { b, pivot: Vec3::splat(8.0), yaw: 0.0, pitch: 0.0, shift: Vec3::ZERO, inset: false };
    match shape {
        // Hinged at the east edge of its north side.
        Shape::Door => vec![Part { b: [0.0, 0.0, 0.0, 16.0, 16.0, 3.0], pivot: Vec3::new(14.5, 8.0, 1.5), yaw: a, pitch: 0.0, shift: Vec3::ZERO, inset: false }],
        // Hinged along the south edge, swinging up (or down from the top half).
        Shape::Trapdoor => {
            if variant & 8 != 0 {
                vec![Part { b: [0.0, 13.0, 0.0, 16.0, 16.0, 16.0], pivot: Vec3::new(8.0, 14.5, 14.5), yaw: 0.0, pitch: -a, shift: Vec3::ZERO, inset: false }]
            } else {
                vec![Part { b: [0.0, 0.0, 0.0, 16.0, 3.0, 16.0], pivot: Vec3::new(8.0, 1.5, 14.5), yaw: 0.0, pitch: a, shift: Vec3::ZERO, inset: false }]
            }
        }
        // Two leaves folding back against their posts.
        Shape::FenceGate => {
            let mut v = vec![still([0.0, 5.0, 7.0, 2.0, 16.0, 9.0]), still([14.0, 5.0, 7.0, 16.0, 16.0, 9.0])];
            for b in [[2.0, 6.0, 7.0, 8.0, 9.0, 9.0], [2.0, 12.0, 7.0, 8.0, 15.0, 9.0], [6.0, 9.0, 7.0, 8.0, 12.0, 9.0]] {
                v.push(Part { b, pivot: Vec3::new(1.0, 8.0, 8.0), yaw: -a, pitch: 0.0, shift: Vec3::ZERO, inset: false });
            }
            for b in [[8.0, 6.0, 7.0, 14.0, 9.0, 9.0], [8.0, 12.0, 7.0, 14.0, 15.0, 9.0], [8.0, 9.0, 7.0, 10.0, 12.0, 9.0]] {
                v.push(Part { b, pivot: Vec3::new(15.0, 8.0, 8.0), yaw: a, pitch: 0.0, shift: Vec3::ZERO, inset: false });
            }
            v
        }
        _ => Vec::new(),
    }
}
