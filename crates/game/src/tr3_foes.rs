//! L19 of the Terraria side's third round (plan/terraria-completo.md, "Reauditoria 3"): the
//! common monsters each land was still missing, and what makes each one its own in 3D.
//!
//! - Desert: antlions buried in the sand that lob grit at you, antlion chargers, and below
//!   ground the tomb crawler (a worm that swims through sand and stone and leaps out), lamias
//!   and ghouls in hardmode.
//! - Bats in every cave (plain, jungle, ice, lava, giant), flitting in jerks so they are hard
//!   to hit.
//! - Jungle: man eaters and snatchers rooted in the ground that bite whoever comes near,
//!   derplings that leap at you from far, giant tortoises that pull in and roll at you.
//! - The Prism (hallow): gastropods, enchanted swords that spin and dash, chaos elementals
//!   that blink about.
//! - Hardmode nights: possessed armour, wraiths that drift through walls.
//! - Marble: the medusa. When her eyes flare, look away: meeting her gaze turns you to stone
//!   for a moment. Granite: granite elementals that hurl themselves at you.
//! - Snow caves: flinxes and undead vikings. Glowing mushroom caves: anomuras and fungi bulbs
//!   that puff spores. Spider caves: wall creepers and, in hardmode, black recluses.
//! - The Underworld: demons throwing scythes and voodoo demons. A voodoo demon's doll thrown
//!   into lava down there (or in the Nether) wakes the World Breaker, as in Terraria.
//! - The dungeon in hardmode: armoured bones, ragged casters and skeleton snipers, whose red
//!   aiming line shows a second before they fire.
//!
//! The worm-like ones (the tomb crawler and L18's wyvern) are a head mob followed in the mob
//! list by its links; `steer_chains` moves them and pools the damage on the head.

use glam::Vec3;
use voxel::physics::Terrain;
use voxel::worldgen::deep::{Layer, UBiome};
use voxel::worldgen::Biome;

