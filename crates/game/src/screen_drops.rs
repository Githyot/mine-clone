//! Raindrops on the screen: out in the rain the view gets a few wet beads, as if on a lens.
//! Small ones sit still and dry out, bigger ones start to slide down with a thin trail, stop
//! and go, swallow the beads in their way and leave the bottom of the screen. Looking up
//! catches more of them; under a roof or a tree no new ones land and the old ones dry, and
//! diving washes them all off (coming up again leaves a few running down).
//!
//! `Wet` is the simulation (CPU, a few dozen drops). `DropFx` draws them after the shader pack
//! and before the HUD: one quad per drop that bends a copy of the picture behind it (a lens
//! turns it over and shrinks it) with a dark rim and a highlight. Displays that don't allow
//! copying from the screen get the rim and highlight only.

use voxel::rng::Rng;

/// Most drops on the screen at once.
const MAX_DROPS: usize = 72;
/// Option levels: (name in the menu, how many land, how long they last).
pub const LEVELS: [(&str, f32, f32); 3] = [("Não", 0.0, 0.0), ("Sutil", 1.8, 1.2), ("Mais", 3.6, 1.5)];
/// Beads from this size up (screen heights) can slide.
const SLIDE_R: f32 = 0.0085;

#[derive(Clone, Debug)]
pub struct Drop {
    /// Centre: x in 0..aspect, y in 0..1 from the top (screen heights).
    pub x: f32,
    pub y: f32,
    /// Radius in screen heights.
    pub r: f32,
    /// Length of the wet trail above a sliding drop.
    pub trail: f32,
    pub vy: f32,
    /// Seconds left before it starts drying; then `fade` counts down to gone.
    pub life: f32,
    pub fade: f32,
    /// Seconds before a big bead lets go, or a pause while sliding.
    pub hold: f32,
    pub seed: f32,
    /// Grows from 0 to 1 when it lands (a splat, not a pop).
    pub grow: f32,
}

impl Drop {
    fn sliding(&self) -> bool {
        self.r >= SLIDE_R && self.hold <= 0.0
    }
    fn alpha(&self) -> f32 {
        (self.fade / FADE).clamp(0.0, 1.0) * self.grow.min(1.0)
    }
}

/// Seconds a drop takes to dry once its life is over.
const FADE: f32 = 1.2;

/// What the camera is in this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Exposure {
    /// How hard it rains where the camera is (0 = dry, snow or desert).
    pub rain: f32,
    /// Vertical part of the view direction (-1 down .. 1 up).
    pub look_y: f32,
    /// Something above the camera keeps the rain off.
    pub roofed: bool,
    /// The camera is in water (or lava).
    pub submerged: bool,
    /// The setting (index into `LEVELS`).
    pub level: u8,
}

pub struct Wet {
    pub drops: Vec<Drop>,
    rng: Rng,
    /// Fractional drops waiting to land.
    due: f32,
    was_submerged: bool,
    /// Width over height of the screen.
    pub aspect: f32,
}

/// One drop for the GPU: centre and radius in pixels, trail length, then alpha, seed and
/// how far along a slide it is (stretches the bead).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct DropGpu {
    pub a: [f32; 4],
    pub b: [f32; 4],
}

impl Wet {
    pub fn new(seed: u64) -> Wet {
        Wet { drops: Vec::new(), rng: Rng::new(seed ^ 0xd209_5ca1), due: 0.0, was_submerged: false, aspect: 16.0 / 9.0 }
    }

    /// Drops per second landing now.
    pub fn rate(e: &Exposure) -> f32 {
        let (_, many, _) = LEVELS[e.level.min(2) as usize];
        if e.roofed || e.submerged || e.rain <= 0.05 {
            return 0.0;
        }
        // Looking at the horizon still catches a few; straight up, about four times as many.
        let up = ((e.look_y + 0.35) / 1.1).clamp(0.0, 1.0);
        e.rain * many * (1.1 + up * up * 4.2)
    }

