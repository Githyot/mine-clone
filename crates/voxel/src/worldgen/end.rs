//! The End: a big island of end stone floating in a dark void, ringed by tall obsidian
//! pillars with glowing crystals on top, a bedrock fountain in the middle that turns into the
//! way home once the guardian falls, and smaller islands with chorus plants far out past a gap.
//! Our own take on the idea, from the public descriptions of the game.

use super::Generator;
use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos};
use crate::rng::Rng;

/// Height of the exit fountain's floor at the middle of the main island.
pub const END_EXIT_Y: i32 = 64;
/// Where travellers from the Overworld arrive: an obsidian platform off the island's edge.
pub const END_PLATFORM: (i32, i32, i32) = (98, 63, 0);
/// Past this distance the void opens up until the outer islands.
const MAIN_RADIUS: f32 = 92.0;
const OUTER_START: f32 = 640.0;
const CELL: i32 = 80;

#[derive(Clone)]
pub(super) struct EndPalette {
    pub(super) end_stone: BlockId,
    obsidian: BlockId,
    bedrock: BlockId,
    crystal: BlockId,
    chorus: BlockId,
    flower: BlockId,
}

impl EndPalette {
    pub(super) fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        EndPalette {
            end_stone: g("end_stone"),
            obsidian: g("obsidian"),
            bedrock: g("bedrock"),
            crystal: g("end_crystal"),
            chorus: g("chorus_plant"),
            flower: g("chorus_flower"),
        }
    }
}

/// One of the obsidian pillars around the main island: its middle, radius and top.
#[derive(Clone, Copy, Debug)]
pub struct EndPillar {
    pub x: i32,
    pub z: i32,
    pub r: i32,
    pub top: i32,
}

impl EndPillar {
    /// Where its crystal sits.
    pub fn crystal(&self) -> (i32, i32, i32) {
        (self.x, self.top + 2, self.z)
    }
}

/// The ten pillars of a world (the same every time for a seed).
pub fn end_pillars(seed: u64) -> Vec<EndPillar> {
    let mut rng = Rng::for_pos(seed, 0, 0, 0xE11D);
    (0..10)
        .map(|k| {
            let a = k as f32 / 10.0 * std::f32::consts::TAU;
            EndPillar { x: (a.cos() * 42.0).round() as i32, z: (a.sin() * 42.0).round() as i32, r: 2 + (k % 3), top: 76 + rng.range(0, 27) }
        })
        .collect()
}

impl Generator {
    /// Top and bottom of the land in a column, if there is any.
    pub(super) fn end_land(&self, wx: i32, wz: i32) -> Option<(i32, i32)> {
        let (x, z) = (wx as f32, wz as f32);
        let d = (x * x + z * z).sqrt();
        let n = self.detail.fbm2(x / 38.0 + 900.0, z / 38.0, 3);
        let r = MAIN_RADIUS + n * 14.0;
        if d < r {
            let t = 1.0 - d / r;
            let top = END_EXIT_Y - 2 + (t.sqrt() * 6.0 + n * 2.0).round() as i32;
            let depth = (t.powf(0.7) * 46.0 + 3.0) as i32;
            return Some((top, top - depth));
        }
        if d < OUTER_START {
            return None;
        }
        // Outer islands: at most one per cell, checked in this cell and its neighbours.
        let (cx, cz) = (wx.div_euclid(CELL), wz.div_euclid(CELL));
        let mut best: Option<(i32, i32)> = None;
        for oz in -1..=1 {
            for ox in -1..=1 {
                let mut rng = Rng::for_pos(self.seed, cx + ox, cz + oz, 0xE15);
                if !rng.chance(0.55) {
                    continue;
                }
                let ix = (cx + ox) * CELL + rng.range(16, CELL - 16);
                let iz = (cz + oz) * CELL + rng.range(16, CELL - 16);
                let radius = rng.range(12, 30) as f32 * (0.85 + 0.3 * n);
                let h = rng.range(52, 72);
                let dd = (((wx - ix) * (wx - ix) + (wz - iz) * (wz - iz)) as f32).sqrt();
                if dd >= radius {
                    continue;
                }
                let t = 1.0 - dd / radius;
                let top = h + (t.sqrt() * 3.0 + n) as i32;
                let bottom = top - (t * radius * 0.8) as i32 - 1;
                if best.is_none_or(|b| top > b.0) {
                    best = Some((top, bottom));
                }
            }
        }
        best
    }

