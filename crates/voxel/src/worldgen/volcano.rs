//! Volcanoes (Lote 14, our own content): rare tall cones in the Overworld, made to be climbed.
//!
//! * The cone rises from whatever is there (on land, or straight out of the sea as an island),
//!   steep near the top and gentle at its foot, its sides cut by gullies, old lava flows of
//!   obsidian running down them and steam vents (fumaroles) that throw a climber upwards.
//! * The crater at the top holds a lake of lava; the Magma Titan sleeps there (game side).
//! * Lava tubes open on the flank and climb gently into the mountain: long round tunnels with a
//!   glowing floor, ending in a chamber of magma under the crater.
//! * Around the foot lies a ring of Ash Fields: grey ash, burnt trunks, pumice and ember blooms.
//!
//! Each volcano lives in its own cell of a coarse grid (one cell in a few has one), so finding
//! the one at a column is a hash and a little arithmetic. `Generator::volcano_at` gives the game
//! the cone (where the crater is, for eruptions and the boss).

use super::{Biome, Column, Generator, SEA_LEVEL};
use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos, CHUNK_HEIGHT};
use crate::rng::{hash3, Rng};

/// The grid volcanoes are spread on, and how often a cell has one.
const CELL: i32 = 1700;
const CHANCE: f32 = 0.42;
/// No volcano this close to the middle of the world (where new players start).
const SPAWN_CLEAR: f32 = 900.0;
/// How far the Ash Fields reach past the cone, as a share of its radius.
pub const ASH_REACH: f32 = 1.38;
/// Where the cone's foot meets ordinary ground.
const FOOT: f32 = 58.0;

/// One volcano.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volcano {
    pub cx: f32,
    pub cz: f32,
    /// The cone's radius at its foot.
    pub radius: f32,
    /// The crater's radius at the rim.
    pub crater: f32,
    /// Height of the crater rim, and of the lava lake in it.
    pub rim: i32,
    pub lava: i32,
    /// The crater floor, under the lava.
    pub floor: i32,
    /// Directions (radians) of the lava tubes.
    pub tubes: [f32; 3],
    seed: u64,
}

impl Volcano {
    /// Height of the bare cone (before gullies) at a distance from the middle.
    pub fn cone(&self, d: f32) -> f32 {
        let u = ((d - self.crater) / (self.radius - self.crater)).clamp(0.0, 1.0);
        if d <= self.radius {
            FOOT + (self.rim as f32 - FOOT) * (1.0 - u).powf(2.2)
        } else {
            // A skirt down into the sea floor, so an island cone does not stand on a step.
            FOOT - (d - self.radius) * 0.35
        }
    }

    /// Where the lava tubes open on the flank, and where they end under the crater (in a
    /// chamber of magma), as distances from the middle.
    pub fn tube_ends(&self) -> (f32, f32) {
        (self.radius * 0.6, self.crater + 4.0)
    }

    /// The height of a lava tube's middle at a distance from the volcano's middle: a walkable
    /// slope up from its mouth into the mountain.
    pub fn tube_y(&self, d: f32) -> f32 {
        let (mouth, end) = self.tube_ends();
        self.cone(mouth) - 1.0 + (mouth - d.max(end)) * 0.6
    }

    /// The middle of the lava lake's surface.
    pub fn crater_top(&self) -> (f32, f32, f32) {
        (self.cx, self.lava as f32 + 1.0, self.cz)
    }
}

/// What a volcano does to one column.
#[derive(Clone, Copy, Debug)]
pub(super) struct Cone {
    pub v: Volcano,
    /// Distance from the middle (made a little irregular), and its share of the cone (0 at the
    /// rim, 1 at the foot).
    pub d: f32,
    pub u: f32,
    pub angle: f32,
    /// The column is in the crater bowl.
    pub in_crater: bool,
    /// The cone raised this column (so it is volcano, not the ground around it).
    pub raised: bool,
}

