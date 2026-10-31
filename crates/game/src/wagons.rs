//! Wagons and carts: real ones with spoked wheels that turn, pulled by a horse, a donkey, a
//! pair of horses or a pair of oxen under a yoke, with a driver on the seat. They travel the
//! main roads through villages (coming in at one end and leaving at the other) and cross open
//! country, and parked ones stand in the traders' camps, on the village squares and broken by
//! the roadside. They are ambience: not saved, made again from the world around the player.
//!
//! A wagon is drawn from several models (tools/gen_wagons.py): the bed, its load (a canvas
//! cover or hay), the front assembly (axle and shafts or a pole) that turns with the team like
//! a real four-wheeled wagon, and four wheels. The rear follows the front like a trailer.

use std::collections::{HashMap, HashSet};

use glam::{IVec3, Mat4, Quat, Vec3};
use voxel::block::BlockId;
use voxel::mob::{Anim, Mob};
use voxel::physics::Terrain;
use voxel::worldgen::{Biome, Dim};

use crate::audio::Sfx;
use crate::entity::{Bones, MOUTH_PART};
use crate::money::s;
use crate::sfx_extra::Call;
use crate::Game;

/// Half the distance between the axles (blocks).
const HALF_BASE: f32 = 0.6875;
/// Where the front axle turns, ahead of the bed's middle.
const KINGPIN: f32 = 0.625;
const WHEEL_R: f32 = 0.47;
const WHEEL_X: f32 = 0.84;
/// The models are drawn this much bigger than they are made (HP: the covered wagon 4 blocks
/// high and 2 wide); the distances above are in model blocks, times SIZE in the world.
const SIZE: f32 = 1.1;
/// Most the front may turn against the bed.
/// How far the bare end of a broken wagon's front axle sinks: down from the hub height to
/// just over the ground.
const BROKEN_DROP: f32 = WHEEL_R - 0.12;
/// The canvas sits on top of the side boards, clear of the wheels, never through them.
const COVER_LIFT: f32 = 0.115;

/// A wheel takes a step up or down over this much ground, front axle first, then the rear.
const RAMP: f32 = 1.5;
/// It starts this far before the edge, so the wheel clears the corner.
const LEAD: f32 = 0.3;
const RAMP_N: usize = 25;

/// Offsets along the way where `wheel_rise` wants the ground: under the wheel, then a ramp's
/// worth ahead, then behind.
fn ramp_d(n: usize) -> f32 {
    let step = RAMP / (RAMP_N - 1) as f32;
    match n {
        0..3 => (n as f32 - 1.0) * 0.25,
        _ if n < 3 + RAMP_N => LEAD + (n - 3) as f32 * step,
        _ => -LEAD - (n - 3 - RAMP_N) as f32 * step,
    }
}
const SAMPLES: usize = 3 + 2 * RAMP_N;

/// Where a wheel sits over ground sampled at the `ramp_d` offsets: a weighted look ahead
/// (climbing) or behind (going down), whichever is higher, so it rises smoothly and clears
/// the edge it passes, and never lower than the ground right under it.
fn wheel_rise(g: &[f32; SAMPLES]) -> f32 {
    let (mut ahead, mut behind, mut wsum) = (0.0, 0.0, 0.0);
    for k in 0..RAMP_N {
        let w = 1.0 - k as f32 / RAMP_N as f32;
        ahead += g[3 + k] * w;
        behind += g[3 + RAMP_N + k] * w;
        wsum += w;
    }
    (ahead / wsum).max(behind / wsum).max(g[0]).max(g[1]).max(g[2])
}
const MAX_ARTIC: f32 = 0.6;
/// Wagons further than this from the player are taken away (blocks).
const FAR: f32 = 150.0;
const MAX_MOVING: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Load {
    Covered,
    Hay,
    Empty,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    Horse,
    Donkey,
    Horses,
    Oxen,
    /// Unhitched: the shafts rest on the ground.
    None,
}

impl Team {
    fn animal(self) -> &'static str {
        match self {
            Team::Horse | Team::Horses => "horse",
            Team::Donkey => "donkey",
            Team::Oxen => "ox",
            Team::None => "",
        }
    }

    /// Where each animal walks, from the front axle: (side, ahead).
    fn places(self) -> &'static [(f32, f32)] {
        match self {
            Team::Horse => &[(0.0, 1.45)],
            Team::Donkey => &[(0.0, 1.35)],
            Team::Horses => &[(-0.5, 1.7), (0.5, 1.7)],
            Team::Oxen => &[(-0.56, 1.8), (0.56, 1.8)],
            Team::None => &[],
        }
    }

    fn pair(self) -> bool {
        matches!(self, Team::Horses | Team::Oxen)
    }

    /// How fast it walks along (blocks a second).
    fn cruise(self) -> f32 {
        match self {
            Team::Horse => 1.9,
            Team::Horses => 2.1,
            Team::Donkey => 1.5,
            Team::Oxen => 1.15,
            Team::None => 0.0,
        }
    }

    /// Tilt of the shafts or pole up to the animals.
    fn lift(self) -> f32 {
        match self {
            Team::Horse | Team::Donkey => 0.24,
            Team::Horses => 0.3,
            Team::Oxen => 0.35,
            // Resting on the ground.
            Team::None => -0.2,
        }
    }
}

pub struct Wagon {
    /// Middle of the bed, on the ground between the axles.
    pub pos: Vec3,
    pub yaw: f32,
    /// The front axle and the team (they turn first, the bed follows).
    front_yaw: f32,
    pitch: f32,
    roll: f32,
    /// Side to side lean from the ground under the wheels.
    tilt: f32,
    load: Load,
    team: Team,
    animals: Vec<Mob>,
    driver: Option<Mob>,
    wheel_front: f32,
    wheel_back: f32,
    speed: f32,
    /// Where it is going, in order; when they run out it carries straight on.
    route: Vec<Vec3>,
    parked: bool,
    broken: bool,
    /// The structure prop it stands for (parked ones), or the village it drives through.
    key: Option<(IVec3, usize)>,
    village: Option<IVec3>,
    stuck: f32,
    steer_t: f32,
    want_yaw: f32,
    creak_t: f32,
    crunch_t: f32,
    call_t: f32,
    age: f32,
    /// Test scenes: walks and rolls on the spot, so a screenshot catches it in motion.
    treadmill: bool,
    /// Pulled up to trade with the player (seconds left), and the driver's words (seconds
    /// left of the mouth moving).
    halt_t: f32,
    talk_t: f32,
    /// Picks the travelling merchant's goods from the road.
    seed: u32,
}

#[derive(Default)]
pub struct Wagons {
    pub list: Vec<Wagon>,
    check_t: f32,
    country_t: f32,
    /// When a wagon last came through each village (by its corner).
    village_t: HashMap<IVec3, f32>,
    clock: f32,
    road: Option<HashSet<BlockId>>,
    /// Never ground for a wheel (leaves), and never a way on (trunks, fences, walls, cactus).
    leaves: HashSet<BlockId>,
    rough: HashSet<BlockId>,
    test_done: bool,
    /// Test hook MC_WAGON_TALK: click the first driver once the scene is up.
    talk_test: bool,
}

