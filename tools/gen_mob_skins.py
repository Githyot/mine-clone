#!/usr/bin/env python3
"""Procedurally generate original 64x64 mob skins.

The box models (sizes, origins and uv offsets) are read from
packs/vanilla/mobs.ron and checked against the part roles listed below.  Every
face of every box is painted pixel by pixel from hand-picked short palettes,
value noise and a few hand-placed features (eyes, snouts, ribs ...).  Nothing
external is read except mobs.ron itself.

Box unwrap (same as the comment at the top of mobs.ron), for a box w x h x d
whose skin region starts at (u, v):
    top    (u+d,     v)   w x d      bottom (u+d+w,   v)   w x d
    right  (u,       v+d) d x h      front  (u+d,     v+d) w x h
    left   (u+d+w,   v+d) d x h      back   (u+2d+w,  v+d) w x h

Orientation assumed for each face (local box coords x in [0,w], y in [0,h],
z in [0,d]; row 0 of a side face is the top of the box):
    front (+Z): column i -> x = i          back (-Z): column i -> x = w-1-i
    right (-X): column i -> z = i          left (+X): column i -> z = d-1-i
    top:  column i -> x = i, row j -> z = j  (last row touches the front)
    bottom: column i -> x = i, row j -> z = d-1-j (first row touches the front)
This wraps the four side faces into one continuous strip, so patterns that are
computed from 3D noise line up across box edges.

Usage:
    python3 tools/gen_mob_skins.py

Output: packs/vanilla/textures/entity/<mob>.png (64x64 RGBA, unused pixels
fully transparent) and preview.png (every skin at 4x on a checkerboard).
Deterministic: all randomness is hash-based with fixed seeds.
"""

import os
import re
import zlib
from types import SimpleNamespace

from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MOBS_RON = os.path.join(ROOT, "packs", "vanilla", "mobs.ron")
OUT_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "entity")
SKIN = 64
BASE_SEED = 0x0B5C_A1E5

# Role of each part, in the order the parts appear in mobs.ron.  The geometry
# itself comes from mobs.ron; HARDCODED below must match it exactly (checked).
ROLES = {
    "pig": ["body", "head", "snout", "leg", "leg", "leg", "leg"],
    "cow": ["body", "head", "horn", "horn", "leg", "leg", "leg", "leg"],
    "sheep": ["body", "head", "leg", "leg", "leg", "leg"],
    "chicken": ["body", "head", "beak", "wattle", "wing", "wing", "leg", "leg"],
    "zombie": ["head", "body", "arm", "arm", "leg", "leg"],
    "skeleton": ["head", "body", "arm", "arm", "leg", "leg"],
    "stalker": ["head", "body", "leg", "leg", "leg", "leg"],
    "spider": ["head", "neck", "abdomen", "leg", "leg", "leg", "leg",
               "leg", "leg", "leg", "leg"],
}

# (size, origin, uv) per part, mirroring mobs.ron.
HARDCODED = {
    "pig": [((10, 8, 16), (-5, 6, -8), (0, 0)), ((8, 8, 8), (-4, 8, 7), (0, 24)),
            ((4, 3, 1), (-2, 9, 15), (32, 24)), ((4, 6, 4), (-5, 0, -7), (0, 40)),
            ((4, 6, 4), (1, 0, -7), (0, 40)), ((4, 6, 4), (-5, 0, 3), (0, 40)),
            ((4, 6, 4), (1, 0, 3), (0, 40))],
    "cow": [((12, 10, 18), (-6, 10, -9), (0, 0)), ((8, 8, 6), (-4, 14, 8), (0, 28)),
            ((1, 3, 1), (-5, 20, 11), (28, 28)), ((1, 3, 1), (4, 20, 11), (28, 28)),
            ((4, 10, 4), (-6, 0, -8), (0, 42)), ((4, 10, 4), (2, 0, -8), (0, 42)),
            ((4, 10, 4), (-6, 0, 4), (0, 42)), ((4, 10, 4), (2, 0, 4), (0, 42))],
    "sheep": [((10, 10, 16), (-5, 9, -8), (0, 0)), ((6, 6, 8), (-3, 14, 6), (0, 26)),
              ((4, 10, 4), (-5, 0, -7), (0, 40)), ((4, 10, 4), (1, 0, -7), (0, 40)),
              ((4, 10, 4), (-5, 0, 3), (0, 40)), ((4, 10, 4), (1, 0, 3), (0, 40))],
    "chicken": [((6, 6, 8), (-3, 4, -4), (0, 0)), ((4, 6, 3), (-2, 8, 2), (0, 14)),
                ((4, 2, 2), (-2, 10, 5), (14, 14)), ((2, 2, 1), (-1, 8, 5), (26, 14)),
                ((1, 4, 6), (-4, 6, -3), (0, 23)), ((1, 4, 6), (3, 6, -3), (0, 23)),
                ((2, 4, 2), (-2, 0, -1), (28, 0)), ((2, 4, 2), (0, 0, -1), (28, 0))],
    "zombie": [((8, 8, 8), (-4, 24, -4), (0, 0)), ((8, 12, 4), (-4, 12, -2), (16, 16)),
               ((4, 12, 4), (-8, 12, -2), (40, 16)), ((4, 12, 4), (4, 12, -2), (40, 16)),
               ((4, 12, 4), (-4, 0, -2), (0, 16)), ((4, 12, 4), (0, 0, -2), (0, 16))],
    "skeleton": [((8, 8, 8), (-4, 24, -4), (0, 0)), ((8, 12, 4), (-4, 12, -2), (16, 16)),
                 ((2, 12, 2), (-6, 12, -1), (40, 16)), ((2, 12, 2), (4, 12, -1), (40, 16)),
                 ((2, 12, 2), (-3, 0, -1), (0, 16)), ((2, 12, 2), (1, 0, -1), (0, 16))],
    "stalker": [((8, 8, 8), (-4, 18, -4), (0, 0)), ((8, 12, 4), (-4, 6, -2), (16, 16)),
                ((4, 6, 4), (-4, 0, -6), (0, 16)), ((4, 6, 4), (0, 0, -6), (0, 16)),
                ((4, 6, 4), (-4, 0, 2), (0, 16)), ((4, 6, 4), (0, 0, 2), (0, 16))],
    "spider": [((8, 8, 8), (-4, 4, 3), (32, 4)), ((6, 6, 6), (-3, 5, -3), (0, 0)),
               ((10, 8, 12), (-5, 4, -15), (0, 20))]
              + [((16, 2, 2), (-19, 7, z), (0, 40)) for z in (2, 0, -2, -4)]
              + [((16, 2, 2), (3, 7, z), (0, 40)) for z in (2, 0, -2, -4)],
}


