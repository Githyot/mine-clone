//! Tree shapes, shared by world generation (writing into one chunk) and growing saplings
//! (writing into the live world). A tree only sees a `TreeSpace`, so the same code serves both.

use crate::block::{BlockId, BlockRegistry, AIR};
use crate::rng::Rng;

/// Where a tree is written. Coordinates are whatever the caller uses (chunk-local or world).
pub trait TreeSpace {
    /// The block at a position; anything that must not be touched (outside the chunk) should
    /// read as a solid, non-air block.
    fn get(&self, x: i32, y: i32, z: i32) -> BlockId;
    fn set(&mut self, x: i32, y: i32, z: i32, id: BlockId);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wood {
    Oak,
    Birch,
    Spruce,
    Jungle,
    Acacia,
    DarkOak,
    Cherry,
}

impl Wood {
    pub const ALL: [Wood; 7] = [Wood::Oak, Wood::Birch, Wood::Spruce, Wood::Jungle, Wood::Acacia, Wood::DarkOak, Wood::Cherry];

    pub fn name(self) -> &'static str {
        match self {
            Wood::Oak => "oak",
            Wood::Birch => "birch",
            Wood::Spruce => "spruce",
            Wood::Jungle => "jungle",
            Wood::Acacia => "acacia",
            Wood::DarkOak => "dark_oak",
            Wood::Cherry => "cherry",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeKind {
    /// The classic round tree (oak, birch).
    Round(Wood),
    Spruce,
    /// A small jungle tree with vines.
    Jungle,
    /// A tall jungle tree with a 2×2 trunk, side branches and vines.
    MegaJungle,
    /// Bent trunk and flat, wide canopies.
    Acacia,
    /// A 2×2 trunk under a broad, dense canopy.
    DarkOak,
    /// A puffy pink crown with a few hanging leaves.
    Cherry,
    /// A wide, low oak dripping with vines; it can stand in shallow water.
    SwampOak,
    /// One log hidden in a mound of leaves.
    Bush(Wood),
}

impl TreeKind {
    /// What a sapling grows into.
    pub fn from_sapling(name: &str) -> Option<TreeKind> {
        Some(match name.strip_suffix("_sapling")? {
            "oak" => TreeKind::Round(Wood::Oak),
            "birch" => TreeKind::Round(Wood::Birch),
            "spruce" => TreeKind::Spruce,
            "jungle" => TreeKind::Jungle,
            "acacia" => TreeKind::Acacia,
            "dark_oak" => TreeKind::DarkOak,
            "cherry" => TreeKind::Cherry,
            _ => return None,
        })
    }

    /// Blocks the canopy reaches sideways from the trunk column (the 2×2 trunks also take
    /// one more block toward +X/+Z), so world generation keeps the tree inside its chunk.
    pub fn reach(self) -> i32 {
        match self {
            TreeKind::Round(_) | TreeKind::Spruce | TreeKind::Jungle | TreeKind::Bush(_) => 2,
            TreeKind::SwampOak => 3,
            TreeKind::Acacia | TreeKind::DarkOak | TreeKind::Cherry => 4,
            TreeKind::MegaJungle => 5,
        }
    }

    pub fn wide_trunk(self) -> bool {
        matches!(self, TreeKind::MegaJungle | TreeKind::DarkOak)
    }

    /// Free blocks needed straight above a sapling.
    pub fn min_height(self) -> i32 {
        match self {
            TreeKind::Bush(_) => 2,
            TreeKind::Acacia => 5,
            TreeKind::Round(_) | TreeKind::Cherry => 6,
            TreeKind::SwampOak | TreeKind::Jungle | TreeKind::DarkOak => 7,
            TreeKind::Spruce => 8,
            TreeKind::MegaJungle => 12,
        }
    }
}

/// Block ids trees are made of, resolved once.
#[derive(Clone)]
pub struct TreeBlocks {
    logs: [BlockId; 7],
    leaves: [BlockId; 7],
    vine: BlockId,
    dirt: BlockId,
    grass: BlockId,
    snow_grass: BlockId,
    water: BlockId,
}

impl TreeBlocks {
    pub fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        TreeBlocks {
            logs: Wood::ALL.map(|w| g(&format!("{}_log", w.name()))),
            leaves: Wood::ALL.map(|w| g(&format!("{}_leaves", w.name()))),
            vine: g("vine"),
            dirt: g("dirt"),
            grass: g("grass_block"),
            snow_grass: g("snowy_grass_block"),
            water: g("water"),
        }
    }

    pub fn log(&self, w: Wood) -> BlockId {
        self.logs[w as usize]
    }

    pub fn leaves(&self, w: Wood) -> BlockId {
        self.leaves[w as usize]
    }

    /// Whether this wood's blocks exist (a pack may lack them).
    pub fn has(&self, w: Wood) -> bool {
        self.log(w) != AIR && self.leaves(w) != AIR
    }

    fn soft(&self, id: BlockId) -> bool {
        id == AIR || id == self.water || id == self.vine || self.leaves.contains(&id)
    }
}

/// Grow a tree whose ground block is at (x, y, z); the trunk starts at y + 1.
pub fn grow<W: TreeSpace>(kind: TreeKind, b: &TreeBlocks, w: &mut W, rng: &mut Rng, x: i32, y: i32, z: i32) {
    let mut t = Tree { b, w };
    match kind {
        TreeKind::Round(wood) => t.round(rng, x, y, z, wood),
        TreeKind::Spruce => t.spruce(rng, x, y, z),
        TreeKind::Jungle => t.jungle(rng, x, y, z),
        TreeKind::MegaJungle => t.mega_jungle(rng, x, y, z),
        TreeKind::Acacia => t.acacia(rng, x, y, z),
        TreeKind::DarkOak => t.dark_oak(rng, x, y, z),
        TreeKind::Cherry => t.cherry(rng, x, y, z),
        TreeKind::SwampOak => t.swamp_oak(rng, x, y, z),
        TreeKind::Bush(wood) => t.bush(rng, x, y, z, wood),
    }
}

struct Tree<'a, W: TreeSpace> {
    b: &'a TreeBlocks,
    w: &'a mut W,
}

const TOP: i32 = crate::chunk::CHUNK_HEIGHT as i32 - 1;

impl<W: TreeSpace> Tree<'_, W> {
    fn leaf(&mut self, x: i32, y: i32, z: i32, id: BlockId) {
        if y <= TOP && self.w.get(x, y, z) == AIR {
            self.w.set(x, y, z, id);
        }
    }

