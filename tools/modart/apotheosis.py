"""Apotheosis ideas: gems in three cuts, gem dust, salvage materials, sigils and the two tables."""

import numpy as np

from .lib import ITEMS, N, T, block, from_map, from_rgb, hexrgb, item, outline, pal, put, rng_for, shade, value_noise

GEM_PALS = {
    "vital": pal("#4a0818", "#8e1630", "#d02a48", "#f2707e", "#ffdce0"),
    "bastion": pal("#0a1c4a", "#1c3e92", "#3466d4", "#7aa4f6", "#dceaff"),
    "fury": pal("#4e1e02", "#9a4406", "#e07a10", "#f8b04a", "#fff0c8"),
    "swift": pal("#04402a", "#0e7a50", "#24bc7e", "#7ae8b4", "#e0fff0"),
    "shadow": pal("#22083e", "#4a1a7a", "#7c3cb8", "#b484e8", "#f0e0ff"),
}

ROUGH = [
    "................",
    "................",
    "................",
    "................",
    "......344.......",
    ".....33454......",
    "....233344......",
    "....2233343.....",
    "...12223333.....",
    "...112223332....",
    "....11222322....",
    ".....112221.....",
    "......1111......",
    "................",
    "................",
    "................",
]

CUT = [
    "................",
    "................",
    "................",
    "................",
    ".....455554.....",
    "....44555544....",
    "...3444554443...",
    "...3333333332...",
    "....33333332....",
    ".....333322.....",
    "......3322......",
    ".......32.......",
    "................",
    "................",
    "................",
    "................",
]

FLAWLESS = [
    "................",
    "................",
    "...45555555544..",
    "..4455555554443.",
    ".344445555444433",
    ".333333333333332",
    "..3333353333332.",
    "...33333333332..",
    "....333333332...",
    ".....3333332....",
    "......33332.....",
    ".......332......",
    "........2.......",
    "................",
    "................",
    "................",
]


def gem_img(kind, cut):
    p = GEM_PALS[kind]
    rows = {"rough": ROUGH, "cut": CUT, "flawless": FLAWLESS}[cut]
    img = outline(from_map(rows, {"1": p[0], "2": p[1], "3": p[2], "4": p[3], "5": p[4]}))
    if cut == "flawless":
        # A glint on the crown.
        for x, y in ((12, 1), (13, 0), (13, 2), (14, 1)):
            put(img, x, y, (255, 255, 255))
    return img


for _k in GEM_PALS:
    for _c in ("rough", "cut", "flawless"):
        ITEMS[f"gem_{_k}_{_c}"] = (lambda k=_k, c=_c: gem_img(k, c))


@item("gem_dust")
def gem_dust():
    rng = rng_for("gem_dust")
    cols = [hexrgb(c) for c in ("#d02a48", "#3466d4", "#e07a10", "#24bc7e", "#7c3cb8", "#e8e8f0")]
    rows = ["................"] * 8 + [
        "......3333......",
        "....33333333....",
        "...3333333333...",
        "..333333333333..",
        "..222222222222..",
        "................",
        "................",
        "................",
    ]
    img = from_map(rows, {"3": hexrgb("#b8b4c4"), "2": hexrgb("#8a8698")})
    for _ in range(18):
        x, y = int(rng.integers(3, 13)), int(rng.integers(8, 12))
        if img[y, x, 3] > 0:
            put(img, x, y, cols[int(rng.integers(0, len(cols)))])
    return outline(img)


MAT_PALS = [
    pal("#3a3a3e", "#5e5e64", "#86868c", "#aeaeb4", "#dadade"),
    pal("#0e3a10", "#1e6a22", "#36a03a", "#6ad46a", "#c8f8c0"),
    pal("#0a2452", "#1a4a9a", "#2e72d8", "#6ea6f8", "#d4e6ff"),
    pal("#2a0a4a", "#52208a", "#8040c4", "#b47cec", "#f0dcff"),
    pal("#5a2a02", "#a85808", "#e88e16", "#ffc650", "#fff4c4"),
]

MAT_ROWS = [
    # common: bent scraps of metal
    [
        "................",
        "................",
        "................",
        "..........44....",
        ".........4553...",
        "...44...4553....",
        "..4553.3442.....",
        "...3433332......",
        "....23322.......",
        "....222.243.....",
        "...222..3543....",
        "...12....332....",
        "..........2.....",
        "................",
        "................",
        "................",
    ],
    # uncommon: a shard
    [
        "................",
        "................",
        "..........5.....",
        ".........454....",
        "........4443....",
        ".......44433....",
        "......444332....",
        ".....444332.....",
        "....443332......",
        "....33322.......",
        "...33322........",
        "...3222.........",
        "...221..........",
        "................",
        "................",
        "................",
    ],
    # rare: a cluster of crystals
    [
        "................",
        "................",
        "......4.........",
        ".....454....4...",
        ".....443...454..",
        ".....443...443..",
        "..4..433..4432..",
        ".454.4332.4432..",
        ".4432433224332..",
        ".44332332243322.",
        "..3322222222222.",
        "..1111111111111.",
        "................",
        "................",
        "................",
        "................",
    ],
    # epic: a swirling orb
    [
        "................",
        "................",
        "................",
        ".....444443.....",
        "....45554433....",
        "...4555443332...",
        "...4554433332...",
        "...4443343322...",
        "...4433433322...",
        "...3333333222...",
        "....33322222....",
        ".....222221.....",
        "................",
        "................",
        "................",
        "................",
    ],
    # mythic: a spark (star)
    [
        "................",
        "................",
        ".......5........",
        ".......5........",
        "......454.......",
        "......444.......",
        "..55444544455...",
        "....4445444.....",
        ".....33433......",
        ".....33.33......",
        "....33...33.....",
        "....2.....2.....",
        "................",
        "................",
        "................",
        "................",
    ],
]

