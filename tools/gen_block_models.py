#!/usr/bin/env python3
"""3D models for blocks that used to be two crossed pictures (HP, 2026-10-29: "nosso jogo é 3D,
faça tudo parecido com a estrutura da carroça, na ideia de ser algo com volume").

Writes packs/vanilla/models/blocks.ron (the `models` list: boxes in pixels, 0..16, drawn
facing north, front toward -z) and the few material pictures the models need into
packs/vanilla/textures/block/ (mdl_*.png, rail_side_*.png). A block named in the list
becomes `Shape::Model` (crates/voxel/src/props.rs); its own picture stays its item icon.

Every face of a box shows the part of its texture the box covers, so a 1-pixel box shows a
4x4 patch of a 64x64 texture: the materials are painted in 4x4 cells, one tone each, like
chunky model pixels. Rails and ladders are built in Rust (props.rs) from their own pictures
plus rail_side_*.png.

Plants (flowers, mushrooms, saplings, crops...) are not here: they have their own models.

Usage: python3 tools/gen_block_models.py
"""

import math
import os
import zlib

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BLOCK_TEX = os.path.join(ROOT, "packs", "vanilla", "textures", "block")
OUT = os.path.join(ROOT, "packs", "vanilla", "models", "blocks.ron")
N = 64


def rng(name):
    return np.random.default_rng(zlib.crc32(name.encode()))


def hexc(h):
    h = h.lstrip("#")
    return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], dtype=np.float64)


# ---------------------------------------------------------------------------------------------
# Materials: 64x64, painted in 4x4 cells (one model pixel each) from a colour ramp.

def cells(name, ramp, spread=0.22, sheen=0.0, base=0.5, alpha=None):
    """A tileable picture of 16x16 cells, each one tone of `ramp` around `base` (0..1). `sheen`
    adds a bright top-left texel on some cells (metal, gems)."""
    r = rng(name)
    ramp = [hexc(c) for c in ramp]
    # Smooth-ish cell field: random, blurred once (wrapping) so tones come in small clusters.
    f = r.random((16, 16))
    f = (f + np.roll(f, 1, 0) + np.roll(f, 1, 1) + np.roll(f, -1, 0) + np.roll(f, -1, 1)) / 5.0
    f = (f - f.mean()) / (f.std() + 1e-9)
    img = np.zeros((N, N, 4), dtype=np.uint8)
    for cy in range(16):
        for cx in range(16):
            t = min(max(base + f[cy, cx] * spread * 0.5, 0.0), 1.0)
            k = t * (len(ramp) - 1)
            i = min(int(k), len(ramp) - 2)
            c = ramp[i] * (1 - (k - i)) + ramp[i + 1] * (k - i)
            # Inside a cell: the top-left texels a touch lighter, the bottom-right darker.
            for y in range(4):
                for x in range(4):
                    d = 1.0 + 0.05 * ((1.5 - x) + (1.5 - y)) / 3.0
                    px = np.clip(c * d, 0, 255)
                    if sheen and x == 0 and y == 0 and r.random() < sheen:
                        px = np.clip(c * 1.35 + 30, 0, 255)
                    img[cy * 4 + y, cx * 4 + x, :3] = px
                    img[cy * 4 + y, cx * 4 + x, 3] = 255
    if alpha is not None:
        img[..., 3] = alpha(r)
    return img


def save(name, img):
    Image.fromarray(img, "RGBA").save(os.path.join(BLOCK_TEX, name + ".png"))


def glass_alpha(r):
    """Glass: clear, with a few glints and a frosted streak, so a jar reads as glass."""
    a = np.zeros((N, N), dtype=np.uint8)
    for cy in range(16):
        for cx in range(16):
            if (cx + cy) % 11 in (0, 2) and (cx % 11) < 4:
                a[cy * 4:cy * 4 + 4, cx * 4:cx * 4 + 4] = 255
    return a


