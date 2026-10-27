#!/usr/bin/env python3
"""Data of the mod content pack: writes packs/vanilla/mods/{blocks,items,recipes}.ron.

Each mod has its own module in tools/moddata/ that adds its blocks, items and recipes through
the helpers in tools/moddata/lib.py, so families of blocks (a whole wood set, a stone set) come
from one line. Modules are read in name order (block ids follow that order). Run:

    python3 tools/gen_mod_data.py
"""

import importlib
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from moddata import lib  # noqa: E402
from moddata.lib import q  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "packs", "vanilla", "mods")
MODULES = os.path.join(os.path.dirname(os.path.abspath(__file__)), "moddata")


def write(name, head, body):
    with open(os.path.join(OUT, name), "w") as f:
        f.write(head)
        f.write(body)


def main():
    for f in sorted(os.listdir(MODULES)):
        if f.endswith(".py") and f not in ("lib.py", "__init__.py"):
            importlib.import_module("moddata." + f[:-3])
    os.makedirs(OUT, exist_ok=True)
    note = "// Made by tools/gen_mod_data.py from tools/moddata/: edit those and run it again.\n"
    write("blocks.ron", note, "(blocks: [\n" + "".join(f"    {b},\n" for b in lib.blocks) + "])\n")
    write("items.ron", note, "(items: [\n" + "".join(f"    {i},\n" for i in lib.items) + "])\n")
    t = "".join(f"        {q(k)}: [{', '.join(q(m) for m in v)}],\n" for k, v in lib.tags.items())
    body = "(\n    tags: {\n" + t + "    },\n"
    body += "    shaped: [\n" + "".join(f"        {r},\n" for r in lib.shaped) + "    ],\n"
    body += "    shapeless: [\n" + "".join(f"        {r},\n" for r in lib.shapeless) + "    ],\n"
    body += "    smelting: [\n" + "".join(f"        {r},\n" for r in lib.smelting) + "    ],\n)\n"
    write("recipes.ron", note, body)
    n = len(lib.shaped) + len(lib.shapeless) + len(lib.smelting)
    print(f"{len(lib.blocks)} blocks, {len(lib.items)} items, {n} recipes")


if __name__ == "__main__":
    main()
