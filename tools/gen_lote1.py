#!/usr/bin/env python3
"""Lote 1: original 16x16 designs for a batch of new blocks and items.

Everything is drawn from scratch in code (no art copied from Minecraft or any pack). Some
designs are composed from this repo's own 16x16 designs in tools/tex64/src16 (for example the
deepslate ores are our deepslate with the ore specks of our ores, the white bed is our red bed
re-coloured to white wool).

Usage:
    python3 tools/gen_lote1.py [--force]
    python3 tools/tex64/build.py --only <names printed at the end>

Writes 16x16 PNGs to packs/vanilla/textures/{block,item}/<name>.png. build.py then archives them
in tools/tex64/src16/ and writes the 64x64 versions back into the pack. Without --force the
script refuses to overwrite a texture that already exists in the pack; with --force it only ever
overwrites the names listed here (to re-run after editing a design: --force, then build.py).
"""

import math
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_textures as gt  # noqa: E402
from gen_items import (  # noqa: E402
    ARMOR_SHAPES, NUGGET, lump, mask_from_rows, outline, put, shade_mask,
)
from gen_textures import (  # noqa: E402
    N, ROOT, bevel, canvas, fbm, from_rgb, normalize, pal, rng_for, scale_rgb, shade,
    value_noise,
)

PACK = os.path.join(ROOT, "packs", "vanilla", "textures")
SRC16 = os.path.join(ROOT, "tools", "tex64", "src16")
YY, XX = np.mgrid[0:N, 0:N]


# --------------------------------------------------------------------------
# helpers
# --------------------------------------------------------------------------

def load(kind, name):
    """One of our existing 16x16 designs as float RGBA."""
    p = os.path.join(SRC16, kind, name + ".png")
    return np.asarray(Image.open(p).convert("RGBA"), dtype=np.float64).copy()


def lum(rgb):
    return np.asarray(rgb, dtype=np.float64) @ np.array([0.3, 0.59, 0.11])


def ramp_at(p, t):
    """Colour at 0..1 along a palette, quantised to its steps."""
    i = np.clip(np.floor(np.asarray(t) * len(p)), 0, len(p) - 1).astype(int)
    return p[i]


def recolor(img, mask, p, lo=None, hi=None):
    """Re-colour masked pixels onto palette p by their brightness (keeps the shading)."""
    L = lum(img[..., :3])
    lo = L[mask].min() if lo is None else lo
    hi = L[mask].max() if hi is None else hi
    t = np.clip((L - lo) / (hi - lo + 1e-9), 0, 0.999)
    img[mask, :3] = ramp_at(p, t[mask])
    return img


def flatten(img, k):
    """Pull colours toward the image mean (smooth variants of a block)."""
    m = img[..., :3].reshape(-1, 3).mean(0)
    img[..., :3] = m + (img[..., :3] - m) * k
    return img


def wput(img, x, y, c, a=255):
    put(img, x % N, y % N, c, a)


def wline(img, x0, y0, x1, y1, c):
    """Bresenham line that wraps around the tile (for seamless textures)."""
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        wput(img, x0, y0, c)
        if x0 == x1 and y0 == y1:
            break
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy


def from_map(rows, cols):
    img = canvas()
    assert len(rows) == N, len(rows)
    for y, row in enumerate(rows):
        assert len(row) == N, (y, row)
        for x, ch in enumerate(row):
            if ch != ".":
                put(img, x, y, cols[ch])
    return img


def blank_rows(n):
    return ["." * N] * n


def edges(mask):
    """(lit, dark) edge masks of a shape: lit faces top/left, dark faces bottom/right."""
    pad = np.pad(mask, 1)
    up, down = pad[:-2, 1:-1], pad[2:, 1:-1]
    left, right = pad[1:-1, :-2], pad[1:-1, 2:]
    return mask & (~up | ~left), mask & (~down | ~right)


# --------------------------------------------------------------------------
# palettes
# --------------------------------------------------------------------------

DIRT_PAL = gt.DIRT
PODZOL = pal("#4a2e16", "#58381b", "#674220", "#764d27", "#85592f", "#946637")
ROOT_PAL = pal("#7e6a50", "#9a8466", "#b29d7e")
PEBBLE = pal("#4e4339", "#62574b", "#786c5e", "#8a7e6e")
PATH = pal("#6c5634", "#776039", "#826a41", "#8c7348", "#967c50")
RED_SANDSTONE = pal("#8e4018", "#a04c20", "#b05828", "#bd6431", "#c8703a")
DEEP_COAL = pal("#08080a", "#0f0f12", "#1c1c21")
COAL_BLOCK = pal("#111114", "#17171b", "#1e1e23", "#26262c", "#2f2f36", "#3a3a42")
WOOD = pal("#4a3018", "#6a4826", "#866034", "#a07a46", "#b89058")
DARK_WOOD = pal("#2e1c0e", "#402814", "#52341b", "#634022", "#744d2a", "#845a33")
IRON = pal("#3e4046", "#565960", "#73767d", "#8f9298", "#aaacb1", "#c4c6ca", "#dcdde0")
DARK_IRON = pal("#14161b", "#1e2128", "#2a2e37", "#383d48", "#4a505d", "#5e6574")
KELP = pal("#26321a", "#34441f", "#445626", "#566a2e", "#687c36")
KELP_BULB = pal("#6a6a2c", "#8a8a3c")
DRIED = pal("#1a2212", "#212c16", "#29361a", "#31401e", "#3a4a23")
TWINE = pal("#5a4a2a", "#76633a", "#8e7a4a")
BAMBOO = pal("#3a5a1a", "#4c7224", "#62902e", "#7aaa3c", "#92c050")
BAMBOO_LEAF = pal("#2e5a1e", "#3e7428", "#518c34", "#64a040")
CREAM = pal("#bdb3a2", "#d3cabb", "#e3dcd0", "#eee9e0", "#f7f4ee")
BERRY = pal("#5e1216", "#8a1e22", "#b2302e", "#d45a50")
UNRIPE = pal("#4e5a20", "#6e7a2c", "#9a6a30", "#b0563a")
SPONGE_CAKE = pal("#7a4e26", "#956130", "#ab733c", "#bf8649", "#cd9858")
LEAF = pal("#203a1c", "#2a4c22", "#365e2a", "#447034", "#52823e")
FIRE = pal("#9c3a1a", "#c45622", "#e07c30", "#eca244", "#f2c462", "#f6dc94")
SOUL = pal("#1a3e5c", "#22617e", "#2e8aa0", "#4cb0bc", "#82d0d2", "#c0eae8")
WHITE_WOOL = pal("#8a9095", "#a6abaf", "#bec2c4", "#d0d3d4", "#dcdfdf", "#e6e8e8")
GOLD = pal("#6a4a0a", "#946c14", "#be9020", "#deb43a", "#f2d46c", "#fbeaa4")
IRON_ITEM = pal("#4e5056", "#7a7d84", "#a6a9ae", "#c8cacd", "#e4e5e6")
CHAIN = pal("#3c3e46", "#5e616a", "#7e818a", "#9ea1a8", "#babdc2")
BOWL = {k: pal(v)[0] for k, v in {"R": "#b48e56", "r": "#8c6c3c", "i": "#4a3018",
                                  "h": "#ba935c", "o": "#9a7442", "d": "#6e5028"}.items()}


# --------------------------------------------------------------------------
# blocks: ground
# --------------------------------------------------------------------------

