#!/usr/bin/env python3
"""Writes the data files of the Terraria side (Etapa 5) into packs/vanilla/etapa5/:
blocks.ron, items.ron, recipes.ron and mobs.ron. The game joins them onto the base lists
(crates/game/src/terraria.rs), so the base files stay as they are.

Most of it is repetitive (a set of tools and armour per metal), which is why it is written by
a script. Mob models are copied from base mobs in packs/vanilla/mobs.ron where a new mob is a
recoloured cousin of one (goblins from the player, the colossus from the Forest Guardian).

Usage: python3 tools/gen_etapa5_data.py
"""

import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "packs", "vanilla", "etapa5")

# ---------------------------------------------------------------------------------------------
# Metals of hard mode and the end game: (name, display, tool tier, dig speed, sword damage,
# durability, armour points (head, chest, legs, feet), smelted from an ore?)
METALS = [
    ("cobalt", "Cobalto", 5, 10.0, 8.0, 1800, (3, 7, 6, 3), True),
    ("palladium", "Paládio", 5, 10.5, 8.0, 1900, (3, 7, 6, 3), True),
    ("mythril", "Mitril", 6, 12.0, 9.0, 2300, (3, 8, 6, 3), True),
    ("orichalcum", "Oricalco", 6, 12.5, 9.0, 2400, (3, 8, 6, 3), True),
    ("adamantite", "Adamantita", 7, 14.0, 10.0, 2800, (4, 8, 7, 4), True),
    ("titanium", "Titânio", 7, 14.5, 10.0, 2900, (4, 8, 7, 4), True),
    ("radiant", "Radiante", 7, 15.0, 11.0, 3000, (4, 9, 7, 4), False),
    ("verdite", "Verdita", 8, 17.0, 12.0, 3400, (4, 9, 8, 4), True),
    ("lumite", "Lumita", 9, 20.0, 14.0, 4000, (5, 10, 8, 5), False),
]
# Ores: (metal, block tier needed, rare-pair partner) — tier 4 is diamond.
ORE_TIER = {"cobalt": 4, "palladium": 4, "mythril": 5, "orichalcum": 5, "adamantite": 6, "titanium": 6, "verdite": 7}
FEM = {"mythril", "orichalcum", "adamantite", "radiant", "verdite", "lumite"}

TOOLS = [("pickaxe", "Picareta"), ("axe", "Machado"), ("sword", "Espada")]
ARMOR = [("helmet", "Capacete"), ("chestplate", "Peitoral"), ("leggings", "Calças"), ("boots", "Botas")]


def ron_str(s):
    return '"' + s.replace('"', '\\"') + '"'


def item(name, display, extra=""):
    return f"    (name: {ron_str(name)}, display_name: Some({ron_str(display)}){extra}),"


blocks = []
items = []
shaped = []
shapeless = []
smelting = []

