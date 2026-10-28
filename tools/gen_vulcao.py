#!/usr/bin/env python3
"""The volcanoes' content (Lote 14, crates/voxel/src/worldgen/volcano.rs and crates/game/src/vulcao.rs).

Writes packs/vanilla/vulcao/{blocks,items,recipes,mobs}.ron, the 64x64 textures of the new
blocks, the item icons (32x32 pixel art grown to 64x64, the style of tools/tex64/pixel.py), the
spawn eggs and the creatures' 64x128 skins (tools/tex64/entities.py grows those to 4x).

Every shape and pixel is drawn here: rock with glowing seams, a salamander's crust, embers.

    python3 tools/gen_vulcao.py && python3 tools/tex64/entities.py
"""

import math
import os
import sys
import zlib

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "tex64"))
from dim_mobs_spec import Model  # noqa: E402
from gen_mob_skins import face_rects, fbm, hash01, hexrgb, local_pos, mix, mul  # noqa: E402

ROOT = os.path.dirname(HERE)
PACK = os.path.join(ROOT, "packs", "vanilla")
OUT = os.path.join(PACK, "vulcao")
ENTITY = os.path.join(PACK, "textures", "entity")
BLOCK_TEX = os.path.join(PACK, "textures", "block")
ITEM_TEX = os.path.join(PACK, "textures", "item")

BASALT = ["#16151a", "#222127", "#2f2d35", "#3d3b44", "#4d4a55", "#5f5c68"]
MAGMA = ["#6e1a08", "#b3360c", "#e2611a", "#f79a2e", "#ffd27a"]
CRUST = ["#1a0e0b", "#2a1611", "#3b2016", "#4e2b1b", "#663a22"]
EMBER = ["#7a1f06", "#c44410", "#ef7a20", "#ffb84a", "#fff0b0"]


# ------------------------------------------------------------------------------- creatures


def look(key, pal, pattern, eyes=None, glow=None, belly=None):
    return dict(key=key, pal=pal, pattern=pattern, eyes=eyes, glow=glow, belly=belly)


def shade(pal, t):
    pal = [hexrgb(c) for c in pal]
    t = max(0.0, min(0.999, t))
    k = t * (len(pal) - 1)
    i = int(k)
    return mix(pal[i], pal[min(i + 1, len(pal) - 1)], k - i)