    fn log(&mut self, x: i32, y: i32, z: i32, id: BlockId) {
        if y <= TOP && self.b.soft(self.w.get(x, y, z)) {
            self.w.set(x, y, z, id);
        }
    }

    /// Under a trunk column: grass turns to dirt; on a slope the trunk reaches down to the ground.
    fn root(&mut self, x: i32, y: i32, z: i32, log: BlockId) {
        let g = self.w.get(x, y, z);
        if g == self.b.grass || g == self.b.snow_grass {
            self.w.set(x, y, z, self.b.dirt);
        } else if g == AIR || g == self.b.water {
            for dy in 0..4 {
                if !self.b.soft(self.w.get(x, y - dy, z)) || y - dy < 1 {
                    break;
                }
                self.w.set(x, y - dy, z, log);
            }
        }
    }

    /// Vines hanging from under a leaf block.
    fn hang(&mut self, x: i32, y: i32, z: i32, len: i32) {
        if self.b.vine == AIR {
            return;
        }
        for i in 1..=len {
            if self.w.get(x, y - i, z) != AIR {
                break;
            }
            self.w.set(x, y - i, z, self.b.vine);
        }
    }

    /// A flat disc of leaves around (cx, cz) in block-centre space, trimmed at the rim.
    #[allow(clippy::too_many_arguments)]
    fn disc(&mut self, rng: &mut Rng, cx: f32, y: i32, cz: f32, r: f32, leaves: BlockId, trim: f32, vines: f32) {
        let ri = r.ceil() as i32 + 1;
        let (bx, bz) = (cx.floor() as i32, cz.floor() as i32);
        for dz in -ri..=ri {
            for dx in -ri..=ri {
                let (px, pz) = (bx + dx, bz + dz);
                let (fx, fz) = (px as f32 + 0.5 - cx, pz as f32 + 0.5 - cz);
                let d = (fx * fx + fz * fz).sqrt();
                if d > r || (d > r - 1.0 && rng.chance(trim)) {
                    continue;
                }
                self.leaf(px, y, pz, leaves);
                if vines > 0.0 && d > r - 1.3 && rng.chance(vines) {
                    let len = rng.range(1, 4);
                    self.hang(px, y, pz, len);
                }
            }
        }
    }

