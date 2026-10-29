//! Pictures for the bodies of plants (voxel/src/flora.rs): petals, leaves, stems, mushroom caps,
//! sapling foliage. Each is painted at 16 × 16 (big chunky pixels, a dark rim and a light vein,
//! like the rest of our art) in colours taken from the plant's own texture, so a resource pack
//! that paints its poppy orange gets orange petals too.
//!
//! The layers are named "flora/<plant texture>/<part>" and come right after the pack model
//! pictures; `plan` lists them and the plants that use them.

use image::{Rgba, RgbaImage};
use voxel::block::{BlockId, BlockRegistry, Shape};
use voxel::flora::{self, FloraQuad, Part};

/// The flora layer names (to append to the texture list from `first_layer` on) and every plant
/// with a body: (block, quads, keeps its facing).
pub fn plan(reg: &BlockRegistry, first_layer: usize) -> (Vec<String>, Vec<(BlockId, Vec<FloraQuad>, bool)>) {
    let mut names: Vec<String> = Vec::new();
    let mut plants = Vec::new();
    for (id, b) in reg.blocks.iter().enumerate() {
        if b.shape != Shape::Cross || b.waterlogged {
            continue;
        }
        let Some(kind) = flora::kind_of(&b.name) else { continue };
        let sprite = b.faces[0];
        let Some(tex) = reg.textures.get(sprite as usize) else { continue };
        let raw = flora::build(kind, b.light > 0);
        for p in flora::parts(&raw) {
            let n = format!("flora/{tex}/{}", p.key());
            if p != Part::Sprite && !names.contains(&n) {
                names.push(n);
            }
        }
        let layer = |p: Part| -> Option<u16> {
            if p == Part::Sprite {
                return Some(sprite);
            }
            let n = format!("flora/{tex}/{}", p.key());
            let i = names.iter().position(|x| *x == n)?;
            Some((first_layer + i) as u16)
        };
        let quads = flora::finish(&raw, b.tint, &layer);
        plants.push((id as BlockId, quads, kind.fixed()));
    }
    (names, plants)
}

/// "flora/<texture>/<part>" as (texture, part).
pub fn parse(name: &str) -> Option<(&str, Part)> {
    let rest = name.strip_prefix("flora/")?;
    let (tex, part) = rest.rsplit_once('/')?;
    Some((tex, Part::from_key(part)?))
}

type Rgb = [f32; 3];

/// Dark, middle and light tones of one colour.
#[derive(Clone, Copy, Debug)]
struct Tone {
    dark: Rgb,
    mid: Rgb,
    light: Rgb,
}

impl Tone {
    fn of(c: Rgb) -> Tone {
        Tone { dark: c.map(|v| v * 0.62), mid: c, light: c.map(|v| (v * 1.22 + 0.06).min(1.0)) }
    }
}

/// The colours of a plant's picture: its bloom (petals, cap), an accent (the flower's eye,
/// mushroom spots), its green and its wood.
#[derive(Clone, Copy, Debug)]
struct Palette {
    bloom: Tone,
    accent: Tone,
    green: Tone,
    wood: Tone,
}

fn hsv(c: Rgb) -> (f32, f32, f32) {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    let d = max - min;
    let h = if d < 1e-5 {
        0.0
    } else if max == c[0] {
        60.0 * (((c[1] - c[2]) / d).rem_euclid(6.0))
    } else if max == c[1] {
        60.0 * ((c[2] - c[0]) / d + 2.0)
    } else {
        60.0 * ((c[0] - c[1]) / d + 4.0)
    };
    (h, if max > 0.0 { d / max } else { 0.0 }, max)
}

fn luma(c: Rgb) -> f32 {
    c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11
}

/// The tones of a set of pixels: the average of their darkest, middle and brightest thirds.
fn tone(mut px: Vec<Rgb>) -> Option<Tone> {
    if px.is_empty() {
        return None;
    }
    px.sort_by(|a, b| luma(*a).total_cmp(&luma(*b)));
    let n = px.len();
    let avg = |s: &[Rgb]| {
        let k = s.len().max(1) as f32;
        let t = s.iter().fold([0.0; 3], |a, c| [a[0] + c[0], a[1] + c[1], a[2] + c[2]]);
        t.map(|v| v / k)
    };
    let third = (n / 3).max(1);
    let dark = avg(&px[..third.min(n)]);
    let mid = avg(&px[n / 3..(n - n / 3).max(n / 3 + 1).min(n)]);
    let light = avg(&px[n.saturating_sub(third)..]);
    // Keep some contrast even in a flat picture.
    let dark = if luma(mid) - luma(dark) < 0.08 { mid.map(|v| v * 0.7) } else { dark };
    let light = if luma(light) - luma(mid) < 0.06 { mid.map(|v| (v * 1.2 + 0.05).min(1.0)) } else { light };
    Some(Tone { dark, mid, light })
}

