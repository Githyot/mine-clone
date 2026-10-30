//! Paintings hung on walls (idea from the Paintings++ mod): built-in art from
//! `packs/vanilla/paintings.ron`, plus any PNG the player drops in the `paintings` folder.

use std::path::Path;

use glam::{IVec3, Mat4, Vec3};
use serde::{Deserialize, Serialize};
use voxel::physics::Terrain;
use voxel::AIR;

use crate::embedded;
use crate::entity::{Bones, SKIN_W};
use crate::world::World;

/// Largest painting, in blocks.
pub const MAX_SIZE: u32 = 4;

pub struct PaintingDef {
    pub name: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// RGBA, 16 pixels per block (width*16 × height*16).
    pub pixels: Vec<u8>,
    /// See-through where the picture is (Paintings++ transparency): drawn without its
    /// wooden back, and clear pixels show the wall.
    pub alpha: bool,
}

/// True when a picture has clear pixels (transparent paintings).
pub fn has_alpha(pixels: &[u8]) -> bool {
    pixels.chunks_exact(4).filter(|p| p[3] < 128).count() * 50 > pixels.len() / 4
}

#[derive(Deserialize)]
struct PaintingFile {
    paintings: Vec<PaintingEntry>,
}

#[derive(Deserialize)]
struct PaintingEntry {
    name: String,
    title: String,
    width: u32,
    height: u32,
}

pub(crate) fn load_png(bytes: &[u8], w: u32, h: u32) -> Option<Vec<u8>> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (tw, th) = (w * 16, h * 16);
    let img = if img.width() != tw || img.height() != th { image::imageops::resize(&img, tw, th, image::imageops::FilterType::Triangle) } else { img };
    Some(img.into_raw())
}

/// Built-in paintings first, then the player's own.
pub fn load_defs(base: &Path) -> Vec<PaintingDef> {
    let text = std::fs::read_to_string(base.join("packs/vanilla/paintings.ron")).unwrap_or_else(|_| embedded::PAINTINGS_RON.to_string());
    let mut out = Vec::new();
    match ron::from_str::<PaintingFile>(&text) {
        Ok(f) => {
            for e in f.paintings {
                let w = e.width.clamp(1, MAX_SIZE);
                let h = e.height.clamp(1, MAX_SIZE);
                let bytes = std::fs::read(base.join(format!("packs/vanilla/textures/painting/{}.png", e.name)))
                    .ok()
                    .or_else(|| embedded::PAINTING_TEXTURES.iter().find(|(n, _)| *n == e.name).map(|(_, b)| b.to_vec()));
                match bytes.and_then(|b| load_png(&b, w, h)) {
                    Some(pixels) => out.push(PaintingDef { alpha: has_alpha(&pixels), name: e.name, title: e.title, width: w, height: h, pixels }),
                    None => log::warn!("painting '{}' has no texture", e.name),
                }
            }
        }
        Err(e) => log::error!("paintings.ron: {e}"),
    }
    // The player's folder: the size in blocks comes from the picture's shape.
    let dir = base.join("paintings");
    if let Ok(rd) = std::fs::read_dir(&dir) {
        let mut files: Vec<_> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("png"))).collect();
        files.sort();
        for p in files {
            let Ok(bytes) = std::fs::read(&p) else { continue };
            let Ok((iw, ih)) = image::ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format().map_err(|_| ()).and_then(|r| r.into_dimensions().map_err(|_| ())) else {
                continue;
            };
            let (w, h) = fit_blocks(iw, ih);
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(pixels) = load_png(&bytes, w, h) {
                out.push(PaintingDef { alpha: has_alpha(&pixels), name: format!("custom:{stem}"), title: stem, width: w, height: h, pixels });
            }
        }
    }
    // Painting packs (Paintings++ and data-pack style) in paintings/packs.
    out.extend(crate::mods::paintings_pp::import_packs(&dir.join("packs")));
    log::info!("{} paintings", out.len());
    out
}

