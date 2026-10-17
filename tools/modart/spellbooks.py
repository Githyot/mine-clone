"""Iron's Spells 'n Spellbooks' idea: scrolls sealed in their school's colour, ink, the
Archmage's essence and seal, four spellbooks, eight sets of wizard robes (a pointed hat for the
head) and the inscription table. Our own 16x16 pixels."""

from moddata.spellbooks import ROBES, SPELLS

from .lib import BLOCKS, I, ITEMS, T, from_map, hexrgb, outline, pal, put

# Each school's colours, dark to light (same order as SCHOOLS in mods/spellbooks.rs).
SCHOOL = [
    ("#8a2a0a", "#e8641a", "#ffc060"),  # fire
    ("#1e5a8a", "#6ab4ea", "#d8f4ff"),  # frost
    ("#8a6a0a", "#e8c830", "#fff8b0"),  # spark
    ("#2a348a", "#5a6ae0", "#c0ccff"),  # arcane
    ("#1e5a20", "#3aa040", "#b0f080"),  # nature
    ("#2a3060", "#6a7ab8", "#dae0ff"),  # storm
    ("#8a6a20", "#e8d070", "#fffbe8"),  # holy
    ("#4a1a6a", "#9a3ac8", "#eab0ff"),  # ender
    ("#4a0610", "#a01828", "#f05060"),  # blood
    ("#06323a", "#0e7a80", "#60f0e0"),  # eldritch
    ("#5a4a2a", "#a08a50", "#f0e0a8"),  # evocation
]
SPELL_SCHOOL = {
    "fireball": 0, "fire_breath": 0, "ice_spike": 1, "frost_nova": 1, "lightning_lance": 2, "arc_beam": 2,
    "magic_missile": 3, "arcane_barrier": 3, "thorn_roots": 4, "nature_blessing": 4, "storm_call": 5, "gust": 5,
    "heal": 6, "sacred_light": 6, "fortitude": 6, "teleport": 7, "void_rift": 7, "ender_orb": 7,
    "life_drain": 8, "raise_dead": 8, "blood_slash": 8, "abyss_ray": 9, "abyss_scream": 9,
    "fangs": 10, "summon_wolves": 10, "haste": 10,
}
ROBE_SCHOOL = {"pyromancer": 0, "cryomancer": 1, "electromancer": 2, "priest": 6, "voidwalker": 7, "bloodmage": 8,
               "cultist": 9, "archevoker": 10}

PAPER = {"w": hexrgb("#7a5a2a"), "W": hexrgb("#b8925a"), "H": hexrgb("#e2c98e"),
         "p": hexrgb("#cdbb8e"), "P": hexrgb("#efe2bc")}

SCROLL = [
    "................",
    "..wWWWWWWWWWWw..",
    ".wWHHHHHHHHHHWw.",
    "..wWWWWWWWWWWw..",
    "...pPPPPPPPPp...",
    "...pPrrPrPrrp...",
    "...pPPPPPPPPp...",
    "...pPrPrrrPPp...",
    "...pPPPPPPPPp...",
    "...pPrrPPrrPp...",
    "...pPPPPPPPPp...",
    "..wWWWWSSWWWWw..",
    ".wWHHHHSsSHHHWw.",
    "..wWWWWWSWWWWw..",
    "........S.......",
    "................",
]

# Rune rows differ a little per spell so the scrolls of one school don't look the same.
RUNES = [
    ["PrrPrPrr", "PrPrrrPP", "rrPPrrPP"],
    ["rPrrPrPP", "PPrrPrrP", "PrPPrrrP"],
    ["rrrPPrPr", "PrPrPPrr", "PPrrrPrP"],
]


def scroll(spell):
    dark, mid, light = (hexrgb(c) for c in SCHOOL[SPELL_SCHOOL[spell]])
    k = [s for s, *_ in SPELLS].index(spell) % len(RUNES)
    rows = list(SCROLL)
    for i, y in enumerate((5, 7, 9)):
        rows[y] = rows[y][:4] + RUNES[k][i] + rows[y][12:]
    cols = dict(PAPER, r=dark, S=mid, s=light)
    return outline(from_map(rows, cols))


for _s, *_ in SPELLS:
    ITEMS[f"scroll_{_s}"] = (lambda s=_s: scroll(s))


def blank_scroll():
    rows = [r.replace("r", "P").replace("S", "W").replace("s", "H") for r in SCROLL]
    rows[14] = "................"
    return outline(from_map(rows, PAPER))


ITEMS["blank_scroll"] = blank_scroll


def arcane_ink():
    rows = [
        "................",
        "................",
        "......kkkk......",
        "......cCCc......",
        ".......gg.......",
        ".......gg.......",
        ".....gBBBBg.....",
        "....gBBbBBBg....",
        "...gBBbBBBBBg...",
        "...gBBBBBBBBg...",
        "...gBBBBBBDBg...",
        "...gBBBBBDDBg...",
        "....gBBBBBBg....",
        ".....gggggg.....",
        "................",
        "................",
    ]
    cols = {"k": hexrgb("#5a3a1c"), "c": hexrgb("#8a6034"), "C": hexrgb("#b08850"), "g": hexrgb("#a8c8d8"),
            "B": hexrgb("#2a3cb0"), "b": hexrgb("#7a9aff"), "D": hexrgb("#16207a")}
    return outline(from_map(rows, cols))


ITEMS["arcane_ink"] = arcane_ink


