//! Beds: two blocks long, a head part and a foot part placed together.
//!
//! The variant is the way the head points (0 north, 1 east, 2 south, 3 west, as `shapes::facing_of`)
//! plus 4 for the head part. The model is built here from boxes of four materials, the block's
//! own pictures: `side` is the wooden frame, `top` the blanket (one per colour), `front` the
//! pillow and `bottom` the sheet and mattress.

use crate::mesh::{model_quads, ModelElement, ModelFace, ModelQuad};

/// Variant bit of the head part.
pub const HEAD: u8 = 4;

/// Height of the sleeping surface (the blanket) in pixels.
pub const TOP: u8 = 10;

/// The way the head points, as (dx, dz).
pub fn dir(variant: u8) -> [i32; 2] {
    [[0, -1], [1, 0], [0, 1], [-1, 0]][(variant & 3) as usize]
}

pub fn is_head(variant: u8) -> bool {
    variant & HEAD != 0
}

/// Where the other half of a bed part lies, as (dx, dz) from it.
pub fn other_half(variant: u8) -> [i32; 2] {
    let [x, z] = dir(variant);
    if is_head(variant) {
        [-x, -z]
    } else {
        [x, z]
    }
}

/// Block faces in mesh order: +x, -x, +y, -y, +z, -z. Which picture of the block each
/// material takes (an index into the block's six face layers).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mat {
    /// The wooden frame, legs, headboard and footboard (`side`).
    Wood = 0,
    /// The blanket (`top`).
    Blanket = 2,
    /// Sheet and mattress (`bottom`).
    Sheet = 3,
    /// The pillow (`front`).
    Pillow = 4,
}

/// A box drawn facing north (head toward -z): from, to and its material. `top_uv` gives the
/// top face its own part of the picture.
struct Part {
    from: [u8; 3],
    to: [u8; 3],
    mat: Mat,
    top_uv: Option<[f32; 4]>,
}

const fn p(from: [u8; 3], to: [u8; 3], mat: Mat) -> Part {
    Part { from, to, mat, top_uv: None }
}

/// The head part, headboard on the north side.
const HEAD_PARTS: &[Part] = &[
    // Corner posts, which are also the legs, and the headboard between them: a frame of
    // two rails round a panel set one pixel back.
    p([0, 0, 0], [3, 15, 3], Mat::Wood),
    p([13, 0, 0], [16, 15, 3], Mat::Wood),
    p([3, 12, 0], [13, 14, 2], Mat::Wood),
    p([3, 5, 1], [13, 12, 2], Mat::Wood),
    p([3, 3, 0], [13, 5, 2], Mat::Wood),
    // Side rails and the board the mattress lies on.
    p([0, 3, 3], [1, 7, 16], Mat::Wood),
    p([15, 3, 3], [16, 7, 16], Mat::Wood),
    p([1, 3, 3], [15, 4, 16], Mat::Wood),
    // Mattress.
    p([1, 4, 3], [15, 9, 16], Mat::Sheet),
    // Pillow, with a plumper middle.
    Part { from: [2, 9, 4], to: [14, 11, 9], mat: Mat::Pillow, top_uv: Some([0.0, 0.0, 16.0, 16.0]) },
    Part { from: [3, 11, 5], to: [13, 12, 8], mat: Mat::Pillow, top_uv: Some([2.0, 4.0, 14.0, 12.0]) },
    // The sheet folded back over the blanket, a little proud of it.
    p([0, 7, 10], [16, 11, 12], Mat::Sheet),
    // Blanket, hanging over the side rails.
    p([0, 7, 12], [16, 10, 16], Mat::Blanket),
];

/// The foot part, footboard on the south side.
const FOOT_PARTS: &[Part] = &[
    // Posts and a lower footboard like the headboard.
    p([0, 0, 13], [3, 11, 16], Mat::Wood),
    p([13, 0, 13], [16, 11, 16], Mat::Wood),
    p([3, 9, 14], [13, 11, 16], Mat::Wood),
    p([3, 5, 14], [13, 9, 15], Mat::Wood),
    p([3, 3, 14], [13, 5, 16], Mat::Wood),
    p([0, 3, 0], [1, 7, 13], Mat::Wood),
    p([15, 3, 0], [16, 7, 13], Mat::Wood),
    p([1, 3, 0], [15, 4, 14], Mat::Wood),
    p([1, 4, 0], [15, 9, 13], Mat::Sheet),
    // Blanket over the foot, its end hanging a little lower than the sides.
    p([0, 7, 0], [16, 10, 13], Mat::Blanket),
    p([1, 5, 12], [15, 7, 13], Mat::Blanket),
];

