"""Hand-made 64x64 item textures: tools, weapons, armour icons, ingots, raw ores and gems.

Items are drawn as shapes (polygons, thick strokes, discs) and painted with the same rules as the
blocks: a material ramp, a bevel lit from the top-left, and a 2-pixel outline in the material's
own darkest, cooler colour (never pure black). Tools are held diagonally with the head at the
top-right, like the classic game. A tool is the same drawing for every material, so a new metal
only needs a colour in MATERIALS.
"""

import numpy as np

from style import N, XX, YY, canvas, clusters, dist_inside, from_oklab, hexrgb, light, quantize, ramp, rng_for, segment_dist, to_oklab

MATERIALS = {
    "wooden": "#a07a45", "stone": "#8c8c8c", "iron": "#d4d4d4", "golden": "#f2c53a",
    "diamond": "#52dcd2", "netherite": "#4a4246", "copper": "#d8803f", "bronze": "#bb8a44",
    "lead": "#6c7590", "silver": "#dfe3e8", "tungsten": "#7d9a80", "platinum": "#c3dfe8",
    "tin": "#aab4c0", "leather": "#8c5a34", "chainmail": "#9a9aa2", "gold": "#f2c53a",
    "magmarite": "#e0602a",
}
GEMS = {"diamond": "#52dcd2", "emerald": "#2fc860", "ruby": "#d8304a", "sapphire": "#3060d8",
        "topaz": "#e8a820", "amethyst": "#9a52d8", "lapis_lazuli": "#2f52c4", "quartz": "#ece6dc"}
STICK = "#8a6438"


# ---------------------------------------------------------------- shapes


def poly_mask(pts):
    """Even-odd fill of a polygon given in pixel coordinates."""
    px, py = XX + 0.5, YY + 0.5
    inside = np.zeros((N, N), bool)
    n = len(pts)
    for i in range(n):
        x0, y0 = pts[i]
        x1, y1 = pts[(i + 1) % n]
        cond = (y0 > py) != (y1 > py)
        xint = (x1 - x0) * (py - y0) / ((y1 - y0) + 1e-12) + x0
        inside ^= cond & (px < xint)
    return inside


def stroke(pts, w0, w1=None):
    """Thick polyline whose width goes from w0 to w1 along its length."""
    w1 = w0 if w1 is None else w1
    m = np.zeros((N, N), bool)
    seg = len(pts) - 1
    for i in range(seg):
        (x0, y0), (x1, y1) = pts[i], pts[i + 1]
        wa = w0 + (w1 - w0) * i / seg
        wb = w0 + (w1 - w0) * (i + 1) / seg
        d = segment_dist(x0, y0, x1, y1)
        m |= d <= (wa + wb) / 4
    return m


def disc_mask(cx, cy, r):
    return (XX + 0.5 - cx) ** 2 + (YY + 0.5 - cy) ** 2 <= r * r


def dilate(m, r=1):
    out = m.copy()
    for _ in range(r):
        p = np.pad(out, 1, constant_values=False)
        out = out | p[:-2, 1:-1] | p[2:, 1:-1] | p[1:-1, :-2] | p[1:-1, 2:]
    return out


class Sprite:
    def __init__(self, name):
        self.img = canvas()
        self.rng = rng_for(name)

    def part(self, mask, pal, base=0.55, bevel=3.0, lk=1.6, grain=0.0, extra=None, outline=2, shine=True):
        """Paint one part: outline ring, then the body (later parts go on top)."""
        if not mask.any():
            return
        dark = outline_color(pal)
        ring = dilate(mask, outline) & ~mask
        self.img[ring, :3] = dark
        self.img[ring, 3] = 255
        d = dist_inside(mask, 6)
        h = np.minimum(d, bevel) / bevel * 2.5
        t = base + light(h, wrap=False) * lk + (np.minimum(d, 6) / 6 - 0.5) * 0.12
        if grain:
            t = t + (clusters(self.rng, 2) - 0.5) * grain
        if extra is not None:
            t = t + extra
        col = quantize(np.clip(t, 0, 1), pal, 0.2, self.rng)
        self.img[mask, :3] = col[mask]
        self.img[mask, 3] = 255
        if shine:
            # A bright glint on the lit edge (top-left side), one or two pixels.
            e = (d == 2) & np.roll(~mask, 2, 0) & np.roll(~mask, 2, 1)
            self.img[e & mask, :3] = pal[-1]

    def done(self):
        return self.img


def outline_color(pal):
    lab = to_oklab(pal[0])
    lab[0] *= 0.62
    lab[2] -= 0.02  # cooler
    return from_oklab(lab)


def mat_pal(m, n=10, spread=0.5):
    base = MATERIALS.get(m, m)
    sat = 1.1 if m not in ("iron", "stone", "silver", "chainmail") else 1.0
    return ramp(base, n, spread, 0.12, sat=sat)


