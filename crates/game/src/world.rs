//! Chunk manager: decides which chunks to load around the player and runs generation, lighting and
//! meshing on worker threads. The main thread only applies results, so it never stalls on world work.
//!
//! Each chunk moves through: requested → generated (or loaded from disk) → lit → meshed.
//! Lighting needs the 8 neighbours generated; meshing needs the 8 neighbours lit.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::{unbounded, Receiver, Sender};
use glam::{IVec2, IVec3, Vec3};
use voxel::block::Shape;
use voxel::chunk::{split_xz, Chunk, ChunkPos, CHUNK_HEIGHT};
use voxel::light::compute_light;
use voxel::mesh::{mesh_section, MeshInput, MeshTables, SectionMesh, VIS_ALL};
use voxel::physics::Terrain;
use voxel::save::WorldSave;
use voxel::worldgen::Generator;
use voxel::{BlockId, BlockRegistry, AIR};

type Neighbours = [Option<Arc<Chunk>>; 9];

enum Job {
    Generate(ChunkPos),
    Light(ChunkPos, u32, Neighbours),
    Mesh(ChunkPos, u32, Neighbours),
}

enum Done {
    Generated(ChunkPos, Chunk),
    Lit(ChunkPos, u32, Vec<u8>),
    Meshed(ChunkPos, u32, Vec<(usize, SectionMesh)>),
}

#[derive(Default)]
struct Slot {
    chunk: Option<Arc<Chunk>>,
    gen_pending: bool,
    light_ver: u32,
    light_done: u32,
    light_pending: bool,
    mesh_ver: u32,
    mesh_done: u32,
    mesh_pending: bool,
    urgent: bool,
}

/// A mesh change the renderer must apply.
pub enum MeshEvent {
    /// Replace all sections of the chunk (sections not listed are empty).
    Set(ChunkPos, Vec<(usize, SectionMesh)>),
    Remove(ChunkPos),
}

pub struct World {
    pub reg: Arc<BlockRegistry>,
    pub gen: Arc<Generator>,
    pub save: Arc<WorldSave>,
    slots: HashMap<ChunkPos, Slot>,
    jobs: Sender<Job>,
    urgent_jobs: Sender<Job>,
    done: Receiver<Done>,
    in_flight: usize,
    max_in_flight: usize,
    pub workers: usize,
    pub events: Vec<MeshEvent>,
    /// Chunks edited since the last save.
    unsaved: HashSet<ChunkPos>,
    /// Chunk offsets sorted by distance, for loading nearest-first.
    spiral: Vec<IVec2>,
    spiral_radius: i32,
    pub water: BlockId,
    pub lava: BlockId,
    /// 1 for every level of water, 2 for lava, 0 otherwise.
    fluid: Vec<u8>,
    /// Blocks changed since the fluids last looked (they wake the water and lava around them).
    pub changed: Vec<IVec3>,
    solid: Vec<bool>,
    /// Collision height of each block (1.0 except slabs, beds).
    top: Vec<f32>,
    pub meshed_count: usize,
}

