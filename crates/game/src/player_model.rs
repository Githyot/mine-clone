//! The player's own body: seen in third person (F5) and as an arm in first person.
//! Animations follow the ideas of detailed-animation packs: the body turns lazily after the
//! head, arms and legs swing with speed, the player leans when sneaking, swims face down with
//! strokes, flails while falling, squashes on landing, breathes when idle and swings to hit.

use std::f32::consts::{FRAC_PI_2, PI};

use glam::{IVec3, Mat4, Quat, Vec3};
use voxel::mob::MobDef;

use crate::entity::{Bones, MOUTH_PART};

/// Body part indexes in the "player" model of mobs.ron.
const HEAD: usize = 0;
const BODY: usize = 1;
const R_ARM: usize = 2;
const L_ARM: usize = 3;
const R_LEG: usize = 4;
const L_LEG: usize = 5;

/// What the body is doing this frame (filled in by the game).
#[derive(Default, Clone, Copy)]
pub struct Pose {
    pub pos: Vec3,
    /// Horizontal velocity.
    pub vel: Vec3,
    pub look_yaw: f32,
    pub look_pitch: f32,
    pub on_ground: bool,
    pub in_water: bool,
    pub head_in_water: bool,
    pub flying: bool,
    pub sneaking: f32,
    /// 0..1 lying down.
    pub crawling: f32,
    pub sprinting: bool,
    pub burning: bool,
    pub hurt: bool,
    /// In a cart or on a chair.
    pub sitting: bool,
    /// Something in the right hand: the arm is held a little forward, as in the classic game.
    pub holding: bool,
    /// A torch in the right hand: held up high ahead, lighting the way (torch.rs).
    pub torch: bool,
    /// A shield coming up to block: 0..1, and in which hand (shield.rs).
    pub shield_up: f32,
    pub shield_left: bool,
    /// Pulling up onto a ledge: 0 = not climbing, else 0..1 through the climb, and the
    /// ledge's top above the feet.
    pub mantle: f32,
    pub ledge: f32,
}

/// Smoothed animation state that lives between frames.
#[derive(Default)]
pub struct Animator {
    pub body_yaw: f32,
    walk: f32,
    amount: f32,
    time: f32,
    air_time: f32,
    land: f32,
    swim: f32,
    lean: f32,
    /// 0..1 how far into a sprint: the body leans forward into the run.
    sprint: f32,
    /// Attack swing 0..1 (0 = not swinging).
    pub swing: f32,
    /// How the weapon moves (Better Combat idea, see `mods/combat.rs`): 0 chop down from
    /// above, 1 slash across, 2 thrust ahead, 3 uppercut from below.
    pub swing_style: u8,
    was_ground: bool,
    fall_speed: f32,
    /// 0..1 head under water, and the breaststroke phase for the first-person arms.
    pub underwater: f32,
    stroke: f32,
    /// 0..1 how open the mouth is (voice chat, voice.rs).
    pub talk: f32,
    /// Climbing a ledge (copied from the pose for the first-person arms).
    mantle: f32,
    ledge: f32,
}

fn wrap(a: f32) -> f32 {
    (a + PI).rem_euclid(2.0 * PI) - PI
}

impl Animator {
    /// A body lying still (a corpse), facing `yaw`.
    pub fn lying(yaw: f32) -> Animator {
        Animator { body_yaw: yaw, ..Default::default() }
    }

    pub fn start_swing(&mut self) {
        if self.swing <= 0.0 || self.swing > 0.5 {
            self.swing = 0.001;
            self.swing_style = 0;
        }
    }

    /// Start a weapon swing of the given style right away (combo attacks).
    pub fn start_styled_swing(&mut self, style: u8) {
        self.swing = 0.001;
        self.swing_style = style;
    }

