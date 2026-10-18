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
        g = grid(W, D)
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
        "biomes": ["Swamp", "Mangrove Swamp"],
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


def raider_outpost():
    """A raiders' watchtower (Illager Invasion idea): dark oak posts and cobblestone, a
    lookout deck with a fence rail and a chest of plunder at the top."""
    W = 7
    layers = []
    H = 12
    for y in range(H):
        g = grid(W, W, ".")
        for (x, z) in ((1, 1), (5, 1), (1, 5), (5, 5)):
            g[z][x] = "L"
        if y == 0:
            for z in range(1, 6):
                for x in range(1, 6):
                    g[z][x] = "C"
        if y in (4, 8):
            for z in range(1, 6):
                for x in range(1, 6):
                    if (x, z) not in ((3, 3),):
                        g[z][x] = "P"
        if 0 < y < H - 3 and y not in (4, 8):
            g[3][3] = "."
        if y == H - 3:
            for z in range(W):
                for x in range(W):
                    g[z][x] = "P"
        if y == H - 2:
            for z in range(W):
                for x in range(W):
                    if x in (0, W - 1) or z in (0, W - 1):
                        g[z][x] = "F"
            g[3][3] = "c"
            g[2][2] = "h"
        if y == H - 1:
            for z in range(1, 6):
                for x in range(1, 6):
                    g[z][x] = "s"
        layers.append(g)
    return {
        "name": "posto_dos_saqueadores",
        "biomes": ["Plains", "Taiga", "Savanna", "Snowy Plains"],
        "spacing": 16,
        "chance": 0.3,
        "foundation": "cobblestone",
        "palette": {"L": "dark_oak_log", "C": "cobblestone", "P": "dark_oak_planks", "F": "dark_oak_fence",
                    "c": "chest", "h": "lantern", "s": "dark_oak_slab"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "arrow", 4, 12, 0.7), ("", "emerald", 1, 4, 0.5), ("", "iron_ingot", 1, 3, 0.5),
                 ("", "bow", 1, 1, 0.4), ("", "bread", 1, 4, 0.5), ("", "tnt", 1, 2, 0.15)],
    }


def nether_fortress():
    """A nether brick fortress (YUNG's Better Nether Fortresses idea): a long bridge on arches
    over the lava with fenced walkways, and a two-storey keep in the middle with a chest,
    soul lanterns of shroomlight and iron-barred windows."""
    W, D, H = 29, 11, 12
    layers = [grid(W, D, ".") for _ in range(H)]
    deck = 4
    # The bridge running east-west, with arches underneath.
    for x in range(W):
        for z in range(3, 8):
            layers[deck][z][x] = "N"
            if z in (3, 7):
                layers[deck + 1][z][x] = "f"
        if x % 6 == 0:
            for y in range(deck):
                for z in (3, 7):
                    layers[y][z][x] = "N"
        elif x % 6 in (1, 5):
            for z in (3, 7):
                layers[deck - 1][z][x] = "N"
    # The keep in the middle.
    kx0, kx1 = 9, 19
    for y in range(deck, H):
        for z in range(D):
            for x in range(kx0, kx1 + 1):
                edge = x in (kx0, kx1) or z in (0, D - 1)
                if y in (deck, deck + 4, H - 1):
                    layers[y][z][x] = "N"
                elif edge:
                    layers[y][z][x] = "N"
                else:
                    layers[y][z][x] = "."
    for y in (deck + 1, deck + 2):
        for z in (4, 5, 6):
            layers[y][z][kx0] = "."
            layers[y][z][kx1] = "."
    for y in (deck + 2, deck + 6):
        layers[y][0][14] = "B"
        layers[y][D - 1][14] = "B"
    layers[deck + 1][2][11] = "c"
    for x in range(11, 14):  # a bed of nether wart on soul sand
        layers[deck][8][x] = "Z"
        layers[deck + 1][8][x] = "w"
    layers[deck + 3][5][14] = "L"
    layers[deck + 7][5][14] = "L"
    layers[deck + 5][8][17] = "c"
    for z in range(1, D - 1):  # a stair hole to the upper floor
        pass
    layers[deck + 4][2][17] = "."
    layers[deck + 4][2][16] = "."
    for k, (x, y) in enumerate(((16, deck + 1), (17, deck + 2), (18, deck + 3))):
        layers[y][2][min(x, kx1 - 1)] = "N"
    # Crenellations on the roof.
    for x in range(kx0, kx1 + 1):
        if x % 2 == 0:
            layers[H - 1][0][x] = "N"
    return {
        "name": "fortaleza_do_nether",
        "biomes": ["Nether Wastes", "Soul Sand Valley", "Crimson Forest"],
        "spacing": 14,
        "chance": 0.55,
        "foundation": "nether_bricks",
        "palette": {"N": "nether_bricks", "f": "nether_brick_fence", "B": "iron_bars", "c": "chest", "L": "shroomlight",
                    "Z": "soul_sand", "w": "nether_wart_stage3"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "gold_ingot", 2, 6, 0.6), ("", "netherite_upgrade_smithing_template", 1, 1, 0.15), ("", "diamond", 1, 2, 0.25), ("", "iron_ingot", 2, 5, 0.5),
                 ("", "blaze_rod", 1, 3, 0.4), ("", "obsidian", 2, 5, 0.3), ("", "fire_staff", 1, 1, 0.08),
                 ("", "life_crystal", 1, 1, 0.12)],
    }


