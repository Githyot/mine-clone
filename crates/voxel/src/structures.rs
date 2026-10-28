//! Structures from data (the Towns and Towers idea): ruins, towers, houses and wells written as
//! layers of characters in `structures.ron`, placed by the generator on a coarse grid of
//! regions so each chunk can work out on its own which pieces of which buildings cross it.

use glam::{IVec2, IVec3, Vec3};
use serde::Deserialize;

use crate::block::{BlockId, BlockRegistry, Shape};
use crate::chunk::{Chunk, ChunkPos, CHUNK_HEIGHT};
use crate::rng::Rng;
use crate::worldgen::{Generator, SEA_LEVEL};

fn one() -> f32 {
    1.0
}
fn one_u8() -> u8 {
    1
}

#[derive(Deserialize, Clone, Debug)]
pub struct LootDef {
    pub item: String,
    #[serde(default = "one_u8")]
    pub min: u8,
    #[serde(default = "one_u8")]
    pub max: u8,
    #[serde(default = "one")]
    pub chance: f32,
}

/// How a structure finds its height.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Place {
    /// On fairly flat dry ground (or a cave floor in the Nether).
    #[default]
    Ground,
    /// Floating on open sea (ships): the bottom `sink` layers under the water line.
    Water,
    /// High in the air at this height, whatever is below (airships, sky temples).
    Sky(i32),
}

fn rough() -> i32 {
    4
}

#[derive(Deserialize, Clone, Debug)]
pub struct StructureDef {
    pub name: String,
    /// Biome names (as shown in the F3 screen) it can appear in.
    pub biomes: Vec<String>,
    /// One try per square of this many chunks.
    pub spacing: i32,
    #[serde(default = "one")]
    pub chance: f32,
    /// Character -> block name ("." is air, " " leaves the ground as it is).
    pub palette: Vec<(char, String)>,
    /// Bottom layer first; each layer is rows from north to south, each row west to east.
    pub layers: Vec<Vec<String>>,
    /// How many layers sit below the ground.
    #[serde(default)]
    pub sink: i32,
    /// Fills under the bottom layer down to the ground, so nothing floats on a slope.
    #[serde(default)]
    pub foundation: Option<String>,
    #[serde(default)]
    pub loot: Vec<LootDef>,
    /// Buildings set around this one in a ring (a village square and its houses).
    #[serde(default)]
    pub pieces: Vec<String>,
    /// Only placed as a piece of another structure, never on its own.
    #[serde(default)]
    pub piece_only: bool,
    #[serde(default)]
    pub place: Place,
    /// Most the ground may rise and fall under the footprint (blocks).
    #[serde(default = "rough")]
    pub rough: i32,
    /// Mobs that live inside, spawned once when the player first comes near: (name, count).
    #[serde(default)]
    pub mobs: Vec<(String, u8)>,
    /// Things set in the building that are not blocks (a parked wagon), placed by the game.
    #[serde(default)]
    pub props: Vec<PropDef>,
}

/// Something in a structure that is not a block: what it is (the game knows the kinds), where
/// in the plan (blocks from the plan's corner, layer 0 at the bottom, before turning) and
/// which way it faces (degrees; 0 faces +Z, 90 faces +X).
#[derive(Deserialize, Clone, Debug)]
pub struct PropDef {
    pub kind: String,
    pub at: (f32, f32, f32),
    #[serde(default)]
    pub yaw: f32,
}

/// A prop of a building in the world.
#[derive(Clone, Debug)]
pub struct Prop {
    pub kind: String,
    pub pos: Vec3,
    /// Facing, as an entity's yaw (atan2 of x over z).
    pub yaw: f32,
    /// Which building and which of its props: the same for the same thing every time.
    pub key: (IVec3, usize),
}

#[derive(Deserialize)]
struct StructureFile {
    structures: Vec<StructureDef>,
}

