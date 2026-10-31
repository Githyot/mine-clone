//! Aquaculture idea: the modular fishing rod. A hook, a line and a bait are fitted to it (hold
//! the part and right click); each part changes the fishing:
//!
//! * hooks: iron (plain), gold (more treasure), heavy (bigger fish, less junk), double (two
//!   fish at once now and then);
//! * lines: plain, or reinforced (the fish stays on longer, and the rod wears less);
//! * baits: worms and fish bait, one used up per catch, bring the bite much sooner.
//!
//! The rig belongs to the player (saved with the world's mod state), not to each rod item:
//! item stacks carry no extra data here. The classic rod keeps working as before.

use serde::{Deserialize, Serialize};
use voxel::item::ItemStack;

use crate::Game;

pub const ROD: &str = "modular_fishing_rod";
const HOOKS: [&str; 4] = ["iron_hook", "gold_hook", "heavy_hook", "double_hook"];
const LINES: [&str; 2] = ["fishing_line", "strong_line"];
const BAITS: [&str; 2] = ["worm", "fish_bait"];

/// The parts fitted to the player's modular rod.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Rig {
    pub hook: Option<String>,
    pub line: Option<String>,
    /// Bait and how many are left on the rod.
    pub bait: Option<(String, u8)>,
}

/// More fish by biome: (biomes, item, weight).
pub const MORE_FISH: &[(&[&str], &str, u32)] = &[
    (&["Plains", "Forest", "Birch Forest", "Savanna", "Cherry Grove", "Dark Forest"], "bass", 16),
    (&["Plains", "Forest", "Swamp", "Birch Forest"], "carp", 14),
    (&["Plains", "Forest", "Cherry Grove"], "bluegill", 14),
    (&["Snowy Plains", "Taiga", "Ocean"], "halibut", 12),
    (&["Snowy Plains", "Taiga", "Ocean", "Beach"], "pollock", 14),
    (&["Ocean", "Beach"], "grouper", 12),
    (&["Swamp", "Jungle"], "gar", 14),
];
/// Extra junk the modular rod drags up.
pub const MORE_JUNK: &[(&str, u32)] = &[("driftwood", 3), ("tin_can", 2)];

pub fn is_rod(name: &str) -> bool {
    name == "fishing_rod" || name == ROD || crate::tr_fishing::rod_power(name).is_some()
}

impl Game {
    fn modular_held(&self) -> bool {
        self.held_stack().is_some_and(|s| self.items.get(s.item).name == ROD)
    }

    fn rig(&self) -> Option<&Rig> {
        self.modular_held().then_some(&self.mods.save.rig)
    }

    /// Right click with a rod part: fit it to the modular rod, giving back the one it replaces.
    pub(crate) fn aquaculture_use(&mut self) -> bool {
        let Some(st) = self.held_stack() else { return false };
        let name = self.items.get(st.item).name.clone();
        let slot = if HOOKS.contains(&name.as_str()) {
            0
        } else if LINES.contains(&name.as_str()) {
            1
        } else if BAITS.contains(&name.as_str()) {
            2
        } else {
            return false;
        };
        let has_rod = self.surv.inv.slots.iter().flatten().any(|s| self.items.get(s.item).name == ROD) || !self.survival;
        if !has_rod {
            self.toast("Você precisa de uma vara de pesca modular na mochila", 2.5);
            return true;
        }
        let sel = self.selected;
        let rig = &mut self.mods.save.rig;
        let old: Option<(String, u8)> = match slot {
            0 => rig.hook.replace(name.clone()).map(|n| (n, 1)),
            1 => rig.line.replace(name.clone()).map(|n| (n, 1)),
            _ => {
                // Baits pile up on the rod (up to 16) when they are the same kind.
                let n = st.count.min(16) as u8;
                match rig.bait.take() {
                    Some((b, k)) if b == name => {
                        let total = (k + n).min(16);
                        let used = total - k;
                        rig.bait = Some((b, total));
                        if self.survival {
                            self.surv.inv.take(sel, u16::from(used));
                        }
                        self.toast(format!("Isca na vara: {total}"), 2.0);
                        return true;
                    }
                    prev => {
                        rig.bait = Some((name.clone(), n));
                        if self.survival {
                            self.surv.inv.take(sel, u16::from(n));
                        }
                        if let Some((b, k)) = prev {
                            self.give_back(&b, k);
                        }
                        self.toast(format!("Isca na vara: {n}"), 2.0);
                        return true;
                    }
                }
            }
        };
        if self.survival {
            self.surv.inv.take(sel, 1);
        }
        if let Some((n, k)) = old {
            self.give_back(&n, k);
        }
        let shown = self.items.id(&name).map(|i| self.items.get(i).display_name.clone()).unwrap_or_default();
        self.toast(format!("{shown} colocado na vara modular"), 2.0);
        self.sfx(crate::audio::Sfx::Place(crate::audio::Material::Metal));
        true
    }

