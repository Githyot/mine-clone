#!/usr/bin/env python3
"""Lote 2: the vanilla creatures we had no version of. Writes their entries between the
`// lote2 begin` and `// lote2 end` marks at the end of packs/vanilla/mobs.ron, their 64x128 skin
designs in packs/vanilla/textures/entity/, and spawn eggs (16x16) in .../item/.

Some reuse a model of ours (cave spider from our spider, mooshroom from our cow, mule and skeleton
horse from our donkey and horse, zoglin from our hoglin, bogged from our skeleton, zombie villager
from our settlers, glow squid from our squid, elder guardian from our deep watcher) with a skin
recoloured here; the rest are built from the body plans of tools/dim_mobs_spec.py and painted by
tools/gen_dim_mobs.py. Every pixel is ours; only Minecraft's ideas are used.

    python3 tools/gen_lote2_mobs.py && python3 tools/tex64/build.py --only <the names it prints>
"""

import os
import re
import sys
import zlib

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dim_mobs_spec as spec  # noqa: E402
from dim_mobs_spec import EYE, WHITE, Model, biped, bird, blob, look, quadruped, worm  # noqa: E402
import gen_dim_mobs as gdm  # noqa: E402
from gen_mob_skins import fbm, hash01, hexrgb  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MOBS_RON = os.path.join(ROOT, "packs", "vanilla", "mobs.ron")
ITEMS_RON = os.path.join(ROOT, "packs", "vanilla", "items.ron")
ENTITY = os.path.join(ROOT, "packs", "vanilla", "textures", "entity")
ITEM_TEX = os.path.join(ROOT, "packs", "vanilla", "textures", "item")
SRC16 = os.path.join(ROOT, "tools", "tex64", "src16", "entity")

# -------------------------------------------------------------------------------- recolouring


def lum(c):
    return (0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]) / 255.0


def ramp(pal, t):
    pal = [hexrgb(c) if isinstance(c, str) else c for c in pal]
    t = max(0.0, min(0.999, t))
    k = t * (len(pal) - 1)
    i = int(k)
    a, b = pal[i], pal[min(i + 1, len(pal) - 1)]
    f = k - i
    return tuple(int(a[j] + (b[j] - a[j]) * f) for j in range(3))


def recolour(base, pal, keep=None, lo=None, hi=None, patches=None, seed=1):
    """A new skin from one of ours: every pixel's brightness picks a colour of `pal`. `keep(rgb)`
    leaves special pixels (eyes) as they are; `patches` = (pal2, amount) paints noise patches."""
    img = Image.open(os.path.join(SRC16, base + ".png")).convert("RGBA")
    px = img.load()
    vals = [lum(px[x, y]) for y in range(img.height) for x in range(img.width) if px[x, y][3] > 0]
    lo = min(vals) if lo is None else lo
    hi = max(vals) if hi is None else hi
    for y in range(img.height):
        for x in range(img.width):
            r, g, b, a = px[x, y]
            if a == 0 or (keep and keep((r, g, b))):
                continue
            t = (lum((r, g, b)) - lo) / max(hi - lo, 1e-3)
            c = ramp(pal, t)
            if patches and fbm(seed, x * 0.7, y * 0.7, 0, 6.0) > 1.0 - patches[1]:
                c = ramp(patches[0], t)
            px[x, y] = (*c, a)
    return img


def free_spot(img, w, h, rows=range(0, 64)):
    """A free w x h rectangle in the top half of a skin (for extra boxes)."""
    px = img.load()
    for y in rows:
        if y + h > 64:
            break
        for x in range(0, 64 - w + 1):
            if all(px[x + i, y + j][3] == 0 for j in range(h) for i in range(w)):
                return x, y
    raise RuntimeError("no room on the skin")


