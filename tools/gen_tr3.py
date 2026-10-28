#!/usr/bin/env python3
"""The third round of the Terraria side (plan/terraria-completo.md, "Reauditoria 3", lotes L17
to L22): golf, the town folk and mini-bosses that were missing, the common monsters of each
land, tools and weapons, traps and statues, food, critters, and the world and character things.

Writes packs/vanilla/tr3/{blocks,items,recipes,mobs,structures}.ron, the creatures' 64x128
skins (tools/tex64/entities.py grows them to 4x), the item icons (32x32 pixel art grown to 64x64,
chunky pixels with a crisp outline in each shape's own dark, like tools/gen_vulcao.py) and the
64x64 block textures. Everything is drawn here; nothing is copied from Terraria.

    python3 tools/gen_tr3.py [sheet.png] && python3 tools/tex64/entities.py
"""

import math
import os
import sys

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "tex64"))
from dim_mobs_spec import Model, biped, bird, blob, look, quadruped  # noqa: E402
from gen_dim_mobs import render as render_mob  # noqa: E402
from gen_mob_skins import hexrgb  # noqa: E402
from gen_vulcao import Art, egg, lit, ramp_fill  # noqa: E402

ROOT = os.path.dirname(HERE)
PACK = os.path.join(ROOT, "packs", "vanilla")
OUT = os.path.join(PACK, "tr3")
ENTITY = os.path.join(PACK, "textures", "entity")
BLOCK_TEX = os.path.join(PACK, "textures", "block")
ITEM_TEX = os.path.join(PACK, "textures", "item")
G = 32

# Everything the lotes add, filled in by each lote's section below.
MOBS = []        # dict(name, display, model, ron, egg)
ITEMS = []       # ron lines
BLOCKS = []      # ron lines
SHAPED = []      # ron lines
SHAPELESS = []   # ron lines
STRUCTURES = []  # ron text
ICONS = {}       # item texture name -> fn() -> 64x64 image
BLOCK_ART = {}   # block texture name -> fn() -> 64x64 RGBA float array or PIL image
EN = {}          # Portuguese display name -> English


def item(name, display, en, stack=64, extra=""):
    s = f'    (name: "{name}", display_name: Some("{display}")'
    if stack != 64:
        s += f", max_stack: Some({stack})"
    ITEMS.append(s + (", " + extra if extra else "") + "),")
    EN[display] = en


def block(name, display, en, rest):
    BLOCKS.append(f'    (name: "{name}", display_name: Some("{display}"), {rest}),')
    EN[display] = en


def shaped(pattern, key, result, count=1):
    keys = ", ".join(f"'{k}': \"{v}\"" for k, v in key.items())
    pat = ", ".join(f'"{r}"' for r in pattern)
    SHAPED.append(f'        (pattern: [{pat}], key: {{{keys}}}, result: "{result}"' + (f", count: {count}" if count != 1 else "") + "),")


def shapeless(ings, result, count=1):
    i = ", ".join(f'"{x}"' for x in ings)
    SHAPELESS.append(f'        (ingredients: [{i}], result: "{result}"' + (f", count: {count}" if count != 1 else "") + "),")


def mob(name, display, en, model, ron, eggs=None):
    model.pack()
    MOBS.append(dict(name=name, display=display, model=model, ron=ron, egg=eggs))
    EN[display] = en


# ------------------------------------------------------------------------------- drawing helpers


def sprite(fn):
    """An icon drawn on a 32x32 Art, grown to 64x64 with hard pixels."""
    def make():
        a = Art()
        fn(a)
        return a.image()
    return make


def flat(col):
    return col


def shade_fill(pal, f):
    """fill(xs, ys) from a dark-to-light palette and a 0..1 field (a constant is fine)."""
    return ramp_fill(pal, lambda xs, ys: np.broadcast_to(np.asarray(f(xs, ys), float), xs.shape))


def lin(x0, y0, x1, y1, w, fill, edge, a):
    """A thick straight stroke as a polygon."""
    dx, dy = x1 - x0, y1 - y0
    n = math.hypot(dx, dy) or 1.0
    px, py = -dy / n * w / 2, dx / n * w / 2
    return a.poly([(x0 + px, y0 + py), (x1 + px, y1 + py), (x1 - px, y1 - py), (x0 - px, y0 - py)], fill, edge)


STEEL = ["#2a2e36", "#4a515c", "#737c88", "#a4adb8", "#d8dee6"]
GOLD = ["#5a3a08", "#9a6a14", "#d4a02a", "#f2cc52", "#fff0a0"]
WOOD = ["#2a1a0e", "#4a2e18", "#6e4626", "#946236", "#b88050"]
GRIP = ["#141418", "#26262e", "#3a3a46"]
WHITE = ["#8a8a90", "#b4b4ba", "#d8d8de", "#f2f2f6", "#ffffff"]
RED = ["#4a0a0a", "#8a1414", "#c42424", "#e85050", "#ff9090"]


def club(head):
    """A golf club seen diagonally: grip at the bottom left, shaft, head at the top right."""
    def draw(a):
        lin(7, 26, 22, 8, 2.2, shade_fill(STEEL, lambda xs, ys: 0.75 - (xs - 6) / 40), "#16181c", a)
        lin(4, 30, 9, 24, 3.4, shade_fill(GRIP, lambda xs, ys: 0.6 - (xs - 4) / 12), "#08080a", a)
        head(a)
    return draw


def driver_head(a):
    a.poly([(19, 9), (24, 4), (30, 6), (30, 11), (25, 13)], shade_fill(["#141418", "#262630", "#3e3e4e", "#5a5a6e", "#8a8aa4"], lit(25, 8, 6)), "#08080c")
    a.px(27, 6, "#c8c8e0")
    a.px(26, 7, "#a0a0c0")


def iron_head(a):
    a.poly([(20, 7), (25, 5), (29, 11), (24, 13)], shade_fill(STEEL, lit(24, 9, 5)), "#16181c")
    for y in (8, 9, 10):
        a.px(24 + (y - 8), y + 1, "#5a626e")


def wedge_head(a):
    a.poly([(20, 6), (24, 3), (30, 12), (25, 14)], shade_fill(GOLD[1:] + ["#fff8d0"], lit(25, 8, 6)), "#3a2604")
    for k in range(4):
        a.px(23 + k, 7 + k * 2, "#8a6010")


def putter_head(a):
    a.poly([(17, 9), (29, 5), (31, 9), (19, 13)], shade_fill(["#3a1010", "#6a1c1c", "#9a2a2a", "#c84040", "#ee7070"], lit(24, 9, 7)), "#1a0606")
    a.px(28, 7, "#ffd0d0")


def golf_ball(a):
    a.disc(16, 16, 7.5, shade_fill(WHITE, lambda xs, ys: 1.0 - np.hypot(xs - 13, ys - 13) / 13), "#55555c")
    for x, y in ((14, 12), (18, 13), (12, 16), (16, 17), (20, 17), (14, 20), (18, 21)):
        a.px(x, y, "#a8a8b0")
    a.px(12, 11, "#ffffff")
    a.px(13, 10, "#ffffff")


def cart_key(a):
    a.disc(11, 12, 6.5, shade_fill(["#0a3a1a", "#145a2a", "#1e8040", "#3aaa5e", "#7ad898"], lit(10, 11, 6)), "#04180a")
    a.disc(11, 12, 2.2, "#04180a", "#04180a")
    lin(15, 16, 26, 27, 3.0, shade_fill(GOLD, lambda xs, ys: 0.8 - (xs - 15) / 20), "#3a2604", a)
    a.poly([(21, 24), (24, 21), (26, 23), (23, 26)], shade_fill(GOLD, lambda xs, ys: 0.6), "#3a2604")
    a.poly([(24, 27), (27, 24), (29, 26), (26, 29)], shade_fill(GOLD, lambda xs, ys: 0.5), "#3a2604")


def trophy(pal, edge):
    def draw(a):
        a.poly([(9, 5), (23, 5), (22, 13), (19, 17), (13, 17), (10, 13)], shade_fill(pal, lambda xs, ys: 0.95 - (xs - 9) / 18 - (ys - 5) / 40), edge)
        a.poly([(5, 6), (9, 6), (10, 12), (8, 12)], shade_fill(pal, lambda xs, ys: 0.5), edge)
        a.poly([(23, 6), (27, 6), (24, 12), (22, 12)], shade_fill(pal, lambda xs, ys: 0.4), edge)
        a.poly([(14, 17), (18, 17), (18, 22), (14, 22)], shade_fill(pal, lambda xs, ys: 0.55), edge)
        a.poly([(10, 22), (22, 22), (23, 27), (9, 27)], shade_fill(["#1e140c", "#3a2614", "#5a3c20"], lambda xs, ys: 0.7 - (xs - 9) / 20), "#0c0804")
        a.px(12, 7, "#ffffff")
        a.px(12, 8, pal[-1])
        a.disc(16, 10, 2.0, "#f2f2f6", "#55555c")
    return draw


ICONS.update({
    "golf_club_driver": sprite(club(driver_head)),
    "golf_club_iron": sprite(club(iron_head)),
    "golf_club_wedge": sprite(club(wedge_head)),
    "golf_club_putter": sprite(club(putter_head)),
    "golf_ball": sprite(golf_ball),
    "golf_cart_key": sprite(cart_key),
})


