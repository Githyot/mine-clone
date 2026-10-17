//! Particles: bits of broken blocks, water drops from splashes and bubbles. Simulated on the CPU
//! (there are only ever a few hundred) and drawn as tiny textured cubes.

use glam::{IVec3, Vec3};
use voxel::physics::Terrain;
use wgpu::util::DeviceExt;

use crate::renderer::Renderer;
use crate::world::World;

const MAX: usize = 1500;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuParticle {
    pos: [f32; 4],
    uv: [f32; 4],
    tint: [f32; 4],
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    /// Falls and bounces to a stop.
    Debris,
    /// Falls, then vanishes when it lands or touches water.
    Drop,
    /// Rises under water, pops at the surface.
    Bubble,
    /// Magic: drifts without falling and shines at full brightness.
    Glow,
    /// Ambient bits (Subtle Effects idea): a leaf, ash or dust drifting slowly down, swaying.
    Mote,
}

struct Particle {
    pos: Vec3,
    vel: Vec3,
    life: f32,
    size: f32,
    layer: u32,
    uv: [f32; 2],
    tint: [f32; 3],
    kind: Kind,
}

pub struct Particles {
    list: Vec<Particle>,
    seed: u32,
}

impl Particles {
    pub fn new() -> Particles {
        Particles { list: Vec::new(), seed: 0x9e3779b9 }
    }

    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32
    }

    fn push(&mut self, p: Particle) {
        if self.list.len() >= MAX {
            self.list.swap_remove(0);
        }
        self.list.push(p);
    }

    /// Bits flying out of a block. `count` around 16 for a break, 2–3 for a mining knock.
    pub fn block_bits(&mut self, block: IVec3, layer: u16, tint: [f32; 3], count: usize, burst: f32) {
        for _ in 0..count {
            let off = Vec3::new(self.rand(), self.rand(), self.rand());
            let pos = block.as_vec3() + Vec3::splat(0.15) + off * 0.7;
            let dir = (off - Vec3::splat(0.5)) * 2.0;
            let vel = Vec3::new(dir.x * burst, 1.5 + self.rand() * 2.5 * burst, dir.z * burst);
            let uv = [(self.rand() * 12.0).floor() / 16.0, (self.rand() * 12.0).floor() / 16.0];
            let p = Particle { pos, vel, life: 0.6 + self.rand() * 0.9, size: 0.05 + self.rand() * 0.05, layer: layer as u32, uv, tint, kind: Kind::Debris };
            self.push(p);
        }
    }

    /// Drops thrown up around a splash.
    pub fn splash(&mut self, at: Vec3, layer: u16, strength: f32) {
        let n = (10.0 + strength * 30.0) as usize;
        for _ in 0..n {
            let a = self.rand() * std::f32::consts::TAU;
            let r = 0.3 + self.rand() * 0.9;
            let up = 3.0 + self.rand() * 5.0 * strength;
            let p = Particle {
                pos: at + Vec3::new(a.cos() * r * 0.5, 0.05, a.sin() * r * 0.5),
                vel: Vec3::new(a.cos() * r * 2.0, up, a.sin() * r * 2.0),
                life: 1.5,
                size: 0.05 + self.rand() * 0.05,
                layer: layer as u32,
                uv: [0.4, 0.4],
                tint: [0.75, 0.85, 1.0],
                kind: Kind::Drop,
            };
            self.push(p);
        }
    }

    /// A raindrop hitting the ground: a few tiny droplets bouncing up (Particle Rain idea).
    pub fn rain_drop(&mut self, at: Vec3, layer: u16) {
        for _ in 0..3 {
            let a = self.rand() * std::f32::consts::TAU;
            let r = 0.4 + self.rand() * 0.8;
            let p = Particle {
                pos: at + Vec3::new(0.0, 0.05, 0.0),
                vel: Vec3::new(a.cos() * r, 1.5 + self.rand() * 1.5, a.sin() * r),
                life: 0.35,
                size: 0.03 + self.rand() * 0.03,
                layer: layer as u32,
                uv: [0.4, 0.4],
                tint: [0.75, 0.85, 1.0],
                kind: Kind::Drop,
            };
            self.push(p);
        }
    }

    /// A glowing mote (spell trails and bursts).
    pub fn glow(&mut self, at: Vec3, vel: Vec3, layer: u16, tint: [f32; 3], size: f32, life: f32) {
        let jitter = Vec3::new(self.rand() - 0.5, self.rand() - 0.5, self.rand() - 0.5);
        let p = Particle { pos: at + jitter * size, vel: vel + jitter * 0.6, life: life * (0.7 + self.rand() * 0.6), size, layer: layer as u32, uv: [0.4, 0.4], tint, kind: Kind::Glow };
        self.push(p);
    }

    /// A slow drifting bit lit like the world around it: a falling leaf, ash, dust.
    pub fn mote(&mut self, at: Vec3, layer: u16, tint: [f32; 3], size: f32, fall: f32, life: f32) {
        let a = self.rand() * std::f32::consts::TAU;
        let uv = [(self.rand() * 12.0).floor() / 16.0, (self.rand() * 12.0).floor() / 16.0];
        let p = Particle { pos: at, vel: Vec3::new(a.cos() * 0.2, -fall, a.sin() * 0.2), life: life * (0.7 + self.rand() * 0.6), size, layer: layer as u32, uv, tint, kind: Kind::Mote };
        self.push(p);
    }

    pub fn bubbles(&mut self, at: Vec3, layer: u16, count: usize) {
        for _ in 0..count {
            let off = Vec3::new(self.rand() - 0.5, self.rand() * 0.3, self.rand() - 0.5) * 0.6;
            let p = Particle {
                pos: at + off,
                vel: Vec3::new(0.0, 1.2 + self.rand(), 0.0),
                life: 2.5,
                size: 0.04 + self.rand() * 0.04,
                layer: layer as u32,
                uv: [0.3, 0.3],
                tint: [0.85, 0.95, 1.2],
                kind: Kind::Bubble,
            };
            self.push(p);
        }
    }

    pub fn update(&mut self, world: &World, dt: f32) {
        for p in self.list.iter_mut() {
            p.life -= dt;
            match p.kind {
                Kind::Glow => {
                    p.vel *= 1.0 - dt * 2.5;
                    p.pos += p.vel * dt;
                }
                Kind::Mote => {
                    // Sway side to side as it sinks; gone once it touches the ground.
                    let t = p.life * 2.3 + p.pos.y;
                    p.pos += (p.vel + Vec3::new(t.sin() * 0.35, 0.0, t.cos() * 0.25)) * dt;
                    if world.solid(p.pos.floor().as_ivec3()) || world.liquid(p.pos.floor().as_ivec3()) {
                        p.life = p.life.min(0.2);
                        p.vel = Vec3::ZERO;
                    }
                }
                Kind::Bubble => {
                    p.vel.x += ((p.pos.y * 9.0).sin()) * dt * 0.8;
                    p.pos += p.vel * dt;
                    if !world.liquid(p.pos.floor().as_ivec3()) {
                        p.life = 0.0;
                    }
                }
                _ => {
                    p.vel.y -= 16.0 * dt;
                    p.vel *= 1.0 - dt * 0.8;
                    let next = p.pos + p.vel * dt;
                    let cell = next.floor().as_ivec3();
                    if world.solid(cell) {
                        if p.kind == Kind::Drop {
                            p.life = 0.0;
                        } else {
                            // Land and slide to a stop.
                            p.vel = Vec3::new(p.vel.x * 0.3, 0.0, p.vel.z * 0.3);
                        }
                    } else if p.kind == Kind::Drop && p.vel.y < 0.0 && world.liquid(cell) {
                        p.life = 0.0;
                    } else {
                        p.pos = next;
                    }
                }
            }
        }
        self.list.retain(|p| p.life > 0.0);
    }

    fn gpu_data(&self, world: &World, cam_block: IVec3) -> Vec<GpuParticle> {
        let c = cam_block.as_vec3();
        self.list
            .iter()
            .map(|p| {
                let l = if p.kind == Kind::Glow { 0xff } else { world.light(p.pos.floor().as_ivec3()) };
                // Shrink away at the end of life.
                let s = p.size * (p.life / 0.25).clamp(0.0, 1.0);
                GpuParticle {
                    pos: [p.pos.x - c.x, p.pos.y - c.y, p.pos.z - c.z, s],
                    uv: [p.uv[0], p.uv[1], p.layer as f32, (l >> 4) as f32 / 15.0],
                    tint: [p.tint[0], p.tint[1], p.tint[2], (l & 15) as f32 / 15.0],
                }
            })
            .collect()
    }
}