    pub fn update(&mut self, p: &Pose, dt: f32) {
        if self.time == 0.0 {
            // The first frame: the body starts out facing where the player looks.
            self.body_yaw = p.look_yaw + PI;
        }
        self.time += dt;
        if self.swing > 0.0 {
            self.swing += dt * 3.6;
            if self.swing >= 1.0 {
                self.swing = 0.0;
            }
        }
        let speed = Vec3::new(p.vel.x, 0.0, p.vel.z).length();
        let target = (speed / 4.3).min(1.6);
        self.amount += (target - self.amount) * (dt * 8.0).min(1.0);
        self.walk += speed * dt * if p.crawling > 0.5 { 4.0 } else if p.sprinting { 1.25 } else { 1.6 };
        // The facing direction in model space (+Z forward) for the camera yaw.
        let face = p.look_yaw + PI;
        // Moving: the body turns toward the movement; standing: it follows once the head
        // has turned more than ~50 degrees.
        let mut want = self.body_yaw;
        if speed > 0.5 {
            let move_yaw = p.vel.x.atan2(p.vel.z);
            let rel = wrap(move_yaw - face);
            // Walking backwards keeps facing forward.
            want = if rel.abs() > FRAC_PI_2 { face + wrap(rel + PI) * 0.5 } else { face + rel * 0.5 };
        } else {
            // Lying down the neck can't twist far, so the body follows sooner.
            let slack = 0.9 - 0.6 * p.crawling;
            let d = wrap(face - self.body_yaw);
            if d.abs() > slack {
                want = face - d.signum() * slack;
            }
        }
        self.body_yaw += wrap(want - self.body_yaw) * (dt * 7.0).min(1.0);

        if p.on_ground || p.in_water || p.flying {
            if !self.was_ground && self.air_time > 0.25 {
                self.land = (self.fall_speed / 14.0).clamp(0.2, 1.0);
            }
            self.air_time = 0.0;
            self.fall_speed = 0.0;
        } else {
            self.air_time += dt;
            self.fall_speed = self.fall_speed.max(-p.vel.y);
        }
        self.was_ground = p.on_ground || p.in_water || p.flying;
        self.land = (self.land - dt * 4.0).max(0.0);
        let running = p.sprinting && speed > 3.0 && !p.in_water && !p.flying && p.crawling < 0.5 && !p.sitting;
        self.sprint += ((running as u8 as f32) - self.sprint) * (dt * if running { 5.0 } else { 7.0 }).min(1.0);
        let under = p.head_in_water && !p.flying;
        self.underwater += ((under as u8 as f32) - self.underwater) * (dt * 6.0).min(1.0);
        // Slow treading strokes when still, quicker ones when swimming along.
        self.stroke += dt * (0.55 + (speed / 3.0).min(1.0) * 0.6);
        let swimming = p.head_in_water && speed > 1.0;
        self.swim += ((swimming as u8 as f32) - self.swim) * (dt * 5.0).min(1.0);
        self.mantle = p.mantle;
        self.ledge = p.ledge;
        if p.mantle > 0.0 {
            // Hanging on a ledge is neither a jump nor a fall.
            self.air_time = 0.0;
            self.fall_speed = 0.0;
        }
        let lean_target = if p.flying { (speed / 11.0).min(1.0) } else { 0.0 };
        self.lean += (lean_target - self.lean) * (dt * 4.0).min(1.0);
    }

