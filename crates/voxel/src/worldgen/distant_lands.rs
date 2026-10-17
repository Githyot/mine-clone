//! Distant Lands' biomes (the mod's idea, our own take): five Brazilian landscapes that take
//! over part of a parent climate, so they turn up in ordinary worlds.
//!
//! * Mata Atlântica (in jungles): a dense, wet forest of jabuticabeiras, pink and purple ipês,
//!   açaí palms and jungle trees over bromélias, ferns and fallen hollow logs.
//! * Cerrado (in savannas): open grassland of twisted little trees, yellow ipês and buriti palms
//!   on red earth, with capim-dourado and termite mounds.
//! * Pantanal (hot, half-wet lowlands): flat flooded grassland around shallow pools with water
//!   hyacinths, pink ipês (piúvas) and palms.
//! * Mata de Araucárias (in cool forests): tall umbrella-crowned araucárias over ferns.
//! * Caatinga (in deserts): dry thorny scrub, mandacaru cacti and bare grey trees on cracked
//!   earth.
//!
//! The blocks are the pack's (tools/moddata/distant_lands.py); with a pack that lacks them the
//! biomes still appear, with the base game's plants.

use super::{smoothstep, Biome, ChunkSpace, Column, Generator, SEA_LEVEL};
use crate::block::{BlockId, BlockRegistry, AIR};
use crate::chunk::{Chunk, ChunkPos};
use crate::modtrees::{self, ModTree, Shape};
use crate::rng::Rng;
use crate::trees::{self, TreeKind, Wood};

#[derive(Clone)]
pub(super) struct DistantPalette {
    ipe_pink: ModTree,
    ipe_yellow: ModTree,
    ipe_purple: ModTree,
    araucaria: ModTree,
    buriti: ModTree,
    acai: ModTree,
    jabuticaba: ModTree,
    /// Hollow logs lying along x and along z (fallen trunks in the Atlantic forest).
    hollow: [BlockId; 2],
    red_soil: BlockId,
    cracked_dirt: BlockId,
    mud: BlockId,
    bromelias: [BlockId; 2],
    capim_dourado: BlockId,
    jurema: BlockId,
    samambaia: BlockId,
    mandacaru: BlockId,
    mandacaru_flower: BlockId,
    aguape: BlockId,
    acacia_log: BlockId,
    acacia_leaves: BlockId,
}

impl DistantPalette {
    pub(super) fn new(r: &BlockRegistry) -> Self {
        let g = |n: &str| r.id_or_air(n);
        let tree = |log: &str, leaves: &str, shape, fruit: &str| ModTree { log: g(log), leaves: g(leaves), shape, fruit: g(fruit) };
        let hollow = g("hollow_jabuticaba_log");
        DistantPalette {
            ipe_pink: tree("ipe_log", "ipe_pink_leaves", Shape::Spread, ""),
            ipe_yellow: tree("ipe_log", "ipe_yellow_leaves", Shape::Spread, ""),
            ipe_purple: tree("ipe_log", "ipe_purple_leaves", Shape::Spread, ""),
            araucaria: tree("araucaria_log", "araucaria_leaves", Shape::Umbrella, ""),
            buriti: tree("buriti_log", "buriti_leaves", Shape::Palm, "buriti_cluster"),
            acai: tree("buriti_log", "buriti_leaves", Shape::Palm, "acai_cluster"),
            jabuticaba: tree("jabuticaba_log", "jabuticaba_leaves", Shape::TrunkFruit, "jabuticaba_cluster"),
            hollow: if hollow == AIR { [AIR; 2] } else { [r.variant(hollow, 1), r.variant(hollow, 2)] },
            red_soil: g("red_soil"),
            cracked_dirt: g("cracked_dirt"),
            mud: g("mud"),
            bromelias: [g("bromelia_red"), g("bromelia_orange")],
            capim_dourado: g("capim_dourado"),
            jurema: g("jurema"),
            samambaia: g("samambaia"),
            mandacaru: g("mandacaru"),
            mandacaru_flower: g("mandacaru_flower"),
            aguape: g("aguape"),
            acacia_log: g("acacia_log"),
            acacia_leaves: g("acacia_leaves"),
        }
    }
}

/// Is this one of Distant Lands' biomes?
pub fn is_distant(b: Biome) -> bool {
    matches!(b, Biome::MataAtlantica | Biome::Cerrado | Biome::Pantanal | Biome::Araucaria | Biome::Caatinga)
}

