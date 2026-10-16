#!/usr/bin/env python3
"""Procedurally generate original 16x16 pixel-art block textures.

Everything here is drawn from scratch with hand-picked palettes, tileable
value noise, ordered dithering and a few hand-written pixel shapes.  No
external texture files are read.

Usage:
    python3 tools/gen_textures.py [output_dir]

Output: RGBA 16x16 PNGs in packs/vanilla/textures/block/ plus preview.png
(a 4x contact sheet for review).  Output is deterministic: every texture gets
its own RNG seeded from a fixed base seed and the texture name.
"""

import os
import sys
import zlib

import numpy as np
from PIL import Image

N = 16
BASE_SEED = 0x5EED_B10C
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "block")

YY, XX = np.mgrid[0:N, 0:N]

# --------------------------------------------------------------------------
# Shared helpers: rng, palettes, noise, dithering, canvas ops
# --------------------------------------------------------------------------


def rng_for(name):
    """Deterministic RNG per texture, independent of generation order."""
    return np.random.default_rng(BASE_SEED ^ zlib.crc32(name.encode()))


def hexrgb(h):
    h = h.lstrip("#")
    return [int(h[i:i + 2], 16) for i in (0, 2, 4)]


def pal(*hexes):
    """Palette as an (n, 3) array, ordered dark -> light."""
    return np.array([hexrgb(h) for h in hexes], dtype=np.float64)


def canvas():
    return np.zeros((N, N, 4), dtype=np.float64)


def from_rgb(rgb, alpha=255):
    img = canvas()
    img[..., :3] = rgb
    img[..., 3] = alpha
    return img


def value_noise(rng, cw, ch=None):
    """Tileable value noise with cells of cw x ch pixels (divisors of 16)."""
    ch = ch or cw
    gw, gh = N // cw, N // ch
    grid = rng.random((gh, gw))
    fx = (np.arange(N) + 0.5) / cw - 0.5
    fy = (np.arange(N) + 0.5) / ch - 0.5
    x0 = np.floor(fx).astype(int)
    y0 = np.floor(fy).astype(int)
    tx = fx - x0
    ty = fy - y0
    tx = tx * tx * (3 - 2 * tx)
    ty = ty * ty * (3 - 2 * ty)
    x0m, x1m = x0 % gw, (x0 + 1) % gw
    y0m, y1m = y0 % gh, (y0 + 1) % gh
    a = grid[np.ix_(y0m, x0m)]
    b = grid[np.ix_(y0m, x1m)]
    c = grid[np.ix_(y1m, x0m)]
    d = grid[np.ix_(y1m, x1m)]
    top = a + (b - a) * tx[None, :]
    bot = c + (d - c) * tx[None, :]
    return top + (bot - top) * ty[:, None]


def fbm(rng, octaves=((8, 0.4), (4, 0.3), (2, 0.2), (1, 0.25))):
    """Sum of value-noise octaves, normalised to 0..1."""
    f = sum(w * value_noise(rng, c) for c, w in octaves)
    return normalize(f)


def normalize(f):
    lo, hi = f.min(), f.max()
    return (f - lo) / (hi - lo + 1e-9)


BAYER4 = np.array([[0, 8, 2, 10],
                   [12, 4, 14, 6],
                   [3, 11, 1, 9],
                   [15, 7, 13, 5]], dtype=np.float64) / 16.0 - 0.47


