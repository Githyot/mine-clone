//! Mob rendering: box models from `mobs.ron` turned into meshes once, drawn instanced with one
//! matrix per body part. Skin "layers" are voxelized at load time, so wool, fur and torn clothes
//! stand off the body as real relief instead of a flat painting (the 3D Skin Layers idea).

use std::path::Path;

use glam::{IVec3, Mat4, Quat, Vec3};
use wgpu::util::DeviceExt;

use crate::embedded;
use crate::paintings::PaintingDef;
use crate::renderer::Renderer;
use voxel::mob::{Anim, Mob, MobDef, MobRegistry, DEATH_TIME};

pub const SKIN_W: u32 = 64;
pub const SKIN_H: u32 = 128;
const MAX_PARTS: usize = 16;

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
}

impl Default for Bones {
    fn default() -> Self {
        Bones { m: [Mat4::IDENTITY.to_cols_array_2d(); MAX_PARTS], light: [1.0, 0.0, 0.0, 0.0] }
    }
}

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
    /// Meshes after this index are paintings.
    mob_meshes: usize,
    /// (mesh, first instance, count) for this frame.
    draws: Vec<(usize, u32, u32)>,
    pub instance_count: usize,
}

struct Skin {
    pixels: Vec<u8>, // RGBA, SKIN_W × SKIN_H
}

impl Skin {
    fn alpha(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= SKIN_W as i32 || y >= SKIN_H as i32 {
            return 0;
        }
        self.pixels[((y as u32 * SKIN_W + x as u32) * 4 + 3) as usize]
    }
}

fn load_skin(base: &Path, name: &str) -> Skin {
    let bytes = std::fs::read(base.join(format!("packs/vanilla/textures/entity/{name}.png")))
        .ok()
        .or_else(|| embedded::ENTITY_TEXTURES.iter().find(|(n, _)| *n == name).map(|(_, b)| b.to_vec()));
    let mut pixels = vec![0u8; (SKIN_W * SKIN_H * 4) as usize];
    match bytes.and_then(|b| image::load_from_memory(&b).ok()) {
        Some(img) => {
            let img = img.to_rgba8();
            for y in 0..img.height().min(SKIN_H) {
                for x in 0..img.width().min(SKIN_W) {
                    let p = img.get_pixel(x, y).0;
                    let i = ((y * SKIN_W + x) * 4) as usize;
                    pixels[i..i + 4].copy_from_slice(&p);
                }
            }
        }
        None => {
            log::warn!("missing mob skin '{name}'");
            for y in 0..64 {
                for x in 0..64 {
                    let i = ((y * SKIN_W + x) * 4) as usize;
                    let m = ((x / 8) + (y / 8)) % 2 == 0;
                    pixels[i..i + 4].copy_from_slice(if m { &[248, 0, 248, 255] } else { &[0, 0, 0, 255] });
                }
            }
        }
    }
    Skin { pixels }
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
}

