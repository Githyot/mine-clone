//! Spears held in both hands (HP 2026-10-31: "o personagem deve usar ela com as duas mãos").
//!
//! The spear is the item's extruded picture (a long pole drawn corner to corner, see
//! tools/tex64/arms.py), scaled up to about two blocks and laid along a line instead of sitting
//! in the fist like a sword. The rear (right) hand holds it near the butt at the hip, the front
//! (left) hand further up the shaft, the body turned sideways with the left shoulder forward,
//! the point ahead. A thrust drives both hands forward along the shaft while the torso turns
//! into it and the front foot steps in.
//!
//! Third person: the hold is in the torso's own space (model pixels, +X left, +Y up, +Z ahead),
//! the arms reach it with a two-bone IK (upper arm and forearm, bent at the elbow joint), and
//! the spear is placed with the body bone. First person: the same idea in camera space.

use std::f32::consts::PI;

use glam::{Mat3, Mat4, Quat, Vec3};
use voxel::mob::PartDef;

use crate::entity::Bones;
use crate::player_model::Animator;

/// True for every spear item.
pub fn is_spear(name: &str) -> bool {
    name.ends_with("_spear")
}

/// The spear's length from butt to point, in model pixels (about two blocks).
pub const LENGTH: f32 = 34.0;
/// The same in first person, in blocks.
const FP_LENGTH: f32 = 2.1;
/// How far below the shoulder's pivot the fist holds (model pixels).
const GRIP: f32 = 9.5;

/// 0..1 how far out the thrust is: a quick jab out, a slower pull back.
pub fn thrust(swing: f32) -> f32 {
    if swing <= 0.0 {
        return 0.0;
    }
    let smooth = |x: f32| x * x * (3.0 - 2.0 * x);
    if swing < 0.3 {
        smooth(swing / 0.3)
    } else {
        1.0 - smooth((swing - 0.3) / 0.7)
    }
}

/// The torso's turn while holding a spear: sideways, left shoulder forward, leaning into a
/// thrust. Applied on top of the body's own lean (running, sneaking).
pub fn torso_turn(swing: f32) -> Quat {
    let e = thrust(swing);
    Quat::from_rotation_y(-(0.6 + 0.2 * e)) * Quat::from_rotation_x(0.12 * e)
}

/// Where the spear and both hands are, in the torso's space (model pixels).
#[derive(Clone, Copy, Debug)]
pub struct Hold {
    pub right: Vec3,
    pub left: Vec3,
    pub butt: Vec3,
    /// From the butt to the point.
    pub dir: Vec3,
}

pub fn hold(swing: f32) -> Hold {
    let e = thrust(swing);
    // Ahead of the body and a little up; the thrust drops the point onto the target.
    let aim = Vec3::new(0.0, 0.12 - 0.08 * e, 1.0).normalize();
    let dir = torso_turn(swing).inverse() * aim;
    let right = Vec3::new(-2.5, 16.0, 2.0) + dir * 7.0 * e;
    Hold { right, left: right + dir * 5.0, butt: right - dir * 9.0, dir }
}

/// The turn (in its own frame, about the pivot), elbow bend and shoulder slide of an arm whose
/// fist reaches `target` (model pixels): a two-bone IK with the elbow pointing down.
pub fn reach(part: &PartDef, target: Vec3) -> (Quat, f32, Vec3) {
    let pivot = Vec3::new(part.pivot.0 as f32, part.pivot.1 as f32, part.pivot.2 as f32);
    let jy = crate::entity::joint(part).map_or(pivot.y - 4.0, |(y, _)| y);
    let u = pivot.y - jy;
    let l = GRIP - u;
    let to = target - pivot;
    let d = to.length().max(0.01);
    let dir = to / d;
    // Out of reach: the arm goes straight and the shoulder leans in after it.
    let de = d.clamp((u - l).abs() + 0.5, u + l - 0.01);
    let slide = dir * (d - de);
    let cos_in = ((u * u + l * l - de * de) / (2.0 * u * l)).clamp(-1.0, 1.0);
    let beta = PI - cos_in.acos();
    // In the arm's frame (hanging along -Y), the forearm folds forward (+Z) by beta.
    let hand = Vec3::new(0.0, -u - l * beta.cos(), l * beta.sin());
    let e1 = hand.normalize();
    let e2 = (Vec3::Z - e1 * e1.z).normalize_or_zero();
    let e2 = if e2 == Vec3::ZERO { Vec3::Z } else { e2 };
    // The forearm rises toward the shaft from below: elbows down, a little out.
    let hint = Vec3::new(-pivot.x.signum() * 0.25, 1.0, 0.2);
    let w2 = (hint - dir * hint.dot(dir)).normalize_or_zero();
    let w2 = if w2 == Vec3::ZERO { Vec3::Y } else { w2 };
    let from = Mat3::from_cols(e1, e2, e1.cross(e2));
    let onto = Mat3::from_cols(dir, w2, dir.cross(w2));
    (Quat::from_mat3(&(onto * from.transpose())), -beta, slide)
}

