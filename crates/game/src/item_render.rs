//! Draws items lying on the ground and experience orbs as instanced little cubes or sprites.

use wgpu::util::DeviceExt;

use crate::renderer::Renderer;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuItem {
    /// xyz: centre relative to the camera's block; w: size in blocks.
    pub pos: [f32; 4],
    /// yaw, flat (0 a cube, 1 a flat picture, 2 a picture with real 3D volume), sky light
    /// 0..1, block light 0..1.
    pub info: [f32; 4],
    /// Texture layers side, top, bottom; glow.
    pub layers: [f32; 4],
    /// Pitch and roll (held items); zero for items on the ground. On a 3D picture (info.y 2),
    /// w is how thick it is: 1 (or 0) a tool's full volume, less for flat things like paper.
    pub rot: [f32; 4],
    /// A box of a block instead of a cube (doors swinging, a chest lid): its corners in the
    /// block's own 0..1 frame. `pos` is then the pivot it turns around, `piv` where that
    /// pivot sits in the block, and `piv.w` is 1.
    pub bmin: [f32; 4],
    pub bmax: [f32; 4],
    pub piv: [f32; 4],
}

pub struct ItemGpu {
    pipelines: [wgpu::RenderPipeline; 2],
    /// The soft glow around items on the ground: additive, depth-tested but not written.
    halo_pipelines: [wgpu::RenderPipeline; 2],
    /// Pictures with volume (held items, tools on the ground): `vs_vol`.
    extrude: [wgpu::RenderPipeline; 2],
    /// How thick each texture layer is, cell by cell (`volumes`).
    vols: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    buf: wgpu::Buffer,
    /// The material class of each texture layer (material.rs).
    mats: wgpu::Buffer,
    bg: wgpu::BindGroup,
    cap: usize,
    count: u32,
    /// Glows, stored after the `count` items.
    halos: u32,
    /// The items from here to `count` are extruded (`prepare` puts them last).
    split: u32,
}

/// Cells across a 3D item: its volume is worked out on this grid whatever the texture size.
pub const VOL_GRID: usize = 32;

/// The 3D shape of every texture layer held as an item, from its picture alone (HP 2026-10-29:
/// tools and weapons with real volume, not a two-pixel slab). Each opaque cell of a 32x32 grid
/// gets a half-thickness from how far it is from the picture's edge: a handle three cells wide
/// comes out round, a blade thin at its edges and thick along its ridge, a pick's head thick in
/// the middle and chamfered at its rim. One byte per cell (half-thickness in 1/64 of a cell,
/// 0 = empty), four cells per u32, `VOL_GRID`² cells per layer.
pub fn volumes(t: &crate::textures::TextureSet) -> Vec<u32> {
    const G: usize = VOL_GRID;
    let size = t.size as usize;
    let mut out = vec![0u32; t.layers.len() * G * G / 4];
    let mut d = [0u16; G * G];
    for (li, px) in t.layers.iter().enumerate() {
        // A cell is solid when any texel in it is (thin lines of a fine pack survive).
        let span = (size / G).max(1);
        let mut any = false;
        for cy in 0..G {
            for cx in 0..G {
                let (x0, y0) = (cx * size / G, cy * size / G);
                let mut solid = false;
                'cell: for y in y0..(y0 + span).min(size) {
                    for x in x0..(x0 + span).min(size) {
                        if px.get((y * size + x) * 4 + 3).copied().unwrap_or(0) >= 77 {
                            solid = true;
                            break 'cell;
                        }
                    }
                }
                d[cy * G + cx] = if solid { u16::MAX } else { 0 };
                any |= solid;
            }
        }
        if !any {
            continue;
        }
        // Distance to the nearest empty cell (outside the grid counts as empty), in thirds of a
        // cell: a two-pass 3-4 chamfer transform.
        let at = |d: &[u16; G * G], x: isize, y: isize| -> u16 {
            if x < 0 || y < 0 || x >= G as isize || y >= G as isize {
                0
            } else {
                d[y as usize * G + x as usize]
            }
        };
        for y in 0..G as isize {
            for x in 0..G as isize {
                let i = y as usize * G + x as usize;
                if d[i] == 0 {
                    continue;
                }
                let m = [at(&d, x - 1, y).saturating_add(3), at(&d, x, y - 1).saturating_add(3), at(&d, x - 1, y - 1).saturating_add(4), at(&d, x + 1, y - 1).saturating_add(4)];
                d[i] = d[i].min(*m.iter().min().unwrap());
            }
        }
        for y in (0..G as isize).rev() {
            for x in (0..G as isize).rev() {
                let i = y as usize * G + x as usize;
                if d[i] == 0 {
                    continue;
                }
                let m = [at(&d, x + 1, y).saturating_add(3), at(&d, x, y + 1).saturating_add(3), at(&d, x + 1, y + 1).saturating_add(4), at(&d, x - 1, y + 1).saturating_add(4)];
                d[i] = d[i].min(*m.iter().min().unwrap());
            }
        }
        let base = li * G * G;
        for (i, &v) in d.iter().enumerate() {
            if v == 0 {
                continue;
            }
            let b = (vol_half(v as f32 / 3.0) * 64.0).round().clamp(1.0, 255.0) as u32;
            out[(base + i) / 4] |= b << (8 * ((base + i) % 4));
        }
    }
    out
}

