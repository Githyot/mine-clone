//! Gradient (Perlin-style) noise in 2D and 3D, plus fractal helpers.
//! Our own implementation so world generation has no external dependency and is fully deterministic.

use crate::rng::Rng;

#[derive(Clone)]
pub struct Perlin {
    perm: [u8; 512],
}

const GRAD3: [[f32; 3]; 12] = [
    [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0], [1.0, -1.0, 0.0], [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0], [1.0, 0.0, -1.0], [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0], [0.0, -1.0, 1.0], [0.0, 1.0, -1.0], [0.0, -1.0, -1.0],
];

#[inline(always)]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Perlin {
    pub fn new(seed: u64) -> Self {
        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        let mut rng = Rng::new(seed);
        for i in (1..256).rev() {
            let j = rng.below(i as u32 + 1) as usize;
            p.swap(i, j);
        }
        let mut perm = [0u8; 512];
        for i in 0..512 {
            perm[i] = p[i & 255];
        }
        Perlin { perm }
    }

    #[inline(always)]
    fn g2(&self, hash: u8, x: f32, y: f32) -> f32 {
        let g = GRAD3[(hash % 12) as usize];
        g[0] * x + g[1] * y
    }

    #[inline(always)]
    fn g3(&self, hash: u8, x: f32, y: f32, z: f32) -> f32 {
        let g = GRAD3[(hash % 12) as usize];
        g[0] * x + g[1] * y + g[2] * z
    }

    /// 2D noise, roughly in -1..1.
    pub fn noise2(&self, x: f32, y: f32) -> f32 {
        let xf = x.floor();
        let yf = y.floor();
        let xi = (xf as i32 & 255) as usize;
        let yi = (yf as i32 & 255) as usize;
        let x = x - xf;
        let y = y - yf;
        let u = fade(x);
        let v = fade(y);
        let p = &self.perm;
        let aa = p[p[xi] as usize + yi];
        let ab = p[p[xi] as usize + yi + 1];
        let ba = p[p[xi + 1] as usize + yi];
        let bb = p[p[xi + 1] as usize + yi + 1];
        let r = lerp(
            lerp(self.g2(aa, x, y), self.g2(ba, x - 1.0, y), u),
            lerp(self.g2(ab, x, y - 1.0), self.g2(bb, x - 1.0, y - 1.0), u),
            v,
        );
        r * 1.4
    }

    /// 3D noise, roughly in -1..1.
    pub fn noise3(&self, x: f32, y: f32, z: f32) -> f32 {
        let xf = x.floor();
        let yf = y.floor();
        let zf = z.floor();
        let xi = (xf as i32 & 255) as usize;
        let yi = (yf as i32 & 255) as usize;
        let zi = (zf as i32 & 255) as usize;
        let x = x - xf;
        let y = y - yf;
        let z = z - zf;
        let (u, v, w) = (fade(x), fade(y), fade(z));
        let p = &self.perm;
        let a = p[xi] as usize + yi;
        let aa = p[a] as usize + zi;
        let ab = p[a + 1] as usize + zi;
        let b = p[xi + 1] as usize + yi;
        let ba = p[b] as usize + zi;
        let bb = p[b + 1] as usize + zi;
        lerp(
            lerp(
                lerp(self.g3(p[aa], x, y, z), self.g3(p[ba], x - 1.0, y, z), u),
                lerp(self.g3(p[ab], x, y - 1.0, z), self.g3(p[bb], x - 1.0, y - 1.0, z), u),
                v,
            ),
            lerp(
                lerp(self.g3(p[aa + 1], x, y, z - 1.0), self.g3(p[ba + 1], x - 1.0, y, z - 1.0), u),
                lerp(
                    self.g3(p[ab + 1], x, y - 1.0, z - 1.0),
                    self.g3(p[bb + 1], x - 1.0, y - 1.0, z - 1.0),
                    u,
                ),
                v,
            ),
            w,
        )
    }

    /// Fractal Brownian motion: several octaves added together.
    pub fn fbm2(&self, x: f32, y: f32, octaves: u32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut norm = 0.0;
        for i in 0..octaves {
            // Offset each octave so they don't line up at the origin.
            let o = i as f32 * 17.31;
            sum += self.noise2(x * freq + o, y * freq - o) * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }

    pub fn fbm3(&self, x: f32, y: f32, z: f32, octaves: u32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut norm = 0.0;
        for i in 0..octaves {
            let o = i as f32 * 11.7;
            sum += self.noise3(x * freq + o, y * freq, z * freq - o) * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }

    /// Ridged noise: sharp crests, good for mountain ranges.
    pub fn ridged2(&self, x: f32, y: f32, octaves: u32) -> f32 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut norm = 0.0;
        for i in 0..octaves {
            let o = i as f32 * 23.1;
            let n = 1.0 - self.noise2(x * freq + o, y * freq + o).abs();
            sum += n * n * amp;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_and_bounded() {
        let a = Perlin::new(42);
        let b = Perlin::new(42);
        for i in 0..1000 {
            let x = i as f32 * 0.137;
            let y = i as f32 * 0.071;
            let v = a.noise3(x, y, x * 0.5);
            assert_eq!(v, b.noise3(x, y, x * 0.5));
            assert!(v.abs() <= 1.5, "{v}");
            assert!(a.noise2(x, y).abs() <= 1.5);
        }
    }
}
