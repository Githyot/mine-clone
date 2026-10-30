//! Double chests: two chests side by side, facing the same way, join into one (like the
//! classic game). The halves are variants of the chest (`Turn::Chest`: | 4 the left half,
//! | 8 the right half, as seen from the front) and keep their own 27 slots in the world; while
//! a double chest is open its left half holds all 54 and the right half's are given back when
//! it closes, so the screen, hoppers and saving all keep working on one inventory each.

use glam::IVec3;
use voxel::block::{chest_left_dir, Turn, CHEST_LEFT, CHEST_RIGHT};
use voxel::item::Inventory;
use voxel::BlockId;

use crate::survival::{Container, Open};
use crate::Game;

/// Slots in one half.
pub(crate) const HALF: usize = 27;

/// The other half of the double chest at `pos`, if it is one and its partner is still there.
pub(crate) fn chest_partner(reg: &voxel::block::BlockRegistry, get: impl Fn(IVec3) -> BlockId, pos: IVec3) -> Option<IVec3> {
    let b = reg.get(get(pos));
    let half = b.variant & (CHEST_LEFT | CHEST_RIGHT);
    if b.turn != Turn::Chest || half == 0 {
        return None;
    }
    let l = IVec3::from(chest_left_dir(b.variant));
    // The left half's partner is on its right, and the other way round.
    let other = if half == CHEST_LEFT { pos - l } else { pos + l };
    let o = reg.get(get(other));
    (o.base == b.base && o.variant == b.variant ^ (CHEST_LEFT | CHEST_RIGHT)).then_some(other)
}

impl Game {
    /// The other half of a double chest here.
    pub(crate) fn chest_other(&self, pos: IVec3) -> Option<IVec3> {
        chest_partner(&self.reg, |p| self.world.block_or_air(p), pos)
    }

    /// A chest just put down at `pos` joins a single chest beside it that faces the same way
    /// (the one clicked first); sneaking keeps it on its own.
    pub(crate) fn pair_chest(&mut self, pos: IVec3, clicked: IVec3) {
        let id = self.world.block_or_air(pos);
        let b = self.reg.get(id);
        if b.turn != Turn::Chest || b.variant & (CHEST_LEFT | CHEST_RIGHT) != 0 {
            return;
        }
        let v = b.variant & 3;
        let l = IVec3::from(chest_left_dir(v));
        let mut sides = [(pos + l, CHEST_RIGHT, CHEST_LEFT), (pos - l, CHEST_LEFT, CHEST_RIGHT)];
        if sides[1].0 == clicked {
            sides.swap(0, 1);
        }
        for (other, me, it) in sides {
            let o = self.world.block_or_air(other);
            let ob = self.reg.get(o);
            if ob.base == b.base && ob.variant == v {
                let (mine, theirs) = (self.reg.variant(id, v | me), self.reg.variant(id, v | it));
                self.world.set_block(pos, mine);
                self.world.set_block(other, theirs);
                self.world.mesh_now(other);
                return;
            }
        }
    }