def wood_pal():
    return ramp(STICK, 9, 0.42, 0.12)


# ---------------------------------------------------------------- tools


HANDLE = [(9, 56), (41, 24)]


def handle(s, a=HANDLE[0], b=HANDLE[1], w=6.5):
    m = stroke([a, b], w)
    s.part(m, wood_pal(), base=0.5, bevel=2.0, lk=1.2, grain=0.15, shine=False)


def head_pal(mat):
    if mat == "wooden":
        return ramp(MATERIALS["wooden"], 10, 0.45, 0.12)
    return mat_pal(mat)


def sword(mat):
    s = Sprite(f"{mat}_sword")
    p = head_pal(mat)
    # Grip and pommel.
    grip = stroke([(8, 56), (19, 45)], 5)
    s.part(grip, wood_pal() if mat != "wooden" else ramp("#6e4c2a", 9, 0.4, 0.1), base=0.45, bevel=2, lk=1.0, shine=False)
    s.part(disc_mask(8.5, 55.5, 3.6), p, base=0.55, bevel=2)
    # Blade: tapering to a point, with a lit edge and a fuller down the middle.
    blade = poly_mask([(15, 42), (22, 49), (56, 15), (59, 5), (49, 8)])
    fuller = segment_dist(22, 42, 50, 14) < 1.0
    s.part(blade, p, base=0.6, bevel=2.2, lk=1.4, extra=np.where(fuller, -0.15, 0.0), grain=0.05 if mat == "stone" else 0)
    # Cross guard.
    guard = stroke([(12, 38), (26, 52)], 5)
    s.part(guard, p if mat != "wooden" else wood_pal(), base=0.5, bevel=2)
    return s.done()


def pickaxe(mat):
    s = Sprite(f"{mat}_pickaxe")
    handle(s, (9, 56), (45, 20))
    head = stroke([(12, 15), (24, 9), (36, 8), (46, 12), (52, 18), (56, 28), (55, 40), (51, 52)], 4.5, 7.5)
    head |= stroke([(13, 15), (24, 9), (36, 8), (46, 12)], 7.5, 7.5)
    head &= ~stroke([(9, 56), (34, 31)], 4)
    s.part(head, head_pal(mat), base=0.55, bevel=2.5, grain=0.1 if mat == "stone" else 0)
    return s.done()


def axe(mat):
    s = Sprite(f"{mat}_axe")
    handle(s, (10, 57), (46, 21))
    head = poly_mask([(26, 18), (34, 6), (50, 2), (58, 12), (56, 28), (48, 36), (40, 32), (36, 24)])
    back = stroke([(42, 22), (50, 30)], 7)
    s.part(head | back, head_pal(mat), base=0.55, bevel=2.5, grain=0.1 if mat == "stone" else 0)
    # The cutting edge is lighter.
    edge = (segment_dist(36, 7, 50, 3) < 1.8) | (segment_dist(50, 3, 57, 12) < 1.8)
    s.img[edge & (s.img[..., 3] > 0) & (head), :3] = head_pal(mat)[-2]
    return s.done()


def shovel(mat):
    s = Sprite(f"{mat}_shovel")
    handle(s, (8, 57), (38, 27))
    c, si = np.cos(np.pi / 4), np.sin(np.pi / 4)
    cx, cy = 45, 19
    pts = []
    for (u, v) in [(-7, 10), (-8, -4), (-6, -10), (0, -13), (6, -10), (8, -4), (7, 10)]:
        pts.append((cx + u * c + v * si, cy - u * si + v * c))
    spade = poly_mask(pts)
    s.part(spade, head_pal(mat), base=0.55, bevel=3, extra=(segment_dist(40, 24, 50, 14) < 1.2) * -0.12,
           grain=0.1 if mat == "stone" else 0)
    return s.done()


def hoe(mat):
    s = Sprite(f"{mat}_hoe")
    handle(s, (10, 56), (44, 22))
    head = stroke([(48, 18), (44, 12), (30, 10), (20, 12)], 8, 7)
    head |= stroke([(20, 12), (14, 18), (13, 27)], 7, 5)
    s.part(head, head_pal(mat), base=0.55, bevel=2.5, grain=0.1 if mat == "stone" else 0)
    return s.done()


def hammer(mat):
    s = Sprite(f"{mat}_hammer")
    handle(s, (8, 57), (38, 27), w=6)
    c, si = np.cos(np.pi / 4), np.sin(np.pi / 4)
    cx, cy = 41, 23
    pts = [(cx + u * c + v * si, cy - u * si + v * c) for (u, v) in [(-9, -15), (9, -15), (9, 15), (-9, 15)]]
    blk = poly_mask(pts)
    s.part(blk, head_pal(mat), base=0.5, bevel=3.5, lk=1.8)
    # Steel bands near the faces.
    for k in (-10, 10):
        band = segment_dist(cx + k * si - 9 * c, cy + k * c + 9 * si, cx + k * si + 9 * c, cy + k * c - 9 * si) < 1.2
        s.img[band & blk, :3] = head_pal(mat)[2]
    return s.done()


