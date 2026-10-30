//! Height maps for block depth (parallax, terrain.wgsl `parallax()`), made from the textures
//! themselves when they load, so they work for any resource pack.
//!
//! A texel's height used to be read straight from its brightness in the shader, which made
//! every slightly darker speck a hole: the relief looked random and the real gaps (mortar,
//! the cracks between cobbles, the joints between planks, wood grain) were barely deeper than
//! the noise. Here the gaps are found first (texels clearly darker or greyer than the area
//! around them) and the surface is shaped from them: each gap cuts a groove as deep as the
//! gap is strong, with sloping shoulders, so cobbles come out as rounded stones, planks as
//! boards with bevelled joints, and grain lines as shallow furrows along the grain.

/// Heights are made at most this many texels across (the shader walks at most 32 columns per
/// block; finer textures share heights in 2x2 or 4x4 groups).
pub const MAX_SIZE: u32 = 32;

/// Side of the height maps for textures `size` texels across.
pub fn map_size(size: u32) -> u32 {
    size.min(MAX_SIZE).max(1)
}

/// The height map of one RGBA texture (`size` × `size`): `map_size(size)`² bytes, 255 = the
/// face itself, 0 = the deepest groove.
pub fn height_map(rgba: &[u8], size: u32) -> Vec<u8> {
    let n = map_size(size) as usize;
    let step = (size as usize / n).max(1);
    // How "raised" each texel looks: bright and colourful stays up, dark and grey sinks
    // (pale mortar between red bricks, moss and ore bits standing out of stone).
    let mut s = vec![0f32; n * n];
    let mut solid = vec![false; n * n];
    for y in 0..n {
        for x in 0..n {
            let (mut v, mut a) = (0.0, 0.0);
            for dy in 0..step {
                for dx in 0..step {
                    let i = ((y * step + dy) * size as usize + x * step + dx) * 4;
                    if i + 3 >= rgba.len() {
                        continue;
                    }
                    let [r, g, b, al] =
                        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]].map(|c| c as f32 / 255.0);
                    let hi = r.max(g).max(b);
                    let lo = r.min(g).min(b);
                    v += (0.3 * r + 0.59 * g + 0.11 * b) + 0.9 * (hi - lo) / hi.max(0.2);
                    a += al;
                }
            }
            let k = (step * step) as f32;
            s[y * n + x] = v / k;
            solid[y * n + x] = a / k >= 0.5;
        }
    }
    let at = |v: &[f32], x: isize, y: isize| {
        v[(y.rem_euclid(n as isize) as usize) * n + x.rem_euclid(n as isize) as usize]
    };
    // The area around each texel (textures tile, so it wraps), and how much the picture varies
    // overall: grooves are measured against that, so soft packs get grooves too, while plain
    // textures (wool, smooth stone) don't turn their faint noise into holes.
    let r = (n / 8).max(2) as isize;
    let mut mean = vec![0f32; n * n];
    for y in 0..n as isize {
        for x in 0..n as isize {
            let mut t = 0.0;
            for dy in -r..=r {
                for dx in -r..=r {
                    t += at(&s, x + dx, y + dy);
                }
            }
            mean[y as usize * n + x as usize] = t / ((2 * r + 1) * (2 * r + 1)) as f32;
        }
    }
    let avg = s.iter().sum::<f32>() / s.len() as f32;
    let sigma = (s.iter().map(|v| (v - avg) * (v - avg)).sum::<f32>() / s.len() as f32)
        .sqrt()
        .max(0.07);
    // Gap strength: 0 (surface) to 1 (a clear gap). A gap is a valley: darker than the texels
    // on both sides of it along some direction (across a joint, across a grain line), at most
    // `kmax` texels away. A wide dark patch is no valley (its middle has dark texels on both
    // sides), so blotches in the wood, the stone or the grass stay up instead of turning into
    // pits. A lone dark speck is a valley in
    // every direction: a small pit, kept shallower than a line.
    let kmax = (n / 8).max(2) as isize;
    let dirs = [(1isize, 0isize), (0, 1), (1, 1), (1, -1)];
    let gap: Vec<f32> = (0..n * n)
        .map(|i| {
            let (x, y) = ((i % n) as isize, (i / n) as isize);
            let mut along = [0f32; 4];
            for (j, (dx, dy)) in dirs.iter().enumerate() {
                for k in 1..=kmax {
                    let side = at(&s, x - dx * k, y - dy * k).min(at(&s, x + dx * k, y + dy * k));
                    along[j] = along[j].max(side - s[i]);
                }
            }
            let best = along.iter().cloned().fold(0.0, f32::max);
            let least = along.iter().cloned().fold(f32::MAX, f32::min);
            // Also darker than the area around: texture noise between two bright specks is
            // no groove.
            let darker = (0.75 * (mean[i] - s[i]) + 0.25 * (avg - s[i])) / sigma;
            let line = smoothstep(0.5, 1.6, best / sigma)
                * smoothstep(0.1, 0.9, darker)
                * (1.0 - 0.75 * smoothstep(0.3, 1.2, least / sigma));
            line
        })
        .collect();
    // Each gap cuts a groove: full depth on the gap, rising to the surface over `reach` texels
    // (rounded shoulders, so stones and boards look bevelled, not stamped).
    let reach = (n as f32 / 14.0).max(1.4);
    let w = reach.ceil() as isize;
    let mut out = vec![255u8; n * n];
    for y in 0..n as isize {
        for x in 0..n as isize {
            let i = y as usize * n + x as usize;
            if !solid[i] {
                continue;
            }
            let mut groove = 0f32;
            for dy in -w..=w {
                for dx in -w..=w {
                    let g = at(&gap, x + dx, y + dy);
                    if g <= groove {
                        continue;
                    }
                    let d = ((dx * dx + dy * dy) as f32).sqrt() / reach;
                    if d < 1.0 {
                        let f = 1.0 - d * d;
                        groove = groove.max(g * f * f.sqrt());
                    }
                }
            }
            // A little of the texel's own tone on top, so faces aren't dead flat.
            let fine = smoothstep(-2.0, 2.0, (s[i] - mean[i]) / sigma);
            let h = (1.0 - groove) * (0.94 + 0.06 * fine);
            out[i] = (h.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    out
}

/// Mip chain of a height map (the average of each 2x2), down to 1x1.
pub fn mip_chain(base: &[u8], size: u32) -> Vec<Vec<u8>> {
    let mut out = vec![base.to_vec()];
    let mut s = size as usize;
    while s > 1 {
        let prev = out.last().unwrap();
        let ns = s / 2;
        let mut next = vec![0u8; ns * ns];
        for y in 0..ns {
            for x in 0..ns {
                let t: u32 = [(0, 0), (1, 0), (0, 1), (1, 1)]
                    .iter()
                    .map(|(dx, dy)| prev[(y * 2 + dy) * s + x * 2 + dx] as u32)
                    .sum();
                next[y * ns + x] = (t / 4) as u8;
            }
        }
        out.push(next);
        s = ns;
    }
    out
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Grey bricks with dark joints: the joints sink, the middle of a brick stays on top.
    #[test]
    fn joints_sink_and_faces_stay_up() {
        let size = 16u32;
        let mut px = vec![0u8; (size * size * 4) as usize];
        for y in 0..size {
            for x in 0..size {
                let joint = y % 8 == 0 || x % 8 == 0;
                let v = if joint {
                    40
                } else {
                    150 + ((x * 7 + y * 13) % 9) as u8
                };
                let i = ((y * size + x) * 4) as usize;
                px[i..i + 4].copy_from_slice(&[v, v, v, 255]);
            }
        }
        let h = height_map(&px, size);
        assert!(h[0] < 60, "joint {}", h[0]);
        assert!(h[4 * 16 + 4] > 200, "face {}", h[4 * 16 + 4]);
        // Next to a joint it slopes.
        assert!(h[4 * 16 + 1] < h[4 * 16 + 4]);
    }

    /// A plain texture with faint noise stays flat.
    #[test]
    fn plain_texture_stays_flat() {
        let size = 16u32;
        let mut px = vec![0u8; (size * size * 4) as usize];
        for i in 0..(size * size) as usize {
            let v = 120 + (i * 7 % 5) as u8;
            px[i * 4..i * 4 + 4].copy_from_slice(&[v, v, v, 255]);
        }
        assert!(height_map(&px, size).iter().all(|&h| h > 200));
    }
}