def stronghold():
    """A buried stronghold (YUNG's Better Strongholds idea): a stair well down from a small
    ruin to stone brick halls, a library, a store room and the portal room with its ring
    of twelve empty frames over lava."""
    W, D = 25, 25
    H = 7
    sink = 26
    halls = [grid(W, D) for _ in range(H)]
    def box(x0, z0, x1, z1, y0=0, y1=H - 1, wall="S", floor="S", ceil="S"):
        for y in range(y0, y1 + 1):
            for z in range(z0, z1 + 1):
                for x in range(x0, x1 + 1):
                    edge = x in (x0, x1) or z in (z0, z1)
                    if y == y0:
                        halls[y][z][x] = floor
                    elif y == y1:
                        halls[y][z][x] = ceil
                    elif edge:
                        halls[y][z][x] = "M" if (x * 7 + y * 3 + z) % 5 == 0 else ("K" if (x + y * 5 + z * 3) % 7 == 0 else wall)
                    else:
                        halls[y][z][x] = "."
    def hole(x, z, w=1, axis="x"):
        for y in (1, 2, 3):
            for k in range(-w, w + 1):
                if axis == "x":
                    halls[y][z][x + k] = "."
                else:
                    halls[y][z + k][x] = "."
    box(0, 0, 24, 24)           # outer shell
    box(0, 0, 8, 8)             # stair room (north-west)
    box(16, 0, 24, 10)          # library (north-east)
    box(0, 14, 10, 24)          # store room (south-west)
    box(12, 12, 24, 24)         # portal room (south-east)
    for y in range(1, H - 1):   # the big cross hall in the middle
        for z in range(9, 13):
            for x in range(1, 24):
                halls[y][z][x] = "."
        for x in range(9, 13):
            for z in range(1, 24):
                halls[y][z][x] = "."
    hole(8, 4, axis="z"); hole(4, 8); hole(16, 5, axis="z"); hole(20, 10)
    hole(5, 14); hole(10, 19, axis="z"); hole(18, 12); hole(12, 18, axis="z")
    for z in range(1, 10):      # library shelves
        for x in (17, 19, 21, 23):
            if z not in (5,):
                for y in (1, 2, 3):
                    halls[y][z][x] = "B"
    halls[1][2][18] = "c"
    for z in range(15, 24):     # store room: chests and barrels of supplies
        halls[1][z][1] = "c" if z % 4 == 0 else "W"
    halls[3][19][5] = "h"
    halls[3][5][4] = "h"
    halls[4][10][10] = "h"
    halls[4][4][20] = "h"
    # Portal room: a raised dais with twelve frames round a pool of lava.
    cx, cz = 18, 18
    for z in range(cz - 3, cz + 4):
        for x in range(cx - 3, cx + 4):
            halls[1][z][x] = "S"
    for z in range(cz - 1, cz + 2):
        for x in range(cx - 1, cx + 2):
            halls[1][z][x] = "L"
    for k in (-1, 0, 1):
        for (x, z) in ((cx + k, cz - 2), (cx + k, cz + 2), (cx - 2, cz + k), (cx + 2, cz + k)):
            halls[2][z][x] = "F"
    halls[4][cz][cx] = "h"
    halls[1][13][23] = "c"
    # The stair well up to the surface: a spiral of steps round a pillar in the stair room.
    ring = [(1, 0), (2, 0), (2, 1), (2, 2), (1, 2), (0, 2), (0, 1), (0, 0)]
    shaft = []
    sx, sz = 3, 3
    for y in range(sink + 2):
        g = grid(W, D)
        for z in range(sz - 1, sz + 4):
            for x in range(sx - 1, sx + 4):
                g[z][x] = "S"
        for z in range(sz, sz + 3):
            for x in range(sx, sx + 3):
                g[z][x] = "."
        g[sz + 1][sx + 1] = "S"
        dx, dz = ring[(y + H) % 8]
        g[sz + dz][sx + dx] = "S"
        shaft.append(g)
    layers = [rows(l) for l in halls]
    for k, g in enumerate(shaft):
        if k + H < len(layers):
            continue
        layers.append(rows(g))
    # The spiral carries on down through the stair room to its floor.
    for y in range(1, H):
        for z in range(sz, sz + 3):
            for x in range(sx, sx + 3):
                halls[y][z][x] = "."
        halls[y][sz + 1][sx + 1] = "S"
        dx, dz = ring[y % 8]
        halls[y][sz + dz][sx + dx] = "S"
    layers = [rows(l) for l in halls] + layers[H:]
    # A small mossy ruin marks the top.
    top = grid(W, D)
    for z in range(sz - 1, sz + 4):
        for x in range(sx - 1, sx + 4):
            if (x + z) % 2 == 0 and x in (sx - 1, sx + 3) or z in (sz - 1, sz + 3) and x % 2:
                top[z][x] = "M"
    layers.append(rows(top))
    return {
        "name": "fortaleza_subterranea",
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Savanna", "Snowy Plains", "Dark Forest", "Mountains", "Desert"],
        "spacing": 30,
        "chance": 0.5,
        "sink": H + sink - 1,
        "palette": {"S": "stone_bricks", "M": "mossy_cobblestone", "K": "cobblestone", "B": "bookshelf", "c": "chest",
                    "W": "oak_planks", "h": "lantern:1", "L": "lava", "F": "end_portal_frame"},
        "layers": layers,
        "loot": [("", "ender_pearl", 1, 3, 0.5), ("", "ender_eye", 1, 2, 0.25), ("", "iron_ingot", 2, 6, 0.6),
                 ("", "bread", 1, 4, 0.5), ("", "diamond", 1, 2, 0.15), ("", "emerald", 1, 4, 0.4),
                 ("", "flame_tome", 1, 1, 0.06), ("", "mana_crystal", 1, 1, 0.2)],
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


def desert_temple():
    """A sandstone pyramid (idea from the desert temples): a stepped body with an orange and
    terracotta star on the floor, and under it a hidden room with four chests over a TNT
    trap set off by the plate in the middle."""
    W = 15
    layers = []
    # The trap room, three layers below the floor (sink keeps it underground).
    for y in range(4):
        g = grid(W, W, " ")
        for z in range(4, 11):
            for x in range(4, 11):
                edge = x in (4, 10) or z in (4, 10)
                if y == 0:
                    g[z][x] = "S"
                elif y == 3:
                    g[z][x] = "S"
                else:
                    g[z][x] = "S" if edge else "."
        if y == 0:
            g[7][7] = "X"  # TNT under the plate
        if y == 1:
            g[7][7] = "p"
            for (x, z) in ((7, 5), (7, 9), (5, 7), (9, 7)):
                g[z][x] = "c"
        if y == 3:
            g[7][7] = "."  # the hole down from the hall
        layers.append(g)
    # The pyramid: each layer a step in.
    for k in range(7):
        g = grid(W, W, " ")
        lo, hi = k, W - 1 - k
        for z in range(lo, hi + 1):
            for x in range(lo, hi + 1):
                edge = x in (lo, hi) or z in (lo, hi)
                g[z][x] = "C" if edge and k % 2 == 1 else "S"
        if k <= 2:
            # The hall inside: open from layer 1 to 2, the hole in the middle of its floor.
            for z in range(lo + 1, hi):
                for x in range(lo + 1, hi):
                    if k > 0:
                        g[z][x] = "."
            if k == 0:
                for z in range(4, 11):
                    for x in range(4, 11):
                        d = abs(x - 7) + abs(z - 7)
                        g[z][x] = "O" if d in (1, 3) else ("T" if d == 2 else "S")
                g[7][7] = "."
            # Doorway in the south wall, chiseled blocks either side.
            if k in (0, 1, 2):
                g[hi][7] = "." if k > 0 else "S"
                if k == 1:
                    g[hi][6] = "H"
                    g[hi][8] = "H"
        layers.append(g)
    return {
        "name": "templo_do_deserto",
        "biomes": ["Desert"],
        "spacing": 14,
        "chance": 0.45,
        "sink": 4,
        "foundation": "sandstone",
        "palette": {"S": "sandstone", "C": "cut_sandstone", "H": "chiseled_sandstone", "O": "orange_terracotta", "T": "terracotta",
                    "X": "tnt", "p": "stone_pressure_plate", "c": "chest"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "gold_ingot", 2, 7, 0.6), ("", "emerald", 1, 3, 0.4), ("", "bone", 3, 8, 0.6), ("", "rotten_flesh", 2, 6, 0.5),
                 ("", "diamond", 1, 3, 0.2), ("", "iron_ingot", 1, 5, 0.5), ("", "saddle", 1, 1, 0.25), ("", "topaz", 1, 2, 0.3)],
    }


