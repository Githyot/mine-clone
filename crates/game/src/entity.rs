//! Mob rendering: box models from `mobs.ron` turned into meshes once, drawn instanced with one
//! matrix per body part. Skin "layers" are voxelized at load time, so wool, fur and torn clothes
//! stand off the body as real relief instead of a flat painting (the 3D Skin Layers idea).

use std::path::Path;

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::embedded;
use crate::paintings::PaintingDef;
use crate::renderer::Renderer;
use voxel::mob::{Anim, MobDef, MobRegistry, PartDef};

/// The skin layout, in model pixels (mobs.ron's texture coordinates).
pub const SKIN_W: u32 = 64;
pub const SKIN_H: u32 = 128;
/// Texels per model pixel of the base game's skins (64x art: a block is 16 model pixels).
pub const SKIN_DETAIL: u32 = 4;
pub(crate) const MAX_PARTS: usize = 16;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EntityVertex {
    pos: [f32; 3],
    uv: [f32; 2],
    /// part index (4 bits) | shade * 255 (8 bits)
    packed: u32,
    layer: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Bones {
    pub m: [[[f32; 4]; 4]; MAX_PARTS],
    /// sky light, block light, hurt flash, swell
    pub light: [f32; 4],
    /// Knees and elbows, per part: (joint height, joint depth, bend angle, unused) in model
    /// pixels. Limb meshes are split at the joint (see `joint`), and the shader turns
    /// everything below it about the joint's X axis, blending over a pixel either side.
    pub bend: [[f32; 4]; MAX_PARTS],
}

impl Default for Bones {
    fn default() -> Self {
        let mut m = [Mat4::IDENTITY.to_cols_array_2d(); MAX_PARTS];
        // The last slot carries the eyelids (see `add_eyelids`): folded to a point = eyes open.
        m[EYELID_PART] = [[0.0; 4]; 4];
        Bones { m, light: [1.0, 0.0, 0.0, 0.0], bend: [[0.0; 4]; MAX_PARTS] }
    }
}

impl Bones {
    /// Bend part `i` (a leg or arm) at its knee or elbow by `angle` (positive swings the lower
    /// half back, toward -Z).
    pub fn set_bend(&mut self, i: usize, p: &PartDef, angle: f32) {
        if let (Some((y, z)), true) = (joint(p), i < MAX_PARTS) {
            self.bend[i] = [y, z, angle, 0.0];
        }
    }

    /// Where the lower half of part `i` is (the forearm, for something held in the hand).
    pub fn forearm(&self, i: usize) -> [[f32; 4]; 4] {
        let [y, z, a, _] = self.bend[i];
        let m = Mat4::from_cols_array_2d(&self.m[i]);
        if a == 0.0 {
            return self.m[i];
        }
        let j = Vec3::new(0.0, y, z);
        (m * Mat4::from_translation(j) * Mat4::from_rotation_x(a) * Mat4::from_translation(-j)).to_cols_array_2d()
    }
}

/// The knee or elbow of a limb part: halfway down, in the middle of its depth. Short legs
/// (chickens, spiders) don't get one.
pub(crate) fn joint(p: &PartDef) -> Option<(f32, f32)> {
    if !matches!(p.anim, Anim::Leg(_) | Anim::Arm(_) | Anim::ArmForward(_)) || p.size.1 < 6 {
        return None;
    }
    Some((p.origin.1 as f32 + (p.size.1 / 2) as f32, p.origin.2 as f32 + p.size.2 as f32 * 0.5))
}

/// Test hook for screenshots: MC_BLINK=1 keeps every mob's eyes shut.
pub(crate) fn eyes_shut_test() -> bool {
    static SHUT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *SHUT.get_or_init(|| std::env::var("MC_BLINK").is_ok())
}

/// Part slot of the eyelid quads, for models with fewer parts than that.
pub(crate) const EYELID_PART: usize = MAX_PARTS - 1;

struct MobMesh {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    indices: u32,
}

pub struct EntityRenderer {
    /// [drawing straight to the screen, drawing into the HDR scene for shader packs]
    pipelines: [wgpu::RenderPipeline; 2],
    skin_bg: wgpu::BindGroup,
    inst_layout: wgpu::BindGroupLayout,
    inst_bg: wgpu::BindGroup,
    inst_buf: wgpu::Buffer,
    inst_cap: usize,
    meshes: Vec<MobMesh>,
    pub particles: crate::particles::ParticleGpu,
    pub items: crate::item_render::ItemGpu,
    /// Meshes after this index are paintings.
    mob_meshes: usize,
    /// First armour mesh (material * 4 + piece), when there is a player model to fit it to.
    armor_meshes: Option<usize>,
    /// First armour trim mesh (trim material * 4 + piece), drawn just over the armour.
    trim_meshes: Option<usize>,
    /// (mesh, first instance, count) for this frame.
    draws: Vec<(usize, u32, u32)>,
    pub instance_count: usize,
}

/// A skin picture: the SKIN_W × SKIN_H layout drawn with `scale` texels per model pixel.
struct Skin {
    pixels: Vec<u8>,
    scale: u32,
}

impl Skin {
    fn width(&self) -> u32 {
        SKIN_W * self.scale
    }

    /// Opacity of model pixel (x, y), read at the middle of its texels.
    fn alpha(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= SKIN_W as i32 || y >= SKIN_H as i32 {
            return 0;
        }
        let s = self.scale;
        let (tx, ty) = (x as u32 * s + s / 2, y as u32 * s + s / 2);
        self.pixels[((ty * self.width() + tx) * 4 + 3) as usize]
    }

    /// The same picture with `scale` texels per model pixel (crisp growth, averaged shrinking).
    fn rescaled(self, scale: u32) -> Skin {
        if scale == self.scale {
            return self;
        }
        Skin { pixels: fit_rgba(&self.pixels, self.width(), SKIN_H * self.scale, SKIN_W * scale, SKIN_H * scale), scale }
    }
}

/// Resize an RGBA picture to w × h: by whole multiples with crisp pixels when it grows, by
/// averaging when it shrinks by powers of two, nearest otherwise.
fn fit_rgba(px: &[u8], sw: u32, sh: u32, w: u32, h: u32) -> Vec<u8> {
    if sw == w && sh == h {
        return px.to_vec();
    }
    if sw > w && sw % w == 0 && (sw / w).is_power_of_two() && sh * w == h * sw {
        let levels = (sw / w).trailing_zeros() + 1;
        return crate::textures::mip_chain_rect(px, sw, sh, levels).pop().unwrap();
    }
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = (x * sw / w, y * sh / h);
            let s = ((sy * sw + sx) * 4) as usize;
            let d = ((y * w + x) * 4) as usize;
            out[d..d + 4].copy_from_slice(&px[s..s + 4]);
        }
    }
    out
}

