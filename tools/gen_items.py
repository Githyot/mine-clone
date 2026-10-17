#!/usr/bin/env python3
"""Procedurally generate original 16x16 pixel-art item icons and a few extra
block textures (chest, bed, saplings, xp orb).

Reuses the helpers and palettes of tools/gen_textures.py.  Everything is drawn
from scratch: geometric masks along the classic "diagonal handle" axis for
tools, small hand-written pixel maps for the rest, then a bevel-style shading
pass and an automatic dark 1px outline for items.

Usage:
    python3 tools/gen_items.py [sheet.png]

Output:
    packs/vanilla/textures/item/<name>.png   (transparent background)
    packs/vanilla/textures/block/<name>.png  (only the new block textures;
                                              existing ones are not touched)
Optionally an 8x contact sheet of all item icons at the given path.
"""

import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_textures import (  # noqa: E402
    N, ROOT, WOODS, XX, YY, canvas, contact_sheet, from_rgb, hexrgb, line,
    pal, planks, put, rng_for, scale_rgb, to_image, value_noise, normalize,
    shade,
)

ITEM_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "item")
BLOCK_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "block")

# --------------------------------------------------------------------------
# Item helpers
# --------------------------------------------------------------------------

HANDLE = pal("#3d2812", "#5a3d1e", "#7a5530", "#94693c")

MATERIALS = {
    "wooden": pal("#5a4220", "#86683a", "#a8874e", "#c4a466", "#dcc086"),
    "stone": pal("#3f3f3f", "#5d5d5d", "#7a7a7a", "#949494", "#adadad"),
    "iron": pal("#5c5c5c", "#9a9a9a", "#c4c4c4", "#dedede", "#ffffff"),
    "golden": pal("#7a5508", "#c08a12", "#eab92a", "#fbdb52", "#fff6a8"),
    "diamond": pal("#0f5f60", "#1fa8a4", "#3fd8cf", "#7ef2e8", "#d4fffa"),
    # Etapa 5 metals.
    "copper": pal("#6a2e14", "#a8542a", "#cf7442", "#e8945e", "#fbc49a"),
    "bronze": pal("#5a3812", "#94622a", "#b8843e", "#d4a456", "#ecc87e"),
    "lead": pal("#2a2e38", "#4a5162", "#687083", "#8890a2", "#aab2c2"),
    "silver": pal("#6c7078", "#a4a8b0", "#cfd2d7", "#e6e8eb", "#ffffff"),
    "tungsten": pal("#28322a", "#4f6853", "#719074", "#97b49a", "#c4dcc6"),
    "platinum": pal("#5c7884", "#98b8c4", "#c4dee6", "#e2f2f6", "#ffffff"),
}

# handle axis: from pixel centre (2.5, 13.5) towards the top-right corner
B = np.array([2.5, 13.5])
D = np.array([1.0, -1.0]) / np.sqrt(2)      # along the handle
P = np.array([-1.0, -1.0]) / np.sqrt(2)     # perpendicular, towards top-left
PX = XX + 0.5 - B[0]
PY = YY + 0.5 - B[1]
U = PX * D[0] + PY * D[1]
V = PX * P[0] + PY * P[1]