def dither_index(field, n, strength=0.6):
    """Quantise a 0..1 field into n palette indices with ordered dithering."""
    bayer = np.tile(BAYER4, (N // 4, N // 4))
    idx = np.floor(field * n + bayer * strength)
    return np.clip(idx, 0, n - 1).astype(int)


def shade(field, palette, strength=0.6):
    """Map a 0..1 field to palette colours (dithered)."""
    return palette[dither_index(field, len(palette), strength)]


def scale_rgb(rgb, k):
    return np.clip(np.asarray(rgb, dtype=np.float64) * k, 0, 255)


def put(img, x, y, color, alpha=255):
    img[y % N, x % N, :3] = color
    img[y % N, x % N, 3] = alpha


def line(img, x0, y0, x1, y1, color, alpha=255):
    """Bresenham line (no wrap-around)."""
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        if 0 <= x0 < N and 0 <= y0 < N:
            put(img, x0, y0, color, alpha)
        if x0 == x1 and y0 == y1:
            break
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy


def bevel(img, light=1.18, dark=0.72, inset=0):
    """Light top/left edge, dark bottom/right edge (classic block frame)."""
    a, b = inset, N - 1 - inset
    rgb = img[..., :3]
    rgb[a, a:b + 1] = scale_rgb(rgb[a, a:b + 1], light)
    rgb[a:b + 1, a] = scale_rgb(rgb[a:b + 1, a], light)
    rgb[b, a:b + 1] = scale_rgb(rgb[b, a:b + 1], dark)
    rgb[a + 1:b + 1, b] = scale_rgb(rgb[a + 1:b + 1, b], dark)


def voronoi(rng, count, min_dist):
    """Tileable Voronoi: returns (cell id, d1, d2) per pixel."""
    pts = []
    tries = 0
    while len(pts) < count and tries < 5000:
        tries += 1
        p = rng.random(2) * N
        if all(torus_dist(p, q) >= min_dist for q in pts):
            pts.append(p)
    pts = np.array(pts)
    px = XX[..., None] + 0.5
    py = YY[..., None] + 0.5
    dx = np.abs(px - pts[:, 0])
    dy = np.abs(py - pts[:, 1])
    dx = np.minimum(dx, N - dx)
    dy = np.minimum(dy, N - dy)
    # slightly squashed metric makes stones look chunkier / rounder
    d = np.sqrt(dx ** 2 + (dy * 1.1) ** 2)
    order = np.argsort(d, axis=-1)
    ids = order[..., 0]
    ds = np.take_along_axis(d, order, axis=-1)
    return ids, ds[..., 0], ds[..., 1], len(pts)


def torus_dist(p, q):
    d = np.abs(p - q)
    d = np.minimum(d, N - d)
    return float(np.hypot(*d))


def neighbor_differs(ids, dx, dy):
    return ids != np.roll(np.roll(ids, -dy, axis=0), -dx, axis=1)


# --------------------------------------------------------------------------
# Palettes
# --------------------------------------------------------------------------

STONE = pal("#5d5d5d", "#6b6b6b", "#777777", "#828282", "#8e8e8e", "#9c9c9c")
DIRT = pal("#5a3d24", "#6b4a2d", "#7a5636", "#8a6542", "#9b7551")
GRASS = pal("#3e7a25", "#4b8b2c", "#57993a", "#64a743", "#74b54f", "#86c35d")
SAND = pal("#c9b27a", "#d4bf87", "#dccb93", "#e5d5a1", "#eddfb0")
SANDSTONE = pal("#a99560", "#bba774", "#cbb884", "#d8c793", "#e3d4a2")
SNOW = pal("#d8e2ea", "#e5edf2", "#eff4f8", "#f8fbfd", "#ffffff")
COBBLE = pal("#3b3b3b", "#5a5a5a", "#6f6f6f", "#848484", "#999999", "#adadad")
MOSS = pal("#3a5a24", "#4a6e2c", "#5c8236", "#6d9544")

WOODS = {
    "oak": dict(
        bark=pal("#3c2a16", "#4a3520", "#5a4229", "#6a5033", "#7b5e3c"),
        inner=pal("#8d6f3f", "#a3844f", "#b4955e", "#c2a46b"),
        plank=pal("#6f5430", "#866639", "#9a7744", "#ab874f", "#b9955a"),
        leaves=pal("#22501a", "#2e6421", "#3b782a", "#4b8c33", "#5fa340"),
    ),
    "birch": dict(
        bark=pal("#2a2a26", "#bdbbb2", "#cfcdc5", "#dcdad3", "#e9e8e2"),
        inner=pal("#b9a46c", "#cdb97d", "#dac88d", "#e4d49b"),
        plank=pal("#a8955e", "#bba76c", "#c9b679", "#d4c286", "#ddcc92"),
        leaves=pal("#3e6a24", "#4d7d2c", "#5e9136", "#70a442", "#86b752"),
    ),
    "spruce": dict(
        bark=pal("#21170c", "#2c1f11", "#382816", "#45321c", "#523c23"),
        inner=pal("#6a4f2b", "#7d5f35", "#8e6e3f", "#9c7b48"),
        plank=pal("#4a3420", "#573e26", "#654a2d", "#725434", "#7f5e3a"),
        leaves=pal("#163a24", "#1e4a2d", "#285b37", "#336b41", "#40794c"),
    ),
}

ORES = {
    "coal": pal("#141414", "#262626", "#3a3a3a"),
    "iron": pal("#a0745a", "#c49678", "#e2bea0"),
    "copper": pal("#a04f26", "#d97a3a", "#f0a060", "#3f9f8a"),
    "tin": pal("#7d8896", "#a6b0bd", "#cdd5df"),
    "silver": pal("#b4b8bf", "#dfe2e6", "#ffffff"),
    "gold": pal("#b8860b", "#f0c419", "#fff27a"),
    "diamond": pal("#1e9fa0", "#4fe0d8", "#b8fff6"),
    "redstone": pal("#8f0000", "#d01010", "#ff4a3a"),
    "lapis": pal("#1a2f8a", "#2c4fc0", "#5b80e8"),
    "emerald": pal("#0d7a2f", "#17b54a", "#6cf08e"),
}

WOOL = {
    "white": "#e6e9e9", "orange": "#ea7a1c", "magenta": "#b84bb0",
    "light_blue": "#45a8d6", "yellow": "#f2c52c", "lime": "#74b928",
    "pink": "#e891aa", "gray": "#474c50", "light_gray": "#8f8f88",
    "cyan": "#1a8a92", "purple": "#7a30aa", "blue": "#363c9c",
    "brown": "#704a2c", "green": "#56701f", "red": "#a2302a",
    "black": "#1c1d22",
}

# --------------------------------------------------------------------------
# Natural ground blocks
# --------------------------------------------------------------------------


def stone(name="stone"):
    rng = rng_for(name)
    f = fbm(rng, ((8, 0.25), (4, 0.35), (2, 0.35), (1, 0.3)))
    img = from_rgb(shade(f, STONE))
    # a few short dark cracks / light flecks for the classic speckle
    for _ in range(5):
        x, y = rng.integers(0, N, 2)
        ln = rng.integers(2, 4)
        for i in range(ln):
            put(img, x + i, y, STONE[0] if rng.random() < 0.7 else STONE[1])
    for _ in range(6):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, STONE[5])
    return img


def dirt(name="dirt"):
    rng = rng_for(name)
    f = fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.45)))
    img = from_rgb(shade(f, DIRT, 0.8))
    for _ in range(10):  # pebbles / dark specks
        x, y = rng.integers(0, N, 2)
        put(img, x, y, DIRT[0] if rng.random() < 0.6 else DIRT[4])
    return img


def grass_top(name="grass_block_top"):
    rng = rng_for(name)
    f = fbm(rng, ((4, 0.25), (2, 0.35), (1, 0.5)))
    img = from_rgb(shade(f, GRASS, 0.9))
    for _ in range(12):  # blade highlights
        x, y = rng.integers(0, N, 2)
        put(img, x, y, GRASS[5] if rng.random() < 0.5 else GRASS[0])
    return img


def fringe_side(name, fringe_pal, base_depth=3, drip=True):
    """Dirt with an irregular fringe (grass / snow) hanging from the top."""
    rng = rng_for(name)
    img = dirt(name + "_base")
    f = fbm(rng, ((2, 0.4), (1, 0.6)))
    col = shade(f, fringe_pal, 0.9)
    depth = base_depth + rng.integers(0, 2, N)
    if drip:
        for x in rng.choice(N, 4, replace=False):
            depth[x] += rng.integers(1, 3)
    for x in range(N):
        d = depth[x]
        img[:d, x, :3] = col[:d, x]
        # darker underside pixel gives the fringe some depth
        img[d - 1, x, :3] = scale_rgb(img[d - 1, x, :3], 0.85)
    return img


def grass_side():
    return fringe_side("grass_block_side", GRASS)


def snow_grass_side():
    img = fringe_side("snow_grass_side", SNOW)
    return img


def sand(name="sand"):
    rng = rng_for(name)
    f = 0.4 * fbm(rng, ((4, 0.4), (2, 0.6))) + 0.6 * rng.random((N, N))
    img = from_rgb(shade(f, SAND, 0.4))
    return img


