//! Post-processing on the GPU. With a shader pack on, the 3D scene is drawn into an HDR texture
//! (`HDR`) and one or more full-screen passes turn it into the picture on screen; the HUD is drawn
//! afterwards, untouched. With shaders off none of this runs and nothing extra is allocated.
//!
//! Built-in packs use a bloom chain (bright pass at half size, down-sampled a few times, then
//! added back up), a low-resolution sun ray pass, and a composite pass. User packs get a single
//! composite pass running their own `fs_post` (see `shaderpack.rs` and `post_common.wgsl`).

use crate::settings::ShaderQuality;
use crate::shaderpack::{Builtin, PackSource, ShaderPack, HEADER_WGSL, MAX_OPTIONS};

/// Colour format of the off-screen scene and the bloom levels.
pub const HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Must match `struct Post` in post_common.wgsl.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct PostUniform {
    pub resolution: [f32; 4],
    pub sun: [f32; 4],
    pub sun_dir: [f32; 4],
    pub fog_color: [f32; 4],
    pub params: [f32; 4],
    pub options: [[f32; 4]; 2],
}

struct Active {
    builtin: Option<Builtin>,
    composite: wgpu::RenderPipeline,
    /// Bloom levels (0 = no bloom); level 0 is half the screen size.
    bloom_levels: usize,
    /// Sun rays at 1/`rays_div` of the screen size (0 = no rays).
    rays_div: u32,
}

/// Size-dependent textures and the bind groups that read them.
struct Targets {
    scene: wgpu::TextureView,
    mips: Vec<(wgpu::TextureView, u32, u32)>,
    rays: Option<(wgpu::TextureView, u32, u32)>,
    bg0: wgpu::BindGroup,
    /// down[k] reads level k-1 (k >= 1); up[k] reads level k+1.
    down: Vec<Option<wgpu::BindGroup>>,
    up: Vec<Option<wgpu::BindGroup>>,
    composite: wgpu::BindGroup,
}

pub struct PostFx {
    layout0: wgpu::BindGroupLayout,
    layout1: wgpu::BindGroupLayout,
    builtin_layout: wgpu::PipelineLayout,
    user_layout: wgpu::PipelineLayout,
    builtin_module: wgpu::ShaderModule,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    bright: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    rays: wgpu::RenderPipeline,
    dummy_bg1: wgpu::BindGroup,
    dummy: wgpu::TextureView,
    surface_format: wgpu::TextureFormat,
    active: Option<Active>,
    targets: Option<Targets>,
    pub options: [f32; MAX_OPTIONS],
    pub quality: ShaderQuality,
}

fn tex_entry(binding: u32, sample_type: wgpu::TextureSampleType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture { sample_type, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
        count: None,
    }
}

fn pipeline(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    fs: &str,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState { module, entry_point: Some("vs_post"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(fs),
            targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None,
        cache: None,
    })
}

