//! Aircraft (Immersive Aircraft's idea, our own code and models): a biplane, two airships, a
//! pedal-powered gyrodyne, a quadrocopter and an armed warship, all flown by one flight model
//! fed with different numbers. They are built from crafted parts, burn fuel (the gyrodyne runs
//! on the pilot's legs), take up to three upgrades, and the cargo airship carries a chest.
//!
//! Right click an aircraft to board it; the mouse steers, W/S set the throttle, space climbs,
//! Ctrl sinks, sneak climbs out. Right click it holding fuel or an upgrade to fit it, sneak +
//! right click the cargo airship to open its hold. Gunpowder gives a short rocket boost and the
//! warship's cannon fires with the attack button. The pilot is the only rider for now: the
//! other seats wait for multiplayer (Fase 7).

use glam::{IVec3, Mat4, Vec3};
use serde::{Deserialize, Serialize};
use voxel::item::{Inventory, ItemStack};
use voxel::mob::Anim;
use voxel::physics::{Body, Terrain};
use winit::keyboard::KeyCode;

use crate::audio::Sfx;
use crate::controller::Action;
use crate::entity::Bones;
use crate::survival::{Container, Open};
use crate::Game;

/// Containers at this x are aircraft holds; z is the hold's number.
pub const CARGO_X: i32 = i32::MIN + 11;
const MAX_FUEL: f32 = 1200.0;
const MAX_UPGRADES: usize = 3;

/// One aircraft type: the flight model's numbers.
struct Craft {
    /// Item and model name.
    name: &'static str,
    seats: u8,
    width: f32,
    height: f32,
    /// Blocks a second at full throttle.
    max_speed: f32,
    /// How fast it gets there (blocks a second, per second).
    accel: f32,
    /// Fixed wings: the airspeed below which it sinks. Zero for craft that hover.
    stall: f32,
    /// Climb and sink speed of a hovering craft.
    climb: f32,
    /// Turn rate (radians a second).
    turn: f32,
    /// Fuel burnt a second at full throttle; zero runs on the pilot's legs.
    fuel_use: f32,
    /// Moves sideways with A/D instead of only leaning.
    strafe: bool,
    cannon: bool,
    /// Slots in its hold.
    storage: usize,
    health: f32,
    /// Engine voice (see `Sfx::Engine`).
    sound: u8,
    /// Where the pilot sits (blocks, model space, +Z forward).
    seat: Vec3,
}

const CRAFTS: &[Craft] = &[
    Craft { name: "biplane", seats: 1, width: 2.0, height: 1.2, max_speed: 26.0, accel: 6.0, stall: 9.0, climb: 0.0, turn: 1.5, fuel_use: 1.0, strafe: false, cannon: false, storage: 0, health: 30.0, sound: 0, seat: Vec3::new(0.0, 0.45, -0.25) },
    Craft { name: "airship", seats: 2, width: 1.6, height: 3.4, max_speed: 9.0, accel: 2.5, stall: 0.0, climb: 3.5, turn: 0.9, fuel_use: 0.35, strafe: false, cannon: false, storage: 0, health: 30.0, sound: 2, seat: Vec3::new(0.0, 0.5, 0.0) },
    Craft { name: "cargo_airship", seats: 2, width: 1.6, height: 3.4, max_speed: 7.5, accel: 2.0, stall: 0.0, climb: 3.0, turn: 0.8, fuel_use: 0.4, strafe: false, cannon: false, storage: 27, health: 34.0, sound: 2, seat: Vec3::new(0.0, 0.5, -0.4) },
    Craft { name: "gyrodyne", seats: 2, width: 1.4, height: 1.6, max_speed: 13.0, accel: 3.5, stall: 0.0, climb: 4.0, turn: 1.4, fuel_use: 0.0, strafe: false, cannon: false, storage: 0, health: 24.0, sound: 1, seat: Vec3::new(0.0, 0.6, -0.1) },
    Craft { name: "quadrocopter", seats: 1, width: 1.4, height: 0.9, max_speed: 8.0, accel: 6.0, stall: 0.0, climb: 5.0, turn: 2.2, fuel_use: 0.5, strafe: true, cannon: false, storage: 0, health: 16.0, sound: 1, seat: Vec3::new(0.0, 0.45, 0.0) },
    Craft { name: "warship", seats: 4, width: 2.2, height: 3.2, max_speed: 7.0, accel: 2.0, stall: 0.0, climb: 2.5, turn: 0.7, fuel_use: 0.8, strafe: false, cannon: true, storage: 9, health: 60.0, sound: 2, seat: Vec3::new(0.0, 0.75, -0.75) },
];

