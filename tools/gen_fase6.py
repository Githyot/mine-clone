#!/usr/bin/env python3
"""Writes the Phase 6 data files in packs/vanilla/fase6/: blocks, items, recipes, structures
(trial chambers, ancient cities, trail ruins, desert wells and beach ruins) and mobs (warden,
breeze, sniffer).

The game merges these lists into the base ones, so this stage of the game keeps its data apart
from the files the other stages edit. Every recipe is checked against the items that exist
(base game + this file) so a typo can't stop the game from loading.

Usage:
    python3 tools/gen_fase6.py

Textures and mob skins come from tools/gen_fase6_art.py.
"""

import re
from pathlib import Path
from lote3_structures import ocean_monument  # Lote 3 of plan/o-que-falta.md

ROOT = Path(__file__).resolve().parent.parent
PACK = ROOT / "packs" / "vanilla"
OUT = PACK / "fase6"

# ---------------------------------------------------------------------------
# Potions (read from effects.rs so the two never disagree)
# ---------------------------------------------------------------------------

EFFECTS_RS = (ROOT / "crates" / "game" / "src" / "effects.rs").read_text()
POTIONS = re.findall(r'p!\("([a-z_]+)", "([^"]+)"', EFFECTS_RS)


def potion_names(key, name):
    """Display names of the four forms of a potion."""
    if key == "water":
        return {
            "potion": "Garrafa de Água",
            "splash_potion": "Garrafa de Água Arremessável",
            "lingering_potion": "Garrafa de Água Persistente",
            "tipped_arrow": "Flecha Molhada",
        }
    stem = name.removeprefix("Poção ")
    return {
        "potion": name,
        "splash_potion": f"Poção Arremessável {stem}",
        "lingering_potion": f"Poção Persistente {stem}",
        "tipped_arrow": f"Flecha {stem}",
    }


# ---------------------------------------------------------------------------
# Blocks
# ---------------------------------------------------------------------------