#[derive(Clone)]
pub(super) struct VolcPalette {
    basalt: BlockId,
    smooth_basalt: BlockId,
    blackstone: BlockId,
    tuff: BlockId,
    obsidian: BlockId,
    magma: BlockId,
    lava: BlockId,
    ash: BlockId,
    ash_grass: BlockId,
    coarse_dirt: BlockId,
    gravel: BlockId,
    pumice: BlockId,
    sulfur_ore: BlockId,
    fumarole: BlockId,
    cinder_log: BlockId,
    ember_bloom: BlockId,
    dead_bush: BlockId,
    stone: BlockId,
    dirt: BlockId,
}

impl VolcPalette {
    pub(super) fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        let or = |a: BlockId, b: BlockId| if a == AIR { b } else { a };
        let stone = g("stone");
        let basalt = or(g("basalt"), stone);
        let blackstone = or(g("blackstone"), basalt);
        let gravel = g("gravel");
        let ash = or(g("ash_block"), gravel);
        VolcPalette {
            basalt,
            smooth_basalt: or(g("smooth_basalt"), basalt),
            blackstone,
            tuff: or(g("tuff"), stone),
            obsidian: or(g("obsidian"), blackstone),
            magma: or(g("magma_block"), blackstone),
            lava: g("lava"),
            ash,
            ash_grass: or(g("ash_grass"), ash),
            coarse_dirt: or(g("coarse_dirt"), g("dirt")),
            gravel,
            pumice: or(g("pumice"), g("tuff")),
            sulfur_ore: g("sulfur_ore"),
            fumarole: g("fumarole"),
            cinder_log: g("cinder_log"),
            ember_bloom: g("ember_bloom"),
            dead_bush: g("dead_bush"),
            stone,
            dirt: g("dirt"),
        }
    }
}

fn unit(h: u64, k: u32) -> f32 {
    ((h >> (k * 10)) & 0x3ff) as f32 / 1023.0
}

impl Generator {
    /// The volcano whose cone or ash fields reach this column, if any.
    pub fn volcano_at(&self, wx: i32, wz: i32) -> Option<Volcano> {
        let (ci, cj) = (wx.div_euclid(CELL), wz.div_euclid(CELL));
        let h = hash3(self.seed ^ 0x501C_A40, ci as i64, 7, cj as i64);
        if unit(h, 0) > CHANCE {
            return None;
        }
        let radius = 115.0 + unit(h, 3) * 45.0;
        // Room for the ash fields inside the cell, so only this cell can reach the column.
        let margin = radius * ASH_REACH + 30.0;
        let span = CELL as f32 - 2.0 * margin;
        let cx = (ci * CELL) as f32 + margin + unit(h, 1) * span;
        let cz = (cj * CELL) as f32 + margin + unit(h, 2) * span;
        if cx.hypot(cz) < SPAWN_CLEAR {
            return None;
        }
        let rim = 150 + (unit(h, 4) * 28.0) as i32;
        let t0 = unit(h, 5) * std::f32::consts::TAU;
        let third = std::f32::consts::TAU / 3.0;
        Some(Volcano {
            cx,
            cz,
            radius,
            crater: radius * 0.2,
            rim,
            lava: rim - 11,
            floor: rim - 18,
            tubes: [t0, t0 + third + 0.4, t0 + 2.0 * third - 0.3],
            seed: h,
        })
    }

