//! Life in the water: which kind of water a spot is (lake, river, swamp, sea by temperature and
//! depth, underground lake), the creatures that spawn in each, and schools of fish.
//!
//! Water creatures have their own head count (so a busy night on land doesn't empty the seas)
//! and spawn at a depth inside the water, not on top of it. A mob's `waters` in mobs.ron says
//! where it lives; the older creatures that predate it get theirs from `default_waters`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use glam::{IVec3, Vec3};
use voxel::mob::{Kind, Mob, MobDef, SpawnRule};
use voxel::worldgen::{Biome, Dim, SEA_LEVEL};
use voxel::physics::Terrain;
use voxel::AIR;

use super::Mobs;
use crate::world::World;

/// Test hook (MC_AGUAS): fill the water near the player with life and let it swim a few seconds
/// before the screenshot. `TEST_NEAR` spawns them close, where the camera sees them.
static TEST_FILL: AtomicBool = AtomicBool::new(false);
static TEST_NEAR: AtomicBool = AtomicBool::new(false);
/// Which way the test camera looks (the angle of its forward direction on the ground, as bits).
static TEST_AIM: AtomicU32 = AtomicU32::new(0);

/// Water creatures at once around the player, and how many of them may be monsters.
const MAX_WATER: usize = 26;
const MAX_WATER_HOSTILE: usize = 5;
/// Water creatures farther than this from every player go (fresh ones come near the player).
pub(super) const WATER_DESPAWN: f32 = 80.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Water {
    Lake,
    Swamp,
    River,
    /// A river through the jungle: warm, murky, with piranhas.
    JungleRiver,
    Ocean,
    WarmOcean,
    ColdOcean,
    FrozenOcean,
    /// Far out where the sea floor drops away.
    DeepOcean,
    /// A lake underground, out of the sun.
    Cave,
}

impl Water {
    /// The names a mob's `waters` may use for this water, most specific first.
    pub fn tags(self) -> &'static [&'static str] {
        match self {
            Water::Lake => &["lake"],
            Water::Swamp => &["swamp"],
            Water::River => &["river"],
            Water::JungleRiver => &["jungle_river", "river"],
            Water::Ocean => &["ocean"],
            Water::WarmOcean => &["warm_ocean", "ocean"],
            Water::ColdOcean => &["cold_ocean", "ocean"],
            Water::FrozenOcean => &["frozen_ocean", "cold_ocean", "ocean"],
            Water::DeepOcean => &["deep_ocean", "ocean"],
            Water::Cave => &["cave"],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Water::Lake => "Lago",
            Water::Swamp => "Pântano",
            Water::River => "Rio",
            Water::JungleRiver => "Rio da selva",
            Water::Ocean => "Mar",
            Water::WarmOcean => "Mar quente",
            Water::ColdOcean => "Mar frio",
            Water::FrozenOcean => "Mar congelado",
            Water::DeepOcean => "Mar profundo",
            Water::Cave => "Lago subterrâneo",
        }
    }
}

fn jungle(b: Biome) -> bool {
    matches!(b.base(), Biome::Jungle | Biome::MataAtlantica | Biome::Pantanal)
}

/// What kind of water a column is, from its biome and how deep the water is. `cave` is water
/// with rock over it.
pub fn classify(gen: &voxel::worldgen::Generator, x: i32, z: i32, depth: i32, cave: bool) -> Water {
    if cave {
        return Water::Cave;
    }
    let here = gen.biome_at(x, z);
    // Far offshore, where the sea floor drops away (the seas here are at most ~22 deep).
    let deep = depth >= 14;
    match here {
        Biome::DeepOcean => Water::DeepOcean,
        Biome::WarmOcean | Biome::LukewarmOcean => {
            if deep {
                Water::DeepOcean
            } else {
                Water::WarmOcean
            }
        }
        Biome::ColdOcean => Water::ColdOcean,
        Biome::FrozenOcean => Water::FrozenOcean,
        Biome::Ocean if deep => Water::DeepOcean,
        Biome::Ocean | Biome::Beach | Biome::StonyShore => Water::Ocean,
        Biome::SnowyBeach => Water::ColdOcean,
        Biome::River => {
            // Rivers take after the land they run through.
            let near = [(24, 0), (-24, 0), (0, 24), (0, -24)];
            if near.iter().any(|&(dx, dz)| jungle(gen.biome_at(x + dx, z + dz))) {
                Water::JungleRiver
            } else {
                Water::River
            }
        }
        Biome::FrozenRiver => Water::River,
        Biome::Swamp | Biome::MangroveSwamp | Biome::Pantanal => Water::Swamp,
        _ => Water::Lake,
    }
}

