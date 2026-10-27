//! Musical instruments as real 3D models held with both hands: a lute with a bowl back and a
//! rosette, a wooden flute, a cello with its bow, a painted didgeridoo, a bagpipe with drones,
//! a little keyboard, a drum with a stick and a handpan. Each is built from boxes at load time
//! (our own art, painted here in code) and drawn with the mob pipeline, one bone per moving
//! piece. While a melody plays, the hands follow its notes: strums and bow strokes on each
//! note, fingers sliding up the neck with the pitch, keys going down, the stick hitting.

use std::f32::consts::PI;

use glam::{Mat3, Mat4, Quat, Vec3};
use voxel::mob::{Anim, MobDef};

use crate::entity::Bones;
use crate::Game;

/// Instrument items, in the order of `mods::melodies::INSTRUMENTS`.
pub const COUNT: usize = 8;

pub fn index(name: &str) -> Option<usize> {
    crate::mods::melodies::INSTRUMENTS.iter().position(|n| *n == name)
}

// Materials: one 16 × 16 swatch each in the instrument skin.
const SPRUCE: u8 = 0;
const BOWL: u8 = 1;
const WOOD: u8 = 2;
const STRING: u8 = 3;
const HOLE: u8 = 4;
const BRASS: u8 = 5;
const LEATHER: u8 = 6;
const TARTAN: u8 = 7;
const IVORY: u8 = 8;
const KEY_BLACK: u8 = 9;
const STEEL: u8 = 10;
const SKIN: u8 = 11;
const BONE: u8 = 12;
const VARNISH: u8 = 13;
const DIDGE: u8 = 14;
const EBONY: u8 = 15;
const HAIR: u8 = 16;
const BOXWOOD: u8 = 17;
const ROSETTE: u8 = 18;
const STEEL_DARK: u8 = 19;
const MATERIALS: usize = 20;

/// One box of a model, in its piece's space (model pixels).
#[derive(Clone, Copy)]
pub struct IBox {
    pub lo: Vec3,
    pub hi: Vec3,
    pub mat: u8,
}

/// Boxes that move together: `place` puts them in the space of bone `part`.
pub struct Piece {
    pub part: u32,
    pub place: Mat4,
    pub boxes: Vec<IBox>,
}

fn bx(lo: [f32; 3], hi: [f32; 3], mat: u8) -> IBox {
    IBox { lo: Vec3::from(lo), hi: Vec3::from(hi), mat }
}

fn piece(part: u32, boxes: Vec<IBox>) -> Piece {
    Piece { part, place: Mat4::IDENTITY, boxes }
}

/// A rotation whose X axis is `x` and whose Z axis leans toward `z_hint`.
fn basis(x: Vec3, z_hint: Vec3) -> Quat {
    let x = x.normalize();
    let y = z_hint.cross(x).normalize();
    let z = x.cross(y);
    Quat::from_mat3(&Mat3::from_cols(x, y, z))
}

/// A round-ish tube along Y from 0 to `len`, `w` wide.
fn tube(len: f32, w: f32, mat: u8) -> Vec<IBox> {
    let (h, n) = (w / 2.0, w * 0.32);
    vec![bx([-h, 0.0, -n], [h, len, n], mat), bx([-n, 0.0, -h], [n, len, h], mat)]
}

/// A tube from `a` to `b` (in the instrument's space).
fn rod(part: u32, a: Vec3, b: Vec3, w: f32, mat: u8, rings: &[(f32, u8)]) -> Piece {
    let d = b - a;
    let len = d.length();
    let mut boxes = tube(len, w, mat);
    for &(t, m) in rings {
        let y = t * len;
        boxes.extend(tube(0.6, w + 0.5, m).into_iter().map(|mut q| {
            q.lo.y += y - 0.3;
            q.hi.y += y - 0.3;
            q
        }));
    }
    Piece { part, place: Mat4::from_translation(a) * Mat4::from_quat(Quat::from_rotation_arc(Vec3::Y, d / len)), boxes }
}

/// Piano keys: the white keys ride on bones 2..=14 so each can go down on its own.
const KEYS: usize = 13;

fn key_x(k: usize) -> f32 {
    // Low notes on the player's left (+X).
    7.15 - (k as f32 + 1.0) * 1.1
}

/// Bagpipe: where the chanter runs, in the player's body space.
const CHANTER: (Vec3, Vec3) = (Vec3::new(0.6, 18.0, 6.6), Vec3::new(-1.8, 10.5, 7.8));
const BAG_AT: Vec3 = Vec3::new(4.6, 16.2, 4.8);