    /// The volcano's say over this column: its shape (changes `h`), or nothing past its ash.
    pub(super) fn volcano_shape(&self, x: f32, z: f32, h: &mut f32) -> Option<Cone> {
        let v = self.volcano_at(x as i32, z as i32)?;
        let (dx, dz) = (x - v.cx, z - v.cz);
        let raw = dx.hypot(dz);
        if raw > v.radius * ASH_REACH + 12.0 {
            return None;
        }
        let angle = dz.atan2(dx);
        let (ca, sa) = (angle.cos(), angle.sin());
        // Not a perfect circle: the cone bulges and dents around its sides.
        let wob = self.mountain.noise3(ca * 1.6, sa * 1.6, v.seed as f32 % 97.0) * 0.12;
        let d = raw * (1.0 + wob);
        let u = ((d - v.crater) / (v.radius - v.crater)).clamp(0.0, 1.0);
        let rim = v.rim as f32;
        let cone = v.cone(d);
        // Gullies running down the sides, deepest halfway down.
        let gully = self.detail.noise3(ca * 7.0, sa * 7.0, d / 45.0) * 6.0 * (u * (1.0 - u) * 4.0).min(1.0);
        let mut target = cone + gully;
        let in_crater = d < v.crater;
        if in_crater {
            // A bowl: steep walls, a flat-ish floor under the lava.
            let s = d / v.crater;
            target = v.floor as f32 + (rim - v.floor as f32) * s.powi(3) + self.detail.noise2(x / 6.0, z / 6.0) * 0.8;
        }
        let raised = in_crater || target > *h - 0.5;
        if raised {
            *h = if in_crater { target } else { h.max(target) };
        }
        Some(Cone { v, d, u, angle, in_crater, raised })
    }

    /// Which biome a volcano makes of a column (after `volcano_shape`).
    pub(super) fn volcano_biome(&self, c: &Cone, height: i32) -> Option<Biome> {
        if c.raised && c.d <= c.v.radius * 0.92 && height > SEA_LEVEL + 2 {
            Some(Biome::Volcano)
        } else if c.d <= c.v.radius * ASH_REACH && height > SEA_LEVEL && height < SEA_LEVEL + 70 {
            Some(Biome::AshFields)
        } else {
            None
        }
    }

    /// What a crater's lake shows from afar: its lava.
    pub(super) fn volc_magma(&self) -> BlockId {
        if self.volc.lava != AIR {
            self.volc.lava
        } else {
            self.volc.magma
        }
    }

    /// The cone at a column of a volcano biome, measured again (generation keeps no copy).
    pub(super) fn cone_at(&self, wx: i32, wz: i32) -> Option<Cone> {
        let mut h = 0.0;
        self.volcano_shape(wx as f32, wz as f32, &mut h)
    }

    /// An old lava flow runs down the side here (a narrow band in the angle).
    fn flow(&self, c: &Cone) -> bool {
        let (ca, sa) = (c.angle.cos(), c.angle.sin());
        self.variant.noise3(ca * 11.0, sa * 11.0, c.d / 260.0 + 3.0).abs() < 0.045 && c.u < 0.92
    }

    /// Top, filler and filler depth of a volcano or ash field column.
    pub(super) fn volcano_top_blocks(&self, col: &Column, wx: i32, wz: i32) -> Option<(BlockId, BlockId, i32)> {
        let v = &self.volc;
        let n = self.detail.noise2(wx as f32 / 13.0 + 71.0, wz as f32 / 13.0 - 33.0);
        let spot = (wx.wrapping_mul(73_856_093) ^ wz.wrapping_mul(19_349_663)).rem_euclid(97);
        match col.biome {
            Biome::Volcano => {
                let c = self.cone_at(wx, wz)?;
                Some(if c.in_crater {
                    if n > 0.25 {
                        (v.magma, v.blackstone, 4)
                    } else {
                        (v.blackstone, v.basalt, 4)
                    }
                } else if self.flow(&c) {
                    if c.u < 0.3 {
                        (v.magma, v.obsidian, 3)
                    } else if n > -0.2 {
                        (v.obsidian, v.basalt, 3)
                    } else {
                        (v.smooth_basalt, v.basalt, 3)
                    }
                } else if c.u < 0.12 {
                    // The rim: dark rock and the first ash.
                    if n > 0.1 {
                        (v.blackstone, v.basalt, 4)
                    } else {
                        (v.basalt, v.basalt, 4)
                    }
                } else if spot == 0 {
                    (v.pumice, v.ash, 3)
                } else if n > 0.32 {
                    (v.basalt, v.basalt, 4)
                } else if n < -0.38 {
                    (v.tuff, v.tuff, 3)
                } else {
                    (v.ash, v.ash, 4)
                })
            }
            Biome::AshFields => Some(if n > 0.3 {
                (v.ash, v.ash, 3)
            } else if n < -0.4 {
                (v.coarse_dirt, v.dirt, 3)
            } else if spot < 2 {
                (v.gravel, v.ash, 2)
            } else {
                (v.ash_grass, v.ash, 3)
            }),
            _ => None,
        }
    }