def excavator(mat):
    s = Sprite(f"{mat}_excavator")
    handle(s, (8, 57), (36, 29), w=6)
    c, si = np.cos(np.pi / 4), np.sin(np.pi / 4)
    cx, cy = 43, 21
    pts = [(cx + u * c + v * si, cy - u * si + v * c) for (u, v) in
           [(-11, 9), (-12, -6), (-8, -14), (0, -17), (8, -14), (12, -6), (11, 9)]]
    spade = poly_mask(pts)
    ribs = (segment_dist(36, 24, 44, 16) < 1.0) | (segment_dist(40, 28, 48, 20) < 1.0)
    s.part(spade, head_pal(mat), base=0.55, bevel=3, extra=ribs * -0.14)
    return s.done()


def sickle(mat):
    s = Sprite(f"{mat}_sickle")
    handle(s, (9, 57), (24, 42), w=6)
    ang = np.linspace(np.pi * 0.95, np.pi * 2.25, 14)
    cx, cy, r = 36, 26, 15
    pts = [(cx + np.cos(a) * r * (1 + 0.15 * np.sin(a)), cy + np.sin(a) * r) for a in ang]
    blade = stroke([(24, 42)] + pts, 6.5, 2.0)
    s.part(blade, head_pal(mat), base=0.6, bevel=2)
    return s.done()


# ---------------------------------------------------------------- armour icons


def helmet(mat):
    s = Sprite(f"{mat}_helmet")
    p = mat_pal(mat, spread=0.45)
    shell = poly_mask([(10, 34), (12, 20), (20, 12), (32, 9), (44, 12), (52, 20), (54, 34), (54, 46), (44, 46),
                       (44, 34), (20, 34), (20, 46), (10, 46)])
    s.part(shell, p, base=0.55, bevel=3, grain=0.08 if mat == "leather" else 0)
    crest = stroke([(32, 10), (32, 28)], 3)
    s.img[crest & shell, :3] = p[-2]
    brow = (YY >= 31) & (YY < 34) & shell
    s.img[brow, :3] = p[3]
    return s.done()


def chestplate(mat):
    s = Sprite(f"{mat}_chestplate")
    p = mat_pal(mat, spread=0.45)
    body = poly_mask([(6, 10), (22, 8), (27, 14), (37, 14), (42, 8), (58, 10), (58, 28), (50, 28), (50, 56),
                      (14, 56), (14, 28), (6, 28)])
    s.part(body, p, base=0.55, bevel=3, grain=0.08 if mat == "leather" else 0)
    for y in (38, 47):
        line = (np.abs(YY - y) < 1) & body & (XX > 16) & (XX < 48)
        s.img[line, :3] = p[3]
    mid = (np.abs(XX - 31.5) < 1) & body & (YY > 16)
    s.img[mid, :3] = p[3]
    return s.done()


def leggings(mat):
    s = Sprite(f"{mat}_leggings")
    p = mat_pal(mat, spread=0.45)
    legs = poly_mask([(12, 8), (52, 8), (52, 56), (38, 56), (35, 22), (29, 22), (26, 56), (12, 56)])
    s.part(legs, p, base=0.55, bevel=3, grain=0.08 if mat == "leather" else 0)
    belt = (YY >= 8) & (YY < 14) & legs
    s.img[belt, :3] = p[3]
    s.img[belt & (np.abs(XX - 31.5) < 3), :3] = p[-2]
    return s.done()


def boots(mat):
    s = Sprite(f"{mat}_boots")
    p = mat_pal(mat, spread=0.45)
    left = poly_mask([(10, 16), (26, 16), (26, 40), (30, 44), (30, 54), (4, 54), (4, 46), (10, 42)])
    right = poly_mask([(38, 16), (54, 16), (54, 42), (60, 46), (60, 54), (34, 54), (34, 44), (38, 40)])
    for m in (left, right):
        s.part(m, p, base=0.55, bevel=3, grain=0.08 if mat == "leather" else 0)
        sole = m & (YY >= 47)
        s.img[sole, :3] = p[2]
        cuff = m & (YY < 27)
        s.img[cuff, :3] = p[-3]
    return s.done()


# ---------------------------------------------------------------- materials