    fn round(&mut self, rng: &mut Rng, x: i32, y: i32, z: i32, wood: Wood) {
        let (log, leaves) = (self.b.log(wood), self.b.leaves(wood));
        let h = rng.range(4, 6);
        let top = y + h;
        if top + 2 > TOP {
            return;
        }
        for ly in (top - 2)..=(top + 1) {
            let r: i32 = if ly >= top { 1 } else { 2 };
            for dx in -r..=r {
                for dz in -r..=r {
                    // Trim corners randomly for the classic round-ish look.
                    if dx.abs() == r && dz.abs() == r && (ly >= top || rng.chance(0.5)) {
                        continue;
                    }
                    self.leaf(x + dx, ly, z + dz, leaves);
                }
            }
        }
        for ly in (y + 1)..top {
            self.w.set(x, ly, z, log);
        }
        self.root(x, y, z, log);
    }

    fn spruce(&mut self, rng: &mut Rng, x: i32, y: i32, z: i32) {
        let (log, leaves) = (self.b.log(Wood::Spruce), self.b.leaves(Wood::Spruce));
        let h = rng.range(6, 9);
        let top = y + h;
        if top + 2 > TOP {
            return;
        }
        let mut r = 0i32;
        let mut ly = top + 1;
        while ly > y + 2 {
            for dx in -r..=r {
                for dz in -r..=r {
                    if r > 0 && dx.abs() == r && dz.abs() == r {
                        continue;
                    }
                    self.leaf(x + dx, ly, z + dz, leaves);
                }
            }
            r = if r >= 2 { 1 } else { r + 1 };
            ly -= 1;
        }
        for ly in (y + 1)..=top {
            self.w.set(x, ly, z, log);
        }
        self.root(x, y, z, log);
    }

    fn jungle(&mut self, rng: &mut Rng, x: i32, y: i32, z: i32) {
        let (log, leaves) = (self.b.log(Wood::Jungle), self.b.leaves(Wood::Jungle));
        let h = rng.range(5, 8);
        let top = y + h;
        if top + 2 > TOP {
            return;
        }
        let (cx, cz) = (x as f32 + 0.5, z as f32 + 0.5);
        self.disc(rng, cx, top - 2, cz, 2.6, leaves, 0.3, 0.3);
        self.disc(rng, cx, top - 1, cz, 2.9, leaves, 0.4, 0.0);
        self.disc(rng, cx, top, cz, 2.0, leaves, 0.3, 0.0);
        self.disc(rng, cx, top + 1, cz, 1.2, leaves, 0.0, 0.0);
        for ly in (y + 1)..top {
            self.log(x, ly, z, log);
        }
        self.root(x, y, z, log);
    }

    fn mega_jungle(&mut self, rng: &mut Rng, x: i32, y: i32, z: i32) {
        let (log, leaves) = (self.b.log(Wood::Jungle), self.b.leaves(Wood::Jungle));
        let h = rng.range(12, 18);
        let top = y + h;
        if top + 2 > TOP {
            return;
        }
        let (cx, cz) = (x as f32 + 1.0, z as f32 + 1.0);
        // Side branches: a short arm of logs ending in a ball of leaves.
        for i in 0..rng.range(1, 3) {
            let by = y + h / 2 + i * 3 + rng.range(0, 2);
            let (dx, dz) = [(1, 0), (-1, 0), (0, 1), (0, -1)][rng.below(4) as usize];
            let len = rng.range(2, 3);
            let (mut px, mut pz) = (if dx > 0 { x + 1 } else { x }, if dz > 0 { z + 1 } else { z });
            for _ in 0..len {
                px += dx;
                pz += dz;
                self.log(px, by, pz, log);
            }
            let (fx, fz) = (px as f32 + 0.5, pz as f32 + 0.5);
            self.disc(rng, fx, by, fz, 2.2, leaves, 0.3, 0.2);
            self.disc(rng, fx, by + 1, fz, 1.6, leaves, 0.2, 0.0);
        }
        self.disc(rng, cx, top - 2, cz, 3.6, leaves, 0.4, 0.35);
        self.disc(rng, cx, top - 1, cz, 4.4, leaves, 0.5, 0.25);
        self.disc(rng, cx, top, cz, 3.6, leaves, 0.4, 0.0);
        self.disc(rng, cx, top + 1, cz, 2.2, leaves, 0.2, 0.0);
        for (ox, oz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            for ly in (y + 1)..top {
                self.log(x + ox, ly, z + oz, log);
            }
            self.root(x + ox, y, z + oz, log);
        }
        // Vines climbing down the trunk's outside.
        for (vx, vz) in [(-1, 0), (2, 1), (1, -1), (0, 2)] {
            if rng.chance(0.5) {
                let start = top - 3;
                let len = rng.range(3, h - 3);
                self.hang(x + vx, start + 1, z + vz, len);
            }
        }
    }