/// The model of instrument `i`.
pub fn model(i: usize) -> Vec<Piece> {
    match i {
        // Lute: a bowl back of staves, a spruce top with a carved rosette, a bent-back pegbox.
        0 => {
            let mut b = vec![
                bx([-6.0, -3.0, -1.0], [4.0, 3.0, 1.0], BOWL),
                bx([-5.0, -4.0, -1.0], [3.0, 4.0, 1.0], BOWL),
                bx([-5.0, -2.5, -2.4], [3.0, 2.5, -1.0], BOWL),
                bx([-4.0, -1.5, -3.4], [2.0, 1.5, -2.4], BOWL),
                bx([-6.0, -3.0, 1.0], [4.0, 3.0, 1.3], SPRUCE),
                bx([-5.0, -4.0, 1.0], [3.0, 4.0, 1.3], SPRUCE),
                bx([-0.4, -1.3, 1.3], [2.0, 1.3, 1.36], ROSETTE),
                bx([-4.7, -2.2, 1.3], [-3.9, 2.2, 1.8], WOOD),
                bx([4.0, -1.0, -0.5], [13.0, 1.0, 1.3], WOOD),
                bx([3.0, -1.1, 1.3], [13.0, 1.1, 1.55], EBONY),
                bx([13.0, -0.9, -2.0], [16.5, 0.9, 0.7], WOOD),
            ];
            for k in 0..3 {
                let x = 13.6 + k as f32 * 1.0;
                b.push(bx([x, -1.9, -1.3], [x + 0.5, 1.9, -0.7], EBONY));
            }
            for k in 0..5 {
                let x = 5.0 + k as f32 * 1.6;
                b.push(bx([x, -1.12, 1.55], [x + 0.15, 1.12, 1.6], BONE));
            }
            for y in [-0.75f32, -0.25, 0.25, 0.75] {
                b.push(bx([-4.3, y - 0.07, 1.62], [13.0, y + 0.07, 1.72], STRING));
            }
            vec![piece(0, b)]
        }
        // Flute: boxwood with brass rings, a mouth hole and six finger holes.
        1 => {
            let mut b = vec![bx([0.0, -0.6, -0.6], [16.0, 0.6, 0.6], BOXWOOD)];
            for (x0, x1) in [(0.0, 0.7), (5.0, 5.5), (10.6, 11.1), (15.3, 16.0)] {
                b.push(bx([x0, -0.72, -0.72], [x1, 0.72, 0.72], BRASS));
            }
            b.push(bx([2.5, -0.32, 0.6], [3.3, 0.32, 0.66], HOLE));
            for x in [7.0f32, 8.2, 9.4, 11.7, 12.9, 14.1] {
                b.push(bx([x, -0.26, 0.6], [x + 0.5, 0.26, 0.66], HOLE));
            }
            vec![piece(0, b)]
        }
        // Cello and its bow (bone 1, in the right hand).
        2 => {
            let mut b = vec![
                bx([0.0, -5.0, -2.0], [7.0, 5.0, 2.0], VARNISH),
                bx([0.6, -5.6, -1.6], [6.4, 5.6, 1.6], VARNISH),
                bx([7.0, -3.4, -2.0], [10.0, 3.4, 2.0], VARNISH),
                bx([10.0, -4.4, -2.0], [15.0, 4.4, 2.0], VARNISH),
                bx([10.5, -4.9, -1.6], [14.5, 4.9, 1.6], VARNISH),
                bx([5.0, -2.9, 2.0], [10.0, -2.4, 2.06], HOLE),
                bx([5.0, 2.4, 2.0], [10.0, 2.9, 2.06], HOLE),
                bx([6.0, -2.0, 2.0], [6.6, 2.0, 3.3], SPRUCE),
                bx([1.0, -1.2, 2.0], [5.0, 1.2, 2.45], EBONY),
                bx([8.0, -1.2, 2.2], [24.0, 1.2, 2.8], EBONY),
                bx([15.0, -1.0, -0.6], [24.0, 1.0, 2.2], VARNISH),
                bx([24.0, -0.9, -0.6], [27.0, 0.9, 2.0], VARNISH),
                bx([27.0, -1.0, -0.3], [28.6, 1.0, 1.8], VARNISH),
                bx([-3.0, -0.25, -0.25], [0.0, 0.25, 0.25], STRING),
            ];
            for k in 0..2 {
                let x = 24.6 + k as f32 * 1.2;
                b.push(bx([x, -1.8, 0.2], [x + 0.5, 1.8, 0.8], EBONY));
            }
            for y in [-0.9f32, -0.3, 0.3, 0.9] {
                b.push(bx([4.6, y - 0.07, 3.0], [24.0, y + 0.07, 3.12], STRING));
            }
            let bow = vec![
                bx([0.0, -0.22, -0.22], [19.0, 0.22, 0.22], WOOD),
                bx([1.4, -0.16, -0.9], [18.2, 0.16, -0.55], HAIR),
                bx([0.0, -0.4, -1.2], [2.0, 0.4, 0.22], EBONY),
                bx([18.2, -0.3, -1.0], [19.0, 0.3, 0.22], BONE),
            ];
            vec![piece(0, b), piece(1, bow)]
        }
        // Didgeridoo: a long painted tube that widens to the bell, with a wax mouthpiece.
        3 => {
            let mut b = Vec::new();
            for (x0, x1, h) in [(0.0, 9.0, 1.0), (9.0, 16.0, 1.2), (16.0, 21.0, 1.45), (21.0, 24.0, 1.7)] {
                b.push(bx([x0, -h, -h * 0.7], [x1, h, h * 0.7], DIDGE));
                b.push(bx([x0, -h * 0.7, -h], [x1, h * 0.7, h], DIDGE));
            }
            b.push(bx([-0.6, -1.1, -1.1], [0.0, 1.1, 1.1], EBONY));
            b.push(bx([23.9, -1.2, -1.2], [24.05, 1.2, 1.2], HOLE));
            vec![piece(0, b)]
        }
        // Bagpipe, drawn right in the body's space: the tartan bag (bone 2, it breathes) under
        // the left arm, three drones over the shoulder, the blowpipe and the chanter.
        4 => {
            let bag = Piece {
                part: 2,
                place: Mat4::from_translation(BAG_AT) * Mat4::from_rotation_z(-0.35) * Mat4::from_rotation_y(0.4),
                boxes: vec![
                    bx([-3.0, -2.4, -2.4], [3.0, 2.4, 2.4], TARTAN),
                    bx([-2.4, -3.0, -1.9], [2.4, 3.0, 1.9], TARTAN),
                    bx([-2.4, -1.9, -3.0], [2.4, 1.9, 3.0], TARTAN),
                ],
            };
            let mut out = vec![bag];
            for k in 0..3 {
                let kf = k as f32;
                let a = BAG_AT + Vec3::new(0.6 + kf * 0.9, 1.6, -0.6 - kf * 0.4);
                let b = a + Vec3::new(1.6 + kf * 0.6, 8.0 + kf * 2.0, -4.0 - kf * 0.7);
                out.push(rod(0, a, b, 1.0, WOOD, &[(0.45, BONE), (0.97, BONE)]));
            }
            out.push(rod(0, BAG_AT + Vec3::new(-2.0, 2.0, 0.6), Vec3::new(0.2, 25.3, 4.4), 0.8, WOOD, &[(0.08, BONE)]));
            let mut ch = rod(0, CHANTER.0, CHANTER.1, 1.1, WOOD, &[(0.02, BONE), (0.97, BONE)]);
            // Finger holes on the front of the chanter.
            let len = (CHANTER.1 - CHANTER.0).length();
            for k in 0..7 {
                let y = len * (0.2 + k as f32 * 0.1);
                ch.boxes.push(bx([-0.2, y, 0.38], [0.2, y + 0.4, 0.62], HOLE));
            }
            out.push(ch);
            out
        }
        // A little keyboard, keys up (local Z), the player's side at +Y; white keys move.
        5 => {
            let mut fixed = vec![
                bx([-8.0, -3.2, -1.4], [8.0, 2.8, 0.0], BOWL),
                bx([-8.0, -3.2, 0.0], [8.0, -1.2, 2.0], BOWL),
                bx([-6.5, -3.25, 0.5], [6.5, -3.2, 1.6], HOLE),
                bx([-8.0, -1.2, 0.0], [-7.2, 2.8, 0.9], BOWL),
                bx([7.2, -1.2, 0.0], [8.0, 2.8, 0.9], BOWL),
            ];
            let mut out = Vec::new();
            for k in 0..KEYS {
                let x = key_x(k);
                out.push(piece(2 + k as u32, vec![bx([x + 0.05, -1.1, 0.0], [x + 1.05, 2.6, 0.45], IVORY)]));
            }
            // Black keys: two, then three, like a real keyboard.
            for k in 0..KEYS - 1 {
                if matches!(k % 7, 2 | 6) {
                    continue;
                }
                let x = key_x(k);
                fixed.push(bx([x - 0.3, -1.1, 0.45], [x + 0.35, 0.8, 0.95], KEY_BLACK));
            }
            out.push(piece(0, fixed));
            out
        }
        // Drum: an eight-sided shell, a skin on top, brass hoops; the stick is bone 1.
        6 => {
            let mut b = vec![
                bx([-4.0, -2.6, 0.0], [4.0, 2.6, 4.0], VARNISH),
                bx([-2.6, -4.0, 0.0], [2.6, 4.0, 4.0], VARNISH),
                bx([-3.4, -3.4, 0.0], [3.4, 3.4, 4.0], VARNISH),
                bx([-4.1, -2.7, 3.5], [4.1, 2.7, 4.2], BRASS),
                bx([-2.7, -4.1, 3.5], [2.7, 4.1, 4.2], BRASS),
                bx([-3.5, -3.5, 3.5], [3.5, 3.5, 4.2], BRASS),
                bx([-3.7, -2.4, 4.2], [3.7, 2.4, 4.3], SKIN),
                bx([-2.4, -3.7, 4.2], [2.4, 3.7, 4.3], SKIN),
                bx([-3.1, -3.1, 4.2], [3.1, 3.1, 4.3], SKIN),
                bx([-4.1, -2.7, -0.1], [4.1, 2.7, 0.5], BRASS),
                bx([-2.7, -4.1, -0.1], [2.7, 4.1, 0.5], BRASS),
            ];
            // Rope tensioners around the shell.
            for k in 0..8 {
                let a = k as f32 / 8.0 * 2.0 * PI;
                let (x, y) = (a.cos() * 3.9, a.sin() * 3.9);
                b.push(bx([x - 0.2, y - 0.2, 0.5], [x + 0.2, y + 0.2, 3.5], LEATHER));
            }
            let stick = vec![bx([0.0, -0.25, -0.25], [7.4, 0.25, 0.25], BONE), bx([7.0, -0.7, -0.7], [8.4, 0.7, 0.7], LEATHER)];
            vec![piece(0, b), piece(1, stick)]
        }
        // Handpan: two steel shells, a raised ding in the middle and eight tone fields.
        _ => {
            let mut b = Vec::new();
            for (z0, z1, r) in [(0.0, 1.6, 5.4), (1.6, 2.6, 5.0), (2.6, 3.3, 4.0)] {
                let k = r * 0.66;
                b.push(bx([-r, -k, z0], [r, k, z1], STEEL));
                b.push(bx([-k, -r, z0], [k, r, z1], STEEL));
                let m = r * 0.86;
                b.push(bx([-m, -m * 0.86, z0], [m, m * 0.86, z1], STEEL));
            }
            b.push(bx([-1.3, -1.3, 3.3], [1.3, 1.3, 3.9], STEEL));
            b.push(bx([-0.8, -0.8, 3.9], [0.8, 0.8, 4.05], STEEL_DARK));
            b.push(bx([-1.5, -1.5, -0.05], [1.5, 1.5, 0.0], HOLE));
            for k in 0..8 {
                let (x, y) = handpan_field(k + 1);
                b.push(bx([x - 0.75, y - 0.6, 3.3], [x + 0.75, y + 0.6, 3.36], STEEL_DARK));
            }
            vec![piece(0, b)]
        }
    }
}

