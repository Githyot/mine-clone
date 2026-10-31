//! Multijogador: things with a 3D body that are not mobs (lote 18a). Minions and sentries,
//! boats and minecarts, and the travelling wagons are run where they live (each player's own;
//! the host's world for the wagons); the others get their posed parts (`PropFx`) about 20
//! times a second and draw them as they are. Parked wagons come from the world's seed on
//! every machine, so they are not sent.

use std::collections::HashMap;

use glam::{IVec3, Vec3};
use net::{PropFx, ToGuest, ToHost};

use crate::entity::{Bones, MAX_PARTS};
use crate::Game;

const EVERY: f32 = 0.05;
/// Props farther than this from a player are not sent to them.
const RANGE: f32 = 80.0;
const MOST: usize = 96;
/// No news for this long: the sender went quiet, the picture goes.
const STALE: f32 = 0.5;

#[derive(Default)]
pub struct NetProps {
    t: f32,
    out: bool,
    out_to: std::collections::HashSet<u32>,
    /// Host: each guest's own props as it last sent them.
    from_guests: HashMap<u32, Vec<PropFx>>,
    /// What the others look like now: (sender, picture), and how old our news is per sender.
    shown: HashMap<u32, (Vec<PropFx>, f32)>,
}

fn to_fx(def: usize, b: &Bones, parts: usize) -> PropFx {
    let n = parts.clamp(1, MAX_PARTS);
    let parts = b.m[..n]
        .iter()
        .map(|c| [c[0][0], c[0][1], c[0][2], c[1][0], c[1][1], c[1][2], c[2][0], c[2][1], c[2][2], c[3][0], c[3][1], c[3][2]])
        .collect();
    let mut bend: Vec<[f32; 3]> = b.bend[..n].iter().map(|v| [v[0], v[1], v[2]]).collect();
    while bend.last().is_some_and(|v| *v == [0.0; 3]) {
        bend.pop();
    }
    PropFx { def: def as u16, parts, bend, light: b.light, tint: b.tint }
}

fn bones_of(f: &PropFx, cam: Vec3) -> Bones {
    let mut b = Bones { light: f.light, tint: f.tint, ..Default::default() };
    for (k, p) in f.parts.iter().take(MAX_PARTS).enumerate() {
        b.m[k] = [[p[0], p[1], p[2], 0.0], [p[3], p[4], p[5], 0.0], [p[6], p[7], p[8], 0.0], [p[9] - cam.x, p[10] - cam.y, p[11] - cam.z, 1.0]];
    }
    for (k, v) in f.bend.iter().take(MAX_PARTS).enumerate() {
        b.bend[k] = [v[0], v[1], v[2], 0.0];
    }
    b
}

fn at(f: &PropFx) -> Vec3 {
    f.parts.first().map(|p| Vec3::new(p[9], p[10], p[11])).unwrap_or(Vec3::ZERO)
}

impl Game {
    /// Our things with a body, in world space. `world` adds what this machine runs for the
    /// world (the travelling wagons).
    fn props_here(&self, world: bool) -> Vec<PropFx> {
        let z = IVec3::ZERO;
        let mut all = Vec::new();
        all.extend(self.minion_instances(z));
        all.extend(self.sentry_instances(z));
        all.extend(self.tr_sentry_models(z));
        all.extend(self.boat_instances(z));
        all.extend(self.cart_instances(z));
        if world {
            all.extend(self.wagon_instances_moving(z));
        }
        let reg = &self.mobs.reg;
        all.iter().filter(|(d, _)| *d < reg.defs.len()).map(|(d, b)| to_fx(*d, b, reg.defs[*d].parts.len())).collect()
    }

