#!/usr/bin/env python3
"""Item icons for what the wild animals drop: venison, rabbit and rabbit hide
(16x16, our own art).  Same helpers and look as tools/gen_items.py: small
hand-written pixel maps, short dark->light palettes and the automatic dark
outline.

Usage:
    python3 tools/gen_animal_items.py

Output: packs/vanilla/textures/item/<name>.png
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from gen_items import ITEM_DIR, from_map, outline, pmap  # noqa: E402
from gen_textures import N, hexrgb, pal, to_image  # noqa: E402

# A haunch of venison: a lean, dark cut with a thin rim of fat and a bone end.
HAUNCH = [
    "................",
    "................",
    "................",
    "......ff........",
    "....ff4444f.....",
    "...f44454444....",
    "..f444544w443...",
    "..f44444444433..",
    "..f44w444454333.",
    "...f444444433bb.",
    "....3444443332bb",
    ".....33333322.b.",
    ".......2222.....",
    "................",
    "................",
    "................",
]


def haunch(p, fat, marb):
    cols = pmap(p)
    cols["f"] = fat
    cols["w"] = marb
    cols["b"] = hexrgb("#ecead8")
    return outline(from_map(HAUNCH, cols))


def venison():
    return haunch(pal("#4a0e14", "#6e1a20", "#8e2a2c", "#a63a38", "#c0564e"),
                  hexrgb("#e8d0c4"), hexrgb("#c46a64"))


def cooked_venison():
    return haunch(pal("#2a160a", "#422412", "#5c341a", "#744424", "#8e5830"),
                  hexrgb("#a8784a"), hexrgb("#9a6438"))


# A small dressed rabbit lying on its side: long body, folded legs.
RABBIT = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "...b.......55...",
    "..bb4.....5444..",
    "...4444444444f..",
    "..444454444443..",
    ".444444444w433..",
    ".b3444444443332.",
    "..b3333333332.b.",
    "...2..22222..b..",
    "................",
    "................",
    "................",
]


def rabbit_item(p, fat):
    cols = pmap(p)
    cols["f"] = fat
    cols["w"] = fat
    cols["b"] = hexrgb("#e6e0d0")
    return outline(from_map(RABBIT, cols))


def rabbit():
    return rabbit_item(pal("#7a3a38", "#a0524c", "#c06e66", "#d88a80", "#eeaaa0"),
                       hexrgb("#f4dcd4"))


def cooked_rabbit():
    return rabbit_item(pal("#3e220e", "#5e3618", "#7e4c24", "#9a6232", "#b47a42"),
                       hexrgb("#c89a60"))


def rabbit_hide():
    rows = [
        "................",
        "................",
        "....4......4....",
        "....44....44....",
        ".....444444.....",
        "....44454444....",
        "...4444444443...",
        "...4445444433...",
        "...4444444433...",
        "....44444333....",
        "....4443w333....",
        "...44.3333..3...",
        "...3........2...",
        "................",
        "................",
        "................",
    ]
    p = pal("#4e3e2c", "#6a5640", "#86705a", "#a08a72", "#b8a48c")
    cols = pmap(p)
    cols["w"] = hexrgb("#e8e0d2")
    return outline(from_map(rows, cols))


ITEMS = {
    "venison": venison, "cooked_venison": cooked_venison,
    "rabbit": rabbit, "cooked_rabbit": cooked_rabbit, "rabbit_hide": rabbit_hide,
}


def main():
    os.makedirs(ITEM_DIR, exist_ok=True)
    for name, fn in ITEMS.items():
        img = to_image(fn())
        assert img.size == (N, N)
        img.save(os.path.join(ITEM_DIR, name + ".png"))
    print(f"wrote {len(ITEMS)} items to {ITEM_DIR}")


if __name__ == "__main__":
    main()
