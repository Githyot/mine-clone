//! Mobs: data-driven definitions (models and stats from `mobs.ron`) and their behaviour.
//! Rendering lives in the game crate; this module only moves and thinks.

use glam::Vec3;
use serde::Deserialize;

use crate::physics::{Body, Input, Terrain};
use crate::rng::Rng;

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Anim {
    #[default]
    None,
    /// Turns with the head yaw/pitch.
    Head,
    /// Walking legs; pairs with the same index swing together.
    Leg(u8),
    Arm(u8),
    /// Arms held out in front (zombies).
    ArmForward(u8),
    Wing(u8),
    SpiderLeg(u8),
}

#[derive(Deserialize, Clone, Debug)]
pub struct PartDef {
    pub size: (u8, u8, u8),
    pub origin: (i16, i16, i16),
    pub uv: (u16, u16),
    #[serde(default)]
    pub anim: Anim,
    #[serde(default)]
    pub pivot: (i16, i16, i16),
}

#[derive(Deserialize, Clone, Debug)]
pub struct LayerDef {
    pub part: usize,
    pub inflate: f32,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Passive,
    Hostile,
    /// Leaves the player alone until hurt, then fights back (bears).
    Neutral,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnRule {
    /// On grass, in daylight.
    Grass,
    /// Anywhere dark (night or caves).
    Dark,
    /// On the surface of water, in daylight (ducks).
    Water,
    /// Never spawns by itself (the player's own model).
    Never,
}

fn d_false() -> bool {
    false
}

fn d_group() -> (u8, u8) {
    (2, 4)
}

fn d_weight() -> u32 {
    10
}

#[derive(Deserialize, Clone, Debug)]
pub struct MobDef {
    pub name: String,
    pub display_name: String,
    pub kind: Kind,
    pub health: f32,
    pub speed: f32,
    pub width: f32,
    pub height: f32,
    pub spawn: SpawnRule,
    #[serde(default)]
    pub attack: f32,
    #[serde(default = "d_false")]
    pub ranged: bool,
    #[serde(default = "d_false")]
    pub burns_in_daylight: bool,
    #[serde(default = "d_false")]
    pub explodes: bool,
    #[serde(default = "d_false")]
    pub climbs: bool,
    pub parts: Vec<PartDef>,
    #[serde(default)]
    pub layers: Vec<LayerDef>,
    /// Items dropped when killed by the player.
    #[serde(default)]
    pub drops: Vec<crate::block::DropDef>,
    #[serde(default)]
    pub xp: u8,
    /// Runs away from the player when they come this close (0 = never; hurt animals still panic).
    #[serde(default)]
    pub flee_range: f32,
    /// Floats on the water surface and paddles about instead of bobbing.
    #[serde(default = "d_false")]
    pub swims: bool,
    /// Moves in hops (rabbits).
    #[serde(default = "d_false")]
    pub hops: bool,
    /// Flies a few blocks over the ground (bees).
    #[serde(default = "d_false")]
    pub flies: bool,
    /// Lives under water: swims about below the surface and flops about on land (squid, dolphins, axolotls).
    #[serde(default = "d_false")]
    pub aquatic: bool,
    /// Naps on the ground in the daytime.
    #[serde(default = "d_false")]
    pub sleeps: bool,
    /// Mobs (by name) it hunts now and then.
    #[serde(default)]
    pub prey: Vec<String>,
    /// Biomes (as `Biome::name()` says them) where it spawns; empty means anywhere.
    #[serde(default)]
    pub biomes: Vec<String>,
    /// Blocks it spawns on (names) for the `Grass` rule; empty means grass blocks.
    #[serde(default)]
    pub ground: Vec<String>,
    /// How many come together (animals): min, max.
    #[serde(default = "d_group")]
    pub group: (u8, u8),
    /// How often it is picked among the mobs that may spawn at a spot.
    #[serde(default = "d_weight")]
    pub weight: u32,
    /// Drawn this many times bigger than its model (bosses).
    #[serde(default = "d_scale")]
    pub scale: f32,
    /// A boss: health bar at the top of the screen, never wanders off.
    #[serde(default = "d_false")]
    pub boss: bool,
    /// A town NPC: lives in a house, never despawns, talks and trades.
    #[serde(default = "d_false")]
    pub town: bool,
}

fn d_scale() -> f32 {
    1.0
}

impl MobDef {
    /// Spawns in this biome?
    pub fn in_biome(&self, biome: &str) -> bool {
        self.biomes.is_empty() || self.biomes.iter().any(|b| b == biome)
    }
}

#[derive(Deserialize)]
struct MobFile {
    mobs: Vec<MobDef>,
}

#[derive(Default)]
pub struct MobRegistry {
    pub defs: Vec<MobDef>,
}

impl MobRegistry {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let f: MobFile = ron::from_str(text).map_err(|e| format!("mobs.ron: {e}"))?;
        for m in &f.mobs {
            if m.parts.len() > 16 {
                return Err(format!("mob '{}' has more than 16 parts", m.name));
            }
            for l in &m.layers {
                if l.part >= m.parts.len() {
                    return Err(format!("mob '{}' layer refers to missing part {}", m.name, l.part));
                }
            }
        }
        Ok(MobRegistry { defs: f.mobs })
    }

