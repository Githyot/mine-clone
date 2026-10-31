//! Multijogador: items on the ground that everyone sees and anyone can pick up (lote 18a).
//!
//! Each machine keeps running its own drops (what it broke, killed or threw); the others get
//! a picture of them (`DropFx`) four times a second. Walking over someone else's item asks
//! its owner for it (`Take`); the owner takes it off the ground and hands it over (`Give`),
//! so an item is never in two hands. The host routes between guests.

use std::collections::{HashMap, HashSet};

use glam::Vec3;
use net::{DropFx, ToGuest, ToHost};
use voxel::item::ItemStack;

use crate::Game;

/// Host's own player id (as in multiplayer.rs).
const HOST: u32 = 0;
/// How often the pictures go out.
const EVERY: f32 = 0.25;
/// Drops farther than this from a player are not sent to them.
const RANGE: f32 = 48.0;
/// Most drops in one picture.
const MOST: usize = 200;

/// Someone else's item on the ground.
pub struct RemoteDrop {
    pub owner: u32,
    pub id: u32,
    pub stack: ItemStack,
    pub pos: Vec3,
    pub ready: bool,
    /// We asked for it: hidden until the owner's next picture.
    pub asked: bool,
}

#[derive(Default)]
pub struct NetDrops {
    t: f32,
    /// What we last said: there were drops in it (so the empty one that clears them goes too).
    out: bool,
    /// Host: guests whose last picture had drops in it.
    out_to: HashSet<u32>,
    /// Host: each guest's own drops as it last sent them.
    from_guests: HashMap<u32, Vec<DropFx>>,
    /// Test hook MC_NET_DROP: seconds the first guest has been in (the host then drops a few
    /// items at its feet and calls a goblin invasion and a blood moon, once).
    test_t: Option<f32>,
}

fn stack_of(bytes: &[u8]) -> Option<ItemStack> {
    net::decode::<ItemStack>(bytes)
}

impl Game {
    fn own_drops_fx(&self, owner: u32, near: Option<Vec3>) -> Vec<DropFx> {
        let mut v: Vec<&crate::survival::Dropped> = self.surv.drops.iter().filter(|d| near.is_none_or(|p| (d.body.pos - p).length() < RANGE)).collect();
        if let Some(p) = near {
            v.sort_by(|a, b| (a.body.pos - p).length_squared().total_cmp(&(b.body.pos - p).length_squared()));
        }
        v.into_iter()
            .take(MOST)
            .map(|d| DropFx { owner, id: d.net_id, stack: net::encode(&d.stack), pos: d.body.pos.to_array(), ready: d.age > crate::survival::PICKUP_DELAY })
            .collect()
    }

    fn replace_remote_drops(&mut self, owner: Option<u32>, list: &[DropFx]) {
        match owner {
            Some(o) => self.net_drops.retain(|d| d.owner != o),
            None => self.net_drops.clear(),
        }
        for f in list {
            let Some(stack) = stack_of(&f.stack) else { continue };
            if (stack.item as usize) >= self.items.items.len() {
                continue;
            }
            self.net_drops.push(RemoteDrop { owner: f.owner, id: f.id, stack, pos: Vec3::from(f.pos), ready: f.ready, asked: false });
        }
    }

    /// Every frame: send our pictures now and then, and ask for what we walk over.
    pub(crate) fn update_net_drops(&mut self, dt: f32) {
        if !self.net.is_host() && !self.net.is_guest() {
            self.net_drops.clear();
            self.net_drop_state = NetDrops::default();
            return;
        }
        self.net_drop_state.t += dt;
        if self.net_drop_state.t >= EVERY {
            self.net_drop_state.t = 0.0;
            if self.net.is_host() {
                self.net_drop_test(EVERY);
                self.host_send_drops();
            } else {
                let me = self.net_my_id();
                let list = self.own_drops_fx(me, Some(self.body.pos));
                if !list.is_empty() || self.net_drop_state.out {
                    self.net_drop_state.out = !list.is_empty();
                    self.net_to_host(&ToHost::Drops(list));
                }
            }
        }
        // Walking over someone else's item: ask for it.
        if !self.survival || self.screen == crate::Screen::Dead {
            return;
        }
        let me = self.body.pos + Vec3::Y * 0.9;
        let want: Vec<(u32, u32)> = self.net_drops.iter_mut().filter(|d| d.ready && !d.asked && (d.pos - me).length() < 1.7).map(|d| {
            d.asked = true;
            (d.owner, d.id)
        }).collect();
        for (owner, id) in want {
            log::debug!("asking player {owner} for item {id}");
            if self.net.is_host() {
                self.net_to_guest(owner, &ToGuest::Take { by: HOST, id });
            } else {
                self.net_to_host(&ToHost::Take { owner, id });
            }
        }
    }

