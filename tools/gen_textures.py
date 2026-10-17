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
DIRT = pal("#614329", "#6c4c30", "#775536", "#825f3d", "#8d6945")
# Soft, low-contrast palettes: the classic blocks read as one colour with gentle speckle.
GRASS = pal("#4c7a30", "#548436", "#5b8c3b", "#629440", "#6a9c46", "#74a54e")
SAND = pal("#d3c091", "#d9c799", "#dfcea2", "#e4d5aa", "#eadcb3")
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
    "jungle": dict(
        bark=pal("#3b2b12", "#4d3a1a", "#5f4a22", "#6f582a", "#7d6534"),
        inner=pal("#a2703f", "#b5804b", "#c38f56", "#cf9c62"),
        plank=pal("#7e5434", "#94633e", "#a5714a", "#b47e54", "#c08a5e"),
        leaves=pal("#1f5a12", "#2a7018", "#378620", "#45992a", "#58ad38"),
    ),
    "acacia": dict(
        bark=pal("#3e3a33", "#4f4a42", "#5f5950", "#6e685e", "#7b7569"),
        inner=pal("#a5512a", "#b85e32", "#c96b3a", "#d67844"),
        plank=pal("#8c4422", "#a3512a", "#b45d32", "#c2683a", "#cd7343"),
        leaves=pal("#3b5a1a", "#4a6c20", "#5a7e28", "#6a8f30", "#7c9f3b"),
    ),
    "dark_oak": dict(
        bark=pal("#1d140a", "#271b0e", "#322313", "#3d2b18", "#48331d"),
        inner=pal("#4a3219", "#583c1f", "#664625", "#73502b"),
        plank=pal("#2e1e0e", "#3a2713", "#463018", "#51381c", "#5b4021"),
        leaves=pal("#173c10", "#204c16", "#2a5c1c", "#346b22", "#3f7a2a"),
    ),
    "cherry": dict(
        bark=pal("#2a161c", "#361d24", "#43252c", "#502d34", "#5d363c"),
        inner=pal("#c88d86", "#d79c94", "#e3aba2", "#edb9b0"),
        plank=pal("#c08a80", "#cf998e", "#dba79b", "#e5b3a7", "#efc0b4"),
        leaves=pal("#c25e8e", "#d673a2", "#e488b4", "#ef9fc5", "#f8bcd9"),
    ),
}

NEW_WOODS = ("jungle", "acacia", "dark_oak", "cherry")
RED_SAND = pal("#ae5824", "#b75f28", "#c0662d", "#c96e33", "#d1763a")
TERRACOTTA = {
    "terracotta": "#985e44",
    "white_terracotta": "#cdb1a2",
    "orange_terracotta": "#a2552a",
    "yellow_terracotta": "#b98428",
    "red_terracotta": "#8e3c2d",
    "brown_terracotta": "#4f3424",
    "light_gray_terracotta": "#86695f",
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
    # Etapa 5 ore ladder: the second metal of each pair and the gems.
    "lead": pal("#3a3f4c", "#565d6e", "#7b8396"),
    "tungsten": pal("#3c4a3e", "#5f7a63", "#93b098"),
    "platinum": pal("#7f9aa6", "#b8d2dc", "#eefaff"),
    "ruby": pal("#7a0a1e", "#d0203c", "#ff7a8c"),
    "sapphire": pal("#0c2a7a", "#2456d0", "#7aa4ff"),
    "topaz": pal("#8a5a06", "#e0a218", "#ffe07a"),
    "amethyst": pal("#4a1a7a", "#8a42c8", "#d4a2ff"),
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
    f = 0.55 * fbm(rng, ((4, 0.4), (2, 0.6))) + 0.45 * rng.random((N, N))
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


def ore(kind, bg=None, p=None):
    """Stone background with a few small, outlined clusters of colour."""
    name = kind + "_ore"
    rng = rng_for(name)
    img = bg if bg is not None else stone(name + "_stone")
    p = p if p is not None else ORES[kind]
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
    if wood == "cherry":
        # short horizontal lenticels, lighter than the bark
        for _ in range(6):
            x, y = rng.integers(0, N, 2)
            for i in range(rng.integers(2, 4)):
                put(img, x + i, y, w["bark"][4])
    elif wood == "jungle":
        # a few specks of moss
        moss = pal("#3e5a1e", "#4d6d26")
        for _ in range(7):
            x, y = rng.integers(0, N, 2)
            put(img, x, y, moss[rng.integers(0, 2)])
    elif wood == "acacia":
        # long pale streaks of weathered bark
        for x in rng.choice(N, 3, replace=False):
            y0 = rng.integers(0, N)
            for i in range(rng.integers(3, 7)):
                put(img, x, y0 + i, w["bark"][4])
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
    if wood == "cherry":
        # pale blossoms with a darker heart
        for _ in range(9):
            x, y = rng.integers(0, N, 2)
            put(img, x, y, hexrgb("#fde4ef"))
            put(img, x + 1, y + 1, hexrgb("#b04a7c"))
    elif wood == "jungle":
        # broad leaves: bright veins on some clusters
        for _ in range(6):
            x, y = rng.integers(0, N, 2)
            line(img, x, y, x + 2, y + 2, p[4])
    return img


# --------------------------------------------------------------------------
# Badlands and swamp blocks
# --------------------------------------------------------------------------


def red_sand():
    rng = rng_for("red_sand")
    f = 0.4 * fbm(rng, ((4, 0.4), (2, 0.6))) + 0.6 * rng.random((N, N))
    img = from_rgb(shade(f, RED_SAND, 0.4))
    for _ in range(5):  # a few dark grains
        x, y = rng.integers(0, N, 2)
        put(img, x, y, scale_rgb(RED_SAND[0], 0.85))
    return img


def terracotta(name):
    """Fired clay: a smooth matte colour with faint mottling and a few specks."""
    rng = rng_for(name)
    base = np.array(hexrgb(TERRACOTTA[name]), dtype=np.float64)
    p = np.array([scale_rgb(base, k) for k in (0.9, 0.95, 1.0, 1.04)])
    f = fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.2)))
    img = from_rgb(shade(0.2 + 0.6 * f, p, 0.5))
    for _ in range(6):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, scale_rgb(base, 0.84 if rng.random() < 0.6 else 1.1))
    return img


def mushroom(name, cap, spots, flat):
    img = canvas()
    stem = pal("#b9ad98", "#d6cdb9", "#ece6d6")
    for y in range(9, 16):
        put(img, 7, y, stem[1])
        put(img, 8, y, stem[0] if y > 12 else stem[2])
    if flat:  # wide flat cap
        rows = ["..######..",
                ".########.",
                "##########"]
        y0 = 7
    else:     # round dome
        rows = ["...####...",
                ".########.",
                "##########",
                "##########"]
        y0 = 5
    for j, row in enumerate(rows):
        for i, ch in enumerate(row):
            if ch == "#":
                k = 2 if j == 0 or i < 3 else (0 if j == len(rows) - 1 else 1)
                put(img, 3 + i, y0 + j, cap[k])
    for (x, y) in spots:
        put(img, x, y, hexrgb("#f4efe6"))
    return img


def red_mushroom():
    return mushroom("red_mushroom", pal("#8e1414", "#bd2222", "#de3a32"),
                    ((5, 7), (9, 6), (11, 8), (7, 8)), False)


def brown_mushroom():
    return mushroom("brown_mushroom", pal("#7a5a40", "#957154", "#ad8a6a"),
                    (), True)


def vine():
    """Hanging strands with small leaves (cross sprite, tinted by biome)."""
    rng = rng_for("vine")
    img = canvas()
    p = pal("#2f6a1f", "#3e7f29", "#4f9434", "#62a841")
    for x in (2, 6, 9, 13):
        length = int(rng.integers(10, 17))
        xx = x
        for y in range(length):
            if rng.random() < 0.2:
                xx += int(rng.integers(-1, 2))
            put(img, xx, y, p[1])
            if rng.random() < 0.45:  # a leaf to one side
                side = 1 if rng.random() < 0.5 else -1
                put(img, xx + side, y, p[3] if side < 0 else p[2])
                if rng.random() < 0.5:
                    put(img, xx + side, y + 1, p[0])
    return img


def lily_pad():
    """A round floating leaf seen from above, with a notch and radiating veins."""
    rng = rng_for("lily_pad")
    p = pal("#1f5a1c", "#2a6e24", "#36822c", "#459536")
    img = canvas()
    d = np.hypot(XX - 7.5, YY - 7.5)
    ang = np.arctan2(YY - 7.5, XX - 7.5)
    pad = (d < 7.2 + 0.4 * value_noise(rng, 4)) & ~((np.abs(ang - 0.5) < 0.28) & (d > 1.5))
    f = fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3)))
    rgb = shade(0.3 + 0.5 * f, p, 0.6)
    img[pad, :3] = rgb[pad]
    img[pad, 3] = 255
    # veins
    for a in np.linspace(0, 2 * np.pi, 7, endpoint=False) + 1.2:
        for r in range(2, 7):
            x, y = int(round(7.5 + r * np.cos(a))), int(round(7.5 + r * np.sin(a)))
            if pad[y % N, x % N]:
                put(img, x, y, p[3])
    rim = pad & (d > 6.2)
    img[rim, :3] = p[0]
    return img


def lily_pad_side():
    img = canvas()
    p = pal("#1f5a1c", "#2a6e24")
    for x in range(1, 15):
        put(img, x, 14, p[1])
        put(img, x, 15, p[0])
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