    pub fn id(&self, name: &str) -> Option<usize> {
        self.defs.iter().position(|d| d.name == name)
    }
}

/// Something a mob did that the game must react to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MobEvent {
    /// Melee hit on the player.
    Hit { damage: f32, from: Vec3 },
    /// An arrow leaves `from` aimed at `at` (the player's feet); `hit` says whether the aim is true.
    Shoot { damage: f32, from: Vec3, at: Vec3, hit: bool },
    Explode { pos: Vec3, power: f32 },
    /// Bit its prey (the mob the game set as `Mob::prey`).
    Bite { damage: f32 },
    /// Took damage from the sun; the game shows fire.
    Burning,
    Died { pos: Vec3 },
}

#[derive(Clone, Debug)]
pub struct Mob {
    pub def: usize,
    pub body: Body,
    pub yaw: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub health: f32,
    pub max_health: f32,
    /// Seconds since last hurt (for the red flash and the health bar).
    pub hurt_age: f32,
    /// Walk cycle phase.
    pub walk: f32,
    pub walk_speed: f32,
    pub age: f32,
    pub dead_age: f32,
    /// A per-mob offset for idle animations, so a herd doesn't move in step.
    pub phase: f32,
    wander: Option<Vec3>,
    idle: f32,
    attack_cd: f32,
    fuse: f32,
    pub aggro: bool,
    burn_tick: f32,
    pub stuck: f32,
    /// Seconds left of running away after being hurt, and from where.
    panic: f32,
    panic_from: Vec3,
    /// Napping (drawn lying down).
    pub sleeping: bool,
    sleep_t: f32,
    /// Where the prey it is hunting is now (set by the game each tick), and how long until it
    /// is hungry again after a catch.
    pub prey: Option<Vec3>,
    pub fed: f32,
    /// Seconds left burning (fire spells) and slowed (frost spells).
    pub on_fire: f32,
    pub chill: f32,
    /// Where a town NPC lives; it wanders near here.
    pub home: Option<Vec3>,
    /// Wears a saddle (horses and the like), and has the player on its back.
    pub saddled: bool,
    pub ridden: bool,
    /// A name given with a name tag (shown over its head; it stays around).
    pub name: Option<String>,
    /// On a lead: held by the player, or tied to the fence post at `post`.
    pub leashed: bool,
    pub post: Option<glam::IVec3>,
    fire_tick: f32,
    rng: Rng,
}

pub const DEATH_TIME: f32 = 0.9;

impl Mob {
    /// Walk to a spot (town folk at work), then stand there a moment.
    pub fn go_to(&mut self, p: Vec3, wait: f32) {
        self.wander = Some(p);
        self.idle = wait;
    }