def paint_box(img, uv, size, pal, seed, dots=None):
    """Paint the unwrap of a box (Minecraft layout) at uv with a palette and optional dots."""
    w, h, d = size
    u, v = uv
    px = img.load()
    for j in range(d + h):
        for i in range(2 * (w + d)):
            if j < d and not (d <= i < d + 2 * w):
                continue
            t = 0.35 + 0.5 * hash01(seed, i, j, 3)
            c = ramp(pal, t)
            if dots and hash01(seed + 5, i, j, 1) > 0.72:
                c = dots
            px[u + i, v + j] = (*c, 255)


# -------------------------------------------------------------------------------- mobs.ron text

SRC_TEXT = open(MOBS_RON, encoding="utf-8").read()
START, END = "    // lote2 begin\n", "    // lote2 end\n"
if START in SRC_TEXT:
    SRC_TEXT = SRC_TEXT[: SRC_TEXT.index(START)] + SRC_TEXT[SRC_TEXT.index(END) + len(END):]


def block_of(name):
    """The `( name: ... ),` entry of a mob in mobs.ron, as text."""
    i = SRC_TEXT.index(f'name: "{name}"')
    s = SRC_TEXT.rindex("    (\n", 0, i)
    depth = 0
    for k in range(s + 4, len(SRC_TEXT)):
        ch = SRC_TEXT[k]
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0:
                return SRC_TEXT[s : k + 1] + ",\n"
    raise ValueError(name)


def clone(base, name, display, fields, extra_parts=()):
    """Copy a mob's entry under a new name, replacing the header fields (everything before
    `parts:`) and adding parts."""
    t = block_of(base)
    head, rest = t.split("parts: [", 1)
    out = "    (\n        " + f'name: "{name}", display_name: "{display}", ' + fields + "\n        parts: [" + rest
    if extra_parts:
        out = out.replace("\n        ],", "\n" + "".join(f"            {p},\n" for p in extra_parts) + "        ],", 1)
    return out


ENTRIES = []
SKINS = {}
EGGS = {}

# ---- Cave spider: our spider, small, dark teal; it poisons (crates/game/src/vanilla_mobs.rs).
red_eye = lambda c: c[0] > 120 and c[1] < 60 and c[2] < 60  # noqa: E731
SKINS["cave_spider"] = recolour("spider", ["#06171a", "#0d2c30", "#164046", "#22595e"], keep=red_eye)
ENTRIES.append(clone("spider", "cave_spider", "Aranha da Caverna",
    'kind: Hostile, health: 12, speed: 3.3, scale: 0.7,\n        drops: [(item: "string", min: 0, max: 2), (item: "spider_eye", chance: 0.33)], xp: 5,\n'
    '        width: 0.7, height: 0.5, spawn: Never, attack: 2, climbs: true,'))
EGGS["cave_spider"] = ((12, 66, 76), (163, 0, 0))

# ---- Mooshroom: our cow in red, with mushrooms growing on its back.
moo = recolour("cow", ["#3d0b0b", "#7a1414", "#a81e1e", "#e8e2da", "#f6f2ec"], lo=None)
mush = free_spot(moo, 12, 6)
paint_box(moo, mush, (3, 3, 3), ["#8a1010", "#c41f1f", "#d83030"], 7, dots=(240, 236, 230))
SKINS["mooshroom"] = moo
ENTRIES.append(clone("cow", "mooshroom", "Vaca-Cogumelo",
    'kind: Passive, health: 10, speed: 2.2,\n        drops: [(item: "beef", min: 1, max: 3), (item: "leather", min: 0, max: 2)], xp: 2,\n'
    '        width: 0.9, height: 1.4, spawn: Grass, weight: 8, biomes: ["Mushroom Fields"], ground: ["mycelium", "grass_block"],',
    extra_parts=[f"(size: (3, 3, 3), origin: (-4, 20, -4), uv: {mush})", f"(size: (3, 3, 3), origin: (1, 20, 2), uv: {mush})"]))
EGGS["mooshroom"] = ((160, 13, 19), (183, 183, 183))

