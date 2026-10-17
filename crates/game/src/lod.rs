//! Distant terrain: past the render distance the world is drawn as a coarse, blocky heightmap
//! straight from the world generator, so mountains and coasts show up far away without loading
//! chunks. The ground is cut into square tiles; each tile samples one column every `step` blocks
//! (4 near, 16 far), gives it a flat top in the colour of its surface block and walls down to its
//! neighbours. Tiles are built on their own worker threads, kept while in range, and drawn by the
//! renderer with fog. Where real chunks are drawn the shader throws the tiles away.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{unbounded, Receiver, Sender};
use glam::{IVec2, Vec2, Vec3};
use wgpu::util::DeviceExt;

use crate::textures::TextureSet;
use voxel::worldgen::{Biome, Generator, SEA_LEVEL};
use voxel::{BlockId, BlockRegistry};

/// Tile size in blocks.
pub const TILE: i32 = 128;
/// Extra distances offered in the menu (blocks past the render distance; 0 = off).
pub const DISTANCES: [i32; 4] = [0, 256, 512, 1024];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct LodVertex {
    /// Position relative to the tile's corner.
    pub pos: [f32; 3],
    /// rgb: colour with face shading; a: tree canopy height in blocks (the shader flattens it
    /// near the real terrain).
    pub color: [u8; 4],
}

/// Average colours of block faces, taken from the textures in use.
pub struct LodColors {
    top: Vec<[f32; 3]>,
    side: Vec<[f32; 3]>,
    /// Biome tint for (top, side), like the mesher: grass block sides carry their own dirt.
    tint: Vec<(bool, bool)>,
}

fn average(pixels: &[u8]) -> [f32; 3] {
    let (mut sum, mut n) = ([0f32; 3], 0f32);
    for px in pixels.chunks_exact(4) {
        if px[3] > 127 {
            for c in 0..3 {
                sum[c] += px[c] as f32;
            }
            n += 1.0;
        }
    }
    if n == 0.0 {
        return [0.5; 3];
    }
    sum.map(|s| s / n / 255.0)
}

impl LodColors {
    pub fn new(reg: &BlockRegistry, tex: &TextureSet) -> LodColors {
        let face = |layer: u16| tex.layers.get(layer as usize).map(|p| average(p)).unwrap_or([0.5; 3]);
        let mut c = LodColors { top: Vec::new(), side: Vec::new(), tint: Vec::new() };
        for b in &reg.blocks {
            let leaves = b.name.ends_with("_leaves");
            // Leaves look darker from afar: the gaps show the shaded inside of the tree.
            let k = if leaves { 0.72 } else { 1.0 };
            c.top.push(face(b.faces[2]).map(|v| v * k));
            c.side.push(face(b.faces[4]).map(|v| v * k));
            let tinted = |f: u16| b.tint && !reg.textures.get(f as usize).is_some_and(|n| n.ends_with("_side"));
            c.tint.push((tinted(b.faces[2]), tinted(b.faces[4])));
        }
        c
    }

    fn get(&self, id: BlockId, side: bool, biome: Biome) -> [f32; 3] {
        let i = id as usize;
        let base = if side { self.side.get(i) } else { self.top.get(i) }.copied().unwrap_or([0.5; 3]);
        let (tt, ts) = self.tint.get(i).copied().unwrap_or((false, false));
        if (side && ts) || (!side && tt) {
            let t = biome.tint();
            [base[0] * t[0], base[1] * t[1], base[2] * t[2]]
        } else {
            base
        }
    }
}

/// One sampled cell of a tile.
#[derive(Clone, Copy)]
struct Cell {
    /// Height of the flat top (blocks).
    y: f32,
    /// Extra height of the tree canopy on top of `y`.
    canopy: f32,
    top: [f32; 3],
    side: [f32; 3],
}

fn hash(x: i32, z: i32, seed: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (z as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ seed;
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h >> 40) as f32 / (1u64 << 24) as f32
}

fn cell(gen: &Generator, colors: &LodColors, wx: i32, wz: i32) -> Cell {
    let s = gen.surface(wx, wz);
    if let Some(l) = s.liquid {
        // Water surface as the mesher draws it (2/16 below the block top).
        let c = colors.get(l, false, s.biome);
        let w = s.biome.water_tint();
        let water = [c[0] * 0.85 * w[0], c[1] * 0.9 * w[1], c[2] * w[2]];
        return Cell { y: SEA_LEVEL as f32 + 0.875, canopy: 0.0, top: water, side: water };
    }
    let mut c = Cell { y: s.height as f32 + 1.0, canopy: 0.0, top: colors.get(s.top, false, s.biome), side: colors.get(s.top, true, s.biome) };
    if let Some((leaves, density)) = s.trees {
        // No real tree positions here: a share of cells become canopy, which reads as forest.
        if hash(wx, wz, gen.seed) < density * 0.6 {
            let k = 3.0 + hash(wz, wx, gen.seed) * 2.0;
            c.canopy = k;
            c.top = colors.get(leaves, false, s.biome);
            c.side = c.top;
        }
    }
    c
}

