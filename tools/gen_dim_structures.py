#!/usr/bin/env python3
"""Buildings of the mod pack's dimensions, written to packs/vanilla/mods/dims_structures.ron:
the Aether's bronze, silver and gold dungeons and its ruins, and the Otherside's temple and
outposts. Each is drawn here with a few box helpers; the creatures inside are spawned once
(crates/game/src/mods/dwellers.rs).

    python3 tools/gen_dim_structures.py
"""

import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "packs", "vanilla", "mods", "dims_structures.ron")


class Build:
    def __init__(self, w, h, d):
        self.w, self.h, self.d = w, h, d
        self.g = [[[" "] * w for _ in range(d)] for _ in range(h)]

    def set(self, x, y, z, c):
        if 0 <= x < self.w and 0 <= y < self.h and 0 <= z < self.d:
            self.g[y][z][x] = c

    def box(self, x0, y0, z0, x1, y1, z1, wall, inside=".", floor=None, roof=None):
        """A hollow box from corner to corner (inclusive)."""
        for y in range(y0, y1 + 1):
            for z in range(z0, z1 + 1):
                for x in range(x0, x1 + 1):
                    edge = x in (x0, x1) or z in (z0, z1)
                    if y == y0:
                        c = floor or wall
                    elif y == y1:
                        c = roof or wall
                    else:
                        c = wall if edge else inside
                    self.set(x, y, z, c)

    def fill(self, x0, y0, z0, x1, y1, z1, c):
        for y in range(y0, y1 + 1):
            for z in range(z0, z1 + 1):
                for x in range(x0, x1 + 1):
                    self.set(x, y, z, c)

    def layers(self):
        return [["".join(r) for r in layer] for layer in self.g]


def ron(name, biomes, spacing, chance, palette, build, loot=(), mobs=(), sink=0, foundation=None, rough=4):
    pal = ", ".join(f"('{c}', \"{b}\")" for c, b in palette)
    out = f"    (\n        name: \"{name}\",\n        biomes: [{', '.join(chr(34) + b + chr(34) for b in biomes)}],\n"
    out += f"        spacing: {spacing},\n        chance: {chance},\n        sink: {sink},\n        rough: {rough},\n"
    if foundation:
        out += f"        foundation: Some(\"{foundation}\"),\n"
    out += f"        palette: [{pal}],\n        layers: [\n"
    for layer in build.layers():
        out += "            [" + ", ".join(f'"{r}"' for r in layer) + "],\n"
    out += "        ],\n"
    if loot:
        out += "        loot: [" + ", ".join(f"(item: \"{i}\", min: {a}, max: {b}, chance: {c})" for i, a, b, c in loot) + "],\n"
    if mobs:
        out += "        mobs: [" + ", ".join(f"(\"{m}\", {n})" for m, n in mobs) + "],\n"
    return out + "    ),\n"


AETHER = ["Aether", "Golden Grove"]
OTHER = ["Deeplands", "Echoing Forest", "Overcast Columns", "Blooming Caverns"]
items = []

# Bronze dungeon: a squat block of carved stone half sunk into an island, sentry stones
# glowing in its walls, the Slider waiting in the hall.
b = Build(15, 8, 15)
b.box(0, 0, 0, 14, 7, 14, "C", floor="C", roof="C")
for x in range(2, 14, 3):
    for y in (2, 4):
        b.set(x, y, 0, "S")
        b.set(x, y, 14, "S")
b.box(4, 0, 4, 10, 5, 10, "C", floor="C", roof="C")
b.fill(7, 1, 4, 7, 2, 4, ".")
b.fill(7, 1, 0, 7, 2, 0, ".")
b.set(7, 1, 7, "K")
b.set(5, 1, 5, "S")
b.set(9, 1, 9, "S")
items.append(ron("bronze_dungeon", AETHER, 18, 0.55,
                 [("C", "carved_stone"), ("S", "sentry_stone"), ("K", "chest")], b,
                 loot=[("zanite_gemstone", 2, 6, 0.8), ("ambrosium_shard", 3, 8, 0.9), ("gravitite_ore", 1, 3, 0.5),
                       ("cloud_parachute", 1, 1, 0.5), ("golden_amber", 2, 5, 0.6), ("moa_egg", 1, 1, 0.4)],
                 mobs=[("slider", 1), ("sentry", 3)], sink=2, foundation="holystone"))

# Silver dungeon: a pale temple of angelic stone with glowing pillars and a raised throne.
b = Build(19, 11, 19)
b.box(0, 0, 0, 18, 10, 18, "A", floor="A", roof="L")
for x in (3, 15):
    for z in (3, 15):
        b.fill(x, 1, z, x, 9, z, "L")
b.fill(7, 1, 13, 11, 1, 16, "A")
b.fill(8, 2, 15, 10, 2, 16, "A")
b.fill(8, 1, 0, 10, 4, 0, ".")
for x in range(1, 18, 4):
    b.set(x, 5, 0, "G")
    b.set(x, 5, 18, "G")