/// Where the creatures from before `waters` existed live.
pub fn default_waters(name: &str) -> &'static [&'static str] {
    match name {
        "squid" => &["ocean"],
        "dolphin" => &["ocean"],
        "axolotl" => &["cave", "swamp"],
        "tropical_fish" => &["warm_ocean"],
        "tadpole" => &["swamp", "lake"],
        "glow_squid" => &["cave", "deep_ocean"],
        "deep_watcher" => &["deep_ocean"],
        "duck" => &["lake", "river", "swamp"],
        "giant_catfish" => &["swamp", "jungle_river"],
        "orca" => &["cold_ocean", "deep_ocean"],
        "cachalot_whale" | "frilled_shark" | "giant_squid" => &["deep_ocean"],
        "bass" | "pike" => &["lake", "river"],
        _ => &[],
    }
}

/// Swims in schools (the older fish that predate the `school` flag too).
pub fn schools(def: &MobDef) -> bool {
    def.school || matches!(def.name.as_str(), "tropical_fish" | "bass")
}

fn lives_in(def: &MobDef, water: Water, biome: &str, base: &str) -> bool {
    let own: Vec<&str> = if def.waters.is_empty() { default_waters(&def.name).to_vec() } else { def.waters.iter().map(String::as_str).collect() };
    if own.is_empty() {
        // An old water creature with only biomes: anywhere in the open water of those.
        return water != Water::Cave && (def.in_biome(biome) || def.in_biome(base));
    }
    water.tags().iter().any(|t| own.contains(t))
}

/// Lives in or on the water (counts toward the water creatures, not the land animals).
pub fn is_water_creature(def: &MobDef) -> bool {
    def.spawn == SpawnRule::Water
}

fn is_water(world: &World, p: IVec3) -> bool {
    let b = world.block_or_air(p);
    b != AIR && (world.fluid(b) == 1 || world.is_wet(b))
}