# --------------------------------------------------------------------------
# mobs.ron parsing
# --------------------------------------------------------------------------

PART_RE = re.compile(
    r"\(size:\s*\((\d+),\s*(\d+),\s*(\d+)\),\s*origin:\s*\((-?\d+),\s*(-?\d+),\s*(-?\d+)\),"
    r"\s*uv:\s*\((\d+),\s*(\d+)\)")
NAME_RE = re.compile(r'name:\s*"([a-z_]+)"')


def parse_mobs(path):
    text = open(path, encoding="utf-8").read()
    names = list(NAME_RE.finditer(text))
    mobs = {}
    for k, m in enumerate(names):
        end = names[k + 1].start() if k + 1 < len(names) else len(text)
        parts = []
        for p in PART_RE.finditer(text, m.end(), end):
            g = [int(v) for v in p.groups()]
            parts.append((tuple(g[0:3]), tuple(g[3:6]), tuple(g[6:8])))
        mobs[m.group(1)] = parts
    return mobs


# --------------------------------------------------------------------------
# Box unwrap
# --------------------------------------------------------------------------


def face_rects(size, uv):
    w, h, d = size
    u, v = uv
    return {
        "top": (u + d, v, w, d),
        "bottom": (u + d + w, v, w, d),
        "right": (u, v + d, d, h),
        "front": (u + d, v + d, w, h),
        "left": (u + d + w, v + d, d, h),
        "back": (u + 2 * d + w, v + d, w, h),
    }


def local_pos(face, i, j, size):
    """Centre of face pixel (i, j) in local box coordinates."""
    w, h, d = size
    if face == "front":
        return (i + .5, h - j - .5, d)
    if face == "back":
        return (w - i - .5, h - j - .5, 0)
    if face == "right":
        return (0, h - j - .5, i + .5)
    if face == "left":
        return (w, h - j - .5, d - i - .5)
    if face == "top":
        return (i + .5, h, j + .5)
    return (i + .5, 0, d - j - .5)  # bottom


# --------------------------------------------------------------------------
# Deterministic noise / colour helpers
# --------------------------------------------------------------------------


def hash01(seed, *vals):
    h = seed & 0xFFFFFFFF
    for v in vals:
        h ^= (int(v) * 0x27D4EB2D) & 0xFFFFFFFF
        h = (h * 0x165667B1 + 0x9E3779B9) & 0xFFFFFFFF
        h ^= h >> 15
        h = (h * 0x85EBCA6B) & 0xFFFFFFFF
        h ^= h >> 13
        h = (h * 0xC2B2AE35) & 0xFFFFFFFF
        h ^= h >> 16
    return h / 4294967296.0


def _smooth(t):
    return t * t * (3 - 2 * t)


def vnoise(seed, x, y, z):
    import math
    x0, y0, z0 = math.floor(x), math.floor(y), math.floor(z)
    tx, ty, tz = _smooth(x - x0), _smooth(y - y0), _smooth(z - z0)

    def c(a, b, e):
        return hash01(seed, x0 + a, y0 + b, z0 + e)

    def lerp(a, b, t):
        return a + (b - a) * t
    return lerp(
        lerp(lerp(c(0, 0, 0), c(1, 0, 0), tx), lerp(c(0, 1, 0), c(1, 1, 0), tx), ty),
        lerp(lerp(c(0, 0, 1), c(1, 0, 1), tx), lerp(c(0, 1, 1), c(1, 1, 1), tx), ty),
        tz)


def fbm(seed, x, y, z, scale, octaves=3):
    total, amp, norm, f = 0.0, 1.0, 0.0, 1.0 / scale
    for o in range(octaves):
        total += amp * vnoise(seed + o * 101, x * f, y * f, z * f)
        norm += amp
        amp *= 0.5
        f *= 2.0
    return total / norm


