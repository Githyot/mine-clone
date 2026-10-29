//! Built-in 3D models for blocks that used to be flat pictures: `Shape::Model` blocks
//! (statues, trophies...), rails (sleepers and bars with real thickness, on curves and slopes
//! too) and ladders (stiles and rungs standing off the wall). All of it is chunk mesh, built
//! once per block id: nothing is drawn per frame.

use crate::block::{Block, Shape};
use crate::mesh::{model_quads, ModelElement, ModelFace, ModelQuad};
use crate::shapes::{facing_of, rail_on, rail_shape, RAIL_ENDS};

/// Face index after one quarter turn (north -> east), in mesh order +x, -x, +y, -y, +z, -z:
/// one turn sends (x, z) to (16 - z, x), so -z becomes +x, +x becomes +z, +z becomes -x and
/// -x becomes -z.
const TURN: [u8; 6] = [4, 5, 2, 3, 1, 0];

/// Quads built facing north, turned `k` quarter turns about the middle of the block. The
/// texture turns with them (unlike turning the boxes, which would re-project it).
pub fn turn_quads(quads: &mut [ModelQuad], k: u8) {
    for _ in 0..k & 3 {
        for q in quads.iter_mut() {
            for p in q.pos.iter_mut() {
                *p = [16 - p[2], p[1], p[0]];
            }
            q.face = TURN[q.face as usize];
            if q.cull < 6 {
                q.cull = TURN[q.cull as usize];
            }
        }
    }
}

/// The quads of a block's own 3D model, if it has one (models, rails, ladders).
pub fn block_quads(b: &Block) -> Option<Vec<ModelQuad>> {
    match b.shape {
        Shape::Model => {
            let mut q = model_quads(b.model.as_deref()?);
            turn_quads(&mut q, facing_of(b.variant));
            Some(q)
        }
        Shape::Rail => Some(rail(b)),
        Shape::Ladder => Some(ladder(b)),
        _ => None,
    }
}

/// A box whose faces all show `layer`, each with an optional picture area of its own.
fn boxed(from: [u8; 3], to: [u8; 3], layer: u16, uv: [Option<[f32; 4]>; 6]) -> ModelElement {
    let faces = std::array::from_fn(|f| {
        let axis = f / 2;
        let edge = if f % 2 == 0 { to[axis] >= 16 } else { from[axis] == 0 };
        Some(ModelFace { uv: uv[f], layer, cull: edge.then_some(f as u8), tint: false })
    });
    ModelElement { from: from.map(|v| v as f32), to: to.map(|v| v as f32), faces }
}

/// Picture area for a face `w` pixels wide and `h` high, starting at (u, v) of the picture.
fn area(u: u8, v: u8, w: u8, h: u8) -> Option<[f32; 4]> {
    Some([u as f32, v as f32, (u + w).min(16) as f32, (v + h).min(16) as f32])
}

