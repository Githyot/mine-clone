//! L18 of the Terraria side's third round (plan/terraria-completo.md, "Reauditoria 3"): the town
//! folk, mini-bosses, event bosses and rare monsters that were still missing.
//!
//! - Town folk: the Tavernkeep (after the first boss), three town jellies, the Skeleton Merchant
//!   who wanders the caverns for a few minutes and the Old Man at the dungeon, who at night lets
//!   you break the curse (he turns into the Bone King, as in Terraria).
//! - The Dungeon Guardian comes for whoever stays too long in the cursed dungeon: it flies
//!   through the walls and hits for a lot. The Paladin walks the dungeon in hardmode once the
//!   Vine Queen is gone, throwing his hammer.
//! - The Wyvern: a long sky serpent high above the clouds in hardmode. Its links share one
//!   health; it weaves through the air in long curves, so in 3D you see it coming and dodge
//!   (the links are steered in tr3_foes.rs, with the tomb crawler's).
//! - Event bosses join the wave events of tr_invasions.rs: Mourning Wood (harvest moon),
//!   Everscream and the Santa tank (winter moon), the Flying Dutchman (pirates), the Ogre and
//!   Betsy (crystal defense).
//! - Rare monsters: Tim (a teleporting caster), Doctor Bones (jungle nights), the Undead Miner
//!   (caverns) and Pinky (a tiny jelly worth a gold coin).

use glam::Vec3;
use voxel::worldgen::deep::Layer;

use crate::audio::Sfx;
use crate::Game;

/// (event key, mob, first wave, most at once)
const EVENT_MINIS: &[(&str, &str, u8, usize)] = &[
    ("harvest", "mourning_wood", 2, 2),
    ("winter", "everscream", 2, 1),
    ("winter", "santa_tank", 4, 1),
    ("pirates", "flying_dutchman", 2, 1),
    ("crystal", "ogre", 3, 1),
    ("crystal", "betsy", 5, 1),
];

/// Everything this file steers on top of the ordinary mob AI.
const STEERED: &[&str] = &["dungeon_guardian", "paladin", "mourning_wood", "everscream", "santa_tank", "flying_dutchman", "ogre", "betsy", "tim", "npc_old_man"];

/// Seconds in the cursed dungeon before the guardian comes, and when it is felt coming.
const GUARDIAN_AT: f32 = 40.0;
const GUARDIAN_WARN: f32 = 25.0;
/// How long the skeleton merchant stays.
const MERCHANT_STAY: f32 = 240.0;

#[derive(Default)]
pub struct Folk {
    spawn_t: f32,
    /// Seconds spent in the cursed dungeon (the guardian comes at GUARDIAN_AT).
    cursed_t: f32,
    /// Seconds out of the dungeon while the guardian is about (it leaves after a while).
    out_t: f32,
    merchant_t: f32,
    /// Asked the Old Man once at night: the second ask within these seconds wakes the curse.
    ask_t: f32,
    pub(crate) contact_t: f32,
}

impl Game {
    pub(crate) fn update_folk(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let f = &mut self.t5.tr.r3.folk;
        f.spawn_t -= dt;
        f.contact_t -= dt;
        f.ask_t -= dt;
        self.guardian_watch(dt);
        self.merchant_clock(dt);
        self.steer_folk(dt);
        self.steer_chains(dt);
        if self.t5.tr.r3.folk.spawn_t > 0.0 {
            return;
        }
        self.t5.tr.r3.folk.spawn_t = 5.0;
        if !self.survival || !self.mobs.monsters_may_spawn() || self.dim != voxel::worldgen::Dim::Overworld {
            return;
        }
        self.event_minis();
        self.folk_spawns();
    }