/// Blocks a mod tree's crown reaches sideways from where it is planted (palms lean first).
fn margin(s: Shape) -> i32 {
    match s {
        Shape::Palm => 6,
        Shape::Spread | Shape::Umbrella | Shape::Weeping => 5,
        _ => 3,
    }
}

impl Generator {
    /// The noise that picks where the Brazilian biomes take over their parent climate.
    fn distant_noise(&self, x: f32, z: f32) -> f32 {
        self.variant.fbm2(x / 640.0 + 913.0, z / 640.0 - 411.0, 2)
    }

    /// The Pantanal: hot lowlands between the savanna and the jungle flatten out around sea
    /// level, so about half the ground is under a hand of water. Returns how much of it this
    /// column is (0..1) and changes the height `h`.
    pub(super) fn pantanal_shape(&self, x: f32, z: f32, climate: f32, hum: f32, detail: f32, h: &mut f32) -> f32 {
        let w = smoothstep(0.16, 0.24, climate) * smoothstep(0.02, 0.1, hum) * smoothstep(0.36, 0.28, hum) * smoothstep(0.04, 0.14, self.distant_noise(x, z));
        if w > 0.0 {
            let sea = SEA_LEVEL as f32;
            let (lo, hi) = (sea - 10.0, sea + 18.0);
            if *h > lo && *h < hi {
                let edge = ((*h - lo) / 4.0).min((hi - *h) / 7.0).min(1.0);
                let target = sea + 0.2 + detail * 2.4;
                *h += (target - *h) * w * edge;
            }
        }
        w
    }

    /// The biome the base climate picked, or the Brazilian one that takes its place here.
    pub(super) fn distant_biome(&self, b: Biome, x: f32, z: f32, temp: f32, height: i32, pantanal: f32) -> Biome {
        if pantanal > 0.5 && (SEA_LEVEL - 5..=SEA_LEVEL + 4).contains(&height) && !matches!(b, Biome::Badlands | Biome::Mountains) {
            return Biome::Pantanal;
        }
        let dl = self.distant_noise(x, z);
        match b {
            Biome::Jungle if dl > 0.12 => Biome::MataAtlantica,
            Biome::Savanna if dl > 0.1 => Biome::Cerrado,
            Biome::Desert if dl > 0.08 => Biome::Caatinga,
            Biome::Forest | Biome::BirchForest | Biome::DarkForest if dl > 0.24 && temp < 0.12 => Biome::Araucaria,
            _ => b,
        }
    }

    /// Top, filler and filler depth of a column in one of these biomes.
    pub(super) fn distant_top_blocks(&self, col: &Column, wx: i32, wz: i32) -> (BlockId, BlockId, i32) {
        let p = &self.pal;
        let d = &self.distant;
        let or = |a: BlockId, b: BlockId| if a == AIR { b } else { a };
        let n = self.detail.noise2(wx as f32 / 23.0 + 17.0, wz as f32 / 23.0 - 5.0);
        match col.biome {
            // Red earth showing through the grass, red under it.
            Biome::Cerrado if n > 0.28 => (or(d.red_soil, p.grass), or(d.red_soil, p.dirt), 3),
            Biome::Cerrado => (p.grass, or(d.red_soil, p.dirt), 3),
            Biome::Caatinga if n > 0.12 => (or(d.cracked_dirt, p.sand), p.dirt, 3),
            Biome::Caatinga if n < -0.3 => (p.sand, p.sandstone, 3),
            // Mud along the water's edge and on the bottom of the pools.
            Biome::Pantanal if col.height <= SEA_LEVEL => (or(d.mud, p.dirt), p.dirt, 3),
            _ => (p.grass, p.dirt, 3),
        }
    }

    /// The leaves that show from afar, and how much ground they cover (see `surface`).
    pub(super) fn distant_tree_cover(&self, b: Biome) -> Option<(BlockId, f32)> {
        let d = &self.distant;
        let (leaves, cover) = match b {
            Biome::MataAtlantica => (d.jabuticaba.leaves, 1.0),
            Biome::Cerrado => (d.acacia_leaves, 0.3),
            Biome::Pantanal => (d.ipe_pink.leaves, 0.15),
            Biome::Araucaria => (d.araucaria.leaves, 0.6),
            Biome::Caatinga => (d.acacia_leaves, 0.12),
            _ => return None,
        };
        (leaves != AIR).then_some((leaves, cover))
    }