fn palette(img: &RgbaImage) -> Palette {
    let mut green = Vec::new();
    let mut wood = Vec::new();
    // Hue bins of 30° for coloured pixels, plus white (12) and near black (13).
    let mut bins: Vec<Vec<Rgb>> = vec![Vec::new(); 14];
    let mut all = Vec::new();
    for p in img.pixels() {
        if p.0[3] < 128 {
            continue;
        }
        let c = [p.0[0] as f32 / 255.0, p.0[1] as f32 / 255.0, p.0[2] as f32 / 255.0];
        all.push(c);
        let (h, s, v) = hsv(c);
        if (65.0..170.0).contains(&h) && s > 0.2 && v > 0.12 {
            green.push(c);
        } else if (10.0..48.0).contains(&h) && (0.2..0.75).contains(&s) && v < 0.55 {
            wood.push(c);
            bins[(h / 30.0) as usize % 12].push(c);
        } else if s < 0.16 && v > 0.62 {
            bins[12].push(c);
        } else if v < 0.16 {
            bins[13].push(c);
        } else if s >= 0.16 {
            bins[(h / 30.0) as usize % 12].push(c);
        }
    }
    // A grey picture takes the biome colour (grass, ferns): its greys are its green.
    let grey = !all.is_empty() && all.iter().all(|&c| hsv(c).1 < 0.12);
    let mut order: Vec<usize> = (0..14).filter(|&i| !bins[i].is_empty()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(bins[i].len()));
    let main = order.first().copied();
    // The accent: the next most common colour that is clearly another one.
    let far = |a: usize, b: usize| a >= 12 || b >= 12 || { let d = (a as i32 - b as i32).rem_euclid(12); d.min(12 - d) >= 2 };
    let accent = main.and_then(|m| order.iter().copied().find(|&i| i != m && far(i, m) && bins[i].len() * 25 >= bins[m].len()));
    let fallback_green = Tone::of([0.33, 0.6, 0.22]);
    let green = if grey { tone(all.clone()).unwrap_or(fallback_green) } else { tone(green).unwrap_or(fallback_green) };
    let bloom = main.and_then(|m| tone(bins[m].clone())).unwrap_or(Tone::of([0.85, 0.2, 0.2]));
    let accent = accent.and_then(|a| tone(bins[a].clone())).unwrap_or(Tone { dark: bloom.dark.map(|v| v * 0.5), mid: bloom.dark, light: bloom.mid });
    let wood = tone(wood).unwrap_or(Tone::of([0.42, 0.3, 0.18]));
    Palette { bloom, accent, green, wood }
}

/// A steady per-pixel number in 0..1.
fn noise(x: u32, y: u32, salt: u32) -> f32 {
    let mut h = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ salt.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xFFFF) as f32 / 65535.0
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Whether the shape covers the pixel: `u` -0.5..0.5 across, `v` 0 at the base .. 1 at the tip.
fn shape(part: Part, u: f32, v: f32) -> bool {
    let a = u.abs();
    match part {
        Part::PetalRound => {
            let w = 0.5 * (1.0 - ((v - 0.56) / 0.46).powi(2)).max(0.0).sqrt();
            a < w.min(0.1 + v * 0.75)
        }
        Part::PetalPointed => {
            let w = if v < 0.45 { 0.12 + v * 0.8 } else { 0.48 * ((1.0 - v) / 0.55).powf(0.8) };
            a < w
        }
        Part::PetalCup => {
            let w = if v < 0.75 { 0.36 + v * 0.16 } else { 0.48 * (1.0 - ((v - 0.75) / 0.27).powi(2)).max(0.0).sqrt() };
            a < w
        }
        Part::PetalFrilly => {
            // Broad and fan-shaped, its rounded edge a little wavy.
            let w = (0.2 + v * 0.5).min(0.5);
            let top = 0.86 + 0.12 * (1.0 - (2.0 * u).powi(2)) - 0.05 * ((u * 14.0).cos() * 0.5 + 0.5);
            a < w && v < top
        }
        Part::PetalNarrow => {
            let w = if v < 0.75 { 0.32 } else { 0.32 * (1.0 - ((v - 0.75) / 0.27).powi(2)).max(0.0).sqrt() };
            a < w.max(0.0)
        }
        Part::PetalNotched => {
            let w = 0.14 + v * 0.36;
            a < w && !(v > 0.82 && ((u * 16.0 + 8.0) as i32) % 2 != 0)
        }
        Part::Leaf => {
            let w = 0.46 * (1.0 - ((v - 0.45) / 0.55).powi(2)).max(0.0).sqrt();
            a < w.min(0.08 + v * 1.2)
        }
        Part::LeafLong | Part::Blade => {
            let w = 0.36 * (1.0 - v).powf(0.7) + 0.04;
            a < w
        }
        Part::Frond => {
            let w = 0.46 * (1.0 - v).powf(0.8);
            a < 0.07 || (a < w && (v * 7.0 + a * 2.5).fract() < 0.62)
        }
        _ => true,
    }
}