impl MeshBuilder {
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
                // Outward face.
                self.quad([c, c + su, c + su + sv, c + sv], uv4, f.n, f.shade, part);
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
    let mut mb = MeshBuilder { verts: Vec::new(), indices: Vec::new(), layer };
    for (pi, p) in def.parts.iter().enumerate() {
        let origin = Vec3::new(p.origin.0 as f32, p.origin.1 as f32, p.origin.2 as f32);
        let size = Vec3::new(p.size.0 as f32, p.size.1 as f32, p.size.2 as f32);
        let tex = (p.size.0 as i32, p.size.1 as i32, p.size.2 as i32);
        for f in faces(origin, size, p.uv, tex).iter() {
            mb.face(f, pi as u32);
        }
    }
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

/// A painting: a canvas one pixel thick, picture on the front, wood everywhere else.
fn painting_mesh(p: &PaintingDef, layer: u32) -> (Vec<EntityVertex>, Vec<u32>) {
    let mut mb = MeshBuilder { verts: Vec::new(), indices: Vec::new(), layer };
    let (w, h) = ((p.width * 16) as f32, (p.height * 16) as f32);
    let (iw, ih) = ((p.width * 16) as i32, (p.height * 16) as i32);
    let fs = faces(Vec3::ZERO, Vec3::new(w, h, 1.0), (0, 0), (iw, ih, 1));
    for (k, f) in fs.into_iter().enumerate() {
        let rect = if k == 0 { (0, 0, iw, ih) } else { (0, 64, 16, 16) };
        let shade = if k == 0 { 1.0 } else { f.shade };
        mb.face(&Face { rect, shade, ..f }, 0);
    }
    (mb.verts, mb.indices)
}

impl EntityRenderer {
    pub fn new(r: &Renderer, reg: &MobRegistry, paintings: &[PaintingDef], base: &Path) -> EntityRenderer {
        let device = &r.device;
        let mut skins: Vec<Skin> = reg.defs.iter().map(|d| load_skin(base, &d.name)).collect();
        let back = crate::paintings::back_pixels(base);
        skins.extend(paintings.iter().map(|p| Skin { pixels: crate::paintings::skin_pixels(p, &back) }));
        let layers = skins.len().max(1) as u32;
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mob skins"),
            size: wgpu::Extent3d { width: SKIN_W, height: SKIN_H, depth_or_array_layers: layers },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (i, s) in skins.iter().enumerate() {
            r.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d { x: 0, y: 0, z: i as u32 }, aspect: wgpu::TextureAspect::All },
                &s.pixels,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(SKIN_W * 4), rows_per_image: Some(SKIN_H) },
                wgpu::Extent3d { width: SKIN_W, height: SKIN_H, depth_or_array_layers: 1 },
            );
        }
        let view = tex.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let skin_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mob skins"),
            layout: &r.tex_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&r.sampler) },
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
        EntityRenderer { pipelines, skin_bg, inst_layout, inst_bg, inst_buf, inst_cap, meshes, mob_meshes, draws: Vec::new(), instance_count: 0 }
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

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, hdr: bool) {
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

/// Part matrices for one mob at this moment.
pub fn bones_for(mob: &Mob, def: &MobDef, cam_block: IVec3, sky: f32, blk: f32) -> Bones {
    let mut b = Bones { light: [sky, blk, if mob.hurt_age < 0.3 { 1.0 } else { 0.0 }, 0.0], ..Default::default() };
    let swell = mob.fuse();
    if swell > 0.0 {
        b.light[3] = swell * 0.5 * ((mob.age * 18.0).sin() * 0.5 + 0.5);
    }
    let rel = mob.body.pos - cam_block.as_vec3();
    let mut model = Mat4::from_translation(rel) * Mat4::from_rotation_y(mob.yaw);
    if !mob.alive() {
        // Fall over sideways and sink a little.
        let t = (mob.dead_age / DEATH_TIME).clamp(0.0, 1.0);
        model = model * Mat4::from_translation(Vec3::new(0.0, -t * 0.4, 0.0)) * Mat4::from_rotation_z(t * std::f32::consts::FRAC_PI_2);
    }
    let scale = 1.0 / 16.0 * (1.0 + swell * 0.3);
    model = model * Mat4::from_scale(Vec3::splat(scale));
    let amp = (mob.walk_speed / def.speed.max(0.1)).min(1.3);
    let t = mob.walk;
    let airborne = !mob.body.on_ground && !mob.body.in_water;
    for (i, p) in def.parts.iter().enumerate().take(MAX_PARTS) {
        let pivot = Vec3::new(p.pivot.0 as f32, p.pivot.1 as f32, p.pivot.2 as f32);
        let rot = match p.anim {
            Anim::None => Quat::IDENTITY,
            Anim::Head => Quat::from_rotation_y(mob.head_yaw) * Quat::from_rotation_x(mob.head_pitch),
            Anim::Leg(k) => Quat::from_rotation_x((t + k as f32 * std::f32::consts::PI).sin() * 0.8 * amp),
            Anim::Arm(k) => Quat::from_rotation_x((t + k as f32 * std::f32::consts::PI + std::f32::consts::PI).sin() * 0.7 * amp),
            Anim::ArmForward(k) => Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2 + (mob.age * 2.0 + k as f32).sin() * 0.08),
            Anim::Wing(k) => {
                let side = if k == 0 { 1.0 } else { -1.0 };
                let flap = if airborne { (mob.age * 25.0).sin().abs() * 0.9 } else { 0.0 };
                Quat::from_rotation_z(flap * side)
            }
            Anim::SpiderLeg(k) => {
                let side = if k < 4 { 1.0 } else { -1.0 };
                let kk = (k % 4) as f32;
                let fan = (kk - 1.5) * 0.4 * side;
                let swing = (t * 2.0 + kk * 1.3 + if k < 4 { 0.0 } else { std::f32::consts::PI }).sin() * 0.35 * amp;
                let lift = ((t * 2.0 + kk * 1.3).cos().max(0.0)) * 0.3 * amp;
                Quat::from_rotation_y(fan + swing) * Quat::from_rotation_z((-0.35 - lift) * side)
            }
        };
        let part = Mat4::from_translation(pivot) * Mat4::from_quat(rot) * Mat4::from_translation(-pivot);
        b.m[i] = (model * part).to_cols_array_2d();
    }
    b
}