def coarse_dirt():
    rng = rng_for("coarse_dirt")
    img = load("block", "dirt")
    # a few darker, rougher dirt clumps
    f = value_noise(rng, 4) * 0.6 + value_noise(rng, 2) * 0.4
    dark = normalize(f) > 0.72
    img[dark, :3] = scale_rgb(img[dark, :3], 0.88)
    # small pebbles of 2-4 pixels, lit from the top-left, with a shadow below-right
    shapes = [((0, 0), (1, 0)), ((0, 0), (1, 0), (0, 1)), ((0, 0), (1, 0), (0, 1), (1, 1)),
              ((0, 0), (0, 1)), ((0, 0), (1, 0), (1, 1))]
    spots = [(2, 2), (9, 1), (13, 5), (5, 7), (10, 10), (1, 12), (6, 13), (14, 13), (12, 2)]
    for i, (cx, cy) in enumerate(spots):
        sh = shapes[int(rng.integers(len(shapes)))]
        pix = {(cx + dx, cy + dy) for dx, dy in sh}
        for (x, y) in pix:
            for sx, sy in ((1, 0), (0, 1), (1, 1)):
                q = (x + sx, y + sy)
                if q not in pix:
                    wput(img, *q, scale_rgb(img[q[1] % N, q[0] % N, :3], 0.8))
        for (x, y) in pix:
            tl = (x - 1, y) not in pix and (x, y - 1) not in pix
            br = (x + 1, y) not in pix and (x, y + 1) not in pix
            c = PEBBLE[3] if tl else (PEBBLE[1] if br else PEBBLE[2])
            wput(img, x, y, c)
    return img


def podzol_top():
    rng = rng_for("podzol_top")
    f = fbm(rng, ((4, 0.5), (2, 0.4), (1, 0.2)))
    img = from_rgb(shade(0.15 + 0.55 * f, PODZOL[:5], 0.6))
    # fallen needles: short strokes of 2-3 pixels in light and dark tones
    dirs = [(1, 0), (1, 1), (1, -1), (0, 1)]
    for _ in range(26):
        x, y = int(rng.integers(N)), int(rng.integers(N))
        dx, dy = dirs[int(rng.integers(len(dirs)))]
        ln = int(rng.integers(2, 4))
        c = PODZOL[5] if rng.random() < 0.55 else PODZOL[4]
        if rng.random() < 0.25:
            c = PODZOL[0]
        for k in range(ln):
            wput(img, x + dx * k, y + dy * k, c)
    return img


def podzol_side():
    rng = rng_for("podzol_side")
    img = load("block", "dirt")
    top = podzol_top()
    depth = 3 + rng.integers(0, 2, N)
    for x in rng.choice(N, 4, replace=False):
        depth[x] += int(rng.integers(1, 3))
    for x in range(N):
        d = int(depth[x])
        img[:d, x, :3] = top[:d, x, :3]
        img[d - 1, x, :3] = scale_rgb(img[d - 1, x, :3], 0.85)
    return img


def rooted_dirt():
    rng = rng_for("rooted_dirt")
    img = load("block", "dirt")
    roots = np.zeros((N, N), bool)
    for sx in (1, 6, 10, 14):
        x = sx
        y = int(rng.integers(N))
        step = 0
        for _ in range(int(rng.integers(9, 14))):
            roots[y % N, x % N] = True
            y += 1
            r = rng.random()
            if r < 0.3:
                step = -1
            elif r < 0.6:
                step = 1
            elif r < 0.8:
                step = 0
            x += step
            if rng.random() < 0.15:  # little side branch
                bx = x + (1 if rng.random() < 0.5 else -1)
                roots[y % N, bx % N] = True
                roots[(y + 1) % N, (bx + (bx - x)) % N] = True
    # shadow pixel below-right of each strand, then the strand itself
    sh = np.roll(np.roll(roots, 1, 0), 1, 1) & ~roots
    img[sh, :3] = scale_rgb(img[sh, :3], 0.8)
    lit = roots & ~np.roll(roots, 1, 1)
    img[roots, :3] = ROOT_PAL[1]
    img[lit, :3] = ROOT_PAL[2]
    dk = roots & ~np.roll(roots, -1, 0)
    img[dk & ~lit, :3] = ROOT_PAL[0]
    return img


def dirt_path_top():
    rng = rng_for("dirt_path_top")
    f = 0.6 * value_noise(rng, 4, 2) + 0.4 * value_noise(rng, 2, 1)
    img = from_rgb(shade(0.2 + 0.6 * normalize(f), PATH, 0.5))
    # a few shallow dents
    for (x, y) in ((3, 4), (11, 2), (7, 9), (13, 12), (2, 13)):
        wput(img, x, y, PATH[0])
        wput(img, x + 1, y, PATH[1])
        wput(img, x, y + 1, PATH[4])
    return img


def dirt_path_side():
    rng = rng_for("dirt_path_side")
    img = load("block", "dirt")
    top = dirt_path_top()
    for x in range(N):
        d = 3 + (1 if rng.random() < 0.35 else 0)
        img[:d, x, :3] = top[:d, x, :3]
        img[d - 1, x, :3] = scale_rgb(top[d - 1, x, :3], 0.82)
    img[0, :, :3] = scale_rgb(img[0, :, :3], 1.06)
    return img


# --------------------------------------------------------------------------
# blocks: sandstone family, quartz
# --------------------------------------------------------------------------

def red_sandstone_top():
    rng = rng_for("red_sandstone_top")
    f = fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.3)))
    img = from_rgb(shade(0.35 + 0.45 * f, RED_SANDSTONE, 0.5))
    bevel(img, 1.05, 0.9)
    return img


def red_sandstone_side():
    P = RED_SANDSTONE
    rng = rng_for("red_sandstone_side")
    rough = fbm(rng, ((2, 0.5), (1, 0.6)))
    streak = value_noise(rng, 8, 1)
    img = from_rgb(shade(0.45 + 0.4 * streak, P, 0.3))
    rgb = img[..., :3]
    rgb[0:3] = shade(0.5 + 0.5 * rough, P, 0.8)[0:3]
    rgb[3] = P[1]
    for y in (6, 9):
        xs = rng.random(N) < 0.75
        rgb[y, xs] = P[2]
    bottom = shade(0.2 + 0.5 * rough, P, 0.8)
    for x in range(N):
        top = 12 + int(rng.integers(0, 2))
        rgb[top:, x] = bottom[top:, x]
        rgb[top - 1, x] = P[1]
    return img


def red_sandstone_bottom():
    rng = rng_for("red_sandstone_bottom")
    f = fbm(rng, ((2, 0.5), (1, 0.6)))
    return from_rgb(shade(0.15 + 0.6 * f, RED_SANDSTONE, 0.9))


def smooth_sandstone():
    return flatten(load("block", "sandstone_top"), 0.5)


def smooth_red_sandstone():
    return flatten(red_sandstone_top(), 0.5)


def smooth_quartz():
    img = flatten(load("block", "quartz_block"), 0.4)
    return img


# --------------------------------------------------------------------------
# blocks: ores, coal block, infested stone
# --------------------------------------------------------------------------

def deep_ore(kind, p=None):
    return gt.ore(kind, bg=load("block", "deepslate"), p=p)


def deepslate_coal_ore():
    return deep_ore("coal", DEEP_COAL)


def deepslate_iron_ore():
    return deep_ore("iron")


def deepslate_gold_ore():
    return deep_ore("gold")


def deepslate_diamond_ore():
    return deep_ore("diamond")


def coal_block():
    rng = rng_for("coal_block")
    f = fbm(rng, ((4, 0.5), (2, 0.4), (1, 0.15)))
    img = from_rgb(shade(0.15 + 0.6 * f, COAL_BLOCK[:5], 0.6))
    # glassy facets: short light streaks with a dark edge below
    for (x, y) in ((2, 3), (9, 2), (12, 8), (4, 10), (10, 13)):
        wput(img, x, y, COAL_BLOCK[5])
        wput(img, x + 1, y, COAL_BLOCK[4])
        wput(img, x, y + 1, COAL_BLOCK[0])
        wput(img, x + 1, y + 1, COAL_BLOCK[0])
    bevel(img, 1.25, 0.8)
    return img