    /// Mini-bosses of the wave event under way.
    fn event_minis(&mut self) {
        let Some((key, wave, _)) = self.event_now() else { return };
        for &(ev, who, from, most) in EVENT_MINIS {
            if ev != key || wave < from || self.count_alive(&[who]) >= most || self.surv.rng.f32() > 0.35 {
                continue;
            }
            let flies = matches!(who, "flying_dutchman" | "betsy");
            if self.spawn_around(who, flies) {
                let dn = self.mobs.list.last().map(|m| self.mobs.reg.defs[m.def].display_name.clone()).unwrap_or_default();
                self.toast(format!("{dn} se junta ao ataque!"), 3.0);
                self.sfx(Sfx::Thunder(0.3));
            }
        }
    }

    /// Rare monsters and visitors, by where the player is.
    fn folk_spawns(&mut self) {
        let me = self.body.pos;
        let layer = Layer::at(me.y.floor() as i32);
        let hard = self.t5.hard.on;
        let night = self.is_night();
        let r = self.surv.rng.f32();
        let deep = matches!(layer, Layer::Deep | Layer::Caverns);
        let biome = self.world.gen.base_biome_at(me.x.floor() as i32, me.z.floor() as i32);
        let in_dungeon = self.t5.tr.dungeon.inside;
        let (who, cave, chance) = if hard && layer == Layer::Sky && self.count_alive(&["sky_wyvern"]) == 0 {
            ("sky_wyvern", false, 0.25)
        } else if in_dungeon && hard && self.boss.defeated.iter().any(|b| b == "vine_queen") && self.count_alive(&["paladin"]) == 0 {
            ("paladin", true, 0.06)
        } else if in_dungeon && !self.boss.defeated.iter().any(|b| b == "bone_king") && self.count_alive(&["npc_old_man"]) == 0 {
            ("npc_old_man", true, 1.0)
        } else if deep && self.count_alive(&["npc_skeleton_merchant"]) == 0 && self.t5.tr.r3.folk.merchant_t <= 0.0 && r < 0.5 {
            ("npc_skeleton_merchant", true, 0.06)
        } else if deep && self.count_alive(&["tim"]) == 0 && r < 0.3 {
            ("tim", true, 0.08)
        } else if matches!(layer, Layer::Underground | Layer::Deep | Layer::Caverns) && self.count_alive(&["undead_miner"]) < 2 {
            ("undead_miner", true, 0.12)
        } else if layer == Layer::Surface && night && biome == voxel::worldgen::Biome::Jungle && self.count_alive(&["doctor_bones"]) == 0 {
            ("doctor_bones", false, 0.05)
        } else if layer == Layer::Surface && !night && self.count_alive(&["pinky"]) == 0 {
            ("pinky", false, 0.01)
        } else {
            return;
        };
        if self.surv.rng.f32() >= chance {
            return;
        }
        if who == "sky_wyvern" {
            return self.spawn_wyvern();
        }
        let (min, max) = if who == "npc_old_man" { (3.0, 7.0) } else { (12.0, 24.0) };
        let Some(at) = self.spot_near(me, min, max, cave) else { return };
        if !self.mobs.spawn(who, at + Vec3::Y * 0.1) {
            return;
        }
        let friendly = who.starts_with("npc_");
        if let Some(m) = self.mobs.list.last_mut() {
            m.aggro = !friendly;
        }
        match who {
            "npc_skeleton_merchant" => {
                self.t5.tr.r3.folk.merchant_t = MERCHANT_STAY;
                self.toast("Você ouve ossos chacoalhando... um Mercador Esqueleto anda por aqui.", 4.0);
            }
            "paladin" => self.toast("Passos pesados de armadura ecoam na masmorra...", 3.0),
            _ => {}
        }
    }

    /// The skeleton merchant goes on his way after a while (when you're not looking at him).
    fn merchant_clock(&mut self, dt: f32) {
        let f = &mut self.t5.tr.r3.folk;
        if f.merchant_t <= 0.0 {
            return;
        }
        f.merchant_t -= dt;
        if f.merchant_t > 0.0 {
            return;
        }
        let me = self.body.pos;
        self.mobs.list.retain(|m| !(self.mobs.reg.defs[m.def].name == "npc_skeleton_merchant" && m.body.pos.distance(me) > 6.0));
        // Still talking to him: he waits a little more.
        if self.count_alive(&["npc_skeleton_merchant"]) > 0 {
            self.t5.tr.r3.folk.merchant_t = 20.0;
        } else {
            // A long while before he comes back.
            self.t5.tr.r3.folk.merchant_t = -600.0;
        }
    }