def paint(lk, face, i, j, fw, fh, size, g, seed):
    n = fbm(seed, g[0], g[1], g[2], 3.0)
    jit = hash01(seed, g[0] * 7, g[1] * 13, g[2] * 3) - 0.5
    pat = lk["pattern"]
    w, h, d = size
    up = local_pos(face, i, j, size)[1] / max(h, 1)
    t = 0.3 + 0.5 * n + 0.15 * jit + 0.15 * up
    rgb = None
    if pat == "rock":
        # Basalt plates with glowing seams between them.
        seam = abs(fbm(seed + 31, g[0], g[1] * 0.8, g[2], 2.2) - 0.5) < 0.018
        t = 0.25 + 0.5 * n + 0.2 * jit + (0.15 if face == "top" else 0.0)
        if seam and face != "top":
            rgb = shade(lk["glow"], 0.35 + 0.5 * n)
    elif pat == "crust":
        # A salamander: dark cracked crust with orange glowing spots, a hot belly.
        spot = fbm(seed + 7, g[0], g[1], g[2], 1.4) > 0.68
        t = 0.2 + 0.55 * n + 0.15 * jit
        if spot:
            rgb = shade(lk["glow"], 0.3 + 0.6 * n)
    elif pat == "flame":
        t = 0.2 + 0.75 * fbm(seed + 11, g[0], g[1] * 0.5, g[2], 1.6) + 0.15 * jit + 0.25 * up
    elif pat == "core":
        t = 0.6 + 0.35 * n + 0.1 * jit
    if face == "top" and pat != "flame":
        t += 0.08
    elif face == "bottom":
        t -= 0.15
    if rgb is None:
        rgb = shade(lk["pal"], t)
    if lk.get("belly") and face == "bottom":
        rgb = shade(lk["belly"], 0.45 + 0.4 * n)
    eyes = lk.get("eyes")
    if eyes and face == "front":
        ey = max(1, fh // 2 - 1)
        ex = max(1, fw // 4)
        if j == ey and i in (ex, fw - 1 - ex):
            rgb = hexrgb(eyes)
        elif j == ey - 1 and i in (ex, fw - 1 - ex) and fh > 4:
            rgb = mul(hexrgb(eyes), 0.7)
    return rgb


def render(name, model):
    img = Image.new("RGBA", (64, 128), (0, 0, 0, 0))
    px = img.load()
    seed = zlib.crc32(name.encode())
    done = set()
    for p in model.parts:
        if p["uv"] in done:
            continue
        done.add(p["uv"])
        for face, (x0, y0, fw, fh) in face_rects(p["size"], p["uv"]).items():
            for j in range(fh):
                for i in range(fw):
                    lx, ly, lz = local_pos(face, i, j, p["size"])
                    g = (p["origin"][0] + lx, p["origin"][1] + ly, p["origin"][2] + lz)
                    px[x0 + i, y0 + j] = (*paint(p["look"], face, i, j, fw, fh, p["size"], g, seed), 255)
    # Relief layers: knobs of rock or flame tongues over the part.
    for idx, _ in model.layers:
        p = model.parts[idx]
        for face, (x0, y0, fw, fh) in face_rects(p["size"], p["uv"]).items():
            for j in range(fh):
                for i in range(fw):
                    lx, ly, lz = local_pos(face, i, j, p["size"])
                    g = (p["origin"][0] + lx, p["origin"][1] + ly, p["origin"][2] + lz)
                    if fbm(seed + 77, g[0], g[1], g[2], 1.8) > 0.56 and not (face == "front" and p["look"].get("eyes")):
                        rgb = paint(p["look"], face, i, j, fw, fh, p["size"], g, seed + 1)
                        px[x0 + i, y0 + j + 64] = (*mul(rgb, 1.1), 255)
    return img


def salamander():
    m = Model()
    body = look("sbody", CRUST, "crust", glow=MAGMA, belly=MAGMA[1:])
    head = look("shead", CRUST, "crust", eyes="#ffe08a", glow=MAGMA)
    leg = look("sleg", CRUST, "crust", glow=MAGMA)
    m.box((6, 4, 14), (-3, 3, -7), body)
    m.box((6, 3, 5), (-3, 4, 7), head, "Head", (0, 5, 7))
    m.box((4, 2, 2), (-2, 4, 12), head, "Head", (0, 5, 7))
    for k, (x, z) in enumerate([(-5, 3), (3, 3), (-5, -5), (3, -5)]):
        m.box((2, 3, 2), (x, 0, z), leg, f"Leg({(k + k // 2) % 2})", (x + 1, 3, z + 1))
    m.box((3, 2, 7), (-1, 4, -14), body)
    m.box((2, 1, 5), (-1, 4, -19), body)
    # A ridge of glowing spines down the back.
    m.box((2, 2, 10), (-1, 7, -5), look("sridge", MAGMA, "core"))
    m.layer(0, 0.5)
    m.pack()
    return m


def golem():
    m = Model()
    rock = look("grock", BASALT, "rock", glow=MAGMA)
    head = look("ghead", BASALT, "rock", eyes="#ffcf5a", glow=MAGMA)
    arm = look("garm", BASALT, "rock", glow=MAGMA)
    m.box((14, 12, 9), (-7, 9, -4), rock)
    m.box((8, 7, 7), (-4, 18, -1), head, "Head", (0, 19, 2))
    m.box((5, 15, 5), (-12, 5, -2), arm, "Arm(0)", (-9, 19, 0))
    m.box((5, 15, 5), (7, 5, -2), arm, "Arm(1)", (9, 19, 0))
    m.box((5, 9, 5), (-6, 0, -2), arm, "Leg(0)", (-3, 9, 0))
    m.box((5, 9, 5), (1, 0, -2), arm, "Leg(1)", (3, 9, 0))
    m.layer(0, 0.6)
    m.pack()
    return m


def golem_ball():
    m = Model()
    rock = look("brock", BASALT, "rock", glow=MAGMA)
    m.box((10, 10, 10), (-5, 0, -5), rock)
    m.box((12, 6, 6), (-6, 2, -3), rock)
    m.box((6, 6, 12), (-3, 2, -6), rock)
    m.box((6, 12, 6), (-3, -1, -3), rock)
    m.layer(0, 0.6)
    m.pack()
    return m


def wisp():
    m = Model()
    core = look("wcore", EMBER[2:], "core", eyes="#3a1004")
    flame = look("wflame", EMBER, "flame")
    m.box((5, 5, 5), (-2, 6, -2), core, "Head", (0, 8, 0))
    m.box((7, 7, 7), (-3, 5, -3), flame)
    m.box((3, 6, 3), (-1, 0, -1), flame)
    m.box((2, 5, 4), (-6, 7, -2), flame, "Wing(0)", (-3, 9, 0))
    m.box((2, 5, 4), (4, 7, -2), flame, "Wing(1)", (3, 9, 0))
    m.pack()
    return m


def titan():
    m = Model()
    rock = look("trock", BASALT, "rock", glow=MAGMA)
    head = look("thead", BASALT, "rock", eyes="#fff0b0", glow=MAGMA)
    m.box((12, 10, 7), (-6, 8, -3), rock)
    m.box((8, 7, 7), (-4, 18, -3), head, "Head", (0, 18, 0))
    m.box((5, 13, 5), (-11, 4, -2), rock, "Arm(0)", (-8, 17, 0))
    m.box((5, 13, 5), (6, 4, -2), rock, "Arm(1)", (8, 17, 0))
    m.box((5, 8, 5), (-5, 0, -2), rock, "Leg(0)", (-2, 8, 0))
    m.box((5, 8, 5), (0, 0, -2), rock, "Leg(1)", (2, 8, 0))
    # Shoulders of glowing magma and a crown of it.
    crest = look("tcrest", MAGMA, "core")
    m.box((4, 3, 4), (-11, 17, -2), crest, "Arm(0)", (-8, 17, 0))
    m.box((4, 3, 4), (7, 17, -2), crest, "Arm(1)", (8, 17, 0))
    m.box((6, 2, 5), (-3, 25, -2), crest, "Head", (0, 18, 0))
    m.layer(0, 0.8)
    m.layer(1, 0.6)
    m.pack()
    return m


MOBS = [
    dict(name="magma_salamander", display="Salamandra de Magma", model=salamander(), egg=("#3b2016", "#f07a1c"),
         ron='kind: Neutral, health: 18, speed: 2.3, width: 0.9, height: 0.7, spawn: Never, attack: 4, biomes: ["Volcano"], drops: [(item: "salamander_scale", min: 1, max: 2), (item: "magma_cream", min: 0, max: 1)], xp: 6, swims: true, lit: true, motion: (style: Some("crawler"))'),
    dict(name="basalt_golem", display="Golem de Basalto", model=golem(), egg=("#2f2d35", "#e2611a"),
         ron='kind: Hostile, health: 46, speed: 1.15, width: 1.2, height: 2.0, spawn: Never, attack: 7, biomes: ["Volcano", "Ash Fields"], drops: [(item: "basalt", min: 2, max: 4), (item: "sulfur", min: 0, max: 2)], xp: 12, coins: Some(60), motion: (heavy: Some(0.8))'),
    dict(name="basalt_golem_ball", display="Golem de Basalto", model=golem_ball(), egg=None,
         ron='kind: Hostile, health: 46, speed: 1.0, width: 1.2, height: 1.2, spawn: Never, attack: 9, scale: 1.3, biomes: ["Volcano"], drops: [(item: "basalt", min: 2, max: 4), (item: "sulfur", min: 0, max: 2)], xp: 12, coins: Some(60), motion: (style: Some("static"))'),
    dict(name="ember_wisp", display="Fagulha", model=wisp(), egg=("#c44410", "#fff0b0"),
         ron='kind: Hostile, health: 12, speed: 2.8, width: 0.6, height: 0.8, spawn: Never, attack: 4, ranged: true, flies: true, hover: 3.0, range: 14.0, biomes: ["Volcano", "Ash Fields"], drops: [(item: "sulfur", min: 0, max: 1), (item: "gunpowder", min: 0, max: 1)], xp: 7, lit: true, motion: (style: Some("floater"))'),
    dict(name="magma_titan", display="Titã de Magma", model=titan(), egg=None,
         ron='kind: Hostile, health: 1150, speed: 1.4, width: 4.4, height: 11.0, spawn: Never, attack: 11, scale: 4.4, boss: true, biomes: ["Volcano"], drops: [(item: "magma_heart"), (item: "volcanic_hammer", chance: 0.5), (item: "obsidian", min: 8, max: 16), (item: "sulfur", min: 6, max: 12)], xp: 180, coins: Some(40000), lit: true'),
]


def ron_mob(m):
    parts = []
    for p in m["model"].parts:
        s = f"(size: {p['size']}, origin: {p['origin']}, uv: {p['uv']}"
        if p["anim"]:
            s += f", anim: {p['anim']}, pivot: {p['pivot']}"
        parts.append(s + ")")
    out = f'    (\n        name: "{m["name"]}", display_name: "{m["display"]}", {m["ron"]},\n        parts: [\n'
    out += "".join(f"            {p},\n" for p in parts) + "        ],\n"
    if m["model"].layers:
        out += "        layers: [" + ", ".join(f"(part: {a}, inflate: {b})" for a, b in m["model"].layers) + "],\n"
    return out + "    ),\n"


# ------------------------------------------------------------------------------- item icons


G = 32


class Art:
    """A 32x32 sprite: colours on a grid, then a crisp outline in each shape's own dark."""

    def __init__(self):
        self.c = np.zeros((G, G, 3))
        self.a = np.zeros((G, G), bool)
        self.edge = np.zeros((G, G, 3))

    def poly(self, pts, fill, outline):
        ys, xs = np.mgrid[0:G, 0:G]
        px, py = xs + 0.5, ys + 0.5
        inside = np.zeros((G, G), bool)
        n = len(pts)
        for k in range(n):
            (x1, y1), (x2, y2) = pts[k], pts[(k + 1) % n]
            cond = ((y1 > py) != (y2 > py)) & (px < (x2 - x1) * (py - y1) / (y2 - y1 + 1e-9) + x1)
            inside ^= cond
        self.paint(inside, fill, outline)
        return inside

    def disc(self, cx, cy, r, fill, outline):
        ys, xs = np.mgrid[0:G, 0:G]
        m = (xs + 0.5 - cx) ** 2 + (ys + 0.5 - cy) ** 2 <= r * r
        self.paint(m, fill, outline)
        return m

    def paint(self, m, fill, outline):
        ys, xs = np.mgrid[0:G, 0:G]
        cols = fill(xs, ys) if callable(fill) else np.broadcast_to(hexrgb(fill), (G, G, 3))
        self.c[m] = np.asarray(cols)[m]
        self.a |= m
        self.edge[m] = hexrgb(outline)

    def px(self, x, y, col):
        if 0 <= x < G and 0 <= y < G:
            self.c[y, x] = hexrgb(col)
            self.a[y, x] = True

    def image(self):
        out = np.zeros((G, G, 4))
        out[..., :3] = self.c
        out[..., 3] = self.a * 255
        # The outline: every empty pixel next to the sprite takes its neighbour's dark.
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            nb = np.roll(np.roll(self.a, dy, 0), dx, 1)
            ec = np.roll(np.roll(self.edge, dy, 0), dx, 1)
            m = nb & ~self.a & (out[..., 3] == 0)
            out[m, :3] = ec[m]
            out[m, 3] = 255
        img = Image.fromarray(out.astype(np.uint8), "RGBA")
        return img.resize((64, 64), Image.NEAREST)


def ramp_fill(pal, f):
    """fill(xs, ys) from a dark-to-light palette and a 0..1 field."""
    cols = np.array([hexrgb(c) for c in pal], float)

    def fn(xs, ys):
        t = np.clip(f(xs, ys), 0, 0.999) * (len(cols) - 1)
        return cols[t.astype(int)]
    return fn


def lit(cx, cy, r):
    """Light from the top left over a round-ish shape."""
    return lambda xs, ys: 0.75 - ((xs - cx) + (ys - cy)) / (2.4 * r)


def sulfur():
    a = Art()
    pal = ["#6b5208", "#a88412", "#d9b51f", "#f2d740", "#fff38a"]
    # Three crystals, the middle one tallest; each lit on its left face.
    for (x0, top, x1, base, w) in ((9, 9, 14, 26, 5), (14, 4, 19, 26, 6), (19, 11, 24, 26, 5)):
        mid = (x0 + x1) / 2
        pts = [(x0, base), (x0, top + 4), (mid, top), (x1, top + 4), (x1, base)]
        a.poly(pts, ramp_fill(pal, lambda xs, ys, m=mid: np.where(xs < m, 0.82, 0.45) - (ys - 4) / 60), "#4a3604")
    a.poly([(6, 27), (26, 27), (24, 24), (8, 24)], ramp_fill(["#3b3530", "#57504a", "#6e6660"], lambda xs, ys: 0.6 - (ys - 24) / 6), "#201c18")
    for x, y in ((15, 7), (12, 13), (21, 15)):
        a.px(x, y, "#fffbd6")
    return a.image()


def scale():
    a = Art()
    pal = ["#5a1606", "#9c2e0c", "#d8541a", "#f48a2e", "#ffc46a"]
    pts = [(16, 4), (24, 9), (25, 18), (16, 28), (7, 18), (8, 9)]
    a.poly(pts, ramp_fill(pal, lit(16, 15, 11)), "#2c0a04")
    # Growth lines across the scale and a hot rim.
    for y in (12, 17, 22):
        for x in range(9, 24):
            if a.a[y, x] and (x + y) % 3:
                a.c[y, x] = a.c[y, x] * 0.72
    for x, y in ((12, 8), (11, 9), (10, 11)):
        a.px(x, y, "#ffe2a0")
    return a.image()


def cloak():
    a = Art()
    pal = ["#3b0e06", "#6a1c0a", "#9c3012", "#cc4f1c", "#f0813a"]
    a.poly([(11, 6), (21, 6), (27, 27), (16, 29), (5, 27)], ramp_fill(pal, lambda xs, ys: 0.7 - (xs - 8) / 40 - ((ys + xs) % 6 == 0) * 0.2), "#1e0603")
    a.poly([(9, 4), (23, 4), (22, 8), (10, 8)], ramp_fill(["#5a4a2a", "#9a8048", "#d8b860"], lambda xs, ys: 0.8 - (xs - 9) / 20), "#2a2210")
    a.disc(16, 6, 1.6, "#ffd27a", "#6e4a10")
    return a.image()


def idol():
    a = Art()
    obs = ["#0c0714", "#1a1028", "#2c1b44", "#45306a", "#6a5094"]
    a.poly([(8, 25), (24, 25), (26, 29), (6, 29)], ramp_fill(obs, lambda xs, ys: 0.55 - (xs - 6) / 50), "#05030a")
    a.poly([(11, 10), (21, 10), (22, 25), (10, 25)], ramp_fill(obs, lambda xs, ys: 0.8 - (xs - 10) / 16), "#05030a")
    a.poly([(10, 3), (22, 3), (23, 11), (9, 11)], ramp_fill(obs, lambda xs, ys: 0.85 - (xs - 9) / 18), "#05030a")
    for x in (13, 18):
        a.px(x, 7, "#ffb84a")
        a.px(x + 1, 7, "#ffe08a")
    for x in range(14, 19):
        a.px(x, 9, "#c44410")
    for y in range(14, 23, 3):
        for x in range(13, 20):
            if (x + y) % 2:
                a.px(x, y, "#e2611a" if y < 20 else "#7a1f06")
    return a.image()


def heart():
    a = Art()
    crust = ["#1e1010", "#33191a", "#4a2622", "#63342a"]
    ys, xs = np.mgrid[0:G, 0:G]
    x, y = (xs + 0.5 - 16) / 11, (ys + 0.5 - 15) / 11
    m = (x * x + (-(y) * 1.15 - np.sqrt(np.abs(x)) * 0.7 + 0.25) ** 2) < 1.0
    a.paint(m, ramp_fill(crust, lambda xs, ys: 0.8 - ((xs - 10) + (ys - 8)) / 36), "#0a0404")
    # Glowing cracks and a hot core showing through.
    glow = ["#b3360c", "#e2611a", "#f79a2e", "#ffd27a"]
    cracks = [(16, 9), (16, 12), (15, 14), (17, 16), (16, 19), (13, 12), (11, 11), (19, 13), (21, 12), (14, 18), (19, 18)]
    for k, (cx, cy) in enumerate(cracks):
        a.px(cx, cy, glow[(k * 3) % 4])
    a.disc(16.5, 15.5, 2.6, ramp_fill(glow, lambda xs, ys: 1.0 - np.hypot(xs - 16, ys - 15) / 4), "#5a1606")
    a.px(11, 8, "#8a5040")
    a.px(12, 7, "#8a5040")
    return a.image()


def hammer():
    a = Art()
    # The handle, from the bottom left up to the head.
    wood = ["#2a1a10", "#4a2e1a", "#6e4426", "#8e5a32"]
    a.poly([(4, 26), (7, 29), (21, 14), (18, 11)], ramp_fill(wood, lambda xs, ys: 0.7 - ((xs + ys) % 5 == 0) * 0.3), "#140c06")
    a.poly([(3, 27), (6, 30), (9, 27), (6, 24)], ramp_fill(["#5a5a64", "#8a8a96", "#b8b8c4"], lambda xs, ys: 0.7 - (xs - 3) / 10), "#24242c")
    # The head: a block of basalt turned 45 degrees, its striking face glowing.
    head = [(13, 9), (21, 1), (31, 11), (23, 19)]
    a.poly(head, ramp_fill(BASALT, lambda xs, ys: 0.9 - ((xs - 13) + (ys - 1)) / 34), "#0a090c")
    face = [(26, 6), (31, 11), (29, 13), (24, 8)]
    a.poly(face, ramp_fill(MAGMA, lambda xs, ys: 0.95 - (xs - 24) / 12), "#3a0e04")
    for x, y in ((17, 8), (19, 11), (21, 9), (18, 13), (22, 13)):
        a.px(x, y, "#e2611a")
    a.px(20, 4, "#8a8794")
    a.px(19, 5, "#8a8794")
    return a.image()


def egg(base, spot):
    a = Art()
    ys, xs = np.mgrid[0:G, 0:G]
    x, y = (xs + 0.5 - 16) / 9.5, (ys + 0.5 - 17) / 12.5
    m = x * x + y * y * (1 + 0.25 * y) < 1.0
    bc = np.array(hexrgb(base), float)
    a.paint(m, lambda xs, ys: np.clip(bc[None, None, :] * (1.2 - ((xs - 9) + (ys - 6)) / 46)[..., None], 0, 255), "#140c08")
    for k, (sx, sy) in enumerate(((12, 10), (19, 13), (14, 18), (20, 21), (11, 23), (17, 26), (21, 8))):
        for dx, dy in ((0, 0), (1, 0), (0, 1), (1, 1)):
            if a.a[sy + dy, sx + dx]:
                a.px(sx + dx, sy + dy, spot)
    return a.image()


ICONS = {
    "sulfur": sulfur,
    "salamander_scale": scale,
    "salamander_cloak": cloak,
    "obsidian_idol": idol,
    "magma_heart": heart,
    "volcanic_hammer": hammer,
}


# ------------------------------------------------------------------------------- blocks (64x64)


def blocks():
    import blocks as tb
    from style import clusters, light, quantize, ramp, rgba, rng_for, soften, worley, N, XX, YY

    def pumice():
        rng = rng_for("pumice")
        img = tb.lumpy("pumice_base", "#a49c8c", 20, 10, spread=0.3, kind="natural", tones=0.2)
        # Pores: round holes, dark inside with a lit lower-right lip.
        for _ in range(46):
            cx, cy = rng.random(2) * N
            r = rng.uniform(0.9, 2.4)
            dx, dy = tb.torus_d(cx, cy)
            dd = np.hypot(dx, dy)
            img[dd < r, :3] *= 0.55
            lip = (dd >= r) & (dd < r + 1.0) & (dx + dy > 0)
            img[lip, :3] = np.clip(img[lip, :3] * 1.12, 0, 255)
        return img

    def sulfur_ore():
        return tb.ore("sulfur", bg_fn=tb.blackstone, name="sulfur_ore", size=1.2, color="#e8c62a")

    def crust_ring(img, rng, r0, r1):
        """A yellow sulfur crust around the middle of a vent's top."""
        d = np.hypot(XX - 31.5, YY - 31.5) + clusters(rng, 3) * 5
        ring = (d > r0) & (d < r1)
        pal = ramp("#d9b51f", 7, 0.5, 0.1)
        t = 0.55 + (clusters(rng, 2) - 0.5) * 0.6
        img[ring, :3] = quantize(np.clip(t, 0, 1), pal, 0.3, rng)[ring]
        return img

    def fumarole_top():
        rng = rng_for("fumarole_top")
        img = tb.blackstone()
        img = crust_ring(img, rng, 9, 19)
        d = np.hypot(XX - 31.5, YY - 31.5)
        hole = d < 9
        img[hole, :3] = np.array([24, 18, 18]) + (9 - d[hole])[:, None] * np.array([7, 3, 1])
        img[(d >= 9) & (d < 10.5), :3] *= 0.6
        return img

    def fumarole_side():
        rng = rng_for("fumarole_side")
        img = tb.basalt_side()
        # Sulfur dripping down from the top edge, in streaks.
        pal = ramp("#d9b51f", 7, 0.5, 0.1)
        for x0 in rng.choice(np.arange(2, 62), 9, replace=False):
            ln = int(rng.integers(5, 20))
            w = int(rng.integers(2, 4))
            for y in range(ln):
                for x in range(x0, min(64, x0 + w - (y * w) // max(ln, 1))):
                    img[y, x, :3] = pal[int(np.clip(5 - y * 4 / ln + rng.uniform(-1, 1), 0, 6))]
        return img

    return {"pumice": pumice, "sulfur_ore": sulfur_ore, "fumarole_top": fumarole_top, "fumarole_side": fumarole_side}


# ------------------------------------------------------------------------------- data files


BLOCKS_RON = """// The volcanoes' blocks (crates/voxel/src/worldgen/volcano.rs). Made by tools/gen_vulcao.py.
(blocks: [
    (name: "pumice", display_name: Some("Pedra-Pomes"), hardness: 0.8, tool: Some(Pickaxe)),
    (name: "sulfur_ore", display_name: Some("Minério de Enxofre"), hardness: 2.5, tool: Some(Pickaxe), drops: Some([(item: "sulfur", min: 1, max: 3)])),
    (name: "fumarole", display_name: Some("Fumarola"), hardness: 1.6, tool: Some(Pickaxe), light: 4, textures: (top: Some("fumarole_top"), side: Some("fumarole_side"), bottom: Some("basalt_top"))),
])
"""

ITEMS = [
    ("sulfur", "Enxofre", 64),
    ("salamander_scale", "Escama de Salamandra", 64),
    ("salamander_cloak", "Capa de Salamandra", 1),
    ("obsidian_idol", "Ídolo de Obsidiana", 16),
    ("magma_heart", "Coração de Magma", 1),
]

RECIPES_RON = """// The volcanoes' recipes. Made by tools/gen_vulcao.py.
(
    shaped: [
        (pattern: ["SSS", "SCS", "SSS"], key: {'S': "salamander_scale", 'C': "leather"}, result: "salamander_cloak"),
        (pattern: [" O ", "OSO", "BBB"], key: {'O': "obsidian", 'S': "sulfur", 'B': "basalt"}, result: "obsidian_idol"),
    ],
    shapeless: [
        (ingredients: ["sulfur", "charcoal"], result: "gunpowder", count: 2),
        (ingredients: ["sulfur", "coal"], result: "gunpowder", count: 2),
        (ingredients: ["pumice", "pumice", "pumice", "pumice"], result: "gravel", count: 4),
    ],
)
"""


def main():
    os.makedirs(OUT, exist_ok=True)
    names = []
    text = "// The volcanoes' creatures (crates/game/src/vulcao.rs moves and spawns them). Made by\n// tools/gen_vulcao.py: edit it and run it again.\n(mobs: [\n"
    for m in MOBS:
        text += ron_mob(m)
        render(m["name"], m["model"]).save(os.path.join(ENTITY, m["name"] + ".png"))
    text += "])\n"
    open(os.path.join(OUT, "mobs.ron"), "w", encoding="utf-8").write(text)
    open(os.path.join(OUT, "blocks.ron"), "w", encoding="utf-8").write(BLOCKS_RON)
    open(os.path.join(OUT, "recipes.ron"), "w", encoding="utf-8").write(RECIPES_RON)
    items = "// The volcanoes' items. Made by tools/gen_vulcao.py.\n(items: [\n"
    for n, d, stack in ITEMS:
        items += f'    (name: "{n}", display_name: Some("{d}")' + (f", max_stack: Some({stack})" if stack != 64 else "") + "),\n"
    items += '    (name: "volcanic_hammer", display_name: Some("Martelo Vulcânico"), tool: Some((kind: Pickaxe, tier: 4, speed: 9.0, damage: 9.0, durability: 1800))),\n'
    for m in MOBS:
        if m["egg"]:
            items += f'    (name: "{m["name"]}_spawn_egg", display_name: Some("Ovo de {m["display"]}"), texture: Some("{m["name"]}_spawn_egg")),\n'
    items += "])\n"
    open(os.path.join(OUT, "items.ron"), "w", encoding="utf-8").write(items)
    for n, fn in ICONS.items():
        fn().save(os.path.join(ITEM_TEX, n + ".png"))
        names.append(n)
    for m in MOBS:
        if m["egg"]:
            egg(*m["egg"]).save(os.path.join(ITEM_TEX, m["name"] + "_spawn_egg.png"))
    from style import to_u8
    for n, fn in blocks().items():
        Image.fromarray(to_u8(fn()), "RGBA").save(os.path.join(BLOCK_TEX, n + ".png"))
        names.append(n)
    print("wrote", ", ".join(names), "and", len(MOBS), "creatures")
    if len(sys.argv) > 1:
        sheet = Image.new("RGBA", (64 * 10, 128 + 72), (40, 40, 44, 255))
        for k, m in enumerate(MOBS):
            sheet.paste(render(m["name"], m["model"]), (k * 64, 0))
        for k, n in enumerate(list(ICONS) + [m["name"] + "_spawn_egg" for m in MOBS if m["egg"]]):
            sheet.alpha_composite(Image.open(os.path.join(ITEM_TEX, n + ".png")).convert("RGBA"), (k * 64, 132))
        sheet.resize((sheet.width * 2, sheet.height * 2), Image.NEAREST).save(sys.argv[1])


if __name__ == "__main__":
    main()
