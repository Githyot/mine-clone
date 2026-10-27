//! Lote 8 of the Terraria side (plan/terraria-completo.md): invasions in waves.
//! - Harvest Moon (Lua da Colheita, pumpkin medallion at night): scarecrows, headless riders,
//!   and the Pumpkin King from the fourth wave.
//! - Winter Moon (Lua do Inverno, frost medallion at night): evil elves, snowmen with guns and
//!   the Ice Queen.
//! - Sea raiders (Saqueadores do mar, treasure map): deckhands, gunners and their captain.
//! - Star visitors (Visitantes das estrelas): a probe that is not shot down in time calls
//!   soldiers, officers and a flying saucer.
//! - Crystal Defense (Defesa do Cristal, ether crystal): keep the crystal alive through five
//!   waves of ether goblins and wyverns and the Dark Mage.
//! Kills fill each wave; the moons end at dawn, the defense ends if the crystal breaks.

use glam::Vec3;
use voxel::item::ItemStack;
use voxel::worldgen::Dim;

use crate::audio::Sfx;
use crate::Game;

struct EvDef {
    key: &'static str,
    title: &'static str,
    item: &'static str,
    waves: u8,
    /// (mob, first wave it comes in)
    mobs: &'static [(&'static str, u8)],
    boss: &'static str,
    boss_wave: u8,
    /// What the winner gets at the end: (item, count, chance).
    prize: &'static [(&'static str, u8, f32)],
    night: bool,
}

const EVENTS: &[EvDef] = &[
    EvDef {
        key: "harvest",
        title: "Lua da Colheita",
        item: "pumpkin_medallion",
        waves: 6,
        mobs: &[("scarecrow", 1), ("headless_rider", 3)],
        boss: "pumpkin_king",
        boss_wave: 4,
        prize: &[("horseman_blade", 1, 0.6), ("pumpkin", 8, 1.0), ("emerald", 10, 1.0), ("soul_of_night", 6, 1.0)],
        night: true,
    },
    EvDef {
        key: "winter",
        title: "Lua do Inverno",
        item: "frost_medallion",
        waves: 6,
        mobs: &[("evil_elf", 1), ("snowman_gunner", 2)],
        boss: "ice_queen",
        boss_wave: 4,
        prize: &[("blizzard_staff", 1, 0.6), ("frost_shard", 8, 1.0), ("emerald", 10, 1.0), ("soul_of_light", 6, 1.0)],
        night: true,
    },
    EvDef {
        key: "pirates",
        title: "Saqueadores do Mar",
        item: "pirate_map",
        waves: 3,
        mobs: &[("pirate_deckhand", 1), ("pirate_gunner", 2)],
        boss: "pirate_captain",
        boss_wave: 3,
        prize: &[("cutlass", 1, 0.7), ("gold_ingot", 10, 1.0), ("lucky_coin", 1, 0.25), ("emerald", 12, 1.0)],
        night: false,
    },
    EvDef {
        key: "martians",
        title: "Visitantes das Estrelas",
        item: "",
        waves: 4,
        mobs: &[("martian_soldier", 1), ("martian_officer", 2)],
        boss: "saucer",
        boss_wave: 4,
        prize: &[("laser_rifle", 1, 0.7), ("lumite_ingot", 4, 0.5), ("emerald", 15, 1.0)],
        night: false,
    },
    EvDef {
        key: "crystal",
        title: "Defesa do Cristal",
        item: "ether_crystal",
        waves: 5,
        mobs: &[("ether_goblin", 1), ("ether_wyvern", 2)],
        boss: "dark_mage",
        boss_wave: 5,
        prize: &[("flame_sentry_rod", 1, 0.8), ("ballista_rod", 1, 0.3), ("emerald", 12, 1.0)],
        night: false,
    },
];

/// Folk that shoot at you from afar: (mob, school of the bolt, seconds between shots).
const SHOOTERS: &[(&str, u8, f32)] = &[
    ("snowman_gunner", 1, 2.0),
    ("pirate_gunner", 0, 2.2),
    ("martian_soldier", 2, 1.8),
    ("martian_officer", 2, 1.4),
    ("ice_queen", 1, 0.7),
    ("saucer", 2, 0.5),
    ("dark_mage", 3, 1.0),
    ("pumpkin_king", 0, 1.2),
    ("pirate_captain", 0, 1.5),
];

pub fn own_ai(name: &str) -> bool {
    matches!(name, "pumpkin_king" | "ice_queen" | "pirate_captain" | "saucer" | "dark_mage")
}