/// A rail. The top picture (seen from above, running north-south) lies on the tops of the
/// sleepers and bars; their sides take the `front` picture, whose rows 0-1 are the bar's
/// metal and row 2 the sleeper's wood (`tools/gen_block_models.py`).
fn rail(b: &Block) -> Vec<ModelQuad> {
    let v = b.variant;
    let s = rail_shape(v);
    let top = b.faces[if rail_on(v) || s >= 6 { 2 } else { 0 }];
    let side = b.faces[4];
    let mut e = Vec::new();
    // Sideways faces of a bar `len` long and a sleeper; bars are 2 high, sleepers 1.
    let bar_sides = |len: u8, w: u8| [area(0, 0, len, 2), area(0, 0, len, 2), None, area(0, 0, w, 1), area(0, 0, w, 2), area(0, 0, w, 2)];
    if s < 6 {
        // Straight, built north-south. Sleepers and bars show the top picture on top.
        let with_top = |mut el: ModelElement| {
            el.faces[2].as_mut().unwrap().layer = top;
            el
        };
        for z in [1u8, 5, 9, 13] {
            e.push(with_top(boxed([1, 0, z], [15, 1, z + 2], side, [area(0, 2, 2, 1), area(0, 2, 2, 1), None, area(0, 2, 14, 2), area(0, 2, 14, 1), area(0, 2, 14, 1)])));
        }
        for x in [2u8, 12] {
            e.push(with_top(boxed([x, 1, 0], [x + 2, 3, 16], side, bar_sides(16, 2))));
        }
        // Powered and activator rails carry a redstone line down the middle, detector rails a
        // pressure plate: both stand a pixel high on the ground.
        let n = &b.name;
        if n.starts_with("powered_rail") || n.starts_with("activator_rail") {
            e.push(boxed([7, 0, 0], [9, 1, 16], top, [area(7, 1, 1, 1), area(7, 1, 1, 1), None, None, area(7, 1, 2, 1), area(7, 1, 2, 1)]));
        } else if n.starts_with("detector_rail") {
            e.push(boxed([5, 0, 5], [11, 1, 11], top, [area(5, 5, 6, 1), area(5, 5, 6, 1), None, None, area(5, 5, 6, 1), area(5, 5, 6, 1)]));
        }
        let mut q = model_quads(&e);
        let turn = if matches!(s, 1..=3) { 1 } else { 0 };
        turn_quads(&mut q, turn);
        // Slopes: lift every point by how far along the climb it is.
        for quad in q.iter_mut() {
            for p in quad.pos.iter_mut() {
                let lift = match s {
                    2 => p[0],
                    3 => 16 - p[0],
                    4 => 16 - p[2],
                    5 => p[2],
                    _ => 0,
                };
                p[1] += lift;
            }
            if (2..6).contains(&s) {
                // A sloping face no longer lies on the block's side.
                quad.cull = if quad.face == 3 || quad.face == 2 { 6 } else { quad.cull };
            }
        }
        return q;
    }
    // A curve: two bars bent round the inside corner the two ends meet at, sleepers fanning out.
    let [a, c] = RAIL_ENDS[s];
    let corner = [8.0 + 8.0 * (a[0] + c[0]) as f32, 8.0 + 8.0 * (a[1] + c[1]) as f32];
    let kind = |x: u8, z: u8| -> u8 {
        let (dx, dz) = (x as f32 + 0.5 - corner[0], z as f32 + 0.5 - corner[1]);
        let d = (dx * dx + dz * dz).sqrt();
        if (12.0..14.0).contains(&d) || (2.0..4.0).contains(&d) {
            return 2;
        }
        let ang = dz.abs().atan2(dx.abs());
        let near = (0..4).any(|k| {
            let t = (k as f32 * 2.0 + 1.0) * std::f32::consts::FRAC_PI_8 / 2.0;
            d * (ang - t).sin().abs() < 1.0
        });
        if (1.0..15.0).contains(&d) && near { 1 } else { 0 }
    };
    for z in 0..16u8 {
        let mut x = 0u8;
        while x < 16 {
            let k = kind(x, z);
            let mut end = x + 1;
            while end < 16 && kind(end, z) == k {
                end += 1;
            }
            let w = end - x;
            match k {
                2 => e.push(boxed([x, 1, z], [end, 3, z + 1], side, [area(0, 0, 1, 2), area(0, 0, 1, 2), area(0, 0, w, 1), area(0, 0, w, 1), area(0, 0, w, 2), area(0, 0, w, 2)])),
                1 => e.push(boxed([x, 0, z], [end, 1, z + 1], side, [area(0, 2, 1, 1), area(0, 2, 1, 1), area(0, 3, w, 1), area(0, 3, w, 1), area(0, 2, w, 1), area(0, 2, w, 1)])),
                _ => {}
            }
            x = end;
        }
    }
    model_quads(&e)
}

