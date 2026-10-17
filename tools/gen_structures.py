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
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Mountains", "Snowy Plains", "Savanna"],
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
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Snowy Plains", "Mountains", "Jungle", "Dark Forest", "Cherry Grove"],
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
        "biomes": ["Desert", "Badlands"],
        "spacing": 10,
        "chance": 0.5,
        "sink": 1,
        "foundation": "sandstone",
        "palette": {"S": "sandstone", "w": "water", "s": "stone_slab"},
        "layers": layers,
        "loot": [],
    }


def crypt():
    """A buried crypt (YUNG's Better Dungeons idea): a pillared room of stone bricks and moss
    under the ground, reached by a stair cut down from a mossy opening."""
    W = 11
    L = 9  # floor, three open layers, ceiling, three of earth, one above the ground
    steps = {n: n + 1 for n in range(1, 8)}  # step on layer n sits at z = n + 1
    layers = []
    for y in range(L):
        g = grid(W, W)
        for z in range(W):
            for x in range(W):
                edge = x in (0, W - 1) or z in (0, W - 1)
                if y == 0:
                    g[z][x] = "M" if (x * 7 + z * 3) % 5 == 0 else "B"
                elif y <= 3:
                    g[z][x] = ("M" if (x + z + y) % 4 == 0 else "B") if edge else "."
                elif y == 4:
                    g[z][x] = "M" if (x * 5 + z) % 6 == 0 else "B"
        if 1 <= y <= 3:
            for (px, pz) in ((3, 3), (3, 7), (7, 3), (7, 7)):
                g[pz][px] = "B"
        if y == 1:
            g[9][1] = "c"
            g[1][5] = "c"
            g[5][5] = "o"
        if y == 3:
            g[5][4] = "h"
        layers.append(g)
    # The stair down along x = 9, rising to the south.
    for y in range(1, L):
        for z in range(W):
            n = steps.get(y)
            if n is not None and z == n:
                layers[y][z][9] = "S"
            elif any(z == steps[k] and k < y <= k + 3 for k in steps):
                layers[y][z][9] = "."
    for z in (7, 8, 9):
        layers[8][z][8] = "m"
        layers[8][z][10] = "m"
    layers[8][9][9] = "."
    return {
        "name": "cripta",
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Dark Forest", "Swamp", "Mountains", "Snowy Plains", "Savanna"],
        "spacing": 8,
        "chance": 0.35,
        "sink": 8,
        "palette": {"B": "stone_bricks", "M": "mossy_cobblestone", "m": "mossy_cobblestone", "S": "stone_brick_stairs:2",
                    "c": "chest", "h": "lantern:1", "o": "oak_table"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "bone", 2, 6, 0.7), ("", "gold_ingot", 1, 4, 0.5), ("", "iron_ingot", 2, 5, 0.6), ("", "diamond", 1, 2, 0.2),
                 ("", "emerald", 1, 3, 0.3), ("", "life_crystal", 1, 1, 0.15), ("", "redstone", 3, 8, 0.4), ("", "minecart", 1, 1, 0.2)],
    }


def camp():
    """A traveller's camp (Explorify idea): a wool tent, log seats, a lantern and a chest."""
    layers = [grid(7, 7) for _ in range(4)]
    for z in range(1, 5):
        layers[1][z][1] = "W"
        layers[1][z][5] = "W"
        layers[2][z][2] = "W"
        layers[2][z][4] = "W"
        layers[3][z][3] = "W"
        for x in (2, 3, 4):
            layers[1][z][x] = "."
        layers[2][z][3] = "."
    layers[1][1][3] = "c"
    layers[1][6][1] = "x"
    layers[1][6][5] = "x"
    layers[1][6][3] = "l"
    layers[0] = grid(7, 7)
    return {
        "name": "acampamento",
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Savanna", "Snowy Plains"],
        "spacing": 6,
        "chance": 0.3,
        "sink": 1,
        "palette": {"W": "white_wool", "c": "chest", "x": "oak_log", "l": "lantern"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "apple", 1, 3, 0.6), ("", "cooked_porkchop", 1, 2, 0.4), ("", "torch", 2, 6, 0.6), ("", "leather", 1, 3, 0.4),
                 ("", "iron_ingot", 1, 2, 0.3), ("", "leather_backpack", 1, 1, 0.08)],
    }


