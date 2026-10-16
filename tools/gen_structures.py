"""Writes packs/vanilla/structures.ron: our own small buildings (ideas from Towns and Towers,
village mods and YUNG's), laid out here in code so the stairs, roofs and spirals line up.

Layers go bottom to top; each layer is rows from north (-z) to south (+z), each row west to
east. "." is air, " " keeps the terrain. Facing variants: 0 north, 1 east, 2 south, 3 west
(stairs and roofs rise toward their facing; doors add 8 for the upper half)."""
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "packs" / "vanilla" / "structures.ron"


def grid(w, d, ch=" "):
    return [[ch] * w for _ in range(d)]


def rows(g):
    return ["".join(r) for r in g]


def house(wood, log, roof, biomes, name):
    """A cottage: cobblestone floor, log corners, plank walls, glass windows, a door to the
    south and a gable roof with an overhang."""
    W, D = 7, 9  # walls stand on z 1..7, the roof hangs over z 0 and z 8
    layers = []
    floor = grid(W, D)
    for z in range(1, 8):
        for x in range(W):
            floor[z][x] = "C"
    layers.append(floor)
    for y in (1, 2, 3):
        g = grid(W, D)
        for z in range(1, 8):
            for x in range(W):
                edge_x = x in (0, W - 1)
                edge_z = z in (1, 7)
                if edge_x and edge_z:
                    g[z][x] = "L"
                elif edge_x or edge_z:
                    g[z][x] = "P"
                else:
                    g[z][x] = "."
        # Door in the middle of the south wall, windows on the other three.
        if y == 1:
            g[7][3] = "d"
            g[2][1] = "T"
            g[2][2] = "F"
            g[2][5] = "K"
            g[6][5] = "t"
        if y == 2:
            g[7][3] = "u"
            g[1][3] = "G"
            g[4][0] = "G"
            g[4][W - 1] = "G"
        # Clear the porch in front of the door.
        if y in (1, 2):
            g[8][3] = "."
        layers.append(g)
    # Gable roof: each step in, one layer up; ends filled with planks.
    for k in range(5):
        g = grid(W, D)
        y_rows = (k, D - 1 - k)
        for x in range(W):
            if k < 4:
                g[y_rows[0]][x] = "n"  # north slope rises to the south
                g[y_rows[1]][x] = "s"  # south slope rises to the north
            else:
                g[4][x] = "r"
        if k < 4:
            for z in range(k + 1, D - 1 - k):
                g[z][0] = "P"
                g[z][W - 1] = "P"
                for x in range(1, W - 1):
                    g[z][x] = "."
        layers.append(g)
    return {
        "name": name,
        "biomes": biomes,
        "spacing": 7,
        "chance": 0.45,
        "foundation": "cobblestone",
        "palette": {
            "C": "cobblestone", "L": log, "P": wood, "G": "glass_pane", "d": "oak_door:2", "u": "oak_door:10",
            "T": "crafting_table", "F": "furnace", "K": "chest", "t": "torch",
            "n": f"{roof}:2", "s": roof, "r": "oak_slab",
        },
        "layers": [rows(l) for l in layers],
        "loot": [("bread_or_apple", "apple", 1, 4, 0.8), ("", "coal", 2, 6, 0.6), ("", "oak_sapling", 1, 2, 0.4), ("", "iron_ingot", 1, 2, 0.25), ("", "cooked_porkchop", 1, 3, 0.3)],
    }


def tower():
    """A stone brick watchtower with a spiral stair inside and a lookout on top."""
    W = 5
    H = 12
    ring = [(1, 1), (2, 1), (3, 1), (3, 2), (3, 3), (2, 3), (1, 3), (1, 2)]
    facing = []
    for i, (x, z) in enumerate(ring):
        nx, nz = ring[(i + 1) % len(ring)]
        dx, dz = nx - x, nz - z
        facing.append({(0, -1): 0, (1, 0): 1, (0, 1): 2, (-1, 0): 3}[(dx, dz)])
    layers = []
    # Platform is 7x7 around the 5x5 shaft: shift the shaft by 1.
    S = 7
    off = 1
    base = grid(S, S)
    for z in range(W):
        for x in range(W):
            base[z + off][x + off] = "B"
    layers.append(base)
    for y in range(1, H):
        g = grid(S, S)
        for z in range(W):
            for x in range(W):
                wall = x in (0, W - 1) or z in (0, W - 1)
                g[z + off][x + off] = "B" if wall else "."
        g[2 + off][2 + off] = "B"  # the stair winds round a central pillar
        k = (y - 1) % len(ring)
        sx, sz = ring[k]
        g[sz + off][sx + off] = "0123"[facing[k]]
        if y in (1, 2):
            g[W - 1 + off][2 + off] = "."  # doorway in the south wall
        if y in (4, 8):
            g[0 + off][2 + off] = "w"
            g[2 + off][0 + off] = "w"
            g[2 + off][W - 1 + off] = "w"
        if y == 6:
            g[2 + off][2 + off] = "B"
        layers.append(g)
    # Lookout floor: slabs over the whole 7x7 with the stair coming up through it.
    top = grid(S, S)
    for z in range(S):
        for x in range(S):
            top[z][x] = "B" if 0 < x < S - 1 and 0 < z < S - 1 else "b"
    k = (H - 1) % len(ring)
    sx, sz = ring[k]
    top[sz + off][sx + off] = "."  # head room above the last stair inside
    k2 = H % len(ring)
    sx2, sz2 = ring[k2]
    top[sz2 + off][sx2 + off] = "0123"[facing[k2]]
    layers.append(top)
    rail = grid(S, S)
    for z in range(S):
        for x in range(S):
            if x in (0, S - 1) or z in (0, S - 1):
                rail[z][x] = "W"
            else:
                rail[z][x] = "."
    rail[1][1] = "K"
    rail[S - 2][S - 2] = "t"
    rail[1][S - 2] = "t"
    layers.append(rail)
    return {
        "name": "torre_de_vigia",
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Mountains", "Snowy Plains"],
        "spacing": 18,
        "chance": 0.55,
        "foundation": "cobblestone",
        "palette": {"B": "stone_bricks", "b": "stone_brick_slab", "w": "glass_pane", "W": "cobblestone_wall", "K": "chest", "t": "torch",
                    "0": "stone_brick_stairs", "1": "stone_brick_stairs:1", "2": "stone_brick_stairs:2", "3": "stone_brick_stairs:3"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "arrow", 4, 12, 0.8), ("", "bow", 1, 1, 0.3), ("", "iron_ingot", 1, 4, 0.6), ("", "gold_ingot", 1, 3, 0.35), ("", "diamond", 1, 2, 0.12),
                 ("", "iron_helmet", 1, 1, 0.15), ("", "iron_sword", 1, 1, 0.15), ("", "apple", 1, 3, 0.5)],
    }