def arcane_essence():
    rows = [
        "................",
        "................",
        ".......a........",
        "......aAa.......",
        ".....aAWAa......",
        "....aAWWWAa.....",
        "...aAAWWWAAa....",
        "..aAAAAWAAAAa...",
        "...aAAAAAAAa....",
        "....aAAAAAa.....",
        ".....aAAAa......",
        "......aAa.......",
        ".......a........",
        "................",
        "................",
        "................",
    ]
    cols = {"a": hexrgb("#4a1a8a"), "A": hexrgb("#9a5ae8"), "W": hexrgb("#f0e0ff")}
    img = from_map(rows, cols)
    for x, y in ((12, 3), (3, 11), (13, 10)):
        put(img, x, y, hexrgb("#d8b8ff"))
    return outline(img)


ITEMS["arcane_essence"] = arcane_essence


def corrupted_seal():
    rows = [
        "................",
        "................",
        ".....dddddd.....",
        "....dMMMMMMd....",
        "...dMMLLLLMMd...",
        "..dMMLMMMMLMMd..",
        "..dMLMMEEMMLMd..",
        "..dMLMEggEMLMd..",
        "..dMLMEggEMLMd..",
        "..dMLMMEEMMLMd..",
        "..dMMLMMMMLMMd..",
        "...dMMLLLLMMd...",
        "....dMMMMMMd....",
        ".....dddddd.....",
        "................",
        "................",
    ]
    cols = {"d": hexrgb("#2a0a3a"), "M": hexrgb("#5a1a7a"), "L": hexrgb("#8a3ab0"), "E": hexrgb("#0e7a80"),
            "g": hexrgb("#60f0e0")}
    img = from_map(rows, cols)
    # A crack through the wax.
    for x, y in ((5, 4), (6, 5), (6, 6), (7, 7), (9, 9), (10, 10), (10, 11)):
        put(img, x, y, hexrgb("#14041c"))
    return outline(img)


ITEMS["corrupted_seal"] = corrupted_seal

BOOKS = {
    "spellbook_apprentice": (("#3e2410", "#7a4a24"), ("#3a5ad0", "#a0c0ff")),
    "spellbook_iron": (("#3a3e46", "#7a8089"), ("#c0c8d0", "#ffffff")),
    "spellbook_diamond": (("#145a5a", "#2ea0a0"), ("#a0fff0", "#ffffff")),
    "spellbook_archmage": (("#220c36", "#4e2280"), ("#e8b923", "#fff4a0")),
}
for _n, (_cover, _emblem) in BOOKS.items():
    ITEMS[_n] = (lambda c=_cover, e=_emblem: I.tome(c, e))


# Robes: each set in its school's colours, a little darker than the spells.
def robe_pal(school):
    dark, mid, light = SCHOOL[school]
    d, m, l = hexrgb(dark), hexrgb(mid), hexrgb(light)
    mix = [int((a + b) / 2) for a, b in zip(m, l)]
    deep = [int(v * 0.6) for v in d]
    return pal(*["#%02x%02x%02x" % tuple(c) for c in (deep, d, m, mix, l)])


HAT = [
    "................",
    "........44......",
    ".......443......",
    "......4432......",
    "......4322......",
    ".....43322......",
    ".....433221.....",
    "....4332221.....",
    "....43322211....",
    "....GGGGGGGG....",
    "...4332222221...",
    "..443322222211..",
    ".44333222222211.",
    "..111111111111..",
    "................",
    "................",
]


def hat(p):
    cols = {str(i + 1): p[i] for i in range(5)}
    cols["G"] = hexrgb("#e8b923")
    # light on the left, shade on the right: the map uses 4 for light and 1 for shade
    return outline(from_map(HAT, cols))


for _p, *_ in ROBES:
    I.ARMOR_MATERIALS[_p] = robe_pal(ROBE_SCHOOL[_p])
    ITEMS[f"{_p}_helmet"] = (lambda p=_p: hat(I.ARMOR_MATERIALS[p]))
    for _k in ("chestplate", "leggings", "boots"):
        ITEMS[f"{_p}_{_k}"] = (lambda p=_p, k=_k: I.armor(p, k))


def inscription_table_top():
    img = T.planks("oak")
    T.bevel(img, 1.12, 0.7)
    # A sheet of parchment with rune lines, an inkwell and a quill.
    for y in range(3, 12):
        for x in range(2, 10):
            put(img, x, y, hexrgb("#efe2bc") if (x + y) % 5 else hexrgb("#e2d2a6"))
    for y in (5, 7, 9):
        for x in range(3, 9):
            if (x * 7 + y) % 3:
                put(img, x, y, hexrgb("#2a3cb0"))
    for (x, y) in ((11, 9), (12, 9), (11, 10), (12, 10)):
        put(img, x, y, hexrgb("#16207a"))
    put(img, 11, 8, hexrgb("#a8c8d8"))
    put(img, 12, 8, hexrgb("#a8c8d8"))
    for k in range(5):
        put(img, 13 - k, 7 - k, hexrgb("#f4f4f0") if k > 1 else hexrgb("#c8c8c0"))
    put(img, 14, 3, hexrgb("#f4f4f0"))
    return img


def inscription_table_side():
    img = T.planks("oak")
    rgb = img[..., :3]
    for x in range(16):
        rgb[0, x] = hexrgb("#4a3018")
        rgb[1, x] = hexrgb("#2a3cb0") if x % 4 else hexrgb("#e8b923")
        rgb[2, x] = hexrgb("#4a3018")
    # Legs at the corners, open below the top.
    for y in range(3, 16):
        for x in range(2, 14):
            img[y, x, :3] = img[y, x, :3] * 0.45
    for y in range(3, 16):
        for x in (0, 1, 14, 15):
            img[y, x, :3] = hexrgb("#6a4a26") if x in (0, 14) else hexrgb("#4a3018")
    for x in range(2, 14):
        img[9, x, :3] = hexrgb("#6a4a26")
    return img


BLOCKS["inscription_table_top"] = inscription_table_top
BLOCKS["inscription_table_side"] = inscription_table_side
