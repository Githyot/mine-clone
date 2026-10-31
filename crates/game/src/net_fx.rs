//! Multijogador: what is in flight, seen by everyone (lote 18a). Arrows, tridents, wind
//! charges, the Nether and End's fireballs and every spell bolt are simulated where they were
//! shot (the host for mobs, each player for their own); the others get a picture of them
//! (`ShotFx`) about 20 times a second, moved on between updates so they fly smoothly. Damage
//! does not travel this way: it still comes as `Hurt` / `HurtMob`, so nothing hits twice.

use glam::{IVec3, Vec3};
use net::{ShotFx, SHOT_ARROW, SHOT_BLAZE, SHOT_BOLT, SHOT_DRAGON, SHOT_GHAST, SHOT_SHULKER, SHOT_SKULL, SHOT_TRIDENT, SHOT_WIND};

use crate::item_render::GpuItem;
use crate::nether_end::ShotKind as NeKind;
use crate::weapons::ShotKind as WKind;
use crate::Game;

/// A shot from another machine: whose (player id; the host's world is the host's id) and
/// how long since we last heard of it.
pub struct RemoteShot {
    pub from: u32,
    pub kind: u8,
    pub pos: Vec3,
    pub vel: Vec3,
    pub age: f32,
}

/// A picture older than this (no news) is dropped: the shot ended or the sender went quiet.
const STALE: f32 = 0.35;

pub fn to_fx(kind: u8, pos: Vec3, vel: Vec3) -> ShotFx {
    ShotFx { kind, pos: pos.to_array(), vel: vel.to_array() }
}

impl Game {
    /// Our shots in flight, for the others. `world` adds what this machine runs for the
    /// world (mobs' arrows and fireballs, monsters' spells): the host, or a guest alone in a
    /// dimension.
    pub(crate) fn shots_for_net(&self, world: bool) -> Vec<ShotFx> {
        let mut out = Vec::new();
        for s in &self.prog.shots {
            if s.stuck {
                continue;
            }
            let kind = match s.kind {
                WKind::Arrow { .. } => SHOT_ARROW,
                WKind::Trident { .. } => SHOT_TRIDENT,
                WKind::WindCharge => SHOT_WIND,
            };
            out.push(to_fx(kind, s.pos, s.vel));
        }
        for (pos, vel, school) in self.magic.bolts_for_net(!world) {
            out.push(to_fx(SHOT_BOLT + school.min(200), pos, vel));
        }
        if world {
            for a in self.mobs.arrows.iter().filter(|a| !a.stuck) {
                out.push(to_fx(SHOT_ARROW, a.pos, a.vel));
            }
            for s in &self.ne.shots {
                let kind = match s.kind {
                    NeKind::Ghast => SHOT_GHAST,
                    NeKind::Blaze => SHOT_BLAZE,
                    NeKind::Skull => SHOT_SKULL,
                    NeKind::Shulker => SHOT_SHULKER,
                    NeKind::Dragon => SHOT_DRAGON,
                };
                out.push(to_fx(kind, s.pos, s.vel));
            }
        }
        out
    }

    /// Put in what one sender says is in flight now (replacing what it said before).
    pub(crate) fn take_remote_shots(&mut self, from: u32, list: &[ShotFx]) {
        self.net_shots.retain(|s| s.from != from);
        self.net_shots.extend(list.iter().map(|f| RemoteShot { from, kind: f.kind, pos: Vec3::from(f.pos), vel: Vec3::from(f.vel), age: 0.0 }));
    }

    /// Every frame: the others' shots fly on until the next picture, with their trails.
    pub(crate) fn update_remote_shots(&mut self, dt: f32) {
        if self.net_shots.is_empty() {
            return;
        }
        let glow = self.glow_layer();
        for s in self.net_shots.iter_mut() {
            s.age += dt;
            if matches!(s.kind, SHOT_ARROW | SHOT_TRIDENT) {
                s.vel.y -= crate::mobs::ARROW_GRAVITY * dt;
            }
            s.pos += s.vel * dt;
        }
        self.net_shots.retain(|s| s.age < STALE);
        let trails: Vec<(Vec3, Vec3, [f32; 3], f32)> = self
            .net_shots
            .iter()
            .filter_map(|s| {
                let (tint, size) = match s.kind {
                    SHOT_GHAST => ([1.4, 0.6, 0.15], 0.55),
                    SHOT_BLAZE => ([1.3, 0.75, 0.2], 0.25),
                    SHOT_SKULL => ([0.25, 0.22, 0.3], 0.4),
                    SHOT_SHULKER => ([1.2, 1.15, 0.9], 0.22),
                    SHOT_DRAGON => ([0.9, 0.3, 1.2], 0.45),
                    k if k >= SHOT_BOLT => (crate::magic::color(k - SHOT_BOLT), 0.1),
                    _ => return None,
                };
                Some((s.pos, s.vel, tint, size))
            })
            .collect();
        for (pos, vel, tint, size) in trails {
            self.particles.glow(pos, -vel * 0.05, glow, tint, size, 0.06);
        }
    }

    /// The others' arrows, tridents and wind charges as items in flight (like our own).
    pub(crate) fn remote_shot_instances(&self, cam_block: IVec3) -> Vec<GpuItem> {
        let sprite = |name: &str| {
            self.items.id(name).and_then(|i| match self.items.get(i).look {
                voxel::item::Look::Sprite(l) => Some((self.item_base + l) as f32),
                _ => None,
            })
        };
        let c = cam_block.as_vec3();
        let mut out = Vec::new();
        for s in &self.net_shots {
            let (layer, size) = match s.kind {
                SHOT_ARROW => (sprite("arrow"), 0.5),
                SHOT_TRIDENT => (sprite("trident"), 1.1),
                SHOT_WIND => (sprite("wind_charge"), 0.45),
                _ => continue,
            };
            let Some(layer) = layer else { continue };
            let l = self.world.light(s.pos.floor().as_ivec3());
            let (sky, blk) = ((l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
            let p = s.pos - c;
            if s.kind == SHOT_WIND {
                out.push(GpuItem { pos: [p.x, p.y, p.z, size], info: [s.age * 8.0, 0.0, sky, blk], layers: [layer, layer, layer, 0.0], rot: [0.0; 4], ..bytemuck::Zeroable::zeroed() });
                continue;
            }
            let d = s.vel.normalize_or_zero();
            let flat = (d.x * d.x + d.z * d.z).sqrt();
            let yaw = (-d.z).atan2(d.x);
            let roll = -std::f32::consts::FRAC_PI_4 + d.y.atan2(flat);
            for pitch in [0.0, std::f32::consts::FRAC_PI_2] {
                out.push(GpuItem { pos: [p.x, p.y, p.z, size], info: [yaw, 1.0, sky, blk], layers: [layer, layer, layer, 0.0], rot: [pitch, roll, 0.0, 0.0], ..bytemuck::Zeroable::zeroed() });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shots_round_trip_the_wire() {
        let list = vec![to_fx(SHOT_ARROW, Vec3::new(1.0, 2.0, 3.0), Vec3::X), to_fx(SHOT_BOLT + 3, Vec3::ZERO, Vec3::Y)];
        let m = net::ToGuest::Shots { dim: 0, list: list.clone() };
        let bytes = net::encode(&m);
        assert_eq!(net::decode::<net::ToGuest>(&bytes).unwrap(), m);
        let back = net::ToHost::Shots(list);
        assert_eq!(net::decode::<net::ToHost>(&net::encode(&back)).unwrap(), back);
    }
}