    pub fn new(def_index: usize, def: &MobDef, pos: Vec3, seed: u64) -> Mob {
        let mut body = Body::sized(pos, def.width, def.height);
        body.speed_override = Some(def.speed);
        let mut rng = Rng::new(seed);
        Mob {
            def: def_index,
            body,
            yaw: rng.f32() * std::f32::consts::TAU,
            head_yaw: 0.0,
            head_pitch: 0.0,
            health: def.health,
            max_health: def.health,
            hurt_age: 100.0,
            walk: 0.0,
            walk_speed: 0.0,
            age: 0.0,
            dead_age: 0.0,
            phase: rng.f32() * std::f32::consts::TAU,
            wander: None,
            idle: rng.f32() * 3.0,
            attack_cd: 1.0,
            fuse: 0.0,
            aggro: false,
            burn_tick: 0.0,
            stuck: 0.0,
            panic: 0.0,
            panic_from: pos,
            sleeping: false,
            sleep_t: 0.0,
            prey: None,
            fed: 20.0 + rng.f32() * 40.0,
            on_fire: 0.0,
            chill: 0.0,
            home: None,
            saddled: false,
            ridden: false,
            name: None,
            leashed: false,
            post: None,
            fire_tick: 0.0,
            rng,
        }
    }

    pub fn alive(&self) -> bool {
        self.health > 0.0
    }

    /// Returns true if the mob died from this hit.
    pub fn hurt(&mut self, damage: f32, from: Vec3) -> bool {
        if !self.alive() {
            return false;
        }
        self.health -= damage;
        self.hurt_age = 0.0;
        let dir = self.body.pos - from;
        self.body.knockback(dir, 6.0);
        self.aggro = true;
        self.wander = None;
        self.panic = 5.0;
        self.panic_from = from;
        self.sleeping = false;
        self.health <= 0.0
    }

    /// Fuse progress 0..1 for exploding mobs (drives the swelling animation).
    pub fn fuse(&self) -> f32 {
        (self.fuse / 1.5).clamp(0.0, 1.0)
    }

    /// Lie down for a nap of `secs` seconds.
    pub fn fall_asleep(&mut self, secs: f32) {
        self.sleeping = true;
        self.sleep_t = secs;
        self.wander = None;
    }

    /// Looking for something to hunt?
    pub fn hungry(&self, def: &MobDef) -> bool {
        !def.prey.is_empty() && self.fed <= 0.0 && !self.sleeping && self.alive()
    }