/// Upgrades: (item, speed ×, fuel use ×, health ×).
const UPGRADES: &[(&str, f32, f32, f32)] =
    &[("enhanced_propeller", 1.2, 1.0, 1.0), ("eco_engine", 1.0, 0.6, 1.0), ("nether_engine", 1.35, 1.3, 1.0), ("steel_hull", 0.95, 1.0, 1.6)];

/// Fuel: (item, seconds at full throttle, what is left in hand).
const FUELS: &[(&str, f32, Option<&str>)] =
    &[("coal", 80.0, None), ("charcoal", 80.0, None), ("blaze_rod", 120.0, None), ("lava_bucket", 1000.0, Some("bucket"))];

/// An aircraft as saved with the world.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plane {
    pub kind: String,
    pub dim: u8,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub fuel: f32,
    pub health: f32,
    pub upgrades: Vec<String>,
    /// Number of its hold in the containers (0: none yet).
    pub cargo: i32,
}

/// An aircraft in the loaded dimension.
pub struct Live {
    kind: usize,
    body: Body,
    yaw: f32,
    pitch: f32,
    roll: f32,
    /// Forward speed of a fixed-wing craft.
    speed: f32,
    /// -0.5..1 (hovering craft can back up).
    throttle: f32,
    /// Engine (or rotor) running speed 0..1.
    spin: f32,
    prop: f32,
    fuel: f32,
    health: f32,
    upgrades: Vec<String>,
    cargo: i32,
    hurt: f32,
}

impl Live {
    fn aabb(&self) -> (Vec3, Vec3) {
        let c = &CRAFTS[self.kind];
        let p = self.body.pos;
        (p - Vec3::new(c.width * 0.5, 0.0, c.width * 0.5), p + Vec3::new(c.width * 0.5, c.height, c.width * 0.5))
    }

    fn heading(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos())
    }

    /// Speed, fuel use and toughness after upgrades.
    fn factors(&self) -> (f32, f32, f32) {
        let mut f = (1.0, 1.0, 1.0);
        for u in &self.upgrades {
            if let Some(&(_, s, fu, h)) = UPGRADES.iter().find(|x| x.0 == u) {
                f = (f.0 * s, f.1 * fu, f.2 * h);
            }
        }
        f
    }

    fn max_health(&self) -> f32 {
        CRAFTS[self.kind].health * self.factors().2
    }

    fn to_plane(&self, dim: u8) -> Plane {
        let p = self.body.pos;
        Plane {
            kind: CRAFTS[self.kind].name.to_string(),
            dim,
            pos: [p.x, p.y, p.z],
            yaw: self.yaw,
            fuel: self.fuel,
            health: self.health,
            upgrades: self.upgrades.clone(),
            cargo: self.cargo,
        }
    }

    fn from_plane(p: &Plane) -> Option<Live> {
        let kind = CRAFTS.iter().position(|c| c.name == p.kind)?;
        let mut l = Live::new(kind, Vec3::from(p.pos), p.yaw);
        l.fuel = p.fuel;
        l.health = p.health;
        l.upgrades = p.upgrades.clone();
        l.cargo = p.cargo;
        Some(l)
    }

    fn new(kind: usize, pos: Vec3, yaw: f32) -> Live {
        let c = &CRAFTS[kind];
        Live {
            kind,
            body: Body::sized(pos, c.width, c.height),
            yaw,
            pitch: 0.0,
            roll: 0.0,
            speed: 0.0,
            throttle: 0.0,
            spin: 0.0,
            prop: 0.0,
            fuel: 0.0,
            health: c.health,
            upgrades: Vec::new(),
            cargo: 0,
            hurt: 1.0,
        }
    }
}

#[derive(Default)]
pub struct Aircraft {
    pub live: Vec<Live>,
    /// The aircraft the player flies.
    pub pilot: Option<usize>,
    /// Dimension whose aircraft are in `live`.
    loaded_dim: Option<u8>,
    engine_beat: f32,
    cannon_cd: f32,
    boost: f32,
    smoke_t: f32,
}

fn wrap(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    (a + std::f32::consts::PI).rem_euclid(t) - std::f32::consts::PI
}

impl Game {
    fn aircraft_held_name(&self) -> Option<String> {
        self.held_stack().map(|s| self.items.get(s.item).name.clone())
    }

    fn key_held(&self, k: KeyCode, a: Action) -> bool {
        self.keys.contains(&k) || self.pad_held.contains(&a)
    }