/// A ladder against the wall to the north: two stiles two pixels deep and rungs a pixel off
/// the wall, where the picture has them. Fronts and tops show the ladder picture itself.
fn ladder(b: &Block) -> Vec<ModelQuad> {
    let l = b.faces[4];
    let stile_side = area(1, 0, 2, 16);
    let mut e = Vec::new();
    for x in [1u8, 13] {
        e.push(boxed([x, 0, 0], [x + 2, 16, 2], l, [stile_side, stile_side, None, None, None, None]));
    }
    for y in [1u8, 5, 9, 13] {
        e.push(boxed([3, y, 0], [13, y + 2, 2], l, [None, None, None, None, None, None]));
    }
    let mut q = model_quads(&e);
    turn_quads(&mut q, facing_of(b.variant));
    q
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockRegistry;

    fn registry() -> BlockRegistry {
        BlockRegistry::from_ron_many(&[
            include_str!("../../../packs/vanilla/blocks.ron"),
            include_str!("../../../packs/vanilla/tr3/blocks.ron"),
            include_str!("../../../packs/vanilla/tr/blocks.ron"),
            include_str!("../../../packs/vanilla/fase6/blocks.ron"),
            include_str!("../../../packs/vanilla/mods/blocks.ron"),
            include_str!("../../../packs/vanilla/models/blocks.ron"),
        ])
        .unwrap()
    }

    #[test]
    fn models_stay_small_and_inside_their_block() {
        let reg = registry();
        let statue = reg.id("statue_skeleton").unwrap();
        assert_eq!(reg.get(statue).shape, Shape::Model);
        let mut worst = (0, String::new());
        for b in &reg.blocks {
            let Some(q) = block_quads(b) else { continue };
            assert!(!q.is_empty(), "{} has no quads", b.name);
            // Rails on a slope reach up into the block above, like the flat rail did.
            let top = if b.shape == Shape::Rail { 19 } else { 16 };
            for quad in &q {
                assert!(quad.pos.iter().all(|p| p[0] <= 16 && p[2] <= 16 && p[1] <= top), "{} leaves its block", b.name);
            }
            if q.len() > worst.0 {
                worst = (q.len(), b.name.clone());
            }
        }
        // Every model is chunk mesh; keep the busiest one well under a thousand faces.
        assert!(worst.0 < 700, "{} has {} quads", worst.1, worst.0);
    }

    #[test]
    fn turning_keeps_the_texture_and_moves_the_faces() {
        let reg = registry();
        let id = reg.id("statue_rabbit").unwrap();
        let north = block_quads(reg.get(id)).unwrap();
        let east = block_quads(reg.get(reg.variant(id, 1))).unwrap();
        assert_eq!(north.len(), east.len());
        for (a, b) in north.iter().zip(&east) {
            assert_eq!(a.uv, b.uv);
            assert_eq!(b.face, TURN[a.face as usize]);
            assert_eq!(b.pos[0], [16 - a.pos[0][2], a.pos[0][1], a.pos[0][0]]);
        }
    }

    #[test]
    fn rails_have_sleepers_bars_and_slopes() {
        let reg = registry();
        let rail = reg.id("rail").unwrap();
        let flat = block_quads(reg.get(rail)).unwrap();
        assert!(flat.iter().any(|q| q.face == 2 && q.pos[0][1] == 3), "bars stand 3 pixels high");
        let up = block_quads(reg.get(reg.variant(rail, 4))).unwrap();
        let high = |q: &[ModelQuad], z: u8| q.iter().flat_map(|q| q.pos).filter(|p| p[2] == z).map(|p| p[1]).max().unwrap();
        // Rising north: the north end is a block higher than the south end.
        assert_eq!(high(&up, 0), high(&up, 16) + 16);
        let curve = block_quads(reg.get(reg.variant(rail, 6))).unwrap();
        assert!(curve.len() > flat.len());
        let ladder = reg.id("ladder").unwrap();
        let l = block_quads(reg.get(ladder)).unwrap();
        assert!(l.iter().all(|q| q.pos.iter().all(|p| p[2] <= 2)), "a ladder keeps to the wall");
    }
}
