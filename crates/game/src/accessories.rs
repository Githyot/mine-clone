//! Accessories (Terraria's idea): five slots beside the inventory for things that change how you
//! move and fight. Boots run faster, a cloud in a bottle gives a second jump, wings fly for a
//! moment and then glide, a cape dashes on a double tap, claws climb walls, the horseshoe takes
//! the sting out of falls; rings, amulets and emblems help mana, magic and melee.

use glam::Vec3;
use voxel::physics::{Terrain, WALK_SPEED};
use winit::keyboard::KeyCode;

use crate::audio::Sfx;
use crate::survival::Slot;
use crate::Game;

pub const ACC_SLOTS: usize = 5;

const ACCESSORIES: &[(&str, &str)] = &[
    ("running_boots", "Corre 30% mais rápido"),
    ("frost_spark_boots", "Corre 50% mais rápido"),
    ("cloud_bottle", "Um segundo pulo no ar"),
    ("lucky_horseshoe", "Sem dano de queda"),
    ("wind_cape", "Toque duas vezes A ou D para avançar"),
    ("shiny_balloon", "Pula mais alto"),
    ("feather_wings", "Segure Espaço no ar para voar um pouco e planar"),
    ("angel_wings", "Voa mais tempo e plana; sem dano de queda"),
    ("mana_ring", "Mana volta mais rápido"),
    ("star_amulet", "+5 estrelas de mana"),
    ("warrior_emblem", "+15% de dano corpo a corpo"),
    ("arcane_emblem", "+15% de dano mágico"),
    ("obsidian_charm", "7 s sem queimar na lava, sem recuo"),
    ("regen_band", "Vida volta aos poucos"),
    ("depth_meter", "Mostra a profundidade e onde você está"),
    ("climbing_claws", "Segure Espaço contra a parede para escalar"),
    ("guardian_shield", "+4 de armadura, sem recuo"),
];

pub fn is_accessory(name: &str) -> bool {
    ACCESSORIES.iter().any(|a| a.0 == name)
}

/// What an accessory does, for its tooltip.
pub fn describe(name: &str) -> Option<&'static str> {
    ACCESSORIES.iter().find(|a| a.0 == name).map(|a| a.1)
}

/// Movement state the accessories need from frame to frame.
#[derive(Default)]
pub struct Moves {
    wing_t: f32,
    jumped_twice: bool,
    space_was: bool,
    was_ground: bool,
    balloon_done: bool,
    a_was: bool,
    d_was: bool,
    last_a: f32,
    last_d: f32,
    dash_cd: f32,
    pub lava_t: f32,
    regen_t: f32,
    /// Flying on wings right now (for the HUD).
    pub flying: bool,
}

impl Game {
    /// Is this accessory worn?
    pub(crate) fn wears(&self, name: &str) -> bool {
        self.t5.acc.iter().flatten().any(|s| self.items.get(s.item).name == name)
    }

    /// Armour points from accessories (the guardian shield, the "Guardião" modifier).
    pub(crate) fn accessory_armor(&self) -> u32 {
        let mut n = if self.wears("guardian_shield") { 4 } else { 0 };
        for s in self.t5.acc.iter().flatten() {
            n += crate::affixes::of(s).map(|a| a.armor).unwrap_or(0);
        }
        n
    }

    pub(crate) fn melee_bonus(&self) -> f32 {
        if self.wears("warrior_emblem") {
            1.15
        } else {
            1.0
        }
    }

    pub(crate) fn magic_bonus(&self) -> f32 {
        if self.wears("arcane_emblem") {
            1.15
        } else {
            1.0
        }
    }

    /// Falls don't hurt (horseshoe, angel wings).
    pub(crate) fn no_fall_damage(&self) -> bool {
        self.wears("lucky_horseshoe") || self.wears("angel_wings") || self.wears("feather_wings") && self.t5.moves.flying
    }

    pub(crate) fn no_knockback(&self) -> bool {
        self.wears("obsidian_charm") || self.wears("guardian_shield")
    }

    /// Lava doesn't burn for a while with the obsidian charm.
    pub(crate) fn lava_safe(&mut self, dt: f32) -> bool {
        if !self.wears("obsidian_charm") {
            return false;
        }
        self.t5.moves.lava_t += dt;
        self.t5.moves.lava_t < 7.0
    }

