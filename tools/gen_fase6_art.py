#!/usr/bin/env python3
"""Original 16x16 art for Phase 6: block textures (work blocks, sculk, deepslate, tuff, trial
spawners and vaults, archaeology), item icons (every potion in its four forms, smithing
templates, pottery sherds, netherite gear, crossbow, trident, mace, spears...) and the skins of
the warden, the breeze and the sniffer (64x128, the lower half holding the 3D layers).

Everything is drawn here from noise, small pixel maps and the helpers of gen_textures.py and
gen_items.py; nothing is copied from anywhere.

Usage:
    python3 tools/gen_fase6_art.py [sheet.png]
"""

import os
import re
import sys
import zlib

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_textures import (  # noqa: E402
    N, ROOT, XX, YY, bevel, canvas, contact_sheet, fbm, from_rgb, hexrgb, line, normalize, pal, put, rng_for, scale_rgb, shade,
    to_image, value_noise, voronoi,
)
import gen_items as gi  # noqa: E402
from gen_items import from_map, mask_from_rows, outline, pmap, shade_mask  # noqa: E402

BLOCK_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "block")
ITEM_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "item")
ENTITY_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "entity")

# ---------------------------------------------------------------------------
# Palettes
# ---------------------------------------------------------------------------

DEEPSLATE = pal("#2a2a30", "#34343b", "#3e3e46", "#484851", "#53535c", "#60606a")
TUFF = pal("#4e5049", "#5b5d55", "#686a61", "#75776d", "#82847a", "#909287")
SCULK = pal("#04131a", "#072029", "#0b2d38", "#0f3b48", "#155061")
SCULK_GLOW = pal("#1a7f8f", "#29a9b5", "#5ce0e6", "#a8f6f6")
PRISM = pal("#3e6b62", "#4f8678", "#62a08f", "#79b4a2", "#93c8b8")
OBSIDIAN = pal("#0c0812", "#140e1e", "#1c142a", "#261c38", "#342a4a")
IRON = pal("#3c3c3c", "#4f4f4f", "#626262", "#767676", "#8c8c8c", "#a4a4a4")
COPPER = pal("#6a2e14", "#8e4122", "#b0582e", "#c87040", "#de8f5c")
NETHERITE = pal("#231d1f", "#2f2729", "#3c3335", "#4a4042", "#5a4f51", "#6b6062")
SAND = pal("#c9b47a", "#d6c38c", "#e0cd98", "#e8d8a6", "#efe1b4")
GRAVEL = pal("#5c5654", "#716a67", "#857e7a", "#99928d", "#aaa39e")
PLANK = pal("#4a3420", "#5e432a", "#735435", "#876440", "#9b754c")


def noise_tex(name, p, octaves=((8, 0.3), (4, 0.35), (2, 0.3), (1, 0.2)), strength=0.6):
    rng = rng_for(name)
    return from_rgb(shade(fbm(rng, octaves), p, strength))


def speckle(img, rng, colors, n):
    for _ in range(n):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, colors[rng.integers(len(colors))])


def brick_tex(name, p, bw=8, bh=4, mortar=None, offset=4):
    img = noise_tex(name, p[1:], ((4, 0.4), (2, 0.4), (1, 0.3)), 0.5)
    rgb = img[..., :3]
    m = p[0] if mortar is None else mortar
    for row in range(N // bh):
        y = row * bh + bh - 1
        rgb[y, :] = m
        shift = (row % 2) * offset
        for x in range(shift, N + shift, bw):
            rgb[row * bh:row * bh + bh - 1, x % N] = m
        rgb[row * bh, :] = scale_rgb(rgb[row * bh, :], 1.1)
    return img


def tiles_tex(name, p, cell=4):
    img = noise_tex(name, p[1:], ((4, 0.4), (2, 0.4), (1, 0.3)), 0.5)
    rgb = img[..., :3]
    for k in range(0, N, cell):
        rgb[k + cell - 1, :] = p[0]
        rgb[:, k + cell - 1] = p[0]
        rgb[k, :] = scale_rgb(rgb[k, :], 1.08)
    return img


def polished_tex(name, p):
    img = noise_tex(name, p[2:5], ((8, 0.4), (4, 0.3), (2, 0.2)), 0.35)
    bevel(img, 1.15, 0.7)
    return img


def chiseled_tex(name, p, emblem):
    img = polished_tex(name, p)
    rgb = img[..., :3]
    rgb[2:14, 2] = p[0]
    rgb[2:14, 13] = p[0]
    rgb[2, 2:14] = p[0]
    rgb[13, 2:14] = p[0]
    for (x, y) in emblem:
        put(img, x, y, p[0])
        put(img, x + 1, y + 1, p[-1])
    return img


# ---------------------------------------------------------------------------
# Blocks
# ---------------------------------------------------------------------------

BLOCKS = {}


def blockfn(name):
    def deco(fn):
        BLOCKS[name] = fn
        return fn
    return deco


def register_simple():
    BLOCKS["deepslate"] = lambda: _deepslate_side()
    BLOCKS["deepslate_top"] = lambda: noise_tex("deepslate_top", DEEPSLATE, ((8, 0.3), (4, 0.3), (2, 0.3), (1, 0.3)))
    BLOCKS["cobbled_deepslate"] = lambda: _cobbled(DEEPSLATE, "cobbled_deepslate")
    BLOCKS["polished_deepslate"] = lambda: polished_tex("polished_deepslate", DEEPSLATE)
    BLOCKS["deepslate_bricks"] = lambda: brick_tex("deepslate_bricks", DEEPSLATE, 8, 4, pal("#1c1c21")[0])
    BLOCKS["cracked_deepslate_bricks"] = lambda: _cracked(brick_tex("cracked_deepslate_bricks", DEEPSLATE, 8, 4, pal("#1c1c21")[0]), "cracked_deepslate_bricks")
    BLOCKS["deepslate_tiles"] = lambda: tiles_tex("deepslate_tiles", DEEPSLATE, 4)
    BLOCKS["chiseled_deepslate"] = lambda: chiseled_tex("chiseled_deepslate", DEEPSLATE, [(7, 5), (6, 6), (8, 6), (5, 7), (9, 7), (6, 8), (8, 8), (7, 9)])
    BLOCKS["tuff"] = lambda: _tuff("tuff")
    BLOCKS["polished_tuff"] = lambda: polished_tex("polished_tuff", TUFF)
    BLOCKS["tuff_bricks"] = lambda: brick_tex("tuff_bricks", TUFF, 8, 4, pal("#3b3d37")[0])
    BLOCKS["chiseled_tuff"] = lambda: _chiseled_tuff("chiseled_tuff", False)
    BLOCKS["chiseled_tuff_top"] = lambda: polished_tex("chiseled_tuff_top", TUFF)
    BLOCKS["chiseled_tuff_bricks"] = lambda: _chiseled_tuff("chiseled_tuff_bricks", True)
    BLOCKS["chiseled_tuff_bricks_top"] = lambda: tiles_tex("chiseled_tuff_bricks_top", TUFF, 8)
    BLOCKS["prismarine"] = lambda: _prismarine()
    BLOCKS["prismarine_bricks"] = lambda: brick_tex("prismarine_bricks", pal("#38675a", "#5aa08d", "#6cb39f", "#7ec0ad", "#94d0bf"), 8, 4)
    BLOCKS["dark_prismarine"] = lambda: tiles_tex("dark_prismarine", pal("#16302a", "#224a3f", "#2c5a4d", "#356a5a", "#3f7a68"), 8)
    BLOCKS["netherite_block"] = lambda: _metal_block("netherite_block", NETHERITE)
    BLOCKS["emerald_block"] = lambda: _metal_block("emerald_block", pal("#0a5a2a", "#12793a", "#1a9a4a", "#2ab85e", "#52d880", "#9cf2b8"))
    BLOCKS["lapis_block"] = lambda: _metal_block("lapis_block", pal("#13245e", "#1c3480", "#264aa0", "#335fbe", "#4c7ad4", "#7c9ee6"))
    BLOCKS["sculk"] = lambda: _sculk("sculk")
    BLOCKS["suspicious_sand"] = lambda: _suspicious("suspicious_sand", SAND)
    BLOCKS["suspicious_gravel"] = lambda: _suspicious("suspicious_gravel", GRAVEL)


def _deepslate_side():
    rng = rng_for("deepslate")
    # Deepslate's grain runs up and down.
    f = 0.65 * value_noise(rng, 16, 2) + 0.35 * value_noise(rng, 4, 1)
    img = from_rgb(shade(normalize(f), DEEPSLATE, 0.7))
    speckle(img, rng, [DEEPSLATE[0], DEEPSLATE[5]], 10)
    return img


def _cobbled(p, name):
    rng = rng_for(name)
    ids, d1, d2, _ = voronoi(rng, 9, 4.0)
    edge = (d2 - d1) < 1.1
    f = normalize(value_noise(rng, 4) * 0.6 + (ids % 3) * 0.2)
    img = from_rgb(shade(f * 0.7 + 0.2, p[1:], 0.5))
    img[edge, :3] = p[0]
    return img


def _cracked(img, name):
    rng = rng_for(name + "_cracks")
    for _ in range(3):
        x, y = rng.integers(1, N - 1, 2)
        for _ in range(5):
            put(img, x, y, pal("#141418")[0])
            x += rng.integers(-1, 2)
            y += 1
    return img


def _tuff(name):
    rng = rng_for(name)
    img = noise_tex(name, TUFF, ((8, 0.3), (4, 0.35), (2, 0.35), (1, 0.25)))
    speckle(img, rng, [pal("#a4a68f")[0], pal("#3f413b")[0], pal("#8c8a6c")[0]], 18)
    return img


def _chiseled_tuff(name, bricks):
    img = tiles_tex(name, TUFF, 8) if bricks else polished_tex(name, TUFF)
    rgb = img[..., :3]
    # A band of copper-coloured carving in the middle.
    for x in range(N):
        rgb[6, x] = TUFF[0]
        rgb[9, x] = TUFF[0]
        if x % 4 in (1, 2):
            rgb[7:9, x] = pal("#9a6a3c")[0]
    return img


def _prismarine():
    rng = rng_for("prismarine")
    f = fbm(rng, ((8, 0.4), (4, 0.3), (2, 0.3)))
    img = from_rgb(shade(f, PRISM, 0.6))
    # Faint diagonal currents of lighter green.
    m = ((XX + YY * 2) % 7 == 0)
    img[m, :3] = scale_rgb(img[m, :3], 1.12)
    return img


def _metal_block(name, p):
    img = noise_tex(name, p[1:5], ((8, 0.4), (4, 0.3)), 0.4)
    bevel(img, 1.2, 0.65)
    rgb = img[..., :3]
    rgb[1, 1:15] = p[-1]
    rgb[1:15, 1] = p[-1]
    return img


def _sculk(name):
    rng = rng_for(name)
    f = fbm(rng, ((8, 0.35), (4, 0.35), (2, 0.3), (1, 0.2)))
    img = from_rgb(shade(f, SCULK, 0.6))
    # Glowing specks: the souls it soaked up.
    for _ in range(9):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, SCULK_GLOW[rng.integers(1, 3)])
        if rng.random() < 0.4:
            put(img, x + 1, y, SCULK_GLOW[0])
    return img