    /// The dungeon's curse calls its guardian on those who linger.
    fn guardian_watch(&mut self, dt: f32) {
        let free = self.boss.defeated.iter().any(|b| b == "bone_king");
        let inside = self.t5.tr.dungeon.inside && self.survival && !free && !self.mobs.peaceful;
        let f = &mut self.t5.tr.r3.folk;
        if f.merchant_t < 0.0 {
            f.merchant_t = (f.merchant_t + dt).min(0.0);
        }
        if !inside {
            f.cursed_t = (f.cursed_t - dt * 2.0).max(0.0);
            f.out_t += dt;
            if f.out_t > 8.0 {
                // It goes back into the walls.
                self.mobs.list.retain(|m| self.mobs.reg.defs[m.def].name != "dungeon_guardian");
            }
            return;
        }
        f.out_t = 0.0;
        let before = f.cursed_t;
        f.cursed_t += dt;
        let now = f.cursed_t;
        if before < GUARDIAN_WARN && now >= GUARDIAN_WARN {
            self.toast("Algo muito antigo se mexe nas paredes... saia da masmorra!", 4.0);
            self.sfx(Sfx::Thunder(0.2));
        }
        if now >= GUARDIAN_AT && self.count_alive(&["dungeon_guardian"]) == 0 {
            let f = self.forward();
            let at = self.body.pos - Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 14.0 + Vec3::Y * 2.0;
            if self.mobs.spawn("dungeon_guardian", at) {
                if let Some(m) = self.mobs.list.last_mut() {
                    m.aggro = true;
                }
                self.toast("O Guardião da Masmorra despertou!", 4.0);
                self.sfx(Sfx::Thunder(0.7));
            }
        }
    }