use crate::audio::Sfx;
use crate::Game;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum At {
    /// The surface of these biomes.
    Surface(&'static [Biome]),
    /// Underground, in this cave biome.
    Under(UBiome),
    /// Underground anywhere above the Underworld.
    Caves,
    /// The dungeon (inside).
    Dungeon,
    /// The Prism land, on the surface or under it.
    Prism(bool),
    /// The surface anywhere, at night.
    Night,
}

/// (mob, where, hardmode only, weight, flies)
pub(crate) const SPAWNS: &[(&str, At, bool, u32, bool)] = &[
    ("antlion", At::Surface(&[Biome::Desert]), false, 4, false),
    ("antlion_charger", At::Surface(&[Biome::Desert]), false, 3, false),
    ("antlion", At::Under(UBiome::Desert), false, 2, false),
    ("tomb_crawler", At::Under(UBiome::Desert), false, 2, false),
    ("lamia", At::Under(UBiome::Desert), true, 3, false),
    ("ghoul", At::Under(UBiome::Desert), true, 3, false),
    ("cave_bat", At::Caves, false, 3, true),
    ("giant_bat", At::Caves, true, 2, true),
    ("jungle_bat", At::Under(UBiome::Jungle), false, 4, true),
    ("man_eater", At::Under(UBiome::Jungle), false, 3, false),
    ("giant_tortoise", At::Under(UBiome::Jungle), true, 2, false),
    ("snatcher", At::Surface(&[Biome::Jungle]), false, 3, false),
    ("derpling", At::Surface(&[Biome::Jungle]), true, 3, false),
    ("ice_bat", At::Under(UBiome::Ice), false, 4, true),
    ("flinx", At::Under(UBiome::Ice), false, 4, false),
    ("undead_viking", At::Under(UBiome::Ice), false, 3, false),
    ("medusa", At::Under(UBiome::Marble), true, 2, false),
    ("granite_elemental", At::Under(UBiome::Granite), false, 4, true),
    ("anomura_fungus", At::Under(UBiome::Mushroom), false, 3, false),
    ("fungi_bulb", At::Under(UBiome::Mushroom), false, 3, false),
    ("wall_creeper", At::Under(UBiome::Spider), false, 4, false),
    ("black_recluse", At::Under(UBiome::Spider), true, 3, false),
    ("lava_bat", At::Under(UBiome::Underworld), true, 4, true),
    ("demon", At::Under(UBiome::Underworld), false, 4, true),
    ("voodoo_demon", At::Under(UBiome::Underworld), false, 1, true),
    ("gastropod", At::Prism(false), true, 3, true),
    ("possessed_sword", At::Prism(true), true, 3, true),
    ("chaos_elemental", At::Prism(true), true, 2, false),
    ("possessed_armor", At::Night, true, 3, false),
    ("wraith", At::Night, true, 2, true),
    ("armored_bones", At::Dungeon, true, 4, false),
    ("ragged_caster", At::Dungeon, true, 2, false),
    ("skeleton_sniper", At::Dungeon, true, 2, false),
];

/// Worm-like monsters: (head, body link, tail, links, speed, room between links, digs).
const CHAINS: &[(&str, &str, &str, usize, f32, f32, bool)] = &[
    ("sky_wyvern", "sky_wyvern_body", "sky_wyvern_tail", 12, 15.0, 1.5, false),
    ("tomb_crawler", "tomb_crawler_body", "tomb_crawler_tail", 6, 9.0, 1.0, true),
];

/// How many of this file's monsters may be about at once.
const MOST: usize = 6;

#[derive(Default)]
pub struct Foes {
    spawn_t: f32,
    doll_t: f32,
    /// The medusa's gaze already caught the player this flare.
    stoned_t: f32,
}

fn chain_of(name: &str) -> Option<&'static (&'static str, &'static str, &'static str, usize, f32, f32, bool)> {
    CHAINS.iter().find(|c| c.0 == name)
}

impl Game {
    pub(crate) fn update_foes(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        self.steer_foes(dt);
        self.voodoo_doll(dt);
        let f = &mut self.t5.tr.r3.foes;
        f.spawn_t -= dt;
        f.stoned_t -= dt;
        if f.spawn_t > 0.0 {
            return;
        }
        f.spawn_t = 3.0;
        if !self.survival || !self.mobs.monster_room() || self.dim != voxel::worldgen::Dim::Overworld {
            return;
        }
        self.foe_spawn();
    }