/// Where tone field `k` sits on the handpan top (0 is the ding in the middle).
fn handpan_field(k: usize) -> (f32, f32) {
    if k == 0 {
        return (0.0, 0.0);
    }
    let a = (k as f32 - 1.0) / 8.0 * 2.0 * PI + 0.2;
    (a.cos() * 3.0, a.sin() * 3.0)
}

/// Paint the instrument skin: `scale` texels per model pixel, in the mob skin layout
/// (64 × 128 model pixels), one 16 × 16 swatch per material.
pub fn paint_skin(scale: u32) -> Vec<u8> {
    let (w, h) = (64 * scale, 128 * scale);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let hash = |x: u32, y: u32, s: u32| {
        let mut v = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ s.wrapping_mul(0xC2B2_AE3D);
        v ^= v >> 15;
        v = v.wrapping_mul(0x2C1B_3C6D);
        v ^= v >> 12;
        (v & 0xffff) as f32 / 65535.0
    };
    let n = 16 * scale;
    for m in 0..MATERIALS as u32 {
        let (ox, oy) = ((m % 4) * n, (m / 4) * n);
        for y in 0..n {
            for x in 0..n {
                // u runs along the grain, v across it; both 0..16 model pixels.
                let u = x as f32 / scale as f32;
                let v = y as f32 / scale as f32;
                let r = hash(x, y, m);
                // Grain: stripes across v that wobble along u.
                let grain = |freq: f32, s: u32| {
                    let wob = (u * 0.35 + hash(0, (v * 0.5) as u32, s) * 6.0).sin() * 0.6;
                    ((v + wob) * freq).sin() * 0.5 + 0.5
                };
                let mix = |a: [f32; 3], b: [f32; 3], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
                let c: [f32; 3] = match m as u8 {
                    SPRUCE => mix([206.0, 166.0, 108.0], [178.0, 132.0, 78.0], grain(2.6, 1).powf(3.0) * 0.8 + r * 0.15),
                    BOWL => {
                        // Staves of two woods, light and dark, with thin dark seams.
                        let stave = (v / 2.0).floor() as i32;
                        let base = if stave % 2 == 0 { [150.0, 92.0, 48.0] } else { [112.0, 64.0, 34.0] };
                        let seam = if (v / 2.0).fract() < 0.1 { 0.55 } else { 1.0 };
                        let c = mix(base, [base[0] * 0.8, base[1] * 0.8, base[2] * 0.8], grain(3.1, 2) * 0.6 + r * 0.2);
                        [c[0] * seam, c[1] * seam, c[2] * seam]
                    }
                    WOOD => mix([96.0, 58.0, 32.0], [70.0, 41.0, 22.0], grain(2.2, 3) * 0.8 + r * 0.2),
                    STRING => {
                        let s = 200.0 + r * 30.0;
                        [s, s * 0.97, s * 0.88]
                    }
                    HOLE => [24.0 + r * 8.0, 17.0 + r * 6.0, 12.0 + r * 4.0],
                    BRASS => {
                        let sheen = ((u + v) * 0.7).sin() * 0.5 + 0.5;
                        mix([168.0, 124.0, 52.0], [226.0, 184.0, 96.0], sheen * 0.7 + r * 0.2)
                    }
                    LEATHER => mix([116.0, 70.0, 40.0], [88.0, 50.0, 28.0], r * 0.7 + grain(1.3, 4) * 0.3),
                    TARTAN => {
                        // Our own sett: dark green ground, navy bands, thin rust lines.
                        let band = |t: f32| {
                            let p = t.rem_euclid(8.0);
                            if p < 3.0 {
                                1
                            } else if (5.0..5.5).contains(&p) {
                                2
                            } else {
                                0
                            }
                        };
                        let (a, b) = (band(u), band(v));
                        let col = |k: i32| match k {
                            1 => [34.0, 44.0, 82.0],
                            2 => [170.0, 72.0, 40.0],
                            _ => [38.0, 84.0, 52.0],
                        };
                        let c = mix(col(a), col(b), 0.5);
                        // A twill weave: alternate which thread shows.
                        let tw = if ((x / scale.max(1)) + (y / scale.max(1))) % 2 == 0 { col(a) } else { col(b) };
                        mix(c, tw, 0.45 + r * 0.1)
                    }
                    IVORY => {
                        let s = 236.0 - r * 14.0 - if v > 15.0 { 30.0 } else { 0.0 };
                        [s, s * 0.97, s * 0.9]
                    }
                    KEY_BLACK => [28.0 + r * 10.0, 26.0 + r * 10.0, 30.0 + r * 10.0],
                    STEEL | STEEL_DARK => {
                        // Hammered steel: soft dimples and a bronze sheen from the seasoning.
                        let d = ((u * 1.9).sin() * (v * 1.7).sin()) * 0.5 + 0.5;
                        let c = mix([92.0, 98.0, 104.0], [138.0, 132.0, 118.0], d * 0.6 + r * 0.25);
                        if m as u8 == STEEL_DARK { [c[0] * 0.72, c[1] * 0.7, c[2] * 0.66] } else { c }
                    }
                    SKIN => mix([232.0, 214.0, 176.0], [206.0, 184.0, 140.0], r * 0.5 + grain(0.9, 5) * 0.3),
                    BONE => mix([230.0, 220.0, 196.0], [204.0, 192.0, 164.0], r * 0.6),
                    VARNISH => {
                        let flame = ((v * 1.4 + (u * 0.5).sin() * 1.5).sin() * 0.5 + 0.5) * 0.35;
                        mix([150.0, 66.0, 30.0], [110.0, 42.0, 20.0], grain(2.0, 6) * 0.5 + flame + r * 0.1)
                    }
                    DIDGE => {
                        // Bark-brown wood painted with ochre and white dots in rows.
                        let base = mix([148.0, 92.0, 52.0], [118.0, 70.0, 38.0], grain(1.6, 7) * 0.7 + r * 0.2);
                        let (cu, cv) = ((u * 0.5).fract() - 0.5, ((v + (u * 0.5).floor()) * 0.5).fract() - 0.5);
                        if cu * cu + cv * cv < 0.06 {
                            if ((u * 0.5).floor() as i32) % 3 == 0 { [236.0, 226.0, 200.0] } else { [214.0, 152.0, 60.0] }
                        } else {
                            base
                        }
                    }
                    EBONY => mix([40.0, 26.0, 18.0], [26.0, 17.0, 12.0], grain(3.0, 8) * 0.6 + r * 0.3),
                    HAIR => {
                        let s = 222.0 + ((v * 4.0).sin() * 0.5 + 0.5) * 18.0 - r * 10.0;
                        [s, s * 0.96, s * 0.86]
                    }
                    BOXWOOD => mix([214.0, 180.0, 120.0], [188.0, 150.0, 92.0], grain(2.4, 9) * 0.7 + r * 0.2),
                    _ => {
                        // Rosette: a lattice of thin wood over the dark hole.
                        let (cu, cv) = (u - 8.0, v - 8.0);
                        let a = cv.atan2(cu);
                        let rr = (cu * cu + cv * cv).sqrt();
                        let lattice = ((a * 6.0).sin().abs() < 0.25) || ((rr * 1.6).sin().abs() < 0.22) || (a * 6.0 + rr).sin().abs() < 0.15;
                        if lattice && rr < 7.5 { [196.0, 156.0, 98.0] } else { [26.0, 18.0, 12.0] }
                    }
                };
                let i = (((oy + y) * w + ox + x) * 4) as usize;
                px[i] = c[0].clamp(0.0, 255.0) as u8;
                px[i + 1] = c[1].clamp(0.0, 255.0) as u8;
                px[i + 2] = c[2].clamp(0.0, 255.0) as u8;
                px[i + 3] = 255;
            }
        }
    }
    px
}