    /// Third person: the whole body at `p.pos`.
    pub fn bones(&self, def: &MobDef, p: &Pose, cam_block: IVec3, sky: f32, blk: f32) -> Bones {
        let mut b = Bones { light: [sky, blk, if p.hurt { 1.0 } else { 0.0 }, 0.0], ..Default::default() };
        let rel = p.pos - cam_block.as_vec3();
        let t = self.walk;
        let a = self.amount;
        let idle = (1.0 - a).max(0.0);
        let breathe = (self.time * 1.6).sin();
        let airborne = self.air_time > 0.12 && !p.flying;
        let crouch = p.sneaking;

        // Whole-body transform: squash on landing, crouch, swim and fly tilt.
        let mut root = Mat4::from_translation(rel) * Mat4::from_rotation_y(self.body_yaw);
        // A spring in the step: the body rises as each leg passes under it and the weight
        // shifts from foot to foot.
        let step = if p.on_ground && !p.sitting && p.crawling < 0.5 && !p.in_water { a.min(1.2) } else { 0.0 };
        root = root * Mat4::from_translation(Vec3::new(0.0, t.cos().abs() * (0.04 + p.sprinting as u8 as f32 * 0.03) * step, 0.0)) * Mat4::from_rotation_z(t.sin() * 0.02 * step);
        // Sprinting: the whole body leans forward from the feet into the run, sinking a touch.
        let run = self.sprint;
        if run > 0.001 {
            root = root * Mat4::from_rotation_x(run * 0.26) * Mat4::from_translation(Vec3::new(0.0, -run * 0.05, 0.0));
        }
        let squash = self.land * 0.12;
        root = root * Mat4::from_translation(Vec3::new(0.0, -crouch * 0.18 - squash * 0.5, 0.0));
        // Lying down: the body tips flat and sinks to the ground.
        let prone = p.crawling * (1.0 - self.swim);
        root = root * Mat4::from_translation(Vec3::new(0.0, -prone * 0.6, 0.0));
        // Swimming, flying and crawling pitch the body around its middle.
        let tilt = (self.swim * 1.35 + self.lean * 0.9 + prone * FRAC_PI_2).min(FRAC_PI_2);
        if tilt > 0.0 {
            root = root * Mat4::from_translation(Vec3::Y * 0.75) * Mat4::from_rotation_x(tilt) * Mat4::from_translation(-Vec3::Y * 0.75);
        }
        root = root * Mat4::from_scale(Vec3::new(1.0 / 16.0, (1.0 - squash) / 16.0, 1.0 / 16.0));

        let mut rot = [Quat::IDENTITY; 6];
        let mut shift = [Vec3::ZERO; 6];
        // Legs: stride, longer and faster when running.
        let stride = if p.sprinting { 1.05 } else { 0.8 };
        let leg = t.sin() * stride * a.min(1.2);
        rot[R_LEG] = Quat::from_rotation_x(leg);
        rot[L_LEG] = Quat::from_rotation_x(-leg);
        // Arms swing against the legs, a little outward; idle breathing sways them.
        let arm = -leg * (0.85 + run * 0.45);
        let sway = 0.04 + breathe * 0.03 * idle;
        // Holding something, the right arm swings half as far and stays raised 18° forward.
        let r_arm = if p.torch {
            arm * 0.25 - PI / 4.0
        } else if p.holding {
            arm * 0.5 - PI / 10.0
        } else {
            arm
        };
        rot[R_ARM] = Quat::from_rotation_x(r_arm + breathe * 0.03 * idle) * Quat::from_rotation_z(-sway);
        rot[L_ARM] = Quat::from_rotation_x(-arm - breathe * 0.03 * idle) * Quat::from_rotation_z(sway);
        if run > 0.001 {
            // Running: the chest drives forward over the hips, the head stays up looking ahead,
            // arms pump hard and the knees come up high.
            rot[BODY] = Quat::from_rotation_x(0.16 * run);
            rot[HEAD] = Quat::from_rotation_x(-0.32 * run);
        }
        if airborne && self.swim < 0.5 {
            if self.air_time < 0.5 && !p.in_water {
                // Jump: arms up and out, one knee forward.
                rot[R_ARM] = Quat::from_rotation_x(-0.5) * Quat::from_rotation_z(-0.35);
                rot[L_ARM] = Quat::from_rotation_x(-0.5) * Quat::from_rotation_z(0.35);
                rot[R_LEG] = Quat::from_rotation_x(-0.45);
                rot[L_LEG] = Quat::from_rotation_x(0.2);
            } else {
                // Long fall: arms flail.
                let f = self.time * 14.0;
                rot[R_ARM] = Quat::from_rotation_z(-1.2 - f.sin() * 0.4) * Quat::from_rotation_x(f.cos() * 0.4);
                rot[L_ARM] = Quat::from_rotation_z(1.2 + f.cos() * 0.4) * Quat::from_rotation_x(f.sin() * 0.4);
                rot[R_LEG] = Quat::from_rotation_x((f * 0.7).sin() * 0.5);
                rot[L_LEG] = Quat::from_rotation_x(-(f * 0.7).sin() * 0.5);
            }
        }
        if p.in_water && !p.head_in_water && a < 0.4 {
            // Wading: arms float out and paddle slowly.
            let w = self.time * 2.0;
            rot[R_ARM] = Quat::from_rotation_z(-0.6 - w.sin() * 0.15);
            rot[L_ARM] = Quat::from_rotation_z(0.6 + w.sin() * 0.15);
        }
        if self.swim > 0.05 {
            // Breaststroke and flutter kick.
            let s = self.time * 3.2;
            let k = self.swim;
            let stroke = (s.sin() * 0.5 + 0.5) * 2.4;
            rot[R_ARM] = rot[R_ARM].slerp(Quat::from_rotation_x(-PI + 0.3) * Quat::from_rotation_z(-stroke * 0.6), k);
            rot[L_ARM] = rot[L_ARM].slerp(Quat::from_rotation_x(-PI + 0.3) * Quat::from_rotation_z(stroke * 0.6), k);
            let kick = (self.time * 9.0).sin() * 0.35;
            rot[R_LEG] = rot[R_LEG].slerp(Quat::from_rotation_x(kick), k);
            rot[L_LEG] = rot[L_LEG].slerp(Quat::from_rotation_x(-kick), k);
        }
        if p.flying && self.lean > 0.1 {
            rot[R_LEG] = Quat::from_rotation_x(0.1);
            rot[L_LEG] = Quat::from_rotation_x(0.1);
            rot[R_ARM] = Quat::from_rotation_x(0.35) * Quat::from_rotation_z(-0.1);
            rot[L_ARM] = Quat::from_rotation_x(0.35) * Quat::from_rotation_z(0.1);
        }
        if crouch > 0.01 {
            // Sneak: lean forward, arms a bit forward, legs stay under the hips.
            rot[BODY] = Quat::from_rotation_x(0.5 * crouch);
            shift[HEAD] = Vec3::new(0.0, -1.0, 4.0) * crouch;
            shift[R_ARM] = Vec3::new(0.0, -1.0, 3.5) * crouch;
            shift[L_ARM] = Vec3::new(0.0, -1.0, 3.5) * crouch;
            rot[R_ARM] = rot[R_ARM] * Quat::from_rotation_x(-0.4 * crouch);
            rot[L_ARM] = rot[L_ARM] * Quat::from_rotation_x(-0.4 * crouch);
            shift[R_LEG] = Vec3::new(0.0, 0.0, -2.0) * crouch;
            shift[L_LEG] = Vec3::new(0.0, 0.0, -2.0) * crouch;
        }
        if prone > 0.05 {
            // Army crawl: the arms take turns reaching ahead and pulling, the legs push.
            let c = t;
            let move_k = a.min(1.0);
            let reach = |ph: f32| -PI + 0.5 + (ph.sin() * 0.55) * move_k;
            rot[R_ARM] = rot[R_ARM].slerp(Quat::from_rotation_x(reach(c)) * Quat::from_rotation_z(-0.25 - c.cos().max(0.0) * 0.3 * move_k), prone);
            rot[L_ARM] = rot[L_ARM].slerp(Quat::from_rotation_x(reach(c + PI)) * Quat::from_rotation_z(0.25 + (-c.cos()).max(0.0) * 0.3 * move_k), prone);
            let push = c.sin() * 0.35 * move_k;
            rot[R_LEG] = rot[R_LEG].slerp(Quat::from_rotation_x(-push) * Quat::from_rotation_z(0.08 + push.max(0.0) * 0.5), prone);
            rot[L_LEG] = rot[L_LEG].slerp(Quat::from_rotation_x(push) * Quat::from_rotation_z(-0.08 - (-push).max(0.0) * 0.5), prone);
            // The body rolls a little with each pull.
            rot[BODY] = rot[BODY] * Quat::from_rotation_y(c.sin() * 0.08 * move_k * prone);
            if a < 0.05 {
                // Still: chin on the arms, slow breathing.
                rot[R_ARM] = rot[R_ARM].slerp(Quat::from_rotation_x(-PI + 0.9) * Quat::from_rotation_y(0.6), prone);
                rot[L_ARM] = rot[L_ARM].slerp(Quat::from_rotation_x(-PI + 0.9) * Quat::from_rotation_y(-0.6), prone);
            }
        }
        if p.mantle > 0.0 {
            // Climbing a ledge: both hands on the edge, the body hauls itself up, then leans
            // over the top while one knee comes up onto it and the arms push down behind.
            let (f, over) = mantle_reach(p.mantle);
            let grab = (p.mantle / voxel::physics::MANTLE_GRAB).min(1.0);
            let angle = 0.3 * (1.0 - grab) + mantle_arm(f, p.ledge - 1.4) * grab;
            let kick = (self.time * 11.0).sin() * 0.25 * (1.0 - over);
            rot[R_ARM] = Quat::from_rotation_x(-angle) * Quat::from_rotation_z(-0.12);
            rot[L_ARM] = Quat::from_rotation_x(-angle) * Quat::from_rotation_z(0.12);
            rot[BODY] = Quat::from_rotation_x(0.12 + 0.35 * over);
            // Feet scrabble against the wall, then the right knee steps up.
            rot[R_LEG] = Quat::from_rotation_x(-0.25 + kick - 1.2 * (over * PI).sin());
            rot[L_LEG] = Quat::from_rotation_x(0.1 - kick + 0.35 * over);
        }
        if p.sitting {
            // Seated: thighs forward, a little apart; hands resting on the knees.
            rot[R_LEG] = Quat::from_rotation_x(-1.45) * Quat::from_rotation_z(0.1);
            rot[L_LEG] = Quat::from_rotation_x(-1.45) * Quat::from_rotation_z(-0.1);
            rot[R_ARM] = Quat::from_rotation_x(-0.55 + breathe * 0.02) * Quat::from_rotation_z(-0.05);
            rot[L_ARM] = Quat::from_rotation_x(-0.55 - breathe * 0.02) * Quat::from_rotation_z(0.05);
        }
        if p.burning {
            let f = self.time * 18.0;
            rot[R_ARM] = Quat::from_rotation_x(-2.4 + f.sin() * 0.6);
            rot[L_ARM] = Quat::from_rotation_x(-2.4 + f.cos() * 0.6);
        }
        if p.shield_up > 0.0 {
            // Blocking: the shield arm comes up in front, turned in across the chest.
            let (i, side) = if p.shield_left { (L_ARM, 1.0) } else { (R_ARM, -1.0) };
            let up = Quat::from_rotation_y(-side * 0.45) * Quat::from_rotation_x(-0.95 + arm * 0.15);
            rot[i] = rot[i].slerp(up, p.shield_up.clamp(0.0, 1.0));
        }
        if self.swing > 0.0 {
            // Hit: the right arm comes down from above, the body twists into it. Combo swings
            // cut across, thrust ahead or come up from below instead.
            let s = (self.swing * PI).sin();
            let ph = self.swing;
            match self.swing_style {
                1 => {
                    rot[R_ARM] = Quat::from_rotation_y(0.9 - 1.8 * ph) * Quat::from_rotation_x(-1.45 * s - 0.15) * Quat::from_rotation_z(0.3 * s);
                    rot[BODY] = rot[BODY] * Quat::from_rotation_y(0.35 - 0.7 * ph);
                }
                2 => {
                    rot[R_ARM] = Quat::from_rotation_x(-1.55 * s.sqrt() - 0.05) * Quat::from_rotation_z(0.05 * s);
                    rot[BODY] = rot[BODY] * Quat::from_rotation_y(-s * 0.4) * Quat::from_rotation_x(s * 0.12);
                }
                3 => {
                    rot[R_ARM] = Quat::from_rotation_y(-s * 0.2) * Quat::from_rotation_x(0.5 - 3.0 * ph) * Quat::from_rotation_z(0.15 * s);
                    rot[BODY] = rot[BODY] * Quat::from_rotation_y(-s * 0.2);
                }
                _ => {
                    rot[R_ARM] = Quat::from_rotation_y(-s * 0.3) * Quat::from_rotation_x(-1.9 * s - 0.2) * Quat::from_rotation_z(0.2 * s);
                    rot[BODY] = rot[BODY] * Quat::from_rotation_y(-s * 0.25);
                }
            }
        }
        // Knees and elbows (the limb meshes are split at the joint, see entity::joint).
        // Positive swings the lower half back: knees bend that way, elbows the other.
        let mut bend = [0.0f32; 6];
        let knee_k = if p.sprinting { 1.3 } else { 0.95 } * a.min(1.2);
        // A knee folds while its foot swings forward.
        bend[R_LEG] = (-t.cos()).max(0.0) * knee_k + 0.04 * a.min(1.0);
        bend[L_LEG] = t.cos().max(0.0) * knee_k + 0.04 * a.min(1.0);
        let pump = 1.0 * run;
        bend[R_ARM] = -(0.1 + pump + (-arm).max(0.0) * 0.45 + if p.holding { 0.3 } else { 0.0 });
        bend[L_ARM] = -(0.1 + pump + arm.max(0.0) * 0.45);
        if airborne && self.swim < 0.5 {
            if self.air_time < 0.5 && !p.in_water {
                bend[R_LEG] = 1.0;
                bend[L_LEG] = 0.35;
                bend[R_ARM] = -0.3;
                bend[L_ARM] = -0.3;
            } else {
                let f = self.time * 14.0;
                bend[R_LEG] = 0.45 + (f * 0.7).sin() * 0.3;
                bend[L_LEG] = 0.45 - (f * 0.7).sin() * 0.3;
                bend[R_ARM] = -0.25 - f.cos().max(0.0) * 0.4;
                bend[L_ARM] = -0.25 - f.sin().max(0.0) * 0.4;
            }
        }
        if self.swim > 0.05 {
            let k = self.swim;
            let kick = (self.time * 9.0).sin();
            bend[R_LEG] = bend[R_LEG] * (1.0 - k) + (0.25 + kick.max(0.0) * 0.4) * k;
            bend[L_LEG] = bend[L_LEG] * (1.0 - k) + (0.25 + (-kick).max(0.0) * 0.4) * k;
            let s = (self.time * 3.2).sin() * 0.5 + 0.5;
            bend[R_ARM] = bend[R_ARM] * (1.0 - k) - (0.2 + (1.0 - s) * 0.8) * k;
            bend[L_ARM] = bend[L_ARM] * (1.0 - k) - (0.2 + (1.0 - s) * 0.8) * k;
        }
        if crouch > 0.01 {
            bend[R_LEG] += 0.6 * crouch;
            bend[L_LEG] += 0.6 * crouch;
            bend[R_ARM] -= 0.3 * crouch;
            bend[L_ARM] -= 0.3 * crouch;
        }
        if prone > 0.05 {
            bend[R_LEG] = bend[R_LEG] * (1.0 - prone) + 0.25 * prone;
            bend[L_LEG] = bend[L_LEG] * (1.0 - prone) + 0.25 * prone;
            bend[R_ARM] = bend[R_ARM] * (1.0 - prone) - 0.7 * prone;
            bend[L_ARM] = bend[L_ARM] * (1.0 - prone) - 0.7 * prone;
        }
        if p.sitting {
            // Thighs forward, shins hanging straight down.
            bend[R_LEG] = 1.45;
            bend[L_LEG] = 1.45;
            bend[R_ARM] = -0.5;
            bend[L_ARM] = -0.5;
        }
        if p.burning {
            bend[R_ARM] = -0.8;
            bend[L_ARM] = -0.8;
        }
        if self.swing > 0.0 {
            // Cocked before the blow, nearly straight at its height.
            let s = (self.swing * PI).sin();
            bend[R_ARM] = -0.15 - 0.8 * (1.0 - s);
        }
        // Head follows the camera, relative to the body.
        let head_yaw = wrap(p.look_yaw + PI - self.body_yaw).clamp(-1.3, 1.3);
        // Lying down the head lifts to look ahead instead of at the ground.
        let head_lift = -prone * 1.25;
        rot[HEAD] = Quat::from_rotation_y(head_yaw) * Quat::from_rotation_x(head_lift - p.look_pitch * (1.0 - self.swim * 0.6) * (1.0 - prone * 0.5)) * rot[HEAD];

        for (i, part) in def.parts.iter().enumerate().take(6) {
            let pivot = Vec3::new(part.pivot.0 as f32, part.pivot.1 as f32, part.pivot.2 as f32);
            // The body rotates around the hips, everything above follows it.
            let body = if i == BODY || i == HEAD || i == R_ARM || i == L_ARM {
                let hip = Vec3::new(0.0, 12.0, 0.0);
                Mat4::from_translation(hip) * Mat4::from_quat(rot[BODY]) * Mat4::from_translation(-hip)
            } else {
                Mat4::IDENTITY
            };
            let own = if i == BODY { Mat4::IDENTITY } else { Mat4::from_translation(pivot) * Mat4::from_quat(rot[i]) * Mat4::from_translation(-pivot) };
            b.m[i] = (root * body * Mat4::from_translation(shift[i]) * own).to_cols_array_2d();
            if bend[i] != 0.0 {
                b.set_bend(i, part, bend[i]);
            }
        }
        // Talking: the mouth opens downward from its top edge, two pixels above the chin.
        if self.talk > 0.03 && def.parts.len() <= MOUTH_PART {
            let top = def.parts[HEAD].origin.1 as f32 + 2.0;
            let open = Mat4::from_translation(Vec3::Y * top) * Mat4::from_scale(Vec3::new(1.0, 0.2 + self.talk * 1.1, 1.0)) * Mat4::from_translation(-Vec3::Y * top);
            b.m[MOUTH_PART] = (Mat4::from_cols_array_2d(&b.m[HEAD]) * open).to_cols_array_2d();
        }
        b
    }

