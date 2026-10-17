//! Block shapes made of boxes: slabs, stairs, fences, walls, panes, doors and trapdoors.
//!
//! A block's orientation lives in its id: the registry adds hidden variants after the base
//! block (`oak_stairs:5`), and `variant` says which one it is. Fences, walls and panes have a
//! single id and join their neighbours when meshed or collided with.

use crate::block::Shape;

/// A box in pixels: min x, y, z then max x, y, z (0..16; fences collide up to 24).
pub type PBox = [u8; 6];

/// How many ids a shape needs for its orientations.
pub fn variant_count(shape: Shape) -> u8 {
    match shape {
        Shape::Slab => 3,
        Shape::Stairs | Shape::Layer | Shape::Roof => 8,
        Shape::Door | Shape::Trapdoor => 16,
        // Wire: power 0..15. Lever and button: facing | on 4 | on a wall 8. Repeater: facing |
        // delay - 1 in bits 2..3. Piston: six ways (0..5) | pushed out 8; its head: | sticky 8.
        Shape::Wire | Shape::Lever | Shape::Button | Shape::Repeater | Shape::Piston | Shape::PistonHead => 16,
        Shape::Plate => 2,
        // Torches: 0 on the floor, 1..4 on a wall (facing - 1 is the wall's side).
        Shape::Torch => 5,
        // Comparator: facing | subtract 4 | output strength << 3.
        Shape::Comparator => 128,
        // Six ways | triggered 8.
        Shape::Oriented => 16,
        // Spout: 0 down, 1..4 north, east, south, west.
        Shape::Hopper => 5,
        // Chairs: the way you look when sitting.
        Shape::Chair => 4,
        Shape::Lantern | Shape::Bridge => 2,
        Shape::Portal => 3,
        // Rails: shape 0..9 (see `RAIL_ENDS`); powered, detector and activator rails only
        // bend into 0..5 and add 10 when powered.
        Shape::Rail => 16,
        // Liquids: level 0 is the source, 1..7 flow further away, +8 is falling.
        Shape::Liquid => 16,
        _ => 1,
    }
}

/// Surface height in pixels of a liquid variant.
pub fn liquid_height(variant: u8) -> u32 {
    if variant & 8 != 0 {
        16
    } else {
        // Source 14, then a little lower for each block the water has flowed.
        (8 - (variant & 7) as u32) * 16 / 9
    }
}

/// Facing in the low two bits: 0 north (-z), 1 east (+x), 2 south (+z), 3 west (-x).
pub fn facing_of(variant: u8) -> u8 {
    variant & 3
}

/// The six ways a piston faces: north, east, south, west, up, down (low three bits).
pub const DIRS6: [[i32; 3]; 6] = [[0, 0, -1], [1, 0, 0], [0, 0, 1], [-1, 0, 0], [0, 1, 0], [0, -1, 0]];

pub fn dir6_of(variant: u8) -> usize {
    (variant & 7).min(5) as usize
}

/// Turn a box drawn pointing up to point along `DIRS6[dir]`.
fn orient6(b: PBox, dir: usize) -> PBox {
    match dir {
        4 => b,
        5 => flip_y(b),
        // Tip over so up becomes north, then turn to the side.
        _ => rotate([b[0], b[2], 16 - b[4], b[3], b[5], 16 - b[1]], dir as u8),
    }
}

/// Turn a box drawn facing north by `k` quarter turns (north -> east -> south -> west).
fn rotate(b: PBox, k: u8) -> PBox {
    let mut b = b;
    for _ in 0..k {
        // (x, z) -> (16 - z, x)
        let [x0, y0, z0, x1, y1, z1] = b;
        b = [16 - z1, y0, x0, 16 - z0, y1, x1];
    }
    b
}

fn flip_y(b: PBox) -> PBox {
    [b[0], 16 - b[4], b[2], b[3], 16 - b[1], b[5]]
}

/// Neighbour connection bits for fences, walls and panes: +x, -x, +z, -z.
pub const CONN_PX: u8 = 1;
pub const CONN_NX: u8 = 2;
pub const CONN_PZ: u8 = 4;
pub const CONN_NZ: u8 = 8;

