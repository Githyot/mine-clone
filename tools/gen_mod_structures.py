#!/usr/bin/env python3
"""Writes packs/vanilla/mods/structures.ron: the structures of the mod content pack.

Our own builds, made in code, with ideas from structure mods (nothing copied, only the kind of
thing each mod adds):

* villages per biome and raider outposts (ChoiceTheorem's Overhauled Village idea),
* huge rare structures: coliseum, keeps, palaces, ships on the sea, sky ships and temples
  (When Dungeons Arise idea),
* small and medium structures in the Overworld and the Nether (Moog's Voyager Structures idea),
* structures on the End's outer islands (Moog's End Structures idea),
* big buried dungeons (YUNG's Better Dungeons idea),
* little points of interest (Explorify idea),
* single buildings: inns, smithies, chapels, towers (Towns and Towers idea),
* small Nether formations (Formations Nether idea).

Run it from anywhere: `python3 tools/gen_mod_structures.py`. It checks every block, item, mob
and biome name against the packs before writing and stops on the first unknown one.

Layers go bottom to top; each layer is rows from north (-z) to south (+z), each row west to
east. "." is air, " " keeps the terrain. Facing variants: 0 north, 1 east, 2 south, 3 west
(stairs and roofs rise toward their facing, +4 upside down; doors add 8 for the upper half;
slabs 1 is the top half; lantern 1 hangs).
"""
import random
import re
from math import atan2, cos, degrees, pi, sin
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACK = ROOT / "packs" / "vanilla"
OUT = PACK / "mods" / "structures.ron"

AIR = "air"
N, E, S, W = 0, 1, 2, 3
DV = {N: (0, -1), E: (1, 0), S: (0, 1), W: (-1, 0)}


def opp(f):
    return (f + 2) % 4


def var(name, v):
    return name if v == 0 else f"{name}:{v}"


def stair(name, f, up=False):
    return var(name, f + (4 if up else 0))


def slab(name, top=False):
    return var(name, 1 if top else 0)


def door(name, f):
    """Lower and upper half of a closed door on the `f` side of its cell."""
    return var(name, f), var(name, f + 8)


HANG = "lantern:1"


# ---------------------------------------------------------------------------------------------
# The canvas: a box of block names (None keeps the terrain, AIR clears it).
# ---------------------------------------------------------------------------------------------

class Canvas:
    def __init__(self, w, h, d, fill=None):
        self.w, self.h, self.d = w, h, d
        self.g = [[[fill] * w for _ in range(d)] for _ in range(h)]

    def inb(self, x, y, z):
        return 0 <= x < self.w and 0 <= y < self.h and 0 <= z < self.d

    def set(self, x, y, z, b):
        if self.inb(x, y, z):
            self.g[y][z][x] = b

    def get(self, x, y, z):
        return self.g[y][z][x] if self.inb(x, y, z) else None

    def setif(self, x, y, z, b, only=(None, AIR)):
        """Set only where the cell is still empty (kept terrain or air)."""
        if self.inb(x, y, z) and self.g[y][z][x] in only:
            self.g[y][z][x] = b

    def box(self, x0, y0, z0, x1, y1, z1, b):
        x0, x1 = sorted((x0, x1))
        y0, y1 = sorted((y0, y1))
        z0, z1 = sorted((z0, z1))
        for y in range(y0, y1 + 1):
            for z in range(z0, z1 + 1):
                for x in range(x0, x1 + 1):
                    self.set(x, y, z, b)

    def walls(self, x0, z0, x1, z1, y0, y1, b):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.set(x, y, z0, b)
                self.set(x, y, z1, b)
            for z in range(z0, z1 + 1):
                self.set(x0, y, z, b)
                self.set(x1, y, z, b)

    def room(self, x0, y0, z0, x1, y1, z1, wall, floor=None, ceil=None, inside=AIR):
        self.box(x0, y0, z0, x1, y1, z1, inside)
        self.walls(x0, z0, x1, z1, y0, y1, wall)
        if floor:
            self.box(x0, y0, z0, x1, y0, z1, floor)
        if ceil:
            self.box(x0, y1, z0, x1, y1, z1, ceil)

    def disc(self, cx, cz, r, y, b, hollow=False, thick=1.0):
        for z in range(int(cz - r - 1), int(cz + r + 2)):
            for x in range(int(cx - r - 1), int(cx + r + 2)):
                d = ((x - cx) ** 2 + (z - cz) ** 2) ** 0.5
                if d <= r + 0.3 and (not hollow or d > r - thick + 0.3):
                    self.set(x, y, z, b)

    def cyl(self, cx, cz, r, y0, y1, wall, inside=None, thick=1.0):
        for y in range(y0, y1 + 1):
            if inside is not None:
                self.disc(cx, cz, r, y, inside)
            self.disc(cx, cz, r, y, wall, hollow=True, thick=thick)

    def column(self, x, z, y0, y1, b):
        for y in range(y0, y1 + 1):
            self.set(x, y, z, b)

    def line(self, x0, y0, z0, x1, y1, z1, b):
        n = max(abs(x1 - x0), abs(y1 - y0), abs(z1 - z0), 1)
        for i in range(n + 1):
            t = i / n
            self.set(round(x0 + (x1 - x0) * t), round(y0 + (y1 - y0) * t), round(z0 + (z1 - z0) * t), b)

    def replace(self, frm, to, rng=None, p=1.0):
        for layer in self.g:
            for row in layer:
                for i, b in enumerate(row):
                    if b == frm and (rng is None or rng.random() < p):
                        row[i] = to

    def weather(self, rng, swaps, p):
        """Swap some blocks for worn ones: {block: [choices]}."""
        for layer in self.g:
            for row in layer:
                for i, b in enumerate(row):
                    if b in swaps and rng.random() < p:
                        row[i] = rng.choice(swaps[b])

    def decay(self, rng, p, keep=(), above=1):
        """Knock random holes in a ruin: blocks above layer `above` turn to air."""
        for y in range(above, self.h):
            pp = p * (0.4 + 1.2 * y / self.h)
            for row in self.g[y]:
                for i, b in enumerate(row):
                    if b not in (None, AIR) and b not in keep and rng.random() < pp:
                        row[i] = AIR

    def paste(self, other, ox, oy, oz, skip_none=True):
        for y in range(other.h):
            for z in range(other.d):
                for x in range(other.w):
                    b = other.g[y][z][x]
                    if b is None and skip_none:
                        continue
                    self.set(ox + x, oy + y, oz + z, b)

    def trim_top(self):
        while self.h > 1 and all(b in (None, AIR) for row in self.g[-1] for b in row):
            self.g.pop()
            self.h -= 1


PAL_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!#$%&()*+,-/:;<=>?@[]^_`{|}~"


def to_layers(c):
    names = []
    for layer in c.g:
        for row in layer:
            for b in row:
                if b not in (None, AIR) and b not in names:
                    names.append(b)
    if len(names) > len(PAL_CHARS):
        raise SystemExit(f"too many different blocks ({len(names)})")
    pal = {n: PAL_CHARS[i] for i, n in enumerate(names)}
    layers = []
    for layer in c.g:
        rows = []
        for row in layer:
            rows.append("".join(" " if b is None else "." if b == AIR else pal[b] for b in row))
        layers.append(rows)
    return [(ch, n) for n, ch in pal.items()], layers


def struct(name, biomes, canvas, spacing, chance, loot=(), sink=0, foundation=None, place=None, rough=None, mobs=()):
    canvas.trim_top()
    palette, layers = to_layers(canvas)
    return {
        "name": name, "biomes": list(biomes), "spacing": spacing, "chance": chance, "sink": sink,
        "foundation": foundation, "palette": palette, "layers": layers, "loot": list(loot),
        "place": place, "rough": rough, "mobs": list(mobs), "size": (canvas.w, canvas.h, canvas.d),
    }


# ---------------------------------------------------------------------------------------------
# Building pieces.
# ---------------------------------------------------------------------------------------------

def gable(c, x0, z0, x1, z1, y, roof, fill, cap, axis="x", over=1, inside=AIR):
    """A gable roof over walls x0..x1, z0..z1, starting at layer y (one above the wall tops),
    ridge along `axis`, overhanging by `over`. The gable ends are filled with `fill`."""
    if axis == "x":
        a0, a1 = z0 - over, z1 + over
        k = 0
        while a0 + k < a1 - k:
            for x in range(x0 - over, x1 + over + 1):
                c.set(x, y + k, a0 + k, stair(roof, S))
                c.set(x, y + k, a1 - k, stair(roof, N))
            for z in range(max(a0 + k + 1, z0), min(a1 - k - 1, z1) + 1):
                c.set(x0, y + k, z, fill)
                c.set(x1, y + k, z, fill)
                for x in range(x0 + 1, x1):
                    c.set(x, y + k, z, inside)
            k += 1
        if a0 + k == a1 - k:
            for x in range(x0 - over, x1 + over + 1):
                c.set(x, y + k, a0 + k, cap)
        return y + k
    else:
        a0, a1 = x0 - over, x1 + over
        k = 0
        while a0 + k < a1 - k:
            for z in range(z0 - over, z1 + over + 1):
                c.set(a0 + k, y + k, z, stair(roof, E))
                c.set(a1 - k, y + k, z, stair(roof, W))
            for x in range(max(a0 + k + 1, x0), min(a1 - k - 1, x1) + 1):
                c.set(x, y + k, z0, fill)
                c.set(x, y + k, z1, fill)
                for z in range(z0 + 1, z1):
                    c.set(x, y + k, z, inside)
            k += 1
        if a0 + k == a1 - k:
            for z in range(z0 - over, z1 + over + 1):
                c.set(a0 + k, y + k, z, cap)
        return y + k


def hip(c, x0, z0, x1, z1, y, roof, cap, over=1, inside=AIR):
    """A hipped (pyramid) roof: rings stepping in, each side rising inward."""
    ax0, az0, ax1, az1 = x0 - over, z0 - over, x1 + over, z1 + over
    k = 0
    while ax0 + k < ax1 - k and az0 + k < az1 - k:
        X0, X1, Z0, Z1 = ax0 + k, ax1 - k, az0 + k, az1 - k
        for z in range(Z0, Z1 + 1):
            c.set(X0, y + k, z, stair(roof, E))
            c.set(X1, y + k, z, stair(roof, W))
        for x in range(X0, X1 + 1):
            c.set(x, y + k, Z0, stair(roof, S))
            c.set(x, y + k, Z1, stair(roof, N))
        for z in range(Z0 + 1, Z1):
            for x in range(X0 + 1, X1):
                c.set(x, y + k, z, inside)
        k += 1
    X0, X1, Z0, Z1 = ax0 + k, ax1 - k, az0 + k, az1 - k
    for z in range(Z0, Z1 + 1):
        for x in range(X0, X1 + 1):
            c.set(x, y + k, z, cap)
    return y + k


def flat_roof(c, x0, z0, x1, z1, y, b, rim=None, rim_h=1):
    c.box(x0, y, z0, x1, y, z1, b)
    if rim:
        c.walls(x0, z0, x1, z1, y + 1, y + rim_h, rim)


def lamp_post(c, x, y, z, fence, light="lantern", h=2):
    c.column(x, z, y, y + h - 1, fence)
    c.set(x, y + h, z, light)


def hanging_lamp(c, x, y, z, fence, f):
    """A post with an arm reaching to `f` and a lantern hanging from it."""
    c.column(x, z, y, y + 3, fence)
    dx, dz = DV[f]
    c.set(x + dx, y + 3, z + dz, fence)
    c.set(x + dx, y + 2, z + dz, HANG)


def wood_set(wood):
    return {
        "planks": f"{wood}_planks", "log": f"{wood}_log", "stairs": f"{wood}_stairs", "slab": f"{wood}_slab",
        "fence": f"{wood}_fence", "door": f"{wood}_door", "chair": f"{wood}_chair", "table": f"{wood}_table",
        "bench": f"{wood}_bench", "shelf": f"{wood}_shelf", "drawer": f"{wood}_drawer", "window": f"{wood}_window",
        "trapdoor": f"{wood}_trapdoor",
    }


def furnish(c, x0, z0, x1, z1, y, wd, rng, door_xz, things):
    """Furniture along the inside of walls x0..x1, z0..z1 standing on layer y. `things` is a list
    of block names or the special words bed, table, chest."""
    ring = []
    for x in range(x0 + 1, x1):
        ring.append((x, z0 + 1, S))
    for z in range(z0 + 2, z1):
        ring.append((x1 - 1, z, W))
    for x in range(x1 - 2, x0, -1):
        ring.append((x, z1 - 1, N))
    for z in range(z1 - 2, z0 + 1, -1):
        ring.append((x0 + 1, z, E))
    dx, dz = door_xz
    ring = [r for r in ring if abs(r[0] - dx) + abs(r[1] - dz) > 1]
    # Start from the far side of the door so beds and tables don't block the way in.
    ring.sort(key=lambda r: -(abs(r[0] - dx) + abs(r[1] - dz)))
    used = set()
    for t in things:
        spot = next((r for r in ring if (r[0], r[1]) not in used), None)
        if spot is None:
            break
        x, z, face = spot
        used.add((x, z))
        if t == "table":
            c.set(x, y, z, wd["table"])
            c.set(x, y + 1, z, "lantern")
            nxt = next((r for r in ring if (r[0], r[1]) not in used and abs(r[0] - x) + abs(r[1] - z) == 1), None)
            if nxt:
                used.add((nxt[0], nxt[1]))
                to = (x - nxt[0], z - nxt[1])
                f = {(0, -1): N, (1, 0): E, (0, 1): S, (-1, 0): W}[to]
                c.set(nxt[0], y, nxt[1], var(wd["chair"], f))
        elif t == "bed":
            c.set(x, y, z, "red_bed")
        elif t == "shelf":
            c.set(x, y, z, wd["drawer"])
            c.set(x, y + 1, z, var(wd["shelf"], face))
        else:
            c.set(x, y, z, t)


def windows_on(c, x0, z0, x1, z1, y, glass, skip=(), every=2):
    for x in range(x0 + 2, x1 - 1, every):
        for z in (z0, z1):
            if (x, z) not in skip:
                c.set(x, y, z, glass)
    for z in range(z0 + 2, z1 - 1, every):
        for x in (x0, x1):
            if (x, z) not in skip:
                c.set(x, y, z, glass)


def door_spot(x0, z0, x1, z1, f):
    """The middle of the wall on side f."""
    mx, mz = (x0 + x1) // 2, (z0 + z1) // 2
    return {N: (mx, z0), S: (mx, z1), E: (x1, mz), W: (x0, mz)}[f]