/// Texture rectangle (x, y, w, h in model pixels) for a face of `len_u` × `len_v` pixels.
pub fn swatch(mat: u8, len_u: f32, len_v: f32) -> (f32, f32, f32, f32) {
    let (ox, oy) = ((mat as u32 % 4) as f32 * 16.0, (mat as u32 / 4) as f32 * 16.0);
    (ox + 1.0, oy + 1.0, len_u.clamp(0.25, 14.0), len_v.clamp(0.25, 14.0))
}

/// What the music is doing right now, for the hands.
#[derive(Clone, Copy, Default)]
pub struct Play {
    /// Seconds since the note now sounding began, and how long it lasts.
    pub since: f32,
    pub dur: f32,
    /// 0..1, low to high.
    pub pitch: f32,
    /// Which note of the melody (for alternating strums, bows and hands).
    pub note: usize,
    /// Seconds since the melody began.
    pub t: f32,
}

impl Play {
    /// A steady beat for when the melody isn't known (other players).
    pub fn steady(t: f32) -> Play {
        let beat = 0.36;
        let note = (t / beat) as usize;
        Play { since: t % beat, dur: beat, pitch: ((note * 7) % 12) as f32 / 12.0, note, t }
    }

    /// 1 right at the note's start, fading quickly.
    fn pulse(&self) -> f32 {
        (-self.since * 10.0).exp()
    }
}