/// A mob's skin at `scale` texels per model pixel. Skins may be drawn at any multiple of the
/// 64 × 128 layout (our own are 4×, a 1× skin from an older pack still fits).
fn load_skin(base: &Path, name: &str, scale: u32) -> Skin {
    let bytes = std::fs::read(base.join(format!("packs/vanilla/textures/entity/{name}.png")))
        .ok()
        .or_else(|| embedded::ENTITY_TEXTURES.iter().find(|(n, _)| *n == name).map(|(_, b)| b.to_vec()));
    let (w, h) = (SKIN_W * scale, SKIN_H * scale);
    match bytes.and_then(|b| image::load_from_memory(&b).ok()) {
        Some(img) => {
            let img = img.to_rgba8();
            // Pictures that are not 1:2 (old 64 × 64 skins) sit at the top of the layout.
            let k = (img.width() / SKIN_W).max(1);
            let mut full = vec![0u8; (SKIN_W * k * SKIN_H * k * 4) as usize];
            for y in 0..img.height().min(SKIN_H * k) {
                for x in 0..img.width().min(SKIN_W * k) {
                    let i = ((y * SKIN_W * k + x) * 4) as usize;
                    full[i..i + 4].copy_from_slice(&img.get_pixel(x, y).0);
                }
            }
            Skin { pixels: fit_rgba(&full, SKIN_W * k, SKIN_H * k, w, h), scale }
        }
        None => {
            log::warn!("missing mob skin '{name}'");
            let mut pixels = vec![0u8; (w * h * 4) as usize];
            for y in 0..64 * scale {
                for x in 0..64 * scale {
                    let i = ((y * w + x) * 4) as usize;
                    let m = ((x / (8 * scale)) + (y / (8 * scale))) % 2 == 0;
                    pixels[i..i + 4].copy_from_slice(if m { &[248, 0, 248, 255] } else { &[0, 0, 0, 255] });
                }
            }
            Skin { pixels, scale }
        }
    }
}

/// One face of a box: where texel (0,0) sits, the directions of texture u and v, the outward normal.
struct Face {
    corner: Vec3,
    u: Vec3,
    v: Vec3,
    n: Vec3,
    /// Texture region (x, y, w, h) in pixels.
    rect: (i32, i32, i32, i32),
    shade: f32,
}

fn faces(origin: Vec3, size: Vec3, uv: (u16, u16), tex_size: (i32, i32, i32)) -> [Face; 6] {
    let (x0, y0, z0) = (origin.x, origin.y, origin.z);
    let (w, h, d) = (size.x, size.y, size.z);
    let (tw, th, td) = tex_size;
    let (u, v) = (uv.0 as i32, uv.1 as i32);
    [
        // +Z front
        Face { corner: Vec3::new(x0, y0 + h, z0 + d), u: Vec3::X * w, v: Vec3::NEG_Y * h, n: Vec3::Z, rect: (u + td, v + td, tw, th), shade: 0.8 },
        // -Z back
        Face { corner: Vec3::new(x0 + w, y0 + h, z0), u: Vec3::NEG_X * w, v: Vec3::NEG_Y * h, n: Vec3::NEG_Z, rect: (u + 2 * td + tw, v + td, tw, th), shade: 0.8 },
        // -X right
        Face { corner: Vec3::new(x0, y0 + h, z0), u: Vec3::Z * d, v: Vec3::NEG_Y * h, n: Vec3::NEG_X, rect: (u, v + td, td, th), shade: 0.6 },
        // +X left
        Face { corner: Vec3::new(x0 + w, y0 + h, z0 + d), u: Vec3::NEG_Z * d, v: Vec3::NEG_Y * h, n: Vec3::X, rect: (u + td + tw, v + td, td, th), shade: 0.6 },
        // +Y top
        Face { corner: Vec3::new(x0, y0 + h, z0), u: Vec3::X * w, v: Vec3::Z * d, n: Vec3::Y, rect: (u + td, v, tw, td), shade: 1.0 },
        // -Y bottom
        Face { corner: Vec3::new(x0, y0, z0 + d), u: Vec3::X * w, v: Vec3::NEG_Z * d, n: Vec3::NEG_Y, rect: (u + td + tw, v, tw, td), shade: 0.5 },
    ]
}

struct MeshBuilder {
    verts: Vec<EntityVertex>,
    indices: Vec<u32>,
    layer: u32,
    /// Knee/elbow height per part: side faces are cut there so the limb can bend.
    joints: [Option<f32>; MAX_PARTS],
}

impl MeshBuilder {
    fn new(layer: u32, def: Option<&MobDef>) -> MeshBuilder {
        let mut joints = [None; MAX_PARTS];
        if let Some(d) = def {
            for (i, p) in d.parts.iter().enumerate().take(MAX_PARTS) {
                joints[i] = joint(p).map(|j| j.0);
            }
        }
        MeshBuilder { verts: Vec::new(), indices: Vec::new(), layer, joints }
    }