def jungle_temple():
    """A mossy stone temple in the jungle: two floors joined by a stair, a hidden chest
    behind the stair and vines hanging off the roof."""
    W, D, H = 11, 13, 9
    layers = []
    for y in range(H):
        g = grid(W, D, " ")
        for z in range(D):
            for x in range(W):
                edge = x in (0, W - 1) or z in (0, D - 1)
                mossy = (x * 3 + z * 5 + y) % 4 == 0
                stone = "M" if mossy else "C"
                if y in (0, 4, 8):
                    g[z][x] = stone
                else:
                    g[z][x] = stone if edge else "."
        if y in (1, 2):
            g[D - 1][5] = "."  # entrance
        if y in (5, 6) and True:
            for z in (3, 6, 9):
                g[z][0] = "."  # windows upstairs
                g[z][W - 1] = "."
        # Stair up along the east wall, rising to the north.
        for n in range(4):
            if y == n + 1:
                g[9 - n][8] = "s"
            if y == 4:
                g[9 - n][8] = "."
                g[9 - n][9] = "."
        if y == 1:
            g[10][9] = "c"  # tucked behind the foot of the stair
            g[2][2] = "c"
            g[6][5] = "l"
        if y == 5:
            g[2][8] = "c"
            g[6][2] = "h"
        if y == 8:
            for x in (1, 4, 7, 9):
                g[0][x] = "M"
        layers.append(g)
    # Vines over the walls.
    vines = grid(W, D, " ")
    layers.append(vines)
    return {
        "name": "templo_da_selva",
        "biomes": ["Jungle"],
        "spacing": 14,
        "chance": 0.5,
        "sink": 1,
        "foundation": "cobblestone",
        "palette": {"C": "cobblestone", "M": "mossy_cobblestone", "s": "cobblestone_stairs", "c": "chest", "l": "lantern",
                    "h": "lantern:1"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "gold_ingot", 2, 6, 0.6), ("", "emerald", 1, 4, 0.5), ("", "diamond", 1, 2, 0.25), ("", "bone", 2, 6, 0.5),
                 ("", "saddle", 1, 1, 0.25), ("", "ruby", 1, 2, 0.3), ("", "arrow", 4, 12, 0.5)],
    }


