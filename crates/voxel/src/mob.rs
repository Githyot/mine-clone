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
    /// Flies instead of walking (ghasts, blazes, the Wither), keeping `hover` blocks above
    /// the ground, or above the player while fighting.
    #[serde(default = "d_false")]
    pub flies: bool,
    #[serde(default)]
    pub hover: f32,
    /// How far a ranged mob shoots from.
    #[serde(default = "d_range")]
    pub range: f32,
    /// Lives under water (fish, whales): swims about below the surface, never climbs out onto
    /// the bank, and flops about when stranded. See `Mob::aquatic_steer`.
    #[serde(default = "d_false")]
    pub aquatic: bool,
    /// Hints for the procedural animation (game/src/anim.rs). Everything it leaves out is
    /// worked out from the model, the mob's size and what kind of creature it is.
    #[serde(default)]
    pub motion: MotionDef,
    /// Copper it drops when it dies (money.rs works it out from its health and attack if unset).
    #[serde(default)]
    pub coins: Option<u32>,
}

/// How a mob moves, for the animation. All optional.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct MotionDef {
    /// Body plan: "biped", "quadruped", "bird", "spider", "hopper", "flyer", "swimmer", "floater".
    #[serde(default)]
    pub style: Option<String>,
    /// 0 light and quick .. 1 big and heavy (slower, rolling steps, small bounce).
    #[serde(default)]
    pub heavy: Option<f32>,
    /// How much the tail wags (dogs 1, cats 0.3).
    #[serde(default)]
    pub wag: Option<f32>,
    /// Lowers its head to graze now and then while standing.
    #[serde(default)]
    pub graze: Option<bool>,
    /// Extra bounce in the step (1 = normal).
    #[serde(default)]
    pub bounce: Option<f32>,
    /// Parts to treat as tails or ears, when guessing from the model gets them wrong.
    #[serde(default)]
    pub tails: Vec<usize>,
    #[serde(default)]
    pub ears: Vec<usize>,
}

fn d_range() -> f32 {
    15.0
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
        Self::from_ron_many(&[text])
    }

    /// Several mob files read as one list (the base game, then mod packs).
    pub fn from_ron_many(texts: &[&str]) -> Result<Self, String> {
        let mut f = MobFile { mobs: Vec::new() };
        for text in texts.iter().filter(|t| !t.trim().is_empty()) {
            let g: MobFile = ron::from_str(text).map_err(|e| format!("mobs.ron: {e}"))?;
            f.mobs.extend(g.mobs);
        }
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

/// What a mob is doing with another creature (or the player) right now. The game's social
/// director (game/src/social.rs) hands these out; `Mob::tick` walks the mob to `Social::at`
/// and the animator poses it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Act {
    #[default]
    None,
    /// Walk up and talk: town folk greet, then take turns speaking.
    Chat,
    /// Nose to nose with a friend (or the player's hand): grooming, nuzzling.
    Nuzzle,
    /// Romping: chasing a playmate or running from it.
    Play,
    /// Keep near `at` (a baby after its parent, a herd animal after its herd).
    Follow,
    /// In love: go to the partner.
    Mate,
    /// Being petted by the player: stand still, eyes shut, tail going.
    Petted,
}

impl Act {
    const ALL: [Act; 7] = [Act::None, Act::Chat, Act::Nuzzle, Act::Play, Act::Follow, Act::Mate, Act::Petted];

    pub fn code(self) -> u8 {
        Self::ALL.iter().position(|a| *a == self).unwrap_or(0) as u8
    }