def gravel():
    rng = rng_for("gravel")
    ids, d1, d2, n = voronoi(rng, 22, 2.6)
    stones = pal("#5e5a58", "#706b68", "#837d79", "#958f8a", "#a79f99",
                 "#7a6e64", "#8e8078", "#625a55")
    cols = stones[rng.integers(0, len(stones), n)]
    img = from_rgb(cols[ids])
    rgb = img[..., :3]
    edge = (d2 - d1) < 0.9
    rgb[edge] = pal("#4a4644")[0]
    # light top-left of each pebble
    hi = neighbor_differs(ids, 0, -1) & ~edge
    rgb[hi] = scale_rgb(rgb[hi], 1.15)
    return img


def clay():
    rng = rng_for("clay")
    f = fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.25)))
    p = pal("#8f94a4", "#9ba0b0", "#a6abba", "#b0b5c3")
    img = from_rgb(shade(f, p, 0.5))
    return img


def sandstone_top():
    rng = rng_for("sandstone_top")
    f = fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.3)))
    img = from_rgb(shade(0.35 + 0.45 * f, SANDSTONE, 0.5))
    bevel(img, 1.05, 0.9)
    return img


def sandstone_side():
    rng = rng_for("sandstone_side")
    rough = fbm(rng, ((2, 0.5), (1, 0.6)))
    streak = value_noise(rng, 8, 1)
    img = from_rgb(shade(0.45 + 0.4 * streak, SANDSTONE, 0.3))
    rgb = img[..., :3]
    # rough top band
    rgb[0:3] = shade(0.5 + 0.5 * rough, SANDSTONE, 0.8)[0:3]
    rgb[3] = SANDSTONE[1]
    # faint horizontal seams in the smooth middle
    for y in (6, 9):
        xs = rng.random(N) < 0.75
        rgb[y, xs] = SANDSTONE[2]
    # rough darker bottom band with an irregular top edge
    bottom = shade(0.2 + 0.5 * rough, SANDSTONE, 0.8)
    for x in range(N):
        top = 12 + int(rng.integers(0, 2))
        rgb[top:, x] = bottom[top:, x]
        rgb[top - 1, x] = SANDSTONE[1]
    return img


def sandstone_bottom():
    rng = rng_for("sandstone_bottom")
    f = fbm(rng, ((2, 0.5), (1, 0.6)))
    return from_rgb(shade(0.15 + 0.6 * f, SANDSTONE, 0.9))


def bedrock():
    rng = rng_for("bedrock")
    f = fbm(rng, ((4, 0.3), (2, 0.6), (1, 0.4)))
    p = pal("#1c1c1c", "#333333", "#4f4f4f", "#6e6e6e", "#8f8f8f", "#a8a8a8")
    return from_rgb(shade(f, p, 1.0))


def snow():
    rng = rng_for("snow")
    f = fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3)))
    return from_rgb(shade(0.3 + 0.7 * f, SNOW, 0.5))


def ice():
    rng = rng_for("ice")
    p = pal("#7fa3e6", "#8db0ec", "#9bbcf1", "#acc9f5", "#c3daf9")
    f = 0.6 * value_noise(rng, 8) + 0.4 * value_noise(rng, 4)
    img = from_rgb(shade(normalize(f) * 0.6, p, 0.4))
    # diagonal frosty streaks
    for (x0, y0, ln) in ((2, 12, 6), (8, 9, 5), (11, 4, 4), (3, 4, 3)):
        for i in range(ln):
            put(img, x0 + i, y0 - i, p[4])
            put(img, x0 + i + 1, y0 - i, p[3])
    return img


def water():
    rng = rng_for("water")
    p = pal("#2a4fb8", "#3159c4", "#3964cf", "#4471d8", "#5583e2")
    wave = (np.sin(2 * np.pi * (XX + 2 * YY) / N)
            + 0.6 * np.sin(2 * np.pi * (2 * XX - YY) / N + 1.3))
    f = normalize(wave) * 0.7 + 0.3 * value_noise(rng, 4)
    img = from_rgb(shade(normalize(f) * 0.85 + 0.1, p, 0.5), alpha=170)
    return img


def lava():
    rng = rng_for("lava")
    p = pal("#7e1600", "#b02a00", "#d84a00", "#ef7410", "#ff9f22", "#ffc845",
            "#ffe788")
    f = fbm(rng, ((8, 0.45), (4, 0.45), (2, 0.25), (1, 0.08)))
    f = f ** 1.25
    return from_rgb(shade(f, p, 0.5))


def obsidian():
    rng = rng_for("obsidian")
    p = pal("#0d0a14", "#151020", "#1e162e", "#2b2042", "#3e2e5e", "#5a4682")
    f = fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.4)))
    return from_rgb(shade(f ** 1.6, p, 0.8))


# --------------------------------------------------------------------------
# Ores
# --------------------------------------------------------------------------


def ore(kind):
    """Stone background with a few small, outlined clusters of colour."""
    name = kind + "_ore"
    rng = rng_for(name)
    img = stone(name + "_stone")
    p = ORES[kind]
    accent = p[3] if len(p) > 3 else None
    centers = [(3, 3), (11, 3), (7, 8), (2, 12), (12, 11)]
    jitter = rng.integers(-1, 2, (len(centers), 2))
    used = np.zeros((N, N), bool)
    shapes = []
    for (cx, cy), (jx, jy) in zip(centers, jitter):
        cx, cy = cx + jx, cy + jy
        pix = {(cx, cy), (cx + 1, cy), (cx, cy + 1)}
        if rng.random() < 0.6:
            pix.add((cx + 1, cy + 1))
        target = int(rng.integers(5, 8))
        while len(pix) < target:
            bx, by = sorted(pix)[rng.integers(0, len(pix))]
            dx, dy = [(1, 0), (0, 1), (-1, 0), (0, -1)][rng.integers(0, 4)]
            pix.add((bx + dx, by + dy))
        shapes.append(pix)
        for (x, y) in pix:
            used[y % N, x % N] = True
    # dark drop-shadow below/right of each speck
    for pix in shapes:
        for (x, y) in pix:
            for (sx, sy) in ((1, 1), (0, 1)):
                qx, qy = (x + sx) % N, (y + sy) % N
                if not used[qy, qx]:
                    put(img, qx, qy, scale_rgb(img[qy, qx, :3], 0.72))
    for pix in shapes:
        for (x, y) in pix:
            tl = (x - 1, y) not in pix and (x, y - 1) not in pix
            col = p[2] if tl else p[1]
            if (x + 1, y) not in pix and (x, y + 1) not in pix and len(pix) > 2:
                col = p[0]
            if accent is not None and rng.random() < 0.35:
                col = accent
            put(img, x, y, col)
    return img