def infested_stone():
    return load("block", "stone")


# --------------------------------------------------------------------------
# blocks: wood & iron furniture
# --------------------------------------------------------------------------

def ladder():
    img = canvas()
    for y in range(N):
        put(img, 1, y, WOOD[3])
        put(img, 2, y, WOOD[2])
        put(img, 13, y, WOOD[3])
        put(img, 14, y, WOOD[1])
    # grain specks on the rails (2-pixel runs)
    for (x, y) in ((2, 3), (2, 4), (1, 11), (1, 12), (14, 6), (14, 7), (13, 14), (13, 15)):
        put(img, x, y, WOOD[1] if x in (2, 14) else WOOD[2])
    for y in (1, 5, 9, 13):
        for x in range(3, 13):
            put(img, x, y, WOOD[3] if x % 5 else WOOD[4])
            put(img, x, y + 1, WOOD[1])
        # rung ends sit in the rails: a darker notch
        put(img, 2, y + 1, WOOD[0])
        put(img, 13, y + 1, WOOD[1])
    return img


def iron_plate(rng):
    f = value_noise(rng, 16, 4) * 0.5 + value_noise(rng, 4, 8) * 0.5
    return from_rgb(shade(0.35 + 0.35 * normalize(f), IRON[3:6], 0.4))


def iron_door_lower():
    rng = rng_for("iron_door_lower")
    img = iron_plate(rng)
    rgb = img[..., :3]
    # recessed vertical seams (same places as the oak door's plank joints)
    for x in (5, 10):
        rgb[1:15, x] = IRON[2]
        rgb[1:15, x + 1] = IRON[5]
    # horizontal brace with rivets
    rgb[7, 1:15] = IRON[6]
    rgb[8, 1:15] = IRON[4]
    rgb[9, 1:15] = IRON[2]
    for x in (2, 7, 12):
        rgb[8, x] = IRON[6]
        rgb[9, x] = IRON[1]
    # frame
    rgb[0, :] = IRON[1]
    rgb[15, :] = IRON[0]
    rgb[:, 0] = IRON[1]
    rgb[:, 15] = IRON[0]
    rgb[1, 1:15] = IRON[5]
    rgb[1:15, 1] = IRON[5]
    # hinges on the left, handle on the right (oak door positions)
    for y in (4, 12):
        rgb[y, 1:3] = DARK_IRON[3]
        rgb[y + 1, 1:3] = DARK_IRON[1]
    rgb[2:5, 13] = DARK_IRON[4]
    rgb[2:5, 14] = DARK_IRON[2]
    return img


def iron_door_upper():
    rng = rng_for("iron_door_upper")
    img = iron_plate(rng)
    rgb = img[..., :3]
    for x in (5, 10):
        rgb[8:15, x] = IRON[2]
        rgb[8:15, x + 1] = IRON[5]
    rgb[0, :] = IRON[1]
    rgb[15, :] = IRON[0]
    rgb[:, 0] = IRON[1]
    rgb[:, 15] = IRON[0]
    rgb[1, 1:15] = IRON[5]
    rgb[1:15, 1] = IRON[5]
    # two small windows with a thick iron surround; the glass area is transparent
    for x0 in (3, 9):
        rgb[2, x0 - 1:x0 + 5] = IRON[2]          # shadowed top of the opening
        rgb[2:8, x0 - 1] = IRON[2]
        rgb[7, x0 - 1:x0 + 5] = IRON[6]          # lit sill
        rgb[2:8, x0 + 4] = IRON[6]
        img[3:7, x0:x0 + 4, 3] = 0
        img[3:7, x0:x0 + 4, :3] = 0
    rgb[11, 1:3] = DARK_IRON[3]
    rgb[12, 1:3] = DARK_IRON[1]
    rgb[4, 1:3] = DARK_IRON[3]
    rgb[5, 1:3] = DARK_IRON[1]
    return img


def wood_box(name, p=DARK_WOOD):
    """Wooden box face: vertical boards with a light/dark frame."""
    rng = rng_for(name)
    grain = 0.6 * value_noise(rng, 4, 16) + 0.4 * value_noise(rng, 2, 8)
    img = from_rgb(shade(0.3 + 0.5 * normalize(grain), p[2:5], 0.4))
    rgb = img[..., :3]
    for x in (5, 10):
        rgb[:, x] = p[1]
    rgb[0, :] = p[5]
    rgb[:, 0] = p[5]
    rgb[15, :] = p[0]
    rgb[:, 15] = p[0]
    rgb[1, 1:15] = p[4]
    rgb[14, 1:15] = p[1]
    rgb[1:15, 14] = p[1]
    return img


def note_block():
    img = wood_box("note_block")
    rgb = img[..., :3]
    # speaker grille: a dark recess crossed by thin wooden slats
    rgb[3:13, 3:13] = pal("#1a110a")[0]
    rgb[3, 3:13] = pal("#0f0a06")[0]
    rgb[3:13, 3] = pal("#0f0a06")[0]
    for y in (5, 7, 9, 11):
        rgb[y, 4:13] = DARK_WOOD[3]
        rgb[y, 4] = DARK_WOOD[2]
    rgb[13, 3:13] = DARK_WOOD[5]
    rgb[3:14, 13] = DARK_WOOD[5]
    return img


def jukebox_side():
    img = wood_box("jukebox_side", WOOD[[0, 0, 1, 2, 3, 4]])
    rgb = img[..., :3]
    # inlaid darker band around the middle
    rgb[6, 1:15] = DARK_WOOD[1]
    rgb[7:9, 1:15] = DARK_WOOD[3]
    rgb[9, 1:15] = WOOD[4]
    rgb[7, 1:15] = DARK_WOOD[4]
    return img


def jukebox_top():
    img = wood_box("jukebox_top", WOOD[[0, 0, 1, 2, 3, 4]])
    rgb = img[..., :3]
    rgb[:, 5] = rgb[:, 4]
    rgb[:, 10] = rgb[:, 9]
    # disc slot: dark opening, shadow on its top edge, lit lip below
    rgb[6, 3:13] = WOOD[0]
    rgb[7, 3:13] = pal("#120c07")[0]
    rgb[8, 3:13] = pal("#1e140c")[0]
    rgb[9, 3:13] = WOOD[4]
    rgb[6:9, 2] = WOOD[1]
    rgb[6:10, 13] = WOOD[4]
    return img


def wet_sponge():
    img = load("block", "sponge")
    rgb = img[..., :3]
    L = lum(rgb)
    holes = L < np.percentile(L, 22)
    rgb[:] = rgb * np.array([0.70, 0.70, 0.52]) + np.array([4, 6, 2])
    rgb[holes] = rgb[holes] * 0.72
    # a few extra soaked pores
    rng = rng_for("wet_sponge")
    for _ in range(6):
        x, y = int(rng.integers(N)), int(rng.integers(N))
        wput(img, x, y, scale_rgb(img[y, x, :3], 0.68))
        wput(img, x + 1, y, scale_rgb(img[y, (x + 1) % N, :3], 0.8))
    return img


# --------------------------------------------------------------------------
# blocks: plants
# --------------------------------------------------------------------------

def kelp_stem_x(y):
    return 7 + int(round(math.sin(2 * math.pi * y / N)))