pub struct Structure {
    pub def: StructureDef,
    /// x, y, z size before turning.
    size: IVec3,
    /// None keeps what the terrain has there.
    cells: Vec<Option<BlockId>>,
    foundation: Option<BlockId>,
    /// The block id each id becomes after k quarter turns (stairs, roofs and doors face around).
    turned: [Vec<BlockId>; 4],
    chest: Option<BlockId>,
}

/// One building somewhere in the world.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub index: usize,
    /// Lowest corner in world blocks (after turning).
    pub origin: IVec3,
    pub turns: u8,
}

pub struct Structures {
    pub list: Vec<Structure>,
    /// Longest footprint side of any structure: how far around a chunk to look for them.
    max_side: i32,
    /// Blocks a foundation pushes through: air, water, plants, snow.
    soft: Vec<bool>,
    /// Trees and plants, cleared from around a building so they don't grow through it.
    plant: Vec<bool>,
}

impl Structures {
    pub fn empty() -> Structures {
        Structures { list: Vec::new(), max_side: MAX_SIDE, soft: Vec::new(), plant: Vec::new() }
    }

    pub fn from_ron(text: &str, reg: &BlockRegistry) -> Result<Structures, String> {
        Self::from_ron_many(&[text], reg)
    }

    /// Several structure files read as one list (the base game, then mod packs).
    pub fn from_ron_many(texts: &[&str], reg: &BlockRegistry) -> Result<Structures, String> {
        let mut file = StructureFile { structures: Vec::new() };
        for text in texts.iter().filter(|t| !t.trim().is_empty()) {
            let f: StructureFile = ron::from_str(text).map_err(|e| format!("structures.ron: {e}"))?;
            file.structures.extend(f.structures);
        }
        let mut list = Vec::new();
        for def in file.structures {
            let sy = def.layers.len() as i32;
            let sz = def.layers.iter().map(|l| l.len()).max().unwrap_or(0) as i32;
            let sx = def.layers.iter().flat_map(|l| l.iter().map(|r| r.chars().count())).max().unwrap_or(0) as i32;
            if sx == 0 || sy == 0 || sz == 0 {
                continue;
            }
            let mut cells = vec![None; (sx * sy * sz) as usize];
            for (y, layer) in def.layers.iter().enumerate() {
                for (z, row) in layer.iter().enumerate() {
                    for (x, ch) in row.chars().enumerate() {
                        let id = match ch {
                            ' ' => None,
                            '.' => Some(crate::block::AIR),
                            c => match def.palette.iter().find(|p| p.0 == c) {
                                Some((_, name)) => Some(reg.id(name).ok_or_else(|| format!("structure {}: unknown block {name}", def.name))?),
                                None => return Err(format!("structure {}: '{c}' is not in the palette", def.name)),
                            },
                        };
                        cells[((y as i32 * sz + z as i32) * sx + x as i32) as usize] = id;
                    }
                }
            }
            pair_beds(&mut cells, IVec3::new(sx, sy, sz), reg);
            let mut turned: [Vec<BlockId>; 4] = Default::default();
            for (k, t) in turned.iter_mut().enumerate() {
                *t = (0..reg.blocks.len())
                    .map(|i| {
                        let b = &reg.blocks[i];
                        // Blocks facing one of four ways turn with the structure (an Oriented
                        // block facing up or down stays as it is).
                        let turns = matches!(b.shape, Shape::Stairs | Shape::Roof | Shape::Door | Shape::Trapdoor | Shape::Bed) || b.shape == Shape::Oriented && b.variant & 7 < 4;
                        if k == 0 || !turns {
                            return i as BlockId;
                        }
                        let v = (b.variant & !3) | ((b.variant & 3) + k as u8) & 3;
                        let base = &reg.blocks[b.base as usize].name;
                        if v == 0 {
                            b.base
                        } else {
                            reg.id(&format!("{base}:{v}")).unwrap_or(i as BlockId)
                        }
                    })
                    .collect();
            }
            let foundation = def.foundation.as_deref().and_then(|n| reg.id(n));
            let chest = reg.id("chest");
            list.push(Structure { def, size: IVec3::new(sx, sy, sz), cells, foundation, turned, chest });
        }
        let soft = reg.blocks.iter().map(|b| matches!(b.shape, Shape::Liquid | Shape::Cross | Shape::Layer | Shape::Invisible) || b.name == "air").collect();
        let plant = reg.blocks.iter().map(|b| b.shape == Shape::Cross || b.name.ends_with("_leaves") || b.name.ends_with("_log") || b.name == "vine").collect();
        let max_side = list.iter().map(|s| s.size.x.max(s.size.z)).max().unwrap_or(0).max(MAX_SIDE);
        Ok(Structures { list, max_side, soft, plant })
    }
}

