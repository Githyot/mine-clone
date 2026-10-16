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
        Mobs { reg, list: Vec::new(), spawn_cd: 2.0, seed: seed ^ 0x6d6f6273, grass, enabled: true, peaceful: false, arrows: Vec::new() }
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
        for _ in 0..4 {
            let a = self.rand_f32() * std::f32::consts::TAU;
            let r = 20.0 + self.rand_f32() * 36.0;
            let x = (player.x + a.cos() * r).floor() as i32;
            let z = (player.z + a.sin() * r).floor() as i32;
            let Some(y) = world.surface_y(x, z) else { continue };
            // Monsters also try caves under the surface.
            let cave = self.rand_f32() < 0.5;
            let ground = if cave { self.find_cave_floor(world, x, y, z) } else { Some(IVec3::new(x, y, z)) };
            let Some(g) = ground else { continue };
            let below = world.block_or_air(g);
            if below == AIR || world.is_liquid(below) {
                continue;
            }
            if world.block_or_air(g + IVec3::Y) != AIR || world.block_or_air(g + IVec3::Y * 2) != AIR {
                continue;
            }
            let light = world.light(g + IVec3::Y);
            let sky = (light >> 4) as f32 * daylight;
            let blk = (light & 15) as f32;
            let dark = sky.max(blk) < 7.0;
            let candidates: Vec<usize> = self
                .reg
                .defs
                .iter()
                .enumerate()
                .filter(|(_, d)| match d.spawn {
                    SpawnRule::Grass => !cave && below == self.grass && !dark && passive < MAX_PASSIVE,
                    SpawnRule::Dark => dark && hostile < MAX_HOSTILE && !self.peaceful,
                    SpawnRule::Never => false,
                })
                .map(|(i, _)| i)
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let pick = candidates[(self.next_rand() % candidates.len() as u64) as usize];
            let pos = Vec3::new(g.x as f32 + 0.5, g.y as f32 + 1.0, g.z as f32 + 0.5);
            // Animals come in small groups.
            let n = if self.reg.defs[pick].kind == Kind::Passive { 2 + (self.next_rand() % 3) as usize } else { 1 };
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
        for m in self.list.iter_mut() {
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
                        let flat = Vec3::new(d.x, 0.0, d.z).length();
                        let speed = 18.0;
                        let t = (flat / speed).max(0.05);
                        let vel = Vec3::new(d.x / t, d.y / t + 0.5 * ARROW_GRAVITY * t, d.z / t);
                        self.arrows.push(Arrow { pos: from, vel, age: 0.0, stuck: false, damage });
                        out.shots += 1;
                    }
                    MobEvent::Explode { pos, power } => out.explosions.push((pos, power)),
                    MobEvent::Burning | MobEvent::Died { .. } => {}
                }
            }
            if was_alive && !m.alive() {
                out.deaths.push((m.def, m.body.pos));
            }
        }
        self.update_arrows(world, player, dt, &mut out);
        self.list.retain(|m| {
            let gone = !m.alive() && m.dead_age > DEATH_TIME;
            let far = (m.body.pos - player).length() > DESPAWN_DIST;
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