    /// One simulation step.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        def: &MobDef,
        world: &impl Terrain,
        player: Vec3,
        player_creative: bool,
        sky_light_at_head: u8,
        daylight: bool,
        dt: f32,
        out: &mut Vec<MobEvent>,
    ) {
        self.age += dt;
        self.hurt_age += dt;
        self.attack_cd -= dt;
        self.fed -= dt;
        self.panic -= dt;
        if !self.alive() {
            self.dead_age += dt;
            // Keep falling while the death animation plays.
            self.body.step(world, &Input::default(), dt);
            return;
        }
        let to_player = player - self.body.pos;
        let dist = to_player.length();
        let flat = Vec3::new(to_player.x, 0.0, to_player.z);

        let mut wish = Vec3::ZERO;
        let mut jump = false;
        let mut run = false;
        let mut sneak = false;
        let hostile = def.kind == Kind::Hostile && !player_creative;

        // Set alight by a spell: a point of damage a second until it goes out (water puts it out).
        if self.on_fire > 0.0 {
            self.on_fire = if self.body.in_water { 0.0 } else { self.on_fire - dt };
            self.fire_tick += dt;
            if self.fire_tick > 1.0 {
                self.fire_tick = 0.0;
                self.health -= 1.0;
                self.hurt_age = 0.0;
                out.push(MobEvent::Burning);
                if self.health <= 0.0 {
                    out.push(MobEvent::Died { pos: self.body.center() });
                    return;
                }
            }
        }
        self.chill = (self.chill - dt).max(0.0);
        self.body.speed_override = Some(def.speed * if self.chill > 0.0 { 0.4 } else { 1.0 });
        if def.burns_in_daylight && daylight && sky_light_at_head >= 14 && !self.body.in_water {
            self.burn_tick += dt;
            if self.burn_tick > 1.0 {
                self.burn_tick = 0.0;
                self.health -= 1.0;
                self.hurt_age = 0.0;
                out.push(MobEvent::Burning);
                if self.health <= 0.0 {
                    out.push(MobEvent::Died { pos: self.body.center() });
                    return;
                }
            }
        }

        if hostile && dist < 16.0 {
            self.aggro = true;
        }
        if hostile && dist > 28.0 {
            self.aggro = false;
        }
        // A neutral animal calms down once the player is gone (or can't be hurt).
        if def.kind == Kind::Neutral && (dist > 20.0 || player_creative) {
            self.aggro = false;
        }
        let fights = self.aggro && !player_creative && def.kind != Kind::Passive;
        let flee_player = def.flee_range > 0.0 && dist < def.flee_range && !player_creative;
        let panicking = self.panic > 0.0 && def.kind == Kind::Passive;
        if fights || flee_player || panicking || !daylight {
            self.sleeping = false;
        }

        if fights {
            run = true;
            // Face the player and close in.
            let target_yaw = flat.x.atan2(flat.z);
            self.yaw = turn_towards(self.yaw, target_yaw, dt * 6.0);
            self.head_yaw = 0.0;
            self.head_pitch = (-to_player.y / dist.max(0.1)).asin().clamp(-0.8, 0.8);
            let reach = def.width / 2.0 + 0.3 + 0.6;
            if def.explodes {
                if dist < 3.0 {
                    self.fuse += dt;
                    if self.fuse >= 1.5 {
                        out.push(MobEvent::Explode { pos: self.body.center(), power: 3.0 });
                        self.health = 0.0;
                        return;
                    }
                } else {
                    self.fuse = (self.fuse - dt * 2.0).max(0.0);
                    wish = flat.normalize_or_zero();
                }
            } else if def.ranged {
                // Keep some distance, shoot every couple of seconds.
                if dist < 6.0 {
                    wish = -flat.normalize_or_zero();
                } else if dist > 10.0 {
                    wish = flat.normalize_or_zero();
                }
                if self.attack_cd <= 0.0 && dist < 15.0 {
                    self.attack_cd = 2.0 + self.rng.f32();
                    let hit = self.rng.chance((1.0 - dist / 22.0).clamp(0.2, 0.85));
                    let from = self.body.pos + Vec3::Y * (def.height * 0.75);
                    out.push(MobEvent::Shoot { damage: def.attack, from, at: player, hit });
                }
            } else if dist > reach {
                wish = flat.normalize_or_zero();
            } else if self.attack_cd <= 0.0 {
                self.attack_cd = 1.0;
                out.push(MobEvent::Hit { damage: def.attack, from: self.body.center() });
            }
        } else if let Some(prey) = self.prey.filter(|_| self.hungry(def)) {
            // Hunting: run the prey down and bite it.
            self.fuse = 0.0;
            run = true;
            let d = Vec3::new(prey.x - self.body.pos.x, 0.0, prey.z - self.body.pos.z);
            self.yaw = turn_towards(self.yaw, d.x.atan2(d.z), dt * 8.0);
            self.head_yaw = turn_towards(self.head_yaw, 0.0, dt * 5.0);
            if d.length() > def.width / 2.0 + 0.7 {
                wish = d.normalize_or_zero();
            } else if self.attack_cd <= 0.0 {
                self.attack_cd = 1.0;
                out.push(MobEvent::Bite { damage: def.attack.max(2.0) });
            }
        } else if flee_player || panicking {
            // Run away from whatever scared it.
            self.fuse = 0.0;
            run = true;
            self.wander = None;
            let threat = if panicking { self.panic_from } else { player };
            let mut away = self.body.pos - threat;
            away.y = 0.0;
            let away = if away.length_squared() < 1e-4 { Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos()) } else { away.normalize() };
            wish = away;
            self.yaw = turn_towards(self.yaw, away.x.atan2(away.z), dt * 8.0);
            self.head_yaw = turn_towards(self.head_yaw, 0.0, dt * 5.0);
            self.head_pitch = turn_towards(self.head_pitch, 0.0, dt * 5.0);
        } else if self.sleeping {
            // Curled up with the head down; wakes after a while or when someone comes close.
            self.fuse = 0.0;
            self.sleep_t -= dt;
            self.head_yaw = turn_towards(self.head_yaw, 0.0, dt * 2.0);
            self.head_pitch = turn_towards(self.head_pitch, 0.45, dt * 2.0);
            if self.sleep_t <= 0.0 || (dist < 3.0 && !player_creative) {
                self.sleeping = false;
                self.idle = 1.0 + self.rng.f32() * 2.0;
            }
        } else {
            self.fuse = 0.0;
            // Wander: walk somewhere for a bit, then stand around.
            self.idle -= dt;
            match self.wander {
                Some(t) => {
                    let d = Vec3::new(t.x - self.body.pos.x, 0.0, t.z - self.body.pos.z);
                    if d.length() < 0.6 || self.idle <= 0.0 {
                        self.wander = None;
                        self.idle = 1.5 + self.rng.f32() * 4.0;
                    } else {
                        wish = d.normalize() * 0.7;
                        self.yaw = turn_towards(self.yaw, d.x.atan2(d.z), dt * 4.0);
                    }
                }
                None => {
                    if self.idle <= 0.0 && def.sleeps && daylight && self.rng.chance(0.35) {
                        self.sleeping = true;
                        self.sleep_t = 15.0 + self.rng.f32() * 25.0;
                    } else if self.idle <= 0.0 {
                        let a = self.rng.f32() * std::f32::consts::TAU;
                        // Town folk potter about near home.
                        let (centre, r) = match self.home {
                            Some(h) => (h, self.rng.f32() * 3.0),
                            None => (self.body.pos, 3.0 + self.rng.f32() * 6.0),
                        };
                        self.wander = Some(centre + Vec3::new(a.cos() * r, 0.0, a.sin() * r));
                        self.idle = 3.0 + self.rng.f32() * 4.0;
                    }
                }
            }
            // Look at the player when near, like curious animals do.
            if dist < 6.0 && self.rng.f32() < 0.98 {
                let rel = flat.x.atan2(flat.z) - self.yaw;
                let rel = (rel + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                self.head_yaw = turn_towards(self.head_yaw, rel.clamp(-1.2, 1.2), dt * 5.0);
                self.head_pitch = turn_towards(self.head_pitch, (-to_player.y / dist.max(0.1)).asin().clamp(-0.6, 0.6), dt * 5.0);
            } else {
                self.head_yaw = turn_towards(self.head_yaw, 0.0, dt * 3.0);
                self.head_pitch = turn_towards(self.head_pitch, 0.0, dt * 3.0);
            }
        }

        // Jump over single blocks when the way is blocked (also out of the water onto a bank).
        let before = self.body.pos;
        if wish != Vec3::ZERO && (self.body.on_ground || self.body.in_water) {
            let ahead = self.body.pos + wish.normalize() * (def.width / 2.0 + 0.2);
            let foot = ahead.floor().as_ivec3();
            if world.solid(foot) && !world.solid(foot + glam::IVec3::Y) && !world.solid(foot + glam::IVec3::Y * 2) {
                jump = true;
            }
        }
        if def.hops && wish != Vec3::ZERO && self.body.on_ground {
            jump = true;
        }
        if def.flies {
            // Hover two to four blocks over whatever is below, drifting up and down a little.
            self.body.flying = true;
            wish *= def.speed / crate::physics::FLY_SPEED;
            let mut over = 0;
            while over < 6 && !world.solid((self.body.pos - Vec3::Y * (over as f32 + 0.5)).floor().as_ivec3()) && !world.liquid((self.body.pos - Vec3::Y * (over as f32 + 0.5)).floor().as_ivec3()) {
                over += 1;
            }
            let want = 2.0 + (self.age * 0.7 + self.phase).sin() * 1.0;
            jump = (over as f32) < want;
            sneak = (over as f32) > want + 1.0;
        } else if def.aquatic {
            if self.body.in_water {
                // Stay under: dive when near the surface, rise from the bottom.
                jump = false;
                let near_top = !world.liquid((self.body.pos + Vec3::Y * (def.height + 0.4)).floor().as_ivec3());
                let near_floor = world.solid((self.body.pos - Vec3::Y * 0.6).floor().as_ivec3());
                let bob = (self.age * 0.5 + self.phase).sin();
                self.body.vel.y = if near_top { -1.0 } else if near_floor { 1.0 } else { bob * 0.6 };
                if wish == Vec3::ZERO {
                    wish = Vec3::new((self.age * 0.3 + self.phase).cos(), 0.0, (self.age * 0.3 + self.phase).sin()) * 0.5;
                    self.yaw = turn_towards(self.yaw, wish.x.atan2(wish.z), dt * 2.0);
                }
            } else {
                // Out of the water: flop about.
                jump = self.body.on_ground && self.rng.chance(0.05);
                wish = Vec3::ZERO;
            }
        } else if self.body.in_water {
            if def.swims {
                // Float: paddle up only while the waterline is above the belly.
                if world.liquid((self.body.pos + Vec3::Y * 0.35).floor().as_ivec3()) {
                    jump = true;
                }
            } else {
                jump = true;
            }
        }
        let input = Input { wish, jump, sneak, sprint: run, crawl: false };
        self.body.step(world, &input, dt);
        let moved = (self.body.pos - before) * Vec3::new(1.0, 0.0, 1.0);
        let speed = moved.length() / dt.max(1e-4);
        self.walk_speed += (speed - self.walk_speed) * (dt * 10.0).min(1.0);
        self.walk += self.walk_speed * dt * 2.2;
        if wish != Vec3::ZERO && speed < 0.2 {
            self.stuck += dt;
            if self.stuck > 1.5 {
                self.stuck = 0.0;
                self.wander = None;
                self.idle = 0.5;
            }
        } else {
            self.stuck = 0.0;
        }
    }
}