    fn foe_spawn(&mut self) {
        let me = self.body.pos;
        let names: Vec<&str> = SPAWNS.iter().map(|s| s.0).collect();
        if self.count_alive(&names) >= MOST {
            return;
        }
        let layer = Layer::at(me.y.floor() as i32);
        let under = !matches!(layer, Layer::Sky | Layer::Surface);
        let ub = self.ubiome();
        let biome = self.world.gen.base_biome_at(me.x.floor() as i32, me.z.floor() as i32);
        let night = self.is_night();
        let hard = self.t5.hard.on;
        let dungeon = self.t5.tr.dungeon.inside;
        // A calm night (lanterns) keeps the surface free, like the natural spawning.
        let calm = self.mobs.calm;
        let prism = self.t5.tr.lands.here == Some(crate::tr_world::Land::Prism);
        let fits = |at: At| match at {
            At::Surface(bs) => !under && !dungeon && !calm && bs.contains(&biome),
            At::Under(u) => under && !dungeon && ub == u,
            At::Caves => under && !dungeon && layer != Layer::Underworld && ub != UBiome::Underworld,
            At::Dungeon => dungeon,
            At::Prism(deep) => prism && under == deep,
            At::Night => !under && night && !dungeon && !calm,
        };
        let pool: Vec<(&str, u32, bool, bool)> = SPAWNS.iter().filter(|s| (!s.2 || hard) && fits(s.1)).map(|s| (s.0, s.3, s.4, !matches!(s.1, At::Surface(_) | At::Night | At::Prism(false)))).collect();
        let total: u32 = pool.iter().map(|p| p.1).sum();
        // Most tries come to nothing, so the ordinary monsters still come too.
        if total == 0 || self.surv.rng.f32() > 0.45 {
            return;
        }
        let mut roll = (self.surv.rng.f32() * total as f32) as u32;
        let mut pick = pool[0];
        for p in &pool {
            if roll < p.1 {
                pick = *p;
                break;
            }
            roll -= p.1;
        }
        let (who, _, flies, cave) = pick;
        let Some(at) = self.spot_near(me, 12.0, 26.0, cave) else { return };
        // Torch light keeps them off, as it does the natural monsters (the Underworld's lava
        // lights everything, so not there).
        if ub != UBiome::Underworld && self.world.light((at + Vec3::Y * 0.5).floor().as_ivec3()) & 15 > 0 {
            return;
        }
        let at = at + Vec3::Y * if flies { 2.0 } else { 0.1 };
        if chain_of(who).is_some() {
            let dir = (me - at).normalize_or_zero();
            // The crawler starts inside the ground under where it was found.
            self.spawn_chain(who, at - Vec3::Y * 3.0, dir);
            return;
        }
        if self.mobs.spawn(who, at) {
            if let Some(m) = self.mobs.list.last_mut() {
                m.aggro = true;
            }
        }
    }

    /// A worm-like monster: its head, then its links behind it along `-dir`.
    pub(crate) fn spawn_chain(&mut self, head: &str, at: Vec3, dir: Vec3) -> bool {
        let Some(&(_, body, tail, n, speed, room, _)) = chain_of(head) else { return false };
        if !self.mobs.spawn(head, at) {
            return false;
        }
        for k in 0..n {
            let name = if k + 1 == n { tail } else { body };
            self.mobs.spawn(name, at - dir * (k as f32 + 1.0) * room);
        }
        for m in self.mobs.list.iter_mut().rev().take(n + 1) {
            m.aggro = true;
            m.body.flying = true;
        }
        let last = self.mobs.list.len() - n - 1;
        // The head keeps its own velocity in `prey` (chain heads hunt nothing else).
        self.mobs.list[last].prey = Some(dir * speed * 0.7);
        true
    }