def igloo():
    """A snow dome with an ice window, a bed, a furnace, a table and a chest."""
    layers = [
        [" SSSSS ", "SSSSSSS", "SSSSSSS", "SSSSSSS", "SSSSSSS", "SSSSSSS", " SSSSS "],
        [" SSSSS ", "SbfwwKS", "SwwwwwS", "SwwwwwS", "SwwwwwS", "SwcwwtS", " SS.SS "],
        [" SSSSS ", "S.....S", "S.....S", "I.....I", "S.....S", "S.....S", " SS.SS "],
        ["  SSS  ", " S...S ", "S.....S", "S.....S", "S.....S", " S...S ", "  SSS  "],
        ["       ", "  SSS  ", " SSSSS ", " SSSSS ", " SSSSS ", "  SSS  ", "       "],
    ]
    return {
        "name": "iglu",
        "biomes": ["Snowy Plains", "Snowy Peaks"],
        "spacing": 10,
        "chance": 0.4,
        "foundation": "snow",
        "palette": {"S": "snow", "I": "ice", "w": "white_carpet", "b": "red_bed", "f": "furnace", "K": "crafting_table",
                    "c": "chest", "t": "torch"},
        "layers": layers,
        "loot": [("", "coal", 2, 6, 0.6), ("", "apple", 1, 3, 0.5), ("", "gold_ingot", 1, 2, 0.3), ("", "emerald", 1, 2, 0.3),
                 ("", "salmon", 1, 4, 0.5)],
    }


def mineshaft():
    """An old mine deep in the stone: two crossing tunnels held up by oak frames every few
    blocks, with rails, cobwebs, lanterns and a chest or two."""
    W = 25
    layers = []
    for y in range(5):
        g = grid(W, W, " ")
        for i in range(W):
            for (x, z) in ((i, 12), (12, i)):
                for side in (-1, 0, 1):
                    xx, zz = (x, z + side) if z == 12 and x != 12 else (x + side, z)
                    if not (0 <= xx < W and 0 <= zz < W):
                        continue
                    if y == 0:
                        g[zz][xx] = "P" if (i % 5 == 0 and side == 0) else (g[zz][xx] if g[zz][xx] not in " " else "D")
                    elif y in (1, 2):
                        g[zz][xx] = "."
                        if i % 5 == 0 and side != 0:
                            g[zz][xx] = "f"
                    elif y == 3:
                        g[zz][xx] = "P" if i % 5 == 0 else "."
        layers.append(g)
    # Rails down the east-west tunnel, webs and light.
    for x in range(W):
        if x != 12:
            layers[1][12][x] = "r"
    for (x, z) in ((3, 11), (20, 13), (11, 4), (13, 21), (12, 12)):
        layers[2][z][x] = "w"
    for (x, z) in ((5, 12), (12, 5), (12, 19), (19, 12)):
        layers[3][z][x] = "h"
    layers[1][2][11] = "c"
    layers[1][13][22] = "c"
    # A cave spider spawner in the north-south tunnel, wrapped in webs.
    layers[1][17][12] = "S"
    for (x, z) in ((11, 16), (13, 18), (12, 18)):
        layers[2][z][x] = "w"
    return {
        "name": "mina_abandonada",
        "biomes": ["Plains", "Forest", "Birch Forest", "Taiga", "Dark Forest", "Mountains", "Snowy Plains", "Savanna", "Desert",
                   "Jungle", "Badlands", "Meadow", "Cherry Grove"],
        "spacing": 10,
        "chance": 0.35,
        "sink": 28,
        "palette": {"P": "oak_planks", "D": "cobblestone", "f": "oak_fence", "r": "rail", "w": "cobweb", "h": "lantern:1", "c": "chest", "S": "spawner"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "raw_iron", 2, 6, 0.6), ("", "coal", 3, 10, 0.7), ("", "redstone", 2, 8, 0.5), ("", "rail", 4, 12, 0.5),
                 ("", "torch", 4, 12, 0.6), ("", "diamond", 1, 2, 0.15), ("", "gold_ingot", 1, 3, 0.35), ("", "bread", 1, 3, 0.4)],
    }