# ---- Glow squid: our squid, deep teal with bright specks.
gs = recolour("squid", ["#06262a", "#0b3d42", "#127077", "#38c8b8"], patches=(["#2fb8a8", "#62f0d8", "#a0fff0"], 0.25), seed=11)
SKINS["glow_squid"] = gs
ENTRIES.append(clone("squid", "glow_squid", "Lula Brilhante",
    'kind: Passive, health: 10, speed: 1.5,\n        drops: [(item: "glow_ink_sac", min: 1, max: 3)], xp: 1,\n'
    '        width: 0.8, height: 0.8, spawn: Water, group: (2, 3), weight: 3, aquatic: true, scale: 0.55, flee_range: 4.0,\n'
    '        biomes: ["Ocean", "Frozen Ocean", "Lush Caves"],'))
EGGS["glow_squid"] = ((9, 89, 96), (133, 241, 188))

# ---- Mule (donkey and horse foals) and the skeleton horse.
SKINS["mule"] = recolour("donkey", ["#1e120a", "#3a2416", "#55361f", "#6e4a2c"])
ENTRIES.append(clone("donkey", "mule", "Mula",
    'kind: Passive, health: 22, speed: 4.0,\n        drops: [(item: "leather", min: 0, max: 2)], xp: 3,\n'
    '        width: 1.3965, height: 1.6, spawn: Never, flee_range: 6.0, scale: 0.92,'))
EGGS["mule"] = ((27, 2, 0), (81, 51, 29))
bone = ["#5c5a50", "#9a978a", "#c8c5b8", "#e6e3d8"]
SKINS["skeleton_horse"] = recolour("horse", bone)
ENTRIES.append(clone("horse", "skeleton_horse", "Cavalo-Esqueleto",
    'kind: Passive, health: 15, speed: 5.0,\n        drops: [(item: "bone", min: 0, max: 2)], xp: 3,\n'
    '        width: 1.3965, height: 1.6, spawn: Never, swims: true,'))
EGGS["skeleton_horse"] = ((104, 104, 79), (229, 229, 216))

# ---- Zoglin: a hoglin gone rotten outside the Nether.
SKINS["zoglin"] = recolour("hoglin", ["#3c2a2a", "#7a5656", "#b58a86", "#d9b8b0"], patches=(["#3e5a32", "#5f8a4a", "#7ea866"], 0.3), seed=5)
ENTRIES.append(clone("hoglin", "zoglin", "Zoglin",
    'kind: Hostile, health: 40, speed: 2.6,\n        drops: [(item: "rotten_flesh", min: 1, max: 3)], xp: 5,\n'
    '        width: 1.3965, height: 1.4, spawn: Never, attack: 6, scale: 1.25,'))
EGGS["zoglin"] = ((199, 106, 95), (230, 230, 230))

# ---- Bogged: a skeleton overgrown with moss in the swamps; its arrows poison.
SKINS["bogged"] = recolour("skeleton", ["#1f2a14", "#3f5428", "#6d8448", "#9fb070"], patches=(["#5a3a1a", "#8a5a2a", "#b07a3a"], 0.12), seed=3)
ENTRIES.append(clone("skeleton", "bogged", "Pantanoso",
    'kind: Hostile, health: 16, speed: 2.3,\n        drops: [(item: "bone", min: 0, max: 2), (item: "arrow", min: 0, max: 2), (item: "brown_mushroom", chance: 0.3)], xp: 5,\n'
    '        width: 0.6, height: 1.99, spawn: Dark, attack: 3, ranged: true, burns_in_daylight: true, weight: 6,\n'
    '        biomes: ["Swamp", "Mangrove Swamp"],'))
EGGS["bogged"] = ((138, 160, 96), (54, 75, 40))

# ---- Zombie villager: a settler gone green; a golden apple cures it.
skin_like = lambda c: c[0] > 150 and c[1] > 100 and c[0] > c[2] + 30 and c[0] - c[1] < 70  # noqa: E731