def ron_val(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    return v


def block(name, display, **kw):
    parts = [f'name: "{name}"', f'display_name: Some("{display}")']
    tex = kw.pop("tex", None)
    for k, v in kw.items():
        parts.append(f"{k}: {ron_val(v)}")
    if tex:
        if isinstance(tex, str):
            tex = {"all": tex}
        parts.append("textures: (" + ", ".join(f'{k}: Some("{v}")' for k, v in tex.items()) + ")")
    return "    (" + ", ".join(parts) + "),"


PICK = "Some(Pickaxe)"
AXE = "Some(Axe)"
SHOVEL = "Some(Shovel)"


def drops(*items):
    out = []
    for it in items:
        if isinstance(it, tuple):
            n, lo, hi = it
            out.append(f'(item: "{n}", min: {lo}, max: {hi})')
        else:
            out.append(f'(item: "{it}")')
    return "Some([" + ", ".join(out) + "])"


def stone_family(base, display, texture, slab=True, stairs=True, hardness=3.5, tier=1):
    out = [block(base, display, hardness=hardness, tool=PICK, tier=tier, tex=texture)]
    stem = base.removesuffix("s") if base.endswith("bricks") or base.endswith("tiles") else base
    if slab:
        out.append(block(f"{stem}_slab", f"Laje de {display}", hardness=hardness, shape="Slab", tool=PICK, tier=tier, tex=texture))
    if stairs:
        out.append(block(f"{stem}_stairs", f"Escada de {display}", hardness=hardness, shape="Stairs", tool=PICK, tier=tier, tex=texture))
    return out


BLOCKS = []
B = BLOCKS.append
BX = BLOCKS.extend
B("    // Work blocks")
B(block("enchanting_table", "Mesa de Encantamento", hardness=5.0, tool=PICK, tier=1, light=7, tex={"top": "enchanting_table_top", "side": "enchanting_table_side", "bottom": "enchanting_table_bottom"}))
for n, d in (("anvil", "Bigorna"), ("chipped_anvil", "Bigorna Lascada"), ("damaged_anvil", "Bigorna Danificada")):
    B(block(n, d, hardness=5.0, tool=PICK, tier=1, gravity=True, tex={"top": f"{n}_top", "side": "anvil_side", "bottom": "anvil_side"}))
B(block("grindstone", "Rebolo", hardness=2.0, tool=PICK, tier=1, tex={"top": "grindstone_top", "side": "grindstone_side", "bottom": "grindstone_bottom"}))
B(block("smithing_table", "Bancada de Ferraria", hardness=2.5, tool=AXE, tex={"top": "smithing_table_top", "side": "smithing_table_side", "front": "smithing_table_front", "bottom": "smithing_table_bottom"}))
B(block("brewing_stand", "Suporte de Poções", hardness=0.5, tool=PICK, tier=1, shape="Cross", transparency="Cutout", solid=False, light=1, tex="brewing_stand"))
B(block("beacon", "Sinalizador", hardness=3.0, light=15, transparency="Cutout", tex="beacon"))
B(block("conduit", "Conduíte", hardness=3.0, tool=PICK, light=15, shape="Lantern", transparency="Cutout", tex={"side": "conduit", "top": "conduit_top", "bottom": "conduit_top"}))
B("    // Ocean stone")
B(block("prismarine", "Prismarinho", hardness=1.5, tool=PICK, tier=1, tex="prismarine"))
B(block("prismarine_bricks", "Tijolos de Prismarinho", hardness=1.5, tool=PICK, tier=1, tex="prismarine_bricks"))
B(block("dark_prismarine", "Prismarinho Escuro", hardness=1.5, tool=PICK, tier=1, tex="dark_prismarine"))
B(block("sea_lantern", "Lanterna do Mar", hardness=0.3, light=15, tex="sea_lantern", drops=drops(("prismarine_crystals", 2, 3))))
B("    // Metal blocks for beacons")
B(block("netherite_block", "Bloco de Netherita", hardness=50.0, tool=PICK, tier=4, tex="netherite_block"))
B(block("ancient_debris", "Detritos Ancestrais", hardness=30.0, tool=PICK, tier=4, tex={"side": "ancient_debris_side", "top": "ancient_debris_top"}))
B(block("emerald_block", "Bloco de Esmeralda", hardness=5.0, tool=PICK, tier=3, tex="emerald_block"))
B(block("lapis_block", "Bloco de Lápis-lazúli", hardness=3.0, tool=PICK, tier=2, tex="lapis_block"))
B(block("frosted_ice", "Gelo Fosco", hardness=0.5, transparency="Translucent", in_inventory=False, drops="Some([])", tex="frosted_ice"))
B(block("sugar_cane", "Cana-de-açúcar", shape="Cross", transparency="Cutout", solid=False, hardness=0.0, tex="sugar_cane"))
B(block("cobweb", "Teia", shape="Cross", transparency="Cutout", solid=False, hardness=4.0, drops=drops("string"), tex="cobweb"))
B(block("slime_block", "Bloco de Slime", hardness=0.0, transparency="Translucent", tex="slime_block"))
B(block("candle", "Vela", shape="Torch", transparency="Cutout", solid=False, light=6, hardness=0.1, tex="candle"))
B(block("soul_lantern", "Lanterna de Almas", shape="Lantern", transparency="Cutout", light=10, hardness=1.5, tool=PICK, tex={"side": "soul_lantern", "top": "soul_lantern_top", "bottom": "soul_lantern_top"}))
B("    // Deep dark")
B(block("sculk", "Sculk", hardness=0.2, tool="Some(Hoe)", xp="(1, 1)", drops="Some([])", tex="sculk"))
B(block("sculk_vein", "Veio de Sculk", shape="Layer", transparency="Cutout", solid=False, hardness=0.2, tool="Some(Hoe)", tex="sculk_vein"))
for n, d, extra in (("sculk_sensor", "Sensor de Sculk", {}), ("calibrated_sculk_sensor", "Sensor de Sculk Calibrado", {})):
    B(block(n, d, hardness=1.5, tool="Some(Hoe)", height="Some(8)", transparency="Cutout", light=1, xp="(5, 5)", tex={"top": f"{n}_top", "side": f"{n}_side", "bottom": "sculk"}))
    B(block(f"{n}_active", d, hardness=1.5, tool="Some(Hoe)", height="Some(8)", transparency="Cutout", light=8, in_inventory=False, drops=drops(n), tex={"top": f"{n}_top_active", "side": f"{n}_side", "bottom": "sculk"}))
B(block("sculk_shrieker", "Guinchador de Sculk", hardness=3.0, tool="Some(Hoe)", height="Some(8)", transparency="Cutout", xp="(5, 5)", tex={"top": "sculk_shrieker_top", "side": "sculk_shrieker_side", "bottom": "sculk_shrieker_bottom"}))
B(block("sculk_catalyst", "Catalisador de Sculk", hardness=3.0, tool="Some(Hoe)", light=6, xp="(5, 5)", tex={"top": "sculk_catalyst_top", "side": "sculk_catalyst_side", "bottom": "sculk_catalyst_bottom"}))
B(block("deepslate", "Ardósia", hardness=3.0, tool=PICK, tier=1, drops=drops("cobbled_deepslate"), tex={"side": "deepslate", "top": "deepslate_top"}))
BX(stone_family("cobbled_deepslate", "Pedregulho de Ardósia", "cobbled_deepslate"))
BX(stone_family("polished_deepslate", "Ardósia Polida", "polished_deepslate"))
BX(stone_family("deepslate_bricks", "Tijolos de Ardósia", "deepslate_bricks"))
BX(stone_family("deepslate_tiles", "Ladrilhos de Ardósia", "deepslate_tiles"))
B(block("cracked_deepslate_bricks", "Tijolos de Ardósia Rachados", hardness=3.5, tool=PICK, tier=1, tex="cracked_deepslate_bricks"))
B(block("chiseled_deepslate", "Ardósia Talhada", hardness=3.5, tool=PICK, tier=1, tex="chiseled_deepslate"))
B(block("reinforced_deepslate", "Ardósia Reforçada", hardness=55.0, drops="Some([])", tex={"side": "reinforced_deepslate_side", "top": "reinforced_deepslate_top", "bottom": "reinforced_deepslate_bottom"}))
B("    // Trial chambers")
BX(stone_family("tuff", "Tufo", "tuff", hardness=1.5))
BX(stone_family("polished_tuff", "Tufo Polido", "polished_tuff", hardness=1.5))
BX(stone_family("tuff_bricks", "Tijolos de Tufo", "tuff_bricks", hardness=1.5))
B(block("chiseled_tuff", "Tufo Talhado", hardness=1.5, tool=PICK, tier=1, tex={"side": "chiseled_tuff", "top": "chiseled_tuff_top"}))
B(block("chiseled_tuff_bricks", "Tijolos de Tufo Talhados", hardness=1.5, tool=PICK, tier=1, tex={"side": "chiseled_tuff_bricks", "top": "chiseled_tuff_bricks_top"}))
for n, d, light in (("trial_spawner", "Gerador dos Desafios", 4), ("trial_spawner_active", "Gerador dos Desafios", 8), ("trial_spawner_ominous", "Gerador dos Desafios Agourento", 8)):
    B(block(n, d, hardness=50.0, transparency="Cutout", light=light, drops="Some([])", in_inventory=n == "trial_spawner", tex={"side": f"{n}_side", "top": f"{n}_top", "bottom": "trial_spawner_bottom"}))
B(block("vault", "Cofre", hardness=50.0, transparency="Cutout", light=6, drops="Some([])", tex={"side": "vault_side", "front": "vault_front", "top": "vault_top", "bottom": "vault_bottom"}))
B(block("ominous_vault", "Cofre Agourento", hardness=50.0, transparency="Cutout", light=6, drops="Some([])", tex={"side": "ominous_vault_side", "front": "ominous_vault_front", "top": "ominous_vault_top", "bottom": "vault_bottom"}))
B(block("heavy_core", "Núcleo Pesado", hardness=10.0, tool=PICK, tier=1, shape="Lantern", transparency="Cutout", tex={"side": "heavy_core", "top": "heavy_core_top", "bottom": "heavy_core_top"}))
B("    // Archaeology")
B(block("suspicious_sand", "Areia Suspeita", hardness=0.25, tool=SHOVEL, drops="Some([])", tex="suspicious_sand"))
B(block("suspicious_gravel", "Cascalho Suspeito", hardness=0.25, tool=SHOVEL, drops="Some([])", tex="suspicious_gravel"))
B(block("decorated_pot", "Vaso Decorado", hardness=0.0, transparency="Cutout", shape="Lantern", tex={"side": "decorated_pot_side", "top": "decorated_pot_top", "bottom": "decorated_pot_top"}))
B(block("sniffer_egg", "Ovo de Farejador", hardness=0.5, shape="Lantern", transparency="Cutout", tex={"side": "sniffer_egg", "top": "sniffer_egg_top", "bottom": "sniffer_egg_top"}))
B(block("torchflower", "Flor-tocha", shape="Cross", transparency="Cutout", solid=False, hardness=0.0, light=3, tex="torchflower"))
B(block("pitcher_plant", "Planta-jarro", shape="Cross", transparency="Cutout", solid=False, hardness=0.0, tex="pitcher_plant"))
for k in range(3):
    last = k == 2
    B(block(f"torchflower_stage{k}", "Flor-tocha", shape="Cross", transparency="Cutout", solid=False, hardness=0.0, in_inventory=False,
            drops=drops("torchflower") if last else drops("torchflower_seeds"), tex=f"torchflower_stage{k}"))
for k in range(5):
    last = k == 4
    B(block(f"pitcher_stage{k}", "Planta-jarro", shape="Cross", transparency="Cutout", solid=False, hardness=0.0, in_inventory=False,
            drops=drops("pitcher_plant") if last else drops("pitcher_pod"), tex=f"pitcher_stage{k}"))

# ---------------------------------------------------------------------------
# Items
# ---------------------------------------------------------------------------


def item(name, display, **kw):
    parts = [f'name: "{name}"', f'display_name: Some("{display}")']
    for k, v in kw.items():
        parts.append(f"{k}: {ron_val(v)}")
    return "    (" + ", ".join(parts) + "),"


def tool(kind, tier, speed, damage, dur):
    return f"Some((kind: {kind}, tier: {tier}, speed: {speed}, damage: {damage}, durability: {dur}))"


def armor(slot, points, dur):
    return f"Some((slot: {slot}, points: {points}, durability: {dur}))"


def food(h, s):
    return f"Some((hunger: {h}, saturation: {s}))"


ITEMS = []
I = ITEMS.append
I("    // Potions: every potion in four forms (the bottle colours are drawn per potion)")
for key, name in POTIONS:
    names = potion_names(key, name)
    for form in ("potion", "splash_potion", "lingering_potion"):
        I(item(f"{form}_{key}", names[form], max_stack="Some(1)"))
    I(item(f"tipped_arrow_{key}", names["tipped_arrow"]))
I("    // Brewing")
for n, d in (
    ("glass_bottle", "Frasco de Vidro"), ("blaze_powder", "Pó de Blaze"), ("magma_cream", "Creme de Magma"), ("ghast_tear", "Lágrima de Ghast"),
    ("glistering_melon_slice", "Fatia de Melancia Reluzente"), ("rabbit_foot", "Pé de Coelho"), ("fermented_spider_eye", "Olho de Aranha Fermentado"),
    ("sugar", "Açúcar"), ("glowstone_dust", "Pó de Pedra Luminosa"), ("phantom_membrane", "Membrana de Phantom"), ("turtle_scute", "Escama de Tartaruga"),
    ("slime_ball", "Bola de Slime"), ("dragon_breath", "Bafo do Dragão"), ("paper", "Papel"), ("book", "Livro"),
    ("prismarine_shard", "Fragmento de Prismarinho"), ("prismarine_crystals", "Cristais de Prismarinho"), ("nautilus_shell", "Concha de Náutilo"),
    ("heart_of_the_sea", "Coração do Mar"), ("nether_star", "Estrela do Nether"), ("netherite_scrap", "Fragmento de Netherita"),
    ("netherite_ingot", "Lingote de Netherita"), ("echo_shard", "Fragmento de Eco"), ("breeze_rod", "Vara de Brisa"),
    ("trial_key", "Chave dos Desafios"), ("ominous_trial_key", "Chave dos Desafios Agourenta"), ("torchflower_seeds", "Sementes de Flor-tocha"),
    ("pitcher_pod", "Vagem de Planta-jarro"), ("compass", "Bússola"),
):
    extra = {}
    if n in ("torchflower_seeds",):
        extra["places"] = 'Some("torchflower_stage0")'
    if n in ("pitcher_pod",):
        extra["places"] = 'Some("pitcher_stage0")'
    if n in ("trial_key", "ominous_trial_key", "nether_star", "heart_of_the_sea", "compass"):
        extra["max_stack"] = "Some(64)"
    I(item(n, d, **extra))
I(item("spider_eye", "Olho de Aranha", food=food(2, 3.2)))
I(item("golden_carrot", "Cenoura Dourada", food=food(6, 14.4)))
I(item("golden_apple", "Maçã Dourada", food=food(4, 9.6)))
I(item("enchanted_golden_apple", "Maçã Dourada Encantada", food=food(4, 9.6)))
I(item("melon_slice", "Fatia de Melancia", food=food(2, 1.2)))
I(item("pufferfish", "Baiacu", food=food(1, 0.2)))
I(item("honey_bottle", "Garrafa de Mel", max_stack="Some(16)"))
I(item("milk_bucket", "Balde de Leite", max_stack="Some(1)"))
I(item("experience_bottle", "Frasco de Experiência"))
I(item("ominous_bottle", "Frasco Agourento"))
I(item("enchanted_book", "Livro Encantado", max_stack="Some(1)"))
I(item("recovery_compass", "Bússola de Recuperação", max_stack="Some(1)"))
I(item("brush", "Escova", max_stack="Some(1)", tool=tool("Hoe", 1, 1.0, 1.0, 64)))
I(item("wind_charge", "Carga de Vento"))
I("    // Weapons")
I(item("crossbow", "Besta", max_stack="Some(1)", durability="Some(465)"))
I(item("trident", "Tridente", tool=tool("Sword", 4, 1.0, 9.0, 250)))
I(item("mace", "Maça", tool=tool("Sword", 3, 1.0, 6.0, 500)))
SPEARS = (("wooden", "Madeira", 1, 2.0, 59), ("stone", "Pedra", 2, 3.0, 131), ("copper", "Cobre", 2, 3.5, 190), ("iron", "Ferro", 3, 4.0, 250),
          ("golden", "Ouro", 1, 2.0, 32), ("diamond", "Diamante", 4, 5.0, 1561), ("netherite", "Netherita", 4, 6.0, 2031))
for m, d, tier, dmg, dur in SPEARS:
    I(item(f"{m}_spear", f"Lança de {d}", tool=tool("Sword", tier, 2.0, dmg, dur)))
I("    // Netherite gear")
for kind, d, dmg in (("pickaxe", "Picareta", 6.0), ("axe", "Machado", 10.0), ("shovel", "Pá", 6.5), ("sword", "Espada", 8.0), ("hoe", "Enxada", 1.0)):
    I(item(f"netherite_{kind}", f"{d} de Netherita", tool=tool(kind.capitalize(), 4, 9.0, dmg, 2031)))
for slot, (part, d, pts, dur) in enumerate((("helmet", "Capacete", 3, 407), ("chestplate", "Peitoral", 8, 592), ("leggings", "Calças", 6, 555), ("boots", "Botas", 3, 481))):
    I(item(f"netherite_{part}", f"{d} de Netherita", armor=armor(slot, pts, dur)))
I(item("turtle_helmet", "Casco de Tartaruga", armor=armor(0, 2, 275)))
I("    // Smithing templates")
TRIMS = re.findall(r'\("([a-z]+)", "([^"]+)"\),', (ROOT / "crates/game/src/smithing.rs").read_text().split("pub const MATERIALS")[0])
for p, d in TRIMS:
    I(item(f"{p}_armor_trim_smithing_template", f"Molde de Acabamento {d}"))
I(item("netherite_upgrade_smithing_template", "Molde de Melhoria de Netherita"))
I("    // Pottery sherds")
SHERDS = re.findall(r'\("([a-z_]+)", "([^"]+)"\),', (ROOT / "crates/game/src/archaeology.rs").read_text().split("pub const SHERDS")[1].split("];")[0])
for s, d in SHERDS:
    I(item(f"{s}_pottery_sherd", f"Caco de Cerâmica {d}"))

# ---------------------------------------------------------------------------
# Recipes
# ---------------------------------------------------------------------------

SHAPED = []
SHAPELESS = []
SMELTING = []
TAGS = {
    "pot_faces": ["brick"] + [f"{s}_pottery_sherd" for s, _ in SHERDS],
}


def shaped(pattern, key, result, count=1):
    SHAPED.append((pattern, key, result, count))


def shapeless(ingredients, result, count=1):
    SHAPELESS.append((ingredients, result, count))


def smelt(inp, out, xp=0.1):
    SMELTING.append((inp, out, xp))


shaped([" B ", "D#D", "###"], {"B": "book", "D": "diamond", "#": "obsidian"}, "enchanting_table")
shaped(["MMM", "SES", "WWW"], {"M": "milk_bucket", "S": "sugar", "E": "egg", "W": "wheat"}, "cake")
shaped(["III", " i ", "iii"], {"I": "iron_block", "i": "iron_ingot"}, "anvil")
shaped(["/S/", "P P"], {"/": "stick", "S": "stone_slab", "P": "#planks"}, "grindstone")
shaped(["PPP", "BBB", "PPP"], {"P": "#planks", "B": "book"}, "bookshelf")
shaped(["II", "PP", "PP"], {"I": "iron_ingot", "P": "#planks"}, "smithing_table")
shaped([" B ", "###"], {"B": "blaze_rod", "#": "cobblestone"}, "brewing_stand")
shaped(["GGG", "GNG", "OOO"], {"G": "glass", "N": "nether_star", "O": "obsidian"}, "beacon")
shaped(["NNN", "NHN", "NNN"], {"N": "nautilus_shell", "H": "heart_of_the_sea"}, "conduit")
shaped(["G G", " G "], {"G": "glass"}, "glass_bottle", 3)
shaped(["SS", "SS"], {"S": "prismarine_shard"}, "prismarine")
shaped(["SSS", "SSS", "SSS"], {"S": "prismarine_shard"}, "prismarine_bricks")
shaped(["SSS", "SIS", "SSS"], {"S": "prismarine_shard", "I": "black_wool"}, "dark_prismarine")
shaped(["SCS", "CCC", "SCS"], {"S": "prismarine_shard", "C": "prismarine_crystals"}, "sea_lantern")
for b, i in (("netherite_block", "netherite_ingot"), ("emerald_block", "emerald"), ("lapis_block", "lapis_lazuli")):
    shaped(["###", "###", "###"], {"#": i}, b)
    shapeless([b], i, 9)
shapeless(["netherite_scrap"] * 4 + ["gold_ingot"] * 4, "netherite_ingot")
smelt("ancient_debris", "netherite_scrap", 2.0)
shaped(["SSS"], {"S": "sugar_cane"}, "paper", 3)
shapeless(["sugar_cane"], "sugar")
shapeless(["book", "ink_sac", "feather"], "writable_book")
shapeless(["paper", "paper", "paper", "leather"], "book")
shapeless(["blaze_rod"], "blaze_powder", 2)
shapeless(["slime_ball", "blaze_powder"], "magma_cream")
shapeless(["spider_eye", "brown_mushroom", "sugar"], "fermented_spider_eye")
shaped(["NNN", "NMN", "NNN"], {"N": "gold_nugget", "M": "melon_slice"}, "glistering_melon_slice")
shaped(["NNN", "NCN", "NNN"], {"N": "gold_nugget", "C": "carrot"}, "golden_carrot")
shaped(["GGG", "GAG", "GGG"], {"G": "gold_ingot", "A": "apple"}, "golden_apple")
shapeless(["melon"], "melon_slice", 9)
shaped(["DD", "DD"], {"D": "glowstone_dust"}, "glowstone")
shapeless(["glowstone"], "glowstone_dust", 4)
shaped(["SSS", "SSS", "SSS"], {"S": "slime_ball"}, "slime_block")
shapeless(["slime_block"], "slime_ball", 9)
shaped(["S S", " S ", "S S"], {"S": "string"}, "cobweb")
shaped(["S", "H"], {"S": "string", "H": "honeycomb"}, "candle")
shaped(["III", "IRI", "III"], {"I": "iron_ingot", "R": "redstone"}, "compass")
shaped(["EEE", "ECE", "EEE"], {"E": "echo_shard", "C": "compass"}, "recovery_compass")
shaped(["F", "C", "/"], {"F": "feather", "C": "copper_ingot", "/": "stick"}, "brush")
shaped([" B ", "B B", " B "], {"B": "#pot_faces"}, "decorated_pot")
shaped(["/I/", "S/S", " / "], {"/": "stick", "I": "iron_ingot", "S": "string"}, "crossbow")
shaped(["H", "R"], {"H": "heavy_core", "R": "breeze_rod"}, "mace")
shapeless(["breeze_rod"], "wind_charge", 4)
shaped([" A ", "ASA", " A "], {"A": "amethyst", "S": "sculk_sensor"}, "calibrated_sculk_sensor")
shaped(["I I", "S S", " I "], {"I": "iron_ingot", "S": "soul_sand"}, "soul_lantern")
shaped(["SSS", "S S"], {"S": "turtle_scute"}, "turtle_helmet")
for m, mat in (("wooden", "#planks"), ("stone", "cobblestone"), ("copper", "copper_ingot"), ("iron", "iron_ingot"), ("golden", "gold_ingot"), ("diamond", "diamond")):
    shaped(["  M", " / ", "/  "], {"M": mat, "/": "stick"}, f"{m}_spear")
# Templates copy themselves with seven diamonds and a block of where they are found.
TEMPLATE_BLOCK = {
    "sentry": "cobblestone", "dune": "sandstone", "coast": "cobblestone", "wild": "mossy_cobblestone", "ward": "cobbled_deepslate",
    "eye": "end_stone", "vex": "cobblestone", "tide": "prismarine", "snout": "blackstone", "rib": "netherrack", "spire": "purpur_block",
    "wayfinder": "terracotta", "shaper": "terracotta", "silence": "cobbled_deepslate", "raiser": "terracotta", "host": "terracotta",
    "flow": "breeze_rod", "bolt": "copper_block",
}
for p, _ in TRIMS:
    t = f"{p}_armor_trim_smithing_template"
    shaped(["DTD", "DMD", "DDD"], {"D": "diamond", "T": t, "M": TEMPLATE_BLOCK[p]}, t, 2)
shaped(["DTD", "DMD", "DDD"], {"D": "diamond", "T": "netherite_upgrade_smithing_template", "M": "netherrack"}, "netherite_upgrade_smithing_template", 2)
# Deepslate and tuff families.
smelt("cobbled_deepslate", "deepslate")
shaped(["##", "##"], {"#": "cobbled_deepslate"}, "polished_deepslate", 4)
shaped(["##", "##"], {"#": "polished_deepslate"}, "deepslate_bricks", 4)
shaped(["##", "##"], {"#": "deepslate_bricks"}, "deepslate_tiles", 4)
shaped(["#", "#"], {"#": "cobbled_deepslate_slab"}, "chiseled_deepslate")
smelt("deepslate_bricks", "cracked_deepslate_bricks")
shaped(["##", "##"], {"#": "tuff"}, "polished_tuff", 4)
shaped(["##", "##"], {"#": "polished_tuff"}, "tuff_bricks", 4)
shaped(["#", "#"], {"#": "tuff_slab"}, "chiseled_tuff")
shaped(["#", "#"], {"#": "tuff_brick_slab"}, "chiseled_tuff_bricks")
for base in ("cobbled_deepslate", "polished_deepslate", "deepslate_brick", "deepslate_tile", "tuff", "polished_tuff", "tuff_brick"):
    full = base + ("s" if base.endswith(("brick", "tile")) else "")
    shaped(["###"], {"#": full}, f"{base}_slab", 6)
    shaped(["#  ", "## ", "###"], {"#": full}, f"{base}_stairs", 4)

# ---------------------------------------------------------------------------
# Mobs
# ---------------------------------------------------------------------------

MOBS = """    // The warden: blind guardian of the deep dark, called up by sculk shriekers.
    (
        name: "warden", display_name: "Warden", kind: Hostile, health: 500, speed: 2.2,
        drops: [], xp: 5,
        width: 0.9, height: 2.9, spawn: Never, attack: 30, scale: 1.45,
        parts: [
            (size: (4, 10, 4), origin: (-4, 0, -2), uv: (32, 18), anim: Leg(0), pivot: (-2, 10, 0)),
            (size: (4, 10, 4), origin: (0, 0, -2), uv: (32, 18), anim: Leg(1), pivot: (2, 10, 0)),
            (size: (10, 12, 6), origin: (-5, 10, -3), uv: (0, 18)),
            (size: (10, 10, 8), origin: (-5, 22, -4), uv: (0, 0), anim: Head, pivot: (0, 22, 0)),
            (size: (4, 14, 4), origin: (-9, 8, -2), uv: (36, 0), anim: Arm(0), pivot: (-7, 21, 0)),
            (size: (4, 14, 4), origin: (5, 8, -2), uv: (36, 0), anim: Arm(1), pivot: (7, 21, 0)),
            (size: (3, 5, 1), origin: (-8, 27, 0), uv: (48, 18), anim: Head, pivot: (0, 22, 0)),
            (size: (3, 5, 1), origin: (5, 27, 0), uv: (48, 18), anim: Head, pivot: (0, 22, 0)),
        ],
        layers: [(part: 2, inflate: 0.5), (part: 3, inflate: 0.4), (part: 4, inflate: 0.4), (part: 5, inflate: 0.4)],
    ),
    // The breeze: a whirl of wind that hops about the trial chambers firing wind charges.
    (
        name: "breeze", display_name: "Brisa", kind: Hostile, health: 30, speed: 2.6,
        drops: [], xp: 10,
        width: 0.6, height: 1.77, spawn: Never, attack: 1,
        parts: [
            (size: (8, 8, 8), origin: (-4, 18, -4), uv: (0, 0), anim: Head, pivot: (0, 18, 0)),
            (size: (6, 3, 6), origin: (-3, 13, -3), uv: (32, 0)),
            (size: (4, 4, 4), origin: (-2, 8, -2), uv: (32, 9)),
            (size: (8, 3, 8), origin: (-4, 3, -4), uv: (0, 16)),
            (size: (2, 8, 2), origin: (-1, 10, -1), uv: (32, 17)),
            (size: (1, 6, 1), origin: (-5, 8, 0), uv: (40, 17), anim: Arm(0), pivot: (-5, 14, 0)),
            (size: (1, 6, 1), origin: (4, 8, 0), uv: (40, 17), anim: Arm(1), pivot: (4, 14, 0)),
        ],
        layers: [(part: 0, inflate: 0.5), (part: 3, inflate: 0.6)],
    ),
    // The sniffer: a gentle giant back from the past, hatched from eggs found in ruins.
    (
        name: "sniffer", display_name: "Farejador", kind: Passive, health: 14, speed: 1.0,
        drops: [(item: "moss_block", min: 0, max: 1)], xp: 1,
        width: 1.9, height: 1.75, spawn: Never, attack: 0, scale: 2.0,
        parts: [
            (size: (3, 4, 3), origin: (-6, 0, -7), uv: (32, 23), anim: Leg(0), pivot: (-5, 4, -6)),
            (size: (3, 4, 3), origin: (3, 0, -7), uv: (32, 23), anim: Leg(1), pivot: (4, 4, -6)),
            (size: (3, 4, 3), origin: (-6, 0, -1), uv: (32, 23), anim: Leg(1), pivot: (-5, 4, 0)),
            (size: (3, 4, 3), origin: (3, 0, -1), uv: (32, 23), anim: Leg(0), pivot: (4, 4, 0)),
            (size: (3, 4, 3), origin: (-6, 0, 5), uv: (32, 23), anim: Leg(0), pivot: (-5, 4, 6)),
            (size: (3, 4, 3), origin: (3, 0, 5), uv: (32, 23), anim: Leg(1), pivot: (4, 4, 6)),
            (size: (12, 8, 15), origin: (-6, 4, -7), uv: (0, 0)),
            (size: (8, 6, 8), origin: (-4, 6, 8), uv: (0, 23), anim: Head, pivot: (0, 9, 8)),
            (size: (4, 2, 3), origin: (-2, 6, 16), uv: (44, 23), anim: Head, pivot: (0, 9, 8)),
        ],
        layers: [(part: 6, inflate: 0.6)],
    ),
"""

# ---------------------------------------------------------------------------
# Structures
# ---------------------------------------------------------------------------


def grid(w, d, ch=" "):
    return [[ch] * w for _ in range(d)]


def rows(g):
    return ["".join(r) for r in g]


def trial_chamber():
    """A buried chamber of tuff and copper: a central hall with a vault on a dais, spawners in
    the four corners, corridors with cobwebs and pots, chests on the sides. 31 x 31, 11 tall,
    carved out well under the ground."""
    W = D = 31
    H = 11
    layers = [grid(W, D, "T") for _ in range(H)]
    for y in range(1, H - 1):
        for z in range(1, D - 1):
            for x in range(1, W - 1):
                layers[y][z][x] = "."
    # The hall floor: tuff bricks with a copper ring and chiseled tuff in a cross.
    for z in range(D):
        for x in range(W):
            ring = max(abs(x - 15), abs(z - 15))
            layers[0][z][x] = "b" if ring % 4 else "C"
            if x == 15 or z == 15:
                layers[0][z][x] = "c"
    # Walls: tuff bricks with polished tuff bands and copper bulbs for light.
    for y in range(1, H - 1):
        for z in range(D):
            for x in range(W):
                if x in (0, W - 1) or z in (0, D - 1):
                    layers[y][z][x] = "p" if y in (1, 5) else "b"
                    if y == 3 and (x + z) % 6 == 0:
                        layers[y][z][x] = "L"
    # Ceiling of polished tuff with copper grates letting the light through.
    for z in range(D):
        for x in range(W):
            layers[H - 1][z][x] = "G" if (x % 5 == 0 and z % 5 == 0) else "p"
    # Inner pillars around the hall.
    for (px, pz) in ((7, 7), (23, 7), (7, 23), (23, 23)):
        for y in range(1, H - 1):
            for dz in (0, 1):
                for dx in (0, 1):
                    layers[y][pz + dz][px + dx] = "x" if y % 3 == 0 else "b"
    # Spawners in the corners, a little raised.
    for (sx, sz) in ((3, 3), (27, 3), (3, 27), (27, 27)):
        layers[0][sz][sx] = "p"
        layers[1][sz][sx] = "S"
    # Two side rooms with a spawner each, behind cut copper arches.
    for sx in (15,):
        for sz in (4, 26):
            layers[1][sz][sx] = "S"
    # The vault dais in the middle: two steps, the vault, an ominous one beside.
    for z in range(13, 18):
        for x in range(13, 18):
            layers[1][z][x] = "c"
    layers[2][15][15] = "V"
    layers[2][15][17] = "O"
    layers[2][15][13] = "V"
    # Chests along the sides, decorated pots and cobwebs in the corners of the ceiling.
    for (cx, cz) in ((1, 15), (29, 15)):
        layers[1][cz][cx] = "h"
    for (ox, oz) in ((1, 1), (29, 1), (1, 29), (29, 29), (10, 1), (20, 29), (1, 10), (29, 20)):
        layers[1][oz][ox] = "d"
    for (wx, wz) in ((1, 1), (29, 1), (1, 29), (29, 29), (2, 1), (1, 2), (28, 29), (29, 28)):
        layers[H - 2][wz][wx] = "w"
        layers[H - 3][wz][wx] = "w"
    # Copper doors' frames: openings in the middle of each wall to the corridors (kept as air).
    for y in (1, 2, 3):
        for k in (14, 15, 16):
            layers[y][0][k] = "."
            layers[y][D - 1][k] = "."
            layers[y][k][0] = "."
            layers[y][k][W - 1] = "."
    return {
        "name": "camara_dos_desafios",
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Savanna", "Dark Forest", "Snowy Plains", "Cherry Grove", "Jungle", "Desert", "Badlands", "Mountains", "Swamp"],
        "spacing": 14,
        "chance": 0.55,
        "sink": H + 22,
        "palette": {"T": "tuff", "b": "tuff_bricks", "p": "polished_tuff", "c": "chiseled_tuff", "x": "chiseled_tuff_bricks", "C": "cut_copper",
                    "G": "copper_grate", "L": "copper_bulb_lit", "S": "trial_spawner", "V": "vault", "O": "ominous_vault", "h": "chest",
                    "d": "decorated_pot", "w": "cobweb"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "bread", 1, 3, 0.7), ("", "arrow", 4, 12, 0.6), ("", "wind_charge", 1, 3, 0.5), ("", "trial_key", 1, 1, 0.35),
                 ("", "honey_bottle", 1, 2, 0.4), ("", "tipped_arrow_poison", 2, 6, 0.4), ("", "iron_ingot", 1, 4, 0.5),
                 ("", "emerald", 1, 4, 0.5), ("", "golden_carrot", 1, 3, 0.4), ("", "potion_regeneration", 1, 1, 0.25),
                 ("", "ominous_bottle", 1, 1, 0.15), ("", "experience_bottle", 1, 3, 0.4)],
    }