def shipwreck():
    """A sunken ship on the sea floor: a broken spruce hull lying over, a stub of a mast and
    two chests. The water stays inside."""
    W, L = 7, 17
    layers = []
    for y in range(6):
        g = grid(W, L, " ")
        for z in range(1, L - 1):
            taper = 2 if z < 4 or z > L - 5 else (1 if z < 6 or z > L - 7 else 0)
            lo, hi = taper, W - 1 - taper
            for x in range(lo, hi + 1):
                if y == 0:
                    g[z][x] = "P" if lo < x < hi else "L"
                elif y <= 2:
                    if x in (lo, hi):
                        # Broken planks here and there.
                        g[z][x] = " " if (z * 7 + x + y) % 5 == 0 else "P"
                elif y == 3 and z > L // 2:
                    if x in (lo, hi) or z == L // 2 + 1:
                        g[z][x] = "s"
        if y == 1:
            g[3][3] = "c"
            g[L - 4][3] = "c"
        if 1 <= y <= 5:
            g[L // 2][3] = "L"  # what is left of the mast
        layers.append(g)
    return {
        "name": "naufragio",
        "biomes": ["Ocean", "Frozen Ocean"],
        "spacing": 12,
        "chance": 0.45,
        "palette": {"P": "spruce_planks", "L": "spruce_log", "s": "spruce_slab", "c": "chest"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "gold_ingot", 1, 4, 0.5), ("", "emerald", 1, 3, 0.4), ("", "iron_ingot", 1, 5, 0.5), ("", "paper", 1, 6, 0.6),
                 ("", "diamond", 1, 1, 0.12), ("", "carrot", 1, 4, 0.4), ("", "gold_nugget", 2, 8, 0.4)],
    }

VILLAGE_STYLES = {
    "carvalho": ("oak_planks", "oak_log", "oak_roof", "oak_fence", ["Plains", "Meadow", "Forest", "Birch Forest", "Savanna"]),
    "pinheiro": ("spruce_planks", "spruce_log", "spruce_roof", "spruce_fence", ["Taiga", "Snowy Plains"]),
}


def village(style):
    """A village (vanilla idea): a square with a well in the middle and buildings set round
    it in a ring by the game, each turned with its door to the square. Settlers live in it."""
    plank, log, roof, fence, biomes = VILLAGE_STYLES[style]
    out = []
    # The square: a gravel yard, a well with a roof on posts, lanterns on the corners.
    W = 11
    ground = grid(W, W, "g")
    well_rim = grid(W, W, ".")
    for z in range(3, 8):
        for x in range(3, 8):
            ground[z][x] = "C"
            well_rim[z][x] = "C" if x in (3, 7) or z in (3, 7) else "w"
    for (x, z) in ((0, 0), (W - 1, 0), (0, W - 1), (W - 1, W - 1)):
        well_rim[z][x] = "f"
    posts = grid(W, W, ".")
    lamps = grid(W, W, ".")
    for (x, z) in ((3, 3), (7, 3), (3, 7), (7, 7)):
        posts[z][x] = "f"
        lamps[z][x] = "f"
    for (x, z) in ((0, 0), (W - 1, 0), (0, W - 1), (W - 1, W - 1)):
        posts[z][x] = "l"
    top = grid(W, W, ".")
    for z in range(3, 8):
        for x in range(3, 8):
            top[z][x] = "s"
    top[5][5] = "P"
    clear = grid(W, W, ".")
    pieces = [f"vila_casa_{style}", f"vila_fazenda_{style}", f"vila_ferreiro_{style}", f"vila_casa_{style}_2",
              f"vila_biblioteca_{style}", f"vila_fazenda_{style}_2", f"vila_casa_{style}_3"]
    out.append({
        "name": f"vila_{style}",
        "biomes": biomes, "spacing": 22, "chance": 0.6, "sink": 1, "foundation": "cobblestone",
        "palette": {"g": "gravel", "C": "cobblestone", "w": "water", "f": fence, "l": "lantern", "s": "oak_slab", "P": plank},
        "layers": [rows(l) for l in (ground, well_rim, posts, lamps, top, clear)],
        "loot": [], "pieces": pieces,
    })
    # Homes: the cottage, three times over with the same plan.
    for suffix in ("", "_2", "_3"):
        h = house(plank, log, roof, biomes, f"vila_casa_{style}{suffix}")
        h["piece_only"] = True
        out.append(h)
    # Farms: a fenced field of ripe wheat and carrots round a water channel.
    for suffix, crop in (("", "wheat_stage7"), ("_2", "carrots_stage3")):
        W, D = 9, 9
        soil = grid(W, D, "D")
        plants = grid(W, D, ".")
        rail = grid(W, D, ".")
        for z in range(D):
            for x in range(W):
                edge = x in (0, W - 1) or z in (0, D - 1)
                if edge:
                    soil[z][x] = "L" if x in (0, W - 1) and z in (0, D - 1) else "c"
                    rail[z][x] = "f"
                elif x == 4:
                    soil[z][x] = "w"
                else:
                    plants[z][x] = "h"
        rail[D - 1][4] = "."  # a gap on the square's side to walk in
        plants[0][0] = "o" if suffix == "" else "."
        out.append({
            "name": f"vila_fazenda_{style}{suffix}", "biomes": biomes, "spacing": 22, "chance": 0.0, "sink": 1,
            "foundation": "dirt", "piece_only": True,
            "palette": {"D": "farmland", "c": "cobblestone", "L": log, "w": "water", "h": crop, "f": fence, "o": "composter"},
            "layers": [rows(l) for l in (soil, [[("o" if c == "o" else "h" if c == "h" else ".") for c in r] for r in plants], rail, grid(W, D, "."))],
            "loot": [],
        })
    # The smithy: a stone room open to the front with an anvil-like furnace corner and loot.
    W, D = 9, 8
    layers = []
    floor = grid(W, D, "S")
    layers.append(floor)
    for y in (1, 2, 3):
        g = grid(W, D, ".")
        for z in range(D):
            for x in range(W):
                if z == 0 or x in (0, W - 1):
                    g[z][x] = "L" if x in (0, W - 1) and z in (0, D - 1) else "S"
                elif z == D - 1 and x in (0, W - 1):
                    g[z][x] = "L"
        if y == 1:
            g[1][1] = "F"; g[1][2] = "F"; g[1][3] = "K"; g[1][6] = "T"; g[1][7] = "a"
            g[3][1] = "h"; g[6][7] = "t"
        if y == 2:
            g[1][1] = "S"; g[1][2] = "S"; g[3][0] = "G"; g[3][W - 1] = "G"
        layers.append(g)
    roof_l = grid(W, D, "s")
    layers.append(roof_l)
    out.append({
        "name": f"vila_ferreiro_{style}", "biomes": biomes, "spacing": 22, "chance": 0.0, "foundation": "cobblestone",
        "piece_only": True,
        "palette": {"S": "cobblestone", "L": log, "F": "furnace", "K": "chest", "T": "crafting_table", "a": "stone_bricks",
                    "h": "hay_block", "t": "torch", "G": "glass_pane", "s": "stone_slab"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "iron_ingot", 2, 6, 0.9), ("", "coal", 3, 9, 0.7), ("", "iron_pickaxe", 1, 1, 0.2), ("", "iron_sword", 1, 1, 0.2),
                 ("", "bread", 1, 4, 0.5), ("", "obsidian", 1, 3, 0.3), ("", "diamond", 1, 1, 0.08)],
    })
    # The library: the cottage plan with shelves along the walls.
    b = house(plank, log, roof, biomes, f"vila_biblioteca_{style}")
    b["piece_only"] = True
    lay = [list(map(list, l)) for l in b["layers"]]
    for y in (1, 2):
        for z in range(2, 7):
            lay[y][z][1] = "B" if lay[y][z][1] == "." else lay[y][z][1]
            lay[y][z][5] = "B" if lay[y][z][5] == "." else lay[y][z][5]
    lay[1][2][1] = "B"; lay[1][2][2] = "."; lay[1][2][5] = "K"; lay[1][6][5] = "t"
    b["layers"] = [rows(l) for l in lay]
    b["palette"] = dict(b["palette"], B="bookshelf")
    b["loot"] = [("", "paper", 3, 9, 0.8), ("", "compass", 1, 1, 0.2), ("", "map", 1, 1, 0.3)]
    out.append(b)
    return out