def zombie_skin(base):
    img = Image.open(os.path.join(SRC16, base + ".png")).convert("RGBA")
    px = img.load()
    for y in range(img.height):
        for x in range(img.width):
            r, g, b, a = px[x, y]
            if a and skin_like((r, g, b)):
                t = lum((r, g, b))
                px[x, y] = (*ramp(["#2f4a22", "#4d7038", "#6a9050", "#86ac64"], t * 1.1), a)
            elif a:
                px[x, y] = (int(r * 0.7), int(g * 0.75), int(b * 0.7), a)
    return img


SKINS["zombie_villager"] = zombie_skin("settler_farmer")
ENTRIES.append(clone("settler_farmer", "zombie_villager", "Aldeão Zumbi",
    'kind: Hostile, health: 20, speed: 2.2,\n        drops: [(item: "rotten_flesh", min: 0, max: 2), (item: "carrot", chance: 0.02), (item: "potato", chance: 0.02)], xp: 5,\n'
    '        width: 0.6, height: 1.95, spawn: Dark, attack: 3, burns_in_daylight: true, weight: 2,'))
EGGS["zombie_villager"] = ((86, 60, 52), (121, 156, 101))

# ---- Elder guardian: our deep watcher grown old and pale; guards the ocean monuments.
SKINS["elder_guardian"] = recolour("deep_watcher", ["#6b6a62", "#a8a698", "#cfcabb", "#ece6d6"], keep=lambda c: c[0] > 200 and c[1] > 150 and c[2] < 120)
ENTRIES.append(clone("deep_watcher", "elder_guardian", "Guardião Ancião",
    'kind: Hostile, health: 80, speed: 1.0,\n        drops: [(item: "prismarine_shard", min: 0, max: 2), (item: "prismarine_crystals", min: 0, max: 1), (item: "wet_sponge", min: 1, max: 1), (item: "cod", min: 0, max: 1)], xp: 10,\n'
    '        width: 1.9975, height: 1.9975, spawn: Never, attack: 8, ranged: true, aquatic: true, scale: 2.3,'))
EGGS["elder_guardian"] = ((206, 204, 186), (116, 118, 147))

# -------------------------------------------------------------------------------- new models
NEW = []


def new(name, display, model, kind="Passive", health=10, speed=2.2, size=(0.8, 0.9), spawn="Grass", egg=None, **kw):
    model.pack()
    NEW.append(dict(name=name, display=display, model=model, kind=kind, health=health, speed=speed, size=size, spawn=spawn, kw=kw))
    if egg:
        EGGS[name] = egg


stone = ["#4e4e4e", "#6a6a6a", "#848484", "#9c9c9c"]
new("silverfish", "Traça",
    worm(4, (3, 3, 3), look("sfhead", ["#5a5c60", "#7a7e84", "#9aa0a6"], eyes=(EYE, None)), look("sfbody", ["#5a5c60", "#7a7e84", "#9aa0a6"], pattern="rings")),
    kind="Hostile", health=8, speed=2.6, size=(0.4, 0.3), spawn="Never", attack=1, xp=5, egg=((110, 110, 110), (48, 48, 48)))
bat = Model()
bat.box((6, 6, 6), (-3, 6, -3), look("btbody", ["#2a2016", "#3c2e20", "#4e3c2a"], eyes=((20, 10, 10), None)), "Head", (0, 9, 0))
bat.box((4, 6, 4), (-2, 0, -2), look("btchest", ["#2a2016", "#3c2e20"]))
bat.box((8, 6, 1), (-11, 1, 0), look("btwing", ["#18120c", "#241b12", "#30241a"]), "Wing(0)", (-2, 4, 0))
bat.box((8, 6, 1), (3, 1, 0), look("btwing", ["#18120c", "#241b12", "#30241a"]), "Wing(1)", (2, 4, 0))
bat.box((1, 2, 1), (-3, 12, 0), look("btear", ["#2a2016", "#3c2e20"]), "Head", (0, 9, 0))
bat.box((1, 2, 1), (2, 12, 0), look("btear", ["#2a2016", "#3c2e20"]), "Head", (0, 9, 0))
new("bat", "Morcego", bat, health=6, speed=3.0, size=(0.5, 0.9), spawn="Dark", flies=True, hover=3.0, flee_range=3.0, weight=4, group=(1, 3),
    egg=((76, 62, 48), (15, 15, 15)))
