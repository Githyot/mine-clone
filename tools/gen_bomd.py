#!/usr/bin/env python3
"""Bosses of Mass Destruction idea: the Nether Gauntlet, a huge floating armoured fist with an
eye in its palm. Writes its entry into packs/vanilla/mods/mobs.ron (between the bomd marker
comments, after the mod creatures) and its 64x128 skin into packs/vanilla/textures/entity/.
Shapes, colours and stats are ours; nothing is read from the mod.

    python3 tools/gen_bomd.py
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from dim_mobs_spec import Model, look  # noqa: E402
from gen_dim_mobs import OUT_DIR, render, ron  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MOBS = os.path.join(ROOT, "packs", "vanilla", "mods", "mobs.ron")
BEGIN = "    // ---- bomd (tools/gen_bomd.py) ----\n"
END = "    // ---- end of bomd ----\n"

PLATE = look("plate", ["#1a1012", "#2a1a1c", "#3c2628", "#523436", "#6a4446"], pattern="plate")
GOLD = look("gold", ["#5a3a08", "#86580e", "#b07c18", "#d4a02a", "#f0c850"], pattern="plate")
EYE = look("eye", ["#3a0804", "#6a1006", "#a0220a", "#d83e10", "#ff7a2a"], eyes=((255, 220, 120), "glow"), pattern="flame")


def gauntlet():
    m = Model()
    # Cuff and the back of the hand.
    m.box((10, 6, 8), (-5, 2, -4), GOLD)
    m.box((14, 16, 6), (-7, 8, -3), PLATE)
    # Four fingers that curl (two swing groups) and a thumb.
    for k, x in enumerate((-7, -3, 1, 4)):
        m.box((3, 9, 3), (x, 24, -2), PLATE, f"Arm({k % 2})", (x + 1, 24, 0))
    m.box((3, 8, 3), (7, 12, -1), GOLD, "Arm(1)", (8, 18, 0))
    # The burning eye set in the palm, looking at you.
    m.box((6, 6, 1), (-3, 13, 3), EYE, "Head", (0, 16, 0))
    m.pack()
    return m


def main():
    m = dict(
        name="nether_gauntlet", display="Manopla do Nether", kind="Hostile", health=520, speed=2.2,
        size=(2.0, 4.2), spawn="Never", model=gauntlet(),
        kw=dict(drops=[("blaze_rod", 6, 12), ("gold_block", 2, 4), ("magma_block", 4, 8), ("life_crystal", 1, 1)],
                xp=180, attack=11, scale=2.0, boss=True, flies=True),
    )
    entry = ron(m)
    text = open(MOBS).read()
    if BEGIN in text:
        a = text.index(BEGIN)
        b = text.index(END) + len(END)
        text = text[:a] + text[b:]
    marker = "    // ---- end of creatures ----\n"
    at = text.index(marker) + len(marker)
    text = text[:at] + BEGIN + entry + END + text[at:]
    open(MOBS, "w").write(text)
    os.makedirs(OUT_DIR, exist_ok=True)
    render(m["name"], m["model"]).save(os.path.join(OUT_DIR, "nether_gauntlet.png"))
    print("wrote nether_gauntlet")


if __name__ == "__main__":
    main()
