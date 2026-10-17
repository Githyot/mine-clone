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

/// Summon item, boss, night only, what the screen says.
const SUMMONS: &[(&str, &str, bool, &str)] = &[
    ("ancient_seed", "forest_guardian", true, "O Guardião da Floresta despertou!"),
    ("lich_phylactery", "lich", true, "O Lich se levanta do gelo!"),
    ("obsidian_heart", "obsidilith", false, "O Obsidilito desperta!"),
    ("void_seed", "void_blossom", false, "A Flor do Vazio desabrocha!"),
];

pub fn is_summon(item: &str) -> bool {
    SUMMONS.iter().any(|s| s.0 == item)
}

/// How each boss fights: helpers it calls at half health, the school of its bolts, the
/// ground its stomp throws up, what the screen says when the helpers come, and its special:
/// 0 stomp, 1 frost nova (a ring of bolts), 2 fire pillars under the player, 3 poison rain.
struct Style {
    helper: &'static str,
    school: u8,
    ground: &'static str,
    call: &'static str,
    special: u8,
}

fn style(name: &str) -> Style {
    match name {
        crate::end::GUARDIAN => Style { helper: "void_walker", school: 3, ground: "end_stone", call: "O vazio chama seus andarilhos!", special: 0 },
        "lich" => Style { helper: "skeleton", school: 1, ground: "snow", call: "O Lich ergue os mortos congelados!", special: 1 },
        "obsidilith" => Style { helper: "ash_skeleton", school: 3, ground: "obsidian", call: "O Obsidilito chama as cinzas!", special: 2 },
        "void_blossom" => Style { helper: "spider", school: 4, ground: "grass_block", call: "A flor solta suas aranhas!", special: 3 },
        _ => Style { helper: "zombie", school: 4, ground: "dirt", call: "As raízes acordam os mortos!", special: 0 },
    }
}

impl Game {
    /// The boss in the world, if one is up and alive.
    fn boss_index(&self) -> Option<usize> {
        self.mobs.list.iter().position(|m| m.alive() && self.mobs.reg.defs[m.def].boss)
    }

    pub(crate) fn is_night(&self) -> bool {
        let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
        (a.sin() * 2.2 + 0.45) < 0.3
    }

    /// Right click with a summon item: wake its boss (one at a time; some only at night).
    pub(crate) fn use_summon(&mut self, item: &str) -> bool {
        let Some(&(_, boss, night, text)) = SUMMONS.iter().find(|s| s.0 == item) else { return false };
        if self.boss_index().is_some() {
            self.toast("Já há um chefe acordado", 3.0);
            return true;
        }
        if night && !self.is_night() {
            self.toast("Isso só desperta à noite", 3.0);
            return true;
        }
        let f = self.forward();
        let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let at = self.body.pos + flat * 12.0;
        let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
        let y = self.world.surface_y(x, z).map(|y| y + 1).unwrap_or(self.body.pos.y.floor() as i32);
        if !self.mobs.spawn(boss, Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5)) {
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
        self.toast(text, 4.0);
        true
    }

    /// The boss's special attacks, on top of its walk-and-swing.
    pub(crate) fn update_boss(&mut self, dt: f32) {
        let Some(i) = self.boss_index() else { return };
        if dt == 0.0 {
            return;
        }
        let name = self.mobs.reg.defs[self.mobs.list[i].def].name.clone();
        if name == crate::end::GUARDIAN {
            self.update_void_guardian(i, dt);
        }
        if crate::nether_end::own_boss(&name) {
            return;
        }
        let st = style(&name);
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
                self.mobs.spawn(st.helper, at);
            }
            self.sfx(Sfx::Thunder(0.4));
            self.toast(st.call, 3.0);
        }
        self.boss.attack_t -= dt;
        if self.boss.attack_t > 0.0 {
            return;
        }
        self.boss.attack_t = if angry { 2.4 } else { 3.6 };
        self.boss.turn += 1;
        let special = self.boss.turn % 3 == 0;
        let glow = self.glow_layer();
        if special && st.special == 1 {
            // Frost nova: a ring of ice bolts all around.
            let n = if angry { 16 } else { 12 };
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                let dir = Vec3::new(a.cos(), -0.05, a.sin());
                self.enemy_bolt(pos + Vec3::Y * 2.0 + dir, dir * 12.0, 1, 3.0);
            }
            self.sfx(Sfx::Cast(1));
        } else if special && st.special == 2 {
            // Fire pillars: purple flame bursts up from under the player and around.
            let base = player.floor();
            for k in 0..if angry { 5 } else { 3 } {
                let a = k as f32 * 2.4;
                let off = if k == 0 { Vec3::ZERO } else { Vec3::new(a.cos(), 0.0, a.sin()) * 2.5 };
                for h in 0..10 {
                    let v = Vec3::new(0.0, 6.0 + h as f32, 0.0);
                    self.particles.glow(base + off + Vec3::new(0.5, 0.1, 0.5), v, glow, [0.9, 0.4, 1.4], 0.25, 0.9);
                }
            }
            self.sfx(Sfx::Explode);
            if self.body.on_ground {
                self.body.vel.y += 8.0;
                self.damage(if angry { 7.0 } else { 5.0 }, Some(pos));
            }
        } else if special && st.special == 3 {
            // Poison rain: thorns falling around the player from above.
            for k in 0..if angry { 9 } else { 6 } {
                let a = k as f32 * 2.4;
                let r = 1.0 + (k % 3) as f32 * 1.5;
                let from = player + Vec3::new(a.cos() * r, 9.0, a.sin() * r);
                self.enemy_bolt(from, Vec3::new(0.0, -14.0, 0.0), 4, 3.0);
            }
            self.sfx(Sfx::Cast(4));
        } else if special && dist < 9.0 {
            // Stomp: the ground shakes; standing close throws you up and hurts.
            let ground = self.reg.id_or_air(st.ground);
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
            // A volley of bolts at the player.
            let n = if angry { 5 } else { 3 };
            let to = (self.body.eye() - head).normalize_or_zero();
            let side = to.cross(Vec3::Y).normalize_or_zero();
            for k in 0..n {
                let a = (k as f32 - (n - 1) as f32 / 2.0) * 0.14;
                let dir = (to * a.cos() + side * a.sin()).normalize_or_zero();
                self.enemy_bolt(head + dir, dir * 17.0, st.school, if angry { 4.0 } else { 3.0 });
            }
            self.sfx(Sfx::Cast(st.school));
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
        self.nether_end_boss_defeated(name);
        self.sfx(Sfx::Thunder(0.3));
    }

    /// The boss bar across the top of the screen.
    pub(crate) fn draw_boss_bar(&mut self, w: f32) {
        let Some(i) = self.boss_index() else { return };
        let m = &self.mobs.list[i];
        if (m.body.pos - self.body.pos).length() > 128.0 {
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
