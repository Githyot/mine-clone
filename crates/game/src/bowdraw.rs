//! Bows being drawn: the pictures of each bow with its string pulled back in three stages,
//! painted at load time from the bow's own picture (ours or a resource pack's), and the bow's
//! shape (where the string runs, which side the limb bows out to, where the hand grips it and
//! where the nock is at each stage) for holding it in both hands.
//!
//! The string is found as the long straight run of pale, grey pixels along the picture's
//! diagonal (top right to bottom left, like every bow picture). It is wiped out with its
//! outline and drawn again as a "V" from near the tips to the nock, pulled away from the limb.
//! The layers are named "bowpull/<bow texture>/<stage 1..3>" and come after the other extra
//! pictures (`texture_names`).

use std::collections::HashMap;

use glam::Vec2;
use image::{Rgba, RgbaImage};
use voxel::item::{ItemRegistry, Look};

use crate::textures::TextureSet;

/// The draw stages, as fractions of the full draw (like the classic bow's three pictures).
pub const STAGES: [f32; 3] = [0.4, 0.7, 1.0];

/// Bows: the plain bow and the ore bows (tr_weapons.rs).
pub fn is_bow(name: &str) -> bool {
    name == "bow" || crate::tr_weapons::bow_power(name).is_some()
}

/// Which picture stage (0 = at rest, 1..3 drawn) shows a draw of `pull` (0..1).
pub fn stage(pull: f32) -> usize {
    if pull <= 0.0 {
        0
    } else if pull < 0.45 {
        1
    } else if pull < 0.9 {
        2
    } else {
        3
    }
}

/// The extra texture layers for every bow's drawn pictures, in item order.
pub fn texture_names(items: &ItemRegistry) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for it in &items.items {
        let Look::Sprite(l) = it.look else { continue };
        if !is_bow(&it.name) {
            continue;
        }
        let Some(tex) = items.textures.get(l as usize) else { continue };
        for k in 1..=3 {
            let n = format!("bowpull/{tex}/{k}");
            if !out.contains(&n) {
                out.push(n);
            }
        }
    }
    out
}

/// "bowpull/<texture>/<stage>" as (texture, stage 1..3).
pub fn parse(name: &str) -> Option<(&str, usize)> {
    let rest = name.strip_prefix("bowpull/")?;
    let (tex, k) = rest.rsplit_once('/')?;
    let k: usize = k.parse().ok()?;
    (1..=3).contains(&k).then_some((tex, k))
}

/// A bow's shape in the item model's frame (the picture spans -0.5..0.5, y up).
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    /// The string's ends, top (upper right) and bottom.
    pub top: Vec2,
    pub bottom: Vec2,
    /// Unit, across the string towards the limb (the way the arrow flies).
    pub belly: Vec2,
    /// Where the hand holds the limb.
    pub grip: Vec2,
    /// The middle of the string at rest, and how far back each stage draws it.
    pub rest: Vec2,
    pub pull: [f32; 3],
}

impl Shape {
    /// Where the nock (the string's middle) is at draw stage 0..3.
    pub fn nock(&self, stage: usize) -> Vec2 {
        if stage == 0 {
            self.rest
        } else {
            self.rest - self.belly * self.pull[stage.min(3) - 1]
        }
    }

    /// A bow picture without a string we can find: held as if the string ran corner to
    /// corner with the limb bowing out to the upper left (the classic bow's layout).
    pub fn fallback() -> Shape {
        let belly = Vec2::new(-1.0, 1.0).normalize();
        Shape { top: Vec2::new(0.42, 0.42), bottom: Vec2::new(-0.42, -0.42), belly, grip: belly * 0.3, rest: Vec2::ZERO, pull: STAGES.map(|s| s * 0.3) }
    }
}

/// What the picture analysis finds, in pixels. `d` = x + y runs across the string, `e` = x - y
/// along it (pixel centres).
struct Found {
    w: usize,
    d0: f32,
    /// The string's ends along it, and the margin kept at each tip.
    e0: f32,
    e1: f32,
    margin: f32,
    /// +1 when the limb is on the side of larger `d` (down and right).
    belly: f32,
    /// The pale string's own width across it, in `d`.
    width: f32,
    /// How far the nock goes back at a full draw, in `d`.
    max_pull: f32,
    grip: Vec2,
    erase: Vec<bool>,
    string: [u8; 4],
    edge: [u8; 4],
}