/// How an instrument is held this frame: one matrix per bone of its mesh and the targets
/// for the two hands, all in the player model's body space (pixels, +Z ahead, +X the left).
pub struct Hold {
    pub parts: [Mat4; 16],
    pub right: Vec3,
    pub left: Vec3,
    /// Blown: the head faces ahead to the mouthpiece while playing.
    pub wind: bool,
}

pub fn hold(i: usize, play: Option<Play>) -> Hold {
    let p = play.unwrap_or(Play { since: 9.0, dur: 1.0, pitch: 0.4, ..Default::default() });
    let playing = play.is_some();
    let pulse = if playing { p.pulse() } else { 0.0 };
    let alt = if p.note % 2 == 0 { 1.0 } else { -1.0 };
    let place = |pos: Vec3, rot: Quat, s: f32| Mat4::from_translation(pos) * Mat4::from_quat(rot) * Mat4::from_scale(Vec3::splat(s));
    // Keys up: local Z up, local +Y toward the player.
    let flat = |tilt: f32| Quat::from_rotation_x(tilt) * Quat::from_mat3(&Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y));
    let mut h = Hold { parts: [Mat4::IDENTITY; 16], right: Vec3::ZERO, left: Vec3::ZERO, wind: false };
    let mut moved = [false; 16];
    let base = match i {
        0 => {
            let m = place(Vec3::new(-2.6, 14.2, 6.4), Quat::from_rotation_z(0.38) * Quat::from_rotation_y(-0.25), 0.8);
            // Fingers slide toward the body for higher notes and press on each new note.
            let neck = 12.0 - p.pitch * 4.0;
            h.left = m.transform_point3(Vec3::new(neck, 0.0, 0.3 - pulse * 0.4));
            // Strums across the strings, down and up in turn, quick at the note's start.
            let s = if playing { alt * 1.5 * (PI * (p.since / 0.14).min(1.0)).cos() } else { 0.0 };
            h.right = m.transform_point3(Vec3::new(-1.6, s, 2.8 + pulse * 0.3));
            m
        }
        1 => {
            h.wind = true;
            let rot = basis(Vec3::new(-1.0, -0.1, 0.22), Vec3::new(0.0, 0.7, 1.0));
            let mouth = Vec3::new(0.0, 25.3, 4.7);
            let m = Mat4::from_translation(mouth) * Mat4::from_quat(rot) * Mat4::from_translation(Vec3::new(-2.9, 0.0, 0.0));
            // Fingers lift off the holes and close them again with the notes.
            let lift_l = if p.note % 3 != 1 { pulse * 0.5 } else { 0.0 };
            let lift_r = if p.note % 3 != 0 { pulse * 0.5 } else { 0.0 };
            h.left = m.transform_point3(Vec3::new(8.0, 0.0, -0.4 + lift_l));
            h.right = m.transform_point3(Vec3::new(13.0, 0.0, -0.4 + lift_r));
            m
        }
        2 => {
            let rot = basis(Vec3::new(0.2, 0.97, -0.1), Vec3::Z);
            let s = 0.68;
            let tip = Vec3::new(1.6, 3.2, 7.0);
            let m = Mat4::from_translation(tip) * Mat4::from_quat(rot) * Mat4::from_scale(Vec3::splat(s)) * Mat4::from_translation(Vec3::new(3.0, 0.0, 0.0));
            let neck = 21.5 - p.pitch * 6.0;
            h.left = m.transform_point3(Vec3::new(neck, -0.6, 1.8 - pulse * 0.5));
            // The bow crosses the strings between bridge and fingerboard and draws along its
            // length through each note, down-bow and up-bow in turn.
            let f = if playing { (p.since / p.dur.max(0.2)).min(1.0) } else { 0.5 };
            let along = if alt > 0.0 { 4.0 + 11.0 * f } else { 15.0 - 11.0 * f };
            let contact = Vec3::new(7.6, 0.0, 3.5);
            let bow_rot = Mat3::from_cols(Vec3::NEG_Y, Vec3::X, Vec3::Z);
            let tilt = Quat::from_rotation_x(0.12);
            let bow = Mat4::from_translation(contact + Vec3::Y * along) * Mat4::from_quat(tilt * Quat::from_mat3(&bow_rot)) * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.9));
            h.parts[1] = m * bow;
            moved[1] = true;
            h.right = (m * bow).transform_point3(Vec3::new(1.0, 0.0, 0.6));
            m
        }
        3 => {
            h.wind = true;
            // The whole tube sways a little with the breath.
            let sway = if playing { (p.t * 2.2).sin() * 0.03 } else { 0.0 };
            let rot = basis(Vec3::new(0.0, -0.66 + sway, 0.75), Vec3::Y);
            let mouth = Vec3::new(0.0, 25.2, 4.6);
            let m = Mat4::from_translation(mouth) * Mat4::from_quat(rot) * Mat4::from_scale(Vec3::splat(0.9)) * Mat4::from_translation(Vec3::new(0.6, 0.0, 0.0));
            let tap = pulse * 0.6;
            h.right = m.transform_point3(Vec3::new(7.0, 0.0, 0.0)) + Vec3::new(-1.0, -1.3 + tap, 0.0);
            h.left = m.transform_point3(Vec3::new(11.0, 0.0, 0.0)) + Vec3::new(1.0, -1.3, 0.0);
            m
        }
        4 => {
            h.wind = true;
            // The bag is squeezed slowly under the arm as the piper breathes.
            let breath = if playing { (p.t * 1.6).sin() * 0.5 + 0.5 } else { 0.0 };
            let k = 1.0 - breath * 0.08;
            moved[2] = true;
            h.parts[2] = Mat4::from_translation(BAG_AT) * Mat4::from_scale(Vec3::new(k, 1.0 + breath * 0.03, k)) * Mat4::from_translation(-BAG_AT);
            let (a, b) = CHANTER;
            let lift_l = if p.note % 2 == 0 { pulse * 0.6 } else { 0.0 };
            let lift_r = if p.note % 2 == 1 { pulse * 0.6 } else { 0.0 };
            h.left = a.lerp(b, 0.3) + Vec3::new(0.0, 0.0, 0.9 + lift_l);
            h.right = a.lerp(b, 0.68) + Vec3::new(0.0, 0.0, 0.9 + lift_r);
            Mat4::IDENTITY
        }
        5 => {
            let m = place(Vec3::new(0.0, 14.0, 7.2), flat(-0.4), 0.85);
            // Low notes on the left hand's keys, high ones on the right's.
            let key = ((p.pitch * (KEYS - 1) as f32).round() as usize).min(KEYS - 1);
            let press = pulse;
            if playing {
                // The key stays down while its note sounds.
                let down = if p.since < p.dur { 0.35 } else { 0.35 * press };
                h.parts[2 + key] = m * Mat4::from_translation(Vec3::new(0.0, 0.0, -down));
                moved[2 + key] = true;
            }
            let hover = |k: usize, down: f32| m.transform_point3(Vec3::new(key_x(k) + 0.55, 1.4, 1.5 - down));
            let (lk, rk) = if key < KEYS / 2 { (key, 9) } else { (3, key) };
            h.left = hover(lk, if key < KEYS / 2 { press * 0.8 } else { 0.0 });
            h.right = hover(rk, if key >= KEYS / 2 { press * 0.8 } else { 0.0 });
            m
        }
        6 => {
            let m = place(Vec3::new(1.4, 11.4, 6.2), flat(-0.3), 0.85);
            h.left = m.transform_point3(Vec3::new(4.8, 0.6, 2.2));
            // The stick hits at each note and rises again.
            let lift = if playing { 3.2 * (1.0 - pulse) } else { 2.6 };
            let hit = m.transform_point3(Vec3::new(-0.8, 0.4, 4.3));
            let hand = hit + Vec3::new(-5.0, 0.8 + lift, -0.8);
            let d = hit - hand;
            let stick = Mat4::from_translation(hand)
                * Mat4::from_quat(basis(d, Vec3::Y))
                * Mat4::from_scale(Vec3::new(d.length() / 7.7, 0.85, 0.85))
                * Mat4::from_translation(Vec3::new(-0.3, 0.0, 0.0));
            h.parts[1] = stick;
            moved[1] = true;
            h.right = hand + Vec3::new(0.0, 0.0, -0.3);
            m
        }
        _ => {
            let m = place(Vec3::new(0.0, 12.4, 7.6), flat(-0.5), 0.95);
            // Each note on its own tone field; the hands take turns.
            let field = if playing { (p.note * 5 + (p.pitch * 8.0) as usize) % 9 } else { 0 };
            let (fx, fy) = handpan_field(field);
            let tap = |x: f32, y: f32, down: f32| m.transform_point3(Vec3::new(x, y, 4.6 - down * 1.1));
            let right_turn = field != 0 && fx < 0.0 || (field == 0 && p.note % 2 == 0);
            let rest_r = tap(-3.0, 0.4, 0.0);
            let rest_l = tap(3.0, 0.4, 0.0);
            if playing && right_turn {
                h.right = tap(fx, fy, pulse);
                h.left = rest_l;
            } else if playing {
                h.left = tap(fx, fy, pulse);
                h.right = rest_r;
            } else {
                h.right = rest_r;
                h.left = rest_l;
            }
            m
        }
    };
    // Bones not moved on their own ride with the instrument.
    for (k, part) in h.parts.iter_mut().enumerate() {
        if k == 0 || !moved[k] {
            *part = base;
        }
    }
    h
}