    /// The rock inside a cone (instead of stone), the crater's lava and the lava tubes, for one
    /// column of a fresh chunk.
    pub(super) fn volcano_fill(&self, chunk: &mut Chunk, lx: usize, lz: usize, col: &Column, wx: i32, wz: i32) {
        if col.biome != Biome::Volcano {
            return;
        }
        let Some(c) = self.cone_at(wx, wz) else { return };
        let v = &self.volc;
        let h = col.height;
        // Inside the cone the stone is the volcano's: blackstone with tuff seams.
        for y in (FOOT as i32 - 6).max(1)..h {
            let id = chunk.get(lx, y as usize, lz);
            if id == v.stone {
                let seam = self.cheese.noise3(wx as f32 / 19.0, y as f32 / 9.0, wz as f32 / 19.0);
                let rock = if seam > 0.45 { v.tuff } else if seam < -0.5 { v.basalt } else { v.blackstone };
                chunk.set(lx, y as usize, lz, rock);
            }
        }
        // The crater's lava lake.
        if c.in_crater && v.lava != AIR {
            for y in (h + 1)..=c.v.lava {
                chunk.set(lx, y as usize, lz, v.lava);
            }
        }
        // Lava tubes: round tunnels from the flank up into the mountain, to a chamber of magma
        // under the crater.
        let (mouth, end) = c.v.tube_ends();
        for &a in &c.v.tubes {
            let mut diff = (c.angle - a).rem_euclid(std::f32::consts::TAU);
            if diff > std::f32::consts::PI {
                diff -= std::f32::consts::TAU;
            }
            let wiggle = self.detail.noise2(c.d / 23.0, a * 10.0) * 3.0 * ((c.d - end) / 20.0).clamp(0.0, 1.0);
            let side = c.d * diff - wiggle;
            // The chamber: a round hall at the tube's top end.
            let (ex, ez) = (c.v.cx + a.cos() * end, c.v.cz + a.sin() * end);
            let to_hall = (wx as f32 + 0.5 - ex).hypot(wz as f32 + 0.5 - ez);
            let hall = to_hall < 8.0;
            if !hall && (side.abs() > 4.0 || c.d < end || c.d > mouth + 4.0) {
                continue;
            }
            let ty = c.v.tube_y(c.d) + self.detail.noise2(c.d / 31.0, a * 5.0) * 1.0;
            let (ty, half) = if hall {
                let hy = c.v.tube_y(end) + 2.0;
                (hy, (64.0 - to_hall * to_hall).max(0.0).sqrt() * 0.75)
            } else {
                let r = 2.7 + self.detail.noise2(c.d / 9.0, a * 3.0) * 0.5;
                (ty, (r * r - side * side).max(0.0).sqrt())
            };
            if half <= 0.3 {
                continue;
            }
            let (lo, hi) = ((ty - half).round() as i32, (ty + half).round() as i32);
            for y in lo.max(2)..=hi.min(h + 2) {
                if (y as usize) < CHUNK_HEIGHT {
                    chunk.set(lx, y as usize, lz, AIR);
                }
            }
            if hall {
                // The chamber's floor is a pool of lava ringed with magma.
                let floor = lo - 1;
                if floor > 1 {
                    let fl = if to_hall < 4.5 && v.lava != AIR { v.lava } else { v.magma };
                    chunk.set(lx, floor as usize, lz, fl);
                }
                continue;
            }
            // A glowing floor: magma, with a thread of lava down the middle.
            let floor = lo - 1;
            if floor > 1 && side.abs() < 1.8 {
                let fl = if side.abs() < 0.7 && v.lava != AIR && self.cheese.noise2(c.d / 7.0, a) > 0.1 { v.lava } else { v.magma };
                chunk.set(lx, floor as usize, lz, fl);
            }
        }
    }

