//! Tree shapes for the woods of the mod content pack (skyroot and golden oak of the Aether, the
//! Brazilian woods of Distant Lands, Ecologics' coconut, walnut and maple, echo trees of the
//! Otherside). Any log and leaves pair can take any shape, so a new wood is just data. Like
//! `trees`, they only see a `TreeSpace`, so generation and saplings share the code.

use crate::block::{BlockId, AIR};
use crate::rng::Rng;
use crate::trees::TreeSpace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// A round crown on a short trunk.
    Round,
    /// A tall thin trunk with a small crown high up.
    Tall,
    /// A leaning trunk with drooping fronds and fruit under the crown.
    Palm,
    /// A long bare trunk under flat, dish-shaped tiers (araucaria).
    Umbrella,
    /// A short trunk that forks into a wide, flat crown (ipê, maple).
    Spread,
    /// Fruit growing straight out of the trunk under a round crown (jabuticaba).
    TrunkFruit,
    /// A twisted trunk under a hanging, glowing crown (echo trees).
    Weeping,
}

impl Shape {
    /// Blocks the crown reaches sideways from the trunk.
    pub fn reach(self) -> i32 {
        match self {
            Shape::Round | Shape::Tall | Shape::TrunkFruit => 2,
            Shape::Palm | Shape::Spread | Shape::Weeping => 4,
            Shape::Umbrella => 4,
        }
    }

