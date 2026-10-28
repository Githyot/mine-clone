//! Material feel for items and armour (plan/materiais.md): diamond glints and sparkles, metals
//! take a coloured sheen and mirror the sky, leather, wood, stone and cloth stay matte with a
//! relief taken from their own texture. Every texture layer gets a class here; the shaders
//! (`shaders/material.wgsl`) hold what each class looks like.

pub const NONE: u32 = 0;
pub const DIAMOND: u32 = 1;
pub const GEM: u32 = 2;
pub const GOLD: u32 = 3;
pub const IRON: u32 = 4;
pub const COPPER: u32 = 5;
pub const NETHERITE: u32 = 6;
pub const CHAIN: u32 = 7;
pub const LEATHER: u32 = 8;
pub const CLOTH: u32 = 9;
pub const LACQUER: u32 = 10;
pub const WOOD: u32 = 11;
pub const STONE: u32 = 12;
pub const BONE: u32 = 13;
pub const SHELL: u32 = 14;
pub const CELESTIAL: u32 = 15;

/// Added to a block's class for an ore: only its coloured gem or metal flecks shine.
pub const ORE: u32 = 256;
/// Block texture layers the terrain shader can address (11 bits in the vertex).
pub const BLOCK_LAYERS: usize = 2048;

/// Option levels: Não, Simples (glints and sheen), Completo (also relief and sparkles).
pub const LEVELS: [&str; 3] = ["Não", "Simples", "Completo"];

/// The class of an armour material (`entity::ARMOR_MATERIALS` names, item-name prefixes).
pub fn armor_class(name: &str) -> u32 {
    if name.starts_with("dye_") {
        return LACQUER;
    }
    if name.starts_with("vanity_") || crate::vanity::OUTFITS.contains(&name) {
        return CLOTH;
    }
    match name {
        "leather" => LEATHER,
        "chainmail" => CHAIN,
        "diamond" => DIAMOND,
        "netherite" | "umbral" | "sanguinite" | "magmarite" => NETHERITE,
        "golden" | "radiant" | "gladiator" => GOLD,
        "copper" | "bronze" | "meteor" | "resonarium" => COPPER,
        "iron" | "lead" | "silver" | "tungsten" | "platinum" | "cobalt" | "palladium" | "mythril" | "orichalcum" | "adamantite" | "titanium"
        | "gravitite" | "valkyrie" => IRON,
        "verdite" | "lumite" | "zanite" => GEM,
        "hive" | "jungle" | "sporite" | "beetle" | "turtle" => SHELL,
        "fossil" | "necro" => BONE,
        "spectral" | "solar" | "vortex" | "nebula" | "stardust" | "phoenix" => CELESTIAL,
        "ninja" | "tuxedo" | "pirate" | "explorer" | "royal" | "pyromancer" | "cryomancer" | "electromancer" | "priest" | "voidwalker"
        | "bloodmage" | "cultist" | "archevoker" => CLOTH,
        _ => NONE,
    }
}

/// The class of an armour trim, by its material item (`smithing::MATERIALS`).
pub fn trim_class(item: &str) -> u32 {
    match item {
        "diamond" => DIAMOND,
        "emerald" | "amethyst_shard" | "quartz" | "lapis_lazuli" | "redstone" => GEM,
        "gold_ingot" => GOLD,
        "copper_ingot" => COPPER,
        "netherite_ingot" => NETHERITE,
        "resin_brick" => SHELL,
        _ => IRON,
    }
}

/// The class of an item or block texture, from words in its name ("item/diamond_sword",
/// "gold_block", "stick"...). Armour pieces follow their armour material.
pub fn texture_class(name: &str) -> u32 {
    let b = name.rsplit('/').next().unwrap_or(name);
    if b.contains("spawn_egg") || b.contains("_ore") || b.contains("pottery") {
        return NONE;
    }
    for piece in ["_helmet", "_chestplate", "_leggings", "_boots"] {
        if let Some(prefix) = b.strip_suffix(piece) {
            let c = armor_class(prefix);
            if c != NONE {
                return c;
            }
        }
    }
    let has = |words: &[&str]| words.iter().any(|w| b.contains(w));
    if has(&["diamond"]) {
        DIAMOND
    } else if has(&["netherite", "umbral", "sanguinite"]) {
        NETHERITE
    } else if has(&["chainmail", "chain"]) {
        CHAIN
    } else if has(&["emerald", "amethyst", "crystal", "quartz", "lapis", "prismarine", "lumite", "verdite", "zanite", "ruby", "sapphire", "topaz", "gem"]) {
        GEM
    } else if has(&["solar", "vortex", "nebula", "stardust", "spectral", "phoenix", "star_"]) {
        CELESTIAL
    } else if has(&["gold", "radiant", "gladiator"]) {
        GOLD
    } else if has(&["copper", "bronze", "meteor", "resonarium"]) {
        COPPER
    } else if has(&[
        "iron", "silver", "platinum", "titanium", "tungsten", "tin_", "cobalt", "palladium", "mythril", "orichalcum", "adamantite", "valkyrie", "gravitite", "steel",
        "shears", "bucket", "anvil", "trident",
    ]) || b.starts_with("lead_")
    {
        IRON
    } else if has(&["leather", "saddle", "rabbit_hide"]) {
        LEATHER
    } else if has(&["bone", "fossil", "necro"]) {
        BONE
    } else if has(&["beetle", "turtle", "scute", "sporite", "hive"]) {
        SHELL
    } else if has(&["wooden", "_planks", "_log", "fishing_rod"]) || b == "stick" || b == "bow" || b == "crossbow" || b == "bowl" {
        WOOD
    } else if b.starts_with("stone_") || has(&["flint", "cobblestone"]) || b == "stone" {
        STONE
    } else if has(&["wool", "robe", "banner", "cloth"]) {
        CLOTH
    } else {
        NONE
    }
}