def shade_mask(img, mask, p, rng=None, speck=0.12, light_dir=(-1, -1)):
    """Fill mask with palette p (dark->light) using a bevel look: edges that
    face the light (top-left) get the highlight, opposite edges the shadow."""
    n = len(p)
    m = mask.astype(bool)
    up = np.roll(m, 1, 0)
    up[0] = False
    left = np.roll(m, 1, 1)
    left[:, 0] = False
    down = np.roll(m, -1, 0)
    down[-1] = False
    right = np.roll(m, -1, 1)
    right[:, -1] = False
    lvl = np.full((N, N), n // 2 if n > 3 else 1, dtype=int)
    lit = (~up) | (~left)
    dark = (~down) | (~right)
    lvl[lit] = n - 1
    lvl[dark & ~lit] = 0 if n <= 3 else 1
    lvl[dark & lit] = n // 2
    if rng is not None and speck > 0:
        r = rng.random((N, N))
        inner = up & left & down & right
        lvl[inner & (r < speck)] += 1
        lvl[inner & (r > 1 - speck)] -= 1
    lvl = np.clip(lvl, 0, n - 1)
    img[m, :3] = p[lvl[m]]
    img[m, 3] = 255
    return img


def outline(img, k=0.38, alpha_min=1):
    """Add a 1px dark outline around opaque pixels (4-neighbourhood).  The
    outline colour is the darkest neighbouring colour darkened by k."""
    a = img[..., 3] >= alpha_min
    rgb = img[..., :3]
    lum = rgb @ np.array([0.3, 0.59, 0.11])
    best = np.full((N, N), 1e9)
    col = np.zeros((N, N, 3))
    for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        sa = np.roll(np.roll(a, dy, 0), dx, 1)
        sl = np.roll(np.roll(lum, dy, 0), dx, 1)
        sc = np.roll(np.roll(rgb, dy, 0), dx, 1)
        if dy == 1:
            sa[0] = False
        if dy == -1:
            sa[-1] = False
        if dx == 1:
            sa[:, 0] = False
        if dx == -1:
            sa[:, -1] = False
        take = sa & ~a & (sl < best)
        best[take] = sl[take]
        col[take] = sc[take]
    edge = best < 1e9
    out = img.copy()
    out[edge, :3] = col[edge] * k
    out[edge, 3] = 255
    return out


def from_map(rows, colors):
    """Pixel map: list of 16 strings; '.' transparent, other chars -> colour."""
    img = canvas()
    for y, row in enumerate(rows):
        assert len(row) == N, (y, row)
        for x, ch in enumerate(row):
            if ch != ".":
                put(img, x, y, colors[ch])
    return img


def pmap(p, prefix="12345"):
    """Map characters '1'..'5' (or a custom prefix) to palette entries."""
    return {c: p[i] for i, c in enumerate(prefix[:len(p)])}


def mask_from_rows(rows):
    return np.array([[c != "." for c in r] for r in rows])


def handle(img, u0, u1, p=HANDLE):
    m = (np.abs(V) <= 0.75) & (U >= u0) & (U <= u1)
    img[m & (V > 0), :3] = p[3]
    img[m & (V <= 0), :3] = p[1]
    # darker tip at the bottom end
    img[m & (U < u0 + 0.8) & (V <= 0), :3] = p[0]
    img[m, 3] = 255
    return m


# --------------------------------------------------------------------------
# Tools
# --------------------------------------------------------------------------

TOOL_KINDS = ("pickaxe", "axe", "shovel", "sword", "hoe")


def tool(material, kind):
    rng = rng_for(f"{material}_{kind}")
    p = MATERIALS[material]
    img = canvas()
    if kind == "pickaxe":
        handle(img, 0, 13.2)
        # curved head perpendicular to the handle, tips bending back
        c = 12.2 - 0.07 * V ** 2
        head = (U >= c - 0.9) & (U <= c + 0.9) & (np.abs(V) <= 6.6)
        shade_mask(img, head, p[1:], rng)
    elif kind == "axe":
        handle(img, 0, 13.4)
        # blade on the upper-left side of the handle top, flaring outwards
        blade = (V > 0.6) & (V <= 6.0) & \
                (U >= 9.4 - 0.55 * (V - 0.6)) & (U <= 12.6 + 0.45 * (V - 0.6))
        back = (V < -0.6) & (V >= -2.0) & (U >= 10.6) & (U <= 12.2)
        shade_mask(img, blade | back, p[1:], rng)
        # cutting edge highlight
        edge = blade & (V > 5.0)
        img[edge, :3] = p[4]
    elif kind == "shovel":
        handle(img, 0, 10.5)
        blade = ((U - 12.6) / 3.0) ** 2 + (V / 2.6) ** 2 <= 1.0
        shade_mask(img, blade, p[1:], rng)
        img[blade & (U > 14.6), :3] = p[1]
    elif kind == "sword":
        # short handle, pommel, crossguard, long blade
        hm = (np.abs(V) <= 0.75) & (U >= 0.6) & (U <= 4.2)
        img[hm & (V > 0), :3] = HANDLE[3]
        img[hm & (V <= 0), :3] = HANDLE[1]
        img[hm, 3] = 255
        pommel = (np.abs(V) <= 0.75) & (U >= -0.6) & (U < 0.6)
        img[pommel, :3] = p[2]
        img[pommel, 3] = 255
        guard = (np.abs(V) <= 3.3) & (U >= 4.2) & (U <= 5.6)
        gp = HANDLE if material == "wooden" else p
        shade_mask(img, guard, gp[[0, 1, 2]] if material == "wooden" else p[:3])
        blade = (np.abs(V) <= 1.25 - np.maximum(0, U - 16.0) * 0.9) & \
                (U > 5.6) & (U <= 17.3)
        shade_mask(img, blade, p[1:], None)
        # fuller: centre line slightly lighter than the blade
        img[blade & (np.abs(V) < 0.4) & (U < 16.4), :3] = p[3]
        _ = gp
    elif kind == "hammer":
        # Silent Gear idea: a heavy square head that breaks 3x3.
        handle(img, 0, 11.6)
        head = (U >= 10.2) & (U <= 15.0) & (np.abs(V) <= 4.2)
        shade_mask(img, head, p[1:], rng)
        img[head & (np.abs(V) > 3.4), :3] = p[1]
    elif kind == "excavator":
        handle(img, 0, 9.4)
        blade = ((U - 12.4) / 3.6) ** 2 + (V / 4.0) ** 2 <= 1.0
        shade_mask(img, blade, p[1:], rng)
        img[blade & (U > 15.0), :3] = p[1]
    elif kind == "sickle":
        handle(img, 0, 7.0)
        r = np.sqrt((U - 10.5) ** 2 + (V + 1.5) ** 2)
        blade = (r >= 4.0) & (r <= 5.6) & (U >= 7.0) & (V >= -1.5)
        shade_mask(img, blade, p[1:], rng)
        img[blade & (r > 5.0), :3] = p[4]
    elif kind == "hoe":
        handle(img, 0, 12.4)
        # L-shaped head: flat bar to the left of the handle top, blade
        # turning down at its far end
        rows = [
            "................",
            "................",
            "....##########..",
            "....##########..",
            "...###..........",
            "...###..........",
            "...##...........",
        ] + ["." * N] * 9
        shade_mask(img, mask_from_rows(rows), p[1:], rng)
    return outline(img)


# --------------------------------------------------------------------------
# Materials
# --------------------------------------------------------------------------


def stick():
    img = canvas()
    m = (np.abs(V) <= 0.75) & (U >= 1.0) & (U <= 15.0)
    img[m & (V > 0), :3] = HANDLE[3]
    img[m & (V <= 0), :3] = HANDLE[1]
    img[m & (U < 2.2) & (V <= 0), :3] = HANDLE[0]
    img[m, 3] = 255
    return outline(img)


LUMP = [
    "................",
    "................",
    "................",
    "......####......",
    "....#######.....",
    "...#########....",
    "..###########...",
    "..############..",
    "..############..",
    "...###########..",
    "...##########...",
    "....########....",
    ".....#####......",
    "................",
    "................",
    "................",
]

LUMP2 = [
    "................",
    "................",
    "................",
    "........###.....",
    ".....#######....",
    "....#########...",
    "...##########...",
    "..###########...",
    "..############..",
    "..############..",
    "...###########..",
    "....#########...",
    "......####......",
    "................",
    "................",
    "................",
]


def lump(name, p, rows=LUMP, spots=None, nspots=6):
    rng = rng_for(name)
    img = canvas()
    m = mask_from_rows(rows)
    shade_mask(img, m, p, rng, speck=0.25)
    # chunky facets: a few darker cracks
    ys, xs = np.nonzero(m)
    for _ in range(3):
        i = rng.integers(len(xs))
        x, y = xs[i], ys[i]
        for (dx, dy) in ((0, 0), (1, 1)):
            if m[(y + dy) % N, (x + dx) % N]:
                put(img, x + dx, y + dy, p[0])
    if spots is not None:
        inner = m & np.roll(m, 1, 0) & np.roll(m, -1, 0) & \
            np.roll(m, 1, 1) & np.roll(m, -1, 1)
        ys, xs = np.nonzero(inner)
        for _ in range(nspots):
            i = rng.integers(len(xs))
            x, y = xs[i], ys[i]
            put(img, x, y, spots[1])
            if m[y, x + 1]:
                put(img, x + 1, y, spots[0])
    return outline(img)


def coal():
    return lump("coal", pal("#0e0e0e", "#1c1c1c", "#2c2c2c", "#404040", "#5a5a5a"))


def charcoal():
    return lump("charcoal", pal("#120c08", "#22180f", "#342518", "#4a3624",
                                "#614a32"), LUMP2)


RAW = {
    "iron": (pal("#6e4e3a", "#8f6a52", "#b08a70", "#cfa88c", "#e6c6ad"),
             pal("#5a5a5a", "#d0d0d0")),
    "gold": (pal("#7a5a10", "#a8801c", "#d6a92a", "#f2cd45", "#fff08a"),
             pal("#8a6a10", "#fff6b0")),
    "copper": (pal("#6a2e14", "#97451f", "#c0662f", "#e08a4c", "#f3ae78"),
               pal("#2f7a68", "#5fc0a8")),
    "tin": (pal("#4f5762", "#6d7682", "#8d97a4", "#b0bac6", "#d2dae3"),
            pal("#3e444c", "#e8eef4")),
    "silver": (pal("#6c7078", "#9a9fa8", "#c3c7cd", "#dfe2e6", "#ffffff"),
               pal("#55585e", "#ffffff")),
    "lead": (pal("#2c303a", "#454b5a", "#5f6676", "#7a8292", "#979fae"),
             pal("#1e2128", "#b0b8c6")),
    "tungsten": (pal("#2a2f2a", "#46524a", "#62746a", "#82988a", "#a6bcae"),
                 pal("#1c221e", "#c8e0cc")),
    "platinum": (pal("#5a6e78", "#86a0ac", "#b0cad4", "#d4e8ee", "#f4ffff"),
                 pal("#4a5a62", "#ffffff")),
}


def raw(metal):
    body, spots = RAW[metal]
    return lump(f"raw_{metal}", body, LUMP2, spots, 5)


INGOT = [
    "................",
    "................",
    "................",
    "................",
    "................",
    ".....55555555...",
    "....5444444443..",
    "...54444444442..",
    "..544444444432..",
    "..3333333333321.",
    "..2222222222221.",
    "..1111111111111.",
    "................",
    "................",
    "................",
    "................",
]

INGOT_COL = {
    "iron": pal("#7e7e7e", "#a8a8a8", "#c8c8c8", "#e0e0e0", "#ffffff"),
    "gold": pal("#a0700c", "#d29a16", "#f2c22c", "#fde05a", "#fffbc0"),
    "copper": pal("#7a3818", "#a8542a", "#cf7442", "#e8945e", "#fbc49a"),
    "tin": pal("#606a76", "#828c99", "#a4aebb", "#c4ccd6", "#e6ecf2"),
    "silver": pal("#8a8e96", "#b4b8c0", "#d6d9de", "#eceef1", "#ffffff"),
    "bronze": pal("#6a4416", "#94622a", "#b8843e", "#d4a456", "#ecc87e"),
    "lead": pal("#3a3f4c", "#565d6e", "#737b8d", "#9098a9", "#b4bbc8"),
    "tungsten": pal("#34423a", "#55705c", "#76927c", "#9ab49e", "#c6dcc8"),
    "platinum": pal("#7894a0", "#a4c0ca", "#c8e0e8", "#e2f2f6", "#ffffff"),
}


def ingot(metal):
    p = INGOT_COL[metal]
    img = from_map(INGOT, pmap(p))
    return outline(img)


def diamond(p=None):
    rows = [
        "................",
        "................",
        "................",
        "....45555554....",
        "...4455555443...",
        "..344454445433..",
        "..333333333332..",
        "...3333333332...",
        "....33333322....",
        ".....333322.....",
        "......3322......",
        ".......32.......",
        "................",
        "................",
        "................",
        "................",
    ]
    if p is None:
        p = pal("#0f6a68", "#1fa8a4", "#3fd8cf", "#8ff4ea", "#e4fffb")
    img = from_map(rows, {"1": p[0], "2": p[1], "3": p[2], "4": p[3], "5": p[4]})
    return outline(img)


GEMS = {
    "ruby": pal("#4a0612", "#9a1428", "#d0203c", "#f66a7e", "#ffd0d8"),
    "sapphire": pal("#081c52", "#1a3c9c", "#2c5cd8", "#6e9cff", "#d4e4ff"),
    "topaz": pal("#5a3a04", "#a8700c", "#e0a218", "#ffd45a", "#fff6c8"),
    "amethyst": pal("#2e0c4e", "#5e2496", "#8a42c8", "#c08af0", "#f0dcff"),
}


def gem(name):
    """A cut gem: the diamond's outline, in the gem's colour."""
    return diamond(GEMS[name])


def life_crystal_item():
    rows = [
        "................",
        "................",
        "...444....444...",
        "..45554..45544..",
        ".4555444454443..",
        ".4554444444433..",
        ".4444444444332..",
        ".3444444443332..",
        "..34444443332...",
        "...333333322....",
        "....3333322.....",
        ".....33322......",
        "......322.......",
        ".......2........",
        "................",
        "................",
    ]
    p = pal("#4a0610", "#8a1020", "#c42034", "#ee4a5a", "#ffc0c8")
    return outline(from_map(rows, pmap(p)))


# --------------------------------------------------------------------------
# Magic: wands, staffs, spell books, mana crystal
# --------------------------------------------------------------------------

WAND_GEMS = {
    "ruby": GEMS["ruby"], "sapphire": GEMS["sapphire"], "topaz": GEMS["topaz"],
    "amethyst": GEMS["amethyst"],
    "emerald": pal("#06501c", "#0d7a2f", "#17b54a", "#5ae484", "#c8ffd8"),
}


def wand(gem):
    """A thin wooden wand with a cut gem set at its tip."""
    img = canvas()
    m = (np.abs(V) <= 0.6) & (U >= 1.0) & (U <= 11.5)
    img[m & (V > 0), :3] = HANDLE[3]
    img[m & (V <= 0), :3] = HANDLE[1]
    img[m & (U < 3.0), :3] = HANDLE[0]
    img[m, 3] = 255
    # gold band below the gem
    band = (np.abs(V) <= 1.0) & (U > 10.0) & (U <= 11.5)
    img[band, :3] = hexrgb("#d29a16")
    img[band, 3] = 255
    p = WAND_GEMS[gem]
    rows = {(12, 2): 3, (13, 2): 4, (12, 3): 2, (13, 3): 3, (11, 3): 3, (12, 1): 4, (14, 2): 2, (13, 1): 5, (11, 2): 4}
    for (x, y), k in rows.items():
        put(img, x, y, p[k - 1] if k <= len(p) else p[-1])
    return outline(img)


def staff(gem, metal):
    """A long metal-shod staff with prongs holding a large gem."""
    img = canvas()
    m = (np.abs(V) <= 0.7) & (U >= 0.5) & (U <= 12.5)
    img[m & (V > 0), :3] = HANDLE[3]
    img[m & (V <= 0), :3] = HANDLE[1]
    img[m, 3] = 255
    mp = MATERIALS[metal]
    for u0 in (3.0, 9.5):
        band = (np.abs(V) <= 1.0) & (U > u0) & (U <= u0 + 1.2)
        img[band, :3] = mp[2]
        img[band, 3] = 255
    p = WAND_GEMS[gem]
    head = [
        (11, 1, 2), (12, 1, 3), (13, 1, 2),
        (11, 2, 3), (12, 2, 5), (13, 2, 4), (14, 2, 2),
        (11, 3, 2), (12, 3, 4), (13, 3, 3), (14, 3, 2),
        (12, 4, 2), (13, 4, 2),
    ]
    for x, y, k in head:
        put(img, x, y, p[k - 1])
    # metal prongs around the gem
    for x, y in ((10, 2), (10, 4), (11, 5), (14, 4), (15, 1), (14, 0)):
        put(img, x, y, mp[1])
    return outline(img)


def tome(cover, emblem):
    """A thick spell book seen from the front: coloured cover, gold corners, an emblem."""
    rows = [
        "................",
        "................",
        "...GCCCCCCCCG...",
        "..GCCCCCCCCCCG..",
        "..CCCCCCCCCCCP..",
        "..CCCCEEECCCCP..",
        "..CCCEEEEECCCP..",
        "..CCCEEHEECCCP..",
        "..CCCEEEEECCCP..",
        "..CCCCEEECCCCP..",
        "..CCCCCCCCCCCP..",
        "..CCCCCCCCCCCP..",
        "..GCCCCCCCCCGP..",
        "...DDDDDDDDDDP..",
        "................",
        "................",
    ]
    cols = {"C": hexrgb(cover[1]), "G": hexrgb("#e8b923"), "P": hexrgb("#e8e0c8"),
            "D": hexrgb(cover[0]), "E": hexrgb(emblem[0]), "H": hexrgb(emblem[1])}
    img = from_map(rows, cols)
    # light along the top edge of the cover
    for x in range(4, 12):
        put(img, x, 3, scale_rgb(img[3, x, :3], 1.25))
    return outline(img)


def mana_crystal():
    rows = [
        "................",
        ".......5........",
        "......545.......",
        "......444.......",
        ".....44443......",
        "..5..44433..4...",
        "..44.44333.43...",
        "...443433343....",
        "....44433333....",
        "....44333332....",
        ".....433332.....",
        ".....333322.....",
        "......3322......",
        ".......21.......",
        "................",
        "................",
    ]
    p = pal("#0a1a5a", "#1c3aa8", "#2e62e0", "#6e9cff", "#e0ecff")
    return outline(from_map(rows, pmap(p)))


def lute():
    """A round-bellied lute with a bent neck."""
    rows = [
        "..........kk....",
        ".........kk.....",
        "........nn......",
        ".......nn.......",
        "......nn........",
        ".....nn.........",
        "...bbbbb........",
        "..bBBBBBb.......",
        ".bBBBhBBBb......",
        ".bBBhHhBBb......",
        ".bBBBhBBBb......",
        ".bBBBBBBBb......",
        "..bBBsBBb.......",
        "...bbbbb........",
        "................",
        "................",
    ]
    cols = {"k": hexrgb("#3a2414"), "n": hexrgb("#5a3a20"), "b": hexrgb("#6a4224"), "B": hexrgb("#a8743c"),
            "h": hexrgb("#3a2414"), "H": hexrgb("#1e1208"), "s": hexrgb("#d8c8a0")}
    return outline(from_map(rows, cols))


def flute():
    """A wooden flute held on the slant, with finger holes."""
    rows = [
        "................",
        "............ww..",
        "...........wWw..",
        "..........wWw...",
        ".........whw....",
        "........wWw.....",
        ".......whw......",
        "......wWw.......",
        ".....whw........",
        "....wWw.........",
        "...wWw..........",
        "..wWw...........",
        "..ww............",
        "................",
        "................",
        "................",
    ]
    cols = {"w": hexrgb("#6a4a24"), "W": hexrgb("#b08850"), "h": hexrgb("#2a1a0c")}
    return outline(from_map(rows, cols))


def cello():
    """A cello standing on its spike, f-holes and strings."""
    rows = [
        ".......kk.......",
        ".......nn.......",
        ".......nn.......",
        ".......nn.......",
        ".....bbbbbb.....",
        "....bBBssBBb....",
        "....bBfssfBb....",
        ".....bBssBb.....",
        "....bBBssBBb....",
        "...bBBfssfBBb...",
        "...bBBBssBBBb...",
        "...bBBBssBBBb...",
        "....bBBssBBb....",
        ".....bbbbbb.....",
        ".......kk.......",
        ".......k........",
    ]
    cols = {"k": hexrgb("#1e1208"), "n": hexrgb("#3a2414"), "b": hexrgb("#5a2a10"), "B": hexrgb("#9a4a1c"),
            "f": hexrgb("#1e1208"), "s": hexrgb("#d8c8a0")}
    return outline(from_map(rows, cols))


def blaze_rod():
    """A glowing rod of ember, bright at its heart."""
    rows = [
        "................",
        "............oo..",
        "...........oYo..",
        "..........oYo...",
        ".........oYo....",
        "........oYo.....",
        ".......oYo......",
        "......oYo.......",
        ".....oYo........",
        "....oYo.........",
        "...oYo..........",
        "..oYo...........",
        "..oo............",
        "................",
        "................",
        "................",
    ]
    cols = {"o": hexrgb("#c46a1a"), "Y": hexrgb("#ffd060")}
    return outline(from_map(rows, cols))


def glow_berries():
    """A sprig with three glowing orange berries."""
    rows = [
        "................",
        "........g.......",
        ".......gg.......",
        "......g..g......",
        ".....g....g.....",
        "....oO...oO.....",
        "....OO...OO.....",
        "........g.......",
        ".......oO.......",
        ".......OO.......",
        "................",
        "................",
        "................",
        "................",
        "................",
        "................",
    ]
    cols = {"g": hexrgb("#4a6e26"), "o": hexrgb("#ffe08a"), "O": hexrgb("#f09a2a")}
    return outline(from_map(rows, cols))


def lich_phylactery():
    """A small frosted bottle with a pale blue soul light inside."""
    rows = [
        "................",
        "......cccc......",
        "......cCCc......",
        ".......gg.......",
        "......g..g......",
        ".....g.11.g.....",
        "....g.1221.g....",
        "....g12332.g....",
        "....g12332.g....",
        "....g.1221.g....",
        ".....g.11.g.....",
        "......gggg......",
        "................",
        "................",
        "................",
        "................",
    ]
    cols = {"c": hexrgb("#5a4a3a"), "C": hexrgb("#7a6650"), "g": hexrgb("#a8c4d8"), "1": hexrgb("#3e6e9a"),
            "2": hexrgb("#6ea4d4"), "3": hexrgb("#d4f0ff")}
    return outline(from_map(rows, cols))


def obsidian_heart():
    """A cut lump of obsidian with a purple fire burning in it."""
    rows = [
        "................",
        "................",
        ".....oo..oo.....",
        "....oOOooOOo....",
        "...oOOppppOOo...",
        "...oOpPPPPpOo...",
        "...oOpPWWPpOo...",
        "....oOpPPpOo....",
        ".....oOppOo.....",
        "......oOOo......",
        ".......oo.......",
        "................",
        "................",
        "................",
        "................",
        "................",
    ]
    cols = {"o": hexrgb("#120c1a"), "O": hexrgb("#2a1e3a"), "p": hexrgb("#5a2a8a"), "P": hexrgb("#9a4ad4"),
            "W": hexrgb("#e4b4ff")}
    return outline(from_map(rows, cols))


def void_seed():
    """A dark pod split open on pink petals."""
    rows = [
        "................",
        "................",
        "......p..p......",
        ".....pPppPp.....",
        "....pP.dd.Pp....",
        "....p.dDDd.p....",
        "...pP.dDDd.Pp...",
        "....p.dDDd.p....",
        ".....p.dd.p.....",
        "......gddg......",
        ".......gg.......",
        ".......g........",
        "................",
        "................",
        "................",
        "................",
    ]
    cols = {"p": hexrgb("#b0407a"), "P": hexrgb("#e478b0"), "d": hexrgb("#1a1424"), "D": hexrgb("#3a2a4e"),
            "g": hexrgb("#2e5a2a")}
    return outline(from_map(rows, cols))


def ancient_seed():
    """A big dark acorn with a green glow sprouting from its cap."""
    rows = [
        "................",
        "........gG......",
        ".......gGg......",
        "........g.......",
        ".....cccccc.....",
        "....cCCCCCCc....",
        "....cccccccc....",
        ".....s3333s.....",
        ".....344443.....",
        ".....344543.....",
        ".....334443.....",
        "......3443......",
        "......2332......",
        ".......22.......",
        "................",
        "................",
    ]
    cols = {"g": hexrgb("#3c9a3a"), "G": hexrgb("#9cff9a"), "c": hexrgb("#4a3420"), "C": hexrgb("#6a4c2c"),
            "s": hexrgb("#3a2814"), "2": hexrgb("#3a2410"), "3": hexrgb("#5e3c1c"), "4": hexrgb("#7e5428"),
            "5": hexrgb("#a8763c")}
    return outline(from_map(rows, cols))


def guardian_heart():
    """A knot of living wood with a green light in it."""
    rows = [
        "................",
        "................",
        "...333....333...",
        "..34443..34443..",
        ".3445543344443..",
        ".3456654444443..",
        ".3456654444432..",
        ".3445543444332..",
        "..344444443322..",
        "...3444443322...",
        "....34443322....",
        ".....34332......",
        "......332.......",
        ".......2........",
        "................",
        "................",
    ]
    cols = {"2": hexrgb("#2a1c10"), "3": hexrgb("#4c341c"), "4": hexrgb("#70522e"),
            "5": hexrgb("#3cb850"), "6": hexrgb("#b8ffb0")}
    return outline(from_map(rows, cols))


def emerald():
    rows = [
        "................",
        "................",
        ".......54.......",
        "......5543......",
        ".....554433.....",
        ".....544333.....",
        ".....543332.....",
        ".....443322.....",
        ".....433322.....",
        ".....433222.....",
        ".....333221.....",
        "......3221......",
        ".......21.......",
        "................",
        "................",
        "................",
    ]
    p = pal("#06501c", "#0d7a2f", "#17b54a", "#5ae484", "#c8ffd8")
    return outline(from_map(rows, pmap(p)))


def lapis():
    rows = [
        "................",
        "................",
        "................",
        "................",
        "......4455......",
        "....44445554....",
        "...4433344443...",
        "..443332333433..",
        "..433322233332..",
        "..33322222332...",
        "...332222221....",
        ".....2211.......",
        "................",
        "................",
        "................",
        "................",
    ]
    p = pal("#0e1d5a", "#1a2f8a", "#2c4fc0", "#4a74e0", "#8eaaf6")
    img = from_map(rows, pmap(p))
    for (x, y) in ((6, 7), (9, 9), (11, 6)):
        put(img, x, y, hexrgb("#d8c050"))   # little golden flecks
    return outline(img)


def redstone():
    rng = rng_for("redstone")
    img = canvas()
    p = pal("#4a0000", "#8f0000", "#c41010", "#ff3a2a", "#ff9a8a")
    cx, cy = 7.5, 9.5
    d = np.hypot((XX + 0.5 - 8) / 5.2, (YY + 0.5 - 10) / 3.6)
    m = (d < 1.0) & (rng.random((N, N)) < 1.15 - d * 0.6)
    m |= (d < 0.55)
    lvl = np.clip((1.0 - d) * 3.2 + rng.random((N, N)) * 1.4, 0, 4).astype(int)
    img[m, :3] = p[lvl[m]]
    img[m, 3] = 255
    # stray grains
    for (x, y) in ((3, 6), (12, 6), (8, 4), (13, 12), (2, 11)):
        put(img, x, y, p[3])
    _ = (cx, cy)
    return outline(img)


def flint():
    rows = [
        "................",
        "................",
        "................",
        ".......45.......",
        "......4453......",
        ".....443332.....",
        "....4433322.....",
        "....43332222....",
        "...4433222221...",
        "...43322222211..",
        "...3332222111...",
        "....33222111....",
        ".....22211......",
        "................",
        "................",
        "................",
    ]
    p = pal("#141414", "#2e2e30", "#48484c", "#6a6a70", "#9a9aa2")
    return outline(from_map(rows, pmap(p)))


def flint_and_steel():
    rows = [
        "................",
        "................",
        "..........11....",
        ".........1221...",
        "........12331...",
        "........1231....",
        "......1.111.....",
        ".....1A1........",
        "....1AB1..45....",
        "...1AB1..4553...",
        "..1AB1...43332..",
        "..1B1....33222..",
        "...1......2211..",
        "................",
        "................",
        "................",
    ]
    p = pal("#141414", "#2e2e30", "#48484c", "#6a6a70", "#9a9aa2")
    m = pmap(p)
    m.update({"A": hexrgb("#c8c8cc"), "B": hexrgb("#8a8a90")})
    return outline(from_map(rows, m))


def honeycomb():
    rows = [
        "................",
        "................",
        "....111.111.....",
        "...12321232.....",
        "...1343134321...",
        "....121.12321...",
        "...111.1.1111...",
        "..12321232121...",
        "..13431343432...",
        "...12112321.....",
        "....1.1.1.......",
        "...1232121......",
        "...134343.......",
        "....1221........",
        "................",
        "................",
    ]
    return outline(from_map(rows, pmap(pal("#7a4a08", "#c07a10", "#e8a828", "#ffd660"))))


def minecart_item():
    rows = [
        "................",
        "................",
        "................",
        "................",
        "..1..........1..",
        "..14444444444 1.",
        "..1433333333341.",
        "..1432222222341.",
        "..1432222222341.",
        "..1433333333341.",
        "...14444444441..",
        "...11111111111..",
        "....155...551...",
        "....1551..5511..",
        ".....11....11...",
        "................",
    ]
    rows = [r.replace(" ", "4") for r in rows]
    return outline(from_map(rows, pmap(pal("#1e1e22", "#2e2e34", "#4a4a52", "#6e6e78", "#9a9aa4", "#3a3a40"), "123456")))


def backpack(trim):
    """Leather pack with a flap, two straps and a metal trim for the tier."""
    rows = [
        "................",
        "......1111......",
        ".....1....1.....",
        "....11111111....",
        "...1222222221...",
        "...1233333321...",
        "...12TTTTTT21...",
        "..112333333211..",
        "..142333K33241..",
        "..142333333241..",
        "..142344443241..",
        "..142344443241..",
        "..142333333241..",
        "...1222222221...",
        "...1111111111...",
        "................",
    ]
    m = pmap(pal("#2a1608", "#5e3a1a", "#7e522a", "#3a2412"), "1234")
    t = pal(*trim)
    m.update({"T": t[1], "K": t[0]})
    return outline(from_map(rows, m))


def string_item():
    img = canvas()
    c1, c2 = hexrgb("#f4f4f0"), hexrgb("#c8c8c2")
    pts = [(2, 13), (3, 12), (4, 12), (5, 11), (5, 10), (4, 9), (4, 8), (5, 7),
           (6, 7), (7, 8), (8, 8), (9, 7), (9, 6), (8, 5), (8, 4), (9, 3),
           (10, 3), (11, 4), (12, 4), (13, 3)]
    for i, (x, y) in enumerate(pts):
        put(img, x, y, c1 if i % 3 else c2)
    return outline(img, k=0.32)


def feather():
    rows = [
        "................",
        "............45..",
        "..........44554.",
        ".........445543.",
        "........4455433.",
        ".......4455433..",
        "......4455433...",
        ".....4455433....",
        "....445543 3....",
        "...44543 33.....",
        "...4543333......",
        "...q433 3.......",
        "..q.33..........",
        ".q..............",
        "q...............",
        "................",
    ]
    rows = [r.replace(" ", "3") for r in rows]
    p = pal("#808080", "#a8a8a8", "#cfcfcf", "#e8e8e8", "#ffffff")
    cols = pmap(p)
    cols["q"] = hexrgb("#9a9a9a")
    img = from_map(rows, cols)
    # quill / rachis runs down the middle
    for (x, y) in ((13, 2), (12, 3), (11, 4), (10, 5), (9, 6), (8, 7), (7, 8),
                   (6, 9), (5, 10), (4, 11)):
        put(img, x, y, hexrgb("#bdbdb6"))
    return outline(img, k=0.35)


def leather():
    rows = [
        "................",
        "................",
        "...44......44...",
        "...444444444....",
        "....44444443....",
        "...444434433....",
        "..44443333333...",
        "..44433333332...",
        "...4333333322...",
        "...3333332222...",
        "..33333222222...",
        "..33322222212...",
        "...22......11...",
        "................",
        "................",
        "................",
    ]
    p = pal("#4a2a14", "#6e3e1e", "#8e5428", "#ac6c36", "#c88a4e")
    return outline(from_map(rows, pmap(p)))


def bone():
    rows = [
        "................",
        "............55..",
        "...........5544.",
        "..........55443.",
        "........55443...",
        ".......5543.....",
        "......5543......",
        ".....5543.......",
        "....5543........",
        "...5543.........",
        ".5543...........",
        "5543............",
        "5443............",
        ".33.............",
        "................",
        "................",
    ]
    # rebuild as a clean diagonal shaft with knobbly ends
    img = canvas()
    p = pal("#8c887a", "#b8b4a4", "#d8d4c4", "#ecead8", "#ffffff")
    shaft = (np.abs(V) <= 0.8) & (U >= 2.0) & (U <= 13.0)
    knob = np.zeros((N, N), bool)
    for (uc, vc) in ((1.6, 1.1), (1.6, -1.1), (13.4, 1.1), (13.4, -1.1)):
        knob |= np.hypot(U - uc, V - vc) <= 1.25
    m = shaft | knob
    shade_mask(img, m, p[1:], None)
    _ = rows
    return outline(img, k=0.35)


def gunpowder():
    rng = rng_for("gunpowder")
    img = canvas()
    p = pal("#2a2a2a", "#454545", "#5e5e5e", "#7a7a7a", "#9a9a9a")
    d = np.hypot((XX + 0.5 - 8) / 5.6, (YY + 0.5 - 10.5) / 3.6)
    m = (d < 1.0) & ((rng.random((N, N)) < 1.2 - d * 0.7) | (d < 0.6))
    m &= YY >= 6
    lvl = np.clip((1 - d) * 2.5 + rng.random((N, N)) * 2.0, 0, 4).astype(int)
    img[m, :3] = p[lvl[m]]
    img[m, 3] = 255
    for (x, y) in ((4, 6), (11, 5), (13, 9), (2, 10)):
        put(img, x, y, p[2])
    return outline(img)


def rotten_flesh():
    rows = [
        "................",
        "................",
        "................",
        "......4444......",
        "....44433444....",
        "...4433333344...",
        "..443332233344..",
        "..433322223333..",
        "..33322g22g333..",
        "..33222gg22232..",
        "...3222222222...",
        "...g2221122g....",
        "....g211112.....",
        "......11........",
        "................",
        "................",
    ]
    p = pal("#4a2a1a", "#6e3e2a", "#8e5a3e", "#ad7a52", "#c89a70")
    cols = pmap(p)
    cols["g"] = hexrgb("#5a7a2a")
    return outline(from_map(rows, cols))


def clay_ball():
    rows = [
        "................",
        "................",
        "................",
        "................",
        "......4444......",
        ".....454443.....",
        "....44544433....",
        "....44444333....",
        "....44443333....",
        "....33333332....",
        ".....333322.....",
        "......2222......",
        "................",
        "................",
        "................",
        "................",
    ]
    p = pal("#5e6470", "#7a808c", "#9aa0ac", "#b4bac6", "#d6dce6")
    return outline(from_map(rows, pmap(p)))


def brick():
    rows = [
        "................",
        "................",
        "................",
        "................",
        "................",
        "......55555554..",
        ".....544444443..",
        "....5444444443..",
        "...54444444432..",
        "..333333333322..",
        "..33333333332...",
        "..2222222222....",
        "................",
        "................",
        "................",
        "................",
    ]
    p = pal("#5a2010", "#7e3018", "#a24428", "#bc5838", "#d6785a")
    return outline(from_map(rows, pmap(p)))


NUGGET = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "......###.......",
    ".....#####......",
    "....#######.....",
    "....########....",
    ".....#######....",
    "......####......",
    "................",
    "................",
    "................",
    "................",
    "................",
]