def ancient_city():
    """A ruined city of deepslate deep below: a reinforced deepslate frame in the middle (the
    'portal'), bridges of deepslate tiles, sculk spreading over the floor, shriekers and
    sensors among soul lanterns, chests with echo shards and enchanted books."""
    W = D = 31
    H = 15
    layers = [grid(W, D, ".") for _ in range(H)]
    # The cavern floor: deepslate with sculk spread over it, a little lower at the edges.
    for z in range(D):
        for x in range(W):
            n = (x * 7 + z * 13 + x * z) % 11
            layers[0][z][x] = "s" if n < 6 else ("D" if n < 9 else "k")
    # Raised plazas of deepslate tiles and bricks.
    for z in range(4, 27):
        for x in range(4, 27):
            layers[1][z][x] = "t" if (x + z) % 2 else "B"
    for z in range(4, 27):
        for x in range(4, 27):
            if x in (4, 26) or z in (4, 26):
                layers[2][z][x] = "B" if (x + z) % 3 else "w"
    # The frame: reinforced deepslate, a great arch facing south.
    for y in range(2, 13):
        for x in (9, 21):
            layers[y][15][x] = "R"
    for x in range(9, 22):
        layers[12][15][x] = "R"
        layers[13][15][x] = "R" if 11 <= x <= 19 else "."
    for x in range(10, 21):
        layers[2][15][x] = "R"
    # Inside of the frame: dark, with sculk veins.
    for y in range(3, 12):
        for x in range(10, 21):
            layers[y][15][x] = "."
    # Little ruined houses with chests: deepslate brick walls, half-broken.
    for (hx, hz) in ((5, 5), (21, 5), (5, 21), (21, 21)):
        for y in range(2, 6):
            for z in range(hz, hz + 5):
                for x in range(hx, hx + 5):
                    edge = x in (hx, hx + 4) or z in (hz, hz + 4)
                    broken = (x * 3 + y * 5 + z) % 7 == 0
                    if edge and not broken and not (z == hz + 4 and x == hx + 2 and y < 4):
                        layers[y][z][x] = "B" if y < 5 else "Q"
        layers[2][hz + 2][hx + 2] = "h"
        layers[2][hz + 1][hx + 1] = "L"
    # Sculk machinery in the plazas: shriekers, sensors, catalysts, soul lanterns on posts.
    for (x, z, c) in ((8, 12, "H"), (22, 12, "H"), (12, 20, "H"), (18, 9, "H"), (15, 8, "N"), (11, 24, "N"), (25, 15, "N"), (6, 15, "N"),
                      (15, 24, "A"), (14, 6, "A")):
        layers[2][z][x] = c
    for (x, z) in ((7, 7), (23, 7), (7, 23), (23, 23), (15, 18), (15, 12)):
        layers[2][z][x] = "w"
        layers[3][z][x] = "w"
        layers[4][z][x] = "l"
    # Candles on the tiles.
    for (x, z) in ((10, 10), (20, 10), (10, 20), (20, 20), (13, 17), (17, 17)):
        layers[2][z][x] = "c"
    # A dome of deepslate over all of it, so it sits in a cavern of its own.
    for y in range(H):
        for z in range(D):
            for x in range(W):
                if x in (0, W - 1) or z in (0, D - 1) or y == H - 1:
                    layers[y][z][x] = "D"
    return {
        "name": "cidade_ancestral",
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Savanna", "Dark Forest", "Snowy Plains", "Cherry Grove", "Jungle", "Mountains", "Swamp"],
        "spacing": 24,
        "chance": 0.5,
        "sink": H + 34,
        "palette": {"s": "sculk", "D": "deepslate", "k": "cobbled_deepslate", "t": "deepslate_tiles", "B": "deepslate_bricks",
                    "Q": "cracked_deepslate_bricks", "w": "polished_deepslate", "R": "reinforced_deepslate", "h": "chest",
                    "L": "soul_lantern", "l": "soul_lantern", "H": "sculk_shrieker", "N": "sculk_sensor", "A": "sculk_catalyst",
                    "c": "candle"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "echo_shard", 1, 3, 0.6), ("", "enchanted_book", 1, 1, 0.55), ("", "experience_bottle", 1, 3, 0.5),
                 ("", "golden_apple", 1, 2, 0.35), ("", "enchanted_golden_apple", 1, 1, 0.06), ("", "sculk_sensor", 1, 3, 0.3),
                 ("", "sculk_catalyst", 1, 2, 0.2), ("", "diamond_leggings", 1, 1, 0.12), ("", "diamond_hoe", 1, 1, 0.15),
                 ("", "amethyst", 1, 4, 0.4), ("", "candle", 1, 4, 0.4), ("", "ward_armor_trim_smithing_template", 1, 1, 0.12),
                 ("", "silence_armor_trim_smithing_template", 1, 1, 0.04), ("", "lapis_lazuli", 2, 8, 0.4),
                 ("", "potion_regeneration", 1, 1, 0.2), ("", "recovery_compass", 1, 1, 0.05)],
    }