    /// Bring this dimension's aircraft in and put the last one's away.
    fn sync_aircraft_dim(&mut self) {
        let d = self.dim as u8;
        if self.mods.aircraft.loaded_dim == Some(d) {
            return;
        }
        let a = &mut self.mods.aircraft;
        if let Some(old) = a.loaded_dim {
            let gone: Vec<Plane> = a.live.drain(..).map(|l| l.to_plane(old)).collect();
            self.mods.save.aircraft.extend(gone);
        }
        let (mine, rest): (Vec<Plane>, Vec<Plane>) = std::mem::take(&mut self.mods.save.aircraft).into_iter().partition(|p| p.dim == d);
        self.mods.save.aircraft = rest;
        let a = &mut self.mods.aircraft;
        a.live = mine.iter().filter_map(Live::from_plane).collect();
        a.pilot = None;
        a.loaded_dim = Some(d);
    }

    /// Every saved aircraft, the loaded ones included (for `mods.ron`).
    pub(crate) fn aircraft_to_save(&self) -> Vec<Plane> {
        let mut out = self.mods.save.aircraft.clone();
        if let Some(d) = self.mods.aircraft.loaded_dim {
            out.extend(self.mods.aircraft.live.iter().map(|l| l.to_plane(d)));
        }
        out
    }