def gold_nugget():
    return lump("gold_nugget", pal("#7a5a08", "#b08a14", "#e0b830", "#f6dc60", "#fff4b0"), rows=NUGGET)


def quartz():
    rows = [
        "................",
        "................",
        "........5.......",
        ".......554......",
        "......5544......",
        ".....55443.4....",
        "....554433.44...",
        "...5544333443...",
        "...5443322433...",
        "..554432224332..",
        "..54433222333...",
        "...443322232....",
        "....33222.2.....",
        ".....222........",
        "................",
        "................",
    ]
    p = pal("#8a8078", "#b8aea4", "#d8d0c6", "#ece6de", "#fcfaf6")
    return outline(from_map(rows, pmap(p)))


ORB = [
    "................",
    "................",
    "................",
    "......2222......",
    ".....233332.....",
    "....23444332....",
    "...2345543321...",
    "...2345433321...",
    "...2344333221...",
    "...2333332221...",
    "....23322221....",
    ".....122211.....",
    "......1111......",
    "................",
    "................",
    "................",
]


def ender_pearl():
    p = pal("#0a2a26", "#124a42", "#1e6e60", "#2e9282", "#5ac2b0", "#b8f0e6")
    return outline(from_map(ORB, pmap(p)))


def ender_eye():
    p = pal("#0c2a14", "#1a4e24", "#2e7a3a", "#4aa456", "#8ad890", "#d8f8d0")
    img = from_map(ORB, pmap(p))
    # a dark slit pupil
    for y in range(5, 11):
        img[y, 7, :3] = (14, 20, 12)
    return outline(img)


