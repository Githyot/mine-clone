//! Distant terrain (Distant Horizons idea): past the render distance the world is drawn as a coarse,
//! blocky heightmap straight from the world generator, so mountains and coasts show up far away
//! without loading chunks.
//!
//! The ground is a quadtree of square nodes. Every node, whatever its size, is 32 × 32 cells: a
//! 128-block node samples a column every 4 blocks, a 4096-block node every 128. Near the eye the
//! tree splits into small detailed nodes, far away it stays in big coarse ones, so the polygon
//! count and the number of draw calls hardly grow with the distance. Each cell is a flat top in
//! the colour of its surface block with walls down to its neighbours; equal tops merge into big
//! rectangles and equal walls into long strips, and far nodes round their heights and colours
//! so more of them merge.
//!
//! Nodes are built on worker threads (nearest first, a few in flight; one node takes about a
//! tenth of a millisecond, so they are not kept on disk), uploaded to the GPU under a per-frame
//! budget and kept under a video memory cap. An old node stays drawn until the nodes
//! replacing it are ready, so moving never opens holes. The renderer draws them with fog only:
//! no textures, shadows, reflections or block depth; where real chunks are drawn the shader
//! throws them away, and nodes clear of that area use a cheaper shader without the test.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{unbounded, Receiver, Sender};
use glam::{IVec2, Vec2, Vec3};
use wgpu::util::DeviceExt;

use crate::textures::TextureSet;
use voxel::worldgen::{Biome, Generator, SEA_LEVEL};
use voxel::{BlockId, BlockRegistry};

/// Cells along each side of a node, at every level.
pub const CELLS: i32 = 32;
/// Node sizes are `128 << level` blocks (128 .. 4096).
pub const LEVELS: u8 = 6;
/// Smallest node size in blocks.
pub const TILE: i32 = CELLS * 4;
/// Extra blocks of distant terrain the options offer; 2048 is 128 chunks, 4096 is 256.
pub const DISTANCES: [i32; 6] = [0, 256, 512, 1024, 2048, 4096];
/// Detail presets ("Detalhe LOD"): names, how close (in node sizes) a node must be to split
/// into four finer ones, the finest level used, and the video memory cap in MB.
pub const DETAILS: [(&str, f32, u8, u64); 3] = [("Leve", 1.0, 1, 48), ("Normal", 1.5, 0, 128), ("Alto", 2.5, 0, 320)];
/// Bytes of node meshes uploaded per frame at most, so a burst of finished nodes never stalls a frame.
const UPLOAD_BUDGET: u64 = 1 << 20;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, PartialEq)]
pub struct LodVertex {
    /// x and z in cells (6 bits each), level (3 bits), y × 8 + 8192 (14 bits). Half the size of
    /// three floats, so twice the terrain fits in the same video memory.
    pub pos: u32,
    /// rgb: colour with face shading; a: tree canopy height in blocks (the shader flattens it
    /// near the real terrain).
    pub color: [u8; 4],
}

impl LodVertex {
    fn new(x: i32, y: f32, z: i32, level: u8, color: [u8; 4]) -> LodVertex {
        let y8 = ((y * 8.0).round() as i32 + 8192).clamp(0, 16383) as u32;
        LodVertex { pos: (x as u32 & 63) | ((z as u32 & 63) << 6) | ((level as u32 & 7) << 12) | (y8 << 15), color }
    }

    /// Position relative to the node's corner, in blocks (the shader does the same).
    #[cfg(test)]
    pub fn unpack(&self) -> Vec3 {
        let step = (4 << ((self.pos >> 12) & 7)) as f32;
        let y = ((self.pos >> 15) & 16383) as f32 / 8.0 - 1024.0;
        Vec3::new((self.pos & 63) as f32 * step, y, ((self.pos >> 6) & 63) as f32 * step)
    }
}

/// A node of the quadtree: `level` and its corner in units of its own size.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Node {
    pub level: u8,
    pub pos: IVec2,
}

impl Node {
    pub fn size(self) -> i32 {
        TILE << self.level
    }