golem = Model()
snow = look("sgsnow", ["#d6dee6", "#e6ecf2", "#f2f6fa", "#ffffff"], pattern="cloud")
golem.box((10, 10, 10), (-5, 0, -5), snow)
golem.box((8, 8, 8), (-4, 10, -4), snow)
golem.box((8, 8, 8), (-4, 18, -4), look("sghead", ["#a85410", "#d07018", "#e88a24", "#f4a83a"], eyes=((40, 20, 10), None), mouth=(40, 20, 10), pattern="carved"), "Head", (0, 18, 0))
golem.box((1, 10, 1), (-9, 12, 0), look("sgarm", ["#3a2614", "#5a3a1c"]), "Arm(0)", (-4, 16, 0))
golem.box((1, 10, 1), (8, 12, 0), look("sgarm", ["#3a2614", "#5a3a1c"]), "Arm(1)", (4, 16, 0))
new("snow_golem", "Golem de Neve", golem, health=4, speed=2.0, size=(0.7, 1.9), spawn="Never", drops=[("snowball", 0, 15)], egg=((240, 248, 252), (129, 166, 166)))
fish_cols = [("#e86a1a", "#f8f4f0"), ("#2a7ad8", "#f4e04a"), ("#d83a8a", "#f4f4f4"), ("#3ab858", "#f8d040"), ("#f4c020", "#3a3a8a")]
tf = Model()
tf.box((2, 6, 6), (-1, 0, -3), look("tfbody", ["#c85a10", "#e86a1a", "#f8a050", "#f8f4f0"], eyes=(EYE, None), pattern="rings"))
tf.box((1, 4, 3), (-1, 1, -6), look("tftail", ["#c85a10", "#e86a1a", "#f8f4f0"]))
tf.box((1, 2, 3), (-1, 6, -1), look("tffin", ["#c85a10", "#f8f4f0"]))
new("tropical_fish", "Peixe Tropical", tf, health=3, speed=2.0, size=(0.5, 0.4), spawn="Water", aquatic=True, flee_range=4.0, group=(3, 6), weight=8,
    biomes=["Ocean"], drops=[("tropical_fish", 1, 1)], egg=((239, 105, 21), (255, 249, 239)))
tad = Model()
tad.box((3, 2, 3), (-1, 0, 0), look("tdhead", ["#2a1c10", "#3e2a18", "#52381e"], eyes=(EYE, None)))
tad.box((1, 2, 4), (0, 0, -4), look("tdtail", ["#2a1c10", "#3e2a18"]))
new("tadpole", "Girino", tad, health=6, speed=1.6, size=(0.4, 0.3), spawn="Water", aquatic=True, flee_range=3.0, group=(2, 5), weight=6,
    biomes=["Swamp", "Mangrove Swamp"], egg=((109, 83, 63), (22, 16, 9)))