impl World {
    pub fn new(reg: Arc<BlockRegistry>, gen: Arc<Generator>, save: Arc<WorldSave>, tables: Arc<MeshTables>) -> World {
        let (jobs_tx, jobs_rx) = unbounded::<Job>();
        let (urgent_tx, urgent_rx) = unbounded::<Job>();
        let (done_tx, done_rx) = unbounded::<Done>();
        let workers = thread::available_parallelism().map(|n| n.get()).unwrap_or(4).saturating_sub(1).clamp(1, 16);
        for i in 0..workers {
            let (jobs_rx, urgent_rx, done_tx) = (jobs_rx.clone(), urgent_rx.clone(), done_tx.clone());
            let (reg, gen, save, tables) = (reg.clone(), gen.clone(), save.clone(), tables.clone());
            thread::Builder::new()
                .name(format!("world-{i}"))
                .spawn(move || loop {
                    // Edits by the player jump the queue.
                    let job = match urgent_rx.try_recv() {
                        Ok(j) => j,
                        Err(_) => crossbeam_channel::select! {
                            recv(urgent_rx) -> j => match j { Ok(j) => j, Err(_) => return },
                            recv(jobs_rx) -> j => match j { Ok(j) => j, Err(_) => return },
                        },
                    };
                    let out = match job {
                        Job::Generate(pos) => {
                            let c = save.load_chunk(&reg, pos).unwrap_or_else(|| gen.generate(pos));
                            Done::Generated(pos, c)
                        }
                        Job::Light(pos, ver, n) => {
                            let refs: [Option<&Chunk>; 9] = std::array::from_fn(|i| n[i].as_deref());
                            Done::Lit(pos, ver, compute_light(&reg, refs))
                        }
                        Job::Mesh(pos, ver, n) => {
                            let refs: [Option<&Chunk>; 9] = std::array::from_fn(|i| n[i].as_deref());
                            let input = MeshInput::new(refs);
                            let mut v = Vec::new();
                            for s in 0..16 {
                                if input.section_has_blocks(s) {
                                    let m = mesh_section(&tables, &input, s);
                                    // Empty but closed-off sections still matter for cave culling.
                                    if !m.is_empty() || m.vis != VIS_ALL {
                                        v.push((s, m));
                                    }
                                }
                            }
                            Done::Meshed(pos, ver, v)
                        }
                    };
                    if done_tx.send(out).is_err() {
                        return;
                    }
                })
                .expect("spawn worker");
        }
        let water = reg.id_or_air("water");
        let lava = reg.id_or_air("lava");
        let fluid = reg.blocks.iter().map(|b| if b.base == water && water != AIR { 1 } else if b.base == lava && lava != AIR { 2 } else { 0 }).collect();
        let solid = reg.blocks.iter().map(|b| b.solid && b.shape != Shape::Liquid && b.shape != Shape::Invisible).collect();
        let top = reg.blocks.iter().map(|b| b.height as f32 / 16.0).collect();
        World {
            reg,
            gen,
            save,
            slots: HashMap::new(),
            jobs: jobs_tx,
            urgent_jobs: urgent_tx,
            done: done_rx,
            in_flight: 0,
            max_in_flight: workers * 4,
            workers,
            events: Vec::new(),
            unsaved: HashSet::new(),
            spiral: Vec::new(),
            spiral_radius: -1,
            fluid,
            changed: Vec::new(),
            water,
            lava,
            solid,
            top,
            meshed_count: 0,
        }
    }

    pub fn loaded_chunks(&self) -> usize {
        self.slots.values().filter(|s| s.chunk.is_some()).count()
    }

    pub fn pending_jobs(&self) -> usize {
        self.in_flight
    }

    fn neighbours(&self, pos: ChunkPos) -> Option<Neighbours> {
        let mut out: Neighbours = Default::default();
        for i in 0..9 {
            let p = pos + IVec2::new(i as i32 % 3 - 1, i as i32 / 3 - 1);
            out[i] = Some(self.slots.get(&p)?.chunk.clone()?);
        }
        Some(out)
    }

    fn all_lit(&self, pos: ChunkPos) -> bool {
        (0..9).all(|i| {
            let p = pos + IVec2::new(i % 3 - 1, i / 3 - 1);
            self.slots.get(&p).is_some_and(|s| s.chunk.is_some() && s.light_done == s.light_ver && !s.light_pending)
        })
    }

    /// For the loading screen: 0 not started, 1 generating, 2 lighting or meshing, 3 ready.
    pub fn chunk_status(&self, pos: ChunkPos) -> u8 {
        match self.slots.get(&pos) {
            None => 0,
            Some(s) if s.mesh_done > 0 => 3,
            Some(s) if s.chunk.is_some() => 2,
            Some(s) if s.gen_pending => 1,
            Some(_) => 0,
        }
    }

    pub fn is_meshed(&self, pos: ChunkPos) -> bool {
        self.slots.get(&pos).is_some_and(|s| s.mesh_done > 0)
    }