# Blocks of the Etapa 5 metals and gems: (palette dark..light, faceted like a gem).
METAL_BLOCKS = {
    "tin_block": (pal("#5e6875", "#9aa4b1", "#a9b3bf", "#b8c1cc", "#c8d0d9", "#e0e6ec"), False),
    "silver_block": (pal("#7e838c", "#c6cad0", "#d2d5da", "#dde0e4", "#e8eaed", "#ffffff"), False),
    "bronze_block": (pal("#5e3a12", "#a8763a", "#b58444", "#c29250", "#cfa05c", "#e8c47e"), False),
    "lead_block": (pal("#2e323c", "#4f5666", "#5a6172", "#656d7e", "#717a8b", "#8c95a6"), False),
    "tungsten_block": (pal("#2f3a31", "#56705a", "#617c65", "#6c8870", "#78947c", "#9cb8a0"), False),
    "platinum_block": (pal("#6f8a96", "#b4ced8", "#c2dae3", "#cfe5ec", "#dcf0f6", "#f6ffff"), False),
    "ruby_block": (pal("#5a0614", "#b01a32", "#c2223c", "#d22e48", "#e24a62", "#ff9aaa"), True),
    "sapphire_block": (pal("#081e5a", "#1c46b0", "#2452c0", "#2e60d0", "#4878e0", "#9cc0ff"), True),
    "topaz_block": (pal("#6a4404", "#c48a12", "#d29a18", "#e0aa22", "#ecbc3a", "#fff0a0"), True),
    "amethyst_block": (pal("#36125a", "#7234ae", "#7e3cbc", "#8a46c8", "#a062dc", "#e0b8ff"), True),
}


def life_crystal():
    """A heart-red crystal growing up out of the floor: three tall shards, lit at the tips."""
    img = canvas()
    p = pal("#4a0610", "#8a1020", "#c42034", "#ee4a5a", "#ffb0b8")
    shards = [(7, 2, 3), (3, 6, 2), (11, 5, 2)]  # (x, top y, half width at base)
    for (cx, top, hw) in shards:
        for y in range(top, 16):
            t = (y - top) / max(1, 15 - top)
            w = max(0, int(round(hw * min(1.0, t * 1.6))))
            for x in range(cx - w, cx + w + 1):
                if x < 0 or x >= N:
                    continue
                side = (x - cx) / (w + 0.5) if w else 0
                k = 3 if side < -0.3 else (2 if side < 0.4 else 1)
                if y == top or (y < top + 3 and x == cx):
                    k = 4
                put(img, x, y, p[k])
    for (x, y) in ((7, 4), (3, 8), (11, 7), (6, 9)):
        put(img, x, y, p[4])
    return img


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
    for wood in ("oak", "birch", "spruce") + NEW_WOODS:
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
    for name, (pp, facets) in METAL_BLOCKS.items():
        reg[name] = lambda n=name, q=pp, f=facets: metal_block(n, q, f)
    reg.update({
        "life_crystal": life_crystal,
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
    reg.update({
        "red_sand": red_sand, "red_mushroom": red_mushroom,
        "brown_mushroom": brown_mushroom, "vine": vine,
        "lily_pad": lily_pad, "lily_pad_side": lily_pad_side,
    })
    for name in TERRACOTTA:
        reg[name] = lambda n=name: terracotta(n)
    reg.update(redstone_registry())
    reg.update(phase4_registry())
    reg.update(decor_registry())
    reg.update(extras_registry())
    reg.update(nether_registry())
    reg.update(end_registry())
    reg.update(farming_registry())
    return reg



# --------------------------------------------------------------------------
# Redstone (Fase 4)
# --------------------------------------------------------------------------

SMOOTH = pal("#8c8c8c", "#9a9a9a", "#a6a6a6", "#b0b0b0", "#bababa")
RS_DARK = pal("#3a0606", "#4e0a0a", "#5e1010")
RS_LIT = pal("#a00c0c", "#d81a12", "#ff4a30", "#ff8a6a")


def smooth_stone(name="smooth_stone"):
    rng = rng_for(name)
    f = 0.55 + 0.25 * value_noise(rng, 8) + 0.1 * rng.random((N, N))
    img = from_rgb(shade(np.clip(f, 0, 1), SMOOTH[1:4], 0.4))
    rgb = img[..., :3]
    rgb[0, :] = SMOOTH[4]
    rgb[:, 0] = SMOOTH[4]
    rgb[15, :] = SMOOTH[0]
    rgb[:, 15] = SMOOTH[0]
    return img


def redstone_torch(lit=True):
    img = torch()
    p = RS_LIT if lit else RS_DARK
    for (x, y, k) in ((7, 6, 3 if lit else 2), (8, 6, 2), (7, 7, 2 if lit else 1), (8, 7, 1), (7, 8, 1), (8, 8, 0)):
        put(img, x, y, p[min(k, len(p) - 1)])
    return img


def redstone_block():
    rng = rng_for("redstone_block")
    p = pal("#6a0606", "#8e0c0a", "#b0140e", "#cf2216", "#ec3a26")
    f = fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.4)))
    img = from_rgb(shade(f, p[1:], 0.7))
    rgb = img[..., :3]
    for _ in range(7):
        x, y = rng.integers(1, 15, 2)
        put(img, x, y, p[4])
        put(img, x + 1, y + 1, p[0])
    rgb[0, :] = p[4]
    rgb[:, 0] = p[4]
    rgb[15, :] = p[0]
    rgb[1:, 15] = p[0]
    return img


def redstone_lamp(on=False):
    rng = rng_for("redstone_lamp_on" if on else "redstone_lamp")
    if on:
        frame = pal("#7a4a20", "#a06830", "#c48a48")
        glass = pal("#e8a840", "#f6c860", "#ffe08a", "#fff4c8")
    else:
        frame = pal("#3c2210", "#5a361c", "#76502c")
        glass = pal("#2a1a10", "#3a2616", "#4a3220", "#5e4430")
    f = fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3)))
    img = from_rgb(shade(f, glass, 0.7))
    rgb = img[..., :3]
    # A frame with four panes, like a lantern seen from the side.
    for i in (0, 7, 8, 15):
        rgb[i, :] = frame[1]
        rgb[:, i] = frame[1]
    rgb[0, :] = frame[2]
    rgb[:, 0] = frame[2]
    rgb[15, :] = frame[0]
    rgb[:, 15] = frame[0]
    for (x, y) in ((2, 2), (10, 2), (2, 10), (10, 10)):
        put(img, x, y, glass[-1])
        put(img, x + 1, y, glass[-2])
    return img


def lever():
    img = canvas()
    stick = pal("#4a3218", "#6b4a26", "#8a6334")
    for y in range(6, 16):
        put(img, 7, y, stick[2] if y > 6 else stick[1])
        put(img, 8, y, stick[1] if y > 6 else stick[0])
    return img


def repeater(on=False):
    img = smooth_stone("repeater_on" if on else "repeater")
    p = RS_LIT if on else RS_DARK
    for y in range(1, 15):
        put(img, 7, y, p[1])
        put(img, 8, y, p[2] if on else p[1])
    # An arrow head at the top: the side the signal leaves.
    for (x, y) in ((6, 2), (9, 2), (5, 3), (10, 3)):
        put(img, x, y, p[1])
    return img


def piston_side():
    rng = rng_for("piston_side")
    img = cobblestone("piston_side")
    rgb = img[..., :3]
    plank = WOODS["oak"]["plank"]
    grain = value_noise(rng, 8, 1)
    for y in range(0, 4):
        for x in range(N):
            rgb[y, x] = plank[1 + int(grain[y, x] * 3)]
    rgb[3, :] = plank[0]
    rgb[0, :] = plank[4]
    rgb[4:, 0] = scale_rgb(rgb[4:, 0], 0.7)
    rgb[4:, 15] = scale_rgb(rgb[4:, 15], 0.7)
    rgb[15, :] = scale_rgb(rgb[15, :], 0.7)
    return img


def piston_top(sticky=False):
    rng = rng_for("piston_top_sticky" if sticky else "piston_top")
    img = planks("oak")
    rgb = img[..., :3]
    cheb = np.maximum(np.abs(XX - 7.5), np.abs(YY - 7.5))
    if sticky:
        slime = pal("#3c8a2c", "#58a83c", "#78c450", "#9adc6c")
        f = rng.random((N, N))
        mask = cheb < 5.5
        rgb[mask] = slime[np.clip((f[mask] * 4).astype(int), 0, 3)]
    else:
        iron = pal("#7a7a7a", "#a0a0a0", "#c4c4c4")
        rgb[cheb < 3] = iron[1]
        rgb[(cheb >= 2.5) & (cheb < 3)] = iron[0]
        put(img, 6, 6, iron[2])
    bevel(img, 1.15, 0.75)
    return img


def piston_bottom():
    img = cobblestone("piston_bottom")
    rgb = img[..., :3]
    cheb = np.maximum(np.abs(XX - 7.5), np.abs(YY - 7.5))
    rgb[cheb < 2.5] = scale_rgb(rgb[cheb < 2.5], 0.6)
    bevel(img, 1.15, 0.7)
    return img