    fn quad(&mut self, p: [Vec3; 4], uv: [[f32; 2]; 4], n: Vec3, shade: f32, part: u32) {
        let base = self.verts.len() as u32;
        let packed = part | (((shade * 255.0) as u32) << 4);
        for k in 0..4 {
            self.verts.push(EntityVertex { pos: p[k].to_array(), uv: uv[k], packed, layer: self.layer });
        }
        // Counter-clockwise seen from outside.
        let cross = (p[1] - p[0]).cross(p[3] - p[0]);
        if cross.dot(n) >= 0.0 {
            self.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        } else {
            self.indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }

    fn face(&mut self, f: &Face, part: u32) {
        let (rx, ry, rw, rh) = f.rect;
        let uv = |x: i32, y: i32| [x as f32 / SKIN_W as f32, y as f32 / SKIN_H as f32];
        // A side of a limb: two quads, cut at the joint.
        if let Some(jy) = self.joints.get(part as usize).copied().flatten() {
            let (top, bot) = (f.corner.y, f.corner.y + f.v.y);
            if f.v.x == 0.0 && f.v.z == 0.0 && f.v.y < 0.0 && jy < top - 0.01 && jy > bot + 0.01 {
                let s = (top - jy) / (top - bot);
                let uvf = |x: f32, y: f32| [x / SKIN_W as f32, y / SKIN_H as f32];
                let (x0, x1, y0, y1) = (rx as f32, (rx + rw) as f32, ry as f32, (ry + rh) as f32);
                let ym = y0 + (y1 - y0) * s;
                let mid = f.corner + f.v * s;
                self.quad([f.corner, f.corner + f.u, mid + f.u, mid], [uvf(x0, y0), uvf(x1, y0), uvf(x1, ym), uvf(x0, ym)], f.n, f.shade, part);
                self.quad([mid, mid + f.u, f.corner + f.u + f.v, f.corner + f.v], [uvf(x0, ym), uvf(x1, ym), uvf(x1, y1), uvf(x0, y1)], f.n, f.shade, part);
                return;
            }
        }
        self.quad(
            [f.corner, f.corner + f.u, f.corner + f.u + f.v, f.corner + f.v],
            [uv(rx, ry), uv(rx + rw, ry), uv(rx + rw, ry + rh), uv(rx, ry + rh)],
            f.n,
            f.shade,
            part,
        );
    }

    /// Relief layer: each opaque texel becomes a little block standing `depth` off the body.
    fn relief(&mut self, f: &Face, skin: &Skin, depth: f32, part: u32) {
        let (rx, ry, rw, rh) = f.rect;
        if rw <= 0 || rh <= 0 {
            return;
        }
        let su = f.u / rw as f32;
        let sv = f.v / rh as f32;
        let back = -f.n * depth;
        for j in 0..rh {
            for i in 0..rw {
                if skin.alpha(rx + i, ry + 64 + j) < 128 {
                    continue;
                }
                let c = f.corner + su * i as f32 + sv * j as f32;
                let tex = [(rx + i) as f32 + 0.5, (ry + 64 + j) as f32 + 0.5];
                let uvc = [tex[0] / SKIN_W as f32, tex[1] / SKIN_H as f32];
                let uv4 = [uvc; 4];
                // Outward face: the whole texel square, so a detailed skin shows its detail.
                let uvp = |x: f32, y: f32| [x / SKIN_W as f32, y / SKIN_H as f32];
                let (u0, v0) = ((rx + i) as f32, (ry + 64 + j) as f32);
                let cell = [uvp(u0, v0), uvp(u0 + 1.0, v0), uvp(u0 + 1.0, v0 + 1.0), uvp(u0, v0 + 1.0)];
                self.quad([c, c + su, c + su + sv, c + sv], cell, f.n, f.shade, part);
                // Side walls where the neighbouring texel is empty.
                let open = |di: i32, dj: i32| skin.alpha(rx + i + di, ry + 64 + j + dj) < 128;
                let side_shade = (f.shade * 0.85).max(0.4);
                if open(0, -1) {
                    self.quad([c, c + su, c + su + back, c + back], uv4, -sv.normalize_or_zero(), side_shade, part);
                }
                if open(0, 1) {
                    let b = c + sv;
                    self.quad([b, b + su, b + su + back, b + back], uv4, sv.normalize_or_zero(), side_shade, part);
                }
                if open(-1, 0) {
                    self.quad([c, c + sv, c + sv + back, c + back], uv4, -su.normalize_or_zero(), side_shade, part);
                }
                if open(1, 0) {
                    let b = c + su;
                    self.quad([b, b + sv, b + sv + back, b + back], uv4, su.normalize_or_zero(), side_shade, part);
                }
            }
        }
    }
}

fn build_mesh(def: &MobDef, skin: &Skin, layer: u32) -> (Vec<EntityVertex>, Vec<u32>) {
    let mut mb = MeshBuilder::new(layer, Some(def));
    for (pi, p) in def.parts.iter().enumerate() {
        let origin = Vec3::new(p.origin.0 as f32, p.origin.1 as f32, p.origin.2 as f32);
        let size = Vec3::new(p.size.0 as f32, p.size.1 as f32, p.size.2 as f32);
        let tex = (p.size.0 as i32, p.size.1 as i32, p.size.2 as i32);
        for f in faces(origin, size, p.uv, tex).iter() {
            mb.face(f, pi as u32);
        }
    }
    add_eyelids(&mut mb, def, skin);
    for l in &def.layers {
        let p = &def.parts[l.part];
        let inflate = l.inflate.max(0.1);
        let origin = Vec3::new(p.origin.0 as f32, p.origin.1 as f32, p.origin.2 as f32) - Vec3::splat(inflate);
        let size = Vec3::new(p.size.0 as f32, p.size.1 as f32, p.size.2 as f32) + Vec3::splat(2.0 * inflate);
        let tex = (p.size.0 as i32, p.size.1 as i32, p.size.2 as i32);
        for f in faces(origin, size, p.uv, tex).iter() {
            mb.relief(f, skin, inflate, l.part as u32);
        }
    }
    (mb.verts, mb.indices)
}

/// Eyelids (Fresh Animations idea): the eye pixels on the front of the head are found in the
/// skin, and each gets a little quad just in front of it, textured with the skin right above
/// the eye. They ride on their own part slot, which is folded away except while blinking, so
/// blinking costs nothing but one matrix.
fn add_eyelids(mb: &mut MeshBuilder, def: &MobDef, skin: &Skin) {
    if def.parts.len() > EYELID_PART || def.name == "player" {
        return;
    }
    let Some(hi) = def.parts.iter().position(|p| p.anim == Anim::Head) else { return };
    let p = &def.parts[hi];
    let origin = Vec3::new(p.origin.0 as f32, p.origin.1 as f32, p.origin.2 as f32);
    let size = Vec3::new(p.size.0 as f32, p.size.1 as f32, p.size.2 as f32);
    let tex = (p.size.0 as i32, p.size.1 as i32, p.size.2 as i32);
    let f = &faces(origin, size, p.uv, tex)[0];
    // A relief layer on the head (3D skin layer) shows in front of the base face where it has
    // pixels; lids for those pixels sit on the layer.
    let layer = def.layers.iter().find(|l| l.part == hi).map(|l| {
        let inf = l.inflate.max(0.1);
        let fl = &faces(origin - Vec3::splat(inf), size + Vec3::splat(2.0 * inf), p.uv, tex)[0];
        (fl.corner, fl.u, fl.v)
    });
    let (rx, ry, rw, rh) = f.rect;
    if rw < 3 || rh < 3 {
        return;
    }
    let raw = |x: i32, y: i32| -> [f32; 4] {
        if x < 0 || y < 0 || x >= SKIN_W as i32 || y >= SKIN_H as i32 {
            return [0.0; 4];
        }
        let k = ((y as u32 * SKIN_W + x as u32) * 4) as usize;
        let c = &skin.pixels[k..k + 4];
        [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0]
    };
    // What shows at face pixel (i, j): (colour, skin row it comes from).
    let px = |i: i32, j: i32| -> ([f32; 4], i32) {
        if i < 0 || j < 0 || i >= rw || j >= rh {
            return ([0.0; 4], ry + j);
        }
        if layer.is_some() {
            let o = raw(rx + i, ry + 64 + j);
            if o[3] > 0.5 {
                return (o, ry + 64 + j);
            }
        }
        (raw(rx + i, ry + j), ry + j)
    };
    let luma = |c: [f32; 4]| c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11;
    // The face's usual colour.
    let mut avg = [0.0f32; 3];
    let mut n = 0.0;
    for j in 0..rh {
        for i in 0..rw {
            let c = px(i, j).0;
            if c[3] > 0.5 {
                for k in 0..3 {
                    avg[k] += c[k];
                }
                n += 1.0;
            }
        }
    }
    if n < 4.0 {
        return;
    }
    avg = avg.map(|v| v / n);
    // Eye pixels: very dark, or white, unlike the face, in the upper part of the head.
    let eye = |i: i32, j: i32| -> bool {
        let c = px(i, j).0;
        if c[3] < 0.5 || j as f32 > rh as f32 * 0.6 {
            return false;
        }
        let d = ((c[0] - avg[0]).powi(2) + (c[1] - avg[1]).powi(2) + (c[2] - avg[2]).powi(2)).sqrt();
        let sat = c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2]);
        let l = luma(c);
        d > 0.25 && (l < 0.2 || (l > 0.78 && sat < 0.15))
    };
    // Eyes come in pairs: keep pixels mirrored across the face.
    let cells: Vec<(i32, i32)> = (0..rh).flat_map(|j| (0..rw).map(move |i| (i, j))).filter(|&(i, j)| eye(i, j) && eye(rw - 1 - i, j)).collect();
    if cells.len() < 2 || cells.len() as f32 > n * 0.2 {
        return;
    }
    for &(i, j) in &cells {
        // Lid colour: the nearest face pixel above the eye.
        let mut lj = j - 1;
        while lj >= 0 && (eye(i, lj) || px(i, lj).0[3] < 0.5) {
            lj -= 1;
        }
        if lj < 0 {
            lj = (j + 1..rh).find(|&b| !eye(i, b) && px(i, b).0[3] >= 0.5).unwrap_or(0);
        }
        let row = px(i, lj).1;
        let t = [(rx + i) as f32 + 0.5, row as f32 + 0.5];
        let uv = [t[0] / SKIN_W as f32, t[1] / SKIN_H as f32];
        // On the layer when the eye pixel is drawn by it, else on the head itself.
        let (corner, fu, fv) = match layer {
            Some(l) if px(i, j).1 >= ry + 64 => l,
            _ => (f.corner, f.u, f.v),
        };
        let su = fu / rw as f32;
        let sv = fv / rh as f32;
        let c = corner + su * i as f32 + sv * j as f32 + f.n * 0.04;
        mb.quad([c, c + su, c + su + sv, c + sv], [uv; 4], f.n, f.shade, EYELID_PART as u32);
    }
}