pub struct ParticleGpu {
    pipelines: [wgpu::RenderPipeline; 2],
    layout: wgpu::BindGroupLayout,
    buf: wgpu::Buffer,
    bg: wgpu::BindGroup,
    cap: usize,
    count: u32,
}

impl ParticleGpu {
    pub fn new(r: &Renderer) -> ParticleGpu {
        let device = &r.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particles"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle"),
            source: wgpu::ShaderSource::Wgsl(format!("{}\n{}", include_str!("shaders/common.wgsl"), include_str!("shaders/particle.wgsl")).into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle"),
            bind_group_layouts: &[&r.globals_layout, &r.tex_layout, &layout],
            push_constant_ranges: &[],
        });
        let make = |format: wgpu::TextureFormat| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("particle"),
                layout: Some(&pl),
                vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_particle"), buffers: &[], compilation_options: Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_particle"),
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
            })
        };
        let pipelines = [make(r.config.format), make(crate::post::HDR)];
        let cap = 256;
        let buf = Self::make_buf(device, cap);
        let bg = Self::make_bg(device, &layout, &buf);
        ParticleGpu { pipelines, layout, buf, bg, cap, count: 0 }
    }

    fn make_buf(device: &wgpu::Device, cap: usize) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particles"),
            contents: &vec![0u8; cap * std::mem::size_of::<GpuParticle>()],
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn make_bg(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, buf: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("particles"), layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }] })
    }

    pub fn prepare(&mut self, r: &Renderer, particles: &Particles, world: &World, cam_block: IVec3) {
        let data = particles.gpu_data(world, cam_block);
        self.count = data.len() as u32;
        if data.is_empty() {
            return;
        }
        if data.len() > self.cap {
            self.cap = data.len().next_power_of_two();
            self.buf = Self::make_buf(&r.device, self.cap);
            self.bg = Self::make_bg(&r.device, &self.layout, &self.buf);
        }
        r.queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(&data));
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
