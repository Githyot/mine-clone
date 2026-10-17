#!/usr/bin/env python3
"""Textures of the mod content pack (packs/vanilla/mods/): our own 16x16 art for the blocks and
items of the mods HP asked for (seasons, the Aether, the Otherside, aircraft, Brazilian woods...).

It reuses the helpers of gen_textures.py (blocks) and gen_items.py (items) so everything keeps
the same classic look. Nothing is read from any mod or game file.

Usage:
    python3 tools/gen_mod_textures.py [contact_sheet.png]
"""

import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_textures as T  # noqa: E402
import gen_items as I  # noqa: E402

N = 16
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BLOCK_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "block")
ITEM_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "item")

pal, put, line, canvas, from_rgb, shade = T.pal, T.put, T.line, T.canvas, T.from_rgb, T.shade
rng_for, value_noise, fbm, normalize, hexrgb = T.rng_for, T.value_noise, T.fbm, T.normalize, T.hexrgb
from_map, outline, pmap = I.from_map, I.outline, I.pmap

BLOCKS = {}
ITEMS = {}


def block(name):
    def deco(fn):
        BLOCKS[name] = fn
        return fn
    return deco


def item(name):
    def deco(fn):
        ITEMS[name] = fn
        return fn
    return deco


# --------------------------------------------------------------------------
# Serene Seasons: season sensor and calendar
# --------------------------------------------------------------------------

SEASON_COLORS = {
    "spring": pal("#2f6a2a", "#4f9a3a", "#86c860", "#c4ec9a"),
    "summer": pal("#8a5a0a", "#d49a1a", "#f4cc3a", "#fff08a"),
    "autumn": pal("#6a240e", "#b4501a", "#e0802a", "#f8b45a"),
    "winter": pal("#3a5a7a", "#6a92b8", "#a8c8e4", "#eef6ff"),
}

SENSOR_FRAME = pal("#3a2a18", "#54402a", "#6c5438", "#846a48")


def sensor_top(season, on):
    rng = rng_for("season_sensor_" + season)
    img = from_rgb(shade(value_noise(rng, 4) * 0.6 + 0.2, SENSOR_FRAME, 0.4))
    p = SEASON_COLORS[season]
    # A round dial of glass in a wooden frame, the season's colour inside.
    for y in range(N):
        for x in range(N):
            d = np.hypot(x - 7.5, y - 7.5)
            if d < 5.6:
                k = 3 if (x + y) % 5 == 0 else (2 if d < 3.2 else 1)
                if not on:
                    k = max(0, k - 1)
                put(img, x, y, p[k])
            elif d < 6.6:
                put(img, x, y, SENSOR_FRAME[0])
    # The emblem: a leaf (spring), sun (summer), falling leaf (autumn), flake (winter).
    em = {
        "spring": ["...", ".#.", "###", ".#."],
        "summer": ["#.#", ".#.", "#.#"],
        "autumn": ["##.", "###", ".##"],
        "winter": ["#.#", ".#.", "#.#", ".#."],
    }[season]
    c = (255, 255, 230) if on else tuple(int(v) for v in p[3])
    for j, row in enumerate(em):
        for i, ch in enumerate(row):
            if ch == "#":
                put(img, 6 + i, 6 + j, c)
    return img


def sensor_side():
    rng = rng_for("season_sensor_side")
    img = from_rgb(shade(value_noise(rng, 8, 2) * 0.6 + 0.2, SENSOR_FRAME, 0.4))
    rgb = img[..., :3]
    rgb[0:2, :] = T.scale_rgb(rgb[0:2, :], 1.15)
    rgb[5, :] = SENSOR_FRAME[0]
    return img


for _s in SEASON_COLORS:
    BLOCKS[f"season_sensor_{_s}"] = (lambda s=_s: sensor_top(s, False))
    BLOCKS[f"season_sensor_{_s}_on"] = (lambda s=_s: sensor_top(s, True))
BLOCKS["season_sensor_side"] = sensor_side


@item("calendar")
def calendar():
    rows = [
        "................",
        "..1..1..1..1....",
        ".2222222222222..",
        ".2333333333332..",
        ".2333333333332..",
        ".2444444444442..",
        ".2454545454542..",
        ".2444444444442..",
        ".2454565454542..",
        ".2444444444442..",
        ".2454545454542..",
        ".2444444444442..",
        ".2454545444442..",
        ".2444444444442..",
        ".2222222222222..",
        "................",
    ]
    cols = {
        "1": hexrgb("#5a5a5a"),
        "2": hexrgb("#6a4a2a"),
        "3": hexrgb("#b0302a"),
        "4": hexrgb("#efe6d0"),
        "5": hexrgb("#8a7a64"),
        "6": hexrgb("#c03a2a"),
    }
    return outline(from_map(rows, cols))


# --------------------------------------------------------------------------
# Main
# --------------------------------------------------------------------------


def main():
    sheet = sys.argv[1] if len(sys.argv) > 1 else None
    imgs = []
    for out, reg in ((BLOCK_DIR, BLOCKS), (ITEM_DIR, ITEMS)):
        os.makedirs(out, exist_ok=True)
        for name, fn in reg.items():
            img = T.to_image(fn())
            assert img.size == (N, N), name
            img.save(os.path.join(out, name + ".png"))
            imgs.append(img)
    if sheet:
        T.contact_sheet(imgs, scale=6, gap=2, cols=12).save(sheet)
    print(f"wrote {len(BLOCKS)} block and {len(ITEMS)} item textures")


if __name__ == "__main__":
    main()
