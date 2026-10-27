#!/usr/bin/env python3
"""Textures of the mod content pack: our own 16x16 art for the blocks and items of the mods HP
asked for. Each mod has a module in tools/modart/ registering its textures with @block/@item;
nothing is read from any mod or game file.

Usage:
    python3 tools/gen_mod_textures.py [contact_sheet.png]
"""

import importlib
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from modart import lib  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BLOCK_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "block")
ITEM_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "item")
MODULES = os.path.join(os.path.dirname(os.path.abspath(__file__)), "modart")


def main():
    for f in sorted(os.listdir(MODULES)):
        if f.endswith(".py") and f not in ("lib.py", "__init__.py"):
            importlib.import_module("modart." + f[:-3])
    sheet = sys.argv[1] if len(sys.argv) > 1 else None
    imgs = []
    for out, reg in ((BLOCK_DIR, lib.BLOCKS), (ITEM_DIR, lib.ITEMS)):
        os.makedirs(out, exist_ok=True)
        for name, fn in reg.items():
            img = lib.T.to_image(fn())
            assert img.size == (lib.N, lib.N), name
            img.save(os.path.join(out, name + ".png"))
            imgs.append(img)
    if sheet:
        lib.T.contact_sheet(imgs, scale=6, gap=2, cols=12).save(sheet)
    print(f"wrote {len(lib.BLOCKS)} block and {len(lib.ITEMS)} item textures")


if __name__ == "__main__":
    main()
