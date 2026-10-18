"""Lote 3 of plan/o-que-falta.md: the vanilla structures we were missing, laid out in code like
tools/gen_structures.py does (layers bottom to top, rows north to south, "." air, " " keeps the
terrain). Minecraft's ideas only; the plans are ours and smaller than vanilla's.

The ocean monument uses Phase 6 blocks (prismarine), so it is written by tools/gen_fase6.py; the
mansion, ocean ruins and buried treasure by tools/gen_structures.py.
"""


def grid(w, d, ch=" "):
    return [[ch] * w for _ in range(d)]


def rows(g):
    return ["".join(r) for r in g]


def h(x, y, z, k=0):
    """A small deterministic hash in 0..1 (crumbling, moss)."""
    n = (x * 73856093) ^ (y * 19349663) ^ (z * 83492791) ^ (k * 2654435761)
    n = (n ^ (n >> 13)) * 1274126177
    return ((n ^ (n >> 16)) & 0xFFFF) / 65535.0


def ocean_monument():
    """A temple of prismarine on the sea floor, full of water: an outer ring of halls on
    pillars with a gate in each side, two wings where wet sponges grow, and the great chamber in
    the middle with its core of gold under a lantern-lit tower. Watchers swim the halls and three
    elder guardians keep it."""
    W, H = 31, 12
    c = W // 2
    layers = []
    for y in range(H):
        g = grid(W, W, " ")
        for z in range(W):
            for x in range(W):
                dx, dz = abs(x - c), abs(z - c)
                r = max(dx, dz)
                gate = min(dx, dz) <= 1
                ch = " "
                if y == 0:
                    ch = "B"
                elif r == 15:
                    # The outer wall: pillars at the corners and every five blocks, a gate in
                    # each side.
                    if y <= 5:
                        if gate and 2 <= y <= 4:
                            ch = "W"
                        elif (x % 5 == 0 or z % 5 == 0) or dx == dz:
                            ch = "D"
                        else:
                            ch = "P" if y != 3 else "L" if (x + z) % 6 == 0 else "P"
                elif 5 < r < 15:
                    if y <= 4:
                        ch = "W"
                    elif y == 5:
                        ch = "B"
                    elif y == 6 and r == 14:
                        ch = "S" if (x + z) % 2 == 0 else "W"
                elif r == 5:
                    if y <= 9:
                        ch = "W" if (gate and 2 <= y <= 4) else ("L" if y in (2, 7) and (dx == dz) else "B")
                    elif y == 10:
                        ch = "D"
                elif r < 5:
                    if y <= 9:
                        ch = "W"
                    elif y == 10:
                        ch = "D"
                    elif r <= 1 and y <= 11:
                        ch = "L" if y == 11 else "P"
                # The gold core in the great chamber, wrapped in dark prismarine.
                if 1 <= y <= 2 and dx <= 1 and dz <= 1:
                    ch = "G" if (dx == 0 or dz == 0) and not (dx == 0 and dz == 0) else "D"
                # Wings: west and east halls with wet sponges on the walls.
                if 8 <= dx <= 12 and dz <= 2 and 1 <= y <= 4 and ch == "W":
                    if dz == 2 and h(x, y, z) < 0.45:
                        ch = "s"
                if 1 <= y <= 4 and r == 10 and not gate and (dx == dz):
                    ch = "D"
                g[z][x] = ch
        layers.append(g)
    return {
        "name": "monumento_oceanico",
        "biomes": ["Ocean"],
        "spacing": 32,
        "chance": 0.6,
        "rough": 10,
        "sink": 3,
        "foundation": "prismarine",
        "palette": {"B": "prismarine_bricks", "P": "prismarine", "D": "dark_prismarine", "L": "sea_lantern", "G": "gold_block",
                    "s": "wet_sponge", "S": "prismarine", "W": "water"},
        "layers": [rows(l) for l in layers],
        "loot": [],
        "mobs": [("deep_watcher", 6), ("elder_guardian", 3)],
    }


