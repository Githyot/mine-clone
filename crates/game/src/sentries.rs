//! Sentries (Lote 11): the summoner's towers. A sentry staff plants a turret where you point;
//! it stays there and shoots at monsters in range until you plant more than your sentry slots
//! allow (two, three with the summoner emblem), change worlds or die. They grow with summon
//! damage like the minions in rpg.rs.

use glam::{IVec3, Vec3};
use voxel::physics::Terrain;

use crate::audio::Sfx;
use crate::effects::*;
use crate::item_render::GpuItem;
use crate::rpg::{MOB_BURN, SUMMON};
use crate::Game;

pub struct SentryDef {
    pub staff: &'static str,
    /// The item whose picture is the turret.
    pub sprite: &'static str,
    pub name: &'static str,
    pub damage: f32,
    pub range: f32,
    pub cooldown: f32,
    /// Debuff its shots leave (0 = none).
    pub inflict: u8,
    /// Extra monsters a shot jumps to (lightning).
    pub chain: u8,
    pub tint: [f32; 3],
}

pub const SENTRIES: &[SentryDef] = &[
    SentryDef { staff: "ember_tower_staff", sprite: "sentry_ember", name: "Torre de Brasa", damage: 7.0, range: 14.0, cooldown: 1.1, inflict: MOB_BURN, chain: 0, tint: [1.6, 0.6, 0.2] },
    SentryDef { staff: "frost_crystal_staff", sprite: "sentry_frost", name: "Cristal de Gelo", damage: 6.0, range: 14.0, cooldown: 0.9, inflict: FROSTBURN, chain: 0, tint: [0.6, 1.1, 1.6] },
    SentryDef { staff: "storm_totem_staff", sprite: "sentry_storm", name: "Totem da Tempestade", damage: 9.0, range: 16.0, cooldown: 1.6, inflict: 0, chain: 3, tint: [1.2, 1.2, 1.7] },
    SentryDef { staff: "ballista_cane", sprite: "sentry_ballista", name: "Balista", damage: 22.0, range: 22.0, cooldown: 2.4, inflict: 0, chain: 0, tint: [1.2, 1.0, 0.7] },
];

/// Damage per second of one turret on one monster (balance.rs).
#[cfg(test)]
pub(crate) fn sentry_dps(staff: &str) -> Option<f32> {
    SENTRIES.iter().find(|s| s.staff == staff).map(|s| s.damage / s.cooldown)
}

pub fn sentry_of(staff: &str) -> Option<usize> {
    SENTRIES.iter().position(|s| s.staff == staff)
}

pub struct Sentry {
    pub kind: usize,
    pub pos: Vec3,
    cd: f32,
    /// Where it last aimed (it turns that way).
    aim: Vec3,
}

impl Game {
    pub(crate) fn sentry_slots(&self) -> usize {
        2 + usize::from(self.wears("summoner_emblem"))
    }