def blocks_art():
    """The 64x64 block textures (tools/tex64 style)."""
    import blocks as tb
    from style import N, XX, YY, clusters, ramp, rng_for

    def turf_top():
        img = tb.grass_top("golf_turf_top", base="#62b04a")
        # Mown stripes: every 8 texels lighter, then darker, along x.
        band = ((XX // 8) % 2 == 0)
        img[band, :3] = np.clip(img[band, :3] * 1.12, 0, 255)
        img[~band, :3] *= 0.94
        return img

    def turf_side():
        img = tb.fringe_side("golf_turf_side", tb.dirt, "#62b04a", depth=10)
        return img

    def cup_top():
        img = turf_top()
        d = np.hypot(XX - 31.5, YY - 31.5)
        hole = d < 13
        img[hole, :3] = np.array([16, 14, 12]) + np.clip(13 - d[hole], 0, 13)[:, None] * np.array([1.5, 1.2, 1.0])
        rim = (d >= 13) & (d < 15.5)
        img[rim, :3] = np.array([236, 236, 240]) - (d[rim] - 13)[:, None] * 18
        return img

    def art_block(draw):
        def make():
            a = Art()
            draw(a)
            return a.image()
        return make

    def tee(a):
        a.poly([(14, 22), (18, 22), (17, 30), (15, 30)], shade_fill(["#7a5a2a", "#b08848", "#d8b878"], lambda xs, ys: 0.8 - (xs - 14) / 6), "#3a2a10")
        a.poly([(12, 20), (20, 20), (19, 22), (13, 22)], shade_fill(["#7a5a2a", "#b08848", "#e8c888"], lambda xs, ys: 0.8), "#3a2a10")

    def flag(a):
        lin(15, 31, 15, 2, 1.6, shade_fill(WHITE, lambda xs, ys: 0.8), "#55555c", a)
        a.poly([(16, 2), (28, 6), (16, 11)], shade_fill(RED, lambda xs, ys: 0.9 - (xs - 16) / 16 - (ys - 2) / 30), "#2a0404")
        a.px(18, 4, "#ff9090")

    out = {"golf_turf_top": turf_top, "golf_turf_side": turf_side, "golf_cup_top": cup_top,
           "golf_tee": art_block(tee), "golf_flag": art_block(flag)}
    for k, (pal, edge) in {"bronze": (["#3a1e0a", "#6a3a14", "#9a5a24", "#c8823a", "#eab070"], "#1a0c04"),
                           "silver": (["#3a3e46", "#626a76", "#9aa2ae", "#c8d0da", "#f0f4f8"], "#16181c"),
                           "gold": (GOLD, "#3a2604")}.items():
        out[f"golf_trophy_{k}"] = art_block(trophy(pal, edge))
    return out


# ==============================================================================================
# L17: golf
# ==============================================================================================

item("golf_club_driver", "Taco Driver", "Golf Driver", 1)
item("golf_club_iron", "Taco de Ferro", "Golf Iron", 1)
item("golf_club_wedge", "Taco Wedge", "Golf Wedge", 1)
item("golf_club_putter", "Putter", "Golf Putter", 1)
item("golf_ball", "Bola de Golfe", "Golf Ball", 16)
item("golf_cart_key", "Chave do Carrinho de Golfe", "Golf Cart Key", 1)
block("golf_turf", "Grama de Golfe", "Golf Turf", 'hardness: 0.6, tool: Some(Shovel), drops: Some([(item: "golf_turf")]), textures: (top: Some("golf_turf_top"), side: Some("golf_turf_side"), bottom: Some("dirt"))')
block("golf_cup", "Buraco de Golfe", "Golf Cup", 'hardness: 0.6, tool: Some(Shovel), textures: (top: Some("golf_cup_top"), side: Some("golf_turf_side"), bottom: Some("dirt"))')
block("golf_tee", "Tee de Golfe", "Golf Tee", 'shape: Cross, transparency: Cutout, solid: false, hardness: 0.0, textures: (all: Some("golf_tee"))')
block("golf_flag", "Bandeira de Golfe", "Golf Flag", 'shape: Cross, transparency: Cutout, solid: false, hardness: 0.2, textures: (all: Some("golf_flag"))')
for k, dn, en in (("bronze", "Troféu de Golfe de Bronze", "Bronze Golf Trophy"), ("silver", "Troféu de Golfe de Prata", "Silver Golf Trophy"), ("gold", "Troféu de Golfe de Ouro", "Gold Golf Trophy")):
    block(f"golf_trophy_{k}", dn, en, f'shape: Cross, transparency: Cutout, solid: false, hardness: 0.5, textures: (all: Some("golf_trophy_{k}"))')

shaped([" II", " SI", "S  "], {"I": "iron_ingot", "S": "stick"}, "golf_club_driver")
shaped(["  I", " S ", "S  "], {"I": "iron_ingot", "S": "stick"}, "golf_club_iron")
shaped(["  G", " S ", "S  "], {"G": "gold_ingot", "S": "stick"}, "golf_club_wedge")
shaped([" II", " S ", "S  "], {"I": "copper_ingot", "S": "stick"}, "golf_club_putter")
shapeless(["bone_meal", "clay_ball", "white_dye"], "golf_ball", 4)
shapeless(["grass_block", "grass_block", "wheat_seeds"], "golf_turf", 2)
shapeless(["golf_turf", "bucket"], "golf_cup")
shapeless(["stick", "oak_planks"], "golf_tee", 4)
shapeless(["stick", "red_wool"], "golf_flag")

SKIN = ["#a0745a", "#bc8c6e", "#d6a888"]
golfer = biped(look("glfh", SKIN, eyes=((40, 30, 24), None)), look("glfshirt", ["#a8c8e8", "#c8dcf0", "#e6f0fa"], pattern="cloth"),
               look("glfarm", SKIN), look("glfpants", ["#4a4232", "#6a5e48", "#8a7c60"], pattern="cloth"))
cap = look("glfcap", ["#1a3a6a", "#2a5aa0", "#4a7ac8"], pattern="cloth")
golfer.box((9, 2, 9), (-4, 31, -4), cap, "Head", (0, 24, 0))
golfer.box((8, 1, 4), (-4, 31, 4), cap, "Head", (0, 24, 0))
golfer.box((8, 3, 5), (-4, 12, -2), look("glfbelt", ["#2a1a10", "#4a3020", "#6a4830"]))
golfer.layer(1, 0.3)
mob("npc_golfer", "Golfista", "Golfer", golfer, "kind: Passive, health: 40, speed: 2.0, xp: 0, width: 0.6, height: 1.95, spawn: Never, attack: 0, town: true")


def golf_cart():
    m = Model()
    body = look("gcbody", ["#d8d8d0", "#ececE4".lower(), "#f8f8f4"], pattern="plate")
    seat = look("gcseat", ["#4a2a14", "#6a4022", "#8a5832"], pattern="cloth")
    roof = look("gcroof", ["#1a5a2a", "#22803a", "#3aaa52"], pattern="plate")
    wheel = look("gcwheel", ["#121214", "#222226", "#3a3a40"])
    pole = look("gcpole", ["#6a6a70", "#9a9aa0", "#c8c8d0"])
    m.box((10, 3, 16), (-5, 2, -8), body)
    m.box((8, 2, 4), (-4, 5, -3), seat)
    m.box((8, 4, 1), (-4, 7, -4), seat)
    m.box((10, 2, 3), (-5, 5, 5), body)
    m.box((7, 3, 3), (-3, 5, -8), look("gcbag", ["#3a2a5a", "#5a4282", "#7a5aa8"], pattern="cloth"))
    for x in (-5, 4):
        for z in (-7, 5):
            m.box((1, 9, 1), (x, 5, z), pole)
    m.box((11, 1, 14), (-5, 14, -7), roof)
    for x in (-6, 5):
        for z in (-7, 4):
            m.box((1, 3, 3), (x, 0, z), wheel)
    m.box((3, 1, 1), (-1, 8, 6), look("gcwheel2", ["#121214", "#222226", "#3a3a40"]))
    return m


mob("mount_golf_cart", "Carrinho de Golfe", "Golf Cart", golf_cart(), "kind: Passive, health: 200, speed: 6.5, attack: 0, drops: [], xp: 0, width: 1.4, height: 1.2, spawn: Never, scale: 2.4")


def golf_course():
    """A short hole on the plains: a tee box, a fairway of mown turf around a bunker, a green
    with the cup and its flag, rough at the edges, and a little shed with the gear."""
    W, L = 17, 46
    ground = [[" "] * W for _ in range(L)]
    top = [["."] * W for _ in range(L)]
    roof = [["."] * W for _ in range(L)]
    for z in range(L):
        # The fairway bends gently: its middle wanders across the plan.
        mid = 8 + 2.5 * math.sin(z / L * math.pi * 1.2)
        half = 3.2 + 1.3 * math.sin(z / L * math.pi)
        for x in range(W):
            d = abs(x - mid)
            if d <= half:
                ground[z][x] = "T"
            elif d <= half + 1.6:
                ground[z][x] = "G"
    # The tee box (square of turf) and the tee.
    for z in range(1, 5):
        for x in range(5, 12):
            ground[z][x] = "T"
    top[2][8] = "e"
    # Bunkers.
    for (cx, cz, r) in ((5.5, 24, 2.2), (11.5, 31, 1.8), (10.5, 41, 1.6)):
        for z in range(L):
            for x in range(W):
                if (x - cx) ** 2 + ((z - cz) * 0.8) ** 2 <= r * r:
                    ground[z][x] = "S"
    # The green and the cup.
    for z in range(36, 44):
        for x in range(4, 13):
            if (x - 8) ** 2 + (z - 39.5) ** 2 <= 14:
                ground[z][x] = "T"
    ground[39][8] = "C"
    top[39][8] = "F"
    # The shed by the tee: a 4x4 hut with a chest.
    for z in range(0, 4):
        for x in range(13, 17):
            ground[z][x] = "P"
            wall = x in (13, 16) or z in (0, 3)
            top[z][x] = "W" if wall else "."
            roof[z][x] = "R"
    top[3][14] = "."
    top[1][15] = "K"
    top[1][14] = "t"
    layers = [["".join(r) for r in ground], ["".join(r) for r in top], ["".join(r) for r in roof]]
    rows = lambda ls: "[" + ", ".join(f'"{r}"' for r in ls) + "]"
    return f"""    (
        name: "campo_de_golfe",
        biomes: ["Plains", "Meadow", "Sunflower Plains", "Flower Forest", "Savanna"],
        spacing: 40,
        chance: 0.35,
        palette: [('T', "golf_turf"), ('G', "grass_block"), ('S', "sand"), ('C', "golf_cup"), ('F', "golf_flag"), ('e', "golf_tee"), ('P', "oak_planks"), ('W', "oak_log"), ('R', "oak_slab"), ('K', "chest"), ('t', "torch")],
        layers: [
            {rows(layers[0])},
            {rows(layers[1])},
            {rows(layers[2])},
        ],
        sink: 1,
        rough: 3,
        loot: [(item: "golf_ball", min: 2, max: 5), (item: "golf_club_iron"), (item: "golf_club_putter", chance: 0.6), (item: "golf_club_driver", chance: 0.4), (item: "golf_club_wedge", chance: 0.4), (item: "golf_tee", min: 2, max: 4), (item: "bread", min: 1, max: 3)],
    ),
"""


STRUCTURES.append(golf_course())



# --- More icon shapes (used by L18 onwards) ---------------------------------------------------
BONE_ICON = ["#5a5446", "#8a8270", "#b4ac98", "#d8d2bc", "#f6f2e4"]


def mug(a):
    a.poly([(8, 9), (21, 9), (20, 27), (9, 27)], shade_fill(WOOD, lambda xs, ys: 0.85 - (xs - 8) / 16), "#140c06")
    for y in (13, 22):
        a.poly([(8, y), (21, y), (21, y + 1), (8, y + 1)], shade_fill(STEEL, lambda xs, ys: 0.6), "#16181c")
    a.poly([(21, 12), (26, 12), (26, 22), (21, 22), (21, 19), (24, 19), (24, 15), (21, 15)], shade_fill(WOOD, lambda xs, ys: 0.5), "#140c06")
    a.poly([(7, 5), (22, 5), (22, 9), (7, 9)], shade_fill(["#a89a7a", "#d8ccb0", "#f4ecd8", "#ffffff"], lambda xs, ys: 0.9 - (xs - 7) / 30), "#6a604a")
    a.px(10, 6, "#ffffff")


def hat(a, pal, wide=False):
    w = 13 if wide else 10
    a.poly([(16 - w, 20), (16 + w, 20), (16 + w - 2, 24), (16 - w + 2, 24)], shade_fill(pal, lambda xs, ys: 0.5 - (xs - 4) / 60), "#0a0604")
    a.poly([(9, 10), (23, 10), (24, 20), (8, 20)], shade_fill(pal, lambda xs, ys: 0.95 - (xs - 8) / 18), "#0a0604")
    a.poly([(8, 17), (24, 17), (24, 19), (8, 19)], shade_fill(["#3a1a0a", "#5a2a12", "#7a3c1a"], lambda xs, ys: 0.6), "#0a0604")
    a.px(11, 12, pal[-1])


def cone_hat(a, pal):
    a.poly([(16, 2), (24, 22), (8, 22)], shade_fill(pal, lambda xs, ys: 0.95 - (xs - 8) / 18), "#04040c")
    a.poly([(4, 22), (28, 22), (26, 26), (6, 26)], shade_fill(pal, lambda xs, ys: 0.5), "#04040c")
    for x, y in ((15, 9), (18, 15), (12, 17)):
        a.px(x, y, "#f2cc52")


def helmet(a):
    a.poly([(7, 12), (12, 7), (20, 7), (25, 12), (26, 22), (6, 22)], shade_fill(GOLD, lit(13, 11, 9)), "#3a2604")
    a.poly([(4, 21), (28, 21), (28, 24), (4, 24)], shade_fill(GOLD, lambda xs, ys: 0.4), "#3a2604")
    a.disc(16, 15, 3.2, shade_fill(["#a89040", "#e8d070", "#fff4b0", "#ffffff"], lit(15, 14, 3)), "#5a4a10")


def plank(a, pal):
    a.poly([(4, 10), (28, 6), (28, 22), (4, 26)], shade_fill(pal, lambda xs, ys: 0.4 + 0.4 * np.sin(ys * 0.9 + xs * 0.15) * 0.5 + 0.2), pal[0])
    for y in (12, 18):
        a.poly([(5, y + 2), (27, y - 2), (27, y - 1), (5, y + 3)], shade_fill(pal, lambda xs, ys: 0.1), pal[0])


def wing(a, pal):
    a.poly([(4, 6), (28, 4), (24, 12), (28, 16), (20, 20), (22, 26), (8, 22)], shade_fill(pal, lambda xs, ys: 0.95 - (xs - 4) / 30 - (ys - 4) / 50), "#0a0202")
    for k in range(3):
        lin(6, 8, 26 - k * 3, 6 + k * 8, 1.0, shade_fill(pal, lambda xs, ys: 0.1), pal[0], a)


def key_icon(a, pal):
    a.disc(10, 10, 6.0, shade_fill(pal, lit(9, 9, 5)), "#2a2620")
    a.disc(10, 10, 2.2, "#2a2620", "#2a2620")
    lin(14, 14, 27, 27, 3.0, shade_fill(pal, lambda xs, ys: 0.8 - (xs - 14) / 20), "#2a2620", a)
    a.poly([(21, 25), (24, 22), (26, 24), (23, 27)], shade_fill(pal, lambda xs, ys: 0.5), "#2a2620")


# ==============================================================================================
# L18: town folk, mini-bosses, event bosses and rare monsters
# ==============================================================================================

EYE = ((30, 24, 20), None)
GLOW_RED = ((255, 80, 40), "glow")
GLOW_ORANGE = ((255, 170, 60), "glow")


def folk(key, skin, shirt, pants, arms=None, eyes=EYE, tall=12):
    """A biped townsperson: head of `skin`, shirt, arms (shirt colour unless given), trousers."""
    return biped(look(key + "h", skin, eyes=eyes), look(key + "s", shirt, pattern="cloth"),
                 look(key + "a", arms or skin), look(key + "p", pants, pattern="cloth"), tall=tall)


def head_box(m, size, at, lk, tall=12):
    return m.box(size, at, lk, "Head", (0, 2 * tall, 0))


# --- Town folk -------------------------------------------------------------------------------
tav = folk("tav", SKIN, ["#6a4a2a", "#8a6640", "#aa8458"], ["#3a2a1a", "#52402a", "#6a563a"])
head_box(tav, (8, 3, 2), (-4, 24, 4), look("tavbeard", ["#5a2a10", "#7a3c18", "#9a5226"]))
tav.box((8, 9, 1), (-4, 4, 2), look("tavapron", ["#c8c0b0", "#ddd6c8", "#f0ece2"], pattern="cloth"))
tav.layer(1, 0.3)
mob("npc_tavernkeep", "Taverneiro", "Tavernkeep", tav, "kind: Passive, health: 40, speed: 2.0, xp: 0, width: 0.6, height: 1.95, spawn: Never, attack: 0, town: true")

BONE = ["#8a8270", "#b4ac98", "#d8d2bc", "#f0ecdc"]
skm = folk("skm", BONE, ["#4a3a6a", "#64508a", "#7c68a6"], BONE, arms=BONE, eyes=((40, 200, 220), "glow"))
skm.box((6, 8, 4), (-3, 13, -6), look("skmpack", ["#4a2e14", "#6a4420", "#8a5c2e"], pattern="carved"))
head_box(skm, (10, 1, 10), (-5, 32, -5), look("skmhat", ["#2a1e14", "#3e2c1c", "#544028"], pattern="cloth"))
head_box(skm, (6, 3, 6), (-3, 33, -3), look("skmhat", ["#2a1e14", "#3e2c1c", "#544028"], pattern="cloth"))
mob("npc_skeleton_merchant", "Mercador Esqueleto", "Skeleton Merchant", skm, "kind: Passive, health: 60, speed: 1.6, xp: 0, width: 0.6, height: 1.95, spawn: Never, attack: 0, town: true")

old = folk("oldm", SKIN, ["#4a4a52", "#62626c", "#7c7c88"], ["#3a3a42", "#4e4e58", "#62626e"], arms=["#4a4a52", "#62626c", "#7c7c88"])
head_box(old, (8, 7, 2), (-4, 19, 4), look("oldbeard", ["#a8a8a8", "#cacaca", "#ececec"], pattern="cloth"))
old.box((10, 7, 6), (-5, 5, -3), look("oldrobe", ["#3a3a42", "#4e4e58", "#62626e"], pattern="cloth"))
mob("npc_old_man", "Velho da Masmorra", "Old Man", old, "kind: Passive, health: 9999, speed: 0.6, xp: 0, width: 0.6, height: 1.95, spawn: Never, attack: 0, town: true")

# Town jellies: little blobs that live in houses like the other folk.
for key, dn, en, pal, extra in (
        ("nerd", "Geleia Nerd", "Nerdy Jelly", ["#1a4a8a", "#2a6ac0", "#4a8ae0", "#8ab8f4"], "glasses"),
        ("cool", "Geleia Legal", "Cool Jelly", ["#2a6a1a", "#3e9a2a", "#5ac046", "#9ae082"], "shades"),
        ("elder", "Geleia Anciã", "Elder Jelly", ["#5a2a7a", "#7a42a0", "#9a62c4", "#c8a0e8"], "beard")):
    m = Model()
    m.box((10, 8, 10), (-5, 0, -5), look(f"tj{key}", pal, eyes=((20, 20, 30), None), pattern="noise"), "Head", (0, 4, 0))
    m.box((6, 4, 6), (-3, 1, -3), look(f"tj{key}core", pal[1:] + ["#ffffff"]))
    if extra == "glasses":
        m.box((8, 2, 1), (-4, 4, 5), look("tjglass", ["#101014", "#202028", "#c8e0f0"]), "Head", (0, 4, 0))
    elif extra == "shades":
        m.box((8, 2, 1), (-4, 5, 5), look("tjshades", ["#08080a", "#141418", "#2a2a34"]), "Head", (0, 4, 0))
    else:
        m.box((6, 3, 1), (-3, 0, 5), look("tjbeard", ["#b8b8b8", "#dadada", "#f6f6f6"], pattern="cloth"), "Head", (0, 4, 0))
    m.layer(0, 0.5)
    mob(f"town_jelly_{key}", dn, en, m, "kind: Passive, health: 40, speed: 1.4, xp: 0, width: 0.65, height: 0.5, spawn: Never, attack: 0, town: true, hops: true, motion: (style: Some(\"hopper\"))")

# --- Mini-bosses ------------------------------------------------------------------------------
def big_skull(key, pal, eyes, hands=True):
    m = Model()
    m.box((16, 14, 14), (-8, 6, -7), look(key, pal, eyes=eyes, pattern="carved"), "Head", (0, 12, 0))
    m.box((10, 4, 10), (-5, 2, -4), look(key + "jaw", pal, pattern="carved"), "Head", (0, 12, 0))
    if hands:
        for k, x in enumerate((-16, 10)):
            m.box((6, 8, 6), (x, 4, 0), look(key + "hand", pal, pattern="carved"), f"Arm({k})", (x + 3, 10, 3))
    return m


mob("dungeon_guardian", "Guardião da Masmorra", "Dungeon Guardian", big_skull("dgskull", ["#6a6250", "#9a9280", "#cac2ac", "#f2ecd8"], ((255, 40, 40), "glow")),
    "kind: Hostile, health: 9999, speed: 0, attack: 60, xp: 0, width: 1.4, height: 1.6, spawn: Never, flies: true, hover: 1.0, scale: 1.6, lit: true, coins: Some(0)",
    eggs=("#cac2ac", "#c42424"))

ARMOR = ["#3a3e46", "#5e6672", "#8c96a4", "#c4ccd8"]
pal = biped(look("palh", ARMOR, eyes=((255, 230, 140), "glow"), pattern="plate"), look("palb", ARMOR, pattern="plate"), look("pala", ARMOR, pattern="plate"), look("pall", ARMOR, pattern="plate"))
pal.box((10, 10, 2), (-5, 12, -4), look("palcape", ["#4a0a0a", "#7a1414", "#a42424"], pattern="cloth"))
head_box(pal, (2, 4, 6), (-1, 32, -3), look("palplume", GOLD[:4], pattern="cloth"))
pal.box((2, 4, 12), (6, 8, -1), look("palhammer", ["#2a1e14", "#4a3420", "#6a4a2e"]), "Arm(1)", (6, 22, 0))
pal.box((6, 6, 6), (4, 6, 9), look("palhead", GOLD, pattern="plate"), "Arm(1)", (6, 22, 0))
pal.layer(1, 0.3)
mob("paladin", "Paladino", "Paladin", pal, "kind: Hostile, health: 900, speed: 2.4, attack: 22, xp: 60, width: 0.8, height: 2.6, spawn: Never, scale: 1.35, coins: Some(2500), drops: [(item: \"ectoplasm\", min: 1, max: 3), (item: \"paladin_shield\", min: 1, max: 1, chance: 0.2), (item: \"paladin_hammer\", min: 1, max: 1, chance: 0.2)]",
    eggs=("#8c96a4", "#d4a02a"))

WY = ["#c8d0e0", "#dce4f0", "#eef2fa", "#ffffff"]
wyh = Model()
wyh.box((10, 8, 12), (-5, 0, -6), look("wyhead", WY, eyes=((60, 120, 220), "glow"), pattern="plate"), "Head", (0, 4, 0))
wyh.box((8, 4, 6), (-4, 1, 6), look("wysnout", WY, pattern="plate"), "Head", (0, 4, 0))
for k, x in enumerate((-6, 4)):
    wyh.box((2, 6, 2), (x, 8, -4), look("wyhorn", ["#6a6a74", "#9a9aa4", "#cacad4"]), "Head", (0, 4, 0))
mob("sky_wyvern", "Serpe do Céu", "Wyvern", wyh, "kind: Hostile, health: 1600, speed: 0, attack: 26, xp: 60, width: 1.0, height: 0.9, spawn: Never, flies: true, hover: 0.0, scale: 1.4, coins: Some(4000), drops: [(item: \"soul_of_flight\", min: 5, max: 10)]",
    eggs=("#eef2fa", "#4a8ae0"))
wyb = Model()
wyb.box((8, 8, 10), (-4, 0, -5), look("wybody", WY, pattern="plate"))
wyb.box((2, 2, 8), (-1, 8, -4), look("wyfin", ["#8ab8f4", "#b4d4fa", "#dceafe"], pattern="plate"))
for k, x in enumerate((-14, 4)):
    wyb.box((10, 1, 6), (x, 5, -3), look("wywing", ["#8ab8f4", "#b4d4fa", "#dceafe"], pattern="cloth"), f"Wing({k})", (x + (10 if k == 0 else 0), 5, 0))
mob("sky_wyvern_body", "Serpe do Céu", "Wyvern", wyb, "kind: Hostile, health: 1600, speed: 0, attack: 18, xp: 0, width: 0.9, height: 0.9, spawn: Never, flies: true, hover: 0.0, scale: 1.4, coins: Some(0)")
wyt = Model()
wyt.box((6, 6, 10), (-3, 1, -5), look("wybody", WY, pattern="plate"))
wyt.box((2, 8, 6), (-1, 4, -10), look("wyfin", ["#8ab8f4", "#b4d4fa", "#dceafe"], pattern="plate"))
mob("sky_wyvern_tail", "Serpe do Céu", "Wyvern", wyt, "kind: Hostile, health: 1600, speed: 0, attack: 14, xp: 0, width: 0.8, height: 0.8, spawn: Never, flies: true, hover: 0.0, scale: 1.4, coins: Some(0)")

OGRE = ["#4a5a2a", "#62763a", "#7c924c", "#9cb064"]
ogre = biped(look("ogreh", OGRE, eyes=((240, 200, 60), None), mouth=(230, 220, 190)), look("ogreb", ["#4a3420", "#64482c", "#7e5c3a"], pattern="cloth"), look("ogrea", OGRE), look("ogrel", OGRE), tall=12)
ogre.box((10, 6, 6), (-5, 14, -3), look("ogrebelly", OGRE))
ogre.box((3, 16, 3), (5, -2, -1), look("ogreclub", ["#2a1a0e", "#4a2e18", "#6e4626"], pattern="carved"), "Arm(1)", (6, 22, 0))
ogre.box((5, 6, 5), (4, -6, -2), look("ogreclub", ["#2a1a0e", "#4a2e18", "#6e4626"], pattern="carved"), "Arm(1)", (6, 22, 0))
mob("ogre", "Ogro", "Ogre", ogre, "kind: Hostile, health: 1800, speed: 1.6, attack: 30, xp: 80, width: 1.1, height: 3.6, spawn: Never, scale: 1.8, coins: Some(5000), motion: (heavy: Some(0.9)), drops: [(item: \"ogre_club\", min: 1, max: 1, chance: 0.25)]",
    eggs=("#62763a", "#4a3420"))

BET = ["#4a1408", "#7a2410", "#a8401a", "#d06a2a"]
betsy = quadruped((12, 8, 16), (8, 7, 9), 7, 3, look("betb", BET, pattern="plate", belly=["#a87830", "#c89848", "#e8c070"]), look("beth", BET, eyes=((255, 200, 60), "glow"), pattern="plate"),
                  look("betl", BET[:3] + ["#8a3010"]), snout=(6, 4, 4), wings=(12, 1, 10), wing_look=look("betw", ["#3a0a04", "#5a1408", "#7a2410"], pattern="cloth"), tail=(3, 3, 10))
mob("betsy", "Betsy", "Betsy", betsy, "kind: Hostile, health: 3500, speed: 0, attack: 34, xp: 200, width: 2.6, height: 2.2, spawn: Never, flies: true, hover: 8.0, scale: 3.0, lit: false, coins: Some(20000), drops: [(item: \"betsy_wing\", min: 1, max: 1, chance: 0.25), (item: \"betsy_wrath\", min: 1, max: 1, chance: 0.3)]",
    eggs=("#a8401a", "#e8c070"))


def dutchman():
    m = Model()
    hull = look("fdhull", ["#1e1410", "#3a261a", "#563826", "#6e4a32"], pattern="carved")
    sail = look("fdsail", ["#4a463e", "#6a6456", "#8a8270", "#a8a08c"], pattern="cloth")
    m.box((14, 6, 12), (-7, 0, 0), hull)
    m.box((14, 6, 12), (-7, 0, -12), hull)
    m.box((8, 4, 4), (-4, 2, 12), hull)
    m.box((10, 4, 6), (-5, 6, -12), hull)
    m.box((1, 22, 1), (0, 6, 2), look("fdmast", ["#2a1a0e", "#4a2e18", "#6e4626"]))
    m.box((1, 18, 1), (0, 6, -7), look("fdmast", ["#2a1a0e", "#4a2e18", "#6e4626"]))
    m.box((12, 10, 1), (-6, 14, 3), sail)
    m.box((10, 8, 1), (-5, 12, -6), sail)
    for x in (-8, 7):
        for z in (-6, 0, 6):
            m.box((1, 2, 2), (x, 2, z), look("fdcannon", ["#121214", "#26262e", "#3e3e4a"]))
    m.box((2, 2, 2), (-1, 28, 2), look("fdlamp", ["#2a8a4a", "#4ac06a", "#8af0a0"], glow=True))
    return m


mob("flying_dutchman", "Navio Voador", "Flying Dutchman", dutchman(), "kind: Hostile, health: 4000, speed: 0, attack: 0, xp: 200, width: 3.0, height: 3.4, spawn: Never, flies: true, hover: 10.0, scale: 2.6, coins: Some(30000), drops: [(item: \"lucky_coin\", min: 1, max: 1, chance: 0.2), (item: \"gold_ingot\", min: 4, max: 10)]",
    eggs=("#563826", "#4ac06a"))

BARK = ["#1e140c", "#3a2614", "#563a20", "#70502e"]
mw = Model()
mw.box((12, 24, 12), (-6, 0, -6), look("mwtrunk", BARK, eyes=((255, 120, 30), "glow"), pattern="rings"), "Head", (0, 18, 0))
for k, (x, y, z, s) in enumerate(((-15, 18, -2, (10, 3, 3)), (6, 15, -1, (11, 3, 3)), (-4, 24, -2, (3, 10, 3)))):
    mw.box(s, (x, y, z), look("mwbranch", BARK, pattern="rings"), f"Arm({k % 2})", (x + s[0] // 2, y, z))
for x in (-9, 5):
    mw.box((3, 3, 12), (x, 0, -6), look("mwroot", BARK, pattern="rings"))
mob("mourning_wood", "Árvore Lamentosa", "Mourning Wood", mw, "kind: Hostile, health: 3200, speed: 0.8, attack: 26, xp: 150, width: 1.6, height: 4.6, spawn: Never, scale: 2.4, lit: false, coins: Some(10000), motion: (heavy: Some(1.0)), drops: [(item: \"spooky_wood\", min: 20, max: 40), (item: \"spooky_staff\", min: 1, max: 1, chance: 0.25)]",
    eggs=("#563a20", "#ff7a1e"))

PINE = ["#0e2a16", "#16401e", "#1e5a2a", "#2a7438"]
ev = Model()
ev.box((4, 6, 4), (-2, 0, -2), look("evtrunk", BARK, pattern="rings"))
for k, (w, y) in enumerate(((14, 6), (11, 13), (8, 20), (5, 26))):
    ev.box((w, 7 if k < 2 else (6 if k == 2 else 5), w), (-w // 2, y, -w // 2), look(f"evlayer{k}", PINE, eyes=((255, 60, 40), "glow") if k == 1 else None, pattern="noise"), "Head" if k else None, (0, 12, 0))
ev.box((3, 3, 3), (-1, 31, -1), look("evstar", GOLD, glow=True), "Head", (0, 12, 0))
mob("everscream", "Pinheiro Furioso", "Everscream", ev, "kind: Hostile, health: 3600, speed: 0.6, attack: 28, xp: 160, width: 1.8, height: 4.6, spawn: Never, scale: 2.4, coins: Some(12000), motion: (heavy: Some(1.0)), drops: [(item: \"frost_shard\", min: 6, max: 12), (item: \"festive_wings\", min: 1, max: 1, chance: 0.15)]",
    eggs=("#1e5a2a", "#f2cc52"))

TANK = ["#4a0a0a", "#7a1414", "#a82424", "#d04040"]
nk = Model()
nk.box((12, 7, 16), (-6, 3, -8), look("nkhull", TANK, pattern="plate"))
for x in (-8, 6):
    nk.box((2, 5, 16), (x, 0, -8), look("nktread", ["#141416", "#26262c", "#3e3e48"], pattern="rings"))
nk.box((9, 5, 9), (-4, 10, -4), look("nkturret", TANK, eyes=((255, 255, 160), "glow"), pattern="plate"), "Head", (0, 12, 0))
nk.box((2, 2, 9), (-1, 12, 5), look("nkgun", ["#2a2a30", "#4a4a54", "#6e6e7c"]), "Head", (0, 12, 0))
nk.box((3, 3, 3), (-1, 15, -1), look("nktrim", ["#c8c8c8", "#e6e6e6", "#ffffff"], pattern="cloth"), "Head", (0, 12, 0))
mob("santa_tank", "Tanque Natalino", "Santa-NK1", nk, "kind: Hostile, health: 4200, speed: 1.4, attack: 30, xp: 180, width: 2.0, height: 1.8, spawn: Never, scale: 2.2, coins: Some(14000), motion: (heavy: Some(1.0)), drops: [(item: \"frost_shard\", min: 6, max: 12), (item: \"elf_gun\", min: 1, max: 1, chance: 0.25)]",
    eggs=("#a82424", "#e6e6e6"))

# --- Rare monsters ----------------------------------------------------------------------------
from dim_mobs_spec import wizard  # noqa: E402
tim = wizard(look("timrobe", ["#1a1a4a", "#2a2a72", "#3e3e9a", "#5a5ac0"], pattern="cloth"), look("timtrim", ["#101030", "#1a1a50", "#d4a02a"], pattern="cloth"),
             BONE, look("timhat", ["#141440", "#20206a", "#30309a"], pattern="cloth"), eyes=((120, 200, 255), "glow"))
mob("tim", "Tim", "Tim", tim, "kind: Hostile, health: 220, speed: 1.6, attack: 10, xp: 40, width: 0.6, height: 1.95, spawn: Never, ranged: true, range: 14.0, coins: Some(1500), drops: [(item: \"wizard_hat_tim\", min: 1, max: 1, chance: 0.5), (item: \"magic_mirror\", min: 1, max: 1, chance: 0.1)]",
    eggs=("#2a2a72", "#d4a02a"))

ZOMB = ["#3a5a34", "#4e7446", "#64905a", "#82ac74"]
doc = folk("docb", ZOMB, ["#6a5a3a", "#8a7650", "#a8946a"], ["#4a3a26", "#62503a", "#7a6850"], eyes=((220, 40, 40), None))
head_box(doc, (12, 1, 12), (-6, 32, -6), look("dochat", ["#3a2a1a", "#5a4228", "#7a5a38"], pattern="cloth"))
head_box(doc, (8, 3, 8), (-4, 33, -4), look("dochat", ["#3a2a1a", "#5a4228", "#7a5a38"], pattern="cloth"))
doc.box((1, 8, 1), (8, 4, 2), look("docwhip", ["#2a1a0e", "#4a2e18", "#6e4626"]), "Arm(1)", (6, 22, 0))
mob("doctor_bones", "Doutor Ossos", "Doctor Bones", doc, "kind: Hostile, health: 180, speed: 2.2, attack: 12, xp: 40, width: 0.6, height: 1.95, spawn: Never, coins: Some(1200), drops: [(item: \"explorer_hat\", min: 1, max: 1)]",
    eggs=("#64905a", "#5a4228"))

um = folk("umin", ZOMB, ["#5a4a32", "#76644a", "#928062"], ["#3a3a4a", "#50505e", "#686876"], eyes=((220, 220, 60), None))
head_box(um, (9, 4, 9), (-4, 31, -4), look("uminhelm", ["#5a4a10", "#8a7218", "#b89a2a", "#e0c040"], pattern="plate"))
head_box(um, (2, 2, 1), (-1, 33, 5), look("uminlamp", ["#ffe080", "#fff0b0", "#ffffff"], glow=True))
um.box((1, 12, 1), (8, 6, 2), look("uminpick", ["#2a1a0e", "#4a2e18", "#6e4626"]), "Arm(1)", (6, 22, 0))
um.box((7, 2, 2), (5, 16, 2), look("uminpickh", STEEL[1:]), "Arm(1)", (6, 22, 0))
mob("undead_miner", "Mineiro Morto-Vivo", "Undead Miner", um, "kind: Hostile, health: 120, speed: 2.0, attack: 9, xp: 24, width: 0.6, height: 1.95, spawn: Never, lit: false, coins: Some(800), drops: [(item: \"mining_helmet\", min: 1, max: 1, chance: 0.3), (item: \"bone\", min: 1, max: 3), (item: \"iron_ore\", min: 2, max: 6)]",
    eggs=("#64905a", "#e0c040"))

pinky = blob((5, 4, 5), look("pinky", ["#a83a7a", "#d05a9c", "#ee82bc", "#ffb8dc"], eyes=((40, 10, 30), None)))
mob("pinky", "Pinky", "Pinky", pinky, "kind: Hostile, health: 150, speed: 2.6, attack: 6, xp: 30, width: 0.35, height: 0.3, spawn: Never, hops: true, coins: Some(10000), motion: (style: Some(\"hopper\")), drops: [(item: \"gel\", min: 3, max: 6)]",
    eggs=("#ee82bc", "#a83a7a"))

# Things they drop, sell or pour.
item("ale", "Cerveja", "Ale", 16)
item("explorer_hat", "Chapéu de Explorador", "Archaeologist's Hat", 1)
item("wizard_hat_tim", "Chapéu do Tim", "Wizard Hat", 1)
item("mining_helmet", "Capacete de Minerador", "Mining Helmet", 1)
item("spooky_wood", "Madeira Assombrada", "Spooky Wood")
item("betsy_wing", "Asas da Betsy", "Betsy's Wings", 1)
item("old_man_key", "Chave Antiga da Masmorra", "Old Dungeon Key", 1)
ICONS.update({
    "ale": sprite(lambda a: mug(a)),
    "explorer_hat": sprite(lambda a: hat(a, ["#2a1e10", "#4a3420", "#6a4c30", "#8a6a44", "#a88a5c"], wide=True)),
    "wizard_hat_tim": sprite(lambda a: cone_hat(a, ["#0e0e30", "#1a1a50", "#2a2a7a", "#3e3eaa", "#5a5ad0"])),
    "mining_helmet": sprite(lambda a: helmet(a)),
    "spooky_wood": sprite(lambda a: plank(a, ["#120c10", "#22161e", "#34222e", "#48303e", "#5c4050"])),
    "betsy_wing": sprite(lambda a: wing(a, ["#2a0602", "#4a0e06", "#6e1a0a", "#963212", "#c8561e"])),
    "old_man_key": sprite(lambda a: key_icon(a, BONE_ICON)),
})




def fang(a, pal):
    a.poly([(8, 4), (16, 4), (20, 14), (18, 26), (14, 29), (13, 20), (9, 12)], shade_fill(pal, lambda xs, ys: 0.95 - (xs - 8) / 16 - (ys - 4) / 60), "#0a0604")
    a.px(11, 7, pal[-1])


def scale_icon(a, pal):
    for (cx, cy) in ((11, 11), (21, 11), (16, 20)):
        a.disc(cx, cy, 6.5, shade_fill(pal, lit(cx - 2, cy - 2, 6)), "#2a1e0a")
        a.px(cx - 2, cy - 2, pal[-1])


def shell_icon(a, pal):
    a.disc(16, 17, 11.0, shade_fill(pal, lit(12, 12, 12)), "#040802")
    for x0, y0, x1, y1 in ((16, 7, 16, 27), (7, 14, 25, 14), (8, 21, 24, 21)):
        lin(x0, y0, x1, y1, 1.0, shade_fill(pal, lambda xs, ys: 0.05), pal[0], a)
    a.px(11, 10, pal[-1])


def gel_icon(a, pal):
    a.poly([(6, 26), (8, 16), (13, 9), (19, 9), (24, 16), (26, 26)], shade_fill(pal, lit(13, 13, 10)), pal[0])
    a.px(12, 13, "#ffffff")
    a.px(13, 12, "#ffffff")


def fur_icon(a):
    pal = ["#5a606e", "#8a90a0", "#b8bed0", "#e0e4ee", "#ffffff"]
    for (cx, cy, r) in ((12, 14, 7), (20, 13, 6.5), (16, 20, 7.5)):
        a.disc(cx, cy, r, shade_fill(pal, lit(cx - 2, cy - 3, r)), "#3a3e48")
    for x, y in ((10, 12), (19, 11), (15, 19), (22, 18)):
        a.px(x, y, "#ffffff")


def medusa_icon(a):
    skin = ["#2a3a20", "#3e5a30", "#5a7a4a", "#7a9a62", "#a0c088"]
    a.disc(16, 17, 8.5, shade_fill(skin, lit(13, 14, 8)), "#0e1608")
    for k in range(6):
        ang = -math.pi * (0.15 + k * 0.14)
        x1, y1 = 16 + math.cos(ang) * 14, 15 + math.sin(ang) * 13
        lin(16 + math.cos(ang) * 6, 15 + math.sin(ang) * 6, x1, y1, 2.0, shade_fill(SCALE_ICON, lambda xs, ys: 0.6), "#0e1608", a)
    for x in (12, 20):
        a.px(x, 16, "#ffff70")
        a.px(x, 15, "#fff0a0")


SCALE_ICON = ["#14280e", "#2a4a2a", "#3e6a3a", "#56904e", "#7ab06a"]


def viking_icon(a):
    a.poly([(7, 22), (8, 12), (13, 8), (19, 8), (24, 12), (25, 22)], shade_fill(STEEL, lit(13, 11, 9)), "#16181c")
    a.poly([(5, 21), (27, 21), (27, 24), (5, 24)], shade_fill(STEEL, lambda xs, ys: 0.4), "#16181c")
    for s in (-1, 1):
        x0 = 16 + s * 8
        a.poly([(x0, 13), (x0 + s * 6, 6), (x0 + s * 7, 2), (x0 + s * 3, 10), (x0, 16)], shade_fill(["#6a5e40", "#a89a7a", "#d0c4a4", "#f0e8d0", "#ffffff"], lambda xs, ys: 0.7), "#2a2416")
    lin(16, 9, 16, 21, 1.6, shade_fill(STEEL, lambda xs, ys: 0.2), "#16181c", a)


def nv_icon(a):
    gran = ["#0a0a12", "#1a1a24", "#2a2a3a", "#3e3e54", "#565672"]
    a.poly([(7, 22), (8, 12), (13, 7), (19, 7), (24, 12), (25, 22)], shade_fill(gran, lit(13, 11, 9)), "#04040a")
    for x in (11, 21):
        a.disc(x, 17, 3.4, shade_fill(["#0a3a5a", "#1a6a9a", "#3aa0d8", "#8ad8ff", "#ffffff"], lit(x - 1, 16, 3)), "#04040a")


def doll_icon(a):
    cloth = ["#3a2814", "#5a4024", "#7a5a36", "#9a764a", "#c09a6a"]
    a.disc(16, 9, 5.5, shade_fill(cloth, lit(14, 7, 5)), "#1a1008")
    a.poly([(11, 14), (21, 14), (22, 24), (10, 24)], shade_fill(["#2a3a5a", "#3e5478", "#567098", "#7090b8", "#90b0d8"], lambda xs, ys: 0.8 - (xs - 10) / 16), "#0a1020")
    a.poly([(6, 15), (11, 15), (11, 18), (6, 18)], shade_fill(cloth, lambda xs, ys: 0.5), "#1a1008")
    a.poly([(21, 15), (26, 15), (26, 18), (21, 18)], shade_fill(cloth, lambda xs, ys: 0.5), "#1a1008")
    a.poly([(11, 24), (15, 24), (15, 30), (11, 30)], shade_fill(cloth, lambda xs, ys: 0.4), "#1a1008")
    a.poly([(17, 24), (21, 24), (21, 30), (17, 30)], shade_fill(cloth, lambda xs, ys: 0.4), "#1a1008")
    a.px(14, 8, "#0a0604")
    a.px(18, 8, "#0a0604")
    lin(20, 4, 26, 12, 1.0, shade_fill(STEEL, lambda xs, ys: 0.7), "#16181c", a)


# ==============================================================================================
# L19: the common monsters of each land
# ==============================================================================================

SAND = ["#8a6a3a", "#b08a4e", "#d0aa66", "#e8c884"]
H = "kind: Hostile, spawn: Never"


# --- Desert ---
al = Model()
al.box((12, 6, 10), (-6, 0, -5), look("alhead", SAND, eyes=((20, 10, 6), None), pattern="plate"), "Head", (0, 3, 0))
for k, x in enumerate((-5, 2)):
    al.box((3, 2, 8), (x, 1, 5), look("aljaw", ["#3a2a14", "#5a4220", "#7a5a2e"]), f"Arm({k})", (x + 1, 2, 5))
mob("antlion", "Formiga-Leão", "Antlion", al, f"{H}, health: 40, speed: 0, attack: 0, xp: 6, width: 0.9, height: 0.5, drops: [(item: \"antlion_mandible\", min: 1, max: 2, chance: 0.6)]", eggs=("#d0aa66", "#5a4220"))
ac = quadruped((8, 5, 14), (7, 5, 6), 5, 2, look("acb", SAND, pattern="plate"), look("ach", SAND, eyes=((20, 10, 6), None), pattern="plate"), look("acl", ["#5a4220", "#7a5a2e", "#9a7640"]), snout=(6, 2, 4), snout_look=look("acj", ["#3a2a14", "#5a4220", "#7a5a2e"]))
mob("antlion_charger", "Formiga-Leão Corredora", "Antlion Charger", ac, f"{H}, health: 55, speed: 4.6, attack: 7, xp: 8, width: 0.8, height: 0.8, drops: [(item: \"antlion_mandible\", min: 1, max: 2, chance: 0.6)]", eggs=("#b08a4e", "#3a2a14"))
SCALE_G = ["#2a4a2a", "#3e6a3a", "#56904e", "#7ab06a"]
lam = Model()
lam.box((8, 8, 8), (-4, 24, -4), look("lamh", SKIN, eyes=((200, 220, 60), "glow")), "Head", (0, 24, 0))
lam.box((8, 10, 4), (-4, 14, -2), look("lamb", ["#5a3a6a", "#7a5290", "#9a6ab4"], pattern="cloth"))
for k, x in enumerate((-7, 4)):
    lam.box((3, 10, 3), (x, 14, -1), look("lama", SKIN), f"Arm({k})", (x + 1, 23, 0))
lam.box((8, 6, 6), (-4, 8, -3), look("lamt", SCALE_G, pattern="plate"))
lam.box((6, 4, 10), (-3, 4, -8), look("lamt2", SCALE_G, pattern="plate"))
lam.box((4, 4, 12), (-2, 0, -18), look("lamt3", SCALE_G, pattern="plate"))
head_box(lam, (8, 10, 2), (-4, 18, -5), look("lamhair", ["#101014", "#1e1e24", "#30303a"], pattern="cloth"))
mob("lamia", "Lâmia", "Lamia", lam, f"{H}, health: 360, speed: 2.8, attack: 20, xp: 24, width: 0.7, height: 2.0, drops: [(item: \"desert_scale\", min: 1, max: 2, chance: 0.5)]", eggs=("#56904e", "#7a5290"))
tch = Model()
tch.box((10, 8, 10), (-5, 0, -5), look("tchh", SAND, eyes=((240, 60, 40), "glow"), pattern="plate"), "Head", (0, 4, 0))
tch.box((8, 3, 4), (-4, 1, 5), look("tchj", ["#3a2a14", "#5a4220", "#7a5a2e"]), "Head", (0, 4, 0))
tcb = Model()
tcb.box((8, 7, 8), (-4, 0, -4), look("tchb", SAND, pattern="rings"))
for x in (-6, 4):
    tcb.box((2, 2, 2), (x, 0, -1), look("tchleg", ["#3a2a14", "#5a4220", "#7a5a2e"]))
tct = Model()
tct.box((6, 5, 8), (-3, 1, -4), look("tchb", SAND, pattern="rings"))
tct.box((2, 2, 4), (-1, 2, -8), look("tchj", ["#3a2a14", "#5a4220", "#7a5a2e"]))
for n, m, st in (("tomb_crawler", tch, "health: 110, speed: 0, attack: 12, xp: 12, width: 0.8, height: 0.7, flies: true, scale: 1.2, drops: [(item: \"desert_scale\", min: 1, max: 2, chance: 0.3)]"),
                 ("tomb_crawler_body", tcb, "health: 110, speed: 0, attack: 10, xp: 0, width: 0.7, height: 0.6, flies: true, scale: 1.2, coins: Some(0)"),
                 ("tomb_crawler_tail", tct, "health: 110, speed: 0, attack: 8, xp: 0, width: 0.6, height: 0.5, flies: true, scale: 1.2, coins: Some(0)")):
    mob(n, "Rastejador de Tumbas", "Tomb Crawler", m, f"{H}, {st}", eggs=("#d0aa66", "#c43a24") if n == "tomb_crawler" else None)
ghoul = folk("ghoul", ["#3a3a2a", "#525238", "#6a6a48"], ["#2a2418", "#3e3624", "#544a32"], ["#2a2418", "#3e3624", "#544a32"], eyes=((140, 240, 60), "glow"))
ghoul.layer(1, 0.4)
mob("ghoul", "Carniçal", "Ghoul", ghoul, f"{H}, health: 340, speed: 2.6, attack: 22, xp: 24, width: 0.6, height: 1.95, ranged: true, range: 10.0, drops: [(item: \"desert_scale\", min: 1, max: 2, chance: 0.5)]", eggs=("#525238", "#8af03c"))


# --- Bats, one shape in each land's colours ---
def bat(key, pal, wing, eyes):
    m = Model()
    m.box((4, 4, 4), (-2, 4, -2), look(key + "b", pal, eyes=eyes))
    for k, x in enumerate((-9, 2)):
        m.box((7, 1, 5), (x, 7, -2), look(key + "w", wing, pattern="cloth"), f"Wing({k})", (x + (7 if k == 0 else 0), 7, 0))
    for x in (-2, 1):
        m.box((1, 2, 1), (x, 8, 0), look(key + "b", pal))
    return m


for key, dn, en, pal, wng, eyes, st, egg_ in (
        ("cave_bat", "Morcego das Cavernas", "Cave Bat", ["#2a201a", "#3e3026", "#544234"], ["#1a1410", "#2a201a", "#3e3026"], ((240, 60, 40), None), "health: 14, attack: 4, xp: 3", ("#3e3026", "#c43a24")),
        ("jungle_bat", "Morcego da Selva", "Jungle Bat", ["#1e3a14", "#2e5420", "#40702e"], ["#142a0e", "#1e3a14", "#2e5420"], ((240, 220, 60), None), "health: 22, attack: 6, xp: 4", ("#2e5420", "#f0dc3c")),
        ("ice_bat", "Morcego de Gelo", "Ice Bat", ["#4a6a8a", "#6a8eb0", "#9abcd8"], ["#2a4a6a", "#4a6a8a", "#6a8eb0"], ((40, 60, 120), None), "health: 18, attack: 5, xp: 4", ("#6a8eb0", "#e6f0fa")),
        ("lava_bat", "Morcego de Lava", "Lava Bat", ["#5a1408", "#8a2a0a", "#c84a14"], ["#3a0c04", "#5a1408", "#8a2a0a"], ((255, 220, 60), "glow"), "health: 40, attack: 12, xp: 8, lit: true", ("#8a2a0a", "#ffdc3c")),
        ("giant_bat", "Morcego Gigante", "Giant Bat", ["#3a1a3a", "#542a54", "#703c70"], ["#2a102a", "#3a1a3a", "#542a54"], ((240, 60, 40), "glow"), "health: 120, attack: 16, xp: 14, scale: 1.6", ("#542a54", "#c43a24"))):
    mob(key, dn, en, bat(key, pal, wng, eyes), f"{H}, {st}, speed: 4.2, width: 0.6, height: 0.6, flies: true, hover: 1.2", eggs=egg_)

# --- Jungle ---
LEAF = ["#1a3a10", "#2a5418", "#3e7424", "#5a9634"]


def flytrap(key, head_pal, scale_):
    m = Model()
    m.box((8, 6, 8), (-4, 14, -4), look(key + "h", head_pal, pattern="noise", mouth=(240, 240, 220)), "Head", (0, 12, 0))
    m.box((8, 3, 8), (-4, 11, -4), look(key + "j", head_pal[:3] + ["#f0e0b0"], pattern="noise"), "Head", (0, 12, 0))
    m.box((2, 12, 2), (-1, 0, -1), look(key + "s", LEAF, pattern="rings"))
    for k, x in enumerate((-6, 2)):
        m.box((4, 1, 4), (x, 2, -2), look(key + "l", LEAF, pattern="cloth"))
    return m


mob("man_eater", "Devoradora", "Man Eater", flytrap("maneat", ["#5a0a1a", "#8a1428", "#b82a40", "#e05068"], 1.4), f"{H}, health: 130, speed: 0, attack: 14, xp: 12, width: 0.8, height: 1.6, scale: 1.4, drops: [(item: \"vine\", min: 1, max: 3, chance: 0.6), (item: \"jungle_spore\", min: 1, max: 2, chance: 0.5)]", eggs=("#8a1428", "#3e7424"))
mob("snatcher", "Arrebatadora", "Snatcher", flytrap("snatch", ["#2a4a0a", "#3e6a14", "#5a8e22", "#8ab84a"], 1.0), f"{H}, health: 60, speed: 0, attack: 10, xp: 6, width: 0.6, height: 1.2, drops: [(item: \"vine\", min: 1, max: 2, chance: 0.5)]", eggs=("#3e6a14", "#b82a40"))
derp = blob((12, 8, 12), look("derp", ["#0a2a6a", "#14408a", "#2a5ab0", "#4a7ad0"], eyes=((240, 240, 240), None), pattern="plate"))
for x in (-7, 5):
    derp.box((2, 6, 2), (x, -2, 0), look("derpl", ["#0a1a3a", "#14284a", "#203a62"]))
mob("derpling", "Derpling", "Derpling", derp, f"{H}, health: 300, speed: 3.6, attack: 22, xp: 20, width: 0.9, height: 0.8, hops: true, scale: 1.3, motion: (style: Some(\"hopper\")), drops: [(item: \"beetle_husk\", min: 1, max: 1, chance: 0.2)]", eggs=("#14408a", "#f0f0f0"))
SHELL = ["#2a3a14", "#3e5420", "#56702e", "#708e40"]
tort = quadruped((14, 8, 16), (6, 5, 6), 4, 4, look("tortshell", SHELL, pattern="plate"), look("torth", ["#4a5a2a", "#62763a", "#7c924c"], eyes=((30, 20, 10), None)), look("tortl", ["#4a5a2a", "#62763a", "#7c924c"]))
tort.box((12, 2, 14), (-6, 12, -7), look("tortshell", SHELL, pattern="plate"))
mob("giant_tortoise", "Tartaruga Gigante", "Giant Tortoise", tort, f"{H}, health: 500, speed: 1.4, attack: 30, xp: 30, width: 1.2, height: 1.0, scale: 1.6, motion: (heavy: Some(0.9)), drops: [(item: \"turtle_shell\", min: 1, max: 1, chance: 0.1)]", eggs=("#56702e", "#7c924c"))

# --- The Prism (hallow) ---
gas = Model()
gas.box((8, 8, 8), (-4, 2, -4), look("gasshell", ["#a83a8a", "#d05aac", "#ee82c8", "#ffc0e8"], pattern="rings"))
gas.box((4, 4, 10), (-2, 0, -2), look("gasbody", ["#c8a07a", "#e0bc94", "#f4d8b4"], eyes=((40, 20, 60), None)), "Head", (0, 2, 4))
for x in (-2, 1):
    gas.box((1, 3, 1), (x, 4, 6), look("gasbody", ["#c8a07a", "#e0bc94", "#f4d8b4"]), "Head", (0, 2, 4))
mob("gastropod", "Gastrópode", "Gastropod", gas, f"{H}, health: 220, speed: 2.0, attack: 18, xp: 18, width: 0.7, height: 0.8, flies: true, hover: 3.0, ranged: true, range: 14.0, scale: 1.3, drops: [(item: \"pink_gel\", min: 1, max: 3)]", eggs=("#ee82c8", "#f4d8b4"))
swd = Model()
swd.box((2, 16, 1), (-1, 6, 0), look("swdblade", ["#5a6ab0", "#8a9ad8", "#c0ccf4", "#ffffff"], pattern="plate"))
swd.box((8, 1, 2), (-4, 5, -1), look("swdguard", GOLD[1:]))
swd.box((2, 5, 2), (-1, 0, -1), look("swdgrip", ["#2a1a3a", "#3e2a54", "#5a3e74"]))
mob("possessed_sword", "Espada Possuída", "Enchanted Sword", swd, f"{H}, health: 180, speed: 0, attack: 22, xp: 18, width: 0.5, height: 1.3, flies: true, hover: 1.0, lit: true, drops: [(item: \"soul_of_light\", min: 1, max: 2, chance: 0.4)]", eggs=("#8a9ad8", "#d4a02a"))
chaos = folk("chaos", ["#3a1a5a", "#5a2a8a", "#7a42b4", "#a070e0"], ["#2a0e4a", "#3e1a6a", "#56288e"], ["#2a0e4a", "#3e1a6a", "#56288e"], eyes=((255, 120, 240), "glow"))
chaos.layer(0, 0.4)
mob("chaos_elemental", "Elemental do Caos", "Chaos Elemental", chaos, f"{H}, health: 260, speed: 2.4, attack: 20, xp: 22, width: 0.6, height: 1.95, lit: true, drops: [(item: \"soul_of_light\", min: 1, max: 2, chance: 0.4), (item: \"rod_of_discord\", min: 1, max: 1, chance: 0.02)]", eggs=("#5a2a8a", "#ff78f0"))

# --- Hardmode nights ---
parm = biped(look("parmh", ARMOR, eyes=((255, 60, 40), "glow"), pattern="plate"), look("parmb", ARMOR, pattern="plate"), look("parma", ARMOR, pattern="plate"), look("parml", ARMOR, pattern="plate"))
parm.box((10, 8, 2), (-5, 14, -4), look("parmcape", ["#1a0a2a", "#2a1440", "#3e2058"], pattern="cloth"))
mob("possessed_armor", "Armadura Possuída", "Possessed Armor", parm, f"{H}, health: 320, speed: 2.2, attack: 24, xp: 22, width: 0.6, height: 1.95, motion: (heavy: Some(0.7)), drops: [(item: \"iron_ingot\", min: 1, max: 3, chance: 0.5)]", eggs=("#8c96a4", "#ff3c28"))
wraith = Model()
wraith.box((8, 8, 8), (-4, 16, -4), look("wrh", ["#0a0a10", "#14141c", "#22222e"], eyes=((255, 230, 120), "glow"), pattern="cloth"), "Head", (0, 16, 0))
wraith.box((10, 16, 6), (-5, 0, -3), look("wrb", ["#0a0a10", "#14141c", "#22222e", "#323244"], pattern="cloth"))
for k, x in enumerate((-8, 5)):
    wraith.box((3, 12, 3), (x, 4, -1), look("wra", ["#0a0a10", "#14141c", "#22222e"], pattern="cloth"), f"ArmForward({k})", (x + 1, 15, 0))
mob("wraith", "Espectro", "Wraith", wraith, f"{H}, health: 280, speed: 0, attack: 22, xp: 22, width: 0.7, height: 1.8, flies: true, hover: 0.5, drops: [(item: \"ectoplasm\", min: 1, max: 1, chance: 0.2)]", eggs=("#14141c", "#ffe678"))

# --- Marble and granite ---
MARBLE = ["#9a9890", "#c4c2b8", "#e2e0d8", "#f8f6f0"]
medusa = Model()
medusa.box((8, 8, 8), (-4, 24, -4), look("medh", ["#5a7a4a", "#7a9a62", "#9aba7e"], eyes=((255, 255, 120), "glow")), "Head", (0, 24, 0))
for k, (x, z) in enumerate(((-6, -2), (4, -2), (-2, -6), (-2, 4), (-5, 3), (3, 3))):
    medusa.box((2, 6, 2), (x, 28, z), look("medsnake", SCALE_G, pattern="plate"), "Head", (0, 24, 0))
medusa.box((8, 12, 4), (-4, 12, -2), look("medb", MARBLE, pattern="cloth"))
for k, x in enumerate((-7, 4)):
    medusa.box((3, 11, 3), (x, 13, -1), look("meda", ["#5a7a4a", "#7a9a62", "#9aba7e"]), f"Arm({k})", (x + 1, 22, 0))
medusa.box((8, 8, 6), (-4, 4, -3), look("medt", SCALE_G, pattern="plate"))
medusa.box((6, 4, 12), (-3, 0, -12), look("medt2", SCALE_G, pattern="plate"))
mob("medusa", "Medusa", "Medusa", medusa, f"{H}, health: 420, speed: 1.8, attack: 24, xp: 30, width: 0.7, height: 2.0, drops: [(item: \"medusa_head\", min: 1, max: 1, chance: 0.04)]", eggs=("#7a9a62", "#ffff78"))
GRAN = ["#1a1a24", "#2a2a3a", "#3e3e54", "#565672"]
gran = Model()
gran.box((10, 10, 10), (-5, 4, -5), look("grancore", GRAN, eyes=((80, 200, 255), "glow"), pattern="carved"), "Head", (0, 9, 0))
for k, (x, y, z) in enumerate(((-9, 6, -2), (7, 6, -2), (-3, 15, -3), (-3, 0, -3))):
    gran.box((4, 4, 4) if k < 2 else (6, 3, 6), (x, y, z), look("granrock", GRAN, pattern="carved"), f"Arm({k % 2})" if k < 2 else None, (x + 2, y + 2, z + 2))
mob("granite_elemental", "Elemental de Granito", "Granite Elemental", gran, f"{H}, health: 160, speed: 0, attack: 16, xp: 16, width: 0.8, height: 1.0, flies: true, hover: 1.5, lit: true, drops: [(item: \"granite\", min: 2, max: 6), (item: \"night_vision_helmet\", min: 1, max: 1, chance: 0.05)]", eggs=("#2a2a3a", "#50c8ff"))

# --- Snow underground ---
flinx = blob((8, 7, 8), look("flinx", ["#9aa0b0", "#c4cad8", "#e6eaf2", "#ffffff"], eyes=((20, 30, 50), None), pattern="wool"))
flinx.box((2, 2, 2), (-1, 5, 4), look("flinxn", ["#a83a3a", "#d05a5a", "#ee8080"]))
flinx.layer(0, 0.6)
mob("flinx", "Flinx", "Flinx", flinx, f"{H}, health: 40, speed: 3.2, attack: 8, xp: 5, width: 0.6, height: 0.55, hops: true, motion: (style: Some(\"hopper\")), drops: [(item: \"flinx_fur\", min: 1, max: 2, chance: 0.8)]", eggs=("#e6eaf2", "#d05a5a"))
vik = folk("viking", BONE, ["#3a4a5a", "#526476", "#6e8296"], ["#3a2a1a", "#52402a", "#6a563a"], arms=BONE, eyes=((80, 160, 255), "glow"))
head_box(vik, (9, 3, 9), (-4, 30, -4), look("vikhelm", STEEL[1:], pattern="plate"))
for x in (-6, 4):
    head_box(vik, (2, 5, 2), (x, 31, -1), look("vikhorn", ["#a89a7a", "#d0c4a4", "#f0e8d0"]))
vik.box((2, 10, 8), (-7, 10, -4), look("vikshield", ["#5a2a14", "#7a3a1c", "#a85226"], pattern="rings"), "Arm(0)", (-6, 22, 0))
mob("undead_viking", "Viking Morto-Vivo", "Undead Viking", vik, f"{H}, health: 90, speed: 2.2, attack: 12, xp: 10, width: 0.6, height: 1.95, drops: [(item: \"bone\", min: 1, max: 3), (item: \"viking_helmet\", min: 1, max: 1, chance: 0.05)]", eggs=("#d8d2bc", "#526476"))

# --- The Underworld ---
DEMON = ["#3a0a2a", "#5a1440", "#7a2058", "#a0306e"]


def demon(key, eyes, carry=None):
    m = biped(look(key + "h", DEMON, eyes=eyes), look(key + "b", DEMON, pattern="plate"), look(key + "a", DEMON), look(key + "l", DEMON), wings=(10, 10, 1), wing_look=look(key + "w", ["#1a0410", "#2a081a", "#3e1028"], pattern="cloth"))
    for x in (-5, 3):
        m.box((2, 4, 2), (x, 32, -1), look(key + "horn", ["#2a2620", "#4a4436", "#6a6250"]), "Head", (0, 24, 0))
    if carry:
        m.box((4, 7, 3), (6, 10, 2), carry, "Arm(1)", (6, 22, 0))
    return m


mob("demon", "Demônio", "Demon", demon("demon", ((255, 200, 60), "glow")), f"{H}, health: 120, speed: 0, attack: 16, xp: 14, width: 0.6, height: 1.95, flies: true, hover: 3.0, lit: false, drops: [(item: \"demon_scythe\", min: 1, max: 1, chance: 0.03)]", eggs=("#5a1440", "#ffc83c"))
mob("voodoo_demon", "Demônio Vodu", "Voodoo Demon", demon("voodoo", ((120, 255, 120), "glow"), look("vdoll", ["#6a5030", "#8a6a44", "#aa8a5c"], eyes=((20, 10, 6), None), pattern="cloth")), f"{H}, health: 140, speed: 0, attack: 16, xp: 16, width: 0.6, height: 1.95, flies: true, hover: 3.0, drops: [(item: \"guide_voodoo_doll\", min: 1, max: 1)]", eggs=("#5a1440", "#78ff78"))

# --- Hardmode dungeon ---
ab = folk("armb", BONE, ["#3a3e46", "#5e6672", "#8c96a4"], ["#3a3e46", "#5e6672", "#8c96a4"], arms=BONE, eyes=((255, 120, 40), "glow"))
head_box(ab, (9, 4, 9), (-4, 30, -4), look("armbhelm", ARMOR, pattern="plate"))
ab.box((2, 12, 2), (7, 4, 2), look("armbsword", STEEL), "Arm(1)", (6, 22, 0))
mob("armored_bones", "Ossos Blindados", "Armored Bones", ab, f"{H}, health: 380, speed: 2.4, attack: 26, xp: 24, width: 0.6, height: 1.95, drops: [(item: \"bone\", min: 1, max: 3), (item: \"ectoplasm\", min: 1, max: 1, chance: 0.15)]", eggs=("#d8d2bc", "#5e6672"))
rc = wizard(look("ragrobe", ["#1a0a1a", "#2a1428", "#3e203a", "#562c50"], pattern="cloth"), look("ragtrim", ["#120612", "#1e0c1e", "#8a2a8a"], pattern="cloth"), BONE, look("raghat", ["#120612", "#1e0c1e", "#2a142a"], pattern="cloth"), eyes=((255, 60, 255), "glow"), staff=look("ragstaff", ["#1a0a14", "#2a1420", "#3e2030"]))
mob("ragged_caster", "Conjurador Esfarrapado", "Ragged Caster", rc, f"{H}, health: 300, speed: 1.4, attack: 14, xp: 24, width: 0.6, height: 1.95, drops: [(item: \"ectoplasm\", min: 1, max: 1, chance: 0.2), (item: \"spectral_bone\", min: 1, max: 2, chance: 0.2)]", eggs=("#2a1428", "#ff3cff"))
sn = folk("sniper", BONE, ["#2a3a2a", "#3e543e", "#567056"], ["#2a3a2a", "#3e543e", "#567056"], arms=BONE, eyes=((255, 40, 40), "glow"))
head_box(sn, (9, 2, 9), (-4, 31, -4), look("snhat", ["#1a2a1a", "#2a3e2a", "#3e563e"], pattern="cloth"))
sn.box((2, 2, 14), (6, 14, -2), look("snrifle", ["#1a1a1e", "#2e2e36", "#4a4a56"]), "ArmForward(1)", (6, 22, 0))
mob("skeleton_sniper", "Esqueleto Atirador", "Skeleton Sniper", sn, f"{H}, health: 320, speed: 1.6, attack: 0, xp: 26, width: 0.6, height: 1.95, drops: [(item: \"bone\", min: 1, max: 3), (item: \"sniper_rifle\", min: 1, max: 1, chance: 0.04), (item: \"rifle_scope\", min: 1, max: 1, chance: 0.04)]", eggs=("#d8d2bc", "#3e543e"))

# --- Glowing mushroom caves and spider caves ---
MUSH = ["#0e1a4a", "#16286a", "#2a4a9a", "#4a7ad0", "#8ab8ff"]
ano = quadruped((10, 4, 8), (6, 3, 3), 3, 2, look("anob", ["#4a2a1a", "#6a3e26", "#8a5434"], pattern="plate"), look("anoh", ["#4a2a1a", "#6a3e26", "#8a5434"], eyes=((40, 20, 10), None)), look("anol", ["#3a1e12", "#52301e", "#6a4028"]))
ano.box((10, 5, 8), (-5, 7, -4), look("anocap", MUSH, pattern="noise"))
for k, x in enumerate((-8, 5)):
    ano.box((3, 3, 4), (x, 3, 3), look("anoclaw", ["#4a2a1a", "#6a3e26", "#8a5434"], pattern="plate"), f"Arm({k})", (x + 1, 4, 3))
mob("anomura_fungus", "Anomura Fúngica", "Anomura Fungus", ano, f"{H}, health: 90, speed: 2.0, attack: 12, xp: 10, width: 0.8, height: 0.8, lit: true, drops: [(item: \"glow_mushroom\", min: 1, max: 3)]", eggs=("#6a3e26", "#4a7ad0"))
bulb = Model()
bulb.box((8, 8, 8), (-4, 4, -4), look("bulbcap", MUSH, eyes=((255, 255, 200), "glow"), pattern="noise"), "Head", (0, 4, 0))
bulb.box((4, 4, 4), (-2, 0, -2), look("bulbst", ["#8a9ab0", "#b0c0d4", "#d8e4f0"]))
mob("fungi_bulb", "Bulbo Fúngico", "Fungi Bulb", bulb, f"{H}, health: 70, speed: 0, attack: 0, xp: 8, width: 0.6, height: 0.8, lit: true, drops: [(item: \"glow_mushroom\", min: 1, max: 3)]", eggs=("#2a4a9a", "#ffffc8"))
for key, dn, en, pal, st, egg_ in (
        ("wall_creeper", "Rastejador de Parede", "Wall Creeper", ["#1a1410", "#2e241c", "#463828", "#5e4c36"], "health: 90, attack: 12, xp: 10, scale: 1.2", ("#2e241c", "#c43a24")),
        ("black_recluse", "Viúva Reclusa", "Black Recluse", ["#08080a", "#141418", "#22222a", "#34343e"], "health: 320, attack: 26, xp: 24, scale: 1.4", ("#141418", "#c42424"))):
    sp = quadruped((10, 6, 12), (8, 6, 6), 3, 2, look(key + "b", pal, pattern="noise"), look(key + "h", pal, eyes=((220, 30, 30), "glow")), look(key + "l", pal[:3]))
    for side in (-1, 1):
        for z in (-4, 0, 4):
            x = -13 if side < 0 else 5
            sp.box((8, 1, 1), (x, 4, z), look(key + "l", pal[:3]), f"SpiderLeg({(z + 4) // 4 + (0 if side < 0 else 3)})", (x + (8 if side < 0 else 0), 4, z))
    mob(key, dn, en, sp, f"{H}, {st}, speed: 3.0, width: 1.2, height: 0.7, climbs: true, motion: (style: Some(\"spider\")), drops: [(item: \"string\", min: 1, max: 3), (item: \"spider_fang\", min: 1, max: 2, chance: 0.4)]", eggs=egg_)

# Their drops and the doll.
item("antlion_mandible", "Mandíbula de Formiga-Leão", "Antlion Mandible")
item("desert_scale", "Escama do Deserto", "Desert Scale")
item("turtle_shell", "Casco de Tartaruga", "Turtle Shell")
item("beetle_husk", "Carapaça de Besouro", "Beetle Husk")
item("pink_gel", "Gel Rosa", "Pink Gel")
item("flinx_fur", "Pelo de Flinx", "Flinx Fur")
item("spider_fang", "Presa de Aranha", "Spider Fang")
item("medusa_head", "Cabeça da Medusa", "Medusa Head", 1)
item("viking_helmet", "Elmo Viking", "Viking Helmet", 1)
item("night_vision_helmet", "Elmo de Visão Noturna", "Night Vision Helmet", 1)
item("guide_voodoo_doll", "Boneco Vodu do Guia", "Guide Voodoo Doll", 1)
ICONS.update({
    "antlion_mandible": sprite(lambda a: fang(a, ["#2a1e0e", "#4a3618", "#7a5a2e", "#a8824a", "#d0aa66"])),
    "desert_scale": sprite(lambda a: scale_icon(a, SAND[:3] + ["#f4dca0", "#fff4d0"])),
    "turtle_shell": sprite(lambda a: shell_icon(a, ["#1a2a0a", "#2a3e14", "#3e5420", "#56702e", "#7a944a"])),
    "beetle_husk": sprite(lambda a: shell_icon(a, ["#0a0a2a", "#14144a", "#22227a", "#3a3aaa", "#6a6ad8"])),
    "pink_gel": sprite(lambda a: gel_icon(a, ["#6a1a4a", "#a83a7a", "#d05aac", "#ee82c8", "#ffc8ec"])),
    "flinx_fur": sprite(lambda a: fur_icon(a)),
    "spider_fang": sprite(lambda a: fang(a, ["#08080a", "#1a1a20", "#2e2e38", "#4a4a5a", "#c8c8d8"])),
    "medusa_head": sprite(lambda a: medusa_icon(a)),
    "viking_helmet": sprite(lambda a: viking_icon(a)),
    "night_vision_helmet": sprite(lambda a: nv_icon(a)),
    "guide_voodoo_doll": sprite(lambda a: doll_icon(a)),
})




def drill_icon(a, pal):
    a.poly([(4, 22), (14, 12), (20, 18), (10, 28)], shade_fill(pal, lambda xs, ys: 0.9 - (xs - 4) / 20), "#04040a")
    a.poly([(14, 12), (20, 6), (29, 3), (26, 12), (20, 18)], shade_fill(STEEL, lambda xs, ys: 0.95 - (xs - 14) / 18), "#16181c")
    for k in range(4):
        a.px(19 + k * 2, 11 - k * 2, "#2a2e36")
    a.poly([(6, 26), (10, 30), (8, 31), (3, 27)], shade_fill(GRIP, lambda xs, ys: 0.6), "#08080a")
    a.px(9, 18, pal[-1])


def saw_icon(a, pal):
    a.poly([(3, 24), (12, 17), (17, 22), (8, 29)], shade_fill(pal, lambda xs, ys: 0.9 - (xs - 3) / 16), "#04040a")
    a.poly([(12, 17), (26, 5), (30, 9), (17, 22)], shade_fill(STEEL, lambda xs, ys: 0.95 - (xs - 12) / 20), "#16181c")
    for k in range(6):
        x, y = 14 + k * 3, 15 - k * 2.5
        a.px(int(x), int(y), "#d8dee6")
    a.px(6, 24, pal[-1])


def multi_icon(a, head_a, head_b):
    lin(6, 27, 22, 9, 2.4, shade_fill(WOOD, lambda xs, ys: 0.8 - (xs - 6) / 30), "#140c06", a)
    a.poly([(13, 5), (20, 3), (28, 7), (30, 12), (24, 9), (18, 8)], shade_fill(head_a, lit(20, 5, 6)), "#04040a")
    a.poly([(21, 11), (28, 13), (29, 20), (23, 17)], shade_fill(head_b, lit(25, 14, 5)), "#04040a")


def hamaxe_icon(a, pal):
    lin(6, 27, 21, 10, 2.4, shade_fill(WOOD, lambda xs, ys: 0.8 - (xs - 6) / 30), "#140c06", a)
    a.poly([(14, 4), (22, 4), (22, 11), (14, 11)], shade_fill(pal, lit(17, 6, 5)), "#04040a")
    a.poly([(22, 5), (29, 3), (30, 13), (22, 11)], shade_fill(pal, lit(25, 6, 6)), "#04040a")
    a.px(16, 6, pal[-1])


def rod_icon(a):
    lin(6, 27, 23, 8, 2.0, shade_fill(["#2a0a3a", "#4a145a", "#6a2a8a", "#8a4ab0", "#c08ae0"], lambda xs, ys: 0.8 - (xs - 6) / 30), "#0a0410", a)
    a.disc(24, 7, 4.5, shade_fill(["#3a0a5a", "#7a2aa8", "#c060f0", "#f0b0ff", "#ffffff"], lit(23, 6, 4)), "#14041e")
    for x, y in ((18, 4), (29, 11), (27, 2)):
        a.px(x, y, "#f0b0ff")


def bucket_icon(a, liquid):
    a.poly([(7, 10), (25, 10), (23, 27), (9, 27)], shade_fill(STEEL, lambda xs, ys: 0.95 - (xs - 7) / 20), "#16181c")
    a.poly([(8, 10), (24, 10), (23, 14), (9, 14)], shade_fill(liquid, lambda xs, ys: 0.7 - (xs - 8) / 30), liquid[0])
    a.poly([(6, 6), (26, 6), (26, 7), (6, 7)], shade_fill(["#2a0a3a", "#7a2aa8", "#c060f0"], lambda xs, ys: 0.8), "#14041e")
    lin(6, 7, 7, 10, 1.0, shade_fill(STEEL, lambda xs, ys: 0.4), "#16181c", a)
    lin(26, 7, 25, 10, 1.0, shade_fill(STEEL, lambda xs, ys: 0.4), "#16181c", a)
    a.px(10, 12, liquid[-1])


def sponge_icon(a, pal):
    a.poly([(5, 9), (27, 7), (28, 24), (6, 26)], shade_fill(pal, lambda xs, ys: 0.9 - (xs - 5) / 40 - (ys - 7) / 40), pal[0])
    for x, y in ((9, 12), (15, 10), (22, 12), (11, 19), (19, 17), (24, 21), (8, 23), (16, 22)):
        a.disc(x, y, 1.3, pal[1], pal[1])


def belt_icon(a):
    a.poly([(3, 12), (29, 12), (29, 18), (3, 18)], shade_fill(["#2a1a0e", "#4a2e18", "#6e4626", "#946236", "#b88050"], lambda xs, ys: 0.7), "#140c06")
    a.poly([(13, 11), (19, 11), (19, 19), (13, 19)], shade_fill(GOLD, lit(15, 13, 3)), "#3a2604")
    lin(6, 18, 8, 28, 1.6, shade_fill(STEEL, lambda xs, ys: 0.7), "#16181c", a)
    lin(24, 18, 26, 27, 2.4, shade_fill(WOOD, lambda xs, ys: 0.6), "#140c06", a)


def box_icon(a):
    a.poly([(4, 13), (28, 13), (28, 27), (4, 27)], shade_fill(RED, lambda xs, ys: 0.85 - (xs - 4) / 40), "#2a0404")
    a.poly([(4, 13), (28, 13), (28, 16), (4, 16)], shade_fill(RED, lambda xs, ys: 0.3), "#2a0404")
    a.poly([(11, 8), (21, 8), (21, 13), (19, 13), (19, 10), (13, 10), (13, 13), (11, 13)], shade_fill(STEEL, lambda xs, ys: 0.6), "#16181c")


def grip_icon(a):
    lin(5, 28, 25, 6, 1.8, shade_fill(STEEL, lambda xs, ys: 0.9 - (xs - 5) / 30), "#16181c", a)
    a.poly([(24, 3), (29, 5), (27, 9), (23, 7)], shade_fill(STEEL, lit(26, 5, 3)), "#16181c")
    a.poly([(3, 25), (8, 25), (8, 31), (3, 31)], shade_fill(RED, lambda xs, ys: 0.7), "#2a0404")


def shield_icon(a):
    a.poly([(6, 5), (26, 5), (26, 16), (16, 29), (6, 16)], shade_fill(STEEL, lit(12, 9, 12)), "#16181c")
    a.poly([(9, 8), (23, 8), (23, 15), (16, 25), (9, 15)], shade_fill(RED, lambda xs, ys: 0.8 - (ys - 8) / 30), "#2a0404")
    a.poly([(15, 9), (17, 9), (17, 22), (15, 22)], shade_fill(GOLD, lambda xs, ys: 0.8), "#3a2604")
    a.poly([(11, 13), (21, 13), (21, 15), (11, 15)], shade_fill(GOLD, lambda xs, ys: 0.8), "#3a2604")


def gun_icon(a, kind, pal):
    if kind == "pipe":
        lin(3, 22, 29, 12, 2.6, shade_fill(pal, lambda xs, ys: 0.9 - (xs - 3) / 30), pal[0], a)
        for x in (10, 18):
            a.px(x, 19 - (x - 10) // 3, pal[0])
        return
    if kind == "tube":
        a.poly([(3, 11), (29, 11), (29, 18), (3, 18)], shade_fill(pal, lambda xs, ys: 0.9 - (ys - 11) / 10), pal[0])
        a.poly([(12, 18), (17, 18), (15, 26), (11, 26)], shade_fill(GRIP, lambda xs, ys: 0.6), "#08080a")
        a.poly([(24, 8), (27, 8), (27, 11), (24, 11)], shade_fill(STEEL, lambda xs, ys: 0.6), "#16181c")
        return
    long_ = kind in ("sniper", "shark")
    x1 = 30 if long_ else 27
    a.poly([(4, 12), (x1, 12), (x1, 16), (4, 17)], shade_fill(pal, lambda xs, ys: 0.95 - (ys - 12) / 8 - (xs - 4) / 80), "#04040a")
    a.poly([(4, 13), (10, 13), (8, 22), (3, 21)], shade_fill(pal, lambda xs, ys: 0.5), "#04040a")
    a.poly([(13, 16), (17, 16), (16, 24), (12, 24)], shade_fill(GRIP, lambda xs, ys: 0.6), "#08080a")
    if kind == "shark":
        a.poly([(16, 12), (20, 6), (22, 12)], shade_fill(pal, lambda xs, ys: 0.8), "#04040a")
        a.px(26, 14, "#f0f0f0")
    elif kind == "sniper":
        a.poly([(12, 8), (22, 8), (22, 11), (12, 11)], shade_fill(GUNMETAL, lambda xs, ys: 0.6), "#04040a")
        a.px(21, 9, "#8ad8ff")
    elif kind == "flame":
        a.disc(20, 20, 3.5, shade_fill(["#6a1004", "#b02a08", "#e85a10", "#ff9a30", "#ffe080"], lit(19, 19, 3)), "#2a0602")
        a.px(30, 13, "#ffd060")
    elif kind == "star":
        for x, y in ((29, 9), (31, 13), (29, 17)):
            a.px(x, y, "#fff4a0")
    elif kind == "coin":
        a.disc(20, 21, 3.0, shade_fill(GOLD, lit(19, 20, 3)), "#3a2604")
    a.px(7, 13, pal[-1])


def scope_icon(a):
    a.poly([(4, 12), (28, 12), (28, 19), (4, 19)], shade_fill(GUNMETAL, lambda xs, ys: 0.9 - (ys - 12) / 10), "#04040a")
    a.disc(27, 15, 4.5, shade_fill(GUNMETAL, lit(26, 14, 4)), "#04040a")
    a.disc(27, 15, 2.5, shade_fill(["#0a3a5a", "#1a6a9a", "#3aa0d8", "#8ad8ff", "#ffffff"], lit(26, 14, 2)), "#04040a")
    a.poly([(12, 19), (16, 19), (16, 23), (12, 23)], shade_fill(GUNMETAL, lambda xs, ys: 0.4), "#04040a")


def rocket_icon(a):
    a.poly([(6, 22), (20, 8), (25, 13), (11, 27)], shade_fill(["#3a3a40", "#6a6a74", "#9a9aa6", "#c8c8d4", "#f0f0f8"], lambda xs, ys: 0.9 - (xs - 6) / 30), "#16181c")
    a.poly([(20, 8), (28, 4), (25, 13)], shade_fill(RED, lambda xs, ys: 0.8), "#2a0404")
    a.poly([(6, 22), (2, 22), (6, 18)], shade_fill(RED, lambda xs, ys: 0.5), "#2a0404")
    a.poly([(11, 27), (11, 31), (15, 27)], shade_fill(RED, lambda xs, ys: 0.5), "#2a0404")


def arrow_icon(a, tip):
    lin(5, 27, 23, 9, 1.4, shade_fill(WOOD, lambda xs, ys: 0.8 - (xs - 5) / 30), "#140c06", a)
    a.poly([(21, 6), (29, 3), (26, 11)], shade_fill(tip, lit(25, 6, 4)), tip[0])
    a.poly([(4, 24), (8, 28), (5, 30), (2, 26)], shade_fill(["#a8a8b0", "#d8d8de", "#ffffff"], lambda xs, ys: 0.7), "#55555c")
    a.px(27, 5, tip[-1])


def hammer_icon(a):
    lin(6, 27, 18, 13, 2.6, shade_fill(WOOD, lambda xs, ys: 0.8 - (xs - 6) / 30), "#140c06", a)
    a.poly([(12, 9), (22, 3), (29, 12), (19, 18)], shade_fill(GOLD, lit(18, 7, 7)), "#3a2604")
    a.poly([(16, 9), (22, 6), (25, 11), (19, 14)], shade_fill(STEEL, lit(19, 8, 4)), "#16181c")


def club_icon(a):
    lin(5, 28, 16, 17, 3.0, shade_fill(WOOD, lambda xs, ys: 0.8 - (xs - 5) / 30), "#140c06", a)
    a.poly([(12, 14), (20, 4), (29, 6), (28, 14), (18, 21)], shade_fill(["#2a1a0e", "#4a2e18", "#6e4626", "#946236", "#b88050"], lit(20, 8, 8)), "#140c06")
    for x, y in ((20, 8), (25, 9), (21, 14)):
        a.px(x, y, "#c8c8d0")


def tome_icon(a, pal):
    a.poly([(6, 6), (24, 4), (26, 26), (8, 28)], shade_fill(pal, lambda xs, ys: 0.9 - (xs - 6) / 30), pal[0])
    a.poly([(24, 4), (27, 6), (28, 27), (26, 26)], shade_fill(["#a89a7a", "#d8ccb0", "#f4ecd8"], lambda xs, ys: 0.8), "#4a4030")
    a.disc(16, 16, 4.0, shade_fill(GOLD, lit(15, 15, 3)), "#3a2604")


def pine_icon(a):
    lin(16, 30, 16, 24, 2.0, shade_fill(WOOD, lambda xs, ys: 0.6), "#140c06", a)
    for k, (w, y) in enumerate(((12, 24), (9, 17), (6, 10))):
        a.poly([(16 - w, y), (16 + w, y), (16, y - 9)], shade_fill(["#0e2a16", "#16401e", "#1e5a2a", "#2a7438", "#5aa070"], lambda xs, ys: 0.9 - (xs - 4) / 30), "#04140a")
    a.px(16, 2, "#f2cc52")


def staff_icon(a, wood, gem):
    lin(6, 28, 22, 10, 2.0, shade_fill(wood, lambda xs, ys: 0.8 - (xs - 6) / 30), wood[0], a)
    a.disc(24, 8, 4.5, shade_fill(gem, lit(23, 7, 4)), gem[0])
    a.px(23, 6, "#ffffff")


# ==============================================================================================
# L20: tools and weapons
# ==============================================================================================

COBALT = ["#0a1e4a", "#143a7a", "#2a5aaa", "#4a86d8", "#8ab8f4"]
MYTHRIL = ["#0e3a2a", "#16584a", "#2a8a6a", "#4ab890", "#9ae0c4"]
ADAM = ["#4a0a14", "#7a1424", "#aa2236", "#d84a5a", "#f498a0"]
VERD = ["#1a3a0a", "#2a5a14", "#40821e", "#62aa34", "#a0d870"]
LUMI = ["#0a3a3a", "#145a5a", "#1e8484", "#3ab4b4", "#90f0e8"]

for metal, pal, tier, speed, dur, ingot in (("cobalt", COBALT, 5, 13.0, 1800, "cobalt_ingot"), ("mythril", MYTHRIL, 6, 15.5, 2300, "mythril_ingot"), ("adamantite", ADAM, 7, 18.0, 2800, "adamantite_ingot")):
    dn = {"cobalt": "de Cobalto", "mythril": "de Mitril", "adamantite": "de Adamantita"}[metal]
    en = metal.capitalize()
    item(f"{metal}_drill", f"Broca {dn}", f"{en} Drill", 1, f"tool: Some((kind: Pickaxe, tier: {tier}, speed: {speed}, damage: {tier + 1.0}, durability: {dur}))")
    item(f"{metal}_chainsaw", f"Motosserra {dn}", f"{en} Chainsaw", 1, f"tool: Some((kind: Axe, tier: {tier}, speed: {speed}, damage: {tier + 3.0}, durability: {dur}))")
    shaped(["II ", "IRI", " IS"], {"I": ingot, "R": "redstone", "S": "stick"}, f"{metal}_drill")
    shaped([" II", "IRI", "SI "], {"I": ingot, "R": "redstone", "S": "stick"}, f"{metal}_chainsaw")
    ICONS[f"{metal}_drill"] = sprite(lambda a, p=pal: drill_icon(a, p))
    ICONS[f"{metal}_chainsaw"] = sprite(lambda a, p=pal: saw_icon(a, p))
item("pickaxe_axe", "Picareta-Machado", "Pickaxe Axe", 1, "tool: Some((kind: Pickaxe, tier: 8, speed: 16.0, damage: 14.0, durability: 3200))")
item("picksaw", "Picosserra", "Picksaw", 1, "tool: Some((kind: Pickaxe, tier: 8, speed: 19.0, damage: 16.0, durability: 3600))")
item("molten_hamaxe", "Machado-Martelo de Magma", "Molten Hamaxe", 1, "tool: Some((kind: Axe, tier: 5, speed: 10.0, damage: 10.0, durability: 2000))")
item("lumite_hamaxe", "Machado-Martelo de Lumita", "Luminite Hamaxe", 1, "tool: Some((kind: Axe, tier: 9, speed: 20.0, damage: 26.0, durability: 4000))")
shaped(["MMM", "AS ", " S "], {"M": "mythril_ingot", "A": "adamantite_ingot", "S": "stick"}, "pickaxe_axe")
shaped(["VVV", "LS ", " S "], {"V": "verdite_ingot", "L": "soul_of_might", "S": "stick"}, "picksaw")
shaped(["MM ", "MS ", " S "], {"M": "magmarite_ingot", "S": "stick"}, "molten_hamaxe")
shaped(["LL ", "LS ", " S "], {"L": "lumite_ingot", "S": "stick"}, "lumite_hamaxe")
ICONS.update({
    "pickaxe_axe": sprite(lambda a: multi_icon(a, MYTHRIL, ADAM)),
    "picksaw": sprite(lambda a: multi_icon(a, VERD, STEEL)),
    "molten_hamaxe": sprite(lambda a: hamaxe_icon(a, ["#3a0a04", "#7a1a08", "#c43a10", "#f07020", "#ffc060"])),
    "lumite_hamaxe": sprite(lambda a: hamaxe_icon(a, LUMI)),
})
# Utility.
item("rod_of_discord", "Bastão da Discórdia", "Rod of Discord", 1)
item("bottomless_water_bucket", "Balde de Água Sem Fundo", "Bottomless Water Bucket", 1)
item("bottomless_lava_bucket", "Balde de Lava Sem Fundo", "Bottomless Lava Bucket", 1)
item("absorbent_sponge", "Esponja Superabsorvente", "Super Absorbant Sponge", 1)
item("lava_sponge", "Esponja de Lava", "Lava Absorbant Sponge", 1)
shapeless(["water_bucket", "water_bucket", "water_bucket", "soul_of_light", "soul_of_night"], "bottomless_water_bucket")
shapeless(["lava_bucket", "lava_bucket", "lava_bucket", "soul_of_light", "soul_of_night"], "bottomless_lava_bucket")
shapeless(["sponge", "sponge", "sponge", "soul_of_light"], "absorbent_sponge")
shapeless(["sponge", "sponge", "magmarite_ingot", "soul_of_night"], "lava_sponge")
for n, dn, en, rest in (("toolbelt", "Cinto de Ferramentas", "Toolbelt", None), ("toolbox", "Caixa de Ferramentas", "Toolbox", None), ("extendo_grip", "Pinça Extensora", "Extendo Grip", None), ("architect_gizmo", "Kit do Arquiteto", "Architect Gizmo Pack", None),
                         ("paladin_shield", "Escudo do Paladino", "Paladin's Shield", None), ("festive_wings", "Asas Festivas", "Festive Wings", None)):
    item(n, dn, en, 1)
shaped(["LSL", "L L", "LLL"], {"L": "leather", "S": "string"}, "toolbelt")
shaped(["PIP", "P P", "PPP"], {"P": "oak_planks", "I": "iron_ingot"}, "toolbox")
shaped([" I ", " S ", "S  "], {"I": "iron_ingot", "S": "stick"}, "extendo_grip")
shapeless(["toolbelt", "toolbox", "extendo_grip"], "architect_gizmo")
ICONS.update({
    "rod_of_discord": sprite(lambda a: rod_icon(a)),
    "bottomless_water_bucket": sprite(lambda a: bucket_icon(a, ["#0a2a6a", "#1a4aa0", "#2a6ad0", "#5a9af0", "#a0d0ff"])),
    "bottomless_lava_bucket": sprite(lambda a: bucket_icon(a, ["#6a1004", "#b02a08", "#e85a10", "#ff9a30", "#ffe080"])),
    "absorbent_sponge": sprite(lambda a: sponge_icon(a, ["#5a4a0a", "#8a7414", "#c0a828", "#e8d050", "#fff4a0"])),
    "lava_sponge": sprite(lambda a: sponge_icon(a, ["#4a1a0a", "#7a2a10", "#b04a1a", "#e07a2a", "#ffc060"])),
    "toolbelt": sprite(lambda a: belt_icon(a)),
    "toolbox": sprite(lambda a: box_icon(a)),
    "extendo_grip": sprite(lambda a: grip_icon(a)),
    "architect_gizmo": sprite(lambda a: (box_icon(a), grip_icon(a))),
    "paladin_shield": sprite(lambda a: shield_icon(a)),
    "festive_wings": sprite(lambda a: wing(a, ["#0a2a0e", "#14501a", "#22802a", "#4ab04a", "#f2cc52"])),
})
# Guns and launchers.
GUNMETAL = ["#121216", "#24242c", "#3a3a46", "#5a5a6a", "#8a8aa0"]
for n, dn, en, look_, rec in (
        ("blowpipe", "Zarabatana", "Blowpipe", ("pipe", ["#3a2a10", "#5a4220", "#7a5a2e", "#9a7640", "#c09a5a"]), (["   ", "SSS", "   "], {"S": "sugar_cane"})),
        ("minishark", "Minitubarão", "Minishark", ("shark", GUNMETAL), (["III", "FGR", "  I"], {"I": "iron_ingot", "F": "shark_fin", "G": "gunpowder", "R": "redstone"})),
        ("megashark", "Megatubarão", "Megashark", ("shark", ["#0a1a2a", "#14304a", "#22507a", "#3a7ab0", "#7ab0e0"]), (["MMM", "FSR", "  M"], {"M": "minishark", "F": "shark_fin", "S": "soul_of_might", "R": "adamantite_ingot"})),
        ("sniper_rifle", "Rifle de Precisão", "Sniper Rifle", ("sniper", ["#1a2a1a", "#2a3e2a", "#3e563e", "#5a765a", "#8aa88a"]), None),
        ("flamethrower", "Lança-Chamas", "Flamethrower", ("flame", ["#4a1a0a", "#7a2a10", "#a84418", "#d86a28", "#ffa048"]), (["III", "LSG", "  I"], {"I": "adamantite_ingot", "L": "lava_bucket", "S": "soul_of_fright", "G": "gel"})),
        ("star_cannon", "Canhão de Estrelas", "Star Cannon", ("star", ["#2a2a5a", "#3e3e86", "#5a5ab4", "#8a8ae0", "#d8d8ff"]), (["MMM", "SSG", "  M"], {"M": "minishark", "S": "fallen_star", "G": "gold_ingot"})),
        ("coin_gun", "Arma de Moedas", "Coin Gun", ("coin", GOLD), None),
        ("elf_gun", "Metralhadora Natalina", "Elf Melter", ("shark", ["#0a3a14", "#14601e", "#22902e", "#c42424", "#ff9090"]), None),
        ("rocket_launcher", "Lança-Foguetes", "Rocket Launcher", ("tube", ["#2a3a14", "#3e5420", "#56702e", "#7a944a", "#a8c070"]), (["III", "GTR", "  I"], {"I": "adamantite_ingot", "G": "gunpowder", "T": "tnt", "R": "redstone"})),
        ("grenade_launcher", "Lança-Granadas", "Grenade Launcher", ("tube", ["#3a2a14", "#54401e", "#70582c", "#94763e", "#c0a060"]), (["III", "GBR", "  I"], {"I": "mythril_ingot", "G": "gunpowder", "B": "grenade", "R": "redstone"}))):
    item(n, dn, en, 1)
    if rec:
        shaped(rec[0], rec[1], n)
    ICONS[n] = sprite(lambda a, l=look_: gun_icon(a, *l))
item("rifle_scope", "Mira de Rifle", "Rifle Scope", 1)
ICONS["rifle_scope"] = sprite(lambda a: scope_icon(a))
item("rocket", "Foguete", "Rocket")
shapeless(["gunpowder", "iron_ingot", "paper"], "rocket", 8)
ICONS["rocket"] = sprite(lambda a: rocket_icon(a))
# Arrows of the Terraria side (weapons.rs flies them as arrows, tr3_tools.rs gives each its bite).
for n, dn, en, tip, rec in (
        ("flaming_arrow", "Flecha Flamejante", "Flaming Arrow", ["#6a1a04", "#c04a0a", "#ff8a20", "#ffd060", "#ffffff"], ["arrow", "arrow", "arrow", "torch"]),
        ("frostburn_arrow", "Flecha Gélida", "Frostburn Arrow", ["#0a3a6a", "#1a6aa0", "#3aa0d8", "#8ad8ff", "#ffffff"], ["arrow", "arrow", "arrow", "ice"]),
        ("unholy_arrow", "Flecha Profana", "Unholy Arrow", ["#2a0a3a", "#4a145a", "#702a8a", "#a050c0", "#e0a0ff"], ["arrow", "arrow", "arrow", "bone"]),
        ("jester_arrow", "Flecha do Bobo", "Jester's Arrow", ["#5a4a0a", "#a08a14", "#e8c828", "#fff070", "#ffffff"], ["arrow", "arrow", "arrow", "fallen_star"]),
        ("hellfire_arrow", "Flecha Infernal", "Hellfire Arrow", ["#4a0a04", "#8a1a08", "#d03a10", "#ff7020", "#ffd080"], ["arrow", "arrow", "arrow", "magmarite_ingot"]),
        ("holy_arrow", "Flecha Sagrada", "Holy Arrow", ["#4a3a6a", "#7a6aa8", "#b0a0e0", "#e0d8ff", "#ffffff"], ["arrow", "arrow", "arrow", "soul_of_light"])):
    item(n, dn, en)
    shapeless(rec, n, 3)
    ICONS[n] = sprite(lambda a, t=tip: arrow_icon(a, t))
# Weapons the new monsters drop.
item("paladin_hammer", "Martelo do Paladino", "Paladin's Hammer", 1)
item("ogre_club", "Clava do Ogro", "Ogre's Club", 1, "tool: Some((kind: Sword, tier: 6, speed: 1.0, damage: 13.0, durability: 2400))")
item("betsy_wrath", "Ira de Betsy", "Betsy's Wrath", 1)
item("razorpine", "Pinho Navalha", "Razorpine", 1)
item("spooky_staff", "Cajado Assombrado", "Spooky Staff", 1)
ICONS.update({
    "paladin_hammer": sprite(lambda a: hammer_icon(a)),
    "ogre_club": sprite(lambda a: club_icon(a)),
    "betsy_wrath": sprite(lambda a: tome_icon(a, ["#3a0a04", "#6a1408", "#a02a0e", "#d04a18", "#ff9040"])),
    "razorpine": sprite(lambda a: pine_icon(a)),
    "spooky_staff": sprite(lambda a: staff_icon(a, ["#120c10", "#22161e", "#34222e", "#48303e", "#5c4050"], ["#6a2a04", "#c45a0a", "#ff9a20", "#ffd060", "#ffffff"])),
})


# ==============================================================================================
# Writing it all out
# ==============================================================================================


def ron_mob(m):
    parts = []
    for p in m["model"].parts:
        s = f"(size: {p['size']}, origin: {p['origin']}, uv: {p['uv']}"
        if p["anim"]:
            s += f", anim: {p['anim']}, pivot: {p['pivot']}"
        parts.append(s + ")")
    out = f'    (\n        name: "{m["name"]}", display_name: "{m["display"]}", {m["ron"]},\n        parts: [\n'
    out += "".join(f"            {p},\n" for p in parts) + "        ],\n"
    if m["model"].layers:
        out += "        layers: [" + ", ".join(f"(part: {a}, inflate: {b})" for a, b in m["model"].layers) + "],\n"
    return out + "    ),\n"


def write(name, text):
    with open(os.path.join(OUT, name), "w", encoding="utf-8") as f:
        f.write(text)


def main():
    os.makedirs(OUT, exist_ok=True)
    head = "// The Terraria side, third round (plan/terraria-completo.md, L17-L22). Made by tools/gen_tr3.py:\n// edit it and run it again.\n"
    write("blocks.ron", head + "(blocks: [\n" + "\n".join(BLOCKS) + "\n])\n")
    eggs = []
    for m in MOBS:
        if m["egg"]:
            n = m["name"] + "_spawn_egg"
            eggs.append(f'    (name: "{n}", display_name: Some("Ovo de {m["display"]}"), texture: Some("{n}")),')
            EN["Ovo de " + m["display"]] = EN.get(m["display"], m["display"]) + " Spawn Egg"
    write("items.ron", head + "(items: [\n" + "\n".join(ITEMS + eggs) + "\n])\n")
    write("recipes.ron", head + "(\n    shaped: [\n" + "\n".join(SHAPED) + "\n    ],\n    shapeless: [\n" + "\n".join(SHAPELESS) + "\n    ],\n)\n")
    write("mobs.ron", head + "(mobs: [\n" + "".join(ron_mob(m) for m in MOBS) + "])\n")
    write("structures.ron", head + "(structures: [\n" + "".join(STRUCTURES) + "])\n")
    write("en.ron", "// English names of the third round's things (merged into packs/vanilla/lang/en.ron by gen_tr3.py).\n{\n" + "".join(f'    "{k}": "{v}",\n' for k, v in EN.items()) + "}\n")
    for m in MOBS:
        render_mob(m["name"], m["model"]).save(os.path.join(ENTITY, m["name"] + ".png"))
        if m["egg"]:
            egg(*m["egg"]).save(os.path.join(ITEM_TEX, m["name"] + "_spawn_egg.png"))
    for n, fn in ICONS.items():
        fn().save(os.path.join(ITEM_TEX, n + ".png"))
    from style import to_u8
    for n, fn in blocks_art().items():
        img = fn()
        if isinstance(img, Image.Image):
            img.save(os.path.join(BLOCK_TEX, n + ".png"))
        else:
            Image.fromarray(to_u8(img), "RGBA").save(os.path.join(BLOCK_TEX, n + ".png"))
    merge_english()
    print(f"wrote {len(ITEMS)} items, {len(BLOCKS)} blocks, {len(MOBS)} creatures, {len(ICONS)} icons")
    if len(sys.argv) > 1:
        sheet(sys.argv[1])


def merge_english():
    """Add the English names to packs/vanilla/lang/en.ron between this script's markers."""
    p = os.path.join(PACK, "lang", "en.ron")
    text = open(p, encoding="utf-8").read()
    a, b = "    // gen_tr3.py: begin\n", "    // gen_tr3.py: end\n"
    if a in text:
        i, j = text.index(a), text.index(b) + len(b)
        text = text[:i] + text[j:]
    else:
        i = text.rindex("}")
    # Keys the file already has (a duplicate key would break it).
    have = set(l.split('":', 1)[0].strip().strip('"') for l in text.splitlines() if '":' in l)
    body = a + "".join(f'    "{k}": "{v}",\n' for k, v in EN.items() if k not in have) + b
    text = text[:i] + body + text[i:]
    open(p, "w", encoding="utf-8").write(text)


def sheet(path):
    icons = list(ICONS) + [m["name"] + "_spawn_egg" for m in MOBS if m["egg"]]
    cols = 10
    rows = (len(icons) + cols - 1) // cols
    mobs_rows = (len(MOBS) + cols - 1) // cols
    img = Image.new("RGBA", (cols * 68, rows * 68 + mobs_rows * 132), (40, 40, 44, 255))
    for k, n in enumerate(icons):
        img.alpha_composite(Image.open(os.path.join(ITEM_TEX, n + ".png")).convert("RGBA"), ((k % cols) * 68, (k // cols) * 68))
    for k, m in enumerate(MOBS):
        sk = Image.open(os.path.join(ENTITY, m["name"] + ".png")).convert("RGBA").resize((64, 128), Image.NEAREST)
        img.alpha_composite(sk, ((k % cols) * 68, rows * 68 + (k // cols) * 132))
    img.resize((img.width * 2, img.height * 2), Image.NEAREST).save(path)


if __name__ == "__main__":
    main()