impl Mobs {
    /// Spawn water creatures in the water around `player`: open water from the surface, and
    /// (now and then, or always underground) the lakes of the caves.
    pub(super) fn water_spawn(&mut self, world: &World, player: Vec3, daylight: f32) {
        if world.gen.dim != Dim::Overworld {
            return;
        }
        let (mut tame, mut wild) = (0, 0);
        for m in &self.list {
            let d = &self.reg.defs[m.def];
            if is_water_creature(d) {
                if d.kind == Kind::Hostile {
                    wild += 1;
                } else {
                    tame += 1;
                }
            }
        }
        let max_wild = ((MAX_WATER_HOSTILE as f32 * self.spawn_mult) as usize).max(1);
        let room_wild = wild < max_wild && !self.peaceful;
        if tame >= MAX_WATER && !room_wild {
            return;
        }
        let underground = player.y < SEA_LEVEL as f32 - 20.0;
        for _ in 0..3 {
            let near = TEST_NEAR.load(Ordering::Relaxed);
            let a = if near { f32::from_bits(TEST_AIM.load(Ordering::Relaxed)) + (self.rand_f32() - 0.5) * 1.1 } else { self.rand_f32() * std::f32::consts::TAU };
            let r = if near { 3.0 + self.rand_f32() * 6.5 } else { 10.0 + self.rand_f32() * 34.0 };
            let x = (player.x + a.cos() * r).floor() as i32;
            let z = (player.z + a.sin() * r).floor() as i32;
            let cave = underground || (!TEST_NEAR.load(Ordering::Relaxed) && self.rand_f32() < 0.2);
            // The top water block of the spot.
            let top = if cave {
                let start = player.y as i32 + 12;
                let mut found = None;
                for y in (start - 32..start).rev() {
                    let p = IVec3::new(x, y, z);
                    if world.block(p).is_none() {
                        break;
                    }
                    if is_water(world, p) && !is_water(world, p + IVec3::Y) {
                        found = Some(y);
                        break;
                    }
                }
                found
            } else {
                // Lily pads, duckweed and cattails float on top: the water is under them.
                world.surface_y(x, z).map(|y| if is_water(world, IVec3::new(x, y, z)) { y } else { y - 1 }).filter(|&y| is_water(world, IVec3::new(x, y, z)))
            };
            let Some(top) = top else { continue };
            // Under the ground: a cave lake, whatever we were looking for.
            let covered = top < world.gen.height_at(x, z) - 3;
            if cave && !covered && !underground {
                continue;
            }
            let mut floor = top;
            while floor > top - 48 && is_water(world, IVec3::new(x, floor - 1, z)) {
                floor -= 1;
            }
            let depth = top - floor + 1;
            let water = classify(&world.gen, x, z, depth, covered);
            let here = world.gen.biome_at(x, z);
            let (biome, base) = (here.name(), here.base().name());
            let light = world.light(IVec3::new(x, top + 1, z));
            let dark = ((light >> 4) as f32 * daylight).max((light & 15) as f32) < 7.0;
            let candidates: Vec<usize> = self
                .reg
                .defs
                .iter()
                .enumerate()
                .filter(|(_, d)| {
                    is_water_creature(d)
                        && lives_in(d, water, biome, base)
                        && match d.kind {
                            Kind::Hostile => room_wild && !self.calm && (!d.burns_in_daylight || dark),
                            _ => tame < MAX_WATER,
                        }
                        // Big creatures need deep water; surface swimmers any.
                        && (!d.aquatic || depth as f32 >= (d.height * d.scale.max(1.0)).ceil() + 1.0)
                })
                .map(|(i, _)| i)
                .collect();
            let total: u32 = candidates.iter().map(|&i| self.reg.defs[i].weight).sum();
            if total == 0 {
                continue;
            }
            let mut roll = (self.next_rand() % total as u64) as u32;
            let mut pick = candidates[0];
            for &i in &candidates {
                let w = self.reg.defs[i].weight;
                if roll < w {
                    pick = i;
                    break;
                }
                roll -= w;
            }
            let def = self.reg.defs[pick].clone();
            let room = (depth as f32 - def.height).max(0.0);
            let y = if def.flies {
                top as f32 + 1.0 + def.hover.max(0.5)
            } else if def.aquatic && !def.bottom {
                floor as f32 + 0.1 + room * (0.15 + self.rand_f32() * 0.6)
            } else if def.aquatic || def.bottom {
                floor as f32 + 0.05
            } else {
                // Floats on the surface.
                top as f32 + 0.6
            };
            let (lo, hi) = (def.group.0.max(1), def.group.1.max(def.group.0).max(1));
            let n = lo as usize + (self.next_rand() % (hi - lo + 1) as u64) as usize;
            let school = schools(&def);
            let centre = Vec3::new(x as f32 + 0.5, y, z as f32 + 0.5);
            for k in 0..n {
                let off = if k == 0 {
                    Vec3::ZERO
                } else {
                    let s = if school { 1.6 } else { 2.5 };
                    Vec3::new((self.rand_f32() - 0.5) * s * 2.0, if def.aquatic { (self.rand_f32() - 0.5) * room.min(2.0) } else { 0.0 }, (self.rand_f32() - 0.5) * s * 2.0)
                };
                let mut at = centre + off;
                // Keep everyone in the water (or over it, for flyers and floaters).
                if !def.flies && !is_water(world, at.floor().as_ivec3()) && def.aquatic {
                    at = centre;
                }
                let s = self.next_rand();
                self.push_mob(Mob::new(pick, &self.reg.defs[pick], at, s));
            }
            return;
        }
    }