/// Half-thickness (in cells) of a cell `dist` cells from the picture's edge: thin at the rim,
/// rounding up to a cap in the middle.
pub fn vol_half(dist: f32) -> f32 {
    VOL_MAX * (1.0 - (-dist / 1.6).exp())
}

/// The thickest a 3D item gets: 1.75 cells each side of its middle (3.5 of 32 in all).
pub const VOL_MAX: f32 = 1.75;

impl ItemGpu {
    pub fn new(r: &Renderer) -> ItemGpu {
        let device = &r.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("items"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    // The fragment stage of 3D pictures turns rays into the item's frame.
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("item"),
            source: wgpu::ShaderSource::Wgsl(format!("{}\n{}\n{}", include_str!("shaders/common.wgsl"), include_str!("shaders/material.wgsl"), include_str!("shaders/item.wgsl")).into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("item"),
            bind_group_layouts: &[&r.globals_layout, &r.tex_layout, &layout],
            push_constant_ranges: &[],
        });
        // Glows add light on top: colour times alpha, the alpha channel left alone.
        let add = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::SrcAlpha, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        };
        let make = |format: wgpu::TextureFormat, halo: bool, vs: &str, fs: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if halo { "item glow" } else { "item" }),
                layout: Some(&pl),
                vertex: wgpu::VertexState { module: &shader, entry_point: Some(vs), buffers: &[], compilation_options: Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState { format, blend: halo.then_some(add), write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: !halo,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let pipelines = [make(r.config.format, false, "vs_item", "fs_item"), make(crate::post::HDR, false, "vs_item", "fs_item")];
        let halo_pipelines = [make(r.config.format, true, "vs_halo", "fs_halo"), make(crate::post::HDR, true, "vs_halo", "fs_halo")];
        let extrude = [make(r.config.format, false, "vs_vol", "fs_vol"), make(crate::post::HDR, false, "vs_vol", "fs_vol")];
        let cap = 64;
        let buf = Self::make_buf(device, cap);
        let mats = Self::make_mats(device, &[crate::material::NONE]);
        let vols = Self::make_vols(device, &r.item_volumes);
        let bg = Self::make_bg(device, &layout, &buf, &mats, &vols);
        ItemGpu { pipelines, halo_pipelines, extrude, vols, layout, buf, mats, bg, cap, count: 0, halos: 0, split: 0 }
    }

    fn make_buf(device: &wgpu::Device, cap: usize) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("items"),
            contents: &vec![0u8; cap * std::mem::size_of::<GpuItem>()],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn make_mats(device: &wgpu::Device, classes: &[u32]) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("item materials"), contents: bytemuck::cast_slice(classes), usage: wgpu::BufferUsages::STORAGE })
    }

    fn make_vols(device: &wgpu::Device, vols: &[u32]) -> wgpu::Buffer {
        let data: &[u32] = if vols.is_empty() { &[0] } else { vols };
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("item volumes"), contents: bytemuck::cast_slice(data), usage: wgpu::BufferUsages::STORAGE })
    }

    fn make_bg(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, buf: &wgpu::Buffer, mats: &wgpu::Buffer, vols: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("items"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: mats.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: vols.as_entire_binding() },
            ],
        })
    }

    /// The material class of every texture layer, from the texture names in layer order
    /// (material.rs). Set again when the textures are reloaded.
    pub fn set_materials(&mut self, r: &Renderer, names: &[String]) {
        self.mats = Self::make_mats(&r.device, &crate::material::texture_table(names));
        // The textures were (re)loaded with these names: their volumes too.
        self.vols = Self::make_vols(&r.device, &r.item_volumes);
        self.bg = Self::make_bg(&r.device, &self.layout, &self.buf, &self.mats, &self.vols);
    }

    /// `data`: items, then `halos` glows at its end.
    pub fn prepare(&mut self, r: &Renderer, data: &[GpuItem], halos: usize) {
        self.halos = halos.min(data.len()) as u32;
        self.count = data.len() as u32 - self.halos;
        self.split = self.count;
        if data.is_empty() {
            return;
        }
        let extruded = |i: &GpuItem| i.info[1] > 1.5;
        let (solid, glows) = data.split_at(self.count as usize);
        let sorted;
        let data = if solid.iter().any(extruded) {
            sorted = solid
                .iter()
                .filter(|i| !extruded(i))
                .copied()
                .chain(solid.iter().filter(|i| extruded(i)).copied())
                .chain(glows.iter().copied())
                .collect::<Vec<_>>();
            self.split = sorted[..self.count as usize].iter().position(extruded).unwrap_or(self.count as usize) as u32;
            &sorted[..]
        } else {
            data
        };
        if data.len() > self.cap {
            self.cap = data.len().next_power_of_two();
            self.buf = Self::make_buf(&r.device, self.cap);
            self.bg = Self::make_bg(&r.device, &self.layout, &self.buf, &self.mats, &self.vols);
        }
        r.queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(data));
    }

    /// The glows, after everything solid (they don't write depth).
    pub fn draw_halos<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, hdr: bool, block_tex: &'a wgpu::BindGroup) {
        if self.halos == 0 {
            return;
        }
        pass.set_pipeline(&self.halo_pipelines[hdr as usize]);
        pass.set_bind_group(1, block_tex, &[]);
        pass.set_bind_group(2, &self.bg, &[]);
        pass.draw(0..6, self.count..self.count + self.halos);
    }

    /// Needs the block texture bind group in slot 1.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, hdr: bool, block_tex: &'a wgpu::BindGroup) {
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipelines[hdr as usize]);
        pass.set_bind_group(1, block_tex, &[]);
        pass.set_bind_group(2, &self.bg, &[]);
        if self.split > 0 {
            pass.draw(0..36, 0..self.split);
        }
        if self.split < self.count {
            // A box around each picture; the fragment finds its 3D shape inside (fs_vol).
            pass.set_pipeline(&self.extrude[hdr as usize]);
            pass.draw(0..36, self.split..self.count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn half(v: &[u32], layer: usize, x: usize, y: usize) -> u32 {
        let k = layer * VOL_GRID * VOL_GRID + y * VOL_GRID + x;
        (v[k / 4] >> (8 * (k % 4))) & 255
    }

    #[test]
    fn a_handle_comes_out_round_and_a_head_thick_in_the_middle() {
        // A 64x64 picture: a bar 6 texels (3 cells) wide and a block 16 cells wide.
        let size = 64;
        let mut px = vec![0u8; size * size * 4];
        for y in 0..size {
            for x in 0..size {
                let bar = (20..26).contains(&x) && y >= 40;
                let block = (8..40).contains(&x) && (4..36).contains(&y);
                if bar || block {
                    px[(y * size + x) * 4 + 3] = 255;
                }
            }
        }
        let t = crate::textures::TextureSet { size: size as u32, layers: vec![vec![0; size * size * 4], px], packs: vec![], gui: Default::default() };
        let v = volumes(&t);
        // The empty layer stays empty.
        assert!((0..VOL_GRID * VOL_GRID).all(|k| half(&v, 0, k % VOL_GRID, k / VOL_GRID) == 0));
        // Across the bar (cells 10, 11, 12): thicker in the middle than at its sides.
        let (a, b, c) = (half(&v, 1, 10, 26), half(&v, 1, 11, 26), half(&v, 1, 12, 26));
        assert!(a > 0 && c > 0 && b > a && b > c, "{a} {b} {c}");
        // The block's middle is thicker than its rim, and no thicker than the cap.
        let (rim, mid) = (half(&v, 1, 4, 10), half(&v, 1, 12, 10));
        assert!(mid > rim, "{rim} {mid}");
        assert!(mid as f32 <= VOL_MAX * 64.0 + 1.0);
        // Outside the picture: nothing.
        assert_eq!(half(&v, 1, 30, 2), 0);
    }
}