def chorus_fruit():
    rows = [
        "................",
        "................",
        "................",
        ".....33.44......",
        "....3443443.....",
        "...344444443....",
        "...34433444.3...",
        "..3443323443....",
        "..3433222343....",
        "..34322222433...",
        "...3322222333...",
        "...332232233....",
        "....3322233.....",
        ".....33333......",
        "................",
        "................",
    ]
    p = pal("#3a1e3e", "#5a3460", "#7a4e82", "#9a6aa2", "#c0a0c8")
    return outline(from_map(rows, pmap(p)))


def popped_chorus_fruit():
    img = chorus_fruit()
    rgb = img[..., :3].astype(np.float32)
    img[..., :3] = np.clip(rgb * 1.25 + 18, 0, 255)
    return img



def _sprite(rows, p, extra=None):
    cols = pmap(p)
    if extra:
        cols.update(extra)
    return outline(from_map(rows, cols))


def wheat_seeds():
    rows = ["................"] * 4 + [
        "......2.........",
        "...3.....2......",
        "........3...2...",
        ".....2..........",
        "..2.....3..3....",
        "......3.........",
        "...3.......2....",
        "........2.......",
    ] + ["................"] * 4
    return _sprite(rows, pal("#3a5a12", "#4e7418", "#6a9222", "#84ac30"))


