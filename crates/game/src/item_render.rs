//! Draws items lying on the ground and experience orbs as instanced little cubes or sprites.

use wgpu::util::DeviceExt;

use crate::renderer::Renderer;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuItem {
    /// xyz: centre relative to the camera's block; w: size in blocks.
    pub pos: [f32; 4],
    /// yaw, flat (0 a cube, 1 a flat picture, 2 a picture extruded pixel by pixel), sky light
    /// 0..1, block light 0..1.
    pub info: [f32; 4],
    /// Texture layers side, top, bottom; glow.
    pub layers: [f32; 4],
    /// Pitch and roll (held items); zero for items on the ground.
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
    /// Extruded pictures (held items): `vs_extrude`.
    extrude: [wgpu::RenderPipeline; 2],
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
    /// Item texture width: the planes between pixel columns and rows.
    res: u32,
}

impl ItemGpu {
    pub fn new(r: &Renderer) -> ItemGpu {
        let device = &r.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("items"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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
            bind_group_layouts: &[&r.globals_layout, &r.tex_layout, &layout, &r.fx.layout],
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
        let extrude = [make(r.config.format, false, "vs_extrude", "fs_extrude"), make(crate::post::HDR, false, "vs_extrude", "fs_extrude")];
        let cap = 64;
        let buf = Self::make_buf(device, cap);
        let mats = Self::make_mats(device, &[crate::material::NONE]);
        let bg = Self::make_bg(device, &layout, &buf, &mats);
        ItemGpu { pipelines, halo_pipelines, extrude, layout, buf, mats, bg, cap, count: 0, halos: 0, split: 0, res: r.texture_size }
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

    fn make_bg(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, buf: &wgpu::Buffer, mats: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("items"),
            layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: mats.as_entire_binding() }],
        })
    }

    /// The material class of every texture layer, from the texture names in layer order
    /// (material.rs). Set again when the textures are reloaded.
    pub fn set_materials(&mut self, r: &Renderer, names: &[String]) {
        self.mats = Self::make_mats(&r.device, &crate::material::texture_table(names));
        self.bg = Self::make_bg(&r.device, &self.layout, &self.buf, &self.mats);
    }

    /// `data`: items, then `halos` glows at its end.
    pub fn prepare(&mut self, r: &Renderer, data: &[GpuItem], halos: usize) {
        self.halos = halos.min(data.len()) as u32;
        self.count = data.len() as u32 - self.halos;
        self.split = self.count;
        self.res = r.texture_size.max(1);
        if data.is_empty() {
            return;
        }
        let extruded = |i: &GpuItem| i.info[1] > 1.5;
        let (solid, glows) = data.split_at(self.count as usize);
        let sorted;
        let data = if solid.iter().any(extruded) {
            // The shader's vertex stage can't see the texture: it gets the width in rot.w.
            let res = self.res as f32;
            sorted = solid
                .iter()
                .filter(|i| !extruded(i))
                .copied()
                .chain(solid.iter().filter(|i| extruded(i)).map(|i| GpuItem { rot: [i.rot[0], i.rot[1], i.rot[2], res], ..*i }))
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
            self.bg = Self::make_bg(&r.device, &self.layout, &self.buf, &self.mats);
        }
        r.queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(data));
    }

    /// The glows, after everything solid (they don't write depth).
    pub fn draw_halos<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, hdr: bool, block_tex: &'a wgpu::BindGroup, fx: &'a wgpu::BindGroup) {
        if self.halos == 0 {
            return;
        }
        pass.set_pipeline(&self.halo_pipelines[hdr as usize]);
        pass.set_bind_group(1, block_tex, &[]);
        pass.set_bind_group(2, &self.bg, &[]);
        pass.set_bind_group(3, fx, &[]);
        pass.draw(0..6, self.count..self.count + self.halos);
    }

    /// Needs the block texture bind group in slot 1. `fx`: the shadow map bind group, for
    /// swinging block parts.
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, hdr: bool, block_tex: &'a wgpu::BindGroup, fx: &'a wgpu::BindGroup) {
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipelines[hdr as usize]);
        pass.set_bind_group(1, block_tex, &[]);
        pass.set_bind_group(2, &self.bg, &[]);
        pass.set_bind_group(3, fx, &[]);
        if self.split > 0 {
            pass.draw(0..36, 0..self.split);
        }
        if self.split < self.count {
            // Two faces and a plane between every two columns and every two rows.
            pass.set_pipeline(&self.extrude[hdr as usize]);
            pass.draw(0..6 * (2 + 2 * (self.res + 1)), self.split..self.count);
        }
    }
}