for _i, _n in enumerate(("common", "uncommon", "rare", "epic", "mythic")):
    ITEMS[f"salvage_{_n}"] = (lambda i=_i: outline(from_map(MAT_ROWS[i], {str(k + 1): MAT_PALS[i][k] for k in range(5)})))


SIGIL = [
    "................",
    "................",
    ".....222222.....",
    "....23333332....",
    "...2344444432...",
    "..234444444432..",
    "..234444444432..",
    "..234444444432..",
    "..234444444432..",
    "..234444444432..",
    "..234444444432..",
    "...2344444432...",
    "....23333332....",
    ".....222222.....",
    "................",
    "................",
]

EMBLEMS = {
    # tier sigils: one to four marks climbing up
    "sigil_frontier": ["..#..", ".###.", "....."],
    "sigil_ascent": ["..#..", ".###.", "#####"],
    "sigil_summit": [".#.#.", "#####", "#####"],
    "sigil_pinnacle": ["..#..", "#.#.#", "#####"],
    "sigil_socketing": [".###.", "#...#", ".###."],
    "sigil_withdrawal": ["#...#", ".#.#.", "..#.."],
}
SIGIL_COLS = {
    "sigil_frontier": ("#1e6a22", "#6ad46a"),
    "sigil_ascent": ("#1a4a9a", "#6ea6f8"),
    "sigil_summit": ("#52208a", "#b47cec"),
    "sigil_pinnacle": ("#a85808", "#ffc650"),
    "sigil_socketing": ("#4a4a52", "#d02a48"),
    "sigil_withdrawal": ("#4a4a52", "#24bc7e"),
}


def sigil(name):
    edge, glow = SIGIL_COLS[name]
    img = from_map(SIGIL, {"2": hexrgb("#6a5020"), "3": hexrgb("#d4a83a"), "4": hexrgb(edge)})
    em = EMBLEMS[name]
    for j, row in enumerate(em):
        for i, ch in enumerate(row):
            if ch == "#":
                put(img, 5 + i, 6 + j, hexrgb(glow))
    return outline(img)


for _s in EMBLEMS:
    ITEMS[_s] = (lambda s=_s: sigil(s))


# Tables.
STONE = pal("#5c5c60", "#727276", "#88888c", "#9e9ea2", "#b4b4b8")
WOOD = pal("#4a3018", "#62421f", "#7a5428", "#926633", "#aa7a40")


@block("salvaging_table_top")
def salvaging_top():
    rng = rng_for("salvaging_table_top")
    img = from_rgb(shade(value_noise(rng, 4) * 0.5 + 0.25, STONE, 0.4))
    T.bevel(img, 1.1, 0.75)
    # An iron anvil-like plate with a hammer mark and cracks.
    for y in range(4, 12):
        for x in range(3, 13):
            put(img, x, y, hexrgb("#4a4a50") if (x + y) % 7 else hexrgb("#3a3a40"))
    for x, y in ((5, 6), (6, 7), (7, 7), (8, 8), (9, 9), (10, 9)):
        put(img, x, y, hexrgb("#24242a"))
    for x in range(3, 13):
        put(img, x, 4, hexrgb("#6e6e76"))
    return img


@block("salvaging_table_side")
def salvaging_side():
    rng = rng_for("salvaging_table_side")
    img = from_rgb(shade(value_noise(rng, 8, 2) * 0.5 + 0.25, STONE, 0.4))
    rgb = img[..., :3]
    rgb[0:3, :] = hexrgb("#4a4a50")
    rgb[3, :] = hexrgb("#2e2e34")
    # Bits of salvage on a shelf: green, blue and purple glints.
    for x, c in ((3, "#36a03a"), (7, "#2e72d8"), (11, "#8040c4")):
        put(img, x, 9, hexrgb(c))
        put(img, x + 1, 9, hexrgb(c))
        put(img, x, 10, hexrgb(c))
    rgb[11, :] = hexrgb("#3a3a40")
    return img


@block("gem_cutting_table_top")
def gem_top():
    rng = rng_for("gem_cutting_table_top")
    img = from_rgb(shade(value_noise(rng, 4) * 0.5 + 0.25, WOOD, 0.4))
    T.bevel(img, 1.1, 0.75)
    # A grinding wheel and a gem on the board.
    for y in range(N):
        for x in range(N):
            d = np.hypot(x - 6.0, y - 6.0)
            if d < 4.2:
                put(img, x, y, hexrgb("#9a9aa2") if d > 1.2 else hexrgb("#4a4a50"))
    for x, y, c in ((11, 11, "#f2707e"), (12, 11, "#d02a48"), (11, 12, "#d02a48"), (12, 12, "#8e1630")):
        put(img, x, y, hexrgb(c))
    return img


@block("gem_cutting_table_side")
def gem_side():
    rng = rng_for("gem_cutting_table_side")
    img = from_rgb(shade(value_noise(rng, 8, 2) * 0.5 + 0.25, WOOD, 0.4))
    rgb = img[..., :3]
    rgb[0:2, :] = hexrgb("#aa7a40")
    rgb[2, :] = hexrgb("#4a3018")
    # Little drawers with gem knobs.
    for x0, c in ((2, "#3466d4"), (9, "#24bc7e")):
        for y in range(5, 12):
            for x in range(x0, x0 + 5):
                edge = y in (5, 11) or x in (x0, x0 + 4)
                put(img, x, y, hexrgb("#4a3018") if edge else hexrgb("#7a5428"))
        put(img, x0 + 2, 8, hexrgb(c))
    return img