def saddle():
    rows = [
        "................",
        "................",
        "................",
        "....3333333.....",
        "...344444443....",
        "..34445554443...",
        "..34455555443...",
        "..3444444444321.",
        "...33333333.21..",
        "......2...2.....",
        "......2...2.....",
        ".....121.121....",
        ".....111.111....",
        "................",
        "................",
        "................",
    ]
    return _sprite(rows, pal("#5a5a60", "#2e1a0e", "#4a2c16", "#6a4022", "#8a5a32"))


def nether_wart_item():
    rows = ["................"] * 3 + [
        "......33........",
        ".....3443.......",
        "....344443.33...",
        "....3444433443..",
        ".....33..344443.",
        "..33.....344443.",
        ".3443.....3333..",
        ".34443..33......",
        "..3333.3443.....",
        "......344443....",
        "......34443.....",
        ".......333......",
    ]
    return _sprite(rows, pal("#3a0a0e", "#5a1016", "#7a1a20", "#9a262c"))


def compass_item():
    rows = [
        "................",
        "................",
        ".....333333.....",
        "....34444443....",
        "...3445555443...",
        "...3455515543...",
        "...3455155543...",
        "...3451155543...",
        "...3452255543...",
        "...3455225543...",
        "...3455552543...",
        "...3445555443...",
        "....34444443....",
        ".....333333.....",
        "................",
        "................",
    ]
    return _sprite(rows, pal("#c02020", "#8a8a90", "#4a4a50", "#9a9aa0", "#d8d8dc"), {"1": (200, 30, 30), "2": (60, 60, 70)})


def clock_item():
    rows = [
        "................",
        "................",
        ".....333333.....",
        "....34444443....",
        "...3445555443...",
        "...3455525543...",
        "...3455525543...",
        "...3455521143...",
        "...3455555543...",
        "...3466666643...",
        "...3466666643...",
        "...3446666443...",
        "....34444443....",
        ".....333333.....",
        "................",
        "................",
    ]
    return _sprite(rows, pal("#202028", "#3a3a40", "#8a6a10", "#d0a828", "#f0e8c0"), {"6": (30, 40, 90), "1": (40, 40, 48)})


def spyglass_item():
    rows = [
        "................",
        "............44..",
        "...........4554.",
        "..........45554.",
        ".........45554..",
        "........3444....",
        ".......3443.....",
        "......3443......",
        ".....3443.......",
        "....3443........",
        "...3443.........",
        "..3223..........",
        ".3223...........",
        ".223............",
        "..3.............",
        "................",
    ]
    return _sprite(rows, pal("#5a2a14", "#7a3a1c", "#a85a2a", "#c87a3a", "#9a6ad0"))


def map_item():
    rows = [
        "................",
        "................",
        "..333333333333..",
        "..344444444443..",
        "..345556664443..",
        "..345566664443..",
        "..344666655543..",
        "..344466555543..",
        "..346666555443..",
        "..346665554443..",
        "..344445544443..",
        "..344444444443..",
        "..333333333333..",
        "................",
        "................",
        "................",
    ]
    return _sprite(rows, pal("#000000", "#000000", "#8a7a5a", "#e0d4b0", "#5a9a40"), {"6": (70, 110, 190)})


def boat_item(p):
    """A little rowing boat seen from the side, an oar across it."""
    rows = [
        "................",
        "................",
        "................",
        "................",
        "...........5....",
        "..........5.....",
        ".........5......",
        "1.......5......1",
        "14444445444444 1",
        "1433333533333341",
        ".14322252222341.",
        ".14322522222341.",
        "..143333333341..",
        "...1111111111...",
        "................",
        "................",
    ]
    rows = [r.replace(" ", "4") for r in rows]
    return outline(from_map(rows, pmap(pal("#1a120a", *p, "#3a2a1a"), "123456")))


def ink_sac():
    rows = [
        "................",
        "................",
        "......1111......",
        ".....122221.....",
        "....12333221....",
        "....12333321....",
        "...1233443321...",
        "...1234444321...",
        "...1233443321...",
        "....12333321....",
        "....11222211....",
        "...1.1.11.1.1...",
        "..1..1.1..1..1..",
        ".....1....1.....",
        "................",
        "................",
    ]
    return outline(from_map(rows, pmap(pal("#0a0a12", "#1a1a2a", "#2a2a40", "#4a4a6a"))))


DYES = {
    "white": "#e6e9e9", "orange": "#ea7a1c", "magenta": "#b84bb0",
    "light_blue": "#45a8d6", "yellow": "#f2c52c", "lime": "#74b928",
    "pink": "#e891aa", "gray": "#474c50", "light_gray": "#8f8f88",
    "cyan": "#1a8a92", "purple": "#7a30aa", "blue": "#363c9c",
    "brown": "#704a2c", "green": "#56701f", "red": "#a2302a",
    "black": "#1c1d22",
}


def dye(color):
    """A little heap of powdered dye."""
    rows = [
        "................",
        "................",
        "................",
        "................",
        "................",
        "......1111......",
        ".....143321.....",
        "....14332221....",
        "...1433322221...",
        "..143332222211..",
        "..133322222221..",
        ".13332222222221.",
        ".12222222222221.",
        "..111111111111..",
        "................",
        "................",
    ]
    base = np.array(hexrgb(DYES[color]), dtype=float)
    shades = ["#%02x%02x%02x" % tuple(int(v) for v in np.clip(base * k, 0, 255)) for k in (0.45, 0.85, 1.0, 1.25)]
    return outline(from_map(rows, pmap(pal(*shades))))


def lead_item():
    """A coiled rope with a loop."""
    rows = [
        "................",
        "................",
        "......1111......",
        ".....133331.....",
        "....13.11.31....",
        "....131..131....",
        "....13....31....",
        "....131..131....",
        ".....13..31.....",
        "......1331......",
        ".......13.......",
        ".......131......",
        "........131.....",
        ".........131....",
        "..........11....",
        "................",
    ]
    return outline(from_map(rows, pmap(pal("#3a2410", "#6a4a24", "#a07a44"))))


def name_tag():
    """A paper tag on a string."""
    rows = [
        "................",
        "................",
        "...........11...",
        "..........1..1..",
        ".........1...1..",
        "..1111111.1.1...",
        ".12222222111....",
        "1222333322221...",
        "12223222322221..",
        "1222333322221...",
        ".12222222221....",
        "..111111111.....",
        "................",
        "................",
        "................",
        "................",
    ]
    return outline(from_map(rows, pmap(pal("#4a4a4a", "#d8d0b4", "#8a8270"))))


def bone_meal():
    rows = ["................"] * 3 + [
        "......5.........",
        "....4.55..4.....",
        "...55.4.555.....",
        "..4555555445....",
        "...45555554.5...",
        "..5.4555543.....",
        "....345544......",
        "...5..3443..5...",
        ".....4..3.......",
    ] + ["................"] * 4
    return _sprite(rows, pal("#8c887a", "#b8b4a4", "#d0ccbc", "#e4e2d4", "#f8f6ec"))


def pumpkin_seeds():
    rows = ["................"] * 4 + [
        "......33........",
        "...3..44...33...",
        "..344.4...344...",
        "...3.......4....",
        "........33......",
        "..33...344..33..",
        ".344....3..344..",
        "..3.........3...",
    ] + ["................"] * 4
    return _sprite(rows, pal("#8a7a4a", "#b0a060", "#d8c88a", "#efe2ac"))


def melon_seeds():
    rows = ["................"] * 4 + [
        "......22........",
        "...2..33...22...",
        "..233.3...233...",
        "...2.......3....",
        "........22......",
        "..22...233..22..",
        ".233....2..233..",
        "..2.........2...",
    ] + ["................"] * 4
    return _sprite(rows, pal("#1a1410", "#2e241a", "#4a3a28", "#5e4a32"))


def melon_slice():
    rows = [
        "................",
        "................",
        "................",
        "...........1....",
        "..........151...",
        ".........15541..",
        "........155541..",
        ".......1544541..",
        "......15545441..",
        ".....155445441..",
        "....1554544551..",
        "...15554455521..",
        "..1555555552211.",
        ".122222222221...",
        "..11111111111...",
        "................",
    ]
    return _sprite(rows, pal("#2e5a14", "#5a9a2a", "#000000", "#1a1410", "#d83a2a"), {"5": (232, 70, 52)})


def sugar_cane_item():
    rows = [
        "......2...3.....",
        ".....2...3......",
        "....23..34......",
        ".....3..3.......",
        ".....4..4.......",
        ".....3..3.......",
        ".....3..3.......",
        ".....1..1.......",
        ".....3..3.......",
        ".....4..4.......",
        ".....3..3.......",
        ".....3..3.......",
        ".....1..1.......",
        ".....3..3.......",
        ".....3..3.......",
        "................",
    ]
    return _sprite(rows, pal("#4a6a20", "#6a9a30", "#8aba48", "#a8d468"))


def sugar():
    rows = ["................"] * 5 + [
        ".......4........",
        ".....4.44.4.....",
        "....4444444.....",
        "...444434344....",
        "...43443443.4...",
        "..4.4434444.....",
        "....3443433.....",
    ] + ["................"] * 4
    return _sprite(rows, pal("#a8a8b0", "#c8c8d0", "#dcdce4", "#f0f0f6"))