/// A painting: a canvas one pixel thick, picture on the front, wood everywhere else.
fn painting_mesh(p: &PaintingDef, layer: u32) -> (Vec<EntityVertex>, Vec<u32>) {
    let mut mb = MeshBuilder::new(layer, None);
    let (w, h) = ((p.width * 16) as f32, (p.height * 16) as f32);
    let (iw, ih) = ((p.width * 16) as i32, (p.height * 16) as i32);
    let fs = faces(Vec3::ZERO, Vec3::new(w, h, 1.0), (0, 0), (iw, ih, 1));
    for (k, f) in fs.into_iter().enumerate() {
        // A see-through painting is only its picture, with no wooden board behind it.
        if p.alpha && k != 0 {
            continue;
        }
        let rect = if k == 0 { (0, 0, iw, ih) } else { (0, 64, 16, 16) };
        let shade = if k == 0 { 1.0 } else { f.shade };
        mb.face(&Face { rect, shade, ..f }, 0);
    }
    (mb.verts, mb.indices)
}

/// Armour materials, by the item-name prefix, with their colour.
pub const ARMOR_MATERIALS: [(&str, [u8; 3]); 74] = [
    ("leather", [134, 82, 46]),
    ("chainmail", [150, 150, 156]),
    ("iron", [206, 206, 206]),
    ("golden", [236, 196, 60]),
    ("diamond", [72, 214, 204]),
    ("netherite", [74, 66, 70]),
    ("copper", [206, 116, 66]),
    ("bronze", [184, 132, 62]),
    ("lead", [104, 112, 131]),
    ("silver", [214, 217, 222]),
    ("tungsten", [113, 144, 116]),
    ("platinum", [196, 222, 230]),
    // Hard mode metals (Etapa 5).
    ("cobalt", [52, 98, 196]),
    ("palladium", [222, 120, 70]),
    ("mythril", [86, 170, 140]),
    ("orichalcum", [214, 96, 178]),
    ("adamantite", [196, 40, 60]),
    ("titanium", [150, 156, 170]),
    ("radiant", [240, 214, 110]),
    ("verdite", [96, 206, 70]),
    ("lumite", [120, 236, 220]),
    // The meteor set (gen_tr.py, lote 2).
    ("meteor", [168, 86, 44]),
    // Lote 3: the evil lands' metals and the early sets.
    ("umbral", [90, 58, 132]),
    ("sanguinite", [138, 28, 34]),
    ("hive", [212, 154, 30]),
    ("jungle", [62, 140, 52]),
    ("fossil", [200, 186, 150]),
    ("necro", [176, 172, 156]),
    ("ninja", [36, 36, 46]),
    // Lote 7: the end-game sets.
    ("spectral", [74, 138, 196]),
    ("sporite", [42, 80, 176]),
    ("beetle", [90, 42, 106]),
    ("turtle", [106, 128, 32]),
    ("solar", [224, 90, 16]),
    ("vortex", [24, 136, 122]),
    ("nebula", [122, 42, 160]),
    ("stardust", [42, 128, 196]),
    // The mod pack's dimensions (tools/moddata/aether.py, otherside.py).
    ("zanite", [128, 72, 200]),
    ("gravitite", [200, 74, 180]),
    ("valkyrie", [208, 210, 216]),
    ("phoenix", [232, 100, 26]),
    ("resonarium", [30, 138, 154]),
    // Wizard robes, one set per school (tools/moddata/spellbooks.py).
    ("pyromancer", [176, 52, 22]),
    ("cryomancer", [92, 160, 214]),
    ("electromancer", [214, 184, 50]),
    ("priest", [228, 214, 160]),
    ("voidwalker", [120, 46, 168]),
    ("bloodmage", [130, 18, 30]),
    ("cultist", [18, 110, 116]),
    ("archevoker", [140, 120, 72]),
    // Vanity clothes, worn over the armour (tr_style.rs).
    ("vanity_pirate", [154, 42, 34]),
    ("vanity_wizard", [74, 42, 138]),
    ("vanity_tuxedo", [36, 36, 44]),
    ("vanity_explorer", [128, 100, 52]),
    ("vanity_sailor", [42, 78, 168]),
    ("vanity_ranger", [46, 106, 46]),
    ("vanity_royal", [196, 148, 24]),
    ("vanity_shadow", [42, 34, 56]),
    // Armour dyed in the dye vat (tr_style.rs): it takes the dye's colour.
    ("dye_white", [233, 236, 236]),
    ("dye_orange", [240, 118, 19]),
    ("dye_magenta", [189, 68, 179]),
    ("dye_light_blue", [58, 175, 217]),
    ("dye_yellow", [248, 198, 39]),
    ("dye_lime", [112, 185, 25]),
    ("dye_pink", [237, 141, 172]),
    ("dye_gray", [62, 68, 71]),
    ("dye_light_gray", [142, 142, 134]),
    ("dye_cyan", [21, 137, 145]),
    ("dye_purple", [121, 42, 172]),
    ("dye_blue", [53, 57, 157]),
    ("dye_brown", [114, 71, 40]),
    ("dye_green", [84, 109, 27]),
    ("dye_red", [160, 39, 34]),
    ("dye_black", [20, 21, 25]),
];

