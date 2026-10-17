#!/usr/bin/env python3
"""Creatures of the mod content pack: our own box models, stats and colouring for the animals
and monsters inspired by Alex's Mobs, Naturalist, Mutant Monsters and Enderman Overhaul (ideas
only: every model, number and skin here is our own).

Running it rewrites the part of packs/vanilla/mods/mobs.ron between the two marker comments
(the rest of the file is left alone, so other mods' mobs can live there too):

    python3 tools/mod_creatures.py
    python3 tools/gen_mod_mob_skins.py      # skins + spawn egg sprites
    python3 tools/gen_mod_data.py           # spawn egg items (tools/moddata/zz_mobs.py)

Models are built from a few body plans (four-legged, two-legged, bird, fish, snake, insect)
with the skin unwrap packed automatically into the 64x64 top half of the 64x128 skin.
"""

import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MODS_MOBS = os.path.join(ROOT, "packs", "vanilla", "mods", "mobs.ron")
BEGIN = "    // ---- creatures (made by tools/mod_creatures.py: edit it and run it again) ----\n"
END = "    // ---- end of creatures ----\n"


# --------------------------------------------------------------------------------------------
# Models
# --------------------------------------------------------------------------------------------

class Model:
    def __init__(self):
        self.parts = []  # dicts: role, size, origin, anim, pivot, share
        self.layers = []  # (part index, inflate)

    def box(self, role, size, origin, anim=None, pivot=None, share=None):
        size = tuple(int(v) for v in size)
        assert all(v >= 1 for v in size), (role, size)
        self.parts.append(dict(role=role, size=size, origin=tuple(int(v) for v in origin), anim=anim,
                               pivot=tuple(int(round(v)) for v in pivot) if pivot else None, share=share))
        return len(self.parts) - 1

    def attach(self, specs, ox, oy, oz, ow, anim=None, pivot=None):
        """Parts placed relative to a parent box at (ox, oy, oz) of width ow. Each spec is
        (role, size, (dx, dy, dz)[, mirror[, anim]]). Not mirrored: centred on the parent plus dx.
        Mirrored: a left copy dx in from the parent's left side and a right copy to match."""
        for s in specs:
            role, size, (dx, dy, dz) = s[0], s[1], s[2]
            mirror = len(s) > 3 and s[3]
            a = s[4] if len(s) > 4 else anim
            w = size[0]
            if mirror:
                key = f"{role}{size}"
                self.box(role, size, (ox + dx, oy + dy, oz + dz), a, pivot, share=key)
                self.box(role, size, (ox + ow - dx - w, oy + dy, oz + dz), a, pivot, share=key)
            else:
                self.box(role, size, (ox + ow // 2 - w // 2 + dx, oy + dy, oz + dz), a, pivot)

    def layer(self, role_or_idx, inflate):
        idxs = [role_or_idx] if isinstance(role_or_idx, int) else [i for i, p in enumerate(self.parts) if p["role"] == role_or_idx]
        assert idxs, role_or_idx
        for i in idxs:
            # parts sharing a skin share their relief too: one entry each is fine
            self.layers.append((i, inflate))
        return self

    def pack(self, name):
        """Give every box (or group of boxes sharing a skin) its own place in the 64x64 net."""
        groups = {}
        for i, p in enumerate(self.parts):
            key = p["share"] or f"#{i}"
            if key in groups:
                assert self.parts[groups[key][0]]["size"] == p["size"], (name, key)
            groups.setdefault(key, []).append(i)
        rects = []
        for key, idxs in groups.items():
            w, h, d = self.parts[idxs[0]]["size"]
            rects.append((d + h, 2 * (w + d), key))
        rects.sort(key=lambda r: (-r[0], -r[1]))
        shelves = []  # [y, height, x used]
        for h, w, key in rects:
            assert w <= 64, f"{name}: box {key} too wide for the skin ({w})"
            for s in shelves:
                if s[1] >= h and s[2] + w <= 64:
                    break
            else:
                y = shelves[-1][0] + shelves[-1][1] if shelves else 0
                assert y + h <= 64, f"{name}: parts don't fit in the skin"
                s = [y, h, 0]
                shelves.append(s)
            for i in groups[key]:
                self.parts[i]["uv"] = (s[2], s[0])
            s[2] += w
        return self

    def extent(self):
        lo = [min(p["origin"][a] for p in self.parts) for a in range(3)]
        hi = [max(p["origin"][a] + p["size"][a] for p in self.parts) for a in range(3)]
        return lo, hi


def legs4(m, bx0, bw, bz0, bd, leg, inset=0, splay=0, fz=0, bz=0, y=0):
    lw, lh, ld = leg
    xl = bx0 + inset - splay
    xr = bx0 + bw - inset - lw + splay
    zf = bz0 + bd - ld - 1 - fz
    zb = bz0 + 1 + bz
    for x, z, k in ((xl, zf, 0), (xr, zf, 1), (xl, zb, 1), (xr, zb, 0)):
        m.box("leg", leg, (x, y, z), f"Leg({k})", (x + lw / 2, y + lh, z + ld / 2), share="leg")


def quad(body, leg, head, *, neck=None, head_dy=0, head_dz=0, hp=(), bp=(), inset=0, splay=0, fz=0, bz=0,
         body_dy=0, neck_dz=0):
    """Four legs. `hp` are parts on the head (moving with it), `bp` parts on the body."""
    m = Model()
    bw, bh, bd = body
    lw, lh, ld = leg
    by = lh + body_dy
    bx0, bz0 = -(bw // 2), -(bd // 2)
    front = bz0 + bd
    m.box("body", body, (bx0, by, bz0))
    hw, hh, hd = head
    if neck:
        nw, nh, nd = neck
        ny, nz = by + bh - 3, front - nd - 1 + neck_dz
        pivot = (0, ny + 2, nz + nd / 2)
        m.box("neck", neck, (-(nw // 2), ny, nz), "Head", pivot)
        hy, hz = ny + nh - 2 + head_dy, nz - 1 + head_dz
    else:
        hy, hz = by + bh - hh + 1 + head_dy, front - 2 + head_dz
        pivot = (0, hy + hh / 2, hz + 1)
    m.box("head", head, (-(hw // 2), hy, hz), "Head", pivot)
    m.attach(hp, -(hw // 2), hy, hz, hw, "Head", pivot)
    m.attach(bp, bx0, by, bz0, bw)
    legs4(m, bx0, bw, bz0, bd, leg, inset, splay, fz, bz)
    return m


def biped(head, torso, arm, leg, *, arm_anim="Arm", hp=(), bp=(), head_dz=0, head_dy=0, legsep=0, arm_dy=0,
          torso_dz=0, arm_out=0):
    """Two legs and two arms, the parts in the same order as the base humanoids."""
    m = Model()
    hw, hh, hd = head
    tw, th, td = torso
    aw, ah, ad = arm
    lw, lh, ld = leg
    top = lh + th
    hpivot = (0, top + head_dy, torso_dz)
    hx, hy, hz = -(hw // 2), top + head_dy, -(hd // 2) + head_dz + torso_dz
    m.box("head", head, (hx, hy, hz), "Head", hpivot)
    m.box("body", torso, (-(tw // 2), lh, -(td // 2) + torso_dz))
    ay = top - ah + arm_dy
    az = -(ad // 2) + torso_dz
    for k, x in enumerate((-(tw // 2) - aw - arm_out, tw - tw // 2 + arm_out)):
        m.box("arm", arm, (x, ay, az), f"{arm_anim}({k})", (x + aw / 2, top - 2 + arm_dy, torso_dz), share="arm")
    for k, x in enumerate((-lw - legsep, legsep)):
        m.box("leg", leg, (x, 0, -(ld // 2)), f"Leg({k})", (x + lw / 2, lh, 0), share="leg")
    m.attach(hp, hx, hy, hz, hw, "Head", hpivot)
    m.attach(bp, -(tw // 2), lh, -(td // 2) + torso_dz, tw)
    return m


def bird(body, leg, head, beak, *, wing=None, tail=None, neck=None, head_dy=0, head_dz=0, hp=(), bp=(), legsep=1,
         wing_dy=0):
    """`beak` is (size, dy); `tail` is (size, dy from the top of the body)."""
    m = Model()
    bw, bh, bd = body
    lw, lh, ld = leg
    by, bz0 = lh, -(bd // 2)
    front = bz0 + bd
    m.box("body", body, (-(bw // 2), by, bz0))
    hw, hh, hd = head
    if neck:
        nw, nh, nd = neck
        ny, nz = by + bh - 2, front - nd
        pivot = (0, ny + 1, nz + nd / 2)
        m.box("neck", neck, (-(nw // 2), ny, nz), "Head", pivot)
        hy, hz = ny + nh - 1 + head_dy, nz - 1 + head_dz
    else:
        hy, hz = by + bh - 2 + head_dy, front - 2 + head_dz
        pivot = (0, hy + 1, hz + hd / 2)
    m.box("head", head, (-(hw // 2), hy, hz), "Head", pivot)
    (kw, kh, kd), kdy = beak
    m.box("beak", (kw, kh, kd), (-(kw // 2), hy + kdy, hz + hd), "Head", pivot)
    m.attach(hp, -(hw // 2), hy, hz, hw, "Head", pivot)
    if wing:
        ww, wh, wd = wing
        y = by + bh - wh + wing_dy
        for k, x in enumerate((-(bw // 2) - ww, bw - bw // 2)):
            px = x + ww if k == 0 else x
            m.box("wing", wing, (x, y, bz0 + 1), f"Wing({k})", (px, by + bh + wing_dy, bz0 + 1 + wd / 2), share="wing")
    if tail:
        (tw, th, td), tdy = tail
        m.box("tail", (tw, th, td), (-(tw // 2), by + bh - th - tdy, bz0 - td))
    m.attach(bp, -(bw // 2), by, bz0, bw)
    for k, x in enumerate((-lw - legsep + 1, legsep - 1 + (1 if lw == 1 else 0))):
        m.box("leg", leg, (x, 0, -(ld // 2)), f"Leg({k})", (x + lw / 2, lh, 0), share="leg")
    return m


def flier(body, wing, head, *, y=4, hp=(), bp=(), abdomen=None, legs=None):
    """Small flying critters: flat wings over the back that flap in the air."""
    m = Model()
    bw, bh, bd = body
    bz0 = -(bd // 2)
    m.box("body", body, (-(bw // 2), y, bz0))
    hw, hh, hd = head
    hy, hz = y + max(0, (bh - hh) // 2), bz0 + bd
    m.box("head", head, (-(hw // 2), hy, hz), "Head", (0, hy + hh / 2, hz))
    m.attach(hp, -(hw // 2), hy, hz, hw, "Head", (0, hy + hh / 2, hz))
    if abdomen:
        aw, ah, ad = abdomen
        m.box("abdomen", abdomen, (-(aw // 2), y + max(0, (bh - ah) // 2), bz0 - ad))
    ww, wh, wd = wing
    for k, x in enumerate((-(bw // 2) - ww + 1, bw - bw // 2 - 1)):
        px = x + ww - 1 if k == 0 else x + 1
        m.box("wing", wing, (x, y + bh - 1, bz0 + (bd - wd) // 2), f"Wing({k})", (px, y + bh, bz0 + bd / 2), share="wing")
    m.attach(bp, -(bw // 2), y, bz0, bw)
    if legs:
        lw, lh, ld = legs
        for k, x in enumerate((-(bw // 2), bw - bw // 2 - lw)):
            m.box("leg", legs, (x, y - lh, -(ld // 2)), f"Leg({k})", (x + lw / 2, y, 0), share="leg")
    return m


def fish(body, *, head=None, tail=(1, 4, 4), flukes=False, dorsal=None, fins=None, y=1, hp=(), bp=()):
    m = Model()
    bw, bh, bd = body
    bz0 = -(bd // 2)
    front = bz0 + bd
    m.box("body", body, (-(bw // 2), y, bz0))
    if head:
        (hw, hh, hd), hdy = head
        m.box("head", (hw, hh, hd), (-(hw // 2), y + hdy, front))
        m.attach(hp, -(hw // 2), y + hdy, front, hw)
    tw, th, td = tail
    ty = y + (bh - th) // 2 if not flukes else y + bh // 2
    m.box("tail", tail, (-(tw // 2), ty, bz0 - td))
    if dorsal:
        (dw, dh, dd), dz = dorsal
        m.box("fin", (dw, dh, dd), (-(dw // 2), y + bh, bz0 + dz))
    if fins:
        (fw, fh, fd), fdz = fins
        m.attach([("fin", (fw, fh, fd), (-fw, 1, fdz), True)], -(bw // 2), y, bz0, bw)
    m.attach(bp, -(bw // 2), y, bz0, bw)
    return m


def serpent(head, segs, *, wave=(0, 1, 2, 1, 0, -1, -2, -1, 0, 1, 2, 1), head_dy=0, hp=(), y=0):
    m = Model()
    hw, hh, hd = head
    length = hd + sum(s[2] for s in segs)
    z = length // 2 - hd
    hy = y + head_dy
    pivot = (0, hy + hh / 2, z)
    m.box("head", head, (-(hw // 2), hy, z), "Head", pivot)
    m.attach(hp, -(hw // 2), hy, z, hw, "Head", pivot)
    for k, (sw, sh, sd) in enumerate(segs):
        z -= sd
        m.box("seg", (sw, sh, sd), (-(sw // 2) + wave[k % len(wave)], y, z), share=f"seg{(sw, sh, sd)}")
    return m


def insect(thorax, head, abdomen=None, *, legs=3, leg_len=6, wing=None, hp=(), bp=(), abd_dy=0, y=None,
           head_dy=0):
    m = Model()
    tw, th, td = thorax
    ly = y if y is not None else max(1, round(0.45 * leg_len))
    tz0 = -(td // 2)
    m.box("body", thorax, (-(tw // 2), ly, tz0))
    hw, hh, hd = head
    hy, hz = ly + head_dy, tz0 + td
    pivot = (0, hy + hh / 2, hz)
    m.box("head", head, (-(hw // 2), hy, hz), "Head", pivot)
    m.attach(hp, -(hw // 2), hy, hz, hw, "Head", pivot)
    if abdomen:
        aw, ah, ad = abdomen
        m.box("abdomen", abdomen, (-(aw // 2), ly + abd_dy, tz0 - ad))
    for k in range(legs):
        z = tz0 + (td - 1) * k // max(1, legs - 1) if legs > 1 else tz0 + td // 2
        m.box("leg", (leg_len, 1, 1), (-(tw // 2) - leg_len, ly, z), f"SpiderLeg({k})", (-(tw // 2), ly + 1, z + 0.5), share="leg")
        m.box("leg", (leg_len, 1, 1), (tw - tw // 2, ly, z), f"SpiderLeg({4 + k})", (tw - tw // 2, ly + 1, z + 0.5), share="leg")
    if wing:
        ww, wh, wd = wing
        for k, x in enumerate((-ww, 0)):
            px = 0
            m.box("wing", wing, (x, ly + th, tz0 + td - wd), f"Wing({k})", (px, ly + th, tz0 + td - wd / 2), share="wing")
    m.attach(bp, -(tw // 2), ly, tz0, tw)
    return m


def walker_variant():
    """The void walker's model (same as packs/vanilla/mobs.ron)."""
    m = Model()
    m.box("head", (8, 8, 8), (-4, 24, -4), "Head", (0, 24, 0))
    m.box("body", (8, 12, 4), (-4, 12, -2))
    m.box("arm", (2, 12, 2), (-6, 12, -1), "ArmForward(0)", (-5, 22, 0), share="arm")
    m.box("arm", (2, 12, 2), (4, 12, -1), "Arm(1)", (5, 22, 0), share="arm")
    m.box("leg", (2, 12, 2), (-3, 0, -1), "Leg(0)", (-2, 12, 0), share="leg")
    m.box("leg", (2, 12, 2), (1, 0, -1), "Leg(1)", (2, 12, 0), share="leg")
    for p, uv in zip(m.parts, [(0, 0), (16, 16), (40, 16), (40, 16), (0, 16), (0, 16)]):
        p["uv"] = uv
    m.fixed_uv = True
    m.layer(1, 0.5)
    return m


# --------------------------------------------------------------------------------------------
# Creatures
# --------------------------------------------------------------------------------------------

GROUND = {
    "Desert": ["sand"], "Beach": ["sand"], "Badlands": ["red_sand", "sand", "terracotta"],
    "Mountains": ["grass_block", "stone", "snowy_grass_block"],
    "Snowy Plains": ["snowy_grass_block", "snow_layer", "snow"], "Taiga": ["snowy_grass_block", "snow_layer", "snow"],
    "Nether Wastes": ["netherrack"], "Soul Sand Valley": ["soul_sand", "soul_soil"],
    "Crimson Forest": ["crimson_nylium"], "Warped Forest": ["warped_nylium"], "The End": ["end_stone"],
}

CREATURES = []


def mob(name, display, kind, health, speed, model, look, *, spawn="Grass", biomes=(), drops=(), xp=1, attack=0,
        group=(1, 2), weight=4, scale=1.0, flee=0.0, prey=(), mod="alex", egg=None, ground=None, **flags):
    """One creature. `look` is how the skin is painted (see tools/gen_mod_mob_skins.py)."""
    if not getattr(model, "fixed_uv", False):
        model.pack(name)
    lo, hi = model.extent()
    k = scale / 16.0
    width = round(max(0.3, min(2.8, min(hi[0] - lo[0], hi[2] - lo[2]) * k * 0.95)), 2)
    height = round(max(0.25, hi[1] * k), 2)
    if ground is None and spawn == "Grass":
        g = []
        for b in biomes:
            for n in GROUND.get(b, ["grass_block"]):
                if n not in g:
                    g.append(n)
        ground = g if g != ["grass_block"] else []
    CREATURES.append(dict(name=name, display=display, kind=kind, health=health, speed=speed, model=model, look=look,
                          spawn=spawn, biomes=list(biomes), drops=list(drops), xp=xp, attack=attack, group=group,
                          weight=weight, scale=scale, flee=flee, prey=list(prey), mod=mod, egg=egg,
                          ground=ground or [], width=width, height=height, flags=flags))


def L(base, belly=None, **kw):
    """A look: palettes are lists of hex colours, dark to light."""
    d = dict(base=base, belly=belly)
    d.update(kw)
    return d


# Palettes used more than once.
BROWN = ["#3a2618", "#4a3020", "#5a3c28", "#6a4830", "#7a5638"]
GREY = ["#4a4a4e", "#58585c", "#66666a", "#747478", "#828286"]
HORN = ["#8a7e66", "#a89c80", "#c6ba9a", "#e0d6b8"]
TUSK = ["#c8c0a8", "#dcd4bc", "#ece6d2"]
BLACK = ["#141416", "#1c1c20", "#24242a", "#2c2c32"]
WHITE = ["#c8c6c0", "#d8d6d0", "#e6e4de", "#f2f0ec"]
PINK = ["#a8606a", "#c07880", "#d49098"]
ORANGE_BEAK = ["#b8641a", "#d07a22", "#e8922c"]
YELLOW_BEAK = ["#b88a14", "#d0a41c", "#e8c030"]
GREEN_SCALE = ["#24361c", "#2e4424", "#3a522c", "#466234", "#52703c"]

# ---- Alex's Mobs ideas ---------------------------------------------------------------------

mob("gorilla", "Gorila", "Neutral", 30, 2.6,
    biped((8, 8, 7), (14, 12, 8), (5, 16, 5), (5, 7, 5), head_dz=4, head_dy=-2, legsep=1, arm_dy=-1,
          hp=[("brow", (6, 1, 1), (0, 5, 7)), ("snout", (4, 3, 1), (0, 1, 7))]).layer("body", 0.5).layer("head", 0.4),
    L(["#18181a", "#202024", "#28282e", "#323238", "#3c3c42"], skin=["#4a4446", "#5a5254", "#6a6264"],
      eyes="front", eye=(110, 70, 30), overlay="fur", silverback=True),
    biomes=["Jungle"], drops=[("leather", 0, 2), ("apple", 0, 1)], xp=5, attack=6, group=(1, 3), weight=3, scale=1.1)

mob("crocodile", "Crocodilo", "Neutral", 30, 2.2,
    quad((10, 6, 22), (3, 3, 3), (8, 5, 6), head_dy=-1, splay=2, inset=0,
         hp=[("snout", (6, 3, 8), (0, 0, 6)), ("tooth", (1, 1, 6), (1, -1, 7), True), ("eyebump", (2, 2, 2), (1, 5, 2), True)],
         bp=[("tail", (6, 4, 12), (0, 1, -12)), ("tail", (2, 3, 8), (0, 1, -20))]).layer("body", 0.4).layer(6, 0.4),
    L(["#1e2a14", "#26361a", "#304220", "#3a5028", "#465e30"], belly=["#8a8a5a", "#9e9e6a", "#b2b07c"],
      eyes="side", eye=(200, 170, 40), overlay="scutes", tooth=TUSK),
    biomes=["Swamp", "Jungle"], drops=[("crocodile_scute", 0, 1), ("leather", 0, 2)], xp=6, attack=7, weight=2,
    scale=1.3, prey=["duck", "chicken", "rabbit"], swims=True)

mob("kangaroo", "Canguru", "Neutral", 20, 3.6,
    biped((5, 6, 7), (8, 11, 8), (2, 6, 2), (3, 6, 8), arm_anim="ArmForward", head_dz=1, legsep=1, arm_dy=-1,
          torso_dz=0,
          hp=[("ear", (2, 4, 1), (0, 6, 2), True), ("snout", (3, 3, 2), (0, 0, 7))],
          bp=[("tail", (3, 3, 12), (0, 0, -12)), ("pouch", (6, 4, 1), (0, 1, 8))]).layer("body", 0.3),
    L(["#7a4a2a", "#8c5832", "#9e663c", "#ae7446", "#bc8250"], belly=["#c8a07a", "#d6b08a", "#e2c09c"],
      eyes="side", ear_inner=PINK),
    biomes=["Savanna", "Badlands"], drops=[("leather", 0, 2), ("beef", 1, 2)], xp=3, attack=5, group=(2, 4), weight=3,
    hops=True)

mob("emu", "Emu", "Passive", 12, 4.8,
    bird((7, 7, 10), (2, 11, 2), (3, 3, 4), ((2, 1, 3), 0), neck=(2, 8, 2), tail=((6, 4, 3), 1), legsep=1,
         wing=(1, 4, 6)).layer("body", 0.6),
    L(["#3a2e24", "#4a3c30", "#5a4a3c", "#6a5848", "#7a6656"], skin=["#5a7a9a", "#6a8aaa", "#7a9aba"],
      eyes="side", eye=(200, 90, 30), beak=BLACK, legs=["#5a5650", "#6a6660"], overlay="shaggy"),
    biomes=["Savanna", "Badlands", "Desert"], drops=[("feather", 1, 3), ("chicken", 1, 2)], xp=2, flee=9.0,
    group=(2, 3), weight=3)

mob("komodo_dragon", "Dragão-de-Komodo", "Hostile", 30, 2.4,
    quad((8, 5, 18), (3, 3, 3), (6, 4, 6), head_dy=-1, splay=2,
         hp=[("snout", (4, 3, 5), (0, 0, 6)), ("tongue", (1, 1, 3), (0, 0, 11))],
         bp=[("tail", (4, 3, 10), (0, 1, -10)), ("tail", (2, 2, 8), (0, 1, -18))]),
    L(["#3a3428", "#4a4232", "#5a503c", "#6a5e46", "#786a50"], belly=["#7a7058", "#8a8066", "#9a9076"],
      pattern=("spots", ["#2a2620", "#34302a"], 2.5, 0.62), eyes="side", eye=(180, 140, 40), tongue=["#c8402a", "#e05030"]),
    biomes=["Savanna", "Badlands", "Jungle"], drops=[("leather", 0, 2), ("rotten_flesh", 0, 2)], xp=6, attack=5,
    weight=2, scale=1.2, prey=["chicken", "rabbit", "pig"])

mob("capuchin_monkey", "Macaco-Prego", "Passive", 8, 3.6,
    quad((5, 5, 8), (2, 5, 2), (5, 5, 4), head_dy=2, head_dz=0,
         hp=[("snout", (3, 2, 1), (0, 0, 4)), ("ear", (1, 2, 1), (-1, 2, 1), True)],
         bp=[("tail", (1, 1, 10), (0, 4, -10))]),
    L(["#2a1c12", "#34221a", "#3e2a20", "#4a3226", "#563a2c"], skin=["#c8a080", "#d8b090", "#e6c0a0"],
      eyes="front", face_mask=True),
    biomes=["Jungle"], drops=[], xp=1, flee=6.0, group=(2, 4), weight=3, scale=0.8, climbs=True)

mob("raccoon", "Guaxinim", "Passive", 10, 3.0,
    quad((7, 6, 10), (2, 4, 2), (6, 5, 5), head_dy=0,
         hp=[("snout", (2, 2, 2), (0, 0, 5)), ("ear", (2, 2, 1), (0, 5, 2), True)],
         bp=[("tail", (3, 3, 8), (0, 2, -8))]).layer("body", 0.3),
    L(["#3a3a3c", "#4a4a4c", "#5a5a5c", "#6a6a6c", "#7a7a7c"], belly=["#8a8a8a", "#9a9a9a"],
      eyes="front", mask=True, tail_rings=True, overlay="fur"),
    biomes=["Forest", "Birch Forest", "Dark Forest"], drops=[("leather", 0, 1)], xp=1, flee=6.0, weight=3,
    sleeps=True, prey=["chicken"], attack=2)

mob("skunk", "Gambá-Listrado", "Neutral", 8, 2.6,
    quad((6, 5, 9), (2, 3, 2), (5, 4, 4), head_dy=0,
         hp=[("snout", (2, 2, 2), (0, 0, 4)), ("ear", (1, 1, 1), (0, 4, 1), True)],
         bp=[("tail", (4, 7, 4), (0, 4, -3))]).layer("tail", 0.6),
    L(BLACK, eyes="front", skunk=True, overlay="fluff_white"),
    biomes=["Forest", "Plains", "Birch Forest"], drops=[("leather", 0, 1)], xp=1, attack=3, weight=2, flee=0.0)

mob("gazelle", "Gazela", "Passive", 10, 5.2,
    quad((6, 6, 11), (2, 9, 2), (4, 4, 6), neck=(3, 6, 3), head_dy=0,
         hp=[("ear", (2, 1, 1), (-1, 3, 1), True), ("horn", (1, 6, 1), (1, 4, 1), True)],
         bp=[("tail", (2, 3, 1), (0, 3, -1))]),
    L(["#8a5a2a", "#9c6a34", "#ae7a3e", "#be8a4a", "#cc9856"], belly=WHITE, eyes="side", horn=BLACK,
      flank_stripe=True),
    biomes=["Savanna"], drops=[("venison", 1, 2), ("leather", 0, 1)], xp=2, flee=12.0, group=(3, 5), weight=4)

mob("bison", "Bisão", "Neutral", 40, 2.4,
    quad((14, 12, 18), (4, 9, 4), (8, 8, 6), head_dy=-4, head_dz=0,
         hp=[("horn", (2, 3, 2), (-2, 6, 2), True), ("snout", (6, 3, 1), (0, 0, 6))],
         bp=[("hump", (12, 5, 9), (0, 10, 7)), ("tail", (1, 6, 1), (0, 5, -1))]).layer(0, 0.5).layer("hump", 0.8).layer("head", 0.6),
    L(["#2a1c12", "#34221a", "#3e2a1e", "#4a3424", "#563e2c"], eyes="side", horn=["#1c1a18", "#2c2824", "#3c3630"],
      overlay="mane", mane=["#1e140c", "#2a1c12", "#382616"]),
    biomes=["Plains", "Savanna"], drops=[("beef", 2, 4), ("leather", 1, 3)], xp=5, attack=7, group=(2, 4), weight=3,
    scale=1.25)

mob("moose", "Alce", "Neutral", 36, 2.6,
    quad((10, 10, 18), (3, 14, 3), (6, 6, 9), head_dy=-2, head_dz=0, neck=None,
         hp=[("antler", (6, 1, 4), (-5, 6, 2), True), ("antler", (1, 3, 1), (-4, 7, 2), True),
             ("beard", (2, 4, 1), (0, -4, 5)), ("ear", (2, 2, 1), (-1, 5, 1), True)],
         bp=[("hump", (8, 3, 6), (0, 10, 10))]).layer(0, 0.4),
    L(["#2a1c14", "#34241a", "#3e2c20", "#4a3628", "#564030"], eyes="side", antler=["#8a7a5a", "#a69676", "#c2b292"],
      legs=["#3e3228", "#4a3c30", "#564638"]),
    biomes=["Taiga", "Snowy Plains"], drops=[("venison", 2, 4), ("leather", 1, 2)], xp=5, attack=7, group=(1, 2),
    weight=2, scale=1.35)

mob("grizzly_bear", "Urso-Pardo", "Neutral", 36, 2.7,
    quad((12, 11, 20), (4, 9, 4), (8, 8, 7), head_dy=-1,
         hp=[("snout", (4, 3, 3), (0, 0, 7)), ("ear", (2, 2, 1), (0, 8, 3), True)],
         bp=[("hump", (10, 4, 6), (0, 10, 12))]).layer(0, 0.7).layer("head", 0.5).layer("hump", 0.8),
    L(["#4a2e18", "#5a3820", "#6a4428", "#7a5032", "#8a5e3c"], eyes="front", snout=["#9a7a56", "#ae8e68", "#c0a07a"],
      overlay="fur", claws=True),
    biomes=["Mountains", "Taiga", "Forest"], drops=[("beef", 1, 3), ("leather", 1, 2), ("honeycomb", 0, 1)], xp=6,
    attack=7, group=(1, 1), weight=2, scale=1.1, sleeps=True)

mob("crow", "Corvo", "Passive", 4, 2.6,
    bird((4, 4, 6), (1, 2, 1), (3, 3, 3), ((1, 1, 2), 0), wing=(1, 3, 5), tail=((2, 1, 3), 0)),
    L(BLACK, eyes="side", eye=(30, 30, 30), eye_ring=(80, 80, 90), beak=["#101012", "#1a1a1e", "#26262a"],
      legs=["#18181a", "#222226"], sheen=(40, 50, 90)),
    biomes=["Plains", "Forest", "Dark Forest", "Taiga"], drops=[("feather", 0, 2)], xp=1, flee=7.0, group=(1, 3),
    weight=2, scale=0.8, flies=True)

mob("seagull", "Gaivota", "Passive", 4, 2.6,
    bird((4, 4, 7), (1, 2, 1), (3, 3, 3), ((1, 1, 2), 0), wing=(1, 3, 6), tail=((2, 1, 2), 0)),
    L(WHITE, eyes="side", beak=YELLOW_BEAK, beak_tip=(200, 40, 30), wing_col=["#7a7e86", "#8a8e96", "#9aa0a8"],
      wing_tip=BLACK, legs=PINK),
    biomes=["Beach", "Ocean"], ground=["sand", "grass_block"], drops=[("feather", 0, 2)], xp=1, flee=6.0,
    group=(2, 4), weight=4, scale=0.9, flies=True)

mob("hummingbird", "Beija-Flor", "Passive", 2, 3.0,
    bird((2, 2, 3), (1, 1, 1), (2, 2, 2), ((1, 1, 3), 1), wing=(1, 2, 4), tail=((2, 1, 2), 0), legsep=1),
    L(["#1a5a3a", "#226a44", "#2a7a4e", "#348a58", "#3e9a62"], belly=["#c8c8c0", "#d8d8d0"], eyes="side",
      beak=BLACK, throat=(200, 30, 60), wing_col=["#2a3a3a", "#344646"]),
    biomes=["Jungle", "Cherry Grove", "Plains"], drops=[("feather", 0, 1)], xp=1, flee=5.0, group=(1, 2), weight=2,
    scale=0.5, flies=True)

mob("toucan", "Tucano", "Passive", 4, 2.4,
    bird((4, 5, 6), (1, 2, 1), (4, 4, 4), ((2, 3, 6), 0), wing=(1, 4, 5), tail=((2, 1, 4), 1)),
    L(BLACK, eyes="side", eye_ring=(80, 160, 220), throat=(240, 220, 120), beak=["#d06010", "#e88018", "#f8a020"],
      beak_tip=(30, 20, 20), toucan=True),
    biomes=["Jungle"], drops=[("feather", 0, 2)], xp=1, flee=6.0, group=(1, 2), weight=3, scale=0.8, flies=True)

mob("flamingo", "Flamingo", "Passive", 8, 2.4,
    bird((5, 5, 8), (1, 12, 1), (3, 3, 4), ((2, 2, 3), -1), neck=(2, 9, 2), wing=(1, 4, 6), tail=((3, 1, 2), 0),
         legsep=1),
    L(["#c8506a", "#d8607a", "#e8768c", "#f08ca0", "#f8a2b2"], eyes="side", eye=(220, 200, 40),
      beak=["#1a1a1a", "#262626", "#e8d0c8"], legs=["#c06070", "#d07080"], wing_tip=BLACK),
    biomes=["Swamp", "Beach"], ground=["grass_block", "sand"], drops=[("feather", 1, 2)], xp=1, flee=6.0,
    group=(2, 5), weight=4)

mob("giant_catfish", "Bagre Gigante", "Passive", 20, 2.0,
    fish((8, 6, 16), head=((8, 5, 5), 0), tail=(1, 5, 6), dorsal=((1, 3, 4), 6), fins=((3, 1, 3), 10),
         hp=[("whisker", (1, 1, 6), (-1, 0, 2), True)]),
    L(["#3a3a2a", "#4a4834", "#5a5640", "#6a644a", "#787254"], belly=["#a8a080", "#bab290"], eyes="side",
      eye=(220, 200, 120), pattern=("spots", ["#2a2a20", "#323226"], 2.0, 0.6), whisker=BLACK),
    spawn="Water", biomes=["Swamp", "Jungle"], drops=[("catfish", 2, 4)], xp=4, group=(1, 1), weight=2, scale=1.5,
    aquatic=True)

mob("orca", "Orca", "Neutral", 50, 3.0,
    fish((12, 10, 20), head=((10, 8, 6), 0), tail=(12, 1, 6), flukes=True, dorsal=((2, 8, 4), 9),
         fins=((5, 1, 4), 12)),
    L(BLACK, belly=WHITE, eyes="none", orca=True),
    spawn="Water", biomes=["Ocean"], drops=[("cod", 2, 5), ("salmon", 1, 3)], xp=6, attack=8, group=(1, 3),
    weight=2, scale=1.6, aquatic=True, prey=["seal"])

mob("cachalot_whale", "Cachalote", "Passive", 120, 2.0,
    fish((10, 10, 18), head=((10, 10, 8), 0), tail=(14, 1, 5), flukes=True, dorsal=((2, 2, 4), 6),
         fins=((5, 1, 4), 14), hp=[("jaw", (4, 2, 8), (0, -1, -8))]),
    L(["#3a3c42", "#44464c", "#4e5058", "#585a62", "#62646c"], belly=["#6a6c74", "#7a7c84"], eyes="side",
      eye=(20, 20, 20), scars=True),
    spawn="Water", biomes=["Ocean"], drops=[("cod", 3, 6), ("bone", 2, 5)], xp=12, group=(1, 1), weight=1, scale=3.6,
    aquatic=True)

mob("mantis_shrimp", "Tamarutaca", "Neutral", 12, 2.6,
    insect((5, 4, 6), (4, 4, 3), (4, 3, 8), legs=3, leg_len=4,
           hp=[("eyestalk", (1, 3, 1), (0, 4, 1), True), ("claw", (1, 2, 5), (-1, 0, 2), True)],
           bp=[("tail", (6, 1, 3), (0, 0, -11))]),
    L(["#1a6a5a", "#22806a", "#2a967a", "#34aa8a", "#40bc98"], eyes="stalk", rainbow=True,
      claw_col=["#d03030", "#e85040", "#f87050"]),
    biomes=["Beach"], drops=[("cod", 0, 1)], xp=3, attack=6, group=(1, 1), weight=3, scale=1.1)

mob("frilled_shark", "Tubarão-Cobra", "Hostile", 20, 2.8,
    fish((5, 5, 18), head=((6, 5, 5), 0), tail=(1, 4, 6), dorsal=((1, 2, 3), 3), fins=((2, 1, 3), 13),
         hp=[("frill", (8, 4, 1), (0, 0, 0))]),
    L(["#3a2a28", "#4a3632", "#5a423c", "#684e46", "#765a52"], belly=["#7a625a", "#8a726a"], eyes="side",
      eye=(220, 220, 220), frill=["#a04048", "#b85058"]),
    spawn="Water", biomes=["Ocean"], drops=[("cod", 0, 2), ("bone", 0, 2)], xp=5, attack=5, group=(1, 1), weight=2,
    scale=1.2, aquatic=True)

mob("anaconda", "Sucuri", "Neutral", 30, 2.2,
    serpent((5, 4, 6), [(5, 4, 5), (6, 5, 5), (6, 5, 5), (6, 5, 5), (6, 5, 5), (5, 4, 5), (5, 4, 5), (4, 3, 5),
                        (4, 3, 4), (3, 2, 4), (2, 2, 4)], head_dy=1,
            hp=[("snout", (3, 2, 2), (0, 0, 6)), ("tongue", (1, 1, 2), (0, 0, 8))]),
    L(["#2a3418", "#34401e", "#3e4c24", "#48582a", "#526430"], belly=["#9a9a5a", "#aaa86a"], eyes="side",
      eye=(220, 180, 40), pattern=("spots", ["#141a0c", "#1c2212"], 1.6, 0.58), tongue=["#b02020", "#d03030"]),
    biomes=["Jungle", "Swamp"], drops=[("snake_skin", 0, 2), ("leather", 0, 1)], xp=5, attack=6, group=(1, 1),
    weight=2, scale=1.4, swims=True, prey=["chicken", "rabbit", "capuchin_monkey"])

mob("tarantula_hawk", "Vespa-Caçadora", "Neutral", 14, 3.0,
    insect((4, 4, 5), (4, 4, 3), (4, 4, 9), legs=3, leg_len=6, wing=(7, 1, 6), abd_dy=1,
           hp=[("antenna", (1, 1, 4), (0, 3, 3), True)], bp=[("stinger", (1, 1, 3), (0, 2, -12))]),
    L(["#0e1018", "#141824", "#1c2030", "#24283c", "#2c3048"], eyes="glossy", wing_col=["#c05010", "#d86818", "#f08020"],
      sheen=(40, 60, 140)),
    biomes=["Desert", "Badlands"], drops=[("string", 0, 1), ("mosquito_proboscis", 0, 1)], xp=3, attack=5,
    group=(1, 1), weight=3, scale=1.3, flies=True, prey=["spider"])

mob("cockroach", "Barata", "Passive", 3, 3.2,
    insect((5, 2, 4), (3, 2, 2), (5, 2, 6), legs=3, leg_len=4, hp=[("antenna", (1, 1, 6), (0, 1, 2), True)]),
    L(["#2a1408", "#3a1c0c", "#4a2410", "#5a2e16", "#6a381c"], eyes="glossy", shell_sheen=True),
    spawn="Dark", biomes=["Overworld"], drops=[("string", 0, 1)], xp=1, group=(2, 4), weight=3, scale=0.7, flee=5.0)

mob("fly", "Mosca", "Passive", 2, 3.0,
    insect((3, 3, 3), (3, 3, 2), (3, 3, 4), legs=3, leg_len=3, wing=(4, 1, 4)),
    L(["#1a1a1c", "#24242a", "#2e2e34", "#38383e"], eyes="bigred", wing_col=["#9aa8b0", "#b0bec6", "#c6d2d8"]),
    biomes=["Swamp", "Plains", "Jungle"], drops=[], xp=0, group=(2, 4), weight=2, scale=0.6, flee=3.0, flies=True)

mob("lava_leviathan", "Leviatã de Lava", "Neutral", 80, 2.2,
    serpent((10, 8, 10), [(10, 9, 8), (10, 9, 8), (9, 8, 7), (8, 7, 6), (7, 6, 6), (6, 5, 5), (5, 4, 4)],
            wave=(0, 0, 1, 2, 1, 0, -1), head_dy=2,
            hp=[("tooth", (1, 2, 1), (1, -2, 9), True), ("horn", (2, 4, 2), (0, 8, 2), True)]),
    L(["#3a0c06", "#4a120a", "#5a1a0e", "#6a2212", "#7a2a16"], eyes="glow", eye=(255, 200, 40),
      pattern=("cracks", ["#ff8a1a", "#ffb030"], 4.5, 0.35), horn=["#2a2420", "#3a322c", "#4a4038"], tooth=TUSK),
    spawn="Dark", biomes=["Nether Wastes"], drops=[("blaze_rod", 1, 2), ("quartz", 1, 4), ("leather", 1, 3)], xp=12,
    attack=8, group=(1, 1), weight=1, scale=1.8)

mob("warped_toad", "Sapo Distorcido", "Passive", 10, 2.6,
    quad((8, 5, 8), (3, 3, 4), (8, 4, 5), head_dy=1, splay=1,
         hp=[("eyebump", (2, 2, 2), (1, 4, 2), True)]),
    L(["#0e4a4a", "#126060", "#187474", "#1e8a8a", "#26a0a0"], belly=["#c8a0d8", "#d8b0e6"], eyes="bump",
      eye=(250, 140, 40), pattern=("spots", ["#4a1a5a", "#5a2470"], 2.0, 0.6)),
    biomes=["Warped Forest"], drops=[("slime_ball_placeholder", 0, 0)], xp=2, group=(1, 3), weight=6, hops=True,
    prey=["fly"])

mob("soul_vulture", "Abutre das Almas", "Hostile", 16, 2.8,
    bird((6, 6, 8), (2, 3, 2), (4, 4, 4), ((2, 2, 3), 0), neck=(2, 4, 2), wing=(1, 5, 8), tail=((4, 1, 3), 1)).layer(0, 0.4),
    L(["#0e1a20", "#14262e", "#1a323c", "#20404a", "#2a5260"], eyes="glow", eye=(90, 220, 255), skin=["#5a6a70", "#6a7a80"],
      beak=["#2a3438", "#3a4448"], overlay="ruff", ruff=["#6ad0e8", "#8ae6f8"]),
    spawn="Dark", biomes=["Soul Sand Valley"], drops=[("feather", 1, 3), ("bone", 0, 2)], xp=6, attack=4, weight=4,
    flies=True)

mob("endergrade", "Endergrado", "Passive", 16, 1.6,
    quad((8, 7, 10), (2, 3, 2), (6, 5, 4), head_dy=-1, splay=1,
         hp=[("snout", (2, 2, 2), (0, 0, 4))]).layer(0, 0.4),
    L(["#5a3a6a", "#6a467c", "#7a528e", "#8a5ea0", "#9a6ab2"], belly=["#c0a8cc", "#d0b8dc"], eyes="glow",
      eye=(220, 160, 255), pattern=("segments", ["#4a2e5a"], 3, 0), overlay="fur"),
    biomes=["The End"], drops=[("chorus_fruit", 0, 2)], xp=2, group=(1, 3), weight=6, flies=True)

mob("mimic_cube", "Cubo Mímico", "Hostile", 24, 2.6,
    quad((12, 10, 12), (2, 2, 2), (10, 4, 2), head_dy=4, inset=2,
         hp=[("tooth", (1, 2, 1), (1, -2, 1), True), ("tooth", (1, 2, 1), (4, -2, 1), True)]),
    L(["#3a2a4a", "#4a3660", "#5a4274", "#6a4e88", "#7a5c9c"], eyes="glow", eye=(240, 240, 140),
      pattern=("plates", ["#d8d0a0", "#e8e0b0"], 4, 0), tooth=WHITE),
    spawn="Dark", biomes=["The End"], drops=[("ender_pearl", 0, 1), ("popped_chorus_fruit", 0, 2)], xp=8, attack=6,
    weight=2, hops=True)

mob("cosmaw", "Cosmandíbula", "Passive", 20, 2.2,
    fish((8, 8, 10), head=((8, 6, 4), 1), tail=(1, 6, 5), dorsal=((1, 4, 6), 2), fins=((6, 1, 5), 3), y=2,
         hp=[("jaw", (8, 2, 3), (0, -2, 1))]),
    L(["#2a1a40", "#382456", "#462e6c", "#543a82", "#624698"], belly=["#c8b8e8", "#d8c8f6"], eyes="glow",
      eye=(250, 220, 120), stars=True, fin=["#8a6ad0", "#a080e8"]),
    biomes=["The End"], drops=[("chorus_fruit", 1, 3), ("ender_pearl", 0, 1)], xp=4, group=(1, 2), weight=4,
    scale=1.3, flies=True)

mob("bone_serpent", "Serpente de Ossos", "Hostile", 30, 3.0,
    serpent((6, 5, 7), [(5, 5, 5), (5, 5, 5), (5, 5, 5), (5, 5, 5), (4, 4, 5), (4, 4, 5), (4, 4, 4), (3, 3, 4),
                        (3, 3, 4), (2, 2, 4)], head_dy=1,
            hp=[("tooth", (1, 2, 1), (1, -1, 6), True), ("horn", (1, 3, 1), (1, 5, 1), True)]),
    L(["#a8a090", "#bab2a0", "#ccc4b0", "#dcd4c0", "#e8e2d0"], eyes="glow", eye=(255, 120, 30),
      pattern=("ribs", ["#5a5246", "#6a6256"], 3, 0), tooth=WHITE, horn=["#6a6256", "#7a7266"]),
    spawn="Dark", biomes=["Nether Wastes", "Soul Sand Valley"], drops=[("bone", 2, 5), ("serpent_tooth", 0, 1)],
    xp=8, attack=5, weight=2, scale=1.4)

mob("crimson_mosquito", "Mosquito Carmesim", "Hostile", 10, 3.0,
    insect((4, 4, 4), (3, 3, 3), (4, 4, 8), legs=3, leg_len=7, wing=(6, 1, 5), abd_dy=1,
           hp=[("proboscis", (1, 1, 5), (0, 0, 3))]),
    L(["#2a0a10", "#3a0e16", "#4a121c", "#5a1822", "#6a1e28"], eyes="bigred", abdomen_glow=(220, 40, 50),
      wing_col=["#a07078", "#b88890", "#d0a0a8"]),
    spawn="Dark", biomes=["Crimson Forest"], drops=[("mosquito_proboscis", 0, 1)], xp=4, attack=3, weight=3, scale=0.9,
    flies=True)

mob("dropbear", "Urso-de-Queda", "Hostile", 26, 2.8,
    quad((10, 9, 12), (3, 7, 3), (8, 7, 6), head_dy=0,
         hp=[("ear", (3, 3, 1), (-1, 6, 2), True), ("snout", (4, 3, 2), (0, 0, 6)), ("tooth", (1, 2, 1), (1, -2, 6), True)],
         bp=[("claw", (1, 3, 1), (0, -3, 11), True)]).layer(0, 0.5).layer("head", 0.4),
    L(["#4a1214", "#5a181a", "#6a1e20", "#7a2426", "#8a2a2c"], belly=["#a05050", "#b06060"], eyes="glow",
      eye=(255, 230, 120), overlay="fur", tooth=WHITE, claw_col=WHITE),
    spawn="Dark", biomes=["Nether Wastes", "Crimson Forest"], drops=[("leather", 0, 2), ("gold_nugget", 0, 3)], xp=7,
    attack=6, weight=2, climbs=True)

mob("snapping_turtle", "Tartaruga-Aligátor", "Neutral", 24, 1.4,
    quad((10, 3, 12), (3, 3, 3), (5, 4, 5), head_dy=0, splay=1,
         hp=[("beak", (3, 2, 2), (0, 0, 5))],
         bp=[("shell", (12, 4, 14), (0, 2, -1)), ("tail", (2, 2, 7), (0, 1, -7))]).layer("shell", 0.6),
    L(["#3a3a2a", "#464432", "#52503a", "#5e5c42", "#6a684a"], eyes="side", eye=(200, 160, 40),
      shell=["#2a2a1e", "#343424", "#3e3e2a", "#484830"], overlay="ridges", beak=["#2a2820", "#3a3628"]),
    biomes=["Swamp"], drops=[("crocodile_scute", 0, 1), ("bone", 0, 1)], xp=3, attack=6, group=(1, 1), weight=3,
    swims=True)

mob("tusklin", "Presudo", "Neutral", 30, 2.8,
    quad((10, 9, 16), (4, 6, 4), (8, 7, 7), head_dy=-1,
         hp=[("tusk", (2, 2, 5), (-2, 0, 6), True), ("tusk", (1, 4, 1), (-2, 2, 10), True), ("snout", (4, 3, 1), (0, 1, 7)),
             ("ear", (2, 2, 1), (0, 7, 2), True)]).layer(0, 0.7).layer("head", 0.5),
    L(["#4a3a2e", "#5a483a", "#6a5646", "#7a6452", "#8a725e"], eyes="side", tusk=TUSK, overlay="shaggy",
      fur_col=["#d8d4cc", "#e8e4dc", "#c4c0b8"]),
    biomes=["Snowy Plains", "Mountains", "Taiga"], drops=[("porkchop", 1, 3), ("leather", 0, 2), ("bone", 0, 1)],
    xp=4, attack=6, group=(1, 3), weight=3, scale=1.1)

mob("banana_slug", "Lesma-Banana", "Passive", 6, 0.8,
    quad((4, 3, 12), (1, 1, 1), (3, 3, 3), head_dy=0, inset=1,
         hp=[("eyestalk", (1, 3, 1), (0, 3, 1), True)]),
    L(["#b89a14", "#c8aa1c", "#d8ba26", "#e6c832", "#f0d640"], eyes="stalk", pattern=("spots", ["#3a3010", "#4a3e18"], 1.2, 0.68),
      slime=True),
    biomes=["Forest", "Dark Forest", "Swamp", "Taiga"], drops=[("banana_slug_slime", 1, 2)], xp=1, group=(1, 2),
    weight=2, scale=0.8)

mob("green_jay", "Gaio-Verde", "Passive", 3, 2.4,
    bird((4, 4, 6), (1, 2, 1), (3, 3, 3), ((1, 1, 2), 0), wing=(1, 3, 5), tail=((2, 1, 4), 0)),
    L(["#2a6a2a", "#347a30", "#3e8a38", "#489a40", "#52aa48"], belly=["#c8c040", "#d8d050"], eyes="side",
      head_col=["#2a50a8", "#3460b8"], face_black=True, beak=BLACK),
    biomes=["Jungle", "Savanna"], drops=[("feather", 0, 1)], xp=1, flee=6.0, group=(1, 3), weight=2, scale=0.7,
    flies=True)

mob("roadrunner", "Papa-Léguas", "Passive", 6, 5.4,
    bird((3, 4, 7), (1, 5, 1), (3, 3, 4), ((1, 1, 3), 0), wing=(1, 3, 5), tail=((2, 1, 6), 2),
         hp=[("crest", (1, 3, 2), (0, 3, 0))]),
    L(["#4a3a2a", "#5a4834", "#6a563e", "#7a6448", "#8a7252"], belly=["#c8bca8", "#d8ccb8"], eyes="side",
      pattern=("streaks", ["#2a2018", "#e0d4c0"], 1.5, 0.55), beak=BLACK, legs=["#4a5a6a", "#5a6a7a"]),
    biomes=["Desert", "Badlands"], drops=[("feather", 0, 2)], xp=1, flee=10.0, group=(1, 2), weight=4,
    prey=["rattlesnake", "lizard"], attack=2)

mob("seal", "Foca", "Passive", 14, 1.4,
    quad((8, 6, 14), (3, 1, 3), (6, 5, 5), head_dy=2, inset=-1, fz=3,
         hp=[("snout", (4, 2, 2), (0, 0, 5)), ("whisker", (1, 1, 1), (0, 1, 6), True)],
         bp=[("flipper", (6, 1, 4), (0, 0, -4))]),
    L(["#4a4a50", "#58585e", "#66666c", "#74747a", "#828288"], belly=["#9a9aa0", "#aaaab0"], eyes="front",
      eye=(10, 10, 12), pattern=("spots", ["#36363c", "#2e2e34"], 1.5, 0.64), whisker=WHITE),
    biomes=["Beach", "Snowy Plains", "Ocean"], ground=["sand", "snowy_grass_block", "snow", "snow_layer", "gravel"],
    drops=[("cod", 0, 2)], xp=2, group=(1, 3), weight=4, flee=5.0, swims=True)

mob("platypus", "Ornitorrinco", "Passive", 10, 2.0,
    quad((6, 4, 9), (2, 2, 2), (5, 4, 4), head_dy=0, splay=1,
         hp=[("bill", (4, 1, 4), (0, 1, 4))],
         bp=[("tail", (5, 1, 5), (0, 1, -5))]),
    L(["#3a2414", "#4a2e1a", "#5a3820", "#6a4428", "#7a5030"], belly=["#8a6a4a", "#9a7a5a"], eyes="side",
      bill=["#2a2a2a", "#363636", "#424242"]),
    biomes=["Swamp", "Forest", "Jungle"], drops=[("leather", 0, 1)], xp=2, group=(1, 2), weight=2, flee=5.0,
    swims=True, scale=0.8)

mob("jerboa", "Jerboa", "Passive", 4, 3.8,
    quad((4, 4, 5), (1, 3, 3), (4, 4, 4), head_dy=1, inset=0,
         hp=[("ear", (2, 4, 1), (0, 4, 1), True)],
         bp=[("tail", (1, 1, 10), (0, 1, -10)), ("tuft", (2, 2, 2), (0, 0, -12))]),
    L(["#a07a4a", "#b08a56", "#c09a62", "#d0aa70", "#dcb87e"], belly=WHITE, eyes="side", eye=(10, 10, 10),
      ear_inner=PINK, tuft=["#e8e4dc", "#141414"]),
    biomes=["Desert", "Badlands"], drops=[("rabbit_hide", 0, 1)], xp=1, flee=7.0, group=(1, 3), weight=3, scale=0.6,
    hops=True)

mob("lobster", "Lagosta", "Neutral", 8, 1.8,
    insect((5, 3, 6), (4, 3, 2), (4, 2, 8), legs=3, leg_len=4,
           hp=[("claw", (3, 2, 5), (-2, 0, 2), True), ("antenna", (1, 1, 7), (0, 2, 2), True)],
           bp=[("tail", (6, 1, 3), (0, 0, -11))]),
    L(["#6a1a10", "#801f14", "#96261a", "#ac2e20", "#c03826"], eyes="stalk", claw_col=["#8a2214", "#a02a1a", "#b8341e"]),
    biomes=["Beach"], drops=[("cod", 0, 1)], xp=2, attack=3, group=(1, 2), weight=3, scale=0.9)

mob("giant_squid", "Lula-Gigante", "Neutral", 40, 2.4,
    fish((8, 8, 12), head=((6, 6, 4), 1), tail=(6, 2, 4), fins=((4, 1, 5), -1), y=2,
         hp=[("tentacle", (1, 1, 12), (0, 1, 4), True), ("tentacle", (1, 1, 10), (2, 3, 4), True)]),
    L(["#6a1e1e", "#802626", "#962e2e", "#ac3838", "#c04242"], belly=["#d07070", "#e08080"], eyes="side",
      eye=(250, 240, 200), pattern=("spots", ["#5a1010", "#4a0c0c"], 1.4, 0.62)),
    spawn="Water", biomes=["Ocean"], drops=[("cod", 0, 2)], xp=6, attack=5, group=(1, 1), weight=1, scale=1.6,
    aquatic=True)

mob("rocky_roller", "Rolador Rochoso", "Hostile", 26, 2.8,
    biped((6, 6, 6), (10, 10, 8), (4, 9, 4), (4, 6, 4), legsep=1, head_dz=1,
          bp=[("shell", (12, 8, 10), (0, 4, -2))]).layer("shell", 0.7),
    L(["#4a4440", "#5a544e", "#6a645c", "#7a746a", "#8a8478"], eyes="glow", eye=(250, 180, 60), overlay="rocks",
      shell=["#3a3632", "#4a4640", "#5a564e", "#6a665c"]),
    spawn="Dark", biomes=["Mountains", "Badlands"], drops=[("cobblestone", 1, 3), ("iron_ingot", 0, 1)], xp=6,
    attack=5, weight=2)

# ---- Naturalist ideas ----------------------------------------------------------------------

NAT = "naturalist"

mob("snail", "Caracol", "Passive", 4, 0.7,
    quad((4, 2, 9), (1, 1, 1), (3, 3, 3), head_dy=-1, inset=1,
         hp=[("eyestalk", (1, 3, 1), (0, 3, 1), True)],
         bp=[("shell", (5, 6, 6), (0, 2, 0))]).layer("shell", 0.4),
    L(["#8a7a68", "#9a8a78", "#aa9a88", "#baaa98"], eyes="stalk", shell=["#6a3a1a", "#8a4e24", "#aa6430", "#c87e40"],
      overlay="spiral", slime=True),
    mod=NAT, biomes=["Forest", "Swamp", "Plains", "Birch Forest"], drops=[("clay_ball", 0, 1)], xp=1, group=(1, 3),
    weight=2, scale=0.7)

mob("butterfly", "Borboleta", "Passive", 2, 2.4,
    flier((2, 2, 4), (6, 1, 6), (2, 2, 2), y=4, hp=[("antenna", (1, 3, 1), (0, 2, 1), True)]),
    L(BLACK, eyes="side", wing_col=["#d06010", "#e87818", "#f89028"], butterfly=True),
    mod=NAT, biomes=["Plains", "Forest", "Cherry Grove", "Birch Forest", "Jungle"], drops=[], xp=0, group=(1, 3),
    weight=2, scale=0.6, flee=4.0, flies=True)

mob("firefly", "Vaga-Lume", "Passive", 2, 2.0,
    flier((2, 2, 3), (3, 1, 3), (2, 2, 1), y=4, abdomen=(2, 2, 2), hp=[("antenna", (1, 2, 1), (0, 2, 0), True)]),
    L(BLACK, eyes="side", wing_col=["#4a3a2a", "#5a4834"], abdomen_glow=(240, 255, 120), head_col=["#c83020", "#d84028"]),
    mod=NAT, spawn="Dark", biomes=["Swamp", "Forest", "Plains"], drops=[("glowstone_dust_placeholder", 0, 0)], xp=0,
    group=(3, 5), weight=3, scale=0.6, flies=True, kind_override="Passive")

mob("giraffe", "Girafa", "Passive", 30, 3.0,
    quad((8, 10, 14), (3, 16, 3), (4, 5, 7), neck=(4, 20, 4), head_dy=0, neck_dz=1,
         hp=[("horn", (1, 3, 1), (0, 5, 1), True), ("ear", (2, 1, 1), (-1, 3, 1), True)],
         bp=[("tail", (1, 8, 1), (0, 1, -1))]),
    L(["#d8c49a", "#e0cca4", "#e6d4b0", "#ecdcba", "#f0e0c0"], belly=["#e6d4b0", "#f0e0c0"], eyes="side",
      pattern=("patches", ["#6a3a14", "#7a461c"], 1.3, 0.48), horn=["#3a2a1a", "#4a3624"]),
    mod=NAT, biomes=["Savanna"], drops=[("leather", 1, 3)], xp=4, group=(1, 3), weight=3, scale=1.3, flee=8.0)

mob("vulture", "Urubu", "Passive", 10, 2.4,
    bird((5, 5, 7), (2, 3, 2), (3, 3, 3), ((2, 2, 3), 0), neck=(2, 3, 2), wing=(1, 5, 7), tail=((3, 1, 3), 1)).layer(0, 0.4),
    L(BLACK, eyes="side", skin=["#8a3a3a", "#a04848", "#b85a5a"], beak=["#c8b8a0", "#d8c8b0"], overlay="ruff",
      ruff=["#2a2a2a", "#3a3a3a"]),
    mod=NAT, biomes=["Desert", "Badlands", "Savanna"], drops=[("feather", 1, 3), ("rotten_flesh", 0, 1)], xp=2,
    group=(1, 3), weight=3, flies=True)

mob("hyena", "Hiena", "Neutral", 18, 3.4,
    quad((7, 8, 12), (3, 9, 3), (6, 6, 6), head_dy=-1, bz=0,
         hp=[("snout", (4, 3, 3), (0, 0, 6)), ("ear", (2, 2, 1), (0, 6, 2), True)],
         bp=[("mane", (2, 2, 10), (0, 8, 1)), ("tail", (2, 4, 2), (0, 3, -1))]),
    L(["#7a6a4a", "#8a7a56", "#9a8a62", "#aa9a70", "#b8a87c"], eyes="front", pattern=("spots", ["#3a2e1e", "#2a2216"], 1.4, 0.58),
      snout=BLACK, mane=["#2a2218", "#3a3022"]),
    mod=NAT, biomes=["Savanna"], drops=[("leather", 0, 2), ("bone", 0, 1)], xp=4, attack=4, group=(2, 4), weight=4,
    prey=["rabbit", "gazelle", "chicken"])

mob("coral_snake", "Cobra-Coral", "Neutral", 8, 2.2,
    serpent((3, 2, 4), [(3, 2, 4)] * 6 + [(2, 2, 4)] * 3, head_dy=0, hp=[("tongue", (1, 1, 2), (0, 0, 4))]),
    L(["#1a1a1a", "#222222"], eyes="side", pattern=("bands", ["#c82020", "#141414", "#e8c830", "#141414"], 2, 0),
      tongue=["#202020", "#303030"]),
    mod=NAT, biomes=["Jungle", "Swamp"], drops=[("snake_skin", 0, 1)], xp=2, attack=4, group=(1, 1), weight=2,
    scale=0.9)

mob("rattlesnake", "Cascavel", "Neutral", 10, 2.2,
    serpent((4, 3, 4), [(3, 3, 4)] * 2 + [(4, 3, 4)] * 3 + [(3, 2, 4)] * 2 + [(2, 2, 3), (2, 3, 2)], head_dy=0,
            hp=[("tongue", (1, 1, 2), (0, 0, 4))]),
    L(["#8a7048", "#9a7e54", "#aa8c60", "#ba9a6c", "#c8a878"], belly=["#d8c8a8", "#e6d6b8"], eyes="side",
      pattern=("diamonds", ["#4a3420", "#3a2818"], 4, 0), tongue=["#202020", "#303030"], rattle=True),
    mod=NAT, biomes=["Desert", "Badlands"], drops=[("snake_skin", 0, 1)], xp=2, attack=5, group=(1, 1), weight=3,
    scale=0.9)

mob("alligator", "Jacaré", "Neutral", 30, 2.2,
    quad((10, 6, 20), (3, 3, 3), (8, 5, 5), head_dy=-1, splay=2,
         hp=[("snout", (6, 3, 8), (0, 0, 5)), ("eyebump", (2, 2, 2), (1, 5, 2), True)],
         bp=[("tail", (6, 4, 12), (0, 1, -12)), ("tail", (2, 3, 8), (0, 1, -20))]).layer("body", 0.4),
    L(["#141814", "#1c201a", "#242a22", "#2c342a", "#343e32"], belly=["#6a6a5a", "#7a7a6a"], eyes="side",
      eye=(200, 150, 30), overlay="scutes"),
    mod=NAT, biomes=["Swamp"], drops=[("crocodile_scute", 0, 1), ("leather", 0, 2)], xp=6, attack=7, weight=2,
    scale=1.25, swims=True, prey=["duck", "chicken"])

mob("tortoise", "Jabuti", "Passive", 20, 0.9,
    quad((8, 3, 10), (3, 3, 3), (4, 4, 4), head_dy=0, splay=1,
         hp=[("beak", (2, 1, 1), (0, 0, 4))],
         bp=[("shell", (10, 6, 12), (0, 2, -1)), ("tail", (1, 1, 2), (0, 1, -2))]).layer("shell", 0.4),
    L(["#5a5a3a", "#6a6a44", "#7a7a4e", "#8a8a58"], eyes="side", shell=["#3a2a14", "#4a3818", "#5e481e", "#725a26"],
      overlay="ridges", beak=["#4a4430", "#5a5438"]),
    mod=NAT, biomes=["Desert", "Savanna", "Badlands", "Jungle"], drops=[("bone", 0, 1)], xp=2, group=(1, 2), weight=3)

mob("lizard", "Lagarto", "Passive", 6, 3.2,
    quad((4, 2, 8), (1, 2, 1), (3, 2, 4), head_dy=0, splay=1,
         bp=[("tail", (2, 1, 8), (0, 0, -8))]),
    L(["#4a6a2a", "#567a32", "#628a3a", "#6e9a44", "#7aa84e"], belly=["#c8c890", "#d8d8a0"], eyes="side",
      pattern=("spots", ["#2a3a14", "#c8b850"], 1.0, 0.62)),
    mod=NAT, biomes=["Desert", "Jungle", "Savanna", "Badlands"], drops=[], xp=1, flee=6.0, group=(1, 2), weight=3,
    scale=0.8)

mob("dragonfly", "Libélula", "Passive", 3, 3.4,
    flier((2, 2, 4), (8, 1, 3), (3, 2, 2), y=4, abdomen=(1, 1, 9)),
    L(["#1a5a8a", "#2270a0", "#2a86b6", "#349cca", "#40b0dc"], eyes="bigred", eye=(60, 200, 120),
      wing_col=["#a8c8d8", "#c0dce8", "#d6eef6"]),
    mod=NAT, biomes=["Swamp", "Plains", "Jungle", "Forest"], drops=[], xp=0, group=(1, 2), weight=2, scale=0.8,
    flee=4.0, flies=True)

for _n, _d, _base, _belly, _extra, _biomes in (
        ("finch", "Pintassilgo", ["#a89010", "#c0a418", "#d4b820", "#e6cc30", "#f2dc40"], ["#e6dca0"],
         dict(head_col=["#141414", "#202020"], wing_col=["#1a1a1a", "#e8e8e8"]), ["Plains", "Forest", "Birch Forest"]),
        ("robin", "Sabiá-Americano", ["#3a3634", "#46423e", "#524e4a", "#5e5a56"], ["#c85a20", "#d86a2a", "#e47a36"],
         dict(eye_ring=(230, 230, 230)), ["Forest", "Birch Forest", "Plains"]),
        ("sparrow", "Pardal", ["#5a3e24", "#6a4a2c", "#7a5634", "#8a623e"], ["#c8bca8", "#d6ccb8"],
         dict(pattern=("streaks", ["#2a1c12", "#a89070"], 1.2, 0.55), head_col=["#7a7470", "#8a8480"]),
         ["Plains", "Forest", "Savanna"]),
        ("canary", "Canário", ["#c8a810", "#dabc18", "#eacc22", "#f6dc30", "#fee840"], ["#f8ec80"],
         dict(), ["Jungle", "Cherry Grove", "Plains"])):
    mob(_n, _d, "Passive", 3, 2.4,
        bird((3, 3, 5), (1, 2, 1), (3, 3, 3), ((1, 1, 1), 1), wing=(1, 2, 4), tail=((2, 1, 3), 0)),
        L(_base, belly=_belly, eyes="side", beak=ORANGE_BEAK if _n != "sparrow" else BLACK, **_extra),
        mod=NAT, biomes=_biomes, drops=[("feather", 0, 1)], xp=1, flee=6.0, group=(1, 3), weight=2, scale=0.6,
        flies=True)

mob("caterpillar", "Lagarta", "Passive", 3, 0.8,
    serpent((3, 3, 2), [(3, 3, 2)] * 5, wave=(0, 0, 0, 0, 0), head_dy=0,
            hp=[("antenna", (1, 2, 1), (0, 3, 1), True)]),
    L(["#3a7a1a", "#448a20", "#4e9a28", "#58aa30"], eyes="side", pattern=("bands", ["#e8d020", "#141414", "#e8e8e8", "#141414"], 1, 0),
      eye=(10, 10, 10)),
    mod=NAT, biomes=["Forest", "Cherry Grove", "Birch Forest", "Jungle"], drops=[], xp=0, group=(1, 2), weight=2,
    scale=0.6)

mob("bass", "Robalo", "Passive", 4, 2.2,
    fish((3, 5, 8), head=((3, 4, 3), 0), tail=(1, 4, 3), dorsal=((1, 2, 5), 1), fins=((1, 1, 2), 5)),
    L(["#2a4a1a", "#365a22", "#426a2a", "#4e7a32", "#5a8a3a"], belly=["#c8d0a0", "#d8e0b0"], eyes="side",
      eye=(220, 180, 40), pattern=("lateral", ["#1a2a10"], 0, 0)),
    mod=NAT, spawn="Water", biomes=["Swamp", "Forest", "Plains", "Jungle", "Taiga"], drops=[("perch", 1, 1)], xp=1,
    group=(1, 3), weight=4, scale=0.8, aquatic=True)

mob("pike", "Lúcio", "Passive", 6, 2.6,
    fish((3, 4, 12), head=((3, 3, 4), 0), tail=(1, 4, 3), dorsal=((1, 2, 3), 1), fins=((1, 1, 2), 8)),
    L(["#3a4a1e", "#465828", "#526632", "#5e743c", "#6a8246"], belly=["#d0d4b0", "#e0e4c0"], eyes="side",
      eye=(220, 200, 40), pattern=("spots", ["#c8c890", "#b8b880"], 1.0, 0.62)),
    mod=NAT, spawn="Water", biomes=["Taiga", "Forest", "Snowy Plains", "Birch Forest"], drops=[("trout", 1, 1)], xp=1,
    group=(1, 2), weight=3, scale=0.9, aquatic=True)

mob("lioness", "Leoa", "Neutral", 26, 3.6,
    quad((9, 9, 18), (3, 9, 3), (6, 6, 6), head_dy=-1,
         hp=[("snout", (4, 3, 2), (0, 0, 6)), ("ear", (2, 2, 1), (0, 6, 2), True)],
         bp=[("tail", (1, 1, 10), (0, 7, -10)), ("tuft", (2, 2, 2), (0, 6, -12))]),
    L(["#9a7232", "#ac823c", "#be9248", "#cea256", "#dcb264"], belly=["#e6d0a0", "#f0dcb0"], eyes="front",
      eye=(200, 160, 40), snout=["#d8c090", "#e6d0a0"], tuft=["#3a2a16", "#4a3620"]),
    mod=NAT, biomes=["Savanna"], drops=[("beef", 0, 2), ("leather", 0, 2)], xp=5, attack=6, group=(1, 3), weight=3,
    prey=["gazelle", "zebra", "rabbit"], sleeps=True)

mob("grass_snake", "Cobra-d'Água", "Passive", 6, 2.2,
    serpent((3, 2, 4), [(3, 2, 4)] * 6 + [(2, 2, 4)] * 3, head_dy=0, hp=[("tongue", (1, 1, 2), (0, 0, 4))]),
    L(["#3a4a1a", "#465a22", "#526a2a", "#5e7a32", "#6a8a3a"], belly=["#c8c890", "#d8d8a0"], eyes="side",
      collar=(240, 220, 80), tongue=["#202020", "#303030"]),
    mod=NAT, biomes=["Plains", "Forest", "Swamp", "Birch Forest"], drops=[("snake_skin", 0, 1)], xp=1, group=(1, 1),
    weight=2, scale=0.9, flee=5.0, swims=True)

mob("hedgehog", "Ouriço", "Passive", 6, 1.8,
    quad((5, 4, 7), (1, 1, 1), (4, 3, 3), head_dy=-1, inset=1,
         hp=[("snout", (2, 1, 2), (0, 0, 3))]).layer("body", 0.8),
    L(["#6a5a48", "#7a6a56", "#8a7a64", "#9a8a72"], eyes="side", overlay="spines", snout=BLACK),
    mod=NAT, biomes=["Forest", "Plains", "Birch Forest"], drops=[], xp=1, group=(1, 1), weight=2, scale=0.7,
    sleeps=True)

# ---- Mutant Monsters ideas: mini-bosses (big, tough, hard-hitting; no boss bar) ------------

MUT = "mutant"

mob("mutant_stalker", "Rastejante Mutante", "Hostile", 100, 2.6,
    quad((10, 14, 8), (5, 8, 5), (12, 10, 10), head_dy=9, head_dz=-7, inset=-2, fz=-3, bz=-3,
         bp=[("spike", (2, 4, 2), (1, 13, 2), True), ("spike", (2, 3, 2), (1, 10, -1), True)]).layer(0, 0.8).layer("head", 0.8),
    L(["#24381c", "#2f4a24", "#3d5e2c", "#4d7234", "#5f863f"], eyes="stalker", overlay="moss", legs=["#1c2a16", "#24361c", "#2c421e"]),
    mod=MUT, spawn="Never", drops=[("gunpowder", 6, 12), ("diamond", 0, 1, 0.3), ("emerald", 1, 3)], xp=40,
    attack=9, scale=1.9, group=(1, 1), weight=0)

mob("mutant_void_walker", "Andarilho do Vazio Mutante", "Hostile", 110, 3.0,
    biped((9, 9, 9), (10, 14, 6), (3, 22, 3), (3, 16, 3), legsep=1, arm_out=0, arm_dy=0,
          hp=[("jaw", (6, 2, 1), (0, 0, 9))]).layer("body", 0.5).layer("head", 0.4),
    L(["#0c0a10", "#141018", "#1c1622", "#241c2c", "#2e2436"], eyes="void", eye=(230, 120, 255), overlay="motes"),
    mod=MUT, spawn="Never", drops=[("ender_pearl", 3, 6), ("ender_eye", 0, 1), ("diamond", 0, 1, 0.35)], xp=45,
    attack=10, scale=1.6, group=(1, 1), weight=0)

mob("mutant_snowman", "Boneco de Neve Mutante", "Neutral", 90, 2.4,
    biped((10, 10, 10), (14, 14, 10), (5, 16, 5), (5, 8, 5), legsep=1, head_dz=1, arm_dy=-2,
          hp=[("carrot", (2, 2, 5), (0, 3, 10))]).layer("body", 0.6).layer("head", 0.5),
    L(["#c8d0d8", "#d4dce4", "#e0e6ec", "#ecf0f4", "#f6f8fa"], eyes="pumpkin", carrot=ORANGE_BEAK,
      arm_col=["#3a2614", "#4a301a", "#5a3a20"], overlay="snow"),
    mod=MUT, biomes=["Snowy Plains"], drops=[("snow", 4, 10), ("ice_placeholder", 0, 0), ("carrot", 1, 3),
                                             ("diamond", 0, 1, 0.25)], xp=35,
    attack=8, scale=1.7, group=(1, 1), weight=1)

# ---- Enderman Overhaul ideas: void walkers of more lands --------------------------------------

END_MOD = "enderman"
for _n, _d, _bone, _accent, _style, _biomes in (
        ("void_walker_jungle", "Andarilho da Selva", ["#0e1a0c", "#142410", "#1a2e14", "#22381a", "#2a4420"],
         ["#3a7a1e", "#4a9024", "#5aa82c"], "vines", ["Jungle"]),
        ("void_walker_cave", "Andarilho das Cavernas", ["#1c1c1e", "#262628", "#303032", "#3a3a3c", "#444446"],
         ["#6a6a6c", "#7e7e80", "#94b4c0"], "stones", ["Mountains", "Badlands", "Plains"]),
        ("void_walker_coral", "Andarilho dos Corais", ["#0c1a26", "#102434", "#142e42", "#1a3850", "#20445e"],
         ["#e85a7a", "#f07a40", "#f0c040"], "coral", ["Beach", "Ocean"]),
        ("void_walker_flower", "Andarilho das Flores", ["#1a1018", "#241622", "#2e1c2c", "#382236", "#422840"],
         ["#f0a0c8", "#f8c0dc", "#fff0a0"], "flowers", ["Cherry Grove", "Plains", "Birch Forest"]),
        ("void_walker_mountain", "Andarilho das Montanhas", ["#1a1e24", "#22282e", "#2a3238", "#323c44", "#3a4650"],
         ["#d8e0e8", "#e8eef4", "#7a8a96"], "frost", ["Mountains"]),
        ("void_walker_warped", "Andarilho Distorcido", ["#06161a", "#0a2024", "#0e2a2e", "#123438", "#163e42"],
         ["#18a0a0", "#20c0b8", "#c050d0"], "warped", ["Warped Forest"])):
    mob(_n, _d, "Neutral", 40, 2.9, walker_variant(), L(_bone, accent=_accent, style=_style, walker=True),
        mod=END_MOD, spawn="Dark", biomes=_biomes, drops=[("ender_pearl", 0, 1, 0.6)], xp=5, attack=7, scale=1.45,
        weight=3 if _n != "void_walker_cave" else 2, group=(1, 1))

# Mobs that are ok to spawn without being in the base list (checked by the skin script).
NAMES = [c["name"] for c in CREATURES]
assert len(NAMES) == len(set(NAMES)), "duplicate creature names"


# --------------------------------------------------------------------------------------------
# RON
# --------------------------------------------------------------------------------------------

def fmt(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, float):
        s = f"{v:.2f}".rstrip("0")
        return s + "0" if s.endswith(".") else s
    return str(v)


def ron_entry(c):
    out = []
    kind = c["flags"].pop("kind_override", None) or c["kind"]
    head = (f'name: "{c["name"]}", display_name: "{c["display"]}", kind: {kind}, health: {c["health"]}, '
            f'speed: {fmt(float(c["speed"]))},')
    out.append(head)
    drops = []
    for d in c["drops"]:
        item, mn, mx = d[0], d[1], d[2]
        if item.endswith("_placeholder") or mx == 0:
            continue
        s = f'(item: "{item}", min: {mn}, max: {mx}'
        if len(d) > 3:
            s += f", chance: {d[3]}"
        drops.append(s + ")")
    line = f"drops: [{', '.join(drops)}], xp: {c['xp']},"
    out.append(line)
    line = (f"width: {fmt(c['width'])}, height: {fmt(c['height'])}, spawn: {c['spawn']}, attack: {c['attack']}, "
            f"group: ({c['group'][0]}, {c['group'][1]}), weight: {c['weight']}, scale: {fmt(float(c['scale']))},")
    out.append(line)
    extra = []
    if c["flee"]:
        extra.append(f"flee_range: {fmt(float(c['flee']))}")
    for k in ("ranged", "explodes", "climbs", "swims", "hops", "sleeps", "burns_in_daylight", "aquatic", "flies"):
        if c["flags"].get(k):
            extra.append(f"{k}: true")
    if c["prey"]:
        extra.append("prey: [" + ", ".join(f'"{p}"' for p in c["prey"]) + "]")
    if extra:
        out.append(", ".join(extra) + ",")
    if c["biomes"]:
        out.append("biomes: [" + ", ".join(f'"{b}"' for b in c["biomes"]) + "],")
    if c["ground"]:
        out.append("ground: [" + ", ".join(f'"{b}"' for b in c["ground"]) + "],")
    out.append("parts: [")
    for p in c["model"].parts:
        s = f"    (size: {tuple(p['size'])}, origin: {tuple(p['origin'])}, uv: {tuple(p['uv'])}"
        if p["anim"]:
            s += f", anim: {p['anim']}, pivot: {tuple(p['pivot'])}"
        out.append(s + "),")
    out.append("],")
    if c["model"].layers:
        out.append("layers: [" + ", ".join(f"(part: {i}, inflate: {fmt(float(f))})" for i, f in c["model"].layers) + "],")
    return "    (\n" + "".join(f"        {line}\n" for line in out) + "    ),\n"


MOD_TITLES = {"alex": "Alex's Mobs ideas", "naturalist": "Naturalist ideas", "mutant": "Mutant Monsters ideas (mini-bosses)",
              "enderman": "Enderman Overhaul ideas"}


def main():
    text = open(MODS_MOBS, encoding="utf-8").read()
    body = BEGIN
    last = None
    for c in CREATURES:
        if c["mod"] != last:
            body += f"    // {MOD_TITLES[c['mod']]}.\n"
            last = c["mod"]
        body += ron_entry(dict(c, flags=dict(c["flags"])))
    body += END
    if BEGIN in text:
        a = text.index(BEGIN)
        b = text.index(END) + len(END)
        text = text[:a] + body + text[b:]
    else:
        k = text.rindex("])")
        text = text[:k] + body + text[k:]
    open(MODS_MOBS, "w", encoding="utf-8").write(text)
    print(f"wrote {len(CREATURES)} creatures to {os.path.relpath(MODS_MOBS, ROOT)}")


if __name__ == "__main__":
    main()