    /// The chest at `pos` being opened: its contents (a world-made chest's loot the first time),
    /// both halves' in the left one for a double chest. Returns where the open inventory is.
    pub(crate) fn chest_open_at(&mut self, pos: IVec3) -> IVec3 {
        let Some(other) = self.chest_other(pos) else {
            self.chest_fill(pos);
            return pos;
        };
        let left = self.reg.get(self.world.block_or_air(pos)).variant & CHEST_LEFT != 0;
        let (main, side) = if left { (pos, other) } else { (other, pos) };
        self.chest_fill(main);
        self.chest_fill(side);
        let rest = match self.surv.containers.remove(&side) {
            Some(Container::Chest(i)) => i.slots,
            Some(c) => {
                self.surv.containers.insert(side, c);
                Vec::new()
            }
            None => Vec::new(),
        };
        if let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&main) {
            // Already joined (saved while open): the right half's things fill gaps.
            let mut extra = Vec::new();
            if inv.slots.len() < 2 * HALF {
                inv.slots.resize(HALF, None);
                let mut rest = rest.into_iter();
                inv.slots.extend((0..HALF).map(|_| rest.next().flatten()));
                extra.extend(rest.flatten());
            } else {
                extra.extend(rest.into_iter().flatten());
            }
            let mut spill = Vec::new();
            for s in extra {
                match inv.slots.iter_mut().find(|s| s.is_none()) {
                    Some(slot) => *slot = Some(s),
                    None => spill.push(s),
                }
            }
            let c = main.as_vec3() + glam::Vec3::splat(0.5);
            for s in spill {
                self.pop_drop(c, s);
            }
        }
        main
    }

    /// A chest without contents yet gets them (a world-made chest comes filled).
    fn chest_fill(&mut self, pos: IVec3) {
        if !self.surv.containers.contains_key(&pos) {
            let inv = self.structure_loot(pos);
            self.surv.containers.insert(pos, Container::Chest(inv));
        }
    }

    /// Give a double chest's right half its slots back (on closing, or when the pair broke).
    pub(crate) fn chest_split(&mut self, main: IVec3) {
        let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&main) else { return };
        if inv.slots.len() <= HALF || self.reg.get(self.world.block_or_air(main)).turn != Turn::Chest {
            return;
        }
        let back: Vec<_> = inv.slots.split_off(HALF);
        match self.chest_other(main) {
            Some(side) => {
                self.surv.containers.insert(side, Container::Chest(Inventory { slots: back }));
            }
            None => {
                // The other half is gone: what was in it falls out.
                let c = main.as_vec3() + glam::Vec3::splat(0.5);
                for s in back.into_iter().flatten() {
                    self.pop_drop(c, s);
                }
            }
        }
    }

    /// A block at `pos` was broken: a chest beside it that was the other half of a joined
    /// double chest gives back (drops) what that half held.
    pub(crate) fn chest_neighbours_split(&mut self, pos: IVec3) {
        for d in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
            self.chest_split(pos + d);
        }
    }

    /// The open double chest closes.
    pub(crate) fn chest_closed(&mut self) {
        if let Open::Chest(p) = self.surv.open {
            self.chest_split(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use voxel::block::{BlockRegistry, FRONT_FACE};

    fn reg() -> BlockRegistry {
        crate::game_data(crate::embedded::BLOCKS_RON, crate::embedded::ITEMS_RON, crate::embedded::RECIPES_RON).unwrap().0
    }

    #[test]
    fn turning_blocks() {
        let reg = reg();
        let turn = |n: &str| reg.get(reg.id(n).unwrap()).turn;
        for n in ["furnace", "crafting_table", "smithing_table", "beehive", "safe"] {
            assert_eq!(turn(n), Turn::Facing, "{n}");
        }
        assert_eq!(turn("chest"), Turn::Chest);
        for n in ["oak_log", "spruce_log", "crimson_stem", "hay_block", "purpur_pillar"] {
            assert_eq!(turn(n), Turn::Axis, "{n}");
        }
        for n in ["stone", "oak_planks", "glass", "barrel", "grass_block", "tnt"] {
            assert_eq!(turn(n), Turn::None, "{n}");
        }
        let names: Vec<&str> = reg.blocks.iter().filter(|b| b.variant == 0 && b.turn != Turn::None).map(|b| b.name.as_str()).collect();
        println!("{} turning blocks: {names:?}", names.len());
    }

    #[test]
    fn turned_variants_keep_the_name_and_move_the_front() {
        let reg = reg();
        let f = reg.id("furnace").unwrap();
        let front = reg.get(f).faces[4];
        for v in 0..4u8 {
            let b = reg.get(reg.variant(f, v));
            assert_eq!(b.name, "furnace");
            assert_eq!(b.base, f);
            assert!(reg.is_opaque(reg.variant(f, v)));
            assert_eq!(b.faces[FRONT_FACE[v as usize]], front);
            assert_eq!(b.faces.iter().filter(|&&l| l == front).count(), 1);
            assert_eq!(reg.id(&b.key), Some(reg.variant(f, v)));
        }
        // A log on its side shows its rings east and west.
        let log = reg.id("oak_log").unwrap();
        let top = reg.get(log).faces[2];
        let x = reg.get(reg.variant(log, 1));
        assert_eq!((x.faces[0], x.faces[1]), (top, reg.get(log).faces[3]));
        assert_ne!(x.uv_turn, 0);
        assert_eq!(x.name, "oak_log");
    }

    #[test]
    fn chests_side_by_side_pair_and_split() {
        let reg = reg();
        let chest = reg.id("chest").unwrap();
        let mut m: HashMap<IVec3, BlockId> = HashMap::new();
        // Facing south (placed looking north): left half to the west.
        let (a, b) = (IVec3::new(0, 0, 0), IVec3::new(1, 0, 0));
        m.insert(a, reg.variant(chest, CHEST_LEFT));
        m.insert(b, reg.variant(chest, CHEST_RIGHT));
        let get = |p: IVec3| *m.get(&p).unwrap_or(&voxel::AIR);
        assert_eq!(chest_partner(&reg, get, a), Some(b));
        assert_eq!(chest_partner(&reg, get, b), Some(a));
        let tex = |id: BlockId, f: usize| reg.textures[reg.get(id).faces[f] as usize].clone();
        assert_eq!(tex(get(a), 4), "chest_front_left");
        assert_eq!(tex(get(b), 4), "chest_front_right");
        assert_eq!(tex(get(a), 2), "chest_top_w");
        assert_eq!(tex(get(b), 2), "chest_top_e");
        // Facing east: the left half is to the south.
        let (c, d) = (IVec3::new(5, 0, 1), IVec3::new(5, 0, 0));
        m.insert(c, reg.variant(chest, 3 | CHEST_LEFT));
        m.insert(d, reg.variant(chest, 3 | CHEST_RIGHT));
        let get = |p: IVec3| *m.get(&p).unwrap_or(&voxel::AIR);
        assert_eq!(chest_partner(&reg, get, c), Some(d));
        assert_eq!(tex(get(c), 2), "chest_top_s");
        assert_eq!(tex(get(c), 0), "chest_front_left");
        // Different facings never pair.
        m.insert(d, reg.variant(chest, 2 | CHEST_RIGHT));
        let get = |p: IVec3| *m.get(&p).unwrap_or(&voxel::AIR);
        assert_eq!(chest_partner(&reg, get, c), None);
        // The pictures exist.
        for t in ["front_left", "front_right", "side_left", "side_right", "top_w", "top_e", "top_n", "top_s"] {
            let path = format!("{}/../../packs/vanilla/textures/block/chest_{t}.png", env!("CARGO_MANIFEST_DIR"));
            assert!(std::path::Path::new(&path).exists(), "{path}");
        }
    }

    #[test]
    fn saved_by_key() {
        let reg = reg();
        let chest = reg.id("chest").unwrap();
        let mut c = voxel::chunk::Chunk::new(voxel::chunk::ChunkPos::new(0, 0));
        c.set_w(1, 64, 1, reg.variant(chest, 2 | CHEST_LEFT));
        let back = voxel::save::decode_chunk(&reg, c.pos, &voxel::save::encode_chunk(&reg, &c)).unwrap();
        assert_eq!(back.get_w(1, 64, 1), reg.variant(chest, 2 | CHEST_LEFT));
    }
}

impl Game {
    /// Test hook MC_TURNS=1 (with --screenshot): blocks put down by the real placing code from
    /// different sides on a stone floor ahead: furnaces facing three ways, double chests along x
    /// and z, a lone chest, logs standing and lying. MC_TURNS=open also holds the double chest
    /// open with things inside.
    pub(crate) fn turns_test(&mut self, mode: &str) {
        use glam::Vec3;
        let c = self.body.pos.floor().as_ivec3();
        let g = |n: &str| self.reg.id_or_air(n);
        let (stone, furnace, chest, log, table) = (g("stone"), g("furnace"), g("chest"), g("oak_log"), g("crafting_table"));
        for x in -9..=9 {
            for z in -12..=1 {
                self.world.set_block(c + IVec3::new(x, -1, z), stone);
                for y in 0..5 {
                    self.world.set_block(c + IVec3::new(x, y, z), voxel::AIR);
                }
            }
        }
        let (keep_pos, keep_yaw, keep_pitch) = (self.body.pos, self.yaw, self.pitch);
        // Stand `from` (a way: x, z) of block `p` and put `id` on the floor there, or against
        // the face `face` of the block at `p`.
        let put = |g: &mut Game, p: IVec3, from: [i32; 2], id: BlockId, face: Option<IVec3>| {
            let (at, feet) = match face {
                None => {
                    let at = p.as_vec3() + Vec3::new(0.5, 1.0, 0.5);
                    (at, at + Vec3::new(from[0] as f32 * 2.0, 0.6, from[1] as f32 * 2.0))
                }
                Some(n) => {
                    let at = p.as_vec3() + Vec3::splat(0.5) + n.as_vec3() * 0.5;
                    (at, at + n.as_vec3() * 2.0 + Vec3::new(from[0] as f32 * 0.3, -1.3, from[1] as f32 * 0.3))
                }
            };
            g.body.pos = feet;
            let eye = g.body.eye();
            let d = (at - eye).normalize();
            g.yaw = (-d.x).atan2(-d.z);
            g.pitch = d.y.asin();
            let fwd = g.forward();
            let world = &g.world;
            g.target = voxel::raycast::raycast(eye, fwd, 8.0, |q| world.block(q).is_some_and(|b| b != voxel::AIR));
            let placed = g.place_block_as(id);
            log::info!("turns test: {} at {p} -> {placed:?}", g.reg.get(id).name);
        };
        let o = |x: i32, z: i32| c + IVec3::new(x, 0, z);
        let floor = |p: IVec3| p - IVec3::Y;
        // Furnaces: put down from the south, the east and the west.
        put(self, floor(o(-6, -5)), [0, 1], furnace, None);
        put(self, floor(o(-4, -5)), [1, 0], furnace, None);
        put(self, floor(o(-8, -5)), [-1, 0], furnace, None);
        put(self, floor(o(-6, -8)), [1, 1], table, None);
        // A double chest from the south, one along z from the east, a lone one from the west.
        put(self, floor(o(0, -5)), [0, 1], chest, None);
        put(self, floor(o(1, -5)), [0, 1], chest, None);
        put(self, floor(o(5, -6)), [1, 0], chest, None);
        put(self, floor(o(5, -7)), [1, 0], chest, None);
        put(self, floor(o(-2, -8)), [-1, 0], chest, None);
        // Logs: on the floor (standing), against a stone's west face (along x) and its south face (along z).
        self.world.set_block(o(7, -3), stone);
        self.world.set_block(o(3, -10), stone);
        put(self, floor(o(8, -3)), [0, 1], log, None);
        put(self, o(7, -3), [0, 1], log, Some(IVec3::NEG_X));
        put(self, o(3, -10), [1, 0], log, Some(IVec3::Z));
        self.body.pos = keep_pos;
        (self.yaw, self.pitch) = (keep_yaw, keep_pitch);
        // MC_TURNS_CAM=x,y,z,yaw,pitch: where the camera goes, from the start (degrees).
        let cam: Vec<f32> = std::env::var("MC_TURNS_CAM").unwrap_or_default().split(',').filter_map(|v| v.trim().parse().ok()).collect();
        if let [x, y, z, yaw, pitch] = cam[..] {
            self.body.pos = keep_pos + Vec3::new(x, y, z);
            (self.yaw, self.pitch) = (yaw.to_radians(), pitch.to_radians());
        }
        if mode == "open" {
            let main = self.chest_open_at(o(0, -5));
            if let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&main) {
                for (i, n) in ["diamond", "apple", "oak_log", "iron_ingot", "bread", "torch"].iter().enumerate() {
                    inv.slots[i * 9 + i] = self.items.id(n).map(|id| voxel::item::ItemStack::new(id, 9));
                }
            }
            self.hold_store_open(o(0, -5), 1.0);
            self.hold_store_open(o(1, -5), 1.0);
        }
        for x in -9..=9 {
            for z in -12..=1 {
                self.world.mesh_now(c + IVec3::new(x, 0, z));
            }
        }
    }
}
