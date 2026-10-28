//! Extra lighting for the 3D scene (Sildur's Enhanced Default idea, our own code): sun and moon
//! shadows from a shadow map, water reflections from the drawn scene, and the camera jitter that
//! temporal anti-aliasing (in `post.rs`) needs. Everything here is optional and costs nothing
//! when switched off: the terrain shader then gets 1×1 dummies and skips the work.
//!
//! - Shadows: one orthographic map centred on the camera, snapped to whole texels so it doesn't
//!   shimmer as you walk. Casters are the chunk sections within its radius, drawn depth-only with
//!   the same vertex buffers as the main pass. Fast: 1024² over 48 blocks, one filtered tap.
//!   Fancy: 2048² over 96 blocks, five taps.
//!   Mobs and the player cast into the same map (entity.rs draws them depth-only).
//! - Reflections: the scene (sky, terrain, mobs, clouds) is copied before water is drawn, and the
//!   water shader walks its reflected ray over that copy and the depth buffer. With a shader pack
//!   the copy comes from its HDR texture, without one straight from the screen.
//! - Hand light shadows: a 32³ grid of solid blocks around the player; the terrain shader walks
//!   from each lit point toward the torch in the hand and stops at the first solid block.

use glam::{DMat4, DVec3, IVec3, Mat4, Vec3};

pub const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// (map size, radius in blocks) for shadow level 1 and 2.
const SHADOW_LEVELS: [(u32, f32); 2] = [(1024, 48.0), (2048, 96.0)];
/// How far behind the camera (toward the light) casters can be.
const SHADOW_DEPTH: f32 = 192.0;
/// Side of the solid-block grid around the player for hand light shadows.
pub const OCCL_SIZE: u32 = 32;

/// Halton (2, 3) points for the TAA jitter, in pixels from the centre.
const JITTER: [(f32, f32); 8] = [
    (0.0, -0.1667),
    (-0.25, 0.1667),
    (0.25, -0.3889),
    (-0.375, -0.0556),
    (0.125, 0.2778),
    (-0.125, -0.2778),
    (0.375, 0.0556),
    (-0.4375, 0.3889),
];

pub struct SceneFx {
    pub layout: wgpu::BindGroupLayout,
    cmp: wgpu::Sampler,
    dummy_depth: wgpu::TextureView,
    dummy_color: wgpu::TextureView,
    shadow: Option<(wgpu::TextureView, u32, f32)>,
    /// Shadows on the solid pass (no scene copy) and on the translucent pass (with it).
    pub bg_solid: wgpu::BindGroup,
    pub bg_trans: wgpu::BindGroup,
    /// Copy of the scene before water is drawn: (texture, view, width, height, format).
    copy: Option<(wgpu::Texture, wgpu::TextureView, u32, u32, wgpu::TextureFormat)>,
    /// Solid blocks around the player (1 = blocks light), for hand light shadows.
    occl: wgpu::Texture,
    occl_view: wgpu::TextureView,
    /// The material class of each block texture layer (material.rs), for metal and gem blocks
    /// and ores.
    mats: wgpu::Buffer,
    /// Corner of the uploaded grid and when it was built (blocks change, so it is refreshed).
    occl_built: Option<(IVec3, std::time::Instant)>,
    /// Settings: shadow level 0..2, water reflections.
    pub level: u8,
    pub reflections: bool,
    /// Frame counter for the jitter.
    frame: u32,
    /// Unjittered view-projection and camera block of the last frame (TAA reprojection).
    prev: Option<(Mat4, IVec3)>,
    pub fancy: bool,
}

fn depth_view(device: &wgpu::Device, label: &str, size: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn entry(binding: u32, ty: wgpu::BindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty, count: None }
}