# ---------------------------------------------------------------------------------------------
# Ores, bars, tools and armour.
for metal, disp, tier, speed, dmg, dur, pts, has_ore in METALS:
    de = "da" if metal in FEM else "de"
    if has_ore:
        blocks.append(f"    (name: \"{metal}_ore\", display_name: Some(\"Minério de {disp}\"), hardness: {3.0 + ORE_TIER[metal] * 0.5}, tool: Some(Pickaxe), tier: {ORE_TIER[metal]}, drops: Some([(item: \"raw_{metal}\")])),")
        items.append(item(f"raw_{metal}", f"{disp} Bruto" if metal not in FEM else f"{disp} Bruta"))
        smelting.append(f"        (input: \"raw_{metal}\", result: \"{metal}_ingot\", xp: 1.2),")
    items.append(item(f"{metal}_ingot", f"Barra de {disp}"))
    blocks.append(f"    (name: \"{metal}_block\", display_name: Some(\"Bloco de {disp}\"), hardness: 5.0, tool: Some(Pickaxe), tier: 3),")
    shaped.append(f"        (pattern: [\"XXX\", \"XXX\", \"XXX\"], key: {{'X': \"{metal}_ingot\"}}, result: \"{metal}_block\"),")
    shapeless.append(f"        (ingredients: [\"{metal}_block\"], result: \"{metal}_ingot\", count: 9),")
    for kind, kd in TOOLS:
        d = {"pickaxe": dmg - 3.0, "axe": dmg + 1.5, "sword": dmg}[kind]
        kname = {"pickaxe": "Pickaxe", "axe": "Axe", "sword": "Sword"}[kind]
        items.append(item(f"{metal}_{kind}", f"{kd} {de} {disp}", f", tool: Some((kind: {kname}, tier: {tier}, speed: {speed}, damage: {d}, durability: {dur}))"))
    shaped.append(f"        (pattern: [\"XXX\", \" / \", \" / \"], key: {{'X': \"{metal}_ingot\", '/': \"stick\"}}, result: \"{metal}_pickaxe\"),")
    shaped.append(f"        (pattern: [\"XX\", \"X/\", \" /\"], key: {{'X': \"{metal}_ingot\", '/': \"stick\"}}, result: \"{metal}_axe\"),")
    shaped.append(f"        (pattern: [\"X\", \"X\", \"/\"], key: {{'X': \"{metal}_ingot\", '/': \"stick\"}}, result: \"{metal}_sword\"),")
    adur = [int(dur * k) for k in (0.22, 0.32, 0.30, 0.26)]
    for (kind, kd), p, ad, slot in zip(ARMOR, pts, adur, range(4)):
        items.append(item(f"{metal}_{kind}", f"{kd} {de} {disp}", f", armor: Some((slot: {slot}, points: {p}, durability: {ad}))"))
    shaped.append(f"        (pattern: [\"XXX\", \"X X\"], key: {{'X': \"{metal}_ingot\"}}, result: \"{metal}_helmet\"),")
    shaped.append(f"        (pattern: [\"X X\", \"XXX\", \"XXX\"], key: {{'X': \"{metal}_ingot\"}}, result: \"{metal}_chestplate\"),")
    shaped.append(f"        (pattern: [\"XXX\", \"X X\", \"X X\"], key: {{'X': \"{metal}_ingot\"}}, result: \"{metal}_leggings\"),")
    shaped.append(f"        (pattern: [\"X X\", \"X X\"], key: {{'X': \"{metal}_ingot\"}}, result: \"{metal}_boots\"),")

# A station: the reforge table rerolls an item's modifier for emeralds.
blocks.append('    (name: "reforge_table", display_name: Some("Mesa de Reforja"), hardness: 3.0, tool: Some(Axe), textures: (top: Some("reforge_table_top"), bottom: Some("oak_planks"), side: Some("reforge_table_side"))),')
shaped.append('        (pattern: ["III", "IGI", "PPP"], key: {\'I\': "iron_ingot", \'G\': "gold_ingot", \'P\': "#planks"}, result: "reforge_table"),')

# ---------------------------------------------------------------------------------------------
# Accessories (one per stack).
ACC = [
    ("running_boots", "Botas de Corrida"), ("cloud_bottle", "Nuvem na Garrafa"), ("lucky_horseshoe", "Ferradura da Sorte"),
    ("wind_cape", "Capa do Vento"), ("shiny_balloon", "Balão Brilhante"), ("feather_wings", "Asas de Pena"),
    ("angel_wings", "Asas Radiantes"), ("mana_ring", "Anel de Mana"), ("star_amulet", "Amuleto Estelar"),
    ("warrior_emblem", "Emblema do Guerreiro"), ("arcane_emblem", "Emblema Arcano"), ("obsidian_charm", "Amuleto de Obsidiana"),
    ("regen_band", "Pulseira da Regeneração"), ("depth_meter", "Medidor de Profundidade"), ("climbing_claws", "Garras de Escalada"),
    ("frost_spark_boots", "Botas do Relâmpago"), ("guardian_shield", "Escudo do Guardião"),
]
items.append("    // Accessories: worn in the five slots beside the inventory.")
for n, d in ACC:
    items.append(item(n, d, ", max_stack: Some(1)"))
