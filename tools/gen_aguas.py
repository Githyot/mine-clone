#!/usr/bin/env python3
"""Life for the lakes, rivers, swamps, seas and cave lakes (thread "Povoar lagos, rios e mares").

Writes packs/vanilla/aguas/mobs.ron (the creatures, with the kind of water each lives in), their
64x128 skins in packs/vanilla/textures/entity/, the 16x16 designs of the water plants of
packs/vanilla/aguas/blocks.ron in .../textures/block/, the shark fin and the spawn eggs in
.../textures/item/ (and their entries in packs/vanilla/items.ron).

Box models are built here with the body plans of tools/dim_mobs_spec.py; skins are painted here
with countershading (dark backs, pale bellies), scales, stripes and side eyes. Only the ideas of
Minecraft, Terraria and the mods (Aquaculture, Alex's Mobs, Naturalist) are used: every shape
and pixel is ours.

    python3 tools/gen_aguas.py && python3 tools/tex64/build.py --only <the names it prints>
"""

import math
import os
import re
import sys
import zlib

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_dim_mobs as gdm  # noqa: E402
from dim_mobs_spec import EYE, WHITE, Model, biped, look, worm  # noqa: E402
from gen_mob_skins import face_rects, fbm, hash01, hexrgb, local_pos, mix, mul  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_RON = os.path.join(ROOT, "packs", "vanilla", "aguas", "mobs.ron")
ITEMS_RON = os.path.join(ROOT, "packs", "vanilla", "items.ron")
ENTITY = os.path.join(ROOT, "packs", "vanilla", "textures", "entity")
BLOCK_TEX = os.path.join(ROOT, "packs", "vanilla", "textures", "block")
ITEM_TEX = os.path.join(ROOT, "packs", "vanilla", "textures", "item")

MOBS = []
EGGS = {}


def mob(name, display, model, egg, kind="Passive", health=6, speed=2.0, size=(0.5, 0.4), spawn="Water", **kw):
    model.pack()
    MOBS.append(dict(name=name, display=display, model=model, kind=kind, health=health, speed=speed, size=size, spawn=spawn, kw=kw))
    EGGS[name] = egg


def fl(key, pal, pattern="scales", belly=None, eye=EYE, stripes=None, spots=None, glow=None):
    """A look for water creatures: `pal` dark (back) to light (flanks), `belly` for the
    underside, `eye` painted on both sides near the front (None for blind fish)."""
    d = look(key, pal, pattern=pattern, belly=belly)
    d.update(side_eye=eye, stripes=stripes, spots=spots, glow_spots=glow)
    return d