def trail_ruins():
    """Half-buried ruins of terracotta and bricks with suspicious gravel in their floors."""
    W, D, H = 15, 15, 6
    layers = [grid(W, D, " ") for _ in range(H)]
    for z in range(D):
        for x in range(W):
            n = (x * 5 + z * 11 + x * z * 3) % 13
            layers[0][z][x] = "g" if n < 3 else ("G" if n < 9 else "m")
            layers[1][z][x] = "G" if n % 2 else "d"
    for y in range(2, H):
        for z in range(D):
            for x in range(W):
                inner = 2 <= x <= 12 and 2 <= z <= 12
                wall = x in (2, 12) or z in (2, 12)
                broken = (x * 7 + y * 3 + z * 5) % (y + 2) == 0
                if inner and wall and not broken and y < 5:
                    layers[y][z][x] = "t" if (x + y + z) % 3 else "b"
                elif inner:
                    layers[y][z][x] = "."
    for (x, z) in ((4, 4), (10, 5), (6, 9), (9, 10), (7, 7), (11, 11)):
        layers[2][z][x] = "g"
    layers[2][7][3] = "h"
    return {
        "name": "ruinas_da_trilha",
        "biomes": ["Taiga", "Forest", "Birch Forest", "Jungle", "Snowy Plains", "Dark Forest"],
        "spacing": 10,
        "chance": 0.4,
        "sink": 4,
        "palette": {"g": "suspicious_gravel", "G": "gravel", "m": "mossy_cobblestone", "d": "dirt", "t": "terracotta", "b": "bricks", "h": "chest"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "emerald", 1, 3, 0.5), ("", "wheat", 2, 6, 0.5), ("", "brush", 1, 1, 0.4), ("", "candle", 1, 3, 0.4),
                 ("", "heart_of_the_sea", 1, 1, 0.05)],
    }