def _h(x, y, z, k=0):
    """A fixed pseudo-random number 0..99 for a cell, so cracks and gold land the same each run."""
    return (x * 73856093 ^ y * 19349663 ^ z * 83492791 ^ k * 2654435761) % 100


def bastion():
    """A bastion remnant: a ruined square keep of blackstone bricks with a courtyard, a
    walkway round the walls on basalt posts, and a treasure room of gold at its heart."""
    W, H = 19, 13
    layers = [grid(W, W, ".") for _ in range(H)]

    def wall(x, y, z):
        r = _h(x, y, z)
        return "C" if r < 18 else ("g" if r < 24 else "P")

    for z in range(W):
        for x in range(W):
            layers[0][z][x] = "B" if _h(x, 0, z, 1) < 80 else "b"
    for y in range(1, H):
        broken = y > 8
        for z in range(W):
            for x in range(W):
                edge = x in (0, W - 1) or z in (0, W - 1)
                if edge and not (broken and _h(x, y, z, 2) < 45):
                    layers[y][z][x] = wall(x, y, z)
    # Gates in the middle of two sides.
    for y in (1, 2, 3):
        for x in (8, 9, 10):
            layers[y][0][x] = "."
            layers[y][W - 1][x] = "."
    # The walkway round the inside of the walls at level 6, on basalt posts.
    for z in range(1, W - 1):
        for x in range(1, W - 1):
            if x in (1, 2, W - 3, W - 2) or z in (1, 2, W - 3, W - 2):
                layers[6][z][x] = "P" if _h(x, 6, z, 3) > 10 else "."
    for (x, z) in ((2, 2), (2, W - 3), (W - 3, 2), (W - 3, W - 3), (9, 2), (9, W - 3), (2, 9), (W - 3, 9)):
        for y in range(1, 6):
            layers[y][z][x] = "S"
    # The treasure room: a 7x7 keep with gold in the middle and chests.
    k0, k1 = 6, 12
    for y in range(1, 6):
        for z in range(k0, k1 + 1):
            for x in range(k0, k1 + 1):
                if x in (k0, k1) or z in (k0, k1):
                    layers[y][z][x] = wall(x, y, z)
    for z in range(k0, k1 + 1):
        for x in range(k0, k1 + 1):
            layers[5][z][x] = "P"
    for y in (1, 2):
        layers[y][k0][9] = "."
    for z in range(8, 11):
        for x in range(8, 11):
            layers[1][z][x] = "G"
    layers[2][9][9] = "G"
    layers[1][k0 + 1][k0 + 1] = "c"
    layers[1][k1 - 1][k1 - 1] = "c"
    layers[1][k1 - 1][k0 + 1] = "L"
    layers[1][k0 + 1][k1 - 1] = "L"
    # Chains of light hanging in the courtyard, and a chest on the walkway.
    layers[7][3][3] = "c"
    for (x, z) in ((4, 4), (14, 14), (4, 14), (14, 4)):
        layers[1][z][x] = "M"
    return {
        "name": "bastiao",
        "biomes": ["Nether Wastes", "Crimson Forest", "Warped Forest", "Soul Sand Valley"],
        "spacing": 14,
        "chance": 0.6,
        "foundation": "blackstone",
        "palette": {"B": "blackstone", "b": "gilded_blackstone", "P": "polished_blackstone_bricks",
                    "C": "cracked_polished_blackstone_bricks", "g": "gilded_blackstone", "S": "basalt",
                    "G": "gold_block", "c": "chest", "L": "shroomlight", "M": "magma_block"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "gold_ingot", 3, 9, 0.8), ("", "gold_block", 1, 2, 0.25), ("", "netherite_scrap", 1, 1, 0.2), ("", "netherite_upgrade_smithing_template", 1, 1, 0.5),
                 ("", "ancient_debris", 1, 1, 0.15), ("", "golden_helmet", 1, 1, 0.2), ("", "golden_chestplate", 1, 1, 0.15),
                 ("", "crying_obsidian", 1, 4, 0.3), ("", "diamond", 1, 2, 0.12), ("", "gold_nugget", 5, 15, 0.6),
                 ("", "magma_cream", 2, 5, 0.3)],
    }