/// Pick a size in blocks (1..4 each way) that keeps the picture's shape.
pub(crate) fn fit_blocks(w: u32, h: u32) -> (u32, u32) {
    let aspect = w as f32 / h.max(1) as f32;
    let mut best = (1, 1);
    let mut best_err = f32::MAX;
    for bh in 1..=MAX_SIZE {
        for bw in 1..=MAX_SIZE {
            let err = ((bw as f32 / bh as f32) / aspect).ln().abs();
            // Prefer the bigger size when two fit equally well, like a real canvas.
            let score = err * 10.0 - (bw * bh) as f32 * 0.01 * if w >= 64 || h >= 64 { 1.0 } else { -1.0 };
            if score < best_err {
                best_err = score;
                best = (bw, bh);
            }
        }
    }
    best
}

/// Skin for the entity renderer: painting at the top-left, a wooden back below it.
pub fn skin_pixels(def: &PaintingDef, back: &[u8]) -> Vec<u8> {
    let sw = SKIN_W as usize;
    let mut px = vec![0u8; sw * 128 * 4];
    let (w, h) = ((def.width * 16) as usize, (def.height * 16) as usize);
    for y in 0..h {
        for x in 0..w {
            let s = (y * w + x) * 4;
            let d = (y * sw + x) * 4;
            px[d..d + 4].copy_from_slice(&def.pixels[s..s + 4]);
            if !def.alpha {
                px[d + 3] = 255;
            }
        }
    }
    for y in 0..16 {
        for x in 0..16 {
            let s = (y * 16 + x) * 4;
            let d = ((64 + y) * sw + x) * 4;
            if back.len() >= s + 4 {
                px[d..d + 4].copy_from_slice(&back[s..s + 4]);
            }
            px[d + 3] = 255;
        }
    }
    px
}

pub fn back_pixels(base: &Path) -> Vec<u8> {
    let bytes = std::fs::read(base.join("packs/vanilla/textures/painting/back.png"))
        .ok()
        .or_else(|| embedded::PAINTING_TEXTURES.iter().find(|(n, _)| *n == "back").map(|(_, b)| b.to_vec()));
    bytes.and_then(|b| load_png(&b, 1, 1)).unwrap_or_else(|| [120u8, 90, 60, 255].repeat(256))
}

/// A painting on a wall.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Painting {
    pub name: String,
    /// The bottom-left air cell the painting covers (seen from the front).
    pub anchor: [i32; 3],
    /// Direction the picture faces (away from the wall).
    pub facing: [i32; 3],
    #[serde(skip)]
    pub def: usize,
}

impl Painting {
    pub fn cells(&self, defs: &[PaintingDef]) -> Vec<IVec3> {
        let d = &defs[self.def];
        cells(IVec3::from(self.anchor), IVec3::from(self.facing), d.width, d.height)
    }

    /// Box in world space (for hitting it).
    pub fn aabb(&self, defs: &[PaintingDef]) -> (Vec3, Vec3) {
        let cs = self.cells(defs);
        let n = IVec3::from(self.facing).as_vec3();
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        for c in cs {
            let cv = c.as_vec3();
            lo = lo.min(cv);
            hi = hi.max(cv + Vec3::ONE);
        }
        // Only the thin slab against the wall.
        let thick = 1.0 / 16.0;
        if n.x > 0.0 {
            hi.x = lo.x + thick;
        } else if n.x < 0.0 {
            lo.x = hi.x - thick;
        } else if n.z > 0.0 {
            hi.z = lo.z + thick;
        } else {
            lo.z = hi.z - thick;
        }
        (lo, hi)
    }