    pub fn update(&mut self, center: ChunkPos, radius: i32) {
        // 1. Apply finished work.
        while let Ok(done) = self.done.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
            match done {
                Done::Generated(pos, c) => {
                    if let Some(s) = self.slots.get_mut(&pos) {
                        s.chunk = Some(Arc::new(c));
                        s.gen_pending = false;
                    }
                }
                Done::Lit(pos, ver, light) => {
                    let Some(s) = self.slots.get_mut(&pos) else { continue };
                    s.light_pending = false;
                    if ver != s.light_ver {
                        continue;
                    }
                    if let Some(c) = s.chunk.as_mut() {
                        Arc::make_mut(c).light = light;
                    }
                    s.light_done = ver;
                    for i in 0..9 {
                        let p = pos + IVec2::new(i % 3 - 1, i / 3 - 1);
                        if let Some(n) = self.slots.get_mut(&p) {
                            if n.mesh_done == n.mesh_ver {
                                n.mesh_ver += 1;
                            }
                        }
                    }
                }
                Done::Meshed(pos, ver, meshes) => {
                    let Some(s) = self.slots.get_mut(&pos) else { continue };
                    s.mesh_pending = false;
                    if ver != s.mesh_ver {
                        continue;
                    }
                    if s.mesh_done == 0 {
                        self.meshed_count += 1;
                    }
                    s.mesh_done = ver;
                    self.events.push(MeshEvent::Set(pos, meshes));
                }
            }
        }

        // Relight chunks touched by an edit right away, whatever else is queued.
        let urgent: Vec<ChunkPos> = self.slots.iter().filter(|(_, s)| s.urgent && !s.light_pending).map(|(p, _)| *p).collect();
        for p in urgent {
            if let Some(n) = self.neighbours(p) {
                let s = self.slots.get_mut(&p).unwrap();
                s.urgent = false;
                s.light_pending = true;
                self.in_flight += 1;
                let _ = self.urgent_jobs.send(Job::Light(p, s.light_ver, n));
            }
        }

        // 2. Request missing chunks, nearest first.
        let load_r = radius + 2;
        if self.spiral_radius != load_r {
            self.spiral.clear();
            for z in -load_r..=load_r {
                for x in -load_r..=load_r {
                    if x * x + z * z <= (load_r + 1) * (load_r + 1) {
                        self.spiral.push(IVec2::new(x, z));
                    }
                }
            }
            self.spiral.sort_by_key(|p| p.x * p.x + p.y * p.y);
            self.spiral_radius = load_r;
        }
        let r2 = |p: IVec2| {
            let d = p - center;
            d.x * d.x + d.y * d.y
        };
        let lim = |r: i32| (r + 1) * (r + 1);
        for k in 0..self.spiral.len() {
            if self.in_flight >= self.max_in_flight {
                break;
            }
            let p = center + self.spiral[k];
            let d2 = r2(p);
            let slot = self.slots.entry(p).or_insert_with(|| Slot { light_ver: 1, mesh_ver: 1, ..Default::default() });
            if slot.chunk.is_none() {
                if !slot.gen_pending {
                    slot.gen_pending = true;
                    self.in_flight += 1;
                    let _ = self.jobs.send(Job::Generate(p));
                }
                continue;
            }
            // 3. Light chunks whose neighbours exist.
            if d2 <= lim(radius + 1) && slot.light_done != slot.light_ver && !slot.light_pending {
                if let Some(n) = self.neighbours(p) {
                    let s = self.slots.get_mut(&p).unwrap();
                    s.light_pending = true;
                    let urgent = std::mem::take(&mut s.urgent);
                    self.in_flight += 1;
                    let job = Job::Light(p, s.light_ver, n);
                    let _ = if urgent { self.urgent_jobs.send(job) } else { self.jobs.send(job) };
                }
            }
        }
        // 4. Mesh chunks whose neighbours are lit (edits first, nearest first).
        for k in 0..self.spiral.len() {
            let p = center + self.spiral[k];
            if r2(p) > lim(radius) {
                break;
            }
            let Some(s) = self.slots.get(&p) else { continue };
            if s.chunk.is_none() || s.mesh_done == s.mesh_ver || s.mesh_pending {
                continue;
            }
            let urgent = s.mesh_done > 0;
            if !urgent && self.in_flight >= self.max_in_flight {
                continue;
            }
            if !self.all_lit(p) {
                continue;
            }
            let n = self.neighbours(p).unwrap();
            let s = self.slots.get_mut(&p).unwrap();
            s.mesh_pending = true;
            self.in_flight += 1;
            let job = Job::Mesh(p, s.mesh_ver, n);
            let _ = if urgent { self.urgent_jobs.send(job) } else { self.jobs.send(job) };
        }