    /// Right click with an accessory: into the first free slot (or swap with the first one).
    pub(crate) fn wear_accessory(&mut self) -> bool {
        let sel = self.selected;
        let Some(st) = self.surv.inv.slots[sel] else { return false };
        let name = self.items.get(st.item).name.clone();
        if self.wears(&name) {
            self.toast("Você já usa esse acessório", 2.0);
            return true;
        }
        let i = self.t5.acc.iter().position(|a| a.is_none()).unwrap_or(0);
        self.surv.inv.slots[sel] = self.t5.acc[i].take();
        self.t5.acc[i] = Some(st);
        self.sfx(Sfx::Place(crate::audio::Material::Cloth));
        let d = self.items.get(st.item).display_name.clone();
        self.toast(format!("{d}: {}", describe(&name).unwrap_or("")), 3.0);
        true
    }

    pub(crate) fn update_accessories(&mut self, dt: f32) {
        // Mana: the amulet adds stars, the ring brings them back faster.
        self.magic.extra_max = if self.wears("star_amulet") { 10.0 } else { 0.0 };
        if self.wears("mana_ring") {
            let cap = self.magic.max_mana + self.magic.extra_max;
            self.magic.mana = (self.magic.mana + 2.0 * dt).min(cap);
        }
        if !self.survival || self.screen == crate::Screen::Dead || dt == 0.0 {
            self.body.speed_override = None;
            return;
        }
        if self.wears("regen_band") && self.health < self.max_health {
            self.t5.moves.regen_t += dt;
            if self.t5.moves.regen_t > 4.0 {
                self.t5.moves.regen_t = 0.0;
                self.health = (self.health + 1.0).min(self.max_health);
            }
        }
        // Out of lava, the charm cools down again.
        let feet = self.body.pos.floor().as_ivec3();
        if self.world.fluid(self.world.block_or_air(feet)) != 2 {
            self.t5.moves.lava_t = (self.t5.moves.lava_t - dt * 2.0).max(0.0);
        }
        // Speed: boots, and the "Ágil" modifier on any accessory.
        let mut speed = 1.0;
        if self.wears("frost_spark_boots") {
            speed *= 1.5;
        } else if self.wears("running_boots") {
            speed *= 1.3;
        }
        for s in self.t5.acc.iter().flatten() {
            speed *= crate::affixes::of(s).map(|a| a.move_speed).unwrap_or(1.0);
        }
        self.body.speed_override = if speed > 1.001 && !self.body.in_water { Some(WALK_SPEED * speed) } else { None };

        let playing = self.screen == crate::Screen::Playing;
        let space = playing && self.keys.contains(&KeyCode::Space);
        let pressed = space && !self.t5.moves.space_was;
        self.t5.moves.space_was = space;
        let ground = self.body.on_ground || self.body.in_water;
        if ground {
            self.t5.moves.jumped_twice = false;
            self.t5.moves.wing_t = if self.wears("angel_wings") { 3.2 } else { 1.6 };
            self.t5.moves.flying = false;
            self.t5.moves.balloon_done = false;
        }
        // The balloon: the jump that just left the ground goes higher.
        if self.t5.moves.was_ground && !self.body.on_ground && self.body.vel.y > 8.0 && !self.t5.moves.balloon_done && self.wears("shiny_balloon") {
            self.body.vel.y *= 1.3;
            self.t5.moves.balloon_done = true;
        }
        self.t5.moves.was_ground = self.body.on_ground;
        let glow = self.glow_layer();
        let hooked = self.t5.hook.attached();
        if !ground && !hooked && self.seat.is_none() && self.riding.is_none() {
            let wings = self.wears("angel_wings") || self.wears("feather_wings");
            if pressed && self.wears("cloud_bottle") && !self.t5.moves.jumped_twice {
                // A puff of cloud and a second jump.
                self.t5.moves.jumped_twice = true;
                self.body.vel.y = 9.5;
                self.fall_peak = self.body.pos.y;
                for _ in 0..14 {
                    let v = Vec3::new(self.surv.rng.f32() - 0.5, -0.4, self.surv.rng.f32() - 0.5) * 3.0;
                    self.particles.glow(self.body.pos, v, glow, [1.3, 1.3, 1.35], 0.18, 0.6);
                }
                self.sfx(Sfx::Place(crate::audio::Material::Cloth));
            } else if space && wings {
                if self.t5.moves.wing_t > 0.0 {
                    self.t5.moves.wing_t -= dt;
                    self.body.vel.y = (self.body.vel.y + 45.0 * dt).min(7.0);
                    self.t5.moves.flying = true;
                } else if self.body.vel.y < -2.5 {
                    // Out of flight time: glide down.
                    self.body.vel.y = -2.5;
                }
                self.fall_peak = self.body.pos.y;
                if self.surv.rng.f32() < dt * 20.0 {
                    let tint = if self.wears("angel_wings") { [1.5, 1.4, 0.8] } else { [1.2, 1.2, 1.2] };
                    self.particles.glow(self.body.pos + Vec3::Y * 1.2, Vec3::new(0.0, -1.5, 0.0), glow, tint, 0.08, 0.5);
                }
            }
            // Claws: hold Space against a wall to climb it.
            if space && self.wears("climbing_claws") && self.against_wall() {
                self.body.vel.y = self.body.vel.y.max(3.5);
                self.fall_peak = self.body.pos.y;
            }
        }
        // The cape: tap A or D twice quickly to dash that way.
        let a = playing && self.keys.contains(&KeyCode::KeyA);
        let d = playing && self.keys.contains(&KeyCode::KeyD);
        self.t5.moves.dash_cd -= dt;
        let mut dash = 0.0;
        if a && !self.t5.moves.a_was {
            if self.now - self.t5.moves.last_a < 0.25 {
                dash = -1.0;
            }
            self.t5.moves.last_a = self.now;
        }
        if d && !self.t5.moves.d_was {
            if self.now - self.t5.moves.last_d < 0.25 {
                dash = 1.0;
            }
            self.t5.moves.last_d = self.now;
        }
        self.t5.moves.a_was = a;
        self.t5.moves.d_was = d;
        if dash != 0.0 && self.t5.moves.dash_cd <= 0.0 && self.wears("wind_cape") {
            self.t5.moves.dash_cd = 1.0;
            let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
            self.body.vel += right * dash * 16.0;
            for _ in 0..12 {
                self.particles.glow(self.body.pos + Vec3::Y * 0.8, -right * dash * 2.0, glow, [0.9, 1.1, 1.2], 0.1, 0.4);
            }
            self.sfx(Sfx::Place(crate::audio::Material::Cloth));
        }
    }

