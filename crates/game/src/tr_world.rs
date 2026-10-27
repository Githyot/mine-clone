//! Lote 3 of the Terraria side (plan/terraria-completo.md): the lands that change the world.
//! - Each world has one evil land, the Ruin (purple, chasms with shadow orbs) or the Blight
//!   (red, caves with beating hearts), in patches a few hundred blocks apart. Their grass,
//!   stone and sand turn, an altar stands in the middle and their own ore runs in the stone.
//! - Once the World Breaker falls the Prism (pastel, pixies and unicorns) appears too, and
//!   in that age the lands creep into the blocks around them.
//! - Breaking orbs and hearts gives a reward, brings a meteor the first time and wakes the
//!   land's boss every third time. Purification powder turns the lands back; vile powder
//!   spreads the evil.
//! The patches are laid over chunks already made, a few at a time, like the hard mode ores.

use std::collections::HashSet;

use glam::{IVec2, IVec3, Vec3};
use voxel::block::{BlockId, Shape};
use voxel::chunk::CHUNK_SIZE;
use voxel::item::ItemStack;
use voxel::physics::Terrain;
use voxel::rng::Rng;
use voxel::worldgen::Dim;

use crate::audio::Sfx;
use crate::Game;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Land {
    Ruin,
    Blight,
    Prism,
}

const LANDS: [Land; 3] = [Land::Ruin, Land::Blight, Land::Prism];

impl Land {
    fn key(self) -> &'static str {
        match self {
            Land::Ruin => "ruin",
            Land::Blight => "blight",
            Land::Prism => "prism",
        }
    }
    fn idx(self) -> usize {
        self as usize
    }
}

/// Patches are placed one per cell of this many blocks (or none).
const CELL: i32 = 384;
/// Chasms or heart caves per evil patch.
const FEATURES: usize = 3;

struct Zone {
    center: IVec2,
    radius: f32,
    land: Land,
    salt: u64,
}