shaped += [
    '        (pattern: ["F F", "L L"], key: {\'F\': "feather", \'L\': "leather"}, result: "running_boots"),',
    '        (pattern: ["FFF", "FGF", "FFF"], key: {\'F\': "feather", \'G\': "glass"}, result: "cloud_bottle"),',
    '        (pattern: ["I I", "I I", "GGG"], key: {\'I\': "iron_ingot", \'G\': "gold_ingot"}, result: "lucky_horseshoe"),',
    '        (pattern: ["WWW", "WFW", "W W"], key: {\'W\': "#wool", \'F\': "feather"}, result: "wind_cape"),',
    '        (pattern: ["WWW", "WWW", " S "], key: {\'W\': "white_wool", \'S\': "string"}, result: "shiny_balloon"),',
    '        (pattern: ["F F", "FMF", "F F"], key: {\'F\': "feather", \'M\': "mythril_ingot"}, result: "feather_wings"),',
    '        (pattern: ["R R", "RWR", "R R"], key: {\'R\': "radiant_ingot", \'W\': "feather_wings"}, result: "angel_wings"),',
    '        (pattern: [" S ", "G G", " G "], key: {\'S\': "sapphire", \'G\': "gold_ingot"}, result: "mana_ring"),',
    '        (pattern: [" S ", "S S", " M "], key: {\'S\': "string", \'M\': "mana_crystal"}, result: "star_amulet"),',
    '        (pattern: ["OOO", "ORO", "OOO"], key: {\'O\': "obsidian", \'R\': "ruby"}, result: "obsidian_charm"),',
    '        (pattern: [" G ", "GCG", " G "], key: {\'G\': "gold_ingot", \'C\': "life_crystal"}, result: "regen_band"),',
    '        (pattern: [" C ", "CRC", " C "], key: {\'C\': "copper_ingot", \'R\': "redstone"}, result: "depth_meter"),',
    '        (pattern: ["I I", "I I"], key: {\'I\': "shadow_scale"}, result: "climbing_claws"),',
    '        (pattern: ["T T", "BRB"], key: {\'T\': "topaz", \'B\': "running_boots", \'R\': "radiant_ingot"}, result: "frost_spark_boots"),',
    '        (pattern: ["III", "ISI", " I "], key: {\'I\': "cobalt_ingot", \'S\': "soul_of_might"}, result: "guardian_shield"),',
]

# Grappling hooks.
HOOKS = [("grappling_hook", "Gancho"), ("amethyst_hook", "Gancho de Ametista"), ("topaz_hook", "Gancho de Topázio"),
         ("sapphire_hook", "Gancho de Safira"), ("ruby_hook", "Gancho de Rubi"), ("verdite_hook", "Gancho de Verdita")]
items.append("    // Grappling hooks: right click to throw, it pulls you to where it bites.")
for n, d in HOOKS:
    items.append(item(n, d, ", max_stack: Some(1)"))
shaped.append('        (pattern: ["  I", " S ", "S  "], key: {\'I\': "iron_ingot", \'S\': "string"}, result: "grappling_hook"),')
for g in ("amethyst", "topaz", "sapphire", "ruby"):
    shapeless.append(f"        (ingredients: [\"grappling_hook\", \"{g}\", \"{g}\", \"{g}\"], result: \"{g}_hook\"),")
shapeless.append('        (ingredients: ["grappling_hook", "verdite_ingot", "verdite_ingot", "verdite_ingot"], result: "verdite_hook"),')