def shrine():
    """A small stone shrine on a hill (Explorify idea): posts, a slab roof, a hanging lantern."""
    layers = [
        ["BBBBB", "BBBBB", "BBBBB", "BBBBB", "BBBBB"],
        ["B...B", ".....", "..c..", ".....", "B...B"],
        ["B...B", ".....", ".....", ".....", "B...B"],
        ["B...B", ".....", "..h..", ".....", "B...B"],
        ["sssss", "sssss", "sssss", "sssss", "sssss"],
    ]
    return {
        "name": "santuario",
        "biomes": ["Mountains", "Snowy Plains", "Cherry Grove", "Jungle", "Taiga"],
        "spacing": 7,
        "chance": 0.35,
        "foundation": "stone_bricks",
        "palette": {"B": "stone_bricks", "s": "stone_slab", "c": "chest", "h": "lantern:1"},
        "layers": layers,
        "loot": [("", "gold_ingot", 1, 3, 0.5), ("", "emerald", 1, 2, 0.4), ("", "mana_crystal", 1, 1, 0.2), ("", "apple", 1, 3, 0.5)],
    }


def end_ruin():
    """A sunken stone ruin with a ring of twelve empty end portal frames over a lava pit."""
    S, M, F, L = "S", "M", "F", "L"
    floor = ["SMSSSMS", "SSSSSSS", "MSLLLSS", "SSLLLSM", "SSLLLSS", "SSSSSMS", "SMSSSSS"]
    frames = ["S.....S", "..FFF..", ".F...F.", ".F...F.", ".F...F.", "..FFF..", "S.....S"]
    above = ["S.....S", ".......", ".......", ".......", ".......", ".......", "M.....S"]
    top = ["s.....s", ".......", ".......", ".......", ".......", ".......", "s......"]
    return {
        "name": "ruina_do_end",
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Savanna", "Desert", "Snowy Plains", "Dark Forest", "Swamp", "Jungle", "Badlands"],
        "spacing": 14,
        "chance": 0.3,
        "sink": 1,
        "foundation": "stone_bricks",
        "palette": {"S": "stone_bricks", "M": "mossy_cobblestone", "F": "end_portal_frame", "L": "lava", "s": "stone_slab"},
        "layers": [["SSSSSSS"] * 7, floor, frames, above, top],
        "loot": [],
    }


def wizard_tower():
    """A wizard's tower (Towns and Towers idea): a tall stone room lined with bookshelves, a
    chest of magic things and a purple cone roof."""
    W, H = 7, 9
    layers = []
    for y in range(H):
        g = grid(W, W)
        for z in range(W):
            for x in range(W):
                wall = x in (0, W - 1) or z in (0, W - 1)
                corner = x in (0, W - 1) and z in (0, W - 1)
                if y == 0:
                    g[z][x] = "B"
                elif corner:
                    g[z][x] = "."  # rounded corners
                elif wall:
                    g[z][x] = "M" if (x + z + y) % 5 == 0 else "B"
                else:
                    g[z][x] = "."
        if y in (1, 2):
            g[W - 1][3] = "d" if y == 1 else "u"
            for x in (1, 2, 4, 5):
                g[1][x] = "S"
        if y == 1:
            g[3][1] = "c"
            g[3][5] = "S"
        if y == 3:
            g[0][3] = "G"
            g[3][0] = "G"
            g[3][W - 1] = "G"
        if y == 5:
            g[3][3] = "h"
        if y == H - 1:
            for z in range(W):
                for x in range(W):
                    if not (x in (0, W - 1) and z in (0, W - 1)):
                        g[z][x] = "B"
            g[3][3] = "B"
        layers.append(g)
    layers[H - 2][3][3] = "h"
    layers[5][3][3] = "."
    # Purple cone roof, stepping in.
    for k, r in enumerate((3, 2, 1, 1, 0)):
        g = grid(W, W)
        for z in range(3 - r, 4 + r):
            for x in range(3 - r, 4 + r):
                g[z][x] = "P"
        layers.append(g)
    return {
        "name": "torre_do_mago",
        "biomes": ["Forest", "Birch Forest", "Dark Forest", "Cherry Grove", "Taiga"],
        "spacing": 12,
        "chance": 0.35,
        "foundation": "stone_bricks",
        "palette": {"B": "stone_bricks", "M": "mossy_cobblestone", "S": "bookshelf", "c": "chest", "h": "lantern:1",
                    "G": "glass_pane", "d": "oak_door:2", "u": "oak_door:10", "P": "purple_wool"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "mana_crystal", 1, 2, 0.6), ("", "topaz_wand", 1, 1, 0.3), ("", "amethyst_wand", 1, 1, 0.15),
                 ("", "flame_tome", 1, 1, 0.1), ("", "lapis_lazuli", 3, 9, 0.6), ("", "emerald", 1, 3, 0.4)],
    }