/// Buildings name a bed once, as a single cell: make it a whole bed, the foot there and the
/// head in the next free cell (a neighbouring bed cell first, then air, then a cell left
/// as it is), turned toward it.
fn pair_beds(cells: &mut [Option<BlockId>], size: IVec3, reg: &BlockRegistry) {
    let at = |x: i32, y: i32, z: i32| (x >= 0 && z >= 0 && x < size.x && z < size.z).then(|| ((y * size.z + z) * size.x + x) as usize);
    let lone = |c: Option<BlockId>| c.is_some_and(|id| reg.get(id).shape == Shape::Bed && reg.get(id).variant == 0);
    let mut done = vec![false; cells.len()];
    for y in 0..size.y {
        for z in 0..size.z {
            for x in 0..size.x {
                let i = at(x, y, z).unwrap();
                if done[i] || !lone(cells[i]) {
                    continue;
                }
                let id = cells[i].unwrap();
                // Toward north, east, south, west (the bed variants' facings).
                let ways = [(0, -1), (1, 0), (0, 1), (-1, 0)];
                let pick = |ok: &dyn Fn(Option<BlockId>) -> bool| ways.iter().enumerate().find_map(|(f, &(dx, dz))| at(x + dx, y, z + dz).filter(|&j| !done[j] && ok(cells[j])).map(|j| (f as u8, j)));
                let Some((f, j)) = pick(&|c| lone(c) && c == cells[i]).or_else(|| pick(&|c| c == Some(crate::block::AIR))).or_else(|| pick(&|c| c.is_none())) else {
                    continue;
                };
                cells[i] = Some(reg.variant(id, f));
                cells[j] = Some(reg.variant(id, f | crate::bed::HEAD));
                done[i] = true;
                done[j] = true;
            }
        }
    }
}

impl Structure {
    fn footprint(&self, turns: u8) -> IVec2 {
        if turns & 1 == 0 {
            IVec2::new(self.size.x, self.size.z)
        } else {
            IVec2::new(self.size.z, self.size.x)
        }
    }

    /// The block at a position inside the turned footprint (local, 0-based).
    fn cell(&self, turns: u8, x: i32, y: i32, z: i32) -> Option<BlockId> {
        let (sx, sz) = (self.size.x, self.size.z);
        // Undo the turn: each quarter turn maps (x, z) -> (sz - 1 - z, x).
        let (mut ux, mut uz) = (x, z);
        let mut w = self.footprint(turns);
        for _ in 0..turns {
            // Inverse of (x, z) -> (h - 1 - z, x) on a w.x by w.y area whose source was w.y by w.x.
            let (px, pz) = (uz, w.x - 1 - ux);
            ux = px;
            uz = pz;
            w = IVec2::new(w.y, w.x);
        }
        if ux < 0 || uz < 0 || ux >= sx || uz >= sz || y < 0 || y >= self.size.y {
            return None;
        }
        self.cells[((y * sz + uz) * sx + ux) as usize].map(|id| self.turned[turns as usize][id as usize])
    }
}

/// Smallest distance we look around a chunk for buildings that cross it (bigger buildings
/// raise it, see `Structures::max_side`).
const MAX_SIDE: i32 = 32;
/// How far the pieces of a village reach from its square.
const VILLAGE_REACH: i32 = 48;

impl Structure {
    /// The size before turning (x, layers, z).
    pub fn size(&self) -> IVec3 {
        self.size
    }