    fn net_drop_test(&mut self, dt: f32) {
        static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if !*ON.get_or_init(|| std::env::var("MC_NET_DROP").is_ok()) {
            return;
        }
        let Some(&(_, _, at)) = self.net_guest_spots().first() else { return };
        let t = self.net_drop_state.test_t.get_or_insert(0.0);
        if *t < 0.0 {
            return;
        }
        *t += dt;
        if *t < 15.0 {
            return;
        }
        self.net_drop_state.test_t = Some(-1.0);
        for (k, name) in ["diamond", "apple", "iron_ingot"].iter().enumerate() {
            if let Some(id) = self.items.id(name) {
                let p = at + Vec3::new(k as f32 * 0.6 - 0.6, 0.6, 1.2);
                self.spawn_drop(p, ItemStack::new(id, u16::from(3 + k as u8)), Vec3::ZERO);
            }
        }
        // Screenshots stop the drops' clocks: these are ready at once.
        let n = self.surv.drops.len();
        for d in &mut self.surv.drops[n.saturating_sub(3)..] {
            d.age = 1.0;
        }
        self.start_invasion();
        self.t5.hard.blood = true;
        log::info!("MC_NET_DROP: 3 drops at the guest's feet, goblin invasion, blood moon");
    }

    fn host_send_drops(&mut self) {
        let my_dim = self.dim.as_u8();
        for (gid, dim, at) in self.net_guest_spots() {
            let mut list = if dim == my_dim { self.own_drops_fx(HOST, Some(at)) } else { Vec::new() };
            for (other, theirs) in &self.net_drop_state.from_guests {
                if *other == gid || self.net_guest_spots().iter().find(|g| g.0 == *other).is_none_or(|g| g.1 != dim) {
                    continue;
                }
                list.extend(theirs.iter().filter(|d| (Vec3::from(d.pos) - at).length() < RANGE).cloned());
            }
            list.truncate(MOST * 2);
            let was = self.net_drop_state.out_to.contains(&gid);
            if list.is_empty() && !was {
                continue;
            }
            if list.is_empty() {
                self.net_drop_state.out_to.remove(&gid);
            } else {
                self.net_drop_state.out_to.insert(gid);
            }
            self.net_to_guest(gid, &ToGuest::Drops { dim, list });
        }
    }

    /// Host: a guest's picture of its drops.
    pub(crate) fn host_got_drops(&mut self, from: u32, list: Vec<DropFx>) {
        let list: Vec<DropFx> = list.into_iter().take(MOST).map(|d| DropFx { owner: from, ..d }).collect();
        let here = self.net_guest_spots().iter().find(|g| g.0 == from).is_some_and(|g| g.1 == self.dim.as_u8());
        self.replace_remote_drops(Some(from), if here { &list } else { &[] });
        self.net_drop_state.from_guests.insert(from, list);
    }

    /// Host: a guest left; its drops go with it.
    pub(crate) fn host_guest_drops_gone(&mut self, id: u32) {
        self.net_drop_state.from_guests.remove(&id);
        self.net_drop_state.out_to.remove(&id);
        self.net_drops.retain(|d| d.owner != id);
    }

    /// Guest: the host's picture of the drops around us.
    pub(crate) fn guest_got_drops(&mut self, dim: u8, list: Vec<DropFx>) {
        let me = self.net_my_id();
        let list: Vec<DropFx> = if dim == self.dim.as_u8() { list.into_iter().filter(|d| d.owner != me).collect() } else { Vec::new() };
        self.replace_remote_drops(None, &list);
    }

    /// Host: guest `by` asks for the item `id` of `owner`.
    pub(crate) fn host_got_take(&mut self, by: u32, owner: u32, id: u32) {
        if owner == HOST {
            self.hand_over(by, id);
        } else {
            self.net_to_guest(owner, &ToGuest::Take { by, id });
        }
    }

    /// Host: a guest hands an item to player `to`.
    pub(crate) fn host_got_give(&mut self, to: u32, stack: Vec<u8>) {
        if to == HOST {
            self.receive_net_stack(&stack);
        } else {
            self.net_to_guest(to, &ToGuest::Give { stack });
        }
    }

    /// Our item `id` goes to player `by`, if it is still on the ground.
    pub(crate) fn hand_over(&mut self, by: u32, id: u32) {
        let Some(i) = self.surv.drops.iter().position(|d| d.net_id == id) else {
            log::debug!("player {by} asked for item {id}: gone already");
            return;
        };
        log::debug!("player {by} takes item {id}");
        let stack = net::encode(&self.surv.drops.swap_remove(i).stack);
        if self.net.is_host() {
            self.net_to_guest(by, &ToGuest::Give { stack });
        } else {
            self.net_to_host(&ToHost::Give { to: by, stack });
        }
    }

    /// Someone's item is ours: into the wallet, the inventory, the void bag, or back on the
    /// ground at our feet when there is no room.
    pub(crate) fn receive_net_stack(&mut self, bytes: &[u8]) {
        let Some(st) = stack_of(bytes) else { return };
        if (st.item as usize) >= self.items.items.len() || st.count == 0 {
            return;
        }
        log::info!("got {} x{} from another player", self.items.get(st.item).name, st.count);
        self.sfx(crate::audio::Sfx::Pickup);
        if self.pocket_coins(&st) {
            return;
        }
        let Some(left) = self.surv.inv.add(&self.items, st) else { return };
        if let Some(left) = self.void_stash(left) {
            let at = self.body.pos + Vec3::Y * 0.5;
            self.spawn_drop(at, left, Vec3::ZERO);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn stacks_cross_the_wire_whole() {
        let mut st = voxel::item::ItemStack::new(7, 12);
        st.damage = 40;
        st.sockets = 0x1234;
        let b = net::encode(&st);
        assert_eq!(super::stack_of(&b), Some(st));
        let m = net::ToHost::Give { to: 3, stack: b };
        assert_eq!(net::decode::<net::ToHost>(&net::encode(&m)), Some(m));
    }
}
