//! The torch in hand as a real 3D model: a long wooden haft with an iron collar, an oily rag
//! head bound with leather, charred on top, and a flame of three stacked tongues that sway
//! and flicker on their own bones. Flames, sparks and smoke rise off it as particles. The
//! redstone torch gets the same haft with a glowing redstone head and red dust.
//! Our own art, painted here in code into free swatches of the instrument skin.

use glam::{Mat4, Quat, Vec3};

use crate::entity::Bones;
use crate::instruments::{IBox, Piece};
use crate::Game;

/// Torch models, in mesh order: 0 the fire torch, 1 the redstone torch.
pub const COUNT: usize = 2;

pub fn index(name: &str) -> Option<usize> {
    match name {
        "torch" => Some(0),
        "redstone_torch" => Some(1),
        _ => None,
    }
}

// Materials: swatches 24.. of the instrument skin (instruments.rs uses 0..20).
const HAFT: u8 = 24;
const RAG: u8 = 25;
const CHAR: u8 = 26;
const IRON: u8 = 27;
const CORE: u8 = 28;
const FLAME: u8 = 29;
const TIP: u8 = 30;
const REDSTONE: u8 = 31;
const LEATHER: u8 = 6;

/// Where the flame sits on the haft (torch space, model pixels; +Y runs up the haft from the
/// grip).
const FLAME_BASE: f32 = 16.0;
/// The top of the flame, where the flames and sparks come off.
const FLAME_TOP: f32 = 25.5;
/// Size of the torch against the hand: a little over a block from grip to flame tip at full
/// size, so it stands well above the fist.
const SCALE: f32 = 0.72;

fn bx(lo: [f32; 3], hi: [f32; 3], mat: u8) -> IBox {
    IBox { lo: Vec3::from(lo), hi: Vec3::from(hi), mat }
}

/// A square box around the haft's axis, `w` wide, from `y0` to `y1`.
fn ring(w: f32, y0: f32, y1: f32, mat: u8) -> IBox {
    let h = w / 2.0;
    bx([-h, y0, -h], [h, y1, h], mat)
}

/// Two crossed slabs (a flame tongue seen from any side).
fn tongue(w: f32, n: f32, y0: f32, y1: f32, mat: u8) -> [IBox; 2] {
    let (h, n) = (w / 2.0, n / 2.0);
    [bx([-h, y0, -n], [h, y1, n], mat), bx([-n, y0, -h], [n, y1, h], mat)]
}

fn piece(part: u32, boxes: Vec<IBox>) -> Piece {
    Piece { part, place: Mat4::IDENTITY, boxes }
}

/// The model of torch `i`: bone 0 the haft and head, bones 1..=3 the flame's tongues.
pub fn model(i: usize) -> Vec<Piece> {
    let mut haft = vec![
        // The haft runs well below the fist, like a real torch held near its middle.
        ring(2.4, -4.5, 10.5, HAFT),
        ring(2.9, -5.1, -4.1, IRON),
        ring(3.0, 8.6, 9.8, IRON),
    ];
    if i == 1 {
        haft.extend([ring(2.6, 10.5, 12.0, IRON), ring(3.4, 12.0, 15.4, REDSTONE)]);
        return vec![
            piece(0, haft),
            // A small bright core over the redstone that pulses.
            piece(1, vec![ring(2.2, 15.4, 16.8, REDSTONE)]),
            piece(2, vec![ring(1.2, 16.8, 17.6, CORE)]),
            piece(3, Vec::new()),
        ];
    }
    haft.extend([
        // Rag wound round the head, bound by two leather straps, burnt black on top.
        ring(4.2, 10.0, 14.8, RAG),
        ring(4.5, 10.6, 11.4, LEATHER),
        ring(4.5, 12.8, 13.6, LEATHER),
        ring(3.8, 14.8, FLAME_BASE + 0.4, CHAR),
    ]);
    let core = vec![ring(3.6, FLAME_BASE, 18.6, CORE), ring(2.8, 18.6, 20.2, CORE)];
    let mut mid = tongue(4.4, 2.2, 17.6, 22.4, FLAME).to_vec();
    mid.extend(tongue(4.6, 2.6, 15.9, 17.6, FLAME));
    let mut tip = tongue(2.2, 1.2, 21.6, FLAME_TOP, TIP).to_vec();
    // Stray licks off to the sides.
    tip.push(bx([1.0, 20.6, -0.5], [2.1, 23.4, 0.5], TIP));
    tip.push(bx([-0.5, 19.8, -2.0], [0.5, 22.2, -1.0], TIP));
    vec![piece(0, haft), piece(1, core), piece(2, mid), piece(3, tip)]
}