pub struct TileMesh {
    pub verts: Vec<LodVertex>,
    pub min_y: f32,
    pub max_y: f32,
}

fn rgb8(c: [f32; 3], shade: f32) -> [u8; 3] {
    c.map(|v| (v * shade * 255.0).clamp(0.0, 255.0) as u8)
}

/// Build one tile: `TILE / step` cells a side, each a flat top plus walls where it stands
/// above a neighbour. Tile borders always get a short skirt so tiles of different detail meet
/// without cracks.
pub fn build_tile(gen: &Generator, colors: &LodColors, tile: IVec2, step: i32) -> TileMesh {
    let n = (TILE / step) as usize;
    let ox = tile.x * TILE;
    let oz = tile.y * TILE;
    // Cells -1..=n on each axis: the ring outside the tile gives the border walls their bottom.
    let w = n + 2;
    let mut cells = Vec::with_capacity(w * w);
    for j in 0..w {
        for i in 0..w {
            let wx = ox + (i as i32 - 1) * step + step / 2;
            let wz = oz + (j as i32 - 1) * step + step / 2;
            cells.push(cell(gen, colors, wx, wz));
        }
    }
    let at = |i: usize, j: usize| cells[i + 1 + (j + 1) * w];
    let mut verts = Vec::with_capacity(n * n * 8);
    let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
    let sf = step as f32;
    let mut quad = |pts: [[f32; 3]; 4], raise: [f32; 4], color: [u8; 3]| {
        for k in 0..4 {
            let r = raise[k].round().clamp(0.0, 255.0);
            min_y = min_y.min(pts[k][1]);
            max_y = max_y.max(pts[k][1] + r);
            verts.push(LodVertex { pos: pts[k], color: [color[0], color[1], color[2], r as u8] });
        }
    };
    for j in 0..n {
        // Tops: runs of equal cells along x become one quad (oceans and plains shrink a lot).
        let mut i = 0;
        while i < n {
            let c = at(i, j);
            let color = rgb8(c.top, 1.0);
            let mut end = i + 1;
            while end < n {
                let e = at(end, j);
                if e.y != c.y || e.canopy != c.canopy || rgb8(e.top, 1.0) != color {
                    break;
                }
                end += 1;
            }
            let (x0, x1, z0, z1) = (i as f32 * sf, end as f32 * sf, j as f32 * sf, (j + 1) as f32 * sf);
            quad([[x0, c.y, z0], [x0, c.y, z1], [x1, c.y, z1], [x1, c.y, z0]], [c.canopy; 4], color);
            i = end;
        }
        for i in 0..n {
            let c = at(i, j);
            let (x0, z0) = (i as f32 * sf, j as f32 * sf);
            let (x1, z1) = (x0 + sf, z0 + sf);
            // Walls toward each neighbour that is lower (counting tree tops).
            let top = c.y + c.canopy;
            let sides: [(isize, isize, f32); 4] = [(1, 0, 0.6), (-1, 0, 0.6), (0, 1, 0.8), (0, -1, 0.8)];
            for (dx, dz, shade) in sides {
                let (ni, nj) = (i as isize + dx, j as isize + dz);
                let border = ni < 0 || nj < 0 || ni >= n as isize || nj >= n as isize;
                let nb = cells[(ni + 1) as usize + (nj + 1) as usize * w];
                let ntop = nb.y + nb.canopy;
                if ntop >= top && !border {
                    continue;
                }
                // Wall from this cell's top down to the neighbour's (both may carry canopy, which
                // the shader scales the same way), or a skirt below both at the tile border.
                let (bottom, braise) = if border { (nb.y.min(c.y) - 6.0, 0.0) } else { (nb.y, nb.canopy) };
                let (a, b) = match (dx, dz) {
                    (1, 0) => ([x1, z0], [x1, z1]),
                    (-1, 0) => ([x0, z1], [x0, z0]),
                    (0, 1) => ([x1, z1], [x0, z1]),
                    _ => ([x0, z0], [x1, z0]),
                };
                quad(
                    [[a[0], c.y, a[1]], [a[0], bottom, a[1]], [b[0], bottom, b[1]], [b[0], c.y, b[1]]],
                    [c.canopy, braise, braise, c.canopy],
                    rgb8(c.side, shade),
                );
            }
        }
    }
    TileMesh { verts, min_y, max_y }
}