# ---------------------------------------------------------------------------------------------
# Magic: parchment, blank book, the glyph grimoire and its glyphs, and new spells.
items.append("    // Glyph magic (Ars Nouveau idea): learn glyphs, write your own spells in the grimoire.")
items.append(item("parchment", "Pergaminho"))
items.append(item("blank_book", "Livro em Branco"))
items.append(item("glyph_book", "Grimório de Glifos", ", max_stack: Some(1)"))
shapeless.append('        (ingredients: ["leather"], result: "parchment", count: 3),')
shapeless.append('        (ingredients: ["parchment", "parchment", "parchment", "leather"], result: "blank_book"),')
shapeless.append('        (ingredients: ["blank_book", "lapis_lazuli", "lapis_lazuli", "amethyst"], result: "glyph_book"),')
GLYPHS = [
    # forms
    ("projectile", "Projétil", ["parchment", "arrow_or_feather", "lapis_lazuli"]),
    ("touch", "Toque", ["parchment", "stick", "lapis_lazuli"]),
    ("self", "Si Mesmo", ["parchment", "glass", "lapis_lazuli"]),
    # effects
    ("harm", "Dano", ["parchment", "iron_ingot", "lapis_lazuli"]),
    ("ignite", "Fogo", ["parchment", "coal", "ruby"]),
    ("freeze", "Gelo", ["parchment", "ice", "sapphire"]),
    ("heal", "Cura", ["parchment", "apple", "emerald"]),
    ("break", "Quebrar", ["parchment", "iron_pickaxe", "lapis_lazuli"]),
    ("knockback", "Empurrão", ["parchment", "piston", "lapis_lazuli"]),
    ("pull", "Puxão", ["parchment", "string", "lapis_lazuli"]),
    ("leap", "Salto", ["parchment", "feather", "lapis_lazuli"]),
    ("light", "Luz", ["parchment", "torch", "topaz"]),
    ("blink", "Teleporte", ["parchment", "ender_pearl", "amethyst"]),
    ("explode", "Explosão", ["parchment", "tnt", "ruby"]),
    ("harvest", "Colheita", ["parchment", "wheat_seeds", "emerald"]),
    # augments
    ("amplify", "Amplificar", ["parchment", "diamond", "lapis_lazuli"]),
    ("aoe", "Área", ["parchment", "gunpowder", "lapis_lazuli"]),
    ("split", "Dividir", ["parchment", "quartz", "lapis_lazuli"]),
    ("accelerate", "Acelerar", ["parchment", "sugar_or_redstone", "lapis_lazuli"]),
    ("extend", "Prolongar", ["parchment", "glowstone", "lapis_lazuli"]),
]
for n, d, ing in GLYPHS:
    items.append(item(f"glyph_{n}", f"Glifo: {d}"))
    ing = ["feather" if i == "arrow_or_feather" else "redstone" if i == "sugar_or_redstone" else i for i in ing]
    shapeless.append(f"        (ingredients: [{', '.join(ron_str(i) for i in ing)}], result: \"glyph_{n}\"),")

items.append("    // Spells from the Iron's Spells idea: scrolls and books with one spell each.")
SPELLS = [
    ("blink_scroll", "Pergaminho do Salto Arcano", ["parchment", "ender_pearl", "lapis_lazuli"]),
    ("heal_scroll", "Pergaminho da Cura", ["parchment", "apple", "gold_nugget"]),
    ("ward_tome", "Livro da Proteção", ["blank_book", "iron_ingot", "iron_ingot", "sapphire"]),
    ("chain_tome", "Livro do Raio em Cadeia", ["blank_book", "topaz", "topaz", "redstone"]),
    ("meteor_staff", "Cajado do Meteoro", ["fire_staff", "obsidian", "radiant_ingot"]),
    ("spirit_staff", "Cajado dos Espíritos", ["stick", "stick", "soul_of_fright", "amethyst"]),
    ("ice_spike_staff", "Cajado dos Espinhos de Gelo", ["frost_staff", "ice", "ice", "mythril_ingot"]),
    ("void_tome", "Livro do Vazio", ["blank_book", "ender_eye", "amethyst", "amethyst"]),
    ("star_tome", "Livro das Estrelas", ["blank_book", "lumite_ingot", "mana_crystal"]),
]
for n, d, ing in SPELLS:
    items.append(item(n, d, ", max_stack: Some(1)"))
    shapeless.append(f"        (ingredients: [{', '.join(ron_str(i) for i in ing)}], result: \"{n}\"),")