def fish(body, body_look, fin_look, tail=4, head=None, head_look=None, dorsal=True, pect=True, stem=True, ventral=False, whiskers=None):
    """A fish: a body box, a tapered head in front, a tail in two parts (stem, then the fin)
    that the animation swishes, pectoral fins (Wing) that paddle, a dorsal fin."""
    m = Model()
    w, h, l = body
    m.box(body, (-(w // 2), 0, -(l // 2)), body_look)
    front = l - l // 2
    if head:
        hw, hh, hl = head
        m.box(head, (-(hw // 2), (h - hh) // 2, front), head_look or body_look)
    tw = 1 if w % 2 else 2
    back = -(l // 2)
    if stem:
        sh = max(1, h - 2 if h > 3 else h - 1)
        m.box((tw, sh, 2), (-(tw // 2), (h - sh) // 2, back - 2), body_look)
        back -= 2
    fh = h + 2 if h >= 3 else h + 1
    m.box((tw if tw == 1 else 1, fh, tail), (0 if tw == 2 else 0, (h - fh) // 2, back - tail), fin_look)
    if dorsal:
        dl = max(2, l // 3)
        m.box((1, 2 if h < 6 else 3, dl), (0 if w % 2 else -1 + 1, h, -(dl // 2)), fin_look)
    if pect:
        pw = 2 if w >= 3 else 1
        y = max(0, h // 3)
        z = l // 4 - 1
        m.box((pw, 1, 2), (-(w // 2) - pw, y, z), fin_look, "Wing(0)", (-(w // 2), y, z + 1))
        m.box((pw, 1, 2), (w - w // 2, y, z), fin_look, "Wing(1)", (w - w // 2, y, z + 1))
    if ventral:
        m.box((1, 2, 2), (0 if w % 2 else 0, -2, l // 6), fin_look)
    if whiskers:
        x0 = head[0] // 2 if head else w // 2
        z0 = front + (head[2] if head else 0) - 1
        for sx in (-1, 1):
            m.box((1, 1, 3), (sx * x0 - (1 if sx < 0 else 0), 0, z0 - 1), whiskers)
    return m


def shelled(shell, shell_look, skin_look, head=(4, 3, 4), legs=(2, 1, 3), flippers=False, tail=True, high=1):
    """Turtles: a shell over a flat body, a head out in front, four paddles."""
    m = Model()
    w, h, l = shell
    m.box((w - 2, 2, l - 2), (-(w - 2) // 2, high, -(l - 2) // 2), skin_look)
    m.box(shell, (-(w // 2), high + 1, -(l // 2)), shell_look)
    hw, hh, hl = head
    m.box(head, (-(hw // 2), high, l - l // 2 - 1), skin_look, "Head", (0, high + 1, l - l // 2 - 1))
    lw, lh, ll = legs
    if flippers:
        # Long front flippers that row (Wing), short back paddles (Leg).
        for k, sx in enumerate((-1, 1)):
            ox = -(w // 2) - 5 if sx < 0 else w - w // 2
            m.box((5, 1, 3), (ox, high + 1, l // 4 - 1), skin_look, f"Wing({k})", (ox + (5 if sx < 0 else 0), high + 1, l // 4))
        for k, sx in enumerate((-1, 1)):
            ox = -(w // 2) - 2 if sx < 0 else w - w // 2
            m.box((2, 1, 3), (ox, high, -(l // 2) + 1), skin_look, f"Leg({k})", (ox + 1, high + 1, -(l // 2) + 2))
    else:
        for k, (sx, sz) in enumerate([(-1, 1), (1, 1), (-1, -1), (1, -1)]):
            ox = -(w // 2) - lw + 1 if sx < 0 else w - w // 2 - 1
            oz = l // 2 - ll - 1 if sz > 0 else -(l // 2) + 1
            swing = (k + k // 2) % 2
            m.box((lw, high + 1, ll), (ox, 0, oz), skin_look, f"Leg({swing})", (ox + lw // 2, high + 1, oz + ll // 2))
    if tail:
        m.box((1, 1, 2), (0, high, -(l // 2) - 1), skin_look)
    return m


# ------------------------------------------------------------------------------- palettes
SILVER = ["#3e4a52", "#5e6c74", "#8a979c", "#b8c2c4", "#dfe4e2"]
WHITE_BELLY = ["#d8dcd6", "#eceee8"]

# ================================================================================ LAKES
carp_l = fl("carp", ["#4a3a14", "#6e5520", "#93752e", "#b89442", "#d4b264"], belly=["#d8c48a", "#e8d8a4"])
carp_f = fl("carpfin", ["#5a3a18", "#7a5222", "#9a6a30"], pattern="fin", eye=None)
mob("carp", "Carpa", fish((4, 5, 9), carp_l, carp_f, tail=4, head=(3, 4, 2), whiskers=look("carpwh", ["#6a5020", "#8a6a2a"])),
    ((180, 140, 60), (110, 80, 30)), health=5, speed=1.7, size=(0.45, 0.4), group=(1, 3), weight=8, aquatic=True, flee_range=4.0,
    waters=["lake", "swamp"], drops=[("carp", 1, 1)], xp=1, motion='(style: Some("swimmer"), heavy: Some(0.2))')

blue_l = fl("bluegill", ["#1e3a3e", "#2a5558", "#3c7470", "#5a9488", "#80b49e"], belly=["#e88a2a", "#f4a848"], stripes=("v", ["#1a3034", "#24403f"]))
blue_f = fl("bluegillfin", ["#2a4a4c", "#3c6260", "#4e7a72"], pattern="fin", eye=None)
mob("bluegill", "Peixe-Sol", fish((2, 5, 5), blue_l, blue_f, tail=3, head=(1, 4, 1)),
    ((60, 116, 112), (232, 138, 42)), health=3, speed=2.0, size=(0.35, 0.4), group=(4, 7), weight=10, aquatic=True, school=True, flee_range=4.0,
    waters=["lake"], drops=[("bluegill", 1, 1)], xp=1)

perch_l = fl("perch", ["#2e3a14", "#465a1e", "#64782a", "#8a9a3c", "#b4bc5c"], belly=["#e8e0b0", "#f2ecc4"], stripes=("v", ["#1e2a10", "#2a3a14"]))
perch_f = fl("perchfin", ["#a83a1a", "#c85024", "#e06a30"], pattern="fin", eye=None)
mob("perch", "Perca", fish((2, 4, 7), perch_l, perch_f, tail=3, head=(1, 3, 1)),
    ((100, 120, 42), (200, 80, 36)), health=3, speed=2.1, size=(0.35, 0.35), group=(3, 6), weight=8, aquatic=True, school=True, flee_range=4.0,
    waters=["lake", "river"], drops=[("perch", 1, 1)], xp=1)

turtle_shell = fl("pturtshell", ["#2a2e18", "#3e4222", "#565a2e", "#70723c", "#8c8a4c"], pattern="hex", eye=None)
turtle_skin = fl("pturtskin", ["#2e3a1e", "#44542a", "#5e7038", "#7a8c4a"], pattern="skin", belly=["#c8b878", "#d8c88a"], stripes=("h", ["#d8b830", "#e8c840"]))
mob("pond_turtle", "Tágado", shelled((8, 3, 9), turtle_shell, turtle_skin, head=(3, 3, 3)),
    ((86, 90, 46), (216, 184, 48)), health=8, speed=1.2, size=(0.55, 0.35), group=(1, 2), weight=5, aquatic=True, bottom=True, flee_range=3.0,
    waters=["lake", "swamp", "river"], drops=[("turtle_scute", 0, 1, 0.2)], xp=1, motion='(style: Some("swimmer"), heavy: Some(0.4))')

# The swan: a white bird gliding on the lakes, long neck held up.
swan = Model()
swan_w = look("swanbody", ["#c8ccc8", "#dcdfdc", "#eceeea", "#f8f9f6"], pattern="feathers")
swan.box((6, 5, 10), (-3, 2, -5), swan_w)
swan.box((2, 9, 2), (-1, 6, 4), swan_w, "Head", (0, 6, 5))
swan.box((3, 3, 4), (-2, 14, 3), look("swanhead", ["#dcdfdc", "#f2f4f0"], eyes=(EYE, None)), "Head", (0, 6, 5))
swan.box((1, 2, 3), (-1, 14, 7), look("swanbeak", ["#c85a14", "#e87a22", "#202020"]), "Head", (0, 6, 5))
swan.box((1, 4, 8), (-4, 4, -4), swan_w, "Wing(0)", (-3, 7, 0))
swan.box((1, 4, 8), (3, 4, -4), swan_w, "Wing(1)", (3, 7, 0))
swan.box((4, 2, 3), (-2, 5, -8), swan_w)
swan.box((1, 2, 1), (-2, 0, 0), look("swanleg", ["#1e1e1e", "#2e2e2e"]), "Leg(0)", (-2, 2, 0))
swan.box((1, 2, 1), (1, 0, 0), look("swanleg", ["#1e1e1e", "#2e2e2e"]), "Leg(1)", (1, 2, 0))
mob("swan", "Cisne", swan, ((240, 242, 238), (232, 122, 34)), health=8, speed=1.6, size=(0.6, 1.0), swims=True, group=(1, 2), weight=4, flee_range=5.0,
    waters=["lake", "river"], drops=[("feather", 0, 2)], xp=1)

# ================================================================================ RIVERS
salmon_l = fl("salmon", ["#3a2a2e", "#5a4048", "#8a6670", "#c898a0", "#e8c0c0"], belly=["#e8dcd8", "#f4ece8"], spots=["#2a1e22"])
salmon_f = fl("salmonfin", ["#4a3036", "#6a4650", "#8a5e66"], pattern="fin", eye=None)
mob("salmon", "Salmão", fish((3, 4, 10), salmon_l, salmon_f, tail=4, head=(2, 3, 2)),
    ((160, 60, 60), (70, 150, 120)), health=4, speed=2.6, size=(0.45, 0.4), group=(3, 6), weight=10, aquatic=True, school=True, flee_range=4.0,
    waters=["river", "cold_ocean"], drops=[("salmon", 1, 1)], xp=1)

trout_l = fl("trout", ["#2e3a26", "#46563a", "#687a54", "#98a87a", "#c4cca0"], belly=["#e8e0c8", "#f4ecd8"], spots=["#1e2418"], stripes=("h", ["#d47878", "#e49090"]))
trout_f = fl("troutfin", ["#3a4230", "#525c42", "#6a7454"], pattern="fin", eye=None)
mob("trout", "Truta", fish((3, 4, 9), trout_l, trout_f, tail=3, head=(2, 3, 2)),
    ((104, 122, 84), (212, 120, 120)), health=4, speed=2.4, size=(0.45, 0.4), group=(1, 3), weight=9, aquatic=True, flee_range=4.0,
    waters=["river", "lake"], drops=[("trout", 1, 1)], xp=1)

# The otter: a long, low swimmer with a thick tail; floats on its back and paddles about.
otter = Model()
ofur = look("otterfur", ["#2e1e12", "#46301e", "#5e422a", "#76583a"], pattern="wool", belly=["#a88a6a", "#b89a7a"])
otter.box((5, 4, 12), (-2, 2, -6), ofur)
otter.box((4, 4, 4), (-2, 3, 6), look("otterhead", ["#3a2818", "#523a24", "#6a4e32"], eyes=(EYE, None)), "Head", (0, 5, 6))
otter.box((2, 1, 1), (-1, 3, 10), look("otternose", ["#1a1410", "#2a2018"]), "Head", (0, 5, 6))
otter.box((1, 1, 1), (-2, 7, 7), ofur, "Head", (0, 5, 6))
otter.box((1, 1, 1), (1, 7, 7), ofur, "Head", (0, 5, 6))
otter.box((2, 2, 7), (-1, 2, -13), ofur)
for k, (x, z) in enumerate([(-2, 3), (2, 3), (-2, -5), (2, -5)]):
    otter.box((2, 2, 2), (x - 1 if x < 0 else x - 1, 0, z), look("otterleg", ["#2a1c10", "#3e2a1a"]), f"Leg({(k + k // 2) % 2})", (x, 2, z + 1))
mob("otter", "Lontra", otter, ((94, 66, 42), (168, 138, 106)), health=10, speed=2.6, size=(0.55, 0.45), swims=True, group=(1, 3), weight=5, flee_range=5.0,
    waters=["river", "lake"], prey=["salmon", "trout", "carp", "perch"], xp=1, motion='(style: Some("quadruped"), wag: Some(0.4), bounce: Some(1.2))')

# Crayfish: a little armoured walker on the river bed, claws up.
cray = Model()
cshell = look("crayshell", ["#4a1a10", "#6a2616", "#8a361e", "#a84a2a"], pattern="plate")
cray.box((4, 2, 5), (-2, 1, -1), cshell)
cray.box((2, 2, 4), (-1, 1, -5), cshell)
cray.box((3, 1, 2), (-1, 1, -7), look("craytail", ["#6a2616", "#8a361e"]))
cray.box((1, 1, 3), (-3, 1, 4), cshell, "Arm(0)", (-2, 2, 4))
cray.box((1, 1, 3), (2, 1, 4), cshell, "Arm(1)", (2, 2, 4))
cray.box((2, 1, 2), (-4, 1, 6), look("crayclaw", ["#5a1e12", "#8a2e1a", "#b44a2a"]), "Arm(0)", (-2, 2, 4))
cray.box((2, 1, 2), (2, 1, 6), look("crayclaw", ["#5a1e12", "#8a2e1a", "#b44a2a"]), "Arm(1)", (2, 2, 4))
for k in range(6):
    side = 0 if k < 3 else 1
    z = 2 - (k % 3) * 2
    x = -3 if side == 0 else 2
    cray.box((1, 1, 1), (x, 0, z), look("crayleg", ["#4a1a10", "#6a2616"]), f"SpiderLeg({k % 3 + side * 4})", (x if side else x + 1, 1, z))
mob("crayfish", "Lagostim", cray, ((138, 54, 30), (60, 26, 16)), kind="Neutral", health=4, speed=1.4, size=(0.4, 0.25), group=(1, 3), weight=6,
    aquatic=True, bottom=True, attack=1.0, waters=["river", "lake", "swamp", "cave"], xp=1, motion='(style: Some("spider"))')

pir_l = fl("piranha", ["#3a3e42", "#5a6066", "#808890", "#a8b0b4", "#c8ced0"], belly=["#c8281e", "#e84030"], spots=["#4a5056"])
pir_f = fl("piranhafin", ["#3a2a28", "#5a3e3a", "#a02a20"], pattern="fin", eye=None)
pir = fish((2, 5, 5), pir_l, pir_f, tail=3, head=(2, 4, 2))
pir.box((2, 1, 1), (-1, 0, 4), look("piranhateeth", ["#e8e4d8", "#f8f4ea"]))
mob("piranha", "Piranha", pir, ((200, 40, 30), (120, 128, 134)), kind="Hostile", health=6, speed=3.0, size=(0.35, 0.4), group=(3, 6), weight=10,
    aquatic=True, school=True, attack=2.0, waters=["jungle_river"], drops=[("bone", 0, 1)], xp=2)

ara_l = fl("arapaima", ["#1a2420", "#2a3a32", "#3e5446", "#5a6e58", "#7a8a6a"], belly=["#8a4a3a", "#a85848"], spots=["#a83a2a"])
ara_f = fl("arapaimafin", ["#6a1a14", "#9a2a1e", "#c43a28"], pattern="fin", eye=None)
mob("arapaima", "Pirarucu", fish((5, 6, 16), ara_l, ara_f, tail=5, head=(4, 5, 3)),
    ((42, 58, 50), (196, 58, 40)), kind="Hostile", health=30, speed=2.4, size=(0.9, 0.6), group=(1, 1), weight=4, scale=1.3,
    aquatic=True, attack=5.0, waters=["jungle_river"], drops=[("gar", 1, 2)], xp=6, motion='(style: Some("swimmer"), heavy: Some(0.5))')

# The caiman: lies at the surface like a log, then comes for you.
cai = Model()
cskin = look("caimanskin", ["#1e2216", "#2e3420", "#42482c", "#585e3a"], pattern="ridge", belly=["#a8a078", "#b8b088"])
cai.box((7, 4, 16), (-3, 2, -8), cskin)
cai.box((5, 3, 5), (-2, 2, 8), look("caimanhead", ["#22261a", "#343a24", "#4a5030"], eyes=((200, 170, 40), None)), "Head", (0, 4, 8))
cai.box((3, 2, 6), (-1, 2, 13), cskin, "Head", (0, 4, 8))
cai.box((4, 3, 7), (-2, 2, -15), cskin)
cai.box((2, 2, 7), (-1, 2, -22), cskin)
for k, (x, z) in enumerate([(-5, 4), (3, 4), (-5, -6), (3, -6)]):
    cai.box((2, 2, 3), (x, 0, z), look("caimanleg", ["#1a1e14", "#2a301e"]), f"Leg({(k + k // 2) % 2})", (x + 1, 2, z + 1))
mob("caiman", "Jacaré", cai, ((52, 58, 34), (168, 160, 120)), kind="Hostile", health=24, speed=2.0, size=(0.9, 0.45), group=(1, 1), weight=4,
    swims=True, attack=5.0, waters=["river", "jungle_river", "swamp"], drops=[("leather", 0, 2)], xp=5, motion='(style: Some("quadruped"), heavy: Some(0.6), wag: Some(0.2))')

# ================================================================================ SEAS
cod_l = fl("cod", ["#3a3424", "#5a5036", "#7e7450", "#a89c74", "#d0c6a2"], belly=["#e8e2d0", "#f2eee2"], spots=["#4a4230"])
cod_f = fl("codfin", ["#4a4232", "#625a44", "#7a7058"], pattern="fin", eye=None)
mob("cod", "Bacalhau", fish((3, 4, 8), cod_l, cod_f, tail=3, head=(2, 3, 2), whiskers=look("codwh", ["#8a7e5a", "#a89c74"])),
    ((180, 166, 120), (90, 80, 56)), health=3, speed=2.2, size=(0.4, 0.35), group=(4, 8), weight=12, aquatic=True, school=True, flee_range=4.0,
    waters=["ocean", "cold_ocean"], drops=[("cod", 1, 1)], xp=1)

puff = Model()
pskin = fl("puffer", ["#6a5418", "#9a7c22", "#c8a432", "#e4c650", "#f4e08a"], pattern="spots", belly=["#f0ecd8", "#faf8ec"], spots=["#4a3a12"])
puff.box((6, 6, 6), (-3, 0, -3), pskin)
puff.box((1, 3, 2), (0, 2, -5), fl("pufffin", ["#8a6a1a", "#b48e2a"], pattern="fin", eye=None))
for (x, y, z) in [(-4, 3, 0), (3, 3, 0), (0, 6, 0), (0, 3, 3), (-4, 1, -2), (3, 1, -2), (-2, 6, -2), (1, 6, 2)]:
    puff.box((1, 1, 1), (x if x < 0 else x, y, z), look("puffspike", ["#e8dcb0", "#f8f0d0"]))
mob("pufferfish", "Baiacu", puff, ((246, 178, 0), (55, 194, 238)), kind="Neutral", health=3, speed=1.2, size=(0.45, 0.45), group=(1, 3), weight=5,
    aquatic=True, attack=2.0, waters=["warm_ocean", "ocean"], drops=[("pufferfish", 1, 1)], xp=1)

sea_shell = fl("seashell", ["#2a3018", "#3e4824", "#566232", "#727e44", "#90985a"], pattern="hex", eye=None)
sea_skin = fl("seaskin", ["#3a4a3a", "#566a52", "#748a6c", "#94a888"], pattern="scutes", belly=["#d8d4b0", "#e8e4c4"])
mob("sea_turtle", "Tartaruga-Marinha", shelled((12, 4, 14), sea_shell, sea_skin, head=(4, 4, 4), flippers=True, tail=False),
    ((90, 110, 70), (210, 200, 160)), health=30, speed=1.6, size=(0.95, 0.45), group=(1, 2), weight=4, aquatic=True, flee_range=3.0,
    waters=["warm_ocean"], drops=[("turtle_scute", 0, 1, 0.5), ("seagrass", 0, 2)], xp=3, motion='(style: Some("swimmer"), heavy: Some(0.5))')

# Rays: flat, wide fins that ripple (Wing).
def ray(body, wing, back_look, fin_look, tail_len, sting=False):
    m = Model()
    w, h, l = body
    m.box(body, (-(w // 2), 0, -(l // 2)), back_look)
    for k, sx in enumerate((-1, 1)):
        ox = -(w // 2) - wing[0] if sx < 0 else w - w // 2
        m.box(wing, (ox, 0, -(wing[2] // 2)), back_look, f"Wing({k})", (ox + (wing[0] if sx < 0 else 0), 1, 0))
    m.box((1, 1, tail_len), (0, 0, -(l // 2) - tail_len), fin_look)
    if sting:
        m.box((1, 1, 2), (0, 1, -(l // 2) - tail_len // 2), look("sting", ["#d8d0b0", "#e8e0c4"]))
    return m


manta_back = fl("manta", ["#14181e", "#1e242c", "#2a323c", "#3a444e", "#4c5862"], pattern="skin", belly=["#e8ecea", "#f6f8f6"], eye=(30, 30, 30))
manta = ray((6, 2, 10), (9, 1, 8), manta_back, manta_back, 6)
manta.box((1, 2, 3), (-3, 0, 5), manta_back)
manta.box((1, 2, 3), (2, 0, 5), manta_back)
mob("manta_ray", "Raia-Manta", manta, ((36, 44, 54), (240, 244, 242)), health=24, speed=1.8, size=(1.2, 0.3), group=(1, 2), weight=3, scale=1.5,
    aquatic=True, waters=["warm_ocean", "deep_ocean"], xp=3)

sting_back = fl("stingray", ["#3a3020", "#544630", "#706044", "#8c7c5a", "#a89872"], pattern="spots", belly=["#e8e0d0", "#f4ece0"], spots=["#2a2014", "#c8b890"])
mob("stingray", "Arraia", ray((5, 2, 7), (4, 1, 6), sting_back, sting_back, 7, sting=True),
    ((140, 124, 90), (42, 32, 20)), kind="Neutral", health=10, speed=1.6, size=(0.75, 0.2), group=(1, 2), weight=5, aquatic=True, bottom=True, attack=3.0,
    waters=["warm_ocean", "ocean", "jungle_river"], xp=2)

# Jellyfish: a soft bell and trailing tentacles (Leg: the animation makes them pulse).
jelly = Model()
jbell = fl("jellybell", ["#7a3a8a", "#9a52aa", "#b870c4", "#d494d8", "#ecc0ec"], pattern="jelly", eye=None, glow=True)
jelly.box((8, 5, 8), (-4, 7, -4), jbell)
jelly.box((6, 1, 6), (-3, 6, -3), fl("jellyrim", ["#c890d0", "#e0b4e4"], pattern="jelly", eye=None))
for k, (x, z) in enumerate([(-3, -3), (2, -3), (-3, 2), (2, 2), (0, -1), (-1, 0)]):
    jelly.box((1, 7, 1), (x, 0, z), look("jellytent", ["#b870c4", "#d494d8", "#f0d0f0"]), f"Leg({k % 2})", (x, 7, z))
mob("jellyfish", "Água-Viva", jelly, ((200, 120, 210), (240, 200, 240)), kind="Hostile", health=8, speed=0.9, size=(0.5, 0.75), group=(2, 4), weight=7,
    aquatic=True, attack=2.0, waters=["warm_ocean", "ocean", "deep_ocean"], drops=[("slime_ball", 0, 1)], xp=2, motion='(style: Some("floater"))')

shark_l = fl("shark", ["#2a323a", "#3e4852", "#56626c", "#707c86", "#8c98a0"], pattern="skin", belly=["#e4e8e6", "#f2f4f2"])
shark_f = fl("sharkfin", ["#2a323a", "#3e4852", "#525e68"], pattern="skin", eye=None)
shark = Model()
shark.box((6, 6, 16), (-3, 0, -8), shark_l)
shark.box((5, 5, 5), (-2, 0, 8), fl("sharkhead", ["#2a323a", "#3e4852", "#56626c", "#707c86"], pattern="skin", belly=["#e4e8e6", "#f2f4f2"]), "Head", (0, 3, 8))
shark.box((3, 1, 2), (-1, 0, 11), look("sharkteeth", ["#e8e4dc", "#faf8f2"]), "Head", (0, 3, 8))
shark.box((4, 4, 5), (-2, 1, -13), shark_l)
shark.box((2, 3, 4), (-1, 2, -17), shark_l)
shark.box((1, 9, 4), (0, -1, -21), shark_f)
shark.box((1, 5, 5), (0, 6, -2), shark_f)
shark.box((5, 1, 4), (-8, 1, 2), shark_f, "Wing(0)", (-3, 1, 4))
shark.box((5, 1, 4), (3, 1, 2), shark_f, "Wing(1)", (3, 1, 4))
mob("shark", "Tubarão", shark, ((70, 82, 92), (230, 234, 232)), kind="Hostile", health=30, speed=3.2, size=(1.0, 0.75), group=(1, 1), weight=4, scale=1.3,
    aquatic=True, attack=6.0, waters=["ocean", "warm_ocean", "deep_ocean"], drops=[("shark_fin", 1, 1), ("cod", 0, 2)], xp=8, motion='(style: Some("swimmer"), heavy: Some(0.4))')

# Octopus: a round mantle on eight arms that sweep the sea floor.
octo = Model()
oskin = fl("octopus", ["#5a1e14", "#7e2c1c", "#a43e28", "#c45a3a", "#dc7c56"], pattern="spots", spots=["#e8a07a"], eye=(230, 200, 60))
octo.box((6, 6, 7), (-3, 4, -4), oskin)
octo.box((5, 2, 5), (-2, 2, -2), oskin)
for k in range(8):
    a = k / 8 * math.tau
    x, z = round(math.cos(a) * 2.6), round(math.sin(a) * 2.6)
    octo.box((1, 4, 1), (x, 0, z), look("octoarm", ["#7e2c1c", "#a43e28", "#e8b090"]), f"Leg({k % 2})", (x, 3, z))
mob("octopus", "Polvo", octo, ((164, 62, 40), (232, 160, 122)), kind="Neutral", health=12, speed=1.4, size=(0.5, 0.6), group=(1, 1), weight=4,
    aquatic=True, bottom=True, attack=2.0, waters=["warm_ocean", "ocean"], drops=[("ink_sac", 1, 2)], xp=2)

# The blue whale: far offshore only, huge and slow.
whale = Model()
wskin = fl("bluewhale", ["#1e2a3a", "#2a3a4e", "#3a4e66", "#506680", "#6a809a"], pattern="mottle", belly=["#a8b4c0", "#bcc6d0"], spots=["#7a90a8"])
whale.box((10, 9, 22), (-5, 0, -11), wskin)
whale.box((8, 7, 8), (-4, 0, 11), wskin, "Head", (0, 4, 11))
whale.box((6, 6, 8), (-3, 1, -19), wskin)
whale.box((4, 4, 6), (-2, 2, -25), wskin)
whale.box((14, 1, 5), (-7, 3, -30), wskin)
whale.box((8, 1, 5), (-13, 2, 4), wskin, "Wing(0)", (-5, 2, 6))
whale.box((8, 1, 5), (5, 2, 4), wskin, "Wing(1)", (5, 2, 6))
mob("blue_whale", "Baleia-Azul", whale, ((42, 58, 78), (168, 180, 192)), health=200, speed=1.4, size=(2.4, 1.8), group=(1, 1), weight=1, scale=3.0,
    aquatic=True, waters=["deep_ocean"], xp=10, motion='(style: Some("swimmer"), heavy: Some(1.0))')

# The drowned: a sailor long lost, weed in the hair, walking the bottom at night.
dr = biped(look("drownedhead", ["#24403e", "#36605a", "#4c7c72", "#62968a"], eyes=((120, 220, 230), "glow"), pattern="cloth"),
           look("drownedbody", ["#2e3a44", "#3e4e5a", "#506470", "#627a84"], pattern="cloth"),
           look("drownedarm", ["#2e4a46", "#3e625c", "#4e7a70"]), look("drownedleg", ["#2a3440", "#3a4652"]), arms_forward=True)
dr.box((8, 3, 1), (-4, 21, 4), look("drownedweed", ["#1e4a1c", "#2a6024", "#367a2c"], pattern="leaves"), "Head", (0, 24, 0))
mob("drowned", "Afogado", dr, ((143, 241, 215), (121, 156, 101)), kind="Hostile", health=20, speed=1.6, size=(0.6, 1.95), group=(1, 1), weight=6,
    bottom=True, attack=3.0, burns_in_daylight=True, waters=["ocean", "river", "deep_ocean"],
    drops=[("rotten_flesh", 0, 2), ("copper_ingot", 0, 1, 0.11), ("nautilus_shell", 0, 1, 0.03)], xp=5)

# The crab (Terraria's beach crab): sideways on the sea floor, pinching.
crab = Model()
cshell = look("crabshell", ["#6a1a10", "#8e2616", "#b03a20", "#cc5430", "#e07448"], pattern="plate", eyes=(EYE, None))
crab.box((8, 3, 6), (-4, 2, -3), cshell)
crab.box((1, 2, 1), (-2, 5, 2), look("crabeye", ["#1a1410", "#2a2018"]))
crab.box((1, 2, 1), (1, 5, 2), look("crabeye", ["#1a1410", "#2a2018"]))
crab.box((3, 2, 3), (-7, 2, 2), look("crabclaw", ["#8e2616", "#b03a20", "#e8c8b0"]), "Arm(0)", (-4, 3, 2))
crab.box((3, 2, 3), (4, 2, 2), look("crabclaw", ["#8e2616", "#b03a20", "#e8c8b0"]), "Arm(1)", (4, 3, 2))
for k in range(6):
    side = 0 if k < 3 else 1
    z = 1 - (k % 3) * 2
    x = -6 if side == 0 else 4
    crab.box((2, 1, 1), (x, 1, z), look("crableg", ["#6a1a10", "#8e2616"]), f"SpiderLeg({k % 3 + side * 4})", (-4 if side == 0 else 4, 2, z))
mob("crab", "Caranguejo", crab, ((176, 58, 32), (232, 200, 176)), kind="Hostile", health=10, speed=1.8, size=(0.6, 0.35), group=(1, 3), weight=7,
    bottom=True, attack=2.0, waters=["warm_ocean", "ocean"], xp=2, motion='(style: Some("spider"))')

# Seahorse: upright, curled tail, snout.
sh = Model()
shl = fl("seahorse", ["#8a5a10", "#b07818", "#d09a2a", "#e8bc4a", "#f4d878"], pattern="rings", eye=EYE)
sh.box((2, 5, 3), (-1, 3, -1), shl)
sh.box((2, 3, 3), (-1, 8, 0), shl, "Head", (0, 8, 1))
sh.box((1, 1, 3), (0, 9, 3), shl, "Head", (0, 8, 1))
sh.box((1, 3, 2), (0, 0, -1), shl)
sh.box((1, 1, 2), (0, 0, -3), shl)
sh.box((1, 3, 1), (0, 4, -2), fl("seahorsefin", ["#d09a2a", "#f4d878"], pattern="fin", eye=None))
mob("seahorse", "Cavalo-Marinho", sh, ((232, 188, 74), (138, 90, 16)), health=3, speed=0.9, size=(0.25, 0.65), group=(1, 3), weight=4,
    aquatic=True, flee_range=3.0, waters=["warm_ocean"], xp=1, motion='(style: Some("floater"))')

# ================================================================================ SWAMPS
leech = worm(4, (2, 2, 3), look("leechhead", ["#1a1410", "#2a2018", "#3c2e22"], eyes=((200, 40, 30), None)), look("leechbody", ["#1e1a12", "#2e2a1c", "#423a26"], pattern="rings"))
mob("leech", "Sanguessuga", leech, ((46, 40, 28), (160, 40, 30)), kind="Hostile", health=4, speed=1.4, size=(0.3, 0.2), group=(1, 3), weight=7,
    aquatic=True, attack=1.0, waters=["swamp", "lake"], xp=1, motion='(style: Some("serpent"))')

cat_l = fl("catfish", ["#22201a", "#34302a", "#4a4438", "#62594a", "#7a705c"], pattern="skin", belly=["#c8c0a8", "#d8d0b8"], spots=["#1a1812"])
cat_f = fl("catfishfin", ["#2a2620", "#3e382e", "#524a3c"], pattern="fin", eye=None)
mob("catfish", "Bagre", fish((4, 3, 10), cat_l, cat_f, tail=3, head=(4, 3, 2), dorsal=True, whiskers=look("catwh", ["#3e382e", "#5a5242"])),
    ((74, 68, 56), (200, 192, 168)), health=6, speed=1.5, size=(0.45, 0.3), group=(1, 2), weight=8, aquatic=True, bottom=True, flee_range=3.0,
    waters=["swamp", "river", "lake"], drops=[("catfish", 1, 1)], xp=1)

# ================================================================================ CAVES
blind_l = fl("cavefish", ["#a88a8a", "#c0a2a0", "#d6bcb8", "#e8d4d0", "#f4e8e4"], pattern="scales", belly=["#f4e8e4", "#faf2f0"], eye=None)
blind_f = fl("cavefishfin", ["#d6bcb8", "#e8d4d0", "#f4ece8"], pattern="fin", eye=None)
mob("cavefish", "Peixe-Cego", fish((2, 3, 6), blind_l, blind_f, tail=3, head=(1, 2, 1)),
    ((236, 212, 208), (168, 138, 138)), health=3, speed=1.8, size=(0.3, 0.3), group=(3, 6), weight=10, aquatic=True, school=True, flee_range=3.0,
    waters=["cave"], xp=1)

eel_head = fl("eelhead", ["#101a1e", "#18282e", "#223840", "#2e4a54"], pattern="skin", eye=(120, 240, 220), glow=True)
eel_body = fl("eelbody", ["#101a1e", "#18282e", "#223840", "#2e4a54"], pattern="skin", eye=None, glow=True)
eel = Model()
eel.box((3, 3, 4), (-1, 0, 0), eel_head, "Head", (0, 1, 0))
eel.box((2, 1, 2), (-1, 0, 4), look("eeljaw", ["#2e4a54", "#e8f0ec"]), "Head", (0, 1, 0))
for k in range(1, 5):
    eel.box((2, 2, 4), (-1, 0, -4 * k), eel_body)
eel.box((1, 3, 3), (0, -1, -19), eel_body)
mob("cave_eel", "Enguia das Cavernas", eel, ((30, 50, 60), (120, 240, 220)), kind="Hostile", health=14, speed=2.2, size=(0.35, 0.3), group=(1, 1), weight=5,
    aquatic=True, attack=3.0, waters=["cave"], xp=4, motion='(style: Some("serpent"))')

# ------------------------------------------------------------------------------- painting


def shade(pal, t):
    return gdm.shade(pal, t)


def apaint(lk, face, i, j, fw, fh, size, g, seed):
    """One pixel: the base painter for land-style looks; water creatures get countershading,
    scales or skin, stripes, spots and eyes on the sides."""
    if "side_eye" not in lk:
        return gdm.paint(lk, face, i, j, fw, fh, g, seed)
    pal = [hexrgb(c) for c in lk["pal"]]
    w, h, d = size
    lx, ly, lz = local_pos(face, i, j, size)
    n = fbm(seed, g[0], g[1], g[2], 4.0)
    jit = hash01(seed, g[0] * 7, g[1] * 13, g[2] * 3) - 0.5
    pat = lk["pattern"]
    # Countershading: dark on the back, lighter down the flanks.
    up = ly / max(h, 1)
    t = 0.75 - 0.55 * up + 0.18 * (n - 0.5) + 0.12 * jit
    if face == "top":
        t = 0.12 + 0.2 * n
    if pat == "scales":
        # Rows of little scales: a darker crescent at each scale's back edge.
        u, v = (g[2] + (0.5 if int(g[1] + 64) % 2 else 0)) , g[1]
        if int(u * 1.0 + 64) % 2 == 0 and face in ("left", "right"):
            t -= 0.12
    elif pat == "fin":
        # Fin rays fanning back.
        t = 0.3 + 0.5 * n + (0.18 if (i + j) % 2 == 0 else -0.05)
    elif pat == "skin":
        t += 0.06 * jit
    elif pat == "hex":
        # Turtle shell plates: hexagon-ish cells with dark seams and growth rings.
        cx = (g[0] + 64) % 4
        cz = (g[2] + 64 + (2 if int((g[0] + 64) // 4) % 2 else 0)) % 4
        seam = cx < 0.6 or cz < 0.6
        t = 0.08 if seam else 0.45 + 0.35 * n - 0.15 * (abs(cx - 2) + abs(cz - 2)) / 2
    elif pat == "ridge":
        t = 0.2 + 0.45 * n + (0.25 if int(g[2] + 64) % 3 == 0 else 0.0) + (0.2 if face == "top" and abs(g[0]) < 1.5 else 0.0)
    elif pat == "jelly":
        t = 0.45 + 0.35 * n + (0.2 if (i * 3 + j) % 5 == 0 else 0.0) + 0.2 * up
    elif pat == "spots":
        t = 0.35 + 0.4 * n
    elif pat == "mottle":
        t = 0.3 + 0.4 * fbm(seed + 21, g[0], g[1], g[2], 2.0)
    elif pat == "rings":
        t = 0.75 if int(g[1] + 64) % 2 == 0 else 0.35 + 0.3 * n
    elif pat == "scutes":
        t = 0.3 + 0.5 * n - (0.25 if (i % 3 == 0 or j % 3 == 0) else 0.0)
    rgb = shade(pal, t)
    belly = lk.get("belly")
    if belly and (face == "bottom" or (face != "top" and up < 0.34 and pat not in ("fin", "hex"))):
        bp = [hexrgb(c) for c in belly]
        rgb = shade(bp, 0.4 + 0.5 * n)
    st = lk.get("stripes")
    if st and face in ("left", "right"):
        kind, sp = st
        sp = [hexrgb(c) for c in sp]
        if kind == "v" and int(g[2] + 64) % 3 == 0 and up > 0.25:
            rgb = shade(sp, 0.5 + jit)
        elif kind == "h" and 0.4 < up < 0.62:
            rgb = shade(sp, 0.5 + jit)
    sp = lk.get("spots")
    if sp and face in ("left", "right", "top") and hash01(seed + 3, int(g[0] + 64), int(g[1] + 64), int(g[2] + 64)) > 0.8:
        rgb = shade([hexrgb(c) for c in sp], 0.5)
    if lk.get("glow_spots") and hash01(seed + 9, int(g[0] + 64), int(g[1] + 64), int(g[2] + 64)) > 0.86:
        rgb = mix(rgb, (150, 250, 230), 0.75)
    eye = lk.get("side_eye")
    if eye and face in ("left", "right"):
        # Near the front, a little above the middle.
        ez = d - 1.5 if d > 2 else d - 0.5
        ey = h * 0.62
        if abs(lz - ez) < 0.6 and abs(ly - ey) < 0.6:
            rgb = eye
        elif abs(lz - ez) < 1.1 and abs(ly - ey) < 1.1 and d >= 3 and h >= 4:
            rgb = mix(rgb, (230, 226, 210), 0.6)
    return rgb


def render(name, model):
    img = Image.new("RGBA", (64, 128), (0, 0, 0, 0))
    px = img.load()
    seed = zlib.crc32(name.encode())
    done = set()
    for p in model.parts:
        if p["uv"] in done:
            continue
        done.add(p["uv"])
        for face, (x0, y0, fw, fh) in face_rects(p["size"], p["uv"]).items():
            for j in range(fh):
                for i in range(fw):
                    lx, ly, lz = local_pos(face, i, j, p["size"])
                    g = (p["origin"][0] + lx, p["origin"][1] + ly, p["origin"][2] + lz)
                    rgb = apaint(p["look"], face, i, j, fw, fh, p["size"], g, seed)
                    px[x0 + i, y0 + j] = (*rgb, 255)
    return img


# Small fish are drawn a third bigger than life, so a school reads from a few blocks away in
# the murk (the hitbox grows with them).
BIGGER = {"carp", "bluegill", "perch", "salmon", "trout", "cod", "piranha", "cavefish", "catfish", "seahorse", "pufferfish", "crayfish", "leech"}
for _m in MOBS:
    if _m["name"] in BIGGER and "scale" not in _m["kw"]:
        _m["kw"]["scale"] = 1.35
        _m["size"] = (round(_m["size"][0] * 1.35, 2), round(_m["size"][1] * 1.35, 2))


def ron(m):
    text = gdm.ron(m)
    # gdm writes drops with a chance as a 4th field; keep its format. Turn tuples like
    # `(1, 3)` (group) into RON tuples: already fine.
    return text


# ------------------------------------------------------------------------------- plants (16x16)
N = 16


def canvas():
    return np.zeros((N, N, 4), np.uint8)


def put(img, x, y, rgb, a=255):
    if 0 <= x < N and 0 <= y < N:
        img[y, x, :3] = rgb
        img[y, x, 3] = a


def ramp(pal, t):
    pal = [hexrgb(c) for c in pal]
    return shade(pal, max(0.0, min(0.999, t)))


def rng_for(name):
    return np.random.default_rng(zlib.crc32(name.encode()))


def cattail():
    """Tall flat blades, the lower part under water."""
    img = canvas()
    r = rng_for("cattail")
    for x0 in (3, 6, 8, 11, 13):
        lean = (r.random() - 0.5) * 0.25
        for y in range(16):
            x = int(round(x0 + lean * (15 - y)))
            put(img, x, y, ramp(["#2a4a1a", "#3a6224", "#4e7a2e", "#64923c"], 0.25 + 0.6 * (15 - y) / 15 + 0.1 * r.random()))
    return img


def cattail_top():
    """Blade tips and two brown velvet heads on their stalks."""
    img = canvas()
    r = rng_for("cattail_top")
    for x0 in (3, 8, 13):
        for y in range(6, 16):
            put(img, x0, y, ramp(["#3a6224", "#4e7a2e", "#64923c", "#7aa84a"], 0.4 + 0.5 * r.random()))
    for x0, top in ((5, 1), (10, 3)):
        put(img, x0, top - 1 if top > 0 else 0, hexrgb("#6a8a3a"))
        for y in range(top, top + 6):
            for x in (x0, x0 + 1):
                put(img, x, y, ramp(["#3a2210", "#5a3418", "#744420", "#8a562a"], 0.3 + 0.6 * r.random() * (1 if x == x0 else 0.7)))
        for y in range(top + 6, 16):
            put(img, x0, y, ramp(["#3a6224", "#4e7a2e", "#64923c"], 0.5))
    return img


def duckweed():
    """A floating carpet of tiny round leaves, gaps of water between."""
    img = canvas()
    r = rng_for("duckweed")
    for _ in range(40):
        x, y = int(r.random() * 15), int(r.random() * 15)
        c = ramp(["#3e6a1e", "#528428", "#68a034", "#84b844"], 0.3 + 0.7 * r.random())
        put(img, x, y, c)
        put(img, x + 1, y, c)
        put(img, x, y + 1, mul(c, 0.85))
        if r.random() < 0.5:
            put(img, x + 1, y + 1, mul(c, 1.1))
    return img


def water_lily():
    """A lily pad with a white-and-pink flower in the middle."""
    img = canvas()
    for y in range(N):
        for x in range(N):
            dx, dy = x - 7.5, y - 7.5
            d = math.hypot(dx, dy)
            a = math.atan2(dy, dx)
            if d < 7.4 and not (abs(a - 0.6) < 0.18 and d > 1.5):
                t = 0.35 + 0.35 * (1 - d / 7.4) + (0.15 if int(a * 6) % 2 else 0.0)
                put(img, x, y, ramp(["#1e4a16", "#2e6420", "#3e7e2a", "#56983a"], t))
    for k in range(8):
        a = k / 8 * math.tau
        for rr in (1.0, 2.0, 2.8):
            x, y = int(round(7.5 + math.cos(a) * rr)), int(round(7.5 + math.sin(a) * rr))
            put(img, x, y, ramp(["#d88aa8", "#ecb4c8", "#f8e4ec", "#ffffff"], 0.3 + rr / 3.5))
    put(img, 7, 7, hexrgb("#f4c840"))
    put(img, 8, 8, hexrgb("#e8b030"))
    put(img, 7, 8, hexrgb("#f8d860"))
    put(img, 8, 7, hexrgb("#e8b030"))
    return img


def pondweed():
    """Long ribbon leaves, swaying, a darker freshwater green than sea grass."""
    img = canvas()
    r = rng_for("pondweed")
    for x0 in (2, 5, 8, 10, 13):
        ph = r.random() * 6.28
        h = 10 + int(r.random() * 6)
        for i in range(h):
            y = 15 - i
            x = int(round(x0 + 1.6 * math.sin(ph + i * 0.45)))
            c = ramp(["#24361a", "#344e22", "#46662c", "#5c7e36", "#78964a"], 0.2 + 0.7 * i / h)
            put(img, x, y, c)
            if i % 3 == 1:
                put(img, x + (1 if math.sin(ph + i) > 0 else -1), y, mul(c, 1.1))
    return img


def sea_pickle():
    """Three or four glowing green-yellow pods in a clump."""
    img = canvas()
    for x0, h in ((4, 6), (7, 8), (10, 5), (12, 4)):
        for y in range(16 - h, 16):
            for x in (x0, x0 + 1):
                t = 0.35 + 0.5 * (y - (16 - h)) / h * 0 + (0.25 if x == x0 else 0.0)
                put(img, x, y, ramp(["#3a5a1a", "#5a7e22", "#7ea02c", "#a8c440"], t + 0.2))
        put(img, x0, 16 - h - 1, hexrgb("#e8f88a"))
        put(img, x0 + 1, 16 - h - 1, hexrgb("#d4ec6a"))
    return img


def sea_anemone():
    """A soft column with a crown of waving tentacles, coral pink and violet."""
    img = canvas()
    for y in range(10, 16):
        for x in range(6, 10):
            put(img, x, y, ramp(["#7a2a4a", "#9a3a5e", "#b44e72"], 0.3 + 0.15 * (x - 6)))
    for k in range(9):
        a = math.pi * (0.1 + 0.8 * k / 8)
        for rr in range(1, 7):
            x = int(round(7.5 + math.cos(a) * rr * 1.1))
            y = int(round(10 - math.sin(a) * rr * 0.9 + (0.6 * math.sin(rr + k))))
            put(img, x, y, ramp(["#b44e72", "#d070a0", "#e898c4", "#f4c8e0"], 0.2 + rr / 7))
    return img


def glow_algae():
    """Wisps of algae with glowing cyan tips."""
    img = canvas()
    r = rng_for("glow_algae")
    for x0 in (3, 6, 9, 12):
        h = 5 + int(r.random() * 8)
        ph = r.random() * 6.0
        for i in range(h):
            y = 15 - i
            x = int(round(x0 + math.sin(ph + i * 0.6)))
            put(img, x, y, ramp(["#0e3a3a", "#145050", "#1e6a68", "#2a8a84"], 0.2 + 0.6 * i / h))
        put(img, int(round(x0 + math.sin(ph + h * 0.6))), 15 - h, hexrgb("#a8fff0"))
        put(img, int(round(x0 + math.sin(ph + (h - 1) * 0.6))), 16 - h, hexrgb("#6ae8d8"))
    return img


def shark_fin():
    img = canvas()
    for y in range(2, 14):
        w = int((y - 1) * 0.8)
        for x in range(8 - w // 2, 8 + (w + 1) // 2 + (1 if y > 8 else 0)):
            put(img, x, y, ramp(["#2a323a", "#3e4852", "#56626c", "#707c86"], 0.25 + 0.5 * (x - 4) / 8))
    for x in range(3, 14):
        put(img, x, 14, hexrgb("#e4e8e6"))
    # Outline
    a = img[..., 3] > 0
    for y in range(N):
        for x in range(N):
            if not a[y, x] and any(0 <= x + dx < N and 0 <= y + dy < N and a[y + dy, x + dx] for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                put(img, x, y, hexrgb("#1a2026"))
    return img


PLANTS = {"cattail": cattail, "cattail_top": cattail_top, "duckweed": duckweed, "water_lily": water_lily, "pondweed": pondweed,
          "sea_pickle": sea_pickle, "sea_anemone": sea_anemone, "glow_algae": glow_algae}


# ------------------------------------------------------------------------------- write
def egg(base, spot):
    rows = [(6, 9), (5, 10), (4, 11), (4, 11), (3, 12), (3, 12), (3, 12), (2, 13), (2, 13), (2, 13), (2, 13), (3, 12), (3, 12), (4, 11)]
    spots = [(6, 4), (9, 6), (5, 8), (10, 10), (7, 11), (4, 11), (11, 7), (8, 8)]
    img = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    for i, (a, b) in enumerate(rows):
        y = i + 1
        for x in range(a, b + 1):
            edge = x in (a, b) or i in (0, len(rows) - 1)
            k = 0.6 if edge else (1.1 if x < 7 and y < 8 else 1.0)
            img.putpixel((x, y), tuple(min(255, int(v * k)) for v in base) + (255,))
    for x, y in spots:
        img.putpixel((x, y), spot + (255,))
        img.putpixel((x + 1, y), spot + (255,))
    return img


def main():
    names = []
    os.makedirs(os.path.dirname(OUT_RON), exist_ok=True)
    text = "// The creatures of the lakes, rivers, swamps, seas and cave lakes. Made by tools/gen_aguas.py:\n"
    text += "// edit it and run it again. `waters` says where each lives (crates/game/src/mobs/aquatic.rs).\n(mobs: [\n"
    for m in MOBS:
        text += ron(m)
        render(m["name"], m["model"]).save(os.path.join(ENTITY, m["name"] + ".png"))
        names.append(m["name"])
    text += "])\n"
    with open(OUT_RON, "w", encoding="utf-8") as f:
        f.write(text)
    for n, fn in PLANTS.items():
        img = fn()
        Image.fromarray(img, "RGBA").save(os.path.join(BLOCK_TEX, n + ".png"))
        names.append(n)
    Image.fromarray(shark_fin(), "RGBA").save(os.path.join(ITEM_TEX, "shark_fin.png"))
    names.append("shark_fin")
    items = open(ITEMS_RON, encoding="utf-8").read()
    start, end = "    // aguas begin (tools/gen_aguas.py)\n", "    // aguas end\n"
    if start in items:
        items = items[: items.index(start)] + items[items.index(end) + len(end):]
    add = [start, '    (name: "shark_fin", display_name: Some("Barbatana de Tubarão"), texture: Some("shark_fin")),\n']
    for m in MOBS:
        n = m["name"]
        egg(*EGGS[n]).save(os.path.join(ITEM_TEX, f"{n}_spawn_egg.png"))
        names.append(f"{n}_spawn_egg")
        add.append(f'    (name: "{n}_spawn_egg", display_name: Some("Ovo de {m["display"]}"), texture: Some("{n}_spawn_egg")),\n')
    add.append(end)
    i = items.rindex("]")
    items = items[:i].rstrip() + "\n" + "".join(add) + items[i:]
    with open(ITEMS_RON, "w", encoding="utf-8") as f:
        f.write(items)
    print(",".join(names))
    if len(sys.argv) > 1:
        sheet = Image.new("RGBA", (64 * 8, 128 * ((len(MOBS) + 7) // 8)), (40, 40, 40, 255))
        for k, m in enumerate(MOBS):
            sheet.paste(render(m["name"], m["model"]), ((k % 8) * 64, (k // 8) * 128))
        sheet.resize((sheet.width * 2, sheet.height * 2), Image.NEAREST).save(sys.argv[1])


if __name__ == "__main__":
    main()
