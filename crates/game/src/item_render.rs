//! Draws items lying on the ground and experience orbs as instanced little cubes or sprites.

use wgpu::util::DeviceExt;

use crate::renderer::Renderer;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuItem {
    /// xyz: centre relative to the camera's block; w: size in blocks.
    pub pos: [f32; 4],
    /// yaw, flat (0/1), sky light 0..1, block light 0..1.
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
    layout: wgpu::BindGroupLayout,
    buf: wgpu::Buffer,
    bg: wgpu::BindGroup,
    cap: usize,
    count: u32,
    /// Glows, stored after the `count` items.
    halos: u32,
}

impl ItemGpu {
    pub fn new(r: &Renderer) -> ItemGpu {
        let device = &r.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("items"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("item"),
            source: wgpu::ShaderSource::Wgsl(format!("{}\n{}", include_str!("shaders/common.wgsl"), include_str!("shaders/item.wgsl")).into()),
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
        let make = |format: wgpu::TextureFormat, halo: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if halo { "item glow" } else { "item" }),
                layout: Some(&pl),
                vertex: wgpu::VertexState { module: &shader, entry_point: Some(if halo { "vs_halo" } else { "vs_item" }), buffers: &[], compilation_options: Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if halo { "fs_halo" } else { "fs_item" }),
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
        let pipelines = [make(r.config.format, false), make(crate::post::HDR, false)];
        let halo_pipelines = [make(r.config.format, true), make(crate::post::HDR, true)];
        let cap = 64;
        let buf = Self::make_buf(device, cap);
        let bg = Self::make_bg(device, &layout, &buf);
        ItemGpu { pipelines, halo_pipelines, layout, buf, bg, cap, count: 0, halos: 0 }
    }

    fn make_buf(device: &wgpu::Device, cap: usize) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("items"),
            contents: &vec![0u8; cap * std::mem::size_of::<GpuItem>()],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn make_bg(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, buf: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("items"), layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }] })
    }

    /// `data`: items, then `halos` glows at its end.
    pub fn prepare(&mut self, r: &Renderer, data: &[GpuItem], halos: usize) {
        self.halos = halos.min(data.len()) as u32;
        self.count = data.len() as u32 - self.halos;
        if data.is_empty() {
            return;
        }
        if data.len() > self.cap {
            self.cap = data.len().next_power_of_two();
            self.buf = Self::make_buf(&r.device, self.cap);
            self.bg = Self::make_bg(&r.device, &self.layout, &self.buf);
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
        pass.draw(0..36, 0..self.count);
    }
}
