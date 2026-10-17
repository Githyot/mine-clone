//! Bosses (Terraria's loop: craft a summon item, fight, its drops open the next tier). The
//! first is the Forest Guardian, woken at night with an ancient seed. It walks you down and
//! swings, throws volleys of thorns, stomps the ground, and at half health calls the dead up
//! out of the roots. A bar at the top of the screen shows its health, with a mark at the
//! half where it turns angry.

use glam::Vec3;

use crate::audio::Sfx;
use crate::Game;

#[derive(Default)]
pub struct BossState {
    /// Seconds until the next special attack.
    attack_t: f32,
    /// Which special comes next.
    turn: u32,
    /// Called its helpers already (once, at half health).
    pub(crate) called_help: bool,
    /// Bosses beaten in this world (by mob name), saved with the level.
    pub defeated: Vec<String>,
}

impl BossState {
    pub fn new(defeated: Vec<String>) -> BossState {
        BossState { attack_t: 3.0, defeated, ..Default::default() }
    }
}

impl Game {
    /// The boss in the world, if one is up and alive.
    fn boss_index(&self) -> Option<usize> {
        self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].boss)
    }

    fn is_night(&self) -> bool {
        let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
        (a.sin() * 2.2 + 0.45) < 0.3
    }

    /// Right click with the ancient seed: wake the Forest Guardian (at night, one at a time).
    pub(crate) fn use_ancient_seed(&mut self) -> bool {
        if self.boss_index().is_some() {
            self.toast("O guardião já está acordado", 3.0);
            return true;
        }
        if !self.is_night() {
            self.toast("A semente só desperta à noite", 3.0);
            return true;
        }
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let at = self.body.pos + flat * 12.0;
        let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
        let y = self.world.surface_y(x, z).map(|y| y + 1).unwrap_or(self.body.pos.y.floor() as i32);
        if !self.mobs.spawn("forest_guardian", Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5)) {
            return false;
        }
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        self.boss.attack_t = 4.0;
        self.boss.called_help = false;
        self.weather.flash = 0.6;
        self.sfx(Sfx::Thunder(0.6));
        self.toast("O Guardião da Floresta despertou!", 4.0);
        true
    }

    /// The boss's special attacks, on top of its walk-and-swing.
    pub(crate) fn update_boss(&mut self, dt: f32) {
        let Some(i) = self.boss_index() else { return };
        if dt == 0.0 {
            return;
        }
        let void = self.mobs.reg.defs[self.mobs.list[i].def].name == crate::end::GUARDIAN;
        if void {
            self.update_void_guardian(i, dt);
        }
        let player = self.body.pos;
        let m = &mut self.mobs.list[i];
        // A boss never loses interest.
        m.aggro = true;
        let angry = m.health < m.max_health * 0.5;
        let pos = m.body.pos;
        let head = pos + Vec3::Y * 4.2;
        let dist = (player - pos).length();
        if dist > 48.0 {
            return;
        }
        if angry && !self.boss.called_help {
            self.boss.called_help = true;
            for k in 0..3 {
                let a = k as f32 * 2.1;
                let at = pos + Vec3::new(a.cos() * 3.0, 0.5, a.sin() * 3.0);
                self.mobs.spawn(if void { "void_walker" } else { "zombie" }, at);
            }
            self.sfx(Sfx::Thunder(0.4));
            self.toast(if void { "O vazio chama seus andarilhos!" } else { "As raízes acordam os mortos!" }, 3.0);
        }
        self.boss.attack_t -= dt;
        if self.boss.attack_t > 0.0 {
            return;
        }
        self.boss.attack_t = if angry { 2.4 } else { 3.6 };
        self.boss.turn += 1;
        let stomp = self.boss.turn % 3 == 0 && dist < 9.0;
        if stomp {
            // Stomp: the ground shakes; standing close throws you up and hurts.
            let ground = self.reg.id_or_air(if void { "end_stone" } else { "dirt" });
            let (layer, tint) = self.block_look(ground);
            for k in 0..12 {
                let a = k as f32 / 12.0 * std::f32::consts::TAU;
                let p = pos + Vec3::new(a.cos() * 2.5, -0.5, a.sin() * 2.5);
                self.particles.block_bits(p.floor().as_ivec3(), layer, tint, 3, 1.4);
            }
            self.sfx(Sfx::Explode);
            if dist < 7.0 && self.body.on_ground {
                let away = Vec3::new(player.x - pos.x, 0.0, player.z - pos.z).normalize_or_zero();
                self.body.vel += away * 6.0 + Vec3::Y * 9.0;
                self.damage(if angry { 6.0 } else { 4.0 }, Some(pos));
            }
        } else {
            // A volley of thorns at the player.
            let n = if angry { 5 } else { 3 };
            let to = (self.body.eye() - head).normalize_or_zero();
            let side = to.cross(Vec3::Y).normalize_or_zero();
            for k in 0..n {
                let a = (k as f32 - (n - 1) as f32 / 2.0) * 0.14;
                let dir = (to * a.cos() + side * a.sin()).normalize_or_zero();
                let school = if void { 3 } else { 4 };
                self.enemy_bolt(head + dir, dir * 17.0, school, if angry { 4.0 } else { 3.0 });
            }
            self.sfx(Sfx::Cast(if void { 3 } else { 4 }));
        }
    }

    /// Called when a boss dies.
    pub(crate) fn boss_defeated(&mut self, name: &str, display: &str) {
        if !self.boss.defeated.iter().any(|d| d == name) {
            self.boss.defeated.push(name.to_string());
        }
        self.toast(format!("{display} foi derrotado!"), 5.0);
        if name == crate::end::GUARDIAN {
            self.open_end_exit();
        }
        self.sfx(Sfx::Thunder(0.3));
    }

    /// The boss bar across the top of the screen.
    pub(crate) fn draw_boss_bar(&mut self, w: f32) {
        let Some(i) = self.boss_index() else { return };
        let m = &self.mobs.list[i];
        if (m.body.pos - self.body.pos).length() > 64.0 {
            return;
        }
        let name = self.mobs.reg.defs[m.def].display_name.clone();
        let frac = (m.health / m.max_health).clamp(0.0, 1.0);
        let s = self.ui.scale;
        let bw = 182.0 * s;
        let x = (w - bw) / 2.0;
        let y = 14.0 * s;
        let tw = self.ui.text_width(&name, s);
        self.ui.text((w - tw) / 2.0, y - 10.0 * s, &name, [255, 255, 255, 255]);
        self.ui.rect(x - s, y - s, bw + 2.0 * s, 7.0 * s, [0, 0, 0, 200]);
        self.ui.rect(x, y, bw, 5.0 * s, [60, 20, 20, 255]);
        self.ui.rect(x, y, bw * frac, 5.0 * s, [70, 170, 60, 255]);
        self.ui.rect(x, y, bw * frac, s, [130, 220, 110, 255]);
        // The half-way mark where it turns angry.
        self.ui.rect(x + bw * 0.5 - s * 0.5, y - s, s, 7.0 * s, [230, 230, 230, 255]);
    }
}