def witch_hut():
    """A swamp witch's hut on stilts (YUNG's Better Witch Huts idea): dark planks on log posts
    over the water, a porch with a lantern, mushrooms and a chest of odd things inside."""
    W, D = 9, 9
    layers = []
    posts = {(1, 2), (7, 2), (1, 7), (7, 7), (4, 0)}
    for y in range(3):  # stilts
        g = grid(W, D, ".")
        for (x, z) in posts:
            g[z][x] = "L"
        if y == 2:
            g[0][4] = "F"
        layers.append(g)
    g = grid(W, D, ".")  # floor and porch
    for z in range(0, D):
        for x in range(1, 8):
            if z >= 2 or x in (3, 4, 5):
                g[z][x] = "P"
    layers.append(g)
    for y in range(3):  # walls
        g = grid(W, D, ".")
        for z in range(2, D):
            for x in range(1, 8):
                edge = x in (1, 7) or z in (2, D - 1)
                corner = x in (1, 7) and z in (2, D - 1)
                if corner:
                    g[z][x] = "L"
                elif edge:
                    g[z][x] = "D"
        if y in (0, 1):
            g[2][4] = "d" if y == 0 else "u"
        if y == 1:
            g[5][1] = g[5][7] = g[D - 1][4] = "G"
        if y == 0:
            g[0][3] = g[0][5] = "f"
            g[D - 2][2] = "c"
            g[D - 2][6] = "T"
            g[4][2] = "r"
            g[6][6] = "b"
            g[D - 2][4] = "B"
        if y == 2:
            g[1][4] = "h"
        layers.append(g)
    layers[4][1][4] = "."
    for k, (x0, x1) in enumerate(((0, 8), (1, 7), (2, 6), (3, 5))):  # roof
        g = grid(W, D, ".")
        for z in range(1, D):
            for x in range(x0, x1 + 1):
                g[z][x] = "R" if x in (x0, x1) else ("D" if k == 3 else ".")
        if k == 0:
            g[1][4] = "R"
        layers.append(g)
    layers[6][1][4] = "h"
    return {
        "name": "cabana_da_bruxa",
        "biomes": ["Swamp"],
        "spacing": 10,
        "chance": 0.4,
        "foundation": "spruce_log",
        "palette": {"L": "spruce_log", "P": "spruce_planks", "D": "dark_oak_planks", "R": "oak_slab",
                    "F": "oak_fence", "f": "oak_fence", "G": "glass_pane", "d": "oak_door:2", "u": "oak_door:10",
                    "c": "chest", "T": "dark_oak_table", "r": "red_mushroom", "b": "brown_mushroom",
                    "B": "bookshelf", "h": "lantern:1"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "redstone", 2, 8, 0.6), ("", "gunpowder", 1, 4, 0.5), ("", "string", 1, 5, 0.5),
                 ("", "bone", 1, 4, 0.4), ("", "mana_crystal", 1, 1, 0.3), ("", "amethyst_wand", 1, 1, 0.12),
                 ("", "emerald", 1, 3, 0.35)],
    }


NETHER_BIOMES = ["Nether Wastes", "Soul Sand Valley", "Crimson Forest"]


def nether_ruined_portal():
    """A broken obsidian portal on a blackstone floor (Formations Nether idea), half its frame
    gone, with a chest of gold beside it."""
    floor = ["BBBBBBB", "BMBBBGB", "BBBBBBB", "BBBMBBB", "BBBBBBB"]
    layers = [floor]
    for y in range(5):
        g = grid(7, 5, ".")
        if y == 0:
            g[2][1] = g[2][2] = g[2][3] = g[2][4] = "O"
            g[3][5] = "c"
        else:
            g[2][1] = "O" if y < 4 else "."
            g[2][4] = "O" if y < 3 else "."
            if y == 4:
                g[2][2] = "O"
        layers.append(g)
    return {
        "name": "portal_em_ruinas",
        "biomes": NETHER_BIOMES,
        "spacing": 8,
        "chance": 0.35,
        "foundation": "blackstone",
        "palette": {"B": "blackstone", "M": "magma_block", "G": "nether_gold_ore", "O": "obsidian", "c": "chest"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "gold_ingot", 1, 4, 0.6), ("", "gold_nugget", 3, 9, 0.6), ("", "obsidian", 1, 3, 0.4),
                 ("", "flint_and_steel", 1, 1, 0.3), ("", "quartz", 2, 6, 0.5)],
    }


