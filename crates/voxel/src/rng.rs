//! Tiny deterministic random number helpers (no external dependency).

/// SplitMix64: fast, good-quality 64-bit generator, used for seeding.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    /// Seed from a world seed plus coordinates, so the same place always gets the same numbers.
    pub fn for_pos(seed: u64, x: i32, z: i32, salt: u64) -> Self {
        let h = hash3(seed ^ salt.wrapping_mul(0xD6E8_FEB8_6659_FD93), x as i64, z as i64, 0);
        Rng::new(h)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform integer in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next_u64() >> 32) * n as u64 >> 32) as u32
    }

    /// Uniform integer in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1) as u32) as i32
    }

    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }
}

pub fn hash3(seed: u64, x: i64, y: i64, z: i64) -> u64 {
    let mut h = seed;
    for v in [x, y, z] {
        h ^= (v as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h = h.rotate_left(27).wrapping_mul(0x94D0_49BB_1331_11EB);
    }
    h ^ (h >> 29)
}