/// Paint the torch materials into the instrument skin (`scale` texels per model pixel).
pub fn paint(px: &mut [u8], scale: u32) {
    let w = 64 * scale;
    let n = 16 * scale;
    let hash = |x: u32, y: u32, s: u32| {
        let mut v = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ s.wrapping_mul(0x27D4_EB2F);
        v ^= v >> 15;
        v = v.wrapping_mul(0x2C1B_3C6D);
        v ^= v >> 12;
        (v & 0xffff) as f32 / 65535.0
    };
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
    for m in HAFT..=REDSTONE {
        let m32 = m as u32;
        let (ox, oy) = ((m32 % 4) * n, (m32 / 4) * n);
        for y in 0..n {
            for x in 0..n {
                let (u, v) = (x as f32 / scale as f32, y as f32 / scale as f32);
                let r = hash(x, y, m32);
                let grain = ((v + (u * 0.4).sin() * 0.7) * 2.3).sin() * 0.5 + 0.5;
                let c = match m {
                    HAFT => mix([156.0, 112.0, 64.0], [116.0, 80.0, 44.0], grain.powf(2.0) * 0.7 + r * 0.25),
                    RAG => {
                        // Strips of cloth wound on a slant, soaked dark with pitch.
                        let wind = ((u * 0.9 + v * 1.3) * 1.1).sin();
                        let base = mix([118.0, 92.0, 62.0], [84.0, 62.0, 40.0], r * 0.5);
                        if wind > 0.55 { mix(base, [52.0, 38.0, 26.0], 0.7) } else { base }
                    }
                    CHAR => {
                        // Charred black with embers glowing in the cracks.
                        let crack = ((u * 1.4 + (v * 0.8).sin() * 2.0).sin() * (v * 1.1 + (u * 0.6).cos() * 1.5).sin()).abs();
                        if crack < 0.09 {
                            mix([255.0, 140.0, 40.0], [255.0, 200.0, 90.0], r)
                        } else {
                            mix([40.0, 30.0, 26.0], [22.0, 17.0, 15.0], r)
                        }
                    }
                    IRON => {
                        let rivet = (u - 4.0).rem_euclid(5.0) < 1.0 && (v - 0.5).rem_euclid(14.0) < 1.0;
                        if rivet { [150.0, 148.0, 140.0] } else { mix([62.0, 62.0, 66.0], [96.0, 94.0, 92.0], r * 0.6 + grain * 0.3) }
                    }
                    CORE => mix([255.0, 236.0, 140.0], [255.0, 206.0, 80.0], r * 0.6),
                    FLAME => mix([255.0, 190.0, 70.0], [250.0, 128.0, 30.0], r * 0.5 + (v / 16.0) * 0.4),
                    TIP => mix([246.0, 104.0, 30.0], [196.0, 56.0, 22.0], r * 0.6),
                    _ => {
                        let spark = r > 0.86;
                        if spark { [255.0, 130.0, 100.0] } else { mix([214.0, 28.0, 18.0], [150.0, 12.0, 8.0], grain * 0.5 + r * 0.3) }
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
}

/// The torch held in a fist: its bones and where the flame's top is (relative to the camera's
/// block). `arm` is the right arm bone (shoulder at `pivot`), `k` the scale about the fist,
/// `tilt` how far the torch tips up out of the fist (0 straight ahead, 1 at 45°).
pub fn hold(i: usize, arm: [[f32; 4]; 4], pivot: Vec3, k: f32, tilt: f32, t: f32, light: [f32; 4]) -> (Bones, Vec3) {
    let arm = Mat4::from_cols_array_2d(&arm);
    // Gripped through the fist, pointing forward and tipped up toward the shoulder side, so
    // the flame rises well above the hand.
    let fist = pivot + Vec3::new(0.0, -10.5, 0.2);
    let dir = Vec3::new(0.0, tilt, 1.0).normalize();
    let place = arm * Mat4::from_translation(fist) * Mat4::from_scale(Vec3::splat(k * SCALE)) * Mat4::from_quat(Quat::from_rotation_arc(Vec3::Y, dir));
    let mut b = Bones { light, ..Default::default() };
    b.m[0] = place.to_cols_array_2d();
    // The flame always leans up against gravity a little, whatever the arm does.
    let up = place.inverse().transform_vector3(Vec3::Y).normalize_or_zero();
    let lean = Quat::from_rotation_arc(Vec3::Y, (Vec3::Y * 2.0 + up).normalize());
    let base = Vec3::Y * FLAME_BASE;
    for p in 1..=3usize {
        let f = p as f32;
        // Several unrelated waves so the flicker never looks like a loop.
        let w = |a: f32, b: f32| (t * a + f * 1.7).sin() * 0.6 + (t * b + f * 0.9).sin() * 0.4;
        let (sx, sz) = (w(7.3, 12.9) * 0.06 * f, w(6.1, 15.7) * 0.06 * f);
        let tall = 1.0 + w(11.0, 17.3) * (0.06 + 0.06 * f) + if i == 1 { w(3.0, 5.0) * 0.15 } else { 0.0 };
        let wide = 1.0 - (tall - 1.0) * 0.4;
        let m = place
            * Mat4::from_translation(base)
            * Mat4::from_quat(lean * Quat::from_rotation_x(sx) * Quat::from_rotation_z(sz))
            * Mat4::from_scale(Vec3::new(wide, tall, wide))
            * Mat4::from_translation(-base);
        b.m[p] = m.to_cols_array_2d();
    }
    let top = place.transform_point3(Vec3::Y * if i == 1 { 16.0 } else { FLAME_TOP - 2.0 });
    (b, top)
}

impl Game {
    /// Which torch model the held item uses, if it is a torch.
    pub(crate) fn held_torch(&self) -> Option<usize> {
        let st = self.held_stack()?;
        index(&self.items.get(st.item).name)
    }

    /// Flames, sparks and smoke off the held torch's flame at `top` (world space), for `dt`
    /// seconds, `k` the size of the torch (smaller close to the eye in first person). Nothing
    /// burns under water.
    pub(crate) fn torch_flames(&mut self, i: usize, top: Vec3, k: f32, dt: f32) {
        let carry = self.body.vel;
        let cell = top.floor().as_ivec3();
        if voxel::physics::Terrain::liquid(&self.world, cell) {
            return;
        }
        let glow = self.glow_layer();
        self.torch_fx += dt * if i == 1 { 14.0 } else { 90.0 };
        while self.torch_fx >= 1.0 {
            self.torch_fx -= 1.0;
            if i == 1 {
                self.particles.flame(top, carry * 0.9, glow, 0.035 * k, 0.6, true);
                continue;
            }
            self.particles.flame(top, carry * 0.9, glow, 0.05 * k, 0.42 * k.sqrt(), false);
            self.torch_sparks += 1;
            if self.torch_sparks % 9 == 0 {
                // An ember that flies up higher and drifts.
                self.particles.spark(top, carry * 0.9, glow);
            }
            if self.torch_sparks % 6 == 0 {
                let stone = self.block_look(self.reg.id_or_air("stone")).0;
                self.particles.mote(top + Vec3::Y * 0.12 * k, stone, [0.3, 0.3, 0.3], 0.03 * k, -0.7, 1.1 * k);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torch_items_have_models_with_a_flame_above_the_fist() {
        assert_eq!(index("torch"), Some(0));
        assert_eq!(index("redstone_torch"), Some(1));
        assert_eq!(index("stick"), None);
        for i in 0..COUNT {
            let m = model(i);
            assert!(m.iter().all(|p| p.part < 4));
            let (_, top) = hold(i, Mat4::IDENTITY.to_cols_array_2d(), Vec3::ZERO, 1.0, 0.62, 0.0, [1.0; 4]);
            // Arm space: the fist is at y -10.5; the flame is up toward the shoulder and ahead.
            assert!(top.y > -10.5 && top.z > 5.0, "{top}");
        }
    }

    #[test]
    fn torch_paint_stays_in_free_swatches() {
        let mut px = crate::instruments::paint_skin(1);
        let before = px.clone();
        paint(&mut px, 1);
        // The instrument swatches (rows 0..5) are untouched.
        let row5_end = (5 * 16 * 64 * 4) as usize;
        assert_eq!(&px[..row5_end], &before[..row5_end]);
        assert!(px[row5_end..].chunks(4).filter(|p| p[3] == 255).count() >= 8 * 16 * 16);
    }
}