        // 5. Unload far chunks (saving edits).
        let unload = lim(load_r + 2);
        let far: Vec<ChunkPos> = self.slots.keys().copied().filter(|&p| r2(p) > unload).collect();
        for p in far {
            let s = self.slots.get(&p).unwrap();
            if s.gen_pending || s.light_pending || s.mesh_pending {
                continue;
            }
            if self.unsaved.remove(&p) {
                if let Some(c) = &s.chunk {
                    let _ = self.save.save_chunk(&self.reg, c);
                }
            }
            if s.mesh_done > 0 {
                self.meshed_count -= 1;
                self.events.push(MeshEvent::Remove(p));
            }
            self.slots.remove(&p);
        }
    }

    pub fn chunk(&self, pos: ChunkPos) -> Option<&Arc<Chunk>> {
        self.slots.get(&pos)?.chunk.as_ref()
    }

    pub fn block(&self, p: IVec3) -> Option<BlockId> {
        if p.y < 0 || p.y >= CHUNK_HEIGHT as i32 {
            return Some(AIR);
        }
        let (cp, x, z) = split_xz(p.x, p.z);
        Some(self.chunk(cp)?.get(x, p.y as usize, z))
    }

    pub fn block_or_air(&self, p: IVec3) -> BlockId {
        self.block(p).unwrap_or(AIR)
    }

    pub fn light(&self, p: IVec3) -> u8 {
        if p.y < 0 || p.y >= CHUNK_HEIGHT as i32 {
            return 0xF0;
        }
        let (cp, x, z) = split_xz(p.x, p.z);
        self.chunk(cp).map(|c| c.light_at(x, p.y as usize, z)).unwrap_or(0xF0)
    }

    /// Change a block and schedule relighting/remeshing. Returns false if the chunk isn't loaded.
    pub fn set_block(&mut self, p: IVec3, id: BlockId) -> bool {
        if p.y < 0 || p.y >= CHUNK_HEIGHT as i32 {
            return false;
        }
        let (cp, x, z) = split_xz(p.x, p.z);
        let Some(slot) = self.slots.get_mut(&cp) else { return false };
        let Some(c) = slot.chunk.as_mut() else { return false };
        let c = Arc::make_mut(c);
        c.set(x, p.y as usize, z, id);
        c.modified = true;
        if self.changed.len() < 100_000 {
            self.changed.push(p);
        }
        self.unsaved.insert(cp);
        for i in 0..9 {
            let n = cp + IVec2::new(i % 3 - 1, i / 3 - 1);
            if let Some(s) = self.slots.get_mut(&n) {
                if s.chunk.is_some() {
                    s.light_ver += 1;
                    s.urgent = true;
                }
            }
        }
        true
    }

    /// Remove a block; sand and gravel above it fall down.
    pub fn break_block(&mut self, p: IVec3) -> bool {
        let was = self.block_or_air(p);
        if !self.set_block(p, AIR) {
            return false;
        }
        // A door goes as a whole.
        let wb = self.reg.get(was);
        if wb.shape == Shape::Door {
            let other = if wb.variant & 8 != 0 { p - IVec3::Y } else { p + IVec3::Y };
            let o = self.block_or_air(other);
            if self.reg.get(o).base == wb.base && o != AIR {
                self.set_block(other, AIR);
            }
        }
        let mut y = p.y + 1;
        loop {
            let above = IVec3::new(p.x, y, p.z);
            let id = self.block_or_air(above);
            if id == AIR || !self.reg.get(id).gravity {
                break;
            }
            // Drop it to the first supporting block.
            let mut dest = above - IVec3::Y;
            while dest.y > 0 && self.block_or_air(dest - IVec3::Y) == AIR {
                dest.y -= 1;
            }
            self.set_block(above, AIR);
            self.set_block(dest, id);
            y += 1;
        }
        true
    }

    /// Place a gravity block so it lands on the ground.
    pub fn place_block(&mut self, mut p: IVec3, id: BlockId) -> bool {
        if self.reg.get(id).gravity {
            while p.y > 0 && matches!(self.block(p - IVec3::Y), Some(AIR)) {
                p.y -= 1;
            }
        }
        self.set_block(p, id)
    }

    pub fn is_liquid(&self, id: BlockId) -> bool {
        self.fluid[id as usize] != 0
    }

    /// 1 water, 2 lava, 0 anything else (any level of flow counts).
    pub fn fluid(&self, id: BlockId) -> u8 {
        self.fluid[id as usize]
    }

    pub fn save_all(&mut self) {
        for p in std::mem::take(&mut self.unsaved) {
            if let Some(c) = self.chunk(p) {
                if let Err(e) = self.save.save_chunk(&self.reg, c) {
                    log::error!("could not save chunk {p}: {e}");
                }
            }
        }
    }

    /// Highest solid block at a column if it's loaded.
    pub fn surface_y(&self, x: i32, z: i32) -> Option<i32> {
        let (cp, lx, lz) = split_xz(x, z);
        let c = self.chunk(cp)?;
        for y in (0..CHUNK_HEIGHT as i32).rev() {
            let id = c.get(lx, y as usize, lz);
            if id != AIR && (self.solid[id as usize] || self.is_liquid(id)) {
                return Some(y);
            }
        }
        Some(0)
    }
}