    /// What each of this file's monsters does besides walking at you.
    fn steer_folk(&mut self, dt: f32) {
        let me = self.body.eye();
        let feet = self.body.pos;
        let now = self.now;
        let hunted = self.hunted();
        // (what, where, which mob)
        let mut acts: Vec<(&'static str, Vec3, usize)> = Vec::new();
        for (k, m) in self.mobs.list.iter_mut().enumerate() {
            if !m.alive() {
                continue;
            }
            let name = self.mobs.reg.defs[m.def].name.as_str();
            let Some(&who) = STEERED.iter().find(|n| **n == name) else { continue };
            let pos = m.body.pos;
            let d = pos.distance(me);
            let to = (me - pos).normalize_or_zero();
            let angry = m.health < m.max_health * 0.5;
            // A cheap clock per mob: true about once every `every` seconds.
            let every = |every: f32| (now / every + k as f32 * 0.37).fract() < dt / every;
            match who {
                "dungeon_guardian" => {
                    // Straight at you, through stone; it spins as it comes.
                    m.body.pos += to * 13.0 * dt;
                    m.body.vel = Vec3::ZERO;
                    m.body.flying = true;
                    m.yaw += 9.0 * dt;
                    m.health = m.max_health;
                    if d < 1.8 {
                        acts.push(("guardian_hit", pos, k));
                    }
                }
                "npc_old_man" => {
                    m.body.vel.x = 0.0;
                    m.body.vel.z = 0.0;
                }
                "paladin" if hunted && d < 18.0 && every(if angry { 2.2 } else { 3.2 }) => acts.push(("hammer", pos, k)),
                "mourning_wood" if hunted && d < 26.0 && every(2.6) => acts.push(("seeds", pos, k)),
                "everscream" if hunted && d < 28.0 => {
                    if every(3.0) {
                        acts.push(("needles", pos, k));
                    }
                    if every(7.0) {
                        acts.push(("ornaments", pos, k));
                    }
                }
                "santa_tank" if hunted => {
                    if d > 6.0 {
                        let v = Vec3::new(to.x, 0.0, to.z).normalize_or_zero() * 4.5;
                        m.body.vel.x = v.x;
                        m.body.vel.z = v.z;
                        m.yaw = v.x.atan2(v.z);
                    }
                    if d < 30.0 && every(1.8) {
                        acts.push(("burst", pos, k));
                    }
                    if d < 30.0 && every(6.5) {
                        acts.push(("rockets", pos, k));
                    }
                }
                "flying_dutchman" | "betsy" => {
                    // They circle over you, high up, broadside on.
                    let (r, h, w) = if who == "betsy" { (14.0, 8.0, 0.45) } else { (12.0, 10.0, 0.3) };
                    let a = now * w + k as f32;
                    let spot = feet + Vec3::new(a.cos() * r, h, a.sin() * r);
                    let step = (spot - pos) * (1.2 * dt).min(1.0);
                    m.body.pos += step;
                    m.body.vel = Vec3::ZERO;
                    m.body.flying = true;
                    let dir = Vec3::new(-a.sin(), 0.0, a.cos());
                    m.yaw = dir.x.atan2(dir.z);
                    if hunted && who == "flying_dutchman" && every(1.1) {
                        acts.push(("cannon", pos, k));
                    }
                    if hunted && who == "betsy" {
                        if every(if angry { 3.2 } else { 4.5 }) {
                            acts.push(("breath", pos, k));
                        }
                        if every(11.0) {
                            acts.push(("wyverns", pos, k));
                        }
                    }
                }
                "ogre" if hunted => {
                    if d < 7.0 && m.body.on_ground && every(4.0) {
                        acts.push(("stomp", pos, k));
                    } else if d >= 7.0 && d < 24.0 && every(4.5) {
                        acts.push(("spit", pos, k));
                    }
                }
                "tim" if hunted => {
                    if every(5.0) {
                        acts.push(("blink", pos, k));
                    } else if d < 20.0 && every(1.8) {
                        acts.push(("bolt", pos, k));
                    }
                }
                _ => {}
            }
        }
        for (what, pos, k) in acts {
            let dir = (me - (pos + Vec3::Y * 1.5)).normalize_or_zero();
            match what {
                "guardian_hit" => {
                    if self.t5.tr.r3.folk.contact_t <= 0.0 {
                        self.t5.tr.r3.folk.contact_t = 0.6;
                        self.damage(60.0, Some(pos));
                    }
                }
                "hammer" => {
                    self.enemy_bolt(pos + Vec3::Y * 2.0 + dir, dir * 18.0, 5, 24.0);
                    self.sfx(Sfx::MobHit);
                }
                "seeds" => {
                    // Three burning seeds in a fan.
                    let from = pos + Vec3::Y * 5.0;
                    let d = (me - from).normalize_or_zero();
                    let side = d.cross(Vec3::Y).normalize_or_zero();
                    for s in [-0.25, 0.0, 0.25] {
                        self.enemy_bolt(from + d, (d + side * s).normalize() * 15.0, 0, 14.0);
                    }
                }
                "needles" => {
                    let from = pos + Vec3::Y * 4.0;
                    let d = (me - from).normalize_or_zero();
                    let side = d.cross(Vec3::Y).normalize_or_zero();
                    for s in [-0.4, -0.2, 0.0, 0.2, 0.4] {
                        self.enemy_bolt(from + d, (d + side * s).normalize() * 17.0, 1, 12.0);
                    }
                }
                "ornaments" | "rockets" => {
                    let school = if what == "ornaments" { 1 } else { 0 };
                    for i in 0..5 {
                        let a = i as f32 * 1.26 + self.now;
                        let r = if i == 0 { 0.0 } else { 3.5 };
                        let at = self.body.pos + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                        self.sky_strike(at, 14.0, 12.0, school, 18.0);
                    }
                }
                "burst" => {
                    for i in 0..3 {
                        let jit = Vec3::new(0.0, i as f32 * 0.04, 0.0);
                        self.enemy_bolt(pos + Vec3::Y * 2.2 + dir * 2.0, (dir + jit) * (22.0 + i as f32 * 2.0), 0, 10.0);
                    }
                    self.sfx(Sfx::Explode);
                }
                "cannon" => {
                    let from = pos + Vec3::Y * 1.5;
                    let d = (me - from).normalize_or_zero();
                    self.enemy_bolt(from + d * 3.0, d * 20.0, 0, 16.0);
                    self.sfx(Sfx::Explode);
                }
                "breath" => {
                    let from = pos + Vec3::Y * 2.0;
                    let d = (me - from).normalize_or_zero();
                    let side = d.cross(Vec3::Y).normalize_or_zero();
                    for i in 0..7 {
                        let s = (i as f32 - 3.0) * 0.08;
                        self.enemy_bolt(from + d * 3.0, (d + side * s).normalize() * 19.0, 0, 20.0);
                    }
                    self.sfx(Sfx::Cast(0));
                }
                "wyverns" => {
                    if self.count_alive(&["ether_wyvern"]) < 4 {
                        for j in [-2.0, 2.0] {
                            if self.mobs.spawn("ether_wyvern", pos + Vec3::new(j, 0.0, 0.0)) {
                                if let Some(m) = self.mobs.list.last_mut() {
                                    m.aggro = true;
                                }
                            }
                        }
                    }
                }
                "stomp" => {
                    self.shockwave(pos, 10.0, 20.0);
                    self.mobs.list[k].body.vel.y = 5.0;
                }
                "spit" => {
                    self.enemy_bolt(pos + Vec3::Y * 3.5 + dir, dir * 16.0, 4, 18.0);
                }
                "blink" => {
                    let me_feet = self.body.pos;
                    if let Some(to) = self.spot_near(me_feet, 6.0, 12.0, true).or_else(|| self.spot_near(me_feet, 6.0, 12.0, false)) {
                        let glow = self.glow_layer();
                        for i in 0..16 {
                            let a = i as f32 / 16.0 * std::f32::consts::TAU;
                            self.particles.glow(pos + Vec3::new(a.cos() * 0.6, 1.0, a.sin() * 0.6), Vec3::Y * 1.2, glow, [1.0, 0.6, 2.0], 0.08, 0.5);
                        }
                        self.mobs.list[k].body.pos = to;
                        self.sfx(Sfx::Cast(3));
                    }
                }
                "bolt" => {
                    self.enemy_bolt(pos + Vec3::Y * 1.6 + dir, dir * 14.0, 3, 12.0);
                }
                _ => {}
            }
        }
    }

    /// The wyvern comes from behind you, high up.
    fn spawn_wyvern(&mut self) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let at = self.body.pos - flat * 30.0 + Vec3::Y * 10.0;
        if self.spawn_chain("sky_wyvern", at, flat) {
            self.toast("Um rugido corta o vento: uma Serpe do Céu!", 4.0);
            self.sfx(Sfx::Thunder(0.5));
        }
    }