    /// Right click with a sentry staff: a turret on the block looked at (or just ahead).
    pub(crate) fn sentry_use(&mut self, name: &str) -> Option<bool> {
        let kind = sentry_of(name)?;
        let cost = 12.0 * self.t5.rpg.stats.mana_cost;
        if self.survival {
            if self.magic.mana < cost {
                self.toast("Mana insuficiente", 1.0);
                return Some(true);
            }
            self.magic.mana -= cost;
        }
        let at = match self.target {
            Some(h) if h.distance < 8.0 => (h.block + h.normal).as_vec3() + Vec3::new(0.5, 0.0, 0.5),
            _ => {
                let f = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
                let p = self.body.pos + f * 2.0;
                // Down onto the ground in front.
                let mut q = p.floor().as_ivec3() + IVec3::Y;
                for _ in 0..6 {
                    if self.world.solid(q - IVec3::Y) {
                        break;
                    }
                    q -= IVec3::Y;
                }
                q.as_vec3() + Vec3::new(0.5, 0.0, 0.5)
            }
        };
        while self.t5.sentries.len() >= self.sentry_slots() {
            self.t5.sentries.remove(0);
        }
        self.t5.sentries.push(Sentry { kind, pos: at, cd: 0.6, aim: self.body.pos });
        let glow = self.glow_layer();
        for _ in 0..20 {
            let v = Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32(), self.surv.rng.f32() - 0.5) * 3.0;
            self.particles.glow(at + Vec3::Y * 0.5, v, glow, SENTRIES[kind].tint, 0.1, 0.6);
        }
        self.sfx(Sfx::Cast(3));
        self.anim.start_swing();
        self.toast(format!("{} ({}/{})", SENTRIES[kind].name, self.t5.sentries.len(), self.sentry_slots()), 1.5);
        Some(true)
    }

    pub(crate) fn update_sentries(&mut self, dt: f32) {
        if self.t5.sentries.is_empty() || dt == 0.0 {
            return;
        }
        // Gone with the player, and left behind in another world.
        if self.screen == crate::Screen::Dead || self.t5.sentries.iter().any(|s| s.pos.distance(self.body.pos) > 160.0) {
            self.t5.sentries.retain(|s| s.pos.distance(self.body.pos) <= 160.0);
            if self.screen == crate::Screen::Dead {
                self.t5.sentries.clear();
            }
        }
        let slots = self.sentry_slots();
        while self.t5.sentries.len() > slots {
            self.t5.sentries.remove(0);
        }
        let power = self.t5.rpg.stats.dmg[SUMMON];
        let glow = self.glow_layer();
        let mut list = std::mem::take(&mut self.t5.sentries);
        for st in list.iter_mut() {
            let def = &SENTRIES[st.kind];
            st.cd -= dt;
            let eye = st.pos + Vec3::Y * 0.8;
            let Some(k) = self.nearest_hostile(eye, def.range) else { continue };
            let c = self.mobs.list[k].body.center();
            st.aim = c;
            if st.cd > 0.0 || !self.clear_line(eye, c) {
                continue;
            }
            st.cd = def.cooldown;
            let mut hits = vec![k];
            // Lightning jumps on to the nearest monsters around the first.
            let mut from = c;
            for _ in 0..def.chain {
                let next = (0..self.mobs.list.len())
                    .filter(|j| !hits.contains(j) && self.mobs.list[*j].alive() && self.is_hostile(*j))
                    .map(|j| (j, self.mobs.list[j].body.center().distance(from)))
                    .filter(|(_, d)| *d < 7.0)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                let Some((j, _)) = next else { break };
                from = self.mobs.list[j].body.center();
                hits.push(j);
            }
            let mut prev = eye;
            for (n, &j) in hits.iter().enumerate() {
                let to = self.mobs.list[j].body.center();
                let seg = to - prev;
                let steps = (seg.length() * 3.0).ceil().max(4.0) as usize;
                for s in 0..steps {
                    let jitter = if def.chain > 0 { Vec3::new(self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5, self.surv.rng.f32() - 0.5) * 0.25 } else { Vec3::ZERO };
                    let size = if def.staff == "ballista_cane" { 0.11 } else { 0.07 };
                    self.particles.glow(prev + seg * (s as f32 / steps as f32) + jitter, seg.normalize_or_zero() * 1.5, glow, def.tint, size, 0.3);
                }
                let dmg = def.damage * power * if n == 0 { 1.0 } else { 0.6 };
                self.mobs.list[j].hurt(dmg, prev);
                if def.inflict != 0 {
                    self.add_mob_debuff(j, def.inflict, 3.0);
                }
                if n == 0 {
                    let d = &self.mobs.reg.defs[self.mobs.list[j].def];
                    self.last_hit = Some((d.display_name.clone(), self.mobs.list[j].health.max(0.0), self.mobs.list[j].max_health, self.now + 3.0));
                }
                prev = to;
            }
            if self.body.pos.distance(st.pos) < 24.0 {
                self.sfx(if def.chain > 0 { Sfx::Cast(2) } else { Sfx::Cast(1) });
            }
        }
        self.t5.sentries = list;
    }

    /// Nothing solid between two points (sentries don't shoot through walls).
    fn clear_line(&self, a: Vec3, b: Vec3) -> bool {
        let d = b - a;
        let n = (d.length() * 2.0).ceil() as i32;
        (1..n).all(|i| !self.world.solid((a + d * (i as f32 / n as f32)).floor().as_ivec3()))
    }

    /// Sentries drawn as standing pictures of their turrets, turned toward their target.
    pub(crate) fn sentry_instances(&self, cam_block: IVec3) -> Vec<GpuItem> {
        let c = cam_block.as_vec3();
        let mut out = Vec::new();
        for st in &self.t5.sentries {
            let def = &SENTRIES[st.kind];
            let Some(id) = self.items.id(def.sprite) else { continue };
            let voxel::item::Look::Sprite(l) = self.items.get(id).look else { continue };
            let layer = (self.item_base + l) as f32;
            let p = st.pos + Vec3::Y * (0.5 + (self.now * 2.0 + st.pos.x).sin() * 0.03) - c;
            let dir = st.aim - st.pos;
            let yaw = dir.x.atan2(dir.z) + std::f32::consts::FRAC_PI_2;
            let light = self.world.light(st.pos.floor().as_ivec3());
            let (sky, blk) = ((light >> 4) as f32 / 15.0, ((light & 15) as f32 / 15.0).max(0.6));
            out.push(GpuItem { pos: [p.x, p.y, p.z, 1.0], info: [yaw, 1.0, sky, blk], layers: [layer, layer, layer, 0.0], rot: [0.0; 4], ..bytemuck::Zeroable::zeroed() });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentries_differ_and_reach() {
        assert_eq!(SENTRIES.len(), 4);
        for s in SENTRIES {
            assert!(s.range >= 10.0 && s.cooldown > 0.0 && s.damage > 0.0, "{}", s.name);
        }
        assert!(sentry_of("ballista_cane").is_some());
    }
}