# ---------------------------------------------------------------------------------------------
# Bosses: summon items and what they leave behind.
items.append("    // Boss summons and drops.")
SUMMON_ITEMS = [
    ("worm_bait", "Isca de Verme", ["rotten_flesh", "rotten_flesh", "rotten_flesh", "dirt", "bone"]),
    ("cursed_skull", "Crânio Amaldiçoado", ["bone", "bone", "bone", "bone", "shadow_scale", "lapis_lazuli"]),
    ("breaker_idol", "Ídolo do Quebra-Mundos", ["obsidian", "obsidian", "spectral_bone", "spectral_bone", "gold_ingot"]),
    ("gear_eye", "Olho de Engrenagem", ["iron_ingot", "iron_ingot", "iron_ingot", "redstone", "redstone", "glass"]),
    ("iron_worm", "Verme de Ferro", ["iron_ingot", "iron_ingot", "iron_ingot", "redstone", "redstone", "rotten_flesh"]),
    ("steel_skull", "Crânio de Aço", ["iron_ingot", "iron_ingot", "iron_ingot", "redstone", "redstone", "bone"]),
    ("queen_bulb", "Bulbo da Rainha", ["soul_of_sight", "soul_of_might", "soul_of_fright", "jungle_sapling", "jungle_leaves"]),
    ("celestial_sigil", "Selo Celeste", ["verdite_ingot", "verdite_ingot", "verdite_ingot", "verdite_ingot", "ender_eye", "lapis_lazuli"]),
    ("goblin_battle_standard", "Estandarte Goblin", ["stick", "leather", "leather", "rotten_flesh"]),
    ("bloody_tear", "Lágrima de Sangue", ["rotten_flesh", "rotten_flesh", "redstone", "ruby"]),
]
for n, d, ing in SUMMON_ITEMS:
    items.append(item(n, d))
    shapeless.append(f"        (ingredients: [{', '.join(ron_str(i) for i in ing)}], result: \"{n}\"),")
for n, d in [("shadow_scale", "Escama Sombria"), ("spectral_bone", "Osso Espectral"), ("breaker_core", "Núcleo do Mundo"),
             ("soul_of_sight", "Alma da Visão"), ("soul_of_might", "Alma do Poder"), ("soul_of_fright", "Alma do Medo"),
             ("living_vine", "Vinha Viva"), ("star_fragment", "Fragmento Estelar")]:
    items.append(item(n, d))
items.append(item("worm_staff", "Cajado do Verme", ", max_stack: Some(1)"))
items.append(item("bone_whip", "Chicote de Ossos", ", tool: Some((kind: Sword, tier: 3, speed: 6.0, damage: 7.5, durability: 600))"))
items.append(item("breaker_hammer", "Martelo do Quebra-Mundos", ", tool: Some((kind: Pickaxe, tier: 5, speed: 11.0, damage: 9.0, durability: 2000))"))
items.append(item("vine_blade", "Lâmina das Vinhas", ", tool: Some((kind: Sword, tier: 8, speed: 12.0, damage: 13.0, durability: 3000))"))
items.append(item("celestial_blade", "Lâmina Celeste", ", tool: Some((kind: Sword, tier: 9, speed: 15.0, damage: 17.0, durability: 4000))"))
shapeless.append('        (ingredients: ["star_fragment", "star_fragment", "star_fragment", "star_fragment", "lumite_ingot"], result: "star_tome"),')

# ---------------------------------------------------------------------------------------------
# Mobs. New box models for worms, eyes and skulls; the rest copy a base model.
MOBS_RON = open(os.path.join(ROOT, "packs", "vanilla", "mobs.ron"), encoding="utf-8").read()


