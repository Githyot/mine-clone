#!/usr/bin/env python3
"""16x16 designs for the deep world (packs/vanilla/deep): the Underworld, the glowing caves and
the movement gear. Each design is one of our own designs from src16/ with its shading mapped
onto a new colour ramp, so the shapes stay in the house style. build.py then turns them into
64x64 textures like every other design.

    python3 tools/tex64/deep16.py && python3 tools/tex64/build.py --only <names>
"""

import os

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
SRC = os.path.join(HERE, "src16")
PACK = os.path.join(ROOT, "packs", "vanilla", "textures")


def hexrgb(h):
    h = h.lstrip("#")
    return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], float)


def load(kind, name):
    return np.asarray(Image.open(os.path.join(SRC, kind, name + ".png")).convert("RGBA"), float)


def luma(img):
    return img[..., :3] @ np.array([0.299, 0.587, 0.114])


def chroma(img):
    return img[..., :3].max(-1) - img[..., :3].min(-1)


def gmap(img, ramp, mask=None, lo=None, hi=None):
    """Map the brightness of img onto a dark-to-light list of colours, keeping alpha."""
    y = luma(img)
    a = img[..., 3] > 0
    sel = a if mask is None else a & mask
    if not sel.any():
        return img
    lo = np.percentile(y[sel], 3) if lo is None else lo
    hi = np.percentile(y[sel], 97) if hi is None else hi
    t = np.clip((y - lo) / max(hi - lo, 1.0), 0, 1)
    cols = np.array([hexrgb(c) for c in ramp])
    f = t * (len(cols) - 1)
    i = np.minimum(f.astype(int), len(cols) - 2)
    w = (f - i)[..., None]
    rgb = cols[i] * (1 - w) + cols[i + 1] * w
    out = img.copy()
    out[..., :3] = np.where(sel[..., None], rgb, img[..., :3])
    return out