    /// Footprint after `turns` quarter turns (x, z).
    pub fn turned_footprint(&self, turns: u8) -> IVec2 {
        self.footprint(turns)
    }
}

impl Generator {
    /// Buildings whose footprint may cross this chunk.
    pub fn placements_near(&self, pos: ChunkPos) -> Vec<Placement> {
        let mut out = Vec::new();
        let structs = self.structures();
        let hits = |p: &Placement, f: IVec2| p.origin.x <= pos.x * 16 + 15 && p.origin.x + f.x > pos.x * 16 && p.origin.z <= pos.y * 16 + 15 && p.origin.z + f.y > pos.y * 16;
        let side = structs.max_side;
        for (si, s) in structs.list.iter().enumerate() {
            if s.def.piece_only {
                continue;
            }
            let r = s.def.spacing.max(2) * 16 * self.spread / 100;
            let reach = if s.def.pieces.is_empty() { side } else { side + VILLAGE_REACH };
            let x0 = (pos.x * 16 - reach).div_euclid(r);
            let x1 = (pos.x * 16 + 15 + reach - side).div_euclid(r);
            let z0 = (pos.y * 16 - reach).div_euclid(r);
            let z1 = (pos.y * 16 + 15 + reach - side).div_euclid(r);
            for rz in z0..=z1 {
                for rx in x0..=x1 {
                    if let Some(p) = self.try_place(si, s, rx, rz, r) {
                        if hits(&p, s.footprint(p.turns)) {
                            out.push(p);
                        }
                        for piece in self.village_pieces(&structs, s, &p, rx, rz) {
                            if hits(&piece, structs.list[piece.index].footprint(piece.turns)) {
                                out.push(piece);
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// The buildings of a village around its square: set in a ring, each turned to face the
    /// square, each only where the ground is fairly flat and dry.
    fn village_pieces(&self, structs: &Structures, s: &Structure, center: &Placement, rx: i32, rz: i32) -> Vec<Placement> {
        let mut out: Vec<Placement> = Vec::new();
        if s.def.pieces.is_empty() {
            return out;
        }
        let mut rng = Rng::for_pos(self.seed, rx, rz, 9100);
        let f0 = s.footprint(center.turns);
        let (cx, cz) = (center.origin.x + f0.x / 2, center.origin.z + f0.y / 2);
        let n = s.def.pieces.len();
        let a0 = rng.f32() * std::f32::consts::TAU;
        for (k, name) in s.def.pieces.iter().enumerate() {
            let Some(pi) = structs.list.iter().position(|p| &p.def.name == name) else { continue };
            let piece = &structs.list[pi];
            let a = a0 + k as f32 / n as f32 * std::f32::consts::TAU + (rng.f32() - 0.5) * 0.3;
            let dist = 15.0 + rng.f32() * 9.0 + (k % 2) as f32 * 8.0;
            let (dx, dz) = (a.cos(), a.sin());
            // The door (south in the plan) toward the square.
            let turns = if dz.abs() > dx.abs() {
                if dz < 0.0 { 0 } else { 2 }
            } else if dx > 0.0 {
                1
            } else {
                3
            };
            let f = piece.footprint(turns);
            let (px, pz) = (cx + (dx * dist) as i32 - f.x / 2, cz + (dz * dist) as i32 - f.y / 2);
            // Not on top of another piece.
            if out.iter().any(|o| {
                let g = structs.list[o.index].footprint(o.turns);
                px < o.origin.x + g.x + 2 && o.origin.x < px + f.x + 2 && pz < o.origin.z + g.y + 2 && o.origin.z < pz + f.y + 2
            }) {
                continue;
            }
            let hs = [self.height_at(px, pz), self.height_at(px + f.x - 1, pz), self.height_at(px, pz + f.y - 1), self.height_at(px + f.x - 1, pz + f.y - 1), self.height_at(px + f.x / 2, pz + f.y / 2)];
            let (lo, hi) = (*hs.iter().min().unwrap(), *hs.iter().max().unwrap());
            if lo < SEA_LEVEL || hi - lo > 4 {
                continue;
            }
            out.push(Placement { index: pi, origin: IVec3::new(px, hs[4] + 1 - piece.def.sink, pz), turns });
        }
        out
    }

    /// Village squares near (x, z): their middle, for whoever wants to people them.
    pub fn villages_near(&self, x: i32, z: i32, range: i32) -> Vec<IVec3> {
        let structs = self.structures();
        let mut out = Vec::new();
        for (si, s) in structs.list.iter().enumerate() {
            if s.def.pieces.is_empty() {
                continue;
            }
            let r = s.def.spacing.max(2) * 16 * self.spread / 100;
            for rz in (z - range).div_euclid(r)..=(z + range).div_euclid(r) {
                for rx in (x - range).div_euclid(r)..=(x + range).div_euclid(r) {
                    if let Some(p) = self.try_place(si, s, rx, rz, r) {
                        let f = s.footprint(p.turns);
                        out.push(IVec3::new(p.origin.x + f.x / 2, p.origin.y + s.def.sink + 1, p.origin.z + f.y / 2));
                    }
                }
            }
        }
        out
    }

    fn try_place(&self, si: usize, s: &Structure, rx: i32, rz: i32, r: i32) -> Option<Placement> {
        let mut rng = Rng::for_pos(self.seed, rx, rz, 7000 + si as u64);
        if rng.f32() >= s.def.chance {
            return None;
        }
        let turns = rng.below(4) as u8;
        let f = s.footprint(turns);
        let ox = rx * r + rng.range(0, (r - f.x).max(1));
        let oz = rz * r + rng.range(0, (r - f.y).max(1));
        let (cx, cz) = (ox + f.x / 2, oz + f.y / 2);
        let biome = self.biome_at(cx, cz);
        if !s.def.biomes.iter().any(|b| b == biome.name() || b == biome.base().name()) {
            return None;
        }
        if let Place::Sky(h) = s.def.place {
            return Some(Placement { index: si, origin: IVec3::new(ox, h, oz), turns });
        }
        if s.def.place == Place::Water {
            // Open sea under the whole hull.
            let deep = [(ox, oz), (ox + f.x - 1, oz), (ox, oz + f.y - 1), (ox + f.x - 1, oz + f.y - 1), (cx, cz)].iter().all(|&(x, z)| self.height_at(x, z) < SEA_LEVEL - 3);
            if !deep {
                return None;
            }
            return Some(Placement { index: si, origin: IVec3::new(ox, SEA_LEVEL + 1 - s.def.sink, oz), turns });
        }
        let floating = matches!(self.dim, crate::worldgen::Dim::End | crate::worldgen::Dim::Aether);
        if floating {
            // On the End's outer islands or the Aether's islands, on fairly flat ground.
            let corners = [(ox, oz), (ox + f.x - 1, oz), (ox, oz + f.y - 1), (ox + f.x - 1, oz + f.y - 1), (cx, cz)];
            let mut hs = Vec::with_capacity(5);
            for &(x, z) in &corners {
                let land = if self.dim == crate::worldgen::Dim::End { self.end_land(x, z) } else { self.aether_land(x, z) };
                hs.push(land?.0);
            }
            let (lo, hi) = (*hs.iter().min().unwrap(), *hs.iter().max().unwrap());
            let main_island = self.dim == crate::worldgen::Dim::End && ((cx * cx + cz * cz) as f32) < 400.0 * 400.0;
            if hi - lo > s.def.rough || main_island {
                return None;
            }
            return Some(Placement { index: si, origin: IVec3::new(ox, hs[4] + 1 - s.def.sink, oz), turns });
        }
        if matches!(self.dim, crate::worldgen::Dim::Nether | crate::worldgen::Dim::Otherside) {
            // On a cave floor above the lava sea (or the Otherside's pools), with room overhead.
            let solid = |x: i32, y: i32, z: i32| if self.dim == crate::worldgen::Dim::Nether { self.nether_solid(x, y, z) } else { self.otherside_solid(x, y, z) };
            let tall = s.size.y + 2;
            let mut ground = None;
            for y in crate::worldgen::NETHER_LAVA + 1..crate::worldgen::NETHER_ROOF - tall {
                if solid(cx, y, cz) && (1..=tall).all(|k| !solid(cx, y + k, cz)) {
                    ground = Some(y);
                    break;
                }
            }
            let ground = ground?;
            let flat = [(ox, oz), (ox + f.x - 1, oz), (ox, oz + f.y - 1), (ox + f.x - 1, oz + f.y - 1)]
                .iter()
                .all(|&(x, z)| !solid(x, ground + 2, z) && !solid(x, ground + s.size.y, z));
            if !flat {
                return None;
            }
            return Some(Placement { index: si, origin: IVec3::new(ox, ground + 1 - s.def.sink, oz), turns });
        }
        if self.dim == crate::worldgen::Dim::End {
            // On an island's top with land under every corner; ships float high above one.
            let corners = [(ox, oz), (ox + f.x - 1, oz), (ox, oz + f.y - 1), (ox + f.x - 1, oz + f.y - 1), (cx, cz)];
            let tops: Vec<i32> = corners.iter().filter_map(|&(x, z)| self.end_top(x, z)).collect();
            if s.def.name.starts_with("navio") {
                let ground = self.end_top(cx, cz)?;
                return Some(Placement { index: si, origin: IVec3::new(ox, ground + 22, oz), turns });
            }
            // The middle and at least three corners on land (a keel of rock fills under the rest).
            if tops.len() < corners.len() - 1 || self.end_top(cx, cz).is_none() {
                return None;
            }
            let (lo, hi) = (*tops.iter().min().unwrap(), *tops.iter().max().unwrap());
            if hi - lo > 6 {
                return None;
            }
            let ground = self.end_top(cx, cz)?;
            return Some(Placement { index: si, origin: IVec3::new(ox, ground + 1 - s.def.sink, oz), turns });
        }
        // Fairly flat dry ground only.
        let hs = [self.height_at(ox, oz), self.height_at(ox + f.x - 1, oz), self.height_at(ox, oz + f.y - 1), self.height_at(ox + f.x - 1, oz + f.y - 1), self.height_at(cx, cz)];
        let lo = *hs.iter().min().unwrap();
        let hi = *hs.iter().max().unwrap();
        // Wrecks and the like sit on the sea floor, everything else on dry land.
        let sea_floor = s.def.biomes.iter().any(|b| b == "Ocean" || b == "Frozen Ocean");
        if (if sea_floor { hi > SEA_LEVEL - 6 } else { lo < SEA_LEVEL }) || hi - lo > s.def.rough {
            return None;
        }
        let ground = hs[4];
        Some(Placement { index: si, origin: IVec3::new(ox, ground + 1 - s.def.sink, oz), turns })
    }

    /// Build the parts of nearby buildings that fall in this chunk.
    pub fn place_structures(&self, chunk: &mut Chunk) {
        let pos = chunk.pos;
        let structs = self.structures();
        for p in self.placements_near(pos) {
            let s = &structs.list[p.index];
            let f = s.footprint(p.turns);
            for lz in 0..16 {
                for lx in 0..16 {
                    let (wx, wz) = (pos.x * 16 + lx, pos.y * 16 + lz);
                    let (x, z) = (wx - p.origin.x, wz - p.origin.z);
                    // A clearing one block wider than the building, free of trees.
                    if s.def.place == Place::Ground && x >= -1 && z >= -1 && x <= f.x && z <= f.y {
                        for y in p.origin.y.max(1)..(p.origin.y + s.size.y + 6).min(CHUNK_HEIGHT as i32) {
                            let cur = chunk.get(lx as usize, y as usize, lz as usize);
                            if structs.plant.get(cur as usize).copied().unwrap_or(false) {
                                chunk.set(lx as usize, y as usize, lz as usize, crate::block::AIR);
                            }
                        }
                    }
                    if x < 0 || z < 0 || x >= f.x || z >= f.y {
                        continue;
                    }
                    for y in 0..s.size.y {
                        let wy = p.origin.y + y;
                        if wy <= 0 || wy >= CHUNK_HEIGHT as i32 {
                            continue;
                        }
                        if let Some(id) = s.cell(p.turns, x, y, z) {
                            chunk.set(lx as usize, wy as usize, lz as usize, id);
                        }
                    }
                    // A foundation under solid bottom cells, down to the ground.
                    if let (Some(fd), Some(b)) = (s.foundation, s.cell(p.turns, x, 0, z)) {
                        if b != crate::block::AIR {
                            let mut y = p.origin.y - 1;
                            // Over the End's void a foundation is a short rocky keel, not a column.
                            let floor = if self.dim == crate::worldgen::Dim::End { p.origin.y - 10 } else { 0 };
                            while y > floor.max(0) {
                                let cur = chunk.get(lx as usize, y as usize, lz as usize);
                                if cur != crate::block::AIR && !structs.soft.get(cur as usize).copied().unwrap_or(false) {
                                    break;
                                }
                                chunk.set(lx as usize, y as usize, lz as usize, fd);
                                y -= 1;
                            }
                        }
                    }
                }
            }
        }
    }

    /// The props (parked wagons and the like) of the buildings within `range` blocks of (x, z).
    pub fn props_near(&self, x: i32, z: i32, range: i32) -> Vec<Prop> {
        let structs = self.structures();
        let mut out = Vec::new();
        for (si, s) in structs.list.iter().enumerate() {
            if s.def.props.is_empty() || s.def.piece_only {
                continue;
            }
            let r = s.def.spacing.max(2) * 16 * self.spread / 100;
            for rz in (z - range).div_euclid(r)..=(z + range).div_euclid(r) {
                for rx in (x - range).div_euclid(r)..=(x + range).div_euclid(r) {
                    let Some(p) = self.try_place(si, s, rx, rz, r) else { continue };
                    for (k, d) in s.def.props.iter().enumerate() {
                        // Turn the plan point and the facing the way the building was turned:
                        // each quarter turn maps (u, v) on a w by d plan to (d - v, u).
                        let (mut u, mut v) = (d.at.0, d.at.2);
                        let (mut fx, mut fz) = (d.yaw.to_radians().sin(), d.yaw.to_radians().cos());
                        let (mut w, mut dd) = (s.size.x as f32, s.size.z as f32);
                        for _ in 0..p.turns {
                            (u, v) = (dd - v, u);
                            (fx, fz) = (-fz, fx);
                            (w, dd) = (dd, w);
                        }
                        let _ = w;
                        let pos = Vec3::new(p.origin.x as f32 + u, p.origin.y as f32 + d.at.1, p.origin.z as f32 + v);
                        if (pos.x - x as f32).abs() <= range as f32 && (pos.z - z as f32).abs() <= range as f32 {
                            out.push(Prop { kind: d.kind.clone(), pos, yaw: fx.atan2(fz), key: (p.origin, k) });
                        }
                    }
                }
            }
        }
        out
    }

    /// The nearest building called `name` within `regions` tries of (x, z): its lowest corner and
    /// its turned size.
    pub fn find_structure(&self, name: &str, x: i32, z: i32, regions: i32) -> Option<(IVec3, IVec3)> {
        let structs = self.structures();
        let (si, s) = structs.list.iter().enumerate().find(|(_, s)| s.def.name == name)?;
        let r = s.def.spacing.max(2) * 16 * self.spread / 100;
        let (cx, cz) = (x.div_euclid(r), z.div_euclid(r));
        let mut best: Option<(i32, Placement)> = None;
        for rz in cz - regions..=cz + regions {
            for rx in cx - regions..=cx + regions {
                if let Some(p) = self.try_place(si, s, rx, rz, r) {
                    let d = (p.origin.x - x).abs() + (p.origin.z - z).abs();
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((d, p));
                    }
                }
            }
        }
        best.map(|(_, p)| {
            let f = s.footprint(p.turns);
            (p.origin, IVec3::new(f.x, s.size.y, f.y))
        })
    }

    /// The loot of a chest the generator put at `p`, if it did.
    pub fn structure_chest(&self, p: IVec3) -> Option<&[LootDef]> {
        let pos = ChunkPos::new(p.x.div_euclid(16), p.z.div_euclid(16));
        let structs = self.structures();
        for pl in self.placements_near(pos) {
            let s = &structs.list[pl.index];
            let l = p - pl.origin;
            if s.chest.is_some() && s.cell(pl.turns, l.x, l.y, l.z) == s.chest {
                return Some(&s.def.loot);
            }
        }
        self.deep_chest(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_and_mod_structures_load() {
        let base = include_str!("../../../packs/vanilla/blocks.ron");
        let mods = include_str!("../../../packs/vanilla/mods/blocks.ron");
        let reg = BlockRegistry::from_ron_many(&[base, mods]).unwrap();
        let s = Structures::from_ron_many(
            &[include_str!("../../../packs/vanilla/structures.ron"), include_str!("../../../packs/vanilla/mods/structures.ron")],
            &reg,
        )
        .unwrap();
        let mut names = std::collections::HashSet::new();
        for st in &s.list {
            assert!(names.insert(st.def.name.clone()), "two structures called {}", st.def.name);
            assert!(st.size.x <= 64 && st.size.z <= 64 && st.size.y <= 80, "{} is too big", st.def.name);
        }
    }

    #[test]
    fn a_lone_bed_becomes_a_whole_bed_and_turns() {
        let reg = BlockRegistry::from_ron(include_str!("../../../packs/vanilla/blocks.ron")).unwrap();
        let text = r###"(structures: [(name: "t", biomes: [], spacing: 4, palette: [('b', "red_bed"), ('#', "stone")], layers: [["b.", "##"]])])"###;
        let st = &Structures::from_ron(text, &reg).unwrap().list[0];
        let red = reg.id("red_bed").unwrap();
        // North is outside, so the head goes east, into the air.
        assert_eq!(st.cell(0, 0, 0, 0), Some(reg.variant(red, 1)));
        assert_eq!(st.cell(0, 1, 0, 0), Some(reg.variant(red, 1 | crate::bed::HEAD)));
        // A quarter turn: the foot at (1, 0) points south to its head at (1, 1).
        assert_eq!(st.cell(1, 1, 0, 0), Some(reg.variant(red, 2)));
        assert_eq!(st.cell(1, 1, 0, 1), Some(reg.variant(red, 2 | crate::bed::HEAD)));
    }

    #[test]
    fn turning_keeps_every_cell() {
        let reg = BlockRegistry::from_ron(include_str!("../../../packs/vanilla/blocks.ron")).unwrap();
        let text = r##"(structures: [(name: "t", biomes: [], spacing: 4, palette: [('#', "stone"), ('c', "cobblestone")], layers: [["#c.", "..."]])])"##;
        let s = Structures::from_ron(text, &reg).unwrap();
        let st = &s.list[0];
        let stone = reg.id("stone");
        let cob = reg.id("cobblestone");
        // 3 wide (x) by 2 deep (z); one turn makes it 2 wide by 3 deep.
        assert_eq!(st.cell(0, 0, 0, 0), stone);
        assert_eq!(st.cell(0, 1, 0, 0), cob);
        assert_eq!(st.footprint(1), IVec2::new(2, 3));
        // (x, z) -> (sz - 1 - z, x): stone (0,0) -> (1,0), cobble (1,0) -> (1,1).
        assert_eq!(st.cell(1, 1, 0, 0), stone);
        assert_eq!(st.cell(1, 1, 0, 1), cob);
        for k in 0..4u8 {
            let f = st.footprint(k);
            let n = (0..f.x).flat_map(|x| (0..f.y).map(move |z| (x, z))).filter(|&(x, z)| st.cell(k, x, 0, z).is_some()).count();
            assert_eq!(n, 6, "turn {k}");
        }
    }
}