/// Turn a quad drawn facing north by `k` quarter turns (north -> east -> south -> west).
fn turn(q: &mut ModelQuad, k: u8) {
    // Faces +x, -x, +y, -y, +z, -z after one turn: -z goes to +x, +x to +z, +z to -x, -x to -z.
    const NEXT: [u8; 7] = [4, 5, 2, 3, 1, 0, 6];
    for _ in 0..k & 3 {
        for c in q.pos.iter_mut() {
            *c = [16 - c[2], c[1], c[0]];
        }
        q.face = NEXT[q.face as usize];
        q.cull = NEXT[q.cull as usize];
    }
}

/// The quads of a bed part. `faces` are the block's six face layers.
pub fn quads(variant: u8, faces: [u16; 6]) -> Vec<ModelQuad> {
    let parts = if is_head(variant) { HEAD_PARTS } else { FOOT_PARTS };
    let elements: Vec<ModelElement> = parts
        .iter()
        .map(|pt| {
            let layer = faces[pt.mat as usize];
            let mut f = [Some(ModelFace { uv: None, layer, cull: None, tint: false }); 6];
            // Faces flat on the ground are hidden under it.
            if pt.from[1] == 0 {
                f[3] = Some(ModelFace { uv: None, layer, cull: Some(3), tint: false });
            }
            if let (Some(uv), Some(top)) = (pt.top_uv, f[2].as_mut()) {
                top.uv = Some(uv);
            }
            // Rails, mattress and blanket run on into the other half: no end there.
            if is_head(variant) && pt.to[2] == 16 {
                f[4] = None;
            }
            if !is_head(variant) && pt.from[2] == 0 {
                f[5] = None;
            }
            ModelElement { from: pt.from.map(|v| v as f32), to: pt.to.map(|v| v as f32), faces: f }
        })
        .collect();
    let mut out = model_quads(&elements);
    for q in out.iter_mut() {
        turn(q, variant & 3);
    }
    out
}

/// Boxes for collision and the outline (pixels, facing turned in by `shapes::boxes`).
pub fn outline(variant: u8, collision: bool) -> &'static [[u8; 6]] {
    match (collision, is_head(variant)) {
        (true, _) => &[[0, 0, 0, 16, 9, 16]],
        (false, true) => &[[0, 0, 3, 16, 12, 16], [0, 0, 0, 16, 15, 3]],
        (false, false) => &[[0, 0, 0, 16, 11, 16]],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_point_at_each_other() {
        for f in 0..4 {
            let [a, b] = other_half(f);
            let [c, d] = other_half(f | HEAD);
            assert_eq!((a + c, b + d), (0, 0));
            assert_eq!(other_half(f | HEAD), [-dir(f)[0], -dir(f)[1]]);
        }
    }

    #[test]
    fn model_turns_with_the_bed() {
        let faces = [1, 1, 2, 3, 4, 1];
        let north = quads(HEAD, faces);
        let east = quads(HEAD | 1, faces);
        assert_eq!(north.len(), east.len());
        // The headboard is at the north edge facing north, at the east edge facing east.
        let tall = |qs: &[ModelQuad], edge: fn(&[u8; 3]) -> bool| qs.iter().any(|q| q.layer == 1 && q.pos.iter().all(|p| edge(p)) && q.pos.iter().any(|p| p[1] == 15));
        assert!(tall(&north, |p| p[2] <= 3));
        assert!(tall(&east, |p| p[0] >= 13));
        // Every material shows up and stays inside the block.
        for l in [1, 2, 3, 4] {
            assert!(north.iter().any(|q| q.layer == l), "layer {l}");
        }
        assert!(east.iter().all(|q| q.pos.iter().all(|p| p.iter().all(|&c| c <= 16))));
        // Legs stand on the ground and hide their soles against it.
        assert!(north.iter().any(|q| q.face == 3 && q.cull == 3));
    }
}
