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
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnRule {
    /// On grass, in daylight.
    Grass,
    /// Anywhere dark (night or caves).
    Dark,
}

fn d_false() -> bool {
    false
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
    /// An arrow: hit or miss is decided here.
    Shoot { damage: f32, hit: bool },
    Explode { pos: Vec3, power: f32 },
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
    wander: Option<Vec3>,
    idle: f32,
    attack_cd: f32,
    fuse: f32,
    pub aggro: bool,
    burn_tick: f32,
    pub stuck: f32,
    rng: Rng,
}

pub const DEATH_TIME: f32 = 0.9;

impl Mob {
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
            wander: None,
            idle: rng.f32() * 3.0,
            attack_cd: 1.0,
            fuse: 0.0,
            aggro: false,
            burn_tick: 0.0,
            stuck: 0.0,
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
        self.health <= 0.0
    }

    /// Fuse progress 0..1 for exploding mobs (drives the swelling animation).
    pub fn fuse(&self) -> f32 {
        (self.fuse / 1.5).clamp(0.0, 1.0)
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
        let hostile = def.kind == Kind::Hostile && !player_creative;

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

        if hostile && self.aggro {
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
                    out.push(MobEvent::Shoot { damage: def.attack, hit });
                }
            } else if dist > reach {
                wish = flat.normalize_or_zero();
            } else if self.attack_cd <= 0.0 {
                self.attack_cd = 1.0;
                out.push(MobEvent::Hit { damage: def.attack, from: self.body.center() });
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
                    if self.idle <= 0.0 {
                        let a = self.rng.f32() * std::f32::consts::TAU;
                        let r = 3.0 + self.rng.f32() * 6.0;
                        self.wander = Some(self.body.pos + Vec3::new(a.cos() * r, 0.0, a.sin() * r));
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

        // Jump over single blocks when the way is blocked.
        let before = self.body.pos;
        if wish != Vec3::ZERO && self.body.on_ground {
            let ahead = self.body.pos + wish.normalize() * (def.width / 2.0 + 0.2);
            let foot = ahead.floor().as_ivec3();
            if world.solid(foot) && !world.solid(foot + glam::IVec3::Y) && !world.solid(foot + glam::IVec3::Y * 2) {
                jump = true;
            }
        }
        if self.body.in_water {
            jump = true;
        }
        let input = Input { wish, jump, sneak: false, sprint: hostile && self.aggro };
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
}