# --------------------------------------------------------------------------
# Stones & bricks
# --------------------------------------------------------------------------


def cobblestone(name="cobblestone", moss=False):
    rng = rng_for(name)
    ids, d1, d2, n = voronoi(rng, 11, 4.2)
    base = rng.integers(2, 5, n)
    lvl = base[ids].astype(int)
    edge = (d2 - d1) < 1.05
    # pixels just inside the top/left border are highlighted,
    # bottom/right are shadowed -> rounded look
    above_edge = np.roll(edge, 1, axis=0) | np.roll(edge, 1, axis=1)
    below_edge = np.roll(edge, -1, axis=0) | np.roll(edge, -1, axis=1)
    lvl = np.where(above_edge, lvl + 1, lvl)
    lvl = np.where(below_edge & ~above_edge, lvl - 1, lvl)
    lvl = lvl + (rng.random((N, N)) < 0.12) * rng.choice([-1, 1], (N, N))
    lvl = np.clip(lvl, 1, 5)
    lvl[edge] = 0
    img = from_rgb(COBBLE[lvl])
    if moss:
        f = fbm(rng, ((8, 0.4), (4, 0.4), (2, 0.3), (1, 0.2)))
        m = f + 0.25 * edge > 0.6
        mid = shade(fbm(rng, ((2, 0.5), (1, 0.5))), MOSS, 0.8)
        img[m, :3] = mid[m]
        img[m & edge, :3] = MOSS[0] * 0.7
    return img


def mossy_cobblestone():
    return cobblestone("mossy_cobblestone", moss=True)


def bricks():
    rng = rng_for("bricks")
    reds = pal("#7f3a2c", "#8f4434", "#9c4d3b", "#a85744")
    mortar = pal("#9c968c", "#b4ada2", "#c4beb3")
    img = canvas()
    img[..., 3] = 255
    noise = rng.random((N, N))
    for y in range(N):
        row = y // 4
        off = 4 if row % 2 else 0
        for x in range(N):
            if y % 4 == 3:
                img[y, x, :3] = mortar[1] if noise[y, x] > 0.3 else mortar[0]
                continue
            if (x + off) % 8 == 7:
                img[y, x, :3] = mortar[2] if y % 4 == 0 else mortar[1]
                continue
            brick = (row, ((x + off) // 8))
            k = (zlib.crc32(repr(brick).encode()) % 3)
            c = reds[k + (1 if noise[y, x] > 0.85 else 0)]
            if y % 4 == 0:
                c = scale_rgb(c, 1.1)
            if y % 4 == 2:
                c = scale_rgb(c, 0.9)
            img[y, x, :3] = c
    return img


def stone_bricks():
    rng = rng_for("stone_bricks")
    f = fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.4)))
    p = STONE[1:]
    img = from_rgb(shade(0.25 + 0.55 * f, p, 0.6))
    rgb = img[..., :3]
    dark, mortar, light = pal("#4f4f4f", "#5c5c5c", "#a8a8a8")
    for y in range(N):
        row = y // 8
        off = 4 if row % 2 else 0
        for x in range(N):
            lx, ly = (x + off) % 8, y % 8
            if ly == 7 or lx == 7:
                rgb[y, x] = mortar
            elif ly == 6 or lx == 6:
                rgb[y, x] = scale_rgb(rgb[y, x], 0.78)
            elif ly == 0 or lx == 0:
                rgb[y, x] = scale_rgb(rgb[y, x], 1.12)
    # a couple of cracks
    for _ in range(2):
        x, y = rng.integers(1, 14, 2)
        for i in range(3):
            put(img, x + i, y + (i % 2), dark)
    return img


# --------------------------------------------------------------------------
# Wood family
# --------------------------------------------------------------------------


def log_side(wood):
    rng = rng_for(wood + "_log")
    w = WOODS[wood]
    if wood == "birch":
        f = value_noise(rng, 4, 8) * 0.5 + 0.5 * rng.random((N, N))
        img = from_rgb(shade(0.4 + 0.6 * f, w["bark"][1:], 0.5))
        dark = w["bark"][0]
        # dark horizontal lenticels / patches
        for _ in range(7):
            x, y = rng.integers(0, N, 2)
            ln = rng.integers(2, 6)
            for i in range(ln):
                put(img, x + i, y, dark)
            if rng.random() < 0.5:
                put(img, x + 1, y + 1, scale_rgb(w["bark"][1], 0.6))
        return img
    cols = rng.random(N)
    cols = normalize(cols + np.roll(cols, 1) * 0.5)
    stretch = value_noise(rng, 1, 4)
    f = 0.55 * cols[None, :] + 0.45 * stretch
    img = from_rgb(shade(normalize(f), w["bark"], 0.7))
    # vertical cracks
    for x in rng.choice(N, 4, replace=False):
        y0 = rng.integers(0, N)
        for i in range(rng.integers(4, 10)):
            put(img, x, y0 + i, w["bark"][0])
    return img


def log_top(wood):
    rng = rng_for(wood + "_log_top")
    w = WOODS[wood]
    cx = cy = 7.5
    euc = np.hypot(XX - cx, YY - cy)
    cheb = np.maximum(np.abs(XX - cx), np.abs(YY - cy))
    r = 0.55 * euc + 0.45 * cheb + 0.5 * value_noise(rng, 4)
    rings = (np.floor(r * 0.75) % 2 == 0)
    inner = w["inner"]
    img = from_rgb(np.where(rings[..., None], inner[2], inner[1]))
    rgb = img[..., :3]
    speck = rng.random((N, N))
    rgb[speck > 0.9] = inner[3]
    rgb[(speck < 0.06)] = inner[0]
    rgb[cheb < 1.2] = inner[0]
    # bark rim
    rim = cheb >= 6.9
    bark = w["bark"][2 if wood != "birch" else 3]
    rgb[rim] = bark
    edge2 = (cheb >= 6.9) & (rng.random((N, N)) < 0.4)
    rgb[edge2] = w["bark"][1 if wood != "birch" else 2]
    if wood == "birch":
        rgb[rim & (rng.random((N, N)) < 0.2)] = w["bark"][0]
    # inner edge of bark is darker
    ring_in = (cheb >= 5.9) & (cheb < 6.9)
    rgb[ring_in] = scale_rgb(rgb[ring_in], 0.85)
    return img