def materials():
    save("mdl_stone", cells("mdl_stone", ["#5e606b", "#6c6e79", "#7a7c87", "#888a95"], spread=0.25))
    save("mdl_stone_dark", cells("mdl_stone_dark", ["#2a2b33", "#33353e", "#3c3e48"], spread=0.25))
    save("mdl_gold", cells("mdl_gold", ["#8a5a12", "#b67d1c", "#dca52e", "#f2cb55", "#fff0a0"], spread=0.5, sheen=0.25, base=0.6))
    save("mdl_silver", cells("mdl_silver", ["#5c6370", "#7e8694", "#a3aab6", "#c8cdd6", "#f0f3f8"], spread=0.5, sheen=0.25, base=0.6))
    save("mdl_bronze", cells("mdl_bronze", ["#5a3112", "#7f4a1f", "#a6652c", "#c98744", "#e9b07a"], spread=0.5, sheen=0.25, base=0.6))
    save("mdl_iron_dark", cells("mdl_iron_dark", ["#1c1d22", "#2a2b31", "#383a41", "#4a4c55"], spread=0.4, sheen=0.08))
    save("mdl_stew", cells("mdl_stew", ["#5a2f12", "#7a4219", "#9a5a22", "#b8762f"], spread=0.6))
    save("mdl_water", cells("mdl_water", ["#1d4f9c", "#2a6cc0", "#3c8ae0", "#79b8f5", "#c6e6ff"], spread=0.7, base=0.45))
    save("mdl_glass", cells("mdl_glass", ["#9fc9d8", "#c3e3ee", "#e8f8ff"], spread=0.6, alpha=glass_alpha))
    save("mdl_glass_rim", cells("mdl_glass_rim", ["#7fb3c4", "#a8d4e0", "#d4f0f8"], spread=0.5, sheen=0.2, base=0.6))
    save("mdl_cork", cells("mdl_cork", ["#5e3a1d", "#76492a", "#8c5a35"], spread=0.5))
    save("mdl_firefly", cells("mdl_firefly", ["#5d8a16", "#a6d61e", "#e2ff5c", "#fbffc8"], spread=0.6, base=0.65))
    save("mdl_butterfly", cells("mdl_butterfly", ["#1f3aa8", "#3d63de", "#6f95f5", "#b9cfff"], spread=0.6, base=0.55))
    save("mdl_skin", cells("mdl_skin", ["#b97a5a", "#d39472", "#e9b08e", "#f6cdb0"], spread=0.3, base=0.55))
    save("mdl_beard", cells("mdl_beard", ["#b9b4ad", "#d6d2cb", "#eeebe6", "#ffffff"], spread=0.5, base=0.6))
    save("mdl_hat", cells("mdl_hat", ["#6c1010", "#8f1a17", "#b52a22", "#d64535"], spread=0.4))
    save("mdl_coat", cells("mdl_coat", ["#1b2f6e", "#253f8f", "#3354ad", "#4a6ec9"], spread=0.4))
    save("mdl_boot", cells("mdl_boot", ["#3a2312", "#4f3119", "#653f21"], spread=0.4))
    save("mdl_cloth_red", cells("mdl_cloth_red", ["#8c1414", "#b81f1f", "#de3a2f", "#f05c48"], spread=0.4))
    save("mdl_pole", cells("mdl_pole", ["#c9c9cf", "#e1e1e6", "#f5f5f8"], spread=0.4))
    save("mdl_tee", cells("mdl_tee", ["#8c6232", "#a87a43", "#c49556"], spread=0.4))
    save("mdl_monolith", cells("mdl_monolith", ["#0e0d14", "#17161f", "#211f2b", "#2c2a38"], spread=0.4, sheen=0.05))
    for name, ramp in {
        "solar": ["#7a2a05", "#d0560c", "#ff8a1e", "#ffd08a"],
        "vortex": ["#0c5a4a", "#18a184", "#38e0b8", "#b8fff0"],
        "nebula": ["#5c1270", "#a32ccb", "#e05cff", "#ffc8ff"],
        "stardust": ["#123a8c", "#2c68d8", "#5c9cff", "#d0e4ff"],
    }.items():
        save(f"mdl_glyph_{name}", cells(f"mdl_glyph_{name}", ramp, spread=0.5, base=0.6))
    save("mdl_ruby", cells("mdl_ruby", ["#5e0613", "#9c0f22", "#d42438", "#ff5b69", "#ffc2c8"], spread=0.6, sheen=0.3, base=0.55))
    save("mdl_amethyst", cells("mdl_amethyst", ["#3c1a6b", "#5a2c9a", "#8250c8", "#b58af0", "#ecd8ff"], spread=0.6, sheen=0.3, base=0.55))
    save("mdl_ice", cells("mdl_ice", ["#5f8fcf", "#86b2e6", "#b3d4f6", "#e4f2ff"], spread=0.6, sheen=0.2, base=0.6))
    save("mdl_dripstone", cells("mdl_dripstone", ["#5a4234", "#6f5444", "#856756", "#9c7c69"], spread=0.5))
    save("mdl_silk", cells("mdl_silk", ["#cfd2d6", "#e6e8eb", "#fafbfc"], spread=0.4, base=0.6))
    save("mdl_bark", cells("mdl_bark", ["#3d2814", "#563a1e", "#6e4c29", "#876137"], spread=0.5))
    save("mdl_pebble", cells("mdl_pebble", ["#5d5f66", "#74767e", "#8d8f97", "#a8aab1"], spread=0.5))
    save("mdl_blaze", cells("mdl_blaze", ["#a5620b", "#e09b16", "#ffd23c", "#fff2a0"], spread=0.5, base=0.6))
    for name, c in {"red": ["#7a0d1c", "#c21f37", "#ff5068"], "blue": ["#14307a", "#2a5ad0", "#5f90ff"], "green": ["#0f5c1c", "#1f9c35", "#4fdc65"]}.items():
        save(f"mdl_potion_{name}", cells(f"mdl_potion_{name}", c, spread=0.5))
    rail_sides()