def desert_well():
    """A sandstone well whose floor hides suspicious sand."""
    layers = [
        ["sssss", "ssSss", "sSwSs", "ssSss", "sssss"],
        ["sssss", "s...s", "s.w.s", "s...s", "sssss"],
        ["l...l", ".....", ".....", ".....", "l...l"],
        ["l...l", ".....", ".....", ".....", "l...l"],
        ["lllll", "lllll", "lllll", "lllll", "lllll"],
    ]
    return {
        "name": "poco_do_deserto",
        "biomes": ["Desert"],
        "spacing": 9,
        "chance": 0.45,
        "sink": 2,
        "palette": {"s": "sandstone", "S": "suspicious_sand", "w": "water", "l": "sandstone_slab"},
        "layers": layers,
        "loot": [],
    }


def beach_ruin():
    """A sunken ruin of prismarine and stone bricks on the shore: suspicious sand around it,
    a chest of sea treasure, sometimes a sniffer egg."""
    W = D = 9
    layers = [grid(W, D, " ") for _ in range(4)]
    for z in range(D):
        for x in range(W):
            n = (x * 3 + z * 7 + x * z) % 9
            layers[0][z][x] = "S" if n < 3 else "a"
    for y in (1, 2, 3):
        for z in range(1, 8):
            for x in range(1, 8):
                edge = x in (1, 7) or z in (1, 7)
                broken = (x + z * 2 + y * 3) % (5 - y + 1) == 0
                if edge and not broken:
                    layers[y][z][x] = "p" if (x + z + y) % 2 else "B"
                elif not edge:
                    layers[y][z][x] = "."
    layers[1][4][4] = "h"
    layers[1][3][5] = "S"
    return {
        "name": "ruina_da_praia",
        "biomes": ["Beach"],
        "spacing": 8,
        "chance": 0.45,
        "sink": 2,
        "palette": {"S": "suspicious_sand", "a": "sand", "p": "prismarine", "B": "stone_bricks", "h": "chest"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "prismarine_shard", 2, 6, 0.6), ("", "prismarine_crystals", 1, 4, 0.5), ("", "nautilus_shell", 1, 2, 0.4),
                 ("", "heart_of_the_sea", 1, 1, 0.2), ("", "sniffer_egg", 1, 1, 0.25), ("", "gold_nugget", 2, 6, 0.4),
                 ("", "tide_armor_trim_smithing_template", 1, 1, 0.1), ("", "turtle_scute", 1, 3, 0.3)],
    }