    /// The MC_AGUAS test fill, once the water around the player has loaded.
    pub(super) fn test_fill(&mut self, world: &World, player: Vec3, creative: bool, daylight: f32) {
        if !TEST_FILL.load(Ordering::Relaxed) || [(0, 0), (20, 0), (-20, 0), (0, 20), (0, -20)].iter().any(|&(dx, dz)| world.block(player.as_ivec3() + IVec3::new(dx, 0, dz)).is_none()) {
            return;
        }
        TEST_FILL.store(false, Ordering::Relaxed);
        TEST_NEAR.store(true, Ordering::Relaxed);
        for _ in 0..80 {
            self.water_spawn(world, player, daylight);
        }
        TEST_NEAR.store(false, Ordering::Relaxed);
        self.spawn_cd = 1000.0;
        for _ in 0..30 {
            self.update(world, player, true, daylight, 0.05);
        }
        let mut seen: Vec<&str> = self.list.iter().map(|m| self.reg.defs[m.def].name.as_str()).collect();
        seen.sort_unstable();
        seen.dedup();
        log::info!("aguas: {} creatures: {}", self.list.len(), seen.join(", "));

    }

    /// Schools keep together: each fish swims after the first of its kind near it, and one
    /// fish taking fright sends the whole school darting off.
    pub(super) fn school_up(&mut self) {
        let fish: Vec<usize> = (0..self.list.len()).filter(|&i| self.list[i].alive() && schools(&self.reg.defs[self.list[i].def])).collect();
        if fish.len() < 2 {
            return;
        }
        let mut orders: Vec<(usize, Option<Vec3>, Option<Vec3>)> = Vec::new();
        for &i in &fish {
            let m = &self.list[i];
            let mut leader = None;
            let mut fright = None;
            for &j in &fish {
                let o = &self.list[j];
                if o.def != m.def || j == i {
                    continue;
                }
                let d = (o.body.pos - m.body.pos).length_squared();
                if d > 14.0 * 14.0 {
                    continue;
                }
                if j < i && leader.is_none() {
                    leader = Some(o.body.pos);
                }
                if fright.is_none() && d < 8.0 * 8.0 {
                    fright = o.scared();
                }
            }
            orders.push((i, leader, fright));
        }
        for (i, leader, fright) in orders {
            let m = &mut self.list[i];
            if let Some(from) = fright {
                if m.scared().is_none() {
                    m.startle(from);
                }
                continue;
            }
            if let Some(p) = leader {
                // A place of its own in the school, around and a little behind the leader.
                let a = m.phase * 2.3;
                let r = 0.9 + (m.phase * 7.1).fract() * 1.6;
                m.follow(p + Vec3::new(a.cos() * r, 0.0, a.sin() * r));
            }
        }
    }
}