def ruin():
    """Broken walls of an old stone hut, half taken by moss."""
    plan = [
        ["MCCC.CM", "C.....C", "C.....M", ".......", "M.....C", "C.....C", "CMC.CCM"],
        ["MC....M", "C....c.", "......M", ".......", "M......", "C.....C", "C...C.M"],
        ["M......", ".......", "......M", ".......", ".......", "C......", "......M"],
    ]
    floor = ["ggggggg", "gCCgggg", "gggCggg", "ggggCgg", "gCggggg", "ggggggg", "gggCggg"]
    layers = [floor] + plan
    return {
        "name": "ruina",
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Snowy Plains", "Mountains"],
        "spacing": 9,
        "chance": 0.4,
        "sink": 1,
        "palette": {"M": "mossy_cobblestone", "C": "cobblestone", "g": "gravel", "c": "chest"},
        "layers": layers,
        "loot": [("", "flint", 1, 4, 0.6), ("", "raw_iron", 1, 3, 0.5), ("", "coal", 2, 8, 0.6), ("", "bone", 1, 4, 0.5), ("", "emerald", 1, 1, 0.15), ("", "golden_sword", 1, 1, 0.1)],
    }


def well():
    """A desert well: sandstone rim round a spring, four posts and a slab roof."""
    layers = [
        ["SSSSS", "SSSSS", "SSwSS", "SSSSS", "SSSSS"],
        ["SSSSS", "S...S", "S.w.S", "S...S", "SSSSS"],
        ["S...S", ".....", ".....", ".....", "S...S"],
        ["S...S", ".....", ".....", ".....", "S...S"],
        ["sssss", "sssss", "sssss", "sssss", "sssss"],
    ]
    # The rim keeps the water in; the spring's surface sits one below the rim top.
    layers[1] = ["SSSSS", "SwwwS", "SwwwS", "SwwwS", "SSSSS"]
    layers[0] = ["SSSSS", "SSSSS", "SSSSS", "SSSSS", "SSSSS"]
    return {
        "name": "poco_do_deserto",
        "biomes": ["Desert"],
        "spacing": 10,
        "chance": 0.5,
        "sink": 1,
        "foundation": "sandstone",
        "palette": {"S": "sandstone", "w": "water", "s": "stone_slab"},
        "layers": layers,
        "loot": [],
    }


def ron(structs):
    out = ["// Generated by tools/gen_structures.py: edit that script, not this file.", "(structures: ["]
    for s in structs:
        out.append("    (")
        out.append(f'        name: "{s["name"]}",')
        out.append("        biomes: [" + ", ".join(f'"{b}"' for b in s["biomes"]) + "],")
        out.append(f'        spacing: {s["spacing"]},')
        out.append(f'        chance: {s["chance"]},')
        if s.get("sink"):
            out.append(f'        sink: {s["sink"]},')
        if s.get("foundation"):
            out.append(f'        foundation: Some("{s["foundation"]}"),')
        out.append("        palette: [" + ", ".join(f"('{k}', \"{v}\")" for k, v in s["palette"].items()) + "],")
        out.append("        layers: [")
        for layer in s["layers"]:
            out.append("            [" + ", ".join(f'"{r}"' for r in layer) + "],")
        out.append("        ],")
        if s["loot"]:
            out.append("        loot: [")
            for _, item, lo, hi, ch in s["loot"]:
                out.append(f'            (item: "{item}", min: {lo}, max: {hi}, chance: {ch}),')
            out.append("        ],")
        out.append("    ),")
    out.append("])")
    return "\n".join(out) + "\n"


structs = [
    house("oak_planks", "oak_log", "oak_roof", ["Plains", "Forest"], "casa_de_carvalho"),
    house("spruce_planks", "spruce_log", "spruce_roof", ["Taiga", "Snowy Plains"], "casa_de_pinheiro"),
    house("birch_planks", "birch_log", "brick_roof", ["Birch Forest"], "casa_de_betula"),
    tower(),
    ruin(),
    well(),
]
OUT.write_text(ron(structs))
print(f"wrote {OUT} ({len(structs)} structures)")