    /// Heads steer, links follow, and the links' wounds come off the head.
    pub(crate) fn steer_chains(&mut self, dt: f32) {
        let me = self.body.eye();
        let now = self.now;
        let hunted = self.hunted();
        let n = self.mobs.list.len();
        let mut hits = Vec::new();
        let mut k = 0;
        // The chain the links being walked belong to, while its head lives.
        let mut current: Option<&(&str, &str, &str, usize, f32, f32, bool)> = None;
        while k < n {
            let name = self.mobs.reg.defs[self.mobs.list[k].def].name.clone();
            if let Some(c) = chain_of(&name) {
                let &(_, body, tail, _, speed, room, digs) = c;
                if !self.mobs.list[k].alive() {
                    current = None;
                    k += 1;
                    continue;
                }
                current = Some(c);
                let pos = self.mobs.list[k].body.pos;
                let inside = self.world.solid(pos.floor().as_ivec3());
                let near = pos.distance(me) < 40.0;
                let target = if hunted && near {
                    me + Vec3::new((now * 0.7 + k as f32).sin() * 4.0, (now * 1.3).sin() * 3.0, (now * 0.9).cos() * 4.0)
                } else {
                    pos + Vec3::new((now * 0.3 + k as f32).cos(), 0.0, (now * 0.3).sin()) * 10.0
                };
                let mut vel = self.mobs.list[k].prey.unwrap_or(Vec3::X);
                if !digs || inside {
                    let want = (target - pos).normalize_or_zero() * speed;
                    vel += (want - vel) * ((if digs { 2.0 } else { 1.4 }) * dt).min(1.0);
                } else {
                    // A digger out in the air arcs and falls back in.
                    vel.y -= 22.0 * dt;
                }
                if digs && inside && (now * 3.0 + k as f32).fract() < dt * 3.0 {
                    self.sfx_at(Sfx::Hit(crate::audio::Material::Sand), pos, 0.8, 24.0);
                }
                let m = &mut self.mobs.list[k];
                m.prey = Some(vel);
                m.body.pos += vel * dt;
                m.body.vel = Vec3::ZERO;
                m.body.flying = true;
                m.yaw = vel.x.atan2(vel.z);
                m.head_pitch = (-vel.y / vel.length().max(0.1)).asin().clamp(-1.0, 1.0);
                let mut prev = m.body.pos;
                let attack = self.mobs.reg.defs[m.def].attack;
                if hunted && prev.distance(me) < 1.8 {
                    hits.push((prev, attack));
                }
                let mut lost = 0.0;
                let mut j = k + 1;
                while j < n {
                    let s = &mut self.mobs.list[j];
                    let def = &self.mobs.reg.defs[s.def];
                    if def.name != body && def.name != tail {
                        break;
                    }
                    let to = prev - s.body.pos;
                    let d = to.length();
                    if d > room {
                        s.body.pos += to * ((d - room) / d);
                    }
                    s.body.vel = Vec3::ZERO;
                    s.body.flying = true;
                    s.yaw = to.x.atan2(to.z);
                    s.head_pitch = (-to.y / d.max(0.1)).asin().clamp(-1.0, 1.0);
                    lost += (s.max_health - s.health).max(0.0);
                    s.health = s.max_health;
                    prev = s.body.pos;
                    if hunted && prev.distance(me) < 1.6 {
                        hits.push((prev, def.attack));
                    }
                    j += 1;
                }
                self.mobs.list[k].health -= lost;
                k = j;
                continue;
            }
            let is_link = CHAINS.iter().any(|c| c.1 == name || c.2 == name);
            if is_link {
                // A link whose head is gone falls apart.
                let orphan = current.map_or(true, |c| c.1 != name && c.2 != name);
                let m = &mut self.mobs.list[k];
                if orphan && m.alive() {
                    m.health = 0.0;
                    m.counted = true;
                    m.body.flying = false;
                }
            } else {
                current = None;
            }
            k += 1;
        }
        if let Some(&(at, dmg)) = hits.first() {
            if self.t5.tr.r3.folk.contact_t <= 0.0 {
                self.t5.tr.r3.folk.contact_t = 0.7;
                self.damage(dmg, Some(at));
            }
        }
    }

