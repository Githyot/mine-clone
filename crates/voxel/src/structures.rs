//! Structures from data (the Towns and Towers idea): ruins, towers, houses and wells written as
//! layers of characters in `structures.ron`, placed by the generator on a coarse grid of
//! regions so each chunk can work out on its own which pieces of which buildings cross it.

use glam::{IVec2, IVec3};
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
    /// Blocks a foundation pushes through: air, water, plants, snow.
    soft: Vec<bool>,
    /// Trees and plants, cleared from around a building so they don't grow through it.
    plant: Vec<bool>,
}

impl Structures {
    pub fn empty() -> Structures {
        Structures { list: Vec::new(), soft: Vec::new(), plant: Vec::new() }
    }

    pub fn from_ron(text: &str, reg: &BlockRegistry) -> Result<Structures, String> {
        let file: StructureFile = ron::from_str(text).map_err(|e| format!("structures.ron: {e}"))?;
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
            let mut turned: [Vec<BlockId>; 4] = Default::default();
            for (k, t) in turned.iter_mut().enumerate() {
                *t = (0..reg.blocks.len())
                    .map(|i| {
                        let b = &reg.blocks[i];
                        if k == 0 || !matches!(b.shape, Shape::Stairs | Shape::Roof | Shape::Door | Shape::Trapdoor) {
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
        Ok(Structures { list, soft, plant })
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

/// Biggest footprint side we look around for (larger buildings would be cut off).
const MAX_SIDE: i32 = 32;

impl Generator {
    /// Buildings whose footprint may cross this chunk.
    pub fn placements_near(&self, pos: ChunkPos) -> Vec<Placement> {
        let mut out = Vec::new();
        let structs = self.structures();
        for (si, s) in structs.list.iter().enumerate() {
            let r = s.def.spacing.max(2) * 16 * self.spread / 100;
            let x0 = (pos.x * 16 - MAX_SIDE).div_euclid(r);
            let x1 = (pos.x * 16 + 15).div_euclid(r);
            let z0 = (pos.y * 16 - MAX_SIDE).div_euclid(r);
            let z1 = (pos.y * 16 + 15).div_euclid(r);
            for rz in z0..=z1 {
                for rx in x0..=x1 {
                    if let Some(p) = self.try_place(si, s, rx, rz, r) {
                        let f = s.footprint(p.turns);
                        let hit = p.origin.x <= pos.x * 16 + 15 && p.origin.x + f.x > pos.x * 16 && p.origin.z <= pos.y * 16 + 15 && p.origin.z + f.y > pos.y * 16;
                        if hit {
                            out.push(p);
                        }
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
        if !s.def.biomes.iter().any(|b| b == biome.name()) {
            return None;
        }
        if self.dim == crate::worldgen::Dim::Nether {
            // On a cave floor above the lava sea, with room overhead.
            let tall = s.size.y + 2;
            let mut ground = None;
            for y in crate::worldgen::NETHER_LAVA + 1..crate::worldgen::NETHER_ROOF - tall {
                if self.nether_solid(cx, y, cz) && (1..=tall).all(|k| !self.nether_solid(cx, y + k, cz)) {
                    ground = Some(y);
                    break;
                }
            }
            let ground = ground?;
            let flat = [(ox, oz), (ox + f.x - 1, oz), (ox, oz + f.y - 1), (ox + f.x - 1, oz + f.y - 1)]
                .iter()
                .all(|&(x, z)| !self.nether_solid(x, ground + 2, z) && !self.nether_solid(x, ground + s.size.y, z));
            if !flat {
                return None;
            }
            return Some(Placement { index: si, origin: IVec3::new(ox, ground + 1 - s.def.sink, oz), turns });
        }
        // Fairly flat dry ground only.
        let hs = [self.height_at(ox, oz), self.height_at(ox + f.x - 1, oz), self.height_at(ox, oz + f.y - 1), self.height_at(ox + f.x - 1, oz + f.y - 1), self.height_at(cx, cz)];
        let lo = *hs.iter().min().unwrap();
        let hi = *hs.iter().max().unwrap();
        if lo < SEA_LEVEL || hi - lo > 4 {
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
                    if x >= -1 && z >= -1 && x <= f.x && z <= f.y {
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
                            while y > 0 {
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

    /// The nearest building called `name` within `regions` tries of (x, z): its lowest corner and
    /// its turned size.
    pub fn find_structure(&self, name: &str, x: i32, z: i32, regions: i32) -> Option<(IVec3, IVec3)> {
        let structs = self.structures();
        let (si, s) = structs.list.iter().enumerate().find(|(_, s)| s.def.name == name)?;
        let r = s.def.spacing.max(2) * 16;
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
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

