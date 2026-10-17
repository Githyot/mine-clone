//! The Aether's ways (the Aether mod's ideas): eternal day until the Sun Spirit falls, clouds
//! that catch a fall or throw you up, parachutes, the altar that enchants with ambrosium, the
//! freezer, the incubator that hatches moas, and what its armour does: gravitite boots jump
//! higher, valkyrie armour glides. The dimension itself is in voxel's worldgen/aether.rs.

use glam::{IVec3, Vec3};
use voxel::item::ItemStack;
use voxel::worldgen::Dim;

use crate::audio::Sfx;
use crate::Game;

#[derive(Default)]
pub struct Aether {
    /// Seconds of parachute left.
    pub parachute: f32,
    was_ground: bool,
}

/// What the altar turns an item into (one ambrosium shard each).
const ALTAR: &[(&str, &str)] = &[("gravitite_ore", "enchanted_gravitite"), ("blue_berry", "enchanted_berry"), ("holystone", "healing_stone")];
/// What the freezer turns an item into, and what is left in the hand.
const FREEZER: &[(&str, &str, &str)] = &[("water_bucket", "ice", "bucket"), ("lava_bucket", "obsidian", "bucket"), ("cold_aercloud", "blue_aercloud", ""), ("blue_berry", "snowball", "")];

impl Game {
    pub(crate) fn update_aether(&mut self, dt: f32) {
        if self.dim == Dim::Aether && !self.boss.defeated.iter().any(|d| d == "sun_spirit") {
            // The Sun Spirit holds the sun still over its realm.
            self.time_of_day = 6000.0;
        }
        if dt <= 0.0 {
            return;
        }
        let a = &mut self.mods.aether;
        if a.parachute > 0.0 {
            a.parachute -= dt;
            if self.body.on_ground || self.body.in_water {
                a.parachute = 0.0;
            } else {
                self.body.vel.y = self.body.vel.y.max(-2.5);
                self.fall_peak = self.body.pos.y;
            }
        }
        let feet = self.body.pos - Vec3::Y * 0.1;
        let under = self.reg.get(self.world.block_or_air(feet.floor().as_ivec3())).name.as_str();
        let inside = self.reg.get(self.world.block_or_air(self.body.pos.floor().as_ivec3())).name.as_str();
        match (under, inside) {
            ("blue_aercloud", _) if self.body.on_ground && self.crouch <= 0.5 => {
                // A blue cloud throws you high.
                self.body.vel.y = 20.0;
                self.fall_peak = self.body.pos.y;
                self.sfx(Sfx::Step(crate::audio::Material::Cloth));
            }
            ("cold_aercloud", _) => self.fall_peak = self.body.pos.y,
            (_, "golden_aercloud") => {
                self.body.vel.y = self.body.vel.y.max(-1.5);
                self.fall_peak = self.body.pos.y;
            }
            _ => {}
        }
        let worn = |slot: usize, name: &str| self.surv.armor[slot].is_some_and(|s| self.items.get(s.item).name == name);
        let boots = worn(3, "gravitite_boots");
        let glide = worn(1, "valkyrie_chestplate");
        let ground = self.body.on_ground;
        if self.mods.aether.was_ground && !ground && self.body.vel.y > 4.0 && boots {
            // Gravitite boots: the jump goes half again as high.
            self.body.vel.y *= 1.45;
        }
        self.mods.aether.was_ground = ground;
        if glide && !ground && !self.body.in_water && self.body.vel.y < -5.0 {
            // Valkyrie wings: a slow glide down instead of a fall.
            self.body.vel.y = -5.0;
            self.fall_peak = self.body.pos.y + 2.0;
        }
    }