def planks(wood):
    rng = rng_for(wood + "_planks")
    p = WOODS[wood]["plank"]
    grain = 0.6 * value_noise(rng, 8, 1) + 0.4 * value_noise(rng, 4, 1)
    img = from_rgb(shade(normalize(grain) * 0.7 + 0.2, p[1:], 0.5))
    rgb = img[..., :3]
    seam_x = [3, 11, 6, 14]
    for b in range(4):
        y0 = b * 4
        rgb[y0 + 3, :] = p[0]                       # board bottom seam
        rgb[y0, :] = scale_rgb(rgb[y0, :], 1.06)    # top lit edge
        sx = (seam_x[b] + int(rng.integers(-1, 2))) % N
        rgb[y0:y0 + 3, sx] = p[0]                   # end-grain joint
        # grain streaks
        for _ in range(2):
            gx, gy = rng.integers(0, N), y0 + rng.integers(0, 3)
            for i in range(rng.integers(2, 5)):
                if (gx + i) % N != sx:
                    put(img, gx + i, gy, p[1])
    return img


def leaves(wood):
    rng = rng_for(wood + "_leaves")
    p = WOODS[wood]["leaves"]
    f = fbm(rng, ((4, 0.35), (2, 0.4), (1, 0.45)))
    img = from_rgb(shade(f, p, 1.0))
    # cluster outlines: darken pixels left/below of bright pixels
    holes = (f < 0.22) & (rng.random((N, N)) < 0.75)
    holes |= rng.random((N, N)) < 0.05
    img[holes, 3] = 0
    img[holes, :3] = 0
    return img


# --------------------------------------------------------------------------
# Plants (cross sprites, fully transparent background)
# --------------------------------------------------------------------------


def short_grass():
    rng = rng_for("short_grass")
    img = canvas()
    p = pal("#2f6a1f", "#3e7f29", "#4f9434", "#62a841", "#79bd52")
    for x in list(range(1, 15)):
        if rng.random() < 0.25:
            continue
        h = int(rng.integers(5, 13))
        lean = int(rng.integers(-1, 2))
        for i in range(h):
            y = 15 - i
            xx = x + (lean if i > h * 0.6 else 0)
            t = i / max(h - 1, 1)
            put(img, xx, y, p[min(4, int(t * 4.5))])
    return img