/// Models the wagons are drawn with, looked up once.
struct Defs {
    body: usize,
    cover: usize,
    cargo: usize,
    hay: usize,
    shafts: usize,
    pole: usize,
    wheel: usize,
    harness: Option<usize>,
}

fn fwd(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

fn wrap(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    (a + std::f32::consts::PI).rem_euclid(t) - std::f32::consts::PI
}

/// Biomes travellers cross.
fn open_country(b: Biome) -> bool {
    matches!(
        b.base(),
        Biome::Plains | Biome::Forest | Biome::BirchForest | Biome::Savanna | Biome::Meadow | Biome::Desert | Biome::Taiga | Biome::CherryGrove
    )
}

impl Game {
    fn wagon_defs(&self) -> Option<Defs> {
        let id = |n: &str| self.mobs.reg.id(n);
        Some(Defs {
            body: id("wagon")?,
            cover: id("wagon_cover")?,
            cargo: id("wagon_cargo")?,
            hay: id("wagon_hay")?,
            shafts: id("wagon_shafts")?,
            pole: id("wagon_pole")?,
            wheel: id("wagon_wheel")?,
            harness: id("harness"),
        })
    }

    /// The top of the ground in a column near height `near`: (height, no way on (water, or a
    /// tree trunk, fence or wall top), road under it). Leaves are never ground: a wheel never
    /// rides up onto a bush or a canopy hanging over the way. None where the chunk is not
    /// loaded.
    fn wagon_ground(&self, x: f32, z: f32, near: f32) -> Option<(f32, bool, bool)> {
        let (bx, bz) = (x.floor() as i32, z.floor() as i32);
        let y0 = near.floor() as i32;
        let ground = |p: IVec3| self.world.solid(p) && !self.world.block(p).is_some_and(|id| self.wagons.leaves.contains(&id));
        for y in (y0 - 5..=y0 + 2).rev() {
            let p = IVec3::new(bx, y, bz);
            let id = self.world.block(p)?;
            if ground(p) && !ground(p + IVec3::Y) {
                let wet = self.world.liquid(p + IVec3::Y) || self.wagons.rough.contains(&id);
                let road = self.wagons.road.as_ref().is_some_and(|r| r.contains(&id));
                return Some((y as f32 + self.world.top(p), wet, road));
            }
            if self.world.liquid(p) {
                return Some((y as f32 + 1.0, true, false));
            }
        }
        None
    }

    /// Height of an axle over the ground and its tilt across: each wheel eases up and over a
    /// block edge along a smooth ramp instead of jumping a whole block at once. None where the
    /// chunk is not loaded.
    fn axle_ground(&self, at: Vec3, yaw: f32, near: f32) -> Option<(f32, f32)> {
        let f = fwd(yaw);
        let side = Vec3::new(f.z, 0.0, -f.x);
        let mut h = [0.0; 2];
        let mut g = [0.0; SAMPLES];
        for (i, s) in [-WHEEL_X * SIZE, WHEEL_X * SIZE].into_iter().enumerate() {
            for (n, y) in g.iter_mut().enumerate() {
                let p = at + side * s + f * ramp_d(n);
                *y = self.wagon_ground(p.x, p.z, near)?.0;
            }
            h[i] = wheel_rise(&g);
        }
        let tilt = ((h[1] - h[0]) / (2.0 * WHEEL_X * SIZE)).atan().clamp(-0.25, 0.25);
        Some(((h[0] + h[1]) * 0.5, tilt))
    }

    /// Can the wagon and its team pass this way? Some(road bonus) or None when blocked.
    fn wagon_way(&self, w: &Wagon, yaw: f32) -> Option<f32> {
        let f = fwd(yaw);
        let side = Vec3::new(f.z, 0.0, -f.x);
        let k = w.pos + fwd(w.yaw) * (KINGPIN * SIZE);
        let mut last = w.pos.y;
        let mut bonus = 0.0;
        for (n, d) in [1.5f32, 2.5, 3.5, 4.5, 5.5].iter().enumerate() {
            let mut top = f32::MIN;
            for s in [-0.8f32, 0.0, 0.8] {
                let p = k + f * *d + side * s;
                let (y, wet, road) = self.wagon_ground(p.x, p.z, last + 0.5)?;
                if wet {
                    return None;
                }
                // Room for the team and the cover above the ground.
                let c = IVec3::new(p.x.floor() as i32, y.ceil() as i32, p.z.floor() as i32);
                if self.world.solid(c) || self.world.solid(c + IVec3::Y) {
                    return None;
                }
                // The tall cover needs room up to 4 blocks over the middle.
                if w.load == Load::Covered && s == 0.0 && (self.world.solid(c + IVec3::Y * 2) || self.world.solid(c + IVec3::Y * 3)) {
                    return None;
                }
                if road {
                    bonus += 0.25 / (1.0 + n as f32 * 0.5);
                }
                top = top.max(y);
            }
            if (top - last).abs() > 1.05 {
                return None;
            }
            last = top;
        }
        Some(bonus)
    }

    /// Is the ground just ahead of the front wheels and under the team fit to go on: no step
    /// of more than one block, no water, no trunk or fence top?
    fn wagon_may_step(&self, w: &Wagon) -> bool {
        let f = fwd(w.front_yaw);
        let side = Vec3::new(f.z, 0.0, -f.x);
        let k = w.pos + fwd(w.yaw) * (KINGPIN * SIZE);
        let near = w.pos.y + 1.0;
        for s in [-WHEEL_X * SIZE, 0.0, WHEEL_X * SIZE] {
            let base = k + side * s + f * ((HALF_BASE - KINGPIN) * SIZE);
            let Some((mut last, _, _)) = self.wagon_ground(base.x, base.z, near) else { return false };
            for d in [0.5f32, 1.0, 1.5, 2.0] {
                let p = base + f * (d * SIZE);
                let Some((y, bad, _)) = self.wagon_ground(p.x, p.z, last + 0.5) else { return false };
                if bad || (y - last).abs() > 1.05 {
                    return false;
                }
                last = y;
            }
        }
        true
    }

    fn new_wagon(&mut self, pos: Vec3, yaw: f32, load: Load, team: Team) -> Wagon {
        let mut animals = Vec::new();
        if let Some(di) = self.mobs.reg.id(team.animal()) {
            for k in 0..team.places().len() {
                let def = &self.mobs.reg.defs[di];
                let mut m = Mob::new(di, def, pos, self.surv.rng.f32().to_bits() as u64 ^ k as u64);
                m.yaw = yaw;
                m.phase = self.surv.rng.f32();
                // A pair walks nearly, but not quite, in step.
                m.walk = k as f32 * 0.6;
                animals.push(m);
            }
        }
        let driver = if team == Team::None {
            None
        } else {
            let name = if load == Load::Covered { "npc_merchant" } else { "settler_farmer" };
            self.mobs.reg.id(name).map(|di| {
                let mut m = Mob::new(di, &self.mobs.reg.defs[di], pos, self.surv.rng.f32().to_bits() as u64);
                m.phase = self.surv.rng.f32();
                m
            })
        };
        let r = self.surv.rng.f32();
        Wagon {
            pos,
            yaw,
            front_yaw: yaw,
            pitch: 0.0,
            roll: 0.0,
            tilt: 0.0,
            load,
            team,
            animals,
            driver,
            wheel_front: r * 6.0,
            wheel_back: r * 4.0,
            speed: 0.0,
            route: Vec::new(),
            parked: false,
            broken: false,
            key: None,
            village: None,
            stuck: 0.0,
            steer_t: 0.0,
            want_yaw: yaw,
            creak_t: 2.0 + r * 4.0,
            crunch_t: 0.0,
            call_t: 8.0 + r * 20.0,
            age: 0.0,
            treadmill: false,
            halt_t: 0.0,
            talk_t: 0.0,
            seed: r.to_bits(),
        }
    }

    /// A travelling rig, picked at random.
    fn random_rig(&mut self, biome: Biome) -> (Load, Team) {
        let r = self.surv.rng.f32();
        let load = if r < 0.5 { Load::Covered } else if r < 0.85 { Load::Hay } else { Load::Empty };
        let t = self.surv.rng.f32();
        let team = match load {
            Load::Covered if t < 0.55 => Team::Horses,
            Load::Covered => Team::Oxen,
            _ if biome.base() == Biome::Desert && t < 0.6 => Team::Donkey,
            _ if t < 0.4 => Team::Horse,
            _ if t < 0.7 => Team::Donkey,
            _ => Team::Oxen,
        };
        (load, team)
    }

    pub(crate) fn update_wagons(&mut self, dt: f32) {
        if self.dim != Dim::Overworld {
            self.wagons.list.clear();
            return;
        }
        if self.wagons.road.is_none() {
            let names = [
                "gravel", "rocky_dirt", "dirt_path", "coarse_dirt", "cobblestone", "cobblestone_bricks", "mossy_cobblestone",
                "stone_bricks", "smooth_stone", "packed_mud", "mud_bricks", "pebble",
            ];
            self.wagons.road = Some(names.iter().filter_map(|n| self.reg.id(n)).collect());
            for (i, b) in self.reg.blocks.iter().enumerate() {
                let n = b.name.as_str();
                if n.ends_with("_leaves") || n.contains("leaves") {
                    self.wagons.leaves.insert(i as BlockId);
                } else if ["_log", "_wood", "_stem", "_hyphae", "_fence", "_fence_gate", "_wall", "cactus", "bamboo", "mushroom_block"].iter().any(|k| n.ends_with(k)) {
                    self.wagons.rough.insert(i as BlockId);
                }
            }
        }
        self.wagon_test_scene();
        self.wagon_talk_test();
        if dt <= 0.0 {
            return;
        }
        self.wagons.clock += dt;
        self.wagons.check_t -= dt;
        if self.wagons.check_t <= 0.0 {
            self.wagons.check_t = 3.0;
            self.spawn_wagons();
        }
        let me = self.body.pos;
        let mut list = std::mem::take(&mut self.wagons.list);
        list.retain(|w| w.treadmill || (w.pos - me).length() < if w.parked { 140.0 } else { FAR });
        for w in list.iter_mut() {
            self.drive_wagon(w, dt);
        }
        // Wagons that sat stuck for long, out of the way of the player, give up.
        list.retain(|w| w.stuck < 20.0 || (w.pos - me).length() < 24.0);
        if log::log_enabled!(log::Level::Debug) && (self.wagons.clock % 5.0) < dt {
            for w in list.iter().filter(|w| !w.parked) {
                log::debug!("wagon at {:.1} speed {:.2} stuck {:.1} route {}", w.pos, w.speed, w.stuck, w.route.len());
            }
        }
        self.wagons.list = list;
        self.wagon_push_player();
    }

    /// Bring in parked wagons near their buildings, and now and then a travelling one.
    fn spawn_wagons(&mut self) {
        let me = self.body.pos;
        let (px, pz) = (me.x as i32, me.z as i32);
        let props = self.world.gen.props_near(px, pz, 128);
        let have: HashSet<(IVec3, usize)> = self.wagons.list.iter().filter_map(|w| w.key).collect();
        let mut roads: HashMap<IVec3, Vec<(Vec3, f32)>> = HashMap::new();
        for p in &props {
            if p.kind == "road" {
                roads.entry(p.key.0).or_default().push((p.pos, p.yaw));
                continue;
            }
            if have.contains(&p.key) || (p.pos - me).length() > 110.0 {
                continue;
            }
            let (load, team, broken) = match p.kind.as_str() {
                "covered_wagon" => (Load::Covered, Team::Horses, false),
                "hay_cart" => (Load::Hay, Team::None, false),
                "broken_wagon" => (Load::Covered, Team::None, true),
                _ => continue,
            };
            let mut w = self.new_wagon(p.pos, p.yaw, load, team);
            w.parked = true;
            w.broken = broken;
            w.key = Some(p.key);
            w.driver = None;
            log::debug!("parked {} at {} facing {:.0}", p.kind, p.pos, p.yaw.to_degrees());
            self.settle_wagon(&mut w);
            self.wagons.list.push(w);
        }
        // A guest in the host's world sees the host's travelling wagons (net_props.rs).
        if !self.mobs.enabled || !self.net_simulates() {
            return;
        }
        let moving = self.wagons.list.iter().filter(|w| !w.parked).count();
        if moving >= MAX_MOVING {
            return;
        }
        let now = self.wagons.clock;
        // Through a village: in at one end of a main road, out at the other.
        for (village, ends) in roads {
            if ends.len() < 2 || self.wagons.list.iter().any(|w| w.village == Some(village)) {
                continue;
            }
            if self.wagons.village_t.get(&village).is_some_and(|&t| now - t < 150.0) {
                continue;
            }
            let centre = ends.iter().map(|e| e.0).sum::<Vec3>() / ends.len() as f32;
            let d = (centre - me).length();
            if !(16.0..130.0).contains(&d) || self.surv.rng.f32() > 0.4 {
                continue;
            }
            self.wagons.village_t.insert(village, now);
            // Come in from an end far enough from the player not to pop up under their nose.
            let mut order: Vec<usize> = (0..ends.len()).collect();
            let k0 = self.surv.rng.below(ends.len() as u32) as usize;
            order.rotate_left(k0);
            for k in order {
                let (end, yaw) = ends[k];
                let dir = fwd(yaw);
                let start = end - dir * 10.0;
                if (start - me).length() < 36.0 {
                    continue;
                }
                let Some(exit) = ends.iter().map(|e| e.0).max_by(|a, b| (*a - end).length().total_cmp(&(*b - end).length())) else { continue };
                let Some((y, wet, _)) = self.wagon_ground(start.x, start.z, end.y) else { continue };
                if wet {
                    continue;
                }
                let biome = self.world.gen.biome_at(start.x as i32, start.z as i32);
                let (load, team) = self.random_rig(biome);
                let mut w = self.new_wagon(Vec3::new(start.x, y, start.z), yaw, load, team);
                w.route = vec![end, exit, exit + dir * 70.0];
                w.village = Some(village);
                log::debug!("wagon comes into the village at {village} from {start}");
                self.settle_wagon(&mut w);
                self.wagons.list.push(w);
                return;
            }
        }
        // Across open country, passing the player by.
        self.wagons.country_t -= 3.0;
        if self.wagons.country_t > 0.0 || self.wagons.list.iter().any(|w| !w.parked && w.village.is_none()) {
            return;
        }
        self.wagons.country_t = 60.0 + self.surv.rng.f32() * 120.0;
        let biome = self.world.gen.biome_at(px, pz);
        if !open_country(biome) {
            return;
        }
        // From behind the player, mostly, so it rolls into view.
        let a = self.yaw + std::f32::consts::PI + (self.surv.rng.f32() - 0.5) * 2.4;
        let dir = Vec3::new(-a.sin(), 0.0, -a.cos());
        let side = Vec3::new(dir.z, 0.0, -dir.x) * ((self.surv.rng.f32() - 0.5) * 30.0);
        let start = me + dir * 58.0 + side;
        let target = me - dir * 58.0 + side * 0.5;
        let Some((y, wet, _)) = self.wagon_ground(start.x, start.z, me.y + 3.0) else { return };
        if wet || (y - me.y).abs() > 24.0 {
            return;
        }
        let way = target - start;
        let yaw = way.x.atan2(way.z);
        let (load, team) = self.random_rig(biome);
        let mut w = self.new_wagon(Vec3::new(start.x, y, start.z), yaw, load, team);
        if self.wagon_way(&w, yaw).is_none() {
            return;
        }
        w.route = vec![target, target + way.normalize_or_zero() * 80.0];
        log::debug!("wagon crosses the country from {start} to {target}");
        self.settle_wagon(&mut w);
        self.wagons.list.push(w);
    }

    /// Set a wagon on the ground: height, tilt, and its team in place.
    fn settle_wagon(&self, w: &mut Wagon) {
        let f = fwd(w.yaw);
        let near = w.pos.y + 1.0;
        let yf = self.axle_ground(w.pos + f * (HALF_BASE * SIZE), w.yaw, near);
        let yb = self.axle_ground(w.pos - f * (HALF_BASE * SIZE), w.yaw, near);
        if let (Some(a), Some(b)) = (yf, yb) {
            w.pos.y = (a.0 + b.0) * 0.5;
            if !w.broken {
                w.pitch = ((a.0 - b.0) / (2.0 * (HALF_BASE * SIZE))).atan().clamp(-0.5, 0.5);
                w.tilt = (a.1 + b.1) * 0.5;
                w.roll = w.tilt;
            }
        }
        self.place_team(w, 1.0);
    }

    /// `ease` 1 sets the animals straight on the ground; less lets them step up and down.
    fn place_team(&self, w: &mut Wagon, ease: f32) {
        let k = w.pos + fwd(w.yaw) * (KINGPIN * SIZE);
        let f = fwd(w.front_yaw);
        let side = Vec3::new(f.z, 0.0, -f.x);
        for (m, (s, a)) in w.animals.iter_mut().zip(w.team.places()) {
            let p = k + (f * *a - side * *s) * SIZE;
            let y = self.wagon_ground(p.x, p.z, w.pos.y + 1.0).map(|g| g.0).unwrap_or(w.pos.y);
            let y = if ease >= 1.0 || (m.body.pos.y - y).abs() > 2.5 { y } else { m.body.pos.y + (y - m.body.pos.y) * ease };
            m.body.pos = Vec3::new(p.x, y, p.z);
            m.yaw = w.front_yaw;
        }
    }

    fn drive_wagon(&mut self, w: &mut Wagon, dt: f32) {
        w.age += dt;
        for m in w.animals.iter_mut().chain(w.driver.iter_mut()) {
            m.age += dt;
        }
        // Still in the driver's shop: stay pulled up.
        let shopping = self.surv.open == crate::survival::Open::Shop && w.driver.as_ref().is_some_and(|d| self.mobs.reg.defs[d.def].display_name == self.surv.shop_name);
        if shopping && (w.pos - self.body.pos).length() < 10.0 {
            w.halt_t = w.halt_t.max(2.0);
        }
        w.halt_t = (w.halt_t - dt).max(0.0);
        w.talk_t = (w.talk_t - dt).max(0.0);
        if w.parked {
            // Hitched and waiting: the animals shift and look about now and then.
            for m in w.animals.iter_mut() {
                m.walk_speed = 0.0;
                m.head_pitch = -0.25 + (m.age * 0.21 + m.phase * 5.0).sin().max(0.0) * 0.5;
            }
            self.wagon_calls(w, dt);
            return;
        }
        let cruise = w.team.cruise();
        // Steering: toward the next point of the route, round what is in the way, keeping to
        // the road where there is one.
        while w.route.first().is_some_and(|t| Vec3::new(t.x - w.pos.x, 0.0, t.z - w.pos.z).length() < 3.0) {
            w.route.remove(0);
        }
        w.steer_t -= dt;
        let mut go = true;
        if w.steer_t <= 0.0 && !w.treadmill {
            w.steer_t = 0.4;
            let goal = match w.route.first() {
                Some(t) => (t.x - w.pos.x).atan2(t.z - w.pos.z),
                None => w.front_yaw,
            };
            let mut best: Option<(f32, f32)> = None;
            for off in [0.0f32, 0.3, -0.3, 0.6, -0.6, 0.95, -0.95, 1.4, -1.4, 2.0, -2.0] {
                let yaw = w.front_yaw + off;
                let Some(bonus) = self.wagon_way(w, yaw) else { continue };
                let score = wrap(yaw - goal).cos() * 2.0 - off.abs() * 0.35 + bonus;
                if best.is_none_or(|b| score > b.0) {
                    best = Some((score, yaw));
                }
            }
            match best {
                Some((_, yaw)) => {
                    w.want_yaw = yaw;
                    w.stuck = 0.0;
                }
                None => {
                    go = false;
                    w.stuck += 0.4;
                    // Turn about on the spot, slowly, to find a way out.
                    w.want_yaw = w.front_yaw + 0.8;
                }
            }
        }
        if w.stuck > 0.0 {
            go = w.stuck > 2.0;
        }
        // Stuck: creep round in a slow turn until a way opens.
        let target = if !go || w.halt_t > 0.0 { 0.0 } else if w.stuck > 0.0 { cruise * 0.4 } else { cruise };
        // Heavy going in sand and snow; on ice the wheels slip, slow to start and to stop.
        let g = voxel::physics::Terrain::ground(&self.world, (w.pos - Vec3::Y * 0.05).floor().as_ivec3());
        let hold = (g.grip / 8.0).min(1.0);
        w.speed += (target * g.speed - w.speed).clamp(-dt * 1.5 * hold, dt * 0.8 * hold);
        // Uphill slower.
        let v = w.speed * (1.0 - w.pitch.max(0.0) * 0.8);
        let turn = wrap(w.want_yaw - w.front_yaw).clamp(-0.8 * dt, 0.8 * dt) * (0.3 + v.min(1.0) * 0.7);
        w.front_yaw = wrap(w.front_yaw + turn);
        let artic = wrap(w.front_yaw - w.yaw).clamp(-MAX_ARTIC, MAX_ARTIC);
        w.front_yaw = wrap(w.yaw + artic);
        let mut step = v * dt;
        if !w.treadmill && step > 0.0 && !self.wagon_may_step(w) {
            // Never more than a one block step up or down, never onto a tree or into water:
            // stop, and let the steering find another way round.
            step = 0.0;
            w.speed = 0.0;
            w.steer_t = w.steer_t.min(0.1);
            w.stuck = w.stuck.max(0.4);
        }
        if !w.treadmill && step > 0.0 {
            // The front axle goes where the team pulls it; the rear axle follows.
            let k0 = w.pos + fwd(w.yaw) * (KINGPIN * SIZE);
            let r0 = w.pos - fwd(w.yaw) * (HALF_BASE * SIZE);
            let k1 = k0 + fwd(w.front_yaw) * step;
            let back = Vec3::new(k1.x - r0.x, 0.0, k1.z - r0.z);
            let yaw = back.x.atan2(back.z);
            let r1 = k1 - fwd(yaw) * ((KINGPIN * SIZE) + (HALF_BASE * SIZE));
            w.wheel_back += Vec3::new(r1.x - r0.x, 0.0, r1.z - r0.z).length() / (WHEEL_R * SIZE);
            w.yaw = yaw;
            w.pos = Vec3::new(k1.x, w.pos.y, k1.z) - fwd(yaw) * (KINGPIN * SIZE);
            // Height and tilt from the ground under the axles, eased.
            let f = fwd(w.yaw);
            let near = w.pos.y + 1.0;
            let yf = self.axle_ground(w.pos + f * (HALF_BASE * SIZE), w.yaw, near);
            let yb = self.axle_ground(w.pos - f * (HALF_BASE * SIZE), w.yaw, near);
            if let (Some(a), Some(b)) = (yf, yb) {
                // The ground shape is already smooth; the easing only takes out the jitter.
                let y = (a.0 + b.0) * 0.5;
                w.pos.y += (y - w.pos.y) * (dt * 12.0).min(1.0);
                let p = ((a.0 - b.0) / (2.0 * (HALF_BASE * SIZE))).atan().clamp(-0.5, 0.5);
                w.pitch += (p - w.pitch) * (dt * 10.0).min(1.0);
                w.tilt += ((a.1 + b.1) * 0.5 - w.tilt) * (dt * 10.0).min(1.0);
            }
        } else {
            w.wheel_back += step / (WHEEL_R * SIZE);
        }
        w.wheel_front += step / (WHEEL_R * SIZE);
        // A little sway on rough ground.
        w.roll = w.tilt + (w.age * 2.3).sin() * 0.012 * v.min(1.0);
        let pace = v / cruise.max(0.1);
        let old: Vec<f32> = w.animals.iter().map(|m| m.walk).collect();
        for m in w.animals.iter_mut() {
            let def = &self.mobs.reg.defs[m.def];
            m.walk_speed = def.speed * 0.75 * pace.min(1.0);
            m.walk += v * dt * 4.2 / def.scale.max(0.5);
            m.head_pitch = if w.team == Team::Oxen { -0.18 } else { -0.05 };
            m.head_yaw = 0.0;
        }
        let seat = driver_seat(w);
        if let Some(d) = w.driver.as_mut() {
            let idle = ((d.age * 0.17 + d.phase * 9.0).sin() * 0.9).clamp(-0.6, 0.6) * ((d.age * 0.05).sin().abs());
            // Pulled up to trade: they turn to look at you.
            let want = if w.halt_t > 0.0 {
                let to = self.body.pos - seat;
                wrap(to.x.atan2(to.z) - w.yaw).clamp(-1.1, 1.1)
            } else {
                idle
            };
            d.head_yaw += (want - d.head_yaw) * (dt * if w.halt_t > 0.0 { 6.0 } else { 2.0 }).min(1.0);
        }
        self.place_team(w, (dt * 7.0).min(1.0));
        // Hooves: one clop as each foot comes down (four a stride), softer on soft ground.
        let half = std::f32::consts::FRAC_PI_2;
        for (i, m) in w.animals.iter().enumerate().take(2) {
            if (old[i] / half).floor() != (m.walk / half).floor() && v > 0.2 {
                let at = m.body.pos;
                let hard = self.wagon_ground(at.x, at.z, at.y + 0.5).is_some_and(|g| g.2);
                let name = ["hoof_1", "hoof_2", "hoof_3"][self.surv.rng.below(3) as usize];
                let base = if w.team == Team::Oxen { 0.72 } else if w.team == Team::Donkey { 1.0 } else { 0.9 };
                let rate = base * if hard { 1.05 } else { 0.85 } * (0.95 + self.surv.rng.f32() * 0.1);
                let vol = if hard { 0.42 } else { 0.26 } * if w.team.pair() { 0.8 } else { 1.0 };
                if let Some(fx) = crate::mob_sounds::clip(name, rate) {
                    self.sfx_at(Sfx::Fx(fx), at, vol, 22.0);
                }
            }
        }
        // The wood of the wagon working as it rolls, and the wheels on gravel.
        if v > 0.3 {
            w.creak_t -= dt;
            if w.creak_t <= 0.0 {
                w.creak_t = 2.5 + self.surv.rng.f32() * 5.0;
                let name = if self.surv.rng.f32() < 0.6 { "wagon_creak_1" } else { "wagon_creak_2" };
                if let Some(fx) = crate::mob_sounds::clip(name, 0.8 + self.surv.rng.f32() * 0.15) {
                    self.sfx_at(Sfx::Fx(fx), w.pos + Vec3::Y, 0.3, 20.0);
                }
            }
            w.crunch_t -= dt * v;
            if w.crunch_t <= 0.0 {
                w.crunch_t = 1.1 + self.surv.rng.f32() * 0.6;
                if self.wagon_ground(w.pos.x, w.pos.z, w.pos.y + 0.5).is_some_and(|g| g.2) {
                    let name = if self.surv.rng.f32() < 0.5 { "wagon_gravel_1" } else { "wagon_gravel_2" };
                    if let Some(fx) = crate::mob_sounds::clip(name, 0.65 + self.surv.rng.f32() * 0.1) {
                        self.sfx_at(Sfx::Fx(fx), w.pos, 0.22, 16.0);
                    }
                }
            }
        }
        self.wagon_calls(w, dt);
    }

    /// Now and then one of the team snorts or lows.
    fn wagon_calls(&mut self, w: &mut Wagon, dt: f32) {
        w.call_t -= dt;
        if w.call_t > 0.0 || w.animals.is_empty() {
            return;
        }
        w.call_t = 14.0 + self.surv.rng.f32() * 26.0;
        let m = &w.animals[self.surv.rng.below(w.animals.len() as u32) as usize];
        let def = &self.mobs.reg.defs[m.def];
        let (r1, r2) = (self.surv.rng.f32(), self.surv.rng.f32());
        if let Some(fx) = crate::mob_sounds::pick(&def.name, Call::Idle, def.scale, r1, r2) {
            self.sfx_at(Sfx::Fx(fx), m.body.pos + Vec3::Y * 1.2, 0.45, 24.0);
        }
    }

    /// Right click on a wagon's driver: they pull up, turn to you, say a word and open their
    /// stall (the merchant on a covered wagon, the farmer on a hay cart).
    pub(crate) fn wagon_driver_use(&mut self) -> bool {
        if self.wagons.list.is_empty() {
            return false;
        }
        let (eye, dir) = (self.body.eye(), self.forward());
        let mut best: Option<(usize, f32)> = None;
        for (k, w) in self.wagons.list.iter().enumerate() {
            if w.driver.is_none() || (w.pos - eye).length() > 12.0 {
                continue;
            }
            let (lo, hi) = driver_box(w);
            if let Some(t) = crate::mobs::ray_box(eye, dir, lo, hi) {
                if t <= 4.5 && best.is_none_or(|b| t < b.1) {
                    best = Some((k, t));
                }
            }
        }
        let Some((k, t)) = best else { return false };
        // A mob or a solid block in front of the driver takes the click instead.
        if self.mobs.pick(eye, dir, 4.5).is_some_and(|(_, mt)| mt < t) {
            return false;
        }
        let block_dist = self.target.filter(|h| self.reg.get(self.world.block_or_air(h.block)).shape != voxel::block::Shape::Cross).map(|h| h.distance).unwrap_or(f32::MAX);
        if t > block_dist {
            return false;
        }
        let w = &mut self.wagons.list[k];
        w.halt_t = 30.0;
        w.talk_t = 3.0;
        let seed = w.seed;
        let Some(dr) = w.driver.as_ref() else { return false };
        let def = &self.mobs.reg.defs[dr.def];
        let (name, dn) = (def.name.clone(), def.display_name.clone());
        let mut pick = |n: usize| (self.surv.rng.f32() * n as f32) as usize % n;
        if name.starts_with("settler_") {
            let line = FARMER_LINES[pick(FARMER_LINES.len())];
            if self.settler_trade(&name, &dn) && self.surv.open == crate::survival::Open::Shop {
                self.toast(format!("{dn}: {line}"), 5.0);
            }
            return true;
        }
        // The merchant: their own goods, and three things from the road.
        let line = MERCHANT_LINES[pick(MERCHANT_LINES.len())];
        let mut offers: Vec<(&'static str, u8, u32)> = crate::npcs::SHOPS.iter().find(|s| s.0 == "npc_merchant").map(|s| s.1.to_vec()).unwrap_or_default();
        let mut idx: Vec<usize> = (0..ROAD_GOODS.len()).collect();
        let mut r = seed | 1;
        for i in (1..idx.len()).rev() {
            r ^= r << 13;
            r ^= r >> 17;
            r ^= r << 5;
            idx.swap(i, r as usize % (i + 1));
        }
        offers.extend(idx.iter().map(|&i| ROAD_GOODS[i]).filter(|g| self.items.id(g.0).is_some()).take(3));
        let mut inv = voxel::item::Inventory::new(offers.len());
        let mut prices = Vec::new();
        for &(item, count, price) in &offers {
            if let Some(id) = self.items.id(item) {
                inv.slots[prices.len()] = Some(voxel::item::ItemStack::new(id, count));
                prices.push(price);
            }
        }
        self.surv.shop = inv;
        self.surv.shop_prices = prices;
        self.surv.shop_name = dn.clone();
        self.surv.shop_want = None;
        self.surv.shop_scroll = 0;
        self.surv.open = crate::survival::Open::Shop;
        self.set_screen(crate::Screen::Inventory);
        self.toast(format!("{dn}: {line}"), 5.0);
        true
    }

    /// Test hook MC_WAGON_TALK=1 (shop open) or =look (just the chat): stand by the first
    /// driver and click them.
    fn wagon_talk_test(&mut self) {
        if !self.wagons.talk_test || self.wagons.list.is_empty() {
            return;
        }
        let Some(w) = self.wagons.list.iter().find(|w| w.driver.is_some()) else { return };
        let (lo, hi) = driver_box(w);
        let aim = (lo + hi) * 0.5 + Vec3::Y * 0.3;
        // Beside the seat, at arm's length.
        let side = Quat::from_rotation_y(w.yaw) * Vec3::X;
        let stand = Vec3::new(aim.x, w.pos.y, aim.z) + side * 2.6;
        let Some((y, _, _)) = self.wagon_ground(stand.x, stand.z, w.pos.y + 2.0) else { return };
        self.body.pos = Vec3::new(stand.x, y, stand.z);
        let to = aim - self.body.eye();
        self.yaw = (-to.x).atan2(-to.z);
        self.pitch = (to.y / to.length()).asin();
        self.target = None;
        if self.wagon_driver_use() {
            log::info!("wagon talk test: shop open with {} goods", self.surv.shop_prices.len());
            self.wagons.talk_test = false;
            if std::env::var("MC_WAGON_TALK").as_deref() == Ok("look") {
                self.surv.open = crate::survival::Open::Craft;
                self.set_screen(crate::Screen::Playing);
            }
        }
    }

    /// Keep the player out of the wagons' beds.
    fn wagon_push_player(&mut self) {
        let me = self.body.pos;
        let r = self.body.half_width;
        for w in &self.wagons.list {
            let d = me - w.pos;
            if d.length_squared() > 16.0 {
                continue;
            }
            let top = match w.load {
                Load::Covered => 4.0,
                Load::Hay => 1.4,
                Load::Empty => 1.1,
            };
            if me.y > w.pos.y + top - 0.4 || me.y + self.body.height < w.pos.y + 0.3 {
                continue;
            }
            let (s, c) = w.yaw.sin_cos();
            // Into the wagon's frame: x across, z along.
            let lx = d.x * c - d.z * s;
            let lz = d.x * s + d.z * c;
            let (hx, hz) = (0.8 * SIZE + r, 1.3 * SIZE + r);
            if lx.abs() >= hx || lz.abs() >= hz {
                continue;
            }
            let (px, pz) = (hx - lx.abs(), hz - lz.abs());
            let (nx, nz) = if px < pz { (lx.signum() * px, 0.0) } else { (0.0, lz.signum() * pz) };
            let push = Vec3::new(nx * c + nz * s, 0.0, -nx * s + nz * c);
            self.body.pos += push;
            break;
        }
    }

    /// Test hook: MC_WAGON=covered|hay|oxen|donkey|all puts wagons in front of the player,
    /// walking on the spot (so a screenshot catches them moving; MC_WAGON_DRIVE=1 lets them
    /// drive off). MC_WAGON=road sends one in along the nearest village road.
    fn wagon_test_scene(&mut self) {
        if self.wagons.test_done {
            return;
        }
        let Ok(what) = std::env::var("MC_WAGON") else {
            self.wagons.test_done = true;
            return;
        };
        let me = self.body.pos;
        if what == "road" {
            // Driving in along the nearest village road (MC_WAGON_DIST blocks in from its end).
            let props = self.world.gen.props_near(me.x as i32, me.z as i32, 96);
            let Some(end) = props.iter().filter(|p| p.kind == "road").min_by(|a, b| (a.pos - me).length().total_cmp(&(b.pos - me).length())) else { return };
            let ahead = std::env::var("MC_WAGON_DIST").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let p = end.pos + fwd(end.yaw) * ahead;
            let Some((y, _, _)) = self.wagon_ground(p.x, p.z, p.y + 1.0) else { return };
            let mut w = self.new_wagon(Vec3::new(p.x, y, p.z), end.yaw, Load::Covered, Team::Horses);
            w.speed = Team::Horses.cruise();
            w.route = vec![end.pos + fwd(end.yaw) * 200.0];
            w.village = Some(end.key.0);
            self.settle_wagon(&mut w);
            self.wagons.list.push(w);
            self.wagons.test_done = true;
            return;
        }
        let look = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let side = Vec3::new(-look.z, 0.0, look.x);
        let rigs: Vec<(Load, Team)> = match what.as_str() {
            "hay" => vec![(Load::Hay, Team::Horse)],
            "oxen" => vec![(Load::Covered, Team::Oxen)],
            "donkey" => vec![(Load::Hay, Team::Donkey)],
            "all" => vec![(Load::Covered, Team::Horses), (Load::Hay, Team::Donkey), (Load::Covered, Team::Oxen)],
            _ => vec![(Load::Covered, Team::Horses)],
        };
        let n = rigs.len();
        let mut placed = Vec::new();
        for (k, (load, team)) in rigs.into_iter().enumerate() {
            let along = (k as f32 - (n as f32 - 1.0) * 0.5) * 6.5;
            let dist = std::env::var("MC_WAGON_DIST").ok().and_then(|v| v.parse().ok()).unwrap_or(7.0);
            let p = me + look * (dist + k as f32 * 2.5) + side * along;
            let Some((y, _, _)) = (0..4).find_map(|k| self.wagon_ground(p.x, p.z, me.y + 2.0 - k as f32 * 7.0)) else {
                log::debug!("wagon test scene: no ground yet at {p}");
                return;
            };
            // Side on to the camera, a little turned, the team heading to the right.
            let yaw = side.x.atan2(side.z) + 0.35;
            let mut w = self.new_wagon(Vec3::new(p.x, y, p.z), yaw, load, team);
            w.treadmill = std::env::var("MC_WAGON_DRIVE").is_err();
            w.speed = team.cruise();
            w.front_yaw = yaw + if k % 2 == 0 { 0.25 } else { -0.2 };
            w.route = vec![p + fwd(yaw) * 200.0];
            self.settle_wagon(&mut w);
            placed.push(w);
        }
        log::info!("wagon test scene: {} wagons in front of the player", placed.len());
        self.wagons.talk_test = std::env::var("MC_WAGON_TALK").is_ok();
        self.wagons.list.extend(placed);
        self.wagons.test_done = true;
    }

    /// What to draw for every wagon in sight.
    /// The travelling wagons only, anywhere (multijogador: net_props.rs).
    pub(crate) fn wagon_instances_moving(&self, cam_block: IVec3) -> Vec<(usize, Bones)> {
        self.wagon_instances_of(cam_block, f32::MAX, true)
    }

    pub(crate) fn wagon_instances(&self, cam_block: IVec3, max_dist: f32) -> Vec<(usize, Bones)> {
        self.wagon_instances_of(cam_block, max_dist, false)
    }

    fn wagon_instances_of(&self, cam_block: IVec3, max_dist: f32, moving_only: bool) -> Vec<(usize, Bones)> {
        let mut out = Vec::new();
        if self.wagons.list.is_empty() {
            return out;
        }
        let Some(d) = self.wagon_defs() else { return out };
        let cam = cam_block.as_vec3();
        let reg = &self.mobs.reg;
        let s = |i: usize| Mat4::from_scale(Vec3::splat(reg.defs[i].scale / 16.0));
        for w in &self.wagons.list {
            if (w.pos - cam).length() > max_dist || (moving_only && w.parked) {
                continue;
            }
            let l = self.world.light((w.pos + Vec3::Y * 1.2).floor().as_ivec3());
            let light = [(l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0, 0.0, 0.0];
            let rigid = |def: usize, m: Mat4| {
                let mut b = Bones { light, ..Default::default() };
                let a = m.to_cols_array_2d();
                for (k, mm) in b.m.iter_mut().enumerate() {
                    if k < reg.defs[def].parts.len() {
                        *mm = a;
                    }
                }
                (def, b)
            };
            // A broken one has lost its front right wheel: the front axle tips over onto that
            // end and the bed sinks forward onto it, both rear wheels still on the ground.
            let sink = if w.broken {
                let rear = Vec3::new(0.0, 0.0, -HALF_BASE * SIZE);
                Mat4::from_translation(rear) * Mat4::from_rotation_x(BROKEN_DROP * 0.5 / HALF_BASE / 2.0) * Mat4::from_translation(-rear)
            } else {
                Mat4::IDENTITY
            };
            let body = Mat4::from_translation(w.pos - cam)
                * Mat4::from_rotation_y(w.yaw)
                * sink
                * Mat4::from_rotation_x(-w.pitch)
                * Mat4::from_rotation_z(w.roll)
                * Mat4::from_scale(Vec3::splat(SIZE));
            out.push(rigid(d.body, body * s(d.body)));
            match w.load {
                Load::Covered => {
                    out.push(rigid(d.cover, body * Mat4::from_translation(Vec3::Y * COVER_LIFT) * s(d.cover)));
                    out.push(rigid(d.cargo, body * s(d.cargo)));
                }
                Load::Hay => out.push(rigid(d.hay, body * s(d.hay))),
                Load::Empty => {}
            }
            let wheel = |m: Mat4, x: f32, z: f32, spin: f32| m * Mat4::from_translation(Vec3::new(x, WHEEL_R, z)) * Mat4::from_rotation_x(spin) * s(d.wheel);
            for x in [-WHEEL_X, WHEEL_X] {
                out.push(rigid(d.wheel, wheel(body, x, -HALF_BASE, w.wheel_back)));
            }
            // The front: axle, shafts or pole, front wheels, turned with the team.
            let mut front = body * Mat4::from_translation(Vec3::new(0.0, 0.0, KINGPIN)) * Mat4::from_rotation_y(wrap(w.front_yaw - w.yaw));
            if w.broken {
                // The axle rocks about its middle, which sank half the drop with the bed, so the
                // left wheel stays on the ground and the bare right end rests on it.
                let hub = Vec3::new(0.0, WHEEL_R, HALF_BASE - KINGPIN);
                front = front * Mat4::from_translation(hub) * Mat4::from_rotation_z(-(BROKEN_DROP * 0.5 / WHEEL_X).atan()) * Mat4::from_translation(-hub);
            }
            let pivot = Vec3::new(0.0, 0.4375, HALF_BASE - KINGPIN);
            let lift = Mat4::from_translation(pivot) * Mat4::from_rotation_x(-w.team.lift()) * Mat4::from_translation(-pivot);
            let gear = if w.team.pair() { d.pole } else { d.shafts };
            out.push(rigid(gear, front * lift * s(gear)));
            for x in [-WHEEL_X, WHEEL_X] {
                if w.broken && x > 0.0 {
                    continue;
                }
                out.push(rigid(d.wheel, wheel(front, x, HALF_BASE - KINGPIN, w.wheel_front)));
            }
            if w.broken {
                // The lost wheel lies flat beside it.
                let flat = Mat4::from_translation(w.pos - cam) * Mat4::from_rotation_y(w.yaw);
                let at = Vec3::new(1.75, 0.07, 0.4);
                let m = flat * Mat4::from_translation(at) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2 * 0.96) * Mat4::from_rotation_x(0.7) * s(d.wheel);
                out.push(rigid(d.wheel, m));
            }
            for m in &w.animals {
                let def = &reg.defs[m.def];
                let l = self.world.light((m.body.pos + Vec3::Y).floor().as_ivec3());
                let bones = self.mobs.anim.bones_now(m, def, m.def, cam_block, (l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
                if let Some(h) = d.harness.filter(|_| w.team != Team::Oxen) {
                    let mut hb = Bones { light: bones.light, ..Default::default() };
                    for k in 0..reg.defs[h].parts.len() {
                        hb.m[k] = bones.m[0];
                    }
                    out.push((h, hb));
                }
                out.push((m.def, bones));
            }
            if let Some(dr) = &w.driver {
                out.push((dr.def, driver_bones(dr, &reg.defs[dr.def], body, w.speed, w.talk_t, light)));
            }
        }
        out
    }
}

/// The driver sits on the seat, legs out in front, holding the reins.
fn driver_bones(m: &Mob, def: &voxel::mob::MobDef, body: Mat4, speed: f32, talk: f32, light: [f32; 4]) -> Bones {
    let mut b = Bones { light, ..Default::default() };
    let leg = def.parts.iter().filter(|p| matches!(p.anim, Anim::Leg(_))).map(|p| p.size.1 as f32).fold(0.0, f32::max);
    let k = def.scale / 16.0;
    let bob = (m.age * 9.0).sin().abs() * 0.015 * speed.min(1.0);
    // The wagon is drawn SIZE times bigger; the driver keeps a person's size.
    let model = body * Mat4::from_translation(Vec3::new(0.0, 1.125 - leg * k / SIZE + bob, 0.86)) * Mat4::from_scale(Vec3::splat(k / SIZE));
    let reins = -0.75 + (m.age * 4.5).sin() * 0.04 * speed.min(1.0);
    for (i, p) in def.parts.iter().enumerate().take(b.m.len() - 1) {
        let pivot = Vec3::new(p.pivot.0 as f32, p.pivot.1 as f32, p.pivot.2 as f32);
        let rot = match p.anim {
            Anim::Head => Quat::from_rotation_y(m.head_yaw) * Quat::from_rotation_x(0.08),
            Anim::Leg(_) => Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2 + 0.15) * Quat::from_rotation_z(if p.pivot.0 < 0 { 0.06 } else { -0.06 }),
            Anim::Arm(_) => Quat::from_rotation_x(reins),
            _ => Quat::IDENTITY,
        };
        let part = Mat4::from_translation(pivot) * Mat4::from_quat(rot) * Mat4::from_translation(-pivot);
        b.m[i] = (model * part).to_cols_array_2d();
        // Speaking: the mouth opens and shuts with the syllables, as in anim.rs.
        if i == 0 && talk > 0.0 && crate::entity::has_mouth(def) {
            let syl = ((m.age * 9.0).sin() * 0.5 + 0.5) * (0.6 + 0.4 * (m.age * 3.3).sin().abs());
            let top = p.origin.1 as f32 + 2.0;
            let open = Mat4::from_translation(Vec3::Y * top) * Mat4::from_scale(Vec3::new(1.0, 0.2 + talk.min(1.0) * syl * 1.1, 1.0)) * Mat4::from_translation(-Vec3::Y * top);
            b.m[MOUTH_PART] = (model * part * open).to_cols_array_2d();
        }
    }
    b
}

/// Where the driver sits: the middle of the seat, at the hips.
fn driver_seat(w: &Wagon) -> Vec3 {
    w.pos + Quat::from_rotation_y(w.yaw) * Vec3::new(0.0, 1.125 * SIZE, 0.86 * SIZE)
}

/// The driver's hit box, for a click: hips to the top of the head, a little generous.
fn driver_box(w: &Wagon) -> (Vec3, Vec3) {
    let seat = driver_seat(w);
    (seat - Vec3::new(0.45, 0.25, 0.45), seat + Vec3::new(0.45, 1.4, 0.45))
}

/// What a travelling merchant has brought from far off, on top of the merchant's own goods:
/// three of these, different for every wagon.
const ROAD_GOODS: [(&str, u8, u32); 12] = [
    ("compass", 1, s(25)),
    ("map", 1, s(18)),
    ("name_tag", 1, s(30)),
    ("lead", 2, s(10)),
    ("saddle", 1, s(45)),
    ("spyglass", 1, s(35)),
    ("cocoa_beans", 6, s(6)),
    ("sweet_berries", 8, s(4)),
    ("honey_bottle", 2, s(8)),
    ("cactus", 4, s(4)),
    ("bamboo", 8, s(4)),
    ("sugar_cane", 8, s(4)),
];

/// The merchant on the road.
const MERCHANT_LINES: [&str; 7] = [
    "Opa! Veja o que eu trouxe.",
    "Estrada longa... Vai querer algo?",
    "Os bois descansam, a gente negocia!",
    "Trouxe coisas de além da serra.",
    "À noite, leve uma tocha a mais!",
    "De vila em vila, sempre com novidade.",
    "Bom dia, viajante! Aceito moedas.",
];

/// The farmer taking hay to market.
const FARMER_LINES: [&str; 5] = [
    "Levando feno pra vila. Quer algo?",
    "Colheita boa! Compro seu trigo.",
    "Ô, calma, cavalo! O que vai ser?",
    "Pão fresco e maçã do pé!",
    "Dia bonito pra estrada, né?",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Rolling over a one-block step, up and down, the axle never jumps and never sinks into
    /// the edge.
    #[test]
    fn wheels_ease_over_steps() {
        for up in [1.0f32, -1.0] {
            let ground = |x: f32| if x >= 0.0 { up.max(0.0) } else { (-up).max(0.0) };
            let mut last = None;
            let mut x = -3.0;
            while x < 3.0 {
                let mut g = [0.0; SAMPLES];
                for (n, y) in g.iter_mut().enumerate() {
                    *y = ground(x + ramp_d(n));
                }
                let h = wheel_rise(&g);
                assert!(h >= ground(x) - 1e-4, "sinks at {x}: {h}");
                // The wheel clears the corner of the step.
                let r = WHEEL_R * SIZE;
                let corner = Vec3::new(x, h + r, 0.0) - Vec3::new(0.0, 1.0, 0.0);
                assert!(corner.length() > r - 0.05, "clips the edge at {x}: {h}");
                if let Some(l) = last {
                    let dh: f32 = h - l;
                    assert!(dh.abs() < 0.1, "jumps {dh} at {x}");
                }
                last = Some(h);
                x += 0.02;
            }
        }
    }
}