def structures_ron(structs):
    out = ["// Generated by tools/gen_fase6.py: edit that script, not this file.", "(structures: ["]
    for s in structs:
        sz = (len(s["layers"][0][0]), len(s["layers"][0]))
        assert max(sz) <= 32, (s["name"], sz)
        for k in s["palette"]:
            assert k not in ". ", s["name"]
        out.append("    (")
        out.append(f'        name: "{s["name"]}",')
        out.append("        biomes: [" + ", ".join(f'"{b}"' for b in s["biomes"]) + "],")
        out.append(f'        spacing: {s["spacing"]},')
        out.append(f'        chance: {s["chance"]},')
        if s.get("sink"):
            out.append(f'        sink: {s["sink"]},')
        if s.get("foundation"):
            out.append(f'        foundation: Some("{s["foundation"]}"),')
        if s.get("rough"):
            out.append(f'        rough: {s["rough"]},')
        if s.get("mobs"):
            out.append("        mobs: [" + ", ".join(f'("{m}", {n})' for m, n in s["mobs"]) + "],")
        out.append("        palette: [" + ", ".join(f"('{k}', \"{v}\")" for k, v in s["palette"].items()) + "],")
        out.append("        layers: [")
        for layer in s["layers"]:
            out.append("            [" + ", ".join(f'"{r}"' for r in layer) + "],")
        out.append("        ],")
        if s["loot"]:
            out.append("        loot: [")
            for _, it, lo, hi, ch in s["loot"]:
                out.append(f'            (item: "{it}", min: {lo}, max: {hi}, chance: {ch}),')
            out.append("        ],")
        out.append("    ),")
    out.append("])")
    return "\n".join(out) + "\n"


