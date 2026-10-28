//! GPU side: wgpu (Vulkan on Windows/Linux, Metal on macOS, DirectX 12 as a fallback).
//! One vertex buffer per chunk section, a shared quad index buffer, and one small per-draw
//! instance entry holding the section's origin relative to the camera (so far-away worlds keep
//! full precision).

use std::collections::HashMap;
use std::sync::Arc;

use glam::{IVec2, IVec3, Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;
use voxel::chunk::MIN_Y;
use winit::window::Window;

use crate::post::{PostFx, PostUniform, HDR};
use crate::scenefx::{SceneFx, SHADOW_FORMAT};
use crate::settings::ShaderQuality;
use crate::shaderpack::ShaderPack;
use crate::textures::{mip_chain, TextureSet};
use crate::ui::{UiBatch, UiVertex};
use crate::world::MeshEvent;
use voxel::chunk::ChunkPos;
use voxel::mesh::{SectionMesh, Vertex};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Globals {
    pub view_proj: [[f32; 4]; 4],
    pub inv_view_rot: [[f32; 4]; 4],
    pub cam_pos: [f32; 4],
    pub cam_world: [f32; 4],
    pub fog_color: [f32; 4],
    pub fog: [f32; 4],
    pub sun_dir: [f32; 4],
    pub sky_top: [f32; 4],
    pub sky_horizon: [f32; 4],
    /// x: plant waving strength (1 = classic), y: rippling water 0/1, z: light source glow
    /// (HDR only), w: unused.
    pub effects: [f32; 4],
    /// Light carried in the hand: xyz relative to the camera's block, w: level 0..1.
    pub hand_light: [f32; 4],
    /// x: rain 0..1, y: lightning flash 0..1.
    pub weather: [f32; 4],
    /// Moving lights (LambDynamicLights idea): torches lying on the ground, burning mobs.
    /// xyz relative to the camera's block, w: level 0..1 (0 = unused).
    pub dyn_lights: [[f32; 4]; 8],
    /// Position relative to the camera's block → shadow map clip space (set by the renderer).
    pub shadow_mat: [[f32; 4]; 4],
    /// x: shadow strength (0 = off), y: one shadow texel in clip units, z: 1 = soft (4 taps),
    /// w: shadow distance in blocks.
    pub shadow: [f32; 4],
    /// x: cloud style (0 one layer, 1 layered), y: emissive textures 0/1, z: water reflection
    /// steps (0 = off), w: unused.
    pub extra: [f32; 4],
    /// x, y: screen size in pixels; z: near plane; w: far plane (set by the renderer).
    pub screen: [f32; 4],
    /// Block depth (parallax occlusion mapping): x depth in blocks (0 = off), y most steps
    /// per pixel, z distance in blocks where it has faded out, w self-shadows 0/1.
    pub pom: [f32; 4],
    /// Hand light shadows: xyz corner of the solid-block grid relative to the camera's block,
    /// w: 1 when on.
    pub occl: [f32; 4],
    /// xy: how far the wind has carried the clouds, z: classic clouds' drift, w: cloud clock.
    pub clouds: [f32; 4],
    /// Material feel of items and armour (material.rs): x 0 off, 1 glints, 2 also relief and
    /// sparkles.
    pub mat: [f32; 4],
    /// Animated block textures (fire_fx.rs): texture layer + 1 of x fire, y soul fire, z lava
    /// (0 = none, so the default animates nothing).
    pub anim: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

struct GpuSection {
    section: usize,
    /// Face-to-face visibility through the section (see `voxel::mesh::section_visibility`).
    vis: u64,
    solid: Option<(wgpu::Buffer, u32)>,
    translucent: Option<(wgpu::Buffer, u32)>,
}

pub struct FrameInfo {
    pub sections_drawn: usize,
    pub quads_drawn: usize,
    /// Distant terrain tiles and quads drawn.
    pub lod_drawn: usize,
    pub lod_quads: usize,
}

pub struct Camera {
    /// Integer block the eye is in.
    pub block: IVec3,
    /// Eye position inside that block.
    pub frac: Vec3,
    pub forward: Vec3,
    pub fov_y: f32,
    pub far: f32,
}

/// Pipelines for the 3D scene. There are two sets: one draws straight to the screen (shaders
/// off, the cheapest path) and one draws into the HDR texture that shader packs read.
struct ScenePipes {
    solid: wgpu::RenderPipeline,
    translucent: wgpu::RenderPipeline,
    sky: wgpu::RenderPipeline,
    cloud: wgpu::RenderPipeline,
    line: wgpu::RenderPipeline,
    lod: wgpu::RenderPipeline,
    /// Distant nodes clear of the real chunks: no discard, so early depth rejection works.
    lod_far: wgpu::RenderPipeline,
    rain: wgpu::RenderPipeline,
}

pub struct Renderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter_info: wgpu::AdapterInfo,
    depth: wgpu::TextureView,
    globals_buf: wgpu::Buffer,
    globals_bg: wgpu::BindGroup,
    pub globals_layout: wgpu::BindGroupLayout,
    pub tex_layout: wgpu::BindGroupLayout,
    tex_bg: wgpu::BindGroup,
    block_view: wgpu::TextureView,
    /// Texture layers past `LAYERS_PER_ARRAY` (GPUs take at most 2048 per array).
    block_view2: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pipes: [ScenePipes; 2],
    pub post: PostFx,
    ui_pipe: wgpu::RenderPipeline,
    ui_screen_buf: wgpu::Buffer,
    ui_screen_bg: wgpu::BindGroup,
    ui_tex_layout: wgpu::BindGroupLayout,
    ui_tex_bg: wgpu::BindGroup,
    font_view: wgpu::TextureView,
    quad_ibuf: wgpu::Buffer,
    quad_cap: u32,
    inst_buf: wgpu::Buffer,
    inst_cap: usize,
    line_buf: wgpu::Buffer,
    rain_buf: wgpu::Buffer,
    rain_count: u32,
    /// More lines to draw this frame (the fishing line), relative to the camera's block.
    pub extra_lines: Vec<LineVertex>,
    ui_buf: wgpu::Buffer,
    ui_cap: usize,
    chunks: HashMap<ChunkPos, Vec<GpuSection>>,
    pub texture_size: u32,
    pub vram_bytes: u64,
    copy_src_ok: bool,
    /// Skip sections the camera can't see through caves and solid rock.
    pub cave_cull: bool,
    /// Shadows, water reflections, TAA jitter.
    pub fx: SceneFx,
    shadow_pipe: wgpu::RenderPipeline,
    /// The depth texture was recreated since the effects last bound it.
    depth_changed: bool,
}

/// Layers in one texture array (the most GPUs allow); later layers go in a second array. The
/// shaders that read item sprites pick the array with the same number.
pub const LAYERS_PER_ARRAY: usize = 2048;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Near clipping plane (blocks).
const NEAR: f32 = 0.05;

/// Most rain columns drawn (a circle of radius 10 has about 320).
const RAIN_MAX: usize = 512;
/// Line vertices per frame (the selection box takes 24).
const LINE_MAX: usize = 128;

fn shader(device: &wgpu::Device, name: &str, src: &str) -> wgpu::ShaderModule {
    let full = format!("{}\n{}", include_str!("shaders/common.wgsl"), src);
    device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(name), source: wgpu::ShaderSource::Wgsl(full.into()) })
}