def _end_tower():
    """An End city tower: a purpur keep on an end stone brick base, three storeys joined by
    a ladder well, lit by end rods, with a loot room at the top under a crown of pillars."""
    W, H = 11, 26
    layers = [grid(W, W, ".") for _ in range(H)]
    for z in range(W):
        for x in range(W):
            layers[0][z][x] = "E"
            layers[1][z][x] = "E"
    floors = (2, 9, 16)
    top = 23
    for y in range(2, top + 1):
        for z in range(1, W - 1):
            for x in range(1, W - 1):
                edge = x in (1, W - 2) or z in (1, W - 2)
                corner = x in (1, W - 2) and z in (1, W - 2)
                if corner:
                    layers[y][z][x] = "I"
                elif y in floors or y == top:
                    layers[y][z][x] = "Q"
                elif edge:
                    layers[y][z][x] = "Q"
    # Windows of glass-like gaps and the door.
    for f in floors:
        for x in (4, 6):
            layers[f + 3][1][x] = "."
            layers[f + 3][W - 2][x] = "."
            layers[f + 3][x][1] = "."
            layers[f + 3][x][W - 2] = "."
        layers[f + 1][5][5] = "R"
    for y in (3, 4):
        layers[y][1][5] = "."
    # Stairs of purpur blocks up the inside of the east wall, through a gap in each floor.
    for f, nxt in zip(floors, floors[1:] + (top,)):
        for k in range(nxt - f):
            layers[f + 1 + k][2 + k][W - 3] = "Q"
        for k in range(1, nxt - f - 1):
            layers[nxt][2 + k][W - 3] = "."
    # The loot room at the top.
    layers[top - 6][3][3] = "c"
    layers[top - 6][3][7] = "c"
    layers[top - 5][5][5] = "R"
    # A crown of purpur pillars and rods on the roof.
    for (x, z) in ((1, 1), (1, W - 2), (W - 2, 1), (W - 2, W - 2)):
        layers[top + 1][z][x] = "I"
        layers[top + 2][z][x] = "R"
    for x in range(2, W - 2):
        if x % 2 == 0:
            layers[top + 1][1][x] = "Q"
            layers[top + 1][W - 2][x] = "Q"
            layers[top + 1][x][1] = "Q"
            layers[top + 1][x][W - 2] = "Q"
    return {
        "name": "cidade_do_end",
        "biomes": ["End Highlands"],
        "spacing": 12,
        "chance": 0.5,
        "foundation": "end_stone_bricks",
        "palette": {"E": "end_stone_bricks", "Q": "purpur_block", "I": "purpur_pillar", "R": "end_rod", "c": "chest"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "diamond", 1, 3, 0.3), ("", "iron_ingot", 3, 8, 0.6), ("", "gold_ingot", 2, 7, 0.6),
                 ("", "emerald", 2, 6, 0.4), ("", "diamond_sword", 1, 1, 0.12), ("", "diamond_chestplate", 1, 1, 0.1),
                 ("", "diamond_pickaxe", 1, 1, 0.1), ("", "shulker_shell", 1, 2, 0.3), ("", "ender_pearl", 1, 3, 0.4),
                 ("", "chorus_fruit", 2, 6, 0.4)],
    }