def paper():
    rows = ["................", "................"] + [
        "...44444444444..",
        "...43333333334..",
        "...43222222234..",
        "...43333333334..",
        "...43222222234..",
        "...43333333334..",
        "...43222222234..",
        "...43333333334..",
        "...43222223334..",
        "...43333333334..",
        "...44444444444..",
    ] + ["................"] * 3
    return _sprite(rows, pal("#9a9a8a", "#c8c8b8", "#e4e4d6", "#f6f6ec"))


def wheat_item():
    rows = [
        "...........4.4..",
        "..........4343..",
        ".........43434..",
        "........43434...",
        ".......4343.....",
        "......2.3.......",
        ".....2..........",
        "....2...........",
        "...2............",
        "..2.............",
        ".2..............",
        "2...............",
        "................",
        "................",
        "................",
        "................",
    ]
    return _sprite(rows, pal("#6a5010", "#8a6a16", "#b08c20", "#d0ac34", "#ecd060"))


def bread():
    rows = ["................"] * 4 + [
        "....22222222....",
        "..224444443322..",
        ".24443443444332.",
        ".24434434434432.",
        ".23444444444332.",
        "..233333333322..",
        "...2222222222...",
    ] + ["................"] * 5
    return _sprite(rows, pal("#5a3410", "#7a4818", "#9a5e22", "#c08038", "#dca054"))


def carrot():
    rows = [
        "............g.g.",
        "...........ggg..",
        "..........gg....",
        ".........433....",
        "........4433....",
        ".......44332....",
        "......4433......",
        ".....4332.......",
        "....4332........",
        "...332..........",
        "..32............",
        ".2..............",
        "................",
        "................",
        "................",
        "................",
    ]
    return _sprite(rows, pal("#6a2a06", "#a8440a", "#d86814", "#f08a24", "#ffae4a"), {"g": hexrgb("#3e8a22")})


def potato(baked=False):
    rows = ["................"] * 4 + [
        ".....22222......",
        "...224444322....",
        "..2443444432....",
        "..2444434432....",
        "..2344444332....",
        "...23343332.....",
        "....22222.......",
    ] + ["................"] * 5
    p = pal("#4a3010", "#6a4818", "#8a6224", "#c08a40", "#d8a858") if baked else pal("#5a4420", "#7a6030", "#a8844a", "#c8a464", "#dcc080")
    return _sprite(rows, p)


def beetroot():
    rows = [
        "......g.g.......",
        ".....ggg........",
        "......g.........",
        ".....3333.......",
        "....344433......",
        "...34444332.....",
        "...34443332.....",
        "...33443322.....",
        "....333322......",
        ".....2222.......",
        "......2.........",
        "................",
        "................",
        "................",
        "................",
        "................",
    ]
    return _sprite(rows, pal("#3a0a14", "#5a1020", "#7e1a30", "#a42a44", "#c4405c"), {"g": hexrgb("#3e7a22")})


def beetroot_seeds():
    rows = ["................"] * 5 + [
        "....2....3......",
        "........2...3...",
        "..3...2.........",
        "......3...2.....",
        "...2.......3....",
        "........2.......",
    ] + ["................"] * 5
    return _sprite(rows, pal("#3a2814", "#5a3e20", "#7a542c", "#9a6a3a"))


def nether_brick_item():
    return nether_tint(brick())


def nether_tint(img):
    out = img.copy()
    rgb = out[..., :3].astype(np.float32)
    l = rgb.mean(axis=2, keepdims=True)
    out[..., :3] = np.clip(np.concatenate([l * 0.75 + 10, l * 0.28, l * 0.3], axis=2), 0, 255)
    return out


# --------------------------------------------------------------------------
# Food
# --------------------------------------------------------------------------


def apple():
    rows = [
        "................",
        "........s.......",
        "........sll.....",
        "........slll....",
        "....3333s333....",
        "...334443333....",
        "..33455443332...",
        "..33454433332...",
        "..33444333322...",
        "..33343333222...",
        "..33333333222...",
        "...333332222....",
        "...33322222.....",
        "....332.221.....",
        "................",
        "................",
    ]
    p = pal("#5a0a08", "#9a1410", "#c8221a", "#e64434", "#ff9a88")
    cols = pmap(p)
    cols["s"] = hexrgb("#4a3018")
    cols["l"] = hexrgb("#3e9a2a")
    return outline(from_map(rows, cols))


CHOP = [
    "................",
    "................",
    "................",
    "........ffff....",
    "......ff4444f...",
    ".....f44444443..",
    "....f444554443..",
    "...f4445544433..",
    "...f444444433...",
    "..f444444433b...",
    "..f44444433bb...",
    "..f3333333bb....",
    "...33333....",
    "................",
    "................",
    "................",
]


def chop(name, p, fat, bonec=None):
    rows = [r.ljust(N, ".") for r in CHOP]
    cols = pmap(p)
    cols["f"] = fat
    cols["b"] = bonec if bonec is not None else hexrgb("#ecead8")
    return outline(from_map(rows, cols))


def porkchop():
    return chop("porkchop", pal("#7a3038", "#b0505a", "#d06c76", "#e88a92",
                                "#f8b4b8"), hexrgb("#fde0e0"))


def cooked_porkchop():
    return chop("cooked_porkchop", pal("#4a2a14", "#6a3e1e", "#8a5428",
                                       "#a86e36", "#c48a4c"), hexrgb("#e8c890"))


STEAK = [
    "................",
    "................",
    "................",
    "................",
    ".....ffffff.....",
    "...ff44444fff...",
    "..f4445444444f..",
    "..f44544w44443..",
    "..f444w4444543..",
    "..f44444445433..",
    "...f4444w44333..",
    "...f33333333....",
    "....33333.......",
    "................",
    "................",
    "................",
]


def steak(name, p, fat, marb):
    cols = pmap(p)
    cols["f"] = fat
    cols["w"] = marb
    return outline(from_map(STEAK, cols))


def beef():
    return steak("beef", pal("#5a0c0c", "#8a1a18", "#b02a24", "#c83a32",
                             "#e06a5e"), hexrgb("#f0d6c8"), hexrgb("#f4c8bc"))


def cooked_beef():
    return steak("cooked_beef", pal("#2e1a0c", "#4a2c14", "#66401e", "#7e5228",
                                    "#9a6a38"), hexrgb("#b88a50"), hexrgb("#a8784a"))


DRUMSTICK = [
    "................",
    "................",
    "......5555......",
    ".....544444.....",
    "....54444443....",
    "....44444443....",
    "....44444433....",
    "....44444333....",
    ".....4443332....",
    "......33322.....",
    ".......bb.......",
    "......bb........",
    ".....bb.........",
    "...bbb..........",
    "...bb...........",
    "................",
]


def drumstick(name, p, bonec):
    cols = pmap(p)
    cols["b"] = bonec
    return outline(from_map(DRUMSTICK, cols))


def chicken():
    return drumstick("chicken", pal("#9a6a5a", "#c49484", "#e4b8a8", "#f4d0c2",
                                    "#fff0e8"), hexrgb("#ecead8"))


def cooked_chicken():
    return drumstick("cooked_chicken", pal("#5a3410", "#8a5420", "#b87a34",
                                           "#d89a48", "#f0c070"), hexrgb("#ecead8"))


MUTTON = [
    "................",
    "................",
    "................",
    "...........bb...",
    "..........bbb...",
    ".......ff3bb....",
    ".....ff44433....",
    "....f4454443....",
    "...f44544443....",
    "...f44444433....",
    "...f4444w433....",
    "....f444433.....",
    ".....33333......",
    "................",
    "................",
    "................",
]


def mutton_item(name, p, fat):
    cols = pmap(p)
    cols["f"] = fat
    cols["w"] = fat
    cols["b"] = hexrgb("#ecead8")
    return outline(from_map(MUTTON, cols))


def mutton():
    return mutton_item("mutton", pal("#6a1418", "#9a2a2e", "#c0423e", "#d65a52",
                                     "#ec8a80"), hexrgb("#f4e0d8"))


def cooked_mutton():
    return mutton_item("cooked_mutton", pal("#3a200e", "#5a3418", "#7a4a24",
                                            "#946030", "#b07c44"), hexrgb("#c89a5e"))


# --------------------------------------------------------------------------
# Placeables
# --------------------------------------------------------------------------

BED_RED = pal("#5a0c0c", "#8a1616", "#b22222", "#cc3a32", "#e0605a")
PILLOW = pal("#b8b8b0", "#dcdcd4", "#f4f4ee")
BED_WOOD = WOODS["oak"]["plank"]


def red_bed_item():
    rows = [
        "................",
        "................",
        "................",
        "................",
        "................",
        "...pPPP.........",
        "..pPPPP5555555..",
        "..pppp44444443..",
        "..4444444444433.",
        "..33333333333322",
        "..wwwwwwwwwwwwd.",
        "..W..........W..",
        "..W..........W..",
        "................",
        "................",
        "................",
    ]
    rows[9] = "..33333333333322"[:N]
    rows[8] = "..44444444444333"[:N]
    rows = [r[:N].ljust(N, ".") for r in rows]
    cols = pmap(BED_RED)
    cols.update({"p": PILLOW[1], "P": PILLOW[2], "w": BED_WOOD[2],
                 "W": BED_WOOD[1], "d": BED_WOOD[0]})
    return outline(from_map(rows, cols))