fn color_target(device: &wgpu::Device, label: &str, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HDR,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

/// Turn a wgpu validation error into one line for the HUD, with the line number in the user's
/// own file (the header in front of it shifts every line).
fn short_error(e: &str) -> String {
    let header_lines = format!("{HEADER_WGSL}\n").lines().count();
    let mut msg = e
        .lines()
        .map(str::trim)
        .find(|l| l.contains("error") && !l.starts_with("Validation Error"))
        .or_else(|| e.lines().map(str::trim).rev().find(|l| !l.is_empty()))
        .unwrap_or("erro desconhecido")
        .to_string();
    if let Some(at) = e.find("wgsl:") {
        let digits: String = e[at + 5..].chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(line) = digits.parse::<usize>() {
            if line > header_lines {
                msg = format!("linha {}: {msg}", line - header_lines);
            }
        }
    }
    msg.chars().take(160).collect()
}

impl PostFx {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> PostFx {
        let float = wgpu::TextureSampleType::Float { filterable: true };
        let layout0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post scene"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                tex_entry(1, float),
                tex_entry(2, wgpu::TextureSampleType::Depth),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post inputs"),
            entries: &[tex_entry(0, float), tex_entry(1, float)],
        });
        let builtin_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post builtin"),
            bind_group_layouts: &[&layout0, &layout1],
            push_constant_ranges: &[],
        });
        let user_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("post user"), bind_group_layouts: &[&layout0], push_constant_ranges: &[] });
        let builtin_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(format!("{}\n{}", HEADER_WGSL, include_str!("shaders/post.wgsl")).into()),
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post uniform"),
            size: std::mem::size_of::<PostUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let add = Some(wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent::REPLACE,
        });
        let bright = pipeline(device, "bloom bright", &builtin_layout, &builtin_module, "fs_bright", HDR, None);
        let down = pipeline(device, "bloom down", &builtin_layout, &builtin_module, "fs_down", HDR, None);
        let up = pipeline(device, "bloom up", &builtin_layout, &builtin_module, "fs_up", HDR, add);
        let rays = pipeline(device, "sun rays", &builtin_layout, &builtin_module, "fs_rays", HDR, None);
        let dummy = color_target(device, "post dummy", 1, 1);
        let dummy_bg1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post dummy"),
            layout: &layout1,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&dummy) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&dummy) },
            ],
        });
        PostFx {
            layout0,
            layout1,
            builtin_layout,
            user_layout,
            builtin_module,
            uniform,
            sampler,
            bright,
            down,
            up,
            rays,
            dummy_bg1,
            dummy,
            surface_format,
            active: None,
            targets: None,
            options: [0.0; MAX_OPTIONS],
            quality: ShaderQuality::Fancy,
        }
    }

    pub fn builtin(&self) -> Option<Builtin> {
        self.active.as_ref().and_then(|a| a.builtin)
    }

    /// Switch to a pack (None = off). On a compile error the shader is turned off and the
    /// message comes back for the HUD.
    pub fn set_pack(&mut self, device: &wgpu::Device, pack: Option<&ShaderPack>) -> Result<(), String> {
        self.active = None;
        self.targets = None;
        let Some(pack) = pack else { return Ok(()) };
        let fancy = self.quality == ShaderQuality::Fancy;
        let fmt = self.surface_format;
        match &pack.source {
            PackSource::Builtin(b) => {
                let (fs, bloom_levels, rays_div) = match b {
                    Builtin::Realcado => ("fs_realcado", if fancy { 5 } else { 3 }, if fancy { 2 } else { 4 }),
                    Builtin::Suave => ("fs_suave", 3, 0),
                };
                let composite = pipeline(device, fs, &self.builtin_layout, &self.builtin_module, fs, fmt, None);
                self.active = Some(Active { builtin: Some(*b), composite, bloom_levels, rays_div });
                Ok(())
            }
            PackSource::User(dir) => {
                let user = std::fs::read_to_string(dir.join("post.wgsl")).map_err(|e| format!("post.wgsl: {e}"))?;
                let src = format!("{HEADER_WGSL}\n{user}");
                device.push_error_scope(wgpu::ErrorFilter::Validation);
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(&pack.id), source: wgpu::ShaderSource::Wgsl(src.into()) });
                let composite = pipeline(device, &pack.id, &self.user_layout, &module, "fs_post", fmt, None);
                if let Some(e) = pollster::block_on(device.pop_error_scope()) {
                    let full = e.to_string();
                    log::error!("shader pack '{}' failed to compile:\n{full}", pack.id);
                    return Err(short_error(&full));
                }
                self.active = Some(Active { builtin: None, composite, bloom_levels: 0, rays_div: 0 });
                Ok(())
            }
        }
    }

    /// (Re)create the screen-sized textures. Call after `set_pack` and on every resize.
    pub fn resize(&mut self, device: &wgpu::Device, w: u32, h: u32, depth: &wgpu::TextureView) {
        let Some(a) = &self.active else {
            self.targets = None;
            return;
        };
        let scene = color_target(device, "scene hdr", w, h);
        let mips: Vec<(wgpu::TextureView, u32, u32)> = (1..=a.bloom_levels as u32)
            .map(|k| {
                let (mw, mh) = ((w >> k).max(1), (h >> k).max(1));
                (color_target(device, "bloom level", mw, mh), mw, mh)
            })
            .collect();
        let rays = (a.rays_div > 0).then(|| {
            let (rw, rh) = ((w / a.rays_div).max(1), (h / a.rays_div).max(1));
            (color_target(device, "sun rays", rw, rh), rw, rh)
        });
        let bg0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post scene"),
            layout: &self.layout0,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&scene) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(depth) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        let bg1 = |a: &wgpu::TextureView, b: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("post inputs"),
                layout: &self.layout1,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(a) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(b) },
                ],
            })
        };
        let n = mips.len();
        let down = (0..n).map(|k| (k >= 1).then(|| bg1(&mips[k - 1].0, &self.dummy))).collect();
        let up = (0..n).map(|k| (k + 1 < n).then(|| bg1(&mips[k + 1].0, &self.dummy))).collect();
        let composite = bg1(mips.first().map(|m| &m.0).unwrap_or(&self.dummy), rays.as_ref().map(|r| &r.0).unwrap_or(&self.dummy));
        self.targets = Some(Targets { scene, mips, rays, bg0, down, up, composite });
    }

    /// Where the 3D scene should be drawn while a pack is on.
    pub fn scene_view(&self) -> Option<&wgpu::TextureView> {
        self.active.as_ref().and(self.targets.as_ref()).map(|t| &t.scene)
    }

    /// Run the passes, ending with the composite into `out` (the swapchain image).
    pub fn run(&self, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, out: &wgpu::TextureView, mut u: PostUniform) {
        let (Some(a), Some(t)) = (&self.active, &self.targets) else { return };
        u.options = [[self.options[0], self.options[1], self.options[2], self.options[3]], [self.options[4], self.options[5], self.options[6], self.options[7]]];
        u.params[3] = if self.quality == ShaderQuality::Fancy { 1.0 } else { 0.0 };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&u));

        let pass = |enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, load: bool, pipe: &wgpu::RenderPipeline, bg1: &wgpu::BindGroup| {
            let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: if load { wgpu::LoadOp::Load } else { wgpu::LoadOp::Clear(wgpu::Color::BLACK) }, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            p.set_pipeline(pipe);
            p.set_bind_group(0, &t.bg0, &[]);
            p.set_bind_group(1, bg1, &[]);
            p.draw(0..3, 0..1);
        };
        if !t.mips.is_empty() {
            pass(enc, &t.mips[0].0, false, &self.bright, &self.dummy_bg1);
            for k in 1..t.mips.len() {
                pass(enc, &t.mips[k].0, false, &self.down, t.down[k].as_ref().unwrap());
            }
            for k in (0..t.mips.len() - 1).rev() {
                pass(enc, &t.mips[k].0, true, &self.up, t.up[k].as_ref().unwrap());
            }
        }
        if let Some((rays, ..)) = &t.rays {
            pass(enc, rays, false, &self.rays, &self.dummy_bg1);
        }
        let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("post composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: out,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        p.set_pipeline(&a.composite);
        p.set_bind_group(0, &t.bg0, &[]);
        if a.builtin.is_some() {
            p.set_bind_group(1, &t.composite, &[]);
        }
        p.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_point_at_the_users_line() {
        let header = format!("{HEADER_WGSL}\n").lines().count();
        let e = format!("Validation Error\n\nCaused by:\n    Shader 'x' parsing error: expected ';'\n   ┌─ wgsl:{}:5\n", header + 3);
        let s = short_error(&e);
        assert!(s.starts_with("linha 3:"), "{s}");
        assert!(s.contains("expected"), "{s}");
    }
}