pub struct GpuTile {
    pub buf: wgpu::Buffer,
    pub quads: u32,
    pub step: i32,
    pub min_y: f32,
    pub max_y: f32,
}

/// Tiles in range, the jobs building them, and their GPU buffers.
pub struct Lod {
    jobs: Sender<(u32, IVec2, i32)>,
    done: Receiver<(u32, IVec2, i32, TileMesh)>,
    colors: Arc<std::sync::RwLock<Arc<LodColors>>>,
    gen: Arc<std::sync::RwLock<Arc<Generator>>>,
    pub tiles: HashMap<IVec2, GpuTile>,
    pending: HashSet<IVec2>,
    /// Tiles wanted now, nearest first, with their step.
    wanted: Vec<(IVec2, i32)>,
    wanted_for: Option<(IVec2, i32, i32)>,
    /// Bumped when colours change so results built with the old ones are dropped.
    epoch: u32,
    pub workers: usize,
    pub vram_bytes: u64,
}

impl Lod {
    pub fn new(gen: Arc<Generator>, colors: LodColors) -> Lod {
        let (jobs, job_rx) = unbounded::<(u32, IVec2, i32)>();
        let (done_tx, done) = unbounded();
        let colors = Arc::new(std::sync::RwLock::new(Arc::new(colors)));
        let gen = Arc::new(std::sync::RwLock::new(gen));
        // Few threads: chunk generation near the player matters more.
        let workers = (thread::available_parallelism().map(|n| n.get()).unwrap_or(4) / 4).clamp(1, 3);
        for i in 0..workers {
            let (rx, tx, gen, colors) = (job_rx.clone(), done_tx.clone(), gen.clone(), colors.clone());
            let _ = thread::Builder::new().name(format!("lod-{i}")).spawn(move || {
                while let Ok((epoch, tile, step)) = rx.recv() {
                    let c = colors.read().map(|c| c.clone()).ok();
                    let Some(c) = c else { break };
                    let g = gen.read().map(|g| g.clone()).ok();
                    let Some(gen) = g else { break };
                    if tx.send((epoch, tile, step, build_tile(&gen, &c, tile, step))).is_err() {
                        break;
                    }
                }
            });
        }
        Lod {
            jobs,
            done,
            colors,
            gen,
            tiles: HashMap::new(),
            pending: HashSet::new(),
            wanted: Vec::new(),
            wanted_for: None,
            epoch: 0,
            workers,
            vram_bytes: 0,
        }
    }

    /// New textures: rebuild every tile with the new colours.
    pub fn set_colors(&mut self, colors: LodColors) {
        if let Ok(mut c) = self.colors.write() {
            *c = Arc::new(colors);
        }
        self.epoch += 1;
        self.clear();
    }

    /// Another world (or dimension): drop every tile and build new ones from its generator.
    pub fn set_gen(&mut self, gen: Arc<Generator>) {
        if let Ok(mut g) = self.gen.write() {
            *g = gen;
        }
        self.epoch += 1;
        self.clear();
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
        self.pending.clear();
        self.wanted.clear();
        self.wanted_for = None;
        self.vram_bytes = 0;
    }

    pub fn pending_jobs(&self) -> usize {
        self.pending.len()
    }

    /// Every tile in range is built at the right detail (test screenshots wait for this).
    pub fn is_complete(&self) -> bool {
        self.pending.is_empty() && self.wanted.iter().all(|(t, step)| self.tiles.get(t).map(|g| g.step == *step).unwrap_or(false))
    }

    /// Detail for a tile whose centre is `d` blocks away.
    fn step_for(d: f32) -> i32 {
        if d < 384.0 {
            4
        } else if d < 768.0 {
            8
        } else {
            16
        }
    }