    pub fn step(self) -> i32 {
        4 << self.level
    }

    /// Corner in world blocks.
    pub fn origin(self) -> IVec2 {
        self.pos * self.size()
    }

    fn rect(self) -> (Vec2, Vec2) {
        let lo = self.origin().as_vec2();
        (lo, lo + Vec2::splat(self.size() as f32))
    }

    fn overlaps(self, o: Node) -> bool {
        let (a0, a1) = self.rect();
        let (b0, b1) = o.rect();
        a0.x < b1.x && b0.x < a1.x && a0.y < b1.y && b0.y < a1.y
    }

    /// Closest and farthest distance from `e` to the node's square.
    fn dist(self, e: Vec2) -> (f32, f32) {
        let (lo, hi) = self.rect();
        let near = e.clamp(lo, hi).distance(e);
        let far = (e - lo).abs().max((e - hi).abs()).length();
        (near, far)
    }
}

/// The nodes to draw for an eye at `e`: covering the ring between `inner` and `outer` blocks, fine
/// near the eye and coarse far away, nearest first.
pub fn plan(e: Vec2, inner: f32, outer: f32, detail: usize) -> Vec<Node> {
    let (_, split, min_level, _) = DETAILS[detail.min(DETAILS.len() - 1)];
    let top = LEVELS - 1;
    let root = (TILE << top) as f32;
    let lo = ((e - Vec2::splat(outer)) / root).floor().as_ivec2();
    let hi = ((e + Vec2::splat(outer)) / root).floor().as_ivec2();
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for z in lo.y..=hi.y {
        for x in lo.x..=hi.x {
            stack.push(Node { level: top, pos: IVec2::new(x, z) });
        }
    }
    while let Some(n) = stack.pop() {
        let (near, far) = n.dist(e);
        // Out of range, or hidden entirely by real chunks.
        if near > outer || far < inner {
            continue;
        }
        // Split while the node's cells would look bigger than about 1/(32 × split) radians from
        // its centre.
        let (lo, hi) = n.rect();
        if n.level > min_level && ((lo + hi) * 0.5).distance(e) < split * n.size() as f32 {
            for k in 0..4 {
                stack.push(Node { level: n.level - 1, pos: n.pos * 2 + IVec2::new(k & 1, k >> 1) });
            }
        } else {
            out.push(n);
        }
    }
    out.sort_by(|a, b| a.dist(e).0.total_cmp(&b.dist(e).0));
    out
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

/// One sampled cell of a node.
#[derive(Clone, Copy)]
struct Cell {
    /// Height of the flat top (blocks).
    y: f32,
    /// Extra height of the tree canopy on top of `y`.
    canopy: f32,
    top: [u8; 3],
    side: [u8; 3],
}

fn hash(x: i32, z: i32, seed: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (z as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ seed;
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h >> 40) as f32 / (1u64 << 24) as f32
}

fn rgb8(c: [f32; 3], shade: f32) -> [u8; 3] {
    c.map(|v| (v * shade * 255.0).clamp(0.0, 255.0) as u8)
}

/// Sample one column. Far levels round heights (`q` blocks) and colours so neighbours match and
/// merge into bigger quads: at that distance a cell is a few pixels anyway.
fn cell(gen: &Generator, colors: &LodColors, wx: i32, wz: i32, level: u8) -> Cell {
    let q = (1 << level.saturating_sub(1)).min(8) as f32;
    let cmask = if level >= 3 { 0xF8 } else { 0xFF };
    let col = |c: [f32; 3]| rgb8(c, 1.0).map(|v| v & cmask);
    let s = gen.surface(wx, wz);
    if let Some(l) = s.liquid {
        // Water surface as the mesher draws it (2/16 below the block top).
        let c = colors.get(l, false, s.biome);
        let w = s.biome.water_tint();
        let water = col([c[0] * 0.85 * w[0], c[1] * 0.9 * w[1], c[2] * w[2]]);
        return Cell { y: SEA_LEVEL as f32 + 0.875, canopy: 0.0, top: water, side: water };
    }
    // Land never rounds below the water it borders.
    let y = ((s.height as f32 + 1.0) / q).round() * q;
    let y = if s.height >= SEA_LEVEL { y.max(SEA_LEVEL as f32 + 1.0) } else { y };
    let mut c = Cell { y, canopy: 0.0, top: col(colors.get(s.top, false, s.biome)), side: col(colors.get(s.top, true, s.biome)) };
    if let Some((leaves, density)) = s.trees {
        // No real tree positions here: a share of cells become canopy, which reads as forest.
        if hash(wx, wz, gen.seed) < density * 0.6 {
            let k = 3.0 + hash(wz, wx, gen.seed) * 2.0;
            c.canopy = if level >= 2 { 4.0 } else { k };
            c.top = col(colors.get(leaves, false, s.biome));
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

/// Build one node: 32 × 32 cells, each a flat top plus walls where it stands above a neighbour.
/// Equal tops merge into rectangles and equal walls into strips. Node borders always get a skirt
/// so nodes of different detail meet without cracks.
pub fn build_tile(gen: &Generator, colors: &LodColors, node: Node) -> TileMesh {
    let n = CELLS as usize;
    let step = node.step();
    let o = node.origin();
    // Cells -1..=n on each axis: the ring outside the node gives the border walls their bottom.
    let w = n + 2;
    let mut cells = Vec::with_capacity(w * w);
    for j in 0..w {
        for i in 0..w {
            let wx = o.x + (i as i32 - 1) * step + step / 2;
            let wz = o.y + (j as i32 - 1) * step + step / 2;
            cells.push(cell(gen, colors, wx, wz, node.level));
        }
    }
    let at = |i: isize, j: isize| cells[(i + 1) as usize + (j + 1) as usize * w];
    let mut verts = Vec::with_capacity(n * n * 2);
    let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
    let lvl = node.level;
    let mut quad = |pts: [(i32, f32, i32); 4], raise: [f32; 4], color: [u8; 3]| {
        for k in 0..4 {
            let r = raise[k].round().clamp(0.0, 255.0);
            min_y = min_y.min(pts[k].1);
            max_y = max_y.max(pts[k].1 + r);
            verts.push(LodVertex::new(pts[k].0, pts[k].1, pts[k].2, lvl, [color[0], color[1], color[2], r as u8]));
        }
    };

    // Tops: grow each unused cell into the largest rectangle of equal cells, first along x then z.
    // Where rectangles meet edge to corner the rasterizer can leave hairline gaps; an underlay
    // below the lowest top, in the colour covering the most ground, makes them invisible.
    let (mut low, mut main, mut main_area) = (f32::MAX, [0u8; 3], 0);
    let same = |a: Cell, b: Cell| a.y == b.y && a.canopy == b.canopy && a.top == b.top;
    let mut used = vec![false; n * n];
    for j in 0..n {
        for i in 0..n {
            if used[i + j * n] {
                continue;
            }
            let c = at(i as isize, j as isize);
            let mut wd = 1;
            while i + wd < n && !used[i + wd + j * n] && same(c, at((i + wd) as isize, j as isize)) {
                wd += 1;
            }
            let mut ht = 1;
            'grow: while j + ht < n {
                for k in 0..wd {
                    if used[i + k + (j + ht) * n] || !same(c, at((i + k) as isize, (j + ht) as isize)) {
                        break 'grow;
                    }
                }
                ht += 1;
            }
            for jj in j..j + ht {
                for ii in i..i + wd {
                    used[ii + jj * n] = true;
                }
            }
            low = low.min(c.y);
            if wd * ht > main_area {
                (main, main_area) = (c.top, wd * ht);
            }
            let (x0, x1, z0, z1) = (i as i32, (i + wd) as i32, j as i32, (j + ht) as i32);
            quad([(x0, c.y, z0), (x0, c.y, z1), (x1, c.y, z1), (x1, c.y, z0)], [c.canopy; 4], c.top);
        }
    }

    let under = low - (step as f32 * 0.5).max(1.0);
    let e = CELLS;
    quad([(0, under, 0), (0, under, e), (e, under, e), (e, under, 0)], [0.0; 4], main);

    // Walls toward each lower neighbour (counting tree tops): along each edge line, runs of equal
    // walls become one quad.
    let skirt = (step as f32 * 1.5).max(6.0);
    #[derive(PartialEq, Clone, Copy)]
    struct Wall {
        top: f32,
        bottom: f32,
        traise: f32,
        braise: f32,
        color: [u8; 3],
    }
    let wall = |i: isize, j: isize, dx: isize, dz: isize, shade: f32| -> Option<Wall> {
        let c = at(i, j);
        let (ni, nj) = (i + dx, j + dz);
        let border = ni < 0 || nj < 0 || ni >= n as isize || nj >= n as isize;
        let nb = at(ni, nj);
        if nb.y + nb.canopy >= c.y + c.canopy && !border {
            return None;
        }
        // Down to the neighbour's top (both may carry canopy, which the shader scales the same
        // way), or a skirt below both at the node border.
        let (bottom, braise) = if border { (nb.y.min(c.y) - skirt, 0.0) } else { (nb.y, nb.canopy) };
        let k = shade / 255.0;
        let side = c.side.map(|v| (v as f32 * k * 255.0) as u8);
        Some(Wall { top: c.y, bottom, traise: c.canopy, braise, color: side })
    };
    let sides: [(isize, isize, f32); 4] = [(1, 0, 0.6), (-1, 0, 0.6), (0, 1, 0.8), (0, -1, 0.8)];
    for (dx, dz, shade) in sides {
        for line in 0..n as isize {
            let mut run: Option<(Wall, isize)> = None;
            for t in 0..=n as isize {
                let cur = if t < n as isize {
                    let (i, j) = if dx != 0 { (line, t) } else { (t, line) };
                    wall(i, j, dx, dz, shade)
                } else {
                    None
                };
                if let Some((r, start)) = run {
                    if cur == Some(r) {
                        continue;
                    }
                    // Emit the run [start, t) on this line.
                    let (s, e) = (start as i32, t as i32);
                    let l = line as i32;
                    let (a, b) = match (dx, dz) {
                        (1, 0) => ((l + 1, s), (l + 1, e)),
                        (-1, 0) => ((l, e), (l, s)),
                        (0, 1) => ((e, l + 1), (s, l + 1)),
                        _ => ((s, l), (e, l)),
                    };
                    quad(
                        [(a.0, r.top, a.1), (a.0, r.bottom, a.1), (b.0, r.bottom, b.1), (b.0, r.top, b.1)],
                        [r.traise, r.braise, r.braise, r.traise],
                        r.color,
                    );
                    run = None;
                }
                if let Some(c) = cur {
                    run = Some((c, t));
                }
            }
        }
    }
    TileMesh { verts, min_y, max_y }
}

pub struct GpuTile {
    pub buf: wgpu::Buffer,
    pub quads: u32,
    pub node: Node,
    pub min_y: f32,
    pub max_y: f32,
}

struct Job {
    epoch: u32,
    node: Node,
}

struct Shared {
    colors: Arc<LodColors>,
    gen: Arc<Generator>,
}

/// Nodes in range, the jobs building them, and their GPU buffers.
pub struct Lod {
    jobs: Sender<Job>,
    done: Receiver<(u32, Node, TileMesh, f64)>,
    shared: Arc<std::sync::RwLock<Shared>>,
    /// Jobs of an older epoch are skipped by the workers without building.
    live_epoch: Arc<AtomicU32>,
    pub tiles: HashMap<Node, GpuTile>,
    pending: HashSet<Node>,
    /// Built, waiting for the upload budget.
    ready: VecDeque<(Node, TileMesh)>,
    /// Nodes wanted now, nearest first.
    wanted: Vec<Node>,
    wanted_set: HashSet<Node>,
    wanted_for: Option<(IVec2, i32, i32, usize)>,
    /// Drawn nodes no longer wanted, kept until the nodes covering them are ready.
    stale: Vec<Node>,
    /// Wanted nodes left out by the video memory cap (until the next re-plan).
    over_cap: HashSet<Node>,
    /// Bumped when colours or the world change so results built with the old ones are dropped.
    epoch: u32,
    pub workers: usize,
    pub vram_bytes: u64,
    /// Where real chunks end (blocks from the eye), from the last update.
    pub inner: f32,
    /// Detail preset (index into `DETAILS`).
    pub detail: usize,
    /// Nodes built so far and the worker time they took (for the benchmark).
    pub built: usize,
    pub build_secs: f64,
}

impl Lod {
    pub fn new(gen: Arc<Generator>, colors: LodColors) -> Lod {
        let (jobs, job_rx) = unbounded::<Job>();
        let (done_tx, done) = unbounded();
        let shared = Arc::new(std::sync::RwLock::new(Shared { colors: Arc::new(colors), gen }));
        let live_epoch = Arc::new(AtomicU32::new(0));
        // Few threads: chunk generation near the player matters more. On 4 threads or fewer (weak
        // computers) a single one.
        let workers = (thread::available_parallelism().map(|n| n.get()).unwrap_or(4) / 4).clamp(1, 3);
        for i in 0..workers {
            let (rx, tx, shared, live) = (job_rx.clone(), done_tx.clone(), shared.clone(), live_epoch.clone());
            let _ = thread::Builder::new().name(format!("lod-{i}")).spawn(move || {
                while let Ok(job) = rx.recv() {
                    if job.epoch != live.load(Ordering::Relaxed) {
                        // Stale: answer so the node is no longer pending, without the work.
                        if tx.send((job.epoch, job.node, TileMesh { verts: Vec::new(), min_y: 0.0, max_y: 0.0 }, 0.0)).is_err() {
                            break;
                        }
                        continue;
                    }
                    let Ok((colors, gen)) = shared.read().map(|s| (s.colors.clone(), s.gen.clone())) else { break };
                    let t0 = std::time::Instant::now();
                    let mesh = build_tile(&gen, &colors, job.node);
                    if tx.send((job.epoch, job.node, mesh, t0.elapsed().as_secs_f64())).is_err() {
                        break;
                    }
                }
            });
        }
        Lod {
            jobs,
            done,
            shared,
            live_epoch,
            tiles: HashMap::new(),
            pending: HashSet::new(),
            ready: VecDeque::new(),
            wanted: Vec::new(),
            wanted_set: HashSet::new(),
            wanted_for: None,
            stale: Vec::new(),
            over_cap: HashSet::new(),
            epoch: 0,
            workers,
            vram_bytes: 0,
            inner: 0.0,
            detail: 1,
            built: 0,
            build_secs: 0.0,
        }
    }

    /// New textures: rebuild every node with the new colours.
    pub fn set_colors(&mut self, colors: LodColors) {
        if let Ok(mut s) = self.shared.write() {
            s.colors = Arc::new(colors);
        }
        self.reset();
    }

    /// Another world (or dimension): drop every node and build new ones from its generator.
    pub fn set_gen(&mut self, gen: Arc<Generator>) {
        if let Ok(mut s) = self.shared.write() {
            s.gen = gen;
        }
        self.reset();
    }

    fn reset(&mut self) {
        self.epoch += 1;
        self.live_epoch.store(self.epoch, Ordering::Relaxed);
        self.clear();
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
        self.ready.clear();
        self.wanted.clear();
        self.wanted_set.clear();
        self.stale.clear();
        self.over_cap.clear();
        self.wanted_for = None;
        self.vram_bytes = 0;
    }

    pub fn pending_jobs(&self) -> usize {
        self.pending.len() + self.ready.len()
    }

    /// Every node in range is built (test screenshots wait for this).
    pub fn is_complete(&self) -> bool {
        self.pending.is_empty() && self.ready.is_empty() && self.wanted.iter().all(|n| self.tiles.contains_key(n) || self.over_cap.contains(n))
    }

    fn upload(&mut self, device: &wgpu::Device, node: Node, mesh: TileMesh) {
        let bytes = std::mem::size_of_val(mesh.verts.as_slice()) as u64;
        if self.vram_bytes + bytes > DETAILS[self.detail].3 << 20 {
            // Over the cap: the farthest nodes (asked last) go without.
            self.over_cap.insert(node);
            return;
        }
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lod node"),
            contents: bytemuck::cast_slice(&mesh.verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.vram_bytes += bytes;
        self.tiles.insert(node, GpuTile { buf, quads: (mesh.verts.len() / 4) as u32, node, min_y: mesh.min_y, max_y: mesh.max_y });
    }

    fn drop_tile(&mut self, n: Node) {
        if let Some(old) = self.tiles.remove(&n) {
            self.vram_bytes -= old.buf.size();
        }
    }

    /// Keep nodes between `inner` (where real chunks end) and `outer` blocks from the eye.
    pub fn update(&mut self, device: &wgpu::Device, eye: Vec3, inner: f32, outer: f32, detail: usize) {
        self.inner = inner;
        let detail = detail.min(DETAILS.len() - 1);
        if outer <= inner {
            if !self.tiles.is_empty() || !self.pending.is_empty() || !self.ready.is_empty() {
                self.reset();
            }
            return;
        }
        if detail != self.detail {
            self.detail = detail;
            self.over_cap.clear();
        }
        while let Ok((epoch, node, mesh, secs)) = self.done.try_recv() {
            self.pending.remove(&node);
            if epoch != self.epoch || !self.wanted_set.contains(&node) {
                continue;
            }
            self.built += 1;
            self.build_secs += secs;
            self.ready.push_back((node, mesh));
        }
        // Upload what is ready, within the per-frame budget (at least one node a frame).
        let mut budget = UPLOAD_BUDGET;
        while let Some((node, mesh)) = self.ready.pop_front() {
            if !self.wanted_set.contains(&node) {
                continue;
            }
            let bytes = std::mem::size_of_val(mesh.verts.as_slice()) as u64;
            if bytes > budget && budget < UPLOAD_BUDGET {
                self.ready.push_front((node, mesh));
                break;
            }
            budget = budget.saturating_sub(bytes);
            self.drop_tile(node);
            if !mesh.verts.is_empty() {
                self.upload(device, node, mesh);
            }
        }

        // Re-plan when the eye enters another 32-block cell or the settings change.
        let cell = IVec2::new((eye.x / 32.0).floor() as i32, (eye.z / 32.0).floor() as i32);
        let key = (cell, inner as i32, outer as i32, detail);
        if self.wanted_for != Some(key) {
            self.wanted_for = Some(key);
            self.wanted = plan(Vec2::new(eye.x, eye.z), inner, outer, detail);
            self.wanted_set = self.wanted.iter().copied().collect();
            self.over_cap.clear();
            let gone: Vec<Node> = self.tiles.keys().filter(|n| !self.wanted_set.contains(n)).copied().collect();
            self.stale.extend(gone);
            self.stale.sort_by_key(|n| (n.level, n.pos.x, n.pos.y));
            self.stale.dedup();
        }
        // Old nodes go once every wanted node over them is drawn (or will never be).
        if !self.stale.is_empty() {
            let stale = std::mem::take(&mut self.stale);
            for s in stale {
                if self.wanted_set.contains(&s) {
                    continue;
                }
                let waiting = self.wanted.iter().any(|w| w.overlaps(s) && !self.tiles.contains_key(w) && !self.over_cap.contains(w));
                if waiting {
                    self.stale.push(s);
                } else {
                    self.drop_tile(s);
                }
            }
        }

        // Ask for missing nodes, nearest first, a few at a time.
        // A node takes a fraction of a millisecond, so keep the workers busy between frames.
        let max_in_flight = self.workers * 8;
        for &n in &self.wanted {
            if self.pending.len() + self.ready.len() >= max_in_flight {
                break;
            }
            if self.pending.contains(&n) || self.tiles.contains_key(&n) || self.over_cap.contains(&n) {
                continue;
            }
            self.pending.insert(n);
            let _ = self.jobs.send(Job { epoch: self.epoch, node: n });
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
    fn vertices_pack_and_unpack() {
        for level in 0..LEVELS {
            let v = LodVertex::new(32, -63.125, 17, level, [1, 2, 3, 4]);
            let p = v.unpack();
            let step = (4 << level) as f32;
            assert_eq!(p, Vec3::new(32.0 * step, -63.125, 17.0 * step));
        }
        assert_eq!(LodVertex::new(0, 255.875, 0, 0, [0; 4]).unpack().y, 255.875);
    }

    #[test]
    fn tiles_are_quads_inside_the_tile() {
        let (gen, colors) = setup();
        for level in 0..LEVELS {
            let node = Node { level, pos: IVec2::new(3, -2) };
            let m = build_tile(&gen, &colors, node);
            assert_eq!(m.verts.len() % 4, 0);
            assert!(!m.verts.is_empty());
            let s = node.size() as f32;
            for v in &m.verts {
                let p = v.unpack();
                assert!((0.0..=s).contains(&p.x) && (0.0..=s).contains(&p.z));
            }
            assert!(m.min_y <= m.max_y);
        }
    }

    #[test]
    fn tile_tops_follow_the_generator() {
        let (gen, colors) = setup();
        let m = build_tile(&gen, &colors, Node { level: 0, pos: IVec2::ZERO });
        // The first quad is the top of cell (0, 0), sampled at (2, 2).
        let s = gen.surface(2, 2);
        let expect = if s.liquid.is_some() { SEA_LEVEL as f32 + 0.875 } else { s.height as f32 + 1.0 };
        assert_eq!(m.verts[0].unpack().y, expect);
    }

    #[test]
    fn plan_covers_the_ring_without_overlaps() {
        for detail in 0..DETAILS.len() {
            let e = Vec2::new(37.0, -1200.0);
            let (inner, outer) = (184.0, 2240.0);
            let p = plan(e, inner, outer, detail);
            for (k, a) in p.iter().enumerate() {
                for b in &p[k + 1..] {
                    assert!(!a.overlaps(*b), "{a:?} {b:?}");
                }
            }
            // Every point of the ring is in some node.
            for k in 0..400 {
                let ang = k as f32 * 0.1;
                let r = inner + 10.0 + (outer - inner - 20.0) * (k as f32 / 400.0);
                let q = e + Vec2::new(ang.cos(), ang.sin()) * r;
                assert!(p.iter().any(|n| {
                    let (lo, hi) = n.rect();
                    q.cmpge(lo).all() && q.cmplt(hi).all()
                }));
            }
            // Finer near the eye than far away.
            assert!(p.first().unwrap().level <= p.last().unwrap().level);
        }
    }

    /// `cargo test --release -p game lod_report -- --ignored --nocapture`: polygons, video memory,
    /// draw calls and build time for +2048 blocks around one spot, per detail preset.
    #[test]
    #[ignore]
    fn lod_report() {
        let (gen, colors) = setup();
        let (inner, outer) = (11.5 * 16.0, 12.0 * 16.0 + 2048.0);
        for (d, preset) in DETAILS.iter().enumerate() {
            let nodes = plan(Vec2::ZERO, inner, outer, d);
            let t0 = std::time::Instant::now();
            let mut verts = 0;
            for &n in &nodes {
                verts += build_tile(&gen, &colors, n).verts.len();
            }
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            println!(
                "AFTER {}: nodes/draws {} | quads {} | vram {:.1} MB | build {:.0} ms total, {:.2} ms/node",
                preset.0,
                nodes.len(),
                verts / 4,
                (verts * std::mem::size_of::<LodVertex>()) as f64 / 1e6,
                ms,
                ms / nodes.len() as f64
            );
        }
    }
}