    /// What each common monster of this file does besides walking at you.
    fn steer_foes(&mut self, dt: f32) {
        let me = self.body.eye();
        let feet = self.body.pos;
        let look = self.forward();
        let now = self.now;
        let hunted = self.hunted();
        let mut acts: Vec<(&'static str, Vec3, usize)> = Vec::new();
        for (k, m) in self.mobs.list.iter_mut().enumerate() {
            if !m.alive() {
                continue;
            }
            let name = self.mobs.reg.defs[m.def].name.as_str();
            let Some(&(who, ..)) = SPAWNS.iter().find(|s| s.0 == name) else { continue };
            let pos = m.body.pos;
            let d = pos.distance(me);
            let to = (me - pos).normalize_or_zero();
            let flat = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();
            let every = |every: f32| (now / every + k as f32 * 0.37).fract() < dt / every;
            let cycle = |every: f32| (now / every + k as f32 * 0.37).fract();
            if !hunted || d > 40.0 {
                continue;
            }
            match who {
                "antlion" | "man_eater" | "snatcher" | "fungi_bulb" => {
                    // Rooted where they are.
                    m.body.vel.x = 0.0;
                    m.body.vel.z = 0.0;
                    m.yaw = flat.x.atan2(flat.z);
                    match who {
                        "antlion" if d < 22.0 && every(2.2) => acts.push(("grit", pos, k)),
                        "fungi_bulb" if d < 12.0 && every(2.5) => acts.push(("spores", pos, k)),
                        "man_eater" | "snatcher" if d < if who == "man_eater" { 3.4 } else { 2.6 } && every(1.3) => acts.push(("bite", pos, k)),
                        _ => {}
                    }
                }
                "cave_bat" | "jungle_bat" | "ice_bat" | "lava_bat" | "giant_bat" => {
                    // Jerky flight: a sideways flit now and then.
                    let side = Vec3::new(-to.z, 0.0, to.x);
                    let j = ((now * 3.0 + k as f32 * 1.7).sin() * 3.0).clamp(-1.0, 1.0);
                    m.body.pos += (side * j * 3.5 + Vec3::Y * (now * 4.0 + k as f32).sin() * 1.5) * dt;
                }
                "derpling" if m.body.on_ground && d < 18.0 && every(2.0) => {
                    m.body.vel = flat * (d * 0.6).clamp(4.0, 9.0) + Vec3::Y * 9.0;
                }
                "giant_tortoise" if m.body.on_ground && d > 3.0 && d < 18.0 && every(5.0) => {
                    // Pulls in and rolls at you.
                    m.body.vel = flat * 14.0 + Vec3::Y * 1.5;
                    acts.push(("roll", pos, k));
                }
                "possessed_sword" | "granite_elemental" => {
                    let c = cycle(3.2);
                    if c < 0.18 && d > 1.5 {
                        m.body.pos += to * 15.0 * dt;
                    }
                    if who == "possessed_sword" {
                        m.yaw += 12.0 * dt;
                    }
                }
                "wraith" => {
                    m.body.pos += to * 3.0 * dt;
                    m.body.vel = Vec3::ZERO;
                    m.body.flying = true;
                }
                "chaos_elemental" | "ragged_caster" => {
                    if every(4.5) {
                        acts.push(("blink", pos, k));
                    } else if who == "ragged_caster" && d < 22.0 && every(2.0) {
                        acts.push(("shadow", pos, k));
                    }
                }
                "demon" if d < 26.0 && every(3.0) => acts.push(("scythe", pos, k)),
                "medusa" if d < 22.0 => {
                    let c = cycle(6.0);
                    if every(6.0) {
                        acts.push(("gaze_warn", pos, k));
                    }
                    // Her gaze is live for the second after the warning.
                    if (0.12..0.3).contains(&c) {
                        let seen = look.dot((pos + Vec3::Y * 1.6 - me).normalize_or_zero()) > 0.9;
                        if seen {
                            acts.push(("stone", pos, k));
                        }
                    }
                }
                "skeleton_sniper" if d < 36.0 => {
                    let c = cycle(5.0);
                    m.body.vel.x = 0.0;
                    m.body.vel.z = 0.0;
                    m.yaw = flat.x.atan2(flat.z);
                    if c > 0.72 {
                        acts.push(("aim", pos, k));
                    } else if every(5.0) {
                        acts.push(("snipe", pos, k));
                    }
                }
                _ => {}
            }
        }
        for (what, pos, k) in acts {
            let from = pos + Vec3::Y * 1.4;
            let dir = (me - from).normalize_or_zero();
            match what {
                "grit" => {
                    let from = pos + Vec3::Y * 0.6;
                    let dir = (me - from).normalize_or_zero();
                    self.enemy_bolt(from + dir * 0.8, (dir + Vec3::Y * 0.15).normalize() * 15.0, 5, 8.0);
                    self.sfx_at(Sfx::Hit(crate::audio::Material::Sand), pos, 0.8, 24.0);
                }
                "spores" => {
                    for s in [-0.3f32, 0.0, 0.3] {
                        let side = dir.cross(Vec3::Y).normalize_or_zero();
                        self.enemy_bolt(from + dir * 0.6, (dir + side * s).normalize() * 9.0, 4, 8.0);
                    }
                }
                "bite" => {
                    if self.t5.tr.r3.folk.contact_t <= 0.0 {
                        self.t5.tr.r3.folk.contact_t = 0.5;
                        let dmg = self.mobs.reg.defs[self.mobs.list[k].def].attack;
                        self.mobs.list[k].attack_age = 0.0;
                        self.damage(dmg, Some(pos));
                    }
                }
                "roll" => self.sfx_at(Sfx::Hit(crate::audio::Material::Stone), pos, 0.8, 24.0),
                "blink" => {
                    let Some(to) = self.spot_near(feet, 5.0, 11.0, true).or_else(|| self.spot_near(feet, 5.0, 11.0, false)) else { continue };
                    let glow = self.glow_layer();
                    for i in 0..16 {
                        let a = i as f32 / 16.0 * std::f32::consts::TAU;
                        self.particles.glow(pos + Vec3::new(a.cos() * 0.6, 1.0, a.sin() * 0.6), Vec3::Y * 1.2, glow, [1.8, 0.6, 1.8], 0.08, 0.5);
                    }
                    self.mobs.list[k].body.pos = to;
                    self.sfx_at(Sfx::Cast(3), to, 0.8, 24.0);
                }
                "shadow" => self.enemy_bolt(from + dir, dir * 15.0, 3, 16.0),
                "scythe" => {
                    self.enemy_bolt(from + dir, dir * 12.0, 3, 16.0);
                    self.sfx_at(Sfx::Cast(3), pos, 0.8, 24.0);
                }
                "gaze_warn" => {
                    let glow = self.glow_layer();
                    for i in 0..10 {
                        let s = if i % 2 == 0 { -0.25 } else { 0.25 };
                        let side = dir.cross(Vec3::Y).normalize_or_zero();
                        self.particles.glow(pos + Vec3::Y * 1.75 + side * s + dir * 0.4, dir * (2.0 + i as f32 * 0.3), glow, [2.0, 2.0, 0.6], 0.07, 0.6);
                    }
                    self.sfx_at(Sfx::Cast(2), pos, 0.8, 24.0);
                }
                "stone" => {
                    if self.t5.tr.r3.foes.stoned_t <= 0.0 {
                        self.t5.tr.r3.foes.stoned_t = 3.0;
                        self.add_effect(crate::effects::SLOWNESS, 6, 2.0, false);
                        self.add_effect(crate::effects::WEAKNESS, 1, 2.0, false);
                        self.damage(10.0, Some(pos));
                        self.toast("Você olhou nos olhos da Medusa e virou pedra!", 2.5);
                        self.sfx(Sfx::Hit(crate::audio::Material::Stone));
                    }
                }
                "aim" => {
                    // The red line of the rifle, from the barrel to you.
                    let glow = self.glow_layer();
                    let len = me.distance(from);
                    let n = (len * 1.5) as i32;
                    for i in 0..n.min(40) {
                        if (i + (self.now * 30.0) as i32) % 3 == 0 {
                            let at = from + dir * (0.6 + i as f32 / 1.5);
                            self.particles.glow(at, Vec3::ZERO, glow, [2.2, 0.2, 0.2], 0.03, 0.05);
                        }
                    }
                }
                "snipe" => {
                    self.enemy_bolt(from + dir, dir * 45.0, 5, 40.0);
                    self.sfx_at(Sfx::Explode, pos, 0.8, 24.0);
                }
                _ => {}
            }
        }
    }

    /// A voodoo doll dropped into lava in the Underworld or the Nether wakes the World Breaker.
    fn voodoo_doll(&mut self, dt: f32) {
        self.t5.tr.r3.foes.doll_t -= dt;
        if self.t5.tr.r3.foes.doll_t > 0.0 {
            return;
        }
        self.t5.tr.r3.foes.doll_t = 0.5;
        let Some(doll) = self.items.id("guide_voodoo_doll") else { return };
        let deep = self.dim == voxel::worldgen::Dim::Nether || self.ubiome() == UBiome::Underworld;
        if !deep {
            return;
        }
        let hit = self.surv.drops.iter().position(|d| d.stack.item == doll && {
            let b = self.world.block_or_air(d.body.pos.floor().as_ivec3());
            self.world.fluid(b) == 2
        });
        let Some(i) = hit else { return };
        let at = self.surv.drops[i].body.pos;
        self.surv.drops.remove(i);
        if self.mobs.list.iter().any(|m| m.alive() && self.mobs.reg.defs[m.def].boss) {
            self.toast("O boneco queima... mas um chefe já está desperto.", 3.0);
            return;
        }
        let spot = Vec3::new(at.x, self.body.pos.y.max(at.y + 2.0), at.z);
        if self.mobs.spawn("world_breaker", spot) {
            self.t5_fight_starts();
            self.weather.flash = 0.6;
            self.sfx(Sfx::Thunder(0.7));
            self.toast("O boneco grita nas chamas... O Quebra-Mundos despertou!", 5.0);
        }
    }

    /// Test scenes: MC_TR=foes (the lot in rows), MC_TR=crawler, MC_TR=medusa.
    pub(crate) fn test_foes(&mut self, scene: &str) {
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-flat.z, 0.0, flat.x);
        let ground = |g: &Game, p: Vec3| g.world.surface_y(p.x.floor() as i32, p.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(p.y);
        match scene {
            "foes" | "foes2" => {
                let rows: &[&[&str]] = if scene == "foes" {
                    &[&["antlion", "antlion_charger", "lamia", "ghoul", "man_eater", "snatcher", "derpling", "giant_tortoise"], &["cave_bat", "jungle_bat", "ice_bat", "lava_bat", "giant_bat", "gastropod", "possessed_sword"]]
                } else {
                    &[&["chaos_elemental", "possessed_armor", "wraith", "medusa", "granite_elemental", "flinx", "undead_viking"], &["demon", "voodoo_demon", "armored_bones", "ragged_caster", "skeleton_sniper", "anomura_fungus", "fungi_bulb", "wall_creeper", "black_recluse"]]
                };
                for (r, row) in rows.iter().enumerate() {
                    for (k, n) in row.iter().enumerate() {
                        let p = self.body.pos + flat * (6.0 + r as f32 * 3.5) + side * ((k as f32 - (row.len() - 1) as f32 / 2.0) * 1.9);
                        let flies = SPAWNS.iter().any(|s| s.0 == *n && s.4);
                        self.mobs.spawn(n, Vec3::new(p.x, ground(self, p) + if flies { 1.2 } else { 0.0 }, p.z));
                        let last = self.mobs.list.len() - 1;
                        let m = &mut self.mobs.list[last];
                        let look = self.body.eye() - m.body.center();
                        m.yaw = look.x.atan2(look.z);
                        m.body.flying = flies;
                    }
                }
            }
            "crawler" => {
                let at = self.body.pos + flat * 8.0 + Vec3::Y * 2.0;
                self.spawn_chain("tomb_crawler", at, side);
            }
            _ => {
                let p = self.body.pos + flat * 6.0;
                self.mobs.spawn("medusa", Vec3::new(p.x, ground(self, p), p.z));
                let last = self.mobs.list.len() - 1;
                let m = &mut self.mobs.list[last];
                let look = self.body.eye() - m.body.center();
                m.yaw = look.x.atan2(look.z);
                self.pitch = -0.1;
            }
        }
        self.mobs.calm = true;
        self.t5.tr.pets.test_keep();
    }
}