fn analyse(px: &[u8], w: usize) -> Option<Found> {
    if w < 8 || px.len() < w * w * 4 {
        return None;
    }
    let at = |x: usize, y: usize| -> [u8; 4] {
        let i = (y * w + x) * 4;
        [px[i], px[i + 1], px[i + 2], px[i + 3]]
    };
    let opaque = |x: usize, y: usize| at(x, y)[3] > 127;
    let pale = |x: usize, y: usize| {
        let c = at(x, y);
        let (hi, lo) = (c[0].max(c[1]).max(c[2]) as i32, c[0].min(c[1]).min(c[2]) as i32);
        c[3] > 127 && hi - lo < 64 && (c[0] as i32 + c[1] as i32 + c[2] as i32) > 3 * 135
    };
    // The string is the diagonal line holding the most pale pixels.
    let mut hist = vec![0usize; 2 * w];
    for y in 0..w {
        for x in 0..w {
            if pale(x, y) {
                hist[x + y] += 1;
            }
        }
    }
    let k = (w / 32).max(1) as i32;
    let window = |d: i32| (d - k..=d + k).filter(|&i| i >= 0 && (i as usize) < hist.len()).map(|i| hist[i as usize]).sum::<usize>();
    let best = (0..2 * w as i32).max_by_key(|&d| (window(d), usize::MAX - (d - w as i32).unsigned_abs() as usize))?;
    if window(best) < w / 2 {
        return None;
    }
    let (mut sum, mut n) = (0.0f32, 0.0f32);
    for d in best - k..=best + k {
        if d >= 0 && (d as usize) < hist.len() {
            sum += d as f32 * hist[d as usize] as f32;
            n += hist[d as usize] as f32;
        }
    }
    let d0 = sum / n;
    let near = |x: usize, y: usize| ((x + y) as f32 - d0).abs() <= k as f32 + 0.5;
    let (mut e0, mut e1, mut dmin, mut dmax) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    let mut string = [0u32; 4];
    let mut strings = 0u32;
    for y in 0..w {
        for x in 0..w {
            if pale(x, y) && near(x, y) {
                let e = x as f32 - y as f32;
                e0 = e0.min(e);
                e1 = e1.max(e);
                dmin = dmin.min((x + y) as f32);
                dmax = dmax.max((x + y) as f32);
                let c = at(x, y);
                for i in 0..4 {
                    string[i] += c[i] as u32;
                }
                strings += 1;
            }
        }
    }
    if e1 - e0 < w as f32 * 0.5 {
        return None;
    }
    let string = string.map(|s| (s / strings.max(1)) as u8);
    let margin = (w as f32 / 14.0).max(1.0);
    // Wipe the string with what sticks to it (its outline, a dark backing), walking out across
    // it through opaque pixels; the tips stay so the limbs keep their ends.
    let reach = w as f32 / 6.0;
    let inside = |x: usize, y: usize| {
        let e = x as f32 - y as f32;
        e >= e0 + margin && e <= e1 - margin && ((x + y) as f32 - d0).abs() <= reach
    };
    let mut erase = vec![false; w * w];
    let mut todo: Vec<(usize, usize)> = Vec::new();
    for y in 0..w {
        for x in 0..w {
            if pale(x, y) && near(x, y) && inside(x, y) {
                erase[y * w + x] = true;
                todo.push((x, y));
            }
        }
    }
    while let Some((x, y)) = todo.pop() {
        let next = [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)];
        for (nx, ny) in next {
            if nx < w && ny < w && !erase[ny * w + nx] && opaque(nx, ny) && inside(nx, ny) {
                erase[ny * w + nx] = true;
                todo.push((nx, ny));
            }
        }
    }
    let (mut edge, mut edges) = ([0u32; 4], 0u32);
    let (mut side, mut sides) = (0.0f32, 0.0f32);
    for y in 0..w {
        for x in 0..w {
            if erase[y * w + x] && !pale(x, y) {
                let c = at(x, y);
                for i in 0..4 {
                    edge[i] += c[i] as u32;
                }
                edges += 1;
            } else if opaque(x, y) && !erase[y * w + x] {
                side += (x + y) as f32 - d0;
                sides += 1.0;
            }
        }
    }
    let edge = if edges > 0 { edge.map(|s| (s / edges) as u8) } else { [string[0] / 3, string[1] / 3, string[2] / 3, 255] };
    let belly = if side >= 0.0 { 1.0 } else { -1.0 };
    let _ = sides;
    // The grip: the middle of the limb across from the string's middle.
    let em = (e0 + e1) * 0.5;
    let (mut g, mut gn) = (Vec2::ZERO, 0.0f32);
    for y in 0..w {
        for x in 0..w {
            let e = x as f32 - y as f32;
            if opaque(x, y) && !erase[y * w + x] && (e - em).abs() <= (w as f32 / 16.0).max(1.0) && ((x + y) as f32 - d0) * belly > 0.0 {
                g += Vec2::new(x as f32, y as f32);
                gn += 1.0;
            }
        }
    }
    let grip = if gn > 0.0 { g / gn } else { Vec2::new((d0 + belly * w as f32 * 0.3 + em) * 0.5, (d0 + belly * w as f32 * 0.3 - em) * 0.5) };
    // A full draw: about half the string's length back, staying inside the picture.
    let room = |p: f32| {
        let d = d0 - belly * p;
        let (x, y) = ((d + em) * 0.5, (d - em) * 0.5);
        let m = w as f32 / 16.0;
        x >= m && y >= m && x <= w as f32 - 1.0 - m && y <= w as f32 - 1.0 - m
    };
    let mut max_pull = (e1 - e0) * 0.36;
    while max_pull > 1.0 && !room(max_pull) {
        max_pull -= 0.5;
    }
    Some(Found { w, d0, e0, e1, margin, belly, width: dmax - dmin + 1.0, max_pull, grip, erase, string, edge })
}