    /// Right click: portals, parachutes and the utility blocks. True when used.
    pub(crate) fn aether_use(&mut self) -> bool {
        let Some(st) = self.held_stack() else { return self.aether_block_use(None) };
        let item = self.items.get(st.item).name.clone();
        if self.light_mod_portal(&item) {
            return true;
        }
        if self.aether_block_use(Some(&item)) {
            return true;
        }
        if self.try_mount(&item) {
            return true;
        }
        if (item == "cloud_parachute" || item == "golden_parachute") && !self.body.on_ground {
            self.mods.aether.parachute = if item == "golden_parachute" { 30.0 } else { 12.0 };
            if item == "cloud_parachute" && self.survival {
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
            }
            self.toast("Paraquedas aberto", 1.5);
            return true;
        }
        false
    }

    fn aether_block_use(&mut self, item: Option<&str>) -> bool {
        let Some(hit) = self.target else { return false };
        if self.crouch > 0.5 {
            return false;
        }
        let block = self.reg.get(self.world.block_or_air(hit.block)).name.clone();
        let item = item.unwrap_or("");
        match block.as_str() {
            "altar" => {
                let Some(&(_, to)) = ALTAR.iter().find(|a| a.0 == item) else {
                    // Mend a worn tool or armour piece instead.
                    if let Some(st) = self.held_stack() {
                        if st.damage > 0 && self.pay_ambrosium() {
                            let sel = self.selected;
                            self.surv.inv.slots[sel] = Some(ItemStack { damage: 0, ..st });
                            self.altar_glow(hit.block);
                            return true;
                        }
                    }
                    self.toast("O altar encanta com ambrósio: gravitita, mirtilos, pedra sagrada ou consertos", 3.0);
                    return true;
                };
                if !self.pay_ambrosium() {
                    self.toast("Falta um fragmento de ambrósio", 2.0);
                    return true;
                }
                self.convert_held(to, "");
                self.altar_glow(hit.block);
                true
            }
            "freezer" => {
                let Some(&(_, to, left)) = FREEZER.iter().find(|f| f.0 == item) else {
                    self.toast("O congelador gela baldes, nuvens e mirtilos", 2.5);
                    return true;
                };
                self.convert_held(to, left);
                self.sfx(Sfx::Fizz);
                true
            }
            "incubator" => {
                if item != "moa_egg" {
                    self.toast("A incubadora choca ovos de moa", 2.5);
                    return true;
                }
                if self.survival {
                    let sel = self.selected;
                    self.surv.inv.take(sel, 1);
                }
                let at = (hit.block + IVec3::Y).as_vec3() + Vec3::new(0.5, 0.0, 0.5);
                self.mobs.spawn("moa", at);
                self.toast("Um moa saiu do ovo!", 3.0);
                true
            }
            _ => false,
        }
    }

    /// Take one ambrosium shard from the inventory (free in creative).
    fn pay_ambrosium(&mut self) -> bool {
        if !self.survival {
            return true;
        }
        let Some(id) = self.items.id("ambrosium_shard") else { return false };
        let Some(k) = self.surv.inv.slots.iter().position(|s| s.is_some_and(|s| s.item == id)) else { return false };
        self.surv.inv.take(k, 1);
        true
    }

    /// Turn one held item into `to`, leaving `left` in the hand ("" uses it up).
    fn convert_held(&mut self, to: &str, left: &str) {
        let Some(id) = self.items.id(to) else { return };
        if self.survival {
            if left.is_empty() {
                let sel = self.selected;
                self.surv.inv.take(sel, 1);
            } else {
                self.swap_held(left);
            }
        }
        if let Some(rest) = self.surv.inv.add(&self.items, ItemStack::new(id, 1)) {
            let _ = rest;
        }
        self.anim.start_swing();
    }

    fn altar_glow(&mut self, p: IVec3) {
        let glow = self.glow_layer();
        for k in 0..14 {
            let a = k as f32 * 0.45;
            let v = Vec3::new(a.cos() * 0.8, 1.6, a.sin() * 0.8);
            self.particles.glow(p.as_vec3() + Vec3::new(0.5, 1.1, 0.5), v, glow, [1.0, 0.9, 0.4], 0.12, 1.0);
        }
        self.sfx(Sfx::Cast(2));
    }
}