    pub fn update(&mut self, e: &Exposure, dt: f32) {
        let (_, _, lasting) = LEVELS[e.level.min(2) as usize];
        if e.level == 0 {
            self.drops.clear();
            return;
        }
        // Underwater the screen is clean; coming up leaves a few running down.
        if e.submerged {
            self.drops.clear();
            self.due = 0.0;
            self.was_submerged = true;
            return;
        }
        if std::mem::take(&mut self.was_submerged) {
            for _ in 0..(6.0 + lasting * 5.0) as usize {
                let r = 0.008 + self.rng.f32() * 0.009;
                let y = self.rng.f32() * 0.7;
                let life = 1.5 + self.rng.f32() * 2.0;
                self.land(r, y, life);
            }
        }
        self.due += Self::rate(e) * dt;
        while self.due >= 1.0 {
            self.due -= 1.0;
            // Mostly small beads, now and then a big one.
            let k = self.rng.f32();
            let r = 0.006 + k * k * k.sqrt() * 0.017;
            let y = self.rng.f32() * 1.05 - 0.05;
            let life = (1.8 + self.rng.f32() * 3.5) * lasting;
            self.land(r, y, life);
        }
        // Out of the rain everything dries sooner.
        let dry = if Self::rate(e) <= 0.0 { 2.5 } else { 1.0 };
        for d in &mut self.drops {
            d.grow = (d.grow + dt * 9.0).min(1.0);
            if d.life > 0.0 {
                d.life -= dt * dry;
            } else {
                d.fade -= dt * dry;
                d.r *= 1.0 - dt * 0.25;
            }
            if d.r >= SLIDE_R {
                if d.hold > 0.0 {
                    d.hold -= dt;
                    d.vy *= 0.5;
                } else {
                    // Heavier drops run faster; a little friction keeps them from racing.
                    let top = 0.12 + (d.r - SLIDE_R) * 22.0;
                    d.vy = (d.vy + dt * 0.6).min(top);
                    d.y += d.vy * dt;
                    d.x += (d.seed * 40.0 + d.y * 9.0).sin() * dt * 0.004;
                    d.trail = (d.trail + d.vy * dt).min(0.06 + d.r * 6.0);
                    // The trail costs water; in the rain running drops keep going until they leave.
                    d.r -= dt * 0.0006;
                    if dry == 1.0 {
                        d.life = d.life.max(0.5);
                    }
                    let mut stall = d.seed * 977.0 + d.y * 13.0;
                    stall = stall.fract();
                    if stall < dt * 0.35 {
                        d.hold = 0.15 + d.seed * 0.5;
                    }
                }
            } else if d.trail > 0.0 {
                d.trail = (d.trail - dt * 0.08).max(0.0);
            }
        }
        // A running drop swallows the beads it meets.
        let mut i = 0;
        while i < self.drops.len() {
            if self.drops[i].sliding() {
                let (x, y, r) = (self.drops[i].x, self.drops[i].y, self.drops[i].r);
                let mut j = 0;
                while j < self.drops.len() {
                    let o = &self.drops[j];
                    if j != i && o.r < r && (o.x - x).hypot(o.y - y) < (r + o.r) * 0.8 {
                        let gained = o.r * o.r;
                        self.drops.swap_remove(j);
                        // swap_remove may have moved the runner into slot j.
                        if i == self.drops.len() {
                            i = j;
                        }
                        let d = &mut self.drops[i];
                        d.r = (d.r * d.r + gained).sqrt().min(0.022);
                    } else {
                        j += 1;
                    }
                }
            }
            i += 1;
        }
        self.drops.retain(|d| d.fade > 0.0 && d.y - d.trail < 1.1 && d.r > 0.002);
    }

    fn land(&mut self, r: f32, y: f32, life: f32) {
        if self.drops.len() >= MAX_DROPS {
            // The oldest small bead makes room.
            if let Some(i) = self.drops.iter().position(|d| d.r < SLIDE_R) {
                self.drops.remove(i);
            } else {
                return;
            }
        }
        let x = self.rng.f32() * self.aspect;
        let seed = self.rng.f32();
        let hold = if r >= SLIDE_R { 0.3 + self.rng.f32() * 1.8 } else { 0.0 };
        self.drops.push(Drop { x, y, r, trail: 0.0, vy: 0.0, life, fade: FADE, hold, seed, grow: 0.0 });
    }

    /// The drops in pixels for a screen `h` pixels tall.
    pub fn instances(&self, h: f32) -> Vec<DropGpu> {
        self.drops
            .iter()
            .map(|d| DropGpu {
                a: [d.x * h, d.y * h, d.r * h * d.grow.min(1.0).sqrt(), d.trail * h],
                b: [d.alpha(), d.seed, if d.sliding() { (d.vy * 8.0).min(1.0) } else { 0.0 }, 0.0],
            })
            .collect()
    }
}

/// Must match `struct Drops` in drops.wgsl.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DropUniform {
    /// Width, height, 1 when the picture behind can be read, unused.
    screen: [f32; 4],
}

pub struct DropFx {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    instances: wgpu::Buffer,
    count: u32,
    format: wgpu::TextureFormat,
    can_copy: bool,
    /// Copy of the finished picture (made on the first wet frame, remade on resize).
    copy: Option<(wgpu::Texture, u32, u32)>,
    bind: Option<wgpu::BindGroup>,
    dummy: wgpu::TextureView,
}