# ---------------------------------------------------------------------------
# Checks and output
# ---------------------------------------------------------------------------


def names_in(text):
    return set(re.findall(r'name: *"([a-z0-9_]+)"', text))


def recipes_ron(known):
    def ing(v):
        if v.startswith("#"):
            assert v[1:] in TAGS or v[1:] in BASE_TAGS, v
            return v
        assert v in known, f"unknown item in recipe: {v}"
        return v

    out = ["// Generated by tools/gen_fase6.py: edit that script, not this file.", "(", "    tags: {"]
    for t, lst in TAGS.items():
        for v in lst:
            assert v in known, f"unknown item in tag {t}: {v}"
        out.append(f'        "{t}": [' + ", ".join(f'"{v}"' for v in lst) + "],")
    out.append("    },")
    out.append("    shaped: [")
    for pattern, key, result, count in SHAPED:
        for row in pattern:
            for ch in row:
                assert ch == " " or ch in key, (result, ch)
        k = ", ".join(f"'{c}': \"{ing(v)}\"" for c, v in key.items())
        assert result in known, result
        cnt = f", count: {count}" if count != 1 else ""
        out.append(f'        (pattern: [{", ".join(chr(34) + r + chr(34) for r in pattern)}], key: {{{k}}}, result: "{result}"{cnt}),')
    out.append("    ],")
    out.append("    shapeless: [")
    for ings, result, count in SHAPELESS:
        assert result in known, result
        cnt = f", count: {count}" if count != 1 else ""
        out.append(f'        (ingredients: [{", ".join(chr(34) + ing(v) + chr(34) for v in ings)}], result: "{result}"{cnt}),')
    out.append("    ],")
    out.append("    smelting: [")
    for inp, res, xp in SMELTING:
        ing(inp)
        assert res in known, res
        out.append(f'        (input: "{inp}", result: "{res}", xp: {xp}),')
    out.append("    ],")
    out.append(")")
    return "\n".join(out) + "\n"