def _suspicious(name, p):
    rng = rng_for(name)
    img = noise_tex(name, p, ((4, 0.4), (2, 0.4), (1, 0.4)), 0.7)
    # Something half buried: a pale shard poking out, and disturbed grains.
    for (x, y) in ((6, 7), (7, 7), (8, 8), (9, 8), (7, 8)):
        put(img, x, y, pal("#b0603a")[0] if (x + y) % 2 else pal("#d68a5a")[0])
    speckle(img, rng, [p[0]], 14)
    return img


@blockfn("reinforced_deepslate_side")
def reinforced_side():
    img = brick_tex("reinforced_deepslate_side", DEEPSLATE, 16, 8, pal("#18181c")[0], 8)
    rgb = img[..., :3]
    # Iron-ish bands and a pale bone-coloured core peeking through.
    rgb[3:5, :] = pal("#5e6460")[0]
    rgb[11:13, :] = pal("#5e6460")[0]
    rgb[6:10, 6:10] = pal("#c8c4b0")[0]
    rgb[7:9, 7:9] = pal("#e4e0cc")[0]
    return img


@blockfn("reinforced_deepslate_top")
def reinforced_top():
    img = tiles_tex("reinforced_deepslate_top", DEEPSLATE, 8)
    img[6:10, 6:10, :3] = pal("#c8c4b0")[0]
    return img


@blockfn("reinforced_deepslate_bottom")
def reinforced_bottom():
    return tiles_tex("reinforced_deepslate_bottom", DEEPSLATE, 4)


def _sculk_rim(name, top_color, glow):
    """Top of a sensor or shrieker: sculk with a rim and a coloured middle."""
    img = _sculk(name)
    rgb = img[..., :3]
    rgb[0, :] = SCULK[4]
    rgb[:, 0] = SCULK[4]
    rgb[15, :] = SCULK[0]
    rgb[:, 15] = SCULK[0]
    c = top_color
    for (x, y) in ((4, 4), (11, 4), (4, 11), (11, 11)):
        rgb[y - 1:y + 2, x - 1:x + 2] = c
        put(img, x, y, glow)
    return img


@blockfn("sculk_sensor_top")
def sensor_top():
    return _sculk_rim("sculk_sensor_top", pal("#0f5f6a")[0], SCULK_GLOW[1])


@blockfn("sculk_sensor_top_active")
def sensor_top_on():
    return _sculk_rim("sculk_sensor_top_active", pal("#1d9aa8")[0], SCULK_GLOW[3])


@blockfn("calibrated_sculk_sensor_top")
def cal_top():
    return _sculk_rim("calibrated_sculk_sensor_top", pal("#5a2f7a")[0], pal("#b07ae0")[0])


@blockfn("calibrated_sculk_sensor_top_active")
def cal_top_on():
    return _sculk_rim("calibrated_sculk_sensor_top_active", pal("#8a4fc0")[0], pal("#e8c8ff")[0])