/// The arm bone (shoulder at `pivot`) turned so the fist reaches `target`, in body space.
/// A single box can't bend at the elbow, so when the target is close the arm slides back into
/// the shoulder a little (the elbow is "behind" it) and shortens slightly.
pub fn aim_arm(pivot: Vec3, target: Vec3) -> Mat4 {
    let to = target - pivot;
    let d = to.length().max(0.01);
    let dir = to / d;
    let reach = 10.0;
    let k = (d / reach).clamp(0.86, 1.12);
    let slide = (d - reach * k).clamp(-4.0, 1.0);
    // The palm faces in a little: turn about the arm toward the body's middle.
    let rot = Quat::from_rotation_arc(Vec3::NEG_Y, dir);
    let twist = Quat::from_axis_angle(dir, if pivot.x < 0.0 { -0.35 } else { 0.35 });
    Mat4::from_translation(pivot + dir * slide) * Mat4::from_quat(twist * rot) * Mat4::from_scale(Vec3::new(1.0, k, 1.0)) * Mat4::from_translation(-pivot)
}

/// Part indexes of a humanoid model: body, head, right arm, left arm.
fn humanoid(def: &MobDef) -> Option<(usize, usize, usize, usize)> {
    let head = def.parts.iter().position(|p| p.anim == Anim::Head)?;
    let ra = def.parts.iter().position(|p| p.anim == Anim::Arm(0))?;
    let la = def.parts.iter().position(|p| p.anim == Anim::Arm(1))?;
    let body = def.parts.iter().position(|p| p.anim == Anim::None)?;
    Some((body, head, ra, la))
}

