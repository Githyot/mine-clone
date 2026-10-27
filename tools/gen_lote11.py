#!/usr/bin/env python3
"""Lote 11 (the rest of Terraria's gear): wings, mounts, pets and light pets, vanity outfits,
dyes and sentry staffs. Writes their items and recipes to packs/vanilla/rpg/lote11_*.ron and
draws their original 16x16 icons (tools/tex64/build.py grows them to 64x64 afterwards).

The Rust side: wings.rs, companions.rs, vanity.rs, sentries.rs. Names here must match the
tables there (the game's tests check both ways).

Usage:
    python3 tools/gen_lote11.py [sheet.png]
    python3 tools/tex64/build.py --only <names printed at the end>
"""

import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_etapa5_textures as e5  # noqa: E402
import gen_fase6_art as f6  # noqa: E402
import gen_items as gi  # noqa: E402
import gen_textures as gt  # noqa: E402
from gen_items import outline, pal, pmap  # noqa: E402
from gen_rpg import PACK, known_items, recolor  # noqa: E402

OUT = PACK / "rpg"
from_map = e5.from_map


def icon(rows, p):
    return outline(from_map(rows, pmap(p)))


def hexes(colors):
    return pal(*["#%02x%02x%02x" % tuple(c) for c in colors])


# --------------------------------------------------------------------------
# Shapes (16x16; '1' darkest .. '5' lightest)
# --------------------------------------------------------------------------

FEATHER_WINGS = e5.ACC["feather_wings"][0]

MEMBRANE_WINGS = [
    "................",
    "................",
    ".1..........1...",
    ".11........11...",
    "1331......1331..",
    "13331....13331..",
    "133331..133331..",
    "1323231123232311".replace("11", "1."),
    ".12121.1.12121..",
    ".1.1.1...1.1.1..",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
]

INSECT_WINGS = [
    "................",
    "................",
    ".1111......1111.",
    "144441....144441",
    "1433341..1433341",
    "14333341143333 1".replace(" ", "4"),
    ".1433341143334 1".replace(" ", "1"),
    "..14441..14441..",
    "...1331..1331...",
    "...13441144331..",
    "....1441144 1...".replace(" ", "1"),
    ".....11..11.....",
    "................",
    "................",
    "................",
    "................",
]

LEAF_WINGS = [
    "................",
    "................",
    "..11........11..",
    ".1341......1431.",
    "134431....134431",
    "1344431..1344431",
    ".1344311113443 1".replace(" ", "1"),
    "..1344311344 1..".replace(" ", "1"),
    "...13431134 1...".replace(" ", "1"),
    "....131..131....",
    ".....1....1.....",
    "................",
    "................",
    "................",
    "................",
    "................",
]

SOUL = [
    "................",
    "................",
    "......11........",
    ".....1341.......",
    "....134431......",
    "...13455431.....",
    "...1345543 1....".replace(" ", "1"),
    "....134431.11...",
    ".....1331.1331..",
    "......11..1431..",
    "...........11...",
    "................",
    "................",
    "................",
    "................",
    "................",
]

SADDLE_BLOB = [
    "................",
    "................",
    "................",
    "......1111......",
    "....11222211....",
    "...1255555221...",
    "..125344443521..",
    "..123444444321..",
    ".12344444444321.",
    ".12333333333321.",
    ".11222222222211.",
    "..111111111111..",
    "................",
    "................",
    "................",
    "................",
]

GOGGLES = [
    "................",
    "................",
    "................",
    "................",
    "..1111....1111..",
    ".122221..122221.",
    "12455421124554 1".replace(" ", "2"),
    "1245542112455421",
    "12444421124444 1".replace(" ", "2"),
    ".122221..122221.",
    "..1111....1111..",
    "................",
    "................",
    "................",
    "................",
    "................",
]

CARROT = [
    "................",
    "..........1.1...",
    ".........1515...",
    "..........151...",
    ".........1331...",
    "........13431...",
    ".......134431...",
    "......134431....",
    ".....13431......",
    "....13431.......",
    "...1331.........",
    "..131...........",
    "..11............",
    "................",
    "................",
    "................",
]

APPLE = [
    "................",
    "................",
    "........1.......",
    ".......15.1.....",
    "....111111511...",
    "...1344443441...",
    "..134455444431..",
    "..134554444431..",
    "..134444444331..",
    "..133444443331..",
    "...1334443331...",
    "....13333331....",
    ".....111111.....",
    "................",
    "................",
    "................",
]

