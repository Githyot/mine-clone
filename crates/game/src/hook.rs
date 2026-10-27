//! Grappling hooks (Terraria's idea): right click throws the hook along your view; when it bites
//! into a block it reels you in and holds you there. Jump or sneak lets go; throwing again moves
//! it. Gem hooks reach further and pull faster.

use glam::Vec3;
use voxel::physics::Terrain;

use crate::audio::{Material, Sfx};
use crate::Game;

/// (item, reach in blocks, flight speed, pull speed, chain tint)
const HOOKS: &[(&str, f32, f32, f32, [f32; 3])] = &[
    ("grappling_hook", 18.0, 36.0, 11.0, [0.75, 0.75, 0.8]),
    ("amethyst_hook", 20.0, 40.0, 12.0, [1.0, 0.5, 1.4]),
    ("topaz_hook", 22.0, 42.0, 13.0, [1.4, 1.1, 0.4]),
    ("sapphire_hook", 24.0, 44.0, 14.0, [0.5, 0.8, 1.5]),
    ("ruby_hook", 26.0, 48.0, 15.0, [1.5, 0.45, 0.45]),
    ("verdite_hook", 32.0, 56.0, 18.0, [0.5, 1.5, 0.6]),
];

pub fn is_hook(name: &str) -> bool {
    HOOKS.iter().any(|h| h.0 == name)
}

#[derive(Default, Clone, Copy, PartialEq)]
enum State {
    #[default]
    Idle,
    Flying { pos: Vec3, vel: Vec3, kind: usize },
    Biting { anchor: Vec3, kind: usize },
}

#[derive(Default)]
pub struct Hook {
    state: State,
    /// Seconds before another throw (one click is one throw).
    cd: f32,
}

impl Hook {
    pub fn attached(&self) -> bool {
        matches!(self.state, State::Biting { .. })
    }
}

impl Game {
    pub(crate) fn throw_hook(&mut self, name: &str) -> bool {
        if self.t5.hook.cd > 0.0 {
            return true;
        }
        let Some(kind) = HOOKS.iter().position(|h| h.0 == name) else { return false };
        let f = self.forward();
        let from = self.body.eye() + f * 0.4 - Vec3::Y * 0.2;
        self.t5.hook.state = State::Flying { pos: from, vel: f * HOOKS[kind].2, kind };
        self.t5.hook.cd = 0.25;
        self.sfx(Sfx::Place(Material::Metal));
        self.anim.start_swing();
        true
    }

    pub(crate) fn update_hook(&mut self, dt: f32, _use_held: bool) {
        self.t5.hook.cd -= dt;
        if dt == 0.0 {
            // Still pictures keep the chain in view.
            self.draw_chain();
            return;
        }
        let me = self.body.eye() - Vec3::Y * 0.3;
        match self.t5.hook.state {
            State::Idle => {}
            State::Flying { mut pos, vel, kind } => {
                let step = vel * dt;
                let n = (step.length() / 0.25).ceil().max(1.0) as usize;
                let mut bit = None;
                for _ in 0..n {
                    pos += step / n as f32;
                    if self.world.solid(pos.floor().as_ivec3()) {
                        bit = Some(pos - step / n as f32 * 0.5);
                        break;
                    }
                }
                if let Some(anchor) = bit {
                    self.t5.hook.state = State::Biting { anchor, kind };
                    self.sfx(Sfx::Hit(Material::Stone));
                } else if pos.distance(me) > HOOKS[kind].1 {
                    self.t5.hook.state = State::Idle;
                } else {
                    self.t5.hook.state = State::Flying { pos, vel, kind };
                }
            }
            State::Biting { anchor, kind } => {
                let playing = self.screen == crate::Screen::Playing;
                let let_go = playing
                    && (self.keys.contains(&winit::keyboard::KeyCode::Space)
                        || self.keys.contains(&winit::keyboard::KeyCode::ShiftLeft)
                        || self.keys.contains(&winit::keyboard::KeyCode::ShiftRight));
                if let_go || !self.world.solid(anchor.floor().as_ivec3()) && !self.world.solid((anchor + (anchor - me).normalize_or_zero() * 0.3).floor().as_ivec3()) {
                    self.t5.hook.state = State::Idle;
                    if let_go && self.keys.contains(&winit::keyboard::KeyCode::Space) {
                        // A little hop off the hook.
                        self.body.vel.y = self.body.vel.y.max(7.0);
                    }
                } else {
                    let to = anchor - me;
                    let d = to.length();
                    if d > 1.2 {
                        self.body.vel = to / d * HOOKS[kind].3;
                    } else {
                        // Hanging on: hold still against gravity.
                        self.body.vel = Vec3::ZERO;
                    }
                    self.fall_peak = self.body.pos.y;
                }
            }
        }
        self.draw_chain();
    }

    /// Links of the chain from the hand to the hook, as short-lived glowing bits.
    fn draw_chain(&mut self) {
        let (end, kind) = match self.t5.hook.state {
            State::Idle => return,
            State::Flying { pos, kind, .. } => (pos, kind),
            State::Biting { anchor, kind } => (anchor, kind),
        };
        let from = self.body.eye() - Vec3::Y * 0.35 + self.forward() * 0.3;
        let layer = self.glow_layer();
        let d = end - from;
        let n = (d.length() / 0.45).ceil().max(1.0) as usize;
        let tint = HOOKS[kind].4;
        for k in 0..=n {
            let p = from + d * (k as f32 / n as f32);
            self.particles.glow(p, Vec3::ZERO, layer, tint, 0.045, 0.04);
        }
        self.particles.glow(end, Vec3::ZERO, layer, [tint[0] * 1.3, tint[1] * 1.3, tint[2] * 1.3], 0.12, 0.04);
    }
}