impl SceneFx {
    pub fn new(device: &wgpu::Device) -> SceneFx {
        let tex = |sample_type| wgpu::BindingType::Texture { sample_type, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene fx"),
            entries: &[
                entry(0, tex(wgpu::TextureSampleType::Depth)),
                entry(1, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison)),
                entry(2, tex(wgpu::TextureSampleType::Float { filterable: false })),
                entry(3, tex(wgpu::TextureSampleType::Depth)),
                entry(4, wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false }),
                entry(5, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }),
            ],
        });
        let cmp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow compare"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let dummy_depth = depth_view(device, "fx dummy depth", 1);
        let dummy_color = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("fx dummy color"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: crate::post::HDR,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let occl = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hand light occluders"),
            size: wgpu::Extent3d { width: OCCL_SIZE, height: OCCL_SIZE, depth_or_array_layers: OCCL_SIZE },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let occl_view = occl.create_view(&Default::default());
        let mats = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("block materials"),
            size: (crate::material::BLOCK_LAYERS * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bg = Self::make_bg(device, &layout, &dummy_depth, &cmp, &dummy_color, &dummy_depth, &occl_view, &mats);
        let bg2 = Self::make_bg(device, &layout, &dummy_depth, &cmp, &dummy_color, &dummy_depth, &occl_view, &mats);
        SceneFx {
            layout,
            cmp,
            dummy_depth,
            dummy_color,
            shadow: None,
            bg_solid: bg,
            bg_trans: bg2,
            copy: None,
            occl,
            occl_view,
            mats,
            occl_built: None,
            level: 0,
            reflections: false,
            frame: 0,
            prev: None,
            fancy: true,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn make_bg(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        shadow: &wgpu::TextureView,
        cmp: &wgpu::Sampler,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        occl: &wgpu::TextureView,
        mats: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene fx"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(shadow) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(cmp) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(color) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(depth) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(occl) },
                wgpu::BindGroupEntry { binding: 5, resource: mats.as_entire_binding() },
            ],
        })
    }

    /// Bring textures and bind groups in line with the settings and the screen. `scene`: the
    /// format of the texture the scene is drawn into, when it can be copied (reflections need it).
    pub fn prepare(&mut self, device: &wgpu::Device, scene: Option<wgpu::TextureFormat>, w: u32, h: u32, depth: &wgpu::TextureView, depth_changed: bool) {
        let want_shadow = (self.level > 0).then(|| SHADOW_LEVELS[(self.level as usize - 1).min(1)]);
        let have_shadow = self.shadow.as_ref().map(|s| (s.1, s.2));
        let mut dirty = depth_changed;
        if want_shadow != have_shadow {
            self.shadow = want_shadow.map(|(size, r)| (depth_view(device, "shadow map", size), size, r));
            dirty = true;
        }
        let want_copy = scene.filter(|_| self.reflections).map(|f| (w, h, f));
        let have_copy = self.copy.as_ref().map(|c| (c.2, c.3, c.4));
        if want_copy != have_copy {
            self.copy = want_copy.map(|(w, h, format)| {
                let t = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("scene copy"),
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let v = t.create_view(&Default::default());
                (t, v, w, h, format)
            });
            dirty = true;
        }
        if dirty {
            let sh = self.shadow.as_ref().map(|s| &s.0).unwrap_or(&self.dummy_depth);
            self.bg_solid = Self::make_bg(device, &self.layout, sh, &self.cmp, &self.dummy_color, &self.dummy_depth, &self.occl_view, &self.mats);
            self.bg_trans = match &self.copy {
                Some((_, v, ..)) => Self::make_bg(device, &self.layout, sh, &self.cmp, v, depth, &self.occl_view, &self.mats),
                None => Self::make_bg(device, &self.layout, sh, &self.cmp, &self.dummy_color, &self.dummy_depth, &self.occl_view, &self.mats),
            };
        }
    }

    /// Whether water reflections run this frame (the main pass is split around a scene copy).
    /// Block materials, from the texture names in layer order.
    pub fn set_block_materials(&self, queue: &wgpu::Queue, names: &[String]) {
        queue.write_buffer(&self.mats, 0, bytemuck::cast_slice(&crate::material::block_table(names)));
    }

    pub fn reflecting(&self) -> bool {
        self.copy.is_some()
    }

    pub fn copy_texture(&self) -> Option<&wgpu::Texture> {
        self.copy.as_ref().map(|c| &c.0)
    }

    /// Whether the solid-block grid with this corner should be (re)built: it moved, or a quarter
    /// of a second passed (blocks may have been placed or broken).
    pub fn occluders_stale(&self, corner: IVec3) -> bool {
        !matches!(self.occl_built, Some((c, t)) if c == corner && t.elapsed().as_secs_f32() < 0.25)
    }

    /// Upload the solid-block grid for hand light shadows (OCCL_SIZE³ bytes, x fastest, then y).
    pub fn upload_occluders(&mut self, queue: &wgpu::Queue, corner: IVec3, data: &[u8]) {
        self.occl_built = Some((corner, std::time::Instant::now()));
        queue.write_texture(
            self.occl.as_image_copy(),
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(OCCL_SIZE), rows_per_image: Some(OCCL_SIZE) },
            wgpu::Extent3d { width: OCCL_SIZE, height: OCCL_SIZE, depth_or_array_layers: OCCL_SIZE },
        );
    }

    pub fn shadow_view(&self) -> Option<&wgpu::TextureView> {
        self.shadow.as_ref().map(|s| &s.0)
    }

    /// Radius of the shadowed area in blocks (0 = off).
    pub fn shadow_radius(&self) -> f32 {
        self.shadow.as_ref().map(|s| s.2).unwrap_or(0.0)
    }

    /// Shadow map matrix for positions relative to the camera's block, and the light direction.
    /// None when shadows are off or the light is too low to cast any.
    pub fn shadow_matrix(&self, sun: Vec3, cam_block: IVec3, cam_frac: Vec3) -> Option<Mat4> {
        let (_, size, radius) = self.shadow.as_ref()?;
        let l = if sun.y > -0.05 { sun } else { -sun };
        if l.y < 0.02 {
            return None;
        }
        let l = l.as_dvec3().normalize();
        let up = if l.y.abs() > 0.99 { DVec3::Z } else { DVec3::Y };
        let rot = DMat4::look_to_rh(DVec3::ZERO, -l, up);
        // Centre on the camera in light space, snapped to whole texels.
        let abs = cam_block.as_dvec3() + cam_frac.as_dvec3();
        let c = rot.transform_point3(abs);
        let texel = 2.0 * *radius as f64 / *size as f64;
        let snapped = DVec3::new((c.x / texel).floor() * texel, (c.y / texel).floor() * texel, c.z);
        // rel → abs → light space → around the snapped centre.
        let to_light = DMat4::from_translation(-snapped) * rot * DMat4::from_translation(cam_block.as_dvec3());
        let r = *radius;
        let ortho = Mat4::orthographic_rh(-r, r, -r, r, -SHADOW_DEPTH, SHADOW_DEPTH);
        Some(ortho * to_light.as_mat4())
    }

    /// Fill the shadow fields of the globals.
    pub fn shadow_globals(&self, m: Option<Mat4>) -> ([[f32; 4]; 4], [f32; 4]) {
        match (m, &self.shadow) {
            (Some(m), Some((_, size, r))) => (m.to_cols_array_2d(), [1.0, 2.0 / *size as f32, if self.level >= 2 { 1.0 } else { 0.0 }, *r]),
            _ => (Mat4::IDENTITY.to_cols_array_2d(), [0.0; 4]),
        }
    }

    /// Sub-pixel jitter for this frame's projection (clip-space offset), or zero without TAA.
    pub fn jitter(&mut self, w: u32, h: u32, active: bool) -> Mat4 {
        if !active {
            self.prev = None;
            return Mat4::IDENTITY;
        }
        self.frame = self.frame.wrapping_add(1);
        let (jx, jy) = JITTER[(self.frame % 8) as usize];
        Mat4::from_translation(Vec3::new(jx * 2.0 / w as f32, jy * 2.0 / h as f32, 0.0))
    }

    /// Matrix taking this frame's clip position (jittered) to last frame's (unjittered), and
    /// whether there was a last frame. Remembers this frame for the next one.
    pub fn reprojection(&mut self, jittered: Mat4, plain: Mat4, block: IVec3) -> (Mat4, bool) {
        let out = match self.prev {
            Some((pvp, pblock)) => (pvp * Mat4::from_translation((block - pblock).as_vec3()) * jittered.inverse(), true),
            None => (Mat4::IDENTITY, false),
        };
        self.prev = Some((plain, block));
        out
    }
}