def ingot(mat):
    s = Sprite(f"{mat}_ingot")
    p = mat_pal(mat, spread=0.5)
    top = poly_mask([(12, 30), (40, 18), (54, 26), (26, 38)])
    front = poly_mask([(12, 30), (26, 38), (26, 48), (12, 40)])
    side = poly_mask([(26, 38), (54, 26), (54, 36), (26, 48)])
    s.part(top | front | side, p, base=0.6, bevel=1.5, lk=0.6)
    s.img[top, :3] = quantize(np.full(top.sum(), 0.82), p)
    s.img[front, :3] = quantize(np.full(front.sum(), 0.42), p)
    s.img[side, :3] = quantize(np.full(side.sum(), 0.58), p)
    glint = segment_dist(20, 30, 38, 22) < 1.0
    s.img[glint & top, :3] = p[-1]
    return s.done()


def gem(name, base):
    s = Sprite(name)
    p = ramp(base, 10, 0.6, 0.12, sat=1.15)
    outline_pts = [(32, 6), (52, 22), (32, 58), (12, 22)]
    m = poly_mask(outline_pts)
    s.part(m, p, base=0.55, bevel=2, lk=0.4, shine=False)
    # Facets: crown (top band) light, pavilion darker, split down the middle.
    crown = m & (YY < 22)
    s.img[crown & (XX < 32), :3] = p[8]
    s.img[crown & (XX >= 32), :3] = p[6]
    pav = m & (YY >= 22)
    s.img[pav & (XX < 32), :3] = p[5]
    s.img[pav & (XX >= 32), :3] = p[3]
    table = poly_mask([(26, 14), (38, 14), (42, 22), (22, 22)])
    s.img[table & m, :3] = p[9]
    girdle = (np.abs(YY - 22) < 1) & m
    s.img[girdle, :3] = p[4]
    glint = disc_mask(24, 17, 2.2) & m
    s.img[glint, :3] = 255
    return s.done()


def raw(mat):
    s = Sprite(f"raw_{mat}")
    p = mat_pal(mat, spread=0.5)
    m = np.zeros((N, N), bool)
    for (cx, cy, r) in [(26, 30, 14), (40, 26, 11), (36, 42, 12), (20, 42, 9)]:
        m |= disc_mask(cx, cy, r)
    s.part(m, p, base=0.5, bevel=4, lk=1.8, grain=0.25)
    rock = ramp("#8a8580", 8, 0.4, 0.1)
    rng = s.rng
    for _ in range(9):
        cx, cy = rng.uniform(14, 48, 2)
        d = disc_mask(cx, cy, rng.uniform(2, 3.5)) & m
        s.img[d, :3] = rock[rng.integers(2, 6)]
    return s.done()


def stick():
    s = Sprite("stick")
    m = stroke([(14, 54), (50, 12)], 6)
    s.part(m, wood_pal(), base=0.55, bevel=2.5, grain=0.2)
    knot = disc_mask(34, 31, 1.6)
    s.img[knot & m, :3] = wood_pal()[2]
    return s.done()


REGISTRY = {"stick": stick}
TOOLS = {"sword": sword, "pickaxe": pickaxe, "axe": axe, "shovel": shovel, "hoe": hoe,
         "hammer": hammer, "excavator": excavator, "sickle": sickle}
ARMOR = {"helmet": helmet, "chestplate": chestplate, "leggings": leggings, "boots": boots}
for _m in MATERIALS:
    for _t, _f in TOOLS.items():
        REGISTRY[f"{_m}_{_t}"] = lambda m=_m, f=_f: f(m)
    for _t, _f in ARMOR.items():
        REGISTRY[f"{_m}_{_t}"] = lambda m=_m, f=_f: f(m)
    REGISTRY[f"{_m}_ingot"] = lambda m=_m: ingot(m)
    REGISTRY[f"raw_{_m}"] = lambda m=_m: raw(m)
REGISTRY["gold_ingot"] = lambda: ingot("golden")
REGISTRY["raw_gold"] = lambda: raw("golden")
for _g, _c in GEMS.items():
    REGISTRY[_g] = lambda g=_g, c=_c: gem(g, c)

# Swords and armour have a design of their own per material (arms.py), not one recoloured drawing.
import arms  # noqa: E402

for _m in arms.SWORDS:
    REGISTRY[f"{_m}_sword"] = lambda m=_m: arms.sword(m)
for _m in arms.STYLE:
    for _t, _f in arms.ARMOR_ICONS.items():
        REGISTRY[f"{_m}_{_t}"] = lambda m=_m, f=_f: f(m)

import beds as _beds  # noqa: E402  (each bed's icon is its 3D model)
REGISTRY.update(_beds.ITEMS)

# Pickaxes, axes, shovels and hoes too: a design per material (toolart.py, HP 2026-10-29).
import toolart  # noqa: E402

for _k, _d in toolart.KINDS.items():
    for _m in _d:
        REGISTRY[f"{_m}_{_k}"] = lambda m=_m, k=_k: toolart.tool(k, m)
REGISTRY.update(toolart.EXTRA_ITEMS)