def sapling(wood):
    rng = rng_for(wood + "_sapling")
    leaves = WOODS[wood]["leaves"]
    stem = WOODS[wood]["bark"] if wood != "birch" else pal(
        "#2a2a26", "#9a988e", "#cfcdc5", "#e9e8e2", "#ffffff")
    img = canvas()
    if wood == "spruce":
        rows = [
            "................",
            ".......4........",
            "......443.......",
            ".....44332......",
            "......433.......",
            ".....443322.....",
            "....44433322....",
            "......4332......",
            ".....4433322....",
            "...444333322....",
            "..444433332221..",
            ".....44s322.....",
            ".......s........",
            ".......s........",
            ".......s........",
            ".......s........",
        ]
        lp = leaves
    elif wood == "acacia":
        # a flat umbrella crown on a bent stem
        rows = [
            "................",
            "................",
            "................",
            "...4455443......",
            "..44554433322...",
            ".4443333333221..",
            "....32.s.22.....",
            ".......s........",
            "......s.........",
            "......s..4432...",
            ".....s..443322..",
            ".....s....s.....",
            "......s..s......",
            ".......ss.......",
            ".......s........",
            ".......s........",
        ]
        lp = leaves
    else:
        rows = [
            "................",
            "................",
            "......455.......",
            "....44554433....",
            "...4455443332...",
            "..445443433322..",
            "..4444333.3322..",
            "...44.33s3322...",
            "....43..s.22....",
            "..4433..s.......",
            "..44322.s..43...",
            "...322..s.4332..",
            "........ss322...",
            "........s.......",
            "........s.......",
            "........s.......",
        ]
        lp = leaves
    cols = {"1": lp[0], "2": lp[1], "3": lp[2], "4": lp[3], "5": lp[4],
            "s": stem[2]}
    img = from_map(rows, cols)
    # a little colour jitter so the three saplings don't look stamped
    m = (img[..., 3] > 0) & (rng.random((N, N)) < 0.12)
    img[m, :3] = scale_rgb(img[m, :3], 0.88)
    if wood == "birch":
        # birch stem: white with dark marks
        for y in range(7, 16):
            if img[y, 8, 3] > 0 and y % 3 == 0:
                put(img, 8, y, stem[0])
    return img


def propagule():
    """The mangrove's seedling: a long green spindle under a small tuft of leaves."""
    leaves = WOODS["mangrove"]["leaves"]
    rows = [
        "................",
        "......4..4......",
        ".....443344.....",
        "....44322334....",
        ".....3s.s32.....",
        ".......ss.......",
        ".......gg.......",
        ".......GG.......",
        ".......Gg.......",
        ".......GG.......",
        ".......Gg.......",
        ".......GG.......",
        ".......Gg.......",
        "........G.......",
        "........g.......",
        "................",
    ]
    cols = {"2": leaves[1], "3": leaves[2], "4": leaves[3], "s": WOODS["mangrove"]["bark"][3],
            "g": hexrgb("#4f7a26"), "G": hexrgb("#6a9632")}
    return from_map(rows, cols)


# --------------------------------------------------------------------------
# New block textures
# --------------------------------------------------------------------------

CHEST = pal("#3a2410", "#5a3a1a", "#7a5226", "#966a34", "#ae8044")
LATCH = pal("#3a3a3a", "#8a8a8a", "#c8c8c8")


def chest_base(name):
    rng = rng_for(name)
    grain = 0.6 * value_noise(rng, 8, 1) + 0.4 * value_noise(rng, 4, 1)
    img = from_rgb(shade(normalize(grain) * 0.7 + 0.2, CHEST[2:], 0.5))
    rgb = img[..., :3]
    for y in (4, 9, 14):
        rgb[y, :] = scale_rgb(rgb[y, :], 0.82)
    # dark frame
    rgb[0, :] = CHEST[0]
    rgb[15, :] = CHEST[0]
    rgb[:, 0] = CHEST[0]
    rgb[:, 15] = CHEST[0]
    rgb[1, 1:15] = scale_rgb(rgb[1, 1:15], 1.1)
    rgb[1:15, 1] = scale_rgb(rgb[1:15, 1], 1.08)
    rgb[14, 1:15] = scale_rgb(rgb[14, 1:15], 0.85)
    rgb[1:15, 14] = scale_rgb(rgb[1:15, 14], 0.85)
    return img


def chest_top():
    img = chest_base("chest_top")
    rgb = img[..., :3]
    # metal-free lid: a darker inner band around the lid boards
    rgb[2, 2:14] = CHEST[1]
    rgb[13, 2:14] = CHEST[1]
    rgb[2:14, 2] = CHEST[1]
    rgb[2:14, 13] = CHEST[1]
    return img


def chest_side(front=False):
    img = chest_base("chest_front" if front else "chest_side")
    rgb = img[..., :3]
    # lid seam: dark band separating lid (top 5 rows) from the body
    rgb[5, :] = CHEST[0]
    rgb[6, 1:15] = CHEST[1]
    rgb[4, 1:15] = scale_rgb(rgb[4, 1:15], 0.9)
    if front:
        # small grey latch hanging over the seam
        for y in range(3, 9):
            for x in range(7, 9):
                put(img, x, y, LATCH[1])
        for x in range(6, 10):
            put(img, x, 3, LATCH[2] if x < 9 else LATCH[1])
            put(img, x, 8, LATCH[0])
        put(img, 6, 4, LATCH[2])
        for y in range(4, 8):
            put(img, 6, y, LATCH[2])
            put(img, 9, y, LATCH[0])
        put(img, 7, 6, LATCH[0])   # keyhole
        put(img, 8, 6, LATCH[0])
    return img


def bed_blanket(rng, h=N):
    f = value_noise(rng, 4, 2) * 0.6 + rng.random((N, N)) * 0.4
    return shade(normalize(f) * 0.6 + 0.25, BED_RED[1:], 0.5)


def red_bed_top():
    rng = rng_for("red_bed_top")
    img = from_rgb(bed_blanket(rng))
    rgb = img[..., :3]
    # pillow strip at the head end
    rgb[1:5, 2:14] = PILLOW[2]
    rgb[4, 2:14] = PILLOW[1]
    rgb[1:5, 13] = PILLOW[1]
    rgb[1:5, 2] = PILLOW[2]
    rgb[1, 2:14] = PILLOW[2]
    # blanket fold line under the pillow
    rgb[5, 1:15] = BED_RED[4]
    rgb[6, 1:15] = BED_RED[3]
    # side hem of the blanket
    rgb[7:15, 1] = BED_RED[3]
    rgb[7:15, 14] = BED_RED[1]
    # wooden frame edge
    w = BED_WOOD
    rgb[0, :] = w[1]
    rgb[15, :] = w[0]
    rgb[:, 0] = w[2]
    rgb[:, 15] = w[0]
    rgb[1:5, 1] = w[2]
    return img


def red_bed_side():
    rng = rng_for("red_bed_side")
    img = canvas()
    blank = bed_blanket(rng)
    img[0:6, :, :3] = blank[0:6]
    img[0, :, :3] = BED_RED[4]
    img[5, :, :3] = BED_RED[1]
    # blanket hangs a little over the frame in a wavy hem
    for x in range(N):
        if x % 4 in (1, 2):
            img[6, x, :3] = BED_RED[1]
        else:
            img[6, x, :3] = BED_WOOD[3]
    img[7, :, :3] = BED_WOOD[2]
    img[8, :, :3] = BED_WOOD[0]
    img[6:9, 0, :3] = BED_WOOD[1]
    img[6:9, 15, :3] = BED_WOOD[0]
    img[0:9, :, 3] = 255
    # the game shows the BOTTOM rows of a low block's side texture, so move
    # the 9 px bed band to y = 7..15 and leave y = 0..6 transparent
    return np.roll(img, N - 9, axis=0)


def red_bed_end():
    img = canvas()
    w = BED_WOOD
    rng = rng_for("red_bed_end")
    grain = value_noise(rng, 8, 1)
    img[0:9, :, :3] = shade(normalize(grain) * 0.6 + 0.2, w[1:4], 0.5)[0:9]
    img[0, :, :3] = w[4]
    img[8, :, :3] = w[0]
    img[0:9, 0, :3] = w[3]
    img[0:9, 15, :3] = w[0]
    # headboard posts and a red strip of blanket peeking out
    img[1:8, 1, :3] = w[3]
    img[1:8, 14, :3] = w[1]
    img[2:4, 2:14, :3] = BED_RED[2]
    img[2, 2:14, :3] = BED_RED[3]
    img[4, 2:14, :3] = w[0]
    img[0:9, :, 3] = 255
    # the game shows the BOTTOM rows of a low block's side texture, so move
    # the 9 px bed band to y = 7..15 and leave y = 0..6 transparent
    return np.roll(img, N - 9, axis=0)


def xp_orb():
    img = canvas()
    cx, cy = 7.5, 7.5
    d = np.hypot(XX - cx, YY - cy)
    p = pal("#3a7a10", "#6ab81c", "#a4e030", "#e0f850", "#ffffb0")
    glow = (d <= 6.4) & (d > 5.0)
    img[glow, :3] = p[1]
    img[glow, 3] = 120
    core = d <= 5.0
    lvl = np.clip((5.0 - np.hypot(XX - 6.0, YY - 6.0)) * 0.85, 0, 4).astype(int)
    img[core, :3] = p[lvl[core]]
    img[core, 3] = 255
    rim = core & (d > 4.1)
    img[rim, :3] = scale_rgb(img[rim, :3], 0.75)
    img[rim & (XX + YY > 15), :3] = p[0]
    put(img, 5, 5, p[4])
    put(img, 6, 5, p[4])
    put(img, 5, 6, p[4])
    return img


# --------------------------------------------------------------------------
# Armor
# --------------------------------------------------------------------------

ARMOR_MATERIALS = {
    "leather": pal("#3a200c", "#5e3618", "#7e4c24", "#9c6432", "#bc8248"),
    "iron": MATERIALS["iron"],
    "golden": MATERIALS["golden"],
    "diamond": MATERIALS["diamond"],
    "copper": MATERIALS["copper"],
    "bronze": MATERIALS["bronze"],
    "lead": MATERIALS["lead"],
    "silver": MATERIALS["silver"],
    "tungsten": MATERIALS["tungsten"],
    "platinum": MATERIALS["platinum"],
}

ARMOR_KINDS = ("helmet", "chestplate", "leggings", "boots")

ARMOR_SHAPES = {
    # cap with a face opening at the bottom
    "helmet": [
        "................",
        "................",
        "................",
        ".....######.....",
        "...##########...",
        "..############..",
        "..############..",
        "..############..",
        "..############..",
        "..####....####..",
        "..###......###..",
        "..###......###..",
        "................",
        "................",
        "................",
        "................",
    ],
    # torso with shoulders, short sleeves and a neck cut-out
    "chestplate": [
        "................",
        "..####....####..",
        ".#####....#####.",
        ".######..######.",
        ".##############.",
        ".##############.",
        ".##.########.##.",
        ".##.########.##.",
        "....########....",
        "....########....",
        "....########....",
        "....########....",
        "....########....",
        "....########....",
        "................",
        "................",
    ],
    # waist band and two legs
    "leggings": [
        "................",
        "................",
        "...##########...",
        "...##########...",
        "...##########...",
        "...####..####...",
        "...####..####...",
        "...####..####...",
        "...####..####...",
        "...####..####...",
        "...####..####...",
        "...####..####...",
        "...####..####...",
        "...####..####...",
        "................",
        "................",
    ],
    # two boots side by side, toes pointing outwards
    "boots": [
        "................",
        "................",
        "................",
        "................",
        "..####....####..",
        "..####....####..",
        "..####....####..",
        "..####....####..",
        "..####....####..",
        "..####....####..",
        ".#####....#####.",
        ".#####....#####.",
        "................",
        "................",
        "................",
        "................",
    ],
}