impl DropFx {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, can_copy: bool) -> DropFx {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("screen drops"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/drops.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screen drops"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
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
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("screen drops"), bind_group_layouts: &[&layout], push_constant_ranges: &[] });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("screen drops"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_drop"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<DropGpu>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_drop"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen drops"),
            size: std::mem::size_of::<DropUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen drops"),
            size: (MAX_DROPS * std::mem::size_of::<DropGpu>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("screen drops"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let dummy = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("screen drops dummy"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        DropFx { pipeline, layout, uniform, sampler, instances, count: 0, format, can_copy, copy: None, bind: None, dummy }
    }

    pub fn set(&mut self, queue: &wgpu::Queue, drops: &[DropGpu]) {
        let n = drops.len().min(MAX_DROPS);
        if n > 0 {
            queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&drops[..n]));
        }
        self.count = n as u32;
    }

    /// Draw onto the finished picture `frame` (before the HUD).
    pub fn draw(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, frame: &wgpu::Texture, view: &wgpu::TextureView) {
        if self.count == 0 {
            return;
        }
        let (w, h) = (frame.width(), frame.height());
        if self.bind.is_none() || self.can_copy && self.copy.as_ref().is_none_or(|c| (c.1, c.2) != (w, h)) {
            let tex = self.can_copy.then(|| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("screen drops copy"),
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.format,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                })
            });
            let tv = tex.as_ref().map(|t| t.create_view(&Default::default()));
            self.bind = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("screen drops"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.uniform.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(tv.as_ref().unwrap_or(&self.dummy)) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            }));
            self.copy = tex.map(|t| (t, w, h));
        }
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&DropUniform { screen: [w as f32, h as f32, if self.copy.is_some() { 1.0 } else { 0.0 }, 0.0] }));
        if let Some((t, _, _)) = &self.copy {
            enc.copy_texture_to_texture(frame.as_image_copy(), t.as_image_copy(), wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 });
        }
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("screen drops"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, self.bind.as_ref(), &[]);
        pass.set_vertex_buffer(0, self.instances.slice(..));
        pass.draw(0..4, 0..self.count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rain(look_y: f32) -> Exposure {
        Exposure { rain: 1.0, look_y, roofed: false, submerged: false, level: 1 }
    }

    fn run(w: &mut Wet, e: &Exposure, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            w.update(e, 1.0 / 60.0);
        }
    }

    #[test]
    fn looking_up_wets_more() {
        let (mut a, mut b) = (Wet::new(1), Wet::new(1));
        run(&mut a, &rain(-0.2), 4.0);
        run(&mut b, &rain(0.9), 4.0);
        assert!(!a.drops.is_empty(), "a few even looking ahead");
        assert!(b.drops.len() > a.drops.len() * 2, "{} vs {}", b.drops.len(), a.drops.len());
        assert!(b.drops.len() <= MAX_DROPS);
    }

    #[test]
    fn subtle_by_default() {
        let mut w = Wet::new(2);
        run(&mut w, &rain(0.0), 20.0);
        // Looking ahead in steady rain: a handful of beads, not a curtain.
        assert!((2..=20).contains(&w.drops.len()), "{}", w.drops.len());
    }

    #[test]
    fn roof_dries_and_water_washes() {
        let mut w = Wet::new(3);
        run(&mut w, &rain(1.0), 4.0);
        assert!(!w.drops.is_empty());
        let roof = Exposure { roofed: true, ..rain(1.0) };
        run(&mut w, &roof, 6.0);
        assert!(w.drops.is_empty(), "{} left under the roof", w.drops.len());
        run(&mut w, &rain(1.0), 4.0);
        let dive = Exposure { submerged: true, ..rain(1.0) };
        w.update(&dive, 0.016);
        assert!(w.drops.is_empty());
        // Coming up out of the water leaves a few drops even with no rain.
        w.update(&Exposure { rain: 0.0, ..rain(0.0) }, 0.016);
        assert!(w.drops.len() >= 6);
        let off = Exposure { level: 0, ..rain(1.0) };
        w.update(&off, 0.016);
        assert!(w.drops.is_empty());
    }

    #[test]
    fn big_drops_run_down_and_leave() {
        let mut w = Wet::new(4);
        w.drops.push(Drop { x: 0.5, y: 0.2, r: 0.016, trail: 0.0, vy: 0.0, life: 30.0, fade: FADE, hold: 0.0, seed: 0.3, grow: 1.0 });
        let dry = Exposure { rain: 0.0, ..rain(0.0) };
        w.update(&dry, 0.5);
        assert!(w.drops[0].y > 0.2 && w.drops[0].trail > 0.0);
        run(&mut w, &dry, 12.0);
        assert!(w.drops.is_empty(), "{:?}", w.drops);
    }

    #[test]
    fn running_drop_swallows_beads() {
        let mut w = Wet::new(5);
        let bead = |y| Drop { x: 0.5, y, r: 0.004, trail: 0.0, vy: 0.0, life: 30.0, fade: FADE, hold: 0.0, seed: 0.5, grow: 1.0 };
        w.drops.push(Drop { r: 0.014, seed: 0.1, ..bead(0.3) });
        w.drops.push(bead(0.312));
        w.update(&Exposure { rain: 0.0, ..rain(0.0) }, 0.02);
        assert_eq!(w.drops.len(), 1);
        assert!(w.drops[0].r > 0.014);
    }
}