    pub(super) fn generate_end(&self, pos: ChunkPos) -> Chunk {
        let p = &self.end;
        let mut chunk = Chunk::new(pos);
        let (bx, bz) = (pos.x * 16, pos.y * 16);
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 0xE4D);
        for z in 0..16usize {
            for x in 0..16usize {
                if let Some((top, bottom)) = self.end_land(bx + x as i32, bz + z as i32) {
                    for y in bottom.max(1)..=top {
                        chunk.set(x, y as usize, z, p.end_stone);
                    }
                }
            }
        }
        let inside = |wx: i32, wz: i32| (0..16).contains(&(wx - bx)) && (0..16).contains(&(wz - bz));
        let set = |chunk: &mut Chunk, wx: i32, y: i32, wz: i32, id: BlockId| {
            if inside(wx, wz) && (1..250).contains(&y) {
                chunk.set((wx - bx) as usize, y as usize, (wz - bz) as usize, id);
            }
        };
        // Obsidian pillars, each with a bedrock cap and a crystal.
        for pl in end_pillars(self.seed) {
            if (pl.x - bx).abs() > 24 || (pl.z - bz).abs() > 24 {
                continue;
            }
            for dz in -pl.r..=pl.r {
                for dx in -pl.r..=pl.r {
                    if dx * dx + dz * dz > pl.r * pl.r + pl.r {
                        continue;
                    }
                    for y in 40..=pl.top {
                        set(&mut chunk, pl.x + dx, y, pl.z + dz, p.obsidian);
                    }
                }
            }
            set(&mut chunk, pl.x, pl.top + 1, pl.z, p.bedrock);
            set(&mut chunk, pl.x, pl.top + 2, pl.z, p.crystal);
        }
        // The exit fountain: a bedrock bowl around a pillar, empty until the guardian falls.
        if bx.abs() < 24 && bz.abs() < 24 {
            let y = END_EXIT_Y;
            for dz in -4i32..=4 {
                for dx in -4i32..=4 {
                    let d2 = dx * dx + dz * dz;
                    if d2 > 18 {
                        continue;
                    }
                    for k in 1..9 {
                        set(&mut chunk, dx, y + k, dz, AIR);
                    }
                    set(&mut chunk, dx, y - 1, dz, p.bedrock);
                    if d2 > 8 {
                        set(&mut chunk, dx, y, dz, p.bedrock);
                    } else {
                        set(&mut chunk, dx, y, dz, AIR);
                    }
                }
            }
            for k in 0..4 {
                set(&mut chunk, 0, y + k, 0, p.bedrock);
            }
        }
        // Chorus plants on the outer islands: a stalk that forks, a flower at each tip.
        if ((bx * bx + bz * bz) as f32).sqrt() > OUTER_START {
            for _ in 0..3 {
                let (x, z) = (rng.range(2, 13), rng.range(2, 13));
                let Some((top, _)) = self.end_land(bx + x, bz + z) else { continue };
                if !rng.chance(0.6) {
                    continue;
                }
                let h = rng.range(3, 7);
                let (x, z) = (bx + x, bz + z);
                for k in 1..=h {
                    set(&mut chunk, x, top + k, z, p.chorus);
                }
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if rng.chance(0.45) {
                        let at = top + rng.range(2, h);
                        let arm = rng.range(1, 2);
                        for a in 1..=arm {
                            set(&mut chunk, x + dx * a, at, z + dz * a, p.chorus);
                        }
                        set(&mut chunk, x + dx * arm, at + 1, z + dz * arm, p.flower);
                    }
                }
                set(&mut chunk, x, top + h + 1, z, p.flower);
            }
        }
        chunk
    }
}