def end_city():
    """An End city: the tall tower, a lower tower beside it and a purpur bridge between them."""
    t = _end_tower()
    tall = t["layers"]
    W, D, H = 21, 21, len(tall)
    layers = [grid(W, D, ".") for _ in range(H)]
    for y, layer in enumerate(tall):
        for z, row in enumerate(layer):
            for x, ch in enumerate(row):
                layers[y][z][x] = ch
    # The lower tower in the far corner: 7x7, two storeys, a crown of rods.
    x0, z0, top = 14, 14, 13
    for y in range(0, top + 1):
        for z in range(z0, z0 + 7):
            for x in range(x0, x0 + 7):
                edge = x in (x0, x0 + 6) or z in (z0, z0 + 6)
                corner = x in (x0, x0 + 6) and z in (z0, z0 + 6)
                if y < 2:
                    layers[y][z][x] = "E"
                elif corner:
                    layers[y][z][x] = "I"
                elif y in (2, 9, top) or edge:
                    layers[y][z][x] = "Q"
    for y in (3, 4):
        layers[y][z0][x0 + 3] = "."
    layers[10][z0 + 3][x0 + 3] = "c"
    layers[3][z0 + 3][x0 + 3] = "R"
    for (x, z) in ((x0, z0), (x0 + 6, z0), (x0, z0 + 6), (x0 + 6, z0 + 6)):
        layers[top + 1][z][x] = "R"
    # The bridge from the big tower's second floor to the small tower's upper floor.
    for x in range(9, 18):
        for z in (5, 6):
            layers[9][z][x] = "Q"
        layers[10][4][x] = "R" if x % 3 == 0 else "."
    for z in range(5, z0 + 1):
        for x in (16, 17):
            layers[9][z][x] = "Q"
    for y in (10, 11):
        for z in (5, 6):
            layers[y][z][9] = "."
        for x in (16, 17):
            layers[y][z0][x] = "."
    t["layers"] = [rows(l) for l in layers]
    t["foundation"] = "end_stone"
    return t


def end_ship():
    """An End ship floating over an island: a purpur hull with a mast and a cabin; the
    dragon's head on its bow and a pair of elytra in the chest below deck."""
    W, D, H = 7, 21, 12
    layers = [grid(W, D, ".") for _ in range(H)]
    for z in range(D):
        # The hull narrows toward bow (z=0) and stern.
        half = 3 if 4 <= z <= D - 4 else (2 if 2 <= z <= D - 2 else 1)
        for y in range(4):
            w = max(0, half - (3 - y) if y < 3 else half)
            for x in range(3 - w, 3 + w + 1):
                inner = 3 - w < x < 3 + w and 0 < z < D - 1 and y > 0
                layers[y][z][x] = "." if inner and y < 3 else ("O" if y == 0 else "Q")
        # The deck.
        for x in range(3 - half, 3 + half + 1):
            layers[3][z][x] = "Q"
        layers[4][z][3 - half] = "f" if 1 < z < D - 2 else "."
        layers[4][z][3 + half] = "f" if 1 < z < D - 2 else "."
    # Inside the hull: the chest with the elytra, and a light.
    layers[1][D // 2][3] = "c"
    layers[2][D // 2 + 2][3] = "R"
    layers[3][D // 2 + 3][3] = "."  # a hatch down
    # The mast and its sail of purple glass-like blocks.
    for y in range(4, H):
        layers[y][9][3] = "I"
    for y in range(6, H - 1):
        for x in (1, 2, 4, 5):
            layers[y][9][x] = "M"
    # The cabin at the stern.
    for y in range(4, 7):
        for z in range(D - 6, D - 2):
            for x in range(1, 6):
                if x in (1, 5) or z in (D - 6, D - 3) or y == 6:
                    layers[y][z][x] = "Q"
    layers[4][D - 6][3] = "."
    layers[5][D - 6][3] = "."
    # The dragon's head on the bow.
    layers[4][0][3] = "Q"
    layers[5][0][3] = "D"
    return {
        "name": "navio_do_end",
        "biomes": ["End Highlands"],
        "spacing": 14,
        "chance": 0.35,
        "palette": {"Q": "purpur_block", "O": "obsidian", "I": "purpur_pillar", "f": "end_rod", "R": "end_rod",
                    "M": "magenta_wool", "c": "chest", "D": "dragon_head"},
        "layers": [rows(l) for l in layers],
        "loot": [("", "elytra", 1, 1, 1.0), ("", "diamond", 1, 3, 0.4), ("", "ender_pearl", 2, 4, 0.5),
                 ("", "shulker_shell", 1, 2, 0.4), ("", "gold_ingot", 2, 6, 0.5)],
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
        if s.get("piece_only"):
            out.append("        piece_only: true,")
        if s.get("pieces"):
            out.append("        pieces: [" + ", ".join(f'"{p}"' for p in s["pieces"]) + "],")
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
    raider_outpost(),
    nether_fortress(),
    stronghold(),
    desert_temple(),
    jungle_temple(),
    igloo(),
    mineshaft(),
    shipwreck(),
    *village("carvalho"),
    *village("pinheiro"),
    bastion(),
    end_city(),
    end_ship(),
]
OUT.write_text(ron(structs))
print(f"wrote {OUT} ({len(structs)} structures)")