    /// Steam vents, sulfur, pumice boulders, burnt trunks and ember blooms.
    pub(super) fn decorate_volcano(&self, chunk: &mut Chunk, cols: &[Column], pos: ChunkPos) {
        if !cols.iter().any(|c| matches!(c.biome, Biome::Volcano | Biome::AshFields)) {
            return;
        }
        let v = &self.volc;
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 14);
        let top = |x: usize, z: usize| cols[x + z * 16].height;
        let biome = |x: usize, z: usize| cols[x + z * 16].biome;
        let set = |chunk: &mut Chunk, x: usize, y: i32, z: usize, id: BlockId| {
            if id != AIR && y > 0 && (y as usize) < CHUNK_HEIGHT {
                chunk.set(x, y as usize, z, id);
            }
        };
        // Fumaroles on the slopes, ringed by yellow sulfur.
        for _ in 0..3 {
            let (x, z) = (rng.range(1, 14) as usize, rng.range(1, 14) as usize);
            if biome(x, z) != Biome::Volcano || !rng.chance(0.45) || v.fumarole == AIR {
                continue;
            }
            let (wx, wz) = (pos.x * 16 + x as i32, pos.y * 16 + z as i32);
            let Some(c) = self.cone_at(wx, wz) else { continue };
            if c.in_crater || c.u < 0.08 || c.u > 0.88 {
                continue;
            }
            let y = top(x, z);
            set(chunk, x, y, z, v.fumarole);
            for (dx, dz) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (ax, az) = ((x as i32 + dx) as usize, (z as i32 + dz) as usize);
                if rng.chance(0.6) && v.sulfur_ore != AIR && (top(ax, az) - y).abs() <= 1 {
                    set(chunk, ax, top(ax, az), az, v.sulfur_ore);
                }
            }
        }
        // Sulfur seams in the rock of the cone.
        if v.sulfur_ore != AIR {
            for _ in 0..4 {
                let (x, z) = (rng.range(0, 15) as usize, rng.range(0, 15) as usize);
                if biome(x, z) != Biome::Volcano {
                    continue;
                }
                let y = top(x, z) - rng.range(3, 30);
                for k in 0..rng.range(2, 6) {
                    let (ax, ay, az) = ((x as i32 + k % 2).min(15) as usize, y + k / 2, (z as i32 + k % 3 / 2).min(15) as usize);
                    if ay > 1 && chunk.get(ax, ay as usize, az) == v.blackstone {
                        chunk.set(ax, ay as usize, az, v.sulfur_ore);
                    }
                }
            }
        }
        for z in 0..16usize {
            for x in 0..16usize {
                let y = top(x, z);
                let under = chunk.get(x, y as usize, z);
                let free = (y + 1) < CHUNK_HEIGHT as i32 && chunk.get(x, (y + 1) as usize, z) == AIR;
                if !free {
                    continue;
                }
                match biome(x, z) {
                    Biome::AshFields => {
                        let r = rng.f32();
                        if r < 0.012 && v.cinder_log != AIR && (2..14).contains(&x) && (2..14).contains(&z) {
                            // A burnt trunk, sometimes with a stub of a branch.
                            let tall = rng.range(3, 7);
                            for k in 1..=tall {
                                set(chunk, x, y + k, z, v.cinder_log);
                            }
                            if rng.chance(0.5) {
                                let (bx, bz) = if rng.chance(0.5) { (x + 1, z) } else { (x, z + 1) };
                                set(chunk, bx, y + tall - 1, bz, v.cinder_log);
                            }
                        } else if r < 0.02 {
                            set(chunk, x, y + 1, z, v.pumice);
                        } else if r < 0.034 {
                            set(chunk, x, y + 1, z, v.ember_bloom);
                        } else if r < 0.07 && under != v.gravel {
                            set(chunk, x, y + 1, z, v.dead_bush);
                        }
                    }
                    Biome::Volcano if under == v.ash && rng.chance(0.006) => {
                        // Pumice boulders lying on the ash.
                        set(chunk, x, y + 1, z, v.pumice);
                        if rng.chance(0.5) && x < 15 {
                            set(chunk, x + 1, top(x + 1, z) + 1, z, v.pumice);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen() -> Generator {
        let reg = BlockRegistry::from_ron(include_str!("../../../../packs/vanilla/blocks.ron")).unwrap();
        Generator::new(1234, &reg)
    }

    fn first_volcano(g: &Generator) -> Volcano {
        for j in -6..6 {
            for i in -6..6 {
                if let Some(v) = g.volcano_at(i * CELL + CELL / 2, j * CELL + CELL / 2) {
                    return v;
                }
            }
        }
        panic!("no volcano in 144 cells");
    }

    #[test]
    fn volcanoes_have_a_cone_a_crater_and_ash_around() {
        let g = gen();
        let v = first_volcano(&g);
        let (cx, cz) = (v.cx as i32, v.cz as i32);
        // The middle is a lava lake well above the sea.
        let mid = g.height_at(cx, cz);
        assert!(mid < v.lava && mid >= v.floor - 2, "crater floor {mid}, lava {}", v.lava);
        assert_eq!(g.biome_at(cx, cz), Biome::Volcano);
        // The rim is the highest ground, and the cone falls away from it.
        let rim_r = v.crater as i32 + 2;
        let rim = g.height_at(cx + rim_r, cz);
        let half = g.height_at(cx + (v.radius * 0.5) as i32, cz);
        assert!(rim > v.lava && rim > half + 20, "rim {rim}, halfway {half}");
        let ash = (cx + (v.radius * 1.2) as i32, cz);
        let b = g.biome_at(ash.0, ash.1);
        assert!(matches!(b, Biome::AshFields | Biome::Ocean | Biome::DeepOcean | Biome::WarmOcean | Biome::LukewarmOcean | Biome::ColdOcean | Biome::Beach | Biome::River), "foot is {b:?}");
        // The crater chunk holds lava.
        let c = g.generate(ChunkPos::new(cx >> 4, cz >> 4));
        let lava_id = g.volc.lava;
        assert!(c.all_blocks().contains(&lava_id), "no lava in the crater");
    }

    #[test]
    fn lava_tubes_open_inside_the_cone() {
        let g = gen();
        let v = first_volcano(&g);
        let (mouth, end) = v.tube_ends();
        let mut open = 0;
        let mut under = 0;
        let n = 40;
        for k in 0..n {
            let d = end + 6.0 + (mouth - end - 8.0) * k as f32 / n as f32;
            let a = v.tubes[1];
            let (x, z) = ((v.cx + a.cos() * d) as i32, (v.cz + a.sin() * d) as i32);
            let h = g.height_at(x, z);
            let c = g.generate(ChunkPos::new(x >> 4, z >> 4));
            let (lx, lz) = (x.rem_euclid(16) as usize, z.rem_euclid(16) as usize);
            let ty = v.tube_y(d) as i32;
            if (ty - 1..=ty + 1).any(|y| c.get(lx, y as usize, lz) == AIR) {
                open += 1;
            }
            if h > ty + 3 {
                under += 1;
            }
        }
        assert!(open * 10 >= n * 8, "tube open in only {open} of {n} columns");
        assert!(under * 10 >= n * 6, "tube under ground in only {under} of {n} columns");
        // It climbs gently enough to walk.
        assert!((v.tube_y(end) - v.tube_y(mouth)) / (mouth - end) <= 0.65);
    }

    #[test]
    fn spawn_stays_clear_of_volcanoes() {
        let g = gen();
        let (x, _, z) = g.find_spawn();
        assert!(g.volcano_at(x, z).map(|v| (v.cx - x as f32).hypot(v.cz - z as f32) > v.radius * ASH_REACH).unwrap_or(true));
    }
}