def wire(kind):
    """Redstone dust, light grey so the game can tint it by power. Arms are four pixels wide."""
    rng = rng_for("redstone_wire_" + kind)
    arms = {"line": "NS", "cross": "NESW", "t": "NEW", "corner": "NE"}[kind]
    mask = np.zeros((N, N), bool)
    if "N" in arms:
        mask[0:9, 6:10] = True
    if "S" in arms:
        mask[7:16, 6:10] = True
    if "E" in arms:
        mask[6:10, 7:16] = True
    if "W" in arms:
        mask[6:10, 0:9] = True
    if kind != "line":
        mask[5:11, 5:11] = True
    # Ragged edges: dust, not paint.
    edge = mask & ~(np.roll(mask, 1, 0) & np.roll(mask, -1, 0) & np.roll(mask, 1, 1) & np.roll(mask, -1, 1))
    holes = edge & (rng.random((N, N)) < 0.35)
    mask &= ~holes
    img = canvas()
    shade_ = 0.78 + 0.22 * rng.random((N, N))
    img[..., 0] = img[..., 1] = img[..., 2] = 255 * shade_
    img[..., 3] = np.where(mask, 255, 0)
    return img



def comparator(on=False):
    img = smooth_stone("comparator_on" if on else "comparator")
    p = RS_LIT if on else RS_DARK
    # Lines from the two back torches meeting at the front one.
    for y in range(4, 12):
        put(img, 5, y, p[1])
        put(img, 10, y, p[1])
    for x in range(5, 11):
        put(img, x, 4, p[1])
    put(img, 7, 3, p[2] if on else p[1])
    put(img, 8, 3, p[2] if on else p[1])
    return img


METAL_DARK = pal("#1e1e20", "#2e2e32", "#3c3c42", "#4e4e56", "#646470")


def observer_side():
    rng = rng_for("observer_side")
    f = fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3)))
    img = from_rgb(shade(f, METAL_DARK[1:4], 0.6))
    rgb = img[..., :3]
    # An arrow pointing up: the way it looks.
    for y in range(3, 13):
        rgb[y, 7:9] = METAL_DARK[4]
    for k in range(3):
        rgb[3 + k, 7 - k - 1] = METAL_DARK[4]
        rgb[3 + k, 8 + k + 1] = METAL_DARK[4]
    bevel(img, 1.25, 0.7)
    return img


def observer_front():
    img = from_rgb(METAL_DARK[1])
    rgb = img[..., :3]
    # A face: two dark eye slots over a grille mouth.
    rgb[2:7, 2:7] = METAL_DARK[0]
    rgb[2:7, 9:14] = METAL_DARK[0]
    rgb[3:6, 3:6] = METAL_DARK[3]
    rgb[3:6, 10:13] = METAL_DARK[3]
    for y in (10, 12):
        rgb[y, 3:13] = METAL_DARK[0]
    bevel(img, 1.25, 0.7)
    return img


def observer_back():
    img = observer_front()
    rgb = img[..., :3]
    rgb[2:14, 2:14] = METAL_DARK[2]
    rgb[6:10, 6:10] = RS_DARK[1]
    rgb[7:9, 7:9] = RS_DARK[2]
    return img


def dropper_front(dispenser=False):
    img = furnace_side()
    rgb = img[..., :3]
    frame, inner = pal("#3a3a3a", "#141414")
    if dispenser:
        # A round mouth.
        d = np.hypot(XX - 7.5, YY - 7.5)
        rgb[d < 4.2] = frame
        rgb[d < 3.0] = inner
    else:
        rgb[5:11, 5:11] = frame
        rgb[6:10, 6:10] = inner
    return img


def hopper_outside():
    rng = rng_for("hopper_outside")
    f = fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3)))
    img = from_rgb(shade(f, METAL_DARK[2:], 0.6))
    bevel(img, 1.2, 0.7)
    return img


def hopper_top():
    img = hopper_outside()
    rgb = img[..., :3]
    rgb[2:14, 2:14] = METAL_DARK[0]
    rgb[2, 2:14] = METAL_DARK[1]
    rgb[2:14, 2] = METAL_DARK[1]
    return img

def redstone_registry():
    return {
        "smooth_stone": smooth_stone,
        "redstone_torch": redstone_torch,
        "redstone_torch_off": lambda: redstone_torch(False),
        "redstone_block": redstone_block,
        "redstone_lamp": redstone_lamp,
        "redstone_lamp_on": lambda: redstone_lamp(True),
        "lever": lever,
        "repeater": repeater,
        "repeater_on": lambda: repeater(True),
        "piston_side": piston_side,
        "piston_top": piston_top,
        "piston_top_sticky": lambda: piston_top(True),
        "piston_bottom": piston_bottom,
        "redstone_wire_line": lambda: wire("line"),
        "redstone_wire_cross": lambda: wire("cross"),
        "redstone_wire_t": lambda: wire("t"),
        "redstone_wire_corner": lambda: wire("corner"),
        "comparator": comparator,
        "comparator_on": lambda: comparator(True),
        "observer_side": observer_side,
        "observer_front": observer_front,
        "observer_back": observer_back,
        "dropper_front": dropper_front,
        "dispenser_front": lambda: dropper_front(True),
        "hopper_outside": hopper_outside,
        "hopper_top": hopper_top,
    }

# --------------------------------------------------------------------------
# Phase 4: crafter, the copper family (four stages of rust) and rails.

COPPER_STAGES = ["", "exposed_", "weathered_", "oxidized_"]
COPPER_PAL = [
    pal("#7a3a1c", "#c06a3c", "#cf7a48", "#dc8a55", "#e69d68", "#f6c29a"),
    pal("#6e4a3a", "#a7765e", "#b58467", "#c19172", "#cc9f82", "#dcb59a"),
    pal("#36604e", "#5c8a6e", "#6c9a78", "#7aa884", "#8fb995", "#b0d1b0"),
    pal("#235a4e", "#4a9a85", "#56a690", "#62b29c", "#74c0aa", "#9ad8c4"),
]
# Spots of the next colour along (green on copper, copper on the green).
COPPER_SPOT = ["#4a9e8a", "#5c9a78", "#b5805c", "#3c8a76"]


def copper_base(k, name):
    """Copper sheet of stage k with a few spots of the next stage's colour."""
    p = COPPER_PAL[k]
    rng = rng_for(name)
    shine = normalize(np.sin((XX - YY) * 0.6) + 0.3 * rng.random((N, N)))
    img = from_rgb(shade(0.3 + 0.45 * shine, p[1:-1], 0.6))
    spots = fbm(rng, ((4, 0.5), (2, 0.5))) > (0.78 - 0.08 * k)
    img[spots, :3] = hexrgb(COPPER_SPOT[k])
    return img, p


def copper_stage_block(k):
    name = COPPER_STAGES[k] + "copper" if k else "copper_block"
    img, p = copper_base(k, name)
    rgb = img[..., :3]
    rgb[0, :] = p[-1]
    rgb[:, 0] = p[-1]
    rgb[15, :] = p[0]
    rgb[1:, 15] = p[0]
    for y in (5, 10):
        for x in range(2, 14):
            put(img, x, y, scale_rgb(rgb[y, x], 0.9))
    return img


def cut_copper(k):
    img, p = copper_base(k, COPPER_STAGES[k] + "cut_copper")
    rgb = img[..., :3]
    # Four plates, each with a lit top-left and a dark bottom-right edge.
    for (x0, y0) in ((0, 0), (8, 0), (0, 8), (8, 8)):
        rgb[y0, x0:x0 + 8] = p[-1]
        rgb[y0:y0 + 8, x0] = p[-1]
        rgb[y0 + 7, x0:x0 + 8] = p[0]
        rgb[y0:y0 + 8, x0 + 7] = p[0]
    return img


def chiseled_copper(k):
    img, p = copper_base(k, COPPER_STAGES[k] + "chiseled_copper")
    rgb = img[..., :3]
    bevel(img, 1.25, 0.6)
    # Engraved squares, one inside the other.
    for r, c in ((5, 0), (2, -1)):
        a, b = 7 - r, 8 + r
        col = p[0] if c == 0 else p[-1]
        rgb[a, a:b + 1] = col
        rgb[b, a:b + 1] = col
        rgb[a:b + 1, a] = col
        rgb[a:b + 1, b] = col
    rgb[7:9, 7:9] = p[-1]
    return img


def copper_grate(k):
    img, p = copper_base(k, COPPER_STAGES[k] + "copper_grate")
    rgb = img[..., :3]
    bevel(img, 1.2, 0.65)
    # A lattice: 2×2 holes in every 4×4 cell, inside a frame.
    for y in range(1, 15):
        for x in range(1, 15):
            if (x % 4) in (1, 2) and (y % 4) in (1, 2):
                img[y, x, 3] = 0
            elif (x % 4) == 0 or (y % 4) == 0:
                rgb[y, x] = scale_rgb(rgb[y, x], 0.85)
    return img


def copper_door(k, upper):
    img, p = copper_base(k, COPPER_STAGES[k] + ("copper_door_upper" if upper else "copper_door_lower"))
    rgb = img[..., :3]
    rgb[:, 0] = p[0]
    rgb[:, 15] = p[0]
    if upper:
        rgb[0, :] = p[0]
        # Two small windows.
        for x0 in (3, 9):
            img[3:8, x0:x0 + 4, 3] = 0
            rgb[2, x0 - 1:x0 + 5] = p[-1]
            rgb[8, x0 - 1:x0 + 5] = p[0]
        rgb[11, 2:14] = p[0]
    else:
        rgb[15, :] = p[0]
        rgb[3:14, 3] = p[-1]
        rgb[3, 3:13] = p[-1]
        rgb[3:14, 12] = p[0]
        rgb[13, 3:13] = p[0]
        for (x, y) in ((5, 5), (10, 5), (5, 11), (10, 11)):
            put(img, x, y, p[-1])
        put(img, 13, 7, p[0])
        put(img, 13, 8, p[0])
    return img