def cottage(c, x0, z0, x1, z1, st, f, rng, y=1, wh=3, roof_kind=None, things=None, chimney=None):
    """A small house: walls x0..x1, z0..z1 (inclusive) standing on layer y, door on side f."""
    wd = st["wd"]
    base = st.get("base")
    c.box(x0, y - 1, z0, x1, y - 1, z1, st["floor"])
    c.room(x0, y, z0, x1, y + wh - 1, z1, st["wall"])
    if base:
        c.walls(x0, z0, x1, z1, y, y, base)
    for (x, z) in ((x0, z0), (x1, z0), (x0, z1), (x1, z1)):
        c.column(x, z, y, y + wh - 1, st["corner"])
    dx, dz = door_spot(x0, z0, x1, z1, f)
    windows_on(c, x0, z0, x1, z1, y + 1, st["window"], skip=((dx, dz),))
    lo, hi = door(st["door"], f)
    c.set(dx, y, dz, lo)
    c.set(dx, y + 1, dz, hi)
    ox, oz = DV[f]
    # A step and a lamp outside the door.
    c.set(dx + ox, y - 1, dz + oz, st.get("step", st["floor"]))
    c.set(dx + ox, y, dz + oz, AIR)
    c.set(dx + ox, y + 1, dz + oz, AIR)
    if st.get("porch_lamp", True):
        lx, lz = dx + ox + (oz != 0), dz + oz + (ox != 0)
        if c.get(lx, y, lz) in (None, AIR):
            c.set(lx, y, lz, wd["fence"])
            c.set(lx, y + 1, lz, st["light"])
    kind = roof_kind or st["roof_kind"]
    top = y + wh
    if kind == "gable":
        axis = "x" if (x1 - x0) >= (z1 - z0) else "z"
        gable(c, x0, z0, x1, z1, top, st["roof"], st["gable"], st["cap"], axis=axis)
    elif kind == "hip":
        hip(c, x0, z0, x1, z1, top, st["roof"], st["cap"])
    else:
        flat_roof(c, x0, z0, x1, z1, top, st["flat"], rim=st.get("rim"))
        if st.get("awning"):
            for k in range(-1, 2):
                if ox == 0:
                    c.set(dx + k, y + 2, dz + oz, st["awning"])
                else:
                    c.set(dx + ox, y + 2, dz + k, st["awning"])
    if chimney:
        cx, cz = chimney
        c.column(cx, cz, y, top + (z1 - z0) // 2 + 2, st.get("chimney", "cobblestone"))
    stuff = things if things is not None else ["bed", "table", "crafting_table", "shelf", "furnace", "potted_poppy", "bookshelf"]
    furnish(c, x0, z0, x1, z1, y, wd, rng, (dx, dz), stuff)
    return dx + ox, dz + oz


def farm(c, x0, z0, x1, z1, st, rng, crops=None):
    """Farmland in rows round a water channel, a log border and crops."""
    crops = crops or st["crops"]
    c.box(x0, 0, z0, x1, 0, z1, st["farm_border"])
    long_x = (x1 - x0) >= (z1 - z0)
    mid = (z0 + z1) // 2 if long_x else (x0 + x1) // 2
    crop = rng.choice(crops)
    for z in range(z0 + 1, z1):
        for x in range(x0 + 1, x1):
            on_mid = (z == mid) if long_x else (x == mid)
            if on_mid:
                c.set(x, 0, z, "water")
            else:
                c.set(x, 0, z, "farmland")
                c.set(x, 1, z, crop if rng.random() > 0.08 else rng.choice(crops))
    if st.get("farm_fence"):
        fence = st["wd"]["fence"]
        for x in range(x0, x1 + 1):
            for z in (z0, z1):
                c.set(x, 1, z, fence)
        for z in range(z0, z1 + 1):
            for x in (x0, x1):
                c.set(x, 1, z, fence)
        # A gap to walk in.
        c.set((x0 + x1) // 2, 1, z1, AIR)


def well(c, cx, cz, st):
    """A well: a stone rim round water, four posts and a little roof with a hanging lantern."""
    rim, roofb = st["well_rim"], st["well_roof"]
    c.box(cx - 2, 0, cz - 2, cx + 2, 0, cz + 2, st["plaza"])
    c.box(cx - 1, 0, cz - 1, cx + 1, 1, cz + 1, rim)
    c.set(cx, 0, cz, "water")
    c.set(cx, 1, cz, "water")
    c.set(cx, -1, cz, "water")
    for (x, z) in ((cx - 1, cz - 1), (cx + 1, cz - 1), (cx - 1, cz + 1), (cx + 1, cz + 1)):
        c.column(x, z, 2, 3, st["wd"]["fence"])
    c.box(cx - 1, 4, cz - 1, cx + 1, 4, cz + 1, roofb)
    c.set(cx, 5, cz, st["wd"]["slab"])
    c.set(cx, 3, cz, HANG)


def scatter_plants(c, x0, z0, x1, z1, rng, plants, p=0.3):
    for z in range(z0, z1 + 1):
        for x in range(x0, x1 + 1):
            if c.get(x, 1, z) == AIR and c.get(x, 0, z) in ("grass_block", "snowy_grass_block", "moss_block") and rng.random() < p:
                c.set(x, 1, z, rng.choice(plants))


# ---------------------------------------------------------------------------------------------
# Villages (ChoiceTheorem's Overhauled Village idea): one big template per biome.
# ---------------------------------------------------------------------------------------------

def style(wood, **kw):
    wd = wood_set(wood)
    st = {
        "wd": wd, "ground": "grass_block", "fill": "dirt", "path": ["gravel", "gravel", "rocky_dirt", "cobblestone"],
        "plaza": "cobblestone_bricks", "floor": "cobblestone", "wall": wd["planks"], "corner": wd["log"],
        "roof": "oak_roof", "gable": wd["planks"], "cap": wd["slab"], "window": "glass_pane", "door": wd["door"],
        "roof_kind": "gable", "flat": wd["planks"], "light": "lantern", "farm_border": wd["log"],
        "crops": ["wheat_stage7", "wheat_stage7", "carrots_stage3", "potatoes_stage3", "beetroots_stage3", "wheat_stage5"],
        "plants": ["short_grass", "short_grass", "short_grass", "poppy", "dandelion"], "well_rim": "cobblestone",
        "well_roof": wd["slab"], "decor": ["hay_block", "hay_block", "pumpkin", "melon"], "farm_fence": False,
        "chimney": "cobblestone", "special": "smithy",
    }
    st.update(kw)
    return st


VILLAGE_STYLES = {
    "plains": style("oak", base="cobblestone"),
    "desert": style("jungle", ground="sand", fill="sandstone", path=["sandstone", "smooth_stone", "sandstone", "gravel"],
                    plaza="sandstone", floor="sandstone", wall="sandstone", corner="smooth_stone", roof_kind="flat",
                    flat="sandstone", rim="sandstone_slab", window="jungle_window", farm_border="sandstone",
                    crops=["wheat_stage7", "carrots_stage3", "beetroots_stage3"], plants=["dead_bush", "cactus", "dead_bush"],
                    well_rim="sandstone", well_roof="sandstone_slab", decor=["hay_block", "cactus", "melon"],
                    awning="orange_wool", chimney="sandstone", special="tower"),
    "savanna": style("acacia", roof="thatch_roof", gable="acacia_planks", cap="thatch", base="orange_terracotta",
                     floor="acacia_planks", farm_border="acacia_log", path=["gravel", "rocky_dirt", "dirt", "gravel"],
                     plants=["short_grass", "short_grass", "dead_bush"], well_roof="acacia_slab", special="market"),
    "taiga": style("spruce", roof="spruce_roof", base="cobblestone", floor="spruce_planks", farm_fence=True,
                   plants=["short_grass", "short_grass", "brown_mushroom", "twig", "pebble"], decor=["hay_block", "pumpkin", "spruce_log"],
                   special="hall", palisade="spruce_log"),
    "snowy": style("spruce", ground="snowy_grass_block", roof="spruce_roof", base="cobblestone_bricks", wall="spruce_planks",
                   floor="spruce_planks", path=["gravel", "packed_ice", "gravel", "cobblestone"], plaza="cobblestone_bricks",
                   plants=["snow_layer", "snow_layer", "snow_layer", "twig"], crops=["potatoes_stage3", "carrots_stage3", "beetroots_stage3"],
                   decor=["hay_block", "packed_ice", "pumpkin"], special="igloo", farm_fence=True),
    "jungle": style("jungle", roof="thatch_roof", cap="thatch", gable="jungle_planks", floor="jungle_planks",
                    base="mossy_cobblestone", path=["rocky_dirt", "moss_block", "gravel", "dirt"], plaza="mossy_cobblestone_bricks",
                    plants=["short_grass", "short_grass", "short_grass", "poppy", "vine"], crops=["melon", "wheat_stage7", "carrots_stage3"],
                    decor=["melon", "melon", "jungle_log"], well_rim="mossy_cobblestone", special="tower", stilts=2),
    "swamp": style("dark_oak", roof="spruce_roof", gable="spruce_planks", wall="spruce_planks", corner="dark_oak_log",
                   floor="dark_oak_planks", path=["oak_planks", "dirt", "rocky_dirt", "oak_slab"], plaza="mossy_cobblestone",
                   plants=["short_grass", "brown_mushroom", "red_mushroom", "short_grass"], well_rim="mossy_cobblestone",
                   decor=["hay_block", "brown_mushroom", "spruce_log"], special="hall", stilts=2),
    "badlands": style("dark_oak", ground="red_sand", fill="terracotta", path=["red_terracotta", "gravel", "orange_terracotta", "gravel"],
                      plaza="brown_terracotta", floor="dark_oak_planks", wall="orange_terracotta", corner="dark_oak_log",
                      base="brown_terracotta", roof_kind="flat", flat="dark_oak_planks", rim="dark_oak_fence",
                      farm_border="dark_oak_log", crops=["potatoes_stage3", "wheat_stage7", "beetroots_stage3"],
                      plants=["dead_bush", "dead_bush", "cactus"], well_rim="red_terracotta", decor=["hay_block", "cactus", "dark_oak_log"],
                      awning="red_wool", special="smithy"),
    "mountains": style("spruce", roof="stone_brick_roof", gable="spruce_planks", cap="stone_brick_slab", base="cobblestone_bricks",
                       wall="spruce_planks", corner="stone_bricks", floor="stone_bricks", plaza="stone_bricks",
                       path=["cobblestone", "gravel", "stone_bricks", "rocky_dirt"], plants=["short_grass", "pebble", "short_grass"],
                       well_rim="stone_bricks", well_roof="stone_brick_slab", special="tower", farm_fence=True),
    "cherry": style("cherry", roof="brick_roof", gable="cherry_planks", cap="brick_slab", base="stone_bricks", wall="white_wool",
                    corner="cherry_log", floor="cherry_planks", window="lattice", light="paper_lamp", path=["gravel", "smooth_stone", "gravel", "pebble"],
                    plaza="smooth_stone", plants=["short_grass", "poppy", "short_grass", "dandelion"], well_rim="stone_bricks",
                    well_roof="cherry_slab", decor=["hay_block", "potted_poppy", "red_paper_lamp"], special="pagoda"),
    "beach": style("birch", ground="sand", fill="sandstone", path=["oak_planks", "sandstone", "gravel", "oak_planks"],
                   plaza="sandstone", floor="birch_planks", wall="birch_planks", corner="oak_log", base="sandstone",
                   roof="oak_roof", gable="oak_planks", farm_border="sandstone", crops=["carrots_stage3", "potatoes_stage3"],
                   plants=["short_grass", "dead_bush"], well_rim="sandstone", decor=["oak_planks", "hay_block", "light_blue_wool"],
                   special="dock"),
}


def special_building(c, x0, z0, C, st, rng, f, kind):
    """The one big building of a village, filling a cell of C by C. Returns where its door is."""
    wd = st["wd"]
    if kind == "smithy":
        # Open-fronted smithy: stone floor, a forge with lava behind bars, anvil-like table, chest.
        x1, z1 = x0 + C - 2, z0 + C - 2
        xa, za = x0 + 1, z0 + 1
        c.box(xa, 0, za, x1, 0, z1, "cobblestone")
        c.walls(xa, za, x1, z1, 1, 3, "cobblestone")
        for (x, z) in ((xa, za), (x1, za), (xa, z1), (x1, z1)):
            c.column(x, z, 1, 3, wd["log"])
        c.box(xa + 1, 1, za + 1, x1 - 1, 3, z1 - 1, AIR)
        # Open the side facing the path, with fence posts.
        dx, dz = door_spot(xa, za, x1, z1, f)
        side = [(x, dz) for x in range(xa + 1, x1)] if f in (N, S) else [(dx, z) for z in range(za + 1, z1)]
        for (x, z) in side:
            c.box(x, 1, z, x, 3, z, AIR)
        for (x, z) in side[::3]:
            c.column(x, z, 1, 3, wd["fence"])
        # Forge in the back corner.
        bx, bz = (xa + 1, za + 1) if f in (S, E) else (x1 - 2, z1 - 2)
        c.box(bx, 1, bz, bx + 1, 1, bz + 1, "cobblestone")
        c.set(bx, 1, bz, "lava")
        c.set(bx + 1, 1, bz, "furnace")
        c.set(bx, 2, bz + 1, "iron_bars")
        c.column(bx, bz, 2, 5, "bricks")
        c.set(xa + 1, 1, z1 - 1 if bz == za + 1 else za + 1, "chest")
        c.set(x1 - 1, 1, (za + z1) // 2, "iron_block")
        c.set(x1 - 1, 1, (za + z1) // 2 + 1, wd["table"])
        c.set((xa + x1) // 2, 1, (za + z1) // 2, "crafting_table")
        flat_roof(c, xa, za, x1, z1, 4, wd["slab"])
        c.set((xa + x1) // 2, 3, (za + z1) // 2 + 1, HANG)
        return dx, dz
    if kind == "tower":
        # A watchtower of the village's stone with a lookout and a lantern.
        cx, cz = x0 + C // 2, z0 + C // 2
        stone = st.get("base", "cobblestone") if st.get("base") else st["wall"]
        stone = "cobblestone_bricks" if stone in ("cobblestone", "mossy_cobblestone") else stone
        c.box(cx - 2, 0, cz - 2, cx + 2, 0, cz + 2, stone)
        c.room(cx - 2, 1, cz - 2, cx + 2, 10, cz + 2, stone)
        ring = [(cx - 1, cz - 1), (cx, cz - 1), (cx + 1, cz - 1), (cx + 1, cz), (cx + 1, cz + 1), (cx, cz + 1), (cx - 1, cz + 1), (cx - 1, cz)]
        stairs_name = {"sandstone": "sandstone_stairs", "stone_bricks": "stone_brick_stairs"}.get(stone, wd["stairs"])
        for y in range(1, 11):
            x, z = ring[(y - 1) % 8]
            nx, nz = ring[y % 8]
            fac = {(0, -1): N, (1, 0): E, (0, 1): S, (-1, 0): W}[(nx - x, nz - z)]
            c.set(x, y, z, stair(stairs_name, fac))
            c.set(cx, y, cz, stone)
        dx, dz = door_spot(cx - 2, cz - 2, cx + 2, cz + 2, f)
        c.set(dx, 1, dz, AIR)
        c.set(dx, 2, dz, AIR)
        for y in (4, 7):
            for (x, z) in ((cx, cz - 2), (cx, cz + 2), (cx - 2, cz), (cx + 2, cz)):
                c.set(x, y, z, "iron_bars")
        c.box(cx - 3, 10, cz - 3, cx + 3, 10, cz + 3, wd["planks"])
        x, z = ring[9 % 8]
        c.set(x, 10, z, stair(stairs_name, E))
        for k in (0, 7):
            c.set(ring[k][0], 10, ring[k][1], AIR)
        c.walls(cx - 3, cz - 3, cx + 3, cz + 3, 11, 11, wd["fence"])
        for (x, z) in ((cx - 3, cz - 3), (cx + 3, cz - 3), (cx - 3, cz + 3), (cx + 3, cz + 3)):
            c.column(x, z, 11, 13, wd["fence"])
        hip(c, cx - 3, cz - 3, cx + 3, cz + 3, 14, st["roof"] if st["roof_kind"] != "flat" else "oak_roof", st["cap"], over=0)
        c.set(cx, 13, cz, HANG)
        c.set(cx + 2, 11, cz + 2, "chest")
        return dx, dz
    if kind == "market":
        # Stalls with wool canopies round a square.
        colors = ["red_wool", "yellow_wool", "orange_wool", "lime_wool", "light_blue_wool", "white_wool"]
        c.box(x0 + 1, 0, z0 + 1, x0 + C - 2, 0, z0 + C - 2, st["plaza"])
        for (sx, sz) in ((x0 + 1, z0 + 1), (x0 + C - 4, z0 + 1), (x0 + 1, z0 + C - 4), (x0 + C - 4, z0 + C - 4)):
            col = rng.choice(colors)
            for (x, z) in ((sx, sz), (sx + 2, sz), (sx, sz + 2), (sx + 2, sz + 2)):
                c.column(x, z, 1, 2, wd["fence"])
            c.box(sx, 3, sz, sx + 2, 3, sz + 2, col)
            c.set(sx + 1, 1, sz + 1, wd["table"])
            c.set(sx + 1, 2, sz + 1, rng.choice(["melon", "pumpkin", "hay_block", "lantern"]))
        c.set(x0 + C // 2, 1, z0 + C // 2, "chest")
        lamp_post(c, x0 + C // 2, 1, z0 + C // 2 - 1, wd["fence"], st["light"])
        return door_spot(x0, z0, x0 + C - 1, z0 + C - 1, f)
    if kind == "hall":
        # A long meeting hall with benches, a big table and a chest.
        if f in (N, S):
            xa, za, x1, z1 = x0 + 1, z0 + 1, x0 + C - 2, z0 + C - 3
        else:
            xa, za, x1, z1 = x0 + 1, z0 + 1, x0 + C - 3, z0 + C - 2
        out = cottage(c, xa, za, x1, z1, st, f, rng, y=1, wh=4, things=["chest", "bookshelf", "bookshelf", "furnace", "crafting_table", "shelf", "shelf"])
        cx, cz = (xa + x1) // 2, (za + z1) // 2
        c.set(cx, 1, cz, wd["table"])
        c.set(cx, 2, cz, "lantern")
        for (x, z, fc) in ((cx - 1, cz, E), (cx + 1, cz, W)):
            c.set(x, 1, z, var(wd["chair"], fc))
        c.set(cx, 4, cz, HANG)
        return out
    if kind == "igloo":
        cx, cz = x0 + C // 2, z0 + C // 2
        for y, r in ((1, 3.4), (2, 3.2), (3, 2.6), (4, 1.6)):
            c.disc(cx, cz, r, y, "snow", hollow=(y < 4), thick=1.0)
        c.disc(cx, cz, 3.4, 0, "snow")
        dx, dz = DV[f]
        for k in range(2, 5):
            c.set(cx + dx * k, 1, cz + dz * k, AIR)
            c.set(cx + dx * k, 2, cz + dz * k, AIR)
        c.set(cx + dx * 4 + dz, 1, cz + dz * 4 + dx, "snow")
        c.set(cx + dx * 4 - dz, 1, cz + dz * 4 - dx, "snow")
        c.set(cx + dx * 4 + dz, 2, cz + dz * 4 + dx, "snow")
        c.set(cx + dx * 4 - dz, 2, cz + dz * 4 - dx, "snow")
        c.set(cx + dx * 4, 3, cz + dz * 4, "snow")
        c.set(cx - dx * 2, 1, cz - dz * 2, "red_bed")
        c.set(cx - dx * 2 + dz, 1, cz - dz * 2 + dx, "chest")
        c.set(cx - dx * 2 - dz, 1, cz - dz * 2 - dx, "furnace")
        c.set(cx, 3, cz, HANG)
        c.set(cx, 0, cz, "white_wool")
        return cx + dx * 4, cz + dz * 4
    if kind == "pagoda":
        cx, cz = x0 + C // 2, z0 + C // 2
        c.box(cx - 3, 0, cz - 3, cx + 3, 0, cz + 3, "stone_bricks")
        c.room(cx - 2, 1, cz - 2, cx + 2, 3, cz + 2, "white_wool")
        for (x, z) in ((cx - 2, cz - 2), (cx + 2, cz - 2), (cx - 2, cz + 2), (cx + 2, cz + 2)):
            c.column(x, z, 1, 3, "cherry_log")
        # First flared roof: one ring of roof round a plank deck.
        c.box(cx - 2, 4, cz - 2, cx + 2, 4, cz + 2, "cherry_planks")
        for k in range(-3, 4):
            c.set(k + cx, 4, cz - 3, stair("brick_roof", S))
            c.set(k + cx, 4, cz + 3, stair("brick_roof", N))
            c.set(cx - 3, 4, k + cz, stair("brick_roof", E))
            c.set(cx + 3, 4, k + cz, stair("brick_roof", W))
        # Second storey and its roof.
        c.room(cx - 1, 5, cz - 1, cx + 1, 6, cz + 1, "white_wool")
        for (x, z) in ((cx - 1, cz - 1), (cx + 1, cz - 1), (cx - 1, cz + 1), (cx + 1, cz + 1)):
            c.column(x, z, 5, 6, "cherry_log")
        c.set(cx, 6, cz - 1, "lattice")
        c.set(cx, 6, cz + 1, "lattice")
        hip(c, cx - 1, cz - 1, cx + 1, cz + 1, 7, "brick_roof", "red_paper_lamp", over=1)
        for (x, z) in ((cx, cz - 2), (cx, cz + 2), (cx - 2, cz), (cx + 2, cz)):
            c.set(x, 2, z, "lattice")
        dx, dz = door_spot(cx - 2, cz - 2, cx + 2, cz + 2, f)
        lo, hi = door("cherry_door", f)
        c.set(dx, 1, dz, lo)
        c.set(dx, 2, dz, hi)
        c.set(cx, 1, cz, "chest")
        c.set(cx + 1, 1, cz, "cherry_table")
        c.set(cx - 1, 1, cz, "potted_poppy")
        c.set(cx, 3, cz, "paper_lamp")
        for (x, z) in ((cx - 3, cz - 3), (cx + 3, cz - 3), (cx - 3, cz + 3), (cx + 3, cz + 3)):
            c.set(x, 1, z, "stone_bricks")
            c.set(x, 2, z, "red_paper_lamp")
        return dx + DV[f][0], dz + DV[f][1]
    if kind == "dock":
        # Fishermen's dock: a plank deck with posts, fish racks of fences and a boat.
        c.box(x0, 0, z0, x0 + C - 1, 0, z0 + C - 1, "water")
        c.box(x0, -1, z0, x0 + C - 1, -1, z0 + C - 1, "sand")
        for z in range(z0, z0 + C):
            for x in range(x0 + C // 2 - 1, x0 + C // 2 + 2):
                c.set(x, 1, z, "oak_slab")
        for z in range(z0, z0 + C, 3):
            for x in (x0 + C // 2 - 2, x0 + C // 2 + 2):
                c.column(x, z, -1, 2, "oak_log")
        c.set(x0 + C // 2 + 2, 3, z0 + C - 1, "lantern")
        # A rowboat of slabs and stairs on the water.
        bx, bz = x0 + 1, z0 + 2
        for z in range(bz, bz + 4):
            c.set(bx, 1, z, "spruce_slab")
            c.set(bx + 2, 1, z, "spruce_slab")
            c.set(bx + 1, 0, z, "spruce_planks")
        c.set(bx + 1, 1, bz, stair("spruce_stairs", N))
        c.set(bx + 1, 1, bz + 3, stair("spruce_stairs", S))
        c.set(x0 + C // 2, 2, z0 + 1, "chest")
        return door_spot(x0, z0, x0 + C - 1, z0 + C - 1, f)
    return door_spot(x0, z0, x0 + C - 1, z0 + C - 1, f)


def village(name, biomes, st, seed, C=9, mobs=None, spacing=36, chance=0.5, rough=7, loot=None, H=20):
    """Four by four cells round a cross of main roads, with lanes between all cells."""
    rng = random.Random(seed)
    gaps = [1, C, 2, C, 3, C, 2, C, 1]
    starts = []
    pos = 0
    for g in gaps:
        starts.append(pos)
        pos += g
    size = pos
    cell0 = [starts[1], starts[3], starts[5], starts[7]]
    c = Canvas(size, H, size, AIR)
    st = dict(st)
    stilts = st.get("stilts", 0)
    # The ground: grass everywhere, then lanes and the main roads.
    c.box(0, 0, 0, size - 1, 0, size - 1, st["ground"])
    road = [starts[4] + k for k in range(3)]
    lanes = [starts[2], starts[2] + 1, starts[6], starts[6] + 1]
    for z in range(size):
        for x in range(size):
            on_road = x in road or z in road
            on_lane = x in lanes or z in lanes
            if on_road or (on_lane and rng.random() < 0.85):
                c.set(x, 0, z, rng.choice(st["path"]))
    # Which cell holds what: the well next to the crossing, one special building, farms, houses.
    cells = [(i, j) for j in range(4) for i in range(4)]
    inner = [(1, 1), (2, 1), (1, 2), (2, 2)]
    well_cell = rng.choice(inner)
    rest = [p for p in cells if p != well_cell]
    rng.shuffle(rest)
    n_houses = rng.randint(5, 7)
    plan = {well_cell: "well", rest[0]: "special"}
    for p in rest[1:1 + n_houses]:
        plan[p] = "house"
    for p in rest[1 + n_houses:1 + n_houses + 3]:
        plan[p] = "farm"
    for p in rest[1 + n_houses + 3:]:
        plan[p] = rng.choice(["garden", "farm", "decor"])
    chest_done = False
    for (i, j), what in plan.items():
        x0, z0 = cell0[i], cell0[j]
        x1, z1 = x0 + C - 1, z0 + C - 1
        # Face the main roads where we can.
        f = rng.choice([E if i < 2 else W, S if j < 2 else N])
        if what == "well":
            c.box(x0, 0, z0, x1, 0, z1, st["plaza"])
            well(c, x0 + C // 2, z0 + C // 2, st)
            for (x, z) in ((x0, z0), (x1, z0), (x0, z1), (x1, z1)):
                lamp_post(c, x, 1, z, st["wd"]["fence"], st["light"], h=2)
            for (x, z, fc) in ((x0 + C // 2, z0, S), (x0 + C // 2, z1, N)):
                c.set(x, 1, z, var(st["wd"]["bench"], fc))
        elif what == "special":
            special_building(c, x0, z0, C, st, rng, f, st["special"])
        elif what == "house":
            w = rng.choice([5, 6, 7]) if C <= 9 else rng.choice([7, 8, 9])
            d = rng.choice([5, 6, 7]) if C <= 9 else rng.choice([6, 7, 8, 9])
            w, d = min(w, C - 2), min(d, C - 2)
            # Push the house toward the side its door faces.
            if f == E:
                hx0 = x1 - w
            elif f == W:
                hx0 = x0 + 1
            else:
                hx0 = x0 + (C - w) // 2
            if f == S:
                hz0 = z1 - d
            elif f == N:
                hz0 = z0 + 1
            else:
                hz0 = z0 + (C - d) // 2
            hx1, hz1 = hx0 + w - 1, hz0 + d - 1
            things = ["bed", "table", "crafting_table", "shelf", "furnace", "potted_poppy", "bookshelf"]
            if not chest_done:
                things.insert(1, "chest")
                chest_done = True
            tail = things[2:]
            rng.shuffle(tail)
            things = things[:2] + tail
            y = 1 + stilts
            if stilts:
                for (x, z) in ((hx0, hz0), (hx1, hz0), (hx0, hz1), (hx1, hz1)):
                    c.column(x, z, 0, stilts, st["corner"] if "log" in st["corner"] else st["wd"]["log"])
            chim = (hx0 + 1, hz0 + 1) if rng.random() < 0.5 and st["roof_kind"] != "flat" else None
            ex, ez = cottage(c, hx0, hz0, hx1, hz1, st, f, rng, y=y, wh=rng.choice([3, 3, 4]), things=things, chimney=chim)
            if stilts:
                # Steps down from the door to the ground.
                ox, oz = DV[f]
                for k in range(stilts):
                    c.set(ex + ox * k, y - 1 - k, ez + oz * k, stair(st["wd"]["stairs"], opp(f)))
                c.set(ex - ox, y - 1, ez - oz, st["floor"])
        elif what == "farm":
            fx0, fz0, fx1, fz1 = x0, z0 + 1, x1, z1 - 1
            if rng.random() < 0.5:
                fx0, fz0, fx1, fz1 = x0 + 1, z0, x1 - 1, z1
            farm(c, fx0, fz0, fx1, fz1, st, rng)
            if rng.random() < 0.6:
                c.set(fx0, 1, fz0, rng.choice(st["decor"]))
        elif what == "garden":
            scatter_plants(c, x0, z0, x1, z1, rng, st["plants"], p=0.45)
            if st["ground"] != "sand":
                c.set(x0 + C // 2, 1, z0 + C // 2, st["wd"]["log"])
                c.set(x0 + C // 2, 2, z0 + C // 2, st["wd"]["log"])
            c.set(x0 + 1, 1, z0 + 1, var(st["wd"]["bench"], S))
        else:
            for _ in range(5):
                x, z = rng.randint(x0, x1), rng.randint(z0, z1)
                c.set(x, 1, z, rng.choice(st["decor"]))
                if rng.random() < 0.4:
                    c.set(x, 2, z, rng.choice(st["decor"]))
            hanging_lamp(c, x0 + C // 2, 1, z0 + C // 2, st["wd"]["fence"], rng.choice([N, E, S, W]))
    # Lamp posts along the main roads.
    for k in range(3, size - 3, 8):
        for (x, z, f) in ((road[0] - 1, k, E), (k, road[2] + 1, N)):
            if c.get(x, 1, z) == AIR and c.get(x, 0, z) not in ("water", "farmland"):
                hanging_lamp(c, x, 1, z, st["wd"]["fence"], f)
    # A palisade for villages that want one, open where the roads leave.
    if st.get("palisade"):
        for k in range(size):
            for (x, z) in ((k, 0), (k, size - 1), (0, k), (size - 1, k)):
                if not (x in road or z in road):
                    c.column(x, z, 1, 3, st["palisade"])
    scatter_plants(c, 0, 0, size - 1, size - 1, rng, st["plants"], p=0.06)
    loot = loot or [("bread", 2, 6, 0.8), ("apple", 1, 4, 0.6), ("wheat", 3, 9, 0.6), ("carrot", 2, 6, 0.5), ("potato", 2, 6, 0.5),
                    ("emerald", 1, 4, 0.45), ("iron_ingot", 1, 3, 0.4), ("wheat_seeds", 2, 8, 0.6), ("leather", 1, 3, 0.3),
                    ("iron_hoe", 1, 1, 0.15), ("cooked_porkchop", 1, 3, 0.3)]
    mobs = mobs or [("npc_farmer", 2), ("npc_guard", 1), ("npc_merchant", 1)]
    return struct(name, biomes, c, spacing, chance, loot=loot, sink=1, foundation=st["fill"], rough=rough, mobs=mobs)


# ---------------------------------------------------------------------------------------------
# Checking names against the packs, and writing the file.
# ---------------------------------------------------------------------------------------------

VARIANTS = {"Slab": 3, "Stairs": 8, "Layer": 8, "Roof": 8, "Door": 16, "Trapdoor": 16, "Wire": 16, "Lever": 16, "Button": 16,
            "Repeater": 16, "Piston": 16, "PistonHead": 16, "Plate": 2, "Torch": 5, "Comparator": 128, "Oriented": 16,
            "Hopper": 5, "Chair": 4, "Bench": 4, "Shelf": 4, "Lantern": 2, "Bridge": 2, "Portal": 3, "Rail": 16, "Liquid": 16}
BIOMES = {"Plains", "Forest", "Desert", "Snowy Plains", "Ocean", "Beach", "Mountains", "Birch Forest", "Taiga", "Swamp", "Jungle",
          "Savanna", "Dark Forest", "Badlands", "Cherry Grove", "Nether Wastes", "Soul Sand Valley", "Crimson Forest", "The End",
          "Warped Forest"}


def pack_names():
    blocks = {}
    for f in (PACK / "blocks.ron", PACK / "mods" / "blocks.ron"):
        for line in f.read_text().splitlines():
            m = re.search(r'\(name: "([^"]+)"', line)
            if m:
                sh = re.search(r"shape: (\w+)", line)
                blocks[m.group(1)] = sh.group(1) if sh else "Cube"
    items = set()
    for f in (PACK / "items.ron", PACK / "mods" / "items.ron"):
        items |= set(re.findall(r'\(name: "([^"]+)"', f.read_text()))
    for f in (PACK / "blocks.ron", PACK / "mods" / "blocks.ron"):
        for line in f.read_text().splitlines():
            m = re.search(r'\(name: "([^"]+)"', line)
            if m and "in_inventory: false" not in line:
                items.add(m.group(1))
    mobs = set()
    for f in (PACK / "mobs.ron", PACK / "mods" / "mobs.ron", PACK / "mods" / "dims_mobs.ron"):
        mobs |= set(re.findall(r'^\s*name: "([^"]+)"', f.read_text(), re.M))
        mobs |= set(re.findall(r'\(name: "([^"]+)"', f.read_text()))
    return blocks, items, mobs


def check(structs):
    blocks, items, mobs = pack_names()
    names = set()
    for s in structs:
        n = s["name"]
        if n in names:
            raise SystemExit(f"two structures called {n}")
        names.add(n)
        for _, b in s["palette"]:
            base, _, v = b.partition(":")
            if base not in blocks:
                raise SystemExit(f"{n}: unknown block {b}")
            if v and int(v) >= VARIANTS.get(blocks[base], 1):
                raise SystemExit(f"{n}: {base} has no variant {v}")
        for (item, lo, hi, ch) in s["loot"]:
            if item not in items:
                raise SystemExit(f"{n}: unknown loot item {item}")
        for (m, k) in s["mobs"]:
            if m not in mobs:
                raise SystemExit(f"{n}: unknown mob {m}")
        for b in s["biomes"]:
            if b not in BIOMES:
                raise SystemExit(f"{n}: unknown biome {b}")
        w, h, d = s["size"]
        if w > 64 or d > 64 or h > 80:
            raise SystemExit(f"{n}: too big ({w}x{h}x{d})")
        if s["foundation"] and s["foundation"] not in blocks:
            raise SystemExit(f"{n}: unknown foundation {s['foundation']}")


def ron(structs):
    out = ["// Structures of the mod content pack. Generated by tools/gen_mod_structures.py: edit that",
           "// script, not this file. Read after packs/vanilla/structures.ron, same format.", "(structures: ["]
    for s in structs:
        w, h, d = s["size"]
        out.append(f"    // {s['name']}: {w} x {d}, {h} tall")
        out.append("    (")
        out.append(f'        name: "{s["name"]}",')
        out.append("        biomes: [" + ", ".join(f'"{b}"' for b in s["biomes"]) + "],")
        out.append(f'        spacing: {s["spacing"]},')
        out.append(f'        chance: {s["chance"]},')
        if s["sink"]:
            out.append(f'        sink: {s["sink"]},')
        if s["foundation"]:
            out.append(f'        foundation: Some("{s["foundation"]}"),')
        if s["place"]:
            out.append(f'        place: {s["place"]},')
        if s["rough"] is not None:
            out.append(f'        rough: {s["rough"]},')
        if s["mobs"]:
            out.append("        mobs: [" + ", ".join(f'("{m}", {k})' for m, k in s["mobs"]) + "],")
        out.append("        palette: [" + ", ".join(f"('{k}', \"{v}\")" for k, v in s["palette"]) + "],")
        out.append("        layers: [")
        for layer in s["layers"]:
            out.append("            [" + ", ".join(f'"{r}"' for r in layer) + "],")
        out.append("        ],")
        if s["loot"]:
            out.append("        loot: [")
            for item, lo, hi, ch in s["loot"]:
                out.append(f'            (item: "{item}", min: {lo}, max: {hi}, chance: {ch}),')
            out.append("        ],")
        out.append("    ),")
    out.append("])")
    return "\n".join(out) + "\n"


GROUPS = {}


def group(name):
    def deco(fn):
        GROUPS.setdefault(name, []).append(fn)
        return fn
    return deco


def main():
    import sys
    only = sys.argv[1:]
    structs = []
    for g, fns in GROUPS.items():
        for fn in fns:
            got = fn()
            got = got if isinstance(got, list) else [got]
            structs.extend(got)
            print(f"{g:10} " + ", ".join(s["name"] for s in got))
    check(structs)
    OUT.write_text(ron(structs))
    print(f"wrote {OUT} ({len(structs)} structures)")


# ---------------------------------------------------------------------------------------------
# The structures, group by group.
# ---------------------------------------------------------------------------------------------

@group("ctov")
def villages():
    V = VILLAGE_STYLES
    return [
        village("vila_da_planicie", ["Plains", "Forest"], V["plains"], 101),
        village("vila_do_deserto", ["Desert"], V["desert"], 102, mobs=[("npc_farmer", 1), ("npc_guard", 1), ("npc_merchant", 2)]),
        village("vila_da_savana", ["Savanna"], V["savanna"], 103),
        village("vila_da_taiga", ["Taiga"], V["taiga"], 104, mobs=[("npc_farmer", 2), ("npc_guard", 2), ("npc_smith", 1)]),
        village("vila_nevada", ["Snowy Plains"], V["snowy"], 105),
        village("vila_da_selva", ["Jungle"], V["jungle"], 106, rough=8),
        village("vila_do_pantano", ["Swamp"], V["swamp"], 107, rough=5),
        village("vila_das_badlands", ["Badlands"], V["badlands"], 108, mobs=[("npc_guard", 2), ("npc_smith", 1), ("npc_merchant", 1)]),
        village("vila_da_montanha", ["Mountains"], V["mountains"], 109, C=11, rough=10),
        village("vila_das_cerejeiras", ["Cherry Grove"], V["cherry"], 110),
        village("vila_de_pescadores", ["Beach"], V["beach"], 111, rough=5, mobs=[("npc_farmer", 1), ("npc_guard", 1), ("npc_merchant", 2)]),
        village("aldeia_da_floresta", ["Birch Forest", "Dark Forest"], V["plains"] | {"wd": wood_set("birch"), "wall": "birch_planks",
                "corner": "birch_log", "gable": "birch_planks", "cap": "birch_slab", "door": "birch_door", "window": "birch_window",
                "roof": "brick_roof", "farm_border": "birch_log", "special": "hall"}, 112, C=11),
    ]


def outward(dx, dz):
    """The side a vector points to most."""
    if abs(dx) > abs(dz):
        return E if dx > 0 else W
    return S if dz > 0 else N


def square_ring(x0, z0, x1, z1):
    """Cells round a rectangle's edge, clockwise from the north-west corner."""
    out = [(x, z0) for x in range(x0, x1)]
    out += [(x1, z) for z in range(z0, z1)]
    out += [(x, z1) for x in range(x1, x0, -1)]
    out += [(x0, z) for z in range(z1, z0, -1)]
    return out


def spiral_stairs(c, ring, y0, y1, name, start=0):
    """One step per layer round `ring` (cells in walking order), from y0 up to y1."""
    for y in range(y0, y1 + 1):
        k = (start + y - y0) % len(ring)
        x, z = ring[k]
        nx, nz = ring[(k + 1) % len(ring)]
        c.set(x, y, z, stair(name, outward(nx - x, nz - z)))
        c.set(x, y + 1, z, AIR)
        c.set(x, y + 2, z, AIR)
    return (start + y1 - y0 + 1) % len(ring)


def tent(c, x0, z0, length, wool, along="z", y=1, pole=None):
    """An A-frame tent of wool, open at both ends."""
    for k in range(length):
        for (dx, dy) in ((0, 0), (1, 1), (2, 2), (3, 1), (4, 0)):
            x, z = (x0 + dx, z0 + k) if along == "z" else (x0 + k, z0 + dx)
            c.set(x, y + dy, z, wool)
        for (dx, dy) in ((1, 0), (2, 0), (3, 0), (2, 1)):
            x, z = (x0 + dx, z0 + k) if along == "z" else (x0 + k, z0 + dx)
            c.set(x, y + dy, z, AIR)
    if pole:
        for k in (-1, length):
            x, z = (x0 + 2, z0 + k) if along == "z" else (x0 + k, z0 + 2)
            c.column(x, z, y, y + 2, pole)


def target_dummy(c, x, y, z, fence):
    c.set(x, y, z, fence)
    c.set(x, y + 1, z, "hay_block")
    c.set(x, y + 2, z, "pumpkin")


def flag(c, x, y, z, fence, wool, h=5, f=E):
    c.column(x, z, y, y + h - 1, fence)
    dx, dz = DV[f]
    for k in (1, 2):
        for j in (0, 1):
            c.set(x + dx * k, y + h - 2 - j, z + dz * k, wool)


OUTPOST_LOOT = [("arrow", 6, 16, 0.8), ("emerald", 2, 6, 0.55), ("iron_ingot", 2, 5, 0.5), ("bow", 1, 1, 0.45),
                ("iron_sword", 1, 1, 0.3), ("bread", 2, 5, 0.5), ("tnt", 1, 3, 0.25),
                ("diamond", 1, 2, 0.12), ("gold_ingot", 1, 4, 0.35), ("iron_chestplate", 1, 1, 0.12)]


def outpost(name, biomes, wood, stone, roof, wool, extra=None, seed=0):
    """A raiders' outpost: a timber tower on a stone footing with a spiral stair, a roofed
    lookout and a chest of plunder at the top; tents, a cage and targets round it."""
    rng = random.Random(seed)
    wd = wood_set(wood)
    Wd = 25
    c = Canvas(Wd, 30, Wd)
    t0, t1 = 9, 15  # tower walls
    # Ground patch of stone and gravel under the camp.
    for z in range(Wd):
        for x in range(Wd):
            d = ((x - 12) ** 2 + (z - 12) ** 2) ** 0.5
            if d < 11.5:
                c.set(x, 0, z, rng.choice([stone, "gravel", "rocky_dirt", "cobblestone"]) if d < 7 or rng.random() < 0.5 else None)
                c.box(x, 1, z, x, 5, z, AIR)
    H = 17
    c.room(t0, 0, t0, t1, H, t1, wd["planks"], floor=stone)
    c.walls(t0, t0, t1, t1, 1, 3, stone)
    for (x, z) in ((t0, t0), (t1, t0), (t0, t1), (t1, t1)):
        c.column(x, z, 0, H + 1, wd["log"])
    # Arrow slits.
    for y in (6, 10, 14):
        for (x, z) in ((12, t0), (12, t1), (t0, 12), (t1, 12)):
            c.set(x, y, z, wd["fence"])
    # The way in, and the stair round the inside.
    c.set(12, 1, t1, door(wd["door"], S)[0])
    c.set(12, 2, t1, door(wd["door"], S)[1])
    ring = square_ring(t0 + 1, t0 + 1, t1 - 1, t1 - 1)
    # Start the stair just inside the door, going round toward the east.
    start = ring.index((12, t1 - 1))
    start = (start + 2) % len(ring)
    spiral_stairs(c, ring, 1, H - 1, wd["stairs"], start)
    c.set(12, 1, 12, "chest")
    c.set(12, 5, 12, HANG)
    # Lookout deck, wider than the tower, with a fence rail and a roof on posts.
    c.box(t0 - 1, H, t0 - 1, t1 + 1, H, t1 + 1, wd["planks"])
    last = (start + H - 2) % len(ring)
    for k in (last, (last - 1) % len(ring), (last - 2) % len(ring)):
        c.set(ring[k][0], H, ring[k][1], AIR)
    nx, nz = ring[(last + 1) % len(ring)]
    c.set(nx, H, nz, stair(wd["stairs"], outward(ring[(last + 2) % len(ring)][0] - nx, ring[(last + 2) % len(ring)][1] - nz)))
    c.walls(t0 - 1, t0 - 1, t1 + 1, t1 + 1, H + 1, H + 1, wd["fence"])
    for (x, z) in ((t0 - 1, t0 - 1), (t1 + 1, t0 - 1), (t0 - 1, t1 + 1), (t1 + 1, t1 + 1)):
        c.column(x, z, H + 1, H + 3, wd["log"])
    hip(c, t0 - 1, t0 - 1, t1 + 1, t1 + 1, H + 4, roof, wd["slab"], over=1)
    c.set(12, H + 1, t0, "chest")
    c.set(t1, H + 1, t1, "lantern")
    c.set(12, H + 3, 12, HANG)
    flag(c, 12, H + 8, 12, wd["fence"], wool, h=4)
    # Banners of wool hanging down the tower's front.
    for y in range(9, 14):
        c.set(11, y, t1 + 1, wool if y > 9 else "black_wool")
        c.set(13, y, t1 + 1, wool if y > 9 else "black_wool")
    # Camp: two tents, a cage, targets, a log pile and lamps.
    tent(c, 1, 3, 4, wool, "z", pole=wd["fence"])
    tent(c, 18, 19, 4, "white_wool", "x", pole=wd["fence"])
    cx0, cz0 = 2, 17
    c.box(cx0, 0, cz0, cx0 + 4, 0, cz0 + 4, "cobblestone")
    c.walls(cx0, cz0, cx0 + 4, cz0 + 4, 1, 3, "iron_bars")
    for (x, z) in ((cx0, cz0), (cx0 + 4, cz0), (cx0, cz0 + 4), (cx0 + 4, cz0 + 4)):
        c.column(x, z, 1, 3, wd["log"])
    c.box(cx0, 4, cz0, cx0 + 4, 4, cz0 + 4, wd["slab"])
    for (x, z) in ((19, 4), (21, 7), (19, 10)):
        target_dummy(c, x, 1, z, wd["fence"])
    for x in range(17, 21):
        c.set(x, 1, 15, wd["log"])
    c.set(18, 2, 15, wd["log"])
    for (x, z) in ((7, 7), (17, 17), (7, 17)):
        lamp_post(c, x, 1, z, wd["fence"], "lantern", h=2)
    c.set(6, 1, 12, "brazier")
    if extra:
        extra(c, rng)
    return struct(name, biomes, c, 26, 0.45, loot=OUTPOST_LOOT, sink=1, foundation=stone, rough=6,
                  mobs=[("marauder", 3), ("brute", 1)])


@group("ctov")
def outposts():
    def snow(c, rng):
        for z in range(c.d):
            for x in range(c.w):
                if c.get(x, 1, z) == AIR and c.get(x, 0, z) not in (None, AIR) and rng.random() < 0.35:
                    c.set(x, 1, z, "snow_layer")

    def cactus(c, rng):
        for (x, z) in ((3, 12), (22, 2), (21, 22)):
            c.column(x, z, 1, 2, "cactus")

    def vines(c, rng):
        for y in range(2, 16):
            for (x, z) in ((8, 10), (8, 13), (16, 11)):
                if rng.random() < 0.6:
                    c.set(x, y, z, "vine")
    return [
        outpost("posto_saqueador_da_planicie", ["Plains", "Forest", "Birch Forest"], "dark_oak", "cobblestone", "spruce_roof", "gray_wool", seed=1),
        outpost("posto_saqueador_do_deserto", ["Desert"], "birch", "sandstone", "oak_roof", "orange_wool", cactus, seed=2),
        outpost("posto_saqueador_da_taiga", ["Taiga"], "spruce", "mossy_cobblestone", "spruce_roof", "brown_wool", seed=3),
        outpost("posto_saqueador_nevado", ["Snowy Plains"], "spruce", "cobblestone_bricks", "spruce_roof", "light_gray_wool", snow, seed=4),
        outpost("posto_saqueador_da_savana", ["Savanna"], "acacia", "cobblestone", "thatch_roof", "orange_wool", seed=5),
        outpost("posto_saqueador_das_badlands", ["Badlands"], "dark_oak", "red_terracotta", "brick_roof", "red_wool", cactus, seed=6),
        outpost("posto_saqueador_da_selva", ["Jungle"], "jungle", "mossy_cobblestone", "thatch_roof", "green_wool", vines, seed=7),
        outpost("posto_saqueador_da_montanha", ["Mountains"], "spruce", "stone_bricks", "stone_brick_roof", "black_wool", seed=8),
    ]


# ---------------------------------------------------------------------------------------------
# When Dungeons Arise idea: huge, rare structures full of dwellers and good loot.
# ---------------------------------------------------------------------------------------------

GOOD_LOOT = [("diamond", 1, 3, 0.45), ("emerald", 2, 6, 0.6), ("gold_ingot", 2, 7, 0.7), ("iron_ingot", 3, 8, 0.7),
             ("life_crystal", 1, 1, 0.3), ("mana_crystal", 1, 2, 0.35), ("diamond_sword", 1, 1, 0.12), ("diamond_chestplate", 1, 1, 0.08),
             ("apple", 2, 5, 0.5), ("ender_pearl", 1, 3, 0.3), ("gold_backpack", 1, 1, 0.08),
             ("iron_backpack", 1, 1, 0.12)]


def rare(*extra):
    return list(GOOD_LOOT) + list(extra)


@group("wda")
def coliseum():
    """A great round arena: sand floor, tiers of stone seats, a tall arched outer wall, two
    gates and the lord's box with the treasure."""
    rng = random.Random(21)
    Sz = 57
    cx = cz = 28
    c = Canvas(Sz, 18, Sz)
    R_ARENA, R_SEAT, R_WALL = 13.5, 23.5, 27.5
    for z in range(Sz):
        for x in range(Sz):
            d = ((x - cx) ** 2 + (z - cz) ** 2) ** 0.5
            if d > R_WALL + 0.3:
                continue
            c.set(x, 0, z, "sand" if d <= R_ARENA else "stone_bricks")
            c.box(x, 1, z, x, 17, z, AIR)
            f = outward(x - cx, z - cz)
            if R_ARENA < d <= R_ARENA + 1:
                c.column(x, z, 1, 3, "stone_bricks")  # the arena's wall
                c.set(x, 4, z, "cobblestone_wall")
            elif R_ARENA + 1 < d <= R_SEAT:
                h = 3 + int((d - R_ARENA - 1) / 1.5)
                c.column(x, z, 1, h - 1, "stone_bricks")
                c.set(x, h, z, stair("stone_brick_stairs", f))
            elif R_SEAT < d <= R_WALL:
                c.column(x, z, 1, 15, "stone_bricks")
                ang = (degrees(atan2(z - cz, x - cx)) + 360) % 360
                arch = int(ang / 12) % 2 == 0
                if d > R_WALL - 1.2:
                    if arch:
                        c.column(x, z, 2, 4, "iron_bars")
                        c.column(x, z, 9, 11, "iron_bars")
                    if int(ang / 6) % 2 == 0:
                        c.set(x, 16, z, "stone_bricks")
                elif d <= R_SEAT + 1:
                    c.set(x, 13, z, "stone_brick_slab")
                    c.column(x, z, 14, 15, AIR)
                else:
                    # A walkway inside the wall at the top.
                    c.column(x, z, 14, 15, AIR)
    # Columns and bands on the outside.
    for k in range(0, 360, 12):
        a = k * pi / 180
        x, z = round(cx + (R_WALL + 0.6) * cos(a)), round(cz + (R_WALL + 0.6) * sin(a))
        c.column(x, z, 1, 15, "smooth_stone")
    c.disc(cx, cz, R_WALL + 0.6, 6, "stone_brick_slab", hollow=True)
    # Two gates through the seats, north and south.
    for sgn in (-1, 1):
        for k in range(14, 29):
            z = cz + sgn * k
            for x in range(cx - 2, cx + 3):
                c.set(x, 0, z, "cobblestone")
                c.box(x, 1, z, x, 5, z, AIR)
                c.set(x, 6, z, "stone_bricks")
            c.set(cx - 3, 6, z, "stone_bricks")
            c.set(cx + 3, 6, z, "stone_bricks")
            for x in (cx - 3, cx + 3):
                c.column(x, z, 1, 5, "stone_bricks" if k % 4 else "smooth_stone")
            if k % 4 == 0:
                c.set(cx, 5, z, HANG)
    # The lord's box on the east side: a platform with a canopy, a chest and lanterns.
    bx = cx + 20
    c.box(bx - 2, 1, cz - 3, bx + 2, 9, cz + 3, "stone_bricks")
    c.box(bx - 2, 10, cz - 3, bx + 2, 12, cz + 3, AIR)
    c.walls(bx - 2, cz - 3, bx + 2, cz + 3, 10, 10, "cobblestone_wall")
    c.set(bx - 2, 10, cz, AIR)
    for (x, z) in ((bx - 2, cz - 3), (bx - 2, cz + 3), (bx + 2, cz - 3), (bx + 2, cz + 3)):
        c.column(x, z, 10, 12, "smooth_stone")
    c.box(bx - 2, 13, cz - 3, bx + 2, 13, cz + 3, "red_wool")
    c.set(bx + 1, 10, cz, "chest")
    c.set(bx + 1, 10, cz - 1, "gold_block")
    c.set(bx + 1, 10, cz + 1, "gold_block")
    c.set(bx, 10, cz, var("oak_chair", W))
    c.set(bx, 12, cz - 2, HANG)
    c.set(bx, 12, cz + 2, HANG)
    # In the arena: braziers, a few broken pillars, weapon racks.
    for (x, z) in ((cx - 8, cz - 8), (cx + 8, cz - 8), (cx - 8, cz + 8), (cx + 8, cz + 8)):
        c.column(x, z, 1, rng.randint(2, 4), "smooth_stone")
    for (x, z) in ((cx, cz - 5), (cx, cz + 5), (cx - 5, cz), (cx + 5, cz)):
        c.set(x, 1, z, "brazier")
    c.set(cx - 12, 1, cz, "oak_fence")
    c.set(cx - 12, 2, cz, "iron_bars")
    c.weather(rng, {"stone_bricks": ["mossy_cobblestone_bricks", "cobblestone_bricks", "cobblestone"]}, 0.12)
    return struct("coliseu", ["Plains", "Savanna", "Desert"], c, 64, 0.5, loot=rare(("iron_sword", 1, 1, 0.4), ("golden_sword", 1, 1, 0.3)),
                  sink=1, foundation="stone_bricks", rough=8,
                  mobs=[("zombie", 4), ("skeleton", 4), ("brute", 1), ("mutant_zombie", 1)])


def circle_ring(cx, cz, r):
    """Cells about r from a centre, in order round it (for round spiral stairs)."""
    cells = []
    for z in range(int(cz - r - 2), int(cz + r + 3)):
        for x in range(int(cx - r - 2), int(cx + r + 3)):
            d = ((x - cx) ** 2 + (z - cz) ** 2) ** 0.5
            if r - 0.5 <= d < r + 0.5:
                cells.append((atan2(z - cz, x - cx), x, z))
    cells.sort()
    return [(x, z) for _, x, z in cells]


def round_tower(c, cx, cz, r, y0, y1, wall, floor=None, crenel=None, roof=None, roof_h=0, windows=None, every=6):
    """A round tower: hollow walls, optional floors every few layers, battlements or a cone."""
    for y in range(y0, y1 + 1):
        c.disc(cx, cz, r - 1, y, AIR)
        c.disc(cx, cz, r, y, wall, hollow=True, thick=1.2)
    c.disc(cx, cz, r, y0, floor or wall)
    if windows:
        for y in range(y0 + 3, y1 - 1, every):
            for (dx, dz) in ((0, -1), (0, 1), (-1, 0), (1, 0)):
                c.set(round(cx + dx * r), y, round(cz + dz * r), windows)
    if crenel:
        for (x, z) in circle_ring(cx, cz, r)[::2]:
            c.set(x, y1 + 1, z, crenel)
    if roof:
        for k in range(roof_h):
            rr = (r + 1) * (1 - k / roof_h)
            c.disc(cx, cz, max(rr, 0.4), y1 + 1 + k, roof)


def bite(c, cx, cy, cz, r, floor=1):
    """Take a round bite out of a ruin."""
    for y in range(max(floor, int(cy - r)), int(cy + r) + 1):
        for z in range(int(cz - r), int(cz + r) + 1):
            for x in range(int(cx - r), int(cx + r) + 1):
                if (x - cx) ** 2 + (y - cy) ** 2 + (z - cz) ** 2 <= r * r and c.get(x, y, z) not in (None,):
                    c.set(x, y, z, AIR)


def remove_floaters(c, y0=2):
    """After bites: drop blocks left hanging in the air with nothing under or beside them."""
    changed = True
    while changed:
        changed = False
        for y in range(y0, c.h):
            for z in range(c.d):
                for x in range(c.w):
                    b = c.g[y][z][x]
                    if b in (None, AIR) or c.get(x, y - 1, z) not in (AIR,) or c.get(x, y + 1, z) not in (None, AIR):
                        continue
                    side = sum(1 for (dx, dz) in ((1, 0), (-1, 0), (0, 1), (0, -1)) if c.get(x + dx, y, z + dz) not in (None, AIR))
                    if side == 0:
                        c.g[y][z][x] = AIR
                        changed = True


def overgrow(c, rng, p, plant="vine", y0=2):
    """Vines hanging on the outside of walls."""
    solid_names = None
    for y in range(y0, c.h):
        for z in range(c.d):
            for x in range(c.w):
                if c.g[y][z][x] != AIR:
                    continue
                for (dx, dz) in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    b = c.get(x + dx, y, z + dz)
                    if b not in (None, AIR, plant) and ":" not in b and "fence" not in b and rng.random() < p:
                        c.g[y][z][x] = plant
                        break


def stair_run(c, x, y, z, f, n, name, clear=3):
    """A straight flight of n steps going up toward side f, starting at (x, y, z)."""
    dx, dz = DV[f]
    for k in range(n):
        c.set(x + dx * k, y + k, z + dz * k, stair(name, f))
        for j in range(1, clear + 1):
            c.set(x + dx * k, y + k + j, z + dz * k, AIR)


@group("wda")
def ruined_keep():
    """A ruined castle: curtain walls with round corner towers and a gatehouse round a
    courtyard, and a three-storey keep in the middle with a great hall, barracks and an armoury."""
    rng = random.Random(22)
    Sz = 50
    c = Canvas(Sz, 30, Sz)
    SB, MC, CB = "stone_bricks", "mossy_cobblestone", "cobblestone_bricks"
    # Courtyard ground.
    for z in range(2, Sz - 2):
        for x in range(2, Sz - 2):
            c.set(x, 0, z, rng.choice(["gravel", "cobblestone", "grass_block", "grass_block", "rocky_dirt"]))
            c.box(x, 1, z, x, 12, z, AIR)
    # Curtain walls, two thick, with a walkway and battlements.
    for t in (0, 1):
        c.walls(4 + t, 4 + t, Sz - 5 - t, Sz - 5 - t, 0, 9, SB)
    c.walls(4, 4, Sz - 5, Sz - 5, 10, 10, SB)
    for (x, z) in square_ring(4, 4, Sz - 5, Sz - 5)[::2]:
        c.set(x, 11, z, SB)
    # Round corner towers.
    for (x, z) in ((5, 5), (Sz - 6, 5), (5, Sz - 6), (Sz - 6, Sz - 6)):
        round_tower(c, x, z, 4.5, 0, 14, SB, floor="cobblestone", crenel=SB, windows="iron_bars", every=5)
        c.disc(x, z, 3.4, 8, "spruce_planks")
        c.set(x, 13, z, HANG)
    # Gatehouse in the south wall with a portcullis.
    gx = Sz // 2
    c.room(gx - 6, 0, Sz - 9, gx + 6, 13, Sz - 2, SB, floor="cobblestone")
    c.box(gx - 2, 1, Sz - 9, gx + 2, 5, Sz - 2, AIR)
    c.box(gx - 5, 7, Sz - 8, gx + 5, 12, Sz - 3, AIR)
    c.box(gx - 5, 6, Sz - 8, gx + 5, 6, Sz - 3, "spruce_planks")
    c.box(gx - 2, 6, Sz - 6, gx + 2, 6, Sz - 5, AIR)
    for x in range(gx - 2, gx + 3):
        c.set(x, 5, Sz - 4, "iron_bars")
        c.set(x, 4, Sz - 4, "iron_bars")
    for (x, z) in square_ring(gx - 6, Sz - 9, gx + 6, Sz - 2)[::2]:
        c.set(x, 14, z, SB)
    # Steps up to the wall walk on the inside of the west and east walls.
    stair_run(c, 6, 1, 20, N, 9, "stone_brick_stairs")
    stair_run(c, Sz - 7, 1, 30, S, 9, "stone_brick_stairs")
    # The keep.
    kx0, kz0, kx1, kz1 = 16, 13, 33, 30
    for y0 in (0, 7, 14):
        c.room(kx0, y0, kz0, kx1, y0 + 7, kz1, SB, floor="spruce_planks" if y0 else "stone_bricks", ceil="spruce_planks")
    c.box(kx0, 21, kz0, kx1, 21, kz1, SB)
    for (x, z) in square_ring(kx0, kz0, kx1, kz1)[::2]:
        c.set(x, 22, z, SB)
    for (x, z) in ((kx0, kz0), (kx1, kz0), (kx0, kz1), (kx1, kz1)):
        c.box(x - 1, 0, z - 1, x + 1, 24, z + 1, SB)
        c.set(x, 25, z, "stone_brick_slab")
        c.set(x - 1, 25, z - 1, SB)
        c.set(x + 1, 25, z + 1, SB)
    for y in (3, 10, 17):
        for x in range(kx0 + 3, kx1 - 1, 3):
            c.set(x, y, kz0, "glass_pane")
            c.set(x, y, kz1, "glass_pane")
        for z in range(kz0 + 3, kz1 - 1, 3):
            c.set(kx0, y, z, "glass_pane")
            c.set(kx1, y, z, "glass_pane")
    # Main door facing the gate, with steps and lamps.
    kmx = (kx0 + kx1) // 2
    for x in (kmx, kmx + 1):
        c.box(x, 1, kz1, x, 3, kz1, AIR)
    c.set(kmx, 1, kz1, door("spruce_door", S)[0])
    c.set(kmx, 2, kz1, door("spruce_door", S)[1])
    c.set(kmx + 1, 1, kz1, var("spruce_door", S))
    c.set(kmx + 1, 2, kz1, var("spruce_door", S + 8))
    for x in (kmx - 1, kmx + 2):
        c.set(x, 1, kz1 + 1, "lantern")
    # Stairs between floors: along the west wall going north, then the east wall going south.
    stair_run(c, kx0 + 1, 1, kz1 - 2, N, 7, "spruce_stairs")
    c.box(kx0 + 1, 7, kz1 - 8, kx0 + 1, 7, kz1 - 3, AIR)
    stair_run(c, kx1 - 1, 8, kz0 + 2, S, 7, "spruce_stairs")
    c.box(kx1 - 1, 14, kz0 + 3, kx1 - 1, 14, kz0 + 8, AIR)
    # Great hall: a long table with chairs, a throne, chandeliers and banners.
    for z in range(kz0 + 4, kz1 - 3):
        c.set(kmx, 1, z, "spruce_table")
        c.set(kmx + 1, 1, z, "spruce_table")
        c.set(kmx - 1, 1, z, var("spruce_chair", E))
        c.set(kmx + 2, 1, z, var("spruce_chair", W))
    for z in range(kz1 - 2, kz1):
        c.set(kmx, 0, z, "red_wool")
        c.set(kmx + 1, 0, z, "red_wool")
    c.set(kmx, 1, kz0 + 2, "gold_block")
    c.set(kmx, 2, kz0 + 2, stair("quartz_stairs", N))
    for z in range(kz0 + 5, kz1 - 3, 5):
        c.set(kmx, 6, z, "chain")
        c.set(kmx, 5, z, "chandelier")
        c.set(kmx + 1, 6, z, "chain")
        c.set(kmx + 1, 5, z, "chandelier")
    for z in range(kz0 + 2, kz1 - 1, 4):
        for x in (kx0 + 1, kx1 - 1):
            if c.get(x, 3, z) == AIR:
                c.set(x, 4, z, "red_wool")
                c.set(x, 3, z, "red_wool")
    # Barracks above: beds, drawers and the chest.
    for z in range(kz0 + 2, kz1 - 1, 2):
        c.set(kx0 + 4, 8, z, "red_bed")
        c.set(kx1 - 4, 8, z, "red_bed")
        c.set(kx0 + 5, 8, z, "spruce_drawer")
    c.set(kmx, 8, kz0 + 1, "chest")
    c.set(kmx, 12, (kz0 + kz1) // 2, HANG)
    # Armoury and library on top.
    for x in range(kx0 + 3, kx1 - 2):
        c.set(x, 15, kz0 + 1, "bookshelf")
        c.set(x, 16, kz0 + 1, "bookshelf")
    c.set(kmx, 15, kz1 - 2, "chest")
    for x in range(kx0 + 4, kx1 - 3, 3):
        c.set(x, 15, kz1 - 1, "iron_block")
        c.set(x + 1, 15, kz1 - 1, "spruce_table")
    c.set(kmx, 19, (kz0 + kz1) // 2, HANG)
    # Courtyard bits: a well, a stable and targets.
    c.box(10, 0, 36, 12, 1, 38, "cobblestone")
    c.set(11, 1, 37, "water")
    c.set(11, 0, 37, "water")
    for x in range(36, 44):
        c.set(x, 1, 10, "oak_fence")
        c.set(x, 1, 16, "oak_fence")
    for z in range(10, 17):
        c.set(43, 1, z, "oak_fence")
    for x in range(36, 43, 2):
        c.set(x + 1, 1, 11, "hay_block")
    c.box(36, 4, 9, 44, 4, 17, "spruce_slab")
    for (x, z) in ((36, 9), (44, 9), (36, 17), (44, 17)):
        c.column(x, z, 1, 3, "spruce_log")
    for (x, z) in ((38, 38), (41, 38)):
        target_dummy(c, x, 1, z, "oak_fence")
    # Ruin it: bites out of walls and towers, moss, vines.
    for (x, y, z, r) in ((4, 9, 24, 3.5), (Sz - 5, 8, 14, 4), (20, 10, 4, 3), (Sz - 6, 14, Sz - 6, 3.5), (5, 13, 5, 2.5),
                         (34, 9, Sz - 5, 3), (kx1, 22, kz0, 3), (kx0 + 2, 22, kz1 - 1, 2.5)):
        bite(c, x, y, z, r)
    remove_floaters(c)
    c.weather(rng, {SB: [MC, CB, "cobblestone", "mossy_cobblestone_bricks"]}, 0.22)
    overgrow(c, rng, 0.05)
    return struct("fortaleza_em_ruinas", ["Plains", "Forest", "Dark Forest", "Taiga", "Mountains"], c, 60, 0.45,
                  loot=rare(("iron_sword", 1, 1, 0.35), ("iron_helmet", 1, 1, 0.3), ("bow", 1, 1, 0.3), ("arrow", 8, 20, 0.6)),
                  sink=1, foundation="stone_bricks", rough=8,
                  mobs=[("skeleton", 4), ("zombie", 4), ("marauder", 2), ("mutant_skeleton", 1)])


def dome(c, cx, cy, cz, r, b, top=None):
    for k in range(int(r) + 1):
        rr = (r * r - k * k) ** 0.5
        c.disc(cx, cz, rr, cy + k, b, hollow=True, thick=1.3)
        if rr > 1.5:
            c.disc(cx, cz, rr - 1.2, cy + k, AIR)
    if top:
        c.set(round(cx), cy + int(r) + 1, round(cz), top)


@group("wda")
def desert_palace():
    """A sandstone palace: a raised court behind walls with four minarets, a pool garden with
    palms and a domed throne hall with the treasure behind bars."""
    rng = random.Random(23)
    Sz = 56
    c = Canvas(Sz, 40, Sz)
    SS, SM, TC = "sandstone", "smooth_stone", "orange_terracotta"
    c.box(0, 0, 0, Sz - 1, 1, Sz - 1, SS)
    c.box(1, 2, 1, Sz - 2, 30, Sz - 2, AIR)
    c.box(2, 1, 2, Sz - 3, 1, Sz - 3, SM)
    # Outer wall with a terracotta band and slab battlements.
    c.walls(2, 2, Sz - 3, Sz - 3, 2, 9, SS)
    c.walls(2, 2, Sz - 3, Sz - 3, 6, 6, TC)
    for (x, z) in square_ring(2, 2, Sz - 3, Sz - 3)[::2]:
        c.set(x, 10, z, "sandstone_slab")
    # Gate to the south, steps up to it.
    gx = Sz // 2
    c.box(gx - 2, 2, Sz - 3, gx + 2, 6, Sz - 3, AIR)
    c.box(gx - 3, 7, Sz - 4, gx + 3, 9, Sz - 2, SS)
    c.box(gx - 2, 2, Sz - 4, gx + 2, 6, Sz - 2, AIR)
    for x in range(gx - 2, gx + 3):
        c.set(x, 1, Sz - 1, stair("sandstone_stairs", N))
    c.set(gx - 3, 10, Sz - 3, "gold_block")
    c.set(gx + 3, 10, Sz - 3, "gold_block")
    # Minarets on the corners.
    for (x, z) in ((4, 4), (Sz - 5, 4), (4, Sz - 5), (Sz - 5, Sz - 5)):
        round_tower(c, x, z, 2.6, 2, 26, SS, windows="jungle_window", every=6)
        c.disc(x, z, 3.8, 20, "sandstone_slab")
        c.disc(x, z, 3.8, 21, "jungle_fence", hollow=True)
        c.disc(x, z, 1.6, 20, AIR)
        for y in (12, 13, 22, 23):
            c.disc(x, z, 2.6, y, TC, hollow=True)
        dome(c, x, 27, z, 2.6, "yellow_terracotta", top="gold_block")
        c.set(x, 25, z, HANG)
    # Pool garden in the south court, with palms.
    c.box(gx - 7, 1, 34, gx + 7, 1, 46, "smooth_stone")
    c.box(gx - 5, 1, 36, gx + 5, 1, 44, "water")
    c.walls(gx - 6, 35, gx + 6, 45, 2, 2, "sandstone_slab")
    for x in (gx - 6, gx + 6):
        c.set(x, 2, 40, "sandstone_slab")
    for (px, pz) in ((9, 38), (Sz - 10, 38), (10, 48), (Sz - 11, 48), (9, 30), (Sz - 10, 30)):
        c.set(px, 1, pz, "grass_block")
        h = rng.randint(6, 8)
        c.column(px, pz, 2, 1 + h, "jungle_log")
        top = 2 + h
        c.set(px, top, pz, "jungle_leaves")
        for (dx, dz) in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            for k in (1, 2, 3):
                c.set(px + dx * k, top - (1 if k == 3 else 0), pz + dz * k, "jungle_leaves")
        for (dx, dz) in ((1, 1), (-1, 1), (1, -1), (-1, -1)):
            c.set(px + dx, top, pz + dz, "jungle_leaves")
    for (x, z) in ((gx - 9, 34), (gx + 9, 34), (gx - 9, 46), (gx + 9, 46)):
        c.set(x, 2, z, "brazier")
    # The throne hall: colonnade in front, dome over the middle.
    hx0, hz0, hx1, hz1 = 10, 8, Sz - 11, 28
    c.room(hx0, 1, hz0, hx1, 13, hz1, SS, floor=SM, ceil=SS)
    c.walls(hx0, hz0, hx1, hz1, 9, 9, TC)
    for (x, z) in square_ring(hx0, hz0, hx1, hz1)[::2]:
        c.set(x, 14, z, "sandstone_slab")
    for x in range(hx0 + 2, hx1 - 1, 3):
        c.box(x, 3, hz1, x, 6, hz1, "jungle_window")
    for x in range(hx0, hx1 + 1, 3):
        c.column(x, hz1 + 3, 2, 8, SM)
        c.set(x, 9, hz1 + 3, "sandstone")
    c.box(hx0, 9, hz1 + 1, hx1, 9, hz1 + 3, "sandstone_slab")
    c.box(gx - 2, 2, hz1, gx + 1, 6, hz1, AIR)
    for x in range(gx - 2, gx + 2):
        c.set(x, 1, hz1 + 1, "red_wool")
    # Red carpet to the throne.
    for z in range(hz0 + 3, hz1 + 1):
        for x in (gx - 1, gx):
            c.set(x, 1, z, "red_wool")
    c.box(gx - 3, 2, hz0 + 1, gx + 2, 2, hz0 + 3, "sandstone_slab")
    c.set(gx - 1, 3, hz0 + 2, "gold_block")
    c.set(gx, 3, hz0 + 2, "gold_block")
    c.set(gx - 1, 4, hz0 + 2, stair("quartz_stairs", N))
    c.set(gx, 4, hz0 + 2, stair("quartz_stairs", N))
    for x in range(hx0 + 2, hx1 - 1, 4):
        for z in (hz0 + 6, hz1 - 4):
            c.column(x, z, 2, 12, SM)
    for x in range(hx0 + 4, hx1 - 3, 6):
        c.set(x, 12, (hz0 + hz1) // 2, "chain")
        c.set(x, 11, (hz0 + hz1) // 2, "chandelier")
    # Treasure rooms behind bars on both ends.
    for (tx0, tx1) in ((hx0 + 1, hx0 + 5), (hx1 - 5, hx1 - 1)):
        c.box(tx0 + (4 if tx0 < gx else 0), 2, hz0 + 1, tx0 + (4 if tx0 < gx else 0), 5, hz1 - 1, "iron_bars")
        for z in range(hz0 + 2, hz1 - 1, 3):
            c.set((tx0 + tx1) // 2, 2, z, "gold_block" if z % 2 else "chest")
    # The dome.
    c.box(gx - 7, 14, 11, gx + 6, 14, 25, SS)
    dome(c, gx - 0.5, 15, 18, 7.5, "yellow_terracotta", top="gold_block")
    c.disc(gx - 0.5, 18, 6.0, 13, AIR)
    c.set(gx - 1, 21, 18, HANG)
    # Servants' rooms on the sides of the court.
    for (x0, x1) in ((4, 12), (Sz - 13, Sz - 5)):
        c.room(x0, 1, 31, x1, 6, 50, SS, floor=SM, ceil="sandstone_slab")
        for z in range(33, 50, 3):
            c.set(x0 if x0 > gx else x1, 3, z, "jungle_window")
            c.set((x0 + x1) // 2, 2, z, "red_bed")
            c.set((x0 + x1) // 2 + 1, 2, z, "jungle_drawer")
        c.box(x0 if x0 > gx else x1, 2, 40, x0 if x0 > gx else x1, 3, 41, AIR)
        c.set((x0 + x1) // 2, 5, 40, HANG)
    return struct("palacio_do_deserto", ["Desert"], c, 64, 0.5,
                  loot=rare(("gold_ingot", 4, 10, 0.8), ("gold_block", 1, 2, 0.2), ("golden_sword", 1, 1, 0.3), ("ruby", 1, 3, 0.3), ("topaz", 1, 3, 0.3)),
                  sink=1, foundation="sandstone", rough=7,
                  mobs=[("zombie", 4), ("skeleton", 3), ("invoker", 1), ("brute", 1)])


@group("wda")
def lighthouse():
    """A striped lighthouse on a rocky base by the sea, a spiral stair up to the lamp room, a
    gallery round it and a green copper cap; the keeper's cottage and a jetty beside it."""
    rng = random.Random(24)
    Sz = 27
    c = Canvas(Sz, 50, Sz)
    cx, cz = 13, 11
    # Rocky base.
    for z in range(Sz):
        for x in range(Sz):
            d = ((x - cx) ** 2 + (z - cz) ** 2) ** 0.5
            if d < 10.5:
                c.set(x, 0, z, rng.choice(["stone", "cobblestone", "mossy_cobblestone", "gravel"]))
                c.box(x, 1, z, x, 6, z, AIR)
                if d < 7.5:
                    c.set(x, 1, z, rng.choice(["stone", "cobblestone", "stone"]))
    c.disc(cx, cz, 6.5, 2, "smooth_stone")
    # The tower, striped.
    top = 36
    for y in range(3, top + 1):
        band = "red_terracotta" if (y // 5) % 2 else "white_terracotta"
        c.disc(cx, cz, 3.2, y, AIR)
        c.disc(cx, cz, 4.4, y, band, hollow=True, thick=1.3)
    c.column(cx, cz, 2, top, "stone_bricks")
    ring = circle_ring(cx, cz, 2)
    spiral_stairs(c, ring, 3, top - 1, "stone_brick_stairs")
    for y in range(6, top - 2, 6):
        for (dx, dz) in ((0, -4), (4, 0), (0, 4), (-4, 0)):
            c.set(cx + dx, y, cz + dz, "glass_pane")
    c.box(cx - 1, 3, cz + 4, cx + 1, 4, cz + 4, AIR)
    c.set(cx, 3, cz + 4, door("oak_door", S)[0])
    c.set(cx, 4, cz + 4, door("oak_door", S)[1])
    c.set(cx - 1, 3, cz + 4, "white_terracotta")
    c.set(cx + 1, 3, cz + 4, "white_terracotta")
    c.set(cx - 1, 4, cz + 4, "white_terracotta")
    c.set(cx + 1, 4, cz + 4, "white_terracotta")
    # Gallery and lamp room.
    g = top + 1
    c.disc(cx, cz, 6.2, g, "stone_brick_slab")
    c.disc(cx, cz, 4.4, g, "stone_bricks")
    last = ring[(top - 1 - 3) % len(ring)]
    c.set(last[0], g, last[1], AIR)
    c.disc(cx, cz, 6.2, g + 1, "iron_bars", hollow=True)
    for y in range(g + 1, g + 5):
        c.disc(cx, cz, 3.2, y, "glass", hollow=True)
    for (x, z) in circle_ring(cx, cz, 3)[::3]:
        c.column(x, z, g + 1, g + 4, "white_terracotta")
    c.box(cx - 1, g + 1, cz - 1, cx, g + 3, cz, "glowstone")
    c.set(cx, g, cz, "stone_bricks")
    for k in range(5):
        c.disc(cx, cz, 4.4 - k, g + 5 + k, "oxidized_copper")
    c.set(cx, g + 10, cz, "chain")
    c.set(cx + 4, g + 1, cz, "lantern")
    # Keeper's cottage.
    st = style("spruce", roof="spruce_roof", floor="spruce_planks", wall="white_terracotta", corner="spruce_log", base="cobblestone")
    cottage(c, 2, 16, 8, 22, st, E, rng, y=2, wh=3, things=["bed", "chest", "table", "furnace", "shelf", "bookshelf"])
    # A jetty out to the sea.
    for z in range(17, Sz):
        for x in (16, 17, 18):
            c.set(x, 1, z, "spruce_slab" if z > 19 else "spruce_planks")
        if z % 3 == 0:
            c.column(15, z, -1, 2, "spruce_log")
            c.column(19, z, -1, 2, "spruce_log")
    c.set(19, 3, Sz - 1, "lantern")
    c.set(15, 3, Sz - 1, "lantern")
    return struct("farol", ["Beach"], c, 48, 0.55, loot=rare(("cod", 2, 6, 0.6), ("fishing_rod", 1, 1, 0.4), ("tuna", 1, 3, 0.3)),
                  sink=1, foundation="stone", rough=7, mobs=[("skeleton", 2), ("zombie", 2)])


@group("wda")
def thorn_tower():
    """A dark tower in three narrowing tiers, bristling with thorns of fence and dripstone,
    wrapped in vines and leaves, with a spiral stair through it and a witch's chest on top."""
    rng = random.Random(25)
    Sz = 33
    c = Canvas(Sz, 66, Sz)
    BS, DL, DP = "blackstone", "dark_oak_log", "dark_oak_planks"
    cx = cz = 16
    # Roots and a mossy footing.
    for z in range(Sz):
        for x in range(Sz):
            d = ((x - cx) ** 2 + (z - cz) ** 2) ** 0.5
            if d < 12:
                c.set(x, 0, z, rng.choice(["moss_block", "mossy_cobblestone", "rocky_dirt", "moss_block"]))
                c.box(x, 1, z, x, 4, z, AIR)
    for (dx, dz) in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)):
        c.line(cx + dx * 6, 4, cz + dz * 6, cx + dx * 13, 0, cz + dz * 13, DL)
    tiers = [(6, 0, 20), (5, 21, 38), (4, 39, 52)]
    for (r, y0, y1) in tiers:
        c.room(cx - r, y0, cz - r, cx + r, y1, cz + r, BS, floor=DP if y0 else BS)
        for (x, z) in ((cx - r, cz - r), (cx + r, cz - r), (cx - r, cz + r), (cx + r, cz + r)):
            c.column(x, z, y0, y1 + 1, DL)
        for y in range(y0 + 4, y1 - 1, 5):
            for (x, z) in ((cx, cz - r), (cx, cz + r), (cx - r, cz), (cx + r, cz)):
                c.set(x, y, z, "iron_bars")
        # A ledge of slabs round the top of each tier.
        c.walls(cx - r - 1, cz - r - 1, cx + r + 1, cz + r + 1, y1 + 1, y1 + 1, "blackstone_slab")
    # The spiral stair through all tiers, round a dark oak core.
    ring = square_ring(cx - 2, cz - 2, cx + 2, cz + 2)
    c.column(cx, cz, 1, 52, DL)
    for y in range(1, 53):
        for (x, z) in ((cx - 1, cz - 1), (cx, cz - 1), (cx + 1, cz - 1), (cx - 1, cz), (cx + 1, cz), (cx - 1, cz + 1), (cx, cz + 1), (cx + 1, cz + 1)):
            if c.get(x, y, z) == DP:
                c.set(x, y, z, AIR)
    spiral_stairs(c, ring, 1, 52, "dark_oak_stairs")
    for (r, y0, y1) in tiers[1:]:
        for (x, z) in ring:
            if c.get(x, y0, z) == DP:
                c.set(x, y0, z, AIR)
    c.box(cx - 1, 1, cz + 6, cx + 1, 3, cz + 6, AIR)
    c.set(cx, 1, cz + 6, door("dark_oak_door", S)[0])
    c.set(cx, 2, cz + 6, door("dark_oak_door", S)[1])
    # Spire.
    for k in range(10):
        r = 5 - k * 0.5
        if r < 0.5:
            break
        c.walls(cx - int(r), cz - int(r), cx + int(r), cz + int(r), 53 + k, 53 + k, "blackstone")
    c.column(cx, cz, 53, 64, "dark_oak_fence")
    c.set(cx + 1, 53, cz + 1, "chest")
    c.set(cx - 1, 53, cz - 1, "dark_oak_table")
    c.set(cx - 1, 54, cz - 1, "lantern")
    c.set(cx + 3, 30, cz + 3, "chest")
    c.set(cx - 4, 1, cz - 4, "red_mushroom")
    # Thorns: spikes sticking out of every face, tipped with dripstone.
    for _ in range(70):
        r, y0, y1 = rng.choice(tiers)
        y = rng.randint(y0 + 2, y1 - 1)
        f = rng.choice([N, E, S, W])
        dx, dz = DV[f]
        along = rng.randint(-r + 1, r - 1)
        x = cx + dx * r + (along if dz else 0)
        z = cz + dz * r + (along if dx else 0)
        n = rng.randint(2, 4)
        for k in range(1, n + 1):
            c.setif(x + dx * k, y - (k // 3), z + dz * k, "dark_oak_fence")
        c.setif(x + dx * (n + 1), y - ((n + 1) // 3), z + dz * (n + 1), "pointed_dripstone")
    # Leaves and vines clinging to it.
    for _ in range(26):
        r, y0, y1 = rng.choice(tiers)
        y = rng.randint(y0 + 1, y1)
        f = rng.choice([N, E, S, W])
        dx, dz = DV[f]
        along = rng.randint(-r, r)
        x = cx + dx * (r + 1) + (along if dz else 0)
        z = cz + dz * (r + 1) + (along if dx else 0)
        for (ax, ay, az) in ((0, 0, 0), (dz, 0, dx), (-dz, 0, -dx), (0, 1, 0), (0, -1, 0)):
            c.setif(x + ax, y + ay, z + az, "dark_oak_leaves")
    overgrow(c, rng, 0.1)
    return struct("torre_de_espinhos", ["Dark Forest", "Swamp"], c, 56, 0.5,
                  loot=rare(("string", 3, 9, 0.6), ("amethyst_wand", 1, 1, 0.2), ("redstone", 4, 10, 0.5), ("glow_berries", 2, 6, 0.4)),
                  sink=1, foundation="mossy_cobblestone", rough=8,
                  mobs=[("spider", 4), ("swamp_witch", 1), ("zombie", 3), ("skeleton", 2)])


def building(c, x0, z0, x1, z1, y, wh, wall, pillar, roof, gable_fill, cap, window, floor, doors=(), door_name="oak_door",
             roof_kind="gable", axis=None, win_y=None, every=2):
    """Walls with pillars, windows and a roof, empty inside. doors: list of sides."""
    c.box(x0, y - 1, z0, x1, y - 1, z1, floor)
    c.room(x0, y, z0, x1, y + wh - 1, z1, wall)
    for (x, z) in ((x0, z0), (x1, z0), (x0, z1), (x1, z1)):
        c.column(x, z, y, y + wh - 1, pillar)
    spots = []
    for f in doors:
        dx, dz = door_spot(x0, z0, x1, z1, f)
        lo, hi = door(door_name, f)
        c.set(dx, y, dz, lo)
        c.set(dx, y + 1, dz, hi)
        for k in range(2, wh - 1):
            c.set(dx, y + k, dz, wall)
        spots.append((dx, dz))
    windows_on(c, x0, z0, x1, z1, win_y or y + 1, window, skip=spots, every=every)
    if wh >= 5:
        windows_on(c, x0, z0, x1, z1, (win_y or y + 1) + 1, window, skip=spots, every=every)
    if roof_kind == "gable":
        ax = axis or ("x" if (x1 - x0) >= (z1 - z0) else "z")
        return gable(c, x0, z0, x1, z1, y + wh, roof, gable_fill, cap, axis=ax)
    if roof_kind == "hip":
        return hip(c, x0, z0, x1, z1, y + wh, roof, cap)
    flat_roof(c, x0, z0, x1, z1, y + wh, cap)
    return y + wh


@group("wda")
def monastery():
    """A mountain monastery: a cloister of arches round a garden, a church with a bell tower,
    a dormitory, a library and a refectory."""
    rng = random.Random(26)
    Wd, Dp = 52, 46
    c = Canvas(Wd, 40, Dp)
    SB, CB, SP = "stone_bricks", "cobblestone_bricks", "spruce_planks"
    c.box(0, 0, 0, Wd - 1, 0, Dp - 1, "stone_bricks")
    c.box(0, 1, 0, Wd - 1, 30, Dp - 1, AIR)
    # Cloister garden in the middle.
    gx0, gz0, gx1, gz1 = 16, 14, 35, 31
    c.box(gx0, 0, gz0, gx1, 0, gz1, "grass_block")
    for x in range(gx0, gx1 + 1):
        c.set(x, 0, (gz0 + gz1) // 2, "gravel")
    for z in range(gz0, gz1 + 1):
        c.set((gx0 + gx1) // 2, 0, z, "gravel")
    mx, mz = (gx0 + gx1) // 2, (gz0 + gz1) // 2
    c.box(mx - 1, 0, mz - 1, mx + 1, 1, mz + 1, CB)
    c.set(mx, 1, mz, "water")
    c.set(mx, 0, mz, "water")
    for (x, z) in ((gx0 + 3, gz0 + 3), (gx1 - 3, gz1 - 3), (gx0 + 3, gz1 - 3), (gx1 - 3, gz0 + 3)):
        c.column(x, z, 1, 4, "cherry_log")
        for dy in (4, 5, 6):
            r = 2.5 if dy < 6 else 1.5
            c.disc(x, z, r, dy, "cherry_leaves")
        c.column(x, z, 1, 4, "cherry_log")
    scatter_plants(c, gx0, gz0, gx1, gz1, rng, ["poppy", "dandelion", "short_grass", "short_grass"], p=0.35)
    # The covered walk round the garden: pillars, a slab roof sloping in.
    ax0, az0, ax1, az1 = gx0 - 3, gz0 - 3, gx1 + 3, gz1 + 3
    for (x, z) in square_ring(gx0 - 1, gz0 - 1, gx1 + 1, gz1 + 1):
        if (x + z) % 3 == 0:
            c.column(x, z, 1, 3, SB)
        c.set(x, 4, z, SB)
    for (x, z) in square_ring(ax0, az0, ax1, az1):
        c.column(x, z, 1, 4, SB)
    for x in range(ax0, ax1 + 1):
        c.set(x, 5, az0 + 1, stair("stone_brick_roof", S))
        c.set(x, 5, az1 - 1, stair("stone_brick_roof", N))
        c.set(x, 5, az0, SB)
        c.set(x, 5, az1, SB)
    for z in range(az0 + 1, az1):
        c.set(ax0 + 1, 5, z, stair("stone_brick_roof", E))
        c.set(ax1 - 1, 5, z, stair("stone_brick_roof", W))
        c.set(ax0, 5, z, SB)
        c.set(ax1, 5, z, SB)
    # Church on the north side with a bell tower.
    top = building(c, 12, 1, 39, az0 - 1, 1, 7, SB, "smooth_stone", "stone_brick_roof", SB, "stone_brick_slab", "glass_pane", "smooth_stone",
                   doors=(S,), door_name="spruce_door", every=3)
    for x in range(14, 38, 3):
        c.set(x, 4, 1, "glass")
        c.set(x, 5, 1, "glass")
    for z in range(3, az0 - 2):
        for x in range(16, 36, 2):
            if x not in (25, 26):
                c.set(x, 1, z, var("spruce_bench", N))
    c.box(24, 1, 2, 27, 1, 3, "smooth_stone")
    c.set(25, 2, 2, "gold_block")
    c.set(26, 2, 2, "chest")
    for x in (20, 31):
        c.set(x, 7, 6, "chain")
        c.set(x, 6, 6, "chandelier")
    tx0, tz0 = 40, 1
    c.room(tx0, 1, tz0, tx0 + 5, 22, tz0 + 5, SB, floor="smooth_stone")
    for y in (19, 20):
        for (x, z) in ((tx0 + 2, tz0), (tx0 + 3, tz0), (tx0 + 2, tz0 + 5), (tx0 + 3, tz0 + 5), (tx0, tz0 + 2), (tx0, tz0 + 3), (tx0 + 5, tz0 + 2), (tx0 + 5, tz0 + 3)):
            c.set(x, y, z, AIR)
    c.set(tx0 + 2, 21, tz0 + 2, "chain")
    c.box(tx0 + 2, 19, tz0 + 2, tx0 + 3, 20, tz0 + 3, "gold_block")
    c.set(tx0 + 2, 21, tz0 + 3, "chain")
    hip(c, tx0, tz0, tx0 + 5, tz0 + 5, 23, "stone_brick_roof", "stone_brick_slab")
    spiral_stairs(c, square_ring(tx0 + 1, tz0 + 1, tx0 + 4, tz0 + 4), 2, 17, "stone_brick_stairs")
    c.box(tx0 + 1, 18, tz0 + 1, tx0 + 4, 18, tz0 + 4, SP)
    c.set(tx0 + 1, 18, tz0 + 1, AIR)
    c.set(tx0 + 2, 18, tz0 + 1, AIR)
    c.box(tx0, 1, tz0 + 2, tx0, 2, tz0 + 3, AIR)
    # Dormitory (west), library (east), refectory (south).
    building(c, 1, az0, ax0 - 1, az1, 1, 4, SP, "spruce_log", "spruce_roof", SP, "spruce_slab", "spruce_window", "spruce_planks",
             doors=(E,), door_name="spruce_door", axis="z")
    for z in range(az0 + 2, az1 - 1, 2):
        c.set(2, 1, z, "red_bed")
        c.set(3, 1, z, "spruce_drawer" if z % 4 == 0 else var("spruce_chair", W))
    c.set(7, 3, (az0 + az1) // 2 - 3, HANG)
    building(c, ax1 + 1, az0, Wd - 2, az1, 1, 4, SP, "spruce_log", "spruce_roof", SP, "spruce_slab", "spruce_window", "spruce_planks",
             doors=(W,), door_name="spruce_door", axis="z")
    for z in range(az0 + 1, az1, 1):
        c.set(Wd - 3, 1, z, "bookshelf")
        c.set(Wd - 3, 2, z, "bookshelf")
    for z in range(az0 + 3, az1 - 2, 3):
        c.set(ax1 + 3, 1, z, "spruce_table")
        c.set(ax1 + 3, 2, z, "lantern")
        c.set(ax1 + 4, 1, z, var("spruce_chair", W))
    building(c, 12, az1 + 1, 39, Dp - 2, 1, 4, SB, "smooth_stone", "stone_brick_roof", SB, "stone_brick_slab", "glass_pane", "spruce_planks",
             doors=(S, N), door_name="spruce_door")
    for x in range(15, 37):
        if x not in (25, 26):
            c.set(x, 1, az1 + 4, "spruce_table")
            c.set(x, 1, az1 + 3, var("spruce_bench", S))
            c.set(x, 1, az1 + 5, var("spruce_bench", N))
    for x in range(15, 37, 5):
        c.set(x, 2, az1 + 4, "lantern")
    c.set(14, 1, Dp - 3, "furnace")
    c.set(15, 1, Dp - 3, "crafting_table")
    c.set(16, 1, Dp - 3, "spruce_drawer")
    # Ways through the cloister wall to each building's door.
    for (x, z) in ((ax0, (az0 + az1) // 2), (ax1, (az0 + az1) // 2), ((12 + 39) // 2, az0), ((12 + 39) // 2, az1)):
        c.box(x, 1, z, x, 2, z, AIR)
    c.weather(rng, {SB: [CB, "mossy_cobblestone_bricks", SB]}, 0.1)
    return struct("mosteiro", ["Mountains", "Snowy Plains", "Cherry Grove"], c, 64, 0.45,
                  loot=rare(("bookshelf", 1, 3, 0.4), ("mana_crystal", 1, 3, 0.4), ("thunder_tome", 1, 1, 0.1), ("flame_tome", 1, 1, 0.1), ("bread", 2, 6, 0.6)),
                  sink=1, foundation="stone_bricks", rough=9, mobs=[("zombie", 3), ("skeleton", 3), ("invoker", 1)])


@group("wda")
def foundry():
    """A foundry: a long brick hall with a glowing lava trough behind bars, rows of furnaces,
    chains, three tall chimneys, an ore yard with a rail line and a mine mouth."""
    rng = random.Random(27)
    Wd, Dp = 54, 40
    c = Canvas(Wd, 34, Dp)
    BR, SB, CB = "bricks", "stone_bricks", "cobblestone"
    for z in range(Dp):
        for x in range(Wd):
            c.set(x, 0, z, rng.choice(["gravel", "cobblestone", "rocky_dirt", "gravel", "stone"]))
            c.box(x, 1, z, x, 22, z, AIR)
    hx0, hz0, hx1, hz1 = 4, 4, 41, 23
    building(c, hx0, hz0, hx1, hz1, 1, 9, BR, SB, "stone_brick_roof", BR, "stone_brick_slab", "iron_bars", SB,
             doors=(S,), door_name="oak_door", every=3)
    c.box(20, 1, hz1, 25, 6, hz1, AIR)  # big doors
    for x in (19, 26):
        c.column(x, hz1, 1, 7, SB)
    c.box(19, 7, hz1, 26, 7, hz1, SB)
    # Lava trough down the middle.
    mz = (hz0 + hz1) // 2
    for x in range(hx0 + 3, hx1 - 2):
        c.set(x, 0, mz, "lava")
        c.set(x, 0, mz - 1, CB)
        c.set(x, 0, mz + 1, CB)
        c.set(x, 1, mz - 1, "iron_bars")
        c.set(x, 1, mz + 1, "iron_bars")
        c.set(x, -1, mz, CB)
    for x in range(hx0 + 2, hx1 - 1, 2):
        c.set(x, 1, hz0 + 1, "furnace")
        c.set(x + 1, 1, hz0 + 1, "smooth_stone")
        c.set(x, 2, hz0 + 1, "hopper")
    for x in range(hx0 + 3, hx1 - 2, 4):
        c.set(x, 1, hz1 - 2, "iron_block" if x % 8 else "copper_block")
        c.set(x + 1, 1, hz1 - 2, "oak_table")
        c.set(x + 2, 1, hz1 - 2, "crafting_table" if x % 3 else "crafter")
    # Chains with lanterns from the rafters and a crane hook over the trough.
    for x in range(hx0 + 4, hx1 - 2, 6):
        for y in range(6, 10):
            c.set(x, y, mz - 3, "chain")
            c.set(x, y, mz + 3, "chain")
        c.set(x, 5, mz - 3, "lantern:1")
        c.set(x, 5, mz + 3, "lantern:1")
    for x in range(hx0 + 1, hx1):
        c.set(x, 9, mz, "dark_oak_log")
    for y in range(4, 9):
        c.set(22, y, mz, "chain")
    c.set(22, 3, mz, "iron_block")
    # Chimneys.
    for (x, z) in ((10, hz0 - 1), (22, hz0 - 1), (34, hz0 - 1)):
        c.box(x - 1, 1, z - 2, x + 1, 30, z, BR)
        c.box(x, 10, z - 1, x, 30, z - 1, AIR)
        c.walls(x - 1, z - 2, x + 1, z, 27, 27, SB)
    # Office with the chest at the east end.
    c.room(hx1 + 1, 1, hz0, Wd - 3, 6, hz0 + 9, SB, floor="oak_planks", ceil="oak_planks")
    c.set(hx1 + 1, 1, hz0 + 4, AIR)
    c.set(hx1 + 1, 2, hz0 + 4, AIR)
    c.set(Wd - 4, 1, hz0 + 2, "chest")
    c.set(Wd - 4, 1, hz0 + 4, "oak_table")
    c.set(Wd - 5, 1, hz0 + 4, var("oak_chair", E))
    c.set(Wd - 4, 2, hz0 + 4, "lantern")
    c.set(Wd - 4, 1, hz0 + 6, "bookshelf")
    c.set(Wd - 4, 1, hz0 + 7, "oak_drawer")
    for z in (hz0 + 3, hz0 + 6):
        c.set(Wd - 3, 3, z, "glass_pane")
    # Ore yard and rails out to a mine mouth.
    for (x, z, b) in ((8, 30, "iron_ore"), (9, 30, "coal_ore"), (8, 31, "copper_ore"), (9, 31, "iron_ore"), (8, 30, "iron_ore"),
                      (12, 32, "coal_ore"), (13, 32, "coal_ore"), (12, 33, "tin_ore"), (10, 32, "gravel")):
        c.set(x, 1, z, b)
    c.set(9, 2, 30, "coal_ore")
    c.set(8, 2, 31, "iron_ore")
    for x in range(6, Wd - 6):
        c.set(x, 1, 28, "rail:1")
    c.room(Wd - 8, 0, 25, Wd - 1, 6, 33, "stone", ceil="stone")
    c.box(Wd - 7, 1, 27, Wd - 1, 4, 29, AIR)
    for x in (Wd - 7, Wd - 4):
        c.column(x, 26, 1, 4, "oak_log")
        c.column(x, 30, 1, 4, "oak_log")
        c.box(x, 5, 26, x, 5, 30, "oak_planks")
    c.set(Wd - 6, 4, 28, HANG)
    for (x, z) in ((5, 26), (30, 34), (44, 34)):
        lamp_post(c, x, 1, z, "oak_fence", "lantern", h=3)
    c.weather(rng, {BR: [BR, BR, "stone_bricks"]}, 0.05)
    return struct("fundicao", ["Mountains", "Badlands"], c, 60, 0.45,
                  loot=rare(("iron_ingot", 6, 16, 0.8), ("copper_ingot", 4, 12, 0.6), ("coal", 8, 20, 0.7), ("iron_hammer", 1, 1, 0.2),
                            ("tungsten_ingot", 1, 4, 0.3), ("minecart", 1, 1, 0.3)),
                  sink=1, foundation="cobblestone", rough=8, mobs=[("brute", 2), ("marauder", 3), ("zombie", 2)])


@group("wda")
def bandit_village():
    """A bandit camp behind a pointed log palisade: watch platforms on stilts, colourful tents,
    a longhouse, cages and a fire pit round the chief's tent with the loot."""
    rng = random.Random(28)
    Sz = 48
    c = Canvas(Sz, 16, Sz)
    LOG, PL = "acacia_log", "acacia_planks"
    for z in range(Sz):
        for x in range(Sz):
            c.set(x, 0, z, rng.choice(["rocky_dirt", "gravel", "dirt", "rocky_dirt", "red_sand"]))
            c.box(x, 1, z, x, 15, z, AIR)
    # Palisade.
    for (x, z) in square_ring(1, 1, Sz - 2, Sz - 2):
        h = 4 + (x * 7 + z * 3) % 2
        c.column(x, z, 1, h, LOG)
        c.set(x, h + 1, z, "acacia_fence")
    g = Sz // 2
    for x in range(g - 2, g + 3):
        c.box(x, 1, Sz - 2, x, 6, Sz - 2, AIR)
    for x in (g - 3, g + 3):
        c.column(x, Sz - 2, 1, 8, LOG)
    c.box(g - 3, 7, Sz - 2, g + 3, 7, Sz - 2, PL)
    c.set(g - 3, 9, Sz - 2, "brazier")
    c.set(g + 3, 9, Sz - 2, "brazier")
    # Watch platforms in the corners.
    for (x, z) in ((4, 4), (Sz - 7, 4), (4, Sz - 7), (Sz - 7, Sz - 7)):
        for (dx, dz) in ((0, 0), (2, 0), (0, 2), (2, 2)):
            c.column(x + dx, z + dz, 1, 6, LOG)
        c.box(x - 1, 7, z - 1, x + 3, 7, z + 3, PL)
        c.walls(x - 1, z - 1, x + 3, z + 3, 8, 8, "acacia_fence")
        for (dx, dz) in ((-1, -1), (3, -1), (-1, 3), (3, 3)):
            c.column(x + dx, z + dz, 8, 10, "acacia_fence")
        c.box(x - 1, 11, z - 1, x + 3, 11, z + 3, "thatch")
        stair_run(c, x + 1, 1, z + 4 if z < g else z - 2, N if z < g else S, 6, "acacia_stairs", clear=2)
    # Tents in many colours.
    colors = ["red_wool", "orange_wool", "yellow_wool", "brown_wool", "black_wool", "green_wool"]
    for (x, z, along) in ((9, 12, "z"), (15, 9, "x"), (32, 10, "x"), (37, 16, "z"), (9, 30, "z"), (36, 30, "z")):
        tent(c, x, z, rng.randint(4, 5), rng.choice(colors), along, pole="acacia_fence")
        bx, bz = (x + 2, z + 1) if along == "z" else (x + 1, z + 2)
        c.set(bx, 1, bz, "red_bed" if rng.random() < 0.6 else "hay_block")
    # Longhouse.
    building(c, 18, 30, 30, 38, 1, 4, PL, LOG, "thatch_roof", PL, "thatch", "acacia_window", "acacia_planks", doors=(N,), door_name="acacia_door")
    for x in range(20, 29, 2):
        c.set(x, 1, 37, "red_bed")
    c.set(20, 1, 31, "crafting_table")
    c.set(28, 1, 31, "furnace")
    for x in range(21, 28):
        c.set(x, 1, 34, "acacia_table")
    c.set(24, 3, 34, HANG)
    # Cages.
    for (x0, z0) in ((4, 18), (40, 20)):
        c.box(x0, 0, z0, x0 + 3, 0, z0 + 3, "cobblestone")
        c.walls(x0, z0, x0 + 3, z0 + 3, 1, 3, "iron_bars")
        c.box(x0, 4, z0, x0 + 3, 4, z0 + 3, "acacia_slab")
        c.set(x0 + 1, 1, z0 + 1, "hay_block")
    # Fire pit and the chief's tent with the chest.
    c.disc(g, g - 2, 2.5, 0, "cobblestone")
    c.set(g, 0, g - 2, "magma_block")
    c.set(g, 1, g - 2, "brazier")
    for (x, z, f) in ((g - 2, g - 2, E), (g + 2, g - 2, W), (g, g, N), (g, g - 4, S)):
        c.set(x, 1, z, var("acacia_bench", f))
    cx0, cz0 = g - 4, 16
    c.box(cx0, 0, cz0, cx0 + 8, 0, cz0 + 5, "acacia_planks")
    for (x, z) in ((cx0, cz0), (cx0 + 8, cz0), (cx0, cz0 + 5), (cx0 + 8, cz0 + 5)):
        c.column(x, z, 1, 3, LOG)
    for x in range(cx0, cx0 + 9):
        c.set(x, 4, cz0, "red_wool")
        c.set(x, 4, cz0 + 5, "red_wool")
        for z in range(cz0 + 1, cz0 + 5):
            c.set(x, 5, z, "red_wool" if x % 2 else "yellow_wool")
    c.set(g, 1, cz0 + 1, "chest")
    c.set(g - 1, 1, cz0 + 1, "gold_block")
    c.set(g + 1, 1, cz0 + 1, "hay_block")
    c.set(g, 1, cz0 + 2, var("acacia_chair", N))
    c.set(g, 4, cz0 + 3, HANG)
    for (x, z) in ((14, 24), (33, 24), (g, 40)):
        target_dummy(c, x, 1, z, "acacia_fence")
    return struct("vila_dos_bandidos", ["Savanna", "Badlands", "Desert"], c, 56, 0.45,
                  loot=rare(("emerald", 3, 9, 0.7), ("gold_ingot", 2, 6, 0.6), ("leather", 2, 6, 0.5), ("iron_sword", 1, 1, 0.3), ("tnt", 1, 4, 0.3)),
                  sink=1, foundation="dirt", rough=7, mobs=[("marauder", 4), ("brute", 2), ("invoker", 1)])


def mushroom(c, cx, cz, y, r, h, cap, spot, rng, door_f=S, inside=True):
    """A giant mushroom house: a pale stem with a door and a domed cap with spots."""
    stem = "white_terracotta"
    sr = 2.2 if inside else 1.2
    for yy in range(y, y + h):
        c.disc(cx, cz, sr, yy, stem)
        if inside and yy > y:
            c.disc(cx, cz, sr - 1.1, yy, AIR)
    if inside:
        dx, dz = DV[door_f]
        x, z = round(cx + dx * 2), round(cz + dz * 2)
        c.set(x, y + 1, z, door("spruce_door", door_f)[0])
        c.set(x, y + 2, z, door("spruce_door", door_f)[1])
        c.set(round(cx - dx), y + 1, round(cz - dz), "red_bed")
        c.set(round(cx + dz), y + 1, round(cz + dx), "spruce_table")
        c.set(round(cx + dz), y + 2, round(cz + dx), "lantern")
    top = y + h
    for k in range(int(r * 0.7) + 2):
        rr = r * (1 - (k / (r * 0.7 + 1.5)) ** 2) ** 0.5
        c.disc(cx, cz, rr, top + k, cap, hollow=k == 0, thick=1.5)
        if k == 0:
            c.disc(cx, cz, rr - 1.5, top, "white_terracotta")
    for _ in range(int(r * 3)):
        a = rng.random() * 2 * pi
        k = rng.randint(1, int(r * 0.7))
        rr = r * (1 - (k / (r * 0.7 + 1.5)) ** 2) ** 0.5
        x, z = round(cx + cos(a) * (rr - 0.5)), round(cz + sin(a) * (rr - 0.5))
        if c.get(x, top + k, z) == cap and c.get(x, top + k + 1, z) in (None, AIR):
            c.set(x, top + k, z, spot)
    if inside:
        c.set(round(cx), top - 1, round(cz), "shroomlight")


@group("wda")
def mushroom_village():
    """A village of giant mushroom houses on moss, with mushroom patches and glowing lamps."""
    rng = random.Random(29)
    Sz = 46
    c = Canvas(Sz, 26, Sz)
    for z in range(Sz):
        for x in range(Sz):
            d = ((x - 23) ** 2 + (z - 23) ** 2) ** 0.5
            if d > 22.5:
                continue
            c.set(x, 0, z, rng.choice(["moss_block", "moss_block", "grass_block", "rocky_dirt"]))
            c.box(x, 1, z, x, 25, z, AIR)
    for k in range(Sz):
        for (x, z) in ((k, 23), (23, k), (k, 24), (24, k)):
            if c.get(x, 0, z) is not None:
                c.set(x, 0, z, "gravel" if rng.random() < 0.7 else "rocky_dirt")
    houses = [(11, 11, 6, 7, "red_wool", "white_wool", S), (35, 11, 5.5, 6, "brown_wool", "light_gray_wool", S),
              (11, 35, 5.5, 6, "brown_wool", "white_wool", N), (35, 35, 6.5, 8, "red_wool", "white_wool", N),
              (23, 8, 4.5, 5, "red_wool", "white_wool", S)]
    for (x, z, r, h, cap, spot, f) in houses:
        mushroom(c, x, z, 0, r, h, cap, spot, rng, f)
    # A great mushroom in the middle as a meeting place, with benches and a lamp.
    mushroom(c, 23, 23, 0, 4.5, 9, "red_wool", "white_wool", rng, inside=False)
    for (x, z, f) in ((20, 23, E), (26, 23, W), (23, 20, S), (23, 26, N)):
        c.set(x, 1, z, var("spruce_bench", f))
    # Patches of small mushrooms and shroomlight posts.
    for _ in range(40):
        x, z = rng.randint(3, Sz - 4), rng.randint(3, Sz - 4)
        if c.get(x, 1, z) == AIR and c.get(x, 0, z) in ("moss_block", "grass_block"):
            c.set(x, 1, z, rng.choice(["red_mushroom", "brown_mushroom", "short_grass"]))
    for (x, z) in ((17, 17), (29, 17), (17, 29), (29, 29), (23, 40), (40, 23), (6, 23)):
        c.column(x, z, 1, 2, "spruce_fence")
        c.set(x, 3, z, "shroomlight")
    c.set(36, 1, 33, "chest")
    c.set(10, 1, 33, "chest")
    return struct("vila_dos_cogumelos", ["Dark Forest", "Swamp"], c, 56, 0.45,
                  loot=rare(("red_mushroom", 2, 6, 0.6), ("brown_mushroom", 2, 6, 0.6), ("glow_berries", 2, 6, 0.4), ("ancient_seed", 1, 1, 0.15)),
                  sink=1, foundation="dirt", rough=6, mobs=[("goblin_trader", 1), ("spider", 2), ("zombie", 3), ("swamp_witch", 1)])


@group("wda")
def tree_house():
    """A giant jungle tree with a village in it: decks round the trunk, huts on the branches,
    a stair winding up the outside, rope bridges and a crown of leaves."""
    rng = random.Random(30)
    Sz = 44
    c = Canvas(Sz, 52, Sz)
    cx, cz = 20, 22
    # The trunk, flaring at the roots.
    for y in range(0, 40):
        r = 2.6 if y > 4 else 3.6 - y * 0.2
        c.disc(cx, cz, r, y, "jungle_log")
    for (dx, dz) in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        c.line(cx + dx * 3, 3, cz + dz * 3, cx + dx * 7, -1, cz + dz * 7, "jungle_log")
    # Crown.
    for _ in range(9):
        bx = cx + rng.randint(-10, 10)
        bz = cz + rng.randint(-10, 10)
        by = rng.randint(38, 44)
        c.line(cx, 36, cz, bx, by, bz, "jungle_log")
        for y in range(by - 2, by + 4):
            rr = 5.5 - abs(y - by - 1) * 1.2
            for z in range(int(bz - rr), int(bz + rr) + 1):
                for x in range(int(bx - rr), int(bx + rr) + 1):
                    if (x - bx) ** 2 + (z - bz) ** 2 <= rr * rr and c.get(x, y, z) in (None, AIR):
                        c.set(x, y, z, "jungle_leaves")
    # Decks.
    for (y, r) in ((10, 7.5), (22, 6.5), (33, 6)):
        c.disc(cx, cz, r, y, "jungle_planks")
        c.disc(cx, cz, 2.6, y, "jungle_log")
        c.disc(cx, cz, r, y + 1, "jungle_fence", hollow=True)
        for (x, z) in circle_ring(cx, cz, r - 0.5)[::6]:
            if c.get(x, y - 1, z) in (None, AIR):
                c.set(x, y - 1, z, HANG)
    # Stair winding up round the trunk, outside it.
    ring = circle_ring(cx, cz, 3.6)
    spiral_stairs(c, ring, 1, 33, "jungle_stairs")
    for y in range(1, 34):
        x, z = ring[(y - 1) % len(ring)]
        for (yy, r) in ((10, 7.5), (22, 6.5), (33, 6)):
            if y == yy:
                c.set(x, y, z, "jungle_planks")
            if yy - 3 < y < yy:
                c.set(x, yy, z, AIR)
        c.setif(x, y + 1, z, AIR, only=("jungle_fence",))
    # Huts on the decks.
    st = style("jungle", roof="thatch_roof", cap="thatch", gable="jungle_planks", floor="jungle_planks", porch_lamp=False)
    cottage(c, cx + 3, cz - 3, cx + 7, cz + 2, st, S, rng, y=11, wh=3, things=["bed", "chest", "table", "shelf"])
    cottage(c, cx - 8, cz - 2, cx - 3, cz + 3, st, N, rng, y=23, wh=3, things=["bed", "table", "crafting_table", "shelf"])
    cottage(c, cx - 2, cz + 3, cx + 3, cz + 6, st, E, rng, y=34, wh=3, things=["chest", "bookshelf", "table"])
    # A smaller tree with a platform, joined by a rope bridge.
    tx, tz = 37, 8
    for y in range(0, 26):
        c.disc(tx, tz, 1.2, y, "jungle_log")
    c.disc(tx, tz, 4, 22, "jungle_planks")
    c.disc(tx, tz, 4, 23, "jungle_fence", hollow=True)
    for y in range(25, 30):
        c.disc(tx, tz, 4.5 - abs(y - 27), y, "jungle_leaves")
    c.set(tx, 23, tz + 2, "chest")
    for k in range(cx + 6, tx - 3):
        z = round(cz + (tz - cz) * (k - cx - 6) / (tx - 3 - cx - 6))
        c.set(k, 22, z, "rope_bridge:1")
        c.set(k, 23, z, AIR)
    overgrow(c, rng, 0.04, y0=3)
    return struct("casa_na_arvore", ["Jungle"], c, 56, 0.5,
                  loot=rare(("glow_berries", 2, 6, 0.5), ("jungle_sapling", 1, 3, 0.5), ("arrow", 6, 16, 0.5), ("ancient_seed", 1, 1, 0.12)),
                  sink=1, foundation="dirt", rough=8, mobs=[("spider", 3), ("zombie", 2), ("skeleton", 2)])


def hull(c, x0, L, cz, half, deck, y0=0, shell="dark_oak_planks", stripe=None, deck_b="spruce_planks", rail="dark_oak_fence", stern=6, bow=12):
    """A ship's hull along x (stern at x0, bow at x0 + L - 1), keel at layer y0, deck at layer
    `deck`, `half` blocks wide each side at the widest. Returns half-width per (x, y)."""
    def hw(x, y):
        t = x - x0
        if t < 0 or t >= L:
            return -1
        s = 1.0
        if t < stern:
            s = 0.78 + 0.22 * t / stern
        elif t > L - 1 - bow:
            s = max(0.0, (L - 1 - t) / bow) ** 0.6
        f = 0.4 + 0.6 * ((y - y0) / max(deck - y0, 1)) ** 0.55
        return half * s * f
    for y in range(y0, deck + 1):
        for x in range(x0, x0 + L):
            w = hw(x, y)
            if w < 0:
                continue
            for z in range(int(cz - half - 2), int(cz + half + 3)):
                if abs(z - cz) > w + 0.35:
                    continue
                edge = abs(z - cz) > hw(x, y) - 0.65 or hw(x + 1, y) < abs(z - cz) - 0.35 or hw(x - 1, y) < abs(z - cz) - 0.35 or y == y0
                if y == deck:
                    c.set(x, y, z, shell if edge else deck_b)
                elif edge:
                    c.set(x, y, z, stripe if (stripe and y == deck - 2) else shell)
                else:
                    c.set(x, y, z, AIR)
    # Rails on the deck's edge.
    for x in range(x0, x0 + L):
        w = hw(x, deck)
        if w < 0:
            continue
        for z in range(int(cz - half - 2), int(cz + half + 3)):
            if abs(z - cz) <= w + 0.35 and (abs(z - cz) > w - 0.65 or hw(x + 1, deck) < abs(z - cz) - 0.35 or hw(x - 1, deck) < abs(z - cz) - 0.35):
                c.set(x, deck + 1, z, rail)
    return hw


def mast(c, x, cz, y0, top, yards, sail, half, log="spruce_log", fence="spruce_fence", stripes=None):
    c.column(x, cz, y0, top, log)
    for (ya, yb) in yards:
        for z in range(cz - half - 1, cz + half + 2):
            c.set(x, yb, z, fence)
        for y in range(ya, yb):
            for z in range(cz - half, cz + half + 1):
                w = sail
                if stripes:
                    w = stripes[(y - ya) % len(stripes)]
                c.set(x + 1, y, z, w)


@group("wda")
def pirate_ship():
    """A three-masted pirate ship on the open sea: black sails, a crow's nest, a stern castle
    with the captain's cabin, a forecastle, cannons along the deck and a hold below."""
    rng = random.Random(31)
    L, Wd = 58, 17
    cz = 8
    c = Canvas(L, 44, Wd)
    deck = 6
    hw = hull(c, 1, 52, cz, 6.5, deck, stripe="spruce_planks")
    # Stern castle with the cabin.
    for y in range(deck + 1, deck + 5):
        for x in range(1, 13):
            w = hw(x, deck)
            for z in range(Wd):
                if abs(z - cz) <= w + 0.35:
                    edge = abs(z - cz) > w - 0.65 or x in (1, 12)
                    c.set(x, y, z, ("dark_oak_planks" if edge else AIR) if y < deck + 4 else "spruce_planks")
    for x in range(1, 13):
        for z in range(Wd):
            if c.get(x, deck + 4, z) == "spruce_planks" and (abs(z - cz) > hw(x, deck) - 0.65 or x == 1):
                c.set(x, deck + 5, z, "dark_oak_fence")
    for z in range(cz - 1, cz + 2):
        c.set(12, deck + 5, z, "dark_oak_fence")
    for z in (cz - 3, cz - 1, cz + 1, cz + 3):
        c.set(1, deck + 2, z, "glass_pane")
    for x in (4, 8):
        c.set(x, deck + 2, cz - round(hw(x, deck)), "glass_pane")
        c.set(x, deck + 2, cz + round(hw(x, deck)), "glass_pane")
    c.set(12, deck + 1, cz, door("dark_oak_door", E)[0])
    c.set(12, deck + 2, cz, door("dark_oak_door", E)[1])
    c.set(3, deck + 1, cz, "chest")
    c.set(3, deck + 1, cz - 2, "red_bed")
    c.set(6, deck + 1, cz, "dark_oak_table")
    c.set(7, deck + 1, cz, "dark_oak_table")
    c.set(6, deck + 2, cz, "lantern")
    c.set(5, deck + 1, cz, var("dark_oak_chair", E))
    c.set(9, deck + 1, cz + 3, "bookshelf")
    c.set(9, deck + 1, cz - 3, "dark_oak_drawer")
    c.set(6, deck + 3, cz + 2, HANG)
    stair_run(c, 14, deck + 1, cz + 4, W, 3, "spruce_stairs", clear=2)
    c.set(13, deck + 4, cz + 4, "spruce_planks")
    c.set(10, deck + 5, cz, "dark_oak_fence")
    c.set(10, deck + 6, cz, "lantern")
    # Forecastle.
    for x in range(41, 50):
        w = hw(x, deck)
        for z in range(Wd):
            if abs(z - cz) <= w + 0.35:
                c.set(x, deck + 1, z, "dark_oak_planks")
                c.set(x, deck + 2, z, "spruce_planks")
                if abs(z - cz) > w - 0.65 or hw(x + 1, deck) < abs(z - cz) - 0.35 or x == 41:
                    c.set(x, deck + 3, z, "dark_oak_fence")
    c.box(41, deck + 3, cz - 1, 41, deck + 3, cz + 1, AIR)
    stair_run(c, 39, deck + 1, cz, E, 2, "spruce_stairs", clear=2)
    # Bowsprit.
    c.line(50, deck + 3, cz, 57, deck + 7, cz, "spruce_fence")
    c.line(51, deck + 3, cz, 57, deck + 6, cz, "spruce_log")
    # Masts, sails, crow's nest and the black flag.
    mast(c, 18, cz, 1, 34, [(14, 20), (22, 29)], "black_wool", 6)
    mast(c, 29, cz, 1, 38, [(15, 22), (24, 32)], "black_wool", 7)
    mast(c, 39, cz, 1, 30, [(14, 19), (21, 26)], "black_wool", 5)
    c.box(28, 33, cz - 1, 30, 33, cz + 1, "spruce_slab")
    c.walls(28, cz - 1, 30, cz + 1, 34, 34, "spruce_fence")
    c.set(29, 33, cz, "spruce_log")
    c.box(30, 36, cz, 32, 37, cz, "black_wool")
    c.set(31, 37, cz, "white_wool")
    # Cannons on the main deck.
    for x in range(16, 40, 4):
        for side in (-1, 1):
            z = cz + side * round(hw(x, deck) - 1)
            c.set(x, deck + 1, z, stair("blackstone_stairs", S if side > 0 else N))
            c.set(x, deck + 1, z + side, AIR)
            c.set(x, deck - 2, cz + side * round(hw(x, deck - 2)), "iron_bars")
    # Hatch to the hold.
    c.box(22, deck, cz - 1, 25, deck, cz, AIR)
    for k in range(deck - 1):
        c.set(25 - k, deck - 1 - k, cz - 1, stair("spruce_stairs", E))
    # The hold: barrels, a chest, hay and lanterns.
    for x in range(14, 40, 3):
        for side in (-1, 1):
            z = cz + side * round(hw(x, 2) - 1)
            if c.get(x, 1, z) == AIR:
                c.set(x, 1, z, rng.choice(["spruce_drawer", "hay_block", "spruce_planks", "oak_planks"]))
    c.set(33, 1, cz, "chest")
    c.set(33, 1, cz + 1, "gold_block")
    for x in (16, 30):
        c.set(x, deck - 1, cz + 2, HANG)
    for x in (20, 36):
        c.set(x, deck + 1, cz, "lantern")
    return struct("navio_pirata", ["Ocean"], c, 56, 0.45,
                  loot=rare(("gold_ingot", 4, 10, 0.8), ("gold_nugget", 6, 20, 0.6), ("emerald", 2, 8, 0.6), ("gunpowder", 2, 8, 0.6),
                            ("iron_sword", 1, 1, 0.3), ("cooked_cod", 2, 6, 0.4)),
                  sink=4, place="Water", mobs=[("skeleton", 4), ("zombie", 3), ("marauder", 1)])


@group("wda")
def corsair_galley():
    """A long, low corsair galley: banks of oars, a copper ram, one mast with a striped sail
    and a red canopy over the stern."""
    rng = random.Random(32)
    L, Wd = 52, 17
    cz = 8
    c = Canvas(L, 30, Wd)
    deck = 5
    hw = hull(c, 2, 46, cz, 4.5, deck, shell="spruce_planks", stripe="dark_oak_planks", deck_b="oak_planks", rail="spruce_fence", stern=5, bow=10)
    # Oars, every three blocks down both sides, reaching to the water.
    for x in range(10, 40, 3):
        for side in (-1, 1):
            z0 = cz + side * round(hw(x, deck))
            c.line(x, deck, z0 + side, x, 2, z0 + side * 5, "spruce_fence")
            c.set(x, deck + 1, cz + side * round(hw(x, deck) - 1), var("spruce_bench", W))
    # The ram.
    c.line(46, 3, cz, 51, 2, cz, "cut_copper")
    c.line(46, 4, cz, 50, 3, cz, "cut_copper")
    c.set(48, deck + 1, cz, "lantern")
    # Mast with a striped sail.
    mast(c, 25, cz, 1, 24, [(10, 20)], "red_wool", 6, stripes=["red_wool", "red_wool", "white_wool", "white_wool"])
    c.box(25, 23, cz, 27, 23, cz, "red_wool")
    # Stern canopy over the captain's seat and the chest.
    for (x, z) in ((3, cz - 3), (3, cz + 3), (9, cz - 3), (9, cz + 3)):
        c.column(x, z, deck + 1, deck + 3, "spruce_fence")
    c.box(3, deck + 4, cz - 3, 9, deck + 4, cz + 3, "red_wool")
    for x in range(3, 10):
        c.set(x, deck + 4, cz, "yellow_wool")
    c.set(5, deck + 1, cz, "chest")
    c.set(6, deck + 1, cz, var("spruce_chair", E))
    c.set(7, deck + 3, cz, HANG)
    # A hatch and the rowers' hold.
    c.box(30, deck, cz - 1, 32, deck, cz - 1, AIR)
    for k in range(deck - 1):
        c.set(32 - k, deck - 1 - k, cz - 1, stair("spruce_stairs", E))
    for x in range(10, 40, 4):
        c.set(x, 1, cz + 1, rng.choice(["spruce_drawer", "hay_block", "oak_planks"]))
    c.set(20, deck - 1, cz, HANG)
    return struct("galera_corsaria", ["Ocean"], c, 50, 0.45,
                  loot=rare(("gold_ingot", 3, 8, 0.7), ("emerald", 2, 6, 0.6), ("iron_ingot", 2, 6, 0.6), ("leather", 2, 6, 0.4)),
                  sink=3, place="Water", mobs=[("marauder", 4), ("brute", 1)])


@group("wda")
def airship():
    """An airship high in the sky: a striped wool envelope on ribs, a wooden gondola hung on
    chains with a cabin, a propeller at the back and tail fins."""
    rng = random.Random(33)
    L, Wd = 52, 23
    cz = 11
    c = Canvas(L, 32, Wd)
    # Envelope.
    ex, ey, ez = 26, 20, cz
    rx, ry, rz = 23.0, 7.5, 9.0
    for y in range(c.h):
        for z in range(Wd):
            for x in range(L):
                q = ((x - ex) / rx) ** 2 + ((y - ey) / ry) ** 2 + ((z - ez) / rz) ** 2
                if q <= 1.0:
                    inner = ((x - ex) / (rx - 1.2)) ** 2 + ((y - ey) / (ry - 1.2)) ** 2 + ((z - ez) / (rz - 1.2)) ** 2
                    if inner > 1.0:
                        b = "white_wool"
                        if x % 7 == 0:
                            b = "light_gray_wool"
                        if abs(y - ey) <= 0 and abs(z - ez) > 6:
                            b = "red_wool"
                        c.set(x, y, z, b)
    # Tail fins.
    for k in range(6):
        for j in range(k, 6):
            c.set(3 + j, ey + 7 - k + 1, cz, "light_gray_wool")
            c.set(3 + j, ey, cz - 8 - (5 - k) + 1, "light_gray_wool")
            c.set(3 + j, ey, cz + 8 + (5 - k) - 1, "light_gray_wool")
    # Gondola.
    deck = 4
    hw = hull(c, 12, 30, cz, 4.5, deck, shell="spruce_planks", stripe="dark_oak_planks", deck_b="oak_planks", rail="oak_fence", stern=4, bow=8)
    for (x, z) in ((14, cz - 3), (14, cz + 3), (36, cz - 3), (36, cz + 3), (25, cz - 4), (25, cz + 4)):
        for y in range(deck + 1, c.h):
            if c.get(x, y, z) not in (None, AIR):
                break
            c.set(x, y, z, "chain")
    # Cabin on the gondola.
    building(c, 18, cz - 3, 26, cz + 3, deck + 1, 3, "spruce_planks", "spruce_log", "spruce_roof", "spruce_planks", "spruce_slab",
             "glass_pane", "oak_planks", doors=(E,), door_name="spruce_door")
    c.set(19, deck + 1, cz, "chest")
    c.set(19, deck + 1, cz - 2, "red_bed")
    c.set(19, deck + 1, cz + 2, "crafting_table")
    c.set(22, deck + 1, cz + 2, "spruce_table")
    c.set(22, deck + 2, cz + 2, "lantern")
    c.set(22, deck + 1, cz - 2, "spruce_drawer")
    # Propeller at the stern.
    c.line(9, deck + 2, cz, 12, deck + 2, cz, "oak_fence")
    c.set(9, deck + 2, cz, "iron_block")
    for k in (1, 2, 3):
        c.set(8, deck + 2 + k, cz, "oak_planks")
        c.set(8, deck + 2 - k, cz, "oak_planks")
        c.set(8, deck + 2, cz + k, "oak_planks")
        c.set(8, deck + 2, cz - k, "oak_planks")
    c.set(40, deck + 1, cz, "lantern")
    c.set(31, deck + 1, cz + 2, "lantern")
    c.set(30, deck + 1, cz - 2, "hay_block")
    return struct("aeronave", ["Plains", "Forest", "Birch Forest", "Savanna", "Mountains", "Taiga", "Desert", "Snowy Plains"], c, 70, 0.4,
                  loot=rare(("gunpowder", 2, 8, 0.6), ("feather", 2, 8, 0.5), ("string", 2, 8, 0.5), ("diamond_backpack", 1, 1, 0.04)),
                  place="Sky(150)", mobs=[("marauder", 3), ("skeleton", 2)])


@group("wda")
def sky_temple():
    """The heavenly temple: a floating island with a pond and blossoming trees, a white quartz
    temple of columns and gold, and puffs of cloud round its edge."""
    rng = random.Random(34)
    Sz = 50
    c = Canvas(Sz, 40, Sz)
    cx = cz = 24.5
    top = 12
    # The island: an upside-down cone of stone with a grass top, ragged at the edge.
    for y in range(0, top + 1):
        t = y / top
        for z in range(Sz):
            for x in range(Sz):
                a = atan2(z - cz, x - cx)
                rr = (4 + 18 * t ** 0.65) * (1 + 0.08 * sin(a * 5 + 1.3) + 0.05 * sin(a * 9))
                d = ((x - cx) ** 2 + (z - cz) ** 2) ** 0.5
                if d <= rr:
                    b = "stone" if y < top - 2 else ("dirt" if y < top else "grass_block")
                    if b == "stone" and rng.random() < 0.15:
                        b = rng.choice(["cobblestone", "mossy_cobblestone", "gold_ore", "iron_ore"])
                    c.set(x, y, z, b)
    # Clouds round the island.
    for _ in range(9):
        a = rng.random() * 2 * pi
        d = rng.uniform(17, 22)
        x0, z0 = cx + cos(a) * d, cz + sin(a) * d
        y0 = rng.randint(4, 9)
        for _ in range(4):
            px, pz = x0 + rng.uniform(-3, 3), z0 + rng.uniform(-3, 3)
            for y in (y0, y0 + 1):
                c.disc(px, pz, 2.6 if y == y0 else 1.8, y, "white_wool")
    for z in range(Sz):
        for x in range(Sz):
            if c.get(x, top, z) == "grass_block":
                for y in range(top + 1, c.h):
                    c.set(x, y, z, AIR)
    # The temple: steps, a platform, a ring of columns and the inner shrine.
    t0, t1 = 15, 34
    c.box(t0 - 1, top + 1, t0 - 1, t1 + 1, top + 1, t1 + 1, "quartz_block")
    for (x, z) in square_ring(t0 - 1, t0 - 1, t1 + 1, t1 + 1):
        c.set(x, top + 1, z, stair("quartz_stairs", opp(outward(x - 24.5, z - 24.5))))
    c.box(t0, top + 2, t0, t1, top + 2, t1, "quartz_block")
    for (x, z) in square_ring(t0, t0, t1, t1):
        if (x + z) % 3 == 0:
            c.column(x, z, top + 3, top + 9, "quartz_block")
            c.set(x, top + 10, z, "gold_block")
    c.box(t0, top + 11, t0, t1, top + 11, t1, "quartz_slab")
    c.walls(t0, t0, t1, t1, top + 11, top + 11, "quartz_block")
    gable(c, t0, t0, t1, t1, top + 12, "quartz_stairs", "quartz_block", "gold_block", axis="z", over=0)
    for z in (t0, t1):
        for x in range(t0 + 3, t1 - 2, 3):
            if c.get(x, top + 13, z) == "quartz_block":
                c.set(x, top + 13, z, "glowstone")
    s0, s1 = 19, 30
    c.room(s0, top + 3, s0, s1, top + 8, s1, "quartz_block")
    c.walls(s0, s0, s1, s1, top + 8, top + 8, "gold_block")
    for x in range(s0 + 2, s1 - 1, 3):
        c.set(x, top + 5, s0, "glass_pane")
        c.set(x, top + 5, s1, "glass_pane")
    c.box(23, top + 3, s1, 26, top + 6, s1, AIR)
    for x in (23, 26):
        c.column(x, s1, top + 3, top + 6, "gold_block")
    c.box(23, top + 2, s0 + 2, 26, top + 2, s0 + 6, "glowstone")
    c.box(23, top + 3, s0 + 2, 26, top + 3, s0 + 3, "quartz_block")
    c.set(24, top + 4, s0 + 2, "chest")
    c.set(25, top + 4, s0 + 2, "gold_block")
    c.set(25, top + 5, s0 + 2, "end_crystal")
    c.set(23, top + 4, s0 + 3, "potted_poppy")
    c.set(26, top + 4, s0 + 3, "potted_dandelion")
    for (x, z) in ((s0 + 2, s0 + 2), (s1 - 2, s0 + 2), (s0 + 2, s1 - 2), (s1 - 2, s1 - 2)):
        c.set(x, top + 7, z, HANG)
        c.set(x, top + 3, z, "brazier")
    # A pond and cherry trees in the garden round it.
    c.disc(9, 37, 3.3, top, "water")
    c.disc(9, 37, 3.3, top - 1, "dirt")
    c.set(9, top + 1, 37, "lily_pad")
    for (x, z) in ((10, 11), (38, 12), (39, 36), (12, 27), (35, 41)):
        if c.get(x, top, z) != "grass_block":
            continue
        c.column(x, z, top + 1, top + 4, "cherry_log")
        for dy, r in ((4, 2.6), (5, 2.8), (6, 2.0), (7, 1.1)):
            for zz in range(z - 3, z + 4):
                for xx in range(x - 3, x + 4):
                    if (xx - x) ** 2 + (zz - z) ** 2 <= r * r and c.get(xx, top + dy, zz) == AIR:
                        c.set(xx, top + dy, zz, "cherry_leaves")
    scatter = random.Random(35)
    for z in range(Sz):
        for x in range(Sz):
            if c.get(x, top, z) == "grass_block" and c.get(x, top + 1, z) == AIR and scatter.random() < 0.12:
                c.set(x, top + 1, z, scatter.choice(["poppy", "dandelion", "short_grass", "short_grass"]))
    return struct("templo_celestial", ["Plains", "Forest", "Birch Forest", "Cherry Grove", "Mountains", "Savanna", "Taiga", "Jungle"], c, 80, 0.4,
                  loot=rare(("diamond", 2, 5, 0.6), ("mana_crystal", 2, 4, 0.6), ("life_crystal", 1, 2, 0.4), ("sapphire_wand", 1, 1, 0.15),
                            ("thunder_tome", 1, 1, 0.12), ("ancient_seed", 1, 2, 0.3)),
                  place="Sky(170)", mobs=[("invoker", 2), ("skeleton", 3)])


# ---------------------------------------------------------------------------------------------
# Moog's Voyager Structures idea: small and medium finds all over the Overworld and the Nether.
# ---------------------------------------------------------------------------------------------

def patch(c, rng, blocks, clear=6, round_=True, margin=0.0, y=0):
    """Fill the ground layer with a mix of blocks (round or square) and clear the air above."""
    cx, cz = (c.w - 1) / 2, (c.d - 1) / 2
    rx, rz = c.w / 2 - margin, c.d / 2 - margin
    for z in range(c.d):
        for x in range(c.w):
            if round_ and ((x - cx) / rx) ** 2 + ((z - cz) / rz) ** 2 > 1.0:
                continue
            c.set(x, y, z, rng.choice(blocks))
            c.box(x, y + 1, z, x, y + clear, z, AIR)


SMALL_LOOT = [("iron_ingot", 1, 4, 0.5), ("gold_ingot", 1, 3, 0.35), ("emerald", 1, 3, 0.35), ("bread", 1, 4, 0.5), ("coal", 2, 8, 0.5),
              ("diamond", 1, 1, 0.08), ("torch", 2, 8, 0.4)]


def small(*extra):
    return list(SMALL_LOOT) + list(extra)


@group("voyager")
def watchtower_ruin():
    rng = random.Random(41)
    c = Canvas(13, 18, 13)
    patch(c, rng, ["cobblestone", "gravel", "grass_block", "mossy_cobblestone"], clear=16)
    round_tower(c, 6, 6, 4.4, 0, 14, "stone_bricks", floor="cobblestone", crenel="stone_bricks", windows="iron_bars", every=4)
    spiral_stairs(c, circle_ring(6, 6, 2.5), 1, 12, "stone_brick_stairs")
    c.column(6, 6, 1, 13, "stone_bricks")
    c.box(5, 1, 10, 7, 3, 10, AIR)
    c.set(6, 1, 10, AIR)
    c.disc(6, 6, 3.4, 13, "spruce_planks")
    c.set(4, 14, 4, "chest")
    for (x, y, z, r) in ((10, 14, 7, 2.5), (3, 11, 9, 2.0), (6, 15, 2, 2.0)):
        bite(c, x, y, z, r)
    for (x, z) in ((11, 3), (12, 6), (10, 10), (2, 11), (1, 3)):
        c.set(x, 1, z, rng.choice(["cobblestone", "mossy_cobblestone", "stone_brick_slab"]))
    remove_floaters(c)
    c.weather(rng, {"stone_bricks": ["mossy_cobblestone", "cobblestone_bricks", "mossy_cobblestone_bricks"]}, 0.25)
    overgrow(c, rng, 0.06)
    return struct("torre_de_vigia_arruinada", ["Plains", "Forest", "Birch Forest", "Taiga"], c, 14, 0.4,
                  loot=small(("arrow", 4, 12, 0.6), ("bow", 1, 1, 0.25)), sink=1, foundation="cobblestone", mobs=[("skeleton", 2)])


@group("voyager")
def hunters_camp():
    rng = random.Random(42)
    c = Canvas(17, 8, 15)
    patch(c, rng, ["rocky_dirt", "dirt", "gravel", "grass_block", "grass_block"], clear=6)
    tent(c, 1, 2, 5, "brown_wool", "z", pole="spruce_fence")
    tent(c, 11, 2, 5, "green_wool", "z", pole="spruce_fence")
    c.set(3, 1, 3, "red_bed")
    c.set(13, 1, 3, "red_bed")
    c.set(3, 1, 5, "chest")
    c.disc(8, 10, 1.5, 0, "cobblestone")
    c.set(8, 1, 10, "brazier")
    for (x, z) in ((6, 10), (10, 10)):
        c.set(x, 1, z, "spruce_log")
    # Drying racks.
    for x in (5, 6, 7, 8, 9, 10, 11):
        c.set(x, 3, 13, "spruce_fence")
    for x in (5, 11):
        c.column(x, 13, 1, 2, "spruce_fence")
    for x in (6, 8, 10):
        c.set(x, 2, 13, "brown_wool")
    c.set(14, 1, 11, "hay_block")
    c.set(14, 1, 12, "spruce_log")
    c.set(15, 1, 12, "spruce_log")
    target_dummy(c, 1, 1, 12, "spruce_fence")
    return struct("acampamento_de_cacadores", ["Taiga", "Snowy Plains", "Forest"], c, 12, 0.4,
                  loot=small(("leather", 2, 6, 0.7), ("cooked_venison", 2, 5, 0.6), ("arrow", 4, 12, 0.6), ("bow", 1, 1, 0.3), ("rabbit_hide", 1, 4, 0.5)),
                  sink=1, foundation="dirt")


@group("voyager")
def abandoned_cabin():
    rng = random.Random(43)
    c = Canvas(11, 10, 11)
    patch(c, rng, ["dirt", "rocky_dirt", "grass_block", "moss_block"], clear=9, round_=False)
    st = style("dark_oak", roof="spruce_roof", base="mossy_cobblestone", porch_lamp=False)
    cottage(c, 2, 2, 8, 8, st, S, rng, wh=3, things=["chest", "bed", "table", "shelf", "bookshelf"])
    for (x, y, z, r) in ((7, 6, 3, 2.2), (3, 5, 7, 1.6)):
        bite(c, x, y, z, r)
    remove_floaters(c)
    c.weather(rng, {"dark_oak_planks": ["dark_oak_planks", "spruce_planks", AIR], "glass_pane": [AIR]}, 0.15)
    overgrow(c, rng, 0.08)
    for (x, z) in ((5, 5), (3, 4)):
        if c.get(x, 1, z) == AIR:
            c.set(x, 1, z, rng.choice(["brown_mushroom", "red_mushroom", "twig"]))
    return struct("cabana_abandonada", ["Dark Forest", "Swamp", "Taiga"], c, 12, 0.4,
                  loot=small(("string", 1, 4, 0.5), ("rotten_flesh", 1, 5, 0.6), ("bone", 1, 4, 0.5)), sink=1, foundation="dirt",
                  mobs=[("zombie", 2)])


@group("voyager")
def small_wreck():
    rng = random.Random(44)
    c = Canvas(20, 12, 11)
    patch(c, rng, ["sand", "sand", "gravel"], clear=10, round_=True)
    hw = hull(c, 1, 17, 5, 3.6, 4, y0=0, shell="oak_planks", stripe="spruce_planks", deck_b="oak_planks", rail="oak_fence", stern=4, bow=6)
    c.line(9, 4, 5, 13, 10, 8, "spruce_log")
    for z in range(5, 10):
        if rng.random() < 0.8:
            c.set(12, 9, z, "white_wool")
    c.box(6, 4, 4, 8, 4, 6, AIR)
    c.set(7, 1, 5, "chest")
    c.set(5, 1, 5, "spruce_drawer")
    for (x, y, z, r) in ((15, 4, 3, 2.5), (3, 4, 7, 2.0), (10, 5, 2, 1.5)):
        bite(c, x, y, z, r)
    remove_floaters(c)
    c.weather(rng, {"oak_planks": ["oak_planks", "spruce_planks", "dark_oak_planks"]}, 0.3)
    for (x, z) in ((2, 1), (16, 9), (18, 4)):
        c.set(x, 1, z, rng.choice(["oak_planks", "spruce_slab", "oak_slab"]))
    return struct("naufragio_pequeno", ["Beach"], c, 12, 0.45,
                  loot=small(("cod", 1, 4, 0.5), ("gold_nugget", 3, 9, 0.6), ("string", 1, 3, 0.3), ("emerald", 1, 4, 0.5),
                            ("leather_backpack", 1, 1, 0.08)), sink=2, foundation="sand", rough=5)


@group("voyager")
def desert_obelisk():
    rng = random.Random(45)
    c = Canvas(11, 28, 11)
    # A small chamber under the sand, then the stepped base and the needle.
    c.room(2, 0, 2, 8, 4, 8, "sandstone", floor="smooth_stone", ceil="sandstone")
    c.set(5, 1, 3, "chest")
    c.set(4, 1, 3, "gold_block")
    c.set(6, 1, 3, "gold_block")
    c.set(5, 3, 5, HANG)
    for (x, z) in ((3, 7), (7, 7)):
        c.set(x, 1, z, "orange_terracotta")
        c.set(x, 2, z, "brazier")
    for k, y in enumerate(range(5, 8)):
        c.walls(k, k, 10 - k, 10 - k, y, y, "sandstone")
        c.box(k + 1, y, k + 1, 9 - k, y, 9 - k, "sandstone")
    for y in range(8, 24):
        c.box(4, y, 4, 6, y, 6, "sandstone" if y % 5 else "orange_terracotta")
    c.box(5, 24, 5, 5, 25, 5, "gold_block")
    for y in (10, 14, 18):
        c.set(5, y, 4, "yellow_terracotta")
        c.set(5, y, 6, "yellow_terracotta")
        c.set(4, y, 5, "yellow_terracotta")
        c.set(6, y, 5, "yellow_terracotta")
    # A shaft down to the chamber through the south step.
    c.box(5, 1, 9, 5, 7, 9, AIR)
    c.box(5, 1, 8, 5, 2, 8, AIR)
    return struct("obelisco_do_deserto", ["Desert"], c, 16, 0.4, loot=small(("gold_ingot", 2, 6, 0.6), ("topaz", 1, 2, 0.3), ("bone", 1, 4, 0.5)),
                  sink=5, foundation="sandstone", mobs=[("zombie", 1)])


@group("voyager")
def mossy_statue():
    """A giant old statue of a kneeling knight, gone green with moss, a chest hidden in a niche
    in its plinth."""
    rng = random.Random(46)
    c = Canvas(13, 20, 11)
    patch(c, rng, ["mossy_cobblestone", "moss_block", "grass_block", "cobblestone"], clear=18)
    S_ = "stone_bricks"
    c.box(2, 1, 2, 10, 2, 8, S_)
    c.box(3, 3, 3, 9, 3, 7, "stone_brick_slab")
    # Legs (kneeling), body, arms resting on a sword, head.
    c.box(4, 4, 4, 5, 6, 6, S_)
    c.box(7, 4, 3, 8, 4, 6, S_)
    c.box(7, 5, 3, 8, 6, 4, S_)
    c.box(4, 7, 4, 8, 12, 6, S_)
    c.box(3, 11, 4, 9, 12, 6, S_)
    c.box(2, 8, 5, 3, 12, 5, S_)
    c.box(9, 8, 5, 10, 12, 5, S_)
    c.box(5, 13, 4, 7, 15, 6, S_)
    c.set(6, 16, 5, "stone_brick_slab")
    c.set(5, 14, 3, "glowstone")
    c.set(7, 14, 3, "glowstone")
    c.column(6, 3, 4, 9, "iron_bars")
    c.box(5, 9, 3, 7, 9, 3, "iron_block")
    c.set(6, 10, 3, "iron_block")
    # Niche with the chest.
    c.box(6, 1, 8, 6, 2, 8, AIR)
    c.set(6, 1, 7, "chest")
    c.weather(rng, {S_: ["mossy_cobblestone", "mossy_cobblestone_bricks", "moss_block", "cobblestone_bricks"]}, 0.45)
    overgrow(c, rng, 0.12)
    return struct("estatua_musgosa", ["Jungle", "Forest", "Dark Forest", "Birch Forest"], c, 16, 0.4, loot=small(("iron_sword", 1, 1, 0.3), ("mana_crystal", 1, 1, 0.2)),
                  sink=1, foundation="mossy_cobblestone")


@group("voyager")
def trader_camp():
    """A travelling trader's camp: a covered wagon, stalls under coloured awnings, hay and
    crates, lanterns on posts."""
    rng = random.Random(47)
    c = Canvas(17, 8, 15)
    patch(c, rng, ["rocky_dirt", "gravel", "dirt", "rocky_dirt"], clear=7)
    # Wagon.
    c.box(2, 2, 2, 8, 2, 5, "oak_planks")
    for (x, z) in ((3, 1), (7, 1), (3, 6), (7, 6)):
        c.set(x, 1, z, "dark_oak_log")
    c.set(2, 3, 2, "oak_fence")
    for x in range(2, 9):
        c.set(x, 3, 2, "white_wool")
        c.set(x, 3, 5, "white_wool")
        c.set(x, 4, 2, "white_wool")
        c.set(x, 4, 5, "white_wool")
        c.set(x, 5, 3, "white_wool")
        c.set(x, 5, 4, "white_wool")
    c.set(4, 3, 3, "chest")
    c.set(6, 3, 4, "hay_block")
    c.line(9, 2, 3, 12, 1, 3, "oak_fence")
    c.line(9, 2, 4, 12, 1, 4, "oak_fence")
    # Stalls.
    for (x0, z0, col) in ((2, 9, "yellow_wool"), (8, 9, "lime_wool"), (12, 3, "light_blue_wool")):
        for (dx, dz) in ((0, 0), (3, 0), (0, 2), (3, 2)):
            c.column(x0 + dx, z0 + dz, 1, 2, "oak_fence")
        c.box(x0, 3, z0, x0 + 3, 3, z0 + 2, col)
        c.set(x0 + 1, 1, z0 + 2, "oak_table")
        c.set(x0 + 2, 1, z0 + 2, "oak_table")
        c.set(x0 + 1, 2, z0 + 2, rng.choice(["melon", "pumpkin", "lantern"]))
        c.set(x0 + 2, 2, z0 + 2, rng.choice(["potted_poppy", "hay_block", "botany_pot_wheat"]))
    for (x, z) in ((1, 13), (15, 13), (15, 1)):
        lamp_post(c, x, 1, z, "oak_fence", "lantern", 2)
    c.set(14, 1, 9, "hay_block")
    c.set(14, 2, 9, "hay_block")
    c.set(13, 1, 9, "oak_planks")
    return struct("acampamento_de_mercador", ["Savanna", "Plains", "Desert", "Birch Forest"], c, 14, 0.4,
                  loot=small(("emerald", 2, 6, 0.7), ("apple", 2, 5, 0.5), ("lute", 1, 1, 0.1), ("leather_backpack", 1, 1, 0.1)),
                  sink=1, foundation="dirt", mobs=[("goblin_trader", 1), ("npc_merchant", 1)])


@group("voyager")
def archery_range():
    rng = random.Random(48)
    c = Canvas(23, 8, 11)
    patch(c, rng, ["grass_block", "grass_block", "gravel"], clear=7, round_=False)
    # Shooting shed.
    for (x, z) in ((1, 1), (1, 9), (5, 1), (5, 9)):
        c.column(x, z, 1, 3, "spruce_log")
    for z in range(0, 11):
        for x in range(0, 7):
            c.set(x, 4, z, stair("spruce_roof", E) if x < 3 else (stair("spruce_roof", W) if x > 3 else "spruce_slab"))
    for z in range(2, 9):
        c.set(5, 1, z, "spruce_fence" if z % 2 else "spruce_slab")
    c.set(2, 1, 2, "chest")
    c.set(2, 1, 4, "hay_block")
    c.set(2, 1, 6, "spruce_table")
    c.set(2, 2, 6, "lantern")
    c.set(2, 1, 8, var("spruce_bench", E))
    # Targets at three distances.
    for (x, z) in ((11, 3), (16, 7), (21, 4)):
        c.column(x, z, 1, 1, "spruce_fence")
        c.box(x, 2, z - 1, x, 4, z + 1, "white_wool")
        c.set(x, 3, z, "red_wool")
        c.set(x, 2, z, "red_wool")
        c.set(x, 4, z, "red_wool")
        c.set(x, 3, z - 1, "red_wool")
        c.set(x, 3, z + 1, "red_wool")
        c.set(x, 3, z, "yellow_wool")
        c.set(x + 1, 1, z, "hay_block")
    for x in range(7, 22, 2):
        c.set(x, 0, 0, "gravel")
        c.set(x, 1, 10, "spruce_fence")
    return struct("campo_de_tiro", ["Plains", "Birch Forest", "Forest"], c, 14, 0.35,
                  loot=small(("arrow", 8, 24, 0.9), ("bow", 1, 1, 0.5), ("feather", 2, 6, 0.5)), sink=1, foundation="dirt")


@group("voyager")
def ice_shrine():
    rng = random.Random(49)
    c = Canvas(17, 18, 17)
    patch(c, rng, ["snow", "packed_ice", "snowy_grass_block", "snow"], clear=16)
    c.disc(8, 8, 3.5, 1, "packed_ice")
    c.disc(8, 8, 2.5, 2, "ice")
    c.set(8, 3, 8, "chest")
    c.set(8, 2, 8, "packed_ice")
    for (x, z) in ((8, 6), (8, 10), (6, 8), (10, 8)):
        c.set(x, 3, z, "lantern")
    for k in range(8):
        a = k * pi / 4
        x, z = 8 + cos(a) * 6.2, 8 + sin(a) * 6.2
        h = rng.randint(7, 13)
        for y in range(1, h):
            r = 1.6 * (1 - y / h) + 0.4
            c.disc(x, z, r, y, "packed_ice")
        c.set(round(x), h, round(z), "ice")
    return struct("santuario_de_gelo", ["Snowy Plains"], c, 14, 0.4, loot=small(("sapphire", 1, 3, 0.4), ("frost_staff", 1, 1, 0.1), ("packed_ice", 2, 6, 0.5)),
                  sink=1, foundation="packed_ice", mobs=[("skeleton", 1)])


@group("voyager")
def jungle_temple_ruin():
    rng = random.Random(50)
    c = Canvas(17, 14, 17)
    patch(c, rng, ["mossy_cobblestone", "moss_block", "grass_block", "cobblestone"], clear=12, round_=False)
    M = "mossy_cobblestone_bricks"
    for k, y in enumerate(range(1, 5)):
        c.walls(1 + k * 2, 1 + k * 2, 15 - k * 2, 15 - k * 2, y, y, M)
        c.box(2 + k * 2, y, 2 + k * 2, 14 - k * 2, y, 14 - k * 2, "cobblestone_bricks" if k < 3 else M)
    for z in range(1, 9):
        y = 1 + (z - 1) // 2
        c.set(8, y, z, stair("mossy_cobblestone_stairs", S))
        c.set(7, y, z, stair("mossy_cobblestone_stairs", S))
        c.set(9, y, z, stair("mossy_cobblestone_stairs", S))
    c.room(6, 5, 6, 10, 9, 10, M, ceil=M)
    c.box(7, 5, 10, 9, 7, 10, AIR)
    c.set(8, 5, 7, "chest")
    c.set(7, 5, 7, "gold_block")
    c.set(9, 5, 7, "gold_block")
    c.set(8, 8, 8, HANG)
    for (x, z) in ((1, 1), (15, 1), (1, 15), (15, 15)):
        c.column(x, z, 1, rng.randint(3, 7), M)
    for (x, y, z, r) in ((14, 4, 4, 2.5), (6, 9, 6, 1.8), (2, 2, 13, 2.0)):
        bite(c, x, y, z, r)
    remove_floaters(c)
    c.weather(rng, {M: ["mossy_cobblestone", "cobblestone_bricks", "moss_block"]}, 0.2)
    overgrow(c, rng, 0.15)
    return struct("ruinas_de_templo", ["Jungle"], c, 16, 0.4, loot=small(("emerald", 2, 5, 0.6), ("ruby", 1, 2, 0.3), ("glow_berries", 1, 4, 0.4)),
                  sink=1, foundation="mossy_cobblestone", mobs=[("zombie", 1), ("spider", 1)])


@group("voyager")
def graveyard():
    rng = random.Random(51)
    c = Canvas(19, 9, 17)
    patch(c, rng, ["grass_block", "grass_block", "dirt", "rocky_dirt", "grass_block"], clear=8, round_=False)
    c.walls(0, 0, 18, 16, 1, 1, "dark_oak_fence")
    for (x, z) in square_ring(0, 0, 18, 16)[::4]:
        c.set(x, 1, z, "cobblestone_wall")
        if rng.random() < 0.3:
            c.set(x, 2, z, "lantern")
    c.box(8, 1, 16, 10, 1, 16, AIR)
    c.column(8, 16, 1, 3, "cobblestone_wall")
    c.column(10, 16, 1, 3, "cobblestone_wall")
    c.box(8, 4, 16, 10, 4, 16, "dark_oak_slab")
    for z in range(3, 12, 3):
        for x in range(2, 16, 3):
            if 7 <= x <= 10:
                continue
            c.set(x, 1, z, rng.choice(["cobblestone_wall", "stone_brick_slab", "mossy_cobblestone", "cobblestone_wall"]))
            if c.get(x, 1, z) == "cobblestone_wall" and rng.random() < 0.5:
                c.set(x, 2, z, "stone_brick_slab")
            c.set(x, 0, z + 1, "dirt")
            if rng.random() < 0.4:
                c.set(x, 1, z + 1, rng.choice(["poppy", "dead_bush", "short_grass"]))
    for z in range(1, 16):
        c.set(8, 0, z, "gravel")
        c.set(9, 0, z, "gravel")
    # The mausoleum.
    c.room(6, 0, 1, 12, 5, 6, "stone_bricks", floor="stone_bricks", ceil="stone_brick_slab")
    c.box(8, 1, 6, 9, 3, 6, AIR)
    c.set(9, 1, 2, "chest")
    c.set(8, 1, 2, "stone_brick_slab")
    c.set(10, 1, 2, "stone_brick_slab")
    c.set(9, 4, 4, HANG)
    gable(c, 6, 1, 12, 6, 6, "stone_brick_stairs", "stone_bricks", "stone_brick_slab", axis="z", over=0)
    c.weather(rng, {"stone_bricks": ["mossy_cobblestone_bricks", "cobblestone_bricks"]}, 0.25)
    return struct("cemiterio", ["Plains", "Dark Forest", "Taiga", "Forest"], c, 16, 0.35,
                  loot=small(("bone", 2, 8, 0.8), ("rotten_flesh", 2, 6, 0.6), ("gold_nugget", 2, 8, 0.5), ("mana_crystal", 1, 1, 0.1)),
                  sink=1, foundation="dirt", mobs=[("zombie", 3), ("skeleton", 2)])


@group("voyager")
def fishing_hut():
    rng = random.Random(52)
    c = Canvas(13, 11, 15)
    patch(c, rng, ["dirt", "grass_block", "moss_block", "rocky_dirt"], clear=10, round_=False)
    c.box(1, 0, 8, 11, 0, 14, "water")
    c.box(1, -1, 8, 11, -1, 14, "dirt")
    for (x, z) in ((2, 2), (8, 2), (2, 9), (8, 9), (5, 13), (7, 13)):
        c.column(x, z, 0, 2, "spruce_log")
    st = style("spruce", roof="spruce_roof", floor="spruce_planks", wall="spruce_planks", corner="spruce_log", porch_lamp=False)
    cottage(c, 2, 2, 8, 7, st, S, rng, y=3, wh=3, things=["chest", "bed", "table", "shelf", "furnace"])
    for z in range(8, 14):
        for x in (5, 6, 7):
            c.set(x, 2, z, "spruce_slab:1")
    c.set(5, 3, 13, "spruce_fence")
    c.set(7, 3, 13, "spruce_fence")
    c.set(7, 4, 13, "lantern")
    c.set(6, 3, 12, "hay_block")
    for k in range(3):
        c.set(4 - k, 1 - (k // 2), 7 + k, stair("spruce_stairs", N))
    c.set(3, 1, 11, "lily_pad")
    c.set(10, 1, 9, "lily_pad")
    return struct("cabana_de_pesca", ["Swamp", "Beach", "Taiga"], c, 14, 0.35,
                  loot=small(("cod", 2, 6, 0.7), ("salmon", 1, 4, 0.5), ("fishing_rod", 1, 1, 0.4), ("string", 1, 4, 0.4)), sink=1, foundation="dirt", rough=4)


@group("voyager")
def ancient_gate():
    rng = random.Random(53)
    c = Canvas(19, 16, 9)
    patch(c, rng, ["stone", "cobblestone", "gravel", "grass_block"], clear=14, round_=False)
    S_ = "stone_bricks"
    for x0 in (5, 11):
        c.box(x0, 1, 3, x0 + 2, 10, 5, S_)
        c.box(x0, 11, 3, x0 + 2, 11, 5, "smooth_stone")
    c.box(5, 12, 3, 13, 13, 5, S_)
    c.box(8, 11, 3, 10, 11, 5, S_)
    for x in (8, 10):
        c.set(x, 11, 4, stair("stone_brick_stairs", W if x == 8 else E, up=True))
    c.set(9, 14, 4, "stone_brick_slab")
    c.set(6, 14, 4, "stone_brick_slab")
    c.set(12, 14, 4, "stone_brick_slab")
    for x in (4, 14):
        c.set(x, 1, 2, "lantern")
    for x in list(range(0, 5)) + list(range(14, 19)):
        h = rng.randint(1, 4)
        c.box(x, 1, 3, x, h, 5, S_)
    c.set(9, 1, 4, "chest")
    c.set(9, 1, 3, "stone_brick_slab")
    c.set(9, 1, 5, "stone_brick_slab")
    for (x, y, z, r) in ((13, 13, 4, 2.0), (2, 4, 4, 1.8)):
        bite(c, x, y, z, r)
    remove_floaters(c)
    c.weather(rng, {S_: ["mossy_cobblestone", "cobblestone_bricks", "mossy_cobblestone_bricks", S_]}, 0.3)
    overgrow(c, rng, 0.08)
    return struct("portao_antigo", ["Mountains", "Plains", "Taiga", "Snowy Plains"], c, 16, 0.35,
                  loot=small(("iron_helmet", 1, 1, 0.2), ("lapis_lazuli", 2, 6, 0.4)), sink=1, foundation="stone_bricks", rough=5)


@group("voyager")
def miners_hut():
    rng = random.Random(54)
    c = Canvas(19, 12, 15)
    patch(c, rng, ["gravel", "stone", "rocky_dirt", "cobblestone"], clear=11, round_=False)
    st = style("spruce", roof="spruce_roof", base="cobblestone", floor="spruce_planks")
    cottage(c, 1, 2, 7, 8, st, S, rng, wh=3, things=["chest", "bed", "furnace", "crafting_table", "shelf"])
    # A rocky mound with a timbered tunnel mouth and rails.
    for z in range(1, 14):
        for x in range(10, 19):
            h = int(6 - abs(z - 7) * 0.5 - max(0, 13 - x) * 1.2 + rng.random())
            for y in range(1, max(h, 0) + 1):
                c.set(x, y, z, rng.choice(["stone", "stone", "cobblestone", "gravel"]) if y < h else "grass_block")
    for x in range(11, 19):
        c.box(x, 1, 6, x, 3, 8, AIR)
        if x % 3 == 2:
            c.column(x, 6, 1, 3, "spruce_log")
            c.column(x, 8, 1, 3, "spruce_log")
            c.box(x, 4, 6, x, 4, 8, "spruce_planks")
        c.set(x, 1, 7, "rail:1")
    for x in range(6, 11):
        c.set(x, 1, 7, "rail:1")
    c.set(12, 3, 7, HANG)
    for (x, z, b) in ((8, 11, "iron_ore"), (9, 11, "coal_ore"), (8, 12, "copper_ore"), (9, 12, "gravel")):
        c.set(x, 1, z, b)
    return struct("cabana_de_mineiro", ["Mountains", "Badlands", "Taiga"], c, 14, 0.35,
                  loot=small(("raw_iron", 2, 6, 0.6), ("raw_copper", 2, 6, 0.5), ("iron_pickaxe", 1, 1, 0.2), ("rail", 4, 12, 0.4), ("tnt", 1, 2, 0.15)),
                  sink=1, foundation="cobblestone", rough=6)


@group("voyager")
def palisade_fort():
    rng = random.Random(55)
    Sz = 21
    c = Canvas(Sz, 14, Sz)
    patch(c, rng, ["dirt", "rocky_dirt", "grass_block", "gravel", "grass_block"], clear=12, round_=False)
    for (x, z) in square_ring(1, 1, Sz - 2, Sz - 2):
        c.column(x, z, 1, 4 + (x + z) % 2, "spruce_log")
    c.box(9, 1, Sz - 2, 11, 4, Sz - 2, AIR)
    for (x, z) in ((2, 2), (Sz - 6, 2)):
        for (dx, dz) in ((0, 0), (3, 0), (0, 3), (3, 3)):
            c.column(x + dx, z + dz, 1, 5, "spruce_log")
        c.box(x, 6, z, x + 3, 6, z + 3, "spruce_planks")
        c.walls(x, z, x + 3, z + 3, 7, 7, "spruce_fence")
        c.set(x + 1, 8, z + 1, "lantern")
    stair_run(c, 4, 1, 11, N, 5, "spruce_stairs", clear=2)
    c.box(4, 6, 6, 4, 6, 6, "spruce_planks")
    st = style("spruce", roof="spruce_roof", base="cobblestone", floor="spruce_planks", porch_lamp=False)
    cottage(c, 11, 10, 17, 16, st, W, rng, wh=3, things=["chest", "bed", "bed", "table", "furnace", "shelf"])
    tent(c, 3, 13, 4, "brown_wool", "z", pole="spruce_fence")
    c.set(10, 1, 9, "brazier")
    target_dummy(c, 14, 1, 6, "spruce_fence")
    flag(c, Sz - 4, 8, 4, "spruce_fence", "red_wool", h=4)
    return struct("forte_de_palicada", ["Taiga", "Snowy Plains", "Forest"], c, 18, 0.35,
                  loot=small(("arrow", 4, 12, 0.6), ("iron_sword", 1, 1, 0.2), ("leather", 1, 4, 0.5)), sink=1, foundation="dirt", rough=5,
                  mobs=[("marauder", 2)])


@group("voyager")
def cherry_garden():
    rng = random.Random(56)
    c = Canvas(19, 12, 19)
    patch(c, rng, ["grass_block", "grass_block", "moss_block"], clear=11, round_=False)
    # Gate of cherry posts and red beams.
    for x in (6, 12):
        c.column(x, 17, 1, 5, "cherry_log")
    c.box(5, 6, 17, 13, 6, 17, "red_wool")
    c.box(6, 4, 17, 12, 4, 17, "red_wool")
    c.set(5, 7, 17, "cherry_slab")
    c.set(13, 7, 17, "cherry_slab")
    for z in range(8, 19):
        c.set(9, 0, z, "smooth_stone")
        c.set(8, 0, z, "gravel")
        c.set(10, 0, z, "gravel")
    # Pond.
    c.disc(4, 6, 2.6, 0, "water")
    c.set(4, 1, 6, "lily_pad")
    c.disc(4, 6, 3.6, 1, "stone_brick_slab", hollow=True)
    # Pavilion.
    for (x, z) in ((7, 2), (11, 2), (7, 6), (11, 6)):
        c.column(x, z, 1, 3, "cherry_log")
    c.box(7, 0, 2, 11, 0, 6, "cherry_planks")
    hip(c, 7, 2, 11, 6, 4, "brick_roof", "red_paper_lamp")
    c.set(9, 1, 4, "cherry_table")
    c.set(8, 1, 4, var("cherry_chair", E))
    c.set(9, 1, 3, "chest")
    c.set(9, 3, 4, "paper_lamp")
    for (x, z) in ((7, 10), (11, 10), (7, 14), (11, 14)):
        c.set(x, 1, z, "stone_bricks")
        c.set(x, 2, z, "paper_lamp")
    for (x, z) in ((15, 4), (15, 12), (3, 13)):
        c.column(x, z, 1, 3, "cherry_log")
        for dy, r in ((3, 2.3), (4, 2.5), (5, 1.5)):
            for zz in range(z - 3, z + 4):
                for xx in range(x - 3, x + 4):
                    if (xx - x) ** 2 + (zz - z) ** 2 <= r * r and c.get(xx, dy, zz) == AIR:
                        c.set(xx, dy, zz, "cherry_leaves")
    scatter_plants(c, 0, 0, 18, 18, rng, ["poppy", "short_grass", "dandelion"], p=0.12)
    return struct("jardim_de_cerejeira", ["Cherry Grove"], c, 14, 0.4, loot=small(("cherry_sapling", 1, 3, 0.6), ("flute", 1, 1, 0.12)),
                  sink=1, foundation="dirt", rough=4)


@group("voyager")
def orchard():
    rng = random.Random(57)
    c = Canvas(19, 9, 17)
    patch(c, rng, ["grass_block"], clear=8, round_=False)
    c.walls(0, 0, 18, 16, 1, 1, "oak_fence")
    c.box(8, 1, 16, 10, 1, 16, AIR)
    for (x, z) in ((4, 4), (9, 4), (14, 4), (4, 10), (14, 10)):
        c.column(x, z, 1, 3, "oak_log")
        for dy, r in ((3, 2.2), (4, 2.2), (5, 1.2)):
            for zz in range(z - 3, z + 4):
                for xx in range(x - 3, x + 4):
                    if (xx - x) ** 2 + (zz - z) ** 2 <= r * r and c.get(xx, dy, zz) == AIR:
                        c.set(xx, dy, zz, "oak_leaves")
    st = style("oak", porch_lamp=False)
    cottage(c, 7, 9, 11, 13, st, S, rng, wh=3, things=["chest", "crafting_table", "shelf"])
    for (x, z) in ((2, 14), (3, 14), (16, 14), (16, 13)):
        c.set(x, 1, z, rng.choice(["hay_block", "pumpkin", "melon"]))
    return struct("pomar", ["Plains", "Forest", "Birch Forest"], c, 14, 0.35, loot=small(("apple", 3, 9, 0.9), ("oak_sapling", 1, 3, 0.5)),
                  sink=1, foundation="dirt", rough=4)


@group("voyager")
def knights_grave():
    rng = random.Random(58)
    c = Canvas(9, 9, 11)
    # A buried chest under the grave slab.
    c.room(2, 0, 3, 6, 3, 7, "cobblestone", floor="cobblestone", ceil="cobblestone")
    c.set(4, 1, 5, "chest")
    c.set(4, 2, 5, AIR)
    for z in range(11):
        for x in range(9):
            c.set(x, 3, z, rng.choice(["grass_block", "grass_block", "gravel"]))
            c.box(x, 4, z, x, 8, z, AIR)
    c.box(3, 4, 3, 5, 4, 7, "stone_brick_slab")
    c.set(4, 4, 2, "stone_bricks")
    c.set(4, 5, 2, "stone_bricks")
    c.set(4, 6, 2, "stone_brick_slab")
    c.column(4, 5, 5, 6, "iron_bars")
    c.set(4, 7, 5, "oak_fence")
    for (x, z) in ((1, 1), (7, 1), (1, 9), (7, 9)):
        c.column(x, z, 4, 5, "cobblestone_wall")
        c.set(x, 6, z, "lantern")
    for (x, z) in ((2, 5), (6, 5), (4, 8)):
        c.set(x, 4, z, rng.choice(["poppy", "dandelion"]))
    return struct("tumulo_do_cavaleiro", ["Plains", "Taiga", "Forest", "Birch Forest"], c, 14, 0.3,
                  loot=small(("iron_sword", 1, 1, 0.5), ("iron_chestplate", 1, 1, 0.2), ("golden_helmet", 1, 1, 0.2)), sink=4, foundation="cobblestone")


@group("voyager")
def savanna_lookout():
    rng = random.Random(59)
    c = Canvas(11, 14, 11)
    patch(c, rng, ["grass_block", "rocky_dirt", "dirt"], clear=13)
    for (x, z) in ((3, 3), (7, 3), (3, 7), (7, 7)):
        c.column(x, z, 1, 7, "acacia_log")
    c.box(2, 8, 2, 8, 8, 8, "acacia_planks")
    c.walls(2, 2, 8, 8, 9, 9, "acacia_fence")
    for (x, z) in ((2, 2), (8, 2), (2, 8), (8, 8)):
        c.column(x, z, 9, 10, "acacia_fence")
    hip(c, 2, 2, 8, 8, 11, "thatch_roof", "thatch")
    stair_run(c, 9, 1, 8, N, 7, "acacia_stairs", clear=2)
    c.set(9, 8, 2, "acacia_planks")
    c.set(8, 9, 2, AIR)
    c.set(5, 9, 5, "chest")
    c.set(4, 9, 5, "lantern")
    c.set(6, 9, 4, var("acacia_chair", S))
    return struct("posto_da_savana", ["Savanna"], c, 14, 0.4, loot=small(("arrow", 3, 9, 0.6), ("leather", 1, 4, 0.5)), sink=1, foundation="dirt",
                  mobs=[("marauder", 1)])


@group("voyager")
def snowy_igloo():
    rng = random.Random(60)
    c = Canvas(11, 12, 11)
    # Cellar under it with a chest and a cell.
    c.room(1, 0, 1, 9, 4, 9, "stone_bricks", floor="stone_bricks", ceil="stone_bricks")
    c.set(2, 1, 2, "chest")
    c.set(3, 1, 2, "spruce_table")
    c.set(3, 2, 2, "lantern")
    c.box(6, 1, 6, 8, 3, 8, "iron_bars")
    c.box(7, 1, 7, 8, 3, 8, AIR)
    c.set(8, 1, 8, "red_bed")
    c.set(2, 1, 8, "bookshelf")
    c.set(2, 2, 8, "bookshelf")
    c.set(6, 3, 4, HANG)
    # The igloo on top.
    for z in range(11):
        for x in range(11):
            c.set(x, 4, z, rng.choice(["snow", "snowy_grass_block", "snow"]))
            c.box(x, 5, z, x, 11, z, AIR)
    for y, r in ((5, 3.6), (6, 3.4), (7, 2.8), (8, 1.8)):
        c.disc(5, 5, r, y, "snow", hollow=(y < 8), thick=1.0)
    for k in range(3, 6):
        c.set(5, 5, 5 + k, AIR)
        c.set(5, 6, 5 + k, AIR)
    for (x, z) in ((4, 9), (6, 9), (4, 10), (6, 10)):
        c.set(x, 5, z, "snow")
    c.box(4, 6, 9, 6, 6, 10, "snow")
    c.set(5, 6, 9, AIR)
    c.set(5, 6, 10, AIR)
    c.set(3, 5, 4, "red_bed")
    c.set(7, 5, 4, "furnace")
    c.set(5, 7, 5, HANG)
    c.set(5, 4, 6, "white_wool")
    # A hole in the floor drops into the cellar.
    c.set(4, 4, 5, AIR)
    return struct("iglu_abandonado", ["Snowy Plains"], c, 14, 0.35, loot=small(("sapphire", 1, 2, 0.3), ("cooked_salmon", 1, 4, 0.5), ("frost_staff", 1, 1, 0.06)),
                  sink=5, foundation="snow", rough=4, mobs=[("zombie", 1)])


@group("voyager")
def badlands_ruins():
    rng = random.Random(61)
    c = Canvas(17, 12, 17)
    patch(c, rng, ["red_sand", "terracotta", "orange_terracotta", "red_sand"], clear=11, round_=False)
    for (x0, z0, x1, z1, h) in ((1, 1, 7, 6, 4), (9, 2, 15, 8, 5), (3, 9, 10, 15, 4)):
        c.room(x0, 1, z0, x1, h, z1, "orange_terracotta")
        c.walls(x0, z0, x1, z1, 1, 1, "brown_terracotta")
        mx = (x0 + x1) // 2
        c.box(mx, 1, z1, mx, 2, z1, AIR)
        for x in range(x0 + 2, x1 - 1, 3):
            c.set(x, 2, z0, "dark_oak_fence")
    c.room(12, 1, 11, 15, 9, 14, "red_terracotta")
    c.box(13, 1, 11, 13, 2, 11, AIR)
    c.set(13, 1, 13, "chest")
    for (x, y, z, r) in ((5, 4, 1, 2.5), (15, 5, 5, 2.2), (6, 4, 14, 2.0), (15, 9, 14, 2.5), (1, 3, 3, 1.5)):
        bite(c, x, y, z, r)
    remove_floaters(c)
    for (x, z) in ((8, 8), (1, 15), (16, 1)):
        c.set(x, 1, z, rng.choice(["dead_bush", "cactus"]))
    return struct("ruinas_das_badlands", ["Badlands"], c, 14, 0.4, loot=small(("gold_ingot", 2, 5, 0.6), ("raw_gold", 1, 4, 0.5), ("ruby", 1, 2, 0.2)),
                  sink=1, foundation="terracotta", mobs=[("zombie", 2)])


@group("voyager")
def broken_wagon():
    rng = random.Random(62)
    c = Canvas(13, 7, 11)
    patch(c, rng, ["grass_block", "dirt", "gravel", "rocky_dirt"], clear=6)
    for x in range(0, 13):
        c.set(x, 0, 5, "gravel")
        c.set(x, 0, 6, "dirt")
    c.box(3, 2, 3, 8, 2, 7, "oak_planks")
    c.set(3, 2, 7, "oak_slab")
    c.set(8, 2, 3, AIR)
    for (x, z) in ((4, 2), (7, 2), (4, 8)):
        c.set(x, 1, z, "dark_oak_log")
    c.set(7, 1, 9, "dark_oak_log")
    c.set(8, 1, 9, "dark_oak_log")
    for x in (3, 8):
        c.set(x, 3, 3, "oak_fence")
        c.set(x, 3, 7, "oak_fence")
    for x in range(3, 9):
        c.set(x, 4, 3, "white_wool")
        if x < 6:
            c.set(x, 4, 7, "white_wool")
        if x < 7:
            c.set(x, 5, 5, "white_wool")
    c.set(5, 3, 5, "chest")
    c.set(6, 3, 4, "hay_block")
    c.set(10, 1, 3, "pumpkin")
    c.set(11, 1, 7, "melon")
    c.set(10, 1, 8, "hay_block")
    c.line(9, 2, 4, 12, 1, 4, "oak_fence")
    return struct("carroca_quebrada", ["Plains", "Savanna", "Forest", "Birch Forest"], c, 12, 0.35,
                  loot=small(("bread", 2, 6, 0.6), ("wheat", 2, 8, 0.5), ("emerald", 1, 2, 0.3), ("leather_backpack", 1, 1, 0.05)), sink=1, foundation="dirt")


NETHER_LOOT = [("gold_ingot", 2, 6, 0.6), ("gold_nugget", 4, 12, 0.6), ("quartz", 3, 9, 0.6), ("blaze_rod", 1, 2, 0.3),
               ("nether_wart", 2, 6, 0.4), ("obsidian", 1, 3, 0.3), ("diamond", 1, 1, 0.12), ("fire_staff", 1, 1, 0.06),
               ("magma_block", 1, 4, 0.3)]
NETHER_ALL = ["Nether Wastes", "Soul Sand Valley", "Crimson Forest", "Warped Forest"]


@group("voyager")
def nether_bastion_ruin():
    rng = random.Random(71)
    Sz = 19
    c = Canvas(Sz, 12, Sz)
    patch(c, rng, ["blackstone", "blackstone", "netherrack", "magma_block"], clear=10, round_=False)
    BS = "blackstone"
    c.walls(1, 1, Sz - 2, Sz - 2, 1, 7, BS)
    for (x, z) in ((1, 1), (Sz - 2, 1), (1, Sz - 2), (Sz - 2, Sz - 2)):
        c.box(x - 1, 1, z - 1, x + 1, 9, z + 1, BS)
        c.set(x, 10, z, "blackstone_slab")
    for (x, z) in square_ring(1, 1, Sz - 2, Sz - 2)[::3]:
        c.set(x, 8, z, BS)
    for (x, z) in square_ring(1, 1, Sz - 2, Sz - 2)[1::7]:
        c.set(x, 4, z, "gold_block")
    c.box(9, 1, Sz - 2, 9, 3, Sz - 2, AIR)
    c.box(7, 0, 7, 11, 0, 11, "blackstone")
    c.box(8, 0, 8, 10, 0, 10, "lava")
    c.walls(7, 7, 11, 11, 1, 1, "blackstone_slab")
    c.box(3, 1, 3, 6, 4, 6, BS)
    c.box(4, 1, 4, 5, 3, 5, AIR)
    c.set(4, 1, 4, "chest")
    c.set(5, 1, 4, "gold_block")
    c.box(5, 1, 6, 5, 2, 6, AIR)
    c.set(4, 3, 5, HANG)
    for (x, z) in ((14, 4), (15, 5), (14, 13), (4, 14)):
        c.set(x, 1, z, rng.choice(["gold_block", "nether_gold_ore", "blackstone_slab"]))
    for (x, y, z, r) in ((Sz - 2, 7, 6, 2.5), (6, 8, 1, 2.2), (1, 6, 13, 2.0), (Sz - 2, 10, Sz - 2, 2.0)):
        bite(c, x, y, z, r)
    remove_floaters(c)
    c.weather(rng, {BS: [BS, "nether_bricks", "basalt"]}, 0.15)
    return struct("bastiao_em_ruinas", ["Nether Wastes", "Crimson Forest"], c, 12, 0.45, loot=list(NETHER_LOOT) + [("gold_block", 1, 2, 0.15)],
                  sink=1, foundation="blackstone", mobs=[("nether_zombie", 2), ("ash_skeleton", 2)])


@group("voyager")
def soul_temple():
    rng = random.Random(72)
    Sz = 15
    c = Canvas(Sz, 11, Sz)
    patch(c, rng, ["soul_soil", "soul_sand", "soul_soil"], clear=10, round_=False)
    c.box(2, 0, 2, 12, 0, 12, "blackstone")
    c.room(3, 1, 3, 11, 7, 11, "blackstone", ceil="blackstone_slab")
    c.walls(3, 3, 11, 11, 4, 4, "quartz_block")
    for (x, z) in ((3, 3), (11, 3), (3, 11), (11, 11)):
        c.column(x, z, 1, 9, "basalt")
        c.set(x, 10, z, "brazier")
    c.box(6, 1, 11, 8, 3, 11, AIR)
    for (x, z) in ((5, 11), (9, 11)):
        c.column(x, 12, 1, 3, "basalt")
        c.set(x, 4, 12, "brazier")
    c.box(6, 1, 5, 8, 1, 6, "quartz_block")
    c.set(7, 2, 5, "chest")
    c.set(6, 2, 5, "quartz_slab")
    c.set(8, 2, 5, "quartz_slab")
    c.set(7, 6, 7, HANG)
    for x in (4, 10):
        for z in (5, 7, 9):
            c.set(x, 1, z, "soul_sand")
            c.set(x, 2, z, "nether_wart_stage3")
    hip(c, 3, 3, 11, 11, 8, "blackstone_stairs", "basalt", over=0)
    return struct("templo_das_almas", ["Soul Sand Valley"], c, 12, 0.45, loot=list(NETHER_LOOT) + [("bone", 2, 6, 0.6)],
                  sink=1, foundation="soul_soil", mobs=[("ash_skeleton", 3)])


@group("voyager")
def crimson_hut():
    rng = random.Random(73)
    c = Canvas(13, 10, 13)
    patch(c, rng, ["crimson_nylium", "netherrack", "crimson_nylium"], clear=9, round_=False)
    st = style("crimson", wall="crimson_planks", corner="crimson_stem", floor="crimson_planks", roof="crimson_stairs",
               gable="crimson_planks", cap="nether_wart_block", window="iron_bars", door="crimson_door", light="shroomlight",
               base="nether_bricks", porch_lamp=False)
    st["wd"] = dict(st["wd"], chair="oak_chair", table="oak_table", shelf="oak_shelf", drawer="oak_drawer", bench="oak_bench", window="iron_bars")
    cottage(c, 2, 2, 9, 8, st, S, rng, wh=3, things=["chest", "bed", "table", "shelf", "furnace"])
    for x in range(2, 10):
        c.set(x, 0, 11, "soul_sand")
        c.set(x, 1, 11, "nether_wart_stage3" if x % 3 else "nether_wart_stage2")
    c.set(1, 1, 10, "shroomlight")
    c.set(10, 1, 10, "shroomlight")
    return struct("cabana_carmesim", ["Crimson Forest"], c, 10, 0.4, loot=list(NETHER_LOOT), sink=1, foundation="netherrack",
                  mobs=[("nether_zombie", 1), ("crimson_boar", 1)])


@group("voyager")
def warped_tower():
    rng = random.Random(74)
    c = Canvas(11, 14, 11)
    patch(c, rng, ["warped_nylium", "netherrack", "warped_nylium"], clear=13)
    c.room(2, 0, 2, 8, 10, 8, "warped_planks", floor="warped_planks")
    for (x, z) in ((2, 2), (8, 2), (2, 8), (8, 8)):
        c.column(x, z, 0, 11, "warped_stem")
    c.box(5, 1, 8, 5, 2, 8, AIR)
    spiral_stairs(c, square_ring(3, 3, 7, 7), 1, 8, "crimson_stairs")
    c.box(3, 9, 3, 7, 9, 7, "warped_planks")
    c.set(3, 9, 3, AIR)
    c.set(4, 9, 3, AIR)
    for y in (4, 7):
        for (x, z) in ((5, 2), (2, 5), (8, 5)):
            c.set(x, y, z, "iron_bars")
    c.walls(1, 1, 9, 9, 11, 11, "warped_wart_block")
    c.box(2, 12, 2, 8, 12, 8, "warped_wart_block")
    c.box(4, 13, 4, 6, 13, 6, "warped_wart_block")
    c.set(5, 11, 5, "shroomlight")
    c.set(6, 10, 6, "chest")
    c.set(5, 1, 5, "shroomlight")
    return struct("torre_distorcida", ["Warped Forest"], c, 10, 0.45, loot=list(NETHER_LOOT) + [("ender_pearl", 1, 2, 0.3)], sink=1, foundation="netherrack",
                  mobs=[("ash_skeleton", 1), ("fire_imp", 1)])


@group("voyager")
def nether_camp():
    rng = random.Random(75)
    c = Canvas(15, 7, 15)
    patch(c, rng, ["netherrack", "soul_soil", "blackstone", "netherrack"], clear=6)
    tent(c, 1, 2, 5, "nether_wart_block", "z", pole="nether_brick_fence")
    tent(c, 9, 7, 5, "red_wool", "x", pole="nether_brick_fence")
    c.set(3, 1, 3, "chest")
    c.disc(7, 7, 1.5, 0, "magma_block")
    c.set(7, 1, 7, "brazier")
    for (x, z, f) in ((5, 7, E), (9, 7, W)):
        c.set(x, 1, z, var("crimson_stairs", f))
    for (x, z) in ((11, 2), (12, 2), (12, 3), (11, 3)):
        c.set(x, 1, z, rng.choice(["gold_block", "nether_gold_ore", "quartz_block"]))
    c.set(12, 2, 2, "gold_block")
    return struct("acampamento_do_nether", ["Nether Wastes", "Soul Sand Valley"], c, 10, 0.4, loot=list(NETHER_LOOT), sink=1, foundation="netherrack",
                  mobs=[("nether_zombie", 2), ("fire_imp", 1)])


# ---------------------------------------------------------------------------------------------
# Formations Nether idea: small formations on the Nether's cave floors.
# ---------------------------------------------------------------------------------------------

@group("formations")
def basalt_arch():
    rng = random.Random(81)
    c = Canvas(15, 11, 7)
    for x in range(15):
        t = abs(x - 7) / 7
        h = int(9 * (1 - t ** 2.2)) + 1
        for z in range(2, 5):
            for y in range(max(0, h - 2 - (1 if z != 3 else 0)), h + (1 if z == 3 else 0)):
                c.set(x, y, z, "basalt" if rng.random() > 0.15 else "blackstone")
    for x in (0, 1, 13, 14):
        c.box(x, 0, 1, x, 3, 5, "basalt")
    for (x, z) in ((2, 1), (12, 5), (1, 5), (13, 1)):
        c.set(x, 0, z, "magma_block")
    c.set(7, 0, 3, "magma_block")
    return struct("arco_de_basalto", ["Nether Wastes", "Soul Sand Valley", "Warped Forest"], c, 8, 0.4, foundation="basalt")


@group("formations")
def soul_shrine():
    rng = random.Random(82)
    c = Canvas(11, 10, 11)
    patch(c, rng, ["soul_soil", "soul_sand"], clear=9)
    c.disc(5, 5, 4.4, 0, "soul_soil")
    c.disc(5, 5, 4.4, 1, "blackstone_slab", hollow=True)
    c.box(4, 1, 4, 6, 1, 6, "blackstone")
    c.box(5, 2, 5, 5, 6, 5, "blackstone")
    c.set(5, 7, 5, "brazier")
    for (x, z) in ((5, 1), (5, 9), (1, 5), (9, 5)):
        c.column(x, z, 1, 2, "basalt")
        c.set(x, 3, z, "brazier")
    c.set(4, 2, 4, "quartz_block")
    c.set(6, 2, 6, "quartz_block")
    c.set(5, 2, 4, "chest")
    return struct("santuario_das_almas", ["Soul Sand Valley"], c, 8, 0.4, loot=[("bone", 2, 6, 0.7), ("quartz", 2, 6, 0.5), ("gold_nugget", 3, 9, 0.5),
                                                                                   ("mana_crystal", 1, 1, 0.15)], sink=1, foundation="soul_soil")


@group("formations")
def crimson_garden():
    rng = random.Random(83)
    c = Canvas(13, 11, 13)
    patch(c, rng, ["crimson_nylium", "crimson_nylium", "netherrack"], clear=10)
    for (x, z, h) in ((3, 3, 6), (9, 4, 8), (5, 9, 5)):
        c.column(x, z, 1, h, "crimson_stem")
        c.disc(x, z, 2.4, h, "nether_wart_block")
        c.disc(x, z, 1.4, h + 1, "nether_wart_block")
        c.set(x, h - 1, z + 1, "shroomlight")
        for (dx, dz) in ((2, 0), (-2, 0), (0, 2), (0, -2)):
            if rng.random() < 0.6:
                c.set(x + dx, h - 1, z + dz, "nether_wart_block")
    for (x, z) in ((8, 8), (9, 9), (10, 8), (8, 10), (2, 7)):
        c.set(x, 0, z, "soul_sand")
        c.set(x, 1, z, "nether_wart_stage3")
    return struct("jardim_carmesim", ["Crimson Forest"], c, 7, 0.45, sink=1, foundation="netherrack")


@group("formations")
def blackstone_ruins():
    rng = random.Random(84)
    c = Canvas(13, 8, 13)
    patch(c, rng, ["blackstone", "netherrack", "blackstone", "basalt"], clear=7, round_=False)
    c.walls(1, 1, 11, 11, 1, 5, "blackstone")
    c.box(5, 1, 11, 7, 3, 11, AIR)
    for (x, z) in ((1, 1), (11, 1), (1, 11), (11, 11)):
        c.column(x, z, 1, 6, "nether_bricks")
    c.walls(1, 1, 11, 11, 3, 3, "nether_gold_ore")
    c.set(6, 1, 3, "chest")
    c.set(5, 1, 3, "blackstone_stairs:2")
    c.set(7, 1, 3, "blackstone_stairs:2")
    for (x, y, z, r) in ((11, 5, 5, 2.4), (3, 5, 1, 2.2), (1, 4, 9, 2.0), (8, 6, 11, 1.8)):
        bite(c, x, y, z, r)
    remove_floaters(c)
    c.weather(rng, {"blackstone": ["blackstone", "basalt", "netherrack"]}, 0.15)
    return struct("ruinas_de_pedra_negra", NETHER_ALL, c, 9, 0.4, loot=list(NETHER_LOOT), sink=1, foundation="blackstone", mobs=[("ash_skeleton", 1)])


# ---------------------------------------------------------------------------------------------
# Moog's End Structures idea: finds on the End's outer islands.
# ---------------------------------------------------------------------------------------------

END_LOOT = [("ender_pearl", 1, 4, 0.7), ("chorus_fruit", 2, 6, 0.6), ("popped_chorus_fruit", 1, 4, 0.4), ("diamond", 1, 3, 0.35),
            ("emerald", 2, 5, 0.4), ("gold_ingot", 2, 6, 0.5), ("ender_eye", 1, 2, 0.25), ("amethyst", 1, 3, 0.3),
            ("diamond_sword", 1, 1, 0.08), ("mana_crystal", 1, 2, 0.3)]
EB, PP = "end_stone_bricks", "purpur_block"


@group("end")
def purpur_tower():
    rng = random.Random(91)
    c = Canvas(11, 28, 11)
    patch(c, rng, ["end_stone", "end_stone_bricks", "end_stone"], clear=26)
    c.box(1, 0, 1, 9, 0, 9, EB)
    c.room(2, 1, 2, 8, 22, 8, PP)
    for (x, z) in ((2, 2), (8, 2), (2, 8), (8, 8)):
        c.column(x, z, 1, 23, EB)
    for y in range(4, 21, 4):
        for (x, z) in ((5, 2), (5, 8), (2, 5), (8, 5)):
            c.set(x, y, z, "glass_pane")
            c.set(x, y + 1, z, "glass_pane")
    c.box(5, 1, 8, 5, 2, 8, AIR)
    spiral_stairs(c, square_ring(3, 3, 7, 7), 1, 20, "purpur_stairs")
    c.column(5, 5, 1, 21, PP)
    c.box(1, 21, 1, 9, 21, 9, "purpur_slab:1")
    ring = square_ring(3, 3, 7, 7)
    for k in range(3):
        x, z = ring[(19 - k) % len(ring)]
        c.set(x, 21, z, AIR)
    c.walls(1, 1, 9, 9, 22, 22, "purpur_slab")
    for (x, z) in ((1, 1), (9, 1), (1, 9), (9, 9)):
        c.column(x, z, 22, 24, PP)
        c.set(x, 25, z, "purpur_slab")
    c.set(5, 22, 5, "chest")
    c.set(4, 22, 4, "end_crystal")
    return struct("torre_de_purpura", ["The End"], c, 5, 0.6, loot=list(END_LOOT), sink=1, foundation="end_stone", rough=5, mobs=[("void_walker", 2)])


@group("end")
def end_temple_ruin():
    rng = random.Random(92)
    c = Canvas(15, 12, 15)
    patch(c, rng, ["end_stone", "end_stone"], clear=11)
    c.box(1, 0, 1, 13, 0, 13, EB)
    c.box(2, 1, 2, 12, 1, 12, EB)
    for (x, z) in square_ring(1, 1, 13, 13):
        c.set(x, 1, z, stair("end_stone_brick_stairs", opp(outward(x - 7, z - 7))))
    for (x, z) in square_ring(3, 3, 11, 11)[::2]:
        c.column(x, z, 2, rng.randint(3, 8), PP)
    c.box(6, 2, 6, 8, 2, 8, "obsidian")
    c.set(7, 3, 7, "end_crystal")
    c.set(7, 2, 5, "chest")
    for (x, z) in ((6, 5), (8, 5)):
        c.set(x, 2, z, "purpur_slab")
    c.box(3, 9, 3, 11, 9, 4, "purpur_slab")
    c.box(3, 9, 10, 11, 9, 11, "purpur_slab")
    for (x, y, z, r) in ((11, 9, 4, 3.0), (3, 9, 11, 2.5), (11, 6, 11, 2.0)):
        bite(c, x, y, z, r, floor=3)
    remove_floaters(c, y0=3)
    return struct("templo_do_end_em_ruinas", ["The End"], c, 5, 0.6, loot=list(END_LOOT), sink=1, foundation="end_stone", rough=5, mobs=[("void_walker", 2)])


@group("end")
def chorus_garden():
    rng = random.Random(93)
    c = Canvas(15, 10, 15)
    patch(c, rng, ["end_stone"], clear=9)
    c.walls(1, 1, 13, 13, 1, 1, EB)
    c.walls(1, 1, 13, 13, 2, 2, "end_stone_brick_slab")
    c.box(6, 1, 13, 8, 2, 13, AIR)
    for (x, z) in ((1, 1), (13, 1), (1, 13), (13, 13)):
        c.column(x, z, 1, 3, PP)
        c.set(x, 4, z, "end_crystal")
    c.box(6, 0, 6, 8, 0, 8, "water")
    c.walls(5, 5, 9, 9, 0, 1, "purpur_slab")
    c.walls(5, 5, 9, 9, 0, 0, PP)
    for (x, z) in ((3, 3), (11, 3), (3, 10), (11, 10), (7, 3), (3, 7), (11, 7)):
        h = rng.randint(2, 5)
        c.column(x, z, 1, h, "chorus_plant")
        c.set(x, h + 1, z, "chorus_flower")
        if rng.random() < 0.6:
            dx = rng.choice([-1, 1])
            c.set(x + dx, h, z, "chorus_plant")
            c.set(x + dx, h + 1, z, "chorus_flower")
    for (x, z, f) in ((7, 11, N), (4, 7, E)):
        c.set(x, 1, z, var("purpur_stairs", f))
    c.set(10, 1, 12, "chest")
    return struct("jardim_de_coros", ["The End"], c, 5, 0.6, loot=list(END_LOOT) + [("chorus_flower", 1, 3, 0.5)], sink=1, foundation="end_stone", rough=5)


@group("end")
def floating_library():
    rng = random.Random(94)
    c = Canvas(13, 20, 13)
    patch(c, rng, ["end_stone", "end_stone_bricks"], clear=19)
    for (x, z) in ((2, 2), (10, 2), (2, 10), (10, 10)):
        c.column(x, z, 1, 6, "obsidian")
    c.box(1, 7, 1, 11, 7, 11, PP)
    c.room(2, 8, 2, 10, 13, 10, EB, ceil=PP)
    for (x, z) in ((2, 2), (10, 2), (2, 10), (10, 10)):
        c.column(x, z, 8, 13, PP)
    for x in range(3, 10):
        for y in (8, 9, 10):
            c.set(x, y, 3, "bookshelf")
            c.set(x, y, 9, "bookshelf")
    for z in (5, 7):
        c.set(9, 8, z, "bookshelf")
        c.set(9, 9, z, "bookshelf")
    for (x, z) in ((6, 2), (6, 10), (2, 6), (10, 6)):
        c.box(x, 10, z, x, 12, z, "glass_pane")
    c.set(6, 8, 6, "purpur_slab:2")
    c.set(6, 9, 6, "lantern")
    c.set(5, 8, 6, var("oak_chair", E))
    c.set(7, 8, 7, "chest")
    c.set(6, 12, 6, HANG)
    c.box(2, 8, 6, 2, 9, 6, AIR)
    # Stairs up to the door.
    for k in range(7):
        c.set(0, 1 + k, 12 - k, stair("purpur_stairs", N))
        c.box(0, 2 + k, 12 - k, 0, 4 + k, 12 - k, AIR)
    c.box(0, 7, 6, 1, 7, 6, PP)
    c.set(1, 8, 6, AIR)
    hip(c, 2, 2, 10, 10, 14, "purpur_stairs", "end_crystal", over=1)
    return struct("biblioteca_flutuante", ["The End"], c, 6, 0.6, loot=list(END_LOOT) + [("flame_tome", 1, 1, 0.12), ("thunder_tome", 1, 1, 0.12), ("bookshelf", 1, 3, 0.4)],
                  sink=1, foundation="end_stone", rough=5, mobs=[("void_walker", 1)])


@group("end")
def end_mine():
    rng = random.Random(95)
    c = Canvas(13, 18, 13)
    # Galleries in the island under a headframe.
    sink = 8
    c.room(1, 0, 4, 11, 4, 8, "end_stone", floor="end_stone", ceil="end_stone")
    c.room(4, 0, 1, 8, 4, 11, "end_stone", floor="end_stone", ceil="end_stone")
    c.box(2, 1, 5, 10, 3, 7, AIR)
    c.box(5, 1, 2, 7, 3, 10, AIR)
    for (x, z) in ((2, 5), (2, 7), (10, 5), (10, 7), (5, 2), (7, 2), (5, 10), (7, 10)):
        c.column(x, z, 1, 3, "obsidian")
    for x in range(2, 11):
        c.set(x, 1, 6, "rail:1")
    for (x, z) in ((1, 5), (1, 6), (11, 7), (6, 0), (5, 12), (11, 5)):
        c.set(x, rng.randint(1, 3), z, rng.choice(["amethyst_ore", "diamond_ore", "obsidian", "purpur_block"]))
    c.set(9, 1, 7, "chest")
    c.set(6, 3, 6, HANG)
    c.set(3, 1, 5, "end_crystal")
    # Shaft up to the surface.
    for y in range(4, sink + 1):
        c.walls(5, 5, 7, 7, y, y, EB)
        c.set(6, y, 6, AIR)
    c.box(6, 1, 6, 6, sink, 6, AIR)
    c.box(5, sink - 1, 5, 7, sink - 1, 7, EB)
    c.set(6, sink - 1, 6, AIR)
    # Headframe.
    for (x, z) in ((4, 4), (8, 4), (4, 8), (8, 8)):
        c.column(x, z, sink, sink + 6, PP)
    c.box(4, sink + 7, 4, 8, sink + 7, 8, "purpur_slab")
    c.set(6, sink + 6, 6, "chain")
    c.set(6, sink + 5, 6, "chain")
    c.set(6, sink + 4, 6, HANG)
    c.walls(5, 5, 7, 7, sink, sink, "purpur_slab")
    c.set(6, sink, 7, AIR)
    for (x, z) in ((2, 10), (10, 2)):
        c.set(x, sink, z, rng.choice(["end_stone", "obsidian"]))
    return struct("mina_do_end", ["The End"], c, 6, 0.6, loot=list(END_LOOT) + [("diamond_pickaxe", 1, 1, 0.08), ("obsidian", 2, 6, 0.4)],
                  sink=sink + 1, foundation="end_stone", rough=5, mobs=[("void_walker", 1)])


@group("end")
def void_shrine():
    rng = random.Random(96)
    c = Canvas(11, 9, 11)
    patch(c, rng, ["end_stone", "obsidian", "end_stone", "end_stone"], clear=8)
    c.disc(5, 5, 4.4, 0, "obsidian")
    c.disc(5, 5, 3.4, 0, EB)
    for (x, z) in circle_ring(5, 5, 4)[::2]:
        c.set(x, 1, z, "end_portal_frame")
    for (x, z) in ((1, 1), (9, 1), (1, 9), (9, 9)):
        c.column(x, z, 1, 5, "obsidian")
        c.set(x, 6, z, "end_crystal")
    c.set(5, 1, 5, "purpur_block")
    c.set(5, 2, 5, "chest")
    return struct("santuario_do_vazio", ["The End"], c, 5, 0.6, loot=list(END_LOOT), sink=1, foundation="end_stone", rough=5)


@group("end")
def end_camp():
    rng = random.Random(97)
    c = Canvas(13, 7, 13)
    patch(c, rng, ["end_stone"], clear=6)
    tent(c, 1, 2, 4, "purple_wool", "z", pole="end_stone_brick_slab")
    tent(c, 7, 7, 4, "magenta_wool", "x", pole="end_stone_brick_slab")
    c.set(3, 1, 3, "chest")
    c.set(3, 1, 4, "red_bed")
    c.set(9, 1, 3, "brazier")
    for (x, z) in ((8, 3), (10, 3)):
        c.set(x, 1, z, "purpur_slab")
    c.column(11, 11, 1, 3, "chorus_plant")
    c.set(11, 4, 11, "chorus_flower")
    return struct("acampamento_do_end", ["The End"], c, 4, 0.6, loot=list(END_LOOT), sink=1, foundation="end_stone", rough=5, mobs=[("void_walker", 1)])


# ---------------------------------------------------------------------------------------------
# YUNG's Better Dungeons idea: big dungeons buried under the ground.
# ---------------------------------------------------------------------------------------------

DUNGEON_LOOT = [("bone", 2, 8, 0.7), ("gold_ingot", 2, 6, 0.6), ("iron_ingot", 3, 8, 0.6), ("diamond", 1, 3, 0.35),
                ("emerald", 2, 5, 0.45), ("life_crystal", 1, 1, 0.25), ("mana_crystal", 1, 2, 0.3), ("ender_pearl", 1, 2, 0.25),
                ("iron_sword", 1, 1, 0.25), ("diamond_helmet", 1, 1, 0.06), ("redstone", 3, 9, 0.4), ("iron_backpack", 1, 1, 0.08)]


def shaft(c, x0, z0, y0, y1, wall, stairs, start=0):
    """A 5 by 5 stair well with a spiral round a centre post, from layer y0 up to layer y1."""
    for y in range(y0, y1 + 1):
        c.walls(x0, z0, x0 + 4, z0 + 4, y, y, wall)
        c.box(x0 + 1, y, z0 + 1, x0 + 3, y, z0 + 3, AIR)
        c.set(x0 + 2, y, z0 + 2, wall)
    ring = square_ring(x0 + 1, z0 + 1, x0 + 3, z0 + 3)
    end = spiral_stairs(c, ring, y0, y1 - 1, stairs, start)
    return ring, end


def niche_wall(c, rng, cells, y, fill):
    """Burial niches cut into a corridor wall: (x, z) of the wall cells to hollow."""
    for (x, z) in cells:
        r = rng.random()
        c.set(x, y, z, AIR)
        c.set(x, y + 1, z, AIR)
        if r < 0.55:
            c.set(x, y, z, rng.choice(fill))
        elif r < 0.62:
            c.set(x, y, z, "lantern")


@group("yung")
def catacombs():
    """Catacombs: a grid of vaulted corridors lined with burial niches, ossuary rooms at the
    crossings, a pillared crypt hall in the middle and a stair well up to a small mausoleum."""
    rng = random.Random(101)
    Sz = 41
    sink = 14
    c = Canvas(Sz, sink + 8, Sz)
    SB, MB, CB = "stone_bricks", "mossy_cobblestone_bricks", "cobblestone_bricks"
    bands = [(8, 10), (19, 21), (30, 32)]
    # Corridors: solid shells, then hollow.
    for (a0, a1) in bands:
        c.box(1, 0, a0 - 1, Sz - 2, 5, a1 + 1, SB)
        c.box(a0 - 1, 0, 1, a1 + 1, 5, Sz - 2, SB)
    for (a0, a1) in bands:
        c.box(2, 1, a0, Sz - 3, 3, a1, AIR)
        c.box(a0, 1, 2, a1, 3, Sz - 3, AIR)
        c.box(3, 4, a0 + 1, Sz - 4, 4, a1 - 1, AIR)
        c.box(a0 + 1, 4, 3, a1 - 1, 4, Sz - 4, AIR)
    # Lanterns hanging along the middle of every corridor.
    for (a0, a1) in bands:
        for k in range(4, Sz - 4, 6):
            c.set(k, 4, a0 + 1, HANG)
            c.set(a0 + 1, 4, k, HANG)
    # Niches all along the corridor walls.
    bones = ["quartz_slab", "quartz_slab", "quartz_stairs:1", "brown_mushroom", "cobblestone_slab"]
    for (a0, a1) in bands:
        for k in range(3, Sz - 3, 2):
            if any(b0 - 1 <= k <= b1 + 1 for (b0, b1) in bands):
                continue
            niche_wall(c, rng, [(k, a0 - 1), (k, a1 + 1)], 1, bones)
            niche_wall(c, rng, [(a0 - 1, k), (a1 + 1, k)], 1, bones)
    # Ossuary rooms at the outer crossings.
    for cx in (9, 31):
        for cz in (9, 31):
            c.room(cx - 4, 0, cz - 4, cx + 4, 6, cz + 4, MB, floor=SB, ceil=SB)
            c.box(cx - 3, 1, cz - 3, cx + 3, 5, cz + 3, AIR)
            for (x, z) in ((cx - 4, cz), (cx + 4, cz), (cx, cz - 4), (cx, cz + 4)):
                c.box(x - 1 if x == cx else x, 1, z - 1 if z == cz else z, x + 1 if x == cx else x, 3, z + 1 if z == cz else z, AIR)
            for (x, z) in ((cx - 3, cz - 3), (cx + 3, cz - 3), (cx - 3, cz + 3), (cx + 3, cz + 3)):
                c.column(x, z, 1, 5, "quartz_block" if rng.random() < 0.5 else CB)
                c.set(x, 1, z, "brazier")
            c.set(cx, 5, cz, HANG)
    c.set(8, 1, 6, "chest")
    c.set(33, 1, 34, "chest")
    # The crypt hall in the middle.
    h0, h1 = 13, 27
    c.room(h0, 0, h0, h1, 7, h1, SB, floor="smooth_stone", ceil=SB)
    for (x, z) in ((20, h0), (20, h1), (h0, 20), (h1, 20)):
        c.box(x - 1, 1, z, x + 1, 3, z, AIR)
    for x in range(h0 + 2, h1 - 1, 4):
        for z in (h0 + 2, h1 - 2):
            c.column(x, z, 1, 6, "quartz_block")
            c.column(z, x, 1, 6, "quartz_block")
    for (x, z) in ((16, 17), (16, 23), (24, 17), (24, 23)):
        c.box(x, 1, z - 1, x, 1, z + 1, "stone_brick_slab:2")
        c.box(x, 2, z - 1, x, 2, z + 1, "stone_brick_slab")
    c.box(18, 1, 18, 22, 1, 22, "smooth_stone")
    c.box(19, 2, 19, 21, 2, 21, "stone_brick_slab")
    c.set(20, 2, 20, "chest")
    c.set(19, 2, 20, "gold_block")
    c.set(21, 2, 20, "gold_block")
    for (x, z) in ((18, 18), (22, 18), (18, 22), (22, 22)):
        c.set(x, 2, z, "brazier")
    for (x, z) in ((17, 20), (23, 20), (20, 17), (20, 23)):
        c.set(x, 6, z, "chain")
        c.set(x, 5, z, "chandelier")
    # The mausoleum on the surface, over a stair well down in the south-east corner.
    sx, sz = 34, 34
    m0, m1 = sx - 1, sx + 5
    c.box(m0, sink - 1, m0, m1, sink - 1, m1, SB)
    c.room(m0, sink, m0, m1, sink + 3, m1, SB)
    shaft(c, sx, sz, 1, sink, SB, "stone_brick_stairs")
    c.walls(sx, sz, sx + 4, sz + 4, sink, sink, AIR)
    c.column(sx + 2, sz + 2, sink, sink + 3, SB)
    c.box(sx, 1, sz + 2, sx, 2, sz + 2, AIR)
    c.box(sx - 1, 1, sz + 2, sx - 1, 3, sz + 2, AIR)
    c.set(sx - 1, 0, sz + 2, SB)
    c.box(sx + 2, sink, m0, sx + 2, sink + 1, m0, AIR)
    c.set(sx + 2, sink, m0, door("spruce_door", N)[0])
    c.set(sx + 2, sink + 1, m0, door("spruce_door", N)[1])
    gable(c, m0, m0, m1, m1, sink + 4, "stone_brick_stairs", SB, "stone_brick_slab", axis="z")
    c.set(sx + 1, sink + 3, sz + 1, HANG)
    for (x, z) in ((m0 - 1, m0 - 1), (m1 + 1, m0 - 1)):
        c.column(x, z, sink, sink + 1, "cobblestone_wall")
        c.set(x, sink + 2, z, "lantern")
    c.weather(rng, {SB: [MB, CB, "mossy_cobblestone", SB]}, 0.25)
    return struct("catacumbas", ["Plains", "Forest", "Birch Forest", "Taiga", "Dark Forest", "Savanna", "Snowy Plains", "Mountains"], c, 48, 0.5,
                  loot=list(DUNGEON_LOOT), sink=sink, rough=6,
                  mobs=[("skeleton", 6), ("zombie", 4), ("spider", 2), ("mutant_skeleton", 1)])


@group("yung")
def skeleton_dungeon():
    """A skeleton dungeon: a caged pit with a glowing brazier where the dead rise (the spawner
    room) in a ring hall, prison cells, a guard room, a treasure vault and a long stair down."""
    rng = random.Random(102)
    Wd, Dp = 33, 40
    sink = 11
    c = Canvas(Wd, sink + 5, Dp)
    SB, MB, CB = "stone_bricks", "mossy_cobblestone_bricks", "cobblestone"
    # The whole level, then rooms carved in it.
    c.box(0, 0, 0, Wd - 1, 6, 30, SB)
    c.box(2, 1, 2, Wd - 3, 4, 28, AIR)
    # Inner block holding the spawner room, with a ring hall round it.
    c.box(9, 1, 9, 23, 5, 21, SB)
    c.box(11, 1, 11, 21, 5, 19, AIR)
    c.box(16, 1, 9, 16, 3, 9, AIR)
    c.box(16, 1, 21, 16, 3, 21, AIR)
    c.box(14, 0, 13, 18, 0, 17, "magma_block")
    c.box(15, 0, 14, 17, 0, 16, "soul_sand")
    c.walls(14, 13, 18, 17, 1, 3, "iron_bars")
    c.set(16, 1, 15, "magma_block")
    c.set(16, 2, 15, "brazier")
    c.box(14, 4, 13, 18, 4, 17, "iron_bars")
    for (x, z) in ((12, 12), (20, 12), (12, 18), (20, 18)):
        c.set(x, 1, z, "brazier")
        c.set(x, 5, z, "chain")
        c.set(x, 4, z, HANG)
    # Cells along the north wall.
    for k, x0 in enumerate(range(3, 29, 5)):
        c.walls(x0, 2, x0 + 4, 6, 1, 4, SB)
        c.box(x0 + 1, 1, 3, x0 + 3, 4, 5, AIR)
        c.box(x0 + 1, 1, 6, x0 + 3, 3, 6, "iron_bars")
        c.set(x0 + 2, 1, 3, rng.choice(["red_bed", "hay_block", "quartz_slab"]))
        c.set(x0 + 1, 1, 5, rng.choice(["quartz_slab", AIR, "brown_mushroom"]))
    c.box(3, 1, 7, 28, 4, 8, AIR)
    # Guard room in the west.
    c.box(2, 1, 9, 7, 4, 21, AIR)
    c.walls(1, 9, 8, 22, 1, 4, SB)
    c.box(8, 1, 14, 8, 3, 15, AIR)
    for z in range(11, 20, 3):
        c.set(4, 1, z, "spruce_table")
        c.set(3, 1, z, var("spruce_chair", E))
        c.set(4, 2, z, "lantern")
    for z in range(10, 21, 2):
        c.set(2, 1, z, "spruce_drawer" if z % 4 else "iron_block")
        c.set(2, 2, z, "spruce_fence")
    c.set(6, 1, 20, "red_bed")
    c.set(6, 1, 10, "red_bed")
    # Treasure vault in the east, behind a barred door.
    c.walls(24, 9, 31, 22, 1, 4, SB)
    c.box(25, 1, 10, 30, 4, 21, AIR)
    c.box(24, 1, 15, 24, 3, 15, "iron_bars")
    c.set(24, 1, 15, AIR)
    c.set(24, 2, 15, AIR)
    for (x, z) in ((29, 11), (29, 20), (27, 15)):
        c.set(x, 1, z, "chest")
    for (x, z) in ((30, 11), (30, 12), (30, 20), (28, 15)):
        c.set(x, 1, z, "gold_block")
    c.set(27, 4, 15, HANG)
    # South hall and the stair up.
    c.box(3, 1, 23, 28, 4, 28, AIR)
    for x in range(4, 28, 4):
        c.column(x, 23, 1, 4, "smooth_stone")
        c.column(x, 28, 1, 4, "smooth_stone")
    for x in (8, 24):
        c.set(x, 4, 25, HANG)
    for k in range(sink - 1):
        y = 1 + k
        z = 29 + k
        for x in (15, 16, 17):
            c.set(x, y, z, stair("stone_brick_stairs", S))
            c.box(x, y + 1, z, x, y + 3, z, AIR)
            c.set(x, y - 1, z, SB)
        for x in (14, 18):
            c.box(x, y - 1, z, x, y + 3, z, SB)
        for x in (15, 16, 17):
            c.set(x, y + 4, z, SB)
    c.box(15, 1, 29, 17, 4, 29, AIR)
    c.box(15, 1, 29, 17, 1, 29, "stone_brick_stairs:2")
    # A ruined arch marks the top.
    top = sink
    zt = 29 + sink - 1
    for x in (14, 18):
        c.column(x, zt, top - 1, top + 3, MB)
    c.box(14, top + 3, zt, 18, top + 3, zt, "stone_brick_slab")
    c.set(16, top + 3, zt, MB)
    c.weather(rng, {SB: [MB, "cobblestone_bricks", CB, SB, SB]}, 0.25)
    return struct("masmorra_dos_esqueletos", ["Plains", "Forest", "Taiga", "Dark Forest", "Mountains", "Birch Forest", "Savanna", "Desert", "Badlands"],
                  c, 44, 0.5, loot=list(DUNGEON_LOOT) + [("bow", 1, 1, 0.4), ("arrow", 8, 20, 0.6)], sink=sink, rough=6,
                  mobs=[("skeleton", 7), ("mutant_skeleton", 1), ("zombie", 2)])


@group("yung")
def buried_fortress():
    """A fortress buried whole under the ground: walls and round towers in a great dark hollow,
    a two-floor keep, barracks and a chapel, lit by lanterns hanging from the rock, with a
    stair well up through one tower to a broken turret on the surface."""
    rng = random.Random(103)
    Sz = 46
    sink = 19
    c = Canvas(Sz, sink + 7, Sz)
    SB, MB, CB = "stone_bricks", "mossy_cobblestone_bricks", "cobblestone_bricks"
    # The hollow, with a floor and a rock ceiling.
    c.box(0, 0, 0, Sz - 1, 0, Sz - 1, "cobblestone")
    c.box(1, 1, 1, Sz - 2, 14, Sz - 2, AIR)
    for z in range(1, Sz - 1):
        for x in range(1, Sz - 1):
            c.set(x, 0, z, rng.choice(["gravel", "cobblestone", "stone", "rocky_dirt"]))
    # Curtain walls and towers.
    c.walls(4, 4, Sz - 5, Sz - 5, 1, 9, SB)
    for (x, z) in square_ring(4, 4, Sz - 5, Sz - 5)[::2]:
        c.set(x, 10, z, SB)
    for (x, z) in ((5, 5), (Sz - 6, 5), (5, Sz - 6), (Sz - 6, Sz - 6)):
        round_tower(c, x, z, 3.6, 1, 12, SB, floor="cobblestone", crenel=SB, windows="iron_bars", every=4)
    c.box(21, 1, Sz - 5, 24, 5, Sz - 5, AIR)
    for x in range(21, 25):
        c.set(x, 5, Sz - 5, "iron_bars")
    # Keep.
    k0, k1 = 16, 29
    for y0 in (0, 6):
        c.room(k0, y0, k0, k1, y0 + 6, k1, SB, floor="spruce_planks" if y0 else "stone_bricks", ceil=SB)
    for (x, z) in square_ring(k0, k0, k1, k1)[::2]:
        c.set(x, 13, z, SB)
    c.box(22, 1, k1, 23, 3, k1, AIR)
    for y in (3, 9):
        for x in range(k0 + 2, k1 - 1, 3):
            c.set(x, y, k0, "iron_bars")
            c.set(x, y, k1, "iron_bars")
    stair_run(c, k0 + 1, 1, k1 - 2, N, 6, "stone_brick_stairs")
    c.box(k0 + 1, 6, k1 - 7, k0 + 1, 6, k1 - 3, AIR)
    for z in range(k0 + 3, k1 - 2):
        c.set(22, 1, z, "spruce_table")
        c.set(23, 1, z, "spruce_table")
        c.set(21, 1, z, var("spruce_chair", E))
        c.set(24, 1, z, var("spruce_chair", W))
    c.set(22, 2, k0 + 5, "lantern")
    c.set(23, 2, k0 + 9, "lantern")
    c.set(22, 5, 22, "chain")
    c.set(22, 4, 22, "chandelier")
    c.set(k1 - 2, 7, k0 + 2, "chest")
    for x in range(k0 + 2, k1 - 1, 2):
        c.set(x, 7, k1 - 1, "red_bed")
    for z in range(k0 + 2, k1 - 1):
        c.set(k1 - 1, 7, z, "bookshelf")
        c.set(k1 - 1, 8, z, "bookshelf")
    c.set(22, 11, 22, HANG)
    # Barracks along the west wall and a chapel along the east.
    building(c, 6, 14, 12, 30, 1, 4, "spruce_planks", "spruce_log", "spruce_roof", "spruce_planks", "spruce_slab", "glass_pane", "spruce_planks",
             doors=(E,), door_name="spruce_door", axis="z")
    for z in range(15, 30, 2):
        c.set(7, 1, z, "red_bed")
    c.set(11, 1, 16, "chest")
    building(c, 33, 14, 39, 30, 1, 5, SB, "smooth_stone", "stone_brick_roof", SB, "stone_brick_slab", "glass_pane", "smooth_stone",
             doors=(W,), door_name="spruce_door", axis="z")
    for z in range(17, 28, 2):
        c.set(35, 1, z, var("spruce_bench", W))
        c.set(37, 1, z, var("spruce_bench", W))
    c.set(36, 1, 15, "gold_block")
    c.set(36, 2, 15, "lantern")
    # Lanterns hang from the rock all over the hollow.
    for z in range(3, Sz - 3, 6):
        for x in range(3, Sz - 3, 6):
            if c.get(x, 14, z) == AIR:
                c.set(x, 14, z, "chain")
                c.set(x, 13, z, HANG)
    # Stair well up through the south-east tower, a broken turret on top.
    tx, tz = Sz - 8, Sz - 8
    shaft(c, tx, tz, 1, sink, SB, "stone_brick_stairs")
    c.walls(tx, tz, tx + 4, tz + 4, sink, sink, AIR)
    c.box(tx, 1, tz + 2, tx, 2, tz + 2, AIR)
    for y in range(sink, sink + 4):
        for (x, z) in square_ring(tx - 1, tz - 1, tx + 5, tz + 5):
            if rng.random() < 0.8 - (y - sink) * 0.2:
                c.set(x, y, z, MB)
    c.box(tx + 1, sink, tz - 1, tx + 3, sink + 1, tz - 1, AIR)
    c.weather(rng, {SB: [MB, CB, "cobblestone", SB, SB]}, 0.25)
    overgrow(c, rng, 0.02, plant="cave_vines", y0=5)
    return struct("fortaleza_enterrada", ["Plains", "Forest", "Taiga", "Mountains", "Dark Forest", "Snowy Plains", "Savanna"], c, 60, 0.5,
                  loot=list(DUNGEON_LOOT) + [("diamond_sword", 1, 1, 0.1), ("gold_backpack", 1, 1, 0.06)], sink=sink, rough=6,
                  mobs=[("skeleton", 5), ("zombie", 4), ("marauder", 2), ("brute", 1), ("invoker", 1)])


@group("yung")
def abandoned_mine():
    """An abandoned mine: timbered tunnels with rails crossing under the ground, ore veins in
    the walls, a cavern with a wooden scaffold and a lift shaft up to a shed on the surface."""
    rng = random.Random(104)
    Sz = 48
    sink = 13
    c = Canvas(Sz, sink + 7, Sz)
    tunnels = [("x", 8), ("x", 24), ("x", 40), ("z", 10), ("z", 34)]
    for (axis, k) in tunnels:
        for t in range(2, Sz - 2):
            x, z = (t, k) if axis == "x" else (k, t)
            for d in (-1, 0, 1):
                xx, zz = (x, z + d) if axis == "x" else (x + d, z)
                c.set(xx, 0, zz, rng.choice(["gravel", "stone", "rocky_dirt"]))
                c.box(xx, 1, zz, xx, 3, zz, AIR)
            c.set(x, 1, z, "rail:1" if axis == "x" else "rail")
            if t % 4 == 0:
                for d in (-1, 1):
                    xx, zz = (x, z + d) if axis == "x" else (x + d, z)
                    c.column(xx, zz, 1, 3, "oak_log")
                for d in (-1, 0, 1):
                    xx, zz = (x, z + d) if axis == "x" else (x + d, z)
                    c.set(xx, 4, zz, "oak_planks")
                if t % 8 == 0:
                    c.set(x, 3, z, HANG)
            # Ores in the walls.
            if rng.random() < 0.08:
                for d in (-2, 2):
                    xx, zz = (x, z + d) if axis == "x" else (x + d, z)
                    c.set(xx, rng.randint(1, 3), zz, rng.choice(["iron_ore", "coal_ore", "copper_ore", "gold_ore", "tin_ore", "redstone_ore", "lapis_ore"]))
    # Crossings get proper crossings, not crossed rails.
    for (a, ka) in tunnels:
        for (b, kb) in tunnels:
            if a == "x" and b == "z":
                c.set(kb, 1, ka, AIR)
    # Cavern with a scaffold and minecart chests.
    for y in range(1, 9):
        r = 7.5 - abs(y - 4) * 0.6
        c.disc(24, 24, r, y, AIR)
    c.disc(24, 24, 7.5, 0, "stone")
    for (x, z) in ((21, 21), (27, 21), (21, 27), (27, 27)):
        c.column(x, z, 1, 6, "oak_log")
    c.box(21, 5, 21, 27, 5, 27, "oak_planks")
    c.box(22, 5, 22, 26, 5, 26, AIR)
    c.walls(21, 21, 27, 27, 6, 6, "oak_fence")
    stair_run(c, 20, 1, 26, N, 4, "oak_stairs", clear=2)
    c.set(20, 5, 22, "oak_planks")
    c.set(22, 6, 22, "chest")
    c.set(26, 6, 26, "lantern")
    c.set(24, 7, 24, HANG)
    c.set(5, 1, 8, "chest")
    c.set(43, 1, 34, "chest")
    for (x, z) in ((18, 24), (30, 24), (24, 18)):
        c.set(x, 1, z, rng.choice(["iron_ore", "gold_ore", "diamond_ore", "coal_ore"]))
    # Collapsed spots.
    for (x, z) in ((14, 8), (34, 40), (10, 30)):
        for d in range(-1, 2):
            c.set(x + d, 1, z, "gravel")
            c.set(x, 2, z + d, "gravel")
    # Lift shaft to the surface and a shed over it.
    lx, lz = 40, 8
    c.box(lx - 1, 1, lz - 1, lx + 1, sink, lz + 1, AIR)
    for y in range(1, sink):
        c.walls(lx - 2, lz - 2, lx + 2, lz + 2, y, y, "oak_planks" if y % 4 == 0 else None)
    for (x, z) in ((lx - 2, lz - 2), (lx + 2, lz - 2), (lx - 2, lz + 2), (lx + 2, lz + 2)):
        c.column(x, z, 1, sink - 1, "oak_log")
    for k in range(sink - 1):
        x, z = square_ring(lx - 1, lz - 1, lx + 1, lz + 1)[k % 8]
        nx, nz = square_ring(lx - 1, lz - 1, lx + 1, lz + 1)[(k + 1) % 8]
        c.set(x, 1 + k, z, stair("oak_stairs", outward(nx - x, nz - z)))
    c.set(lx, 1, lz, "oak_planks")
    for y in range(2, sink + 4):
        c.set(lx, y, lz, "chain")
    building(c, lx - 3, lz - 3, lx + 3, lz + 3, sink, 3, "oak_planks", "oak_log", "spruce_roof", "oak_planks", "oak_slab", "glass_pane", "oak_planks",
             doors=(W,), door_name="oak_door")
    c.box(lx - 1, sink - 1, lz - 1, lx + 1, sink - 1, lz + 1, AIR)
    k = sink - 1
    x, z = square_ring(lx - 1, lz - 1, lx + 1, lz + 1)[k % 8]
    c.set(x, sink - 1, z, "oak_planks")
    c.set(lx, sink - 1, lz, AIR)
    c.set(lx + 2, sink, lz + 2, "crafting_table")
    c.set(lx + 2, sink, lz - 2, "chest")
    return struct("mina_abandonada", ["Mountains", "Badlands", "Taiga", "Plains", "Savanna", "Snowy Plains"], c, 48, 0.5,
                  loot=list(DUNGEON_LOOT) + [("rail", 6, 16, 0.5), ("minecart", 1, 1, 0.3), ("raw_gold", 2, 6, 0.5), ("tnt", 1, 4, 0.3), ("iron_pickaxe", 1, 1, 0.3)],
                  sink=sink, rough=6, mobs=[("zombie", 3), ("skeleton", 3), ("spider", 3)])


# ---------------------------------------------------------------------------------------------
# Explorify idea: little vanilla-style points of interest.
# ---------------------------------------------------------------------------------------------

@group("explorify")
def abandoned_campsite():
    rng = random.Random(111)
    c = Canvas(11, 6, 11)
    patch(c, rng, ["grass_block", "dirt", "rocky_dirt", "grass_block"], clear=5)
    for k in range(4):
        for (dx, dy) in ((0, 0), (1, 1), (3, 1), (4, 0)):
            c.set(1 + dx, 1 + dy, 2 + k, "white_wool" if rng.random() > 0.25 else AIR)
    c.set(3, 3, 3, "white_wool")
    c.set(3, 3, 4, AIR)
    c.disc(7, 7, 1.2, 0, "cobblestone")
    c.set(7, 0, 7, "dirt")
    c.set(7, 1, 7, "twig")
    c.set(5, 1, 7, "oak_log")
    c.set(7, 1, 9, "oak_log")
    c.set(3, 1, 3, "chest")
    c.set(9, 1, 3, "pebble")
    c.set(8, 1, 2, "twig")
    return struct("acampamento_abandonado", ["Plains", "Forest", "Birch Forest", "Taiga", "Savanna"], c, 10, 0.3,
                  loot=small(("string", 1, 3, 0.4), ("cooked_porkchop", 1, 2, 0.4)), sink=1, foundation="dirt")


@group("explorify")
def dungeon_entrance():
    """A mossy stair going down from a little arch to a cobblestone room with two chests."""
    rng = random.Random(112)
    c = Canvas(11, 12, 11)
    sink = 7
    c.room(1, 0, 1, 9, 4, 9, "cobblestone", floor="mossy_cobblestone", ceil="cobblestone")
    c.set(5, 1, 2, "chest")
    c.set(4, 1, 2, "chest")
    c.set(5, 3, 5, HANG)
    c.set(2, 1, 8, "quartz_slab")
    c.set(2, 1, 3, "brown_mushroom")
    c.box(4, 1, 6, 6, 1, 7, "stone_brick_slab")
    # A stair along the east side, rising south, walled in up to the surface.
    for k in range(sink - 1):
        y, z = 1 + k, 3 + k
        c.box(7, y - 1, z - 1, 9, min(y + 4, sink - 1), z + 1, "cobblestone")
    for k in range(sink - 1):
        y, z = 1 + k, 3 + k
        c.set(8, y, z, stair("cobblestone_stairs", S))
        c.box(8, y + 1, z, 8, y + 3, z, AIR)
    c.box(8, 1, 2, 8, 3, 2, AIR)
    c.box(7, 1, 2, 7, 2, 5, AIR)
    # The arch at the top.
    zt = 3 + sink - 1
    c.set(8, sink - 1, zt, "mossy_cobblestone")
    for x in (7, 9):
        c.column(x, zt - 1, sink, sink + 2, "mossy_cobblestone")
    c.box(7, sink + 3, zt - 1, 9, sink + 3, zt - 1, "cobblestone_slab")
    c.weather(rng, {"cobblestone": ["mossy_cobblestone", "cobblestone"]}, 0.35)
    return struct("entrada_de_masmorra", ["Plains", "Forest", "Taiga", "Dark Forest", "Mountains", "Birch Forest"], c, 12, 0.35,
                  loot=small(("bone", 1, 5, 0.6), ("redstone", 2, 6, 0.4), ("gunpowder", 1, 4, 0.4)), sink=sink, rough=5, mobs=[("zombie", 2), ("skeleton", 1)])


@group("explorify")
def bone_pile():
    """The bones of some huge old beast half sunk in the ground: a spine, ribs and a skull."""
    rng = random.Random(113)
    c = Canvas(13, 7, 9)
    for x in range(1, 12):
        c.set(x, 1, 4, "quartz_block" if x % 2 else "quartz_slab")
    for x in range(3, 10, 2):
        for sgn in (-1, 1):
            c.set(x, 1, 4 + sgn, "quartz_block")
            c.set(x, 2, 4 + sgn * 2, "quartz_block")
            c.set(x, 2, 4 + sgn * 3, "quartz_slab")
            c.set(x, 1, 4 + sgn * 3, "quartz_block")
    c.box(10, 1, 3, 12, 2, 5, "quartz_block")
    c.set(12, 2, 4, AIR)
    c.set(11, 2, 3, AIR)
    c.set(11, 2, 5, AIR)
    for (x, z) in ((2, 1), (6, 8), (9, 0)):
        c.set(x, 1, z, rng.choice(["dead_bush", "pebble", "quartz_slab"]))
    return struct("pilha_de_ossos", ["Desert", "Badlands", "Savanna", "Snowy Plains"], c, 12, 0.3, sink=2)


@group("explorify")
def hunters_blind():
    """A hunter's hide on stilts, screened with spruce boughs, with a stair up the back."""
    rng = random.Random(114)
    c = Canvas(7, 8, 9)
    for (x, z) in ((1, 1), (5, 1), (1, 5), (5, 5)):
        c.column(x, z, 0, 3, "spruce_log")
    c.box(1, 4, 1, 5, 4, 5, "spruce_planks")
    c.walls(1, 1, 5, 5, 5, 5, "spruce_fence")
    for (x, z) in square_ring(0, 0, 6, 6):
        if rng.random() < 0.7:
            c.set(x, 5, z, "spruce_leaves")
            if rng.random() < 0.5:
                c.set(x, 6, z, "spruce_leaves")
    c.set(3, 5, 5, AIR)
    c.set(3, 5, 6, AIR)
    c.set(3, 6, 6, AIR)
    for k in range(4):
        c.set(3, k, 8 - k, stair("spruce_stairs", N))
    c.set(3, 4, 5, "spruce_planks")
    c.set(3, 5, 1, "spruce_fence")
    c.set(3, 5, 3, "chest")
    c.set(2, 5, 2, var("spruce_chair", N))
    c.box(2, 7, 2, 4, 7, 4, "spruce_slab")
    for (x, z) in ((1, 1), (5, 1), (1, 5), (5, 5)):
        c.column(x, z, 5, 6, "spruce_fence")
    return struct("esconderijo_de_cacador", ["Taiga", "Forest", "Birch Forest", "Dark Forest", "Snowy Plains"], c, 10, 0.3,
                  loot=small(("arrow", 4, 12, 0.7), ("bow", 1, 1, 0.3), ("cooked_venison", 1, 3, 0.5)), foundation="spruce_log")


@group("explorify")
def forgotten_well():
    rng = random.Random(115)
    c = Canvas(7, 16, 7)
    sink = 10
    for y in range(0, sink):
        c.walls(1, 1, 5, 5, y, y, rng.choice(["cobblestone", "mossy_cobblestone", "cobblestone"]))
        c.box(2, y, 2, 4, y, 4, "water" if y < 3 else AIR)
    c.box(2, 0, 2, 4, 0, 4, "gravel")
    c.set(3, 0, 3, "chest")
    c.walls(1, 1, 5, 5, sink, sink, "mossy_cobblestone")
    for (x, z) in ((1, 1), (5, 5)):
        c.column(x, z, sink + 1, sink + 3, "oak_fence")
    c.line(1, sink + 4, 1, 5, sink + 4, 5, "oak_slab")
    c.set(3, sink + 4, 3, "oak_planks")
    for y in range(sink - 3, sink + 4):
        c.set(3, y, 3, "chain")
    for z in range(7):
        for x in range(7):
            if c.get(x, sink - 1, z) is None and rng.random() < 0.5:
                c.set(x, sink - 1, z, "gravel")
    return struct("poco_esquecido", ["Plains", "Forest", "Savanna", "Taiga", "Birch Forest"], c, 12, 0.3,
                  loot=small(("emerald", 1, 4, 0.5), ("gold_nugget", 2, 8, 0.6), ("bucket", 1, 1, 0.4)), sink=sink, rough=4)


@group("explorify")
def ruined_arch():
    rng = random.Random(116)
    c = Canvas(9, 10, 5)
    patch(c, rng, ["netherrack", "stone", "gravel", "magma_block", "obsidian"], clear=9, round_=False)
    for y in range(1, 8):
        c.set(2, y, 2, "obsidian" if rng.random() < 0.8 else AIR)
        c.set(6, y, 2, "obsidian" if y < 5 or rng.random() < 0.4 else AIR)
    for x in (3, 4, 5):
        c.set(x, 7, 2, "obsidian" if x != 5 else AIR)
        c.set(x, 1, 2, "obsidian")
    c.set(1, 1, 3, "chest")
    c.set(4, 1, 0, "netherrack")
    c.set(7, 1, 4, "magma_block")
    c.set(3, 2, 1, "vine")
    c.set(6, 1, 0, "lava")
    c.set(6, 0, 0, "netherrack")
    remove_floaters(c)
    return struct("arco_em_ruinas", ["Plains", "Forest", "Desert", "Taiga", "Savanna", "Jungle", "Mountains", "Swamp"], c, 14, 0.3,
                  loot=[("obsidian", 1, 4, 0.6), ("flint_and_steel", 1, 1, 0.4), ("gold_ingot", 1, 4, 0.5), ("gold_nugget", 3, 9, 0.6),
                        ("golden_sword", 1, 1, 0.2), ("fire_staff", 1, 1, 0.05)], sink=1, foundation="netherrack")


@group("explorify")
def desert_tomb():
    """A buried sandstone tomb: a stair down from two little pillars to a burial chamber with
    a raised stone coffin, braziers and a chest."""
    rng = random.Random(117)
    c = Canvas(9, 12, 13)
    sink = 6
    c.room(1, 0, 1, 7, 4, 8, "sandstone", floor="smooth_stone", ceil="sandstone")
    c.box(3, 1, 2, 5, 1, 4, "sandstone_slab:2")
    c.set(4, 2, 3, "orange_terracotta")
    c.set(4, 1, 6, "chest")
    c.set(2, 1, 7, "gold_block")
    c.set(6, 1, 7, "brazier")
    c.set(2, 1, 2, "brazier")
    c.set(4, 3, 6, HANG)
    for z in range(2, 8, 2):
        c.set(1, 2, z, "orange_terracotta")
        c.set(7, 2, z, "orange_terracotta")
    # The stair up to the south.
    for k in range(sink - 1):
        y, z = 1 + k, 8 + k
        c.box(3, y - 1, z, 5, min(y + 3, sink - 1), z, "sandstone")
        c.set(4, y, z, stair("sandstone_stairs", S))
        c.box(4, y + 1, z, 4, y + 3, z, AIR)
    c.box(4, 1, 8, 4, 2, 8, AIR)
    c.set(4, 1, 8, stair("sandstone_stairs", S))
    zt = 8 + sink - 2
    for x in (3, 5):
        c.column(x, zt, sink, sink + 1, "sandstone")
        c.set(x, sink + 2, zt, "sandstone_slab")
    c.box(4, sink, zt, 4, sink + 1, zt, AIR)
    return struct("tumba_do_deserto", ["Desert"], c, 12, 0.35,
                  loot=small(("gold_ingot", 2, 6, 0.6), ("bone", 2, 6, 0.6), ("topaz", 1, 2, 0.3), ("golden_helmet", 1, 1, 0.15)), sink=sink, rough=4,
                  mobs=[("zombie", 2)])


@group("explorify")
def witch_altar():
    rng = random.Random(118)
    c = Canvas(9, 6, 9)
    patch(c, rng, ["moss_block", "dirt", "mossy_cobblestone"], clear=5)
    for (x, z) in circle_ring(4, 4, 3.3)[::2]:
        c.set(x, 1, z, "dark_oak_fence")
        if rng.random() < 0.4:
            c.set(x, 2, z, "dark_oak_fence")
    c.box(3, 1, 3, 5, 1, 5, "mossy_cobblestone")
    c.set(4, 2, 4, "brazier")
    c.set(3, 2, 3, "red_mushroom")
    c.set(5, 2, 5, "brown_mushroom")
    c.set(5, 2, 3, "potted_poppy")
    c.set(3, 2, 5, "chest")
    for (x, z) in ((1, 1), (7, 7)):
        c.set(x, 1, z, "dark_oak_log")
        c.set(x, 2, z, "lantern")
    return struct("altar_da_bruxa", ["Swamp", "Dark Forest"], c, 10, 0.35,
                  loot=small(("redstone", 2, 6, 0.6), ("glow_berries", 1, 4, 0.5), ("amethyst_wand", 1, 1, 0.08), ("string", 1, 4, 0.4)),
                  sink=1, foundation="dirt", mobs=[("swamp_witch", 1)])


@group("explorify")
def mine_entrance():
    rng = random.Random(119)
    c = Canvas(9, 10, 13)
    sink = 5
    # A timbered adit going down into the ground.
    for k in range(12):
        z = 12 - k
        y = sink - (k // 2)
        y = max(y, 1)
        c.box(3, y, z, 5, y + 2, z, AIR)
        c.set(4, y - 1, z, "rail")
        c.box(3, y - 1, z, 5, y - 1, z, "gravel")
        c.set(4, y, z, "rail")
        if k % 3 == 0:
            c.column(2, z, y - 1, y + 2, "oak_log")
            c.column(6, z, y - 1, y + 2, "oak_log")
            c.box(2, y + 3, z, 6, y + 3, z, "oak_planks")
    c.set(4, 3, 3, HANG)
    c.set(3, 1, 1, "chest")
    c.box(3, 1, 0, 5, 3, 1, AIR)
    for (x, z) in ((1, 12), (7, 12)):
        c.column(x, z, sink, sink + 1, "oak_fence")
        c.set(x, sink + 2, z, "lantern")
    return struct("entrada_de_mina", ["Mountains", "Taiga", "Badlands", "Plains"], c, 12, 0.3,
                  loot=small(("raw_iron", 2, 5, 0.6), ("coal", 3, 9, 0.6), ("rail", 2, 8, 0.4), ("iron_pickaxe", 1, 1, 0.15)), sink=sink, rough=5)


@group("explorify")
def sunken_cellar():
    """A farmhouse cellar left behind: a stair down from a fenced hatch to a room of crates,
    drawers and a chest of stores."""
    rng = random.Random(120)
    c = Canvas(9, 9, 11)
    sink = 5
    c.room(1, 0, 1, 7, 4, 7, "oak_planks", floor="cobblestone", ceil="oak_planks")
    for (x, z) in ((1, 1), (7, 1), (1, 7), (7, 7)):
        c.column(x, z, 0, 4, "oak_log")
    for z in range(2, 7):
        c.set(2, 1, z, rng.choice(["oak_drawer", "hay_block", "oak_planks", "pumpkin"]))
        c.set(6, 1, z, rng.choice(["oak_drawer", "oak_planks", "melon"]))
    c.set(4, 1, 2, "chest")
    c.set(4, 3, 4, HANG)
    for k in range(sink - 1):
        y, z = 1 + k, 6 + k
        c.box(3, y - 1, z, 5, min(y + 3, sink - 1), z, "oak_planks")
        c.set(4, y, z, stair("oak_stairs", S))
        c.box(4, y + 1, z, 4, y + 3, z, AIR)
    zt = 6 + sink - 1
    c.box(4, sink, zt, 4, sink + 1, zt, AIR)
    for x in (3, 5):
        c.set(x, sink, zt, "oak_fence")
    c.set(3, sink + 1, zt, "lantern")
    return struct("adega_submersa", ["Plains", "Forest", "Birch Forest", "Taiga", "Cherry Grove"], c, 12, 0.3,
                  loot=small(("potato", 2, 6, 0.6), ("carrot", 2, 6, 0.6), ("apple", 2, 5, 0.6), ("wheat", 2, 8, 0.5), ("bread", 1, 4, 0.5)),
                  sink=sink, rough=4)


# ---------------------------------------------------------------------------------------------
# Towns and Towers idea: single buildings.
# ---------------------------------------------------------------------------------------------

@group("towns")
def inn():
    """A roadside inn: two storeys, a common room with tables and a hearth, rooms with beds
    upstairs, a sign post and a stable with hay."""
    rng = random.Random(131)
    c = Canvas(21, 16, 15)
    patch(c, rng, ["grass_block", "gravel", "dirt", "grass_block"], clear=15, round_=False)
    x0, z0, x1, z1 = 1, 2, 13, 11
    c.box(x0, 0, z0, x1, 0, z1, "cobblestone")
    c.room(x0, 1, z0, x1, 4, z1, "oak_planks")
    c.walls(x0, z0, x1, z1, 1, 1, "cobblestone")
    c.box(x0, 5, z0, x1, 5, z1, "spruce_planks")
    c.room(x0, 6, z0, x1, 8, z1, "white_wool")
    for (x, z) in ((x0, z0), (x1, z0), (x0, z1), (x1, z1)):
        c.column(x, z, 1, 8, "oak_log")
    for x in range(x0, x1 + 1, 4):
        c.column(x, z0, 6, 8, "oak_log")
        c.column(x, z1, 6, 8, "oak_log")
    c.walls(x0, z0, x1, z1, 5, 5, "oak_log")
    windows_on(c, x0, z0, x1, z1, 2, "glass_pane", skip=((7, z1),), every=3)
    windows_on(c, x0, z0, x1, z1, 7, "glass_pane", every=3)
    gable(c, x0, z0, x1, z1, 9, "spruce_roof", "white_wool", "spruce_slab", axis="x")
    c.set(7, 1, z1, door("oak_door", S)[0])
    c.set(7, 2, z1, door("oak_door", S)[1])
    # Common room.
    for (tx, tz) in ((4, 5), (4, 8), (10, 8)):
        c.set(tx, 1, tz, "oak_table")
        c.set(tx, 2, tz, "lantern")
        c.set(tx - 1, 1, tz, var("oak_chair", E))
        c.set(tx + 1, 1, tz, var("oak_chair", W))
    for x in range(9, 13):
        c.set(x, 1, 4, "spruce_table")
    c.set(10, 1, 3, "chest")
    c.set(11, 1, 3, "spruce_drawer")
    c.set(12, 1, 3, "furnace")
    c.box(2, 1, 3, 2, 3, 3, "bricks")
    c.set(2, 1, 4, "brazier")
    c.column(2, 3, 4, 13, "bricks")
    for x in (5, 9):
        c.set(x, 4, 6, "chain")
        c.set(x, 3, 6, "chandelier")
    # Stairs up and the rooms.
    stair_run(c, 12, 1, 10, N, 4, "oak_stairs", clear=2)
    c.box(12, 5, 6, 12, 5, 9, AIR)
    for x in (3, 6, 9):
        c.set(x, 6, 3, "red_bed")
        c.set(x + 1, 6, 3, "oak_drawer")
        c.set(x, 6, 10, "red_bed")
    for z in (6, 7):
        c.set(6, 6, z, "oak_planks")
        c.set(6, 7, z, "oak_planks")
    c.set(7, 8, 6, HANG)
    # Sign post and stable.
    c.column(15, 13, 1, 3, "oak_fence")
    c.box(15, 3, 12, 15, 3, 12, "oak_planks")
    c.set(15, 4, 13, "lantern")
    for (x, z) in ((15, 2), (19, 2), (15, 9), (19, 9)):
        c.column(x, z, 1, 3, "spruce_log")
    c.box(14, 4, 1, 20, 4, 10, "spruce_slab")
    c.walls(15, 2, 19, 9, 1, 1, "oak_fence")
    c.box(15, 1, 5, 15, 1, 6, AIR)
    for z in (3, 5, 7):
        c.set(19, 1, z, "hay_block")
    return struct("estalagem", ["Plains", "Forest", "Birch Forest", "Taiga", "Savanna"], c, 22, 0.35,
                  loot=small(("bread", 2, 6, 0.8), ("cooked_beef", 1, 4, 0.5), ("apple", 2, 5, 0.5), ("lute", 1, 1, 0.1)),
                  sink=1, foundation="cobblestone", rough=5, mobs=[("npc_merchant", 1), ("npc_guide", 1)])


@group("towns")
def smithy():
    """A blacksmith's forge: a stone workshop open on one side, a brick hearth with lava behind
    bars, an anvil block, racks of tools and the smith's little house."""
    rng = random.Random(132)
    c = Canvas(17, 12, 13)
    patch(c, rng, ["gravel", "cobblestone", "grass_block", "rocky_dirt"], clear=11, round_=False)
    c.box(1, 0, 1, 9, 0, 11, "cobblestone")
    for (x, z) in ((1, 1), (9, 1), (1, 11), (9, 11), (9, 6)):
        c.column(x, z, 1, 4, "spruce_log")
    c.walls(1, 1, 9, 11, 1, 1, "cobblestone")
    c.box(2, 1, 11, 8, 4, 11, "cobblestone")
    c.box(1, 1, 2, 1, 4, 10, "cobblestone")
    c.box(2, 1, 1, 8, 4, 1, "cobblestone")
    c.box(9, 1, 2, 9, 1, 10, AIR)
    gable(c, 1, 1, 9, 11, 5, "stone_brick_roof", "cobblestone", "stone_brick_slab", axis="z")
    c.box(2, 1, 2, 4, 2, 4, "bricks")
    c.set(3, 1, 3, "lava")
    c.set(3, 2, 3, "bricks")
    c.set(3, 1, 4, "iron_bars")
    c.column(2, 2, 3, 10, "bricks")
    c.set(6, 1, 6, "iron_block")
    c.set(6, 1, 7, "smooth_stone")
    for z in range(6, 11, 2):
        c.set(2, 1, z, "furnace" if z == 6 else "spruce_drawer")
        c.set(2, 2, z, var("spruce_shelf", E))
    c.set(5, 1, 10, "chest")
    c.set(6, 1, 10, "crafting_table")
    c.set(7, 1, 10, "spruce_table")
    c.set(5, 4, 6, HANG)
    st = style("spruce", roof="spruce_roof", base="cobblestone", floor="spruce_planks")
    cottage(c, 11, 4, 15, 9, st, W, rng, wh=3, things=["bed", "table", "shelf"])
    return struct("ferraria", ["Plains", "Taiga", "Mountains", "Forest", "Snowy Plains"], c, 22, 0.35,
                  loot=small(("iron_ingot", 3, 8, 0.8), ("iron_sword", 1, 1, 0.3), ("iron_pickaxe", 1, 1, 0.3), ("iron_helmet", 1, 1, 0.2),
                             ("iron_hammer", 1, 1, 0.12)), sink=1, foundation="cobblestone", rough=5, mobs=[("npc_smith", 1)])


@group("towns")
def chapel():
    """A stone chapel with a pointed steeple, tall windows, pews and an altar."""
    rng = random.Random(133)
    c = Canvas(13, 26, 19)
    patch(c, rng, ["grass_block", "gravel", "grass_block"], clear=25, round_=False)
    x0, z0, x1, z1 = 2, 5, 10, 17
    building(c, x0, z0, x1, z1, 1, 6, "stone_bricks", "smooth_stone", "stone_brick_roof", "stone_bricks", "stone_brick_slab",
             "glass_pane", "smooth_stone", doors=(), axis="z", every=3)
    for z in range(z0 + 2, z1 - 1, 3):
        for y in (2, 3, 4):
            c.set(x0, y, z, "glass_pane")
            c.set(x1, y, z, "glass_pane")
    for z in range(z0 + 3, z1 - 2):
        for x in (4, 5, 7, 8):
            c.set(x, 1, z, var("oak_bench", N))
    c.box(4, 1, z0 + 1, 8, 1, z0 + 1, "smooth_stone")
    c.set(6, 2, z0 + 1, "gold_block")
    c.set(5, 2, z0 + 1, "potted_poppy")
    c.set(7, 2, z0 + 1, "lantern")
    c.set(6, 1, z0 + 2, "chest")
    c.set(6, 6, 11, "chain")
    c.set(6, 5, 11, "chandelier")
    # Steeple over the door at the south end.
    s0 = z1 - 3
    c.room(4, 1, s0, 8, 13, z1 + 1, "stone_bricks")
    c.box(5, 1, z1 + 1, 7, 13, z1 + 1, "stone_bricks")
    c.box(6, 1, z1 + 1, 6, 3, z1 + 1, AIR)
    c.box(6, 1, s0, 6, 3, s0, AIR)
    c.set(6, 1, z1 + 1, door("spruce_door", S)[0])
    c.set(6, 2, z1 + 1, door("spruce_door", S)[1])
    for y in (11, 12):
        for (x, z) in ((6, s0), (6, z1 + 1), (4, (s0 + z1 + 1) // 2), (8, (s0 + z1 + 1) // 2)):
            c.set(x, y, z, AIR)
    c.box(5, 11, s0 + 1, 7, 12, z1, AIR)
    c.set(6, 12, (s0 + z1 + 1) // 2, "chain")
    c.set(6, 11, (s0 + z1 + 1) // 2, "gold_block")
    for k in range(6):
        r = 2.6 - k * 0.45
        if r <= 0.3:
            break
        for z in range(s0 - 1, z1 + 3):
            for x in range(3, 10):
                if abs(x - 6) <= r and abs(z - (s0 + z1 + 1) / 2) <= r + 0.5:
                    c.set(x, 14 + k * 2, z, "stone_bricks")
                    c.set(x, 15 + k * 2, z, "stone_bricks")
    c.column(6, (s0 + z1 + 1) // 2, 14, 25, "stone_bricks")
    c.set(6, 25, (s0 + z1 + 1) // 2, "gold_block")
    c.weather(rng, {"stone_bricks": ["cobblestone_bricks", "stone_bricks"]}, 0.1)
    return struct("capela", ["Plains", "Forest", "Birch Forest", "Mountains", "Snowy Plains", "Cherry Grove"], c, 24, 0.35,
                  loot=small(("mana_crystal", 1, 2, 0.4), ("gold_ingot", 1, 4, 0.5), ("life_crystal", 1, 1, 0.12)),
                  sink=1, foundation="stone_bricks", rough=5, mobs=[("npc_nurse", 1)])


def tall_tower(name, biomes, wall, accent, roof, wood, ground, seed, top_kind="cone"):
    """A tall round tower with a spiral stair, floors with furniture, windows and a pointed or
    flat roof (Towns and Towers style)."""
    rng = random.Random(seed)
    wd = wood_set(wood)
    c = Canvas(13, 32, 13)
    patch(c, rng, ground, clear=31)
    cx = cz = 6
    H = 22
    round_tower(c, cx, cz, 4.6, 0, H, wall, floor=wall, windows="glass_pane", every=5)
    for y in range(1, H + 1, 6):
        c.disc(cx, cz, 4.6, y, accent, hollow=True, thick=1.2)
    ring = circle_ring(cx, cz, 2.7)
    spiral_stairs(c, ring, 1, H - 1, wd["stairs"])
    c.column(cx, cz, 1, H, wd["log"])
    for y in (8, 15):
        c.disc(cx, cz, 2.0, y, wd["planks"])
        c.set(cx, y, cz, wd["log"])
        c.set(cx + 1, y + 1, cz, wd["table"])
        c.set(cx - 1, y + 1, cz, "lantern" if y == 8 else "bookshelf")
    c.disc(cx, cz, 3.6, H, wd["planks"])
    k = (H - 2) % len(ring)
    for j in range(3):
        x, z = ring[(k - j) % len(ring)]
        c.set(x, H, z, AIR)
    c.set(cx + 1, H + 1, cz + 1, "chest")
    c.set(cx - 1, H + 1, cz - 1, "red_bed")
    c.box(cx - 1, 1, cz + 4, cx + 1, 3, cz + 5, AIR)
    c.set(cx, 1, cz + 4, door(wd["door"], S)[0])
    c.set(cx, 2, cz + 4, door(wd["door"], S)[1])
    if top_kind == "cone":
        for k in range(9):
            r = 5.8 - k * 0.65
            c.disc(cx, cz, max(r, 0.4), H + 4 + k, roof, hollow=r > 1.5, thick=1.3)
        c.disc(cx, cz, 4.6, H + 1, wall, hollow=True, thick=1.2)
        c.disc(cx, cz, 4.6, H + 2, wall, hollow=True, thick=1.2)
        c.disc(cx, cz, 4.6, H + 3, wall, hollow=True, thick=1.2)
        for (dx, dz) in ((0, -1), (0, 1), (-1, 0), (1, 0)):
            c.set(round(cx + dx * 4.6), H + 2, round(cz + dz * 4.6), "glass_pane")
        c.set(cx, H + 3, cz, HANG)
    else:
        c.disc(cx, cz, 5.6, H + 1, accent)
        c.disc(cx, cz, 4.4, H + 1, wd["planks"])
        for (x, z) in circle_ring(cx, cz, 5.4)[::2]:
            c.set(x, H + 2, z, accent)
        c.set(cx, H + 2, cz, "lantern")
        for j in range(3):
            x, z = ring[(k - j) % len(ring)]
            c.set(x, H + 1, z, AIR)
    return struct(name, biomes, c, 22, 0.35, loot=small(("arrow", 2, 8, 0.4), ("mana_crystal", 1, 1, 0.2), ("bow", 1, 1, 0.2)),
                  sink=1, foundation=wall, rough=5)


@group("towns")
def towers():
    return [
        tall_tower("torre_de_pedra", ["Plains", "Forest", "Birch Forest", "Dark Forest"], "stone_bricks", "cobblestone_bricks", "spruce_planks", "spruce",
                   ["grass_block", "gravel", "cobblestone"], 141),
        tall_tower("torre_de_arenito", ["Desert", "Badlands"], "sandstone", "orange_terracotta", "sandstone", "jungle",
                   ["sand", "sandstone", "smooth_stone"], 142, top_kind="flat"),
        tall_tower("torre_nevada", ["Snowy Plains", "Taiga"], "cobblestone_bricks", "spruce_planks", "light_blue_wool", "spruce",
                   ["snow", "snowy_grass_block", "gravel"], 143),
    ]


def wizard_tower(name, biomes, wall, accent, roof, ground, mobs, seed):
    """Iron's Spells 'n Spellbooks' idea: a wizard's tower. A round stone tower banded with
    amethyst under a tall crooked hat of a roof; a library on the first floor, the inscription
    table on the second, a study at the top. Chests hold scrolls, ink and sometimes the seal
    that calls the Corrupted Archmage; wizards keep watch."""
    rng = random.Random(seed)
    wd = wood_set("dark_oak")
    c = Canvas(17, 48, 17)
    patch(c, rng, ground, clear=46)
    cx = cz = 8
    H = 27
    r = 5.6
    round_tower(c, cx, cz, r, 0, H, wall, floor=wall, windows="glass_pane", every=4)
    for y in (1, 9, 18, H):
        c.disc(cx, cz, r, y, accent, hollow=True, thick=1.2)
    ring = circle_ring(cx, cz, 3.6)
    spiral_stairs(c, ring, 1, H - 1, wd["stairs"])
    c.column(cx, cz, 1, H, "bookshelf")
    floors = (9, 18)
    for y in floors + (H,):
        c.disc(cx, cz, r - 1.2, y, wd["planks"])
        c.disc(cx, cz, 2.6, y, wd["planks"])
        k = (y - 2) % len(ring)
        for j in range(3):
            x, z = ring[(k - j) % len(ring)]
            c.set(x, y, z, AIR)
    # Ground floor: a hall lined with shelves.
    for (x, z) in circle_ring(cx, cz, r - 1.1)[::3]:
        if abs(z - cz) > 1 or x < cx:
            c.set(x, 1, z, "bookshelf")
            c.set(x, 2, z, "bookshelf")
    c.set(cx, 3, cz + 2, HANG)
    # First floor: the library.
    y = floors[0] + 1
    for (x, z) in circle_ring(cx, cz, r - 1.1)[::2]:
        c.set(x, y, z, "bookshelf")
        c.set(x, y + 1, z, "bookshelf")
    c.set(cx + 2, y, cz - 2, "chest")
    c.set(cx - 2, y + 3, cz, HANG)
    # Second floor: the inscription table, a desk and the ink.
    y = floors[1] + 1
    c.set(cx + 2, y, cz + 1, "inscription_table")
    c.set(cx - 2, y, cz + 1, wd["table"])
    c.set(cx - 2, y + 1, cz + 1, "lantern")
    c.set(cx + 2, y, cz - 2, "chest")
    for (x, z) in circle_ring(cx, cz, r - 1.1)[1::4]:
        c.setif(x, y, z, "bookshelf")
    # Top: the study under the roof, with the best chest and amethyst.
    y = H + 1
    for k in range(4):
        c.disc(cx, cz, r, y + k, wall, hollow=True, thick=1.2)
    for (dx, dz) in ((0, -1), (0, 1), (-1, 0), (1, 0)):
        c.set(round(cx + dx * r), y + 1, round(cz + dz * r), "glass_pane")
    c.set(cx, y, cz, "chest")
    c.set(cx + 2, y, cz + 2, "amethyst_block")
    c.set(cx - 2, y, cz - 2, "amethyst_block")
    c.set(cx + 2, y, cz - 2, "inscription_table")
    c.set(cx, y + 3, cz, HANG)
    # The roof: a tall pointed hat whose tip leans to one side.
    base = y + 4
    c.disc(cx, cz, r + 1.6, base, roof)
    c.disc(cx, cz, r + 0.2, base + 1, accent, hollow=True, thick=1.0)
    n = 16
    for k in range(1, n):
        t = k / n
        rr = (r + 0.2) * (1 - t) ** 1.25
        lean = round(t ** 2.2 * 3.5)
        c.disc(cx + lean, cz, max(rr, 0.45), base + 1 + k, roof)
    # Door.
    c.box(cx - 1, 1, cz + 5, cx + 1, 3, cz + 6, AIR)
    c.set(cx, 1, cz + 5, door(wd["door"], S)[0])
    c.set(cx, 2, cz + 5, door(wd["door"], S)[1])
    c.weather(rng, {wall: [wall, "mossy_cobblestone"] if wall == "stone_bricks" else [wall]}, 0.08)
    loot = [("arcane_ink", 2, 6, 0.85), ("blank_scroll", 2, 6, 0.7), ("spellbook_apprentice", 1, 1, 0.25), ("mana_crystal", 1, 2, 0.3),
            ("corrupted_seal", 1, 1, 0.12), ("arcane_essence", 1, 1, 0.06), ("lapis_lazuli", 2, 6, 0.4)]
    for s in ("fireball", "ice_spike", "magic_missile", "heal", "teleport", "fortitude", "arcane_barrier", "lightning_lance",
              "thorn_roots", "gust", "life_drain", "fangs", "summon_wolves", "haste", "void_rift", "storm_call"):
        loot.append((f"scroll_{s}", 1, 2, 0.1))
    loot.append(("scroll_abyss_ray", 1, 1, 0.03))
    for p in ("pyromancer", "cryomancer", "priest", "voidwalker", "archevoker"):
        loot.append((f"{p}_{rng.choice(['helmet', 'chestplate', 'leggings', 'boots'])}", 1, 1, 0.05))
    return struct(name, biomes, c, 40, 0.3, loot=loot, sink=1, foundation=wall, rough=5, mobs=mobs)


@group("spellbooks")
def wizard_towers():
    return [
        wizard_tower("torre_arcana", ["Plains", "Forest", "Birch Forest", "Dark Forest", "Swamp"], "stone_bricks", "amethyst_block",
                     "purple_wool", ["grass_block", "gravel", "cobblestone", "grass_block"], [("pyromancer", 1), ("necromancer", 1)], 171),
        wizard_tower("torre_arcana_nevada", ["Snowy Plains", "Taiga", "Mountains"], "cobblestone_bricks", "amethyst_block",
                     "blue_wool", ["snow", "snowy_grass_block", "gravel"], [("cryomancer", 2)], 172),
    ]


@group("towns")
def barn():
    """A big red barn with a hay loft, stalls with hay, a fenced paddock and a water trough."""
    rng = random.Random(144)
    c = Canvas(23, 15, 17)
    patch(c, rng, ["grass_block", "dirt", "grass_block"], clear=14, round_=False)
    x0, z0, x1, z1 = 1, 2, 11, 14
    building(c, x0, z0, x1, z1, 1, 5, "red_terracotta", "dark_oak_log", "spruce_roof", "red_terracotta", "spruce_slab", "oak_window", "spruce_planks",
             doors=(), axis="z", every=4)
    c.box(5, 1, z1, 7, 4, z1, AIR)
    c.box(5, 1, z0, 7, 3, z0, AIR)
    for z in range(z0 + 1, z1):
        c.set(x0 + 1, 3, z, "spruce_planks")
        c.set(x0 + 2, 3, z, "spruce_planks")
        c.set(x1 - 1, 3, z, "spruce_planks")
        c.set(x1 - 2, 3, z, "spruce_planks")
    for z in range(z0 + 1, z1, 2):
        c.set(x0 + 1, 4, z, "hay_block")
        c.set(x1 - 1, 4, z, "hay_block")
        c.set(x0 + 3, 1, z, "spruce_fence")
        c.set(x1 - 3, 1, z, "spruce_fence")
    c.set(x0 + 1, 1, z0 + 2, "hay_block")
    c.set(x1 - 1, 1, z1 - 2, "hay_block")
    c.set(x0 + 1, 1, z1 - 1, "chest")
    c.set(6, 5, 8, HANG)
    stair_run(c, x1 - 1, 1, z1 - 4, N, 2, "spruce_stairs", clear=2)
    # Paddock.
    c.walls(13, 2, 21, 14, 1, 1, "oak_fence")
    c.box(13, 1, 7, 13, 1, 9, AIR)
    c.box(17, 0, 3, 19, 0, 3, "water")
    c.box(16, 1, 3, 20, 1, 3, "oak_slab")
    c.box(17, 1, 3, 19, 1, 3, AIR)
    for (x, z) in ((19, 12), (15, 11)):
        c.set(x, 1, z, "hay_block")
    return struct("celeiro", ["Plains", "Savanna", "Forest", "Birch Forest"], c, 20, 0.35,
                  loot=small(("wheat", 4, 12, 0.8), ("wheat_seeds", 3, 9, 0.6), ("leather", 1, 4, 0.4), ("carrot", 2, 6, 0.5)),
                  sink=1, foundation="dirt", rough=5, mobs=[("npc_farmer", 1)])


# ==== END OF STRUCTURES ====

if __name__ == "__main__":
    main()