    /// Trees, fallen logs, cacti, termite mounds and plants of the Brazilian biomes. Runs on its
    /// own random stream after the base game's trees, so other chunks keep their places.
    pub(super) fn decorate_distant(&self, chunk: &mut Chunk, cols: &[Column], pos: ChunkPos) {
        let biome = cols[8 + 8 * 16].biome;
        if !cols.iter().any(|c| is_distant(c.biome)) {
            return;
        }
        let mut rng = Rng::for_pos(self.seed, pos.x, pos.y, 3);
        let rng = &mut rng;
        if is_distant(biome) {
            let attempts = match biome {
                Biome::MataAtlantica => 16,
                Biome::Cerrado => 5,
                Biome::Pantanal => 3,
                Biome::Araucaria => 6,
                _ => 7,
            };
            for _ in 0..attempts {
                self.distant_tree(chunk, rng, cols, biome);
            }
            match biome {
                Biome::MataAtlantica if rng.chance(0.4) => self.fallen_log(chunk, rng, cols),
                Biome::Cerrado if rng.chance(0.35) => self.termite_mound(chunk, rng, cols),
                _ => {}
            }
        }
        self.distant_plants(chunk, rng, cols);
    }

    /// One tree attempt in a chunk whose middle is in `biome`.
    fn distant_tree(&self, chunk: &mut Chunk, rng: &mut Rng, cols: &[Column], biome: Biome) {
        let p = &self.pal;
        let d = &self.distant;
        enum T {
            Mod(ModTree),
            Base(TreeKind),
            Twisted { leafy: f32 },
            Cactus,
        }
        let r = rng.f32();
        let t = match biome {
            Biome::MataAtlantica => {
                if r < 0.3 {
                    T::Mod(d.jabuticaba)
                } else if r < 0.4 {
                    T::Mod(if rng.chance(0.6) { d.ipe_pink } else { d.ipe_purple })
                } else if r < 0.55 {
                    T::Mod(d.acai)
                } else if r < 0.65 {
                    T::Base(TreeKind::MegaJungle)
                } else if r < 0.85 {
                    T::Base(TreeKind::Jungle)
                } else {
                    T::Base(TreeKind::Bush(Wood::Jungle))
                }
            }
            Biome::Cerrado => {
                if r < 0.6 {
                    T::Twisted { leafy: 1.0 }
                } else if r < 0.8 {
                    T::Mod(d.ipe_yellow)
                } else {
                    T::Mod(d.buriti)
                }
            }
            Biome::Pantanal => {
                if r < 0.5 {
                    T::Mod(d.ipe_pink)
                } else if r < 0.85 {
                    T::Mod(d.buriti)
                } else {
                    T::Base(TreeKind::Bush(Wood::Oak))
                }
            }
            Biome::Araucaria => {
                if r < 0.75 {
                    T::Mod(d.araucaria)
                } else if r < 0.9 {
                    T::Base(TreeKind::Round(Wood::Oak))
                } else {
                    T::Base(TreeKind::Bush(Wood::Spruce))
                }
            }
            _ => {
                if r < 0.4 {
                    T::Cactus
                } else if r < 0.75 {
                    T::Twisted { leafy: 0.25 }
                } else {
                    T::Base(TreeKind::Bush(Wood::Acacia))
                }
            }
        };
        let m = match &t {
            T::Mod(mt) if mt.log == AIR || mt.leaves == AIR => return,
            T::Mod(mt) => margin(mt.shape),
            T::Base(k) => k.reach() + k.wide_trunk() as i32,
            T::Twisted { .. } => 4,
            T::Cactus => 2,
        };
        let x = rng.range(m, 15 - m);
        let z = rng.range(m, 15 - m);
        let col = &cols[(x + z * 16) as usize];
        let y = col.height;
        if !is_distant(col.biome) || !(SEA_LEVEL..=200).contains(&y) {
            return;
        }
        let ground = chunk.get(x as usize, y as usize, z as usize);
        let soil = ground == p.grass || (ground != AIR && (ground == d.red_soil || ground == d.cracked_dirt));
        if !soil || chunk.get(x as usize, y as usize + 1, z as usize) != AIR {
            return;
        }
        match t {
            T::Mod(mt) => modtrees::grow(mt, &mut ChunkSpace(chunk), rng, x, y, z),
            T::Base(k) => {
                if ground == p.grass {
                    trees::grow(k, &self.trees, &mut ChunkSpace(chunk), rng, x, y, z);
                }
            }
            T::Twisted { leafy } => {
                if d.acacia_log != AIR {
                    twisted_tree(&mut ChunkSpace(chunk), rng, d.acacia_log, d.acacia_leaves, leafy, x, y, z);
                }
            }
            T::Cactus => {
                if d.mandacaru != AIR {
                    mandacaru(&mut ChunkSpace(chunk), rng, d.mandacaru, d.mandacaru_flower, x, y, z);
                }
            }
        }
    }