def copper_trapdoor(k):
    img, p = copper_base(k, COPPER_STAGES[k] + "copper_trapdoor")
    rgb = img[..., :3]
    bevel(img, 1.2, 0.6)
    for (x0, y0) in ((3, 3), (9, 3), (3, 9), (9, 9)):
        img[y0:y0 + 4, x0:x0 + 4, 3] = 0
    rgb[7:9, 1:15] = p[-1]
    rgb[1:15, 7:9] = p[-1]
    return img


def copper_bulb(k, lit=False):
    img, p = copper_base(k, COPPER_STAGES[k] + ("copper_bulb_lit" if lit else "copper_bulb"))
    rgb = img[..., :3]
    bevel(img, 1.25, 0.6)
    d = np.hypot(XX - 7.5, YY - 7.5)
    if lit:
        glow = pal("#c06a20", "#f0a040", "#ffd27a", "#fff3c8")
        f = np.clip(1.0 - d / 5.5, 0, 1)
        core = d < 5.2
        rgb[core] = glow[np.clip((f[core] * 3.99).astype(int), 0, 3)]
    else:
        dark = pal("#2a2420", "#3e3630", "#524840")
        core = d < 5.2
        rgb[core] = dark[np.clip(((1 - d[core] / 5.2) * 2.99).astype(int), 0, 2)]
    # The cage over the glass.
    rgb[core & ((XX == 7) | (XX == 8) | (YY == 7) | (YY == 8))] = p[1]
    ring = (d >= 5.2) & (d < 6.3)
    rgb[ring] = p[0]
    return img


def crafter_side():
    img = from_rgb(METAL_DARK[2])
    rgb = img[..., :3]
    wood = planks("oak")[..., :3]
    rgb[3:13, 1:15] = wood[3:13, 1:15]
    rgb[3, 1:15] = METAL_DARK[1]
    rgb[12, 1:15] = METAL_DARK[1]
    for x in (1, 14):
        rgb[3:13, x] = METAL_DARK[1]
    put(img, 7, 7, RS_DARK[2])
    put(img, 8, 7, RS_DARK[2])
    bevel(img, 1.3, 0.7)
    return img


def crafter_front():
    img = from_rgb(METAL_DARK[2])
    rgb = img[..., :3]
    # A 3×3 window of crafting cells.
    for i in range(3):
        for j in range(3):
            x0, y0 = 2 + i * 4, 2 + j * 4
            rgb[y0:y0 + 4, x0:x0 + 4] = METAL_DARK[0]
            rgb[y0 + 1:y0 + 4, x0 + 1:x0 + 4] = METAL_DARK[1]
    bevel(img, 1.3, 0.7)
    return img


def crafter_back():
    img = crafter_side()
    rgb = img[..., :3]
    rgb[6:10, 6:10] = RS_DARK[1]
    return img


IRON_RAIL = pal("#5a5a5e", "#8c8c92", "#b4b4ba")
GOLD_RAIL = pal("#8a6410", "#d8a830", "#f4d660")
TIE = pal("#4a3018", "#6e4a26", "#8a6034")


def rail_texture(kind="rail", on=False, corner=False):
    """Two metal rails over wooden ties, transparent between. Straight runs north-south."""
    img = canvas()
    metal = GOLD_RAIL if kind == "powered" else IRON_RAIL
    tie = TIE
    if corner:
        # Joins north (top) and east (right): arcs around the top-right corner.
        r = np.hypot(XX - 15.5, YY + 0.5)
        ties = np.zeros((N, N), bool)
        ang = np.arctan2(YY + 0.5, 15.5 - XX)
        for a in np.linspace(0.12, np.pi / 2 - 0.12, 4):
            ties |= (np.abs(ang - a) < 0.14) & (r > 2.5) & (r < 14.5)
        img[ties, :3] = tie[1]
        img[ties, 3] = 255
        for lo, hi in ((3.0, 5.0), (11.5, 13.5)):
            m = (r >= lo) & (r < hi)
            img[m, :3] = metal[1]
            img[m & (r < lo + 0.8), :3] = metal[2]
            img[m, 3] = 255
        return img
    for y0 in (1, 5, 9, 13):
        img[y0:y0 + 2, 1:15, :3] = tie[1]
        img[y0, 1:15, :3] = tie[2]
        img[y0:y0 + 2, 1:15, 3] = 255
    if kind in ("powered", "activator"):
        # A strip of redstone down the middle.
        lit = RS_LIT if on else RS_DARK
        img[:, 7:9, :3] = lit[1]
        img[::2, 7, :3] = lit[2]
        img[:, 7:9, 3] = 255
    if kind == "detector":
        img[5:11, 5:11, :3] = pal("#6a6a6a")[0]
        img[6:10, 6:10, :3] = pal("#8a8a8a")[0]
        img[7:9, 7:9, :3] = (RS_LIT if on else RS_DARK)[2]
        img[5:11, 5:11, 3] = 255
    for x0 in (2, 12):
        img[:, x0:x0 + 2, :3] = metal[1]
        img[:, x0, :3] = metal[2]
        img[:, x0:x0 + 2, 3] = 255
    return img


def phase4_registry():
    reg = {
        "crafter_side": crafter_side,
        "crafter_front": crafter_front,
        "crafter_back": crafter_back,
        "rail": rail_texture,
        "rail_corner": lambda: rail_texture(corner=True),
    }
    for kind in ("powered", "detector", "activator"):
        reg[f"{kind}_rail"] = lambda k=kind: rail_texture(k)
        reg[f"{kind}_rail_on"] = lambda k=kind: rail_texture(k, True)
    for k, sp in enumerate(COPPER_STAGES):
        if k:
            reg[f"{sp}copper"] = lambda k=k: copper_stage_block(k)
        reg[f"{sp}cut_copper"] = lambda k=k: cut_copper(k)
        reg[f"{sp}chiseled_copper"] = lambda k=k: chiseled_copper(k)
        reg[f"{sp}copper_grate"] = lambda k=k: copper_grate(k)
        reg[f"{sp}copper_door_lower"] = lambda k=k: copper_door(k, False)
        reg[f"{sp}copper_door_upper"] = lambda k=k: copper_door(k, True)
        reg[f"{sp}copper_trapdoor"] = lambda k=k: copper_trapdoor(k)
        reg[f"{sp}copper_bulb"] = lambda k=k: copper_bulb(k)
        reg[f"{sp}copper_bulb_lit"] = lambda k=k: copper_bulb(k, True)
    return reg


def lantern_side(metal):
    """Lantern sides: a metal frame round a warm glow, drawn where the boxes take their faces."""
    img = canvas()
    m = metal
    glow = pal("#d9822b", "#f0a548", "#ffc96b", "#ffe39a")
    for y in range(5, 16):
        for x in range(5, 11):
            d = abs(x - 7.5) + abs(y - 11.5) * 0.6
            put(img, x, y, glow[max(0, 3 - int(d))])
    for y in range(5, 16):
        put(img, 5, y, m[1])
        put(img, 10, y, m[0])
    for x in range(5, 11):
        put(img, x, 5, m[2])
        put(img, x, 6, m[1])
        put(img, x, 15, m[0])
    # the chain above
    for y in range(0, 5):
        put(img, 7, y, m[1 + (y % 2)])
        put(img, 8, y, m[(y + 1) % 2])
    return img


def lantern_top(metal):
    img = canvas()
    for y in range(4, 12):
        for x in range(4, 12):
            put(img, x, y, metal[1])
    for y in range(6, 10):
        for x in range(6, 10):
            put(img, x, y, metal[2] if (x + y) % 2 else metal[0])
    put(img, 7, 7, metal[0])
    put(img, 8, 8, metal[0])
    return img


def paper_lamp(paper, frame):
    """A paper lamp block: a lit paper panel in a thin wooden frame with a cross brace."""
    rng = rng_for("paper_lamp" + paper[0].tobytes().hex())
    f = normalize(value_noise(rng, 8))
    img = from_rgb(shade(f * 0.6 + 0.3, paper, 0.4))
    for i in range(N):
        for (x, y) in ((i, 0), (i, 15), (0, i), (15, i), (i, 7), (7, i)):
            put(img, x, y, frame[1] if (x + y) % 3 else frame[0])
    return img


def wood_window(wood):
    """Glass in a wooden frame with a cross of glazing bars (Macaw's Windows idea)."""
    img = glass()
    p = WOODS[wood]["plank"]
    for i in range(N):
        for (x, y) in ((i, 0), (i, 1), (i, 14), (i, 15), (0, i), (1, i), (14, i), (15, i), (i, 7), (i, 8), (7, i), (8, i)):
            put(img, x, y, p[2] if x in (0, 7) or y in (0, 7) else p[1])
    return img


def decor_registry():
    iron = pal("#2f2f33", "#4a4a50", "#6c6c74")
    copper = pal("#6b3a24", "#a2593a", "#c87a52")
    reg = {
        "lantern": lambda: lantern_side(iron),
        "lantern_top": lambda: lantern_top(iron),
        "copper_lantern": lambda: lantern_side(copper),
        "copper_lantern_top": lambda: lantern_top(copper),
        "paper_lamp": lambda: paper_lamp(pal("#e8d8b0", "#f2e6c4", "#fbf3dc"), pal("#4a3520", "#6a5033")),
        "red_paper_lamp": lambda: paper_lamp(pal("#b8402e", "#cf5a3c", "#e2784e"), pal("#2e1e0e", "#463018")),
    }
    for wood in WOODS:
        reg[f"{wood}_window"] = lambda w=wood: wood_window(w)
    return reg