impl crate::Game {
    /// Test hook: stand under the water of the nearest lake, river, sea... (MC_AGUAS=lake,
    /// river, jungle_river, swamp, ocean, warm_ocean, cold_ocean, frozen_ocean, deep_ocean,
    /// cave) and fill it with its creatures.
    pub(crate) fn test_aguas(&mut self, kind: &str) {
        let want = match kind {
            "swamp" => Water::Swamp,
            "river" => Water::River,
            "jungle_river" => Water::JungleRiver,
            "ocean" => Water::Ocean,
            "warm_ocean" => Water::WarmOcean,
            "cold_ocean" => Water::ColdOcean,
            "frozen_ocean" => Water::FrozenOcean,
            "deep_ocean" => Water::DeepOcean,
            "cave" => Water::Cave,
            _ => Water::Lake,
        };
        let gen = self.world.gen.clone();
        let me = self.body.pos.floor().as_ivec3();
        let f = self.forward();
        TEST_AIM.store(f.z.atan2(f.x).to_bits(), Ordering::Relaxed);
        let need = match want {
            Water::DeepOcean => 15,
            Water::Lake | Water::Swamp => 2,
            Water::River | Water::JungleRiver => 4,
            _ => 7,
        };
        let (reach, step) = if want == Water::Cave { (800, 32) } else { (9000, 16) };
        for r in (0..reach).step_by(step) {
            for k in 0..(r / 8).max(1) {
                let a = k as f32 / (r / 8).max(1) as f32 * std::f32::consts::TAU;
                let (x, z) = (me.x + (a.cos() * r as f32) as i32, me.z + (a.sin() * r as f32) as i32);
                if want == Water::Cave {
                    let cp = glam::IVec2::new(x.div_euclid(16), z.div_euclid(16));
                    let chunk = gen.generate(cp);
                    let (lx, lz) = (x.rem_euclid(16) as usize, z.rem_euclid(16) as usize);
                    let water = self.reg.id_or_air("water");
                    let top = gen.height_at(x, z);
                    for y in (-150..(top - 10).min(SEA_LEVEL - 12)).rev() {
                        let deep = (0..4).all(|d| chunk.get_i(lx, y - d, lz) == water);
                        if deep && chunk.get_i(lx, y + 1, lz) != water {
                            self.body.pos = Vec3::new(x as f32 + 0.5, y as f32 - 2.0, z as f32 + 0.5);
                            self.body.flying = true;
                            TEST_FILL.store(true, Ordering::Relaxed);
                            log::info!("aguas: cave lake at {x},{y},{z}");
                            return;
                        }
                    }
                    continue;
                }
                let depth = SEA_LEVEL - gen.height_at(x, z);
                if depth < need {
                    continue;
                }
                // Open water around, so the picture shows the water and not a bank.
                let open = [(6, 0), (-6, 0), (0, 6), (0, -6)].iter().all(|&(dx, dz)| {
                    let d = SEA_LEVEL - gen.height_at(x + dx, z + dz);
                    d >= need.min(4) && classify(&gen, x + dx, z + dz, d, false) == want
                });
                if !open || classify(&gen, x, z, depth, false) != want {
                    continue;
                }
                // MC_AGUAS_ABOVE: from over the water, to see the plants on it and the banks.
                let above = std::env::var("MC_AGUAS_ABOVE").is_ok();
                let y = if above { SEA_LEVEL as f32 + 3.0 } else { SEA_LEVEL as f32 - (depth as f32 * 0.5).clamp(1.2, 5.0) };
                self.body.pos = Vec3::new(x as f32 + 0.5, y, z as f32 + 0.5);
                self.body.flying = true;
                TEST_FILL.store(true, Ordering::Relaxed);
                log::info!("aguas: {kind} at {x},{y},{z} (depth {depth})");
                return;
            }
        }
        log::warn!("aguas: no {kind} found");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_reach_the_broad_kinds() {
        assert!(Water::JungleRiver.tags().contains(&"river"));
        assert!(Water::FrozenOcean.tags().contains(&"ocean"));
        assert!(Water::DeepOcean.tags().contains(&"ocean"));
        assert!(!Water::Lake.tags().contains(&"river"));
        assert!(!Water::Cave.tags().contains(&"lake"));
    }

    #[test]
    fn every_water_has_creatures() {
        let reg = super::super::registry_for_tests();
        for w in [Water::Lake, Water::Swamp, Water::River, Water::JungleRiver, Water::Ocean, Water::WarmOcean, Water::ColdOcean, Water::FrozenOcean, Water::DeepOcean, Water::Cave] {
            let n = reg.defs.iter().filter(|d| is_water_creature(d) && lives_in(d, w, "", "")).count();
            assert!(n >= 4, "{w:?} has only {n} creatures");
            let hostile = reg.defs.iter().any(|d| is_water_creature(d) && d.kind == Kind::Hostile && lives_in(d, w, "", ""));
            if !matches!(w, Water::Lake | Water::FrozenOcean) {
                assert!(hostile, "{w:?} has no monsters");
            }
        }
    }
}
