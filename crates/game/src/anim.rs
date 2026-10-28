//! Lively mob animation (the Fresh Animations / Detailed Animations idea), all procedural and
//! cheap: a few sines and springs per mob per frame on the CPU, then the usual part matrices.
//!
//! Each mob definition gets a `Rig` once: which box is the body, the head, the ears, the tail,
//! which legs are in front, and a body plan (biped, quadruped, bird, spider, hopper, flyer,
//! swimmer). Personality comes from the model's size and kind, a few name hints and the
//! optional `motion` block in mobs.ron. Each living mob keeps a small `MobAnim` of smoothed
//! values between frames (walk amount, turn rate, head look, landing spring, tail and ear
//! springs, grazing, greeting) so poses blend instead of snapping.
//!
//! Test hooks for screenshots: MC_POSE=walk|run|idle|graze|attack|hurt|jump|fall|swim|sleep|
//! greet|land|aim forces a pose on every mob; MC_ANIM_T=seconds sets the animation clock.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};

use glam::{IVec3, Mat4, Quat, Vec3};
use voxel::mob::{Act, Anim, Kind, Mob, MobDef, DEATH_TIME};

use crate::entity::{eyes_shut_test, Bones, EYELID_PART, MAX_PARTS, MOUTH_PART};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Style {
    Biped,
    Quadruped,
    Bird,
    Spider,
    /// No legs, moves in hops (jellies, slimes).
    Hopper,
    Flyer,
    /// Lives under water: the whole body undulates.
    Swimmer,
    /// No legs and no hops: drifts and wobbles.
    Floater,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Role {
    Fixed,
    Body,
    Head,
    /// On the head; side is -1 or 1.
    Ear(f32),
    Leg { pair: u8, front: bool, side: f32 },
    Arm(u8),
    ArmForward(u8),
    Wing(u8),
    Tail,
    Spider(u8),
}

/// How one kind of mob is put together and how it likes to move.
#[derive(Clone, Debug)]
pub struct Rig {
    roles: Vec<Role>,
    pivots: Vec<Vec3>,
    pub style: Style,
    /// Where the legs meet the body (bipeds bend the upper body around this).
    hip: Vec3,
    centre: Vec3,
    head: Option<usize>,
    leg_h: f32,
    /// 0 light and quick .. 1 big and heavy.
    heavy: f32,
    wag: f32,
    /// Tail swish speed (dogs fast, cats slow).
    wag_rate: f32,
    graze: bool,
    bounce: f32,
    shamble: bool,
    rattle: bool,
    town: bool,
    /// Moves its head in quick little jerks (birds).
    jerky: bool,
}

fn has(name: &str, words: &[&str]) -> bool {
    words.iter().any(|w| name.contains(w))
}

fn bounds(p: &voxel::mob::PartDef) -> (Vec3, Vec3) {
    let lo = Vec3::new(p.origin.0 as f32, p.origin.1 as f32, p.origin.2 as f32);
    (lo, lo + Vec3::new(p.size.0 as f32, p.size.1 as f32, p.size.2 as f32))
}

fn volume(p: &voxel::mob::PartDef) -> f32 {
    p.size.0 as f32 * p.size.1 as f32 * p.size.2 as f32
}

impl Rig {
    pub fn new(def: &MobDef) -> Rig {
        let n = def.parts.len().min(MAX_PARTS);
        let parts = &def.parts[..n];
        let pivot_of = |i: usize| {
            let p = &parts[i].pivot;
            Vec3::new(p.0 as f32, p.1 as f32, p.2 as f32)
        };
        let count = |f: fn(&Anim) -> bool| parts.iter().filter(|p| f(&p.anim)).count();
        let legs = count(|a| matches!(a, Anim::Leg(_)));
        let arms = count(|a| matches!(a, Anim::Arm(_) | Anim::ArmForward(_)));
        let wings = count(|a| matches!(a, Anim::Wing(_)));
        let spider = count(|a| matches!(a, Anim::SpiderLeg(_)));
        let name = def.name.as_str();

        // The body: the biggest box that isn't animated.
        let body = (0..n).filter(|&i| parts[i].anim == Anim::None).max_by(|&a, &b| volume(&parts[a]).total_cmp(&volume(&parts[b])));
        let head = (0..n).filter(|&i| parts[i].anim == Anim::Head).max_by(|&a, &b| volume(&parts[a]).total_cmp(&volume(&parts[b])));
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in parts {
            let (a, b) = bounds(p);
            lo = lo.min(a);
            hi = hi.max(b);
        }
        if n == 0 {
            lo = Vec3::ZERO;
            hi = Vec3::ONE;
        }
        let leg_h = parts.iter().filter(|p| matches!(p.anim, Anim::Leg(_))).map(|p| p.size.1 as f32).fold(0.0, f32::max);
        let (body_lo, body_hi) = body.map(|b| bounds(&parts[b])).unwrap_or((lo, hi));
        let body_mid = (body_lo + body_hi) * 0.5;

        let mut style = if def.aquatic {
            Style::Swimmer
        } else if def.flies {
            Style::Flyer
        } else if spider > 0 {
            Style::Spider
        } else if legs == 0 && def.hops {
            Style::Hopper
        } else if legs == 0 && arms == 0 {
            Style::Floater
        } else if wings > 0 && legs == 2 {
            Style::Bird
        } else if legs >= 4 {
            Style::Quadruped
        } else {
            Style::Biped
        };
        if let Some(s) = &def.motion.style {
            style = match s.as_str() {
                "biped" => Style::Biped,
                "quadruped" => Style::Quadruped,
                "bird" => Style::Bird,
                "spider" => Style::Spider,
                "hopper" => Style::Hopper,
                "flyer" => Style::Flyer,
                "swimmer" => Style::Swimmer,
                "floater" => Style::Floater,
                _ => style,
            };
        }

        let head_hi = head.map(|h| bounds(&parts[h]).1.y).unwrap_or(hi.y);
        let mut roles = Vec::with_capacity(n);
        let mut pivots = Vec::with_capacity(n);
        for (i, p) in parts.iter().enumerate() {
            let (a, b) = bounds(p);
            let mid = (a + b) * 0.5;
            let mut piv = pivot_of(i);
            let small = volume(p) <= 16.0;
            let role = match p.anim {
                Anim::None if Some(i) == body => Role::Body,
                // A tail: something behind the body's back that isn't the body.
                Anim::None if def.motion.tails.contains(&i) || (body.is_some() && !def.motion.ears.contains(&i) && mid.z < body_lo.z + 0.5 && a.y >= body_lo.y - 2.0 && volume(p) < volume(&parts[body.unwrap()]) * 0.6) => {
                    // Hinged where it meets the body: a long tail at its front end, a hanging
                    // one at its top.
                    piv = if p.size.2 >= p.size.1 { Vec3::new(mid.x, mid.y, b.z.min(body_lo.z + 1.0)) } else { Vec3::new(mid.x, b.y, mid.z) };
                    Role::Tail
                }
                Anim::None => Role::Fixed,
                Anim::Head if def.motion.ears.contains(&i) || (small && Some(i) != head && a.y >= head_hi - 0.5 && style != Style::Flyer) => {
                    piv = Vec3::new(mid.x, a.y, mid.z);
                    Role::Ear(if mid.x < 0.0 { -1.0 } else { 1.0 })
                }
                Anim::Head => Role::Head,
                Anim::Leg(k) => Role::Leg { pair: k, front: piv.z >= body_mid.z, side: if piv.x < 0.0 { -1.0 } else { 1.0 } },
                Anim::Arm(k) => Role::Arm(k),
                Anim::ArmForward(k) => Role::ArmForward(k),
                Anim::Wing(k) => Role::Wing(k),
                Anim::SpiderLeg(k) => Role::Spider(k),
            };
            roles.push(role);
            pivots.push(piv);
        }

        // Size sets the temperament: big animals roll along slowly, small ones scurry.
        let s = def.scale.max(0.1);
        let mass = def.width * def.width * def.height;
        let mut heavy = ((mass - 0.35) / 3.0).clamp(0.0, 1.0);
        if s > 1.5 {
            heavy = heavy.max(((s - 1.0) / 2.0).min(1.0));
        }
        let heavy = def.motion.heavy.unwrap_or(heavy);
        let dog = has(name, &["wolf", "dog", "fox", "hound", "pup", "jackal", "coyote"]);
        let cat = has(name, &["cat", "ocelot", "lion", "tiger", "leopard", "panther", "lynx"]);
        let wag = def.motion.wag.unwrap_or(if dog { 1.0 } else if cat { 0.35 } else { 0.15 });
        let wag_rate = if dog { 11.0 } else if cat { 2.2 } else { 3.0 };
        let graze = def.motion.graze.unwrap_or(style == Style::Quadruped && def.kind == Kind::Passive && def.prey.is_empty() && !def.town && head.is_some() && !cat);
        Rig {
            roles,
            pivots,
            style,
            hip: Vec3::new(0.0, if leg_h > 0.0 { leg_h } else { lo.y + (hi.y - lo.y) * 0.4 }, body_mid.z),
            centre: Vec3::new(body_mid.x, (lo.y + hi.y) * 0.5, body_mid.z),
            head,
            leg_h,
            heavy,
            wag,
            wag_rate,
            graze,
            bounce: def.motion.bounce.unwrap_or(1.0),
            shamble: arms > 0 && parts.iter().any(|p| matches!(p.anim, Anim::ArmForward(_))) || has(name, &["zombie", "husk", "drowned", "ghoul"]),
            rattle: has(name, &["skeleton", "stray", "bogged", "wither_sk"]),
            town: def.town,
            jerky: style == Style::Bird || has(name, &["chicken", "parrot", "duck", "penguin", "bird"]),
        }
    }
}

/// Test hooks, read once.
struct Hooks {
    pose: Option<String>,
    time: Option<f32>,
}

fn hooks() -> &'static Hooks {
    static H: std::sync::OnceLock<Hooks> = std::sync::OnceLock::new();
    H.get_or_init(|| Hooks { pose: std::env::var("MC_POSE").ok(), time: std::env::var("MC_ANIM_T").ok().and_then(|v| v.parse().ok()) })
}