    fn against_wall(&self) -> bool {
        let f = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let p = self.body.pos + f * 0.5 + Vec3::Y * 0.5;
        self.world.solid(p.floor().as_ivec3()) || self.world.solid((p + Vec3::Y).floor().as_ivec3())
    }

    /// Slots of the accessory column, left of the inventory panel at (x0, y0).
    pub(crate) fn accessory_slots(&mut self, x0: f32, y0: f32) -> Vec<(Slot, f32, f32)> {
        let s = self.ui.scale;
        let (px, py) = (x0 - 26.0 * s, y0 + 6.0 * s);
        let ph = (ACC_SLOTS as f32 * 18.0 + 16.0) * s;
        self.ui.rect(px - 2.0 * s, py, 24.0 * s, ph, [198, 198, 198, 255]);
        self.ui.frame(px - 2.0 * s, py, 24.0 * s, ph, s, [255, 255, 255, 255]);
        self.ui.label(px - 1.0 * s, py + 2.0 * s, "Ac.", [64, 64, 64, 255], s);
        (0..ACC_SLOTS).map(|i| (Slot::Acc(i), px + 1.0 * s, py + (12.0 + i as f32 * 18.0) * s)).collect()
    }

    pub(crate) fn draw_accessory_hud(&mut self, _w: f32, h: f32) {
        if !self.survival {
            return;
        }
        let s = self.ui.scale;
        if self.wears("depth_meter") {
            let p = self.body.pos;
            let depth = 64.0 - p.y;
            let where_ = if depth > 0.0 { format!("{:.0} abaixo do mar", depth) } else { format!("{:.0} acima do mar", -depth) };
            let t = format!("Profundidade: {where_}  ({:.0}, {:.0})", p.x, p.z);
            self.ui.text(4.0 * s, h - 24.0 * s, &t, [220, 220, 160, 255]);
        }
        if self.t5.moves.flying && self.t5.moves.wing_t > 0.0 {
            let max = if self.wears("angel_wings") { 3.2 } else { 1.6 };
            let k = (self.t5.moves.wing_t / max).clamp(0.0, 1.0);
            self.ui.rect(4.0 * s, h - 34.0 * s, 40.0 * s, 3.0 * s, [30, 30, 30, 200]);
            self.ui.rect(4.0 * s, h - 34.0 * s, 40.0 * s * k, 3.0 * s, [240, 230, 160, 230]);
        }
    }
}
