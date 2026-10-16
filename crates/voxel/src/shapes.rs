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
        Shape::Stairs => 8,
        Shape::Door | Shape::Trapdoor => 16,
        _ => 1,
    }
}

/// Facing in the low two bits: 0 north (-z), 1 east (+x), 2 south (+z), 3 west (-x).
pub fn facing_of(variant: u8) -> u8 {
    variant & 3
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
        Shape::Stairs => {
            let up = variant & 4 != 0;
            let parts = [[0, 0, 0, 16, 8, 16], rotate([0, 8, 0, 16, 16, 8], f)];
            for p in parts {
                out.push(if up { flip_y(p) } else { p });
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
        _ => out.push([0, 0, 0, 16, 16, 16]),
    }
}

/// Fences, walls and panes join others of their kind and full solid blocks.
pub fn joins(shape: Shape, other: Shape, other_opaque: bool) -> bool {
    other_opaque || other == shape
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn fences_collide_taller() {
        let mut v = Vec::new();
        boxes(Shape::Fence, 0, CONN_PX, true, &mut v);
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|b| b[4] == 24));
    }
}