/// The class of every texture layer, in layer order (the item shader's table).
pub fn texture_table(names: &[String]) -> Vec<u32> {
    let mut v: Vec<u32> = names.iter().map(|n| texture_class(n)).collect();
    if v.is_empty() {
        v.push(NONE);
    }
    v
}

/// The class of a block texture for the terrain: only metal and gem blocks and ores shine
/// (stone, wood and the rest already have their relief and stay matte).
pub fn block_class(name: &str) -> u32 {
    let b = name.rsplit('/').next().unwrap_or(name);
    if b.starts_with("raw_") || b.contains("glass") || b.contains("spawn_egg") {
        return NONE;
    }
    if let Some(i) = b.find("_ore") {
        let metal = &b[..i];
        let has = |w: &[&str]| w.iter().any(|x| metal.contains(x));
        let c = if has(&["diamond"]) {
            DIAMOND
        } else if has(&["emerald", "lapis", "redstone", "quartz", "amethyst", "ruby", "sapphire", "topaz", "crystal", "gem", "lumite", "verdite", "zanite"]) {
            GEM
        } else if has(&["gold"]) {
            GOLD
        } else if has(&["copper", "meteor"]) {
            COPPER
        } else if has(&["coal"]) {
            NONE
        } else {
            IRON
        };
        return if c == NONE { NONE } else { c | ORE };
    }
    if !(b.ends_with("_block") || b.contains("_bars") || b.contains("chain") || b.contains("anvil") || b.starts_with("budding_")) {
        return NONE;
    }
    match texture_class(b) {
        c @ (DIAMOND | GEM | GOLD | IRON | COPPER | NETHERITE | CHAIN | CELESTIAL) => c,
        _ => NONE,
    }
}

/// The block class of every terrain texture layer (the terrain shader's table, always
/// `BLOCK_LAYERS` long).
pub fn block_table(names: &[String]) -> Vec<u32> {
    let mut v: Vec<u32> = names.iter().take(BLOCK_LAYERS).map(|n| block_class(n)).collect();
    v.resize(BLOCK_LAYERS, NONE);
    v
}

/// `Globals::mat`: x the option level (0 off, 1 simple, 2 full).
pub fn uniform(level: u8, fast_quality: bool) -> [f32; 4] {
    let l = if fast_quality { level.min(1) } else { level.min(2) };
    [l as f32, 0.0, 0.0, 0.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_by_name() {
        assert_eq!(texture_class("item/diamond_sword"), DIAMOND);
        assert_eq!(texture_class("item/diamond_chestplate"), DIAMOND);
        assert_eq!(texture_class("item/golden_helmet"), GOLD);
        assert_eq!(texture_class("item/iron_pickaxe"), IRON);
        assert_eq!(texture_class("item/wooden_sword"), WOOD);
        assert_eq!(texture_class("item/stone_axe"), STONE);
        assert_eq!(texture_class("item/leather_boots"), LEATHER);
        assert_eq!(texture_class("item/stick"), WOOD);
        assert_eq!(texture_class("diamond_ore"), NONE);
        assert_eq!(texture_class("item/jungle_helmet"), SHELL);
        assert_eq!(texture_class("item/apple"), NONE);
        assert_eq!(armor_class("dye_red"), LACQUER);
        assert_eq!(block_class("diamond_block"), DIAMOND);
        assert_eq!(block_class("iron_block"), IRON);
        assert_eq!(block_class("deepslate_diamond_ore"), DIAMOND | ORE);
        assert_eq!(block_class("lapis_ore"), GEM | ORE);
        assert_eq!(block_class("iron_ore"), IRON | ORE);
        assert_eq!(block_class("tin_ore"), IRON | ORE);
        assert_eq!(block_class("coal_ore"), NONE);
        assert_eq!(block_class("stone"), NONE);
        assert_eq!(block_class("raw_iron_block"), NONE);
        assert_eq!(armor_class("vanity_pirate"), CLOTH);
    }

    #[test]
    fn every_armour_has_a_class() {
        for (n, _) in crate::entity::ARMOR_MATERIALS {
            assert_ne!(armor_class(n), NONE, "{n}");
        }
    }
}