impl Found {
    /// Pixel (x, y) of a point given as (d, e).
    fn xy(d: f32, e: f32) -> Vec2 {
        Vec2::new((d + e) * 0.5, (d - e) * 0.5)
    }

    fn model(&self, p: Vec2) -> Vec2 {
        let w = self.w as f32;
        Vec2::new((p.x + 0.5) / w - 0.5, 0.5 - (p.y + 0.5) / w)
    }

    fn tips(&self) -> (Vec2, Vec2) {
        (Self::xy(self.d0, self.e1 - self.margin), Self::xy(self.d0, self.e0 + self.margin))
    }

    fn nock(&self, stage: usize) -> Vec2 {
        let p = if stage == 0 { 0.0 } else { self.max_pull * STAGES[stage - 1] };
        Self::xy(self.d0 - self.belly * p, (self.e0 + self.e1) * 0.5)
    }

    fn shape(&self) -> Shape {
        let (top, bottom) = self.tips();
        let rest = self.model(self.nock(0));
        // Across the string, towards larger `d` is (1, 1) in pixels: (1, -1) in the model.
        let belly = Vec2::new(1.0, -1.0).normalize() * self.belly;
        let pull = [1, 2, 3].map(|k| (self.model(self.nock(k)) - rest).length());
        Shape { top: self.model(top), bottom: self.model(bottom), belly, grip: self.model(self.grip), rest, pull }
    }
}

/// The shape of the bow picture `px` (`w` × `w` RGBA).
pub fn shape_of(px: &[u8], w: usize) -> Shape {
    analyse(px, w).map(|f| f.shape()).unwrap_or_else(Shape::fallback)
}