/// The boxes of one armour piece (helmet, chestplate, leggings, boots) on the player model:
/// (part, origin, size, texture-net origin, inflate). Every box has its own place in the
/// 64×64 armour texture.
fn armor_boxes(def: &MobDef, piece: usize) -> Vec<(usize, Vec3, Vec3, (u16, u16), f32)> {
    let part = |i: usize| {
        let p = &def.parts[i];
        (Vec3::new(p.origin.0 as f32, p.origin.1 as f32, p.origin.2 as f32), Vec3::new(p.size.0 as f32, p.size.1 as f32, p.size.2 as f32))
    };
    if def.parts.len() < 6 {
        return Vec::new();
    }
    let mut v = Vec::new();
    match piece {
        0 => {
            let (o, sz) = part(0);
            v.push((0, o, sz, (0, 0), 1.0));
        }
        1 => {
            let (o, sz) = part(1);
            v.push((1, o, sz, (32, 0), 1.0));
            for (k, uv) in [(2, (0, 16)), (3, (16, 16))] {
                let (o, sz) = part(k);
                // Sleeves cover the upper arm.
                v.push((k, o + Vec3::Y * 5.0, Vec3::new(sz.x, sz.y - 5.0, sz.z), uv, 1.0));
            }
        }
        2 => {
            let (o, sz) = part(1);
            v.push((1, o, Vec3::new(sz.x, 5.0, sz.z), (0, 32), 0.6));
            for (k, uv) in [(4, (32, 16)), (5, (48, 16))] {
                let (o, sz) = part(k);
                v.push((k, o + Vec3::Y * 3.0, Vec3::new(sz.x, sz.y - 3.0, sz.z), uv, 0.6));
            }
        }
        _ => {
            for (k, uv) in [(4, (24, 32)), (5, (40, 32))] {
                let (o, sz) = part(k);
                v.push((k, o, Vec3::new(sz.x, 4.0, sz.z), uv, 1.0));
            }
        }
    }
    v
}

