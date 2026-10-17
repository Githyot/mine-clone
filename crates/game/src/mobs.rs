//! The living things in the world: spawning, simulation and picking.

use glam::{IVec3, Vec3};
use voxel::mob::{Kind, Mob, MobEvent, MobRegistry, SpawnRule, DEATH_TIME};
use voxel::{BlockId, AIR};

use crate::entity::{bones_for, Bones};
use crate::world::World;

const MAX_PASSIVE: usize = 14;
const MAX_HOSTILE: usize = 12;
const DESPAWN_DIST: f32 = 128.0;

pub struct Mobs {
    pub reg: MobRegistry,
    pub list: Vec<Mob>,
    spawn_cd: f32,
    seed: u64,
    grass: BlockId,
    pub enabled: bool,
    /// Arrows in flight or stuck in blocks.
    pub arrows: Vec<Arrow>,
    /// Peaceful difficulty: no monsters at all.
    pub peaceful: bool,
    /// Blocks each mob may spawn on (from `MobDef::ground`), resolved on first use.
    ground: Vec<Vec<BlockId>>,
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
        Mobs { reg, list: Vec::new(), spawn_cd: 2.0, seed: seed ^ 0x6d6f6273, grass, enabled: true, peaceful: false, arrows: Vec::new(), ground: Vec::new() }
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
        self.list.push(Mob::new(i, &self.reg.defs[i], pos, s));
        true
    }

    /// Natural spawning around the player: animals on grass in the light, monsters in the dark.
    fn natural_spawn(&mut self, world: &World, player: Vec3, daylight: f32) {
        let passive = self.list.iter().filter(|m| self.reg.defs[m.def].kind != Kind::Hostile).count();
        let hostile = self.list.len() - passive;
        // The Nether: everything lives in its caves, in the dark, and only its own creatures.
        // The End: always dark, on the islands' tops. Overworld creatures stay home.
        let nether = world.gen.dim == voxel::worldgen::Dim::Nether;
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
            let biome = world.gen.biome_at(x, z).name();
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
                    (d.in_biome(biome) || (!away && d.biomes.iter().any(|b| b == "Overworld")))
                        && (!away || !d.biomes.is_empty())
                        && match d.spawn {
                            SpawnRule::Grass => animals && !water && self.ground[*i].contains(&below),
                            SpawnRule::Water => animals && water,
                            SpawnRule::Dark => dark && !water && hostile < MAX_HOSTILE && !self.peaceful,
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
                self.list.push(Mob::new(pick, &self.reg.defs[pick], pos + off, s));
            }
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
        if self.peaceful {
            let reg = &self.reg;
            self.list.retain(|m| reg.defs[m.def].kind != Kind::Hostile);
        }
        self.spawn_cd -= dt;
        if self.spawn_cd <= 0.0 {
            self.spawn_cd = 1.0;
            self.natural_spawn(world, player, daylight);
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
                .filter(|(_, o)| o.alive() && def.prey.iter().any(|p| *p == self.reg.defs[o.def].name))
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
            let was_alive = m.alive();
            events.clear();
            m.tick(def, world, player, creative, sky, day, dt, &mut events);
            for e in &events {
                match *e {
                    MobEvent::Hit { damage, from } => out.damage.push((damage, Some(from))),
                    MobEvent::Shoot { damage, from, at, hit } => {
                        // A real arrow: aimed at the player's middle (a little off on a miss),
                        // lobbed so gravity brings it there.
                        let mut target = at + Vec3::Y * 0.9;
                        if !hit {
                            let a = (m.body.pos.x * 13.1 + m.body.pos.z * 7.7).sin() * 3.0;
                            target += Vec3::new(a.cos(), 0.4, a.sin()) * 1.6;
                        }
                        let d = target - from;
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
                            bites.push((i, j, damage));
                        }
                    }
                    MobEvent::Burning | MobEvent::Died { .. } => {}
                }
            }
            if was_alive && !m.alive() {
                out.deaths.push((m.def, m.body.pos));
            }
        }
        for (i, j, damage) in bites {
            let from = self.list[i].body.center();
            if self.list[j].hurt(damage, from) {
                out.deaths.push((self.list[j].def, self.list[j].body.pos));
                // A full hunter naps rather than hunting again soon; a town guard keeps watch.
                self.list[i].fed = if self.reg.defs[self.list[i].def].town { 1.0 } else { 90.0 + (self.next_rand() % 60) as f32 };
            }
        }
        self.update_arrows(world, player, dt, &mut out);
        let reg = &self.reg;
        self.list.retain(|m| {
            let gone = !m.alive() && m.dead_age > DEATH_TIME;
            let keep = m.name.is_some() || m.leashed;
            let far = (m.body.pos - player).length() > if keep { DESPAWN_DIST + 48.0 } else { DESPAWN_DIST } && !reg.defs[m.def].town;
            let fell = m.body.pos.y < -64.0;
            !(gone || far || fell)
        });
        out
    }

    /// The closest living mob the ray hits within `max` blocks.
    pub fn pick(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (i, m) in self.list.iter().enumerate() {
            if !m.alive() {
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

    /// Bone matrices for every mob within draw distance.
    pub fn instances(&self, world: &World, cam_block: IVec3, max_dist: f32) -> Vec<(usize, Bones)> {
        let c = cam_block.as_vec3();
        self.list
            .iter()
            .filter(|m| (m.body.pos - c).length() < max_dist)
            .map(|m| {
                let def = &self.reg.defs[m.def];
                let l = world.light((m.body.pos + Vec3::Y * (def.height * 0.6)).floor().as_ivec3());
                (m.def, bones_for(m, def, cam_block, (l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0))
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
        _ => None,
    }
}