def kelp_draw(img, y0=0, tip=False):
    """Stem from y0 to the bottom, 2 px wide, wavy; blades on alternate sides."""
    for y in range(y0, N):
        x = kelp_stem_x(y)
        put(img, x, y, KELP[3])
        put(img, x + 1, y, KELP[1])
    if tip:
        x = kelp_stem_x(y0)
        put(img, x, y0, KELP[4])
        put(img, x + 1, y0, KELP[2])
        put(img, x + 1, y0 - 1, KELP[3])
        put(img, x + 1, y0 - 2, KELP[4])
    # blades: (start y, side)
    blades = [(3, -1), (11, 1)] if not tip else [(6, -1), (11, 1)]
    for (by, side) in blades:
        bx = kelp_stem_x(by) + (0 if side < 0 else 1)
        pts = [(bx + side, by), (bx + 2 * side, by - 1), (bx + 3 * side, by - 1),
               (bx + 4 * side, by - 2), (bx + 5 * side, by - 2)]
        for i, (x, y) in enumerate(pts):
            put(img, x, y, KELP[4] if i % 2 == 0 else KELP[3])
            if i < 4:
                put(img, x, y + 1, KELP[2])
        # small air bladder at the base of the blade
        put(img, bx + side, by + 1, KELP_BULB[1])
        put(img, bx + side, by + 2, KELP_BULB[0])
    return img


def kelp():
    return kelp_draw(canvas())


def kelp_top():
    return kelp_draw(canvas(), y0=3, tip=True)