    pub fn from_code(c: u8) -> Act {
        Self::ALL.get(c as usize).copied().unwrap_or(Act::None)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Social {
    pub act: Act,
    /// Seconds left of this act, and seconds since it began.
    pub t: f32,
    pub age: f32,
    /// Where to go (refreshed by the director while the act lasts).
    pub at: Vec3,
    /// How close to get before standing still.
    pub gap: f32,
    /// The other mob's network id (0: the player or a place).
    pub with: u32,
    /// Talks first in a chat (the other answers).
    pub first: bool,
}

impl Social {
    /// Chatting and this mob has the word now (turns of about 1.7 seconds, after the hello).
    pub fn speaking(&self) -> bool {
        self.act == Act::Chat && self.age > 1.8 && (((self.age - 1.8) / 1.7) as i32 % 2 == 0) == self.first
    }
}

/// How long a baby takes to grow up (seconds).
pub const GROW_TIME: f32 = 600.0;

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
    /// Seconds since it last struck, bit or shot (drives the attack animation).
    pub attack_age: f32,
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
    /// Seconds left soaked (water spells, rain, swimming): fire won't take, frost freezes it
    /// solid and lightning hurts twice as much.
    pub wet: f32,
    /// Seconds left frozen in a block of ice: it can't move or attack.
    pub frozen: f32,
    /// Rain is falling on it (set by the game each tick): undead don't burn in sunlight then.
    pub rained_on: bool,
    /// Lasting harm the game put on it (Terraria-style debuffs: poison, frostburn, ichor...):
    /// effect id and seconds left. The game ticks them; `vuln` is how much more a hit hurts.
    pub debuffs: Vec<(u8, f32)>,
    pub vuln: f32,
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
    /// Set by the game's Apotheosis mod: elite rank (see mods/apotheosis.rs) and how much
    /// harder this mob hits than its kind (world tier, elites).
    pub elite: u8,
    pub power: f32,
    /// Its death was handed to the game (drops, bosses); set once.
    pub counted: bool,
    fire_tick: f32,
    rng: Rng,
    /// Unique for the life of the program: how multiplayer names this mob.
    pub net_id: u32,
    /// Drawn this much bigger than its kind (a giant bee to ride, a little fox pet).
    pub size: f32,
    /// Summoned by an item (pets and Terraria mounts): nothing can hurt or hit it, it hunts
    /// nothing and the game moves it.
    pub ghost: bool,
    /// Who it is with and what they're doing (see `Act`).
    pub social: Social,
    /// Seconds of being frightened of `scare_from` (a cat for a creeper, a wolf for a sheep):
    /// it runs, whatever its kind, and doesn't fight meanwhile.
    pub scare: f32,
    pub scare_from: Vec3,
    /// Seconds of a grudge against the mob `foe` (net id): it fights that one like prey.
    pub grudge: f32,
    pub foe: u32,
    /// The last mob that hurt it (net id; 0 for the player), for packs and guards to answer.
    pub hurt_by: u32,
    /// Seconds left in love (fed its food), and before it can breed again.
    pub love: f32,
    pub breed_cd: f32,
    /// A baby: seconds left until grown (0 for adults), and its parent's net id.
    pub grow: f32,
    pub parent: u32,
}

static NEXT_NET_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

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
            attack_age: 100.0,
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
            wet: 0.0,
            frozen: 0.0,
            rained_on: false,
            debuffs: Vec::new(),
            vuln: 0.0,
            home: None,
            saddled: false,
            ridden: false,
            name: None,
            leashed: false,
            post: None,
            elite: 0,
            power: 1.0,
            counted: false,
            fire_tick: 0.0,
            rng,
            net_id: NEXT_NET_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            size: 1.0,
            ghost: false,
            social: Social::default(),
            scare: 0.0,
            scare_from: pos,
            grudge: 0.0,
            foe: 0,
            hurt_by: 0,
            love: 0.0,
            breed_cd: 0.0,
            grow: 0.0,
            parent: 0,
        }
    }

    pub fn alive(&self) -> bool {
        self.health > 0.0
    }

    /// Returns true if the mob died from this hit.
    pub fn hurt(&mut self, damage: f32, from: Vec3) -> bool {
        if !self.alive() || self.ghost {
            return false;
        }
        self.health -= damage * (1.0 + self.vuln);
        self.hurt_age = 0.0;
        let dir = self.body.pos - from;
        self.body.knockback(dir, 6.0);
        self.aggro = true;
        self.wander = None;
        self.panic = 5.0;
        self.panic_from = from;
        self.sleeping = false;
        // The player, unless a mob's attack says otherwise right after.
        self.hurt_by = 0;
        self.social.act = Act::None;
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

    /// Goes after `prey` this tick: hungry, or settling a grudge.
    pub fn hunting(&self, def: &MobDef) -> bool {
        (self.hungry(def) || self.grudge > 0.0) && self.alive()
    }

    pub fn baby(&self) -> bool {
        self.grow > 0.0
    }

    /// Start (or refresh) a social act.
    pub fn begin(&mut self, act: Act, at: Vec3, gap: f32, secs: f32, with: u32) {
        if self.social.act != act || self.social.with != with {
            self.social = Social { act, t: secs, age: 0.0, at, gap, with, first: false };
        } else {
            self.social.at = at;
        }
    }

    /// Free for something new: awake, calm, not busy with anyone.
    pub fn idle_now(&self) -> bool {
        self.alive() && !self.sleeping && !self.aggro && self.scare <= 0.0 && self.panic <= 0.0 && self.social.act == Act::None && self.prey.is_none() && !self.ridden && !self.ghost
    }

    /// Get a fright from something at `from`: run from it for `secs`.
    pub fn frighten(&mut self, from: Vec3, secs: f32) {
        self.scare = self.scare.max(secs);
        self.scare_from = from;
        self.sleeping = false;
        self.wander = None;
        if self.social.act != Act::Play {
            self.social.act = Act::None;
        }
    }

    /// A baby: smaller, with a smaller box, growing up over `GROW_TIME`.
    pub fn make_baby(&mut self, def: &MobDef, parent: u32) {
        self.grow = GROW_TIME;
        self.parent = parent;
        self.size = 0.5;
        self.body.half_width = def.width * 0.25;
        self.body.height = def.height * 0.5;
        self.fed = 1e9;
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
        self.attack_age += dt;
        self.attack_cd -= dt;
        self.fed -= dt;
        self.panic -= dt;
        self.scare = (self.scare - dt).max(0.0);
        self.grudge = (self.grudge - dt).max(0.0);
        self.love = (self.love - dt).max(0.0);
        self.breed_cd = (self.breed_cd - dt).max(0.0);
        if self.social.act != Act::None {
            self.social.t -= dt;
            self.social.age += dt;
            if self.social.t <= 0.0 || !self.alive() {
                self.social.act = Act::None;
            }
        }
        if self.grow > 0.0 {
            self.grow = (self.grow - dt).max(0.0);
            // Babies stay small most of their childhood, then shoot up.
            let k = 1.0 - (self.grow / GROW_TIME);
            self.size = 0.5 + 0.5 * k * k;
            self.body.half_width = def.width / 2.0 * self.size;
            self.body.height = def.height * self.size;
            if self.grow <= 0.0 {
                self.fed = 20.0 + self.rng.f32() * 40.0;
            }
        }
        if !self.alive() {
            self.dead_age += dt;
            // Keep falling while the death animation plays.
            self.body.step(world, &Input::default(), dt);
            return;
        }
        if self.ghost {
            // The game moves summoned pets and mounts itself.
            self.on_fire = 0.0;
            self.debuffs.clear();
            return;
        }
        let to_player = player - self.body.pos;
        let dist = to_player.length();
        let flat = Vec3::new(to_player.x, 0.0, to_player.z);

        let mut wish = Vec3::ZERO;
        let mut jump = false;
        let mut run = false;
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
        // Water and rain soak it; a soaked mob doesn't keep burning.
        if self.body.in_water || self.rained_on {
            self.wet = self.wet.max(5.0);
        } else {
            self.wet = (self.wet - dt).max(0.0);
        }
        if self.wet > 0.0 && self.on_fire > 0.0 {
            self.on_fire = 0.0;
        }
        // Frozen solid: it only falls until the ice lets go.
        if self.frozen > 0.0 {
            self.frozen = (self.frozen - dt).max(0.0);
            self.body.vel.x = 0.0;
            self.body.vel.z = 0.0;
            self.body.step(world, &Input::default(), dt);
            return;
        }
        if def.burns_in_daylight && daylight && sky_light_at_head >= 14 && !self.body.in_water && !self.rained_on {
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
        // A monster forgets a player it can't hurt (creative), so nothing that reads `aggro`
        // keeps it coming.
        if def.kind == Kind::Hostile && player_creative {
            self.aggro = false;
        }
        // A neutral animal calms down once the player is gone (or can't be hurt).
        if def.kind == Kind::Neutral && (dist > 20.0 || player_creative) {
            self.aggro = false;
        }
        let scared = self.scare > 0.0;
        // A score to settle with another mob comes before the player.
        let feud = self.grudge > 0.0 && self.prey.is_some();
        let fights = self.aggro && !player_creative && def.kind != Kind::Passive && !scared && !feud;
        let flee_player = def.flee_range > 0.0 && dist < def.flee_range && !player_creative;
        let panicking = (self.panic > 0.0 && def.kind == Kind::Passive) || scared;
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
                let (near, far) = (def.range * 0.4, def.range * 0.67);
                if dist < near {
                    wish = -flat.normalize_or_zero();
                } else if dist > far {
                    wish = flat.normalize_or_zero();
                }
                if self.attack_cd <= 0.0 && dist < def.range {
                    self.attack_cd = 2.0 + self.rng.f32();
                    self.attack_age = 0.0;
                    let hit = self.rng.chance((1.0 - dist / 22.0).clamp(0.2, 0.85));
                    let from = self.body.pos + Vec3::Y * (def.height * 0.75);
                    out.push(MobEvent::Shoot { damage: def.attack, from, at: player, hit });
                }
            } else if dist > reach {
                wish = flat.normalize_or_zero();
            } else if self.attack_cd <= 0.0 {
                self.attack_cd = 1.0;
                self.attack_age = 0.0;
                out.push(MobEvent::Hit { damage: def.attack, from: self.body.center() });
            }
        } else if let Some(prey) = self.prey.filter(|_| self.hunting(def) && !scared) {
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
                self.attack_age = 0.0;
                out.push(MobEvent::Bite { damage: def.attack.max(2.0) });
            }
        } else if flee_player || panicking {
            // Run away from whatever scared it.
            self.fuse = 0.0;
            run = true;
            self.wander = None;
            let threat = if scared { self.scare_from } else if panicking { self.panic_from } else { player };
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
        } else if self.social.act != Act::None {
            // With company (social.rs): go to them, then stay facing them.
            self.fuse = 0.0;
            let sc = self.social;
            let d = Vec3::new(sc.at.x - self.body.pos.x, 0.0, sc.at.z - self.body.pos.z);
            let far = d.length();
            match sc.act {
                Act::Play => {
                    run = true;
                    if far > 0.8 {
                        wish = d / far;
                        self.yaw = turn_towards(self.yaw, d.x.atan2(d.z), dt * 8.0);
                    }
                    if self.body.on_ground && self.rng.chance(dt * 0.6) {
                        jump = true;
                    }
                }
                Act::Follow => {
                    if far > sc.gap {
                        run = far > sc.gap * 3.0;
                        wish = d / far * if run { 1.0 } else { 0.8 };
                        self.yaw = turn_towards(self.yaw, d.x.atan2(d.z), dt * 5.0);
                    }
                }
                _ if far > sc.gap => {
                    wish = d / far * 0.8;
                    self.yaw = turn_towards(self.yaw, d.x.atan2(d.z), dt * 5.0);
                }
                _ => {
                    if far > 0.05 {
                        self.yaw = turn_towards(self.yaw, d.x.atan2(d.z), dt * 5.0);
                    }
                    self.head_yaw = turn_towards(self.head_yaw, 0.0, dt * 4.0);
                    let up = sc.at.y + 0.6 - (self.body.pos.y + def.height * self.size * 0.85);
                    self.head_pitch = turn_towards(self.head_pitch, (-up / far.max(0.5)).atan().clamp(-0.6, 0.6), dt * 4.0);
                }
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
        if self.body.in_water && !def.aquatic {
            if def.swims {
                // Float: paddle up only while the waterline is above the belly.
                if world.liquid((self.body.pos + Vec3::Y * 0.35).floor().as_ivec3()) {
                    jump = true;
                }
            } else {
                jump = true;
            }
        }
        let mut sneak = false;
        if def.flies && def.hover <= 0.0 {
            (wish, jump, sneak) = self.fly(def, world, player, fights, wish);
        }
        if def.aquatic {
            self.aquatic_steer(world, &mut wish, &mut jump);
        }
        let input = Input { wish, jump, sneak, sprint: run, crawl: false };
        if def.flies && def.hover > 0.0 {
            self.fly_step(def, world, wish, if fights { Some(player) } else { None }, run, dt);
        } else {
            self.body.step(world, &input, dt);
        }
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

impl Mob {
    /// Flying: glide toward `wish` at the mob's speed and hold a height, `hover` blocks over the
    /// ground below (or over the player it fights), bobbing a little.
    fn fly_step(&mut self, def: &MobDef, world: &impl Terrain, wish: Vec3, target: Option<Vec3>, run: bool, dt: f32) {
        let at = self.body.pos;
        let ground = (0..40).map(|k| at.y.floor() as i32 - k).find(|&y| world.solid(glam::IVec3::new(at.x.floor() as i32, y, at.z.floor() as i32)));
        let bob = (self.age * 1.3 + self.phase).sin() * 0.6;
        let want_y = match (target, ground) {
            (Some(p), _) => p.y + def.hover + bob,
            (None, Some(g)) => g as f32 + 1.0 + def.hover + bob,
            (None, None) => at.y - 1.0,
        };
        let speed = def.speed * if run { 1.3 } else { 1.0 } * if self.chill > 0.0 { 0.4 } else { 1.0 };
        let mut v = Vec3::new(wish.x, 0.0, wish.z) * speed;
        v.y = ((want_y - at.y) * 1.5).clamp(-speed, speed);
        // Ease into the new speed so turns are smooth.
        let k = (dt * 4.0).min(1.0);
        let vel = self.body.vel + (v - self.body.vel) * k;
        self.body.fly(world, vel, dt);
    }

    /// Flying: climb or sink toward the wanted height (a few blocks over the ground, bobbing,
    /// or level with the player when fighting) and slow the wish down to the mob's own speed.
    fn fly(&mut self, def: &MobDef, world: &impl Terrain, player: Vec3, fights: bool, wish: Vec3) -> (Vec3, bool, bool) {
        self.body.flying = true;
        let ground = (0..40).map(|k| (self.body.pos - Vec3::Y * k as f32).floor().as_ivec3()).find(|&p| world.solid(p)).map(|p| p.y as f32 + 1.0);
        let want = if fights {
            player.y + if def.ranged { 4.0 } else { 0.5 }
        } else {
            let hover = 3.0 + (self.phase.sin() * 0.5 + 0.5) * 5.0 + (self.age * 0.4 + self.phase).sin();
            ground.map(|g| g + hover).unwrap_or(self.body.pos.y)
        };
        let dy = want - self.body.pos.y;
        let k = def.speed / crate::physics::FLY_SPEED;
        (wish * k, dy > 0.5, dy < -0.5)
    }

    /// Water creatures (`MobDef::aquatic`): in the water they keep off the bottom and away from
    /// the surface, and never swim out onto land (a wander goal or chase that leads out of the
    /// water is dropped); stranded on land they flop about until they reach water again.
    fn aquatic_steer(&mut self, world: &impl Terrain, wish: &mut Vec3, jump: &mut bool) {
        let at = |p: Vec3| p.floor().as_ivec3();
        if self.body.in_water {
            if *wish != Vec3::ZERO {
                let ahead = self.body.pos + Vec3::Y * 0.3 + wish.normalize() * 1.2;
                if !world.liquid(at(ahead)) {
                    *wish = Vec3::ZERO;
                    self.wander = None;
                    self.idle = 0.5 + self.rng.f32();
                }
            }
            // Rise off the bottom; sink back when close under the surface.
            let near_bottom = world.solid(at(self.body.pos - Vec3::Y * 0.6));
            let near_top = !world.liquid(at(self.body.pos + Vec3::Y * 1.4));
            *jump = near_bottom && !near_top;
        } else {
            // Stranded: flop about now and then.
            *jump = self.body.on_ground && self.rng.chance(0.04);
            if *jump {
                let a = self.rng.f32() * std::f32::consts::TAU;
                *wish = Vec3::new(a.cos(), 0.0, a.sin());
            }
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

    #[test]
    fn monsters_leave_a_creative_player_alone() {
        let reg = vanilla();
        let player = Vec3::new(4.0, 10.0, 0.5);
        for name in ["zombie", "skeleton", "stalker", "spider"] {
            let z = reg.id(name).unwrap();
            let def = &reg.defs[z];
            let mut m = Mob::new(z, def, Vec3::new(0.5, 10.0, 0.5), 1);
            // Already angry (spawned by a trial spawner, say) when the player is in creative.
            m.aggro = true;
            let mut ev = Vec::new();
            for _ in 0..400 {
                m.tick(def, &Flat, player, true, 0, false, 0.02, &mut ev);
            }
            assert!(!m.aggro, "{name} still angry");
            assert!(!ev.iter().any(|e| matches!(e, MobEvent::Hit { .. } | MobEvent::Shoot { .. } | MobEvent::Explode { .. })), "{name} attacked");
            assert_eq!(m.fuse, 0.0, "{name}");
        }
    }

    #[test]
    fn zombie_burns_in_sun_but_not_in_rain() {
        let reg = vanilla();
        let z = reg.id("zombie").unwrap();
        let def = &reg.defs[z];
        let far = Vec3::new(500.0, 10.0, 0.5);
        for rain in [false, true] {
            let mut m = Mob::new(z, def, Vec3::new(0.5, 10.0, 0.5), 3);
            let mut ev = Vec::new();
            for _ in 0..200 {
                m.rained_on = rain;
                m.tick(def, &Flat, far, false, 15, true, 0.02, &mut ev);
            }
            let burned = ev.iter().any(|e| matches!(e, MobEvent::Burning));
            assert_eq!(burned, !rain, "rain={rain}");
        }
    }

    #[test]
    fn a_frightened_creeper_runs_instead_of_chasing() {
        let reg = vanilla();
        let c = reg.id("stalker").unwrap();
        let def = &reg.defs[c];
        let mut m = Mob::new(c, def, Vec3::new(0.5, 10.0, 0.5), 2);
        let player = Vec3::new(6.0, 10.0, 0.5);
        let mut ev = Vec::new();
        for _ in 0..100 {
            m.frighten(Vec3::new(2.0, 10.0, 0.5), 1.0);
            m.tick(def, &Flat, player, false, 0, false, 0.02, &mut ev);
        }
        assert!(m.body.pos.x < -0.5, "{}", m.body.pos.x);
        assert!(!ev.iter().any(|e| matches!(e, MobEvent::Explode { .. })));
    }

    #[test]
    fn chatting_mobs_walk_up_and_take_turns() {
        let reg = vanilla();
        let z = reg.id("cow").unwrap();
        let def = &reg.defs[z];
        let mut m = Mob::new(z, def, Vec3::new(0.5, 10.0, 0.5), 2);
        let far = Vec3::new(500.0, 10.0, 0.5);
        m.begin(Act::Nuzzle, Vec3::new(6.0, 10.0, 0.5), 1.0, 10.0, 7);
        let mut ev = Vec::new();
        for _ in 0..300 {
            m.tick(def, &Flat, far, false, 15, true, 0.02, &mut ev);
        }
        assert!((m.body.pos.x - 5.0).abs() < 0.8, "{}", m.body.pos.x);
        assert_eq!(m.social.act, Act::Nuzzle);
        let mut a = Social { act: Act::Chat, first: true, ..Default::default() };
        let mut b = Social { act: Act::Chat, ..Default::default() };
        let mut turns = (0, 0);
        for k in 0..100 {
            a.age = k as f32 * 0.1;
            b.age = a.age;
            assert!(!(a.speaking() && b.speaking()));
            turns.0 += a.speaking() as u32;
            turns.1 += b.speaking() as u32;
        }
        assert!(turns.0 > 20 && turns.1 > 20, "{turns:?}");
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

    fn with_mods() -> MobRegistry {
        MobRegistry::from_ron_many(&[include_str!("../../../packs/vanilla/mobs.ron"), include_str!("../../../packs/vanilla/mods/mobs.ron")]).unwrap()
    }

    #[test]
    fn mod_creatures_load_with_unique_names_and_known_prey() {
        let reg = with_mods();
        for n in ["gorilla", "crocodile", "cachalot_whale", "snail", "giraffe", "mutant_stalker", "void_walker_jungle"] {
            assert!(reg.id(n).is_some(), "{n}");
        }
        let mut names: Vec<&str> = reg.defs.iter().map(|d| d.name.as_str()).collect();
        names.sort();
        let n = names.len();
        names.dedup();
        assert_eq!(n, names.len(), "duplicate mob names");
        for d in &reg.defs {
            for p in &d.prey {
                assert!(reg.id(p).is_some(), "{} hunts unknown {p}", d.name);
            }
            // Mini-bosses never get the boss bar.
            assert!(!(d.name.starts_with("mutant_") && d.boss), "{}", d.name);
        }
        assert!(reg.defs[reg.id("orca").unwrap()].aquatic);
    }

    /// Water from y=5 to 9 for x < 3, dry land (ground at y=9) beyond.
    struct Lake;
    impl Terrain for Lake {
        fn solid(&self, p: IVec3) -> bool {
            p.y < 5 || (p.x >= 3 && p.y < 10)
        }
        fn liquid(&self, p: IVec3) -> bool {
            p.x < 3 && (5..10).contains(&p.y)
        }
    }

    #[test]
    fn fish_stay_in_the_water() {
        let reg = with_mods();
        let i = reg.id("bass").unwrap();
        let def = &reg.defs[i];
        let mut m = Mob::new(i, def, Vec3::new(1.5, 7.0, 0.5), 9);
        let far = Vec3::new(100.0, 10.0, 100.0);
        let mut ev = Vec::new();
        for k in 0..3000 {
            if k % 300 == 0 {
                // Tempt it towards the shore.
                m.go_to(Vec3::new(8.5, 10.0, 0.5), 5.0);
            }
            m.tick(def, &Lake, far, false, 15, true, 0.02, &mut ev);
            assert!(m.body.pos.x < 3.6, "swam out at {}", m.body.pos);
        }
        assert!(m.body.in_water && m.body.pos.y < 9.5 && m.body.pos.y > 4.9, "{}", m.body.pos);
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