/// The picture of one part, 16 × 16.
fn paint(part: Part, pal: &Palette, salt: u32) -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    let inside = |x: i32, y: i32| (0..16).contains(&x) && (0..16).contains(&y) && shape(part, (x as f32 + 0.5) / 16.0 - 0.5, 1.0 - (y as f32 + 0.5) / 16.0);
    let (main, petal) = match part {
        Part::Leaf | Part::LeafLong | Part::Blade | Part::Frond | Part::Stem | Part::HeadBack | Part::Foliage | Part::Needles => (pal.green, false),
        Part::Center | Part::Seeds => (pal.accent, false),
        Part::Wood => (pal.wood, false),
        Part::MushStem | Part::CapUnder => (Tone::of(mix(pal.accent.light, [0.92, 0.88, 0.8], 0.5)), false),
        Part::Bell => (if luma(pal.bloom.mid) > 0.55 { pal.bloom } else { Tone::of([0.95, 0.95, 0.9]) }, true),
        _ => (pal.bloom, true),
    };
    for y in 0..16i32 {
        for x in 0..16i32 {
            if !inside(x, y) {
                continue;
            }
            let (u, v) = ((x as f32 + 0.5) / 16.0 - 0.5, 1.0 - (y as f32 + 0.5) / 16.0);
            let n = noise(x as u32, y as u32, salt);
            let rim = !inside(x - 1, y) || !inside(x + 1, y) || !inside(x, y - 1) || !inside(x, y + 1);
            let mut c = match part {
                Part::Stem | Part::Wood | Part::MushStem => {
                    // Fibres along the length: dark edges, a light streak.
                    let col = [main.dark, main.mid, main.light, main.mid, main.mid, main.dark][(x as usize + (y as usize / 5)) % 6];
                    mix(col, main.mid, if n > 0.7 { 0.5 } else { 0.0 })
                }
                Part::Center | Part::Seeds => {
                    let r = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt();
                    // Seeds in crossing spirals: dots along two families of curves.
                    let ring = if part == Part::Seeds { ((x * 3 + y * 5) % 7 == 0 || (x * 5 + y * 2) % 11 == 0) as u8 as f32 } else { (n > 0.72) as u8 as f32 };
                    let base = if r > 6.0 { main.dark } else { main.mid };
                    mix(base, main.light, ring * 0.8)
                }
                Part::Floret => {
                    // Tiny star flowers packed into a ball.
                    let star = (x % 4 == 1 && y % 4 == 1) || (x % 4 == 3 && y % 4 == 3);
                    if star { main.light } else if n < 0.3 { main.dark } else { main.mid }
                }
                Part::Cap => {
                    // Smooth and glossy, with big pale spots when the plant has them.
                    let spot = pal.accent;
                    // Only bright caps (fly agaric) are spotted; brown ones stay plain.
                    let spotted = luma(spot.mid) > luma(main.mid) + 0.15 && hsv(main.mid).1 > 0.55;
                    let (cx, cy) = (x.rem_euclid(6), y.rem_euclid(6));
                    let cell = noise((x / 6) as u32, (y / 6) as u32, salt) < 0.55;
                    let in_spot = cell && (1..=3).contains(&cx) && (1..=3).contains(&cy) && !(cx != 2 && cy != 2 && (cx + cy) % 2 == 0);
                    if spotted && in_spot {
                        if cy == 3 { spot.mid } else { spot.light }
                    } else if (x + y) % 7 == 0 && n > 0.5 {
                        mix(main.mid, main.light, 0.25)
                    } else {
                        main.mid
                    }
                }
                Part::CapUnder => {
                    let gill = (x + y) % 3 == 0 || (x - y).rem_euclid(3) == 0;
                    if gill { main.dark } else { main.mid }
                }
                Part::HeadBack => {
                    if (x / 2 + y / 2) % 3 == 0 { main.dark } else { main.mid }
                }
                Part::Foliage | Part::Needles => {
                    // Clumps of leaves four pixels wide: a light top-left edge, a dark lower
                    // right one, so the box reads as a bundle of leaves.
                    let (cx, cy) = (x.rem_euclid(4), (y + if (x / 4) % 2 == 0 { 0 } else { 2 }).rem_euclid(4));
                    let tone = noise((x / 4) as u32, ((y + if (x / 4) % 2 == 0 { 0 } else { 2 }) / 4) as u32, salt);
                    let body = if tone < 0.3 { mix(main.mid, main.dark, 0.35) } else if tone > 0.75 { mix(main.mid, main.light, 0.3) } else { main.mid };
                    if part == Part::Needles && x % 2 == 0 && cy != 0 {
                        mix(body, main.light, 0.25)
                    } else if cx == 0 && cy == 0 {
                        main.light
                    } else if cx == 3 || cy == 3 {
                        mix(body, main.dark, 0.6)
                    } else {
                        body
                    }
                }
                _ => {
                    // Petals and leaves: darker at the base, a light vein, a dark rim.
                    let base_shadow = (1.0 - v * 2.2).clamp(0.0, 1.0) * 0.55;
                    let mut c = mix(main.mid, main.dark, base_shadow);
                    if u.abs() < 0.07 && v > 0.12 && v < 0.88 {
                        c = if petal { mix(c, main.light, 0.55) } else { main.light };
                    } else if petal && v > 0.6 && u.abs() > 0.12 && u.abs() < 0.26 {
                        // A soft sheen toward the tip on each side of the vein.
                        c = mix(c, main.light, 0.35);
                    }
                    c
                }
            };
            if rim && !matches!(part, Part::Blade | Part::Frond | Part::Stem | Part::Wood | Part::MushStem | Part::Cap | Part::CapUnder | Part::Foliage | Part::Needles | Part::HeadBack | Part::Floret | Part::Seeds) {
                c = mix(c, main.dark, 0.7);
            }
            let jitter = 0.97 + n * 0.05;
            let c = c.map(|v| (v * jitter).clamp(0.0, 1.0));
            img.put_pixel(x as u32, y as u32, Rgba([(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255]));
        }
    }
    img
}