BASE_RECIPES = (PACK / "recipes.ron").read_text()
BASE_TAGS = set(re.findall(r'"([a-z_]+)": \[', BASE_RECIPES.split("shaped:")[0]))


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    # Tags belong to their file: copy the base game's ones this file uses.
    head = BASE_RECIPES.split("shaped:")[0]
    for t in BASE_TAGS:
        used = any(v == f"#{t}" for _, key, _, _ in SHAPED for v in key.values()) or any(f"#{t}" in i for i, _, _ in SHAPELESS)
        if used and t not in TAGS:
            TAGS[t] = re.findall(r'"([a-z0-9_]+)"', re.search(rf'"{t}": \[([^\]]*)\]', head).group(1))
    # Things the base game (or another stage) already defines stay out of this file.
    base_b, base_i = names_in((PACK / "blocks.ron").read_text()), names_in((PACK / "items.ron").read_text())
    BLOCKS[:] = [b for b in BLOCKS if not (names_in(b) & base_b)]
    ITEMS[:] = [i for i in ITEMS if not (names_in(i) & (base_i | base_b))]
    blocks_text = "// Generated by tools/gen_fase6.py: edit that script, not this file.\n(blocks: [\n" + "\n".join(BLOCKS) + "\n])\n"
    items_text = "// Generated by tools/gen_fase6.py: edit that script, not this file.\n(items: [\n" + "\n".join(ITEMS) + "\n])\n"
    base_blocks = names_in((PACK / "blocks.ron").read_text())
    base_items = names_in((PACK / "items.ron").read_text())
    known = base_blocks | base_items | names_in(blocks_text) | names_in(items_text)
    # Check the shape of a smelting entry against the base file.
    sample = re.search(r"smelting: \[\s*\(([^)]*)\)", BASE_RECIPES)
    assert sample and "input:" in sample.group(1), "smelting entries changed shape"
    structs = [trial_chamber(), ancient_city(), trail_ruins(), beach_ruin(), ocean_monument()]  # the base game has its own desert well
    for s in structs:
        for v in s["palette"].values():
            assert v.split(":")[0] in known, f"{s['name']}: unknown block {v}"
        for _, it, *_ in s["loot"]:
            assert it in known, f"{s['name']}: unknown loot {it}"
    (OUT / "blocks.ron").write_text(blocks_text)
    (OUT / "items.ron").write_text(items_text)
    base_lines = {l.strip() for l in BASE_RECIPES.splitlines()}
    (OUT / "recipes.ron").write_text("\n".join(l for l in recipes_ron(known).splitlines() if not (l.strip().startswith(("(pattern", "(ingredients", "(input")) and l.strip() in base_lines)) + "\n")
    (OUT / "structures.ron").write_text(structures_ron(structs))
    (OUT / "mobs.ron").write_text("// Generated by tools/gen_fase6.py: edit that script, not this file.\n(mobs: [\n" + MOBS + "])\n")
    print(f"{len([b for b in BLOCKS if b.strip().startswith('(')])} blocks, {len([i for i in ITEMS if i.strip().startswith('(')])} items, "
          f"{len(SHAPED) + len(SHAPELESS) + len(SMELTING)} recipes, {len(structs)} structures, 3 mobs")


if __name__ == "__main__":
    main()
