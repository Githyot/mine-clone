#!/usr/bin/env python3
"""Data of the mod content pack: writes packs/vanilla/mods/{blocks,items,recipes}.ron.

Each mod adds its blocks, items and recipes through the small helpers below, so families of
blocks (a whole wood set, a stone set) come from one line. Edit this file, then run:

    python3 tools/gen_mod_data.py
"""

import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "packs", "vanilla", "mods")

blocks = []
items = []
shaped = []
shapeless = []
smelting = []
tags = {}


def q(s):
    return '"' + s.replace('"', '\\"') + '"'


def opt(v):
    return f"Some({v})"


def B(name, display, **kw):
    """One block. Keyword arguments are written as they are (RON values as strings)."""
    parts = [f"name: {q(name)}", f"display_name: Some({q(display)})"]
    for k, v in kw.items():
        parts.append(f"{k}: {v}")
    blocks.append("(" + ", ".join(parts) + ")")


def tex(**faces):
    return "(" + ", ".join(f"{k}: Some({q(v)})" for k, v in faces.items()) + ")"


def drops(*entries):
    out = []
    for e in entries:
        if isinstance(e, str):
            out.append(f"(item: {q(e)})")
        else:
            name, mn, mx, *ch = e
            s = f"(item: {q(name)}, min: {mn}, max: {mx}"
            if ch:
                s += f", chance: {ch[0]}"
            out.append(s + ")")
    return "Some([" + ", ".join(out) + "])"


def It(name, display, **kw):
    parts = [f"name: {q(name)}", f"display_name: Some({q(display)})"]
    for k, v in kw.items():
        parts.append(f"{k}: {v}")
    items.append("(" + ", ".join(parts) + ")")


def tool(kind, tier, speed, damage, durability):
    return f"Some((kind: {kind}, tier: {tier}, speed: {speed}, damage: {damage}, durability: {durability}))"


def armor(slot, points, durability):
    return f"Some((slot: {slot}, points: {points}, durability: {durability}))"


def food(hunger, saturation):
    return f"Some((hunger: {hunger}, saturation: {saturation}))"


def R(pattern, key, result, count=1):
    keys = ", ".join(f"'{k}': {q(v)}" for k, v in key.items())
    pat = ", ".join(q(r) for r in pattern)
    c = f", count: {count}" if count != 1 else ""
    shaped.append(f"(pattern: [{pat}], key: {{{keys}}}, result: {q(result)}{c})")


def S(ingredients, result, count=1):
    ing = ", ".join(q(i) for i in ingredients)
    c = f", count: {count}" if count != 1 else ""
    shapeless.append(f"(ingredients: [{ing}], result: {q(result)}{c})")


def F(inp, result, xp=0.1):
    smelting.append(f"(input: {q(inp)}, result: {q(result)}, xp: {xp})")


def tag(name, *members):
    tags.setdefault(name, []).extend(members)


# --------------------------------------------------------------------------
# Serene Seasons
# --------------------------------------------------------------------------

SEASONS = [("spring", "Primavera"), ("summer", "Verão"), ("autumn", "Outono"), ("winter", "Inverno")]
for s, pt in SEASONS:
    for on in (False, True):
        n = f"season_sensor_{s}" + ("_on" if on else "")
        extra = {} if (s == "spring" and not on) else {"in_inventory": "false"}
        B(n, f"Sensor de Estação ({pt})", hardness="0.6", tool="Some(Axe)", height="Some(6)", shape="Slab",
          textures=tex(top=f"season_sensor_{s}" + ("_on" if on else ""), side="season_sensor_side", bottom="oak_planks"),
          drops=drops("season_sensor_spring"), **extra)
It("calendar", "Calendário")
R(["PPP", "PRP", "PPP"], {"P": "#planks", "R": "redstone"}, "calendar")
R(["GGG", "QCQ", "WWW"], {"G": "glass", "Q": "quartz", "C": "calendar", "W": "oak_slab"}, "season_sensor_spring")


# --------------------------------------------------------------------------
# Write
# --------------------------------------------------------------------------


def write(name, head, body):
    with open(os.path.join(OUT, name), "w") as f:
        f.write(head)
        f.write(body)


def main():
    os.makedirs(OUT, exist_ok=True)
    note = "// Made by tools/gen_mod_data.py: edit that file and run it again.\n"
    write("blocks.ron", note, "(blocks: [\n" + "".join(f"    {b},\n" for b in blocks) + "])\n")
    write("items.ron", note, "(items: [\n" + "".join(f"    {i},\n" for i in items) + "])\n")
    t = "".join(f"        {q(k)}: [{', '.join(q(m) for m in v)}],\n" for k, v in tags.items())
    body = "(\n    tags: {\n" + t + "    },\n"
    body += "    shaped: [\n" + "".join(f"        {r},\n" for r in shaped) + "    ],\n"
    body += "    shapeless: [\n" + "".join(f"        {r},\n" for r in shapeless) + "    ],\n"
    body += "    smelting: [\n" + "".join(f"        {r},\n" for r in smelting) + "    ],\n)\n"
    write("recipes.ron", note, body)
    print(f"{len(blocks)} blocks, {len(items)} items, {len(shaped) + len(shapeless) + len(smelting)} recipes")


if __name__ == "__main__":
    main()