    /// A fallen hollow trunk lying on flat ground.
    fn fallen_log(&self, chunk: &mut Chunk, rng: &mut Rng, cols: &[Column]) {
        let d = &self.distant;
        if d.hollow[0] == AIR {
            return;
        }
        let along_x = rng.chance(0.5);
        let len = rng.range(3, 5);
        let (x0, z0) = (rng.range(1, 14 - len), rng.range(1, 14 - len));
        let y = cols[(x0 + z0 * 16) as usize].height;
        let cells: Vec<(i32, i32)> = (0..len).map(|k| if along_x { (x0 + k, z0) } else { (x0, z0 + k) }).collect();
        let flat = cells.iter().all(|&(x, z)| {
            cols[(x + z * 16) as usize].height == y && chunk.get(x as usize, y as usize, z as usize) == self.pal.grass && chunk.get(x as usize, y as usize + 1, z as usize) == AIR
        });
        if !flat || y < SEA_LEVEL {
            return;
        }
        let id = d.hollow[if along_x { 0 } else { 1 }];
        for (x, z) in cells {
            chunk.set(x as usize, y as usize + 1, z as usize, id);
        }
    }

    /// A cupinzeiro: a lump of red earth standing out of the grass.
    fn termite_mound(&self, chunk: &mut Chunk, rng: &mut Rng, cols: &[Column]) {
        let soil = self.distant.red_soil;
        if soil == AIR {
            return;
        }
        let (x, z) = (rng.range(2, 13), rng.range(2, 13));
        let y = cols[(x + z * 16) as usize].height;
        if !(SEA_LEVEL..=200).contains(&y) {
            return;
        }
        let tall = rng.range(2, 3);
        for k in 1..=tall {
            if chunk.get(x as usize, (y + k) as usize, z as usize) == AIR {
                chunk.set(x as usize, (y + k) as usize, z as usize, soil);
            }
        }
        // A wider foot on one or two sides.
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, nz) = ((x + dx) as usize, (z + dz) as usize);
            if rng.chance(0.4) && cols[nx + nz * 16].height == y && chunk.get(nx, (y + 1) as usize, nz) == AIR {
                chunk.set(nx, (y + 1) as usize, nz, soil);
            }
        }
    }

    /// Ground plants of each column in the Brazilian biomes; other columns are left alone.
    fn distant_plants(&self, chunk: &mut Chunk, rng: &mut Rng, cols: &[Column]) {
        let p = &self.pal;
        let d = &self.distant;
        let s = SEA_LEVEL as usize;
        for z in 0..16usize {
            for x in 0..16usize {
                let col = &cols[x + z * 16];
                if !is_distant(col.biome) {
                    continue;
                }
                let y = col.height;
                if col.biome == Biome::Pantanal && y < SEA_LEVEL {
                    // Water hyacinths and lily pads on the pools.
                    if chunk.get(x, s, z) == p.water && chunk.get(x, s + 1, z) == AIR {
                        let r = rng.f32();
                        let pick = if r < 0.07 { d.aguape } else if r < 0.1 { p.lily_pad } else { AIR };
                        if pick != AIR {
                            chunk.set(x, s + 1, z, pick);
                        }
                    }
                    continue;
                }
                if !(SEA_LEVEL..=250).contains(&y) || chunk.get(x, y as usize + 1, z) != AIR {
                    continue;
                }
                let ground = chunk.get(x, y as usize, z);
                let r = rng.f32();
                let pick = match col.biome {
                    Biome::MataAtlantica if ground == p.grass => {
                        if r < 0.03 {
                            d.bromelias[rng.below(2) as usize]
                        } else if r < 0.12 {
                            d.samambaia
                        } else if r < 0.42 {
                            p.short_grass
                        } else {
                            AIR
                        }
                    }
                    Biome::Cerrado if ground == p.grass => {
                        if r < 0.04 {
                            d.capim_dourado
                        } else if r < 0.4 {
                            p.short_grass
                        } else if r < 0.41 {
                            p.dandelion
                        } else {
                            AIR
                        }
                    }
                    Biome::Pantanal if ground == p.grass => {
                        if r < 0.45 {
                            p.short_grass
                        } else if r < 0.47 {
                            p.poppy
                        } else {
                            AIR
                        }
                    }
                    Biome::Araucaria if ground == p.grass => {
                        if r < 0.12 {
                            d.samambaia
                        } else if r < 0.3 {
                            p.short_grass
                        } else {
                            AIR
                        }
                    }
                    Biome::Caatinga => {
                        if r < 0.03 {
                            d.jurema
                        } else if r < 0.05 {
                            p.dead_bush
                        } else if r < 0.06 && ground == p.grass {
                            d.bromelias[0]
                        } else if r < 0.12 && ground == p.grass {
                            p.short_grass
                        } else {
                            AIR
                        }
                    }
                    _ => AIR,
                };
                if pick != AIR {
                    chunk.set(x, y as usize + 1, z, pick);
                }
            }
        }
    }
}