TRUFFLE = [
    "................",
    "................",
    "................",
    ".....111111.....",
    "...1145544411...",
    "..145544444441..",
    ".14454444445441.",
    ".13333333333331.",
    "..111222222111..",
    ".....12332......",
    ".....12332......",
    ".....12332......",
    "......1111......",
    "................",
    "................",
    "................",
]

BELL = [
    "................",
    "................",
    ".......11.......",
    "......1551......",
    ".....134431.....",
    "....13444431....",
    "....13444431....",
    "....13444431....",
    "...1344444431...",
    "..133333333331..",
    "..111111111111..",
    "......1221......",
    ".......11.......",
    "................",
    "................",
    "................",
]

FISH = e5.ACC.get("swim_fins", (None,))[0] if False else [
    "................",
    "................",
    "................",
    "................",
    "...........11...",
    "....11111.1331..",
    "..1144444113431.",
    ".14554444443331.",
    ".14444444443331.",
    "..1133333113331.",
    "....11111.1331..",
    "...........11...",
    "................",
    "................",
    "................",
    "................",
]

ACORN = [
    "................",
    "................",
    ".......11.......",
    "......1331......",
    "....11222211....",
    "...1255555521...",
    "...1222222221...",
    "....13444431....",
    "....14455441....",
    "....13444431....",
    ".....134431.....",
    "......1331......",
    ".......11.......",
    "................",
    "................",
    "................",
]

EGG = [
    "................",
    "................",
    "................",
    "......1111......",
    ".....134431.....",
    "....13454431....",
    "...1345444431...",
    "...1344443431...",
    "...1344344431...",
    "...1334444331...",
    "....13333331....",
    ".....111111.....",
    "................",
    "................",
    "................",
    "................",
]

CRACKER = [
    "................",
    "................",
    "................",
    "................",
    "...1111111111...",
    "...1444444441...",
    "...1454545451...",
    "...1444444441...",
    "...1454545451...",
    "...1444444441...",
    "...1333333331...",
    "...1111111111...",
    "................",
    "................",
    "................",
    "................",
]

JAR = [
    "................",
    "......1111......",
    "......1221......",
    ".....111111.....",
    "....1......1....",
    "....1..5...1....",
    "....1.555..1....",
    "....1..5.5.1....",
    "....1...555.....".replace(".....", "1....", 1) if False else "....1...5551....",
    "....1.5..5.1....",
    "....1.555..1....",
    "....1..5...1....",
    ".....111111.....",
    "................",
    "................",
    "................",
]

LANTERN = [
    "................",
    ".......11.......",
    "......1..1......",
    ".....111111.....",
    ".....122221.....",
    "....12544521....",
    "....14555541....",
    "....14555541....",
    "....12544521....",
    ".....122221.....",
    ".....111111.....",
    "................",
    "................",
    "................",
    "................",
    "................",
]

TOWER = [
    "................",
    "......1111......",
    ".....145541.....",
    ".....155551.....",
    ".....145541.....",
    "......1111......",
    "......1221......",
    ".....123321.....",
    ".....123321.....",
    "....12333321....",
    "....12333321....",
    "...1233333321...",
    "...1222222221...",
    "..111111111111..",
    "................",
    "................",
]

CRYSTAL = [
    "................",
    ".......1........",
    "......141.......",
    "......1451......",
    ".....14551......",
    ".....145541.....",
    ".....145441.....",
    "......1441......",
    "......1331......",
    ".....123321.....",
    "....12333321....",
    "...1222222221...",
    "..111111111111..",
    "................",
    "................",
    "................",
]

TOTEM = [
    "................",
    "......1111......",
    ".....155551.....",
    ".....151151.....",
    ".....155551.....",
    "......1111......",
    "....11233211....",
    "...1523333251...",
    "....11233211....",
    ".....123321.....",
    ".....143341.....",
    ".....123321.....",
    "....12222221....",
    "...1111111111...",
    "................",
    "................",
]

BALLISTA = [
    "................",
    "................",
    "1..............1",
    "21............12",
    ".21....11....12.",
    "..2111144111112.".replace("2.", "2."),
    "...12555555521..",
    "....1233333211..",
    ".......11.......",
    "......1221......",
    ".....123321.....",
    "....12333321....",
    "...1222222221...",
    "..111111111111..",
    "................",
    "................",
]