pub struct Invasion {
    ev: usize,
    wave: u8,
    points: u32,
    spawn_t: f32,
    boss_out: bool,
    /// Where it happens (the crystal, for the defense).
    center: Vec3,
    shot_t: f32,
}

#[derive(Default)]
pub struct Invasions {
    pub(crate) now: Option<Invasion>,
    probe_t: f32,
    /// Seconds before an unharmed probe flies off to call the invasion.
    probe_left: Option<f32>,
}

fn need(wave: u8) -> u32 {
    10 + wave as u32 * 8
}

impl Game {
    pub(crate) fn invasion_use(&mut self, item: &str) -> Option<bool> {
        let k = EVENTS.iter().position(|e| !e.item.is_empty() && e.item == item)?;
        let e = &EVENTS[k];
        if self.dim != Dim::Overworld || self.t5.tr.inv.now.is_some() {
            self.toast("Agora não dá para começar isso", 2.5);
            return Some(true);
        }
        if e.night && !self.is_night() {
            self.toast("Isso só funciona à noite", 2.5);
            return Some(true);
        }
        if !self.t5.hard.on {
            self.toast("Só no modo difícil", 2.5);
            return Some(true);
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.start_wave_event(k);
        Some(true)
    }

    fn start_wave_event(&mut self, k: usize) {
        let me = self.body.pos;
        let mut center = me;
        if EVENTS[k].key == "crystal" {
            let f = self.forward();
            center = me + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 3.0 + Vec3::Y * 0.5;
            if self.mobs.spawn("ether_crystal_mob", center) {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.body.flying = true;
                }
            }
        }
        self.t5.tr.inv.now = Some(Invasion { ev: k, wave: 1, points: 0, spawn_t: 2.0, boss_out: false, center, shot_t: 0.0 });
        self.toast(format!("{} começou! Onda 1", EVENTS[k].title), 4.0);
        self.weather.flash = 0.5;
        self.sfx(Sfx::Thunder(0.5));
    }

    fn end_wave_event(&mut self, won: bool) {
        let Some(inv) = self.t5.tr.inv.now.take() else { return };
        let e = &EVENTS[inv.ev];
        // Its folk leave with it.
        let names: Vec<&str> = e.mobs.iter().map(|m| m.0).chain([e.boss, "ether_crystal_mob"]).collect();
        let reg = &self.mobs.reg;
        self.mobs.list.retain(|m| !names.contains(&reg.defs[m.def].name.as_str()));
        if !won {
            self.toast(format!("{} acabou.", e.title), 4.0);
            return;
        }
        self.toast(format!("Você venceu: {}!", e.title), 5.0);
        self.sfx(Sfx::Xp);
        let at = self.body.pos + Vec3::Y;
        for &(item, n, chance) in e.prize {
            if self.surv.rng.f32() >= chance {
                continue;
            }
            if let Some(id) = self.items.id(item) {
                self.spawn_drop(at, ItemStack::new(id, n), Vec3::Y * 3.0);
            }
        }
    }