/// Our own armour picture for one material, drawn in the classic style at `scale` texels per
/// model pixel: a softly speckled metal surface with a thin dark rim and a lit bevel where plates
/// meet, round glints in the top corners, a collar and belly plates on the chest, a belt and knee
/// caps on the leggings, soles and cuffs on the boots and a crest on the helmet. Chainmail is a
/// mesh of round rings.
fn armor_skin(def: &MobDef, color: [u8; 3], chain: bool, matte: bool, scale: u32) -> Skin {
    let sc = scale.max(1);
    let w = SKIN_W * sc;
    let mut pixels = vec![0u8; (w * SKIN_H * sc * 4) as usize];
    let glint = if matte { 0.18 } else { 0.5 };
    let speck = [0.74, 0.84, 0.84, 0.9, 0.9, 0.9, 0.96, 0.96, 0.96, 0.96, 1.0, 1.0, 1.0, 1.04, 1.08, 0.8];
    for piece in 0..4 {
        for (b, (_, _, size, uv, _)) in armor_boxes(def, piece).into_iter().enumerate() {
            let tex = (size.x as i32, size.y as i32, size.z as i32);
            for (fi, f) in faces(Vec3::ZERO, size, uv, tex).iter().enumerate() {
                let (rx, ry, rw, rh) = f.rect;
                let front = fi == 0;
                let top = fi == 4;
                let bottom = fi == 5;
                for j in 0..rh {
                    for i in 0..rw {
                        let (x, y) = (rx + i, ry + j);
                        if x < 0 || y < 0 || x >= 64 || y >= 64 {
                            continue;
                        }
                        let mut k = 1.0f32;
                        let mut white = 0.0f32;
                        let edge = i == 0 || j == 0 || i == rw - 1 || j == rh - 1;
                        let mut a = 255u8;
                        if bottom {
                            k *= 0.7;
                        }
                        let mid = rw / 2;
                        match piece {
                            0 => {
                                // Helmet: open below the brow, with a dark lip over the eyes and a
                                // raised crest along the top.
                                if front && j >= 3 && i >= 1 && i < rw - 1 {
                                    a = 0;
                                }
                                if front && j == 2 && i >= 1 && i < rw - 1 {
                                    k *= 0.74;
                                }
                                if top && (i == mid || i == mid - 1) {
                                    k *= 1.12;
                                    white = glint * 0.4;
                                }
                            }
                            1 if b == 0 => {
                                // Chestplate: a V collar, belly plates and a lit breast.
                                if front && j == 1 && (i - mid).abs() <= 1 {
                                    k *= 0.7;
                                }
                                if (front || fi == 1) && j > 2 && j == rh - 4 && !edge {
                                    k *= 0.78;
                                }
                                if front && j > 1 && j < 5 && (i == 2 || i == rw - 3) {
                                    k *= 1.08;
                                }
                            }
                            1 => {
                                // Shoulder plates: lit on top, a dark lower rim.
                                if j == 1 && !top && !bottom {
                                    k *= 1.12;
                                }
                                if j == rh - 2 && !top && !bottom {
                                    k *= 0.86;
                                }
                            }
                            2 if b == 0 => {
                                // Leggings belt with a buckle in front.
                                if j == 1 && !top && !bottom {
                                    k *= 0.72;
                                    if front && (i == mid || i == mid - 1) {
                                        k = 1.1;
                                        white = glint;
                                    }
                                }
                            }
                            2 => {
                                // Knee caps.
                                let knee = rh / 2;
                                if front && (j == knee || j == knee - 1) && i >= 1 && i < rw - 1 {
                                    k *= if j == knee - 1 { 1.1 } else { 0.8 };
                                }
                            }
                            _ => {
                                // Boots: a lit cuff, a dark sole and a shiny toe.
                                if j == 0 && !top && !bottom {
                                    k = 1.04;
                                }
                                if j == rh - 1 && !top && !bottom {
                                    k *= 0.6;
                                }
                                if front && j == rh - 2 && i >= 1 && i < rw - 1 {
                                    k *= 1.1;
                                }
                            }
                        }
                        // Every model pixel is `sc` × `sc` texels; details are drawn at that finer size.
                        for sj in 0..sc {
                            for si in 0..sc {
                                let fx = i as f32 + (si as f32 + 0.5) / sc as f32;
                                let fy = j as f32 + (sj as f32 + 0.5) / sc as f32;
                                let (tx, ty) = ((x as u32 * sc + si) as i32, (y as u32 * sc + sj) as i32);
                                let h = (((tx * 73 + ty * 151) ^ (tx * ty * 17)) + (tx * 31 ^ ty * 7)) & 15;
                                // Soft speckle: the classic flecks, at a third less contrast.
                                let mut kk = k * (1.0 + (speck[h as usize] - 0.93) * 0.65);
                                let mut wh = white;
                                let mut aa = a;
                                // A thin dark rim around each plate and a lit bevel just inside it.
                                let rim = (fx.min(fy).min(rw as f32 - fx).min(rh as f32 - fy)) * sc as f32;
                                if !top && !bottom {
                                    if rim < 1.0 {
                                        kk *= 0.62;
                                    } else if rim < 2.0 && (fx < 1.0 || fy < 1.0) {
                                        kk *= 1.1;
                                    } else if rim < 2.0 {
                                        kk *= 0.86;
                                    }
                                }
                                // Round glint in the top-left corner of each face.
                                if !bottom {
                                    let d = ((fx - 1.4).powi(2) + (fy - 1.4).powi(2)).sqrt();
                                    wh = wh.max(glint * (1.0 - d / 1.2).clamp(0.0, 1.0));
                                }
                                if !edge && h == 15 && !matte && j < rh / 2 {
                                    wh = wh.max(glint * 0.3);
                                }
                                if chain && !(piece == 0 && front && j < 3) && !edge {
                                    let (cx, cy) = (fx.fract() - 0.5, fy.fract() - 0.5);
                                    let r = (cx * cx + cy * cy).sqrt();
                                    if r < 0.2 || ((i + j) % 2 == 1 && r > 0.46) {
                                        aa = 0;
                                    }
                                    kk *= 1.0 + (0.3 - r) * 0.5 - cy * 0.3;
                                }
                                let o = ((ty as u32 * w + tx as u32) * 4) as usize;
                                for c in 0..3 {
                                    let v = (color[c] as f32 * kk).min(255.0);
                                    pixels[o + c] = (v + (255.0 - v) * wh) as u8;
                                }
                                pixels[o + 3] = aa;
                            }
                        }
                    }
                }
            }
        }
    }
    Skin { pixels, scale: sc }
}