# --------------------------------------------------------------------------
# Content
# --------------------------------------------------------------------------

WINGS = [
    # name, display, shape, colours (dark, mid, light) as in wings.rs
    ("harpy_wings", "Asas de Harpia", FEATHER_WINGS, [[60, 84, 120], [96, 136, 176], [170, 206, 228]]),
    ("demon_wings", "Asas de Demônio", MEMBRANE_WINGS, [[50, 14, 20], [110, 28, 36], [170, 60, 60]]),
    ("bee_wings", "Asas de Abelha", INSECT_WINGS, [[120, 96, 30], [220, 200, 110], [250, 244, 200]]),
    ("fairy_wings", "Asas de Fada", INSECT_WINGS, [[150, 60, 140], [230, 140, 210], [255, 220, 250]]),
    ("butterfly_wings", "Asas de Borboleta", INSECT_WINGS, [[24, 40, 110], [50, 110, 210], [150, 210, 255]]),
    ("bone_wings", "Asas de Osso", MEMBRANE_WINGS, [[110, 104, 90], [196, 190, 170], [236, 232, 218]]),
    ("leaf_wings", "Asas de Folha", LEAF_WINGS, [[30, 80, 26], [70, 150, 50], [150, 210, 90]]),
    ("frozen_wings", "Asas Congeladas", FEATHER_WINGS, [[60, 120, 170], [130, 200, 236], [220, 248, 255]]),
    ("flame_wings", "Asas de Chamas", FEATHER_WINGS, [[140, 30, 10], [236, 110, 30], [255, 220, 120]]),
    ("breaker_wings", "Asas do Quebra-Mundos", MEMBRANE_WINGS, [[36, 30, 48], [90, 70, 120], [170, 140, 220]]),
    ("celestial_wings", "Asas Celestiais", FEATHER_WINGS, [[40, 90, 120], [90, 200, 210], [210, 255, 250]]),
]