def rail_sides():
    """Sides of rail bars (rows 0-1: the bar's metal, lit from above) and sleepers (row 2 the
    side, row 3 the top of a curve's sleepers), as props.rs reads them. Colours from the rails'
    own pictures."""
    for name, metal in {"rail_side_iron": ["#f0f0f4", "#bcbcc2", "#9a9aa0", "#76767c", "#5c5c62"], "rail_side_gold": ["#fff2a8", "#f6d863", "#d7a92c", "#b8860a", "#8a6200"]}.items():
        img = np.zeros((N, N, 4), dtype=np.uint8)
        img[..., 3] = 255
        rows = [hexc(c) for c in metal]
        for y in range(8):
            c = rows[min(y * len(rows) // 8, len(rows) - 1)]
            img[y, :, :3] = c
        # Bolt heads along the bar, every 16 texels.
        for x in range(6, N, 16):
            img[2:5, x:x + 3, :3] = np.clip(hexc(metal[0]) * 1.0, 0, 255)
            img[5, x:x + 3, :3] = hexc(metal[-1])
        wood = [hexc("#8f6439"), hexc("#7a5330"), hexc("#64431f"), hexc("#4e3216")]
        r = rng(name)
        for y in range(8, 16):
            for x in range(N):
                k = (y - 8) // 2 if y < 12 else min(int(r.random() * 2) + 1, 3)
                img[y, x, :3] = wood[min(k, 3)] * (0.94 + 0.12 * ((x // 4 + y) % 3) / 2)
        # The rest repeats rows 0-15 so nothing reads transparent.
        for y in range(16, N):
            img[y] = img[y % 16]
        save(name, img)


# ---------------------------------------------------------------------------------------------
# Models. B(x0, y0, z0, x1, y1, z1, tex, top=None, glow=False): a box in pixels, front at -z.

MODELS = {}


def B(x0, y0, z0, x1, y1, z1, tex, top=None, glow=False):
    return (min(x0, x1), min(y0, y1), min(z0, z1), max(x0, x1), max(y0, y1), max(z0, z1), tex, top, glow)


def model(name):
    def deco(f):
        boxes = f()
        for b in boxes:
            assert all(0 <= v <= 16 for v in b[:6]), (name, b)
            assert b[0] < b[3] and b[1] < b[4] and b[2] < b[5], (name, b)
        MODELS[name] = boxes
        return f
    return deco


ST, SD = "mdl_stone", "mdl_stone_dark"


def pedestal(h=2):
    """A statue's base: a dark slab with a lighter plinth on it."""
    return [B(1, 0, 1, 15, 1, 15, SD), B(2, 1, 2, 14, h, 14, ST)]


def row(y, z0, z1, spans, tex):
    """Boxes along x on one row: spans is a list of (x0, x1, tex or None for `tex`)."""
    return [B(a, y[0], z0, b, y[1], z1, t or tex) for a, b, t in spans]


@model("statue_skeleton")
def _():
    m = pedestal(2)
    # Feet and legs (bones 2 px thick), knees a touch wider.
    m += [B(4, 2, 5, 7, 3, 9, ST), B(9, 2, 5, 12, 3, 9, ST)]
    m += [B(5, 3, 6, 7, 6, 8, ST), B(9, 3, 6, 11, 6, 8, ST)]
    m += [B(4, 6, 6, 7, 7, 8, ST), B(9, 6, 6, 12, 7, 8, ST)]
    m += [B(5, 7, 6, 7, 8, 8, ST), B(9, 7, 6, 11, 8, 8, ST)]
    # Pelvis, spine and the ribcage with gaps between the ribs.
    m += [B(4, 8, 5, 12, 9, 9, ST), B(7, 9, 7, 9, 12, 9, ST)]
    for y in (9, 10):
        if y == 9:
            m += [B(4, y, 5, 12, y + 1, 6, ST), B(4, y, 5, 5, y + 1, 9, ST), B(11, y, 5, 12, y + 1, 9, ST)]
    m += [B(5, 10, 6, 11, 11, 8, SD), B(4, 10, 5, 12, 11, 6, ST)]
    # Shoulders, upper arms hanging, forearms reaching forward.
    m += [B(3, 11, 6, 13, 12, 8, ST)]
    m += [B(2, 8, 6, 4, 11, 8, ST), B(12, 8, 6, 14, 11, 8, ST)]
    m += [B(2, 7, 3, 4, 8, 8, ST), B(12, 7, 3, 14, 8, 8, ST)]
    # Skull: the back, then a face plate with dark eye sockets, a nose hole and teeth.
    m += [B(4, 12, 5, 12, 16, 11, ST)]
    m += [B(4, 15, 4, 12, 16, 5, ST)]
    m += row((13, 15), 4, 5, [(4, 5, None), (5, 7, SD), (7, 9, None), (9, 11, SD), (11, 12, None)], ST)
    m += [B(5, 12, 4, 6, 13, 5, ST), B(6, 12, 4, 7, 13, 5, SD), B(7, 12, 4, 8, 13, 5, ST), B(8, 12, 4, 9, 13, 5, SD), B(9, 12, 4, 10, 13, 5, ST), B(10, 12, 4, 11, 13, 5, SD)]
    return m


@model("statue_rabbit")
def _():
    m = pedestal(2)
    # Sitting: big haunches at the back, a round body, front paws, head up front, tall ears.
    m += [B(4, 2, 7, 12, 6, 13, ST), B(5, 6, 8, 11, 8, 13, ST), B(6, 8, 9, 10, 9, 12, ST)]
    m += [B(3, 2, 9, 5, 4, 14, ST), B(11, 2, 9, 13, 4, 14, ST)]  # hind feet
    m += [B(5, 2, 4, 7, 4, 7, ST), B(9, 2, 4, 11, 4, 7, ST)]  # front paws
    m += [B(5, 4, 5, 11, 8, 9, ST)]  # chest
    m += [B(5, 8, 3, 11, 12, 8, ST), B(6, 7, 2, 10, 9, 3, ST)]  # head and muzzle
    m += [B(6, 10, 2, 7, 11, 3, SD), B(9, 10, 2, 10, 11, 3, SD)]  # eyes
    m += [B(7, 8, 1, 9, 9, 2, SD)]  # nose
    m += [B(5, 12, 5, 7, 16, 7, ST), B(9, 12, 5, 11, 16, 7, ST)]  # ears
    m += [B(6, 12, 4, 7, 15, 5, SD), B(9, 12, 4, 10, 15, 5, SD)]  # inner ears
    m += [B(7, 4, 13, 9, 6, 15, ST)]  # tail
    return m


@model("statue_butterfly")
def _():
    m = pedestal(2)
    # A post holding the butterfly up, its body along z, four wings spread in a shallow V.
    m += [B(7, 2, 7, 9, 7, 9, SD), B(6, 7, 6, 10, 8, 10, ST)]
    m += [B(7, 8, 4, 9, 10, 13, ST), B(7, 10, 3, 9, 12, 6, ST)]  # body, head
    m += [B(6, 12, 2, 7, 15, 3, SD), B(9, 12, 2, 10, 15, 3, SD)]  # antennae
    for side in (-1, 1):
        def X(a, b):
            return (8 + side * a, 8 + side * b)
        x0, x1 = X(1, 4)
        m += [B(x0, 9, 4, x1, 10, 9, ST)]  # wing roots
        x0, x1 = X(4, 7)
        m += [B(x0, 10, 3, x1, 11, 9, ST)]
        x0, x1 = X(1, 7)
        m += [B(x0, 11, 3, x1, 12, 8, ST)]  # upper wing rising
        x0, x1 = X(3, 8)
        m += [B(x0, 12, 2, x1, 13, 7, ST), B(*X(5, 8)[:1], 13, 2, *X(5, 8)[1:], 14, 6, ST)]
        x0, x1 = X(2, 6)
        m += [B(x0, 8, 9, x1, 9, 13, ST), B(*X(3, 6)[:1], 7, 10, *X(3, 6)[1:], 8, 13, ST)]  # lower wing
        x0, x1 = X(4, 6)
        m += [B(x0, 11, 3, x1, 12, 4, SD)]  # wing spot
    return m


@model("statue_bat")
def _():
    m = pedestal(2)
    m += [B(7, 2, 7, 9, 7, 9, SD), B(6, 7, 6, 10, 8, 10, ST)]
    # Body, head with ears, wings spread wide with scalloped lower edges.
    m += [B(6, 8, 6, 10, 12, 10, ST), B(6, 12, 5, 10, 15, 9, ST)]
    m += [B(6, 15, 6, 7, 16, 8, ST), B(9, 15, 6, 10, 16, 8, ST)]
    m += [B(6, 13, 4, 7, 14, 5, SD), B(9, 13, 4, 10, 14, 5, SD)]
    m += [B(7, 12, 4, 9, 13, 5, ST)]
    for side in (-1, 1):
        def X(a, b):
            return (8 + side * a, 8 + side * b)
        m += [B(*X(2, 8)[:1], 11, 7, *X(2, 8)[1:], 12, 9, ST)]  # arm bone across the top
        m += [B(*X(2, 4)[:1], 8, 7, *X(2, 4)[1:], 11, 8, ST)]
        m += [B(*X(4, 6)[:1], 9, 7, *X(4, 6)[1:], 11, 8, ST)]
        m += [B(*X(6, 8)[:1], 8, 7, *X(6, 8)[1:], 11, 8, ST)]
        m += [B(*X(7, 8)[:1], 12, 7, *X(7, 8)[1:], 13, 8, ST)]  # wing tip claw
    return m


@model("statue_jelly")
def _():
    m = pedestal(2)
    # A round jelly (slime) blob: wide at the bottom, domed, with eyes and a mouth.
    m += [B(3, 2, 3, 13, 5, 13, ST), B(2, 3, 4, 14, 5, 12, ST), B(4, 3, 2, 12, 5, 14, ST)]
    m += [B(3, 5, 3, 13, 8, 13, ST), B(4, 8, 4, 12, 10, 12, ST), B(5, 10, 5, 11, 11, 11, ST), B(6, 11, 6, 10, 12, 10, ST)]
    m += [B(5, 6, 2, 7, 8, 3, SD), B(9, 6, 2, 11, 8, 3, SD), B(7, 4, 2, 9, 5, 3, SD)]
    m += [B(11, 7, 3, 12, 9, 4, ST)]  # a shine bump
    return m


def trophy(metal):
    m = [B(3, 0, 3, 13, 2, 13, "dark_oak_planks"), B(4, 2, 4, 12, 3, 12, "dark_oak_planks")]
    m += [B(7, 3, 7, 9, 6, 9, metal), B(6, 6, 6, 10, 7, 10, metal)]  # stem and knot
    # Cup: a bowl widening upward, hollow on top (four walls round the rim).
    m += [B(5, 7, 5, 11, 9, 11, metal), B(4, 9, 4, 12, 10, 12, metal)]
    m += [B(3, 10, 3, 13, 14, 4, metal), B(3, 10, 12, 13, 14, 13, metal), B(3, 10, 4, 4, 14, 12, metal), B(12, 10, 4, 13, 14, 12, metal)]
    m += [B(4, 10, 4, 12, 11, 12, metal)]  # bottom inside the cup
    m += [B(1, 11, 7, 3, 12, 9, metal), B(1, 12, 7, 2, 14, 9, metal), B(13, 11, 7, 15, 12, 9, metal), B(14, 12, 7, 15, 14, 9, metal)]  # handles
    return m


for _metal, _name in (("mdl_bronze", "golf_trophy_bronze"), ("mdl_silver", "golf_trophy_silver"), ("mdl_gold", "golf_trophy_gold")):
    MODELS[_name] = trophy(_metal)


@model("golf_tee")
def _():
    return [B(7, 0, 7, 9, 3, 9, "mdl_tee"), B(6, 3, 6, 10, 4, 10, "mdl_tee"), B(7, 4, 7, 9, 5, 9, "mdl_beard")]


@model("golf_flag")
def _():
    # A pole in a cup, a red pennant waving a little (two slabs a pixel apart).
    m = [B(6, 0, 6, 10, 1, 10, "mdl_iron_dark"), B(7, 0, 7, 9, 16, 9, "mdl_pole")]
    m += [B(9, 10, 8, 13, 15, 9, "mdl_cloth_red"), B(13, 11, 7, 15, 14, 8, "mdl_cloth_red"), B(15, 12, 7, 16, 13, 8, "mdl_cloth_red")]
    return m


@model("cooking_pot")
def _():
    m = [B(3, 0, 3, 5, 2, 5, "mdl_iron_dark"), B(11, 0, 3, 13, 2, 5, "mdl_iron_dark"), B(3, 0, 11, 5, 2, 13, "mdl_iron_dark"), B(11, 0, 11, 13, 2, 13, "mdl_iron_dark")]
    m += [B(3, 2, 3, 13, 3, 13, "mdl_iron_dark")]
    # Pot walls round a stew, a rim lip, side handles.
    m += [B(2, 3, 2, 14, 11, 3, "mdl_iron_dark"), B(2, 3, 13, 14, 11, 14, "mdl_iron_dark"), B(2, 3, 3, 3, 11, 13, "mdl_iron_dark"), B(13, 3, 3, 14, 11, 13, "mdl_iron_dark")]
    m += [B(1, 11, 1, 15, 12, 2, "mdl_iron_dark"), B(1, 11, 14, 15, 12, 15, "mdl_iron_dark"), B(1, 11, 2, 2, 12, 14, "mdl_iron_dark"), B(14, 11, 2, 15, 12, 14, "mdl_iron_dark")]
    m += [B(3, 3, 3, 13, 9, 13, "mdl_stew")]
    m += [B(0, 8, 7, 2, 9, 9, "mdl_iron_dark"), B(14, 8, 7, 16, 9, 9, "mdl_iron_dark")]
    return m


def jar(inside):
    m = [B(3, 1, 3, 13, 11, 4, "mdl_glass"), B(3, 1, 12, 13, 11, 13, "mdl_glass"), B(3, 1, 4, 4, 11, 12, "mdl_glass"), B(12, 1, 4, 13, 11, 12, "mdl_glass")]
    # Thick glass at the corners, the base and the shoulder catch the light; a cork on top.
    m += [B(x, 1, z, x + 1, 11, z + 1, "mdl_glass_rim") for x in (3, 12) for z in (3, 12)]
    m += [B(3, 0, 3, 13, 1, 13, "mdl_glass_rim"), B(3, 10, 3, 13, 11, 13, "mdl_glass_rim"), B(4, 11, 4, 12, 12, 12, "mdl_glass_rim"), B(5, 12, 5, 11, 14, 11, "mdl_cork")]
    return m + inside


MODELS["firefly_jar"] = jar([B(6, 4, 6, 10, 8, 10, "mdl_firefly", glow=True), B(7, 8, 7, 9, 9, 9, "mdl_firefly", glow=True), B(5, 5, 7, 6, 7, 9, "mdl_firefly", glow=True)])
MODELS["butterfly_jar"] = jar([B(7, 3, 7, 9, 8, 9, "mdl_butterfly"), B(4, 5, 6, 7, 9, 10, "mdl_butterfly"), B(9, 5, 6, 12, 9, 10, "mdl_butterfly"), B(5, 3, 7, 7, 5, 9, "mdl_butterfly"), B(9, 3, 7, 11, 5, 9, "mdl_butterfly")])


def fountain(stone, trim):
    # A basin with water, a column, a top bowl spilling water down its sides.
    m = [B(0, 0, 0, 16, 1, 16, stone), B(0, 1, 0, 16, 4, 1, stone), B(0, 1, 15, 16, 4, 16, stone), B(0, 1, 1, 1, 4, 15, stone), B(15, 1, 1, 16, 4, 15, stone)]
    m += [B(1, 1, 1, 15, 3, 15, "mdl_water")]
    m += [B(6, 3, 6, 10, 11, 10, trim), B(5, 3, 5, 11, 4, 11, trim)]
    m += [B(3, 11, 3, 13, 12, 13, trim), B(3, 12, 3, 13, 14, 4, trim), B(3, 12, 12, 13, 14, 13, trim), B(3, 12, 4, 4, 14, 12, trim), B(12, 12, 4, 13, 14, 12, trim)]
    m += [B(4, 12, 4, 12, 13, 12, "mdl_water"), B(7, 13, 7, 9, 16, 9, "mdl_water")]
    # Falling streams from the bowl's lip into the basin.
    m += [B(7, 3, 2, 9, 11, 3, "mdl_water"), B(7, 3, 13, 9, 11, 14, "mdl_water"), B(2, 3, 7, 3, 11, 9, "mdl_water"), B(13, 3, 7, 14, 11, 9, "mdl_water")]
    return m


for _name, _stone, _trim in (
    ("desert_fountain", "smooth_sandstone", "sandstone_top"),
    ("snow_fountain", "packed_ice", "snow"),
    ("jungle_fountain", "mossy_cobblestone", "moss_block"),
    ("ocean_fountain", "prismarine", "dark_prismarine"),
    ("evil_fountain", "purpur_block", "obsidian"),
):
    MODELS[_name] = fountain(_stone, _trim)


def monolith(glyph):
    g = f"mdl_glyph_{glyph}"
    m = [B(3, 0, 3, 13, 2, 13, "mdl_monolith"), B(4, 2, 4, 12, 13, 12, "mdl_monolith"), B(5, 13, 5, 11, 15, 11, "mdl_monolith"), B(7, 15, 7, 9, 16, 9, g, glow=True)]
    # Glowing glyphs set into all four faces.
    for y in (4, 7, 10):
        m += [B(6, y, 3, 10, y + 2, 4, g, glow=True), B(6, y, 12, 10, y + 2, 13, g, glow=True), B(3, y, 6, 4, y + 2, 10, g, glow=True), B(12, y, 6, 13, y + 2, 10, g, glow=True)]
    return m


for _g in ("solar", "vortex", "nebula", "stardust"):
    MODELS[f"{_g}_monolith"] = monolith(_g)


@model("garden_gnome")
def _():
    m = [B(4, 0, 5, 7, 2, 10, "mdl_boot"), B(9, 0, 5, 12, 2, 10, "mdl_boot")]
    m += [B(4, 2, 5, 12, 8, 11, "mdl_coat"), B(3, 6, 6, 4, 8, 10, "mdl_coat"), B(12, 6, 6, 13, 8, 10, "mdl_coat")]
    m += [B(3, 4, 6, 4, 6, 8, "mdl_skin"), B(12, 4, 6, 13, 6, 8, "mdl_skin")]  # hands
    m += [B(5, 4, 4, 11, 9, 5, "mdl_beard"), B(6, 3, 4, 10, 4, 5, "mdl_beard")]  # beard down the chest
    m += [B(5, 8, 5, 11, 11, 10, "mdl_skin"), B(7, 9, 4, 9, 10, 5, "mdl_skin")]  # head and nose
    m += [B(6, 10, 4, 7, 11, 5, "mdl_monolith"), B(9, 10, 4, 10, 11, 5, "mdl_monolith")]  # eyes
    m += [B(4, 11, 4, 12, 12, 11, "mdl_hat"), B(5, 12, 5, 11, 13, 10, "mdl_hat"), B(6, 13, 6, 10, 14, 9, "mdl_hat"), B(7, 14, 7, 9, 15, 9, "mdl_hat"), B(8, 15, 8, 9, 16, 10, "mdl_hat")]
    return m


@model("brewing_stand")
def _():
    m = [B(1, 0, 1, 15, 1, 15, "polished_blackstone"), B(7, 1, 7, 9, 14, 9, "mdl_blaze", glow=True), B(6, 14, 6, 10, 15, 10, "mdl_blaze", glow=True)]
    # Three arms holding bottles: glass with coloured potion inside.
    for (x, z, c) in ((2, 3, "red"), (11, 3, "blue"), (6, 11, "green")):
        m += [B(x, 1, z, x + 3, 5, z + 3, f"mdl_potion_{c}"), B(x + 1, 5, z + 1, x + 2, 7, z + 2, "mdl_glass")]
    m += [B(4, 10, 4, 8, 11, 5, "mdl_iron_dark"), B(8, 10, 4, 12, 11, 5, "mdl_iron_dark"), B(7, 10, 9, 9, 11, 12, "mdl_iron_dark")]
    m += [B(3, 7, 4, 4, 10, 5, "mdl_iron_dark"), B(12, 7, 4, 13, 10, 5, "mdl_iron_dark"), B(7, 7, 12, 9, 10, 13, "mdl_iron_dark")]
    return m


@model("dungeon_spikes")
def _():
    m = [B(0, 0, 0, 16, 2, 16, "mdl_stone_dark")]
    for x in (1, 6, 11):
        for z in (1, 6, 11):
            m += [B(x, 2, z, x + 4, 4, z + 4, "mdl_silver"), B(x + 1, 4, z + 1, x + 3, 7, z + 3, "mdl_silver"), B(x + 1, 7, z + 1, x + 2, 9, z + 2, "mdl_pole")]
    return m


def line3(a, b):
    """Integer cells on the line from a to b (3D)."""
    n = int(max(abs(b[i] - a[i]) for i in range(3)))
    out = []
    for k in range(n + 1):
        t = k / max(n, 1)
        c = tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))
        if c not in out:
            out.append(c)
    return out


@model("cobweb")
def _():
    # Strands from a knot in the middle out to the corners and edges, a pixel thick, and a ring.
    cells = set()
    c = (7.5, 7.5, 7.5)
    ends = [(x, y, z) for x in (0, 15) for y in (0, 15) for z in (0, 15)] + [(7, 15, 0), (0, 7, 15), (15, 0, 7)]
    for e in ends:
        cells.update(line3(c, e))
    for k in range(16):
        a = 2 * math.pi * k / 16
        cells.add((int(7.5 + 5 * math.cos(a)), int(7.5 + 5 * math.sin(a)), 7))
    return [B(x, y, z, x + 1, y + 1, z + 1, "mdl_silk") for (x, y, z) in sorted(cells) if 0 <= min(x, y, z) and max(x, y, z) <= 15]


def crystal(x, z, w, h, tex):
    """A crystal: a square prism of `w` with a pointed top, `h` high."""
    m = [B(x, 0, z, x + w, h - w // 2 - 1, z + w, tex, glow=True)]
    y = h - w // 2 - 1
    s = w
    while s > 1 and y < 16:
        s = max(s - 2, 1) if s > 2 else 1
        o = (w - s) / 2
        m.append(B(int(x + o), y, int(z + o), int(x + o) + s, y + 1, int(z + o) + s, tex, glow=True))
        y += 1
    return m


@model("life_crystal")
def _():
    m = [B(2, 0, 3, 14, 1, 13, "mdl_stone_dark"), B(3, 1, 4, 13, 2, 12, "mdl_stone_dark")]
    for (x, z, w, h) in ((6, 6, 4, 15), (3, 5, 3, 10), (10, 4, 3, 12), (4, 9, 2, 8), (10, 9, 3, 9)):
        m += [(b[0], b[1] + 2, b[2], b[3], min(b[4] + 2, 16), b[5], b[6], b[7], b[8]) for b in crystal(x, z, w, h - 1, "mdl_ruby")]
    return m


@model("amethyst_cluster")
def _():
    m = []
    for (x, z, w, h) in ((6, 6, 4, 15), (2, 3, 3, 9), (11, 3, 3, 11), (3, 11, 3, 8), (10, 10, 4, 12), (7, 2, 2, 6), (7, 12, 2, 7)):
        m += crystal(x, z, w, h, "mdl_amethyst")
    return m


def spike(x, z, w, h, tex, down=False, glow=False):
    """A tapering spike from the ground up (or from the ceiling down)."""
    m = []
    y = 0
    s = w
    while y < h and s >= 1:
        step = max(1, h // (w // 1 + 1))
        y1 = min(y + step, h)
        o = (w - s) // 2
        a, b = (16 - y1, 16 - y) if down else (y, y1)
        m.append(B(x + o, a, z + o, x + o + s, b, z + o + s, tex, glow=glow))
        y = y1
        s -= 1 if s > 2 else 1
        if s == 0:
            break
    return m


@model("pointed_dripstone")
def _():
    return spike(5, 5, 6, 16, "mdl_dripstone") + spike(2, 10, 3, 6, "mdl_dripstone") + spike(11, 3, 2, 4, "mdl_dripstone")


@model("pointed_dripstone_down")
def _():
    return spike(5, 5, 6, 16, "mdl_dripstone", down=True) + spike(10, 10, 3, 7, "mdl_dripstone", down=True) + spike(3, 2, 2, 4, "mdl_dripstone", down=True)


@model("icicle")
def _():
    m = [B(1, 15, 1, 15, 16, 15, "mdl_ice")]
    for (x, z, w, h) in ((6, 6, 4, 15), (2, 3, 3, 9), (11, 2, 3, 11), (3, 11, 2, 6), (10, 10, 3, 12)):
        m += spike(x, z, w, h, "mdl_ice", down=True)
    return m


@model("twig")
def _():
    # Two sticks crossing on the ground, a small fork on one.
    return [B(2, 0, 7, 14, 1, 9, "mdl_bark"), B(5, 0, 3, 7, 1, 7, "mdl_bark"), B(7, 0, 9, 9, 1, 13, "mdl_bark"), B(12, 0, 5, 13, 1, 7, "mdl_bark"), B(10, 1, 7, 12, 2, 9, "mdl_bark")]


@model("pebble")
def _():
    return [B(3, 0, 4, 7, 2, 8, "mdl_pebble"), B(4, 2, 5, 6, 3, 7, "mdl_pebble"), B(9, 0, 9, 12, 1, 12, "mdl_pebble"), B(11, 0, 4, 13, 1, 6, "mdl_pebble")]


# ---------------------------------------------------------------------------------------------

def fmt(v):
    return str(int(v)) + ".0"


def write():
    lines = ["// 3D models for blocks (tools/gen_block_models.py). Boxes in pixels, drawn facing north.", "(models: ["]
    for name in sorted(MODELS):
        boxes = []
        for (x0, y0, z0, x1, y1, z1, tex, top, glow) in MODELS[name]:
            extra = (f", top: Some(\"{top}\")" if top else "") + (", glow: true" if glow else "")
            boxes.append(f"(from: ({fmt(x0)}, {fmt(y0)}, {fmt(z0)}), to: ({fmt(x1)}, {fmt(y1)}, {fmt(z1)}), tex: \"{tex}\"{extra})")
        lines.append(f"    (block: \"{name}\", boxes: [")
        lines += [f"        {b}," for b in boxes]
        lines.append("    ]),")
    lines.append("])")
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"{len(MODELS)} models, {sum(len(b) for b in MODELS.values())} boxes -> {OUT}")


if __name__ == "__main__":
    materials()
    write()