/// The picture for a flora layer, from the plant's own texture (None: not found, a plain
/// palette is used).
pub fn generate(part: Part, sprite: Option<&RgbaImage>, tex_name: &str) -> RgbaImage {
    let pal = match sprite {
        Some(img) => palette(img),
        None => palette(&RgbaImage::new(1, 1)),
    };
    let salt = tex_name.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    paint(part, &pal, salt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite(cols: &[([u8; 3], u32)]) -> RgbaImage {
        let mut img = RgbaImage::new(16, 16);
        let mut k = 0u32;
        for &(c, n) in cols {
            for _ in 0..n {
                img.put_pixel(k % 16, k / 16, Rgba([c[0], c[1], c[2], 255]));
                k += 1;
            }
        }
        img
    }

    #[test]
    fn petals_take_the_flowers_colour_and_leaves_its_green() {
        // A poppy: mostly red, some green, a black eye.
        let img = sprite(&[([200, 30, 30], 40), ([150, 20, 25], 20), ([60, 140, 50], 30), ([20, 15, 15], 10)]);
        let pal = palette(&img);
        assert!(pal.bloom.mid[0] > pal.bloom.mid[1] * 2.0, "{:?}", pal.bloom);
        assert!(pal.green.mid[1] > pal.green.mid[0], "{:?}", pal.green);
        let petal = generate(Part::PetalFrilly, Some(&img), "poppy");
        let red = petal.pixels().filter(|p| p.0[3] > 0 && p.0[0] > p.0[1] * 2).count();
        assert!(red > 60, "{red}");
        // The petal has a cut-out shape; a stem fills its picture.
        assert!(petal.pixels().any(|p| p.0[3] == 0));
        assert!(generate(Part::Stem, Some(&img), "poppy").pixels().all(|p| p.0[3] == 255));
    }

    #[test]
    fn red_mushroom_caps_get_white_spots() {
        let img = sprite(&[([190, 30, 30], 60), ([240, 240, 235], 20), ([220, 210, 190], 10)]);
        let cap = generate(Part::Cap, Some(&img), "red_mushroom");
        let white = cap.pixels().filter(|p| p.0[0] > 200 && p.0[1] > 200).count();
        assert!(white > 3, "{white}");
    }

    #[test]
    fn names_round_trip() {
        assert_eq!(parse("flora/poppy/petal_frilly"), Some(("poppy", Part::PetalFrilly)));
        assert_eq!(parse("poppy"), None);
    }
}