    /// The aircraft under the crosshair, nearer than the block it points at.
    fn aircraft_in_sight(&self) -> Option<usize> {
        let eye = self.body.eye();
        let f = self.forward();
        let block = self.target.map(|t| t.distance).unwrap_or(f32::MAX);
        let mut best = None;
        for (i, l) in self.mods.aircraft.live.iter().enumerate() {
            if self.mods.aircraft.pilot == Some(i) {
                continue;
            }
            let (lo, hi) = l.aabb();
            if let Some(t) = crate::mobs::ray_box(eye, f, lo, hi) {
                if t < self.reach() + 1.0 && t < block + 0.3 && best.is_none_or(|(_, bt)| t < bt) {
                    best = Some((i, t));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    fn take_held(&mut self, leave: Option<&str>) {
        if !self.survival {
            return;
        }
        let sel = self.selected;
        self.surv.inv.take(sel, 1);
        if let Some(l) = leave.and_then(|n| self.items.id(n)) {
            if self.surv.inv.slots[sel].is_none() {
                self.surv.inv.slots[sel] = Some(ItemStack::new(l, 1));
            } else if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(l, 1)) {
                let at = self.body.eye();
                self.spawn_drop(at, left, Vec3::ZERO);
            }
        }
    }

    /// Right click: board, refuel, upgrade, open a hold, place an aircraft or fire a booster.
    pub(crate) fn aircraft_use(&mut self) -> bool {
        self.sync_aircraft_dim();
        let held = self.aircraft_held_name();
        if let Some(i) = self.mods.aircraft.pilot {
            if held.as_deref() == Some("gunpowder") && self.mods.aircraft.boost <= 0.0 {
                self.mods.aircraft.boost = 3.0;
                self.take_held(None);
                self.sfx(Sfx::Explode);
                return true;
            }
            return held.is_some_and(|h| self.refuel(i, &h));
        }
        if let Some(i) = self.aircraft_in_sight() {
            if let Some(h) = held.as_deref() {
                if self.refuel(i, h) {
                    return true;
                }
                if UPGRADES.iter().any(|u| u.0 == h) {
                    let l = &mut self.mods.aircraft.live[i];
                    if l.upgrades.iter().any(|u| u == h) {
                        self.toast("Essa melhoria já está instalada", 2.0);
                    } else if l.upgrades.len() >= MAX_UPGRADES {
                        self.toast(format!("No máximo {MAX_UPGRADES} melhorias por aeronave"), 2.0);
                    } else {
                        let before = l.max_health();
                        l.upgrades.push(h.to_string());
                        l.health *= l.max_health() / before;
                        self.take_held(None);
                        self.sfx(Sfx::Place(crate::audio::Material::Metal));
                        self.toast(format!("Melhoria instalada: {}", self.item_display(h)), 2.0);
                    }
                    return true;
                }
            }
            if self.crouch > 0.5 && CRAFTS[self.mods.aircraft.live[i].kind].storage > 0 {
                self.open_hold(i);
                return true;
            }
            if self.riding.is_none() && self.mods.mount.ridden.is_none() {
                self.mods.aircraft.pilot = Some(i);
                let l = &self.mods.aircraft.live[i];
                let c = &CRAFTS[l.kind];
                let power = if c.fuel_use > 0.0 {
                    format!("combustível {:.0}%", l.fuel / MAX_FUEL * 100.0)
                } else {
                    "segure espaço para pedalar".to_string()
                };
                let seats = if c.seats > 1 { format!(", {} lugares", c.seats) } else { String::new() };
                self.toast(format!("{}{seats}: W/S acelera, espaço sobe, Ctrl desce, agache para sair ({power})", self.item_display(c.name)), 4.0);
                self.sfx(Sfx::Place(crate::audio::Material::Wood));
                return true;
            }
            return false;
        }
        // Place one from the hand on the block looked at.
        let Some(h) = held else { return false };
        let Some(kind) = CRAFTS.iter().position(|c| c.name == h) else { return false };
        let Some(hit) = self.target else { return false };
        let pos = hit.block.as_vec3() + Vec3::new(0.5, 1.0, 0.5);
        self.sync_aircraft_dim();
        let mut l = Live::new(kind, pos, self.yaw + std::f32::consts::PI);
        if !self.survival {
            l.fuel = MAX_FUEL;
        }
        self.mods.aircraft.live.push(l);
        self.take_held(None);
        self.anim.start_swing();
        self.sfx(Sfx::Place(crate::audio::Material::Wood));
        true
    }

    fn item_display(&self, name: &str) -> String {
        self.items.id(name).map(|i| self.items.get(i).display_name.clone()).unwrap_or_else(|| name.to_string())
    }

    fn refuel(&mut self, i: usize, held: &str) -> bool {
        let Some(&(_, secs, leave)) = FUELS.iter().find(|f| f.0 == held) else { return false };
        let l = &mut self.mods.aircraft.live[i];
        if CRAFTS[l.kind].fuel_use == 0.0 {
            self.toast("O girodino anda com as pernas do piloto: segure espaço para pedalar", 2.5);
            return true;
        }
        if l.fuel + secs * 0.5 > MAX_FUEL {
            self.toast("O tanque já está cheio", 1.5);
            return true;
        }
        l.fuel = (l.fuel + secs).min(MAX_FUEL);
        let pct = l.fuel / MAX_FUEL * 100.0;
        self.take_held(leave);
        self.sfx(Sfx::Place(crate::audio::Material::Metal));
        self.toast(format!("Combustível: {pct:.0}%"), 1.5);
        true
    }

    fn open_hold(&mut self, i: usize) {
        let n = CRAFTS[self.mods.aircraft.live[i].kind].storage;
        if self.mods.aircraft.live[i].cargo == 0 {
            let next = self.surv.containers.keys().filter(|p| p.x == CARGO_X).map(|p| p.z).max().unwrap_or(0) + 1;
            self.mods.aircraft.live[i].cargo = next;
        }
        let p = IVec3::new(CARGO_X, 0, self.mods.aircraft.live[i].cargo);
        self.surv.containers.entry(p).or_insert_with(|| Container::Chest(Inventory::new(n)));
        self.surv.open = Open::Chest(p);
        self.set_screen(crate::Screen::Inventory);
        self.sfx(Sfx::Place(crate::audio::Material::Wood));
    }

    /// Left click: the warship's cannon while flying it, or a hit on an aircraft (a few break
    /// it and give it back with its upgrades and cargo; one in creative).
    pub(crate) fn aircraft_attack(&mut self) -> bool {
        self.sync_aircraft_dim();
        if let Some(i) = self.mods.aircraft.pilot {
            let l = &self.mods.aircraft.live[i];
            if !CRAFTS[l.kind].cannon {
                return false;
            }
            if self.mods.aircraft.cannon_cd <= 0.0 {
                self.mods.aircraft.cannon_cd = 0.7;
                let muzzle = l.body.pos + l.heading() * 1.6 + Vec3::Y * 1.2;
                let f = self.forward();
                self.cannon_shot(muzzle + f * 0.5, f);
            }
            return true;
        }
        let Some(i) = self.aircraft_in_sight() else { return false };
        let l = &mut self.mods.aircraft.live[i];
        l.health -= if self.survival { 6.0 } else { 1000.0 };
        l.hurt = 0.0;
        self.anim.start_swing();
        self.sfx(Sfx::Hit(crate::audio::Material::Wood));
        if self.mods.aircraft.live[i].health <= 0.0 {
            self.wreck(i, true);
        }
        true
    }

    /// An aircraft is gone: its upgrades and cargo fall out, and the whole aircraft too when it
    /// was taken apart rather than crashed.
    fn wreck(&mut self, i: usize, whole: bool) {
        let l = self.mods.aircraft.live.remove(i);
        let a = &mut self.mods.aircraft;
        a.pilot = match a.pilot {
            Some(p) if p == i => None,
            Some(p) if p > i => Some(p - 1),
            p => p,
        };
        let at = l.body.pos + Vec3::Y * 0.5;
        let mut out: Vec<ItemStack> = Vec::new();
        if whole {
            out.extend(self.items.id(CRAFTS[l.kind].name).map(|it| ItemStack::new(it, 1)));
        } else {
            self.sfx(Sfx::Explode);
            for n in ["aircraft_hull", "aircraft_propeller"] {
                out.extend(self.items.id(n).map(|it| ItemStack::new(it, 1)));
            }
        }
        out.extend(l.upgrades.iter().filter_map(|u| self.items.id(u)).map(|it| ItemStack::new(it, 1)));
        if l.cargo != 0 {
            if let Some(Container::Chest(inv)) = self.surv.containers.remove(&IVec3::new(CARGO_X, 0, l.cargo)) {
                out.extend(inv.slots.into_iter().flatten());
            }
        }
        if self.survival || !whole {
            for st in out {
                let v = Vec3::new(self.surv.rng.f32() - 0.5, 1.0, self.surv.rng.f32() - 0.5) * 3.0;
                self.spawn_drop(at, st, v);
            }
        }
    }

    /// Climb out beside the aircraft (or on top of it when walled in).
    fn leave_aircraft(&mut self) {
        let Some(i) = self.mods.aircraft.pilot.take() else { return };
        let l = &self.mods.aircraft.live[i];
        let c = l.body.pos;
        let w = CRAFTS[l.kind].width * 0.5 + 0.6;
        let right = l.heading().cross(Vec3::Y);
        for d in [right, -right, -l.heading(), l.heading()] {
            let mut b = self.body.clone();
            b.pos = c + d * w + Vec3::Y * 0.1;
            if b.fits(&self.world, voxel::physics::PLAYER_HEIGHT) {
                self.body.pos = b.pos;
                self.body.vel = l.body.vel;
                self.fall_peak = self.body.pos.y;
                return;
            }
        }
        self.body.pos = c + Vec3::Y * (CRAFTS[l.kind].height + 0.1);
        self.fall_peak = self.body.pos.y;
    }

    /// The player flying: their inputs drive the aircraft and they ride along in its seat.
    /// True while flying.
    pub(crate) fn pilot_step(&mut self, wish: Vec3, sneak: bool, dt: f32) -> bool {
        let Some(i) = self.mods.aircraft.pilot else { return false };
        if i >= self.mods.aircraft.live.len() {
            self.mods.aircraft.pilot = None;
            return false;
        }
        if sneak {
            self.leave_aircraft();
            return false;
        }
        let fwd = self.forward();
        let flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let side_dir = flat.cross(Vec3::Y);
        let ahead = wish.dot(flat);
        let side = wish.dot(side_dir);
        let up = self.key_held(KeyCode::Space, Action::Jump);
        let down = self.key_held(KeyCode::ControlLeft, Action::Sprint) || self.keys.contains(&KeyCode::ControlRight);
        let (cam_yaw, cam_pitch) = (self.yaw + std::f32::consts::PI, self.pitch);
        let boost = self.mods.aircraft.boost > 0.0;
        let survival = self.survival;
        let mut l = std::mem::replace(&mut self.mods.aircraft.live[i], Live::new(0, Vec3::ZERO, 0.0));
        let c = &CRAFTS[l.kind];
        let (speed_mul, fuel_mul, _) = l.factors();
        // Steering: the nose follows the view.
        let d = wrap(cam_yaw - l.yaw);
        let turn = c.turn * dt * if c.stall > 0.0 && l.body.on_ground { 0.6 } else { 1.0 };
        l.yaw = wrap(l.yaw + d.clamp(-turn, turn));
        let bank = if c.stall > 0.0 { -d.clamp(-0.7, 0.7) } else { -side * 0.2 - d.clamp(-0.4, 0.4) * 0.3 };
        l.roll += (bank - l.roll) * (dt * 3.0).min(1.0);
        // Throttle.
        let lo = if c.stall > 0.0 { 0.0 } else { -0.5 };
        if ahead > 0.3 {
            l.throttle = (l.throttle + dt * 0.8).min(1.0);
        } else if ahead < -0.3 {
            l.throttle = (l.throttle - dt * 0.8).max(lo);
        } else if c.stall == 0.0 {
            l.throttle *= (-dt * 2.0).exp();
        }
        // Power: fuel, or the pilot pedalling.
        if c.fuel_use > 0.0 {
            let running = l.fuel > 0.0 || !survival;
            l.spin += ((if running { 1.0 } else { 0.0 }) - l.spin) * (dt * 0.8).min(1.0);
            if running && survival {
                l.fuel = (l.fuel - c.fuel_use * fuel_mul * l.throttle.abs().max(0.15) * dt).max(0.0);
            }
        } else if up {
            l.spin = (l.spin + dt * 0.45).min(1.0);
            if survival {
                self.surv.exhaustion += 0.25 * dt;
            }
        } else {
            l.spin = (l.spin - dt * if l.body.on_ground { 0.3 } else { 0.06 }).max(0.0);
        }
        let max = c.max_speed * speed_mul * if boost { 1.7 } else { 1.0 };
        let heading = l.heading();
        if c.stall > 0.0 {
            // Fixed wings: speed along the nose, lift from airspeed.
            let target = l.throttle * l.spin * max;
            let a = c.accel * if boost { 2.5 } else { 1.0 } * dt;
            l.speed += (target - l.speed).clamp(-a, a);
            if l.body.on_ground && l.throttle < 0.05 {
                l.speed *= (-dt * 1.5).exp();
            }
            let lift = (l.speed / c.stall).min(1.0);
            let want_pitch = if lift >= 1.0 { cam_pitch.clamp(-0.7, 0.6) } else if l.body.on_ground { 0.0 } else { -0.25 };
            l.pitch += (want_pitch - l.pitch) * (dt * 2.0).min(1.0);
            if l.body.on_ground && l.pitch < 0.0 {
                l.pitch = 0.0;
            }
            let dir = heading * l.pitch.cos() + Vec3::Y * l.pitch.sin();
            let sink = (1.0 - lift) * 9.0;
            let vy = dir.y * l.speed - sink;
            l.body.vel = Vec3::new(dir.x * l.speed, l.body.vel.y + (vy - l.body.vel.y) * (dt * 4.0).min(1.0), dir.z * l.speed);
        } else {
            let lifting = l.spin > 0.35;
            let mut want = heading * l.throttle * max * l.spin;
            if c.strafe {
                want += heading.cross(Vec3::Y) * side * max * 0.7 * l.spin;
            }
            let k = (c.accel / max.max(1.0) * dt * if boost { 2.5 } else { 1.0 }).min(1.0);
            l.body.vel.x += (want.x - l.body.vel.x) * k;
            l.body.vel.z += (want.z - l.body.vel.z) * k;
            if lifting {
                let climb = c.climb * l.spin;
                let vy = if up && c.fuel_use > 0.0 { climb } else if up { climb * ((l.spin - 0.35) / 0.65) } else if down { -climb } else { 0.0 };
                l.body.vel.y += (vy - l.body.vel.y) * (dt * 3.0).min(1.0);
            } else if c.name.contains("airship") || c.name == "warship" {
                // The balloon lets it down gently.
                l.body.vel.y += (-1.5 - l.body.vel.y) * (dt * 2.0).min(1.0);
            } else {
                l.body.vel.y = (l.body.vel.y - 20.0 * dt).max(-30.0);
            }
            if l.body.on_ground && !lifting {
                l.body.vel.x *= (-dt * 4.0).exp();
                l.body.vel.z *= (-dt * 4.0).exp();
            }
            l.pitch += (-l.throttle * 0.12 - l.pitch) * (dt * 3.0).min(1.0);
        }
        let crash = self.move_aircraft(&mut l, dt);
        l.prop += dt * (4.0 + 40.0 * l.spin * (0.3 + l.throttle.abs()));
        let seat = Mat4::from_rotation_y(l.yaw).transform_vector3(c.seat);
        self.body.pos = l.body.pos + seat;
        self.body.vel = l.body.vel;
        self.body.on_ground = true;
        self.body.fall_distance = 0.0;
        self.fall_peak = self.body.pos.y;
        let (sound, spin, throttle) = (c.sound, l.spin, l.throttle);
        self.mods.aircraft.live[i] = l;
        // Engine noise and a puff of smoke now and then.
        let burns = CRAFTS[self.mods.aircraft.live[i].kind].fuel_use > 0.0;
        let a = &mut self.mods.aircraft;
        a.engine_beat -= dt;
        if spin > 0.2 && a.engine_beat <= 0.0 {
            a.engine_beat = 0.45;
            self.sfx(Sfx::Engine(sound, (throttle.abs() * 4.0).round() as u8));
        }
        let a = &mut self.mods.aircraft;
        a.smoke_t -= dt * spin * (0.5 + throttle.abs());
        if a.smoke_t <= 0.0 && spin > 0.2 && burns {
            a.smoke_t = 0.12;
            let l = &self.mods.aircraft.live[i];
            let at = l.body.pos + Vec3::Y * 0.6 - l.heading() * 0.8;
            let layer = self.glow_layer();
            self.particles.mote(at, layer, [0.35, 0.35, 0.38], 0.12, -0.6, 1.4);
        }
        if crash > 0.0 {
            self.crash(i, crash);
        }
        true
    }

    /// Move an aircraft by its velocity in short steps. Returns how hard it hit something.
    fn move_aircraft(&self, l: &mut Live, dt: f32) -> f32 {
        let before = l.body.vel;
        let steps = ((before.length() * dt / 0.3).ceil() as usize).clamp(1, 10);
        let h = dt / steps as f32;
        for _ in 0..steps {
            l.body.travel(&self.world, h);
        }
        if l.body.on_ground && l.speed > 0.0 {
            // Wheels on the ground: the nose can't push into it.
            l.pitch = l.pitch.max(0.0);
        }
        let lost = (before - l.body.vel).length();
        if CRAFTS[l.kind].stall > 0.0 {
            let ahead = l.body.vel.dot(l.heading()).max(0.0);
            if ahead + 1.0 < l.speed {
                l.speed = ahead;
            }
        }
        if lost > 11.0 {
            lost - 11.0
        } else {
            0.0
        }
    }

    /// A hard hit: the aircraft takes damage and the pilot a share of it.
    fn crash(&mut self, i: usize, hard: f32) {
        let l = &mut self.mods.aircraft.live[i];
        l.health -= hard * 3.0;
        l.hurt = 0.0;
        let broken = l.health <= 0.0;
        self.sfx(Sfx::Hit(crate::audio::Material::Wood));
        if self.mods.aircraft.pilot == Some(i) {
            self.damage((hard * 0.8).floor(), None);
        }
        if broken {
            if self.mods.aircraft.pilot == Some(i) {
                self.leave_aircraft();
            }
            self.wreck(i, false);
            self.toast("A aeronave se despedaçou", 2.5);
        }
    }

    /// Aircraft nobody flies: they settle, the airships drift down on their balloons.
    pub(crate) fn update_aircraft(&mut self, dt: f32) {
        self.sync_aircraft_dim();
        let a = &mut self.mods.aircraft;
        a.cannon_cd -= dt;
        a.boost -= dt;
        if a.live.is_empty() {
            return;
        }
        let pilot = a.pilot;
        let mut live = std::mem::take(&mut self.mods.aircraft.live);
        for (i, l) in live.iter_mut().enumerate() {
            l.hurt += dt;
            if pilot == Some(i) || self.world.block(l.body.pos.floor().as_ivec3()).is_none() {
                continue;
            }
            l.spin = (l.spin - dt * 0.5).max(0.0);
            l.prop += dt * 30.0 * l.spin;
            l.throttle = 0.0;
            l.speed *= (-dt).exp();
            l.pitch *= (-dt * 2.0).exp();
            l.roll *= (-dt * 2.0).exp();
            let c = &CRAFTS[l.kind];
            let floaty = c.name.contains("airship") || c.name == "warship";
            l.body.vel.y = if floaty { (l.body.vel.y - 4.0 * dt).max(-1.5) } else { (l.body.vel.y - 20.0 * dt).max(-30.0) };
            if l.body.on_ground {
                l.body.vel.x *= (-dt * 5.0).exp();
                l.body.vel.z *= (-dt * 5.0).exp();
            }
            if l.body.vel.length_squared() > 1e-4 || !l.body.on_ground {
                self.move_aircraft(l, dt);
            }
        }
        self.mods.aircraft.live = live;
    }

    /// Fuel, throttle, speed and condition while flying.
    pub(crate) fn draw_aircraft_hud(&mut self, w: f32, h: f32) {
        let Some(i) = self.mods.aircraft.pilot else { return };
        let Some(l) = self.mods.aircraft.live.get(i) else { return };
        let c = &CRAFTS[l.kind];
        let s = self.ui.scale;
        let name = self.item_display(c.name);
        let speed = l.body.vel.length() * 3.6;
        let lines = [
            name,
            format!("Velocidade: {speed:.0} km/h   Altitude: {:.0}", l.body.pos.y),
            format!("Acelerador: {:.0}%{}", l.throttle * 100.0, if self.mods.aircraft.boost > 0.0 { "  [FOGUETE]" } else { "" }),
        ];
        let (bw, bh) = (170.0 * s, 62.0 * s);
        let (x, y) = (w - bw - 6.0 * s, h - bh - 50.0 * s);
        self.ui.rect(x, y, bw, bh, [0, 0, 0, 140]);
        for (k, t) in lines.iter().enumerate() {
            self.ui.text(x + 5.0 * s, y + (4.0 + 10.0 * k as f32) * s, t, if k == 0 { [240, 210, 120, 255] } else { [220, 220, 220, 255] });
        }
        // Bars: fuel (or pedal power) and the airframe's health.
        let (power, label, col) = if c.fuel_use > 0.0 { (l.fuel / MAX_FUEL, "Combustível", [220, 150, 50, 255]) } else { (l.spin, "Pedal", [120, 200, 110, 255]) };
        let hp = (l.health / l.max_health()).clamp(0.0, 1.0);
        for (k, (v, t, col)) in [(power, label, col), (hp, "Estrutura", [200, 70, 60, 255])].into_iter().enumerate() {
            let yy = y + (36.0 + 12.0 * k as f32) * s;
            self.ui.text(x + 5.0 * s, yy, t, [200, 200, 200, 255]);
            let bx = x + 62.0 * s;
            self.ui.rect(bx, yy + 1.0 * s, 100.0 * s, 7.0 * s, [40, 40, 40, 200]);
            self.ui.rect(bx, yy + 1.0 * s, 100.0 * s * v.clamp(0.0, 1.0), 7.0 * s, col);
        }
    }

    /// Models for the entity renderer: propellers and rotors spin, the warship's gun follows
    /// the pilot's aim.
    pub(crate) fn aircraft_instances(&self, cam_block: IVec3) -> Vec<(usize, Bones)> {
        let a = &self.mods.aircraft;
        let mut out = Vec::new();
        for (i, l) in a.live.iter().enumerate() {
            let Some(id) = self.mobs.reg.id(CRAFTS[l.kind].name) else { continue };
            let def = &self.mobs.reg.defs[id];
            let li = self.world.light((l.body.pos + Vec3::Y * 0.5).floor().as_ivec3());
            let mut b = Bones { light: [(li >> 4) as f32 / 15.0, (li & 15) as f32 / 15.0, if l.hurt < 0.25 { 1.0 } else { 0.0 }, 0.0], ..Default::default() };
            let base = Mat4::from_translation(l.body.pos - cam_block.as_vec3())
                * Mat4::from_rotation_y(l.yaw)
                * Mat4::from_rotation_x(-l.pitch)
                * Mat4::from_rotation_z(l.roll)
                * Mat4::from_scale(Vec3::splat(def.scale / 16.0));
            let (aim_yaw, aim_pitch) = if a.pilot == Some(i) { (wrap(self.yaw + std::f32::consts::PI - l.yaw), self.pitch) } else { (0.0, 0.0) };
            for (k, p) in def.parts.iter().enumerate().take(b.m.len()) {
                let pivot = Vec3::new(p.pivot.0 as f32, p.pivot.1 as f32, p.pivot.2 as f32);
                let turn = match p.anim {
                    Anim::Wing(0) => Mat4::from_rotation_z(l.prop),
                    Anim::Wing(_) => Mat4::from_rotation_y(l.prop),
                    Anim::Head => Mat4::from_rotation_y(aim_yaw) * Mat4::from_rotation_x(-aim_pitch.clamp(-0.5, 0.6)),
                    _ => Mat4::IDENTITY,
                };
                let m = base * Mat4::from_translation(pivot) * turn * Mat4::from_translation(-pivot);
                b.m[k] = m.to_cols_array_2d();
            }
            out.push((id, b));
        }
        out
    }

    /// MC_AIRCRAFT=name puts that aircraft, fuelled, in front of the player (test pictures).
    pub(crate) fn aircraft_test_hook(&mut self) {
        let Ok(name) = std::env::var("MC_AIRCRAFT") else { return };
        self.sync_aircraft_dim();
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let names: Vec<&str> = if name == "all" { CRAFTS.iter().map(|c| c.name).collect() } else { vec![name.as_str()] };
        let right = flat.cross(Vec3::Y);
        for (k, n) in names.iter().enumerate() {
            let Some(kind) = CRAFTS.iter().position(|c| c.name == *n) else { continue };
            let off = (k as f32 - (names.len() as f32 - 1.0) * 0.5) * 7.0;
            let mut at = self.body.pos + flat * (14.0 + (k % 2) as f32 * 6.0) + right * off;
            at.y = self.ground_height(at) + 0.01;
            let mut l = Live::new(kind, at, self.yaw + std::f32::consts::PI * 0.75);
            l.fuel = MAX_FUEL;
            l.spin = 1.0;
            l.prop = k as f32;
            self.mods.aircraft.live.push(l);
        }
    }

    fn ground_height(&self, at: Vec3) -> f32 {
        let mut p = at.floor().as_ivec3() + IVec3::Y * 8;
        for _ in 0..40 {
            if self.world.solid(p - IVec3::Y) {
                return p.y as f32;
            }
            p.y -= 1;
        }
        at.y
    }
}