impl Terrain for World {
    fn solid(&self, p: IVec3) -> bool {
        match self.block(p) {
            // Unloaded chunks act like walls so you can't fall out of the world.
            None => true,
            Some(id) => self.solid[id as usize],
        }
    }
    fn liquid(&self, p: IVec3) -> bool {
        self.block(p).is_some_and(|id| self.fluid[id as usize] == 1)
    }
    fn top(&self, p: IVec3) -> f32 {
        self.block(p).map(|id| self.top[id as usize]).unwrap_or(1.0)
    }
    fn boxes(&self, p: IVec3, out: &mut Vec<[f32; 6]>) {
        let Some(id) = self.block(p) else {
            out.push([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
            return;
        };
        if !self.solid[id as usize] {
            return;
        }
        let b = self.reg.get(id);
        if b.shape.is_boxes() {
            let mut v = Vec::new();
            voxel::shapes::boxes(b.shape, b.variant, self.connections(p, b.shape), true, &mut v);
            out.extend(v.iter().map(|q| q.map(|c| c as f32 / 16.0)));
        } else {
            out.push([0.0, 0.0, 0.0, 1.0, self.top[id as usize], 1.0]);
        }
    }
}

impl World {
    /// Ray against the boxes of a shaped block (stairs, slabs, doors...) at `p`: distance and
    /// the normal of the face hit. None for other blocks or when the ray passes beside them.
    pub fn ray_shape(&self, o: Vec3, d: Vec3, p: IVec3) -> Option<(f32, IVec3)> {
        let id = self.block(p)?;
        let b = self.reg.get(id);
        if !b.shape.is_boxes() {
            return None;
        }
        let mut v = Vec::new();
        voxel::shapes::boxes(b.shape, b.variant, self.connections(p, b.shape), false, &mut v);
        let inv = Vec3::ONE / d;
        let mut best: Option<(f32, IVec3)> = None;
        for q in v {
            let lo = p.as_vec3() + Vec3::new(q[0] as f32, q[1] as f32, q[2] as f32) / 16.0;
            let hi = p.as_vec3() + Vec3::new(q[3] as f32, q[4] as f32, q[5] as f32) / 16.0;
            let t1 = (lo - o) * inv;
            let t2 = (hi - o) * inv;
            let near = t1.min(t2);
            let tmin = near.max_element();
            let tmax = t1.max(t2).min_element();
            if tmax < tmin.max(0.0) || best.is_some_and(|b| b.0 <= tmin) {
                continue;
            }
            let axis = if near.x >= near.y && near.x >= near.z { 0 } else if near.y >= near.z { 1 } else { 2 };
            let mut n = IVec3::ZERO;
            n[axis] = if d[axis] > 0.0 { -1 } else { 1 };
            best = Some((tmin.max(0.0), n));
        }
        best
    }

    /// Which sides a fence, wall or pane at `p` joins (see `voxel::shapes`).
    pub fn connections(&self, p: IVec3, shape: Shape) -> u8 {
        use voxel::shapes::{joins, CONN_NX, CONN_NZ, CONN_PX, CONN_PZ};
        if !matches!(shape, Shape::Fence | Shape::Wall | Shape::Pane) {
            return 0;
        }
        let mut c = 0;
        for (bit, o) in [(CONN_PX, IVec3::X), (CONN_NX, -IVec3::X), (CONN_PZ, IVec3::Z), (CONN_NZ, -IVec3::Z)] {
            let n = self.block_or_air(p + o);
            if joins(shape, self.reg.get(n).shape, self.reg.is_opaque(n)) {
                c |= bit;
            }
        }
        c
    }
}