arm = Model()
shell = look("adshell", ["#7a4a3a", "#9a5e48", "#b8765a", "#d09070"], pattern="plate")
arm.box((7, 6, 10), (-3, 3, -5), shell)
arm.box((4, 4, 4), (-2, 3, 5), look("adhead", ["#8a5a48", "#a87058", "#c08870"], eyes=(EYE, None)), "Head", (0, 5, 5))
arm.box((1, 3, 1), (-2, 7, 7), look("adear", ["#a87058", "#c08870"]), "Head", (0, 5, 5))
arm.box((1, 3, 1), (1, 7, 7), look("adear", ["#a87058", "#c08870"]), "Head", (0, 5, 5))
arm.box((2, 3, 5), (-1, 3, -10), shell)
for k, (x, z) in enumerate([(-3, -4), (2, -4), (-3, 3), (2, 3)]):
    arm.box((2, 3, 2), (x, 0, z), look("adleg", ["#6a4436", "#8a5a48"]), f"Leg({(k + k // 2) % 2})", (x + 1, 3, z + 1))
new("armadillo", "Tatu", arm, health=12, speed=1.8, size=(0.7, 0.65), spawn="Grass", flee_range=5.0, group=(1, 2), weight=8,
    biomes=["Savanna", "Badlands"], ground=["grass_block", "red_sand", "coarse_dirt", "terracotta"], drops=[("armadillo_scute", 0, 1)],
    egg=((173, 113, 107), (130, 74, 66)))
allay = biped(look("alhead", ["#3a8ad8", "#5aa8f0", "#8ac8ff"], eyes=((30, 50, 120), None)), look("albody", ["#3a8ad8", "#5aa8f0", "#8ac8ff"], pattern="cloud"),
              look("alarm", ["#3a8ad8", "#5aa8f0"]), look("alleg", ["#2a70b8", "#3a8ad8"]), tall=4,
              wings=(1, 6, 6), wing_look=look("alwing", ["#bfe4ff", "#e4f4ff", "#ffffff"]))
new("allay", "Allay", allay, health=20, speed=3.0, size=(0.35, 0.6), spawn="Never", flies=True, hover=1.5, egg=((0, 218, 255), (0, 173, 255)))
vex = biped(look("vxhead", ["#7a8a9a", "#98a8b8", "#b8c8d8"], eyes=((220, 40, 40), "glow")), look("vxbody", ["#6a7a8a", "#8898a8"], pattern="cloth"),
            look("vxarm", ["#7a8a9a", "#98a8b8"]), look("vxleg", ["#5a6a7a", "#6a7a8a"]), tall=4,
            wings=(1, 6, 7), wing_look=look("vxwing", ["#c8d0d8", "#e0e6ec", "#f4f8fc"]))
new("vex", "Vex", vex, kind="Hostile", health=14, speed=4.0, size=(0.4, 0.8), spawn="Never", flies=True, hover=1.0, attack=9, xp=3,
    egg=((122, 144, 164), (232, 237, 241)))
rav = quadruped((12, 12, 18), (8, 8, 8), 12, 5,
                look("rvbody", ["#2e2a26", "#46403a", "#5e5650", "#7a7068"], pattern="plate"),
                look("rvhead", ["#2e2a26", "#46403a", "#5e5650"], eyes=((200, 40, 30), None), mouth=WHITE),
                look("rvleg", ["#262220", "#3a3430"]), snout=(4, 4, 3), snout_look=look("rvsnout", ["#3a3430", "#4e4640"]),
                ears=(2, 4, 2))
new("ravager", "Devastador", rav, kind="Hostile", health=100, speed=2.6, size=(1.95, 2.2), spawn="Never", attack=12, scale=1.6,
    drops=[("saddle", 1, 1)], xp=20, egg=((93, 89, 84), (71, 67, 64)))
cr = biped(look("crhead", ["#2a2420", "#3e3630", "#524840"], eyes=((240, 150, 40), "glow"), pattern="planks"),
           look("crbody", ["#26201c", "#3a322c", "#4e443a", "#6a5e50"], pattern="planks"), look("crarm", ["#2a2420", "#3e3630"], pattern="planks"),
           look("crleg", ["#221c18", "#36302a"], pattern="planks"), tall=16, slim=True)