/// An armour trim in one colour: a bright line along the rims of each plate and a chevron down
/// the front, see-through everywhere else. It sits a hair over the armour.
fn trim_skin(def: &MobDef, color: [u8; 3]) -> Skin {
    let mut pixels = vec![0u8; (SKIN_W * SKIN_H * 4) as usize];
    for piece in 0..4 {
        for (_, _, size, uv, _) in armor_boxes(def, piece) {
            let tex = (size.x as i32, size.y as i32, size.z as i32);
            for (fi, f) in faces(Vec3::ZERO, size, uv, tex).iter().enumerate() {
                let (rx, ry, rw, rh) = f.rect;
                if fi >= 4 {
                    continue;
                }
                for j in 0..rh {
                    for i in 0..rw {
                        let (x, y) = (rx + i, ry + j);
                        if x < 0 || y < 0 || x >= 64 || y >= 64 {
                            continue;
                        }
                        let rim = j == rh - 1 || (piece == 0 && j == 2);
                        let mid = rw / 2;
                        let chevron = fi == 0 && rw >= 4 && (j == (i - mid).abs() + 1 || j == (i - mid + 1).abs() + 1) && j < rh - 1;
                        if !(rim || chevron) {
                            continue;
                        }
                        let k = if (x + y) % 3 == 0 { 1.15 } else { 0.92 };
                        let o = ((y * SKIN_W as i32 + x) * 4) as usize;
                        for c in 0..3 {
                            pixels[o + c] = (color[c] as f32 * k).min(255.0) as u8;
                        }
                        pixels[o + 3] = 255;
                    }
                }
            }
        }
    }
    Skin { pixels, scale: 1 }
}

fn armor_mesh(def: &MobDef, piece: usize, layer: u32) -> (Vec<EntityVertex>, Vec<u32>) {
    armor_mesh_over(def, piece, layer, 0.0)
}

fn armor_mesh_over(def: &MobDef, piece: usize, layer: u32, over: f32) -> (Vec<EntityVertex>, Vec<u32>) {
    let mut mb = MeshBuilder::new(layer, Some(def));
    for (part, origin, size, uv, inflate) in armor_boxes(def, piece) {
        let inflate = inflate + over;
        let tex = (size.x as i32, size.y as i32, size.z as i32);
        for f in faces(origin - Vec3::splat(inflate), size + Vec3::splat(2.0 * inflate), uv, tex).iter() {
            // Both sides, so an open sleeve or helmet still looks solid from inside.
            mb.face(f, part as u32);
        }
    }
    (mb.verts, mb.indices)
}