def chain_side():
    """Chain links down columns 7 and 8, alternating faces so they read as links."""
    img = canvas()
    m = pal("#2a2e36", "#454b56", "#6a717e")
    for y in range(N):
        k = y % 4
        if k in (0, 3):
            put(img, 7, y, m[2] if k == 0 else m[1])
            put(img, 8, y, m[1] if k == 0 else m[0])
        else:
            put(img, 7, y, m[1])
            put(img, 8, y, m[0])
    return img


def iron_bars():
    img = canvas()
    m = pal("#3a3e46", "#5a606a", "#7c838e", "#9aa1ac")
    for x in (1, 5, 10, 14):
        for y in range(N):
            put(img, x, y, m[2])
            put(img, x + 1, y, m[1])
    for y in (0, 15):
        for x in range(N):
            put(img, x, y, m[3] if y == 0 else m[0])
    return img


def rope_bridge(top):
    """Plank deck with rope: the top shows boards across, ropes along both edges; the side
    shows the deck edge low down and the rope rails and posts above it."""
    p = WOODS["oak"]["plank"]
    rope = pal("#6e5636", "#8c7048", "#a8895a")
    img = canvas()
    if top:
        for y in range(N):
            for x in range(N):
                put(img, x, y, p[2] if y % 4 else p[0])
        for y in range(N):
            for x in (0, 15):
                put(img, x, y, rope[1 + (y % 2)])
    else:
        for x in range(N):
            put(img, x, 14, p[3])
            put(img, x, 15, p[1])
            put(img, x, 7, rope[1 + (x % 2)])
            put(img, x, 11, rope[(x % 2)])
        for y in range(6, 14):
            put(img, 0, y, p[1])
            put(img, 15, y, p[1])
    return img


def extras_registry():
    return {
        "chain": chain_side,
        "iron_bars": iron_bars,
        "rope_bridge_top": lambda: rope_bridge(True),
        "rope_bridge_side": lambda: rope_bridge(False),
    }


# --------------------------------------------------------------------------
# The Nether
# --------------------------------------------------------------------------

NETHERRACK = pal("#3e1414", "#561c1c", "#6c2422", "#7e2e2a", "#904036", "#a45244")


def netherrack(name="netherrack"):
    rng = rng_for(name)
    f = fbm(rng, ((8, 0.3), (4, 0.35), (2, 0.35), (1, 0.35)))
    img = from_rgb(shade(f, NETHERRACK, 0.8))
    for _ in range(14):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, NETHERRACK[0] if rng.random() < 0.6 else NETHERRACK[5])
    return img


def soul_sand():
    rng = rng_for("soul_sand")
    p = pal("#2e221a", "#3e2e22", "#4e3a2c", "#5e4636", "#6e5442")
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.45))), p, 0.8))
    # faint faces in the sand
    for cx, cy in ((4, 5), (11, 10)):
        for (dx, dy) in ((0, 0), (2, 0), (0, 3), (1, 3), (2, 3)):
            put(img, cx + dx, cy + dy, p[0])
    return img


def soul_soil():
    rng = rng_for("soul_soil")
    p = pal("#2a1e16", "#382a20", "#46362a", "#544234")
    return from_rgb(shade(fbm(rng, ((8, 0.3), (4, 0.4), (2, 0.4))), p, 0.9))


def magma_block():
    rng = rng_for("magma_block")
    img = from_rgb(shade(fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3))), pal("#3a1208", "#521a0c", "#6a2410"), 0.8))
    ids, d1, d2, n = voronoi(rng, 7, 5.0)
    edge = (d2 - d1) < 1.2
    glow = pal("#c43c0c", "#ee7018", "#ffb040")
    for y in range(N):
        for x in range(N):
            if edge[y, x]:
                put(img, x, y, glow[int(rng.integers(0, 3))])
    return img


def nether_bricks():
    img = bricks()
    rgb = img[..., :3].astype(np.float32)
    l = rgb.mean(axis=2, keepdims=True)
    img[..., :3] = np.clip(np.concatenate([l * 0.55 + 8, l * 0.2, l * 0.24], axis=2), 0, 255)
    return img


def crimson_nylium_top():
    rng = rng_for("crimson_nylium")
    p = pal("#5a0c10", "#7a1418", "#961c1e", "#ae2a26", "#c43a30")
    return from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.5))), p, 0.9))


def crimson_nylium_side():
    img = netherrack("crimson_nylium_side")
    top = crimson_nylium_top()
    rng = rng_for("crimson_nylium_side_fringe")
    for x in range(N):
        d = 3 + int(rng.integers(0, 3))
        for y in range(d):
            img[y, x] = top[y, x]
    return img


CRIMSON = pal("#3a1424", "#4e1c30", "#62243c", "#782e4a", "#8c3a56")


def crimson_stem():
    rng = rng_for("crimson_stem")
    img = canvas()
    img[..., 3] = 255
    for x in range(N):
        stripe = rng.random()
        for y in range(N):
            k = int(np.clip(2 + (stripe - 0.5) * 3 + rng.normal(0, 0.6), 0, 4))
            img[y, x, :3] = CRIMSON[k]
    glow = pal("#c43c3c", "#e86a50")
    for _ in range(6):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, glow[int(rng.integers(0, 2))])
    return img


def crimson_stem_top():
    img = crimson_planks()
    for y in range(N):
        for x in range(N):
            if x in (0, 15) or y in (0, 15):
                put(img, x, y, CRIMSON[1])
    return img


def crimson_planks():
    rng = rng_for("crimson_planks")
    p = pal("#4a1c2e", "#5e2438", "#702c44", "#82364e", "#923e58")
    grain = 0.6 * value_noise(rng, 8, 1) + 0.4 * value_noise(rng, 4, 1)
    img = from_rgb(shade(normalize(grain) * 0.7 + 0.2, p[1:], 0.5))
    for b in range(4):
        img[b * 4 + 3, :, :3] = p[0]
    return img


def nether_wart_block():
    rng = rng_for("nether_wart_block")
    p = pal("#5a0606", "#740a0a", "#8e1010", "#a81818", "#c02424")
    return from_rgb(shade(fbm(rng, ((8, 0.3), (4, 0.4), (2, 0.4), (1, 0.3))), p, 0.95))


def shroomlight():
    rng = rng_for("shroomlight")
    p = pal("#c46a1c", "#de8a2a", "#f0aa44", "#fcc868", "#ffe2a0")
    img = from_rgb(shade(fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3))), p, 0.8))
    bevel(img)
    return img


BASALT = pal("#2a2a2e", "#36363c", "#44444a", "#52525a", "#62626a")


def basalt_side():
    rng = rng_for("basalt_side")
    img = canvas()
    img[..., 3] = 255
    for x in range(N):
        base = 1 + (x * 7 % 5) % 3
        for y in range(N):
            img[y, x, :3] = BASALT[int(np.clip(base + rng.normal(0, 0.5), 0, 4))]
    for x in (4, 9, 13):
        img[:, x, :3] = BASALT[0]
    return img


def basalt_top():
    rng = rng_for("basalt_top")
    img = from_rgb(shade(fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3))), BASALT, 0.8))
    for i in range(N):
        for (x, y) in ((i, 0), (i, 15), (0, i), (15, i)):
            put(img, x, y, BASALT[0])
    return img


def blackstone():
    rng = rng_for("blackstone")
    p = pal("#141216", "#1e1a20", "#28242a", "#343036", "#403c44")
    img = from_rgb(shade(fbm(rng, ((8, 0.3), (4, 0.35), (2, 0.35), (1, 0.3))), p, 0.8))
    for _ in range(8):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, pal("#5a4a3a")[0])
    return img


def nether_portal():
    """A deep violet swirl, bright at the rims of each whorl."""
    rng = rng_for("nether_portal")
    p = pal("#2a0a5a", "#3e1280", "#561ca6", "#7030c8", "#9a5ae6", "#c89aff")
    img = canvas()
    img[..., 3] = 255
    for y in range(N):
        for x in range(N):
            dx, dy = x - 7.5, y - 7.5
            r = (dx * dx + dy * dy) ** 0.5
            a = np.arctan2(dy, dx)
            v = 0.5 + 0.5 * np.sin(r * 1.3 - a * 2.0) + rng.normal(0, 0.08)
            img[y, x, :3] = p[int(np.clip(v * 5.5, 0, 5))]
    return img


def quartz_block():
    rng = rng_for("quartz_block")
    p = pal("#cfc6bc", "#ddd6cc", "#e8e2da", "#f2eee8", "#faf8f4")
    img = from_rgb(shade(fbm(rng, ((4, 0.2), (2, 0.3), (1, 0.4))), p, 0.4))
    bevel(img, 1.04, 0.9)
    return img


def warped(img):
    """Turn a crimson texture teal: the red becomes green-blue, the dark stays dark."""
    out = img.copy()
    rgb = img[..., :3].astype(np.float32)
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    out[..., 0] = np.clip(g * 0.6 + b * 0.2, 0, 255)
    out[..., 1] = np.clip(r * 0.8 + 6, 0, 255)
    out[..., 2] = np.clip(r * 0.7 + b * 0.25, 0, 255)
    return out