/// The bow picture with its string drawn back to `stage` (1..3). Without a string we can
/// find, the picture as it is.
pub fn generate(sprite: Option<&RgbaImage>, stage: usize) -> RgbaImage {
    let Some(img) = sprite else { return RgbaImage::new(16, 16) };
    let mut img = img.clone();
    if img.width() != img.height() {
        return img;
    }
    let w = img.width() as usize;
    let Some(f) = analyse(img.as_raw(), w) else { return img };
    for y in 0..w {
        for x in 0..w {
            if f.erase[y * w + x] {
                img.put_pixel(x as u32, y as u32, Rgba([0, 0, 0, 0]));
            }
        }
    }
    // Bits of the string's outline the wipe left behind, cut off from the limb.
    let mut seen = vec![false; w * w];
    for start in 0..w * w {
        if seen[start] || img.as_raw()[start * 4 + 3] < 128 {
            continue;
        }
        let (mut group, mut todo) = (vec![start], vec![start]);
        seen[start] = true;
        while let Some(i) = todo.pop() {
            let (x, y) = (i % w, i / w);
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (-1, 1), (1, -1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= w as i32 {
                    continue;
                }
                let j = ny as usize * w + nx as usize;
                if !seen[j] && img.as_raw()[j * 4 + 3] >= 128 {
                    seen[j] = true;
                    group.push(j);
                    todo.push(j);
                }
            }
        }
        if group.len() < w * w / 300 {
            for i in group {
                img.put_pixel((i % w) as u32, (i / w) as u32, Rgba([0, 0, 0, 0]));
            }
        }
    }
    // Drawn on the art's own pixel grid (2×2 at 64x) so it stays as chunky as the rest.
    let g = (w / 32).max(1) as f32;
    let (top, bottom) = f.tips();
    let nock = f.nock(stage.clamp(1, 3));
    let r = (f.width / std::f32::consts::SQRT_2 * 0.5).max(0.5 * g);
    let outline = if w >= 32 { g } else { 0.0 };
    let mut paint = |radius: f32, col: [u8; 4], over: bool| {
        let cells = (w as f32 / g) as i32;
        for cy in 0..cells {
            for cx in 0..cells {
                let c = Vec2::new((cx as f32 + 0.5) * g - 0.5, (cy as f32 + 0.5) * g - 0.5);
                let dist = seg_dist(c, top, nock).min(seg_dist(c, bottom, nock));
                if dist > radius {
                    continue;
                }
                for y in (cy as f32 * g) as u32..((cy + 1) as f32 * g) as u32 {
                    for x in (cx as f32 * g) as u32..((cx + 1) as f32 * g) as u32 {
                        if over || img.get_pixel(x, y)[3] < 128 {
                            img.put_pixel(x, y, Rgba(col));
                        }
                    }
                }
            }
        }
    };
    if outline > 0.0 {
        paint(r + outline, f.edge, false);
    }
    paint(r, f.string, true);
    img
}

fn seg_dist(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Every bow's drawn layers and shape, by the layer of its picture.
#[derive(Default)]
pub struct Bows {
    by_layer: HashMap<u16, ([u16; 3], Shape)>,
}

impl Bows {
    /// From the full texture list (`names`, as loaded into `tex`).
    pub fn new(names: &[String], tex: &TextureSet) -> Bows {
        let mut by_layer: HashMap<u16, ([u16; 3], Shape)> = HashMap::new();
        for (i, n) in names.iter().enumerate() {
            let Some((base, k)) = parse(n) else { continue };
            let Some(b) = names.iter().position(|x| x == base) else { continue };
            let e = by_layer.entry(b as u16).or_insert_with(|| {
                let shape = tex.layers.get(b).map(|px| shape_of(px, tex.size as usize)).unwrap_or_else(Shape::fallback);
                ([b as u16; 3], shape)
            });
            e.0[k - 1] = i as u16;
        }
        Bows { by_layer }
    }

    /// The drawn layers and the shape of the bow whose picture is `layer`.
    pub fn get(&self, layer: u16) -> Option<&([u16; 3], Shape)> {
        self.by_layer.get(&layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Option<RgbaImage> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/vanilla/textures/item").join(format!("{name}.png"));
        image::open(p).ok().map(|i| i.to_rgba8())
    }

    #[test]
    fn finds_the_string_of_our_bows() {
        for name in ["bow", "copper_bow", "silver_bow", "platinum_bow", "umbral_bow", "sanguinite_bow", "magmarite_bow", "cobalt_bow", "mythril_bow", "adamantite_bow", "radiant_bow", "verdite_bow", "lumite_bow"] {
            let Some(img) = load(name) else { continue };
            let f = analyse(img.as_raw(), img.width() as usize).unwrap_or_else(|| panic!("{name}: no string"));
            let s = f.shape();
            // The string runs from the upper right to the lower left.
            assert!(s.top.x > s.bottom.x + 0.3 && s.top.y > s.bottom.y + 0.3, "{name}: {s:?}");
            // The grip is on the limb's side, the nock goes back the other way.
            assert!((s.grip - s.rest).dot(s.belly) > 0.1, "{name}: {s:?}");
            assert!(s.pull[2] > s.pull[0] && s.pull[2] > 0.12, "{name}: {s:?}");
            if std::env::var("MC_BOWDUMP").is_ok() {
                for k in 1..=3 {
                    generate(Some(&img), k).save(format!("/tmp/{name}_pull{k}.png")).unwrap();
                }
            }
        }
    }

    #[test]
    fn the_plain_bow_bows_out_to_the_upper_left() {
        let Some(img) = load("bow") else { return };
        let s = shape_of(img.as_raw(), img.width() as usize);
        assert!(s.belly.x < 0.0 && s.belly.y > 0.0, "{s:?}");
    }
}