def woodland_mansion():
    """A dark oak mansion in the dark forest: three floors on a cobblestone base, log posts,
    rows of windows, rooms off a middle hall on every floor, a chest on each, a stepped roof.
    Brutes and invokers live in it, and allays wait in a cage of bars upstairs."""
    W, D = 25, 19
    F = 5  # floor height
    layers = []
    for y in range(3 * F + 6):
        g = grid(W, D, " ")
        floor, ly = divmod(y, F)
        for z in range(D):
            for x in range(W):
                edge_x, edge_z = x in (0, W - 1), z in (0, D - 1)
                post = (edge_x and (z % 6 == 0 or edge_z)) or (edge_z and (x % 6 == 0 or edge_x))
                ch = " "
                if y < 3 * F:
                    if ly == 0:
                        ch = "C" if floor == 0 else "P"
                    elif edge_x or edge_z:
                        if post:
                            ch = "O"
                        elif ly in (2, 3) and ((x % 3 == 1 and edge_z) or (z % 3 == 1 and edge_x)):
                            ch = "G"
                        else:
                            ch = "C" if floor == 0 else ("P" if ly < 4 else "O")
                    else:
                        ch = "."
                        # A hall down the middle (east-west), rooms on either side.
                        if z in (6, 12) and ly < 4:
                            ch = "P" if x % 6 != 3 else "."
                        if x in (8, 16) and z not in range(7, 12) and ly < 4:
                            ch = "P" if z not in (3, 15) else "."
                        if ly == 4:
                            ch = "P" if not (x in (11, 12, 13) and 8 <= z <= 10) else "."
                    # Stairs up through the floors (a gap in the ceiling in the hall).
                    if 1 <= ly <= 3 and floor < 2 and x == 12 + ly - 1 and z == 9:
                        ch = "P"
                else:
                    # The roof: steps in one block a layer.
                    k = y - 3 * F
                    inside = k <= x <= W - 1 - k and k <= z <= D - 1 - k
                    if k < 6 and inside:
                        ch = "R" if k == 5 else "P"
                g[z][x] = ch
        # The south door, chests, lanterns and the cage.
        if y in (1, 2):
            g[D - 1][12] = "."
        if y == 1:
            g[3][3], g[15][21] = "c", "c"
            g[9][2] = "h"
        if y == F + 1:
            g[3][20] = "c"
            for (x, z) in ((3, 3), (4, 3), (3, 4), (4, 4)):
                g[z][x] = "."
            for x in range(2, 6):
                g[2][x] = "I"
                g[5][x] = "I"
            for z in range(2, 6):
                g[z][2] = "I"
                g[z][5] = "I"
        if y == F + 2:
            for x in range(2, 6):
                g[2][x] = "I"
                g[5][x] = "I"
            for z in range(2, 6):
                g[z][2] = "I"
                g[z][5] = "I"
        if y == 2 * F + 1:
            g[15][3] = "c"
            g[9][22] = "h"
        layers.append(g)
    return {
        "name": "mansao_da_floresta",
        "biomes": ["Dark Forest"],
        "spacing": 40,
        "chance": 0.5,
        "rough": 6,
        "foundation": "cobblestone",
        "palette": {"C": "cobblestone", "P": "dark_oak_planks", "O": "dark_oak_log", "G": "glass", "R": "dark_oak_slab",
                    "c": "chest", "h": "lantern:1", "I": "iron_bars"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "emerald", 1, 4, 0.5), ("", "golden_apple", 1, 1, 0.2), ("", "enchanted_book", 1, 1, 0.3),
                 ("", "diamond", 1, 2, 0.15), ("", "iron_ingot", 1, 4, 0.5), ("", "chainmail_chestplate", 1, 1, 0.15),
                 ("", "lead", 1, 2, 0.3), ("", "redstone", 2, 6, 0.4), ("", "music_disc_void", 1, 1, 0.1),
                 ("", "totem_of_undying", 1, 1, 0.05)],
        "mobs": [("brute", 4), ("marauder", 2), ("invoker", 2), ("allay", 2)],
    }


def ocean_ruin(kind):
    """A small sunken ruin on the sea floor, stone bricks or sandstone, half fallen in, with a
    chest and two of the drowned."""
    W, H = 9, 5
    stone = kind == "pedra"
    layers = []
    for y in range(H):
        g = grid(W, W, " ")
        for z in range(W):
            for x in range(W):
                wall = x in (0, W - 1) or z in (0, W - 1)
                top = 4 - int(h(x, 0, z, 7) * 4)  # walls crumble to different heights
                if y == 0:
                    ch = "F"
                elif wall and y <= top:
                    r = h(x, y, z, 3)
                    ch = ("M" if r < 0.3 else "K" if r < 0.5 else "B") if stone else ("B" if r < 0.6 else "K")
                elif y < 3:
                    ch = "W"
                else:
                    ch = " "
                g[z][x] = ch
        if y == 1:
            g[4][4] = "c"
            g[W - 1][4] = "W"
        if y == 2:
            g[W - 1][4] = "W"
        layers.append(g)
    pal = {"F": "gravel" if stone else "sand", "B": "stone_bricks" if stone else "sandstone", "M": "mossy_stone_bricks",
           "K": "cracked_stone_bricks" if stone else "smooth_sandstone", "W": "water", "c": "chest"}
    return {
        "name": f"ruina_oceanica_{kind}",
        "biomes": ["Ocean", "Frozen Ocean"] if stone else ["Ocean"],
        "spacing": 14,
        "chance": 0.5,
        "rough": 6,
        "palette": pal,
        "layers": [rows(l) for l in layers],
        "loot": [("", "coal", 2, 6, 0.6), ("", "emerald", 1, 2, 0.3), ("", "gold_nugget", 2, 8, 0.5), ("", "iron_nugget", 2, 8, 0.5),
                 ("", "wheat", 2, 6, 0.4), ("", "fishing_rod", 1, 1, 0.15), ("", "golden_apple", 1, 1, 0.05),
                 ("", "tropical_fish", 1, 3, 0.3)],
        "mobs": [("sunken", 2)],
    }


def buried_treasure():
    """A chest under the sand of a beach, three blocks down: the heart of the sea, gold, iron
    and an emerald or two."""
    layers = [rows([[" ", " ", " "], [" ", "c", " "], [" ", " ", " "]])] + [rows(grid(3, 3, " ")) for _ in range(3)]
    return {
        "name": "tesouro_enterrado",
        "biomes": ["Beach"],
        "spacing": 12,
        "chance": 0.45,
        "sink": 4,
        "palette": {"c": "chest"},
        "layers": layers,
        "loot": [("", "heart_of_the_sea", 1, 1, 1.0), ("", "gold_ingot", 1, 4, 0.8), ("", "iron_ingot", 1, 4, 0.8),
                 ("", "emerald", 1, 4, 0.5), ("", "diamond", 1, 2, 0.4), ("", "tnt", 1, 2, 0.3), ("", "prismarine_crystals", 1, 4, 0.4),
                 ("", "cooked_cod", 2, 4, 0.5), ("", "chainmail_chestplate", 1, 1, 0.2), ("", "iron_sword", 1, 1, 0.2)],
    }