def armor(material, kind):
    rng = rng_for(f"{material}_{kind}")
    p = ARMOR_MATERIALS[material]
    img = canvas()
    m = mask_from_rows(ARMOR_SHAPES[kind])
    shade_mask(img, m, p[1:], rng, speck=0.08 if material == "leather" else 0.0)
    leather = material == "leather"
    if kind == "helmet":
        # rim band just above the face opening and the cheek guards
        for x in range(3, 13):
            put(img, x, 8, p[1])
        img[9, 2:6, :3] = p[2]
        img[9, 10:14, :3] = p[2]
        put(img, 2, 9, p[4])
        put(img, 10, 9, p[3])
        if not leather:
            # raised centre ridge on the crown
            for y in range(4, 8):
                put(img, 7, y, p[4])
                put(img, 8, y, p[2])
            put(img, 4, 6, p[4])
            put(img, 11, 6, p[2])
    elif kind == "chestplate":
        # collar trim around the neck cut-out
        put(img, 6, 3, p[1])
        put(img, 9, 3, p[1])
        img[4, 7:9, :3] = p[1]
        # centre seam and a bottom hem
        for y in range(5, 13):
            put(img, 8, y, p[2])
        img[13, 5:11, :3] = p[1]
        if leather:
            # stitched lacing down the front
            for y in (6, 8, 10):
                put(img, 7, y, p[4])
                put(img, 8, y, p[1])
        else:
            # plate edge line under the shoulders
            img[5, 2:14, :3] = p[2]
            put(img, 1, 5, p[3])
            img[5, 7:9, :3] = p[1]
    elif kind == "leggings":
        # waist band
        img[4, 4:12, :3] = p[1]
        img[2, 3:13, :3] = p[4]
        if leather:
            put(img, 7, 3, p[4])   # little belt buckle / lace
            put(img, 8, 3, p[1])
        else:
            img[3, 7:9, :3] = p[1]
        # knee plates
        for x0 in (4, 10):
            put(img, x0, 9, p[4])
            put(img, x0 + 1, 9, p[3])
    elif kind == "boots":
        # light cuff at the top, darker sole at the bottom
        img[4, 2:6, :3] = p[4]
        img[4, 10:14, :3] = p[4]
        img[5, 3:5, :3] = p[1]
        img[5, 11:13, :3] = p[1]
        img[11, 1:6, :3] = p[0] if leather else p[1]
        img[11, 10:15, :3] = p[0] if leather else p[1]
    return outline(img)


# --------------------------------------------------------------------------
# Registry & output
# --------------------------------------------------------------------------


def item_registry():
    reg = {}
    for mat in MATERIALS:
        for kind in TOOL_KINDS:
            reg[f"{mat}_{kind}"] = lambda m=mat, k=kind: tool(m, k)
    for mat in ("iron", "diamond", "platinum", "tungsten"):
        if mat in MATERIALS:
            for kind in ("hammer", "excavator", "sickle"):
                reg[f"{mat}_{kind}"] = lambda m=mat, k=kind: tool(m, k)
    reg.update({
        "stick": stick, "coal": coal, "charcoal": charcoal,
    })
    for metal in RAW:
        reg[f"raw_{metal}"] = lambda m=metal: raw(m)
    for metal in INGOT_COL:
        reg[f"{metal}_ingot"] = lambda m=metal: ingot(m)
    for g in GEMS:
        reg[g] = lambda n=g: gem(n)
    reg["life_crystal"] = life_crystal_item
    for g in WAND_GEMS:
        reg[f"{g}_wand"] = lambda n=g: wand(n)
    reg["fire_staff"] = lambda: staff("ruby", "golden")
    reg["frost_staff"] = lambda: staff("sapphire", "silver")
    reg["thunder_tome"] = lambda: tome(("#1c2a5a", "#2c4290"), ("#f2c22c", "#fff6c8"))
    reg["flame_tome"] = lambda: tome(("#4a0c0c", "#8a1a14"), ("#f07a20", "#ffe080"))
    reg["mana_crystal"] = mana_crystal
    reg["ancient_seed"] = ancient_seed
    reg["lich_phylactery"] = lich_phylactery
    reg["glow_berries"] = glow_berries
    reg["blaze_rod"] = blaze_rod
    reg["lute"] = lute
    reg["flute"] = flute
    reg["cello"] = cello
    reg["obsidian_heart"] = obsidian_heart
    reg["void_seed"] = void_seed
    reg["guardian_heart"] = guardian_heart
    reg["guardian_staff"] = lambda: staff("emerald", "golden")
    reg.update({
        "diamond": diamond, "emerald": emerald, "lapis_lazuli": lapis,
        "redstone": redstone, "flint": flint, "string": string_item,
        "flint_and_steel": flint_and_steel,
        "honeycomb": honeycomb, "minecart": minecart_item,
        "gold_nugget": gold_nugget, "quartz": quartz, "nether_brick": nether_brick_item,
        "oak_boat": lambda: boat_item(('#4a3018', '#7a5430', '#a07848', '#c09a62')), "spruce_boat": lambda: boat_item(('#2e1e0e', '#4e3418', '#6e4c26', '#8a6436')), "birch_boat": lambda: boat_item(('#7a6c48', '#b4a46e', '#d0c28a', '#e6daa6')), "jungle_boat": lambda: boat_item(('#4a2c1a', '#7a4c30', '#9a6644', '#b6805a')), "acacia_boat": lambda: boat_item(('#6a2e14', '#a24a24', '#c46434', '#dc804a')), "dark_oak_boat": lambda: boat_item(('#1e140a', '#3a2612', '#52381e', '#6a4a2a')), "cherry_boat": lambda: boat_item(('#7a4a48', '#c8908a', '#e0aca4', '#f2c8c0')), "mangrove_boat": lambda: boat_item(('#3e160f', '#6a271d', '#86352a', '#a04436')), "ink_sac": ink_sac, "lead": lead_item, "name_tag": name_tag, "bone_meal": bone_meal, **{f"{c}_dye": (lambda c=c: dye(c)) for c in DYES}, "compass": compass_item, "clock": clock_item, "spyglass": spyglass_item, "map": map_item, "nether_wart": nether_wart_item, "saddle": saddle, "pumpkin_seeds": pumpkin_seeds, "melon_seeds": melon_seeds, "melon_slice": melon_slice,
        "sugar_cane": sugar_cane_item, "sugar": sugar, "paper": paper,
        "slime_ball": lambda: lump("slime_ball", pal("#2a7a1e", "#3a9a2a", "#52b23a", "#6cc84e", "#8ade6a")),
        "turtle_scute": lambda: lump("turtle_scute", pal("#1e3a14", "#2a4e1c", "#3a6426", "#4a7a30", "#5e9040")),
        "wheat_seeds": wheat_seeds, "wheat": wheat_item, "bread": bread, "carrot": carrot, "potato": potato,
        "baked_potato": lambda: potato(True), "beetroot": beetroot, "beetroot_seeds": beetroot_seeds,
        "ender_pearl": ender_pearl, "ender_eye": ender_eye, "chorus_fruit": chorus_fruit, "popped_chorus_fruit": popped_chorus_fruit,
        "leather_backpack": lambda: backpack(("#5a3a1a", "#a0703a")),
        "iron_backpack": lambda: backpack(("#7a7a80", "#d0d0d6")),
        "gold_backpack": lambda: backpack(("#a07810", "#f4d050")),
        "diamond_backpack": lambda: backpack(("#1a8a8a", "#6ae8e0")),
        "feather": feather, "leather": leather, "bone": bone,
        "gunpowder": gunpowder, "rotten_flesh": rotten_flesh,
        "clay_ball": clay_ball, "brick": brick,
        "apple": apple, "porkchop": porkchop,
        "cooked_porkchop": cooked_porkchop, "beef": beef,
        "cooked_beef": cooked_beef, "chicken": chicken,
        "cooked_chicken": cooked_chicken, "mutton": mutton,
        "cooked_mutton": cooked_mutton,
        "red_bed": red_bed_item,
    })
    for wood in ("oak", "birch", "spruce", "jungle", "acacia", "dark_oak", "cherry"):
        reg[f"{wood}_sapling"] = lambda w=wood: sapling(w)
    reg["mangrove_propagule"] = propagule
    for mat in ARMOR_MATERIALS:
        for kind in ARMOR_KINDS:
            reg[f"{mat}_{kind}"] = lambda m=mat, k=kind: armor(m, k)
    return reg


def block_registry():
    reg = {
        "chest_top": chest_top,
        "chest_side": lambda: chest_side(False),
        "chest_front": lambda: chest_side(True),
        "red_bed_top": red_bed_top, "red_bed_side": red_bed_side,
        "red_bed_end": red_bed_end, "xp_orb": xp_orb,
    }
    for wood in ("oak", "birch", "spruce", "jungle", "acacia", "dark_oak", "cherry"):
        reg[f"{wood}_sapling"] = lambda w=wood: sapling(w)
    reg["mangrove_propagule"] = propagule
    return reg


def main():
    sheet_path = sys.argv[1] if len(sys.argv) > 1 else None
    os.makedirs(ITEM_DIR, exist_ok=True)
    items = []
    for name, fn in item_registry().items():
        img = to_image(fn())
        assert img.size == (N, N)
        img.save(os.path.join(ITEM_DIR, name + ".png"))
        items.append(img)
    blocks = []
    for name, fn in block_registry().items():
        img = to_image(fn())
        assert img.size == (N, N)
        img.save(os.path.join(BLOCK_DIR, name + ".png"))
        blocks.append(img)
    if sheet_path:
        contact_sheet(items + blocks, scale=8, gap=4, cols=10).save(sheet_path)
    print(f"wrote {len(items)} items to {ITEM_DIR}, "
          f"{len(blocks)} blocks to {BLOCK_DIR}")


if __name__ == "__main__":
    main()
