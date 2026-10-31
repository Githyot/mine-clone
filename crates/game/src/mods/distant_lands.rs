//! Distant Lands' carpenter table (the mod's idea): one station that cuts any wood into its
//! shapes without crafting patterns. Right click with an empty hand picks what it makes
//! (planks, slabs, stairs, fences or hollow logs); right click holding a log or planks cuts
//! one into that, more of it than the crafting grid gives. The wood of a log is found from its
//! planks recipe, so every wood of the game and of the packs works. The biomes and trees are
//! in voxel's worldgen/distant_lands.rs, sapling growth in mods/saplings.rs.

use voxel::item::ItemStack;

use crate::audio::{Material, Sfx};
use crate::Game;

/// What the table makes: (name, suffix of the result, how many from a log, from one plank).
const MODES: [(&str, &str, u8, u8); 5] = [
    ("Tábuas", "_planks", 6, 0),
    ("Lajes", "_slab", 12, 2),
    ("Escadas", "_stairs", 6, 1),
    ("Cercas", "_fence", 4, 1),
    ("Troncos Ocos", "hollow", 1, 0),
];

impl Game {
    /// The planks a log is sawn into (from the shapeless planks recipes).
    fn planks_of(&self, item: voxel::item::ItemId) -> Option<String> {
        self.recipes.shapeless.iter().find_map(|r| {
            let name = &self.items.get(r.result.item).name;
            (r.ingredients.len() == 1 && r.ingredients[0].contains(&item) && name.ends_with("_planks")).then(|| name.clone())
        })
    }

    /// Right click on a carpenter table. True when the click was used.
    pub(crate) fn carpenter_use(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        if self.crouch > 0.5 || self.reg.get(self.world.block_or_air(hit.block)).name != "carpenter_table" {
            return false;
        }
        let mode = self.mods.carpenter as usize % MODES.len();
        let Some(held) = self.held_stack() else {
            let next = (mode + 1) % MODES.len();
            self.mods.carpenter = next as u8;
            self.toast(&format!("Mesa de Carpinteiro: {}", MODES[next].0), 2.0);
            self.sfx(Sfx::Click);
            return true;
        };
        let name = self.items.get(held.item).name.clone();
        let (wood, from_log) = if let Some(w) = name.strip_suffix("_planks") {
            (w.to_string(), false)
        } else if let Some(p) = self.planks_of(held.item) {
            (p.trim_end_matches("_planks").to_string(), true)
        } else {
            self.toast(&format!("Segure troncos ou tábuas ({})", MODES[mode].0), 2.5);
            return true;
        };
        let (label, suffix, per_log, per_plank) = MODES[mode];
        let out = match suffix {
            "hollow" => name.strip_suffix("_log").map(|w| format!("hollow_{w}_log")),
            _ => Some(format!("{wood}{suffix}")),
        };
        let count = if from_log { per_log } else { per_plank };
        let Some(out) = out.filter(|_| count > 0).and_then(|o| self.items.id(&o)) else {
            self.toast(&format!("Não dá para fazer {} com isso", label.to_lowercase()), 2.5);
            return true;
        };
        if self.survival {
            let sel = self.selected;
            self.surv.inv.take(sel, 1);
        }
        if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(out, u16::from(count))) {
            self.throw_stack(left);
        }
        self.anim.start_swing();
        self.sfx(Sfx::Hit(Material::Wood));
        true
    }
}
