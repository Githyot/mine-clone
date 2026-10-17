//! Bosses of Mass Destruction idea: the Nether Gauntlet's special attacks (special 4 in
//! `bosses.rs` style()). From afar the fist punches: it hurls itself at you, leaving a trail
//! of embers. Up close it opens and its eye fires a laser, a tight stream of fire bolts.
//! The bosses wait in their lairs (tools/gen_mod_structures.py, group bomd) as dwellers.

use glam::Vec3;

use crate::audio::Sfx;
use crate::Game;

impl Game {
    pub(crate) fn gauntlet_special(&mut self, i: usize, angry: bool) {
        let player = self.body.eye();
        let glow = self.glow_layer();
        let m = &mut self.mobs.list[i];
        let pos = m.body.pos + Vec3::Y * 2.0;
        let to = player - pos;
        let dir = to.normalize_or_zero();
        if to.length() > 7.0 {
            // Punch: a straight dash at the player.
            let speed = if angry { 22.0 } else { 17.0 };
            m.body.vel = dir * speed;
            for k in 0..14 {
                let back = -dir * (k as f32 * 0.25);
                self.particles.glow(pos + back, -dir * 2.0 + Vec3::Y, glow, [1.4, 0.5, 0.1], 0.3, 0.6);
            }
            self.sfx(Sfx::Explode);
        } else {
            // Laser: bolts in a line, slow to fast so they string out into a beam.
            let n = if angry { 10 } else { 7 };
            for k in 0..n {
                let v = 10.0 + k as f32 * 2.2;
                self.enemy_bolt(pos + dir * 1.5, dir * v, 0, if angry { 3.0 } else { 2.0 });
            }
            self.sfx(Sfx::Cast(0));
        }
    }
}