def flower(name, petal, center, head_y, big):
    rng = rng_for(name)
    img = canvas()
    stem = pal("#2c6620", "#3f8a2c", "#55a33b")
    for y in range(head_y + 2, 16):
        put(img, 7, y, stem[1])
        put(img, 8, y, stem[0] if y % 3 else stem[1])
    # leaves on the stem
    for (x, y, c) in ((6, 12, 2), (5, 11, 2), (9, 13, 2), (10, 12, 1), (6, 13, 1)):
        put(img, x, y, stem[c])
    if big:  # poppy: chunky 6x5 bloom with dark eye
        shape = ["..##..",
                 ".####.",
                 "######",
                 "######",
                 ".####.",
                 "..##.."]
    else:    # dandelion: small puff
        shape = [".##.",
                 "####",
                 "####",
                 ".##."]
    h = len(shape)
    w = len(shape[0])
    x0 = 8 - w // 2
    for j, row in enumerate(shape):
        for i, ch in enumerate(row):
            if ch == "#":
                k = 2 if (i + j) < w // 2 + 1 else (0 if (i + j) > w else 1)
                if rng.random() < 0.15:
                    k = min(2, k + 1)
                put(img, x0 + i, head_y + j - h // 2, petal[k])
    cy = head_y - h // 2 + h // 2
    put(img, 7, cy - (0 if big else 1), center)
    put(img, 8, cy, center)
    return img


def poppy():
    return flower("poppy", pal("#9a0e10", "#cc1f1a", "#ee3b2e"),
                  hexrgb("#2a1a10"), 6, True)


def dandelion():
    return flower("dandelion", pal("#d6a10e", "#f2c919", "#fff15a"),
                  hexrgb("#e09a10"), 7, False)


def dead_bush():
    img = canvas()
    p = pal("#5a3c1c", "#73502a", "#8b6436")
    branches = [((8, 15), (8, 10)), ((8, 12), (4, 8)), ((8, 11), (12, 6)),
                ((8, 10), (7, 4)), ((5, 9), (3, 5)), ((5, 9), (2, 9)),
                ((11, 7), (13, 4)), ((10, 9), (14, 9)), ((7, 6), (5, 3)),
                ((8, 13), (11, 12)), ((7, 5), (9, 2))]
    for i, ((x0, y0), (x1, y1)) in enumerate(branches):
        line(img, x0, y0, x1, y1, p[i % 3])
    return img


def cactus_side():
    rng = rng_for("cactus_side")
    p = pal("#0d4a16", "#126020", "#1a7a2a", "#259135", "#36a845")
    cols = np.array([1, 2, 3, 3, 2, 1, 2, 3, 4, 3, 2, 1, 2, 3, 2, 1])
    lvl = cols[None, :] + (rng.random((N, N)) < 0.2) * rng.choice([-1, 1], (N, N))
    img = from_rgb(p[np.clip(lvl, 0, 4)])
    for x in (0, 5, 11, 15):
        img[:, x, :3] = p[0]
    spike = hexrgb("#e8e6b0")
    for y in range(1, N, 4):
        for x in (2, 8, 13):
            put(img, x, y + (x % 3) - 1, spike)
            put(img, x, y + (x % 3), p[0])
    return img


def cactus_top():
    rng = rng_for("cactus_top")
    p = pal("#0d4a16", "#126020", "#1a7a2a", "#259135", "#36a845", "#5ac063")
    cheb = np.maximum(np.abs(XX - 7.5), np.abs(YY - 7.5))
    lvl = np.full((N, N), 3)
    lvl[cheb > 6.9] = 0
    lvl[(cheb > 5.9) & (cheb <= 6.9)] = 2
    lvl[cheb < 4] = 4
    lvl[cheb < 2] = 5
    lvl = lvl + (rng.random((N, N)) < 0.15) * -1
    img = from_rgb(p[np.clip(lvl, 0, 5)])
    for (x, y) in ((4, 4), (11, 4), (4, 11), (11, 11), (7, 3), (3, 8), (12, 7), (8, 12)):
        put(img, x, y, hexrgb("#e8e6b0"))
    return img


def torch():
    img = canvas()
    stick = pal("#4a3218", "#6b4a26", "#8a6334")
    for y in range(9, 16):
        put(img, 7, y, stick[2] if y < 15 else stick[1])
        put(img, 8, y, stick[1] if y < 15 else stick[0])
    put(img, 7, 6, hexrgb("#fffbe0"))
    put(img, 8, 6, hexrgb("#ffec70"))
    put(img, 7, 7, hexrgb("#ffe040"))
    put(img, 8, 7, hexrgb("#ffc020"))
    put(img, 7, 8, hexrgb("#f59a18"))
    put(img, 8, 8, hexrgb("#d86a10"))
    return img


# --------------------------------------------------------------------------
# Solid crafted blocks
# --------------------------------------------------------------------------


def metal_block(name, p, facets=False, patina=None):
    rng = rng_for(name)
    shine = normalize(np.sin((XX - YY) * 0.6) + 0.3 * rng.random((N, N)))
    img = from_rgb(shade(0.3 + 0.45 * shine, p[1:-1], 0.6))
    rgb = img[..., :3]
    rgb[0, :] = p[-1]
    rgb[:, 0] = p[-1]
    rgb[15, :] = p[0]
    rgb[1:, 15] = p[0]
    rgb[1, 1:15] = scale_rgb(rgb[1, 1:15], 1.06)
    rgb[14, 1:15] = scale_rgb(rgb[14, 1:15], 0.9)
    if facets:
        for (x, y) in ((3, 3), (11, 4), (5, 10), (12, 11), (8, 7)):
            put(img, x, y, p[-1])
            put(img, x + 1, y, p[-2])
            put(img, x, y + 1, p[-2])
            put(img, x + 1, y + 1, p[1])
    else:
        for y in (5, 10):
            for x in range(2, 14):
                if rng.random() < 0.8:
                    put(img, x, y, scale_rgb(rgb[y, x], 0.9))
    if patina is not None:
        for _ in range(9):
            x, y = rng.integers(1, 15, 2)
            put(img, x, y, patina)
    return img


def iron_block():
    return metal_block("iron_block", pal("#8a8a8a", "#c8c8c8", "#d6d6d6",
                                         "#e2e2e2", "#ececec", "#ffffff"))


def gold_block():
    return metal_block("gold_block", pal("#a5781a", "#e8b923", "#f4cb32",
                                         "#fcdb4a", "#fde873", "#fffbc4"))


def diamond_block():
    return metal_block("diamond_block", pal("#2a8f8c", "#5ad8cf", "#73e4dc",
                                            "#94eee6", "#b6f6f0", "#ebfffd"),
                       facets=True)


def copper_block():
    return metal_block("copper_block", pal("#7a3a1c", "#c06a3c", "#cf7a48",
                                           "#dc8a55", "#e69d68", "#f6c29a"),
                       patina=hexrgb("#4a9e8a"))


def glowstone():
    rng = rng_for("glowstone")
    ids, d1, d2, n = voronoi(rng, 12, 3.6)
    p = pal("#8a5a22", "#b8803a", "#d9a24a", "#f0c260", "#ffdd85", "#fff1b8")
    lvl = rng.integers(1, 6, n)[ids]
    edge = (d2 - d1) < 0.9
    lvl[edge] = 0
    img = from_rgb(p[lvl])
    return img


def glass():
    img = canvas()
    frame = pal("#a6c2cc", "#d4e8ee", "#ffffff")
    for i in range(N):
        put(img, i, 0, frame[2])
        put(img, 0, i, frame[2])
        put(img, i, 15, frame[0])
        put(img, 15, i, frame[0])
    put(img, 15, 0, frame[1])
    put(img, 0, 15, frame[1])
    for (x0, y0, ln) in ((3, 6, 4), (4, 7, 2), (9, 13, 5), (11, 13, 2)):
        for i in range(ln):
            put(img, x0 + i, y0 - i, frame[1], 200)
    return img


def crafting_top():
    img = planks("oak")
    rng = rng_for("crafting_table_top")
    dark = pal("#3e2a14")[0]
    mid = pal("#5c4024")[0]
    rgb = img[..., :3]
    rgb[0, :] = rgb[15, :] = dark
    rgb[:, 0] = rgb[:, 15] = dark
    rgb[1, 1:15] = mid
    rgb[1:15, 1] = mid
    # 3x3 crafting grid
    for k in (5, 10):
        rgb[2:14, k] = dark
        rgb[k, 2:14] = dark
    _ = rng
    return img


def crafting_side(front=False):
    img = planks("oak")
    rgb = img[..., :3]
    dark = pal("#3e2a14")[0]
    top = pal("#5c4024", "#7a5631", "#93693b")
    rgb[0:3] = top[1]
    rgb[0] = top[2]
    rgb[3] = dark
    rgb[3:, 0:2] = top[0]
    rgb[3:, 14:16] = top[0]
    rgb[3:, 1] = top[1]
    rgb[15, :] = dark
    metal = pal("#5d5d5d", "#9a9a9a", "#d0d0d0")
    handle = pal("#4a3218", "#6b4a26")
    if front:
        # hammer
        line(img, 4, 13, 8, 9, handle[1])
        line(img, 5, 13, 9, 9, handle[0])
        for (x, y) in ((8, 6), (9, 6), (10, 6), (11, 7), (9, 7), (10, 7), (10, 8), (11, 8), (12, 9)):
            put(img, x, y, metal[1])
        put(img, 8, 6, metal[2])
        put(img, 12, 9, metal[0])
    else:
        # saw
        for x in range(4, 12):
            put(img, x, 6, metal[2])
            put(img, x, 7, metal[1])
            if x % 2 == 0:
                put(img, x, 8, metal[0])
        for (x, y) in ((11, 5), (12, 5), (12, 6), (12, 7), (11, 8), (13, 6)):
            put(img, x, y, handle[1])
        # pliers/chisel
        line(img, 5, 10, 10, 12, metal[1])
        line(img, 10, 12, 12, 13, handle[1])
    return img


def furnace_top():
    rng = rng_for("furnace_top")
    f = fbm(rng, ((4, 0.3), (2, 0.3), (1, 0.4)))
    img = from_rgb(shade(0.35 + 0.45 * f, STONE, 0.5))
    bevel(img, 1.15, 0.7)
    return img


def furnace_side():
    img = cobblestone("furnace_side_cobble")
    rgb = img[..., :3]
    rgb[0:3] = furnace_top()[0:3, :, :3]
    rgb[0] = scale_rgb(rgb[0], 1.15)
    rgb[3] = COBBLE[0]
    bevel(img, 1.1, 0.7)
    return img


def furnace_front():
    img = furnace_side()
    rgb = img[..., :3]
    frame, inner, grate = pal("#3a3a3a", "#141414", "#2a2a2a")
    rgb[8:14, 3:13] = frame
    rgb[9:13, 4:12] = inner
    for x in range(4, 12, 2):
        rgb[11, x] = grate
    rgb[8, 3:13] = pal("#7a7a7a")[0]
    # slot above the mouth
    rgb[5, 5:11] = frame
    return img


def bookshelf():
    rng = rng_for("bookshelf")
    img = planks("oak")
    rgb = img[..., :3]
    back = pal("#2a1d10")[0]
    books = pal("#7a2420", "#2a4d7a", "#2f6a34", "#8a6a2a", "#5c2e6e", "#3d3d3d",
                "#a0522d", "#6b2a2a")
    for (y0, y1) in ((2, 7), (10, 15)):
        rgb[y0 - 1 if y0 > 1 else y0, 1:15] = scale_rgb(rgb[y0 - 1, 1:15], 0.75)
        x = 1
        while x < 15:
            w = int(rng.integers(1, 3))
            top = y0 + int(rng.integers(0, 2))
            c = books[rng.integers(0, len(books))]
            rgb[y0:y1, x:x + w] = back
            if rng.random() < 0.1:
                x += w
                continue
            rgb[top:y1, x:min(x + w, 15)] = c
            rgb[top:y1, x] = scale_rgb(c, 1.25)
            band = top + 1 + int(rng.integers(0, 2))
            if band < y1:
                rgb[band, x:min(x + w, 15)] = pal("#d8c070")[0]
            x += w
        rgb[y1 - 1 if False else y1, 1:15] = scale_rgb(rgb[y1, 1:15], 1.0)
    rgb[:, 0] = scale_rgb(rgb[:, 0], 0.8)
    rgb[:, 15] = scale_rgb(rgb[:, 15], 0.8)
    return img


TNT_RED = pal("#8e1b10", "#b4261a", "#d23a26", "#e0533c")


def tnt_side():
    rng = rng_for("tnt_side")
    img = canvas()
    img[..., 3] = 255
    rgb = img[..., :3]
    for x in range(N):
        stripe = (x // 2) % 2  # paper-wrapped sticks
        for y in range(N):
            k = 2 if stripe else 1
            if rng.random() < 0.15:
                k += 1
            rgb[y, x] = TNT_RED[k]
        rgb[:, x] = rgb[:, x]
    rgb[:, 1::4] = scale_rgb(rgb[:, 1::4], 0.85)
    paper = pal("#c9c9c0", "#e9e9e2", "#ffffff")
    rgb[5:11] = paper[1]
    rgb[5] = paper[2]
    rgb[10] = paper[0]
    rgb[4] = rgb[11] = TNT_RED[0]
    font = {"T": ["###", ".#.", ".#.", ".#."],
            "N": ["#..#", "##.#", "#.##", "#..#"]}
    x = 2
    for ch in "TNT":
        g = font[ch]
        for j, row in enumerate(g):
            for i, c in enumerate(row):
                if c == "#":
                    put(img, x + i, 6 + j, hexrgb("#1a1a1a"))
        x += len(g[0]) + 1
    return img


def tnt_top(bottom=False):
    rng = rng_for("tnt_bottom" if bottom else "tnt_top")
    f = rng.random((N, N))
    img = from_rgb(TNT_RED[np.where(f > 0.8, 2, 1)])
    cheb = np.maximum(np.abs(XX - 7.5), np.abs(YY - 7.5))
    rgb = img[..., :3]
    # stick ends: rings of 4x4 cells
    cell = ((XX % 4 == 0) | (YY % 4 == 0))
    rgb[cell] = TNT_RED[0]
    if not bottom:
        rgb[cheb < 2] = pal("#3a3a3a")[0]
        rgb[(cheb < 1)] = pal("#6a6a6a")[0]
        put(img, 8, 7, hexrgb("#a0a0a0"))
    else:
        rgb[cheb < 2] = TNT_RED[3]
    bevel(img, 1.1, 0.8)
    return img


def pumpkin_side():
    rng = rng_for("pumpkin_side")
    p = pal("#a8540e", "#c6680f", "#dd7d14", "#ec921c", "#f6a830")
    ridge = np.array([0, 2, 3, 4, 3, 2, 1, 2, 3, 4, 4, 3, 2, 1, 3, 2])
    lvl = ridge[None, :] + (rng.random((N, N)) < 0.18) * rng.choice([-1, 1], (N, N))
    lvl[0, :] -= 1
    lvl[15, :] -= 1
    img = from_rgb(p[np.clip(lvl, 0, 4)])
    for x in (0, 6, 13):
        img[:, x, :3] = p[0]
    return img


def pumpkin_top():
    rng = rng_for("pumpkin_top")
    p = pal("#a8540e", "#c6680f", "#dd7d14", "#ec921c", "#f6a830")
    ang = np.arctan2(YY - 7.5, XX - 7.5)
    ridge = np.cos(ang * 6)
    cheb = np.maximum(np.abs(XX - 7.5), np.abs(YY - 7.5))
    f = 0.55 + 0.35 * ridge - 0.25 * (cheb / 8) + 0.1 * rng.random((N, N))
    img = from_rgb(shade(np.clip(f, 0, 1), p, 0.4))
    stem = pal("#3d4a16", "#566a20", "#6f852c")
    for (x, y, k) in ((7, 7, 2), (8, 7, 1), (7, 8, 1), (8, 8, 0), (6, 7, 0), (7, 6, 2)):
        put(img, x, y, stem[k])
    bevel(img, 1.0, 0.85)
    return img


def melon_side():
    rng = rng_for("melon_side")
    p = pal("#4a6e12", "#5c8418", "#709a20", "#86ad2c", "#9cc03e")
    stripes = np.array([0, 1, 3, 4, 3, 1, 0, 2, 3, 4, 3, 2, 0, 1, 3, 2])
    lvl = stripes[None, :] + (rng.random((N, N)) < 0.25) * rng.choice([-1, 1], (N, N))
    return from_rgb(p[np.clip(lvl, 0, 4)])


def melon_top():
    rng = rng_for("melon_top")
    p = pal("#4a6e12", "#5c8418", "#709a20", "#86ad2c", "#9cc03e")
    ang = np.arctan2(YY - 7.5, XX - 7.5)
    f = 0.5 + 0.45 * np.cos(ang * 8) + 0.15 * rng.random((N, N))
    img = from_rgb(shade(np.clip(f, 0, 1), p, 0.5))
    stem = pal("#5a4a20", "#7a6430")
    put(img, 7, 7, stem[1])
    put(img, 8, 8, stem[0])
    put(img, 8, 7, stem[0])
    put(img, 7, 8, stem[0])
    return img


def sponge():
    rng = rng_for("sponge")
    p = pal("#8f8a20", "#b4ac30", "#c8c040", "#d8d052", "#e6de6a")
    f = fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.3)))
    img = from_rgb(shade(0.4 + 0.5 * f, p, 0.6))
    for _ in range(14):  # pores: dark hole with a lit lower rim
        x, y = rng.integers(0, N, 2)
        put(img, x, y, p[0] * 0.8)
        if rng.random() < 0.5:
            put(img, x + 1, y, p[0])
        put(img, x, y + 1, p[4])
    return img