def sparks(img, n, col, seed, where=None):
    """A few glowing pixels, placed on the darkest spots so they read as cracks."""
    rng = np.random.default_rng(seed)
    y = luma(img) + rng.random((16, 16)) * 30
    ok = img[..., 3] > 0 if where is None else where
    y = np.where(ok, y, 1e9)
    out = img.copy()
    for k in np.argsort(y, axis=None)[:n]:
        out[k // 16, k % 16, :3] = hexrgb(col)
    return out


ASH = ["#2c2826", "#47403c", "#635953", "#837870"]
ASH_GRASS = ["#3b3236", "#574a4c", "#77666a", "#9a8a88"]
CHAR = ["#161211", "#2a2320", "#41352f", "#5a4a40"]
EMBER = ["#4a1306", "#8a2a0c", "#d0561a", "#f39a3a"]
FLAME = ["#7a2008", "#c84a12", "#f08a28", "#ffd27a"]
BASALT = ["#1d1c20", "#302e34", "#47444c", "#5e5a62"]
MAGMA = ["#6a1606", "#b8380e", "#ee7424", "#ffc060"]
OBSIDIAN = ["#0b0812", "#1b1428", "#2e2242", "#47365e"]
INFERNAL = ["#2a0a08", "#5a160e", "#8e2a16", "#c4502a"]
TEAL = ["#0b2c34", "#145a62", "#28a09c", "#6ae6d6"]
MUD = ["#2a2420", "#3e3530", "#56493f", "#6c5c4e"]
CYAN = ["#0a2a44", "#145c80", "#2aa6c8", "#9cf0ff"]


def ash_grass_side():
    src = load("block", "grass_block_side")
    green = (src[..., 1] > src[..., 0] + 8) & (src[..., 3] > 0)
    out = gmap(src, ASH, ~green)
    return gmap(out, ASH_GRASS, green)


def glow_moss_side():
    src = load("block", "grass_block_side")
    green = (src[..., 1] > src[..., 0] + 8) & (src[..., 3] > 0)
    out = gmap(src, MUD, ~green)
    return gmap(out, TEAL, green)


def ore(base, ramp_bg, ramp_ore):
    src = load("block", base)
    gem = chroma(src) > 40
    out = gmap(src, ramp_bg, ~gem)
    return gmap(out, ramp_ore, gem)


BLOCKS = {
    "ash_block": lambda: gmap(load("block", "gravel"), ASH),
    "ash_grass_top": lambda: sparks(gmap(load("block", "grass_block_top"), ASH_GRASS), 3, "#e8642a", 11),
    "ash_grass_side": ash_grass_side,
    "ember_bloom": lambda: gmap(load("block", "torchflower"), FLAME),
    "cinder_log": lambda: sparks(gmap(load("block", "oak_log"), CHAR), 6, "#d8501c", 5),
    "cinder_log_top": lambda: sparks(gmap(load("block", "oak_log_top"), CHAR), 3, "#e0601e", 6),
    "ember_leaves": lambda: gmap(load("block", "oak_leaves"), EMBER),
    "magmarite_ore": lambda: ore("diamond_ore", BASALT, MAGMA),
    "obsidian_bricks": lambda: gmap(load("block", "stone_bricks"), OBSIDIAN),
    "infernal_bricks": lambda: sparks(gmap(load("block", "nether_bricks"), INFERNAL), 4, "#f08030", 9),
    "glow_moss_top": lambda: gmap(load("block", "moss_block"), TEAL),
    "glow_moss_side": glow_moss_side,
    "glow_mushroom": lambda: gmap(load("block", "red_mushroom"), CYAN),
    "glow_mushroom_block": lambda: gmap(load("block", "red_mushroom_block"), CYAN),
}

ORANGE = ["#4a1406", "#9a340e", "#e06a22", "#ffb45a"]
DIAMOND = ["#0e3a44", "#1d7a80", "#46c8c2", "#b8fff4"]
WEB = ["#4a4a52", "#8a8a94", "#c8c8d0", "#f4f4f8"]
IVY = ["#123414", "#245e24", "#46963a", "#9ad66a"]
DUAL = ["#2a1240", "#52287a", "#8a52c0", "#d0a8f0"]
LEAF = ["#1c3418", "#2f5e2a", "#55964a", "#a6dc88"]
ICE = ["#284a6a", "#4a82aa", "#8ac4e6", "#e8f8ff"]
SAND = ["#6a4a24", "#a87c40", "#dcb26a", "#f8e6b0"]
CLOUD = ["#6a7482", "#a2acb8", "#d8e0e8", "#ffffff"]
ROCKET = ["#4a1010", "#8e2218", "#d24a2a", "#f8a070"]
COMET = ["#24143c", "#4c2c78", "#9a5ac8", "#f2c86a"]
WATER = ["#0e2850", "#1e4e8e", "#3a8ad0", "#a8dcff"]
STEEL = ["#26303c", "#46566a", "#7a8ea6", "#d0dcea"]
GEAR = ["#2a2218", "#4a3c2a", "#7a6446", "#c0a478"]

ITEMS = {
    "magmarite_ingot": lambda: gmap(load("item", "iron_ingot"), ORANGE),
    "magmarite_hook": lambda: gmap(load("item", "grappling_hook"), ORANGE),
    "diamond_hook": lambda: gmap(load("item", "grappling_hook"), DIAMOND),
    "web_hook": lambda: gmap(load("item", "grappling_hook"), WEB),
    "ivy_hook": lambda: gmap(load("item", "grappling_hook"), IVY),
    "dual_hook": lambda: gmap(load("item", "double_hook"), DUAL),
    "leap_boots": lambda: gmap(load("item", "running_boots"), LEAF),
    "blizzard_bottle": lambda: gmap(load("item", "cloud_bottle"), ICE),
    "sandstorm_bottle": lambda: gmap(load("item", "cloud_bottle"), SAND),
    "cloud_balloon": lambda: gmap(load("item", "shiny_balloon"), CLOUD),
    "rocket_boots": lambda: gmap(load("item", "steam_boots"), ROCKET),
    "comet_boots": lambda: gmap(load("item", "running_boots"), COMET),
    "water_walking_boots": lambda: gmap(load("item", "running_boots"), WATER),
    "charge_shield": lambda: gmap(load("item", "iron_buckler"), STEEL),
    "climbing_gear": lambda: gmap(load("item", "climbing_claws"), GEAR),
}
for _p in ("pickaxe", "axe", "sword", "helmet", "chestplate", "leggings", "boots"):
    ITEMS[f"magmarite_{_p}"] = lambda p=_p: gmap(load("item", f"diamond_{p}"), ORANGE)


def main():
    for kind, table in (("block", BLOCKS), ("item", ITEMS)):
        for name, f in table.items():
            img = np.clip(f(), 0, 255).astype(np.uint8)
            Image.fromarray(img, "RGBA").save(os.path.join(PACK, kind, name + ".png"))
    print(",".join(list(BLOCKS) + list(ITEMS)))


if __name__ == "__main__":
    main()