def base_model(name):
    """parts: [...] and layers: [...] of a base mob, as text."""
    m = re.search(r'name:\s*"' + name + r'".*?(parts:\s*\[.*?\],)\s*(layers:\s*\[.*?\],)?', MOBS_RON, re.S)
    assert m, name
    return m.group(1) + ("\n        " + m.group(2) if m.group(2) else "")


WORM_HEAD = """parts: [
            (size: (14, 12, 16), origin: (-7, 0, -8), uv: (0, 0), anim: Head, pivot: (0, 6, 0)),
            (size: (3, 4, 6), origin: (-7, 1, 8), uv: (0, 28), anim: Head, pivot: (0, 6, 0)),
            (size: (3, 4, 6), origin: (4, 1, 8), uv: (18, 28), anim: Head, pivot: (0, 6, 0)),
        ],
        layers: [(part: 0, inflate: 0.6)],"""
WORM_BODY = """parts: [
            (size: (12, 12, 12), origin: (-6, 0, -6), uv: (0, 0), anim: Head, pivot: (0, 6, 0)),
            (size: (2, 4, 2), origin: (-1, 12, -1), uv: (0, 24), anim: Head, pivot: (0, 6, 0)),
        ],
        layers: [(part: 0, inflate: 0.5)],"""
WORM_TAIL = """parts: [
            (size: (8, 8, 12), origin: (-4, 2, -6), uv: (0, 0), anim: Head, pivot: (0, 6, 0)),
            (size: (4, 4, 6), origin: (-2, 4, -12), uv: (0, 20), anim: Head, pivot: (0, 6, 0)),
        ],
        layers: [(part: 0, inflate: 0.5)],"""
EYE = """parts: [
            (size: (16, 16, 16), origin: (-8, 0, -8), uv: (0, 0), anim: Head, pivot: (0, 8, 0)),
            (size: (2, 2, 8), origin: (-5, 7, -16), uv: (0, 32), anim: Head, pivot: (0, 8, 0)),
            (size: (2, 2, 8), origin: (3, 9, -16), uv: (0, 32), anim: Head, pivot: (0, 8, 0)),
            (size: (2, 2, 8), origin: (-1, 3, -16), uv: (0, 32), anim: Head, pivot: (0, 8, 0)),
        ],
        layers: [(part: 0, inflate: 0.5)],"""
SKULL = """parts: [
            (size: (16, 14, 16), origin: (-8, 10, -8), uv: (0, 0), anim: Head, pivot: (0, 14, 0)),
            (size: (14, 4, 14), origin: (-7, 6, -6), uv: (0, 30), anim: Head, pivot: (0, 14, 0)),
            (size: (6, 8, 4), origin: (-20, 4, -2), uv: (0, 48), anim: Arm(0), pivot: (-17, 12, 0)),
            (size: (6, 8, 4), origin: (14, 4, -2), uv: (0, 48), anim: Arm(1), pivot: (17, 12, 0)),
        ],
        layers: [(part: 0, inflate: 0.6), (part: 2, inflate: 0.4)],"""

PLAYER = base_model("player")
ZOMBIE = base_model("zombie")
GUARDIAN = base_model("forest_guardian")
SKELETON = base_model("skeleton")


def mob(name, display, model, kind="Hostile", health=20, speed=2.0, w=0.6, h=1.95, attack=3, scale=1.0, boss=False,
        drops="", xp=5, spawn="Never", extra=""):
    bits = [f"name: \"{name}\", display_name: \"{display}\", kind: {kind}, health: {health}, speed: {speed},"]
    if drops:
        bits.append(f"drops: [{drops}], xp: {xp},")
    else:
        bits.append(f"xp: {xp},")
    tail = f"width: {w}, height: {h}, spawn: {spawn}, attack: {attack}"
    if scale != 1.0:
        tail += f", scale: {scale}"
    if boss:
        tail += ", boss: true"
    tail += extra + ","
    bits.append(tail)
    body = "\n        ".join(bits)
    return f"    (\n        {body}\n        {model}\n    ),"


