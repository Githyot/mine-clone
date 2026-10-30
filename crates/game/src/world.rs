//! Chunk manager: decides which chunks to load around the player and runs generation, lighting and
//! meshing on worker threads. The main thread only applies results, so it never stalls on world work.
//!
//! Each chunk moves through: requested → generated (or loaded from disk) → lit → meshed.
//! Lighting needs the 8 neighbours generated; meshing needs the 8 neighbours lit.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam_channel::{unbounded, Receiver, Sender};
use glam::{IVec2, IVec3, Vec3};
use voxel::block::Shape;
use voxel::chunk::{local_y, split_xz, Chunk, ChunkPos, BASE_SECTION, CHUNK_HEIGHT, MIN_Y, SECTIONS};
use voxel::light::compute_light;
use voxel::mesh::{mesh_section, MeshInput, MeshTables, SectionMesh, VIS_ALL};
use voxel::physics::Terrain;
use voxel::save::WorldSave;
use voxel::worldgen::Generator;
use voxel::{BlockId, BlockRegistry, AIR};

type Neighbours = [Option<Arc<Chunk>>; 9];

/// Local sections `lo..hi` that are lit and drawn (see `World::set_focus`).
type Band = (usize, usize);

enum Job {
    /// Make (or load) a chunk with its depths down to this local section.
    Generate(ChunkPos, usize),
    /// Generate local sections `lo..hi` of a chunk's depths (someone is going down there).
    Deepen(ChunkPos, usize, usize),
    Light(ChunkPos, u32, Neighbours, Band),
    /// The blocks left out (a door that is swinging is drawn on its own meanwhile).
    Mesh(ChunkPos, u32, Neighbours, Vec<IVec3>, Band),
}

enum Done {
    /// The chunk, and whether it is new from the seed (not read from disk).
    Generated(ChunkPos, Chunk, bool),
    Deepened(ChunkPos, usize, usize, Vec<voxel::chunk::PalettedSection>),
    Lit(ChunkPos, u32, Vec<u8>, Band),
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
    deep_pending: bool,
}

/// A mesh change the renderer must apply.
pub enum MeshEvent {
    /// Replace all sections of the chunk (sections not listed are empty).
    Set(ChunkPos, Vec<(usize, SectionMesh)>),
    /// Replace only these sections of an already drawn chunk (None = now empty).
    Sections(ChunkPos, Vec<(usize, Option<SectionMesh>)>),
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
    /// Chunks edited (or newly generated) since the last save.
    unsaved: HashSet<ChunkPos>,
    /// Chunks handed to the saver thread and not yet on disk. Loading looks here first, so a
    /// chunk that comes back into view before its file is written is never generated anew.
    saving: Arc<Mutex<HashMap<ChunkPos, Arc<Chunk>>>>,
    saver: Sender<Arc<Chunk>>,
    /// Chunk offsets sorted by distance, for loading nearest-first.
    spiral: Vec<IVec2>,
    spiral_radius: i32,
    /// 1 for every level of water, 2 for lava, 0 otherwise.
    fluid: Vec<u8>,
    /// Blocks changed since the fluids last looked (they wake the water and lava around them).
    pub changed: Vec<IVec3>,
    solid: Vec<bool>,
    /// Collision height of each block (1.0 except slabs, beds).
    top: Vec<f32>,
    /// Ladders and vines.
    climb: Vec<bool>,
    /// Cobwebs: bodies inside crawl through (see `Terrain::sticky`).
    sticky: Vec<bool>,
    /// How each block feels underfoot (ice slides, sand and snow drag).
    ground: Vec<voxel::physics::Ground>,
    /// Blocks that stand in water (kelp): bodies swim through them.
    wet: Vec<bool>,
    pub meshed_count: usize,
    /// What the mesh workers draw with (shared with them; resource pack models swap inside).
    pub tables: Arc<MeshTables>,
    /// The dedicated server draws nothing: lit chunks count as meshed without building meshes.
    pub headless: bool,
    /// Multiplayer: every block change, for the network to pass on (None = not recording).
    pub net_log: Option<Vec<(IVec3, BlockId)>>,
    /// Multiplayer host: other players' chunks, kept loaded and lit (not meshed) around them.
    pub extra_centers: Vec<(ChunkPos, i32)>,
    /// Doors, trapdoors and gates that just opened or closed: (where, before, after), for
    /// their swing (`swing.rs`).
    pub swings: Vec<(IVec3, BlockId, BlockId)>,
    /// Blocks the chunk meshes leave out while they are drawn moving.
    hidden: HashSet<IVec3>,
    /// Blocks the player just placed or broke, whose sections are meshed on the spot.
    edits: Vec<IVec3>,
    /// The sections lit and drawn: a band around the player's height, so a 1024-block world
    /// costs no more light and mesh work than a 256-block one.
    pub band: Band,
    /// Blocks with a moving model drawn over them (windmill sails, tall banners), by block id.
    spins: Vec<bool>,
    /// Where those blocks are in the loaded chunks, kept up to date as chunks come and go and
    /// blocks change, so drawing them never has to search the world.
    pub spinners: HashMap<ChunkPos, Vec<IVec3>>,
}