/// A small gnarled tree: a crooked trunk that wanders a block now and then, two or three
/// short branches and flat little crowns (cerrado; bare and grey in the caatinga).
fn twisted_tree<W: trees::TreeSpace>(w: &mut W, rng: &mut Rng, log: BlockId, leaves: BlockId, leafy: f32, x: i32, y: i32, z: i32) {
    let set_log = |w: &mut W, x: i32, y: i32, z: i32| {
        let g = w.get(x, y, z);
        if g == AIR || g == leaves {
            w.set(x, y, z, log);
        }
    };
    let h = rng.range(3, 5);
    let (mut cx, mut cz) = (x, z);
    for k in 1..=h {
        if k > 1 && rng.chance(0.45) {
            let (dx, dz) = [(1, 0), (-1, 0), (0, 1), (0, -1)][rng.below(4) as usize];
            if (cx + dx - x).abs() <= 1 && (cz + dz - z).abs() <= 1 {
                cx += dx;
                cz += dz;
                set_log(w, cx, y + k - 1, cz);
            }
        }
        set_log(w, cx, y + k, cz);
    }
    let top = y + h;
    let n = 2 + rng.below(2) as i32;
    let mut ends = vec![(cx, top, cz)];
    for _ in 0..n {
        let (dx, dz) = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)][rng.below(8) as usize];
        let len = rng.range(1, 2);
        let (mut bx, mut by, mut bz) = (cx, top - rng.range(0, 1), cz);
        for _ in 0..len {
            bx += dx;
            bz += dz;
            by += i32::from(rng.chance(0.5));
            set_log(w, bx, by, bz);
        }
        ends.push((bx, by, bz));
    }
    if leaves == AIR {
        return;
    }
    for (ex, ey, ez) in ends {
        for dy in 0..=1 {
            let r: i32 = 2 - dy;
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dz * dz > r * r + 1 || !rng.chance(leafy * if dx.abs() == r || dz.abs() == r { 0.6 } else { 0.95 }) {
                        continue;
                    }
                    let (lx, ly, lz) = (ex + dx, ey + dy, ez + dz);
                    if w.get(lx, ly, lz) == AIR {
                        w.set(lx, ly, lz, leaves);
                    }
                }
            }
        }
    }
}