/// The turn that lays the item picture's diagonal (butt bottom-left, point top-right) along
/// `dir`, its face toward `facing`.
fn lay(dir: Vec3, facing: Vec3) -> Quat {
    let along = Vec3::new(1.0, 1.0, 0.0).normalize();
    let to = dir.normalize();
    let n = (facing - to * facing.dot(to)).normalize_or_zero();
    let n = if n == Vec3::ZERO { to.any_orthonormal_vector() } else { n };
    let from = Mat3::from_cols(along, Vec3::Z.cross(along), Vec3::Z);
    let onto = Mat3::from_cols(to, n.cross(to), n);
    Quat::from_mat3(&(onto * from.transpose()))
}

/// The item model's centre frame (one picture wide before its scale) for a spear `len` long
/// from `butt` along `dir`. The pole runs from about (-0.43, -0.43) to (0.41, 0.41) of the
/// picture (arms.py: butt at pixel 4.6,59.4, point at u 76.5 along the diagonal).
pub fn frame(butt: Vec3, dir: Vec3, facing: Vec3, len: f32) -> Mat4 {
    let diag = (0.428 + 0.417) * std::f32::consts::SQRT_2;
    let s = len / diag;
    let center = butt + dir.normalize() * (0.428 * std::f32::consts::SQRT_2 * s);
    Mat4::from_translation(center) * Mat4::from_quat(lay(dir, facing)) * Mat4::from_scale(Vec3::splat(s))
}

/// The spear in third person: its frame in the world (relative to the camera's block), from
/// the body bone of a model posed with `Pose::spear`.
pub fn third_person(bones: &Bones, body: usize, swing: f32) -> Mat4 {
    let h = hold(swing);
    // The flat of the head stands upright, so the blade's shape shows from the side.
    let side = Vec3::Y.cross(h.dir);
    Mat4::from_cols_array_2d(&bones.m[body]) * frame(h.butt, h.dir, side, LENGTH)
}

/// First person, in camera space (x right, y up, -z ahead, blocks): the butt, the direction,
/// and both fists. The butt rests by the right hip below the view, the point aims just under
/// the crosshair; a thrust drives it ahead.
pub fn first_person_hold(anim: &Animator) -> (Vec3, Vec3, Vec3, Vec3) {
    let e = thrust(anim.swing);
    let dir = Vec3::new(-0.3, 0.15 - 0.05 * e, -1.0).normalize();
    let drop = Vec3::Y * -(1.0 - anim.hand_height.clamp(0.0, 1.0)) * 0.6;
    let right = Vec3::new(0.38, -0.44, -0.5) + anim.hand_sway() + drop + dir * 0.55 * e;
    let left = right + dir * 0.36;
    let butt = right - dir * 0.25;
    (butt, dir, right, left)
}

/// The spear's frame in first person: `cam` is the camera's frame.
pub fn first_person_frame(anim: &Animator, cam: Mat4) -> Mat4 {
    let (butt, dir, _, _) = first_person_hold(anim);
    // The flat of the head faces up, so the leaf shape shows looking down the shaft.
    cam * frame(butt, dir, Vec3::new(0.25, 1.0, 0.1), FP_LENGTH)
}