    pub fn min_height(self) -> i32 {
        match self {
            Shape::Umbrella => 14,
            Shape::Tall => 10,
            _ => 7,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ModTree {
    pub log: BlockId,
    pub leaves: BlockId,
    pub shape: Shape,
    /// Coconuts, jabuticabas, glow berries: hung under the crown or on the trunk.
    pub fruit: BlockId,
}

const TOP: i32 = crate::chunk::CHUNK_HEIGHT as i32 - 1;

struct G<'a, W: TreeSpace> {
    w: &'a mut W,
    t: ModTree,
}

impl<W: TreeSpace> G<'_, W> {
    fn leaf(&mut self, x: i32, y: i32, z: i32) {
        if (1..=TOP).contains(&y) && self.w.get(x, y, z) == AIR {
            self.w.set(x, y, z, self.t.leaves);
        }
    }

    fn log(&mut self, x: i32, y: i32, z: i32) {
        if (1..=TOP).contains(&y) {
            let g = self.w.get(x, y, z);
            if g == AIR || g == self.t.leaves {
                self.w.set(x, y, z, self.t.log);
            }
        }
    }

    fn fruit(&mut self, x: i32, y: i32, z: i32) {
        if self.t.fruit != AIR && (1..=TOP).contains(&y) && self.w.get(x, y, z) == AIR {
            self.w.set(x, y, z, self.t.fruit);
        }
    }

    /// A rough ball of leaves.
    fn blob(&mut self, rng: &mut Rng, cx: i32, cy: i32, cz: i32, r: i32, flat: f32) {
        for dy in -r..=r {
            for dz in -r..=r {
                for dx in -r..=r {
                    let d = (dx * dx + dz * dz) as f32 + (dy as f32 / flat).powi(2);
                    let edge = (r * r) as f32 + 0.6;
                    if d <= edge && (d < edge - 1.5 || rng.chance(0.7)) {
                        self.leaf(cx + dx, cy + dy, cz + dz);
                    }
                }
            }
        }
    }
}

/// Grow a tree whose ground block is at (x, y, z); the trunk starts at y + 1.
pub fn grow<W: TreeSpace>(t: ModTree, w: &mut W, rng: &mut Rng, x: i32, y: i32, z: i32) {
    let mut g = G { w, t };
    match t.shape {
        Shape::Round | Shape::TrunkFruit => {
            let h = 4 + rng.range(0, 3);
            for k in 1..=h {
                g.log(x, y + k, z);
            }
            g.blob(rng, x, y + h, z, 2, 1.2);
            g.leaf(x, y + h + 2, z);
            if t.shape == Shape::TrunkFruit {
                for k in 1..h - 1 {
                    for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        if rng.chance(0.35) {
                            g.fruit(x + dx, y + k, z + dz);
                        }
                    }
                }
            }
        }
        Shape::Tall => {
            let h = 8 + rng.range(0, 4);
            for k in 1..=h {
                g.log(x, y + k, z);
            }
            g.blob(rng, x, y + h, z, 2, 1.6);
            g.blob(rng, x, y + h - 3, z, 1, 1.0);
        }
        Shape::Palm => {
            let h = 5 + rng.range(0, 4);
            let (lx, lz) = [(1, 0), (-1, 0), (0, 1), (0, -1)][rng.below(4) as usize];
            let (mut cx, mut cz) = (x, z);
            for k in 1..=h {
                if k == h / 2 + 1 || k == h - 1 {
                    cx += lx;
                    cz += lz;
                }
                g.log(cx, y + k, cz);
            }
            let top = y + h + 1;
            g.leaf(cx, top, cz);
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                // Fronds: out two, then down one more.
                g.leaf(cx + dx, top, cz + dz);
                g.leaf(cx + dx * 2, top, cz + dz * 2);
                g.leaf(cx + dx * 3, top - 1, cz + dz * 3);
                if dx * dz == 0 {
                    g.leaf(cx + dx * 4, top - 2, cz + dz * 4);
                }
            }
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                if rng.chance(0.5) {
                    g.fruit(cx + dx, top - 1, cz + dz);
                }
            }
        }
        Shape::Umbrella => {
            let h = 12 + rng.range(0, 5);
            for k in 1..=h {
                g.log(x, y + k, z);
            }
            // Two dish-shaped tiers, the rim curling up.
            for (lift, r) in [(0, 4), (2, 3)] {
                let cy = y + h - 2 + lift;
                for dz in -r..=r {
                    for dx in -r..=r {
                        let d2 = dx * dx + dz * dz;
                        if d2 > r * r + 1 || (d2 > r * r - r && rng.chance(0.3)) {
                            continue;
                        }
                        let rim = (d2 as f32).sqrt() >= r as f32 - 0.8;
                        g.leaf(x + dx, cy + i32::from(rim), z + dz);
                    }
                }
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    for k in 1..r {
                        g.log(x + dx * k, cy - 1 + i32::from(k == r - 1), z + dz * k);
                    }
                }
            }
            g.leaf(x, y + h + 1, z);
        }
        Shape::Spread => {
            let h = 4 + rng.range(0, 2);
            for k in 1..=h {
                g.log(x, y + k, z);
            }
            let n = 2 + rng.below(2) as i32;
            let a0 = rng.f32() * std::f32::consts::TAU;
            for b in 0..n {
                let a = a0 + b as f32 * std::f32::consts::TAU / n as f32;
                let (dx, dz) = (a.cos(), a.sin());
                let mut end = (x, y + h, z);
                for k in 1..=3 {
                    end = (x + (dx * k as f32).round() as i32, y + h + k / 2 + 1, z + (dz * k as f32).round() as i32);
                    g.log(end.0, end.1, end.2);
                }
                g.blob(rng, end.0, end.1 + 1, end.2, 2, 0.8);
            }
            g.blob(rng, x, y + h + 2, z, 2, 0.8);
        }
        Shape::Weeping => {
            let h = 5 + rng.range(0, 3);
            let mut cx = x;
            for k in 1..=h {
                if k == 3 && rng.chance(0.5) {
                    cx += 1;
                }
                g.log(cx, y + k, z);
            }
            let top = y + h;
            g.blob(rng, cx, top + 1, z, 3, 0.7);
            // Strands hanging from the crown's rim.
            for (dx, dz) in [(3, 0), (-3, 0), (0, 3), (0, -3), (2, 2), (-2, -2), (2, -2), (-2, 2)] {
                let len = 2 + rng.below(3) as i32;
                for k in 0..len {
                    g.leaf(cx + dx, top - k, z + dz);
                }
                g.fruit(cx + dx, top - len, z + dz);
            }
        }
    }
}