fn mix(seed: u64, a: i32, b: i32, salt: u64) -> u64 {
    seed ^ (a as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (b as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ salt
}

/// The world's evil land.
pub fn world_evil(seed: u64) -> Land {
    if (seed >> 7) & 1 == 0 {
        Land::Ruin
    } else {
        Land::Blight
    }
}

/// The patches whose cells are around (x, z); the Prism only in hard mode.
fn zones_near(seed: u64, x: i32, z: i32, prism: bool) -> Vec<Zone> {
    let (cx, cz) = (x.div_euclid(CELL), z.div_euclid(CELL));
    let mut out = Vec::new();
    for dx in -1..=1 {
        for dz in -1..=1 {
            let (gx, gz) = (cx + dx, cz + dz);
            let mut rng = Rng::new(mix(seed, gx, gz, 0x5EED_1A4D));
            let roll = rng.f32();
            let land = if roll < 0.5 && (gx, gz) != (0, 0) {
                world_evil(seed)
            } else if prism && roll > 0.6 {
                Land::Prism
            } else {
                continue;
            };
            let center = IVec2::new(gx * CELL + rng.range(64, CELL - 64), gz * CELL + rng.range(64, CELL - 64));
            let radius = 40.0 + rng.f32() * 30.0;
            out.push(Zone { center, radius, land, salt: mix(seed, gx, gz, 0xFEA7) });
        }
    }
    out
}

fn in_zone(z: &Zone, x: i32, wz: i32) -> bool {
    let d = Vec3::new((x - z.center.x) as f32, 0.0, (wz - z.center.y) as f32);
    let a = d.z.atan2(d.x);
    let wob = (a * 5.0 + (z.salt % 97) as f32).sin() * 0.12 + (a * 3.0 + (z.salt % 31) as f32).cos() * 0.08;
    d.length() < z.radius * (0.9 + wob)
}

/// A chasm or heart cave of a patch: where it is and how deep it goes.
fn features(z: &Zone) -> Vec<IVec2> {
    let mut rng = Rng::new(z.salt ^ 0xC4A5);
    (0..FEATURES)
        .map(|_| {
            let a = rng.f32() * std::f32::consts::TAU;
            let r = rng.f32() * z.radius * 0.55;
            z.center + IVec2::new((a.cos() * r) as i32, (a.sin() * r) as i32)
        })
        .collect()
}

/// The blocks of the lands, looked up once.
pub struct LandIds {
    grass: [BlockId; 3],
    stone: [BlockId; 3],
    sand: [BlockId; 3],
    ore: [BlockId; 2],
    orb: [BlockId; 2],
    altar: [BlockId; 2],
    from_grass: Vec<BlockId>,
    from_stone: Vec<BlockId>,
    from_sand: Vec<BlockId>,
    pure: [BlockId; 3],
}

#[derive(Default)]
pub struct Lands {
    pub(crate) done: HashSet<(i32, i32)>,
    pub(crate) prism_done: HashSet<(i32, i32)>,
    pub(crate) orbs: u32,
    ids: Option<LandIds>,
    scan_t: f32,
    spread_t: f32,
    spawn_t: f32,
    /// The land the player stands in (for the sky and the monsters).
    pub(crate) here: Option<Land>,
    here_k: f32,
}

impl Game {
    fn land_ids(&mut self) -> &LandIds {
        if self.t5.tr.lands.ids.is_none() {
            let r = &self.reg;
            let id = |n: String| r.id_or_air(&n);
            let ids = |names: &[&str]| names.iter().map(|n| r.id_or_air(n)).filter(|&i| i != voxel::AIR).collect::<Vec<_>>();
            let l = LandIds {
                grass: LANDS.map(|l| id(format!("{}_grass", l.key()))),
                stone: LANDS.map(|l| id(format!("{}_stone", l.key()))),
                sand: LANDS.map(|l| id(format!("{}_sand", l.key()))),
                ore: [id("umbral_ore".into()), id("sanguinite_ore".into())],
                orb: [id("ruin_orb".into()), id("blight_heart".into())],
                altar: [id("ruin_altar".into()), id("blight_altar".into())],
                from_grass: ids(&["grass_block", "podzol", "mycelium", "dirt_path"]),
                from_stone: ids(&["stone", "deepslate", "andesite", "diorite", "granite", "tuff", "cobblestone", "calcite"]),
                from_sand: ids(&["sand", "red_sand"]),
                pure: [r.id_or_air("grass_block"), r.id_or_air("stone"), r.id_or_air("sand")],
            };
            self.t5.tr.lands.ids = Some(l);
        }
        self.t5.tr.lands.ids.as_ref().unwrap()
    }

    /// What a block turns into in `land` (None: it stays).
    fn convert(ids: &LandIds, id: BlockId, land: Land) -> Option<BlockId> {
        let i = land.idx();
        let to = if ids.from_grass.contains(&id) || ids.grass.contains(&id) {
            ids.grass[i]
        } else if ids.from_stone.contains(&id) || ids.stone.contains(&id) {
            ids.stone[i]
        } else if ids.from_sand.contains(&id) || ids.sand.contains(&id) {
            ids.sand[i]
        } else {
            return None;
        };
        (to != id && to != voxel::AIR).then_some(to)
    }

    /// Which land a block belongs to.
    fn land_of(ids: &LandIds, id: BlockId) -> Option<Land> {
        LANDS.into_iter().find(|l| ids.grass[l.idx()] == id || ids.stone[l.idx()] == id || ids.sand[l.idx()] == id)
    }

    pub(crate) fn update_lands(&mut self, dt: f32) {
        if self.dim != Dim::Overworld {
            self.t5.tr.lands.here = None;
            return;
        }
        let l = &mut self.t5.tr.lands;
        l.scan_t -= dt;
        l.spread_t -= dt;
        l.spawn_t -= dt;
        if l.scan_t <= 0.0 {
            self.t5.tr.lands.scan_t = 0.3;
            self.seed_lands();
            self.notice_land();
        }
        if self.t5.hard.on && self.t5.tr.lands.spread_t <= 0.0 {
            self.t5.tr.lands.spread_t = 0.5;
            self.spread_lands();
        }
        if self.t5.tr.lands.spawn_t <= 0.0 {
            self.t5.tr.lands.spawn_t = 7.0;
            self.land_spawns();
        }
        let want = if self.t5.tr.lands.here.is_some() { 1.0 } else { 0.0 };
        let k = &mut self.t5.tr.lands.here_k;
        *k += (want - *k) * (dt * 0.5).min(1.0);
    }

    /// The land under the player's feet (or of the patch they are in).
    fn notice_land(&mut self) {
        let p = self.body.pos.floor().as_ivec3();
        let prism = self.t5.hard.on;
        let seed = self.seed;
        let mut here = zones_near(seed, p.x, p.z, prism).into_iter().find(|z| in_zone(z, p.x, p.z)).map(|z| z.land);
        if here.is_none() {
            let below: Vec<BlockId> = (1..4).map(|k| self.world.block_or_air(p - IVec3::Y * k)).collect();
            let ids = self.land_ids();
            here = below.into_iter().find_map(|b| Self::land_of(ids, b));
        }
        self.t5.tr.lands.here = here;
    }

    /// Lay the lands over loaded chunks near the player that don't have them yet.
    fn seed_lands(&mut self) {
        let center = IVec2::new((self.body.pos.x.floor() as i32).div_euclid(CHUNK_SIZE as i32), (self.body.pos.z.floor() as i32).div_euclid(CHUNK_SIZE as i32));
        let prism = self.t5.hard.on;
        let mut work = 0;
        for r in 0..=5i32 {
            for dx in -r..=r {
                for dz in -r..=r {
                    if dx.abs() != r && dz.abs() != r {
                        continue;
                    }
                    let cp = center + IVec2::new(dx, dz);
                    let key = (cp.x, cp.y);
                    let need = !self.t5.tr.lands.done.contains(&key) || (prism && !self.t5.tr.lands.prism_done.contains(&key));
                    if !need || self.world.chunk(cp).is_none() {
                        continue;
                    }
                    if (0..9).any(|i| self.world.chunk(cp + IVec2::new(i % 3 - 1, i / 3 - 1)).is_none()) {
                        continue;
                    }
                    let evil_pass = !self.t5.tr.lands.done.contains(&key);
                    self.t5.tr.lands.done.insert(key);
                    if prism {
                        self.t5.tr.lands.prism_done.insert(key);
                    }
                    if self.lay_lands(cp, evil_pass, prism) {
                        work += 1;
                        if work >= 2 {
                            return;
                        }
                    }
                }
            }
        }
    }

    /// One chunk: true when something in it changed.
    fn lay_lands(&mut self, cp: IVec2, evil: bool, prism: bool) -> bool {
        let base = IVec2::new(cp.x * CHUNK_SIZE as i32, cp.y * CHUNK_SIZE as i32);
        let mid = base + IVec2::splat(CHUNK_SIZE as i32 / 2);
        let zones: Vec<Zone> = zones_near(self.seed, mid.x, mid.y, prism)
            .into_iter()
            .filter(|z| if z.land == Land::Prism { prism } else { evil })
            .filter(|z| (z.center - mid).as_vec2().length() < z.radius * 1.1 + 24.0)
            .collect();
        if zones.is_empty() {
            return false;
        }
        self.land_ids();
        let ids = self.t5.tr.lands.ids.take().unwrap();
        let mut changed = false;
        for z in &zones {
            for lx in 0..CHUNK_SIZE as i32 {
                for lz in 0..CHUNK_SIZE as i32 {
                    let (x, wz) = (base.x + lx, base.y + lz);
                    if !in_zone(z, x, wz) {
                        continue;
                    }
                    let Some(top) = self.world.surface_y(x, wz) else { continue };
                    // Plants die on the evil lands.
                    let plant = IVec3::new(x, top + 1, wz);
                    let pid = self.world.block_or_air(plant);
                    if z.land != Land::Prism && pid != voxel::AIR && self.reg.get(pid).shape == Shape::Cross {
                        self.world.set_block(plant, voxel::AIR);
                    }
                    for y in (top - 45).max(-60)..=top {
                        let p = IVec3::new(x, y, wz);
                        let id = self.world.block_or_air(p);
                        if id == voxel::AIR {
                            continue;
                        }
                        if let Some(to) = Self::convert(&ids, id, z.land) {
                            self.world.set_block(p, to);
                            changed = true;
                        }
                    }
                }
            }
            if z.land != Land::Prism {
                self.lay_features(z, base, &ids);
                // Veins of the land's ore in its stone.
                let ore = ids.ore[z.land.idx()];
                let stone = ids.stone[z.land.idx()];
                let mut rng = Rng::new(mix(self.seed, cp.x, cp.y, 0x0E5E));
                for _ in 0..4 {
                    let x = base.x + rng.range(1, 15);
                    let wz = base.y + rng.range(1, 15);
                    if !in_zone(z, x, wz) {
                        continue;
                    }
                    let h = self.world.gen.height_at(x, wz);
                    let mut p = IVec3::new(x, h - rng.range(8, 40), wz);
                    for _ in 0..6 {
                        if self.world.block_or_air(p) == stone {
                            self.world.set_block(p, ore);
                        }
                        p += [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z][rng.below(6) as usize];
                    }
                }
            }
        }
        self.t5.tr.lands.ids = Some(ids);
        changed
    }

    /// The chasms (Ruin) or heart caves (Blight) of a patch and its altar, the parts of them
    /// inside the chunk at `base`.
    fn lay_features(&mut self, z: &Zone, base: IVec2, ids: &LandIds) {
        let land = z.land.idx();
        let lo = base;
        let hi = base + IVec2::splat(CHUNK_SIZE as i32 - 1);
        let inside = |p: IVec3| p.x >= lo.x && p.x <= hi.x && p.z >= lo.y && p.z <= hi.y;
        let stone = ids.stone[land];
        for (k, f) in features(z).into_iter().enumerate() {
            if f.x + 8 < lo.x || f.x - 8 > hi.x || f.y + 8 < lo.y || f.y - 8 > hi.y {
                continue;
            }
            let h = self.world.gen.height_at(f.x, f.y);
            let depth = 30 + (z.salt as i32 + k as i32 * 7).rem_euclid(10);
            let phase = (z.salt % 13) as f32 + k as f32;
            for x in lo.x.max(f.x - 8)..=hi.x.min(f.x + 8) {
                for wz in lo.y.max(f.y - 8)..=hi.y.min(f.y + 8) {
                    for y in (h - depth - 6)..=(h + 3) {
                        let p = IVec3::new(x, y, wz);
                        if !inside(p) {
                            continue;
                        }
                        let down = (h - y) as f32;
                        // The shaft wanders a little as it goes down.
                        let sx = f.x as f32 + (down * 0.15 + phase).sin() * 2.0;
                        let sz = f.y as f32 + (down * 0.11 + phase).cos() * 1.5;
                        let ds = Vec3::new(x as f32 - sx, 0.0, wz as f32 - sz).length();
                        let pocket = Vec3::new((x - f.x) as f32, (y - (h - depth)) as f32 * 1.3, (wz - f.y) as f32).length();
                        let (shaft_r, pocket_r) = if z.land == Land::Ruin { (2.3, 4.5) } else { (1.4, 5.5) };
                        let air = (y >= h - depth && ds < shaft_r) || pocket < pocket_r;
                        let wall = (y >= h - depth && ds < shaft_r + 1.3) || pocket < pocket_r + 1.3;
                        let id = self.world.block_or_air(p);
                        if air {
                            if id != voxel::AIR && !self.world.liquid(p) {
                                self.world.set_block(p, voxel::AIR);
                            }
                        } else if wall && id != voxel::AIR && !self.world.liquid(p) && y < h {
                            self.world.set_block(p, stone);
                        }
                    }
                }
            }
            let orb = IVec3::new(f.x, h - depth, f.y);
            if inside(orb) {
                self.world.set_block(orb, ids.orb[land]);
            }
        }
        let altar = IVec3::new(z.center.x, 0, z.center.y);
        if inside(altar) {
            let h = self.world.surface_y(altar.x, altar.z).unwrap_or(self.world.gen.height_at(altar.x, altar.z));
            for dx in -1..=1 {
                self.world.set_block(IVec3::new(altar.x + dx, h + 1, altar.z), ids.altar[land]);
            }
        }
    }

    /// Hard mode: the lands creep into the blocks around them.
    fn spread_lands(&mut self) {
        self.land_ids();
        let ids = self.t5.tr.lands.ids.take().unwrap();
        let me = self.body.pos.floor().as_ivec3();
        for _ in 0..48 {
            let r = &mut self.surv.rng;
            let p = me + IVec3::new(r.range(-32, 33), r.range(-16, 17), r.range(-32, 33));
            let id = self.world.block_or_air(p);
            let Some(land) = Self::land_of(&ids, id) else { continue };
            let r = &mut self.surv.rng;
            let q = p + IVec3::new(r.range(-2, 3), r.range(-2, 3), r.range(-2, 3));
            let target = self.world.block_or_air(q);
            if Self::land_of(&ids, target).is_some() {
                continue;
            }
            if let Some(to) = Self::convert(&ids, target, land) {
                self.world.set_block(q, to);
            }
        }
        self.t5.tr.lands.ids = Some(ids);
    }

    fn land_spawns(&mut self) {
        let Some(land) = self.t5.tr.lands.here else { return };
        if self.mobs.peaceful || self.mobs.calm || !self.survival {
            return;
        }
        let names: &[&str] = match land {
            Land::Ruin => &["soul_eater"],
            Land::Blight => &["blight_flyer", "blight_brute"],
            Land::Prism => &["prism_pixie", "prism_unicorn"],
        };
        let alive = self.mobs.list.iter().filter(|m| m.alive() && names.contains(&self.mobs.reg.defs[m.def].name.as_str())).count();
        if alive >= 4 || (land == Land::Prism && !self.t5.hard.on) {
            return;
        }
        let k = (self.surv.rng.f32() * names.len() as f32) as usize % names.len();
        let name = names[k];
        let flies = matches!(name, "soul_eater" | "blight_flyer" | "prism_pixie");
        self.spawn_around(name, flies);
    }

    /// The sky of the lands, mixed in as you walk into them.
    pub(crate) fn land_sky(&self, top: &mut [f32; 3], horizon: &mut [f32; 3]) {
        let k = self.t5.tr.lands.here_k;
        if k < 0.02 {
            return;
        }
        let Some(land) = self.t5.tr.lands.here else { return };
        let (t, h) = match land {
            Land::Ruin => ([0.22, 0.12, 0.32], [0.42, 0.3, 0.5]),
            Land::Blight => ([0.32, 0.08, 0.08], [0.55, 0.28, 0.24]),
            Land::Prism => ([0.55, 0.75, 0.95], [0.95, 0.8, 0.95]),
        };
        let a = k * 0.6;
        for i in 0..3 {
            top[i] += (t[i] - top[i]) * a * top[i].max(0.15).min(1.0);
            horizon[i] += (h[i] - horizon[i]) * a * horizon[i].max(0.15).min(1.0);
        }
    }

    /// An orb or heart was broken: a reward, the first one calls a meteor, every third one
    /// wakes the land's boss.
    pub(crate) fn orb_broken(&mut self, p: IVec3, ruin: bool) {
        self.t5.tr.lands.orbs += 1;
        let n = self.t5.tr.lands.orbs;
        let at = p.as_vec3() + Vec3::splat(0.5);
        let rewards: &[(&str, u32)] = if ruin {
            &[("ruin_thorn", 1), ("star_band", 1), ("umbral_ingot", 6), ("shadow_scale", 5)]
        } else {
            &[("blight_rod", 1), ("panic_charm", 1), ("sanguinite_ingot", 6), ("blight_tissue", 5)]
        };
        let (name, count) = rewards[(n as usize * 7 + self.surv.rng.below(4) as usize) % rewards.len()];
        if let Some(id) = self.items.id(name) {
            self.spawn_drop(at, ItemStack::new(id, count as u8), Vec3::Y * 2.0);
        }
        let glow = self.glow_layer();
        let tint = if ruin { [1.2, 0.5, 2.0] } else { [2.0, 0.4, 0.4] };
        for _ in 0..30 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.3, self.surv.rng.f32() - 0.5) * 6.0;
            self.particles.glow(at, v, glow, tint, 0.12, 1.0);
        }
        self.sfx(Sfx::Thunder(0.3));
        if n == 1 {
            self.toast("Um meteoro foi atraído para este mundo...", 4.0);
            self.t5.tr.ev.meteor_in = Some(40.0);
        }
        if n % 3 == 0 && self.mobs.list.iter().all(|m| !(m.alive() && self.mobs.reg.defs[m.def].boss)) {
            if ruin {
                let spot = self.body.pos - Vec3::Y * 6.0;
                if self.mobs.spawn("deep_worm", spot) {
                    self.spawn_worm_body("deep_worm", spot);
                    self.t5.fight = Default::default();
                    self.toast("O Verme das Profundezas acordou com o barulho!", 4.0);
                }
            } else if self.spawn_tr_boss("blight_mind", at + Vec3::Y * 3.0) {
                self.toast("A Mente da Chaga despertou!", 4.0);
            }
        } else if n % 3 != 0 {
            self.toast("Algo se agita nas profundezas...", 3.0);
        }
    }

    /// An altar was smashed (only a World Breaker age pickaxe can): the land bursts out.
    pub(crate) fn altar_broken(&mut self, p: IVec3, ruin: bool) {
        let land = if ruin { Land::Ruin } else { Land::Blight };
        self.land_ids();
        let ids = self.t5.tr.lands.ids.take().unwrap();
        for dx in -6..=6 {
            for dy in -4..=4 {
                for dz in -6..=6 {
                    if dx * dx + dy * dy + dz * dz > 36 {
                        continue;
                    }
                    let q = p + IVec3::new(dx, dy, dz);
                    if let Some(to) = Self::convert(&ids, self.world.block_or_air(q), land) {
                        self.world.set_block(q, to);
                    }
                }
            }
        }
        self.t5.tr.lands.ids = Some(ids);
        self.weather.flash = 0.4;
        self.sfx(Sfx::Thunder(0.6));
        self.toast("O altar se parte e a terra em volta apodrece!", 4.0);
        let name = if ruin { "soul_eater" } else { "blight_flyer" };
        for _ in 0..3 {
            self.spawn_around(name, true);
        }
    }

    /// Right click with purification or vile powder: the blocks around you turn.
    pub(crate) fn use_powder(&mut self, pure: bool) -> bool {
        self.land_ids();
        let ids = self.t5.tr.lands.ids.take().unwrap();
        let evil = world_evil(self.seed);
        let me = self.body.pos.floor().as_ivec3();
        let mut n = 0;
        for dx in -5..=5 {
            for dy in -3..=3 {
                for dz in -5..=5 {
                    if dx * dx + dy * dy + dz * dz > 25 {
                        continue;
                    }
                    let q = me + IVec3::new(dx, dy, dz);
                    let id = self.world.block_or_air(q);
                    let to = if pure {
                        Self::land_of(&ids, id).and_then(|_| {
                            let k = (0..3).find(|&k| ids.grass[k] == id).map(|_| 0).or_else(|| (0..3).find(|&k| ids.stone[k] == id).map(|_| 1)).unwrap_or(2);
                            Some(ids.pure[k])
                        })
                    } else {
                        Self::convert(&ids, id, evil)
                    };
                    if let Some(to) = to.filter(|&t| t != voxel::AIR) {
                        self.world.set_block(q, to);
                        n += 1;
                    }
                }
            }
        }
        self.t5.tr.lands.ids = Some(ids);
        let glow = self.glow_layer();
        let tint = if pure { [0.8, 1.8, 0.8] } else { [1.2, 0.5, 1.8] };
        for _ in 0..40 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() * 0.4, self.surv.rng.f32() - 0.5) * 8.0;
            self.particles.glow(self.body.pos + Vec3::Y, v, glow, tint, 0.08, 1.0);
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        if n == 0 && pure {
            self.toast("Não há nada para purificar aqui", 2.0);
        }
        true
    }

    /// Test: a patch of the world's evil (or the Prism) right in front of the player.
    pub(crate) fn test_land(&mut self, land: Land) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let at = self.body.pos + flat * 22.0;
        let z = Zone { center: IVec2::new(at.x as i32, at.z as i32), radius: 26.0, land, salt: 99 };
        self.land_ids();
        let ids = self.t5.tr.lands.ids.take().unwrap();
        let c = self.body.pos.floor().as_ivec3();
        for x in c.x - 50..c.x + 50 {
            for wz in c.z - 50..c.z + 50 {
                if !in_zone(&z, x, wz) {
                    continue;
                }
                let Some(top) = self.world.surface_y(x, wz) else { continue };
                let plant = IVec3::new(x, top + 1, wz);
                let pid = self.world.block_or_air(plant);
                if land != Land::Prism && pid != voxel::AIR && self.reg.get(pid).shape == Shape::Cross {
                    self.world.set_block(plant, voxel::AIR);
                }
                for y in top - 20..=top {
                    let p = IVec3::new(x, y, wz);
                    if let Some(to) = Self::convert(&ids, self.world.block_or_air(p), land) {
                        self.world.set_block(p, to);
                    }
                }
            }
        }
        if land != Land::Prism {
            for cx in (c.x - 50).div_euclid(16)..=(c.x + 50).div_euclid(16) {
                for cz in (c.z - 50).div_euclid(16)..=(c.z + 50).div_euclid(16) {
                    self.lay_features(&z, IVec2::new(cx * 16, cz * 16), &ids);
                }
            }
        }
        self.t5.tr.lands.ids = Some(ids);
        self.t5.tr.lands.here = Some(land);
        self.t5.tr.lands.here_k = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_are_stable_and_spaced() {
        let a = zones_near(42, 1000, -700, true);
        let b = zones_near(42, 1000, -700, true);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.center, y.center);
            assert_eq!(x.land, y.land);
        }
        // No evil right at the world's spawn cell.
        for z in zones_near(42, 10, 10, false) {
            assert!(!(z.center.x.div_euclid(CELL) == 0 && z.center.y.div_euclid(CELL) == 0));
        }
    }
}
