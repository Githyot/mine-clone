//! Smithing (Phase 6): the smithing table turns diamond gear into netherite with an upgrade
//! template, and puts trims on armor — a pattern from a template, in the colour of a material.
//! The trim is kept on the stack with the enchantments (`enchanting::TRIM`) and drawn as a
//! coloured band over the worn armor.

use voxel::item::{ItemRegistry, ItemStack};

use crate::enchanting::TRIM;

/// Trim patterns, in the order their ids go (1-based in the stack).
pub const PATTERNS: [(&str, &str); 18] = [
    ("sentry", "Sentinela"),
    ("dune", "Duna"),
    ("coast", "Costa"),
    ("wild", "Selvagem"),
    ("ward", "Guarda"),
    ("eye", "Olho"),
    ("vex", "Vex"),
    ("tide", "Maré"),
    ("snout", "Focinho"),
    ("rib", "Costela"),
    ("spire", "Agulha"),
    ("wayfinder", "Desbravador"),
    ("shaper", "Modelador"),
    ("silence", "Silêncio"),
    ("raiser", "Criador"),
    ("host", "Anfitrião"),
    ("flow", "Fluxo"),
    ("bolt", "Raio"),
];

/// Trim materials: item, name, colour.
pub const MATERIALS: [(&str, &str, [u8; 3]); 11] = [
    ("iron_ingot", "Ferro", [236, 236, 236]),
    ("copper_ingot", "Cobre", [180, 104, 77]),
    ("gold_ingot", "Ouro", [222, 177, 45]),
    ("lapis_lazuli", "Lápis-lazúli", [65, 103, 172]),
    ("emerald", "Esmeralda", [17, 160, 54]),
    ("diamond", "Diamante", [110, 236, 210]),
    ("netherite_ingot", "Netherita", [98, 89, 89]),
    ("redstone", "Redstone", [151, 22, 7]),
    ("amethyst_shard", "Ametista", [154, 92, 198]),
    ("quartz", "Quartzo", [227, 212, 196]),
    ("resin_brick", "Resina", [239, 120, 30]),
];

pub fn template_item(pattern: &str) -> String {
    format!("{pattern}_armor_trim_smithing_template")
}

pub const UPGRADE: &str = "netherite_upgrade_smithing_template";

/// Tooltip line for a trim pair (pattern id in the pair's id slot is TRIM; its level packs
/// pattern * 12 + material).
pub fn trim_line(packed: u8) -> Option<String> {
    let (p, m) = unpack(packed)?;
    Some(format!("Acabamento: {} ({})", PATTERNS[p].1, MATERIALS[m].1))
}

pub fn unpack(packed: u8) -> Option<(usize, usize)> {
    let p = (packed / 12) as usize;
    let m = (packed % 12) as usize;
    (p >= 1 && p <= PATTERNS.len() && m < MATERIALS.len()).then(|| (p - 1, m))
}

pub fn trim_of(s: &ItemStack) -> Option<(usize, usize)> {
    unpack(s.ench.get(TRIM))
}

/// The trim colour on a worn piece, if any.
pub fn trim_color(s: &ItemStack) -> Option<[u8; 3]> {
    trim_of(s).map(|(_, m)| MATERIALS[m].2)
}

/// What the smithing table makes from (template, base, addition), if anything.
pub fn result(items: &ItemRegistry, template: Option<ItemStack>, base: Option<ItemStack>, add: Option<ItemStack>) -> Option<ItemStack> {
    let (t, b, a) = (template?, base?, add?);
    let tn = items.get(t.item).name.as_str();
    let bn = items.get(b.item).name.as_str();
    let an = items.get(a.item).name.as_str();
    if tn == UPGRADE {
        if an != "netherite_ingot" {
            return None;
        }
        let up = bn.strip_prefix("diamond_")?;
        let id = items.id(&format!("netherite_{up}"))?;
        return Some(ItemStack { item: id, count: 1, damage: b.damage, ench: b.ench });
    }
    let pattern = tn.strip_suffix("_armor_trim_smithing_template")?;
    let p = PATTERNS.iter().position(|x| x.0 == pattern)?;
    let m = MATERIALS.iter().position(|x| x.0 == an)?;
    items.get(b.item).armor?;
    let mut out = ItemStack { count: 1, ..b };
    out.ench.set(TRIM, ((p + 1) * 12 + m) as u8);
    (out != b).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_pack_and_unpack() {
        for p in 0..PATTERNS.len() {
            for m in 0..MATERIALS.len() {
                let packed = ((p + 1) * 12 + m) as u8;
                assert_eq!(unpack(packed), Some((p, m)));
            }
        }
        assert_eq!(unpack(0), None);
    }
}