impl Renderer {
    pub async fn new(window: Arc<Window>, vsync: bool, textures: &TextureSet, font: &crate::ui::FontAtlas) -> Renderer {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL | wgpu::Backends::DX12,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).expect("create surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No Vulkan/Metal/DX12 graphics adapter found. Update your graphics driver.");
        let adapter_info = adapter.get_info();
        log::info!("GPU: {} ({:?})", adapter_info.name, adapter_info.backend);
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits { max_texture_array_layers: 2048.min(adapter.limits().max_texture_array_layers), ..wgpu::Limits::default() },
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("request device");

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        // Classic look: colours go straight to the screen without sRGB conversion.
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
        let copy_src_ok = caps.usages.contains(wgpu::TextureUsages::COPY_SRC);
        let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        if copy_src_ok {
            usage |= wgpu::TextureUsages::COPY_SRC;
        }
        let config = wgpu::SurfaceConfiguration {
            usage,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync },
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let depth = Self::make_depth(&device, &config);

        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let globals_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals_buf.as_entire_binding() }],
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blocks"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pixel"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            // Blend between mip levels: with 64x textures the switch from one level to the next
            // would otherwise show as a line on the ground.
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let ([block_view, block_view2], texture_size) = Self::upload_blocks(&device, &queue, textures);
        let tex_bg = Self::make_tex_bg(&device, &tex_layout, &block_view, &block_view2, &sampler);

        let terrain = shader(&device, "terrain", concat!(include_str!("shaders/material.wgsl"), "\n", include_str!("shaders/terrain.wgsl")));
        let sky = shader(&device, "sky", include_str!("shaders/sky.wgsl"));
        let lines = shader(&device, "lines", include_str!("shaders/lines.wgsl"));
        let lod = shader(&device, "lod", include_str!("shaders/lod.wgsl"));
        let rain = shader(&device, "rain", include_str!("shaders/rain.wgsl"));
        let rain_layout = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<crate::weather::RainColumn>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
        }];
        let ui = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui.wgsl").into()),
        });

        let fx = SceneFx::new(&device);
        let terrain_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain"),
            bind_group_layouts: &[&globals_layout, &tex_layout, &fx.layout],
            push_constant_ranges: &[],
        });
        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow"),
            bind_group_layouts: &[&globals_layout, &tex_layout],
            push_constant_ranges: &[],
        });
        let globals_only = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("globals only"),
            bind_group_layouts: &[&globals_layout],
            push_constant_ranges: &[],
        });

        let vbufs = [
            wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Uint32x3],
            },
            wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![1 => Float32x3],
            },
        ];
        let blend_alpha = Some(wgpu::BlendState::ALPHA_BLENDING);
        let depth_state = |write: bool, compare: wgpu::CompareFunction| {
            Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: write,
                depth_compare: compare,
                stencil: Default::default(),
                bias: Default::default(),
            })
        };
        let make = |label: &str,
                    format: wgpu::TextureFormat,
                    layout: &wgpu::PipelineLayout,
                    module: &wgpu::ShaderModule,
                    vs: &str,
                    fs: &str,
                    buffers: &[wgpu::VertexBufferLayout],
                    blend: Option<wgpu::BlendState>,
                    cull: Option<wgpu::Face>,
                    depth: Option<wgpu::DepthStencilState>,
                    topology: wgpu::PrimitiveTopology| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState { module, entry_point: Some(vs), buffers, compilation_options: Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { topology, cull_mode: cull, front_face: wgpu::FrontFace::Ccw, ..Default::default() },
                depth_stencil: depth,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        use wgpu::CompareFunction as C;
        use wgpu::PrimitiveTopology as T;
        let line_layout = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
        }];
        let lod_layout = [
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<crate::lod::LodVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Uint32, 1 => Unorm8x4],
            },
            wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![2 => Float32x3],
            },
        ];
        let scene_pipes = |f: wgpu::TextureFormat| ScenePipes {
            solid: make("solid", f, &terrain_layout, &terrain, "vs_main", "fs_solid", &vbufs, None, Some(wgpu::Face::Back), depth_state(true, C::Less), T::TriangleList),
            translucent: make(
                "translucent",
                f,
                &terrain_layout,
                &terrain,
                "vs_main",
                "fs_translucent",
                &vbufs,
                blend_alpha,
                None,
                depth_state(false, C::Less),
                T::TriangleList,
            ),
            sky: make("sky", f, &globals_only, &sky, "vs_sky", "fs_sky", &[], None, None, depth_state(false, C::Always), T::TriangleList),
            cloud: make("clouds", f, &globals_only, &sky, "vs_sky", "fs_clouds", &[], blend_alpha, None, depth_state(false, C::Less), T::TriangleList),
            line: make("lines", f, &globals_only, &lines, "vs_lines", "fs_lines", &line_layout, blend_alpha, None, depth_state(false, C::LessEqual), T::LineList),
            lod: make("lod", f, &globals_only, &lod, "vs_lod", "fs_lod", &lod_layout, None, None, depth_state(true, C::Less), T::TriangleList),
            lod_far: make("lod far", f, &globals_only, &lod, "vs_lod", "fs_lod_far", &lod_layout, None, None, depth_state(true, C::Less), T::TriangleList),
            rain: make("rain", f, &globals_only, &rain, "vs_rain", "fs_rain", &rain_layout, blend_alpha, None, depth_state(false, C::Less), T::TriangleList),
        };
        let pipes = [scene_pipes(format), scene_pipes(HDR)];
        // Depth only, seen from the sun; a slope bias keeps lit faces from shadowing themselves.
        let shadow_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow map"),
            layout: Some(&shadow_layout),
            vertex: wgpu::VertexState { module: &terrain, entry_point: Some("vs_shadow"), buffers: &vbufs, compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState { module: &terrain, entry_point: Some("fs_shadow"), targets: &[], compilation_options: Default::default() }),
            primitive: wgpu::PrimitiveState { topology: T::TriangleList, cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: SHADOW_FORMAT,
                depth_write_enabled: true,
                depth_compare: C::Less,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let post = PostFx::new(&device, format);

        // UI
        let ui_screen_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui screen"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ui_screen_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui screen"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: ui_screen_buf.as_entire_binding() }],
        });
        let ui_tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui tex"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let font_tex = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("font"),
                size: wgpu::Extent3d { width: font.width, height: font.height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &font.pixels,
        );
        let font_view = font_tex.create_view(&Default::default());
        let gui_view = Self::upload_gui(&device, &queue, &textures.gui);
        let ui_tex_bg = Self::make_ui_bg(&device, &ui_tex_layout, &font_view, &block_view, &block_view2, &gui_view, &sampler);
        let ui_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui"),
            bind_group_layouts: &[&globals_layout, &ui_tex_layout],
            push_constant_ranges: &[],
        });
        let ui_vlayout = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<UiVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4, 3 => Sint32],
        }];
        // The HUD gets its own pass after the scene (and after any shader pack), without depth.
        let ui_pipe = make("ui", format, &ui_layout, &ui, "vs_ui", "fs_ui", &ui_vlayout, blend_alpha, None, None, T::TriangleList);

        let quad_cap = 1 << 16;
        let quad_ibuf = Self::make_quad_ibuf(&device, quad_cap);
        let inst_cap = 4096;
        let inst_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (inst_cap * 12) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rain_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rain"),
            size: RAIN_MAX as u64 * std::mem::size_of::<crate::weather::RainColumn>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let line_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lines"),
            size: LINE_MAX as u64 * std::mem::size_of::<LineVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ui_cap = 1 << 14;
        let ui_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui"),
            size: (ui_cap * std::mem::size_of::<UiVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Renderer {
            surface,
            device,
            queue,
            config,
            adapter_info,
            depth,
            globals_buf,
            globals_bg,
            globals_layout,
            tex_layout,
            tex_bg,
            block_view,
            block_view2,
            sampler,
            pipes,
            post,
            ui_pipe,
            ui_screen_buf,
            ui_screen_bg,
            ui_tex_layout,
            ui_tex_bg,
            font_view,
            quad_ibuf,
            quad_cap,
            inst_buf,
            inst_cap,
            line_buf,
            rain_buf,
            rain_count: 0,
            extra_lines: Vec::new(),
            ui_buf,
            ui_cap,
            chunks: HashMap::new(),
            texture_size,
            vram_bytes: 0,
            copy_src_ok,
            cave_cull: true,
            fx,
            shadow_pipe,
            depth_changed: true,
        }
    }

    fn make_depth(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH,
                // Shader packs read depth (haze, sun rays).
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }

    /// What the shader pack needs to know about this frame (see post_common.wgsl).
    fn post_uniform(&self, view_proj: &Mat4, g: &Globals, cam: &Camera) -> PostUniform {
        let (w, h) = (self.config.width as f32, self.config.height as f32);
        let sun = Vec3::new(g.sun_dir[0], g.sun_dir[1], g.sun_dir[2]);
        // A direction has w = 0, so the camera position drops out of the projection.
        let clip = *view_proj * sun.extend(0.0);
        let underwater = g.fog[2] > 0.5;
        let (mut sx, mut sy, mut vis) = (0.5, -1.0, 0.0);
        if clip.w > 1e-4 {
            sx = clip.x / clip.w * 0.5 + 0.5;
            sy = 0.5 - clip.y / clip.w * 0.5;
            // Fade in as the sun clears the horizon, and out as it leaves the screen.
            let above = ((sun.y + 0.02) / 0.08).clamp(0.0, 1.0);
            let edge = |v: f32| (1.0 - ((v - 0.5).abs() - 0.5).max(0.0) / 0.25).clamp(0.0, 1.0);
            vis = above * edge(sx) * edge(sy) * if underwater { 0.0 } else { 1.0 };
        }
        let sunset = (1.0 - sun.y.abs() * 3.5).clamp(0.0, 1.0) * if sun.y > -0.15 { 1.0 } else { 0.0 };
        PostUniform {
            resolution: [w, h, 1.0 / w, 1.0 / h],
            sun: [sx, sy, vis, g.fog_color[3]],
            sun_dir: [sun.x, sun.y, sun.z, sunset],
            fog_color: [g.fog_color[0], g.fog_color[1], g.fog_color[2], g.fog[2]],
            params: [g.cam_pos[3], NEAR, cam.far, 0.0],
            options: [[0.0; 4]; 2],
            ..Default::default()
        }
    }

    fn make_quad_ibuf(device: &wgpu::Device, quads: u32) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad indices"),
            contents: bytemuck::cast_slice(&voxel::mesh::quad_indices(quads as usize)),
            usage: wgpu::BufferUsages::INDEX,
        })
    }

    pub fn make_tex_bg(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, view: &wgpu::TextureView, view2: &wgpu::TextureView, sampler: &wgpu::Sampler) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blocks"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(view2) },
            ],
        })
    }

    fn make_ui_bg(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        font: &wgpu::TextureView,
        blocks: &wgpu::TextureView,
        blocks2: &wgpu::TextureView,
        gui: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui tex"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(font) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(blocks) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(gui) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(blocks2) },
            ],
        })
    }

    /// The packs' interface art (a single transparent pixel when there is none).
    fn upload_gui(device: &wgpu::Device, queue: &wgpu::Queue, g: &crate::textures::GuiAtlas) -> wgpu::TextureView {
        let (w, h, px) = if g.pixels.is_empty() { (1, 1, vec![0u8; 4]) } else { (g.width, g.height, g.pixels.clone()) };
        let tex = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("gui"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &px,
        );
        tex.create_view(&Default::default())
    }

    /// The texture layers go into two arrays: the first `LAYERS_PER_ARRAY` (every block texture
    /// is among them, so the terrain only ever reads the first) and the rest (item sprites, cracks).
    fn upload_blocks(device: &wgpu::Device, queue: &wgpu::Queue, t: &TextureSet) -> ([wgpu::TextureView; 2], u32) {
        let levels = t.size.trailing_zeros() + 1;
        let split = t.layers.len().min(LAYERS_PER_ARRAY);
        let make = |layers: &[Vec<u8>], label: &str| {
            let n = layers.len().max(1) as u32;
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: t.size, height: t.size, depth_or_array_layers: n },
                mip_level_count: levels,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            for (layer, pixels) in layers.iter().enumerate() {
                for (level, data) in mip_chain(pixels, t.size, levels).iter().enumerate() {
                    let s = (t.size >> level).max(1);
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &tex,
                            mip_level: level as u32,
                            origin: wgpu::Origin3d { x: 0, y: 0, z: layer as u32 },
                            aspect: wgpu::TextureAspect::All,
                        },
                        data,
                        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * s), rows_per_image: Some(s) },
                        wgpu::Extent3d { width: s, height: s, depth_or_array_layers: 1 },
                    );
                }
            }
            tex.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() })
        };
        let first = make(&t.layers[..split], "block textures");
        let second = make(&t.layers[split..], "more textures");
        ([first, second], t.size)
    }

    /// Swap in new block textures (resource pack change) without restarting.
    pub fn reload_textures(&mut self, t: &TextureSet) {
        let ([view, view2], size) = Self::upload_blocks(&self.device, &self.queue, t);
        self.tex_bg = Self::make_tex_bg(&self.device, &self.tex_layout, &view, &view2, &self.sampler);
        let gui = Self::upload_gui(&self.device, &self.queue, &t.gui);
        self.ui_tex_bg = Self::make_ui_bg(&self.device, &self.ui_tex_layout, &self.font_view, &view, &view2, &gui, &self.sampler);
        self.block_view = view;
        self.block_view2 = view2;
        self.texture_size = size;
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.depth = Self::make_depth(&self.device, &self.config);
        self.depth_changed = true;
        self.post.resize(&self.device, w, h, &self.depth);
    }

    /// Turn a shader pack on (or off with None). On a compile error the pack stays off and the
    /// error message is returned.
    pub fn set_shader(&mut self, pack: Option<&ShaderPack>, quality: ShaderQuality) -> Result<(), String> {
        self.post.quality = quality;
        let r = self.post.set_pack(&self.device, pack);
        self.post.resize(&self.device, self.config.width, self.config.height, &self.depth);
        r
    }

    pub fn set_vsync(&mut self, vsync: bool) {
        self.config.present_mode = if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        self.surface.configure(&self.device, &self.config);
    }

    pub fn apply_mesh_events(&mut self, events: Vec<MeshEvent>) {
        for e in events {
            match e {
                MeshEvent::Remove(p) => {
                    if let Some(old) = self.chunks.remove(&p) {
                        self.vram_bytes -= old.iter().map(section_bytes).sum::<u64>();
                    }
                }
                MeshEvent::Set(p, meshes) => {
                    let v: Vec<GpuSection> = meshes.into_iter().map(|(s, m)| self.upload_section(s, m)).collect();
                    self.vram_bytes += v.iter().map(section_bytes).sum::<u64>();
                    if let Some(old) = self.chunks.insert(p, v) {
                        self.vram_bytes -= old.iter().map(section_bytes).sum::<u64>();
                    }
                }
                MeshEvent::Sections(p, meshes) => {
                    if !self.chunks.contains_key(&p) {
                        continue;
                    }
                    for (s, m) in meshes {
                        let new = m.map(|m| self.upload_section(s, m));
                        let v = self.chunks.get_mut(&p).unwrap();
                        if let Some(i) = v.iter().position(|g| g.section == s) {
                            self.vram_bytes -= section_bytes(&v.remove(i));
                        }
                        if let Some(g) = new {
                            self.vram_bytes += section_bytes(&g);
                            v.push(g);
                        }
                    }
                }
            }
        }
    }

    fn upload_section(&mut self, section: usize, m: SectionMesh) -> GpuSection {
        let mut mk = |verts: &[Vertex], label: &str| -> Option<(wgpu::Buffer, u32)> {
            if verts.is_empty() {
                return None;
            }
            let quads = (verts.len() / 4) as u32;
            if quads > self.quad_cap {
                self.quad_cap = quads.next_power_of_two();
                self.quad_ibuf = Self::make_quad_ibuf(&self.device, self.quad_cap);
            }
            let b = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            Some((b, quads))
        };
        let solid = mk(&m.solid, "section solid");
        let translucent = mk(&m.translucent, "section translucent");
        GpuSection { section, vis: m.vis, solid, translucent }
    }

    /// Cave culling: walk from the camera's section outwards, entering a neighbour only through
    /// faces the current section connects with open blocks and never turning back. Sections
    /// never reached are behind rock. Returns a grid of `reach` chunks around the camera.
    fn cave_visible(&self, cam: &Camera, reach: i32, visible: &dyn Fn(Vec3, Vec3) -> bool) -> Option<Vec<bool>> {
        const NS: i32 = voxel::chunk::SECTIONS as i32;
        let csy = (cam.block.y - MIN_Y) >> 4;
        if !self.cave_cull || !(0..NS).contains(&csy) {
            return None;
        }
        let (ccx, ccz) = (cam.block.x >> 4, cam.block.z >> 4);
        let n = 2 * reach + 1;
        let index = |dx: i32, sy: i32, dz: i32| (((dx + reach) * n + dz + reach) * NS + sy) as usize;
        let mut seen = vec![false; (n * n * NS) as usize];
        let vis_of = |dx: i32, sy: i32, dz: i32| -> u64 {
            match self.chunks.get(&IVec2::new(ccx + dx, ccz + dz)) {
                Some(v) => v.iter().find(|s| s.section == sy as usize).map_or(voxel::mesh::VIS_ALL, |s| s.vis),
                None => voxel::mesh::VIS_ALL,
            }
        };
        const STEP: [(i32, i32, i32); 6] = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)];
        let mut queue = std::collections::VecDeque::new();
        seen[index(0, csy, 0)] = true;
        // (chunk dx, section y, chunk dz, face entered through or 6, directions taken)
        queue.push_back((0, csy, 0, 6u8, 0u8));
        while let Some((x, y, z, from, dirs)) = queue.pop_front() {
            let vis = vis_of(x, y, z);
            for (f, &(sx, sy, sz)) in STEP.iter().enumerate() {
                if dirs & (1 << (f ^ 1)) != 0 {
                    continue;
                }
                if from != 6 && vis & (1 << (from as usize * 6 + f)) == 0 {
                    continue;
                }
                let (nx, ny, nz) = (x + sx, y + sy, z + sz);
                if nx.abs() > reach || nz.abs() > reach || !(0..NS).contains(&ny) || seen[index(nx, ny, nz)] {
                    continue;
                }
                let o = Vec3::new(((ccx + nx) * 16 - cam.block.x) as f32, (ny * 16 + MIN_Y - cam.block.y) as f32, ((ccz + nz) * 16 - cam.block.z) as f32);
                if !visible(o, o + Vec3::splat(16.0)) {
                    continue;
                }
                seen[index(nx, ny, nz)] = true;
                queue.push_back((nx, ny, nz, (f ^ 1) as u8, dirs | (1 << f)));
            }
        }
        Some(seen)
    }

    /// Forget every chunk (another world is being opened).
    pub fn clear_chunks(&mut self) {
        self.chunks.clear();
        self.vram_bytes = 0;
    }

    /// Rain and snow columns for the next frames (empty when it's dry).
    pub fn set_rain(&mut self, cols: &[crate::weather::RainColumn]) {
        let n = cols.len().min(RAIN_MAX);
        if n > 0 {
            self.queue.write_buffer(&self.rain_buf, 0, bytemuck::cast_slice(&cols[..n]));
        }
        self.rain_count = n as u32;
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Draw a frame. Returns RGBA pixels as well when `capture` is set.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        cam: &Camera,
        globals: &mut Globals,
        selection: Option<IVec3>,
        clouds: bool,
        entities: Option<&crate::entity::EntityRenderer>,
        lod: Option<&crate::lod::Lod>,
        ui: &UiBatch,
        capture: bool,
    ) -> (FrameInfo, Option<(u32, u32, Vec<u8>)>) {
        let aspect = self.config.width as f32 / self.config.height as f32;
        let (sw, sh) = (self.config.width, self.config.height);
        let proj = Mat4::perspective_rh(cam.fov_y, aspect, NEAR, cam.far);
        let view = Mat4::look_to_rh(cam.frac, cam.forward, Vec3::Y);
        let plain_vp = proj * view;
        // Shader-pack-only effects: TAA jitters the projection a little every frame.
        let hdr_on = self.post.scene_view().is_some();
        let taa = hdr_on && self.post.taa;
        let view_proj = self.fx.jitter(sw, sh, taa) * plain_vp;
        let reproj = if taa { Some(self.fx.reprojection(view_proj, plain_vp, cam.block)) } else { None };
        // Reflections copy the picture before water is drawn: the shader pack's HDR texture, or
        // the screen itself when the display allows copying from it.
        let scene_format = if hdr_on { Some(crate::post::HDR) } else { self.copy_src_ok.then_some(self.config.format) };
        self.fx.prepare(&self.device, scene_format, sw, sh, &self.depth, self.depth_changed);
        self.depth_changed = false;
        // Sun shadows where there is a sun (the Overworld and the Aether).
        let sun = Vec3::new(globals.sun_dir[0], globals.sun_dir[1], globals.sun_dir[2]);
        let smat = if globals.weather[2] <= 0.0 { self.fx.shadow_matrix(sun, cam.block, cam.frac) } else { None };
        (globals.shadow_mat, globals.shadow) = self.fx.shadow_globals(smat);
        globals.extra[2] = if self.fx.reflecting() { if self.fx.fancy { 40.0 } else { 20.0 } } else { 0.0 };
        globals.screen = [sw as f32, sh as f32, NEAR, cam.far];
        let view_rot = Mat4::look_to_rh(Vec3::ZERO, cam.forward, Vec3::Y);
        globals.view_proj = view_proj.to_cols_array_2d();
        globals.inv_view_rot = (proj * view_rot).inverse().to_cols_array_2d();
        self.queue.write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(globals));
        self.queue.write_buffer(
            &self.ui_screen_buf,
            0,
            bytemuck::cast_slice(&[self.config.width as f32, self.config.height as f32, 0.0, 0.0]),
        );

        // Frustum planes (row combinations of the view-projection matrix).
        let m = view_proj.transpose();
        let (r0, r1, r2, r3) = (m.x_axis, m.y_axis, m.z_axis, m.w_axis);
        let planes = [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2];
        let visible = |min: Vec3, max: Vec3| -> bool {
            planes.iter().all(|p: &Vec4| {
                let v = Vec3::new(if p.x >= 0.0 { max.x } else { min.x }, if p.y >= 0.0 { max.y } else { min.y }, if p.z >= 0.0 { max.z } else { min.z });
                p.x * v.x + p.y * v.y + p.z * v.z + p.w >= 0.0
            })
        };

        let mut instances: Vec<[f32; 3]> = Vec::new();
        let mut solid_draws: Vec<(usize, &wgpu::Buffer, u32)> = Vec::new();
        let mut trans_draws: Vec<(f32, usize, &wgpu::Buffer, u32)> = Vec::new();
        let max_d = cam.far + 24.0;
        let reach = (max_d / 16.0).ceil() as i32 + 1;
        let seen = self.cave_visible(cam, reach, &visible);
        for (pos, sections) in &self.chunks {
            let cx = (pos.x * 16 - cam.block.x) as f32;
            let cz = (pos.y * 16 - cam.block.z) as f32;
            let dx = (cx + 8.0 - cam.frac.x).abs().max(0.0) - 8.0;
            let dz = (cz + 8.0 - cam.frac.z).abs().max(0.0) - 8.0;
            if dx.max(0.0).hypot(dz.max(0.0)) > max_d {
                continue;
            }
            for s in sections {
                let origin = Vec3::new(cx, (s.section as i32 * 16 + MIN_Y - cam.block.y) as f32, cz);
                if !visible(origin, origin + Vec3::splat(16.0)) {
                    continue;
                }
                if let Some(seen) = &seen {
                    let (dx, dz) = (pos.x - (cam.block.x >> 4), pos.y - (cam.block.z >> 4));
                    if dx.abs() <= reach && dz.abs() <= reach && !seen[(((dx + reach) * (2 * reach + 1) + dz + reach) * voxel::chunk::SECTIONS as i32) as usize + s.section] {
                        continue;
                    }
                }
                let idx = instances.len();
                instances.push(origin.to_array());
                if let Some((b, q)) = &s.solid {
                    solid_draws.push((idx, b, *q));
                }
                if let Some((b, q)) = &s.translucent {
                    let d = (origin + Vec3::splat(8.0) - cam.frac).length_squared();
                    trans_draws.push((d, idx, b, *q));
                }
            }
        }
        // Distant terrain nodes share the instance buffer (their corner relative to the camera).
        // Nodes reaching into the real chunks (and the band where they sink) need the discard.
        let mut lod_draws: Vec<(usize, &wgpu::Buffer, u32, bool)> = Vec::new();
        let lod_inner = lod.map(|l| l.inner).unwrap_or(0.0) + 64.0;
        let eye2 = glam::Vec2::new(cam.frac.x, cam.frac.z);
        for tile in lod.map(|l| l.tiles.values()).into_iter().flatten() {
            let s = tile.node.size() as f32;
            let o = tile.node.origin();
            let origin = Vec3::new((o.x - cam.block.x) as f32, -cam.block.y as f32, (o.y - cam.block.z) as f32);
            let lo = origin + Vec3::new(0.0, tile.min_y - 8.0, 0.0);
            let hi = origin + Vec3::new(s, tile.max_y + 1.0, s);
            if !visible(lo, hi) {
                continue;
            }
            if tile.quads > self.quad_cap {
                self.quad_cap = tile.quads.next_power_of_two();
                self.quad_ibuf = Self::make_quad_ibuf(&self.device, self.quad_cap);
            }
            let near = eye2.clamp(glam::Vec2::new(lo.x, lo.z), glam::Vec2::new(hi.x, hi.z)).distance(eye2) < lod_inner;
            lod_draws.push((instances.len(), &tile.buf, tile.quads, near));
            instances.push(origin.to_array());
        }
        // Shadow casters: every section with solid blocks around the camera, seen or not.
        let mut shadow_draws: Vec<(usize, &wgpu::Buffer, u32)> = Vec::new();
        if smat.is_some() {
            let reach = self.fx.shadow_radius() + 16.0;
            for (pos, sections) in &self.chunks {
                let cx = (pos.x * 16 - cam.block.x) as f32;
                let cz = (pos.y * 16 - cam.block.z) as f32;
                let dx = (cx + 8.0 - cam.frac.x).abs() - 8.0;
                let dz = (cz + 8.0 - cam.frac.z).abs() - 8.0;
                if dx.max(0.0).hypot(dz.max(0.0)) > reach {
                    continue;
                }
                for s in sections {
                    let y = (s.section as i32 * 16 + MIN_Y - cam.block.y) as f32;
                    // Far below the camera nothing it casts can reach the lit area.
                    if y + 16.0 < -reach * 1.5 {
                        continue;
                    }
                    if let Some((b, q)) = &s.solid {
                        shadow_draws.push((instances.len(), b, *q));
                        instances.push([cx, y, cz]);
                    }
                }
            }
        }
        lod_draws.sort_by(|a, b| Vec3::from(instances[a.0]).length_squared().total_cmp(&Vec3::from(instances[b.0]).length_squared()));
        trans_draws.sort_by(|a, b| b.0.total_cmp(&a.0));
        // Near-to-far for opaque geometry so the depth test rejects hidden pixels early.
        solid_draws.sort_by(|a, b| {
            let da = Vec3::from(instances[a.0]).length_squared();
            let db = Vec3::from(instances[b.0]).length_squared();
            da.total_cmp(&db)
        });

        if instances.len() > self.inst_cap {
            self.inst_cap = instances.len().next_power_of_two();
            self.inst_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instances"),
                size: (self.inst_cap * 12) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !instances.is_empty() {
            self.queue.write_buffer(&self.inst_buf, 0, bytemuck::cast_slice(&instances));
        }

        // Selection box.
        let mut line_verts: Vec<LineVertex> = Vec::new();
        if let Some(b) = selection {
            let o = (b - cam.block).as_vec3() - Vec3::splat(0.002);
            let s = 1.004;
            let c = [0.0, 0.0, 0.0, 0.55];
            let corners: Vec<Vec3> = (0..8).map(|i| o + Vec3::new((i & 1) as f32 * s, ((i >> 1) & 1) as f32 * s, ((i >> 2) & 1) as f32 * s)).collect();
            let edges = [(0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)];
            line_verts.extend(edges.iter().flat_map(|&(a, b)| [LineVertex { pos: corners[a].to_array(), color: c }, LineVertex { pos: corners[b].to_array(), color: c }]));
        }
        line_verts.extend(self.extra_lines.iter().copied());
        line_verts.truncate(LINE_MAX);
        if !line_verts.is_empty() {
            self.queue.write_buffer(&self.line_buf, 0, bytemuck::cast_slice(&line_verts));
        }
        let line_count = line_verts.len() as u32;

        // UI vertices.
        if ui.verts.len() > self.ui_cap {
            self.ui_cap = ui.verts.len().next_power_of_two();
            self.ui_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui"),
                size: (self.ui_cap * std::mem::size_of::<UiVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !ui.verts.is_empty() {
            self.queue.write_buffer(&self.ui_buf, 0, bytemuck::cast_slice(&ui.verts));
        }

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return (FrameInfo { sections_drawn: 0, quads_drawn: 0, lod_drawn: 0, lod_quads: 0 }, None);
            }
            Err(e) => {
                log::warn!("frame skipped: {e:?}");
                return (FrameInfo { sections_drawn: 0, quads_drawn: 0, lod_drawn: 0, lod_quads: 0 }, None);
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        let fc = globals.fog_color;
        let mut quads = 0usize;
        // With a shader pack the scene goes to the HDR texture, otherwise straight to the screen.
        let hdr_view = self.post.scene_view();
        let hdr = hdr_view.is_some();
        let pipes = &self.pipes[hdr as usize];
        if let (Some(_), Some(sv)) = (smat, self.fx.shadow_view()) {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow map"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: sv,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.shadow_pipe);
            pass.set_bind_group(0, &self.globals_bg, &[]);
            pass.set_bind_group(1, &self.tex_bg, &[]);
            pass.set_index_buffer(self.quad_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.set_vertex_buffer(1, self.inst_buf.slice(..));
            for (i, b, q) in &shadow_draws {
                pass.set_vertex_buffer(0, b.slice(..));
                pass.draw_indexed(0..q * 6, 0, *i as u32..*i as u32 + 1);
            }
            // The player and mobs cast shadows too.
            if let Some(e) = entities {
                e.draw_shadow(&mut pass);
            }
        }
        // With water reflections the pass is split: everything opaque first, then a copy of
        // the picture for the water to reflect, then water, glass, rain and lines on top.
        let split = self.fx.reflecting();
        let late = LateDraws {
            pipes,
            tex_bg: &self.tex_bg,
            quad_ibuf: &self.quad_ibuf,
            inst_buf: &self.inst_buf,
            trans: &trans_draws,
            rain: (self.rain_count > 0).then_some((&self.rain_buf, self.rain_count)),
            lines: (line_count > 0).then_some((&self.line_buf, line_count)),
        };
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: hdr_view.unwrap_or(&view),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: fc[0] as f64, g: fc[1] as f64, b: fc[2] as f64, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.globals_bg, &[]);
            pass.set_pipeline(&pipes.sky);
            pass.draw(0..3, 0..1);

            pass.set_pipeline(&pipes.solid);
            pass.set_bind_group(1, &self.tex_bg, &[]);
            pass.set_bind_group(2, &self.fx.bg_solid, &[]);
            pass.set_index_buffer(self.quad_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.set_vertex_buffer(1, self.inst_buf.slice(..));
            for (i, b, q) in &solid_draws {
                pass.set_vertex_buffer(0, b.slice(..));
                pass.draw_indexed(0..q * 6, 0, *i as u32..*i as u32 + 1);
                quads += *q as usize;
            }
            let mut lod_pipe = None;
            for (i, b, q, near) in &lod_draws {
                if lod_pipe != Some(*near) {
                    lod_pipe = Some(*near);
                    pass.set_pipeline(if *near { &pipes.lod } else { &pipes.lod_far });
                }
                pass.set_vertex_buffer(0, b.slice(..));
                pass.draw_indexed(0..q * 6, 0, *i as u32..*i as u32 + 1);
            }
            if let Some(e) = entities {
                e.draw(&mut pass, hdr, &self.tex_bg, &self.fx.bg_solid);
            }
            if clouds {
                pass.set_pipeline(&pipes.cloud);
                pass.draw(0..3, 0..1);
            }
            if !split {
                quads += late.draw(&mut pass, &self.fx.bg_solid);
            }
        }
        if split {
            if let (Some(src), Some(dst)) = (self.post.scene_texture().or(Some(&frame.texture)), self.fx.copy_texture()) {
                enc.copy_texture_to_texture(src.as_image_copy(), dst.as_image_copy(), wgpu::Extent3d { width: sw, height: sh, depth_or_array_layers: 1 });
            }
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("water and glass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: hdr_view.unwrap_or(&view),
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                // Read-only depth: the water shader reads it for its reflections.
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &self.depth, depth_ops: None, stencil_ops: None }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.globals_bg, &[]);
            quads += late.draw(&mut pass, &self.fx.bg_trans);
        }
        if hdr {
            let mut u = self.post_uniform(&view_proj, globals, cam);
            if let Some((m, valid)) = reproj {
                u.reproj = m.to_cols_array_2d();
                u.taa = [1.0, if valid { 1.0 } else { 0.0 }, 0.0, 0.0];
            }
            self.post.run(&self.queue, &mut enc, &view, u);
        }
        if !ui.verts.is_empty() {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.ui_pipe);
            pass.set_bind_group(0, &self.ui_screen_bg, &[]);
            pass.set_bind_group(1, &self.ui_tex_bg, &[]);
            pass.set_vertex_buffer(0, self.ui_buf.slice(..));
            pass.draw(0..ui.verts.len() as u32, 0..1);
        }

        let mut readback = None;
        if capture && self.copy_src_ok {
            let (w, h) = (self.config.width, self.config.height);
            let bpr = (w * 4).div_ceil(256) * 256;
            let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("screenshot"),
                size: (bpr * h) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            enc.copy_texture_to_buffer(
                frame.texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: Some(h) } },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            readback = Some((buf, bpr, w, h));
        } else if capture {
            log::warn!("this display does not allow screenshots (no COPY_SRC on the surface)");
        }
        self.queue.submit([enc.finish()]);
        let shot = readback.and_then(|(buf, bpr, w, h)| {
            let slice = buf.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            self.device.poll(wgpu::Maintain::Wait);
            rx.recv().ok()?.ok()?;
            let data = slice.get_mapped_range();
            let bgra = matches!(self.config.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
            let mut out = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                let row = &data[(y * bpr) as usize..(y * bpr + w * 4) as usize];
                for px in row.chunks_exact(4) {
                    if bgra {
                        out.extend_from_slice(&[px[2], px[1], px[0], 255]);
                    } else {
                        out.extend_from_slice(&[px[0], px[1], px[2], 255]);
                    }
                }
            }
            Some((w, h, out))
        });
        frame.present();
        let lod_quads = lod_draws.iter().map(|d| d.2 as usize).sum();
        (FrameInfo { sections_drawn: solid_draws.len() + trans_draws.len(), quads_drawn: quads, lod_drawn: lod_draws.len(), lod_quads }, shot)
    }
}