mobs = []
add = mobs.append
add("    // Deep Worm: a cave boss whose body follows its head through rock.")
add(mob("deep_worm", "Verme das Profundezas", WORM_HEAD, health=420, speed=0.1, w=1.3, h=1.2, attack=5, scale=1.6, boss=True,
        drops='(item: "shadow_scale", min: 6, max: 12), (item: "worm_staff", chance: 0.5), (item: "life_crystal"), (item: "emerald", min: 3, max: 6)', xp=120))
add(mob("deep_worm_body", "Verme das Profundezas", WORM_BODY, health=9999, speed=0.1, w=1.2, h=1.2, attack=3, scale=1.6, xp=0))
add(mob("deep_worm_tail", "Verme das Profundezas", WORM_TAIL, health=9999, speed=0.1, w=0.9, h=0.9, attack=2, scale=1.6, xp=0))
add("    // The Bone King: a huge flying skull with two hands, woken at night.")
add(mob("bone_king", "Rei dos Ossos", SKULL, health=600, speed=0.1, w=1.8, h=2.0, attack=6, scale=1.8, boss=True,
        drops='(item: "spectral_bone", min: 4, max: 8), (item: "bone_whip", chance: 0.5), (item: "obsidian_charm", chance: 0.4), (item: "emerald", min: 4, max: 8)', xp=160))
add("    // The World Breaker: a colossus of rock and lava from the Nether. Beating it changes the world.")
add(mob("world_breaker", "Quebra-Mundos", GUARDIAN, health=900, speed=1.6, w=2.4, h=6.5, attack=10, scale=3.0, boss=True,
        drops='(item: "breaker_core"), (item: "breaker_hammer"), (item: "warrior_emblem", chance: 0.5), (item: "arcane_emblem", chance: 0.5), (item: "emerald", min: 8, max: 14)', xp=255))
add("    // Hard mode: three machines.")
add(mob("mech_eye", "Olho Mecânico", EYE, health=1100, speed=0.1, w=1.6, h=1.6, attack=8, scale=1.7, boss=True,
        drops='(item: "soul_of_sight", min: 8, max: 14), (item: "radiant_ingot", min: 10, max: 18), (item: "emerald", min: 6, max: 10)', xp=255))
add(mob("mech_worm", "Serpente de Aço", WORM_HEAD, health=1500, speed=0.1, w=1.5, h=1.4, attack=9, scale=2.0, boss=True,
        drops='(item: "soul_of_might", min: 8, max: 14), (item: "radiant_ingot", min: 10, max: 18), (item: "emerald", min: 6, max: 10)', xp=255))
add(mob("mech_worm_body", "Serpente de Aço", WORM_BODY, health=9999, speed=0.1, w=1.5, h=1.5, attack=6, scale=2.0, xp=0))
add(mob("mech_worm_tail", "Serpente de Aço", WORM_TAIL, health=9999, speed=0.1, w=1.1, h=1.1, attack=4, scale=2.0, xp=0))
add(mob("mech_skull", "Crânio de Aço", SKULL, health=1300, speed=0.1, w=1.8, h=2.0, attack=9, scale=1.9, boss=True,
        drops='(item: "soul_of_fright", min: 8, max: 14), (item: "radiant_ingot", min: 10, max: 18), (item: "emerald", min: 6, max: 10)', xp=255))
add("    // The Vine Queen: a flower of the deep jungle, after the machines.")
add(mob("vine_queen", "Rainha das Vinhas", GUARDIAN, health=2000, speed=2.0, w=2.0, h=5.6, attack=11, scale=2.6, boss=True,
        drops='(item: "living_vine", min: 8, max: 14), (item: "vine_blade", chance: 0.5), (item: "verdite_ingot", min: 6, max: 10), (item: "emerald", min: 8, max: 14)', xp=255))