def wing_icon(rows, cols):
    d, m, l = cols
    mid2 = [(a + b) // 2 for a, b in zip(d, m)]
    p = hexes([d, mid2, m, l, [min(255, c + 30) for c in l]])
    # The feather shape uses 1..4; map 4 to the lightest.
    return icon(rows, p)


SIMPLE = [
    # (name, display, rows, palette, extra)
    ("soul_of_flight", "Alma do Voo", SOUL, pal("#1e4a6a", "#3a7ab0", "#6ab8e8", "#b0e8ff", "#f0fcff"), ""),
    # Mounts.
    ("slime_saddle", "Sela de Geleia", SADDLE_BLOB, pal("#1e5a2a", "#3a8a3c", "#5ab04a", "#8ad070", "#d0f8b8"), "max_stack: Some(1)"),
    ("honeyed_goggles", "Óculos com Mel", GOGGLES, pal("#4a3008", "#8a5a10", "#c48a18", "#f0c040", "#fff0a0"), "max_stack: Some(1)"),
    ("fuzzy_carrot", "Cenoura Felpuda", CARROT, pal("#6a2a08", "#b04a10", "#e07a20", "#ffb050", "#80c040"), "max_stack: Some(1)"),
    ("blessed_apple", "Maçã Abençoada", APPLE, pal("#6a4a08", "#b08a18", "#e0c040", "#fff080", "#60a030"), "max_stack: Some(1)"),
    ("cosmic_truffle", "Trufa Cósmica", TRUFFLE, pal("#2a1a4a", "#5a3a8a", "#8a6ac8", "#c0a8f0", "#f0e8ff"), "max_stack: Some(1)"),
    # Pets.
    ("fox_bell", "Sininho de Raposa", BELL, pal("#6a3008", "#b05a10", "#e08a30", "#ffc070", "#fff0d0"), "max_stack: Some(1)"),
    ("frozen_fish", "Peixe Congelado", FISH, pal("#2a4a6a", "#4a7aa0", "#7ab0d0", "#b8e0f0", "#f0fcff"), "max_stack: Some(1)"),
    ("golden_acorn", "Bolota Dourada", ACORN, pal("#5a3a08", "#8a5a10", "#c49418", "#f0d040", "#fffbd8"), "max_stack: Some(1)"),
    ("spiny_apple", "Maçã Espinhenta", APPLE, pal("#3a2a10", "#6a4a20", "#9a7040", "#c8a070", "#60a030"), "max_stack: Some(1)"),
    ("axolotl_egg", "Ovo de Axolote", EGG, pal("#6a2a4a", "#b05a80", "#e08ab0", "#ffc0d8", "#fff0f8"), "max_stack: Some(1)"),
    ("parrot_cracker", "Biscoito de Papagaio", CRACKER, pal("#5a3a10", "#8a5a20", "#c08a40", "#e8b868", "#fff0c0"), "max_stack: Some(1)"),
    ("firefly_jar", "Pote de Vaga-Lumes", JAR, pal("#3a4a50", "#6a8088", "#a0b8c0", "#d0e0e4", "#f0f060"), "max_stack: Some(1)"),
    ("allay_bell", "Sino do Allay", BELL, pal("#1a3a6a", "#3a6ab0", "#6aa0e8", "#b0d8ff", "#f0fcff"), "max_stack: Some(1)"),
    ("wisp_lantern", "Lanterna do Fogo-Fátuo", LANTERN, pal("#1a3a4a", "#2a6a80", "#40a0c0", "#80e0f0", "#e0ffff"), "max_stack: Some(1)"),
    # Sentry turrets (the pictures the sentries are drawn with).
    ("sentry_ember", "Torre de Brasa", TOWER, pal("#3a0804", "#8a1a08", "#d04a10", "#ff9a30", "#fff0a0"), "max_stack: Some(1)"),
    ("sentry_frost", "Cristal de Gelo", CRYSTAL, pal("#1a3a5a", "#3a6a9a", "#6aa8d8", "#b0e0ff", "#f0fcff"), "max_stack: Some(1)"),
    ("sentry_storm", "Totem da Tempestade", TOTEM, pal("#2a2a3a", "#4a4a6a", "#7a7aa0", "#b0b0d8", "#f0f0a0"), "max_stack: Some(1)"),
    ("sentry_ballista", "Balista", BALLISTA, pal("#2a1a0a", "#5a3a1a", "#8a5a30", "#b88a50", "#d8d8e0"), "max_stack: Some(1)"),
]

SENTRY_STAFFS = [
    ("ember_tower_staff", "Cajado da Torre de Brasa", "ember_gem", pal("#3a0804", "#8a1a08", "#d04a10", "#ff9a30", "#fff0a0"), "golden"),
    ("frost_crystal_staff", "Cajado do Cristal de Gelo", "frost_gem", pal("#1a3a5a", "#3a6a9a", "#6aa8d8", "#b0e0ff", "#f0fcff"), "iron"),
    ("storm_totem_staff", "Cajado do Totem da Tempestade", "storm_gem", pal("#2a2a4a", "#4a4a8a", "#8080c8", "#c0c0f0", "#f8f8b0"), "copper"),
    ("ballista_cane", "Bengala da Balista", "ballista_gem", pal("#2a1a0a", "#5a3a1a", "#8a5a30", "#b88a50", "#f0f0f8"), "iron"),
]

DYES = [
    # name, display, liquid colour, how to make it (shapeless, with a glass bottle)
    ("red_tint", "Tintura Vermelha", [200, 40, 40], ["red_dye"]),
    ("orange_tint", "Tintura Laranja", [230, 120, 30], ["orange_dye"]),
    ("yellow_tint", "Tintura Amarela", [230, 200, 50], ["yellow_dye"]),
    ("lime_tint", "Tintura Verde-Limão", [120, 210, 50], ["lime_dye"]),
    ("green_tint", "Tintura Verde", [40, 150, 60], ["green_dye"]),
    ("cyan_tint", "Tintura Ciano", [40, 180, 190], ["cyan_dye"]),
    ("blue_tint", "Tintura Azul", [50, 80, 210], ["blue_dye"]),
    ("purple_tint", "Tintura Roxa", [130, 50, 190], ["purple_dye"]),
    ("pink_tint", "Tintura Rosa", [230, 110, 170], ["pink_dye"]),
    ("black_tint", "Tintura Preta", [40, 40, 46], ["black_dye"]),
    ("silver_tint", "Tintura Prateada", [200, 204, 214], ["light_gray_dye"]),
    ("brown_tint", "Tintura Marrom", [120, 76, 40], ["brown_dye"]),
    ("bright_blue_tint", "Tintura Azul Brilhante", [90, 150, 255], ["blue_dye", "glowstone_dust"]),
    ("bright_green_tint", "Tintura Verde Brilhante", [90, 255, 120], ["lime_dye", "glowstone_dust"]),
    ("rainbow_tint", "Tintura Arco-Íris", [220, 120, 200], ["red_dye", "yellow_dye", "blue_dye", "green_dye"]),
    ("flame_tint", "Tintura de Chamas", [250, 110, 20], ["orange_dye", "blaze_powder"]),
]

OUTFITS = [
    # material, display word, palette, wool
    ("tuxedo", "de Gala", pal("#121216", "#22222a", "#34343e", "#4c4c58", "#6a6a78"), "black_wool"),
    ("pirate", "de Pirata", pal("#3a0a08", "#6a1a14", "#962a20", "#b84030", "#d8604a"), "red_wool"),
    ("explorer", "de Explorador", pal("#3a2810", "#5e4420", "#80603a", "#a8844e", "#c8a46a"), "brown_wool"),
    ("royal", "Real", pal("#22103a", "#3c1e60", "#5a2e8a", "#7c48b0", "#a070d0"), "purple_wool"),
]
PIECES = [("helmet", "Chapéu"), ("chestplate", "Casaco"), ("leggings", "Calças"), ("boots", "Sapatos")]
PIECE_PATTERNS = {"helmet": ["XXX", "X X"], "chestplate": ["X X", "XXX", "XXX"], "leggings": ["XXX", "X X", "X X"], "boots": ["X X", "X X"]}


def items():
    out = []
    for name, display, rows, cols in WINGS:
        out.append((name, display, (lambda r=rows, c=cols: wing_icon(r, c)), "max_stack: Some(1)"))
    for name, display, rows, p, extra in SIMPLE:
        out.append((name, display, (lambda r=rows, p=p: icon(r, p)), extra))
    for name, display, gem, p, metal in SENTRY_STAFFS:
        gi.WAND_GEMS[gem] = p

        def draw(g=gem, m=metal):
            return gi.staff(g, m)
        out.append((name, display, draw, "max_stack: Some(1)"))
    for name, display, color, _ in DYES:
        out.append((name, display, (lambda c=color: outline(f6.bottle(f6.BOTTLE, np.array(c, dtype=float)), 0.4)), "max_stack: Some(16)"))
    for mat, word, p, _ in OUTFITS:
        gi.ARMOR_MATERIALS[mat] = p
        for slot, (piece, pname) in enumerate(PIECES):
            out.append((f"{mat}_{piece}", f"{pname} {word}", (lambda m=mat, k=piece: gi.armor(m, k)), f"armor: Some((slot: {slot}, points: 0, durability: 400))"))
    return out


SHAPED = [
    (["F F", "FSF", "F F"], {"F": "feather", "S": "soul_of_flight"}, "harpy_wings"),
    (["D D", "DSD", "D D"], {"D": "shadow_scale", "S": "soul_of_flight"}, "demon_wings"),
    (["H H", "HSH", "H H"], {"H": "honeycomb", "S": "soul_of_flight"}, "bee_wings"),
    (["G G", "GSG", "P P"], {"G": "glowstone_dust", "P": "pink_dye", "S": "soul_of_flight"}, "fairy_wings"),
    (["L L", "LSL", "L L"], {"L": "lapis_lazuli", "S": "soul_of_flight"}, "butterfly_wings"),
    (["B B", "BSB", "B B"], {"B": "bone", "S": "soul_of_flight"}, "bone_wings"),
    (["V V", "VSV", "V V"], {"V": "vine", "S": "soul_of_flight"}, "leaf_wings"),
    (["I I", "ISI", "I I"], {"I": "blue_ice", "S": "soul_of_flight"}, "frozen_wings"),
    (["R R", "RSR", "R R"], {"R": "blaze_rod", "S": "soul_of_flight"}, "flame_wings"),
    (["FSF", "FWF", "F F"], {"F": "star_fragment", "S": "soul_of_flight", "W": "breaker_wings"}, "celestial_wings"),
    # Mounts.
    (["SSS", "SLS"], {"S": "slime_ball", "L": "saddle"}, "slime_saddle"),
    (["HHH", "G G"], {"H": "honeycomb", "G": "glass"}, "honeyed_goggles"),
    (["RRR", "RCR", "RRR"], {"R": "rabbit_hide", "C": "golden_carrot"}, "fuzzy_carrot"),
    (["SSS", "SAS", "SSS"], {"S": "soul_of_sight", "A": "golden_apple"}, "blessed_apple"),
    (["FFF", "FMF", "FFF"], {"F": "star_fragment", "M": "brown_mushroom"}, "cosmic_truffle"),
    # Sentry staffs.
    (["  M", " R ", "R  "], {"M": "magma_block", "R": "blaze_rod"}, "ember_tower_staff"),
    (["  M", " R ", "R  "], {"M": "blue_ice", "R": "stick"}, "frost_crystal_staff"),
    (["  M", " R ", "R  "], {"M": "copper_block", "R": "copper_ingot"}, "storm_totem_staff"),
    (["  M", " R ", "R  "], {"M": "crossbow", "R": "soul_of_might"}, "ballista_cane"),
]

SHAPELESS = [
    (["gold_nugget", "sweet_berries", "string"], "fox_bell", 1),
    (["cod", "ice", "snowball"], "frozen_fish", 1),
    (["gold_ingot", "oak_sapling"], "golden_acorn", 1),
    (["apple", "cactus"], "spiny_apple", 1),
    (["egg", "kelp", "slime_ball"], "axolotl_egg", 1),
    (["wheat_seeds", "bread", "feather"], "parrot_cracker", 1),
    (["glass_bottle", "glowstone_dust", "glow_berries"], "firefly_jar", 1),
    (["gold_nugget", "amethyst", "glowstone_dust"], "allay_bell", 1),
    (["soul_lantern", "soul_of_fright"], "wisp_lantern", 1),
]


def write_data(all_items):
    known = known_items() | {i[0] for i in all_items}
    lines = ["// Lote 11: wings, mounts, pets, vanity, dyes and sentries. Written by tools/gen_lote11.py.", "(", "    items: ["]
    for name, display, _, extra in all_items:
        x = f", {extra}" if extra else ""
        lines.append(f'        (name: "{name}", display_name: Some("{display}"){x}),')
    lines += ["    ],", ")", ""]
    (OUT / "lote11_items.ron").write_text("\n".join(lines))
    shaped = list(SHAPED)
    for mat, _, _, wool in OUTFITS:
        for piece, _ in PIECES:
            shaped.append((PIECE_PATTERNS[piece], {"X": wool}, f"{mat}_{piece}"))
    shapeless = list(SHAPELESS) + [(["glass_bottle"] + ings, name, 1) for name, _, _, ings in DYES]
    bad = []
    rl = ["// Lote 11 recipes. Written by tools/gen_lote11.py.", "(", "    shaped: ["]
    for pattern, key, result in shaped:
        used = set("".join(pattern)) - {" "}
        assert used == set(key), (result, used, set(key))
        bad += [(result, v) for v in list(key.values()) + [result] if v not in known]
        pat = ", ".join(f'"{r}"' for r in pattern)
        k = ", ".join(f"'{c}': \"{v}\"" for c, v in key.items())
        rl.append(f"        (pattern: [{pat}], key: {{{k}}}, result: \"{result}\"),")
    rl += ["    ],", "    shapeless: ["]
    for ings, result, count in shapeless:
        bad += [(result, v) for v in ings + [result] if v not in known]
        ing = ", ".join(f'"{i}"' for i in ings)
        c = f", count: {count}" if count != 1 else ""
        rl.append(f"        (ingredients: [{ing}], result: \"{result}\"{c}),")
    rl += ["    ],", ")", ""]
    if bad:
        raise SystemExit(f"unknown items in recipes: {bad}")
    (OUT / "lote11_recipes.ron").write_text("\n".join(rl))


def main():
    all_items = items()
    write_data(all_items)
    imgs = []
    for name, _, draw, _ in all_items:
        im = gt.to_image(draw())
        im.save(os.path.join(gi.ITEM_DIR, name + ".png"))
        imgs.append(im)
    print(f"{len(all_items)} items, {len(SHAPED) + len(SHAPELESS) + len(DYES) + 16} recipes")
    print(",".join(i[0] for i in all_items))
    if len(sys.argv) > 1:
        gt.contact_sheet(imgs, scale=6, gap=3, cols=12).save(sys.argv[1])


if __name__ == "__main__":
    main()