    fn near(list: &[PropFx], p: Vec3) -> impl Iterator<Item = &PropFx> + '_ {
        list.iter().filter(move |f| (at(f) - p).length() < RANGE)
    }

    fn show_props(&mut self, from: u32, list: Vec<PropFx>) {
        let n = self.mobs.reg.defs.len();
        let list: Vec<PropFx> = list.into_iter().filter(|f| (f.def as usize) < n).take(MOST).collect();
        if list.is_empty() {
            self.net_props.shown.remove(&from);
        } else {
            self.net_props.shown.insert(from, (list, 0.0));
        }
    }

    /// Every frame: send our pictures, age the others'.
    pub(crate) fn update_net_props(&mut self, dt: f32) {
        if !self.net.is_host() && !self.net.is_guest() {
            self.net_props = NetProps::default();
            return;
        }
        for (_, (_, age)) in self.net_props.shown.iter_mut() {
            *age += dt;
        }
        self.net_props.shown.retain(|_, (_, age)| *age < STALE);
        self.net_props.t += dt;
        if self.net_props.t < EVERY {
            return;
        }
        self.net_props.t = 0.0;
        if self.net.is_guest() {
            let me = self.body.pos;
            let list: Vec<PropFx> = Self::near(&self.props_here(self.net_simulates()), me).take(MOST).cloned().collect();
            if !list.is_empty() || self.net_props.out {
                self.net_props.out = !list.is_empty();
                self.net_to_host(&ToHost::Props(list));
            }
            return;
        }
        let my_dim = self.dim.as_u8();
        let ours = self.props_here(true);
        let spots = self.net_guest_spots();
        for &(gid, dim, p) in &spots {
            let mut list: Vec<PropFx> = if dim == my_dim { Self::near(&ours, p).cloned().collect() } else { Vec::new() };
            for (other, theirs) in &self.net_props.from_guests {
                if *other != gid && spots.iter().any(|s| s.0 == *other && s.1 == dim) {
                    list.extend(Self::near(theirs, p).cloned());
                }
            }
            list.truncate(MOST * 2);
            let was = self.net_props.out_to.contains(&gid);
            if list.is_empty() && !was {
                continue;
            }
            if list.is_empty() {
                self.net_props.out_to.remove(&gid);
            } else {
                self.net_props.out_to.insert(gid);
            }
            self.net_to_guest(gid, &ToGuest::Props { dim, list });
        }
    }

    /// Host: a guest's picture of its things.
    pub(crate) fn host_got_props(&mut self, from: u32, list: Vec<PropFx>) {
        let list: Vec<PropFx> = list.into_iter().take(MOST).collect();
        let here = self.net_guest_spots().iter().any(|g| g.0 == from && g.1 == self.dim.as_u8());
        self.show_props(from, if here { list.clone() } else { Vec::new() });
        self.net_props.from_guests.insert(from, list);
    }

    pub(crate) fn host_guest_props_gone(&mut self, id: u32) {
        self.net_props.from_guests.remove(&id);
        self.net_props.out_to.remove(&id);
        self.net_props.shown.remove(&id);
    }

    /// Guest: the host's picture of the things near us.
    pub(crate) fn guest_got_props(&mut self, dim: u8, list: Vec<PropFx>) {
        let list = if dim == self.dim.as_u8() { list } else { Vec::new() };
        self.show_props(0, list);
    }

    /// The others' things, for drawing with ours.
    pub(crate) fn remote_prop_instances(&self, cam_block: IVec3) -> Vec<(usize, Bones)> {
        let c = cam_block.as_vec3();
        self.net_props.shown.values().flat_map(|(l, _)| l.iter()).map(|f| (f.def as usize, bones_of(f, c))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, Quat};

    #[test]
    fn a_posed_body_crosses_the_wire() {
        let mut b = Bones::default();
        let m = Mat4::from_scale_rotation_translation(Vec3::new(0.5, 1.0, 2.0), Quat::from_rotation_y(0.7), Vec3::new(100.0, 64.0, -30.0));
        b.m[0] = m.to_cols_array_2d();
        b.bend[1] = [3.0, 1.0, 0.4, 0.0];
        b.light = [1.0, 0.5, 0.0, 0.0];
        let f = to_fx(9, &b, 3);
        assert_eq!(f.parts.len(), 3);
        assert_eq!(f.bend.len(), 2);
        let wire = net::encode(&net::ToGuest::Props { dim: 0, list: vec![f.clone()] });
        let Some(net::ToGuest::Props { list, .. }) = net::decode::<net::ToGuest>(&wire) else { panic!() };
        let back = bones_of(&list[0], Vec3::new(100.0, 64.0, -30.0));
        let want = (Mat4::from_translation(Vec3::new(-100.0, -64.0, 30.0)) * m).to_cols_array_2d();
        for (a, w) in back.m[0].iter().flatten().zip(want.iter().flatten()) {
            assert!((a - w).abs() < 1e-4);
        }
        assert_eq!(back.bend[1], b.bend[1]);
        assert_eq!(at(&f), Vec3::new(100.0, 64.0, -30.0));
    }
}