new("creaking", "Rangedor", cr, kind="Hostile", health=1000, speed=3.0, size=(0.9, 2.7), spawn="Dark", attack=3, weight=2,
    biomes=["Dark Forest", "Pale Garden"], egg=((95, 95, 95), (252, 120, 0)))
stand = Model()
wood = look("aswood", ["#6a4a2a", "#8a6238", "#a87a48", "#c49660"], pattern="planks")
stand.box((2, 7, 2), (-1, 24, -1), wood, "Head", (0, 24, 0))
stand.box((12, 3, 3), (-6, 21, -1), wood)
stand.box((2, 9, 2), (-1, 12, -1), wood)
stand.box((8, 2, 2), (-4, 12, -1), wood)
stand.box((2, 11, 2), (-6, 11, -1), wood)
stand.box((2, 11, 2), (4, 11, -1), wood)
stand.box((2, 11, 2), (-3, 1, -1), wood)
stand.box((2, 11, 2), (1, 1, -1), wood)
stand.box((12, 1, 12), (-6, 0, -6), look("asbase", ["#7a7a7a", "#949494", "#acacac"], pattern="plate"))
new("armor_stand", "Suporte de Armadura", stand, health=1, speed=0.0, size=(0.5, 1.975), spawn="Never", drops=[("armor_stand", 1, 1)])

# -------------------------------------------------------------------------------- write


def main():
    names = []
    text = START
    for e in ENTRIES:
        text += e
    for m in NEW:
        text += gdm.ron(m)
    text += END
    body = SRC_TEXT.rstrip()
    assert body.endswith("])")
    with open(MOBS_RON, "w", encoding="utf-8") as f:
        f.write(body[:-2].rstrip() + "\n" + text + "])\n")
    for n, img in SKINS.items():
        img.save(os.path.join(ENTITY, n + ".png"))
        names.append(n)
    for m in NEW:
        gdm.render(m["name"], m["model"]).save(os.path.join(ENTITY, m["name"] + ".png"))
        names.append(m["name"])
    # Spawn eggs, drawn like tools/gen_eggs.py does.
    rows = [(6, 9), (5, 10), (4, 11), (4, 11), (3, 12), (3, 12), (3, 12), (2, 13), (2, 13), (2, 13), (2, 13), (3, 12), (3, 12), (4, 11)]
    spots = [(6, 4), (9, 6), (5, 8), (10, 10), (7, 11), (4, 11), (11, 7), (8, 8)]
    items = open(ITEMS_RON, encoding="utf-8").read()
    add = []
    displays = {e["name"]: e["display"] for e in NEW}
    for e in ENTRIES:
        mm = re.search(r'name: "([a-z_]+)", display_name: "([^"]+)"', e)
        displays[mm.group(1)] = mm.group(2)
    for name, (base, spot) in EGGS.items():
        img = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
        for i, (a, b) in enumerate(rows):
            y = i + 1
            for x in range(a, b + 1):
                edge = x in (a, b) or i in (0, len(rows) - 1)
                shade = 0.6 if edge else (1.1 if x < 7 and y < 8 else 1.0)
                img.putpixel((x, y), tuple(min(255, int(v * shade)) for v in base) + (255,))
        for x, y in spots:
            img.putpixel((x, y), spot + (255,))
            img.putpixel((x + 1, y), spot + (255,))
        img.save(os.path.join(ITEM_TEX, f"{name}_spawn_egg.png"))
        names.append(f"{name}_spawn_egg")
        if f'"{name}_spawn_egg"' not in items:
            add.append(f'    (name: "{name}_spawn_egg", display_name: Some("Ovo de {displays[name]}"), texture: Some("{name}_spawn_egg")),\n')
    if add:
        i = items.rindex("]")
        items = items[:i] + "    // Lote 2 spawn eggs (tools/gen_lote2_mobs.py).\n" + "".join(add) + items[i:]
        with open(ITEMS_RON, "w", encoding="utf-8") as f:
            f.write(items)
    print(",".join(names))


if __name__ == "__main__":
    main()