fn pivot(def: &MobDef, part: usize) -> Vec3 {
    let p = def.parts[part].pivot;
    Vec3::new(p.0 as f32, p.1 as f32, p.2 as f32)
}

/// Put the instrument in the hands of a humanoid drawn with `bones`: both arms reach for it
/// (and the head faces the mouthpiece of a wind instrument being played). Returns the
/// instrument's own bones.
pub fn pose(def: &MobDef, bones: &mut Bones, i: usize, play: Option<Play>) -> Option<Bones> {
    let (body, head, ra, la) = humanoid(def)?;
    let h = hold(i, play);
    let b = Mat4::from_cols_array_2d(&bones.m[body]);
    bones.m[ra] = (b * aim_arm(pivot(def, ra), h.right)).to_cols_array_2d();
    bones.m[la] = (b * aim_arm(pivot(def, la), h.left)).to_cols_array_2d();
    if h.wind && play.is_some() {
        let hp = pivot(def, head);
        let nod = play.map(|p| (p.t * 2.1).sin() * 0.03).unwrap_or(0.0);
        bones.m[head] = (b * Mat4::from_translation(hp) * Mat4::from_rotation_x(nod) * Mat4::from_translation(-hp)).to_cols_array_2d();
    }
    Some(instrument_bones(b, &h, bones.light))
}

fn instrument_bones(body: Mat4, h: &Hold, light: [f32; 4]) -> Bones {
    let mut out = Bones { light: [light[0], light[1], 0.0, 0.0], ..Default::default() };
    for (o, p) in out.m.iter_mut().zip(h.parts.iter()) {
        *o = (body * *p).to_cols_array_2d();
    }
    out
}

/// First person: the two arms and the instrument in front of the camera. Returns the bones
/// for the player model (only the arms drawn) and for the instrument.
pub fn first_person(def: &MobDef, i: usize, play: Option<Play>, eye: Vec3, yaw: f32, pitch: f32, light: [f32; 4]) -> Option<(Bones, Bones)> {
    let (_, _, ra, la) = humanoid(def)?;
    let mut h = hold(i, play);
    // Between the hands, in camera space (x right, y up, -z ahead), and how far the
    // instrument tips its face up toward the eye, like looking down at it while playing.
    let (at, turn) = match i {
        0 => (Vec3::new(0.08, -0.16, -0.55), -2.0),
        1 => (Vec3::new(0.1, -0.13, -0.5), -0.6),
        2 => (Vec3::new(0.08, -0.26, -0.62), -2.0),
        3 => (Vec3::new(0.08, -0.26, -0.5), -1.1),
        4 => (Vec3::new(0.12, -0.16, -0.72), -0.9),
        5 => (Vec3::new(0.0, -0.2, -0.5), -0.6),
        6 => (Vec3::new(0.0, -0.2, -0.5), -0.7),
        _ => (Vec3::new(0.0, -0.2, -0.52), -0.7),
    };
    let (at, turn) = match std::env::var("MC_FP").ok().map(|v| v.split(',').filter_map(|x| x.parse::<f32>().ok()).collect::<Vec<_>>()) {
        Some(v) if v.len() >= 4 => (Vec3::new(v[1], v[2], v[3]), v[0]),
        _ => (at, turn),
    };
    let k = std::env::var("MC_FPK").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.6);
    let center = (h.right + h.left) * 0.5;
    let spin = Mat4::from_translation(center) * Mat4::from_rotation_x(turn) * Mat4::from_translation(-center);
    for p in h.parts.iter_mut() {
        *p = spin * *p;
    }
    h.right = spin.transform_point3(h.right);
    h.left = spin.transform_point3(h.left);
    let cam = Mat4::from_translation(eye) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(pitch);
    // Body space (+Z ahead, +X left, pixels) to camera space.
    let local = Mat4::from_translation(at) * Mat4::from_rotation_y(PI) * Mat4::from_scale(Vec3::splat(k / 16.0)) * Mat4::from_translation(-center);
    let b = cam * local;
    let to_body = local.inverse();
    let mut arms = Bones { light, ..Default::default() };
    for m in arms.m.iter_mut() {
        *m = [[0.0; 4]; 4];
    }
    // The arms come up from the lower corners of the view: each shoulder sits one arm's
    // length from its hand, down, out and back toward the eye.
    for (part, target, x) in [(ra, h.right, 1.0f32), (la, h.left, -1.0)] {
        let pv = pivot(def, part);
        let hand = local.transform_point3(target);
        let shoulder = to_body.transform_point3(hand + Vec3::new(x * 0.45, -0.75, 0.5).normalize() * (10.0 * k / 16.0));
        let shift = shoulder - pv;
        arms.m[part] = (b * Mat4::from_translation(shift) * aim_arm(pv, target - shift)).to_cols_array_2d();
    }
    Some((arms, instrument_bones(b, &h, light)))
}

/// A town NPC playing a tune for the people around (one at a time, near the player).
pub struct NpcMusic {
    pub mob: u32,
    pub inst: usize,
    pub tune: usize,
    pub t: f32,
    pub len: f32,
}

#[derive(Default)]
pub struct Musicians {
    pub now: Option<NpcMusic>,
    pub wait: f32,
}