/// The boxes of a block. `collision` asks for what bodies bump into (fences are taller and
/// open doors still block their edge).
pub fn boxes(shape: Shape, variant: u8, conn: u8, collision: bool, out: &mut Vec<PBox>) {
    let f = facing_of(variant);
    match shape {
        Shape::Slab => out.push(match variant {
            0 => [0, 0, 0, 16, 8, 16],
            1 => [0, 8, 0, 16, 16, 16],
            _ => [0, 0, 0, 16, 16, 16],
        }),
        Shape::Layer => {
            // Two pixels per layer; you stand one layer lower than it looks, like the classic snow.
            let h = 2 * (variant.min(7) + 1);
            let h = if collision { h - 2 } else { h };
            if h > 0 {
                out.push([0, 0, 0, 16, h, 16]);
            }
        }
        Shape::Stairs => {
            let up = variant & 4 != 0;
            let parts = [[0, 0, 0, 16, 8, 16], rotate([0, 8, 0, 16, 16, 8], f)];
            for p in parts {
                out.push(if up { flip_y(p) } else { p });
            }
        }
        Shape::Roof => {
            // Two-pixel steps from full height at the back down to one step at the front,
            // fine enough to read as a slope and low enough to walk up.
            let up = variant & 4 != 0;
            for k in 0..8u8 {
                let b = rotate([0, 0, 2 * k, 16, 16 - 2 * k, 2 * k + 2], f);
                out.push(if up { flip_y(b) } else { b });
            }
        }
        Shape::Fence | Shape::Wall | Shape::Pane => {
            let (lo, hi, top, arm_lo, arm_hi, arm_top) = match shape {
                Shape::Fence => (6, 10, 16, 7, 9, 15),
                Shape::Wall => (4, 12, 16, 5, 11, 14),
                _ => (7, 9, 16, 7, 9, 16),
            };
            let tall = if collision && shape != Shape::Pane { 24 } else { top };
            let arm_tall = if collision && shape != Shape::Pane { 24 } else { arm_top };
            if !(shape == Shape::Pane && conn != 0) {
                out.push([lo, 0, lo, hi, tall, hi]);
            }
            let arm_y = if shape == Shape::Fence && !collision { 6 } else { 0 };
            let arm = |b: PBox, out: &mut Vec<PBox>| {
                if shape == Shape::Fence && !collision {
                    // Two rails.
                    out.push([b[0], 6, b[2], b[3], 9, b[5]]);
                    out.push([b[0], 12, b[2], b[3], 15, b[5]]);
                } else {
                    out.push(b);
                }
            };
            let (a0, a1) = if shape == Shape::Pane { (7, 9) } else { (lo, hi) };
            if conn & CONN_PX != 0 {
                arm([a1, arm_y, arm_lo, 16, arm_tall, arm_hi], out);
            }
            if conn & CONN_NX != 0 {
                arm([0, arm_y, arm_lo, a0, arm_tall, arm_hi], out);
            }
            if conn & CONN_PZ != 0 {
                arm([arm_lo, arm_y, a1, arm_hi, arm_tall, 16], out);
            }
            if conn & CONN_NZ != 0 {
                arm([arm_lo, arm_y, 0, arm_hi, arm_tall, a0], out);
            }
        }
        Shape::Door => {
            let open = variant & 4 != 0;
            // Closed it sits on the side the player faced; open it swings against its hinge.
            let b = if open { [13, 0, 0, 16, 16, 16] } else { [0, 0, 0, 16, 16, 3] };
            out.push(rotate(b, f));
        }
        Shape::Trapdoor => {
            let open = variant & 4 != 0;
            let top = variant & 8 != 0;
            let b = if open {
                rotate([0, 0, 13, 16, 16, 16], f)
            } else if top {
                [0, 13, 0, 16, 16, 16]
            } else {
                [0, 0, 0, 16, 3, 16]
            };
            out.push(b);
        }
        Shape::Wire => out.push([0, 0, 0, 16, 1, 16]),
        Shape::Lever => {
            let on = variant & 4 != 0;
            // Drawn with the block it hangs on to the north; the handle is the second box.
            let parts = if variant & 8 != 0 {
                [[5, 4, 0, 11, 12, 3], if on { [7, 10, 3, 9, 12, 10] } else { [7, 4, 3, 9, 6, 10] }]
            } else {
                [[5, 0, 4, 11, 3, 12], if on { [7, 3, 5, 9, 11, 7] } else { [7, 3, 9, 9, 11, 11] }]
            };
            out.extend(parts.map(|p| rotate(p, f)));
        }
        Shape::Button => {
            let h = if variant & 4 != 0 { 1 } else { 2 };
            let b = if variant & 8 != 0 { [5, 6, 0, 11, 10, h] } else { [5, 0, 6, 11, h, 10] };
            out.push(rotate(b, f));
        }
        Shape::Plate => out.push([1, 0, 1, 15, if variant & 1 != 0 { 1 } else { 2 }, 15]),
        Shape::Repeater => {
            // Facing is where the signal goes out; the back torch moves back for longer delays.
            let z = 6 + 2 * ((variant >> 2) & 3);
            for b in [[0, 0, 0, 16, 2, 16], [7, 2, 2, 9, 7, 4], [7, 2, z, 9, 7, z + 2]] {
                out.push(rotate(b, f));
            }
        }
        Shape::Piston => {
            let d = dir6_of(variant);
            if variant & 8 != 0 {
                out.push(orient6([0, 0, 0, 16, 12, 16], d));
                out.push(orient6([6, 12, 6, 10, 16, 10], d));
            } else {
                out.push([0, 0, 0, 16, 16, 16]);
            }
        }
        Shape::Comparator => {
            for b in [[0, 0, 0, 16, 2, 16], [7, 2, 2, 9, 6, 4], [4, 2, 11, 6, 7, 13], [10, 2, 11, 12, 7, 13]] {
                out.push(rotate(b, f));
            }
        }
        Shape::Hopper => {
            out.push([0, 10, 0, 16, 16, 16]);
            out.push([4, 4, 4, 12, 10, 12]);
            out.push(if variant == 0 { [6, 0, 6, 10, 4, 10] } else { rotate([6, 4, 0, 10, 8, 4], (variant - 1) & 3) });
        }
        Shape::Rail => out.push(if rail_rises(variant).is_some() { [0, 0, 0, 16, 8, 16] } else { [0, 0, 0, 16, 2, 16] }),
        Shape::Chair => {
            if collision {
                out.push([2, 0, 2, 14, 10, 14]);
                out.push(rotate([2, 10, 12, 14, 16, 14], f));
                return;
            }
            // Drawn facing north: four legs, the seat, and the back on the south side.
            for b in [
                [2, 0, 2, 4, 8, 4],
                [12, 0, 2, 14, 8, 4],
                [2, 0, 12, 4, 8, 14],
                [12, 0, 12, 14, 8, 14],
                [2, 8, 2, 14, 10, 14],
                [2, 10, 12, 4, 16, 14],
                [12, 10, 12, 14, 16, 14],
                [4, 12, 12, 12, 15, 13],
            ] {
                out.push(rotate(b, f));
            }
        }
        Shape::Lantern => {
            let y = if variant & 1 != 0 { 2 } else { 0 };
            out.push([5, y, 5, 11, y + 7, 11]);
            out.push([6, y + 7, 6, 10, y + 9, 10]);
            if variant & 1 != 0 {
                out.push([7, 11, 7, 9, 16, 9]);
            }
        }
        Shape::Chain => out.push([7, 0, 7, 9, 16, 9]),
        Shape::Pot => out.push([3, 0, 3, 13, 6, 13]),
        Shape::Portal => out.push(match variant {
            // Upright across x or z (nether portal), or lying flat (end portal).
            0 => [0, 0, 6, 16, 16, 10],
            1 => [6, 0, 0, 10, 16, 16],
            _ => [0, 0, 0, 16, 12, 16],
        }),
        Shape::Bridge => {
            let k = variant & 1;
            out.push(rotate([0, 0, 0, 16, 2, 16], k));
            if collision {
                // Rails you can't walk off.
                out.push(rotate([0, 2, 0, 1, 16, 16], k));
                out.push(rotate([15, 2, 0, 16, 16, 16], k));
            } else {
                for x in [0, 15] {
                    out.push(rotate([x, 2, 0, x + 1, 10, 1], k));
                    out.push(rotate([x, 8, 1, x + 1, 9, 16], k));
                    out.push(rotate([x, 4, 1, x + 1, 5, 16], k));
                }
            }
        }
        Shape::Table => {
            out.push([0, 13, 0, 16, 16, 16]);
            // A leg at each corner where neither side runs on into another table.
            for (x, z, bx, bz) in [(1, 1, CONN_NX, CONN_NZ), (13, 1, CONN_PX, CONN_NZ), (1, 13, CONN_NX, CONN_PZ), (13, 13, CONN_PX, CONN_PZ)] {
                if conn & (bx | bz) == 0 {
                    out.push([x, 0, z, x + 2, 13, z + 2]);
                }
            }
        }
        Shape::PistonHead => {
            let d = dir6_of(variant);
            out.push(orient6([0, 12, 0, 16, 16, 16], d));
            out.push(orient6([6, 0, 6, 10, 12, 10], d));
        }
        _ => out.push([0, 0, 0, 16, 16, 16]),
    }
}