def basalt_altar():
    """A ring of basalt pillars around a magma altar topped by shroomlight."""
    W = 7
    layers = []
    for y in range(6):
        g = grid(W, W, ".")
        for (x, z) in ((0, 3), (6, 3), (3, 0), (3, 6), (1, 1), (5, 1), (1, 5), (5, 5)):
            tall = 5 if (x + z) % 2 == 0 else 3
            if y < tall:
                g[z][x] = "S"
        if y == 0:
            for z in range(2, 5):
                for x in range(2, 5):
                    g[z][x] = "B"
            g[3][3] = "M"
        if y == 1:
            g[3][3] = "L"
        layers.append(g)
    return {
        "name": "altar_de_basalto",
        "biomes": ["Nether Wastes", "Soul Sand Valley"],
        "spacing": 9,
        "chance": 0.3,
        "foundation": "basalt",
        "palette": {"S": "basalt", "B": "blackstone", "M": "magma_block", "L": "shroomlight"},
        "layers": [rows(l) for l in layers],
        "loot": [],
    }


def brick_shrine():
    """A small nether brick shrine with an arched door and a chest."""
    W, D = 5, 5
    layers = []
    for y in range(5):
        g = grid(W, D, ".")
        for z in range(D):
            for x in range(W):
                edge = x in (0, W - 1) or z in (0, D - 1)
                if y == 0 or (edge and y < 4) or y == 4:
                    g[z][x] = "N"
        if y in (1, 2):
            g[0][2] = "."
        if y == 1:
            g[3][2] = "c"
            g[2][0] = g[2][W - 1] = "F"
        if y == 4:
            g[2][2] = "L"
        layers.append(g)
    return {
        "name": "santuario_de_tijolos",
        "biomes": NETHER_BIOMES,
        "spacing": 10,
        "chance": 0.3,
        "foundation": "nether_bricks",
        "palette": {"N": "nether_bricks", "c": "chest", "F": "iron_bars", "L": "shroomlight"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "gold_ingot", 1, 3, 0.5), ("", "quartz", 3, 8, 0.6), ("", "diamond", 1, 1, 0.12),
                 ("", "fire_staff", 1, 1, 0.08), ("", "magma_block", 1, 4, 0.4)],
    }


def windmill():
    """A windmill (Towns and Towers idea): a cobblestone base, a plank cap and four sails of
    wool on fence arms over the north side."""
    W, D, H = 9, 6, 13
    layers = [grid(W, D) for _ in range(H)]
    for y in range(H):
        for z in range(1, D):
            for x in range(2, 7):
                wall = x in (2, 6) or z in (1, D - 1)
                if y == 0:
                    layers[y][z][x] = "C"
                elif y <= 5:
                    layers[y][z][x] = "C" if wall else "."
                elif y <= 8:
                    layers[y][z][x] = "P" if wall else "."
                elif y == 9:
                    layers[y][z][x] = "s"
    layers[1][D - 1][4] = "d"
    layers[2][D - 1][4] = "u"
    layers[3][1][4] = "G"
    layers[1][2][3] = "c"
    layers[1][4][5] = "K"
    # Sails on the north face: a cross of fence arms with wool panels.
    hub = (4, 6)  # x, y
    layers[hub[1]][0][hub[0]] = "L"
    for k in range(1, 5):
        for (dx, dy) in ((0, 1), (0, -1), (1, 0), (-1, 0)):
            x, y = hub[0] + dx * k, hub[1] + dy * k
            if 0 <= x < W and 1 <= y < H:
                layers[y][0][x] = "f"
                # The sail beside each arm, turning the same way round.
                sx, sy = x + dy, y - dx
                if 0 <= sx < W and 1 <= sy < H and k >= 2:
                    layers[sy][0][sx] = "W"
    return {
        "name": "moinho",
        "biomes": ["Plains", "Savanna", "Cherry Grove"],
        "spacing": 12,
        "chance": 0.35,
        "foundation": "cobblestone",
        "palette": {"C": "cobblestone", "P": "oak_planks", "s": "oak_slab", "G": "glass_pane", "d": "oak_door:2", "u": "oak_door:10",
                    "c": "chest", "K": "crafting_table", "L": "oak_log", "f": "oak_fence", "W": "white_wool"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "oak_sapling", 1, 3, 0.5), ("", "apple", 1, 4, 0.5), ("", "string", 1, 4, 0.4), ("", "iron_ingot", 1, 2, 0.3)],
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
    house("birch_planks", "birch_log", "brick_roof", ["Birch Forest", "Cherry Grove"], "casa_de_betula"),
    house("acacia_planks", "acacia_log", "brick_roof", ["Savanna"], "casa_de_acacia"),
    house("dark_oak_planks", "dark_oak_log", "spruce_roof", ["Dark Forest", "Swamp"], "cabana_escura"),
    tower(),
    ruin(),
    well(),
    crypt(),
    camp(),
    shrine(),
    wizard_tower(),
    windmill(),
    end_ruin(),
    witch_hut(),
    nether_ruined_portal(),
    basalt_altar(),
    brick_shrine(),
]
OUT.write_text(ron(structs))
print(f"wrote {OUT} ({len(structs)} structures)")