impl Game {
    /// The instrument in hand and what its melody is doing, if one is held.
    pub(crate) fn held_instrument(&self) -> Option<(usize, Option<Play>)> {
        let st = self.held_stack()?;
        let i = index(&self.items.get(st.item).name)?;
        if let Some(t) = std::env::var("MC_PLAY").ok().and_then(|v| v.parse::<f32>().ok()) {
            // Test hook: frozen this many seconds into the picked melody.
            return Some((i, Some(self.mods.melodies.play_at(i, self.mods.melodies.pick, t))));
        }
        let play = self.mods.melodies.playing.as_ref().filter(|p| p.inst == i && self.melody_cd > 0.0).map(|p| {
            let t = (p.len - self.melody_cd).max(0.0);
            self.mods.melodies.play_at(i, p.tune, t)
        });
        Some((i, play))
    }

    /// Now and then a town NPC near the player takes out an instrument and plays a tune,
    /// standing still until it ends.
    pub(crate) fn update_musicians(&mut self, dt: f32) {
        if let Some(n) = self.musicians.now.as_mut() {
            n.t += dt;
            let id = n.mob;
            let alive = self.mobs.list.iter().any(|m| m.net_id == id && m.alive());
            if n.t > n.len || !alive {
                self.musicians.now = None;
                self.musicians.wait = 50.0 + self.surv.rng.f32() * 60.0;
            }
            return;
        }
        let forced = std::env::var("MC_NPC_MUSIC").ok().and_then(|v| v.parse::<usize>().ok());
        self.musicians.wait -= dt;
        if self.musicians.wait > 0.0 && forced.is_none() {
            return;
        }
        self.musicians.wait = 20.0;
        let me = self.body.pos;
        let near: Vec<usize> = (0..self.mobs.list.len())
            .filter(|&k| {
                let m = &self.mobs.list[k];
                let d = &self.mobs.reg.defs[m.def];
                d.town && m.alive() && !m.sleeping && m.body.pos.distance(me) < 20.0 && humanoid(d).is_some()
            })
            .collect();
        if near.is_empty() {
            return;
        }
        let k = near[(self.surv.rng.f32() * near.len() as f32) as usize % near.len()];
        crate::mods::melodies::load_custom(&self.base.join("melodies"));
        let tunes = crate::mods::melodies::melody_count().max(1);
        let tune = (self.surv.rng.f32() * tunes as f32) as usize % tunes;
        let m = &self.mobs.list[k];
        // Each NPC has a favourite instrument.
        let inst = forced.unwrap_or((m.phase * 977.0) as usize) % COUNT;
        let len = crate::mods::melodies::melody_seconds(tune, inst as u8).clamp(4.0, 40.0);
        let (id, pos) = (m.net_id, m.body.pos);
        self.mobs.list[k].go_to(pos, len + 1.0);
        self.musicians.now = Some(NpcMusic { mob: id, inst, tune, t: if forced.is_some() { 2.3 } else { 0.0 }, len });
        self.sfx_at(crate::audio::Sfx::Tune(inst as u8, tune as u16), pos + Vec3::Y * 1.2, 0.9, 28.0);
    }

    /// The playing NPC drawn with the instrument: its entry in `items` gets the two-handed
    /// pose; the instrument is returned (it is no mob, so it joins the list later).
    pub(crate) fn musician_instances(&self, items: &mut [(usize, Bones)], cam_block: glam::IVec3) -> Option<(usize, Bones)> {
        let n = self.musicians.now.as_ref()?;
        let m = self.mobs.list.iter().find(|m| m.net_id == n.mob)?;
        let def = &self.mobs.reg.defs[m.def];
        let l = self.world.light((m.body.pos + Vec3::Y * (def.height * 0.6)).floor().as_ivec3());
        let b0 = self.mobs.anim.bones_now(m, def, m.def, cam_block, (l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
        let entry = items.iter_mut().find(|(d, b)| *d == m.def && b.m[0] == b0.m[0] && b.m[1] == b0.m[1])?;
        let play = self.mods.melodies.play_at(n.inst, n.tune, n.t);
        let ib = pose(def, &mut entry.1, n.inst, Some(play))?;
        Some((self.entities.instrument_mesh(n.inst)?, ib))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_instrument_has_a_model_and_hands_within_reach() {
        for i in 0..COUNT {
            let pieces = model(i);
            assert!(pieces.iter().map(|p| p.boxes.len()).sum::<usize>() > 3, "instrument {i}");
            for p in &pieces {
                assert!(p.part < 15);
                for b in &p.boxes {
                    assert!(b.hi.cmpge(b.lo).all(), "instrument {i}: inside-out box");
                }
            }
            for play in [None, Some(Play { since: 0.05, dur: 0.4, pitch: 0.7, note: 3, t: 1.0 })] {
                let h = hold(i, play);
                for (target, shoulder) in [(h.right, Vec3::new(-6.0, 22.0, 0.0)), (h.left, Vec3::new(6.0, 22.0, 0.0))] {
                    let d = target.distance(shoulder);
                    assert!((5.0..16.0).contains(&d), "instrument {i}: hand {d:.1} px from the shoulder");
                    // In front of the chest, not inside it.
                    assert!(target.z > 1.5, "instrument {i}: hand behind the chest {target}");
                }
            }
        }
    }

    #[test]
    fn aimed_arm_reaches_the_target() {
        let pv = Vec3::new(-6.0, 22.0, 0.0);
        for t in [Vec3::new(-2.0, 14.0, 7.0), Vec3::new(0.0, 25.0, 6.0), Vec3::new(4.0, 18.0, 5.0)] {
            let m = aim_arm(pv, t);
            let fist = m.transform_point3(pv + Vec3::new(0.0, -10.0, 0.0));
            assert!(fist.distance(t) < 3.0, "fist {fist} target {t}");
        }
    }

    #[test]
    fn skin_is_opaque_everywhere_it_is_used() {
        let px = paint_skin(1);
        for m in 0..MATERIALS as u32 {
            let (x, y) = ((m % 4) * 16 + 8, (m / 4) * 16 + 8);
            assert_eq!(px[((y * 64 + x) * 4 + 3) as usize], 255);
        }
    }
}