    fn give_back(&mut self, name: &str, n: u8) {
        if !self.survival || n == 0 {
            return;
        }
        if let Some(id) = self.items.id(name) {
            if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(id, u16::from(n))) {
                let at = self.body.pos + glam::Vec3::Y;
                self.pop_drop(at, left);
            }
        }
    }

    /// How much longer than usual the wait for a bite is (bait shortens it).
    pub(crate) fn rod_bite_factor(&self) -> f32 {
        let Some(r) = self.rig() else { return 1.0 };
        match r.bait.as_ref().map(|b| b.0.as_str()) {
            Some("worm") => 0.55,
            Some("fish_bait") => 0.4,
            _ => 0.9,
        }
    }

    /// How long a biting fish stays on the hook, in seconds.
    pub(crate) fn rod_bite_window(&self) -> f32 {
        match self.rig().and_then(|r| r.line.as_deref()) {
            Some("strong_line") => 1.8,
            Some("fishing_line") => 1.3,
            _ => 0.9,
        }
    }

    /// Chances of a fish and of junk (the rest is treasure).
    pub(crate) fn rod_odds(&self) -> (f32, f32) {
        match self.rig().and_then(|r| r.hook.as_deref()) {
            Some("gold_hook") => (0.75, 0.08),
            Some("heavy_hook") => (0.9, 0.04),
            Some("iron_hook") | Some("double_hook") => (0.87, 0.08),
            _ if self.modular_held() => (0.85, 0.1),
            _ => (0.85, 0.1),
        }
    }

    /// Fish weights for a catch: the heavy hook favours the big ones.
    pub(crate) fn rod_fish_weight(&self, name: &str, w: u32) -> u32 {
        let big = matches!(name, "salmon" | "tuna" | "pirarucu" | "catfish" | "halibut" | "grouper" | "gar");
        match self.rig().and_then(|r| r.hook.as_deref()) {
            Some("heavy_hook") if big => w * 3,
            _ => w,
        }
    }

    /// After a catch: the bait is used up, and a double hook may bring a second fish.
    pub(crate) fn rod_caught(&mut self, fish: Option<&'static str>, at: glam::Vec3) {
        if !self.modular_held() {
            return;
        }
        if let Some((_, k)) = self.mods.save.rig.bait.as_mut() {
            *k = k.saturating_sub(1);
            if *k == 0 {
                self.mods.save.rig.bait = None;
                self.toast("A isca acabou", 2.0);
            }
        }
        if let Some(f) = fish {
            if self.mods.save.rig.hook.as_deref() == Some("double_hook") && self.surv.rng.chance(0.35) {
                if let Some(id) = self.items.id(f) {
                    let to = self.body.eye() - at;
                    let vel = glam::Vec3::new(to.x, to.y + to.length().sqrt() * 1.6, to.z);
                    self.spawn_drop(at + glam::Vec3::Y * 0.2, ItemStack::new(id, 1), vel);
                }
            }
        }
    }

    /// Wear on the rod per reel: the reinforced line spares it.
    pub(crate) fn rod_wear(&self, n: u16) -> u16 {
        if self.rig().and_then(|r| r.line.as_deref()) == Some("strong_line") {
            n.saturating_sub(1)
        } else {
            n
        }
    }

    /// The rig, under the hotbar while the modular rod is in hand.
    pub(crate) fn draw_rod_hud(&mut self, w: f32, h: f32) {
        let Some(r) = self.rig() else { return };
        let name = |n: &Option<String>| n.as_ref().and_then(|n| self.items.id(n)).map(|i| self.items.get(i).display_name.clone()).unwrap_or_else(|| "—".into());
        let bait = match &r.bait {
            Some((b, k)) => format!("{} ×{k}", self.items.id(b).map(|i| self.items.get(i).display_name.clone()).unwrap_or_default()),
            None => "—".into(),
        };
        let text = format!("Anzol: {}   Linha: {}   Isca: {}", name(&r.hook), name(&r.line), bait);
        let s = self.ui.scale;
        let tw = self.ui.text_width(&text, s);
        let y = h - 64.0 * s;
        self.ui.rect((w - tw) / 2.0 - 3.0 * s, y - 2.0 * s, tw + 6.0 * s, 12.0 * s, [0, 0, 0, 120]);
        self.ui.text((w - tw) / 2.0, y, &text, [210, 230, 255, 255]);
    }
}