    /// The Old Man at the dungeon: by day he warns you off; at night, ask twice and the curse
    /// takes him and wakes the Bone King. True: the talk is all there is.
    pub(crate) fn old_man_talk(&mut self, dn: &str) -> bool {
        if self.boss.defeated.iter().any(|b| b == "bone_king") {
            self.toast(format!("{dn}: Obrigado por me libertar. Finalmente posso descansar."), 4.0);
            return true;
        }
        if !self.is_night() {
            self.toast(format!("{dn}: Não posso deixar você entrar. A maldição... volte à noite, se tiver coragem."), 5.0);
            return true;
        }
        if self.t5.tr.r3.folk.ask_t <= 0.0 {
            self.t5.tr.r3.folk.ask_t = 10.0;
            self.toast(format!("{dn}: Quer mesmo tentar quebrar a maldição? Fale comigo de novo e não haverá volta."), 5.0);
            return true;
        }
        if self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].boss) {
            self.toast(format!("{dn}: Agora não! Já há um mal desperto."), 3.0);
            return true;
        }
        let Some(k) = self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].name == "npc_old_man") else { return true };
        let at = self.mobs.list[k].body.pos;
        self.mobs.list.remove(k);
        if self.mobs.spawn("bone_king", at + Vec3::Y * 4.0) {
            self.t5_fight_starts();
            self.weather.flash = 0.6;
            self.sfx(Sfx::Thunder(0.6));
            self.toast("O velho se contorce... e o Rei dos Ossos se ergue!", 4.0);
        }
        true
    }

    /// The town jellies' chatter (they have nothing to sell).
    pub(crate) fn jelly_talk(&mut self, name: &str, dn: &str) {
        let k = (self.now * 0.37) as usize % 3;
        let line = match (name, k) {
            ("town_jelly_nerd", 0) => "Sabia que geleias são 98% gel? Eu calculei.",
            ("town_jelly_nerd", 1) => "Li todos os livros da biblioteca. Duas vezes.",
            ("town_jelly_nerd", _) => "Meus óculos? Grau de geleia. Muito raro.",
            ("town_jelly_cool", 0) => "E aí. Tudo tranquilo, tudo gelatinoso.",
            ("town_jelly_cool", 1) => "Óculos escuros até de noite. É estilo.",
            ("town_jelly_cool", _) => "Pula comigo? Não? Beleza.",
            (_, 0) => "Em meu tempo, a Rainha Geleia era só uma princesinha.",
            (_, 1) => "Esta barba levou trezentos anos para crescer.",
            _ => "Paciência, jovem. Tudo escorre no seu tempo.",
        };
        self.toast(format!("{dn}: {line}"), 4.0);
    }

    /// The Old Man is freed once the Bone King falls.
    pub(crate) fn folk_boss_defeated(&mut self, name: &str) {
        if name == "bone_king" {
            self.mobs.list.retain(|m| self.mobs.reg.defs[m.def].name != "npc_old_man" && self.mobs.reg.defs[m.def].name != "dungeon_guardian");
            self.t5.tr.r3.folk.cursed_t = 0.0;
        }
    }

    /// Test scene: MC_TR=folk, this lote's town folk and monsters in a row facing the camera.
    pub(crate) fn test_folk(&mut self, scene: &str) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-flat.z, 0.0, flat.x);
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        let rows: &[&[&str]] = if scene == "folk" {
            &[&["npc_tavernkeep", "npc_skeleton_merchant", "npc_old_man", "town_jelly_nerd", "town_jelly_cool", "town_jelly_elder", "tim", "doctor_bones", "undead_miner", "pinky"]]
        } else {
            &[&["dungeon_guardian", "paladin", "ogre", "mourning_wood", "everscream", "santa_tank"], &["betsy", "flying_dutchman"]]
        };
        for (r, row) in rows.iter().enumerate() {
            let dist = if scene == "folk" { 6.0 } else { 14.0 + r as f32 * 14.0 };
            let gap = if scene == "folk" { 1.4 } else { 5.0 + r as f32 * 6.0 };
            for (k, n) in row.iter().enumerate() {
                let p = self.body.pos + flat * dist + side * ((k as f32 - (row.len() - 1) as f32 / 2.0) * gap);
                let y = ground(self, p) + if r == 1 { 6.0 } else { 0.0 };
                self.mobs.spawn(n, Vec3::new(p.x, y, p.z));
                let last = self.mobs.list.len() - 1;
                let m = &mut self.mobs.list[last];
                let look = self.body.eye() - m.body.center();
                m.yaw = look.x.atan2(look.z);
                m.body.flying = r == 1;
            }
        }
        if scene == "wyvern" {
            self.mobs.list.clear();
            self.body.pos.y += 4.0;
            let at = self.body.pos + flat * 16.0 + Vec3::Y * 3.0;
            self.spawn_chain("sky_wyvern", at, -side);
        }
        self.mobs.calm = true;
        self.t5.tr.pets.test_keep();
    }
}