b.set(9, 3, 16, "K")
items.append(ron("silver_dungeon", ["Aether"], 26, 0.5,
                 [("A", "angelic_stone"), ("L", "light_angelic_stone"), ("G", "glass"), ("K", "chest")], b,
                 loot=[("victory_medal", 2, 5, 0.9), ("valkyrie_helmet", 1, 1, 0.3), ("valkyrie_boots", 1, 1, 0.3),
                       ("golden_parachute", 1, 1, 0.5), ("enchanted_gravitite", 1, 3, 0.5), ("blue_berry", 4, 10, 0.8)],
                 mobs=[("valkyrie_queen", 1), ("valkyrie", 4)], foundation="holystone", rough=6))

# Gold dungeon: a hall of hellfire stone sunk deep in a golden grove's island, lit like embers.
b = Build(17, 10, 17)
b.box(0, 0, 0, 16, 9, 16, "H", floor="H", roof="H")
for x in range(2, 16, 3):
    for z in range(2, 16, 3):
        b.set(x, 1, z, "E")
        b.set(x, 8, z, "E")
b.fill(7, 1, 0, 9, 3, 0, ".")
b.fill(7, 4, 1, 9, 9, 1, "H")
b.set(8, 1, 14, "K")
items.append(ron("gold_dungeon", ["Golden Grove"], 30, 0.6,
                 [("H", "hellfire_stone"), ("E", "light_hellfire_stone"), ("K", "chest")], b,
                 loot=[("phoenix_feather", 3, 6, 0.9), ("enchanted_gravitite", 2, 5, 0.8), ("golden_amber", 4, 10, 0.9),
                       ("phoenix_helmet", 1, 1, 0.25), ("enchanted_berry", 2, 4, 0.7)],
                 mobs=[("sun_spirit", 1), ("fire_minion", 3)], sink=5, foundation="holystone", rough=5))

# Ruins: broken holystone walls round a chest.
b = Build(9, 4, 9)
for x in range(9):
    for z in range(9):
        if x in (0, 8) or z in (0, 8):
            hgt = (x * 7 + z * 3) % 4
            for y in range(hgt):
                b.set(x, y, z, "B" if (x + y + z) % 3 else "M")
b.set(4, 0, 4, "K")
items.append(ron("aether_ruin", AETHER, 10, 0.5, [("B", "holystone_bricks"), ("M", "mossy_holystone"), ("K", "chest")], b,
                 loot=[("ambrosium_shard", 2, 6, 0.9), ("skyroot_stick", 2, 6, 0.6), ("zanite_gemstone", 1, 3, 0.5),
                       ("blue_berry", 2, 6, 0.7)], mobs=[("swet", 2)]))

# Otherside: a temple of gloomslate bricks with the Stalker inside, and small sculk outposts.
b = Build(21, 13, 21)
b.box(0, 0, 0, 20, 12, 20, "G", floor="G", roof="G")
for x in (4, 16):
    for z in (4, 16):
        b.fill(x, 1, z, x, 11, z, "P")
b.fill(9, 1, 0, 11, 5, 0, ".")
b.fill(8, 1, 8, 12, 1, 12, "S")
b.set(10, 2, 18, "K")
for x in range(2, 20, 4):
    b.set(x, 6, 0, "L")
    b.set(x, 6, 20, "L")
items.append(ron("otherside_temple", ["Deeplands", "Echoing Forest"], 28, 0.6,
                 [("G", "gloomslate_bricks"), ("P", "polished_gloomslate"), ("S", "sculk_grime"), ("L", "echo_lantern"), ("K", "chest")], b,
                 loot=[("echo_crystal", 3, 8, 0.9), ("heart_of_the_deep", 1, 1, 0.2), ("sculk_bone", 2, 5, 0.7),
                       ("resonarium", 2, 4, 0.6)], mobs=[("otherside_stalker", 1), ("shattered", 3)], foundation="gloomslate",
                 rough=6))
b = Build(7, 6, 7)
b.box(0, 0, 0, 6, 5, 6, "G", floor="G", roof="P")
b.fill(3, 1, 0, 3, 2, 0, ".")
b.set(3, 1, 5, "K")
b.set(1, 3, 1, "L")
items.append(ron("otherside_outpost", OTHER, 12, 0.5,
                 [("G", "gloomslate_bricks"), ("P", "polished_gloomslate"), ("L", "echo_lantern"), ("K", "chest")], b,
                 loot=[("echo_crystal", 1, 3, 0.7), ("sculk_bone", 1, 3, 0.6), ("glow_berries", 2, 5, 0.5)],
                 mobs=[("shattered", 1)], foundation="gloomslate"))

with open(OUT, "w") as f:
    f.write("// Made by tools/gen_dim_structures.py: edit that script and run it again.\n(structures: [\n")
    f.write("".join(items))
    f.write("])\n")
print(f"wrote {len(items)} structures")