/// Both first-person arms gripping the spear: the right at the rear, the left ahead on the
/// shaft, each coming up from its lower corner of the view.
pub fn first_person_arms(anim: &Animator, def: &voxel::mob::MobDef, cam: Mat4, light: [f32; 4]) -> Bones {
    let mut b = Bones { light, ..Default::default() };
    for m in b.m.iter_mut() {
        *m = [[0.0f32; 4]; 4];
    }
    let (_, along, right, left) = first_person_hold(anim);
    for (part, fist, off) in [(2usize, right, Vec3::new(0.46, -0.66, 0.41)), (3usize, left, Vec3::new(-0.28, -0.74, 0.42))] {
        let Some(p) = def.parts.get(part) else { continue };
        let shoulder = fist + off;
        let d = (fist - shoulder).normalize();
        // The palm wraps the shaft: the arm's front faces along it.
        let x = along.cross(d).normalize_or_zero();
        let z = d.cross(x);
        let turn = if x == Vec3::ZERO { Quat::from_rotation_arc(Vec3::NEG_Y, d) } else { Quat::from_mat3(&Mat3::from_cols(x, -d, z)) };
        let pivot = Vec3::new(p.pivot.0 as f32, p.pivot.1 as f32, p.pivot.2 as f32);
        let fist_px = Vec3::new(0.0, -8.5, 0.0);
        let place = Mat4::from_translation(fist) * Mat4::from_quat(turn) * Mat4::from_rotation_y(PI) * Mat4::from_scale(Vec3::splat(1.0 / 16.0)) * Mat4::from_translation(-fist_px - pivot);
        b.m[part] = (cam * place).to_cols_array_2d();
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm(pivot: (i32, i32, i32), origin: (i32, i32, i32)) -> PartDef {
        let src = format!("(size: (4, 12, 4), origin: {origin:?}, uv: (0, 0), anim: Arm(0), pivot: {pivot:?})");
        ron::from_str(&src).unwrap()
    }

    /// Where the fist ends up for an arm turned and bent by `reach`.
    fn fist_of(part: &PartDef, target: Vec3) -> Vec3 {
        let (q, bend, slide) = reach(part, target);
        let pivot = Vec3::new(part.pivot.0 as f32, part.pivot.1 as f32, part.pivot.2 as f32);
        let (jy, _) = crate::entity::joint(part).unwrap();
        let u = pivot.y - jy;
        let l = GRIP - u;
        // The forearm folds by -bend about X (negative bends forward).
        let local = Vec3::new(0.0, -u, 0.0) + Quat::from_rotation_x(bend) * Vec3::new(0.0, -l, 0.0);
        pivot + slide + q * local
    }

    #[test]
    fn both_fists_land_on_the_shaft_through_a_thrust() {
        let right = arm((-6, 22, 0), (-8, 12, -2));
        let left = arm((6, 22, 0), (4, 12, -2));
        for k in 0..=10 {
            let s = k as f32 / 10.0;
            let h = hold(s);
            for (p, t) in [(&right, h.right), (&left, h.left)] {
                let f = fist_of(p, t);
                assert!((f - t).length() < 0.05, "swing {s}: fist {f} target {t}");
                // The shoulder never has to lean far out of its socket.
                assert!(reach(p, t).2.length() < 3.5, "swing {s}: slide {}", reach(p, t).2);
            }
            // Both hands on the pole, between the butt and the point.
            let along = |p: Vec3| (p - h.butt).dot(h.dir);
            assert!(along(h.right) > 0.0 && along(h.left) < LENGTH * 0.6);
        }
    }

    #[test]
    fn the_thrust_goes_out_and_back() {
        assert_eq!(thrust(0.0), 0.0);
        assert!(thrust(0.3) > 0.99);
        assert!(thrust(0.99) < 0.01);
        let rest = hold(0.0);
        let out = hold(0.3);
        // In the body's own frame the point moves ahead.
        let ahead = |h: &Hold, s: f32| (torso_turn(s) * h.right).z;
        assert!(ahead(&out, 0.3) > ahead(&rest, 0.0) + 5.0);
    }

    #[test]
    fn the_spear_points_ahead_of_the_body() {
        for s in [0.0, 0.3] {
            let d = torso_turn(s) * hold(s).dir;
            assert!(d.z > 0.95, "{d}");
        }
    }

    #[test]
    fn the_frame_puts_butt_and_point_where_asked() {
        let butt = Vec3::new(1.0, 2.0, 3.0);
        let dir = Vec3::new(0.3, 0.2, 1.0).normalize();
        let m = frame(butt, dir, Vec3::Y, 2.0);
        let b = m.transform_point3(Vec3::new(-0.428, -0.428, 0.0));
        let t = m.transform_point3(Vec3::new(0.417, 0.417, 0.0));
        assert!((b - butt).length() < 1e-3, "{b}");
        assert!((t - (butt + dir * 2.0)).length() < 1e-3, "{t}");
    }
}
