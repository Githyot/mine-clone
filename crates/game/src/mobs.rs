//! The living things in the world: spawning, simulation and picking.

use glam::{IVec3, Vec3};
use voxel::mob::{Kind, Mob, MobEvent, MobRegistry, SpawnRule, DEATH_TIME};
use voxel::{BlockId, AIR};

use crate::entity::Bones;
use crate::world::World;
use voxel::physics::Terrain;

pub mod aquatic;

const MAX_PASSIVE: usize = 14;
const MAX_HOSTILE: usize = 12;
const DESPAWN_DIST: f32 = 128.0;

pub struct Mobs {
    pub reg: MobRegistry,
    pub list: Vec<Mob>,
    spawn_cd: f32,
    /// Next time the schools of fish regroup.
    school_cd: f32,
    seed: u64,
    grass: BlockId,
    pub enabled: bool,
    /// Arrows in flight or stuck in blocks.
    pub arrows: Vec<Arrow>,
    /// Peaceful difficulty: no monsters at all.
    pub peaceful: bool,
    /// A calm night (tr_events.rs): no monsters spawn on the surface, the ones there stay.
    pub calm: bool,
    /// World difficulty (tr_modes.rs): monsters spawn with this much more health.
    pub hp_scale: f32,
    /// Candles and potions (tr_comfort.rs): more or fewer monsters at once.
    pub spawn_mult: f32,
    /// Blocks each mob may spawn on (from `MobDef::ground`), resolved on first use.
    ground: Vec<Vec<BlockId>>,
    /// Multiplayer: the other players in this dimension (where, creative), so monsters hunt
    /// them too and mobs spawn around them.
    pub others: Vec<(Vec3, bool)>,
    /// It is raining in this dimension: mobs under open sky are wet and don't burn.
    pub raining: bool,
    /// Smoothed animation for each mob (anim.rs).
    pub anim: crate::anim::Animator,
}

/// What happened to the player this tick.
#[derive(Default)]
pub struct MobOutcome {
    pub damage: Vec<(f32, Option<Vec3>)>,
    /// Arrows that struck something this tick, for the sound.
    pub arrow_hits: usize,
    /// Arrows shot this tick.
    pub shots: usize,
    pub explosions: Vec<(Vec3, f32)>,
    /// Fireballs thrown by fire imps: (from, velocity, damage).
    pub fireballs: Vec<(Vec3, Vec3, f32, u8)>,
    /// Mobs that died this tick: (def index, where).
    pub deaths: Vec<(usize, Vec3)>,
    /// Multiplayer: hits on other players (index in `Mobs::others`, damage, from where).
    pub remote_hits: Vec<(usize, f32, Vec3)>,
    /// Shots of the Nether and End creatures (ghast fireballs, wither skulls...): kind, from, at.
    pub special: Vec<(crate::nether_end::ShotKind, Vec3, Vec3)>,
}

pub const ARROW_GRAVITY: f32 = 20.0;

/// A skeleton's arrow.
#[derive(Clone, Debug)]
pub struct Arrow {
    pub pos: Vec3,
    pub vel: Vec3,
    pub age: f32,
    /// Stuck in a block (it stays a while, then goes).
    pub stuck: bool,
    pub damage: f32,
}

impl Mobs {
    /// Fly the arrows: gravity, sticking into blocks, hurting the player they reach.
    fn update_arrows(&mut self, world: &World, player: Vec3, dt: f32, out: &mut MobOutcome) {
        for a in self.arrows.iter_mut() {
            a.age += dt;
            if a.stuck {
                continue;
            }
            a.vel.y -= ARROW_GRAVITY * dt;
            // Small steps so fast arrows don't pass through things.
            let steps = ((a.vel.length() * dt) / 0.25).ceil().max(1.0) as i32;
            let step = a.vel * dt / steps as f32;
            for _ in 0..steps {
                a.pos += step;
                let rel = a.pos - player;
                if rel.x.abs() < 0.4 && rel.z.abs() < 0.4 && rel.y > -0.1 && rel.y < 1.9 {
                    out.damage.push((a.damage, Some(a.pos - a.vel.normalize_or_zero())));
                    out.arrow_hits += 1;
                    a.age = 1000.0;
                    break;
                }
                if voxel::physics::Terrain::solid(world, a.pos.floor().as_ivec3()) {
                    a.pos -= step * 0.5;
                    a.stuck = true;
                    out.arrow_hits += 1;
                    break;
                }
            }
        }
        self.arrows.retain(|a| a.age < if a.stuck { 30.0 } else { 8.0 });
    }