    fn acacia(&mut self, rng: &mut Rng, x: i32, y: i32, z: i32) {
        let (log, leaves) = (self.b.log(Wood::Acacia), self.b.leaves(Wood::Acacia));
        let stem = rng.range(2, 3);
        let lean = rng.range(2, 3);
        if y + stem + lean + 2 > TOP {
            return;
        }
        // Lean toward the middle of the chunk so the crown stays inside it.
        let dx = if (x & 15) < 8 { 1 } else { -1 };
        let dz = if rng.chance(0.5) { 0 } else if (z & 15) < 8 { 1 } else { -1 };
        for ly in (y + 1)..=(y + stem) {
            self.log(x, ly, z, log);
        }
        let (mut px, mut py, mut pz) = (x, y + stem, z);
        for _ in 0..lean {
            px += dx;
            py += 1;
            pz += dz;
            self.log(px, py, pz, log);
        }
        let (cx, cz) = (px as f32 + 0.5, pz as f32 + 0.5);
        self.disc(rng, cx, py, cz, 3.3, leaves, 0.35, 0.0);
        self.disc(rng, cx, py + 1, cz, 1.6, leaves, 0.3, 0.0);
        // Sometimes a second, smaller crown on the other side.
        if rng.chance(0.5) {
            let (mut qx, mut qy, mut qz) = (x, y + stem - 1 + rng.range(0, 1), z);
            for _ in 0..2 {
                qx -= dx;
                qy += 1;
                qz -= if dz == 0 { 0 } else { dz };
                self.log(qx, qy, qz, log);
            }
            let (fx, fz) = (qx as f32 + 0.5, qz as f32 + 0.5);
            self.disc(rng, fx, qy + 1, fz, 2.3, leaves, 0.3, 0.0);
        }
        self.root(x, y, z, log);
    }

    fn dark_oak(&mut self, rng: &mut Rng, x: i32, y: i32, z: i32) {
        let (log, leaves) = (self.b.log(Wood::DarkOak), self.b.leaves(Wood::DarkOak));
        let h = rng.range(6, 8);
        let top = y + h;
        if top + 2 > TOP {
            return;
        }
        let (cx, cz) = (x as f32 + 1.0, z as f32 + 1.0);
        // Stubby branches poking into the crown.
        for (bx, bz) in [(-1, 0), (2, 1), (1, -1), (0, 2)] {
            if rng.chance(0.4) {
                self.log(x + bx, top - 1, z + bz, log);
            }
        }
        self.disc(rng, cx, top - 2, cz, 3.4, leaves, 0.5, 0.0);
        self.disc(rng, cx, top - 1, cz, 4.4, leaves, 0.45, 0.0);
        self.disc(rng, cx, top, cz, 3.8, leaves, 0.35, 0.0);
        self.disc(rng, cx, top + 1, cz, 2.3, leaves, 0.3, 0.0);
        for (ox, oz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            for ly in (y + 1)..top {
                self.log(x + ox, ly, z + oz, log);
            }
            self.root(x + ox, y, z + oz, log);
        }
    }

    fn cherry(&mut self, rng: &mut Rng, x: i32, y: i32, z: i32) {
        let (log, leaves) = (self.b.log(Wood::Cherry), self.b.leaves(Wood::Cherry));
        let h = rng.range(5, 6);
        let top = y + h;
        if top + 2 > TOP {
            return;
        }
        // The upper trunk steps sideways once, which gives the tree a little character.
        let (sx, sz) = [(1, 0), (-1, 0), (0, 1), (0, -1), (0, 0)][rng.below(5) as usize];
        let bend = top - 2;
        for ly in (y + 1)..top {
            let (lx, lz) = if ly >= bend { (x + sx, z + sz) } else { (x, z) };
            self.log(lx, ly, lz, log);
        }
        let (cx, cz) = ((x + sx) as f32 + 0.5, (z + sz) as f32 + 0.5);
        // Hanging fringe below the crown's rim.
        let ri = 3;
        for dz in -ri..=ri {
            for dx in -ri..=ri {
                let d2 = dx * dx + dz * dz;
                if (5..=9).contains(&d2) && rng.chance(0.3) {
                    let (px, pz) = (x + sx + dx, z + sz + dz);
                    self.leaf(px, top - 2, pz, leaves);
                    self.leaf(px, top - 3, pz, leaves);
                }
            }
        }
        self.disc(rng, cx, top - 1, cz, 3.3, leaves, 0.4, 0.0);
        self.disc(rng, cx, top, cz, 3.0, leaves, 0.3, 0.0);
        self.disc(rng, cx, top + 1, cz, 2.0, leaves, 0.3, 0.0);
        self.root(x, y, z, log);
    }

