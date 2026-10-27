//! Backpacks (ideas from Sophisticated Backpacks): an item that carries its own storage, in
//! four sizes. Each backpack gets a number the first time it is opened, kept in the stack's
//! `damage` field, and its contents live with the other containers under a position no block
//! can have, so they are saved with them.

use glam::{IVec3, Vec3};
use voxel::item::{Inventory, ItemStack};

use crate::survival::{Container, Open};
use crate::Game;

/// Containers at this x are backpacks; z is the backpack's number.
pub const BACKPACK_X: i32 = i32::MIN + 7;

/// How many slots a backpack has.
pub fn slots(name: &str) -> Option<usize> {
    match name {
        "leather_backpack" => Some(27),
        "iron_backpack" => Some(36),
        "gold_backpack" => Some(45),
        "diamond_backpack" => Some(54),
        _ => None,
    }
}

pub fn is_backpack(p: IVec3) -> bool {
    p.x == BACKPACK_X
}

impl Game {
    /// Right click with a backpack in hand: open it.
    pub(crate) fn open_backpack(&mut self) -> bool {
        let sel = self.selected;
        let Some(st) = self.surv.inv.slots[sel] else { return false };
        let Some(n) = slots(&self.items.get(st.item).name) else { return false };
        let id = if st.damage == 0 {
            let next = self.surv.containers.keys().filter(|p| is_backpack(**p)).map(|p| p.z).max().unwrap_or(0) + 1;
            self.surv.inv.slots[sel] = Some(ItemStack { damage: next as u16, ..st });
            next
        } else {
            st.damage as i32
        };
        let p = IVec3::new(BACKPACK_X, 0, id);
        let c = self.surv.containers.entry(p).or_insert_with(|| Container::Chest(Inventory::new(n)));
        // An upgraded backpack keeps what it held and gains the new rows.
        if let Container::Chest(inv) = c {
            if inv.slots.len() < n {
                inv.slots.resize(n, None);
            }
        }
        self.surv.open = Open::Chest(p);
        self.set_screen(crate::Screen::Inventory);
        self.sfx(crate::audio::Sfx::Place(crate::audio::Material::Cloth));
        true
    }

    /// A backpack can't hold backpacks: any put in come back out when it closes.
    pub(crate) fn backpack_closed(&mut self, p: IVec3) {
        let Some(Container::Chest(inv)) = self.surv.containers.get_mut(&p) else { return };
        let mut out = Vec::new();
        for s in inv.slots.iter_mut() {
            if s.is_some_and(|st| slots(&self.items.get(st.item).name).is_some()) {
                out.push(s.take().unwrap());
            }
        }
        for st in out {
            if let Some(left) = self.surv.inv.add(&self.items, st) {
                let f = self.forward();
                self.spawn_drop(self.body.eye() - Vec3::Y * 0.3, left, f * 3.0);
            }
        }
    }
}