def wool(color):
    """Knitted cloth: shared stitch pattern applied to a base colour."""
    rng = rng_for("wool_pattern")  # same weave for every colour
    stitch = (((XX + YY) % 4 == 0) | ((XX - YY) % 4 == 0)).astype(float)
    curls = value_noise(rng, 2) * 0.5 + rng.random((N, N)) * 0.5
    off = (curls - 0.5) * 0.26 - stitch * 0.07
    base = np.array(hexrgb(WOOL[color]), dtype=float)
    rgb = base * (1 + off[..., None]) + off[..., None] * 40
    return from_rgb(np.clip(rgb, 0, 255))


# --------------------------------------------------------------------------
# Registry & output
# --------------------------------------------------------------------------


def build_registry():
    reg = {
        "stone": stone, "cobblestone": cobblestone,
        "mossy_cobblestone": mossy_cobblestone, "dirt": dirt,
        "grass_block_top": grass_top, "grass_block_side": grass_side,
        "sand": sand, "gravel": gravel, "clay": clay,
        "sandstone_top": sandstone_top, "sandstone_side": sandstone_side,
        "sandstone_bottom": sandstone_bottom, "bedrock": bedrock,
        "water": water, "lava": lava,
    }
    for wood in ("oak", "birch", "spruce"):
        reg[f"{wood}_log"] = lambda w=wood: log_side(w)
        reg[f"{wood}_log_top"] = lambda w=wood: log_top(w)
        reg[f"{wood}_planks"] = lambda w=wood: planks(w)
        reg[f"{wood}_leaves"] = lambda w=wood: leaves(w)
    reg.update({
        "glass": glass, "snow": snow, "ice": ice,
        "cactus_side": cactus_side, "cactus_top": cactus_top,
        "short_grass": short_grass, "poppy": poppy, "dandelion": dandelion,
        "dead_bush": dead_bush,
    })
    for kind in ORES:
        reg[f"{kind}_ore"] = lambda k=kind: ore(k)
    reg.update({
        "iron_block": iron_block, "gold_block": gold_block,
        "diamond_block": diamond_block, "copper_block": copper_block,
        "bricks": bricks, "stone_bricks": stone_bricks, "obsidian": obsidian,
        "glowstone": glowstone, "torch": torch,
        "crafting_table_top": crafting_top,
        "crafting_table_side": lambda: crafting_side(False),
        "crafting_table_front": lambda: crafting_side(True),
        "furnace_front": furnace_front, "furnace_side": furnace_side,
        "furnace_top": furnace_top, "bookshelf": bookshelf,
        "tnt_side": tnt_side, "tnt_top": tnt_top,
        "tnt_bottom": lambda: tnt_top(bottom=True),
        "pumpkin_side": pumpkin_side, "pumpkin_top": pumpkin_top,
        "melon_side": melon_side, "melon_top": melon_top, "sponge": sponge,
        "snow_grass_side": snow_grass_side,
    })
    for color in WOOL:
        reg[f"{color}_wool"] = lambda c=color: wool(c)
    return reg