add("    // The Celestial Herald: the last boss, from the night sky.")
add(mob("celestial_herald", "Arauto Celeste", SKELETON, health=3200, speed=0.1, w=1.4, h=5.0, attack=12, scale=2.6, boss=True,
        drops='(item: "lumite_ingot", min: 14, max: 24), (item: "star_fragment", min: 6, max: 10), (item: "celestial_blade", chance: 0.5), (item: "emerald", min: 12, max: 20)', xp=255))
add("    // Small fry: demon eyes fly at night (more under a blood moon), blood zombies, goblins.")
add(mob("demon_eye", "Olho Demoníaco", EYE, health=12, speed=0.1, w=0.7, h=0.7, attack=3, scale=0.7,
        drops='(item: "string", chance: 0.3), (item: "redstone", chance: 0.3)', xp=5))
add(mob("blood_zombie", "Zumbi Sangrento", ZOMBIE, health=26, speed=2.6, attack=5,
        drops='(item: "rotten_flesh", min: 1, max: 3), (item: "bloody_tear", chance: 0.05), (item: "emerald", chance: 0.2)', xp=8, extra=", burns_in_daylight: true"))
add(mob("goblin_warrior", "Goblin Guerreiro", PLAYER, health=30, speed=2.7, w=0.6, h=1.7, attack=5,
        drops='(item: "leather", chance: 0.4), (item: "emerald", chance: 0.4)', xp=8))
add(mob("goblin_archer", "Goblin Arqueiro", PLAYER, health=22, speed=2.4, w=0.6, h=1.7, attack=4,
        drops='(item: "string", chance: 0.5), (item: "emerald", chance: 0.4)', xp=8, extra=", ranged: true"))
add(mob("goblin_sorcerer", "Feiticeiro Goblin", PLAYER, health=24, speed=2.2, w=0.6, h=1.7, attack=5,
        drops='(item: "lapis_lazuli", min: 1, max: 3), (item: "emerald", chance: 0.5)', xp=10, extra=", ranged: true"))
add("    // New town folk: the goblin tinkerer (after an invasion), the mechanic, the sorceress.")
for n, d in [("npc_tinkerer", "Engenhoqueiro Goblin"), ("npc_mechanic", "Mecânica"), ("npc_sorceress", "Feiticeira")]:
    add(mob(n, d, PLAYER, kind="Passive", health=40, speed=2.0, w=0.6, h=1.95, attack=0, xp=0, extra=", town: true"))


def write(path, head, body):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(head + "\n".join(body) + "\n])\n")


HEAD = "// Etapa 5 (the Terraria side): written by tools/gen_etapa5_data.py, joined onto the base list.\n"
write(os.path.join(OUT, "blocks.ron"), HEAD + "(blocks: [\n", blocks)
write(os.path.join(OUT, "items.ron"), HEAD + "(items: [\n", items)
write(os.path.join(OUT, "mobs.ron"), HEAD + "(mobs: [\n", mobs)
with open(os.path.join(OUT, "recipes.ron"), "w", encoding="utf-8") as f:
    f.write(HEAD + "(\n    tags: {\n")
    f.write('        "planks": ["oak_planks", "birch_planks", "spruce_planks", "jungle_planks", "acacia_planks", "dark_oak_planks", "cherry_planks"],\n')
    f.write('        "wool": ["white_wool", "orange_wool", "magenta_wool", "light_blue_wool", "yellow_wool", "lime_wool", "pink_wool", "gray_wool", "light_gray_wool", "cyan_wool", "purple_wool", "blue_wool", "brown_wool", "green_wool", "red_wool", "black_wool"],\n')
    f.write("    },\n    shaped: [\n" + "\n".join(shaped) + "\n    ],\n")
    f.write("    shapeless: [\n" + "\n".join(shapeless) + "\n    ],\n")
    f.write("    smelting: [\n" + "\n".join(smelting) + "\n    ],\n)\n")
print(f"{len(blocks)} blocks, {len(items)} item lines, {len(shaped) + len(shapeless) + len(smelting)} recipes, {len([m for m in mobs if 'name:' in m])} mobs")