fn turn_towards(cur: f32, target: f32, max: f32) -> f32 {
    let mut d = target - cur;
    while d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    while d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    cur + d.clamp(-max, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    struct Flat;
    impl Terrain for Flat {
        fn solid(&self, p: IVec3) -> bool {
            p.y < 10
        }
        fn liquid(&self, _: IVec3) -> bool {
            false
        }
    }

    #[test]
    fn parses_vanilla_mobs_and_zombie_chases() {
        let reg = MobRegistry::from_ron(include_str!("../../../packs/vanilla/mobs.ron")).unwrap();
        assert!(reg.defs.len() >= 8);
        let z = reg.id("zombie").unwrap();
        let def = &reg.defs[z];
        let mut m = Mob::new(z, def, Vec3::new(0.5, 10.0, 0.5), 1);
        let player = Vec3::new(8.0, 10.0, 0.5);
        let mut ev = Vec::new();
        for _ in 0..400 {
            m.tick(def, &Flat, player, false, 0, false, 0.02, &mut ev);
        }
        assert!(m.body.pos.x > 5.0, "{}", m.body.pos.x);
        assert!(ev.iter().any(|e| matches!(e, MobEvent::Hit { .. })));
    }

    fn vanilla() -> MobRegistry {
        MobRegistry::from_ron(include_str!("../../../packs/vanilla/mobs.ron")).unwrap()
    }

    fn run(reg: &MobRegistry, name: &str, at: Vec3, player: Vec3, secs: f32, world: &impl Terrain) -> (Mob, Vec<MobEvent>) {
        let i = reg.id(name).unwrap();
        let def = &reg.defs[i];
        let mut m = Mob::new(i, def, at, 7);
        let mut ev = Vec::new();
        for _ in 0..(secs / 0.02) as usize {
            m.tick(def, world, player, false, 15, true, 0.02, &mut ev);
        }
        (m, ev)
    }

    #[test]
    fn old_mob_data_still_loads_with_defaults() {
        let text = r#"(mobs: [(name: "blob", display_name: "Blob", kind: Passive, health: 4, speed: 2.0,
            width: 0.5, height: 0.5, spawn: Grass, parts: [(size: (4, 4, 4), origin: (-2, 0, -2), uv: (0, 0))])])"#;
        let reg = MobRegistry::from_ron(text).unwrap();
        let d = &reg.defs[0];
        assert_eq!(d.flee_range, 0.0);
        assert!(!d.swims && !d.hops && !d.sleeps && d.prey.is_empty());
        assert!(d.in_biome("Desert") && d.in_biome("Some Future Biome"));
        assert_eq!(d.group, (2, 4));
        assert_eq!(d.weight, 10);
    }

    #[test]
    fn new_animals_are_there_and_biome_bound() {
        let reg = vanilla();
        for n in ["deer", "fox", "rabbit", "bear", "duck"] {
            assert!(reg.id(n).is_some(), "{n}");
        }
        let deer = &reg.defs[reg.id("deer").unwrap()];
        assert!(deer.in_biome("Taiga") && !deer.in_biome("Desert"));
        assert_eq!(reg.defs[reg.id("bear").unwrap()].kind, Kind::Neutral);
        assert_eq!(reg.defs[reg.id("duck").unwrap()].spawn, SpawnRule::Water);
    }

    #[test]
    fn deer_runs_from_the_player() {
        let reg = vanilla();
        let player = Vec3::new(0.5, 10.0, 0.5);
        let (m, _) = run(&reg, "deer", Vec3::new(5.5, 10.0, 0.5), player, 3.0, &Flat);
        assert!((m.body.pos - player).length() > 9.5, "{}", m.body.pos);
    }

    #[test]
    fn bear_is_calm_until_hurt() {
        let reg = vanilla();
        let i = reg.id("bear").unwrap();
        let def = &reg.defs[i];
        let player = Vec3::new(2.0, 10.0, 0.5);
        let mut m = Mob::new(i, def, Vec3::new(0.5, 10.0, 0.5), 3);
        let mut ev = Vec::new();
        for _ in 0..200 {
            m.tick(def, &Flat, player, false, 15, true, 0.02, &mut ev);
        }
        assert!(!ev.iter().any(|e| matches!(e, MobEvent::Hit { .. })));
        m.hurt(1.0, player);
        for _ in 0..300 {
            m.tick(def, &Flat, player, false, 15, true, 0.02, &mut ev);
        }
        assert!(ev.iter().any(|e| matches!(e, MobEvent::Hit { .. })));
    }

    /// Ground up to y=4, water from 5 to 9.
    struct Pond;
    impl Terrain for Pond {
        fn solid(&self, p: IVec3) -> bool {
            p.y < 5
        }
        fn liquid(&self, p: IVec3) -> bool {
            (5..10).contains(&p.y)
        }
    }

    #[test]
    fn duck_floats_on_the_surface() {
        let reg = vanilla();
        let far = Vec3::new(100.0, 10.0, 100.0);
        let (m, _) = run(&reg, "duck", Vec3::new(0.5, 7.0, 0.5), far, 6.0, &Pond);
        assert!(m.body.pos.y > 9.3 && m.body.pos.y < 10.2, "{}", m.body.pos.y);
        let (m, _) = run(&reg, "duck", Vec3::new(0.5, 9.6, 0.5), far, 6.0, &Pond);
        assert!(m.body.pos.y > 9.3 && m.body.pos.y < 10.2, "{}", m.body.pos.y);
    }

    #[test]
    fn hungry_fox_bites_its_prey_and_sleeps_by_day() {
        let reg = vanilla();
        let i = reg.id("fox").unwrap();
        let def = &reg.defs[i];
        let far = Vec3::new(100.0, 10.0, 100.0);
        let mut m = Mob::new(i, def, Vec3::new(0.5, 10.0, 0.5), 5);
        m.fed = 0.0;
        let mut ev = Vec::new();
        for _ in 0..200 {
            m.prey = Some(Vec3::new(4.5, 10.0, 0.5));
            m.tick(def, &Flat, far, false, 15, true, 0.02, &mut ev);
        }
        assert!(ev.iter().any(|e| matches!(e, MobEvent::Bite { .. })));
        m.fed = 100.0;
        m.prey = None;
        m.fall_asleep(30.0);
        let at = m.body.pos;
        for _ in 0..200 {
            m.tick(def, &Flat, far, false, 15, true, 0.02, &mut ev);
        }
        assert!(m.sleeping);
        assert!((m.body.pos - at).length() < 0.3);
    }
}