    /// Keep tiles between `inner` (where real chunks end) and `outer` blocks from the eye.
    pub fn update(&mut self, device: &wgpu::Device, eye: Vec3, inner: f32, outer: f32) {
        if outer <= inner {
            if !self.tiles.is_empty() || !self.pending.is_empty() {
                self.epoch += 1;
                self.clear();
            }
            return;
        }
        while let Ok((epoch, tile, step, mesh)) = self.done.try_recv() {
            self.pending.remove(&tile);
            if epoch != self.epoch || !self.wanted.iter().any(|w| w.0 == tile) {
                continue;
            }
            let bytes = (mesh.verts.len() * std::mem::size_of::<LodVertex>()) as u64;
            if let Some(old) = self.tiles.remove(&tile) {
                self.vram_bytes -= old.buf.size();
            }
            if mesh.verts.is_empty() {
                continue;
            }
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("lod tile"),
                contents: bytemuck::cast_slice(&mesh.verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            self.vram_bytes += bytes;
            self.tiles.insert(tile, GpuTile { buf, quads: (mesh.verts.len() / 4) as u32, step, min_y: mesh.min_y, max_y: mesh.max_y });
        }

        // Re-plan when the eye enters another 32-block cell or the distances change.
        let cell = IVec2::new((eye.x / 32.0).floor() as i32, (eye.z / 32.0).floor() as i32);
        let key = (cell, inner as i32, outer as i32);
        if self.wanted_for != Some(key) {
            self.wanted_for = Some(key);
            let e = Vec2::new(eye.x, eye.z);
            let r = (outer / TILE as f32).ceil() as i32 + 1;
            let ct = IVec2::new((eye.x / TILE as f32).floor() as i32, (eye.z / TILE as f32).floor() as i32);
            self.wanted.clear();
            for tz in -r..=r {
                for tx in -r..=r {
                    let t = ct + IVec2::new(tx, tz);
                    let lo = Vec2::new((t.x * TILE) as f32, (t.y * TILE) as f32);
                    let hi = lo + Vec2::splat(TILE as f32);
                    let near = e.clamp(lo, hi).distance(e);
                    let far = [lo, hi, Vec2::new(lo.x, hi.y), Vec2::new(hi.x, lo.y)].iter().map(|c| c.distance(e)).fold(0.0, f32::max);
                    // Skip tiles out of range, and tiles hidden entirely by real chunks.
                    if near > outer || far < inner {
                        continue;
                    }
                    let d = ((lo + hi) * 0.5).distance(e);
                    self.wanted.push((t, Self::step_for(d)));
                }
            }
            self.wanted.sort_by(|a, b| {
                let da = (a.0 * TILE + TILE / 2).as_vec2().distance(e);
                let db = (b.0 * TILE + TILE / 2).as_vec2().distance(e);
                da.total_cmp(&db)
            });
            let keep: HashSet<IVec2> = self.wanted.iter().map(|w| w.0).collect();
            let mut freed = 0;
            self.tiles.retain(|t, g| {
                let k = keep.contains(t);
                if !k {
                    freed += g.buf.size();
                }
                k
            });
            self.vram_bytes -= freed;
        }

        // Ask for missing tiles and tiles at the wrong detail, nearest first.
        let max_in_flight = self.workers * 3;
        for &(t, step) in &self.wanted {
            if self.pending.len() >= max_in_flight {
                break;
            }
            if self.pending.contains(&t) || self.tiles.get(&t).is_some_and(|g| g.step == step) {
                continue;
            }
            self.pending.insert(t);
            let _ = self.jobs.send((self.epoch, t, step));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Generator, LodColors) {
        let reg = BlockRegistry::from_ron(include_str!("../../../packs/vanilla/blocks.ron")).unwrap();
        let gen = Generator::new(12345, &reg);
        let tex = TextureSet { size: 2, layers: vec![vec![128; 16]; reg.textures.len()], packs: Vec::new(), gui: Default::default() };
        let colors = LodColors::new(&reg, &tex);
        (gen, colors)
    }

    #[test]
    fn tiles_are_quads_inside_the_tile() {
        let (gen, colors) = setup();
        for step in [4, 8, 16] {
            let m = build_tile(&gen, &colors, IVec2::new(3, -2), step);
            let n = (TILE / step) as usize;
            assert_eq!(m.verts.len() % 4, 0);
            // At least one top per row.
            assert!(m.verts.len() / 4 >= n);
            for v in &m.verts {
                assert!((0.0..=TILE as f32).contains(&v.pos[0]) && (0.0..=TILE as f32).contains(&v.pos[2]));
            }
            assert!(m.min_y <= m.max_y);
        }
    }

    #[test]
    fn tile_tops_follow_the_generator() {
        let (gen, colors) = setup();
        let m = build_tile(&gen, &colors, IVec2::new(0, 0), 8);
        // The first quad is the top of cell (0, 0), sampled at (4, 4).
        let s = gen.surface(4, 4);
        let expect = if s.liquid.is_some() { SEA_LEVEL as f32 + 0.875 } else { s.height as f32 + 1.0 };
        assert_eq!(m.verts[0].pos[1], expect);
    }
}