def dried_kelp_block_side():
    rng = rng_for("dried_kelp_block_side")
    base = rng.random(N // 2)
    v = value_noise(rng, 2, 4)
    img = canvas()
    for x in range(N):
        t = 0.25 + 0.45 * base[x // 2] + 0.3 * v[:, x]
        col = ramp_at(DRIED, np.clip(t, 0, 0.999))
        if x % 2 == 1:
            col = scale_rgb(col, 0.82)
        img[:, x, :3] = col
    img[..., 3] = 255
    # twine ties
    for y in (3, 12):
        img[y, :, :3] = TWINE[2]
        img[y + 1, :, :3] = TWINE[0]
        for x in range(0, N, 4):
            img[y, x, :3] = TWINE[1]
        img[y - 1, :, :3] = scale_rgb(img[y - 1, :, :3], 0.8)
    return img


def dried_kelp_block_top():
    rng = rng_for("dried_kelp_block_top")
    d = np.hypot(XX - 7.5, YY - 7.5)
    ang = np.arctan2(YY - 7.5, XX - 7.5)
    # rolled strands: a loose spiral of light and dark bands
    s = (d * 0.9 + ang / (2 * math.pi) * 1.8) % 2.0
    f = value_noise(rng, 4) * 0.3
    t = np.where(s < 1.0, 0.65, 0.3) + f
    img = from_rgb(ramp_at(DRIED, np.clip(t, 0, 0.999)))
    rgb = img[..., :3]
    rgb[0, :] = TWINE[1]
    rgb[:, 0] = TWINE[1]
    rgb[15, :] = TWINE[0]
    rgb[:, 15] = TWINE[0]
    return img


def bamboo_stalk():
    img = canvas()
    for y in range(N):
        put(img, 7, y, BAMBOO[4])
        put(img, 8, y, BAMBOO[2])
    for y in (4, 12):
        put(img, 7, y, BAMBOO[1])
        put(img, 8, y, BAMBOO[0])
        put(img, 7, y + 1, BAMBOO[3])
    # leaves sprouting from the nodes
    for (pts, side) in (([(9, 4), (10, 3), (11, 3), (12, 2), (13, 2)], 1),
                        ([(6, 12), (5, 11), (4, 11), (3, 10), (2, 10)], -1)):
        for i, (x, y) in enumerate(pts):
            put(img, x, y, BAMBOO_LEAF[3] if i < 3 else BAMBOO_LEAF[2])
            if 0 < i < 4:
                put(img, x, y + 1, BAMBOO_LEAF[1])
    return img


def leaf_blob(img, cx, cy, w, h, p):
    for y in range(cy, cy + h):
        for x in range(cx, cx + w):
            c = p[2]
            if y == cy and x == cx:
                c = p[4]
            elif y == cy or x == cx:
                c = p[3]
            elif y == cy + h - 1:
                c = p[1]
            put(img, x, y, c)


def sweet_berry_bush(stage):
    rng = rng_for("sweet_berry_bush")
    img = canvas()
    height = [5, 9, 13, 14][stage]
    width = [5, 9, 13, 14][stage]
    # woody stems
    for sx in ((7,), (6, 9), (4, 7, 10), (3, 7, 11))[stage]:
        for y in range(N - min(height, 6), N):
            put(img, sx, y, pal("#3a2e1a")[0])
    blobs = []
    for _ in range([5, 12, 22, 26][stage]):
        w = int(rng.integers(2, 4))
        h = 2
        cx = int(8 - width / 2 + rng.random() * (width - w))
        top = N - height
        cy = int(top + rng.random() ** 0.7 * (height - 2))
        blobs.append((cy, cx, w, h))
    for cy, cx, w, h in sorted(blobs):
        leaf_blob(img, cx, cy, w, h, LEAF)
    rb = rng_for("sweet_berry_bush_berries")
    spots = [(4, 7), (9, 6), (11, 10), (6, 11), (2, 10), (8, 9), (12, 5), (5, 4), (9, 12)]
    nb = {2: 3, 3: 9}.get(stage, 0)
    for i, (x, y) in enumerate(spots[:nb]):
        if stage == 2:
            p = UNRIPE if i != 1 else BERRY
        else:
            p = BERRY
        put(img, x, y, p[3])
        put(img, x + 1, y, p[2])
        put(img, x, y + 1, p[2])
        put(img, x + 1, y + 1, p[1])
    return img


# --------------------------------------------------------------------------
# blocks: cake
# --------------------------------------------------------------------------

def berry_dot(img, x, y):
    put(img, x, y, BERRY[3])
    put(img, x + 1, y, BERRY[2])
    put(img, x, y + 1, BERRY[1])
    put(img, x + 1, y + 1, BERRY[0])


def cake_top():
    rng = rng_for("cake_top")
    f = value_noise(rng, 4) * 0.6 + value_noise(rng, 2) * 0.4
    img = from_rgb(shade(0.3 + 0.5 * normalize(f), CREAM[1:], 0.4))
    rgb = img[..., :3]
    rgb[0, :] = CREAM[4]
    rgb[:, 0] = CREAM[4]
    rgb[15, :] = CREAM[0]
    rgb[:, 15] = CREAM[0]
    for (x, y) in ((3, 3), (10, 2), (7, 7), (12, 9), (3, 11), (9, 12)):
        berry_dot(img, x, y)
    return img


def cake_band(name, inner=False):
    """The 8 px tall cake is shown with the BOTTOM rows of its side texture (like the bed)."""
    rng = rng_for(name)
    img = canvas()
    f = value_noise(rng, 4, 2) * 0.6 + value_noise(rng, 2, 1) * 0.4
    sponge = shade(0.25 + 0.5 * normalize(f), SPONGE_CAKE[1:], 0.5)
    img[8:16, :, :3] = sponge[8:16]
    img[8:16, :, 3] = 255
    rgb = img[..., :3]
    # icing on top, dripping down a little
    rgb[8, :] = CREAM[4]
    rgb[9, :] = CREAM[3]
    if not inner:
        for x in (1, 2, 6, 10, 11, 14):
            rgb[10, x] = CREAM[2]
        for x in (2, 10):
            rgb[11, x] = CREAM[1]
    # cream filling between two sponge layers
    rgb[12, :] = CREAM[2]
    rgb[13, :] = SPONGE_CAKE[0]
    if inner:
        for x in (3, 4, 9, 13):
            rgb[12, x] = BERRY[2]
    rgb[15, :] = SPONGE_CAKE[0]
    return img


def cake_side():
    return cake_band("cake_side")


def cake_inner():
    return cake_band("cake_inner", inner=True)


def cake_bottom():
    rng = rng_for("cake_bottom")
    f = fbm(rng, ((4, 0.5), (2, 0.4), (1, 0.1)))
    img = from_rgb(shade(0.2 + 0.5 * f, SPONGE_CAKE[:4], 0.5))
    bevel(img, 1.08, 0.85)
    return img


# --------------------------------------------------------------------------
# blocks: pumpkins, fire, spawner
# --------------------------------------------------------------------------

FACE = [
    "................",
    "................",
    "................",
    "....#......#....",
    "...###....###...",
    "..#####..#####..",
    "................",
    ".......##.......",
    "................",
    "...###.##.###...",
    "...##########...",
    "....###..###....",
    "................",
    "................",
    "................",
    "................",
]


def face(glow):
    img = load("block", "pumpkin_side")
    m = mask_from_rows(FACE)
    # hole pixels next to the uncut skin on the top/left show the cut wall (lit)
    pad = np.pad(m, 1)
    rim = m & (~pad[:-2, 1:-1] | ~pad[1:-1, :-2])
    rgb = img[..., :3]
    if glow:
        core = pal("#f6cf5e")[0]
        edge = pal("#e8902a")[0]
        rgb[m] = core
        rgb[rim] = edge
        bright = m & pad[:-2, 1:-1] & pad[1:-1, :-2] & pad[2:, 1:-1]
        rgb[bright] = pal("#fbe49a")[0]
    else:
        rgb[m] = pal("#2a1608")[0]
        rgb[rim] = pal("#a8601c")[0]
    return img


def carved_pumpkin():
    return face(False)


def jack_o_lantern():
    return face(True)


def flames(name, p):
    rng = rng_for(name)
    tongues = [(1.5, 11, 2.2), (5.0, 14, 2.6), (8.5, 10, 2.2), (11.5, 13.5, 2.6), (14.8, 9, 2.0)]
    h = np.zeros(N)
    for x in range(N):
        best = 3.0
        for (c, ht, w) in tongues:
            for cc in (c - N, c, c + N):  # wrap so side-by-side fire faces join
                d = abs(x + 0.5 - cc) / w
                if d < 1:
                    best = max(best, ht * (1 - d ** 1.6))
        h[x] = best + rng.random() * 0.8
    top = N - h
    mask = YY + 0.5 >= top[None, :]
    # inner depth: distance (in px) to the flame edge
    depth = np.zeros((N, N))
    cur = mask.copy()
    for k in range(4):
        depth += cur
        pad = np.pad(cur, 1, constant_values=False)
        cur = cur & pad[:-2, 1:-1] & pad[1:-1, :-2] & pad[1:-1, 2:]
    t = (depth / 4.0) * 0.6 + ((YY - top[None, :]) / np.maximum(h, 1)[None, :]) * 0.55
    img = canvas()
    img[mask, :3] = ramp_at(p, np.clip(t[mask] * 0.95, 0, 0.999))
    img[mask, 3] = 255
    return img


def fire():
    return flames("fire", FIRE)


def soul_fire():
    return flames("soul_fire", SOUL)


def spawner():
    img = canvas()
    P = DARK_IRON
    bars_x = (0, 5, 10, 15)
    bars_y = (0, 5, 10, 15)
    for y in range(N):
        for x in bars_x:
            put(img, x, y, P[3] if x != 15 else P[1])
    for x in range(N):
        for y in bars_y:
            put(img, x, y, P[4] if y != 15 else P[1])
    # thicker frame: inner dark line under the top bar and right of the left bar
    for i in range(1, 15):
        put(img, i, 1, P[2])
        put(img, 1, i, P[2])
        put(img, i, 14, P[2])
        put(img, 14, i, P[2])
    # joints: lit rivet with a shadow
    for x in bars_x:
        for y in bars_y:
            put(img, x, y, P[5])
            if x < 15 and y < 15:
                put(img, x + 1, y + 1, P[1])
    put(img, 0, 0, P[5])
    return img


# --------------------------------------------------------------------------
# blocks: white bed, white shulker
# --------------------------------------------------------------------------

def is_red(img):
    r, g, b = img[..., 0], img[..., 1], img[..., 2]
    return (img[..., 3] > 127) & (r > 50) & (r > g * 1.6) & (r > b * 1.6)


def to_white_wool(img):
    m = is_red(img)
    dark = m & (lum(img[..., :3]) < 36)   # the red's darkest tone (outlines) -> cool dark grey
    img = recolor(img, m, WHITE_WOOL, lo=28, hi=140)
    img[dark, :3] = pal("#3c4046")[0]
    return img


def white_bed(name):
    return to_white_wool(load("block", name.replace("white", "red")))


def white_bed_top():
    img = white_bed("white_bed_top")
    # make the pillow read against the white blanket: a soft grey seam around it
    rgb = img[..., :3]
    rgb[5, 1:15] = WHITE_WOOL[2]
    return img


def white_bed_side():
    return white_bed("white_bed_side")


def white_bed_end():
    return white_bed("white_bed_end")


SHULKER_WHITE = pal("#7c8086", "#9a9ea3", "#b5b9bd", "#cbcfd1", "#dcdfe0", "#eaecec")


def white_shulker(name):
    img = load("block", name.replace("white_", ""))
    m = img[..., 3] > 127
    return recolor(img, m, SHULKER_WHITE)


def white_shulker_box():
    return white_shulker("white_shulker_box")


def white_shulker_box_top():
    return white_shulker("white_shulker_box_top")


# --------------------------------------------------------------------------
# items
# --------------------------------------------------------------------------

def disc_mask(cx, cy, r):
    return np.hypot(XX + 0.5 - cx, YY + 0.5 - cy) <= r


def iron_nugget():
    return lump("iron_nugget", IRON_ITEM, rows=NUGGET)


BOWL_ROWS = blank_rows(6) + [
    "...rRRRRRRRRr...",
    ".rRiiiiiiiiiiRr.",
    ".Riiiiiiiiiiiir.",
    ".rRRRRRRRRRRRrr.",
    "..hoooooooooood.",
    "...hoooooooodd..",
    "....hoooooodd...",
    ".....dddddd.....",
] + blank_rows(2)


def bowl():
    return outline(from_map(BOWL_ROWS, BOWL))


def stew(name, p, bits):
    rng = rng_for(name)
    img = from_map(BOWL_ROWS, BOWL)
    # soup surface fills the inside and heaps a little above the rim
    for y, x0, x1 in ((6, 5, 11), (7, 3, 13), (8, 2, 14)):
        for x in range(x0, x1):
            c = p[2] if y == 6 else (p[1] if y == 7 else p[0])
            put(img, x, y, c)
    for i, (x, y) in enumerate(((4, 7), (8, 6), (11, 7), (6, 8), (10, 8), (12, 8))):
        put(img, x, y, bits[i % len(bits)])
        if rng.random() < 0.5:
            put(img, x + 1, y, bits[i % len(bits)])
    return outline(img)


def mushroom_stew():
    return stew("mushroom_stew", pal("#8a6a48", "#a6845c", "#c09e72"),
                pal("#6a4a30", "#d8c8a8"))


def beetroot_soup():
    return stew("beetroot_soup", pal("#5c1424", "#7c2032", "#9c3242"),
                pal("#c05a6a", "#4a0e1a"))


def rabbit_stew():
    return stew("rabbit_stew", pal("#6a4424", "#865a30", "#a0723e"),
                pal("#d07a2a", "#d8c080", "#5a8a3a"))


def suspicious_stew():
    return stew("suspicious_stew", pal("#3e4226", "#525832", "#686e3e"),
                pal("#7a6236", "#4e6a3a", "#8a8a52"))


def cookie():
    rng = rng_for("cookie")
    img = canvas()
    m = disc_mask(8, 8.5, 5.6)
    shade_mask(img, m, pal("#8a5a2a", "#a8743a", "#c08c4c", "#d4a462"), rng, speck=0.1)
    for (x, y) in ((5, 6), (9, 5), (7, 9), (10, 10), (4, 10)):
        put(img, x, y, pal("#3a2014")[0])
        put(img, x + 1, y, pal("#4e2c1a")[0])
    return outline(img)


def pumpkin_pie():
    rows = blank_rows(3) + [
        "....cCcCcCcC....",
        "..cCFFFFFFFFCc..",
        ".cFFFFFFFFFFFFC.",
        ".CFFFFFFFFFFFFc.",
        ".cFFFFFFFFFFFFC.",
        "..CcFFFFFFFFcC..",
        ".bcCcCcCcCcCcCb.",
        ".bbbbbbbbbbbbbB.",
        "..bbbbbbbbbbbB..",
    ] + blank_rows(4)
    img = from_map(rows, {"c": pal("#c49456")[0], "C": pal("#ddb070")[0],
                          "b": pal("#a8743c")[0], "B": pal("#7e5428")[0], "F": [0, 0, 0]})
    m = np.array([[c == "F" for c in r] for r in rows])
    shade_mask(img, m, pal("#a8561e", "#c46c2a", "#d8843a", "#e8a050"), rng_for("pumpkin_pie"),
               speck=0.1)
    return outline(img)


def potato_like(name, p, spots=None):
    rows = blank_rows(4) + [
        ".....22222......",
        "...224444322....",
        "..2443444432....",
        "..2444434432....",
        "..2344444332....",
        "...23343332.....",
        "....22222.......",
    ] + blank_rows(5)
    img = from_map(rows, {str(i + 1): p[i] for i in range(len(p))})
    if spots is not None:
        for (x, y) in ((5, 7), (8, 8), (6, 9)):
            put(img, x, y, spots)
    return outline(img)


def poisonous_potato():
    return potato_like("poisonous_potato",
                       pal("#3c4620", "#56622a", "#748438", "#94a24c", "#b2be6a"),
                       spots=pal("#4a5a24")[0])


def sweet_berries():
    rows = blank_rows(1) + [
        "........gG......",
        ".........gg.....",
        "........g..g....",
        ".......g....g...",
        "...LLM.g...LLM..",
        "..LhMMD...LhMMD.",
        "..LMMMD...LMMMD.",
        "..MMMDD...MMMDD.",
        "...DDD.LLM.DDD..",
        "......LhMMD.....",
        "......LMMMD.....",
        "......MMMDD.....",
        ".......DDD......",
    ] + blank_rows(2)
    cols = {"g": pal("#3e6a2a")[0], "G": pal("#5a8a3a")[0], "L": BERRY[2], "M": BERRY[1],
            "D": BERRY[0], "h": BERRY[3]}
    return outline(from_map(rows, cols))


def dried_kelp():
    rows = blank_rows(2) + [
        "...........DD...",
        "..........DMLD..",
        ".........DMLMD..",
        "........DDLMD...",
        ".......DMLMD....",
        "......DMLDD.....",
        ".....DMLMD......",
        "....DDLMD.......",
        "...DMLMD........",
        "..DMLDD.........",
        "..DMMD..........",
        "...DD...........",
    ] + blank_rows(2)
    return outline(from_map(rows, {"D": DRIED[0], "M": DRIED[2], "L": DRIED[4]}))


def tropical_fish():
    img = canvas()
    body = ((XX + 0.5 - 7.0) / 5.5) ** 2 + ((YY + 0.5 - 8.5) / 3.6) ** 2 <= 1
    tail = (XX >= 12) & (XX <= 14) & (np.abs(YY + 0.5 - 8.5) <= (XX - 11) * 1.1 + 0.5)
    fin = (YY >= 4) & (YY <= 5) & (XX >= 6) & (XX <= 8 + (YY - 4))
    m = body | tail | fin
    shade_mask(img, m, pal("#a8481a", "#c8622a", "#e07e38", "#eea05a"))
    white = pal("#b8b4aa", "#e8e4dc")
    for x in (5, 9):
        col = body & (XX == x)
        img[col, :3] = white[1]
        img[col & (YY >= 9), :3] = white[0]
    tip = tail & (XX == 14)
    img[tip, :3] = white[1]
    put(img, 3, 7, pal("#1e1a22")[0])
    put(img, 3, 8, pal("#4a3a3a")[0])
    return outline(img)


def cocoa_beans():
    img = canvas()
    P = pal("#3a1e10", "#542c16", "#704020", "#8a542c")
    for (cx, cy, rx, ry) in ((5.0, 10.0, 2.6, 1.9), (10.0, 6.0, 2.6, 1.9), (11.0, 11.5, 2.2, 1.7)):
        m = ((XX + 0.5 - cx) / rx) ** 2 + ((YY + 0.5 - cy) / ry) ** 2 <= 1
        shade_mask(img, m, P)
        ys, xs = np.nonzero(m)
        y = int(round(cy - 0.5))
        for x in range(xs.min() + 1, xs.max()):
            if m[y, x]:
                put(img, x, y, P[0])
    return outline(img)


def snowball():
    img = canvas()
    shade_mask(img, disc_mask(8, 8.5, 5.2),
               pal("#9aaab8", "#bccad6", "#d8e2ea", "#eaf0f4", "#fafcfd"),
               rng_for("snowball"), speck=0.12)
    return outline(img)


def egg():
    img = canvas()
    m = ((XX + 0.5 - 8) / 4.2) ** 2 + ((YY + 0.5 - 8.8) / 5.4) ** 2 <= 1
    m &= ~((YY < 6) & (np.abs(XX + 0.5 - 8) > 3.0))
    shade_mask(img, m, pal("#a48c68", "#c4ae8a", "#dccaa6", "#ece0c4", "#f8f0de"))
    for (x, y) in ((6, 6), (9, 9), (7, 11)):
        put(img, x, y, pal("#a88a64")[0])
        put(img, x + 1, y, pal("#bca07a")[0])
    return outline(img)


def armor_stand():
    rows = blank_rows(1) + [
        ".......hH.......",
        ".......hH.......",
        "..ssssssssssss..",
        "..SSSSSSSSSSSS..",
        "......hH.hH.....",
        "......hH.hH.....",
        "....bbbbbbbb....",
        "....BBBBBBBB....",
        "......hH.hH.....",
        "......hH.hH.....",
        "......hH.hH.....",
        "..tttttttttttt..",
        "..TTTTTTTTTTTT..",
    ] + blank_rows(2)
    cols = {"h": WOOD[4], "H": WOOD[2], "s": WOOD[4], "S": WOOD[2], "b": WOOD[4], "B": WOOD[2],
            "t": pal("#a8a8a8")[0], "T": pal("#727272")[0]}
    return outline(from_map(rows, cols))


def shield():
    rng = rng_for("shield")
    img = canvas()
    span = {11: (3, 12), 12: (4, 11), 13: (5, 10), 14: (6, 9)}
    m = np.zeros((N, N), bool)
    for y in range(1, 15):
        x0, x1 = span.get(y, (2, 13))
        m[y, x0:x1 + 1] = True
    grain = value_noise(rng, 16, 4)
    wood = shade(0.3 + 0.5 * normalize(grain), WOOD[1:5], 0.4)
    img[m, :3] = wood[m]
    img[m, 3] = 255
    for x in (5, 8, 11):  # plank joints
        img[m[:, x] & (YY[:, x] > 1), x, :3] = WOOD[1]
    lit, dark = edges(m)
    img[m & lit, :3] = IRON[5]
    img[m & dark & ~lit, :3] = IRON[2]
    # iron boss in the centre
    put(img, 7, 6, IRON[6])
    put(img, 8, 6, IRON[4])
    put(img, 7, 7, IRON[4])
    put(img, 8, 7, IRON[2])
    return outline(img)


def shears():
    img = canvas()
    # two blades opening upwards in a V, meeting at the pivot
    for y in range(1, 8):
        xl = 4 + y // 2
        xr = 11 - y // 2
        put(img, xl, y, IRON[6] if y > 1 else IRON[5])
        put(img, xl + 1, y, IRON[3])
        put(img, xr - 1, y, IRON[5])
        put(img, xr, y, IRON[2])
    # handles cross below the pivot and end in two rings of darker iron
    H = pal("#3a3638", "#5a5250", "#7c726c")
    for (x, y, c) in ((8, 9, H[2]), (9, 10, H[1]), (7, 9, H[2]), (6, 10, H[1])):
        put(img, x, y, c)
    for x0 in (2, 10):
        y0 = 10
        for y in range(y0, y0 + 4):
            for x in range(x0, x0 + 4):
                if not (x0 < x < x0 + 3 and y0 < y < y0 + 3):
                    put(img, x, y, H[2] if (y == y0 or x == x0) else H[1])
    put(img, 7, 8, IRON[4])
    put(img, 8, 8, pal("#2e2e34")[0])  # pivot rivet
    return outline(img)


def totem_of_undying():
    rows = [
        "................",
        "......GGGG......",
        ".....GGGGGG.....",
        ".....GeGGeG.....",
        ".....GGmmGG.....",
        "......GGGG......",
        "..GG.GGGGGG.GG..",
        "..GGGGGGGGGGGG..",
        "...GGGGGGGGGG...",
        ".....GGGGGG.....",
        ".....bbbbbb.....",
        "....GGGGGGGG....",
        "....GGGGGGGG....",
        "...GGGGGGGGGG...",
        "................",
        "................",
    ]
    img = canvas()
    m = np.array([[c != "." for c in r] for r in rows])
    shade_mask(img, m, GOLD[:5], rng_for("totem_of_undying"), speck=0.08)
    for y, r in enumerate(rows):
        for x, c in enumerate(r):
            if c == "e":
                put(img, x, y, pal("#2aa860")[0])
            elif c == "m":
                put(img, x, y, GOLD[1])
            elif c == "b":
                put(img, x, y, GOLD[2] if x % 2 else GOLD[1])
    put(img, 6, 3, pal("#5ae08a")[0])
    return outline(img)


def writable_book():
    img = load("item", "book")
    feather = pal("#b8b8b4", "#dcdcd8", "#f4f4f0")
    # quill leaning across the cover, nib at the bottom left
    for i in range(7):
        x, y = 14 - i, 1 + i
        put(img, x, y, feather[1])
        if i < 5:
            put(img, x + 1, y, feather[0] if i else feather[2])
            put(img, x, y - 1, feather[2])
    put(img, 7, 8, pal("#6a6a70")[0])
    put(img, 6, 9, pal("#2c2c34")[0])
    return outline(img)


def written_book():
    img = load("item", "book")
    G = pal("#94701a", "#d0a832", "#ecc85a")
    for y in range(4, 7):
        for x in range(4, 10):
            put(img, x, y, G[1])
    img[4, 4:10, :3] = G[2]
    img[6, 4:10, :3] = G[0]
    img[5, 5:9, :3] = pal("#7a3a1a")[0]  # title line
    return img


def carrot_on_a_stick():
    img = load("item", "fishing_rod")
    img[11:13, 13:15] = 0
    img[11, 14] = 0
    O = pal("#9a4a14", "#d0742a", "#ec9440")
    g = pal("#3e7a2a", "#5a9a3a")
    put(img, 13, 11, g[1])
    put(img, 15, 11, g[0])
    put(img, 14, 11, g[1])
    for (x, y, c) in ((13, 12, O[2]), (14, 12, O[1]), (15, 12, O[0]),
                      (13, 13, O[1]), (14, 13, O[0]), (13, 14, O[0])):
        put(img, x, y, c)
    return outline(img)


def bamboo():
    img = canvas()
    for y in range(2, 15):
        put(img, 6, y, BAMBOO[4])
        put(img, 7, y, BAMBOO[3])
        put(img, 8, y, BAMBOO[1])
    for y in (5, 10):
        put(img, 6, y, BAMBOO[2])
        put(img, 7, y, BAMBOO[1])
        put(img, 8, y, BAMBOO[0])
    for i, (x, y) in enumerate(((9, 4), (10, 3), (11, 3), (12, 2))):
        put(img, x, y, BAMBOO_LEAF[3] if i < 2 else BAMBOO_LEAF[2])
        put(img, x, y + 1, BAMBOO_LEAF[1])
    return outline(img)


def chainmail(kind):
    rng = rng_for("chainmail_" + kind)
    img = canvas()
    m = mask_from_rows(ARMOR_SHAPES[kind])
    shade_mask(img, m, CHAIN[1:], rng, speck=0.0)
    pad = np.pad(m, 1)
    inner = m & pad[:-2, 1:-1] & pad[2:, 1:-1] & pad[1:-1, :-2] & pad[1:-1, 2:]
    # little rings: a staggered grid of dark holes
    holes = inner & (YY % 2 == 1) & ((XX + YY // 2) % 2 == 0)
    img[holes, :3] = CHAIN[0]
    if kind == "helmet":
        img[8, 3:13, :3] = CHAIN[2]
    elif kind == "chestplate":
        img[13, 5:11, :3] = CHAIN[1]
    elif kind == "leggings":
        img[2, 3:13, :3] = CHAIN[4]
        img[4, 4:12, :3] = CHAIN[1]
    elif kind == "boots":
        img[4, 2:6, :3] = CHAIN[4]
        img[4, 10:14, :3] = CHAIN[4]
        img[11, 1:6, :3] = CHAIN[1]
        img[11, 10:15, :3] = CHAIN[1]
    return outline(img)


VINYL = pal("#121218", "#1c1c24", "#25252e", "#30303a", "#464656")


def music_disc(label):
    lc = pal(label)[0]
    img = canvas()
    d = np.hypot(XX + 0.5 - 8, YY + 0.5 - 8)
    ang = np.degrees(np.arctan2(YY + 0.5 - 8, XX + 0.5 - 8))
    m = d <= 7.2
    img[m, :3] = VINYL[1]
    img[m, 3] = 255
    groove = m & (((d > 3.6) & (d < 4.4)) | ((d > 5.2) & (d < 5.9)))
    img[groove, :3] = VINYL[2]
    # sheen on the upper-left of the record
    sheen = m & (d > 3.4) & (d < 6.4) & (ang > -160) & (ang < -110)
    img[sheen, :3] = VINYL[3]
    img[sheen & (d > 4.4) & (d < 5.3), :3] = VINYL[4]
    # rim: the disc's own darkest tone as its outline
    rim = m & (d > 6.3)
    img[rim, :3] = VINYL[0]
    lab = d <= 2.6
    img[lab, :3] = lc
    img[lab & (XX + YY <= 13), :3] = scale_rgb(lc, 1.12)
    img[lab & (XX + YY >= 17), :3] = scale_rgb(lc, 0.74)
    for (x, y) in ((7, 7), (8, 7), (7, 8), (8, 8)):
        put(img, x, y, VINYL[0])
    return img


DISCS = {
    "music_disc_first_light": "#e6da8e",
    "music_disc_moss": "#6a8a3c",
    "music_disc_still_water": "#3c6aae",
    "music_disc_block_by_block": "#ae703c",
    "music_disc_embers": "#d0542c",
    "music_disc_void": "#6c3e9a",
    "music_disc_beginning": "#e8e8e2",
    "music_disc_wandering": "#2a9a8a",
    "music_disc_harvest": "#d2a84c",
}


# extra items: white bed, iron door, cake, kelp ---------------------------------------------

def white_bed_item():
    return to_white_wool(load("item", "red_bed"))


def iron_door_item():
    img = load("item", "oak_door")
    rgb = img[..., :3]
    mapping = {"513612": IRON[1], "6f5430": IRON[2], "ab874f": IRON[4], "c49e5f": IRON[5],
               "46464a": DARK_IRON[2]}
    out = rgb.copy()
    for hx, c in mapping.items():
        col = np.array([int(hx[i:i + 2], 16) for i in (0, 2, 4)])
        sel = np.all(np.abs(rgb - col) < 1, axis=-1) & (img[..., 3] > 127)
        out[sel] = c
    rgb[:] = out
    return img


def cake_item():
    img = canvas()
    # top of the cake
    top = [(4, 4, 11), (5, 2, 13), (6, 1, 14), (7, 1, 14), (8, 1, 14)]
    for y, x0, x1 in top:
        for x in range(x0, x1 + 1):
            put(img, x, y, CREAM[3] if x < 8 else CREAM[2])
    for x in range(4, 12):
        put(img, x, 4, CREAM[4])
    # front: icing lip, sponge, filling, sponge
    for x in range(1, 15):
        left = x < 8
        put(img, x, 9, CREAM[3] if left else CREAM[1])
        for y in (10, 11, 13, 14):
            put(img, x, y, SPONGE_CAKE[3] if left else SPONGE_CAKE[2])
        put(img, x, 12, CREAM[2] if left else CREAM[1])
        put(img, x, 14, SPONGE_CAKE[1] if left else SPONGE_CAKE[0])
    for x in (2, 6, 11):
        put(img, x, 10, CREAM[2])
    for (x, y) in ((4, 5), (9, 5), (6, 7), (11, 7), (2, 7)):
        put(img, x, y, BERRY[2])
        put(img, x + 1, y, BERRY[1])
    return outline(img)


def kelp_item():
    img = canvas()
    for y in range(2, 15):
        x = 7 + int(round(math.sin(2 * math.pi * y / 10)))
        put(img, x, y, KELP[4])
        put(img, x + 1, y, KELP[2])
    put(img, 8, 1, KELP[4])
    for (by, side) in ((5, -1), (10, 1)):
        bx = 7 + int(round(math.sin(2 * math.pi * by / 10))) + (0 if side < 0 else 1)
        for i in range(4):
            put(img, bx + side * (i + 1), by - i // 2, KELP[4] if i % 2 == 0 else KELP[3])
            put(img, bx + side * (i + 1), by - i // 2 + 1, KELP[1])
        put(img, bx + side, by + 2, KELP_BULB[1])
    return outline(img)


# --------------------------------------------------------------------------
# registry & output
# --------------------------------------------------------------------------

BLOCKS = {
    "coarse_dirt": coarse_dirt, "podzol_top": podzol_top, "podzol_side": podzol_side,
    "rooted_dirt": rooted_dirt, "dirt_path_top": dirt_path_top, "dirt_path_side": dirt_path_side,
    "red_sandstone_top": red_sandstone_top, "red_sandstone_side": red_sandstone_side,
    "red_sandstone_bottom": red_sandstone_bottom, "smooth_sandstone": smooth_sandstone,
    "smooth_red_sandstone": smooth_red_sandstone, "smooth_quartz": smooth_quartz,
    "deepslate_coal_ore": deepslate_coal_ore, "deepslate_iron_ore": deepslate_iron_ore,
    "deepslate_gold_ore": deepslate_gold_ore, "deepslate_diamond_ore": deepslate_diamond_ore,
    "coal_block": coal_block, "ladder": ladder, "iron_door_lower": iron_door_lower,
    "iron_door_upper": iron_door_upper, "note_block": note_block, "jukebox_side": jukebox_side,
    "jukebox_top": jukebox_top, "wet_sponge": wet_sponge, "kelp": kelp, "kelp_top": kelp_top,
    "dried_kelp_block_side": dried_kelp_block_side, "dried_kelp_block_top": dried_kelp_block_top,
    "bamboo_stalk": bamboo_stalk, "cake_top": cake_top, "cake_side": cake_side,
    "cake_bottom": cake_bottom, "cake_inner": cake_inner,
    "sweet_berry_bush_stage0": lambda: sweet_berry_bush(0),
    "sweet_berry_bush_stage1": lambda: sweet_berry_bush(1),
    "sweet_berry_bush_stage2": lambda: sweet_berry_bush(2),
    "sweet_berry_bush_stage3": lambda: sweet_berry_bush(3),
    "carved_pumpkin": carved_pumpkin, "jack_o_lantern": jack_o_lantern, "fire": fire,
    "soul_fire": soul_fire, "spawner": spawner, "infested_stone": infested_stone,
    "white_bed_top": white_bed_top, "white_bed_side": white_bed_side,
    "white_bed_end": white_bed_end, "white_shulker_box": white_shulker_box,
    "white_shulker_box_top": white_shulker_box_top,
}

ITEMS = {
    "iron_nugget": iron_nugget, "bowl": bowl, "cookie": cookie, "pumpkin_pie": pumpkin_pie,
    "poisonous_potato": poisonous_potato, "sweet_berries": sweet_berries,
    "mushroom_stew": mushroom_stew, "beetroot_soup": beetroot_soup, "rabbit_stew": rabbit_stew,
    "suspicious_stew": suspicious_stew, "dried_kelp": dried_kelp, "tropical_fish": tropical_fish,
    "cocoa_beans": cocoa_beans, "snowball": snowball, "egg": egg, "armor_stand": armor_stand,
    "shield": shield, "shears": shears, "totem_of_undying": totem_of_undying,
    "writable_book": writable_book, "written_book": written_book,
    "carrot_on_a_stick": carrot_on_a_stick, "bamboo": bamboo,
    "chainmail_helmet": lambda: chainmail("helmet"),
    "chainmail_chestplate": lambda: chainmail("chestplate"),
    "chainmail_leggings": lambda: chainmail("leggings"),
    "chainmail_boots": lambda: chainmail("boots"),
    "white_bed": white_bed_item, "iron_door": iron_door_item, "cake": cake_item,
    "kelp": kelp_item,
}
ITEMS.update({k: (lambda c=c: music_disc(c)) for k, c in DISCS.items()})


def to_png(img, path):
    arr = np.clip(np.rint(img), 0, 255).astype(np.uint8)
    arr[arr[..., 3] < 128, 3] = 0
    arr[arr[..., 3] == 0, :3] = 0
    arr[arr[..., 3] > 0, 3] = 255
    Image.fromarray(arr, "RGBA").save(path)


def main():
    force = "--force" in sys.argv
    out = PACK
    if "--out" in sys.argv:  # write somewhere else to look at the designs first
        out = sys.argv[sys.argv.index("--out") + 1]
        force = True
        for k in ("block", "item"):
            os.makedirs(os.path.join(out, k), exist_ok=True)
    jobs = [("block", n, f) for n, f in BLOCKS.items()] + [("item", n, f) for n, f in ITEMS.items()]
    # never overwrite someone else's texture
    clash = [f"{k}/{n}" for k, n, _ in jobs if os.path.exists(os.path.join(PACK, k, n + ".png"))]
    if clash and not force:
        sys.exit("refusing to overwrite existing textures (use --force to re-run lote 1): "
                 + ", ".join(clash))
    for kind, name, fn in jobs:
        img = fn()
        assert img.shape == (N, N, 4), name
        to_png(img, os.path.join(out, kind, name + ".png"))
    names = sorted({n for _, n, _ in jobs})
    print(f"wrote {len(jobs)} 16x16 designs; now run:")
    print("python3 tools/tex64/build.py --only " + ",".join(names))


if __name__ == "__main__":
    main()