fn wrap(a: f32) -> f32 {
    (a + PI).rem_euclid(2.0 * PI) - PI
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Smoothed values one mob carries between frames.
#[derive(Clone, Default, Debug)]
pub struct MobAnim {
    seen: bool,
    /// Its own clock (idle motions, breathing).
    t: f32,
    /// Stride phase, walk amount 0..1.4 and running 0..1.
    stride: f32,
    amp: f32,
    run: f32,
    yaw: f32,
    /// Turning speed, radians per second (for banking and the lagging tail).
    turn: f32,
    head_yaw: f32,
    head_pitch: f32,
    was_ground: bool,
    air: f32,
    fall: f32,
    vy: f32,
    /// Landing spring: positive is squashed flat.
    squash: f32,
    squash_v: f32,
    swim: f32,
    tail: f32,
    tail_v: f32,
    ear: f32,
    ear_v: f32,
    graze: f32,
    /// Seconds of waving left, and the wait before it waves again.
    greet: f32,
    greet_cd: f32,
    /// Seconds since hurt / since it attacked.
    hurt: f32,
    attack: f32,
    /// How near the player is, 0 far .. 1 close (tails wag more).
    near: f32,
    /// Social poses (social.rs), each eased 0..1: in a chat, speaking now, nose to nose,
    /// being petted, a play bow.
    chat: f32,
    talk: f32,
    nuzzle: f32,
    petted: f32,
    bow: f32,
    /// The chat partner it already waved to.
    waved: u32,
}

impl MobAnim {
    pub fn update(&mut self, mob: &Mob, def: &MobDef, rig: &Rig, player: Vec3, dt: f32) {
        if !self.seen {
            self.seen = true;
            self.yaw = mob.yaw;
            self.head_yaw = mob.head_yaw;
            self.head_pitch = mob.head_pitch;
            self.was_ground = true;
            self.greet_cd = 3.0 + mob.phase;
        }
        let k = |rate: f32| 1.0 - (-rate * dt).exp();
        self.t += dt;
        self.stride = mob.walk;
        self.hurt = mob.hurt_age;
        self.attack = mob.attack_age;
        self.vy = mob.body.vel.y;
        let rel = mob.walk_speed / def.speed.max(0.1);
        self.amp += (rel.min(1.4) - self.amp) * k(6.0);
        let running = ((rel - 1.08) / 0.3).clamp(0.0, 1.0);
        self.run += (running - self.run) * k(4.0);

        let dyaw = wrap(mob.yaw - self.yaw);
        self.yaw = mob.yaw;
        let rate = if dt > 1e-4 { (dyaw / dt).clamp(-6.0, 6.0) } else { 0.0 };
        self.turn += (rate - self.turn) * k(5.0);
        // The head eases toward where the brain wants to look.
        self.head_yaw += wrap(mob.head_yaw - self.head_yaw) * k(if rig.jerky { 25.0 } else { 9.0 });
        self.head_pitch += (mob.head_pitch - self.head_pitch) * k(9.0);

        let ground = mob.body.on_ground || mob.body.in_water || def.flies;
        if ground {
            if !self.was_ground && self.air > 0.12 {
                // Landing: kick the squash spring, harder after a longer drop.
                let hit = (self.fall / 9.0).clamp(0.25, 1.4) * if rig.style == Style::Hopper { 5.0 } else { 3.2 };
                self.squash_v += hit;
            }
            self.air = 0.0;
            self.fall = 0.0;
        } else {
            self.air += dt;
            self.fall = self.fall.max(-mob.body.vel.y);
        }
        self.was_ground = ground;
        // Springs, stepped finely so they stay stable at any frame rate.
        let (stiff, damp) = if rig.style == Style::Hopper { (150.0, 9.0) } else { (190.0 - rig.heavy * 70.0, 13.0) };
        let tail_target = -self.turn * 0.16 + (mob.body.vel.y * 0.04).clamp(-0.3, 0.3);
        let ear_target = -(self.amp.min(1.0) * 0.25 + self.run * 0.35) - self.turn.abs() * 0.04;
        let mut left = dt.min(0.25);
        while left > 0.0 {
            let h = left.min(1.0 / 120.0);
            left -= h;
            self.squash_v += (-stiff * self.squash - damp * self.squash_v) * h;
            self.squash = (self.squash + self.squash_v * h).clamp(-0.45, 0.5);
            self.tail_v += (-70.0 * (self.tail - tail_target) - 7.0 * self.tail_v) * h;
            self.tail += self.tail_v * h;
            self.ear_v += (-120.0 * (self.ear - ear_target) - 9.0 * self.ear_v) * h;
            self.ear += self.ear_v * h;
        }
        let wet = mob.body.in_water && !def.aquatic && !def.swims && !def.flies;
        self.swim += ((wet as u8 as f32) - self.swim) * k(4.0);

        // Grazers put their head down for a few seconds now and then while standing about.
        let calm = mob.alive() && !mob.sleeping && !mob.aggro && self.amp < 0.12 && mob.hurt_age > 3.0;
        let cycle = (self.t + mob.phase * 5.0) % (11.0 + mob.phase.fract() * 6.0);
        let want = (rig.graze && calm && cycle < 4.5) as u8 as f32;
        self.graze += (want - self.graze) * k(2.5);

        let d = (player - mob.body.pos).length();
        self.near += (((6.0 - d) / 4.0).clamp(0.0, 1.0) - self.near) * k(2.0);
        // Town folk wave when the player walks up to them.
        self.greet = (self.greet - dt).max(0.0);
        self.greet_cd -= dt;
        if rig.town && mob.alive() && d < 4.5 && self.greet_cd <= 0.0 && self.amp < 0.3 {
            self.greet = 1.8;
            self.greet_cd = 30.0;
        } else if rig.town && d > 10.0 {
            self.greet_cd = self.greet_cd.min(2.0);
        }
        // Mobs among themselves.
        let sc = &mob.social;
        let act = if mob.alive() { sc.act } else { Act::None };
        let chatting = act == Act::Chat;
        self.chat += ((chatting as u8 as f32) - self.chat) * k(4.0);
        self.talk += ((sc.speaking() as u8 as f32) - self.talk) * k(10.0);
        self.nuzzle += (((act == Act::Nuzzle || act == Act::Mate) && self.amp < 0.3) as u8 as f32 - self.nuzzle) * k(4.0);
        self.petted += ((act == Act::Petted) as u8 as f32 - self.petted) * k(6.0);
        let bowing = act == Act::Play && sc.first && sc.age < 0.7;
        self.bow += (bowing as u8 as f32 - self.bow) * k(10.0);
        if chatting && self.waved != sc.with && self.amp < 0.3 && mob.body.pos.distance(sc.at) < sc.gap + 0.8 {
            // Hello first.
            self.waved = sc.with;
            self.greet = 1.8;
            self.greet_cd = self.greet_cd.max(8.0);
        }
        if act == Act::Petted || act == Act::Play {
            self.near = 1.0;
        }
        self.apply_hooks();
    }

    /// Forced poses for test pictures.
    fn apply_hooks(&mut self) {
        let h = hooks();
        if let Some(t) = h.time {
            self.t = t;
            self.stride = t * 4.0;
        }
        let Some(pose) = &h.pose else { return };
        match pose.as_str() {
            "walk" => self.amp = 1.0,
            "run" => {
                self.amp = 1.35;
                self.run = 1.0;
            }
            "graze" => self.graze = 1.0,
            "attack" => self.attack = 0.16,
            "hurt" => self.hurt = 0.08,
            "jump" => {
                self.air = 0.25;
                self.was_ground = false;
                self.vy = 5.0;
            }
            "fall" => {
                self.air = 0.8;
                self.was_ground = false;
                self.vy = -8.0;
            }
            "land" => self.squash = 0.3,
            "swim" => {
                self.swim = 1.0;
                self.amp = 0.6;
            }
            "greet" => self.greet = 1.0,
            "talk" => {
                self.chat = 1.0;
                self.talk = 1.0;
            }
            "nuzzle" => self.nuzzle = 1.0,
            "petted" => {
                self.petted = 1.0;
                self.near = 1.0;
            }
            "bow" => {
                self.bow = 1.0;
                self.near = 1.0;
            }
            "turn" => {
                self.amp = 1.0;
                self.turn = 2.5;
                self.tail = -0.4;
            }
            _ => {}
        }
    }
}

/// The pose overrides a hook asks for that live outside `MobAnim`.
fn hook_pose(p: &str) -> bool {
    hooks().pose.as_deref() == Some(p)
}

/// Mob animation state for everything alive, by network id.
#[derive(Default)]
pub struct Animator {
    pub rigs: Vec<Rig>,
    states: HashMap<u32, MobAnim>,
    prune: f32,
}

impl Animator {
    /// Advance every mob's animation by a frame.
    pub fn update(&mut self, defs: &[MobDef], mobs: &[Mob], player: Vec3, dt: f32) {
        while self.rigs.len() < defs.len() {
            self.rigs.push(Rig::new(&defs[self.rigs.len()]));
        }
        for m in mobs {
            if m.def >= defs.len() {
                continue;
            }
            // Only the ones near enough to see are worth the trouble.
            if (m.body.pos - player).length_squared() > 160.0 * 160.0 {
                continue;
            }
            self.states.entry(m.net_id).or_default().update(m, &defs[m.def], &self.rigs[m.def], player, dt);
        }
        self.prune -= dt;
        if self.prune <= 0.0 || self.states.len() > mobs.len() * 2 + 64 {
            self.prune = 5.0;
            let ids: std::collections::HashSet<u32> = mobs.iter().map(|m| m.net_id).collect();
            self.states.retain(|id, _| ids.contains(id));
        }
    }

    pub fn bones(&mut self, mob: &Mob, def: &MobDef, def_index: usize, cam_block: IVec3, sky: f32, blk: f32) -> Bones {
        while self.rigs.len() <= def_index {
            // Only reachable before the first update; the rig for any index will do then.
            self.rigs.push(Rig::new(def));
        }
        let rig = &self.rigs[def_index];
        match self.states.get(&mob.net_id) {
            Some(st) => bones_for(mob, def, rig, st, cam_block, sky, blk),
            None => {
                let mut st = MobAnim::default();
                st.update(mob, def, rig, Vec3::splat(1e6), 0.0);
                bones_for(mob, def, rig, &st, cam_block, sky, blk)
            }
        }
    }

    /// Like `bones`, without touching the animator: for mobs drawn outside the mob list
    /// (wagon teams) or drawn again (a playing musician).
    pub fn bones_now(&self, mob: &Mob, def: &MobDef, def_index: usize, cam_block: IVec3, sky: f32, blk: f32) -> Bones {
        let fresh;
        let rig = match self.rigs.get(def_index) {
            Some(r) => r,
            None => {
                fresh = Rig::new(def);
                &fresh
            }
        };
        match self.states.get(&mob.net_id) {
            Some(st) => bones_for(mob, def, rig, st, cam_block, sky, blk),
            None => {
                let mut st = MobAnim::default();
                st.update(mob, def, rig, Vec3::splat(1e6), 0.0);
                bones_for(mob, def, rig, &st, cam_block, sky, blk)
            }
        }
    }
}

/// Part matrices for one mob at this moment.
pub fn bones_for(mob: &Mob, def: &MobDef, rig: &Rig, st: &MobAnim, cam_block: IVec3, sky: f32, blk: f32) -> Bones {
    let mut b = Bones { light: [sky, blk, if mob.hurt_age < 0.3 { 1.0 } else { 0.0 }, 0.0], ..Default::default() };
    let swell = mob.fuse();
    if swell > 0.0 {
        b.light[3] = swell * 0.5 * ((mob.age * 18.0).sin() * 0.5 + 0.5);
    }
    let alive = mob.alive();
    let sleeping = (mob.sleeping || hook_pose("sleep")) && alive;
    let awake = alive && !sleeping;
    let style = rig.style;
    let clock = st.t + mob.phase * 3.0;
    let t = st.stride;
    let amp = if awake { st.amp } else { 0.0 };
    let run = if awake { st.run } else { 0.0 };
    let idle = (1.0 - amp).clamp(0.0, 1.0) * awake as u8 as f32;
    let breathe = (clock * if sleeping { 0.9 } else { 1.7 - rig.heavy * 0.6 }).sin();
    let airborne = alive && st.air > 0.15 && st.swim < 0.5 && !matches!(style, Style::Flyer | Style::Swimmer);
    let hurt = if alive { (1.0 - st.hurt / 0.35).max(0.0) } else { 0.0 };
    // Attack: a quick wind-up and strike over half a second.
    let strike = if alive && st.attack < 0.5 { (st.attack / 0.5 * PI).sin() } else { 0.0 };
    let aiming = alive && def.ranged && (mob.aggro || hook_pose("aim")) && def.kind == Kind::Hostile;
    let swim = st.swim * alive as u8 as f32;
    let greet = smooth(st.greet.min(1.8 - st.greet).min(0.35) / 0.35) * alive as u8 as f32;

    // World placement.
    let rel = mob.body.pos - cam_block.as_vec3();
    let mut model = Mat4::from_translation(rel) * Mat4::from_rotation_y(mob.yaw);
    if !alive {
        // The knees give, then it topples sideways and settles with a little bounce.
        let d = (mob.dead_age / DEATH_TIME).clamp(0.0, 1.0);
        let buckle = smooth(d / 0.3);
        let fall = smooth((d - 0.15) / 0.75);
        let settle = if d > 0.85 { ((d - 0.85) / 0.15 * PI).sin() * 0.08 } else { 0.0 };
        model = model * Mat4::from_translation(Vec3::new(0.0, -fall * 0.4 - buckle * 0.1, 0.0)) * Mat4::from_rotation_z(fall * FRAC_PI_2 - settle) * Mat4::from_rotation_x(buckle * 0.15 * (1.0 - fall));
    }
    let scale = def.scale * mob.size / 16.0 * (1.0 + swell * 0.3);
    model *= Mat4::from_scale(Vec3::splat(scale));

    // The whole body, in model pixels around the feet.
    let mut root = Mat4::IDENTITY;
    if sleeping {
        // Asleep: legs folded flat, the body resting on the ground, slow deep breaths.
        root *= Mat4::from_translation(Vec3::new(0.0, -(rig.leg_h - 1.0).max(0.0), 0.0));
    }
    if awake {
        let bounce = rig.bounce * (1.0 - rig.heavy * 0.45);
        let step = t.cos().abs();
        let mut bob = match style {
            Style::Biped => step * (0.9 + run * 0.9) * amp.min(1.3) * bounce,
            Style::Quadruped => step * 0.8 * amp.min(1.3) * bounce + (t * 2.0).sin() * 0.5 * run,
            Style::Bird => (t * 2.0).cos().abs() * 0.9 * amp.min(1.2),
            Style::Spider => (t * 4.0).sin().abs() * 0.35 * amp,
            Style::Flyer => (clock * (2.4 - rig.heavy)).sin() * (1.4 - rig.heavy * 0.6),
            Style::Floater => (clock * 1.6).sin() * 0.8,
            _ => 0.0,
        };
        bob += breathe * 0.12 * idle;
        if def.swims && mob.body.in_water {
            // Floating: riding the ripples.
            bob = (clock * 1.6).sin() * 0.5;
        }
        root *= Mat4::from_translation(Vec3::new(0.0, bob, 0.0));

        // Leaning into turns; flyers bank hard, walkers just a little.
        let bank_k = match style {
            Style::Flyer => 0.12,
            Style::Swimmer => 0.06,
            _ => 0.035 * amp.min(1.0) + 0.02 * run,
        };
        let bank = (-st.turn * bank_k).clamp(-0.4, 0.4);
        // Weight shifting from foot to foot: heavy animals roll, zombies lurch.
        let sway_k = match style {
            Style::Biped => 0.035 + if rig.shamble { 0.09 } else { 0.0 },
            Style::Quadruped => 0.012 + rig.heavy * 0.05,
            Style::Bird => 0.08,
            _ => 0.0,
        };
        let sway = t.sin() * sway_k * amp.min(1.0) + if rig.shamble { (clock * 0.7).sin() * 0.04 * idle } else { 0.0 };
        // Pitch: galloping rocks, flyers nose into their dive, swimmers lift the nose.
        let mut pitch = match style {
            Style::Quadruped => (t + 0.6).sin() * 0.07 * run,
            Style::Flyer => (-st.vy * 0.07).clamp(-0.35, 0.35) + amp.min(1.0) * 0.15,
            _ => 0.0,
        };
        pitch -= swim * 0.22;
        pitch += st.graze * 0.1;
        // A play bow: chest down, rump up.
        if style == Style::Quadruped {
            pitch += st.bow * 0.32;
        }
        // Recoil from a hit, lunge into a bite.
        pitch -= hurt * 0.22;
        if !matches!(style, Style::Biped) {
            pitch += strike * 0.14;
        }
        let c = rig.centre;
        root *= Mat4::from_translation(Vec3::new(0.0, c.y, c.z))
            * Mat4::from_rotation_z(bank + sway)
            * Mat4::from_rotation_x(pitch)
            * Mat4::from_translation(Vec3::new(0.0, -c.y, -c.z));
        if !matches!(style, Style::Biped) && strike > 0.0 {
            root *= Mat4::from_translation(Vec3::new(0.0, 0.0, strike * 1.5));
        }
        if style == Style::Swimmer {
            // Swimming: the body snakes from side to side, faster when in a hurry.
            if mob.body.in_water {
                let w = (clock * (3.0 + amp * 5.0)).sin() * (0.06 + amp.min(1.0) * 0.1);
                root *= Mat4::from_translation(c) * Mat4::from_rotation_y(w) * Mat4::from_translation(-c);
            } else {
                // Stranded: on its side, flopping.
                let flop = (clock * 7.0).sin().abs();
                root *= Mat4::from_translation(Vec3::new(0.0, flop * 2.5, 0.0)) * Mat4::from_translation(c) * Mat4::from_rotation_z(1.35) * Mat4::from_rotation_y((clock * 7.0).cos() * 0.3) * Mat4::from_translation(-c);
            }
        }
        if swell > 0.0 {
            // About to burst: trembling.
            root *= Mat4::from_translation(Vec3::new((clock * 61.0).sin(), 0.0, (clock * 47.0).cos()) * swell * 0.35);
        }
        // Squash and stretch: landing flattens it, rising stretches it (jellies much more).
        let mut s = st.squash * if style == Style::Hopper { 1.0 } else { 0.55 } + hurt * 0.06;
        if style == Style::Hopper {
            s += (-st.vy * 0.03).clamp(-0.25, 0.15) * (!mob.body.on_ground) as u8 as f32;
            s += (clock * 3.1).sin() * 0.035 * idle;
        }
        if airborne && style != Style::Hopper {
            s -= (st.vy * 0.012).clamp(-0.06, 0.06);
        }
        if s.abs() > 1e-3 {
            let side = 1.0 + s * 0.55;
            root *= Mat4::from_scale(Vec3::new(side, 1.0 - s, side));
        }
    }

    // Bipeds bend at the hips: leaning into a run or a swing, twisting with each step.
    let biped = matches!(style, Style::Biped);
    let torso = if biped && alive {
        let lean = run * 0.2 + amp.min(1.0) * 0.04 + strike * 0.22 - hurt * 0.25 + if rig.shamble { 0.08 } else { 0.0 } + swim * 0.3;
        let twist = t.sin() * 0.1 * amp.min(1.2) + if strike > 0.0 { -strike * 0.25 } else { 0.0 };
        let shift = if rig.town && idle > 0.5 { (clock * 0.35).sin() * 0.05 * idle } else { 0.0 };
        let h = rig.hip;
        Mat4::from_translation(h) * Mat4::from_rotation_x(lean) * Mat4::from_rotation_y(twist) * Mat4::from_rotation_z(shift) * Mat4::from_translation(-h)
    } else {
        Mat4::IDENTITY
    };

    // The head.
    let mut look = ((clock * 0.31).sin() * 0.45 + (clock * 0.13).sin() * 0.3) * idle;
    if rig.jerky {
        // Birds look about in little snaps.
        look = (look * 3.0).round() / 3.0;
    }
    let lead = (st.turn * 0.12).clamp(-0.4, 0.4) * amp.min(1.0);
    let mut nod = (t * 2.0).sin() * (0.06 + rig.heavy * 0.04 + run * 0.08) * amp.min(1.0) + breathe * 0.02 * idle;
    let graze = st.graze * awake as u8 as f32;
    nod += graze * (1.25 + (clock * 8.0).sin() * 0.05);
    if rig.jerky && idle > 0.5 {
        // A peck at the ground every few seconds.
        let p = (clock + mob.phase * 2.0) % 4.3;
        if p < 0.35 {
            nod += (p / 0.35 * PI).sin() * 0.9;
        }
    }
    nod -= hurt * 0.35;
    if sleeping {
        nod += 0.3;
    }
    // Company: talkers bob their head as they speak, listeners nod along; friends lower their
    // heads nose to nose; a petted animal pushes its head up into the hand.
    let chat = st.chat * awake as u8 as f32;
    let talk = st.talk * awake as u8 as f32;
    let nuzzle = st.nuzzle * awake as u8 as f32;
    let petted = st.petted * awake as u8 as f32;
    nod += (clock * 7.0).sin() * 0.05 * talk + (clock * 2.1).sin().max(0.0) * 0.12 * (chat - talk).max(0.0);
    nod += nuzzle * (0.42 + (clock * 3.0).sin() * 0.06) - petted * 0.3 + st.bow * 0.25 * (style == Style::Quadruped) as u8 as f32;
    if !biped {
        nod += strike * 0.3;
    }
    let tilt = (clock * 0.23).sin().max(0.0).powi(3) * 0.25 * idle
        + if rig.shamble { 0.18 + (clock * 0.9).sin() * 0.06 } else { 0.0 }
        + (clock * 2.4).sin() * 0.22 * nuzzle
        + (clock * 1.7).sin() * 0.25 * petted
        + (clock * 0.8).sin() * 0.1 * (chat - talk).max(0.0);
    let rattle = if rig.rattle && awake { (clock * 37.0).sin() * 0.015 } else { 0.0 };
    let head_q = Quat::from_rotation_y(st.head_yaw + (look * (1.0 - graze)) + lead + rattle) * Quat::from_rotation_x(st.head_pitch * (1.0 - graze) + nod) * Quat::from_rotation_z(tilt + rattle);
    let head_pivot = rig.head.map(|h| rig.pivots[h]).unwrap_or(Vec3::ZERO);
    // Grazing stretches the neck down and forward (nuzzling a little).
    let neck = Vec3::new(0.0, -3.0, 1.0) * graze + Vec3::new(0.0, -1.0, 1.2) * nuzzle * (!biped) as u8 as f32;
    // Babies have big heads.
    let big = if mob.baby() { 1.0 + 0.4 * (mob.grow / voxel::mob::GROW_TIME).clamp(0.0, 1.0).sqrt() } else { 1.0 };
    let head_local = Mat4::from_translation(head_pivot + neck) * Mat4::from_quat(head_q) * Mat4::from_scale(Vec3::splat(big)) * Mat4::from_translation(-head_pivot);

    let base = model * root;
    for (i, p) in def.parts.iter().enumerate().take(MAX_PARTS) {
        let pivot = rig.pivots[i];
        let role = rig.roles[i];
        // Knee or elbow angle: positive swings the lower half back.
        let mut bend = 0.0f32;
        let rot = match role {
            Role::Fixed | Role::Body | Role::Head | Role::Ear(_) => Quat::IDENTITY,
            Role::Leg { .. } if sleeping => {
                // Front legs tuck forward, hind legs back.
                Quat::from_rotation_x(if p.pivot.2 >= 0 { -FRAC_PI_2 } else { FRAC_PI_2 })
            }
            Role::Leg { pair, front, side } => {
                let k = pair as f32 * PI;
                let swing_at = |t: f32| match style {
                    Style::Quadruped => {
                        let walk = (t + k).sin() * (0.7 - rig.heavy * 0.15);
                        // Gallop: front legs together, hind legs together, a beat apart.
                        let lag = if side > 0.0 { 0.0 } else { 0.45 };
                        let gallop = (t + if front { 0.0 } else { PI } + lag).sin() * 0.75;
                        (walk * (1.0 - run) + gallop * run) * amp.min(1.1)
                    }
                    Style::Bird => (t * 1.2 + k).sin() * 0.9 * amp.min(1.1),
                    _ => (t + k).sin() * (0.7 + run * 0.2) * amp.min(1.1),
                };
                let mut a = swing_at(t);
                // The knee folds while the foot swings forward through the air, and stays a
                // little soft while it carries the weight.
                let forward = ((a - swing_at(t + 0.15)) / 0.15).max(0.0);
                let mut knee = forward * (1.1 + run * 0.4) + 0.05 * amp.min(1.0);
                if swim > 0.0 {
                    // Paddling.
                    let paddle = (clock * 6.0 + k + if front { 0.0 } else { 1.3 }).sin() * 0.6 - 0.2;
                    a = a * (1.0 - swim) + paddle * swim;
                    knee = knee * (1.0 - swim) + (0.5 + (clock * 6.0 + k + 1.2).sin() * 0.35) * swim;
                }
                if airborne {
                    let leap = if style == Style::Quadruped {
                        if front { -0.55 } else { 0.5 }
                    } else if style == Style::Bird {
                        0.6
                    } else if st.air < 0.5 {
                        if pair == 0 { -0.45 } else { 0.2 }
                    } else {
                        (clock * 10.0 + k).sin() * 0.5
                    };
                    let w = smooth((st.air - 0.15) / 0.15);
                    a = a * (1.0 - w) + leap * w;
                    let tuck = if style == Style::Quadruped { 0.85 } else if st.air < 0.5 { if pair == 0 { 1.0 } else { 0.35 } } else { 0.45 + (clock * 10.0 + k).cos() * 0.3 };
                    knee = knee * (1.0 - w) + tuck * w;
                }
                // Hind legs of four-legged animals bend the other way (the hock).
                bend = if style == Style::Quadruped && !front { -knee * 0.8 } else { knee };
                if !alive {
                    bend = 0.25 * (mob.dead_age / DEATH_TIME).clamp(0.0, 1.0);
                    // Gone limp, splayed a little.
                    let d = (mob.dead_age / DEATH_TIME).clamp(0.0, 1.0);
                    return_leg(d, side)
                } else {
                    Quat::from_rotation_x(a)
                }
            }
            Role::Arm(k) => {
                let side = if k == 0 { 1.0 } else { -1.0 };
                let swing = (t + k as f32 * PI + PI).sin() * 0.7 * (1.0 + run * 0.45) * amp.min(1.3);
                let mut q = Quat::from_rotation_x(swing + breathe * 0.04 * idle * side) * Quat::from_rotation_z(side * (0.04 + breathe * 0.03) * idle);
                // Elbows: soft at rest, bending as the arm swings forward, pumping when running.
                let mut elbow = -(0.12 + (-swing).max(0.0) * 0.5 + run * 0.75);
                if (rig.town || chat > 0.0) && idle > 0.6 && greet <= 0.0 && (k == 0 || talk > 0.0) {
                    // Talking with the hands now and then (both hands when it's its turn in a
                    // chat; still while listening).
                    let c = (clock + mob.phase * 4.0) % 9.0;
                    let own = if chat > 0.0 { 0.0 } else { smooth(c / 0.4).min(smooth((2.6 - c) / 0.4)).max(0.0) };
                    let g = own.max(talk * if k == 0 { 1.0 } else { 0.6 });
                    let clock = clock + k as f32 * 1.3;
                    if g > 0.0 {
                        let talk = Quat::from_rotation_x(-0.85 + (clock * 3.5).sin() * 0.18) * Quat::from_rotation_y(0.25 + (clock * 2.3).sin() * 0.15);
                        q = q.slerp(talk, g);
                        elbow = elbow * (1.0 - g) - (1.0 + (clock * 3.1).sin() * 0.2) * g;
                    }
                }
                if greet > 0.0 && k == 0 {
                    let wave = Quat::from_rotation_x(-PI + 0.35) * Quat::from_rotation_z(-side * (0.45 + (clock * 11.0).sin() * 0.3));
                    q = q.slerp(wave, greet);
                    elbow = elbow * (1.0 - greet) - (0.55 + (clock * 11.0).cos() * 0.25) * greet;
                }
                if aiming {
                    // Bow up, both arms reaching toward the target.
                    let aim = Quat::from_rotation_x(-FRAC_PI_2 + st.head_pitch) * Quat::from_rotation_y(-side * 0.12);
                    q = q.slerp(aim, 0.9);
                    // The bow arm straight, the string hand drawn back to the cheek.
                    elbow = if k == 0 { -0.05 } else { -0.9 };
                } else if strike > 0.0 {
                    let hit = Quat::from_rotation_x(-0.4 - 1.6 * strike) * Quat::from_rotation_z(side * 0.15 * strike);
                    q = q.slerp(hit, if k == 0 { 1.0 } else { 0.4 });
                    // Cocked on the way up, snapping straight into the blow.
                    let ph = (st.attack / 0.5).clamp(0.0, 1.0);
                    elbow = elbow * (1.0 - strike) - if ph < 0.45 { 1.1 * strike } else { 0.1 * strike };
                }
                if airborne {
                    let w = smooth((st.air - 0.15) / 0.15);
                    let up = if st.air < 0.5 { Quat::from_rotation_x(-0.5) * Quat::from_rotation_z(-side * 0.4) } else { Quat::from_rotation_z(-side * (1.1 + (clock * 14.0 + k as f32).sin() * 0.35)) };
                    q = q.slerp(up, w);
                    elbow = elbow * (1.0 - w) - 0.35 * w;
                }
                if swim > 0.0 {
                    let s = (clock * 3.2).sin() * 0.5 + 0.5;
                    let stroke = Quat::from_rotation_x(-PI + 0.4 + s * 0.6) * Quat::from_rotation_z(-side * s * 1.0);
                    q = q.slerp(stroke, swim * 0.85);
                    elbow = elbow * (1.0 - swim) - (0.2 + (1.0 - s) * 0.9) * swim;
                }
                if hurt > 0.0 {
                    q = Quat::from_rotation_z(-side * 0.45 * hurt) * q;
                }
                if !alive {
                    q = Quat::from_rotation_z(-side * 0.3);
                    elbow = -0.2;
                }
                bend = elbow;
                q
            }
            Role::ArmForward(k) => {
                let side = if k == 0 { 1.0 } else { -1.0 };
                let reach = -FRAC_PI_2 + (clock * 2.0 + k as f32).sin() * 0.08 + (t + k as f32 * PI).sin() * 0.12 * amp.min(1.0);
                // Attack: arms rise, then slam down.
                let slam = if strike > 0.0 {
                    let ph = (st.attack / 0.5).clamp(0.0, 1.0);
                    if ph < 0.4 { -0.7 * smooth(ph / 0.4) } else { -0.7 + 1.1 * smooth((ph - 0.4) / 0.6) }
                } else {
                    0.0
                };
                let q = Quat::from_rotation_x(reach + slam - hurt * 0.4) * Quat::from_rotation_z(side * (0.06 + (clock * 1.3 + k as f32).sin() * 0.04));
                // Forearms hang a little from the elbows, bobbing; raised high for the slam.
                bend = if alive { 0.3 + (clock * 2.0 + k as f32).sin() * 0.06 - slam.min(0.0) * 0.6 } else { 0.0 };
                if alive { q } else { Quat::from_rotation_z(-side * 0.3) }
            }
            Role::Wing(k) => {
                let side = if k == 0 { 1.0 } else { -1.0 };
                let flap = match style {
                    Style::Flyer => {
                        let rate = 26.0 - rig.heavy * 18.0;
                        (clock * rate + k as f32 * 0.2).sin() * 0.55 + 0.35
                    }
                    _ if airborne || (!mob.body.on_ground && !mob.body.in_water && alive) => (clock * 25.0).sin().abs() * 0.95,
                    _ if awake => {
                        // Running flaps for balance; now and then a quick ruffle.
                        let r = (clock + mob.phase * 3.0) % 7.0;
                        let ruffle = if r < 0.45 { (r * 40.0).sin().abs() * 0.5 } else { 0.0 };
                        run * 0.5 + amp.min(1.0) * 0.12 + ruffle + hurt * 0.8
                    }
                    _ => 0.0,
                };
                Quat::from_rotation_z(flap * side)
            }
            Role::Spider(k) => {
                let side = if k < 4 { 1.0 } else { -1.0 };
                let kk = (k % 4) as f32;
                let fan = (kk - 1.5) * 0.4 * side;
                let swing = (t * 2.0 + kk * 1.3 + if k < 4 { 0.0 } else { PI }).sin() * 0.35 * amp;
                let mut lift = ((t * 2.0 + kk * 1.3).cos().max(0.0)) * 0.3 * amp;
                // Idle: a leg twitches now and then.
                let tw = (clock * 0.8 + k as f32 * 1.7).sin();
                if tw > 0.93 {
                    lift += (tw - 0.93) * 4.0 * idle;
                }
                if !alive {
                    // Curled up.
                    lift = 0.9 * (mob.dead_age / DEATH_TIME).clamp(0.0, 1.0);
                }
                Quat::from_rotation_y(fan + swing) * Quat::from_rotation_z((0.5 - lift) * side)
            }
            Role::Tail => {
                let mood = if mob.aggro { 0.15 } else { 0.35 + st.near * 0.65 + petted * 0.5 + st.bow * 0.5 };
                let rate = rig.wag_rate * (1.0 + petted * 0.6 + st.bow * 0.6);
                let wag = if awake { (clock * rate).sin() * 0.5 * rig.wag * mood * (1.0 - run * 0.7) } else { 0.0 };
                let sway = (clock * 1.1).sin() * 0.12 * idle;
                let swish = if style == Style::Swimmer { (clock * (3.0 + amp * 5.0) - 0.8).sin() * (0.25 + amp.min(1.0) * 0.3) } else { 0.0 };
                let mut lift = (t * 2.0).cos() * 0.08 * amp.min(1.0) + run * 0.18 - if sleeping || !alive { 0.35 } else { 0.0 };
                if mob.aggro && def.kind != Kind::Passive {
                    lift += 0.3;
                }
                Quat::from_rotation_y(st.tail + wag + sway + swish) * Quat::from_rotation_x(lift)
            }
        };
        let own = Mat4::from_translation(pivot) * Mat4::from_quat(rot) * Mat4::from_translation(-pivot);
        let above_hip = biped && match role {
            Role::Body | Role::Head | Role::Ear(_) | Role::Arm(_) | Role::ArmForward(_) | Role::Wing(_) => true,
            Role::Fixed | Role::Tail => pivot.y.max(p.origin.1 as f32) >= rig.hip.y - 0.5,
            _ => false,
        };
        let chain = if above_hip { torso } else { Mat4::IDENTITY };
        let m = match role {
            Role::Head => chain * head_local,
            Role::Ear(side) => {
                // Ears flick now and then, lie back when running, swing with the spring.
                let f = (clock * 0.9 + pivot.x * 0.7).sin().max(0.0).powi(30) * awake as u8 as f32;
                let q = Quat::from_rotation_x(st.ear * awake as u8 as f32 - if sleeping { 0.4 } else { 0.0 }) * Quat::from_rotation_z(side * (f * 0.5 + st.tail.abs() * 0.1));
                chain * head_local * Mat4::from_translation(pivot) * Mat4::from_quat(q) * Mat4::from_translation(-pivot)
            }
            Role::Body => {
                // Breathing: the chest swells a touch.
                let br = breathe * if sleeping { 0.035 } else { 0.015 * (0.3 + idle * 0.7) } * alive as u8 as f32;
                let (lo, hi) = bounds(p);
                let c = (lo + hi) * 0.5;
                chain * Mat4::from_translation(c) * Mat4::from_scale(Vec3::new(1.0 + br, 1.0 + br * 1.2, 1.0 + br * 0.6)) * Mat4::from_translation(-c)
            }
            _ => chain * own,
        };
        b.m[i] = (base * m).to_cols_array_2d();
        if bend != 0.0 {
            b.set_bend(i, p, bend);
        }
        // Blinking: every few seconds (each mob on its own beat) for a tenth of a second, and
        // always while asleep or dead.
        if Some(i) == rig.head && def.parts.len() <= EYELID_PART {
            let period = 3.2 + mob.phase.fract() * 3.0;
            let beat = (st.t + mob.age + mob.phase * 7.0) % period;
            let double = mob.phase.fract() > 0.7 && (0.25..0.37).contains(&beat);
            let content = petted > 0.5 || (nuzzle > 0.6 && (clock * 0.7).sin() > 0.2);
            if beat < 0.12 || double || sleeping || !alive || eyes_shut_test() || content {
                b.m[EYELID_PART] = b.m[i];
            }
        }
        // Speaking: the mouth (town folk have one, see entity::add_mouth) opens and shuts with
        // the syllables.
        if i == 0 && talk > 0.03 && def.parts.len() <= MOUTH_PART && crate::entity::has_mouth(def) {
            let syl = ((clock * 9.0).sin() * 0.5 + 0.5) * (0.6 + 0.4 * (clock * 3.3).sin().abs());
            let top = p.origin.1 as f32 + 2.0;
            let open = Mat4::from_translation(Vec3::Y * top) * Mat4::from_scale(Vec3::new(1.0, 0.2 + talk * syl * 1.1, 1.0)) * Mat4::from_translation(-Vec3::Y * top);
            b.m[MOUTH_PART] = (Mat4::from_cols_array_2d(&b.m[0]) * open).to_cols_array_2d();
        }
    }
    b
}

fn return_leg(d: f32, side: f32) -> Quat {
    Quat::from_rotation_z(side * 0.35 * d) * Quat::from_rotation_x(0.2 * d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defs() -> Vec<MobDef> {
        let texts: Vec<String> = ["mobs.ron", "mods/mobs.ron", "etapa5/mobs.ron", "fase6/mobs.ron", "tr/mobs.ron"]
            .iter()
            .filter_map(|f| std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/vanilla").join(f)).ok())
            .collect();
        let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        voxel::mob::MobRegistry::from_ron_many(&refs).expect("mob files").defs
    }

    #[test]
    fn rigs_read_the_models() {
        let defs = defs();
        assert!(defs.len() > 100, "mob files not found");
        let find = |n: &str| defs.iter().find(|d| d.name == n).map(Rig::new).unwrap();
        let wolf = find("wolf");
        assert_eq!(wolf.style, Style::Quadruped);
        assert!(wolf.roles.contains(&Role::Tail), "wolf tail: {:?}", wolf.roles);
        assert!(wolf.roles.iter().any(|r| matches!(r, Role::Ear(_))), "wolf ears: {:?}", wolf.roles);
        assert!(wolf.wag > 0.9);
        assert_eq!(find("zombie").style, Style::Biped);
        assert!(find("zombie").shamble);
        assert_eq!(find("chicken").style, Style::Bird);
        assert_eq!(find("jelly_green").style, Style::Hopper);
        assert_eq!(find("great_eye").style, Style::Flyer);
        assert!(find("cow").graze);
        assert!(find("npc_dryad").town);
    }

    #[test]
    fn every_mob_animates_without_nan() {
        let defs = defs();
        let pose = |m: &Mob, d: &MobDef, rig: &Rig, dt: f32, st: &mut MobAnim| {
            st.update(m, d, rig, Vec3::new(2.0, 0.0, 2.0), dt);
            bones_for(m, d, rig, st, IVec3::ZERO, 1.0, 0.0)
        };
        for (i, d) in defs.iter().enumerate() {
            let rig = Rig::new(d);
            let mut m = Mob::new(i, d, Vec3::new(0.0, 0.0, 0.0), i as u64);
            m.walk_speed = d.speed * 1.3;
            m.walk = 1.3;
            m.attack_age = 0.2;
            m.hurt_age = 0.1;
            m.body.on_ground = false;
            m.body.vel = Vec3::new(1.0, -6.0, 0.0);
            let mut st = MobAnim::default();
            for f in 0..30 {
                m.yaw += 0.05;
                m.body.on_ground = f > 10;
                let b = pose(&m, d, &rig, 1.0 / 60.0, &mut st);
                for mm in b.m.iter() {
                    assert!(mm.iter().flatten().all(|v| v.is_finite()), "{} gave a bad matrix", d.name);
                }
            }
            m.health = 0.0;
            m.dead_age = 0.5;
            let b = pose(&m, d, &rig, 1.0 / 60.0, &mut st);
            assert!(b.m[0].iter().flatten().all(|v| v.is_finite()));
        }
    }
}
#[cfg(test)]
mod graze_test {
    use super::*;
    #[test]
    fn graze_lowers_head() {
        let texts = [std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../packs/vanilla/mobs.ron")).unwrap()];
        let reg = voxel::mob::MobRegistry::from_ron_many(&[texts[0].as_str()]).unwrap();
        let i = reg.id("cow").unwrap();
        let d = &reg.defs[i];
        let rig = Rig::new(d);
        let m = Mob::new(i, d, Vec3::ZERO, 1);
        let mut st = MobAnim::default();
        st.update(&m, d, &rig, Vec3::splat(100.0), 0.0);
        let h = rig.head.unwrap();
        let a = Mat4::from_cols_array_2d(&bones_for(&m, d, &rig, &st, IVec3::ZERO, 1.0, 0.0).m[h]).transform_point3(Vec3::new(0.0, 18.0, 14.0));
        st.graze = 1.0;
        let b = Mat4::from_cols_array_2d(&bones_for(&m, d, &rig, &st, IVec3::ZERO, 1.0, 0.0).m[h]).transform_point3(Vec3::new(0.0, 18.0, 14.0));
        println!("{:?} {:?} roles {:?}", a, b, rig.roles);
        assert!(b.y < a.y - 0.2);
    }
}