/// A mandacaru: a ribbed column with a few arms that turn up, sometimes in flower.
fn mandacaru<W: trees::TreeSpace>(w: &mut W, rng: &mut Rng, cactus: BlockId, flower: BlockId, x: i32, y: i32, z: i32) {
    let put = |w: &mut W, x: i32, y: i32, z: i32, id: BlockId| {
        if w.get(x, y, z) == AIR {
            w.set(x, y, z, id);
        }
    };
    let h = rng.range(3, 6);
    for k in 1..=h {
        put(w, x, y + k, z, cactus);
    }
    if flower != AIR && rng.chance(0.3) {
        put(w, x, y + h + 1, z, flower);
    }
    let mut dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for i in 0..4 {
        let j = i + rng.below(4 - i as u32) as usize;
        dirs.swap(i, j);
    }
    let arms = if h >= 4 { rng.range(1, 3) } else { rng.range(0, 1) };
    for &(dx, dz) in dirs.iter().take(arms as usize) {
        let at = rng.range(2, h - 1);
        let up = rng.range(1, (h - at + 1).max(1));
        put(w, x + dx, y + at, z + dz, cactus);
        for k in 1..=up {
            put(w, x + dx, y + at + k, z + dz, cactus);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::ChunkPos;

    fn registry() -> BlockRegistry {
        BlockRegistry::from_ron_many(&[include_str!("../../../../packs/vanilla/blocks.ron"), include_str!("../../../../packs/vanilla/mods/blocks.ron")]).unwrap()
    }

    /// The middle of a patch of `biome` near the origin (searching outward).
    fn find(g: &Generator, biome: Biome) -> Option<(i32, i32)> {
        for r in (0..12000).step_by(64) {
            for k in 0..(r / 64).max(1) * 8 {
                let a = k as f32 / ((r / 64).max(1) * 8) as f32 * std::f32::consts::TAU;
                let (x, z) = ((a.cos() * r as f32) as i32, (a.sin() * r as f32) as i32);
                if g.biome_at(x, z) == biome && [(24, 0), (-24, 0), (0, 24), (0, -24)].iter().all(|(dx, dz)| g.biome_at(x + dx, z + dz) == biome) {
                    return Some((x, z));
                }
            }
        }
        None
    }

    /// Where each Brazilian biome is (for screenshots):
    /// `SEED=1 cargo test -p voxel --release -- --ignored --nocapture print_brazilian`.
    #[test]
    #[ignore]
    fn print_brazilian_biomes() {
        let reg = registry();
        let seed = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1234);
        let g = Generator::new(seed, &reg);
        for biome in [Biome::MataAtlantica, Biome::Cerrado, Biome::Pantanal, Biome::Araucaria, Biome::Caatinga] {
            let at = find(&g, biome).map(|(x, z)| (x, g.height_at(x, z), z));
            println!("{:>20}: {:?}", biome.name(), at);
        }
    }

    #[test]
    fn brazilian_biomes_grow_their_own_plants() {
        let reg = registry();
        let g = Generator::new(1234, &reg);
        let id = |n: &str| reg.id(n).unwrap();
        for (biome, wants) in [
            (Biome::MataAtlantica, vec![id("jabuticaba_log"), id("buriti_log")]),
            (Biome::Cerrado, vec![id("acacia_log"), id("capim_dourado")]),
            (Biome::Pantanal, vec![id("water"), id("mud")]),
            (Biome::Araucaria, vec![id("araucaria_log"), id("samambaia")]),
            (Biome::Caatinga, vec![id("mandacaru"), id("jurema")]),
        ] {
            let (x, z) = find(&g, biome).unwrap_or_else(|| panic!("no {}", biome.name()));
            let mut seen = vec![false; wants.len()];
            for cz in -3..=3 {
                for cx in -3..=3 {
                    let c = g.generate(ChunkPos::new((x >> 4) + cx, (z >> 4) + cz));
                    for b in c.all_blocks() {
                        if let Some(i) = wants.iter().position(|&w| w == b) {
                            seen[i] = true;
                        }
                    }
                }
            }
            for (w, s) in wants.iter().zip(seen) {
                assert!(s, "no {} in the {} at {x},{z}", reg.get(*w).name, biome.name());
            }
        }
    }

    #[test]
    fn brazilian_trees_stay_in_their_chunk_and_keep_surface_in_step() {
        let reg = registry();
        let g = Generator::new(77, &reg);
        for biome in [Biome::MataAtlantica, Biome::Cerrado, Biome::Pantanal, Biome::Araucaria, Biome::Caatinga] {
            let Some((x, z)) = find(&g, biome) else { panic!("no {}", biome.name()) };
            let (cx, cz) = (x >> 4, z >> 4);
            let c = g.generate(ChunkPos::new(cx, cz));
            for (lx, lz) in [(0usize, 0usize), (7, 3), (15, 15)] {
                let s = g.surface(cx * 16 + lx as i32, cz * 16 + lz as i32);
                let at = c.get(lx, s.height as usize, lz);
                // A tree's roots turn the grass under its trunk to dirt.
                assert!(at == s.top || at == reg.id_or_air("dirt"), "{}: {} instead of {}", biome.name(), reg.get(at).name, reg.get(s.top).name);
            }
        }
    }
}