/// The band for a player in local section `ps`: about a hundred blocks below, more above (all the
/// way up to the sky near the surface, where mountains far away show).
fn band_for(ps: usize) -> Band {
    let lo = ps.saturating_sub(7) / 4 * 4;
    let hi = (ps + 13) / 4 * 4;
    (lo, if hi >= BASE_SECTION + 2 { SECTIONS } else { hi.min(SECTIONS) })
}

impl World {
    pub fn new(reg: Arc<BlockRegistry>, gen: Arc<Generator>, save: Arc<WorldSave>, tables: Arc<MeshTables>) -> World {
        let (jobs_tx, jobs_rx) = unbounded::<Job>();
        let (urgent_tx, urgent_rx) = unbounded::<Job>();
        let (done_tx, done_rx) = unbounded::<Done>();
        let workers = thread::available_parallelism().map(|n| n.get()).unwrap_or(4).saturating_sub(1).clamp(1, 16);
        let saving: Arc<Mutex<HashMap<ChunkPos, Arc<Chunk>>>> = Arc::default();
        let (saver_tx, saver_rx) = unbounded::<Arc<Chunk>>();
        {
            let (reg, save, saving) = (reg.clone(), save.clone(), saving.clone());
            thread::Builder::new()
                .name("world-saver".into())
                .spawn(move || {
                    for c in saver_rx {
                        if let Err(e) = save.save_chunk(&reg, &c) {
                            log::error!("could not save chunk {}: {e}", c.pos);
                        }
                        let mut m = saving.lock().unwrap_or_else(|e| e.into_inner());
                        if m.get(&c.pos).is_some_and(|q| Arc::ptr_eq(q, &c)) {
                            m.remove(&c.pos);
                        }
                    }
                })
                .expect("spawn saver");
        }
        for i in 0..workers {
            let (jobs_rx, urgent_rx, done_tx) = (jobs_rx.clone(), urgent_rx.clone(), done_tx.clone());
            let (reg, gen, save, tables, saving) = (reg.clone(), gen.clone(), save.clone(), tables.clone(), saving.clone());
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
                        Job::Generate(pos, lo) => {
                            let queued = saving.lock().unwrap_or_else(|e| e.into_inner()).get(&pos).cloned();
                            match queued {
                                Some(c) => Done::Generated(pos, (*c).clone(), false),
                                None => {
                                    let (c, fresh) = load_or_generate_fresh(&save, &gen, &reg, pos, lo);
                                    Done::Generated(pos, c, fresh)
                                }
                            }
                        }
                        Job::Deepen(pos, lo, hi) => Done::Deepened(pos, lo, hi, gen.deep_sections(pos, lo, hi)),
                        Job::Light(pos, ver, n, band) => {
                            let refs: [Option<&Chunk>; 9] = std::array::from_fn(|i| n[i].as_deref());
                            Done::Lit(pos, ver, compute_light(&reg, refs, band.0, band.1), band)
                        }
                        Job::Mesh(pos, ver, n, hidden, band) => {
                            let refs: [Option<&Chunk>; 9] = std::array::from_fn(|i| n[i].as_deref());
                            let mut input = MeshInput::band(refs, band.0, band.1);
                            for h in hidden {
                                input.hide(h.x - pos.x * 16, h.y, h.z - pos.y * 16);
                            }
                            let mut v = Vec::new();
                            for s in band.0..band.1 {
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
        let climb = reg.blocks.iter().map(|b| b.shape == Shape::Ladder || matches!(b.name.as_str(), "vine" | "weeping_vines" | "twisting_vines" | "scaffolding" | "cave_vines")).collect();
        let sticky = reg.blocks.iter().map(|b| b.name == "cobweb").collect();
        let wet = reg.blocks.iter().map(|b| b.waterlogged).collect();
        let ground = reg.blocks.iter().map(|b| voxel::physics::Ground::of_block(&reg.blocks[b.base as usize].name, b.variant)).collect();
        let spins = reg.blocks.iter().map(|b| reg.blocks[b.base as usize].name == "windmill_sails" || b.shape == Shape::TallBanner).collect();
        World {
            tables,
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
            saving,
            saver: saver_tx,
            spiral: Vec::new(),
            spiral_radius: -1,
            fluid,
            changed: Vec::new(),
            solid,
            top,
            climb,
            sticky,
            ground,
            wet,
            meshed_count: 0,
            headless: false,
            net_log: None,
            extra_centers: Vec::new(),
            swings: Vec::new(),
            hidden: HashSet::new(),
            edits: Vec::new(),
            band: band_for(BASE_SECTION + 4),
            spins,
            spinners: HashMap::new(),
        }
    }

    /// Follow the player's height: when they get near the edge of the lit band, move it and
    /// light and draw every chunk again for the new band (old meshes stay until then).
    pub fn set_focus(&mut self, y: f32) {
        let ps = local_y((y.floor() as i32).clamp(MIN_Y, CHUNK_HEIGHT as i32 - 1)) / 16;
        let (lo, hi) = self.band;
        let near_bottom = lo > 0 && ps < lo + 3;
        let near_top = hi < SECTIONS && ps + 4 > hi;
        if !near_bottom && !near_top {
            return;
        }
        let band = band_for(ps);
        if band == self.band {
            return;
        }
        self.band = band;
        for s in self.slots.values_mut() {
            if s.chunk.is_some() {
                s.light_ver += 1;
            }
        }
    }

    /// Note down the turning blocks of a chunk that just arrived.
    fn find_spinners(&mut self, c: &Chunk) {
        self.spinners.remove(&c.pos);
        let spins = &self.spins;
        if !spins.iter().any(|&b| b) {
            return;
        }
        let mut found = Vec::new();
        for (si, sec) in c.sections.iter().enumerate() {
            if sec.is_empty() || !sec.may_contain(|b| spins[b as usize]) {
                continue;
            }
            for y in si * 16..si * 16 + 16 {
                for z in 0..16 {
                    for x in 0..16 {
                        if spins[c.get_local(x, y, z) as usize] {
                            found.push(IVec3::new(c.pos.x * 16 + x as i32, y as i32 + MIN_Y, c.pos.y * 16 + z as i32));
                        }
                    }
                }
            }
        }
        if !found.is_empty() {
            self.spinners.insert(c.pos, found);
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

    /// The lowest local section of the depths chunks need right now (the band and a margin).
    fn deep_target(&self) -> usize {
        self.band.0.saturating_sub(2)
    }

    /// The chunk and its neighbours have their depths made as far down as lighting looks.
    fn deep_ready(&self, pos: ChunkPos) -> bool {
        let need = self.band.0.saturating_sub(1);
        (0..9).all(|i| {
            let p = pos + IVec2::new(i % 3 - 1, i / 3 - 1);
            self.slots.get(&p).and_then(|s| s.chunk.as_ref()).is_none_or(|c| c.deep_lo <= need)
        })
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
                Done::Generated(pos, c, fresh) => {
                    if let Some(s) = self.slots.get_mut(&pos) {
                        // A chunk that came over the network meanwhile wins.
                        if s.chunk.is_none() {
                            let c = Arc::new(c);
                            s.chunk = Some(c.clone());
                            s.gen_pending = false;
                            // Every visited chunk is kept, so a newer version of the terrain
                            // generator never reshapes places the player has already seen.
                            if fresh {
                                self.unsaved.insert(pos);
                            }
                            self.find_spinners(&c);
                        } else {
                            s.gen_pending = false;
                        }
                    }
                }
                Done::Deepened(pos, lo, hi, secs) => {
                    let Some(s) = self.slots.get_mut(&pos) else { continue };
                    s.deep_pending = false;
                    let Some(c) = s.chunk.as_mut() else { continue };
                    if c.deep_lo != hi {
                        continue;
                    }
                    let c = Arc::make_mut(c);
                    for (k, sec) in secs.into_iter().enumerate() {
                        c.sections[lo + k] = sec;
                    }
                    c.deep_lo = lo;
                    self.unsaved.insert(pos);
                    for i in 0..9 {
                        let p = pos + IVec2::new(i % 3 - 1, i / 3 - 1);
                        if let Some(n) = self.slots.get_mut(&p) {
                            if n.chunk.is_some() {
                                n.light_ver += 1;
                            }
                        }
                    }
                }
                Done::Lit(pos, ver, light, band) => {
                    let Some(s) = self.slots.get_mut(&pos) else { continue };
                    s.light_pending = false;
                    if ver != s.light_ver {
                        continue;
                    }
                    // Only chunks whose mesh sees light that changed are remeshed: an edit out in
                    // the open often changes none (the edit itself asks for its own chunks).
                    let first = s.light_done == 0;
                    let old = s.chunk.as_ref().map(|c| (&c.light, (c.light_lo, c.light_hi)));
                    let (lo, hi) = match old {
                        Some((old, ob)) if !first && ob == band && old.len() == light.len() => light_change(old, &light),
                        _ => Some(([0, 0], [15, 15])),
                    }
                    .map_or((IVec2::ONE, IVec2::NEG_ONE), |(a, b)| (IVec2::from(a), IVec2::from(b)));
                    if lo.x <= hi.x {
                        if let Some(c) = s.chunk.as_mut() {
                            let c = Arc::make_mut(c);
                            c.light = light;
                            (c.light_lo, c.light_hi) = band;
                        }
                    }
                    s.light_done = ver;
                    for i in 0..9 {
                        let o = IVec2::new(i % 3 - 1, i / 3 - 1);
                        // The neighbour's mesh border is our edge row facing it.
                        let near = |o: i32, lo: i32, hi: i32| match o {
                            -1 => lo == 0,
                            1 => hi == 15,
                            _ => true,
                        };
                        if lo.x > hi.x || !near(o.x, lo.x, hi.x) || !near(o.y, lo.y, hi.y) {
                            continue;
                        }
                        let p = pos + o;
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
        let urgent: Vec<ChunkPos> = self.slots.iter().filter(|(p, s)| s.urgent && !s.light_pending && self.deep_ready(**p)).map(|(p, _)| *p).collect();
        for p in urgent {
            if let Some(n) = self.neighbours(p) {
                let s = self.slots.get_mut(&p).unwrap();
                s.urgent = false;
                s.light_pending = true;
                self.in_flight += 1;
                let _ = self.urgent_jobs.send(Job::Light(p, s.light_ver, n, self.band));
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
            let deep_to = self.deep_target();
            let slot = self.slots.entry(p).or_insert_with(|| Slot { light_ver: 1, mesh_ver: 1, ..Default::default() });
            if slot.chunk.is_none() {
                if !slot.gen_pending {
                    slot.gen_pending = true;
                    self.in_flight += 1;
                    let _ = self.jobs.send(Job::Generate(p, deep_to));
                }
                continue;
            }
            // The depths the band reaches into are made the first time they are needed.
            let have = slot.chunk.as_ref().map_or(0, |c| c.deep_lo);
            if have > deep_to && !slot.deep_pending {
                slot.deep_pending = true;
                self.in_flight += 1;
                let _ = self.jobs.send(Job::Deepen(p, deep_to, have));
                continue;
            }
            // 3. Light chunks whose neighbours exist.
            if d2 <= lim(radius + 1) && slot.light_done != slot.light_ver && !slot.light_pending && self.deep_ready(p) {
                if let Some(n) = self.neighbours(p) {
                    let s = self.slots.get_mut(&p).unwrap();
                    s.light_pending = true;
                    let urgent = std::mem::take(&mut s.urgent);
                    self.in_flight += 1;
                    let job = Job::Light(p, s.light_ver, n, self.band);
                    let _ = if urgent { self.urgent_jobs.send(job) } else { self.jobs.send(job) };
                }
            }
        }
        // Other players' surroundings (multiplayer host): generated and lit, never meshed.
        for (c, r) in self.extra_centers.clone() {
            for k in 0..self.spiral.len() {
                let off = self.spiral[k];
                if off.x * off.x + off.y * off.y > lim(r + 1) {
                    break;
                }
                if self.in_flight >= self.max_in_flight {
                    break;
                }
                let p = c + off;
                let slot = self.slots.entry(p).or_insert_with(|| Slot { light_ver: 1, mesh_ver: 1, ..Default::default() });
                if slot.chunk.is_none() {
                    if !slot.gen_pending {
                        slot.gen_pending = true;
                        self.in_flight += 1;
                        let _ = self.jobs.send(Job::Generate(p, self.band.0.saturating_sub(2)));
                    }
                    continue;
                }
                if off.x * off.x + off.y * off.y <= lim(r) && slot.light_done != slot.light_ver && !slot.light_pending && self.deep_ready(p) {
                    if let Some(n) = self.neighbours(p) {
                        let s = self.slots.get_mut(&p).unwrap();
                        s.light_pending = true;
                        s.urgent = false;
                        self.in_flight += 1;
                        let _ = self.jobs.send(Job::Light(p, s.light_ver, n, self.band));
                    }
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
            if self.headless {
                let s = self.slots.get_mut(&p).unwrap();
                if s.mesh_done == 0 {
                    self.meshed_count += 1;
                }
                s.mesh_done = s.mesh_ver;
                continue;
            }
            let n = self.neighbours(p).unwrap();
            let s = self.slots.get_mut(&p).unwrap();
            s.mesh_pending = true;
            self.in_flight += 1;
            let hidden: Vec<IVec3> = self.hidden.iter().copied().filter(|h| (h.x.div_euclid(16) - p.x).abs() <= 1 && (h.z.div_euclid(16) - p.y).abs() <= 1).collect();
            let job = Job::Mesh(p, s.mesh_ver, n, hidden, self.band);
            let _ = if urgent { self.urgent_jobs.send(job) } else { self.jobs.send(job) };
        }

        // 5. Unload far chunks (saving edits).
        let unload = lim(load_r + 2);
        let extra = &self.extra_centers;
        let near_other = |p: ChunkPos| {
            extra.iter().any(|&(c, r)| {
                let d = p - c;
                d.x * d.x + d.y * d.y <= (r + 3) * (r + 3)
            })
        };
        let far: Vec<ChunkPos> = self.slots.keys().copied().filter(|&p| r2(p) > unload && !near_other(p)).collect();
        for p in far {
            let s = self.slots.get(&p).unwrap();
            if s.gen_pending || s.light_pending || s.mesh_pending || s.deep_pending {
                continue;
            }
            if self.unsaved.remove(&p) {
                if let Some(c) = &s.chunk {
                    self.queue_save(c.clone());
                }
            }
            if s.mesh_done > 0 {
                self.meshed_count -= 1;
                self.events.push(MeshEvent::Remove(p));
            }
            self.slots.remove(&p);
            self.spinners.remove(&p);
        }
    }

    /// Put in a whole chunk (sent by a multiplayer host), relighting and remeshing around it.
    pub fn replace_chunk(&mut self, chunk: Chunk) {
        let cp = chunk.pos;
        self.find_spinners(&chunk);
        let slot = self.slots.entry(cp).or_insert_with(|| Slot { light_ver: 1, mesh_ver: 1, ..Default::default() });
        slot.chunk = Some(Arc::new(chunk));
        self.unsaved.insert(cp);
        for i in 0..9 {
            let n = cp + IVec2::new(i % 3 - 1, i / 3 - 1);
            if let Some(s) = self.slots.get_mut(&n) {
                if s.chunk.is_some() {
                    s.light_ver += 1;
                    s.mesh_ver += 1;
                    s.urgent = true;
                }
            }
        }
    }

    pub fn chunk(&self, pos: ChunkPos) -> Option<&Arc<Chunk>> {
        self.slots.get(&pos)?.chunk.as_ref()
    }

    pub fn block(&self, p: IVec3) -> Option<BlockId> {
        if p.y < MIN_Y || p.y >= CHUNK_HEIGHT as i32 {
            return Some(AIR);
        }
        let (cp, x, z) = split_xz(p.x, p.z);
        Some(self.chunk(cp)?.get_w(x, p.y, z))
    }

    pub fn block_or_air(&self, p: IVec3) -> BlockId {
        self.block(p).unwrap_or(AIR)
    }

    pub fn light(&self, p: IVec3) -> u8 {
        if p.y >= CHUNK_HEIGHT as i32 {
            return 0xF0;
        }
        if p.y < MIN_Y {
            return 0;
        }
        let (cp, x, z) = split_xz(p.x, p.z);
        self.chunk(cp).map(|c| c.light_w(x, p.y, z)).unwrap_or(0xF0)
    }

    /// Change a block and schedule relighting/remeshing. Returns false if the chunk isn't loaded.
    pub fn set_block(&mut self, p: IVec3, id: BlockId) -> bool {
        if p.y < MIN_Y || p.y >= CHUNK_HEIGHT as i32 {
            return false;
        }
        let (cp, x, z) = split_xz(p.x, p.z);
        let Some(slot) = self.slots.get_mut(&cp) else { return false };
        let Some(c) = slot.chunk.as_mut() else { return false };
        // Depths not made yet stay as they are until they are.
        if local_y(p.y) < c.deep_lo * 16 {
            return false;
        }
        let c = Arc::make_mut(c);
        let was = c.get_w(x, p.y, z);
        c.set_w(x, p.y, z, id);
        if self.spins[was as usize] != self.spins[id as usize] {
            let list = self.spinners.entry(cp).or_default();
            list.retain(|&q| q != p);
            if self.spins[id as usize] {
                list.push(p);
            }
        }
        if was != id && self.swings.len() < 64 {
            let (a, b) = (self.reg.get(was), self.reg.get(id));
            if a.base == b.base && a.variant ^ b.variant == 4 && matches!(a.shape, Shape::Door | Shape::Trapdoor | Shape::FenceGate) {
                self.swings.push((p, was, id));
            }
        }
        c.modified = true;
        if let Some(log) = self.net_log.as_mut() {
            log.push((p, id));
        }
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
                    // Remesh the chunks whose mesh border holds the block; a mesh still in
                    // flight is thrown away when it comes back.
                    let (dx, dz) = (p.x - n.x * 16, p.z - n.y * 16);
                    if (-1..=16).contains(&dx) && (-1..=16).contains(&dz) {
                        s.mesh_ver += 1;
                    }
                }
            }
        }
        true
    }

    /// Show the change at `p` in this very frame: its sections are meshed on the main thread at
    /// the next `remesh_edits` (with the old light; the workers bring the new light soon after).
    pub fn mesh_now(&mut self, p: IVec3) {
        if self.edits.len() < 8 && !self.edits.contains(&p) {
            self.edits.push(p);
        }
    }

    /// Mesh the sections around the blocks given to `mesh_now`, a few at most, so placing or
    /// breaking a block never waits for the workers.
    pub fn remesh_edits(&mut self) {
        let edits = std::mem::take(&mut self.edits);
        if edits.is_empty() || self.headless {
            return;
        }
        // Sections touched, by chunk (a block on an edge shows in its neighbours' meshes too).
        let mut todo: Vec<(ChunkPos, Vec<usize>)> = Vec::new();
        for &p in &edits {
            for d in [IVec3::ZERO, IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z] {
                let q = p + d;
                if q.y < MIN_Y || q.y >= CHUNK_HEIGHT as i32 {
                    continue;
                }
                let cp = split_xz(q.x, q.z).0;
                let s = local_y(q.y) >> 4;
                if !(self.band.0..self.band.1).contains(&s) {
                    continue;
                }
                if !self.slots.get(&cp).is_some_and(|s| s.mesh_done > 0) {
                    continue;
                }
                match todo.iter_mut().find(|t| t.0 == cp) {
                    Some(t) if !t.1.contains(&s) => t.1.push(s),
                    Some(_) => {}
                    None => todo.push((cp, vec![s])),
                }
            }
        }
        for (cp, secs) in todo {
            let Some(n) = self.neighbours(cp) else { continue };
            let refs: [Option<&Chunk>; 9] = std::array::from_fn(|i| n[i].as_deref());
            let (lo, hi) = (*secs.iter().min().unwrap(), *secs.iter().max().unwrap());
            let mut input = MeshInput::new_sections(refs, lo, hi);
            for h in &self.hidden {
                input.hide(h.x - cp.x * 16, h.y, h.z - cp.y * 16);
            }
            for &p in &edits {
                if self.block_or_air(p) == AIR {
                    input.guess_light(p.x - cp.x * 16, p.y, p.z - cp.y * 16);
                }
            }
            let out = secs
                .into_iter()
                .map(|s| {
                    let m = input.section_has_blocks(s).then(|| mesh_section(&self.tables, &input, s));
                    (s, m.filter(|m| !m.is_empty() || m.vis != VIS_ALL))
                })
                .collect();
            self.events.push(MeshEvent::Sections(cp, out));
        }
    }

    /// Leave a block out of the chunk meshes (or put it back), remeshing the chunks it touches.
    pub fn set_hidden(&mut self, p: IVec3, hide: bool) {
        let changed = if hide { self.hidden.insert(p) } else { self.hidden.remove(&p) };
        if !changed {
            return;
        }
        let mut cps = Vec::new();
        for d in [IVec3::ZERO, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
            let cp = split_xz(p.x + d.x, p.z + d.z).0;
            if !cps.contains(&cp) {
                cps.push(cp);
            }
        }
        for cp in cps {
            if let Some(s) = self.slots.get_mut(&cp) {
                if s.mesh_done > 0 {
                    // A mesh still in flight is thrown away when it comes back.
                    s.mesh_ver += 1;
                }
            }
        }
    }

    /// Mesh every loaded chunk again (the block models changed).
    pub fn remesh_all(&mut self) {
        for s in self.slots.values_mut() {
            if s.mesh_done > 0 {
                s.mesh_ver += 1;
            }
        }
    }

    /// Remove a block; sand and gravel above it fall down.
    pub fn break_block(&mut self, p: IVec3) -> bool {
        let was = self.block_or_air(p);
        if !self.set_block(p, AIR) {
            return false;
        }
        // A door goes as a whole.
        let wb = self.reg.get(was);
        let was_frame = wb.shape == Shape::Portal || wb.name == "obsidian";
        let (shape, variant) = (wb.shape, wb.variant);
        if wb.shape == Shape::Door {
            let other = if wb.variant & 8 != 0 { p - IVec3::Y } else { p + IVec3::Y };
            let o = self.block_or_air(other);
            if self.reg.get(o).base == wb.base && o != AIR {
                self.set_block(other, AIR);
            }
        }
        // So does a bed.
        if shape == Shape::Bed {
            let [dx, dz] = voxel::bed::other_half(variant);
            let other = p + IVec3::new(dx, 0, dz);
            let o = self.block_or_air(other);
            let pair = self.reg.get(o).shape == Shape::Bed && self.reg.get(o).variant == variant ^ voxel::bed::HEAD;
            if pair {
                self.set_block(other, AIR);
            }
        }
        // Half of a double chest: the other half is a chest on its own again.
        let (turn, base) = (self.reg.get(was).turn, self.reg.get(was).base);
        if turn == voxel::block::Turn::Chest && variant & (voxel::block::CHEST_LEFT | voxel::block::CHEST_RIGHT) != 0 {
            let l = IVec3::from(voxel::block::chest_left_dir(variant));
            for other in [p + l, p - l] {
                let o = self.block_or_air(other);
                let ob = self.reg.get(o);
                if ob.base == base && ob.variant == variant ^ (voxel::block::CHEST_LEFT | voxel::block::CHEST_RIGHT) {
                    let single = self.reg.variant(o, variant & 3);
                    self.set_block(other, single);
                }
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
            while dest.y > MIN_Y && self.block_or_air(dest - IVec3::Y) == AIR {
                dest.y -= 1;
            }
            self.set_block(above, AIR);
            self.set_block(dest, id);
            y += 1;
        }
        // A broken portal frame (or portal) puts the whole portal out.
        if was_frame {
            let mut todo: Vec<IVec3> = [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z].iter().map(|d| p + *d).collect();
            let mut n = 0;
            while let Some(q) = todo.pop() {
                if self.reg.get(self.block_or_air(q)).shape != Shape::Portal || n > 512 {
                    continue;
                }
                self.set_block(q, AIR);
                n += 1;
                todo.extend([IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z].iter().map(|d| q + *d));
            }
        }
        true
    }

    /// Place a gravity block so it lands on the ground.
    pub fn place_block(&mut self, mut p: IVec3, id: BlockId) -> bool {
        if self.reg.get(id).gravity {
            while p.y > MIN_Y && matches!(self.block(p - IVec3::Y), Some(AIR)) {
                p.y -= 1;
            }
        }
        self.set_block(p, id)
    }

    pub fn is_liquid(&self, id: BlockId) -> bool {
        self.fluid[id as usize] != 0
    }

    /// Stands in water (kelp): leaves water behind when broken.
    pub fn is_wet(&self, id: BlockId) -> bool {
        self.wet[id as usize]
    }

    /// 1 water, 2 lava, 0 anything else (any level of flow counts).
    pub fn fluid(&self, id: BlockId) -> u8 {
        self.fluid[id as usize]
    }

    /// Writes the chunk on the saver thread, off the frame.
    fn queue_save(&self, c: Arc<Chunk>) {
        self.saving.lock().unwrap_or_else(|e| e.into_inner()).insert(c.pos, c.clone());
        if self.saver.send(c.clone()).is_err() {
            // No saver (it never ends while the world lives, but just in case): write here.
            let _ = self.save.save_chunk(&self.reg, &c);
            self.saving.lock().unwrap_or_else(|e| e.into_inner()).remove(&c.pos);
        }
    }

    /// Saves every changed chunk and waits until all of them are on disk.
    pub fn save_all(&mut self) {
        for p in std::mem::take(&mut self.unsaved) {
            if let Some(c) = self.slots.get(&p).and_then(|s| s.chunk.clone()) {
                self.queue_save(c);
            }
        }
        let t = std::time::Instant::now();
        while !self.saving.lock().unwrap_or_else(|e| e.into_inner()).is_empty() {
            if t.elapsed().as_secs() > 30 {
                log::error!("chunks still saving after 30 s");
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
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

/// A saved chunk, or a new one from the seed. Chunks saved before the deep world get their
/// depths generated under them (and lose the old bedrock floor at y 0).
pub fn load_or_generate(save: &WorldSave, gen: &Generator, reg: &BlockRegistry, pos: ChunkPos, deep_lo: usize) -> Chunk {
    load_or_generate_fresh(save, gen, reg, pos, deep_lo).0
}

/// As `load_or_generate`, and true when the chunk is new from the seed.
fn load_or_generate_fresh(save: &WorldSave, gen: &Generator, reg: &BlockRegistry, pos: ChunkPos, deep_lo: usize) -> (Chunk, bool) {
    let Some(mut c) = save.load_chunk(reg, pos) else { return (gen.generate_band(pos, deep_lo), true) };
    fill_legacy(gen, reg, &mut c);
    (c, false)
}

pub fn fill_legacy(gen: &Generator, reg: &BlockRegistry, c: &mut Chunk) {
    if c.legacy {
        let pos = c.pos;
        let fresh = gen.generate(pos);
        c.deep_lo = fresh.deep_lo;
        let bedrock = reg.id_or_air("bedrock");
        for s in 0..BASE_SECTION {
            c.sections[s] = fresh.sections[s].clone();
        }
        if gen.dim == voxel::worldgen::Dim::Overworld && bedrock != AIR {
            for y in 0..5 {
                for z in 0..16 {
                    for x in 0..16 {
                        if c.get(x, y, z) == bedrock {
                            c.set(x, y, z, fresh.get(x, y, z));
                        }
                    }
                }
            }
        }
        c.legacy = false;
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
        self.block(p).is_some_and(|id| self.fluid[id as usize] == 1 || self.wet[id as usize])
    }
    fn climbable(&self, p: IVec3) -> bool {
        self.block(p).is_some_and(|id| self.climb[id as usize])
    }
    fn sticky(&self, p: IVec3) -> bool {
        self.block(p).is_some_and(|id| self.sticky[id as usize])
    }
    fn top(&self, p: IVec3) -> f32 {
        self.block(p).map(|id| self.top[id as usize]).unwrap_or(1.0)
    }
    fn ground(&self, p: IVec3) -> voxel::physics::Ground {
        self.block(p).map(|id| self.ground[id as usize]).unwrap_or(voxel::physics::Ground::FIRM)
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
        if !matches!(shape, Shape::Fence | Shape::Wall | Shape::Pane | Shape::Table) {
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

/// The x/z bounds of the columns where two light arrays of a chunk differ (None: the same).
fn light_change(old: &[u8], new: &[u8]) -> Option<([i32; 2], [i32; 2])> {
    let (mut lo, mut hi) = ([16, 16], [-1, -1]);
    // One layer is 256 columns (x | z << 4): equal layers are skipped whole.
    for (a, b) in old.chunks(256).zip(new.chunks(256)) {
        if a == b {
            continue;
        }
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            if x != y {
                let (cx, cz) = ((i & 15) as i32, (i >> 4) as i32);
                lo = [lo[0].min(cx), lo[1].min(cz)];
                hi = [hi[0].max(cx), hi[1].max(cz)];
            }
        }
    }
    (hi[0] >= 0).then_some((lo, hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visited_chunks_are_kept_on_disk() {
        let reg = Arc::new(BlockRegistry::from_ron(r#"(blocks: [(name: "stone"), (name: "dirt"), (name: "grass_block"), (name: "water")])"#).unwrap());
        let dir = std::env::temp_dir().join(format!("mc-visited-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let save = Arc::new(WorldSave::new(dir.clone()));
        let gen = Arc::new(Generator::new(5, &reg));
        let tables = Arc::new(MeshTables::new(&reg, vec![[1.0; 3]; voxel::worldgen::Biome::COUNT]));
        let mut w = World::new(reg.clone(), gen, save.clone(), tables);
        w.headless = true;
        let t = std::time::Instant::now();
        while w.chunk(ChunkPos::new(0, 0)).is_none() && t.elapsed().as_secs() < 60 {
            w.update(ChunkPos::new(0, 0), 1);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(w.chunk(ChunkPos::new(0, 0)).is_some());
        w.save_all();
        // Never edited, but visited: it is on disk now, and comes back the same.
        assert!(save.has_chunk(ChunkPos::new(0, 0)));
        let back = save.load_chunk(&reg, ChunkPos::new(0, 0)).unwrap();
        let now = w.chunk(ChunkPos::new(0, 0)).unwrap();
        assert_eq!(back.all_blocks(), now.all_blocks());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