def _half_side(name, upper, lower):
    """Side of an 8-pixel-tall block: the art sits in the lower half (the renderer crops)."""
    img = _sculk(name)
    rgb = img[..., :3]
    for y in range(N):
        for x in range(N):
            if y < 8:
                rgb[y, x] = upper[(y + x // 5) % len(upper)]
            else:
                rgb[y, x] = lower[(y + x // 4) % len(lower)]
    # Tendrils: the feelers that listen, standing up in the upper half.
    for x in (3, 12):
        rgb[2:8, x] = SCULK_GLOW[1]
        put(img, x, 1, SCULK_GLOW[3])
    return img


@blockfn("sculk_sensor_side")
def sensor_side():
    return _half_side("sculk_sensor_side", SCULK[2:], SCULK[:4])


@blockfn("calibrated_sculk_sensor_side")
def cal_side():
    img = _half_side("calibrated_sculk_sensor_side", SCULK[2:], SCULK[:4])
    img[9:11, 2:14, :3] = pal("#7a4aa8")[0]
    return img


@blockfn("sculk_shrieker_top")
def shrieker_top():
    img = _sculk("sculk_shrieker_top")
    rgb = img[..., :3]
    bone = pal("#b8b2a0", "#d6d0bc", "#ece6d2")
    rgb[2:14, 2:14] = bone[1]
    rgb[2, 2:14] = bone[2]
    rgb[13, 2:14] = bone[0]
    rgb[5:11, 5:11] = SCULK[1]
    rgb[6:10, 6:10] = SCULK_GLOW[0]
    rgb[7:9, 7:9] = SCULK_GLOW[2]
    return img


@blockfn("sculk_shrieker_side")
def shrieker_side():
    img = _sculk("sculk_shrieker_side")
    rgb = img[..., :3]
    bone = pal("#b8b2a0", "#d6d0bc", "#ece6d2")
    for y in range(8, 16):
        for x in range(N):
            rgb[y, x] = SCULK[(y + x // 3) % 3]
    for x in range(1, 15, 3):
        rgb[3:8, x] = bone[1]
        put(img, x, 2, bone[2])
    rgb[8, :] = bone[0]
    return img


@blockfn("sculk_shrieker_bottom")
def shrieker_bottom():
    return _sculk("sculk_shrieker_bottom")


@blockfn("sculk_catalyst_top")
def catalyst_top():
    img = _sculk("sculk_catalyst_top")
    rgb = img[..., :3]
    bone = pal("#b8b2a0", "#d6d0bc", "#ece6d2")
    rgb[3:13, 3:13] = bone[1]
    rgb[3, 3:13] = bone[2]
    rgb[12, 3:13] = bone[0]
    rgb[6:10, 6:10] = SCULK_GLOW[1]
    rgb[7:9, 7:9] = SCULK_GLOW[3]
    return img


@blockfn("sculk_catalyst_side")
def catalyst_side():
    img = _sculk("sculk_catalyst_side")
    rgb = img[..., :3]
    bone = pal("#b8b2a0", "#d6d0bc", "#ece6d2")
    rgb[0:3, :] = bone[1]
    rgb[0, :] = bone[2]
    for x in range(2, 14, 4):
        rgb[3:7, x:x + 2] = bone[0]
    rgb[9:12, 6:10] = SCULK_GLOW[1]
    return img


@blockfn("sculk_catalyst_bottom")
def catalyst_bottom():
    return _sculk("sculk_catalyst_bottom")


@blockfn("sculk_vein")
def sculk_vein():
    rng = rng_for("sculk_vein")
    img = canvas()
    # Thin branching lines of sculk over an otherwise clear block.
    for _ in range(5):
        x, y = rng.integers(0, N, 2)
        for _ in range(9):
            put(img, x, y, SCULK[rng.integers(1, 5)])
            x = (x + rng.integers(-1, 2)) % N
            y = (y + rng.integers(-1, 2)) % N
    speckle(img, rng, [SCULK_GLOW[1]], 3)
    return img


@blockfn("enchanting_table_top")
def ench_top():
    img = noise_tex("enchanting_table_top", OBSIDIAN)
    rgb = img[..., :3]
    cloth = pal("#5a0e18", "#7c1622", "#9c2030")
    rgb[1:15, 1:15] = cloth[1]
    rgb[1, 1:15] = cloth[2]
    rgb[1:15, 1] = cloth[2]
    rgb[14, 1:15] = cloth[0]
    # The open book: pale pages, a dark spine, lines of writing.
    page = pal("#cfc4a0", "#e4dab8", "#f2ead0")
    rgb[4:12, 3:13] = page[1]
    rgb[4:12, 7:9] = pal("#4a2c18")[0]
    for y in (5, 7, 9):
        rgb[y, 4:7] = page[0]
        rgb[y, 9:12] = page[0]
    rgb[4, 3:13] = page[2]
    return img


@blockfn("enchanting_table_side")
def ench_side():
    img = noise_tex("enchanting_table_side", OBSIDIAN)
    rgb = img[..., :3]
    cloth = pal("#5a0e18", "#7c1622", "#9c2030")
    rgb[0:4, :] = cloth[1]
    rgb[0, :] = cloth[2]
    rgb[4, :] = cloth[0]
    # Little golden glyph hanging from the cloth.
    for x in (3, 8, 12):
        rgb[5:7, x] = pal("#c89a28")[0]
    for k in range(5):
        put(img, 2 + k * 3, 11 + (k % 2), pal("#6a4ac0")[0])
    return img


@blockfn("enchanting_table_bottom")
def ench_bottom():
    return noise_tex("enchanting_table_bottom", OBSIDIAN)


@blockfn("anvil_side")
def anvil_side():
    # Plain forged iron: the anvil is built from boxes (Shape::Anvil), so the picture is the
    # metal, not a cut-out of the anvil's outline.
    return noise_tex("anvil_side", IRON[0:4], ((4, 0.4), (2, 0.4), (1, 0.2)), 0.5)


def _anvil_top(name, cracks):
    img = noise_tex(name, IRON[1:5], ((4, 0.4), (2, 0.4)), 0.4)
    bevel(img, 1.15, 0.7)
    rng = rng_for(name)
    for _ in range(cracks):
        x, y = rng.integers(2, 14, 2)
        for _ in range(4):
            put(img, x, y, IRON[0])
            x += rng.integers(-1, 2)
            y += 1
    return img


BLOCKS["anvil_top"] = lambda: _anvil_top("anvil_top", 0)
BLOCKS["chipped_anvil_top"] = lambda: _anvil_top("chipped_anvil_top", 2)
BLOCKS["damaged_anvil_top"] = lambda: _anvil_top("damaged_anvil_top", 5)


@blockfn("grindstone_side")
def grind_side():
    img = noise_tex("grindstone_side", pal("#6c6c6c", "#7c7c7c", "#8c8c8c", "#9c9c9c"))
    rgb = img[..., :3]
    r = np.sqrt((XX - 7.5) ** 2 + (YY - 7.5) ** 2)
    rgb[r > 7.2] = PLANK[1]
    rgb[(r > 2.2) & (r < 3.2)] = pal("#585858")[0]
    rgb[r < 1.6] = PLANK[3]
    return img


@blockfn("grindstone_top")
def grind_top():
    img = noise_tex("grindstone_top", pal("#6c6c6c", "#7c7c7c", "#8c8c8c", "#9c9c9c"))
    img[:, 0:2, :3] = PLANK[2]
    img[:, 14:16, :3] = PLANK[2]
    return img


@blockfn("grindstone_bottom")
def grind_bottom():
    img = gi.canvas() if hasattr(gi, "canvas") else canvas()
    img = from_rgb(shade(fbm(rng_for("grindstone_bottom")), PLANK, 0.5))
    return img


def _smithing(name, top=False, front=False):
    img = from_rgb(shade(fbm(rng_for(name)), PLANK, 0.5))
    rgb = img[..., :3]
    if top:
        rgb[0:16, 0:16] = pal("#2a2a2e")[0]
        rgb[1:15, 1:15] = pal("#3a3a40")[0]
        rgb[3:6, 3:13] = pal("#585860")[0]
        rgb[10:13, 4:12] = pal("#585860")[0]
        return img
    rgb[0:4, :] = pal("#2a2a2e")[0]
    rgb[1:3, :] = pal("#46464e")[0]
    if front:
        # Tools hanging on the front: tongs and a hammer.
        rgb[6:13, 4] = IRON[3]
        rgb[6, 3:6] = IRON[4]
        rgb[6:12, 11] = PLANK[4]
        rgb[6:8, 9:14] = IRON[2]
    return img


BLOCKS["smithing_table_top"] = lambda: _smithing("smithing_table_top", top=True)
BLOCKS["smithing_table_side"] = lambda: _smithing("smithing_table_side")
BLOCKS["smithing_table_front"] = lambda: _smithing("smithing_table_front", front=True)
BLOCKS["smithing_table_bottom"] = lambda: from_rgb(shade(fbm(rng_for("smithing_table_bottom")), PLANK, 0.5))


@blockfn("brewing_stand")
def brewing_stand():
    rows = [
        "................",
        ".......#........",
        ".......#........",
        ".......#........",
        "...###.#.###....",
        "..#...###...#...",
        "..#....#....#...",
        ".###...#...###..",
        "#bbb#..#..#rrr#.",
        "#bbb#.ggg.#rrr#.",
        ".###.#ggg#.###..",
        ".....#ggg#......",
        "......###.......",
        "................",
        ".#############..",
        ".#############..",
    ]
    colors = {"#": IRON[3], "b": pal("#3a6ad0")[0], "r": pal("#c03050")[0], "g": pal("#40b060")[0]}
    img = from_map(rows, colors)
    img[1:8, 7, :3] = pal("#e0b040")[0]
    return img


@blockfn("beacon")
def beacon():
    img = canvas()
    img[..., :3] = pal("#bfe8ee")[0]
    img[..., 3] = 120
    img[0, :, 3] = img[15, :, 3] = img[:, 0, 3] = img[:, 15, 3] = 255
    img[4:12, 4:12, :3] = pal("#6ff0f0")[0]
    img[4:12, 4:12, 3] = 255
    img[6:10, 6:10, :3] = pal("#f4ffff")[0]
    img[13:15, 1:15, :3] = OBSIDIAN[3]
    img[13:15, 1:15, 3] = 255
    return img


@blockfn("conduit")
def conduit():
    img = canvas()
    rows = [
        "................",
        "................",
        "................",
        "................",
        "................",
        "......####......",
        ".....#oooo#.....",
        ".....#oeeo#.....",
        ".....#oeeo#.....",
        ".....#oooo#.....",
        "......####......",
        "................",
        "................",
        "................",
        "................",
        "................",
    ]
    return from_map(rows, {"#": pal("#6a5a3c")[0], "o": pal("#a8905c")[0], "e": pal("#4ae0f0")[0]})


BLOCKS["conduit_top"] = conduit


@blockfn("sea_lantern")
def sea_lantern():
    img = noise_tex("sea_lantern", pal("#a6c8c0", "#bcdcd4", "#d4ece6", "#ecfaf6"), ((4, 0.4), (2, 0.4)), 0.5)
    rgb = img[..., :3]
    rgb[0, :] = rgb[15, :] = rgb[:, 0] = rgb[:, 15] = pal("#6a9a90")[0]
    rgb[4:12, 4:12] = pal("#f4fffc")[0]
    rgb[7:9, :] = pal("#c4e4dc")[0]
    rgb[:, 7:9] = pal("#c4e4dc")[0]
    return img


@blockfn("ancient_debris_side")
def debris_side():
    rng = rng_for("ancient_debris_side")
    img = from_rgb(shade(normalize(0.6 * value_noise(rng, 16, 2) + 0.4 * value_noise(rng, 2, 1)), pal("#3a2a24", "#4a3830", "#5c4a40", "#6e5c50"), 0.6))
    for y in (3, 8, 12):
        img[y, :, :3] = scale_rgb(img[y, :, :3], 0.7)
    speckle(img, rng, [pal("#8a6a5a")[0]], 10)
    return img


@blockfn("ancient_debris_top")
def debris_top():
    rng = rng_for("ancient_debris_top")
    img = from_rgb(shade(fbm(rng), pal("#3a2a24", "#4a3830", "#5c4a40", "#6e5c50"), 0.6))
    r = np.sqrt((XX - 7.5) ** 2 + (YY - 7.5) ** 2)
    img[(r > 3) & (r < 4), :3] = pal("#2a1c18")[0]
    img[(r > 5.5) & (r < 6.4), :3] = pal("#2a1c18")[0]
    return img


@blockfn("frosted_ice")
def frosted_ice():
    img = noise_tex("frosted_ice", pal("#8ab4e8", "#9cc4f0", "#b0d4f8", "#c8e4ff"), ((4, 0.4), (2, 0.4)))
    img[..., 3] = 200
    rng = rng_for("frosted_ice_c")
    for _ in range(6):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, pal("#eaf6ff")[0], 230)
    return img


@blockfn("sugar_cane")
def sugar_cane():
    img = canvas()
    green = pal("#4e8a2c", "#6aaa3c", "#86c44e", "#a4dc6a")
    for x0 in (3, 8, 12):
        for y in range(N):
            img[y, x0:x0 + 2, :3] = green[1] if y % 5 else green[0]
            img[y, x0, :3] = green[2] if y % 5 else green[0]
            img[y, x0:x0 + 2, 3] = 255
        put(img, x0 + 2, 4 + x0 % 3, green[3])
        put(img, x0 + 2, 5 + x0 % 3, green[2])
    return img


@blockfn("cobweb")
def cobweb():
    img = canvas()
    w = pal("#e8e8e8")[0]
    for k in range(N):
        put(img, k, k, w, 220)
        put(img, N - 1 - k, k, w, 220)
        put(img, 7, k, w, 200)
        put(img, k, 8, w, 200)
    for r in (3, 6):
        for a in range(32):
            t = a / 32 * 2 * np.pi
            put(img, int(7.5 + r * np.cos(t)), int(7.5 + r * np.sin(t)), w, 180)
    return img


@blockfn("slime_block")
def slime_block():
    img = noise_tex("slime_block", pal("#5aa848", "#6cbc58", "#80cc6a", "#94dc7e"), ((4, 0.5), (2, 0.4)), 0.4)
    img[..., 3] = 190
    img[3:13, 3:13, 3] = 235
    img[3:13, 3:13, :3] = scale_rgb(img[3:13, 3:13, :3], 0.85)
    img[0, :, 3] = img[15, :, 3] = img[:, 0, 3] = img[:, 15, 3] = 255
    return img


@blockfn("candle")
def candle():
    img = canvas()
    img[6:16, 6:10, :3] = pal("#e6dcc0")[0]
    img[6:16, 6, :3] = pal("#f6eed8")[0]
    img[6:16, 9, :3] = pal("#c8bc9c")[0]
    img[6:16, 6:10, 3] = 255
    put(img, 7, 5, pal("#303030")[0])
    put(img, 7, 4, pal("#ffd050")[0])
    put(img, 8, 3, pal("#fff0a0")[0])
    return img


def _lantern(name, flame):
    img = canvas()
    img[3:14, 4:12, :3] = pal("#2a2a32")[0]
    img[3:14, 4:12, 3] = 255
    img[5:12, 5:11, :3] = flame
    img[3, 4:12, :3] = pal("#4a4a54")[0]
    img[13, 4:12, :3] = pal("#4a4a54")[0]
    img[0:3, 7:9, :3] = pal("#3a3a42")[0]
    img[0:3, 7:9, 3] = 255
    return img


BLOCKS["soul_lantern"] = lambda: _lantern("soul_lantern", pal("#60e8f0")[0])
BLOCKS["soul_lantern_top"] = lambda: noise_tex("soul_lantern_top", pal("#2a2a32", "#3a3a44", "#4a4a54"))


def _spawner(name, flame, ominous=False):
    img = canvas()
    frame = TUFF if not ominous else pal("#2a3440", "#344250", "#3e5060", "#486070", "#527080")
    bars = IRON[3] if not ominous else pal("#7090a8")[0]
    img[..., 3] = 255
    img[..., :3] = frame[1]
    img[1:15, 1:15, 3] = 0
    for x in range(2, 15, 3):
        img[1:15, x, :3] = bars
        img[1:15, x, 3] = 255
    for y in (5, 10):
        img[y, 1:15, :3] = bars
        img[y, 1:15, 3] = 255
    img[0, :, :3] = frame[3]
    img[:, 0, :3] = frame[3]
    if flame is not None:
        img[7:9, 6:10, :3] = flame
        img[7:9, 6:10, 3] = 255
    return img


BLOCKS["trial_spawner_side"] = lambda: _spawner("trial_spawner_side", None)
BLOCKS["trial_spawner_top"] = lambda: _spawner("trial_spawner_top", None)
BLOCKS["trial_spawner_active_side"] = lambda: _spawner("trial_spawner_active_side", pal("#f0a030")[0])
BLOCKS["trial_spawner_active_top"] = lambda: _spawner("trial_spawner_active_top", pal("#f0a030")[0])
BLOCKS["trial_spawner_ominous_side"] = lambda: _spawner("trial_spawner_ominous_side", pal("#50a8f0")[0], True)
BLOCKS["trial_spawner_ominous_top"] = lambda: _spawner("trial_spawner_ominous_top", pal("#50a8f0")[0], True)
BLOCKS["trial_spawner_bottom"] = lambda: polished_tex("trial_spawner_bottom", TUFF)


def _vault(name, part, ominous):
    frame = TUFF if not ominous else pal("#2a3440", "#344250", "#3e5060", "#486070", "#527080")
    img = polished_tex(name, frame)
    rgb = img[..., :3]
    trim = pal("#b0703a")[0] if not ominous else pal("#4a90b8")[0]
    rgb[0:2, :] = trim
    rgb[14:16, :] = trim
    if part == "front":
        rgb[4:12, 4:12] = pal("#1a1a1e")[0]
        rgb[6:10, 6:10] = pal("#f0c050")[0] if not ominous else pal("#80d8ff")[0]
        rgb[7:9, 7] = pal("#2a2a2e")[0]
    elif part == "top":
        rgb[5:11, 5:11] = trim
        rgb[7:9, 7:9] = pal("#1a1a1e")[0]
    elif part == "side":
        for x in range(3, 13, 3):
            rgb[4:12, x] = frame[0]
    return img


for _n, _p, _o in (("vault_side", "side", False), ("vault_front", "front", False), ("vault_top", "top", False), ("vault_bottom", "bottom", False),
                   ("ominous_vault_side", "side", True), ("ominous_vault_front", "front", True), ("ominous_vault_top", "top", True)):
    BLOCKS[_n] = (lambda n=_n, p=_p, o=_o: _vault(n, p, o))


@blockfn("heavy_core")
def heavy_core():
    img = canvas()
    p = pal("#2a2c34", "#3a3e48", "#4c505c", "#626674", "#7a7e8c")
    m = np.zeros((N, N), bool)
    m[4:13, 4:12] = True
    shade_mask(img, m, p, rng_for("heavy_core"))
    img[6:11, 6:10, :3] = pal("#1a1c22")[0]
    img[7:9, 7:9, :3] = pal("#6a7a9a")[0]
    return img


@blockfn("heavy_core_top")
def heavy_core_top():
    img = canvas()
    img[4:12, 4:12, :3] = pal("#4c505c")[0]
    img[4:12, 4:12, 3] = 255
    img[6:10, 6:10, :3] = pal("#2a2c34")[0]
    return img


@blockfn("decorated_pot_side")
def pot_side():
    img = canvas()
    clay = pal("#7a3a26", "#9a4c32", "#b45e3e", "#c8724e")
    rows = [
        "................",
        "......####......",
        ".....######.....",
        "......####......",
        "....########....",
        "...##########...",
        "..############..",
        "..############..",
        "..############..",
        "..############..",
        "..############..",
        "..############..",
        "...##########...",
        "....########....",
        "................",
        "................",
    ]
    shade_mask(img, mask_from_rows(rows), clay, rng_for("decorated_pot_side"), speck=0.05)
    # A painted band of little figures.
    for x in range(4, 12, 2):
        put(img, x, 8, pal("#3a1a12")[0])
        put(img, x + 1, 9, pal("#3a1a12")[0])
    return img


@blockfn("decorated_pot_top")
def pot_top():
    img = canvas()
    r = np.sqrt((XX - 7.5) ** 2 + (YY - 7.5) ** 2)
    m = r < 6.2
    img[m, :3] = pal("#b45e3e")[0]
    img[m, 3] = 255
    img[r < 2.2, :3] = pal("#3a1a12")[0]
    return img


@blockfn("sniffer_egg")
def sniffer_egg():
    img = canvas()
    rows = [
        "................",
        "................",
        "................",
        "......####......",
        ".....######.....",
        "....########....",
        "....########....",
        "...##########...",
        "...##########...",
        "...##########...",
        "...##########...",
        "...##########...",
        "....########....",
        ".....######.....",
        "................",
        "................",
    ]
    shade_mask(img, mask_from_rows(rows), pal("#7a2e22", "#9a3e2c", "#b45038", "#c86648", "#dc8060"), rng_for("sniffer_egg"), speck=0.1)
    rng = rng_for("sniffer_egg_spots")
    for _ in range(7):
        x, y = rng.integers(4, 12), rng.integers(5, 13)
        if img[y, x, 3] > 0:
            put(img, x, y, pal("#2a6a4a")[0])
    return img


@blockfn("sniffer_egg_top")
def sniffer_egg_top():
    img = canvas()
    r = np.sqrt((XX - 7.5) ** 2 + (YY - 7.5) ** 2)
    img[r < 4.5, :3] = pal("#b45038")[0]
    img[r < 4.5, 3] = 255
    return img


def _plant(name, stem_h, flower=None, leaves=2, pitcher=False):
    img = canvas()
    green = pal("#3a6a24", "#4e8a2c", "#6aaa3c", "#86c44e")
    top = N - stem_h
    img[top:N, 7:9, :3] = green[1]
    img[top:N, 7, :3] = green[2]
    img[top:N, 7:9, 3] = 255
    for k in range(leaves):
        y = N - 3 - k * 3
        if y <= top:
            break
        for d in range(1, 4):
            put(img, 7 - d, y - d // 2, green[2])
            put(img, 8 + d, y - d // 2, green[1])
    if flower == "torch":
        c = pal("#c03010", "#e05a18", "#f08a30", "#ffd060")
        img[top - 4:top + 1, 6:10, :3] = c[1]
        img[top - 4:top + 1, 6:10, 3] = 255
        img[top - 4, 6:10, :3] = c[3]
        img[top - 3, 7:9, :3] = c[2]
        img[top - 5, 7:9, :3] = c[0]
        img[top - 5, 7:9, 3] = 255
    elif flower == "bud":
        img[top - 2:top + 1, 7:9, :3] = pal("#c86a20")[0]
        img[top - 2:top + 1, 7:9, 3] = 255
    if pitcher:
        c = pal("#3a6aa0", "#4a8ac8", "#6aaae0", "#a0d0f0")
        img[top - 5:top, 5:11, :3] = c[1]
        img[top - 5:top, 5:11, 3] = 255
        img[top - 5:top, 5, :3] = c[3]
        img[top - 6, 6:10, :3] = c[2]
        img[top - 6, 6:10, 3] = 255
    return img


BLOCKS["torchflower"] = lambda: _plant("torchflower", 9, "torch")
BLOCKS["torchflower_stage0"] = lambda: _plant("torchflower_stage0", 3)
BLOCKS["torchflower_stage1"] = lambda: _plant("torchflower_stage1", 6, "bud")
BLOCKS["torchflower_stage2"] = lambda: _plant("torchflower_stage2", 9, "torch")
BLOCKS["pitcher_plant"] = lambda: _plant("pitcher_plant", 10, None, 3, True)
for _k in range(5):
    BLOCKS[f"pitcher_stage{_k}"] = (lambda k=_k: _plant(f"pitcher_stage{k}", 2 + k * 2, None, 1 + k // 2, k == 4))

# ---------------------------------------------------------------------------
# Items
# ---------------------------------------------------------------------------

ITEMS = {}

BOTTLE = [
    "................",
    "......####......",
    "......#cc#......",
    ".......##.......",
    ".......##.......",
    "......#gg#......",
    ".....#gggg#.....",
    "....#gLLLLg#....",
    "...#gLLLLLLg#...",
    "...#LLhLLLLL#...",
    "...#LhLLLLLL#...",
    "...#LLLLLLLL#...",
    "...#LLLLLLLL#...",
    "....#LLLLLL#....",
    ".....######.....",
    "................",
]

SPLASH = [
    "................",
    ".......##.......",
    "......#cc#......",
    ".......##.......",
    "......#gg#......",
    ".....#gggg#.....",
    "....#gLLLLg#....",
    "...#gLLLLLLg#...",
    "..#LLhLLLLLLL#..",
    "..#LhLLLLLLLL#..",
    "..#LLLLLLLLLL#..",
    "..#LLLLLLLLLL#..",
    "...#LLLLLLLL#...",
    "....#LLLLLL#....",
    ".....######.....",
    "................",
]

LINGER = [
    "................",
    "......####......",
    "......#cc#......",
    ".......##.......",
    ".......##.......",
    "......#gg#......",
    "......#gg#......",
    ".....#gLLg#.....",
    "....#gLLLLg#....",
    "...#LLhLLLLL#...",
    "..#LhLLLLLLLL#..",
    "..#LLLLLLLLLL#..",
    "..#LLLLLLLLLL#..",
    "...#LLLLLLLL#...",
    "....########....",
    "................",
]

GLASS = pal("#9ab8c4", "#c4dce4", "#e8f4f8")


def bottle(rows, liquid, cork=None, empty=False):
    liquid = np.asarray(liquid, dtype=float)
    colors = {
        "#": GLASS[0] * 0.55,
        "c": pal("#8a6a3c")[0] if cork is None else cork,
        "g": GLASS[1],
        "L": GLASS[1] if empty else liquid,
        "h": GLASS[2] if empty else np.clip(liquid * 1.35 + 30, 0, 255),
    }
    img = from_map(rows, colors)
    if empty:
        a = img[..., 3]
        inside = np.array([[c in "Lgh" for c in r] for r in rows])
        a[inside] = 110
    else:
        # A darker bottom to the liquid.
        for y in range(N):
            for x in range(N):
                if rows[y][x] == "L" and y >= 12:
                    img[y, x, :3] = scale_rgb(liquid, 0.78)
    return img


ARROW = [
    "................",
    "............###.",
    "...........#hh#.",
    "..........#hhh#.",
    "...........#h#..",
    "..........#/....",
    ".........#/.....",
    "........#/......",
    ".......#/.......",
    "......#/........",
    "..ff.#/.........",
    ".fFf#/..........",
    ".fFF/...........",
    "..fFf...........",
    "...ff...........",
    "................",
]


def tipped_arrow(color):
    c = np.asarray(color, dtype=float)
    img = from_map(ARROW, {"#": pal("#3a2a18")[0], "/": pal("#8a6a3c")[0], "h": c, "f": c * 0.7, "F": np.clip(c * 1.2 + 20, 0, 255)})
    return outline(img, 0.4)


def effect_colors():
    rs = (os.path.join(ROOT, "crates", "game", "src", "effects.rs"))
    text = open(rs).read()
    consts = dict((n, int(v)) for n, v in re.findall(r"pub const ([A-Z_]+): u8 = (\d+);", text))
    colors = {}
    for name, rgb in re.findall(r"([A-Z_]+) => \(\"[^\"]+\", \[(\d+, \d+, \d+)\]", text):
        if name in consts:
            colors[consts[name]] = [int(v) for v in rgb.split(",")]
    potions = []
    for key, body in re.findall(r'p!\("([a-z_]+)", "[^"]+", \[([^\]]*)\]\)', text):
        effs = re.findall(r"\(([A-Z_]+), (\d+), [\d.]+\)", body)
        potions.append((key, [consts[e] for e, _ in effs]))
    return colors, potions


def potion_color(effects, colors):
    if not effects:
        return [56, 92, 196]
    c = np.mean([colors[e] for e in effects], axis=0)
    return list(c)


def register_potions():
    colors, potions = effect_colors()
    for key, effs in potions:
        c = potion_color(effs, colors)
        if key in ("mundane", "thick", "awkward"):
            c = [80, 110, 200]
        ITEMS[f"potion_{key}"] = (lambda c=c: outline(bottle(BOTTLE, c), 0.4))
        ITEMS[f"splash_potion_{key}"] = (lambda c=c: outline(bottle(SPLASH, c, pal("#6a6a6a")[0]), 0.4))
        ITEMS[f"lingering_potion_{key}"] = (lambda c=c: outline(bottle(LINGER, c, pal("#c8c8c8")[0]), 0.4))
        ITEMS[f"tipped_arrow_{key}"] = (lambda c=c: tipped_arrow(c))
    ITEMS["glass_bottle"] = lambda: outline(bottle(BOTTLE, [0, 0, 0], empty=True), 0.4)
    ITEMS["experience_bottle"] = lambda: outline(bottle(BOTTLE, [120, 230, 60]), 0.4)
    ITEMS["honey_bottle"] = lambda: outline(bottle(BOTTLE, [230, 160, 30]), 0.4)
    ITEMS["ominous_bottle"] = lambda: outline(bottle(LINGER, [20, 110, 80], pal("#2a2a2a")[0]), 0.4)
    ITEMS["dragon_breath"] = lambda: outline(bottle(BOTTLE, [210, 110, 190], pal("#e0a0d0")[0]), 0.4)


def sprite(rows, colors, line_k=0.4):
    return outline(from_map(rows, colors), line_k)


def item(name):
    def deco(fn):
        ITEMS[name] = fn
        return fn
    return deco


@item("book")
def book():
    rows = [
        "................",
        "................",
        "...#########....",
        "..#bbbbbbbbP#...",
        "..#bBBBBBBbP#...",
        "..#bBBBBBBbP#...",
        "..#bbbbbbbbP#...",
        "..#bbbbbbbbP#...",
        "..#bbbbbbbbP#...",
        "..#bbbbbbbbP#...",
        "..#bbbbbbbbP#...",
        "..#bbbbbbbbP#...",
        "..#bbbbbbbbP#...",
        "...#########....",
        "................",
        "................",
    ]
    return sprite(rows, {"#": pal("#3a1a0c")[0], "b": pal("#7a3a1a")[0], "B": pal("#a85a2a")[0], "P": pal("#e8e0c8")[0]})


@item("enchanted_book")
def enchanted_book():
    img = book()
    for (x, y) in ((5, 4), (7, 4), (9, 4), (6, 8), (8, 9), (5, 11)):
        put(img, x, y, pal("#c070ff")[0])
    img[3:13, 3:11, :3] = scale_rgb(img[3:13, 3:11, :3], 1.0) * 0.6 + np.array([90, 30, 110]) * 0.4
    return img


@item("paper")
def paper():
    rows = [
        "................",
        "................",
        "...#########....",
        "...#ppppppp#....",
        "...#pPPPPPp#....",
        "...#ppppppp#....",
        "...#pPPPPpp#....",
        "...#ppppppp#....",
        "...#pPPPPPp#....",
        "...#ppppppp#....",
        "...#pPPPppp#....",
        "...#ppppppp#....",
        "...#########....",
        "................",
        "................",
        "................",
    ]
    return sprite(rows, {"#": pal("#a8a090")[0], "p": pal("#f0ece0")[0], "P": pal("#d4ccb8")[0]})


def blob(name, p, rows=None, spots=None):
    rows = rows or [
        "................",
        "................",
        "................",
        "......####......",
        "....########....",
        "...##########...",
        "...##########...",
        "..############..",
        "..############..",
        "..############..",
        "...##########...",
        "...##########...",
        "....########....",
        "......####......",
        "................",
        "................",
    ]
    img = canvas()
    shade_mask(img, mask_from_rows(rows), p, rng_for(name), speck=0.15)
    if spots is not None:
        rng = rng_for(name + "s")
        for _ in range(6):
            x, y = rng.integers(4, 12), rng.integers(5, 12)
            if img[y, x, 3] > 0:
                put(img, x, y, spots)
    return outline(img)


DUST = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "........#.......",
    "......#####.....",
    ".....########...",
    "....##########..",
    "...############.",
    "..##############",
    "..##############",
    "...############.",
    "................",
    "................",
    "................",
]

ITEMS["blaze_powder"] = lambda: blob("blaze_powder", pal("#a04a08", "#d27010", "#f0a020", "#ffd050"), DUST)
ITEMS["glowstone_dust"] = lambda: blob("glowstone_dust", pal("#9a7a28", "#c8a040", "#e8c860", "#fff0a0"), DUST)
ITEMS["sugar"] = lambda: blob("sugar", pal("#c8c8c8", "#dedede", "#f0f0f0", "#ffffff"), DUST)
ITEMS["magma_cream"] = lambda: blob("magma_cream", pal("#5a2a08", "#a04a10", "#e07a20", "#ffb040"), spots=pal("#ffe080")[0])
ITEMS["slime_ball"] = lambda: blob("slime_ball", pal("#3a7a2a", "#5aa848", "#80cc6a", "#b0f0a0"))
ITEMS["ghast_tear"] = lambda: blob("ghast_tear", pal("#8aa8b8", "#b0ccd8", "#d8eef6", "#ffffff"), [
    "................", "................", "................", "................", "........#.......", ".......###......",
    "......#####.....", "......#####.....", ".....#######....", ".....#######....", ".....#######....", "......#####.....",
    ".......###......", "................", "................", "................"])
ITEMS["spider_eye"] = lambda: blob("spider_eye", pal("#5a0c14", "#8a1a24", "#b42a38", "#d84a58"), spots=pal("#1a0408")[0])
ITEMS["fermented_spider_eye"] = lambda: blob("fermented_spider_eye", pal("#4a1a2a", "#7a3a4a", "#a45a6a", "#c48090"), spots=pal("#3a5a2a")[0])
ITEMS["melon_slice"] = lambda: sprite([
    "................", "................", "................", "................", "...#..........#.", "...#rrrrrrrrrr#.", "....#rRrrRrrr#..",
    ".....#rrrrrrr#..", ".....#rrRrrr#...", "......#rrrr#....", "......#gggg#....", ".......####.....", "................",
    "................", "................", "................"], {"#": pal("#2a5a1a")[0], "r": pal("#d83a3a")[0], "R": pal("#3a1a10")[0], "g": pal("#6aaa3a")[0]})


@item("glistering_melon_slice")
def glistering():
    img = ITEMS["melon_slice"]()
    for (x, y) in ((5, 5), (9, 6), (12, 5), (8, 8), (7, 10)):
        put(img, x, y, pal("#ffe060")[0])
    return img


@item("golden_carrot")
def golden_carrot():
    img = gi.carrot()
    m = img[..., 3] > 0
    lum = img[..., :3] @ np.array([0.3, 0.59, 0.11])
    gold = np.array([250, 200, 60]) * (lum[..., None] / 160.0)
    img[m, :3] = np.clip(gold[m], 0, 255)
    return img


@item("golden_apple")
def golden_apple():
    img = gi.apple()
    m = img[..., 3] > 0
    lum = img[..., :3] @ np.array([0.3, 0.59, 0.11])
    gold = np.array([255, 205, 60]) * (lum[..., None] / 120.0)
    img[m, :3] = np.clip(gold[m], 0, 255)
    return img


@item("enchanted_golden_apple")
def enchanted_golden_apple():
    img = golden_apple()
    for (x, y) in ((4, 5), (11, 7), (6, 11), (9, 4)):
        put(img, x, y, pal("#f0b0ff")[0])
    return img


ITEMS["rabbit_foot"] = lambda: sprite([
    "................", "................", "................", "........##......", ".......#bb#.....", ".......#bb#.....", "......#bbbb#....",
    "......#bbbb#....", ".....#bbbbb#....", ".....#bbbbb#....", "....#bbbbb#.....", "...#wwbbbb#.....", "...#wwwwww#.....",
    "....######......", "................", "................"], {"#": pal("#3a2a1a")[0], "b": pal("#a07a50")[0], "w": pal("#e8dcc8")[0]})
ITEMS["phantom_membrane"] = lambda: sprite([
    "................", "................", "..##........##..", "..#mm######mm#..", "..#mMmmmmmmMm#..", "...#mmMmmMmm#...", "...#mmmMMmmm#...",
    "....#mmmmmm#....", "....#mmMMmm#....", ".....#mmmm#.....", ".....#mmmm#.....", "......#mm#......", "......####......",
    "................", "................", "................"], {"#": pal("#5a6a5a")[0], "m": pal("#c8d8c0")[0], "M": pal("#e8f0e0")[0]})
ITEMS["turtle_scute"] = lambda: blob("turtle_scute", pal("#2a5a1a", "#3a7a2a", "#4a9a3a", "#6aba50"))
ITEMS["prismarine_shard"] = lambda: sprite([
    "................", "................", "...........##...", "..........#pp#..", ".........#pPp#..", "........#pPp#...", ".......#pPp#....",
    "......#pPp#.....", ".....#pPp#......", "....#pPp#.......", "...#pPp#........", "..#ppp#.........", "..####..........",
    "................", "................", "................"], {"#": pal("#2a4a42")[0], "p": PRISM[2], "P": PRISM[4]})
ITEMS["prismarine_crystals"] = lambda: blob("prismarine_crystals", pal("#7aa8a0", "#a8d0c8", "#d0f0e8", "#ffffff"), spots=pal("#b0ffe8")[0])
ITEMS["nautilus_shell"] = lambda: blob("nautilus_shell", pal("#a87a60", "#c89a7a", "#e8c0a0", "#f8e0c8"), spots=pal("#8a4a3a")[0])
ITEMS["heart_of_the_sea"] = lambda: blob("heart_of_the_sea", pal("#0a3a6a", "#1a5a9a", "#3a8ad0", "#8ad0ff"), spots=pal("#e0f8ff")[0])
ITEMS["nether_star"] = lambda: sprite([
    "................", ".......##.......", ".......#w#......", "......#ww#......", "##....#ww#....##", "#ww##.#ww#.##ww#", ".#wwww#ww#wwww#.",
    "..#wwwwWWwwww#..", "..#wwwwWWwwww#..", ".#wwww#ww#wwww#.", "#ww##.#ww#.##ww#", "##....#ww#....##", "......#ww#......",
    ".......#w#......", ".......##.......", "................"], {"#": pal("#8a8a9a")[0], "w": pal("#f0f0ff")[0], "W": pal("#fff8d0")[0]})
ITEMS["netherite_scrap"] = lambda: blob("netherite_scrap", pal("#2a1e1a", "#4a3630", "#6a5048", "#8a6a60"), spots=pal("#a08070")[0])
ITEMS["echo_shard"] = lambda: sprite([
    "................", "................", "..........##....", ".........#ee#...", "........#eEe#...", ".......#eEee#...", "......#eEee#....",
    ".....#eEee#.....", "....#eEee#......", "...#eeee#.......", "...#eee#........", "....###.........", "................",
    "................", "................", "................"], {"#": pal("#062a30")[0], "e": pal("#0f5a64")[0], "E": pal("#3ad0d8")[0]})
ITEMS["breeze_rod"] = lambda: sprite([
    "................", "...........##...", "..........#bb#..", ".........#bBb#..", "........#bBb#...", ".......#bBb#....", "......#bBb#.....",
    ".....#bBb#......", "....#bBb#.......", "...#bBb#........", "..#bbb#.........", "..####..........", "................",
    "................", "................", "................"], {"#": pal("#4a5a8a")[0], "b": pal("#a8b8f0")[0], "B": pal("#e0e8ff")[0]})
ITEMS["wind_charge"] = lambda: blob("wind_charge", pal("#7a8ac0", "#a0b0e8", "#c8d4ff", "#ffffff"), spots=pal("#5a6aa8")[0])


def key_item(name, bit, shine):
    return sprite([
        "................", "................", "..####..........", ".#bbbb#.........", "#bb..bb#........", "#b....b#........", "#bb..bb#........",
        ".#bbbb#b#.......", "..####.#b#......", ".......#bb#.....", "........#bbb#...", ".........#bk#k#.", "..........#bkk#.",
        "...........####.", "................", "................"], {"#": pal("#2a2a32")[0], "b": bit, "k": shine})


ITEMS["trial_key"] = lambda: key_item("trial_key", pal("#c88a3a")[0], pal("#f0c060")[0])
ITEMS["ominous_trial_key"] = lambda: key_item("ominous_trial_key", pal("#3a6a8a")[0], pal("#80d0ff")[0])
ITEMS["torchflower_seeds"] = lambda: sprite([
    "................", "................", "................", "................", "................", "......#.........", ".....#s#...#....",
    "......#...#s#...", "...#.......#....", "..#s#...#.......", "...#...#s#......", ".......#s#......", "........#.......",
    "................", "................", "................"], {"#": pal("#3a2a1a")[0], "s": pal("#c87a3a")[0]})
ITEMS["pitcher_pod"] = lambda: blob("pitcher_pod", pal("#3a5a2a", "#4a7a3a", "#6aa04a", "#8ac86a"), [
    "................", "................", "........##......", ".......#..#.....", "......####......", ".....######.....", "....########....",
    "....########....", "....########....", ".....######.....", "......####......", "................", "................",
    "................", "................", "................"])


def compass_face(name, needle):
    img = canvas()
    r = np.sqrt((XX - 7.5) ** 2 + (YY - 7.5) ** 2)
    ring = r < 6.6
    img[ring, :3] = pal("#7a7a7a")[0]
    img[ring, 3] = 255
    img[r < 5.4, :3] = pal("#c8c0a8")[0] if name == "compass" else pal("#1a3a40")[0]
    for k in range(4):
        put(img, 7 + k // 2 + 0, 4 + k, needle)
        put(img, 7 - k // 2 + 1, 11 - k, pal("#6a6a6a")[0])
    return outline(img)


ITEMS["compass"] = lambda: compass_face("compass", pal("#d02a2a")[0])
ITEMS["recovery_compass"] = lambda: compass_face("recovery_compass", pal("#4ae8e0")[0])


@item("milk_bucket")
def milk_bucket():
    rows = [
        "................",
        "................",
        "...##########...",
        "..#wwwwwwwwww#..",
        "..#iWWWWWWWWi#..",
        "..#iiiiiiiiii#..",
        "...#iiiiiiii#...",
        "...#iiiiiiii#...",
        "...#iiiiiiii#...",
        "....#iiiiii#....",
        "....#iiiiii#....",
        "....########....",
        "................",
        "................",
        "................",
        "................",
    ]
    return sprite(rows, {"#": pal("#3a3a3a")[0], "i": IRON[4], "w": pal("#e8e8e8")[0], "W": pal("#ffffff")[0]})


@item("pufferfish")
def pufferfish():
    return blob("pufferfish", pal("#a07a10", "#d0a020", "#f0c840", "#fff080"), spots=pal("#3a2a10")[0])


@item("brush")
def brush():
    img = canvas()
    gi.handle(img, 0, 9.0)
    head = (gi.U >= 9.0) & (gi.U <= 11.5) & (np.abs(gi.V) <= 2.4)
    img[head, :3] = COPPER[3]
    img[head, 3] = 255
    bristle = (gi.U > 11.5) & (gi.U <= 15.5) & (np.abs(gi.V) <= 2.2)
    img[bristle, :3] = pal("#e8dcc0")[0]
    img[bristle, 3] = 255
    return outline(img)


def long_weapon(name, head_p, head_shape, shaft=gi.HANDLE):
    img = canvas()
    gi.handle(img, 0, 11.5, shaft)
    shade_mask(img, head_shape, head_p, rng_for(name))
    return outline(img)


@item("trident")
def trident():
    p = pal("#1a4a4a", "#2a7a70", "#40a898", "#70d0c0", "#b0f0e4")
    U, V = gi.U, gi.V
    prong = ((np.abs(V) <= 0.7) & (U >= 11.0) & (U <= 16.5)) | ((np.abs(V - 2.6) <= 0.7) & (U >= 11.0) & (U <= 15.0)) | \
        ((np.abs(V + 2.6) <= 0.7) & (U >= 11.0) & (U <= 15.0)) | ((np.abs(V) <= 3.2) & (U >= 10.2) & (U <= 11.4))
    return long_weapon("trident", p[1:], prong, pal("#1a4a4a", "#2a6a60", "#3a8a7a", "#4aa090"))


@item("mace")
def mace():
    img = canvas()
    gi.handle(img, 0, 9.5, pal("#3a3a42", "#5a5a64", "#7a7a86", "#9a9aa6"))
    U, V = gi.U, gi.V
    head = ((U - 12.0) ** 2 + V ** 2) <= 9.5
    shade_mask(img, head, pal("#3a3e48", "#525866", "#6a7282", "#8a92a4", "#b0b8c8"), rng_for("mace"))
    for (u, v) in ((12, 3.6), (12, -3.6), (15.3, 0.0)):
        spike = ((U - u) ** 2 + (V - v) ** 2) <= 0.7
        img[spike, :3] = pal("#c8d0e0")[0]
        img[spike, 3] = 255
    return outline(img)


def spear(material):
    p = gi.MATERIALS.get(material) if material != "netherite" else NETHERITE[1:]
    if material == "wooden":
        p = gi.MATERIALS["wooden"]
    U, V = gi.U, gi.V
    head = (np.abs(V) <= 1.8 - np.maximum(0, U - 12.5) * 0.65) & (U >= 10.0) & (U <= 15.4)
    return long_weapon(f"{material}_spear", p[1:] if len(p) > 4 else p, head)


for _m in ("wooden", "stone", "copper", "iron", "golden", "diamond", "netherite"):
    ITEMS[f"{_m}_spear"] = (lambda m=_m: spear(m))


@item("crossbow")
def crossbow():
    rows = [
        "................",
        "..##........##..",
        "..#ww######ww#..",
        "...#w#ssss#w#...",
        "....##.ss.##....",
        "......#ii#......",
        "......#ii#......",
        "......#ww#......",
        "......#ww#......",
        "......#ww#......",
        "......#ww#......",
        "......#ww#......",
        ".......##.......",
        "................",
        "................",
        "................",
    ]
    return sprite(rows, {"#": pal("#2a1a0c")[0], "w": PLANK[3], "s": pal("#d8d8d8")[0], "i": IRON[4]})


TEMPLATE = [
    "................",
    "..############..",
    "..#bbbbbbbbbb#..",
    "..#bddddddddb#..",
    "..#bd......db#..",
    "..#bd.XXXX.db#..",
    "..#bd.X..X.db#..",
    "..#bd.XXXX.db#..",
    "..#bd..XX..db#..",
    "..#bd.X..X.db#..",
    "..#bd......db#..",
    "..#bddddddddb#..",
    "..#bbbbbbbbbb#..",
    "..############..",
    "................",
    "................",
]


def template(name, color, base=pal("#3a5a5a", "#5a8080", "#7aa8a8")):
    img = from_map(TEMPLATE, {"#": base[0] * 0.5, "b": base[1], "d": base[0], "X": color})
    # Each pattern gets its own little mark from its name.
    h = zlib.crc32(name.encode())
    for k in range(4):
        x = 6 + (h >> (k * 3)) % 4
        y = 5 + (h >> (k * 3 + 12)) % 5
        put(img, x, y, scale_rgb(color, 1.25))
    return outline(img, 0.5)


SHERD = [
    "................",
    "................",
    "....######......",
    "...#cccccc##....",
    "..#cccccccccc#..",
    "..#cCCCCCCCCc#..",
    "..#cC......Cc#..",
    "..#cC......Cc#..",
    "..#cC......Cc#..",
    "..#cC......Cc#..",
    "..#cCCCCCCCCc#..",
    "...#cccccccc#...",
    "....#cccccc#....",
    ".....######.....",
    "................",
    "................",
]


def sherd(name):
    img = from_map(SHERD, {"#": pal("#4a1e12")[0], "c": pal("#a0503a")[0], "C": pal("#7a3a2a")[0]})
    # A small picture drawn from the name: dots and strokes in dark clay.
    h = zlib.crc32(name.encode())
    ink = pal("#3a1208")[0]
    for k in range(7):
        x = 6 + (h >> (k * 2)) % 4
        y = 6 + (h >> (k * 2 + 14)) % 4
        put(img, x, y, ink)
    return outline(img, 0.5)


def register_lists():
    text = open(os.path.join(ROOT, "crates", "game", "src", "smithing.rs")).read()
    pats = re.findall(r'\("([a-z]+)", "[^"]+"\),', text.split("pub const MATERIALS")[0])
    mats = [[int(v) for v in c.split(",")] for c in re.findall(r"\[(\d+, \d+, \d+)\]", text.split("pub const MATERIALS")[1].split("];")[0])]
    for i, p in enumerate(pats):
        ITEMS[f"{p}_armor_trim_smithing_template"] = (lambda p=p, i=i: template(p, np.array(mats[i % len(mats)], dtype=float)))
    ITEMS["netherite_upgrade_smithing_template"] = lambda: template("netherite_upgrade", NETHERITE[4], pal("#3a2a2a", "#5a4040", "#7a5a5a"))
    arch = open(os.path.join(ROOT, "crates", "game", "src", "archaeology.rs")).read()
    for s in re.findall(r'\("([a-z_]+)", "[^"]+"\),', arch.split("pub const SHERDS")[1].split("];")[0]):
        ITEMS[f"{s}_pottery_sherd"] = (lambda s=s: sherd(s))
    # Netherite gear through the base item drawings.
    gi.MATERIALS["netherite"] = pal("#1e1a1c", "#3a3234", "#4e4446", "#655a5c", "#80767a")
    gi.ARMOR_MATERIALS["netherite"] = gi.MATERIALS["netherite"]
    gi.ARMOR_MATERIALS["turtle"] = pal("#1e4a14", "#2e6a20", "#3e8a2e", "#58a840", "#7cc860")
    for kind in ("pickaxe", "axe", "shovel", "sword", "hoe"):
        ITEMS[f"netherite_{kind}"] = (lambda k=kind: gi.tool("netherite", k))
    for kind in ("helmet", "chestplate", "leggings", "boots"):
        ITEMS[f"netherite_{kind}"] = (lambda k=kind: gi.armor("netherite", k))
    ITEMS["turtle_helmet"] = lambda: gi.armor("turtle", "helmet")
    ITEMS["netherite_ingot"] = lambda: outline(from_map(gi.INGOT, pmap(pal("#2a2224", "#3e3436", "#544648", "#6c5e60", "#8a7c7e"))))


# ---------------------------------------------------------------------------
# Mob skins (64 x 128: the model in the top half, its 3D layers in the bottom half)
# ---------------------------------------------------------------------------


def parse_parts(mob):
    text = open(os.path.join(ROOT, "packs", "vanilla", "fase6", "mobs.ron")).read()
    body = text.split(f'name: "{mob}"')[1].split("layers:")[0]
    parts = re.findall(r"size: \((\d+), (\d+), (\d+)\), origin: \([^)]*\), uv: \((\d+), (\d+)\)", body)
    return [tuple(int(v) for v in p) for p in parts]


def faces(w, h, d, u, v):
    return {
        "top": (u + d, v, w, d),
        "bottom": (u + d + w, v, w, d),
        "right": (u, v + d, d, h),
        "front": (u + d, v + d, w, h),
        "left": (u + d + w, v + d, d, h),
        "back": (u + 2 * d + w, v + d, w, h),
    }


def paint_skin(mob, palette, features, layer_fn=None):
    skin = np.zeros((128, 64, 4), dtype=np.float64)
    rng = np.random.default_rng(zlib.crc32(mob.encode()))
    big = rng.random((128, 64))
    for idx, (w, h, d, u, v) in enumerate(parse_parts(mob)):
        for face, (x0, y0, fw, fh) in faces(w, h, d, u, v).items():
            for y in range(fh):
                for x in range(fw):
                    px, py = x0 + x, y0 + y
                    if px >= 64 or py >= 64:
                        continue
                    t = 0.55 * big[py, px] + 0.45 * (y / max(1, fh - 1) if face not in ("top", "bottom") else 0.5)
                    k = int(np.clip(t * len(palette), 0, len(palette) - 1))
                    c = palette[len(palette) - 1 - k] if face == "top" else palette[k]
                    skin[py, px, :3] = c
                    skin[py, px, 3] = 255
        if layer_fn:
            layer_fn(skin, idx, w, h, d, u, v, rng)
    for fn in features:
        fn(skin)
    return skin


def warden_features(skin):
    # Head front (uv 0,0; w 10, h 10, d 8): no eyes, a wide dark mouth, teal soul specks in the chest.
    hx, hy = 8, 8
    skin[hy + 6:hy + 9, hx + 2:hx + 8, :3] = hexrgb("#0a1a1e")
    skin[hy + 7, hx + 3:hx + 7, :3] = hexrgb("#2a5a60")
    # Chest front (uv 0,18; w 10, h 12, d 6): the ribs and the glowing souls inside.
    cx, cy = 6, 24
    for k in range(4):
        skin[cy + 2 + k * 2, cx + 1:cx + 9, :3] = hexrgb("#c8c4b0")
    skin[cy + 3:cy + 9, cx + 4:cx + 6, :3] = hexrgb("#4ae0f0")
    # Ear horns (uv 48,18): teal.
    skin[18:24, 48:56, :3] = hexrgb("#1a8a94")
    skin[18:24, 48:56, 3] = 255


def warden_layer(skin, idx, w, h, d, u, v, rng):
    # Layers of the body, head and arms: sparse teal growths and soul pockets standing off it.
    if idx not in (2, 3, 4, 5):
        return
    for face, (x0, y0, fw, fh) in faces(w, h, d, u, v).items():
        if face in ("top", "bottom"):
            continue
        for y in range(fh):
            for x in range(fw):
                if rng.random() < 0.18:
                    c = hexrgb("#0f5f6a") if rng.random() < 0.7 else hexrgb("#3ad0d8")
                    skin[64 + y0 + y, x0 + x, :3] = c
                    skin[64 + y0 + y, x0 + x, 3] = 255


def breeze_features(skin):
    # Head front (uv 0,0; 8x8x8): two pale eyes in a dark band.
    skin[8 + 3:8 + 5, 8:16, :3] = hexrgb("#2a3a6a")
    skin[8 + 3:8 + 5, 9:11, :3] = hexrgb("#e8f0ff")
    skin[8 + 3:8 + 5, 13:15, :3] = hexrgb("#e8f0ff")


def breeze_layer(skin, idx, w, h, d, u, v, rng):
    if idx not in (0, 3):
        return
    for face, (x0, y0, fw, fh) in faces(w, h, d, u, v).items():
        for y in range(fh):
            for x in range(fw):
                if (x + y) % 3 == 0 and rng.random() < 0.6:
                    skin[64 + y0 + y, x0 + x, :3] = hexrgb("#c8d4ff")
                    skin[64 + y0 + y, x0 + x, 3] = 255


def sniffer_features(skin):
    # Head front (uv 0,23; w 8, h 6, d 8): small dark eyes.
    fx, fy = 8, 31
    skin[fy + 1, fx + 1:fx + 3, :3] = hexrgb("#1a1010")
    skin[fy + 1, fx + 5:fx + 7, :3] = hexrgb("#1a1010")
    # Nose (uv 44,23): green-yellow.
    skin[23:28, 44:58, :3] = hexrgb("#c8b040")


def sniffer_layer(skin, idx, w, h, d, u, v, rng):
    # Moss and grass growing on its back: a layer on the top and upper sides of the body.
    if idx != 6:
        return
    for face, (x0, y0, fw, fh) in faces(w, h, d, u, v).items():
        rows = range(fh) if face == "top" else range(min(3, fh))
        if face == "bottom":
            continue
        for y in rows:
            for x in range(fw):
                if rng.random() < 0.55:
                    skin[64 + y0 + y, x0 + x, :3] = hexrgb("#4a8a2a") if rng.random() < 0.7 else hexrgb("#7ac04a")
                    skin[64 + y0 + y, x0 + x, 3] = 255


SKINS = {
    "warden": (pal("#0a2a30", "#0f3b44", "#164e58", "#1c606a", "#24707a"), [warden_features], warden_layer),
    "breeze": (pal("#5a6aa8", "#7a8ac8", "#9aaae0", "#bccaf4", "#dce4ff"), [breeze_features], breeze_layer),
    "sniffer": (pal("#6a2418", "#8a3020", "#a8402a", "#c05436", "#d46a48"), [sniffer_features], sniffer_layer),
}


def main():
    register_simple()
    register_potions()
    register_lists()
    sheet = sys.argv[1] if len(sys.argv) > 1 else None
    os.makedirs(BLOCK_DIR, exist_ok=True)
    os.makedirs(ITEM_DIR, exist_ok=True)
    imgs = []
    for name, fn in BLOCKS.items():
        img = to_image(fn())
        img.save(os.path.join(BLOCK_DIR, name + ".png"))
        imgs.append(img)
    for name, fn in ITEMS.items():
        img = to_image(fn())
        img.save(os.path.join(ITEM_DIR, name + ".png"))
        imgs.append(img)
    for mob, (p, feats, layer) in SKINS.items():
        skin = paint_skin(mob, p, feats, layer)
        Image.fromarray(np.clip(np.rint(skin), 0, 255).astype(np.uint8), "RGBA").save(os.path.join(ENTITY_DIR, mob + ".png"))
    if sheet:
        contact_sheet(imgs, scale=4, gap=2, cols=24).save(sheet)
    print(f"wrote {len(BLOCKS)} block textures, {len(ITEMS)} item icons, {len(SKINS)} skins")


if __name__ == "__main__":
    main()