def to_image(arr):
    arr = np.clip(np.rint(arr), 0, 255).astype(np.uint8)
    # fully transparent pixels get zeroed colour so cutout edges stay clean
    arr[arr[..., 3] == 0, :3] = 0
    return Image.fromarray(arr, "RGBA")


def contact_sheet(images, scale=4, gap=2, cols=12):
    tile = N * scale
    rows = (len(images) + cols - 1) // cols
    w = cols * tile + (cols + 1) * gap
    h = rows * tile + (rows + 1) * gap
    sheet = Image.new("RGBA", (w, h), (128, 128, 128, 255))
    for i, img in enumerate(images):
        r, c = divmod(i, cols)
        big = img.resize((tile, tile), Image.NEAREST)
        x = gap + c * (tile + gap)
        y = gap + r * (tile + gap)
        sheet.alpha_composite(big, (x, y))
    return sheet


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else OUT_DIR
    os.makedirs(out, exist_ok=True)
    images = []
    for name, fn in build_registry().items():
        img = to_image(fn())
        assert img.size == (N, N)
        img.save(os.path.join(out, name + ".png"))
        images.append(img)
    contact_sheet(images).save(os.path.join(out, "preview.png"))
    print(f"wrote {len(images)} textures + preview.png to {out}")


if __name__ == "__main__":
    main()