def warped_nylium_side():
    img = netherrack("warped_nylium_side")
    top = warped(crimson_nylium_top())
    rng = rng_for("crimson_nylium_side_fringe")
    for x in range(N):
        d = 3 + int(rng.integers(0, 3))
        for y in range(d):
            img[y, x] = top[y, x]
    return img


def nether_registry():
    quartz = pal("#bdb2a6", "#e2dbd2", "#fffaf2")
    gold = pal("#b07a10", "#e0b830", "#fff080")
    return {
        "netherrack": netherrack,
        "nether_quartz_ore": lambda: ore("nether_quartz", netherrack("nqo_bg"), quartz),
        "nether_gold_ore": lambda: ore("nether_gold", netherrack("ngo_bg"), gold),
        "soul_sand": soul_sand,
        "soul_soil": soul_soil,
        "magma_block": magma_block,
        "nether_bricks": nether_bricks,
        "crimson_nylium": crimson_nylium_top,
        "crimson_nylium_side": crimson_nylium_side,
        "crimson_stem": crimson_stem,
        "crimson_stem_top": crimson_stem_top,
        "crimson_planks": crimson_planks,
        "nether_wart_block": nether_wart_block,
        "warped_nylium": lambda: warped(crimson_nylium_top()),
        "warped_nylium_side": warped_nylium_side,
        "warped_stem": lambda: warped(crimson_stem()),
        "warped_stem_top": lambda: warped(crimson_stem_top()),
        "warped_planks": lambda: warped(crimson_planks()),
        "warped_wart_block": lambda: warped(nether_wart_block()),
        "shroomlight": shroomlight,
        "basalt_side": basalt_side,
        "basalt_top": basalt_top,
        "blackstone": blackstone,
        "nether_portal": nether_portal,
        "quartz_block": quartz_block,
    }

END_STONE = pal("#bdb889", "#cdc898", "#dbd6a6", "#e6e2b4", "#f0edc4")


def end_stone(name="end_stone"):
    rng = rng_for(name)
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.35))), END_STONE, 0.7))
    # little pits
    for _ in range(9):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, END_STONE[0])
    return img


def end_stone_bricks():
    img = bricks()
    rgb = img[..., :3].astype(np.float32)
    l = rgb.mean(axis=2, keepdims=True) / 255.0
    base = np.array([226, 222, 170], np.float32)
    img[..., :3] = np.clip(l * 1.25 * base, 0, 255)
    return img


PURPUR = pal("#7c4e7c", "#8e5e8e", "#a06ea0", "#b07eb0", "#c092c0")


def purpur_block():
    rng = rng_for("purpur_block")
    img = from_rgb(shade(fbm(rng, ((4, 0.2), (2, 0.3), (1, 0.4))), PURPUR, 0.5))
    for i in range(N):
        for (x, y) in ((i, 0), (0, i), (i, 8), (8, i)):
            put(img, x, y, PURPUR[0])
    return img


def end_portal_frame_side():
    img = end_stone("frame_side")
    p = pal("#1e4a40", "#2a5e52", "#367264")
    for x in range(N):
        for y in range(3):
            put(img, x, y, p[1 if (x + y) % 3 else 0])
    return img


def end_portal_frame_top(eye=False):
    rng = rng_for("frame_top")
    p = pal("#1e4a40", "#2a5e52", "#367264", "#448270")
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.3))), p, 0.7))
    for i in range(N):
        for (x, y) in ((i, 0), (i, 15), (0, i), (15, i)):
            put(img, x, y, END_STONE[1])
    for y in range(5, 11):
        for x in range(5, 11):
            put(img, x, y, (12, 20, 18))
    if eye:
        g = pal("#0c3a20", "#1e6a3c", "#38a85e", "#7ee09a")
        for y in range(5, 11):
            for x in range(5, 11):
                d = abs(x - 7.5) + abs(y - 7.5)
                put(img, x, y, g[3 - min(3, int(d))])
        put(img, 7, 7, (8, 12, 10))
        put(img, 8, 8, (8, 12, 10))
    return img


def end_portal():
    """Deep space: near-black teal with scattered stars."""
    rng = rng_for("end_portal")
    img = canvas()
    img[..., 3] = 255
    img[..., :3] = (6, 12, 16)
    for _ in range(22):
        x, y = rng.integers(0, N, 2)
        c = [(40, 110, 100), (90, 170, 160), (150, 220, 210), (60, 80, 140)][int(rng.integers(0, 4))]
        put(img, x, y, c)
    return img


def end_crystal():
    """A pink-violet gem in a dark cage, with see-through gaps."""
    img = canvas()
    p = pal("#6a2a7a", "#9a4aaa", "#c878d4", "#f0b6f6", "#fff0ff")
    cage = (40, 30, 46)
    for y in range(N):
        for x in range(N):
            edge = x in (0, 15) or y in (0, 15)
            d = abs(x - 7.5) + abs(y - 7.5)
            if edge and (x + y) % 4 < 2:
                img[y, x] = (*cage, 255)
            elif d < 6.5:
                img[y, x] = (*p[min(4, int(6.5 - d))], 255)
    return img


def chorus_plant():
    rng = rng_for("chorus_plant")
    p = pal("#4a2a4a", "#5e3a5e", "#74507a", "#8a6890", "#a080a8")
    img = canvas()
    for y in range(N):
        for x in range(1, 15):
            v = int(np.clip(2 + rng.normal(0, 0.8) + (1 if 5 <= x <= 10 else -1), 0, 4))
            img[y, x] = (*p[v], 255)
    return img


def chorus_flower():
    rng = rng_for("chorus_flower")
    p = pal("#6a4a76", "#8a6a96", "#a88ab4", "#c6aad0", "#e2cce8")
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.3))), p, 0.8))
    bevel(img)
    return img


def end_registry():
    return {
        "end_stone": end_stone,
        "end_stone_bricks": end_stone_bricks,
        "purpur_block": purpur_block,
        "end_portal_frame_side": end_portal_frame_side,
        "end_portal_frame_top": end_portal_frame_top,
        "end_portal_frame_eye": lambda: end_portal_frame_top(True),
        "end_portal": end_portal,
        "end_crystal": end_crystal,
        "chorus_plant": chorus_plant,
        "chorus_flower": chorus_flower,
    }


# ---- farming ---------------------------------------------------------------


def farmland():
    """Tilled earth: dark furrows across, a little damp."""
    rng = rng_for("farmland")
    p = pal("#3a2414", "#4a2e1a", "#5a3a22", "#6a462a", "#7a5232")
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.4))), p, 0.8))
    for y in range(N):
        if y % 4 == 0:
            for x in range(N):
                put(img, x, y, p[0])
    for i in range(N):
        for (x, y) in ((i, 0), (0, i), (15, i), (i, 15)):
            put(img, x, y, p[1])
    return img


HAY = pal("#7a6010", "#9a7a18", "#b89422", "#d0ac34", "#e2c44e")