/// The two ends of each rail shape as (dx, dz): north-south, east-west, rising to the east,
/// west, north and south (the first end is the high one), then the curves south-east,
/// south-west, north-west and north-east.
pub const RAIL_ENDS: [[[i32; 2]; 2]; 10] = [
    [[0, -1], [0, 1]],
    [[1, 0], [-1, 0]],
    [[1, 0], [-1, 0]],
    [[-1, 0], [1, 0]],
    [[0, -1], [0, 1]],
    [[0, 1], [0, -1]],
    [[0, 1], [1, 0]],
    [[0, 1], [-1, 0]],
    [[0, -1], [-1, 0]],
    [[0, -1], [1, 0]],
];

/// Which of the ten rail shapes a variant is.
pub fn rail_shape(variant: u8) -> usize {
    (if variant >= 10 { variant - 10 } else { variant }) as usize % 10
}

/// A powered, detector or activator rail that is on.
pub fn rail_on(variant: u8) -> bool {
    variant >= 10
}

/// The direction (dx, dz) a sloping rail climbs toward.
pub fn rail_rises(variant: u8) -> Option<[i32; 2]> {
    let s = rail_shape(variant);
    (2..6).contains(&s).then(|| RAIL_ENDS[s][0])
}

/// Boxes after the first of a lever or repeater are its handle or torches, drawn with the
/// block's `front` texture like a little torch.
pub fn has_parts(shape: Shape) -> bool {
    matches!(shape, Shape::Lever | Shape::Repeater | Shape::Comparator)
}