/// What is drawn after the opaque scene: water and glass (far to near), rain, lines.
struct LateDraws<'a> {
    pipes: &'a ScenePipes,
    tex_bg: &'a wgpu::BindGroup,
    quad_ibuf: &'a wgpu::Buffer,
    inst_buf: &'a wgpu::Buffer,
    trans: &'a [(f32, usize, &'a wgpu::Buffer, u32)],
    rain: Option<(&'a wgpu::Buffer, u32)>,
    lines: Option<(&'a wgpu::Buffer, u32)>,
}

impl LateDraws<'_> {
    /// Returns the quads drawn.
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, fx_bg: &wgpu::BindGroup) -> usize {
        let mut quads = 0;
        pass.set_pipeline(&self.pipes.translucent);
        pass.set_bind_group(1, self.tex_bg, &[]);
        pass.set_bind_group(2, fx_bg, &[]);
        // Mobs and paintings bind their own index buffers; water needs the shared quad one back.
        pass.set_index_buffer(self.quad_ibuf.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(1, self.inst_buf.slice(..));
        for (_, i, b, q) in self.trans {
            pass.set_vertex_buffer(0, b.slice(..));
            pass.draw_indexed(0..q * 6, 0, *i as u32..*i as u32 + 1);
            quads += *q as usize;
        }
        if let Some((buf, n)) = self.rain {
            pass.set_pipeline(&self.pipes.rain);
            pass.set_vertex_buffer(0, buf.slice(..));
            pass.draw(0..6, 0..n);
        }
        if let Some((buf, n)) = self.lines {
            pass.set_pipeline(&self.pipes.line);
            pass.set_vertex_buffer(0, buf.slice(..));
            pass.draw(0..n, 0..1);
        }
        quads
    }
}

fn section_bytes(s: &GpuSection) -> u64 {
    s.solid.as_ref().map(|b| b.0.size()).unwrap_or(0) + s.translucent.as_ref().map(|b| b.0.size()).unwrap_or(0)
}

#[allow(dead_code)]
pub fn chunk_of(p: IVec3) -> IVec2 {
    IVec2::new(p.x >> 4, p.z >> 4)
}