    pub fn new(reg: MobRegistry, grass: BlockId, seed: u64) -> Mobs {
        Mobs { reg, list: Vec::new(), spawn_cd: 2.0, school_cd: 0.0, seed: seed ^ 0x6d6f6273, grass, enabled: true, peaceful: false, calm: false, hp_scale: 1.0, spawn_mult: 1.0, arrows: Vec::new(), ground: Vec::new(), others: Vec::new(), raining: false, anim: Default::default() }
    }

    fn next_rand(&mut self) -> u64 {
        // splitmix64
        self.seed = self.seed.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn rand_f32(&mut self) -> f32 {
        (self.next_rand() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn spawn(&mut self, name: &str, pos: Vec3) -> bool {
        let Some(i) = self.reg.id(name) else { return false };
        let s = self.next_rand();
        self.push_mob(Mob::new(i, &self.reg.defs[i], pos, s));
        true
    }

    /// Add a mob; monsters get the world difficulty's extra health.
    fn push_mob(&mut self, mut m: Mob) {
        if self.hp_scale != 1.0 && self.reg.defs[m.def].kind == Kind::Hostile {
            m.health *= self.hp_scale;
            m.max_health *= self.hp_scale;
        }
        self.list.push(m);
    }

    /// Natural spawning around the player: animals on grass in the light, monsters in the dark.
    fn natural_spawn(&mut self, world: &World, player: Vec3, daylight: f32) {
        let passive = self.list.iter().filter(|m| self.reg.defs[m.def].kind != Kind::Hostile).count();
        let hostile = self.list.len() - passive;
        let max_hostile = ((MAX_HOSTILE as f32 * self.spawn_mult) as usize).max(1);
        // The Nether: everything lives in its caves, in the dark, and only its own creatures.
        // The End: always dark, on the islands' tops. Overworld creatures stay home.
        let nether = matches!(world.gen.dim, voxel::worldgen::Dim::Nether | voxel::worldgen::Dim::Otherside);
        if world.gen.dim == voxel::worldgen::Dim::Overworld && player.y < -16.0 {
            // The deep world has its own monsters, near the player's own depth.
            if hostile < max_hostile && !self.peaceful {
                self.deep_spawn(world, player);
            }
            return;
        }
        let away = world.gen.dim != voxel::worldgen::Dim::Overworld;
        for _ in 0..4 {
            let a = self.rand_f32() * std::f32::consts::TAU;
            let r = 20.0 + self.rand_f32() * 36.0;
            let x = (player.x + a.cos() * r).floor() as i32;
            let z = (player.z + a.sin() * r).floor() as i32;
            let Some(y) = world.surface_y(x, z) else { continue };
            // Monsters also try caves under the surface.
            let cave = nether || self.rand_f32() < 0.5;
            let ground = if cave { self.find_cave_floor(world, x, y, z) } else { Some(IVec3::new(x, y, z)) };
            let Some(g) = ground else { continue };
            let below = world.block_or_air(g);
            let water = world.fluid(below) == 1;
            if below == AIR || (world.is_liquid(below) && !water) {
                continue;
            }
            if world.block_or_air(g + IVec3::Y) != AIR || world.block_or_air(g + IVec3::Y * 2) != AIR {
                continue;
            }
            let light = world.light(g + IVec3::Y);
            let sky = (light >> 4) as f32 * daylight;
            let blk = (light & 15) as f32;
            let dark = away || sky.max(blk) < 7.0;
            let here = world.gen.biome_at(x, z);
            let (biome, base) = (here.name(), here.base().name());
            if self.ground.len() != self.reg.defs.len() {
                let grass = self.grass;
                self.ground = self
                    .reg
                    .defs
                    .iter()
                    .map(|d| if d.ground.is_empty() { vec![grass] } else { d.ground.iter().map(|n| world.reg.id_or_air(n)).filter(|&b| b != AIR).collect() })
                    .collect();
            }
            let animals = if away { passive < MAX_PASSIVE } else { !cave && !dark && passive < MAX_PASSIVE };
            let candidates: Vec<usize> = self
                .reg
                .defs
                .iter()
                .enumerate()
                .filter(|(i, d)| {
                    (d.in_biome(biome) || d.in_biome(base) || (!away && d.biomes.iter().any(|b| b == "Overworld")))
                        && (!away || !d.biomes.is_empty())
                        && match d.spawn {
                            SpawnRule::Grass => animals && !water && self.ground[*i].contains(&below),
                            // In the Overworld the water creatures have their own spawning
                            // (aquatic.rs); elsewhere they go by biome.
                            SpawnRule::Water => animals && water && away,
                            SpawnRule::Dark => dark && !water && hostile < max_hostile && !self.peaceful && !self.calm && (self.reg.defs[*i].ground.is_empty() || self.ground[*i].contains(&below)),
                            SpawnRule::Never => false,
                        }
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
            let pos = Vec3::new(g.x as f32 + 0.5, g.y as f32 + 1.0, g.z as f32 + 0.5);
            // Animals come in small groups.
            let def = &self.reg.defs[pick];
            let n = if def.kind != Kind::Hostile {
                let (lo, hi) = (def.group.0.max(1), def.group.1.max(def.group.0).max(1));
                lo as usize + (self.next_rand() % (hi - lo + 1) as u64) as usize
            } else {
                1
            };
            for k in 0..n {
                let off = Vec3::new((k % 2) as f32 * 1.2 - 0.6, 0.0, (k / 2) as f32 * 1.2);
                let s = self.next_rand();
                self.push_mob(Mob::new(pick, &self.reg.defs[pick], pos + off, s));
            }
            return;
        }
    }

    /// Monsters of the deep layers (worldgen/deep.rs): a cave floor in the dark within a few
    /// blocks of the player's height, and a monster that fits the layer and underground biome.
    fn deep_spawn(&mut self, world: &World, player: Vec3) {
        use voxel::worldgen::deep::{Layer, UBiome};
        for _ in 0..4 {
            let a = self.rand_f32() * std::f32::consts::TAU;
            let r = 14.0 + self.rand_f32() * 26.0;
            let x = (player.x + a.cos() * r).floor() as i32;
            let z = (player.z + a.sin() * r).floor() as i32;
            let start = player.y as i32 + 16 - (self.next_rand() % 8) as i32;
            let mut g = None;
            for y in (start - 32..start).rev() {
                let p = IVec3::new(x, y, z);
                if world.block(p - IVec3::Y).is_none() {
                    break;
                }
                if world.block_or_air(p) == AIR && world.block_or_air(p + IVec3::Y) == AIR && world.solid(p - IVec3::Y) {
                    g = Some(p);
                    break;
                }
            }
            let Some(p) = g else { continue };
            if world.light(p) & 15 >= 7 && self.rand_f32() < 0.8 {
                continue;
            }
            let uw = Layer::at(p.y) == Layer::Underworld;
            let names: &[(&str, u32)] = if uw {
                &[("fire_imp", 6), ("magma_cube", 5), ("wither_skeleton", 3), ("blaze", 3), ("ember_wraith", 3), ("ash_skeleton", 4), ("jelly_lava", 4)]
            } else {
                match world.gen.ubiome_at(p.x, p.y, p.z) {
                    UBiome::Ice => &[("frost_archer", 5), ("skeleton", 4), ("zombie", 2), ("jelly_ice", 3)],
                    UBiome::Desert => &[("mummy", 6), ("spider", 3), ("skeleton", 2)],
                    UBiome::Jungle => &[("ooze", 5), ("cave_spider", 3), ("zombie", 3), ("hive_drone", 2)],
                    UBiome::Spider => &[("cave_spider", 7), ("spider", 5)],
                    UBiome::Mushroom => &[("ooze", 3), ("zombie", 2)],
                    _ if Layer::at(p.y) == Layer::Caverns => &[("skeleton", 5), ("zombie", 4), ("stalker", 3), ("spider", 3), ("wither_skeleton", 1), ("ooze", 2), ("jelly_red", 3), ("jelly_pink", 1)],
                    _ => &[("zombie", 5), ("skeleton", 5), ("spider", 3), ("stalker", 2), ("cave_spider", 1), ("jelly_blue", 2), ("jelly_red", 1)],
                }
            };
            let total: u32 = names.iter().map(|n| n.1).sum();
            let mut roll = (self.next_rand() % total as u64) as u32;
            let mut pick = names[0].0;
            for &(n, w) in names {
                if roll < w {
                    pick = n;
                    break;
                }
                roll -= w;
            }
            self.spawn(pick, Vec3::new(p.x as f32 + 0.5, p.y as f32, p.z as f32 + 0.5));
            return;
        }
    }

    fn find_cave_floor(&mut self, world: &World, x: i32, top: i32, z: i32) -> Option<IVec3> {
        let start = 8 + (self.next_rand() % (top.max(9) - 8) as u64) as i32;
        let mut y = start;
        while y > 2 {
            let p = IVec3::new(x, y, z);
            if world.block_or_air(p) == AIR && world.block_or_air(p - IVec3::Y) != AIR && world.block(p - IVec3::Y).is_some() {
                return Some(p - IVec3::Y);
            }
            y -= 1;
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(&mut self, world: &World, player: Vec3, creative: bool, daylight: f32, dt: f32) -> MobOutcome {
        let mut out = MobOutcome::default();
        if !self.enabled {
            self.list.clear();
            return out;
        }
        self.test_fill(world, player, creative, daylight);
        if self.peaceful {
            let reg = &self.reg;
            self.list.retain(|m| reg.defs[m.def].kind != Kind::Hostile);
        }
        self.spawn_cd -= dt;
        if self.spawn_cd <= 0.0 {
            self.spawn_cd = 1.0 / self.spawn_mult.clamp(0.25, 4.0);
            self.natural_spawn(world, player, daylight);
            self.water_spawn(world, player, daylight);
            for k in 0..self.others.len() {
                let p = self.others[k].0;
                self.natural_spawn(world, p, daylight);
                self.water_spawn(world, p, daylight);
            }
        }
        self.school_cd -= dt;
        if self.school_cd <= 0.0 {
            self.school_cd = 0.3;
            self.school_up();
        }
        let mut events = Vec::new();
        let day = daylight > 0.5;
        // Hungry hunters pick the closest prey in sight.
        let mut target: Vec<Option<usize>> = vec![None; self.list.len()];
        for (i, m) in self.list.iter().enumerate() {
            let def = &self.reg.defs[m.def];
            if !m.hungry(def) {
                continue;
            }
            target[i] = self
                .list
                .iter()
                .enumerate()
                .filter(|(_, o)| o.alive() && !o.ghost && def.prey.iter().any(|p| *p == self.reg.defs[o.def].name))
                .map(|(j, o)| (j, (o.body.pos - m.body.pos).length()))
                .filter(|&(_, d)| d < 12.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(j, _)| j);
        }
        let prey_pos: Vec<Vec3> = self.list.iter().map(|m| m.body.pos).collect();
        let mut bites: Vec<(usize, usize, f32)> = Vec::new();
        for (i, m) in self.list.iter_mut().enumerate() {
            m.prey = target[i].map(|j| prey_pos[j]);
            // A mount under the player is moved by the player.
            if m.ridden {
                continue;
            }
            let def = &self.reg.defs[m.def];
            // Far away and unloaded: freeze until the player comes back.
            if world.block(m.body.pos.floor().as_ivec3()).is_none() {
                continue;
            }
            let head = (m.body.pos + Vec3::Y * def.height).floor().as_ivec3();
            let sky = world.light(head) >> 4;
            events.clear();
            // The player it goes after: the closest one that can be hurt (else the closest).
            let mut who: Option<usize> = None;
            let (mut tp, mut tc) = (player, creative);
            let score = |p: Vec3, c: bool| (p - m.body.pos).length() + if c { 1000.0 } else { 0.0 };
            for (k, &(p, c)) in self.others.iter().enumerate() {
                if score(p, c) < score(tp, tc) {
                    (tp, tc, who) = (p, c, Some(k));
                }
            }
            m.rained_on = self.raining && sky >= 14;
            m.tick(def, world, tp, tc, sky, day, dt, &mut events);
            for e in &events {
                match *e {
                    MobEvent::Hit { damage, from } if who.is_some() => out.remote_hits.push((who.unwrap(), damage * m.power, from)),
                    MobEvent::Shoot { damage, from, hit, .. } if who.is_some() => {
                        if hit {
                            out.remote_hits.push((who.unwrap(), damage * m.power, from));
                        }
                    }
                    MobEvent::Hit { damage, from } => out.damage.push((damage * m.power, Some(from))),
                    MobEvent::Shoot { damage, from, at, hit } => {
                        let damage = damage * m.power;
                        // A real arrow: aimed at the player's middle (a little off on a miss),
                        // lobbed so gravity brings it there.
                        let mut target = at + Vec3::Y * 0.9;
                        if !hit {
                            let a = (m.body.pos.x * 13.1 + m.body.pos.z * 7.7).sin() * 3.0;
                            target += Vec3::new(a.cos(), 0.4, a.sin()) * 1.6;
                        }
                        let d = target - from;
                        if let Some(kind) = crate::nether_end::shot_kind(&def.name) {
                            out.special.push((kind, from, target));
                            continue;
                        }
                        if let Some(school) = caster_school(&def.name) {
                            // Casters throw a straight bolt of their magic instead of an arrow.
                            out.fireballs.push((from, d.normalize_or_zero() * 14.0, damage, school));
                            continue;
                        }
                        let flat = Vec3::new(d.x, 0.0, d.z).length();
                        let speed = 18.0;
                        let t = (flat / speed).max(0.05);
                        let vel = Vec3::new(d.x / t, d.y / t + 0.5 * ARROW_GRAVITY * t, d.z / t);
                        self.arrows.push(Arrow { pos: from, vel, age: 0.0, stuck: false, damage });
                        out.shots += 1;
                    }
                    MobEvent::Explode { pos, power } => out.explosions.push((pos, power)),
                    MobEvent::Bite { damage } => {
                        if let Some(j) = target[i] {
                            bites.push((i, j, damage * m.power));
                        }
                    }
                    MobEvent::Burning | MobEvent::Died { .. } => {}
                }
            }
            // Every death counts once, however it came (a hit from the player or a spell lands
            // between ticks, fire and falls inside one).
            if !m.alive() && !m.counted {
                m.counted = true;
                out.deaths.push((m.def, m.body.pos));
            }
        }
        for (i, j, damage) in bites {
            let from = self.list[i].body.center();
            if self.list[j].hurt(damage, from) && !self.list[j].counted {
                self.list[j].counted = true;
                out.deaths.push((self.list[j].def, self.list[j].body.pos));
                // A full hunter naps rather than hunting again soon; a town guard keeps watch.
                self.list[i].fed = if self.reg.defs[self.list[i].def].town { 1.0 } else { 90.0 + (self.next_rand() % 60) as f32 };
            }
        }
        self.update_arrows(world, player, dt, &mut out);
        let reg = &self.reg;
        let others = &self.others;
        self.list.retain(|m| {
            let gone = !m.alive() && m.dead_age > DEATH_TIME;
            let keep = m.name.is_some() || m.leashed;
            let reach = if keep {
                DESPAWN_DIST + 48.0
            } else if aquatic::is_water_creature(&reg.defs[m.def]) {
                aquatic::WATER_DESPAWN
            } else {
                DESPAWN_DIST
            };
            let far = (m.body.pos - player).length() > reach && !reg.defs[m.def].town && others.iter().all(|(p, _)| (m.body.pos - *p).length() > reach);
            let fell = m.body.pos.y < voxel::chunk::MIN_Y as f32 - 64.0;
            !(gone || far || fell)
        });
        out
    }

    /// The closest living mob the ray hits within `max` blocks.
    pub fn pick(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (i, m) in self.list.iter().enumerate() {
            if !m.alive() || m.ghost {
                continue;
            }
            let (lo, hi) = m.body.aabb();
            if let Some(t) = ray_box(origin, dir, lo, hi) {
                if t <= max && best.is_none_or(|(_, bt)| t < bt) {
                    best = Some((i, t));
                }
            }
        }
        best
    }

    /// Move every mob's animation on by a frame (render time, not simulation time).
    pub fn animate(&mut self, player: Vec3, dt: f32) {
        self.anim.update(&self.reg.defs, &self.list, player, dt);
    }

    /// Bone matrices for every mob within draw distance.
    pub fn instances(&mut self, world: &World, cam_block: IVec3, max_dist: f32) -> Vec<(usize, Bones)> {
        let c = cam_block.as_vec3();
        let anim = &mut self.anim;
        self.list
            .iter()
            .filter(|m| (m.body.pos - c).length() < max_dist)
            .map(|m| {
                let def = &self.reg.defs[m.def];
                let l = world.light((m.body.pos + Vec3::Y * (def.height * 0.6)).floor().as_ivec3());
                (m.def, anim.bones(m, def, m.def, cam_block, (l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0))
            })
            .collect()
    }
}

pub(crate) fn ray_box(o: Vec3, d: Vec3, lo: Vec3, hi: Vec3) -> Option<f32> {
    let inv = Vec3::ONE / d;
    let t1 = (lo - o) * inv;
    let t2 = (hi - o) * inv;
    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();
    if tmax >= tmin.max(0.0) {
        Some(tmin.max(0.0))
    } else {
        None
    }
}

/// Ranged mobs that throw magic from their hands instead of shooting a bow, with its school
/// (0 fire, 1 frost, 3 void, 4 nature).
pub fn caster_school(name: &str) -> Option<u8> {
    match name {
        "fire_imp" => Some(0),
        "swamp_witch" => Some(4),
        "invoker" => Some(3),
        "ember_wraith" => Some(0),
        // The wizards of the spellbooks mod (mods/spellbooks.rs casts their bigger spells).
        "pyromancer" => Some(0),
        "cryomancer" => Some(1),
        "necromancer" => Some(3),
        _ => None,
    }
}

/// Every creature of the game, as the game loads them (for tests elsewhere).
#[cfg(test)]
pub fn registry_for_tests() -> voxel::mob::MobRegistry {
    use crate::embedded;
    let text = crate::terraria::with_extra(&crate::merge_list(embedded::MOBS_RON, embedded::FASE6_MOBS_RON), embedded::T5_MOBS_RON);
    let text = crate::terraria::with_extra(&crate::terraria::with_extra(&text, embedded::TR_MOBS_RON), embedded::AGUAS_MOBS_RON);
    voxel::mob::MobRegistry::from_ron_many(&[&text, embedded::MODS_MOBS_RON, embedded::MODS_DIMS_MOBS_RON]).unwrap()
}