impl EntityRenderer {
    /// `texture_res` is the texture resolution setting (pixels per block face): skins get one
    /// texel per model pixel at 16, up to SKIN_DETAIL at 64 and above.
    pub fn new(r: &Renderer, reg: &MobRegistry, paintings: &[PaintingDef], base: &Path, texture_res: u32) -> EntityRenderer {
        let device = &r.device;
        let scale = (texture_res / 16).clamp(1, SKIN_DETAIL).next_power_of_two();
        let mut skins: Vec<Skin> = reg.defs.iter().map(|d| load_skin(base, &d.name, scale)).collect();
        let back = crate::paintings::back_pixels(base);
        skins.extend(paintings.iter().map(|p| Skin { pixels: crate::paintings::skin_pixels(p, &back), scale: 1 }.rescaled(scale)));
        let player = reg.id("player").map(|i| &reg.defs[i]);
        let armor_layer0 = skins.len();
        if let Some(pd) = player {
            skins.extend(ARMOR_MATERIALS.iter().map(|(n, c)| armor_skin(pd, *c, *n == "chainmail", *n == "leather", scale)));
        }
        let trim_layer0 = skins.len();
        if let Some(pd) = player {
            skins.extend(crate::smithing::MATERIALS.iter().map(|m| trim_skin(pd, m.2).rescaled(scale)));
        }
        let layers = skins.len().max(1) as u32;
        let (tw, th) = (SKIN_W * scale, SKIN_H * scale);
        let levels = scale.trailing_zeros() + 1;
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mob skins"),
            size: wgpu::Extent3d { width: tw, height: th, depth_or_array_layers: layers },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // Mip levels down to one texel per model pixel, so detailed skins don't shimmer far away.
        for (i, s) in skins.iter().enumerate() {
            for (level, data) in crate::textures::mip_chain_rect(&s.pixels, tw, th, levels).iter().enumerate() {
                let (lw, lh) = (tw >> level, th >> level);
                r.queue.write_texture(
                    wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: level as u32, origin: wgpu::Origin3d { x: 0, y: 0, z: i as u32 }, aspect: wgpu::TextureAspect::All },
                    data,
                    wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(lw * 4), rows_per_image: Some(lh) },
                    wgpu::Extent3d { width: lw, height: lh, depth_or_array_layers: 1 },
                );
            }
        }
        let view = tex.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let skin_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mob skins"),
            layout: &r.tex_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&r.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&view) },
            ],
        });
        let inst_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bones"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let inst_cap = 64;
        let inst_buf = Self::make_inst_buf(device, inst_cap);
        let inst_bg = Self::make_inst_bg(device, &inst_layout, &inst_buf);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("entity"),
            source: wgpu::ShaderSource::Wgsl(format!("{}\n{}", include_str!("shaders/common.wgsl"), include_str!("shaders/entity.wgsl")).into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("entity"),
            bind_group_layouts: &[&r.globals_layout, &r.tex_layout, &inst_layout],
            push_constant_ranges: &[],
        });
        let make = |format: wgpu::TextureFormat| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("entity"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_entity"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<EntityVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Uint32, 3 => Uint32],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_entity"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let pipelines = [make(r.config.format), make(crate::post::HDR)];
        let mut meshes = Vec::new();
        for (i, d) in reg.defs.iter().enumerate() {
            let (v, idx) = build_mesh(d, &skins[i], i as u32);
            log::info!("mob '{}': {} vertices ({} with relief layers)", d.name, v.len(), d.layers.len());
            meshes.push(MobMesh {
                vbuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("mob verts"), contents: bytemuck::cast_slice(&v), usage: wgpu::BufferUsages::VERTEX }),
                ibuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("mob idx"), contents: bytemuck::cast_slice(&idx), usage: wgpu::BufferUsages::INDEX }),
                indices: idx.len() as u32,
            });
        }
        let mob_meshes = meshes.len();
        for (i, p) in paintings.iter().enumerate() {
            let (v, idx) = painting_mesh(p, (mob_meshes + i) as u32);
            meshes.push(MobMesh {
                vbuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("painting verts"), contents: bytemuck::cast_slice(&v), usage: wgpu::BufferUsages::VERTEX }),
                ibuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("painting idx"), contents: bytemuck::cast_slice(&idx), usage: wgpu::BufferUsages::INDEX }),
                indices: idx.len() as u32,
            });
        }
        let armor_meshes = player.map(|pd| {
            let first = meshes.len();
            for m in 0..ARMOR_MATERIALS.len() {
                for piece in 0..4 {
                    let (v, idx) = armor_mesh(pd, piece, (armor_layer0 + m) as u32);
                    meshes.push(MobMesh {
                        vbuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("armor verts"), contents: bytemuck::cast_slice(&v), usage: wgpu::BufferUsages::VERTEX }),
                        ibuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("armor idx"), contents: bytemuck::cast_slice(&idx), usage: wgpu::BufferUsages::INDEX }),
                        indices: idx.len() as u32,
                    });
                }
            }
            first
        });
        let trim_meshes = player.map(|pd| {
            let first = meshes.len();
            for m in 0..crate::smithing::MATERIALS.len() {
                for piece in 0..4 {
                    let (v, idx) = armor_mesh_over(pd, piece, (trim_layer0 + m) as u32, 0.12);
                    meshes.push(MobMesh {
                        vbuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("trim verts"), contents: bytemuck::cast_slice(&v), usage: wgpu::BufferUsages::VERTEX }),
                        ibuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("trim idx"), contents: bytemuck::cast_slice(&idx), usage: wgpu::BufferUsages::INDEX }),
                        indices: idx.len() as u32,
                    });
                }
            }
            first
        });
        let particles = crate::particles::ParticleGpu::new(r);
        let items = crate::item_render::ItemGpu::new(r);
        EntityRenderer { particles, items, pipelines, skin_bg, inst_layout, inst_bg, inst_buf, inst_cap, meshes, mob_meshes, armor_meshes, trim_meshes, draws: Vec::new(), instance_count: 0 }
    }

    /// Mesh index of an armour piece (0 helmet .. 3 boots) for the item named `item`.
    pub fn armor_mesh(&self, item: &str, piece: usize) -> Option<usize> {
        let first = self.armor_meshes?;
        let m = ARMOR_MATERIALS.iter().position(|(n, _)| item.starts_with(&format!("{n}_")))?;
        Some(first + m * 4 + piece.min(3))
    }

    /// Mesh index of the trim over an armour piece, by trim material.
    pub fn trim_mesh(&self, material: usize, piece: usize) -> Option<usize> {
        Some(self.trim_meshes? + material * 4 + piece.min(3))
    }

    /// Mesh index of a painting, for `prepare`.
    pub fn painting_mesh(&self, i: usize) -> usize {
        self.mob_meshes + i
    }

    fn make_inst_buf(device: &wgpu::Device, cap: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bones"),
            size: (cap * std::mem::size_of::<Bones>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn make_inst_bg(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, buf: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bones"),
            layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }],
        })
    }

    /// Upload this frame's instances. `items` are (mob definition index, bones), any order.
    pub fn prepare(&mut self, r: &Renderer, mut items: Vec<(usize, Bones)>) {
        items.sort_by_key(|(d, _)| *d);
        self.instance_count = items.len();
        self.draws.clear();
        if items.is_empty() {
            return;
        }
        if items.len() > self.inst_cap {
            self.inst_cap = items.len().next_power_of_two();
            self.inst_buf = Self::make_inst_buf(&r.device, self.inst_cap);
            self.inst_bg = Self::make_inst_bg(&r.device, &self.inst_layout, &self.inst_buf);
        }
        let bones: Vec<Bones> = items.iter().map(|(_, b)| *b).collect();
        r.queue.write_buffer(&self.inst_buf, 0, bytemuck::cast_slice(&bones));
        let mut start = 0usize;
        while start < items.len() {
            let d = items[start].0;
            let mut end = start;
            while end < items.len() && items[end].0 == d {
                end += 1;
            }
            if d < self.meshes.len() {
                self.draws.push((d, start as u32, (end - start) as u32));
            }
            start = end;
        }
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, hdr: bool, block_tex: &'a wgpu::BindGroup) {
        self.particles.draw(pass, hdr, block_tex);
        self.items.draw(pass, hdr, block_tex);
        if self.draws.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipelines[hdr as usize]);
        pass.set_bind_group(1, &self.skin_bg, &[]);
        pass.set_bind_group(2, &self.inst_bg, &[]);
        for &(m, first, count) in &self.draws {
            let mesh = &self.meshes[m];
            pass.set_vertex_buffer(0, mesh.vbuf.slice(..));
            pass.set_index_buffer(mesh.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.indices, 0, first..first + count);
        }
    }
}