def hexrgb(hx):
    hx = hx.lstrip("#")
    return tuple(int(hx[i:i + 2], 16) for i in (0, 2, 4))


def pal(*hexes):
    """Palette ordered dark -> light."""
    return [hexrgb(hx) for hx in hexes]


def pick(palette, t):
    k = int(t * len(palette))
    return palette[max(0, min(len(palette) - 1, k))]


def mul(rgb, k):
    return tuple(max(0, min(255, int(round(c * k)))) for c in rgb)


def mix(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


SIDES = ("front", "back", "left", "right")


# --------------------------------------------------------------------------
# Painters.  Each receives a context `c` describing one face pixel:
#   c.role, c.face, c.i, c.j (pixel in face), c.fw, c.fh (face size),
#   c.x, c.y, c.z (local position), c.w, c.h, c.d (box size),
#   c.gx, c.gy, c.gz (model-space position), c.px, c.py (skin pixel),
#   c.jit (per-pixel random 0..1), c.seed
# and returns an (r, g, b) colour.
# --------------------------------------------------------------------------

# ---- pig -----------------------------------------------------------------
PIG = pal("#c8737c", "#dc8c92", "#e9a2a4", "#f3b6b4", "#f9c8c3")
PIG_SNOUT = pal("#b85e6c", "#c8707b", "#d6828a", "#e19399")
PIG_HOOF = pal("#4a2e2e", "#5e3b39")


def paint_pig(c):
    n = fbm(c.seed, c.gx, c.gy, c.gz, 5.0)
    t = 0.18 + 0.62 * n + 0.28 * (c.jit - 0.5)
    if c.role == "body":
        if c.face == "bottom":
            t -= 0.2
        if c.face == "top":
            t += 0.1
        col = pick(PIG, t)
        if c.face == "back":  # a little curly tail
            tail = {(4, 1), (5, 1), (5, 2), (4, 3), (5, 3)}
            if (c.i, c.j) in tail:
                return PIG_SNOUT[0]
            if (c.i, c.j) in {(4, 2)}:
                return PIG[1]
        return col
    if c.role == "head":
        col = pick(PIG, t)
        if c.face == "front":
            eyes = {(1, 3): (250, 246, 240), (2, 3): (24, 18, 20),
                    (5, 3): (24, 18, 20), (6, 3): (250, 246, 240)}
            if (c.i, c.j) in eyes:
                return eyes[(c.i, c.j)]
            if c.j == 2 and c.i in (1, 2, 5, 6):  # soft brow shadow
                return mul(col, 0.9)
            if c.j == 0 and c.i in (0, 1, 6, 7):  # ear flaps
                return PIG[1] if c.jit < 0.6 else PIG[0]
            if c.j == 7 and 2 <= c.i <= 5:  # mouth line under the snout
                return PIG[0]
        if c.face in ("left", "right") and c.j <= 1 and 5 <= c.i <= 7:
            return PIG[1]  # ears seen from the side
        return col
    if c.role == "snout":
        col = pick(PIG_SNOUT, 0.25 + 0.6 * c.jit)
        if c.face == "front":
            if (c.i, c.j) in ((1, 1), (2, 1)):
                return (92, 40, 52)
            if c.j == 0:
                return PIG_SNOUT[3]
        return col
    # legs
    if c.face == "bottom" or (c.face in SIDES and c.j >= c.fh - 1):
        return pick(PIG_HOOF, c.jit)
    if c.face in SIDES and c.j == c.fh - 2:
        return pick(PIG, 0.1 + 0.2 * c.jit)
    return pick(PIG, t)


# ---- cow -----------------------------------------------------------------
COW_BROWN = pal("#3a2416", "#4a2f1d", "#5a3a25", "#6a462e")
COW_WHITE = pal("#cfc9bd", "#ddd8cd", "#e9e5dc", "#f4f1ea")
COW_MUZZLE = pal("#b98f7f", "#cba292", "#d9b3a3")
COW_HOOF = pal("#1e1712", "#2c221a")
COW_HORN = pal("#a99f86", "#cfc6aa", "#e6dec5")


def cow_coat(c, seed_off=0):
    n = fbm(c.seed + seed_off, c.gx * 1.0, c.gy * 1.3, c.gz * 0.9, 7.0, 3)
    t = 0.2 + 0.6 * fbm(c.seed + 77, c.gx, c.gy, c.gz, 3.0, 2) + 0.3 * (c.jit - 0.5)
    edge = abs(n - 0.5) < 0.025
    if n > 0.5:
        return pick(COW_WHITE, t + (-0.25 if edge else 0))
    return pick(COW_BROWN, t + (0.2 if edge else 0))


def paint_cow(c):
    if c.role == "body":
        col = cow_coat(c)
        if c.face == "bottom":
            col = mul(col, 0.88)
            # udder near the rear
            if 4 <= c.i <= 7 and 12 <= c.j <= 15:
                return pick(COW_MUZZLE, 0.3 + 0.6 * c.jit)
        return col
    if c.role == "head":
        if c.face == "front":
            if 5 <= c.j <= 7 and 1 <= c.i <= 6:  # muzzle
                if c.j == 6 and c.i in (2, 5):
                    return (64, 40, 36)
                if c.j == 7 and c.i in (1, 6):
                    return COW_MUZZLE[0]
                return pick(COW_MUZZLE, 0.35 + 0.55 * c.jit)
            if c.j == 3 and c.i in (1, 6):
                return (14, 10, 10)
            if c.j == 3 and c.i in (2, 5):
                return (236, 232, 226)
            if c.j == 2 and c.i in (1, 2, 5, 6):
                return COW_BROWN[0]
            if 3 <= c.i <= 4 and c.j <= 4:  # white blaze
                return pick(COW_WHITE, 0.4 + 0.5 * c.jit)
            return pick(COW_BROWN, 0.3 + 0.6 * c.jit)
        if c.face in ("left", "right") and c.j == 2 and c.i in (1, 2):
            return COW_BROWN[0]  # ear flap shading
        if c.face == "top":
            return pick(COW_BROWN, 0.3 + 0.6 * c.jit)
        return cow_coat(c)
    if c.role == "horn":
        if c.face == "top" or (c.face in SIDES and c.j == 0):
            return COW_HORN[2]
        if c.face in SIDES and c.j == 2 or c.face == "bottom":
            return COW_HORN[0]
        return COW_HORN[1]
    # legs: local coordinates so all four legs look the same
    lc = SimpleNamespace(**vars(c))
    lc.gx, lc.gy, lc.gz = c.x + 20, c.y, c.z + 20
    if c.face == "bottom" or (c.face in SIDES and c.j >= c.fh - 2):
        return pick(COW_HOOF, c.jit)
    if c.face in SIDES and c.j == c.fh - 3:
        return pick(COW_WHITE, 0.1 + 0.3 * c.jit)
    if c.face in SIDES and c.j >= 5:
        return pick(COW_WHITE, 0.2 + 0.7 * c.jit)
    return cow_coat(lc, 5)


# ---- sheep ---------------------------------------------------------------
WOOL = pal("#b9b1a3", "#cbc4b7", "#d9d3c8", "#e6e1d8", "#f1eee7")
SHEEP_FACE = pal("#6f6152", "#85766a", "#9a8b7c", "#ad9f8f")
SHEEP_HOOF = pal("#2e2622", "#3d332d")


def wool(c, scale=2.2):
    n = fbm(c.seed, c.gx, c.gy, c.gz, scale, 2)
    curl = hash01(c.seed + 9, c.px // 2, c.py // 2)
    t = 0.15 + 0.65 * n + 0.25 * (c.jit - 0.5) + 0.12 * (curl - 0.5)
    return pick(WOOL, t)


def paint_sheep(c):
    if c.role == "body":
        col = wool(c)
        if c.face == "bottom":
            col = mul(col, 0.9)
        return col
    if c.role == "head":
        t = 0.3 + 0.6 * c.jit
        if c.face == "front":
            if c.j == 0 or (c.j == 1 and c.i in (0, 5)):
                return wool(c)
            if c.j == 2 and c.i in (1, 4):
                return (16, 12, 10)
            if c.j == 2 and c.i in (0, 5):
                return (226, 220, 206)
            if c.j == 4 and c.i in (2, 3):
                return SHEEP_FACE[0]
            if c.j == 5 and 2 <= c.i <= 3:
                return SHEEP_FACE[1]
            return pick(SHEEP_FACE, t)
        if c.face in ("left", "right"):
            # wool cap on the back half / top rows; skin near the face
            near_face = c.z > c.d - 3
            if c.j <= 1 or not near_face:
                if c.j >= 4 and near_face:
                    return pick(SHEEP_FACE, t)
                return wool(c)
            return pick(SHEEP_FACE, t)
        if c.face == "bottom":
            return pick(SHEEP_FACE, t - 0.2) if c.z > c.d - 3 else wool(c)
        return wool(c)
    # legs: woolly top half, skin below, dark hooves
    if c.face == "top":
        return wool(c)
    if c.face == "bottom" or c.j >= c.fh - 1:
        return pick(SHEEP_HOOF, c.jit)
    if c.j < 5 or (c.j == 5 and c.jit < 0.5):
        return wool(c)
    return pick(SHEEP_FACE, 0.2 + 0.6 * c.jit)


# ---- chicken ---------------------------------------------------------------
FEATHER = pal("#cfcfc6", "#e0e0d8", "#ecece6", "#f8f8f4")
WING = pal("#9e9e96", "#b4b4ac", "#c8c8c0", "#dadad2")
BEAK = pal("#c98a1c", "#e2aa2a", "#f2c845")
WATTLE = pal("#8e1a1e", "#b52428", "#d8383a")
CHICK_LEG = pal("#b8701c", "#d68e26", "#eaaa38")


def paint_chicken(c):
    t = 0.25 + 0.6 * c.jit
    if c.role == "body":
        col = pick(FEATHER, t)
        if c.face == "back" and c.j <= 1 and 2 <= c.i <= 3:
            return FEATHER[0]  # tail tuft shadow
        if c.face == "bottom":
            col = mul(col, 0.92)
        return col
    if c.role == "head":
        col = pick(FEATHER, t)
        if c.face == "front" and c.j == 1 and c.i in (0, 3):
            return (14, 12, 12)
        if c.face in ("left", "right") and c.j == 1 and c.i == c.fw - 1:
            return (14, 12, 12)
        if c.face == "top" and c.j == c.fh - 1 and c.i in (1, 2):
            return WATTLE[2]  # little red comb
        if c.face == "top" and c.i in (1, 2):
            return WATTLE[1]
        return col
    if c.role == "beak":
        if c.face == "top":
            return BEAK[2]
        if c.face == "bottom" or (c.face in SIDES and c.j == 1):
            return BEAK[0] if c.face != "front" else BEAK[1]
        return BEAK[2] if c.jit > 0.3 else BEAK[1]
    if c.role == "wattle":
        if c.face == "bottom" or c.j == 1:
            return WATTLE[0] if c.jit < 0.5 else WATTLE[1]
        return WATTLE[2] if c.jit > 0.4 else WATTLE[1]
    if c.role == "wing":
        if c.face in ("left", "right"):
            # feather rows; darker tips at the bottom and the back end
            if c.j == c.fh - 1 or (c.j >= 2 and c.i <= 1):
                return pick(WING, 0.1 + 0.3 * c.jit)
            if c.j == 1 and c.i % 2 == 0:
                return WING[1]
            return pick(WING, 0.4 + 0.6 * c.jit)
        return pick(WING, 0.3 + 0.5 * c.jit)
    # legs
    if c.face == "bottom" or (c.face in SIDES and c.j == c.fh - 1):
        return CHICK_LEG[0]
    return pick(CHICK_LEG, 0.35 + 0.6 * c.jit)


# ---- zombie ----------------------------------------------------------------
ZSKIN = pal("#3e5640", "#4c6a4c", "#5a7a57", "#6b8c66", "#7d9c74")
ZSHIRT = pal("#173f44", "#1f5257", "#286468", "#33787a")
ZPANTS = pal("#1c1b3a", "#25244a", "#2f2e5a", "#3a3a6c")
ZSHOE = pal("#1d1714", "#2a211b")
ZROPE = pal("#6e5532", "#8c6d40")
STITCH = (28, 22, 24)
SCAR = (134, 120, 112)


def zskin(c):
    n = fbm(c.seed + 3, c.gx, c.gy, c.gz, 3.0, 2)
    return pick(ZSKIN, 0.15 + 0.6 * n + 0.3 * (c.jit - 0.5))


def zshirt(c):
    n = fbm(c.seed + 5, c.gx, c.gy, c.gz, 3.0, 2)
    return pick(ZSHIRT, 0.15 + 0.6 * n + 0.3 * (c.jit - 0.5))


def paint_zombie(c):
    if c.role == "head":
        col = zskin(c)
        if c.face == "front":
            # hollow eyes: black socket, dim green glint, darker rim
            sockets = {(1, 3), (2, 3), (1, 4), (2, 4), (5, 3), (6, 3), (5, 4), (6, 4)}
            if (c.i, c.j) in sockets:
                return (40, 255 // 4, 44) if (c.i, c.j) in ((2, 4), (5, 4)) else (12, 14, 12)
            if c.j == 2 and c.i in (1, 2, 5, 6):
                return ZSKIN[0]
            if c.j == 5 and c.i in (3, 4):
                return ZSKIN[0]  # nose holes
            if c.j == 6 and 2 <= c.i <= 5:  # crooked mouth
                return (30, 24, 24) if c.i != 4 else (190, 178, 150)
            if c.j == 7 and c.i == 5:
                return (30, 24, 24)
            # stitched scar from forehead down the left cheek
            scar = {(5, 0), (6, 1), (7, 2)}
            if (c.i, c.j) in scar:
                return SCAR
            if (c.i, c.j) in {(6, 0), (7, 1)}:
                return STITCH
            return col
        if c.face == "top":
            # thin, patchy dark hair
            if fbm(c.seed + 11, c.gx, c.gy, c.gz, 2.0, 2) < 0.45:
                return (34, 32, 28) if c.jit < 0.6 else (48, 44, 38)
            return col
        if c.face in ("back", "left", "right") and c.j <= 1:
            if fbm(c.seed + 11, c.gx, c.gy, c.gz, 2.0, 2) < 0.5:
                return (34, 32, 28)
        if c.face == "left" and c.i == 4 and 3 <= c.j <= 4:
            return ZSKIN[0]  # ear
        if c.face == "right" and c.i == 3 and 3 <= c.j <= 4:
            return ZSKIN[0]
        return col
    if c.role == "body":
        if c.face in ("top", "bottom"):
            return zshirt(c) if c.face == "top" else ZPANTS[1]
        # rope belt on the last row
        if c.j == c.fh - 1:
            return pick(ZROPE, c.jit)
        # ragged hem: skin peeks through above the belt
        hem = 8 + int(hash01(c.seed + 21, int(c.gx * 2), int(c.gz * 2)) * 3)
        if c.j >= hem:
            return zskin(c)
        col = zshirt(c)
        if c.face == "front":
            # torn hole on the right chest with stitched edge
            if (c.i, c.j) in {(5, 3), (6, 3), (5, 4), (6, 5)}:
                return zskin(c)
            if (c.i, c.j) in {(4, 3), (4, 4), (5, 5)}:
                return ZSHIRT[0]
            # ochre patch, sewn on with dark stitches
            if 1 <= c.i <= 2 and 5 <= c.j <= 6:
                return (156, 118, 46) if c.jit > 0.3 else (134, 98, 38)
            if (c.i, c.j) in {(0, 5), (3, 6), (1, 7), (2, 4)}:
                return STITCH
            # collar
            if c.j == 0 and 2 <= c.i <= 5:
                return zskin(c) if 3 <= c.i <= 4 else ZSHIRT[0]
        if c.face == "back" and (c.i, c.j) in {(2, 2), (3, 3), (4, 2), (3, 1), (2, 4)}:
            return zskin(c)  # gash in the back
        return col
    if c.role == "arm":
        if c.face == "top":
            return zshirt(c)
        if c.face == "bottom":
            return zskin(c)
        # short sleeve with a ragged edge, then green arm; darker hands
        sleeve = 3 + (1 if hash01(c.seed + 31, c.i, zlib.crc32(c.face.encode())) > 0.5 else 0)
        if c.j < sleeve:
            if c.j == sleeve - 1 and c.jit < 0.3:
                return ZSHIRT[0]
            return zshirt(c)
        col = zskin(c)
        if c.j >= c.fh - 3:
            col = mul(col, 0.88)
        if c.face == "front" and c.j == 6 and c.i in (1, 2):
            return STITCH if c.i == 1 else SCAR
        return col
    # legs
    if c.face == "bottom" or (c.face in SIDES and c.j >= c.fh - 2):
        return pick(ZSHOE, c.jit)
    if c.face == "top":
        return ZPANTS[1]
    n = fbm(c.seed + 7, c.x + 30, c.y, c.z, 3.0, 2)
    col = pick(ZPANTS, 0.15 + 0.6 * n + 0.3 * (c.jit - 0.5))
    if c.face == "front" and 5 <= c.j <= 6 and 1 <= c.i <= 2:
        return (64, 58, 52)  # knee patch
    if c.face == "front" and c.j == 7 and c.i == 3:
        return STITCH
    if c.face in ("left", "right") and c.j == c.fh - 3:
        return ZPANTS[0]  # frayed cuff
    return col


# ---- skeleton --------------------------------------------------------------
BONE = pal("#8a877d", "#a8a59a", "#c2bfb3", "#d8d5c9", "#e8e6dc")
VOID = (26, 24, 24)
VOID2 = (44, 41, 40)


def paint_skeleton(c):
    t = 0.3 + 0.6 * c.jit
    if c.role == "head":
        col = pick(BONE, t)
        if c.face == "bottom":
            return pick(BONE, t - 0.35)
        if c.face == "front":
            sockets = {(1, 3), (2, 3), (1, 4), (2, 4), (5, 3), (6, 3), (5, 4), (6, 4)}
            if (c.i, c.j) in sockets:
                return VOID
            if c.j == 2 and c.i in (1, 2, 5, 6):
                return BONE[1]
            if (c.i, c.j) in ((3, 5), (4, 5)):
                return VOID2  # nose cavity
            if c.j == 6:
                if 1 <= c.i <= 6:
                    return VOID if c.i % 2 == 0 else BONE[4]
                return BONE[1]
            if c.j == 7:
                return BONE[2] if 1 <= c.i <= 6 else BONE[1]
            if c.j == 5 and c.i in (0, 7):
                return BONE[1]  # cheekbone shadow
        if c.face in ("left", "right") and c.j >= 5:
            return pick(BONE, t - 0.2)
        if c.face == "top" and (c.i, c.j) in {(2, 2), (3, 3), (3, 4), (4, 5)}:
            return BONE[1]  # hairline crack
        return col
    if c.role == "body":
        if c.face in ("top", "bottom"):
            return VOID2 if not (3 <= c.i <= 4) else pick(BONE, t)
        if c.face in ("front", "back"):
            if 3 <= c.i <= 4 and c.j <= 8:  # spine
                return BONE[3] if (c.j % 2 == 0) == (c.i == 3) else BONE[2]
            if c.j in (1, 3, 5, 7) and 1 <= c.i <= 6:  # ribs
                return pick(BONE, t)
            if c.j in (2, 4, 6) and c.i in (0, 7):  # rib ends curving down
                return BONE[1]
            if c.j >= 9:  # pelvis
                if c.j == 9 and 1 <= c.i <= 6:
                    return pick(BONE, t)
                if c.j == 10 and c.i in (1, 2, 5, 6):
                    return pick(BONE, t - 0.2)
                if c.j == 11 and c.i in (2, 5):
                    return BONE[1]
            return VOID
        # sides: rib ends
        if c.j in (1, 3, 5, 7) or (c.j == 9):
            return pick(BONE, t - 0.25)
        return VOID
    # arms and legs: bone shaft with a knobby joint in the middle
    if c.face in ("top", "bottom"):
        return BONE[2] if c.face == "top" else BONE[1]
    shade = 0.0
    if c.face in ("back", "right"):
        shade = -0.2
    if c.i == c.fw - 1:
        shade -= 0.12
    if c.j in (5, 6):
        return pick(BONE, 0.75 + shade) if c.j == 5 else pick(BONE, 0.2 + shade)
    if c.j == c.fh - 1:
        return BONE[1] if c.role == "leg" else BONE[2]
    return pick(BONE, t + shade)


# ---- stalker ---------------------------------------------------------------
MOSS = pal("#24381c", "#2f4a24", "#3d5e2c", "#4d7234", "#5f863f", "#749a4e")
LICHEN = pal("#8d9c6a", "#a7b37e", "#bfc596")
ROOT_C = pal("#2c2a1a", "#3b3822", "#4a462b")
FACE_DARK = (14, 18, 12)
FACE_MID = (30, 40, 24)


def moss(c):
    n = fbm(c.seed, c.gx, c.gy, c.gz, 3.5, 3)
    t = 0.05 + 0.85 * n + 0.32 * (c.jit - 0.5)
    lich = fbm(c.seed + 50, c.gx, c.gy, c.gz, 1.6, 2)
    if lich > 0.68:
        return pick(LICHEN, (lich - 0.68) * 3.2 + 0.2 * (c.jit - 0.5))
    return pick(MOSS, t)


def paint_stalker(c):
    if c.role == "head":
        col = moss(c)
        if c.face == "front":
            # sad face on a paler lichen mask: brows that droop outwards, two
            # small round eyes with a wet glint, a thin vertical slit mouth
            # and one tear.  (Deliberately not a blocky 2x2-eyes face.)
            mask = 1 <= c.i <= 6 and 1 <= c.j <= 7 and (c.i, c.j) not in (
                (1, 1), (6, 1), (1, 7), (6, 7))
            if (c.i, c.j) in {(1, 2), (2, 1), (5, 1), (6, 2)}:
                return FACE_MID  # brows, inner ends raised
            if (c.i, c.j) in {(2, 4), (5, 4)}:
                return FACE_DARK  # small round eyes
            if (c.i, c.j) in {(2, 3), (5, 3)}:
                return MOSS[3]  # lid shadow
            if (c.i, c.j) == (2, 5):
                return (150, 196, 178)  # a single tear
            if c.i == 3 and 5 <= c.j <= 7:
                return FACE_DARK  # vertical slit mouth
            if c.i == 4 and 5 <= c.j <= 7:
                return FACE_MID if c.j != 5 else MOSS[1]
            if mask:
                return pick(MOSS, 0.62 + 0.4 * c.jit) if c.jit < 0.9 else LICHEN[0]
            return col
        if c.face == "top":
            # leafy crown: four leaves radiating from a small bud
            leaf = {(3, 3): 2, (4, 4): 2, (3, 4): 3, (4, 3): 3,
                    (2, 2): 5, (1, 1): 4, (5, 2): 5, (6, 1): 4,
                    (2, 5): 5, (1, 6): 4, (5, 5): 5, (6, 6): 4}
            if (c.i, c.j) in leaf:
                k = leaf[(c.i, c.j)]
                if k == 3:
                    return (150, 64, 72)  # bud
                if k == 2:
                    return (110, 40, 52)
                return MOSS[k]
        return col
    if c.role == "body":
        col = moss(c)
        if c.face in SIDES:
            # darker vertical vine ribs
            k = c.i if c.face in ("front", "back") else c.i * 2
            if k % 3 == 1 and hash01(c.seed + 60, c.px, c.py // 3) < 0.6:
                return mul(col, 0.78)
            if c.face == "front" and (c.i, c.j) in {(3, 5), (4, 6), (3, 7)}:
                return (60, 30, 36)  # faint reddish vein (it's going to pop)
        return col
    # legs: moss above, rooty bark below
    if c.face == "bottom":
        return pick(ROOT_C, c.jit)
    if c.face in SIDES and c.j >= c.fh - 2:
        return pick(ROOT_C, 0.2 + 0.7 * c.jit)
    if c.face in SIDES and c.j == c.fh - 3 and c.jit < 0.5:
        return ROOT_C[2]
    lc = SimpleNamespace(**vars(c))
    lc.gx, lc.gy, lc.gz = c.x + 40, c.y, c.z + 40
    return moss(lc)


# ---- spider ----------------------------------------------------------------
SPIDER = pal("#120e0c", "#1c1612", "#261e18", "#30261e", "#3c3026")
HAIR = (78, 64, 50)
RED_EYE = (210, 28, 30)
RED_EYE_HI = (255, 112, 96)
RED_EYE_DIM = (140, 20, 24)


def spider_fur(c, bias=0.0):
    n = fbm(c.seed, c.gx, c.gy, c.gz, 2.5, 2)
    h = hash01(c.seed + 99, c.px, c.py)
    if h > 0.9:
        return HAIR
    if h > 0.82:
        return SPIDER[4]
    return pick(SPIDER, 0.1 + 0.55 * n + 0.3 * (c.jit - 0.5) + bias)


def paint_spider(c):
    if c.role == "head":
        col = spider_fur(c)
        if c.face == "front":
            # eight eyes: two big ones, a tiny centre pair, two pairs at the sides
            if (c.i, c.j) in {(2, 3), (5, 3)}:
                return RED_EYE_HI
            if (c.i, c.j) in {(2, 4), (5, 4)}:
                return RED_EYE
            if (c.i, c.j) in {(3, 1), (4, 1)}:
                return RED_EYE if c.i == 3 else RED_EYE_DIM
            if (c.i, c.j) in {(0, 2), (7, 2), (0, 4), (7, 4)}:
                return RED_EYE_DIM
            # fangs
            if (c.i, c.j) in {(2, 6), (5, 6)}:
                return (150, 132, 104)
            if (c.i, c.j) in {(2, 7), (5, 7)}:
                return (96, 82, 64)
            if c.j >= 6:
                return SPIDER[0]
        return col
    if c.role == "neck":
        return spider_fur(c, -0.2)
    if c.role == "abdomen":
        col = spider_fur(c, 0.05)
        if c.face == "top":
            # faint rusty chevrons down the back
            k = c.j % 4
            off = abs(c.i * 2 + 1 - c.w)  # 1,3,5,... from the centre line
            if off // 2 == k and c.j < 11:
                return (96, 44, 30) if c.jit > 0.3 else (74, 34, 24)
        return col
    # legs: 16 long; lighter knobby joints at 1/3 and 2/3, darker claw tips
    if c.face in ("left", "right"):
        return SPIDER[0]
    along = c.x  # 0..16 local along the leg
    if 5.0 < along < 6.0 or 10.0 < along < 11.0:
        return (88, 72, 56) if c.jit > 0.35 else (70, 56, 44)
    if along < 1.5 or along > 14.5:
        return SPIDER[0]
    return spider_fur(c, 0.05)


PAINTERS = {
    "pig": paint_pig, "cow": paint_cow, "sheep": paint_sheep,
    "chicken": paint_chicken, "zombie": paint_zombie,
    "skeleton": paint_skeleton, "stalker": paint_stalker, "spider": paint_spider,
}


# --------------------------------------------------------------------------
# Driver
# --------------------------------------------------------------------------


def render_skin(name, parts):
    img = Image.new("RGBA", (SKIN, SKIN), (0, 0, 0, 0))
    px = img.load()
    owner = {}
    seed = BASE_SEED ^ zlib.crc32(name.encode())
    painter = PAINTERS[name]
    done_uv = set()
    for role, (size, origin, uv) in zip(ROLES[name], parts):
        if uv in done_uv:
            continue  # shares pixels with an earlier part
        done_uv.add(uv)
        w, h, d = size
        for face, (x0, y0, fw, fh) in face_rects(size, uv).items():
            for j in range(fh):
                for i in range(fw):
                    sx, sy = x0 + i, y0 + j
                    assert 0 <= sx < SKIN and 0 <= sy < SKIN, (name, role, face)
                    prev = owner.get((sx, sy))
                    assert prev is None, f"{name}: {role}.{face} overlaps {prev} at {(sx, sy)}"
                    owner[(sx, sy)] = f"{role}.{face}"
                    lx, ly, lz = local_pos(face, i, j, size)
                    c = SimpleNamespace(
                        role=role, face=face, i=i, j=j, fw=fw, fh=fh,
                        x=lx, y=ly, z=lz, w=w, h=h, d=d,
                        gx=origin[0] + lx, gy=origin[1] + ly, gz=origin[2] + lz,
                        px=sx, py=sy, jit=hash01(seed + 1, sx, sy), seed=seed)
                    rgb = painter(c)
                    px[sx, sy] = (*rgb[:3], 255)
    return img


def checker(size, cell=8):
    bg = Image.new("RGBA", size, (60, 60, 66, 255))
    d = ImageDraw.Draw(bg)
    for y in range(0, size[1], cell):
        for x in range(0, size[0], cell):
            if (x // cell + y // cell) % 2:
                d.rectangle([x, y, x + cell - 1, y + cell - 1], fill=(78, 78, 86, 255))
    return bg


def main():
    parsed = parse_mobs(MOBS_RON)
    assert set(parsed) == set(ROLES), f"mob list differs: {sorted(parsed)}"
    for name, parts in parsed.items():
        assert parts == HARDCODED[name], f"{name}: parts in mobs.ron changed"
        assert len(parts) == len(ROLES[name]), name
    os.makedirs(OUT_DIR, exist_ok=True)
    order = ["pig", "cow", "sheep", "chicken", "zombie", "skeleton", "stalker", "spider"]
    scale, gap = 4, 8
    cols = 4
    rows = (len(order) + cols - 1) // cols
    sheet_w = cols * SKIN * scale + (cols + 1) * gap
    sheet_h = rows * SKIN * scale + (rows + 1) * gap
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (24, 24, 28, 255))
    for k, name in enumerate(order):
        img = render_skin(name, parsed[name])
        path = os.path.join(OUT_DIR, f"{name}.png")
        img.save(path)
        print("wrote", os.path.relpath(path, ROOT))
        big = img.resize((SKIN * scale, SKIN * scale), Image.NEAREST)
        tile = checker(big.size)
        tile.alpha_composite(big)
        x = gap + (k % cols) * (SKIN * scale + gap)
        y = gap + (k // cols) * (SKIN * scale + gap)
        sheet.paste(tile, (x, y))
    path = os.path.join(OUT_DIR, "preview.png")
    sheet.save(path)
    print("wrote", os.path.relpath(path, ROOT))


if __name__ == "__main__":
    main()