    pub fn bones(&self, cam_block: IVec3, world: &World) -> Bones {
        let n = IVec3::from(self.facing);
        let r = right_of(n).as_vec3();
        let c = IVec3::from(self.anchor).as_vec3() + Vec3::splat(0.5);
        let corner = c - r * 0.5 - Vec3::Y * 0.5 - n.as_vec3() * 0.5;
        let yaw = (n.x as f32).atan2(n.z as f32);
        let model = Mat4::from_translation(corner - cam_block.as_vec3()) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(1.0 / 16.0));
        let l = world.light(IVec3::from(self.anchor));
        let mut b = Bones { light: [(l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0, 0.0, 0.0], ..Default::default() };
        b.m[0] = model.to_cols_array_2d();
        b
    }
}

/// Texture "right" for a picture facing `n` (as seen by someone looking at it).
pub fn right_of(n: IVec3) -> IVec3 {
    IVec3::new(n.z, 0, -n.x)
}

pub fn cells(anchor: IVec3, n: IVec3, w: u32, h: u32) -> Vec<IVec3> {
    let r = right_of(n);
    let mut v = Vec::new();
    for j in 0..h as i32 {
        for i in 0..w as i32 {
            v.push(anchor + r * i + IVec3::Y * j);
        }
    }
    v
}

/// Where a painting of this size can hang if the player clicks `cell` (an air cell in front
/// of a wall facing `n`): tries to centre it on the cell, then shifts it to fit.
pub fn fit(world: &World, existing: &[Painting], defs: &[PaintingDef], cell: IVec3, n: IVec3, w: u32, h: u32) -> Option<IVec3> {
    let r = right_of(n);
    let taken: Vec<IVec3> = existing.iter().flat_map(|p| p.cells(defs)).collect();
    let cx = (w as i32 - 1) / 2;
    let cy = (h as i32 - 1) / 2;
    let mut tries = Vec::new();
    tries.push((cx, cy));
    for dy in 0..h as i32 {
        for dx in 0..w as i32 {
            if (dx, dy) != (cx, cy) {
                tries.push((dx, dy));
            }
        }
    }
    for (dx, dy) in tries {
        let anchor = cell - r * dx - IVec3::Y * dy;
        let ok = cells(anchor, n, w, h).iter().all(|&c| {
            let wall = c - n;
            world.block(c) == Some(AIR) && world.block(wall).is_some() && Terrain::solid(world, wall) && !taken.contains(&c)
        });
        if ok {
            return Some(anchor);
        }
    }
    None
}

/// The painting a ray hits first, if any, and how far away.
pub fn pick(list: &[Painting], defs: &[PaintingDef], o: Vec3, d: Vec3, max: f32) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;
    for (i, p) in list.iter().enumerate() {
        let (lo, hi) = p.aabb(defs);
        let inv = Vec3::ONE / d;
        let t1 = (lo - o) * inv;
        let t2 = (hi - o) * inv;
        let tmin = t1.min(t2).max_element().max(0.0);
        let tmax = t1.max(t2).min_element();
        if tmax >= tmin && tmin <= max && best.is_none_or(|(_, b)| tmin < b) {
            best = Some((i, tmin));
        }
    }
    best
}

pub fn save(dir: &Path, list: &[Painting]) {
    let _ = std::fs::create_dir_all(dir);
    match ron::ser::to_string_pretty(&list.to_vec(), ron::ser::PrettyConfig::default()) {
        Ok(t) => {
            let _ = voxel::save::write_atomic(dir.join("paintings.ron"), t);
        }
        Err(e) => log::error!("could not save paintings: {e}"),
    }
}

pub fn load(dir: &Path, defs: &[PaintingDef]) -> Vec<Painting> {
    let Ok(text) = std::fs::read_to_string(dir.join("paintings.ron")) else { return Vec::new() };
    let list: Vec<Painting> = ron::from_str(&text).unwrap_or_default();
    list.into_iter()
        .filter_map(|mut p| {
            p.def = defs.iter().position(|d| d.name == p.name)?;
            Some(p)
        })
        .collect()
}