    fn swamp_oak(&mut self, rng: &mut Rng, x: i32, y: i32, z: i32) {
        let (log, leaves) = (self.b.log(Wood::Oak), self.b.leaves(Wood::Oak));
        let h = rng.range(5, 7);
        let top = y + h;
        if top + 2 > TOP {
            return;
        }
        let (cx, cz) = (x as f32 + 0.5, z as f32 + 0.5);
        self.disc(rng, cx, top - 2, cz, 3.3, leaves, 0.3, 0.45);
        self.disc(rng, cx, top - 1, cz, 3.0, leaves, 0.3, 0.0);
        self.disc(rng, cx, top, cz, 2.0, leaves, 0.3, 0.0);
        self.disc(rng, cx, top + 1, cz, 1.2, leaves, 0.0, 0.0);
        for ly in (y + 1)..top {
            self.log(x, ly, z, log);
        }
        self.root(x, y, z, log);
    }

    fn bush(&mut self, rng: &mut Rng, x: i32, y: i32, z: i32, wood: Wood) {
        let (log, leaves) = (self.b.log(wood), self.b.leaves(wood));
        if y + 3 > TOP {
            return;
        }
        let (cx, cz) = (x as f32 + 0.5, z as f32 + 0.5);
        self.disc(rng, cx, y + 1, cz, 2.3, leaves, 0.5, 0.0);
        self.disc(rng, cx, y + 2, cz, 1.3, leaves, 0.4, 0.0);
        self.log(x, y + 1, z, log);
        self.root(x, y, z, log);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Open air above a flat grass floor at y = 10.
    struct Space(HashMap<(i32, i32, i32), BlockId>, BlockId);

    impl TreeSpace for Space {
        fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
            self.0.get(&(x, y, z)).copied().unwrap_or(if y <= 10 { self.1 } else { AIR })
        }
        fn set(&mut self, x: i32, y: i32, z: i32, id: BlockId) {
            self.0.insert((x, y, z), id);
        }
    }

    #[test]
    fn every_sapling_grows_its_own_wood() {
        let reg = BlockRegistry::from_ron(include_str!("../../../packs/vanilla/blocks.ron")).unwrap();
        let b = TreeBlocks::new(&reg);
        for wood in Wood::ALL {
            assert!(b.has(wood), "{} blocks missing", wood.name());
            let kind = TreeKind::from_sapling(&format!("{}_sapling", wood.name())).unwrap();
            let mut s = Space(HashMap::new(), reg.id("grass_block").unwrap());
            grow(kind, &b, &mut s, &mut Rng::new(5), 0, 10, 0);
            assert_eq!(s.get(0, 11, 0), b.log(wood), "{} trunk", wood.name());
            assert!(s.0.values().any(|&id| id == b.leaves(wood)), "{} leaves", wood.name());
        }
    }

    #[test]
    fn trees_fit_in_their_chunk() {
        let reg = BlockRegistry::from_ron(include_str!("../../../packs/vanilla/blocks.ron")).unwrap();
        let b = TreeBlocks::new(&reg);
        let kinds = [
            TreeKind::Round(Wood::Oak),
            TreeKind::Spruce,
            TreeKind::Jungle,
            TreeKind::MegaJungle,
            TreeKind::Acacia,
            TreeKind::DarkOak,
            TreeKind::Cherry,
            TreeKind::SwampOak,
            TreeKind::Bush(Wood::Oak),
        ];
        for kind in kinds {
            // World generation places a tree between `reach` and the far edge minus `reach`.
            let (lo, hi) = (kind.reach(), 15 - kind.reach() - kind.wide_trunk() as i32);
            for seed in 0..30 {
                for (x, z) in [(lo, lo), (hi, hi), (lo, hi), (hi, lo)] {
                    let mut s = Space(HashMap::new(), reg.id("grass_block").unwrap());
                    grow(kind, &b, &mut s, &mut Rng::new(seed), x, 10, z);
                    for &(px, _, pz) in s.0.keys() {
                        assert!((0..16).contains(&px) && (0..16).contains(&pz), "{kind:?} from {x},{z} reaches {px},{pz}");
                    }
                }
            }
        }
    }
}