def hay_block_side():
    rng = rng_for("hay_side")
    img = canvas()
    img[..., 3] = 255
    for x in range(N):
        for y in range(N):
            v = 2 + rng.normal(0, 0.9) + (0.6 if (x * 3 + y // 5) % 4 == 0 else 0)
            img[y, x, :3] = HAY[int(np.clip(v, 0, 4))]
    for x in range(N):  # two red bands of twine
        for y in (3, 12):
            put(img, x, y, (120, 40, 24) if x % 2 else (150, 56, 34))
    return img


def hay_block_top():
    rng = rng_for("hay_top")
    img = from_rgb(shade(fbm(rng, ((2, 0.4), (1, 0.6))), HAY, 0.9))
    for i in range(N):
        for (x, y) in ((i, 0), (0, i), (15, i), (i, 15)):
            put(img, x, y, HAY[0])
    return img


def crop(kind, stage, stages):
    """Cutout crop rows: stalks grow taller with each stage; the last shows the harvest."""
    rng = rng_for(f"{kind}{stage}")
    img = canvas()
    t = (stage + 1) / stages
    leaf = pal("#2e5a14", "#3e7418", "#52901e", "#6aa826")
    ripe = stage == stages - 1
    h = max(2, int(round(t * (14 if kind == "wheat" else 9))))
    cols = (1, 4, 7, 10, 13) if kind == "wheat" else (2, 6, 10, 13)
    for c in cols:
        x0 = c + int(rng.integers(-1, 2))
        hh = max(1, h - int(rng.integers(0, 3)))
        for k in range(hh):
            y = 15 - k
            x = int(np.clip(x0 + (1 if k > hh * 0.6 and c % 2 else 0), 0, 15))
            if kind == "wheat":
                if ripe or stage >= 6:
                    col = HAY[int(np.clip(1 + k / hh * 3 + rng.normal(0, 0.4), 0, 4))]
                else:
                    col = leaf[int(np.clip(k / hh * 3 + rng.normal(0, 0.4), 0, 3))]
                put(img, x, y, col)
                if k >= hh - 3 and (ripe or stage >= 5):  # ears
                    put(img, x + 1, y, HAY[3] if ripe else leaf[3])
            else:
                put(img, x, y, leaf[int(np.clip(k / hh * 3 + rng.normal(0, 0.4), 0, 3))])
                if k > 1 and rng.random() < 0.5:
                    put(img, int(np.clip(x + rng.choice([-1, 1]), 0, 15)), y, leaf[2])
        if ripe and kind != "wheat":
            root = {"carrots": (230, 120, 30), "potatoes": (200, 160, 90), "beetroots": (150, 30, 50)}[kind]
            put(img, x0, 15, root)
            put(img, int(np.clip(x0 + 1, 0, 15)), 15, tuple(int(v * 0.8) for v in root))
            put(img, x0, 14, root)
    return img


def nether_wart(stage):
    """Knobbly red nether wart, taller and redder each stage."""
    rng = rng_for(f"nether_wart{stage}")
    img = canvas()
    red = pal("#3a0a0e", "#5a1218", "#7a1c22", "#9a2a2e", "#b83a3a")
    h = 3 + stage * 3
    for c in (2, 6, 10, 13):
        x0 = c + int(rng.integers(-1, 2))
        hh = max(2, h - int(rng.integers(0, 3)))
        for k in range(hh):
            y = 15 - k
            x = int(np.clip(x0 + (1 if (k // 2) % 2 else 0), 0, 15))
            put(img, x, y, red[int(np.clip(k / hh * 3 + rng.normal(0, 0.5), 0, 4))])
            if k > hh - 3 or rng.random() < 0.3:
                put(img, int(np.clip(x + 1, 0, 15)), y, red[int(np.clip(2 + stage * 0.6, 0, 4))])
    return img


def drawer_front(wood):
    """Planks behind a darker frame, with two drawers and their handles."""
    img = planks(wood)
    rgb = img[..., :3]
    dark = WOODS[wood]["plank"][0]
    rgb[0, :] = dark
    rgb[15, :] = dark
    rgb[:, 0] = dark
    rgb[:, 15] = dark
    rgb[7:9, :] = dark
    for y0 in (1, 9):
        rgb[y0:y0 + 6, 1] = scale_rgb(rgb[y0:y0 + 6, 1], 0.85)
        rgb[y0 + 5, 1:15] = scale_rgb(rgb[y0 + 5, 1:15], 0.8)
        hy = y0 + 2
        rgb[hy:hy + 2, 6:10] = pal("#3a3a3e", "#5c5c62", "#7a7a80", "#909096")[1]
        rgb[hy, 6:10] = pal("#3a3a3e", "#5c5c62", "#7a7a80", "#909096")[3]
    return img


def stem(kind, stage):
    """A pumpkin or melon stem: a thin stalk that bends and yellows as it ripens."""
    img = canvas()
    young = pal("#2e5a14", "#3e7418", "#52901e", "#6aa826")
    old = pal("#6a5a14", "#8a7a1e", "#a8962a", "#c4b03a")
    p = old if stage == 7 else young
    h = 3 + stage * 12 // 7
    for k in range(h):
        y = 15 - k
        x = 7 + (1 if k > h * 0.6 else 0)
        put(img, x, y, p[2 if k % 3 else 1])
        if k % 4 == 2:
            put(img, x - 1, y, p[3])
            put(img, x + 1, y - 1, p[1])
    return img


def sugar_cane_block():
    img = canvas()
    p = pal("#4a6a20", "#6a9a30", "#8aba48", "#a8d468")
    for x0 in (3, 7, 12):
        for y in range(16):
            put(img, x0, y, p[1 if y % 5 == 0 else 2])
            put(img, x0 + 1, y, p[0 if y % 5 == 0 else 1])
        put(img, x0 + 2, 2 + x0 % 5, p[3])
        put(img, x0 - 1, 9 - x0 % 4, p[3])
    return img


def composter_side():
    img = planks("oak")
    rgb = img[..., :3]
    dark = WOODS["oak"]["plank"][0]
    rgb[:, 0] = dark
    rgb[:, 15] = dark
    rgb[0, :] = dark
    rgb[15, :] = dark
    for x in (5, 10):
        rgb[1:15, x] = scale_rgb(rgb[1:15, x], 0.75)
    return img


def composter_top(ready=False):
    img = composter_side()
    rgb = img[..., :3]
    rng = rng_for("compost" + str(ready))
    fill = pal("#d8d4c4", "#ecead8", "#f8f6ec") if ready else pal("#2a1e10", "#3a2a16", "#4e3a1e")
    for y in range(2, 14):
        for x in range(2, 14):
            rgb[y, x] = fill[int(rng.integers(0, 3))]
    return img


MYCELIUM = pal("#5a4a5a", "#6a5868", "#7a6676", "#8a7484", "#9a8494")


def mycelium_top():
    rng = rng_for("mycelium_top")
    f = 0.6 * fbm(rng, ((4, 0.4), (2, 0.6))) + 0.4 * rng.random((N, N))
    img = from_rgb(shade(f, MYCELIUM, 0.6))
    for _ in range(10):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, hexrgb("#b8a8b4"))
    return img


def mushroom_block(name, p, spots=None):
    rng = rng_for(name)
    f = fbm(rng, ((4, 0.4), (2, 0.6)))
    img = from_rgb(shade(f, p, 0.6))
    if spots:
        for _ in range(5):
            x, y = rng.integers(1, N - 2, 2)
            for dx, dy in ((0, 0), (1, 0), (0, 1), (1, 1)):
                put(img, x + dx, y + dy, spots[int(rng.integers(0, len(spots)))])
    return img


def mushroom_stem():
    rng = rng_for("mushroom_stem")
    p = pal("#bab2a0", "#c8c0ae", "#d6cebc", "#e2dccc")
    f = 0.5 * fbm(rng, ((8, 0.3), (2, 0.7))) + 0.5 * rng.random((N, N)) * 0.4
    img = from_rgb(shade(f, p, 0.5))
    for x in (3, 9, 13):
        img[:, x, :3] = scale_rgb(img[:, x, :3], 0.9)
    return img


FLOWERS = {
    "cornflower": (pal("#22408a", "#3a5cb8", "#5a7cd8"), "#1a2a5a", 7, False),
    "allium": (pal("#7a3aa0", "#9a52c4", "#b874dc"), "#5a2a7a", 5, True),
    "azure_bluet": (pal("#b8bcc8", "#d8dce4", "#f0f2f6"), "#e8c840", 9, False),
    "oxeye_daisy": (pal("#c8c8c0", "#e0e0d8", "#f6f6f0"), "#e8b820", 7, True),
    "red_tulip": (pal("#8a1210", "#b82018", "#d83424"), "#5a0e0a", 6, False),
    "orange_tulip": (pal("#b04a10", "#d8661a", "#f08428"), "#7a300a", 6, False),
    "white_tulip": (pal("#b8b8b0", "#d4d4cc", "#ececE4"), "#8a8a80", 6, False),
    "pink_tulip": (pal("#c06a84", "#dc8aa2", "#f0a8bc"), "#8a4a5c", 6, False),
    "blue_orchid": (pal("#1a6aa8", "#2a8ad0", "#48aaec"), "#0e3a6a", 6, True),
    "lily_of_the_valley": (pal("#c8ccc4", "#e0e4dc", "#f8faf4"), "#a8aca4", 8, False),
}


def farming_registry():
    reg = {"farmland": farmland, "hay_block_side": hay_block_side, "hay_block_top": hay_block_top,
           "botany_pot_side": botany_pot_side, "botany_pot_top": botany_pot_top, "botany_pot_empty": botany_pot_empty,
           "cobblestone_bricks": cobblestone_bricks,
           "mossy_cobblestone_bricks": lambda: cobblestone_bricks("mossy_cobblestone_bricks", True),
           "moss_block": moss_block, "cave_vines_lit": cave_vines_lit, "packed_ice": packed_ice,
           "dripstone_block": dripstone_block, "pointed_dripstone_up_tip": lambda: pointed_dripstone(True),
           "pointed_dripstone_down_tip": lambda: pointed_dripstone(False),
           "rocky_dirt": rocky_dirt, "thatch": thatch, "lattice": lattice, "chandelier": chandelier_side,
           "brazier": brazier_side, "brazier_top": brazier_top, "twig": twig, "pebble": pebble,
           "red_curtain": lambda: curtain(pal("#5a1010", "#7a1818", "#962222", "#ae2c2c")),
           "white_curtain": lambda: curtain(pal("#a8a49a", "#c4c0b6", "#d8d4ca", "#e8e4da")),
           "green_curtain": lambda: curtain(pal("#1e3a14", "#2a4e1c", "#386226", "#467630"))}
    for wood in ("oak", "birch", "spruce") + NEW_WOODS:
        reg[f"{wood}_drawer_front"] = lambda w=wood: drawer_front(w)
    reg["sugar_cane"] = sugar_cane_block
    reg["mycelium_top"] = mycelium_top
    reg["mycelium_side"] = lambda: fringe_side("mycelium_side", MYCELIUM)
    reg["mushroom_stem"] = mushroom_stem
    reg["red_mushroom_block"] = lambda: mushroom_block("red_mushroom_block", pal("#8a1410", "#a81c16", "#c0241c", "#d02e24"), [hexrgb("#e8e4dc"), hexrgb("#d4d0c8")])
    reg["brown_mushroom_block"] = lambda: mushroom_block("brown_mushroom_block", pal("#6a5038", "#7a5e44", "#8a6c50", "#9a7a5c"))
    for fname, (petal, center, hy, big) in FLOWERS.items():
        reg[fname] = lambda fname=fname, petal=petal, center=center, hy=hy, big=big: flower(fname, petal, hexrgb(center), hy, big)
    reg["composter_side"] = composter_side
    reg["composter_top"] = composter_top
    reg["composter_ready"] = lambda: composter_top(True)
    for kind in ("pumpkin", "melon"):
        for k in range(8):
            reg[f"{kind}_stem_stage{k}"] = lambda kind=kind, k=k: stem(kind, k)
    for k in range(4):
        reg[f"nether_wart_stage{k}"] = lambda k=k: nether_wart(k)
    for kind, n in (("wheat", 8), ("carrots", 4), ("potatoes", 4), ("beetroots", 4)):
        for k in range(n):
            reg[f"{kind}_stage{k}"] = lambda kind=kind, k=k, n=n: crop(kind, k, n)
    return reg


POT = pal("#6a3420", "#7e4028", "#924c30", "#a65a3a", "#b86a46")


def botany_pot_side():
    """Terracotta, only the lower six rows matter (the pot is six pixels tall)."""
    rng = rng_for("botany_pot_side")
    img = canvas()
    img[..., 3] = 255
    for y in range(N):
        for x in range(N):
            v = 2 + rng.normal(0, 0.6) - (0.8 if y >= 15 else 0)
            img[y, x, :3] = POT[int(np.clip(v, 0, 4))]
    for x in range(N):  # the rim
        put(img, x, 10, POT[4])
        put(img, x, 11, POT[3])
    return img


def botany_pot_top():
    """Soil inside a terracotta rim (the pot covers pixels 3..12)."""
    rng = rng_for("botany_pot_top")
    soil = pal("#2a1a0e", "#362214", "#422a1a", "#4e3420")
    img = from_rgb(shade(fbm(rng, ((2, 0.5), (1, 0.5))), soil, 0.9))
    for i in range(3, 13):
        for (x, y) in ((i, 3), (3, i), (12, i), (i, 12)):
            put(img, x, y, POT[4])
    return img


def botany_pot_empty():
    return canvas()


def cobblestone_bricks(name="cobblestone_bricks", moss=False):
    """Cobblestone cut into rough bricks (Twigs idea)."""
    rng = rng_for(name)
    f = fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.4)))
    img = from_rgb(shade(0.3 + 0.5 * f, COBBLE[2:], 0.7))
    rgb = img[..., :3]
    for y in range(N):
        row = y // 4
        off = 4 if row % 2 else 0
        for x in range(N):
            if y % 4 == 3 or (x + off) % 8 == 7:
                rgb[y, x] = COBBLE[0]
            elif y % 4 == 0:
                rgb[y, x] = scale_rgb(rgb[y, x], 1.12)
    if moss:
        m = fbm(rng, ((8, 0.4), (4, 0.4), (2, 0.3))) > 0.58
        rgb[m] = shade(fbm(rng, ((2, 0.5), (1, 0.5))), MOSS, 0.8)[m]
    return img


def rocky_dirt():
    """Dirt with stones pressed into it (Twigs idea)."""
    img = dirt("rocky_dirt")
    rng = rng_for("rocky_dirt_stones")
    for _ in range(7):
        cx, cy = rng.integers(1, 14), rng.integers(1, 14)
        for dy in range(2):
            for dx in range(2 + rng.integers(0, 2)):
                put(img, (cx + dx) % N, (cy + dy) % N, COBBLE[3 - dy + (dx == 0)])
    return img


def thatch():
    """Straw bundled in rows, darker than hay (Decorative Blocks idea)."""
    rng = rng_for("thatch")
    straw = pal("#5e4a1e", "#74602a", "#8a7434", "#a08a42", "#b49c52")
    img = canvas()
    img[..., 3] = 255
    for y in range(N):
        for x in range(N):
            v = 2 + rng.normal(0, 0.8) + (0.8 if (x + y // 4) % 3 == 0 else 0) - (1.2 if y % 4 == 3 else 0)
            img[y, x, :3] = straw[int(np.clip(v, 0, 4))]
    return img


def lattice():
    """Thin wooden strips crossing on the diagonal, open between (Decorative Blocks idea)."""
    p = WOODS["oak"]["plank"]
    img = canvas()
    for y in range(N):
        for x in range(N):
            if (x + y) % 6 == 0 or (x - y) % 6 == 0:
                put(img, x, y, p[2] if (x + y) % 6 == 0 else p[1])
    for i in range(N):
        for (x, y) in ((i, 0), (i, 15), (0, i), (15, i)):
            put(img, x, y, p[0])
    return img


def chandelier_side():
    """An iron ring hung on a chain with candle flames on it."""
    m = pal("#2e3036", "#4a4e56", "#6a707a")
    img = canvas()
    for y in range(0, 7):
        put(img, 7, y, m[1 + y % 2])
        put(img, 8, y, m[(y + 1) % 2])
    for x in range(3, 13):
        put(img, x, 12, m[1])
        put(img, x, 13, m[0])
    for x in (4, 8, 11):
        put(img, x, 11, (230, 220, 190))
        put(img, x, 10, (230, 220, 190))
        put(img, x, 9, (255, 190, 80))
        put(img, x, 8, (255, 230, 140))
    for y in range(7, 12):
        put(img, 6 - (y - 7) // 2, y, m[1])
        put(img, 9 + (y - 7) // 2, y, m[1])
    return img


def brazier_side():
    """An iron basket of coals with flames licking out of the top."""
    m = pal("#26282c", "#3e4248", "#5a5f68")
    img = canvas()
    for y in range(9, 16):
        for x in range(5, 11):
            put(img, x, y, m[1] if (x + y) % 3 else m[0])
    for x in range(5, 11):
        put(img, x, 9, m[2])
    flame = pal("#c4441a", "#e8742a", "#ffb04a", "#ffe08a")
    for y in range(5, 9):
        for x in range(6, 10):
            k = 3 - (8 - y) // 1 + (1 if x in (7, 8) else 0)
            if (x + y) % 4 or y > 6:
                put(img, x, y, flame[int(np.clip(k, 0, 3))])
    return img


def brazier_top():
    img = canvas()
    coal = pal("#2a1a12", "#c4441a", "#ff9a3a")
    for y in range(4, 12):
        for x in range(4, 12):
            put(img, x, y, coal[(x * 3 + y * 5) % 3])
    return img


def twig():
    """A couple of sticks lying on the ground (Twigs idea)."""
    p = WOODS["oak"]["log"] if "log" in WOODS["oak"] else WOODS["oak"]["plank"]
    img = canvas()
    for x in range(2, 13):
        put(img, x, 14 - (x // 6), p[1] if x % 3 else p[2])
    for x in range(6, 14):
        put(img, x, 15 - (x - 6) // 5, p[0])
    put(img, 9, 12, (70, 110, 40))
    return img


def pebble():
    """A few small stones on the ground (Twigs idea)."""
    img = canvas()
    for (x, y, w) in ((3, 14, 3), (9, 14, 2), (11, 15, 3), (6, 15, 2)):
        for dx in range(w):
            put(img, x + dx, y, COBBLE[4 if dx == 0 else 3])
            put(img, x + dx, y + 1 if y < 15 else y, COBBLE[2])
    return img


def curtain(cloth):
    """Hanging cloth in soft vertical folds on a rod (Another Furniture idea)."""
    img = canvas()
    for y in range(N):
        for x in range(N):
            k = (1, 2, 3, 2, 1, 0)[x % 6]
            put(img, x, y, cloth[k])
    rod = pal("#3a2a1a", "#5a4228")
    for x in range(N):
        put(img, x, 0, rod[1])
        put(img, x, 1, rod[0])
    return img


def moss_block():
    rng = rng_for("moss_block")
    return from_rgb(shade(fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3))), pal("#3a5a1e", "#4a6e26", "#5a8230", "#6a943a", "#7aa444"), 0.9))


def cave_vines_lit():
    """A hanging vine with glowing orange berries."""
    img = canvas()
    leaf = MOSS
    for y in range(N):
        x = 7 + (1 if (y // 3) % 2 else 0)
        put(img, x, y, leaf[1])
        put(img, x + 1, y, leaf[2])
        if y % 4 == 1:
            put(img, x - 1, y, leaf[3])
            put(img, x + 2, y + 1 if y < 15 else y, leaf[2])
    for (x, y) in ((5, 4), (10, 9), (6, 13)):
        for dx, dy in ((0, 0), (1, 0), (0, 1), (1, 1)):
            put(img, x + dx, y + dy, (255, 180, 60) if dx + dy else (255, 230, 140))
    return img


def packed_ice():
    rng = rng_for("packed_ice")
    f = fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3)))
    img = from_rgb(shade(f, pal("#7a9ccc", "#8aaad8", "#9ab8e2", "#acc6ea", "#c0d6f2"), 0.8))
    for k in range(5):
        y0 = int(rng.integers(0, N))
        for x in range(N):
            if rng.random() < 0.7:
                put(img, x, (y0 + x // 3) % N, (214, 230, 250))
    return img


DRIP = pal("#6a5444", "#7c6452", "#8c7460", "#9c846e", "#ac947c")


def dripstone_block():
    rng = rng_for("dripstone_block")
    img = canvas()
    img[..., 3] = 255
    for y in range(N):
        for x in range(N):
            v = 2 + rng.normal(0, 0.5) + (0.9 if (x * 2 + y // 3) % 7 == 0 else 0) - (0.8 if y % 5 == 4 else 0)
            img[y, x, :3] = DRIP[int(np.clip(v, 0, 4))]
    return img


def pointed_dripstone(up):
    """A stone spike narrowing to a point (up from the floor, or down from the roof)."""
    img = canvas()
    for y in range(N):
        t = y if up else 15 - y  # distance from the tip
        half = t / 15 * 3.5
        for x in range(N):
            if abs(x - 7.5) <= half + 0.5:
                put(img, x, y, DRIP[1 + (x + y) % 3 if abs(x - 7.5) < half else 0])
    return img


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