    pub(crate) fn update_invasions(&mut self, dt: f32) {
        if dt == 0.0 || self.dim != Dim::Overworld {
            return;
        }
        self.martian_probe(dt);
        let Some(inv) = self.t5.tr.inv.now.as_mut() else { return };
        let e = &EVENTS[inv.ev];
        inv.spawn_t -= dt;
        inv.shot_t -= dt;
        let spawn = inv.spawn_t <= 0.0;
        if spawn {
            inv.spawn_t = 1.6;
        }
        let shoot_tick = inv.shot_t <= 0.0;
        if shoot_tick {
            inv.shot_t = 0.1;
        }
        let (wave, boss_out, center, key) = (inv.wave, inv.boss_out, inv.center, e.key);
        // The moons end at dawn.
        if e.night && !self.is_night() {
            return self.end_wave_event(false);
        }
        if key == "crystal" {
            let alive = self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].name == "ether_crystal_mob");
            if !alive {
                self.toast("O Cristal Etéreo se partiu!", 4.0);
                return self.end_wave_event(false);
            }
        }
        let names: Vec<&str> = e.mobs.iter().filter(|m| m.1 <= wave).map(|m| m.0).collect();
        let alive = self.mobs.list.iter().filter(|m| m.alive() && (names.contains(&self.mobs.reg.defs[m.def].name.as_str()))).count();
        if spawn && alive < 10 && !names.is_empty() {
            let name = names[(self.surv.rng.f32() * names.len() as f32) as usize % names.len()];
            let flies = name == "ether_wyvern";
            let around = if key == "crystal" { center } else { self.body.pos };
            self.spawn_invader(name, around, flies);
        }
        if wave >= e.boss_wave && !boss_out {
            let flies = matches!(e.boss, "ice_queen" | "saucer");
            let around = if key == "crystal" { center } else { self.body.pos };
            if self.spawn_invader(e.boss, around, flies) {
                let dn = self.mobs.list.last().map(|m| self.mobs.reg.defs[m.def].display_name.clone()).unwrap_or_default();
                self.toast(format!("{dn} chegou!"), 3.0);
                self.sfx(Sfx::Thunder(0.4));
            }
            if let Some(inv) = self.t5.tr.inv.now.as_mut() {
                inv.boss_out = true;
            }
        }
        self.steer_invaders(dt, shoot_tick, key, center);
    }

    fn spawn_invader(&mut self, name: &str, around: Vec3, flies: bool) -> bool {
        let a = self.surv.rng.f32() * std::f32::consts::TAU;
        let r = 16.0 + self.surv.rng.f32() * 8.0;
        let x = around.x + a.cos() * r;
        let z = around.z + a.sin() * r;
        let y = self.world.surface_y(x.floor() as i32, z.floor() as i32).map_or(around.y, |y| y as f32 + 1.0) + if flies { 6.0 } else { 0.0 };
        if !self.mobs.spawn(name, Vec3::new(x, y, z)) {
            return false;
        }
        if let Some(m) = self.mobs.list.last_mut() {
            m.aggro = true;
        }
        true
    }

    fn steer_invaders(&mut self, dt: f32, shoot_tick: bool, key: &str, center: Vec3) {
        let me = self.body.eye();
        let now = self.now;
        let mut shots = Vec::new();
        for (k, m) in self.mobs.list.iter_mut().enumerate() {
            if !m.alive() {
                continue;
            }
            let name = self.mobs.reg.defs[m.def].name.as_str();
            let pos = m.body.pos;
            match name {
                "ice_queen" | "saucer" => {
                    // They hover over you, circling.
                    let a = now * 0.6 + k as f32;
                    let spot = me + Vec3::new(a.cos() * 8.0, 6.0, a.sin() * 8.0);
                    m.body.pos += (spot - pos) * (1.5 * dt).min(1.0);
                    m.body.vel = Vec3::ZERO;
                    m.body.flying = true;
                }
                "ether_crystal_mob" => {
                    m.body.pos = center;
                    m.body.vel = Vec3::ZERO;
                    m.body.flying = true;
                    continue;
                }
                _ => {}
            }
            // In the defense the goblins go for the crystal unless you are in the way.
            if key == "crystal" && matches!(name, "ether_goblin" | "dark_mage") && pos.distance(me) > 5.0 {
                let to = Vec3::new(center.x - pos.x, 0.0, center.z - pos.z);
                if to.length() > 1.6 {
                    let v = to.normalize() * 3.0;
                    m.body.vel.x = v.x;
                    m.body.vel.z = v.z;
                    m.yaw = v.x.atan2(v.z);
                }
            }
            if shoot_tick {
                if let Some(&(_, school, every)) = SHOOTERS.iter().find(|s| s.0 == name) {
                    // A cheap per-mob clock: roughly one shot each `every` seconds.
                    if pos.distance(me) < 32.0 && (now / every + k as f32 * 0.37).fract() < 0.1 / every {
                        shots.push((pos + Vec3::Y * 1.5, school, self.mobs.reg.defs[m.def].attack as f32));
                    }
                }
            }
        }
        for (from, school, dmg) in shots {
            let dir = (me - from).normalize_or_zero();
            self.enemy_bolt(from + dir, dir * 22.0, school, dmg * 0.7);
        }
        // Goblins at the crystal wear it down.
        if key == "crystal" {
            let hits = self.mobs.list.iter().filter(|m| m.alive() && matches!(self.mobs.reg.defs[m.def].name.as_str(), "ether_goblin" | "dark_mage") && m.body.pos.distance(center) < 2.5).count();
            if hits > 0 {
                if let Some(c) = self.mobs.list.iter_mut().find(|m| m.alive() && self.mobs.reg.defs[m.def].name == "ether_crystal_mob") {
                    c.health -= hits as f32 * 6.0 * dt;
                }
            }
        }
    }

    /// Deaths: each invader fills the wave.
    pub(crate) fn invasion_deaths(&mut self, deaths: &[(usize, Vec3)]) {
        let Some(inv) = self.t5.tr.inv.now.as_mut() else { return };
        let e = &EVENTS[inv.ev];
        let mut gained = 0;
        for &(def, _) in deaths {
            let n = self.mobs.reg.defs[def].name.as_str();
            if n == e.boss {
                gained += 10;
                inv.boss_out = false;
            } else if e.mobs.iter().any(|m| m.0 == n) {
                gained += 1;
            }
        }
        if gained == 0 {
            return;
        }
        inv.points += gained;
        if inv.points < need(inv.wave) {
            return;
        }
        if inv.wave >= e.waves {
            return self.end_wave_event(true);
        }
        inv.wave += 1;
        inv.points = 0;
        let (title, wave) = (e.title, inv.wave);
        self.toast(format!("{title}: onda {wave}"), 3.0);
        self.sfx(Sfx::Thunder(0.3));
    }

    /// The star visitors' probe: after the Scale Idol, now and then over the surface.
    fn martian_probe(&mut self, dt: f32) {
        let probe = self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].name == "martian_probe");
        if let Some(i) = probe {
            // It hovers and watches you; left alone it flies off to tell the others.
            let me = self.body.eye();
            let a = self.now * 0.8;
            let spot = me + Vec3::new(a.cos() * 10.0, 5.0, a.sin() * 10.0);
            let m = &mut self.mobs.list[i];
            m.body.pos += (spot - m.body.pos) * (1.2 * dt).min(1.0);
            m.body.vel = Vec3::ZERO;
            m.body.flying = true;
            let look = me - m.body.pos;
            m.yaw = look.x.atan2(look.z);
            if let Some(t) = self.t5.tr.inv.probe_left.as_mut() {
                *t -= dt;
                if *t <= 0.0 {
                    self.t5.tr.inv.probe_left = None;
                    self.mobs.list.remove(i);
                    self.toast("A sonda fugiu... os Visitantes das Estrelas estão chegando!", 4.0);
                    if self.t5.tr.inv.now.is_none() {
                        self.start_wave_event(EVENTS.iter().position(|e| e.key == "martians").unwrap());
                    }
                }
            }
            return;
        }
        self.t5.tr.inv.probe_left = None;
        self.t5.tr.inv.probe_t -= dt;
        if self.t5.tr.inv.probe_t > 0.0 {
            return;
        }
        self.t5.tr.inv.probe_t = 120.0;
        let ready = self.t5.hard.on && self.boss.defeated.iter().any(|b| b == "scale_idol") && self.body.pos.y > 50.0 && self.survival && !self.mobs.peaceful;
        if ready && self.t5.tr.inv.now.is_none() && self.surv.rng.f32() < 0.15 && self.spawn_around("martian_probe", true) {
            self.t5.tr.inv.probe_left = Some(20.0);
            self.toast("Uma sonda estranha está te observando. Derrube-a antes que fuja!", 4.0);
        }
    }

    /// The HUD line of a running invasion: (title, wave, waves, progress 0..1).
    pub(crate) fn invasion_hud(&self) -> Option<(String, f32)> {
        let inv = self.t5.tr.inv.now.as_ref()?;
        let e = &EVENTS[inv.ev];
        Some((format!("{}: onda {}/{}", e.title, inv.wave, e.waves), inv.points as f32 / need(inv.wave) as f32))
    }

    /// Test scenes: MC_TR=harvest, winter, pirates, martians, crystal (wave 4, boss out).
    pub(crate) fn test_invasion(&mut self, scene: &str) -> bool {
        let Some(k) = EVENTS.iter().position(|e| e.key == scene) else { return false };
        self.start_wave_event(k);
        let e = &EVENTS[k];
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = flat.cross(Vec3::Y);
        let eye = self.body.eye();
        let mut list: Vec<&str> = e.mobs.iter().flat_map(|m| [m.0, m.0]).collect();
        list.insert(list.len() / 2, e.boss);
        let n = list.len() as f32;
        for (k, name) in list.iter().enumerate() {
            let boss = *name == e.boss;
            let p = self.body.pos + flat * if boss { 13.0 } else { 8.0 } + side * (k as f32 - (n - 1.0) / 2.0) * 2.2;
            let flies = matches!(*name, "ice_queen" | "saucer" | "ether_wyvern");
            let y = self.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map_or(p.y, |y| y as f32 + 1.0) + if flies { 3.0 } else { 0.0 };
            self.mobs.spawn(name, Vec3::new(p.x, y, p.z));
            if let Some(m) = self.mobs.list.last_mut() {
                let look = eye - m.body.center();
                m.yaw = look.x.atan2(look.z);
                m.body.flying = flies;
            }
        }
        if let Some(inv) = self.t5.tr.inv.now.as_mut() {
            inv.wave = e.boss_wave;
            inv.boss_out = true;
            inv.points = need(inv.wave) / 2;
        }
        true
    }
}