    /// First person: only the right arm, held in front of the camera. `eye` is relative to
    /// the camera's block; `bob` is the walking phase for the sway.
    /// Camera space: x right, y up, -z forward.
    fn camera(eye: Vec3, yaw: f32, pitch: f32) -> Mat4 {
        Mat4::from_translation(eye) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(pitch)
    }

    /// Where the right arm sits in camera space, with walking sway and the swing. The shoulder
    /// is the origin and the arm hangs along -Y from it.
    fn arm_place(&self) -> Mat4 {
        let s = if self.swing > 0.0 { (self.swing * PI).sin() } else { 0.0 };
        let s2 = if self.swing > 0.0 { (self.swing.sqrt() * PI).sin() } else { 0.0 };
        let bob = self.walk;
        let a = self.amount.min(1.0);
        let sway = Vec3::new((bob).sin() * 0.03 * a, -((bob).cos().abs()) * 0.035 * a, 0.0);
        let ph = self.swing.max(0.0);
        let (swing_off, turn, tip) = match self.swing_style {
            // Slash: the fist sweeps from the right across to the left.
            1 => (Vec3::new(0.2 * s - 0.55 * ph * s, 0.12 * s, -0.22 * s), 0.25 + s * (0.7 - 1.6 * ph), 1.75 + s * 0.15),
            // Thrust: straight into the screen and back.
            2 => (Vec3::new(-0.12 * s, 0.06 * s, -0.5 * s), 0.25 - s * 0.15, 1.75 + s * 0.05),
            // Uppercut: from low up past the middle.
            3 => (Vec3::new(-0.15 * s, -0.25 * s + 0.5 * ph * s, -0.2 * s), 0.25 - s * 0.4, 1.75 + s * (1.1 - 1.2 * ph)),
            _ => (Vec3::new(-0.18 * s2, 0.12 * (s2 * 2.0 * PI / PI).sin().abs() * 0.5, -0.25 * s), 0.25 - s * 0.6, 1.75 + s * 0.5),
        };
        // Tipping the arm forward makes it reach into the screen.
        Mat4::from_translation(Vec3::new(0.40, -0.46, -0.30) + sway + swing_off)
            * Mat4::from_rotation_y(turn)
            * Mat4::from_rotation_x(tip)
            * Mat4::from_rotation_z(0.15)
    }