/// Fences, walls and panes join others of their kind and full solid blocks.
pub fn joins(shape: Shape, other: Shape, other_opaque: bool) -> bool {
    (other_opaque && shape != Shape::Table) || other == shape
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_share_legs_and_chairs_turn() {
        let mut v = Vec::new();
        boxes(Shape::Table, 0, 0, false, &mut v);
        assert_eq!(v.len(), 5, "a lone table has a top and four legs");
        v.clear();
        boxes(Shape::Table, 0, CONN_PX | CONN_NX, false, &mut v);
        assert_eq!(v.len(), 1, "in the middle of a row only the top is left");
        v.clear();
        boxes(Shape::Chair, 2, 0, false, &mut v);
        // Facing south: the back stands on the north side.
        assert!(v[5][2] < 8 && v[5][4] == 16);
        assert!(joins(Shape::Table, Shape::Table, false) && !joins(Shape::Table, Shape::Cube, true));
    }

    #[test]
    fn stairs_rotate_their_step() {
        let mut v = Vec::new();
        boxes(Shape::Stairs, 1, 0, false, &mut v);
        // Facing east: the tall half is on the +x side.
        assert_eq!(v[1], [8, 8, 0, 16, 16, 16]);
        v.clear();
        boxes(Shape::Stairs, 4, 0, false, &mut v);
        assert_eq!(v[0], [0, 8, 0, 16, 16, 16]);
        assert_eq!(v[1], [0, 0, 0, 16, 8, 8]);
    }

    #[test]
    fn pistons_point_six_ways() {
        let mut v = Vec::new();
        for (d, want) in [(0, [0, 0, 0, 16, 16, 4]), (1, [12, 0, 0, 16, 16, 16]), (2, [0, 0, 12, 16, 16, 16]), (3, [0, 0, 0, 4, 16, 16]), (4, [0, 12, 0, 16, 16, 16]), (5, [0, 0, 0, 16, 4, 16])] {
            v.clear();
            boxes(Shape::PistonHead, d, 0, false, &mut v);
            assert_eq!(v[0], want, "head facing {d}");
        }
    }

    #[test]
    fn fences_collide_taller() {
        let mut v = Vec::new();
        boxes(Shape::Fence, 0, CONN_PX, true, &mut v);
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|b| b[4] == 24));
    }
}