    pub fn first_person_arm(&self, def: &MobDef, eye: Vec3, yaw: f32, pitch: f32, sky: f32, blk: f32) -> Bones {
        let mut b = Bones { light: [sky, blk, 0.0, 0.0], ..Default::default() };
        let zero = [[0.0f32; 4]; 4];
        for m in b.m.iter_mut() {
            *m = zero;
        }
        let cam = Self::camera(eye, yaw, pitch);
        let place = self.arm_place();
        let arm_model = |part: usize, place: Mat4| {
            let p = &def.parts[part];
            let pivot = Vec3::new(p.pivot.0 as f32, p.pivot.1 as f32, p.pivot.2 as f32);
            // Model space has +Z forward; turn it so the arm points into the screen (-Z).
            (cam * place * Mat4::from_rotation_y(PI) * Mat4::from_scale(Vec3::splat(1.0 / 16.0)) * Mat4::from_translation(-pivot)).to_cols_array_2d()
        };
        if self.mantle > 0.0 {
            // Climbing a ledge: both hands grip the edge in front and stay on it while the view
            // rises past, pressing down until they drop out of sight below. Placed level, not
            // tipped with the look, since the hands hold onto the world.
            let flat = Mat4::from_translation(eye) * Mat4::from_rotation_y(yaw);
            let (f, over) = mantle_reach(self.mantle);
            let grab = (self.mantle / voxel::physics::MANTLE_GRAB).min(1.0);
            // On the edge while it is in sight; once the view has risen past it the hands keep
            // pressing at the bottom of the view, then slide down out of it while swinging over.
            let ledge = self.ledge - voxel::physics::EYE_HEIGHT;
            let up = ledge.clamp(-0.32, 0.1) - over * 0.6;
            // While the edge is still above the view the hands show on the wall's face below
            // it, stretched up; from there on they rest on top.
            let below = ((ledge - up) / 0.15).clamp(0.0, 1.0);
            let ahead = (f.max(0.0) + 0.22) * (1.0 - below) + 0.27 * below;
            for (part, side) in [(R_ARM, 1.0f32), (L_ARM, -1.0f32)] {
                // Reaching out from below at first, then on the edge.
                let hand = Vec3::new(side * 0.27, up - (1.0 - grab) * 0.35, -ahead);
                let shoulder = Vec3::new(side * 0.42, -0.62, -0.1);
                let d = (hand - shoulder).normalize();
                let place = Mat4::from_translation(hand - d * 0.62) * Mat4::from_quat(Quat::from_rotation_arc(Vec3::NEG_Y, d));
                b.m[part] = arm_model(part, cam.inverse() * flat * place);
            }
            return b;
        }
        let w = self.underwater;
        if w > 0.05 && self.swing <= 0.0 {
            // Under water: both arms do the breaststroke. Reach ahead together, sweep out and
            // back, then fold in under the chin and reach again.
            let ph = self.stroke.fract() * 2.0 * PI;
            let sweep = (1.0 - ph.cos()) * 0.5; // 0 ahead, 1 fully out
            let pull = ph.sin().max(0.0); // pulling back during the outward half
            let fold = (-ph.sin()).max(0.0); // elbows in on the way back
            for (part, side) in [(R_ARM, 1.0f32), (L_ARM, -1.0f32)] {
                let shoulder = Vec3::new(side * (0.16 + sweep * 0.14), -0.31 - fold * 0.08 + (1.0 - w) * -0.5, -0.30 + pull * 0.12 + fold * 0.1);
                let place = Mat4::from_translation(shoulder)
                    * Mat4::from_rotation_y(-side * (sweep * 0.55 - 0.1))
                    * Mat4::from_rotation_x(1.62 - fold * 0.35 - pull * 0.15)
                    * Mat4::from_rotation_z(side * 0.1);
                b.m[part] = arm_model(part, place);
            }
            return b;
        }
        b.m[R_ARM] = arm_model(R_ARM, place);
        b
    }
}

/// How far ahead of the body the ledge's edge is (blocks, negative once the body has swung
/// past it), and 0..1 through the swing over the top.
fn mantle_reach(u: f32) -> (f32, f32) {
    use voxel::physics::MANTLE_PULL;
    let x = ((u - MANTLE_PULL) / (1.0 - MANTLE_PULL)).clamp(0.0, 1.0);
    let over = x * x * (3.0 - 2.0 * x);
    (0.4 - 0.8 * over, over)
}

/// The arm's forward swing (radians from hanging down) to reach a hand `ahead` blocks in front
/// and `up` blocks above the shoulder.
fn mantle_arm(ahead: f32, up: f32) -> f32 {
    ahead.atan2(-up)
}

impl Animator {
    /// First-person view bobbing while walking on the ground, like the classic option.
    pub fn view_bob(&self, yaw: f32, on_ground: bool) -> Vec3 {
        if !on_ground {
            return Vec3::ZERO;
        }
        let a = self.amount.min(1.2);
        let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
        right * self.walk.sin() * 0.035 * a + Vec3::Y * -(self.walk.cos().abs()) * 0.05 * a
    }
}
