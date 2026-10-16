#!/usr/bin/env python3
"""Procedurally generate original pixel-art paintings (16 px per block).

Every painting is drawn from scratch here with short hand-picked palettes,
ordered (Bayer) dithering for gradients, hash noise and simple aliased shapes.
No external images are read.

Usage:
    python3 tools/gen_paintings.py

Output:
    packs/vanilla/textures/painting/<name>.png   one per painting
    packs/vanilla/textures/painting/back.png     16x16 wooden canvas back
    packs/vanilla/textures/painting/preview.png  4x contact sheet
    packs/vanilla/paintings.ron                  list with Portuguese titles
Deterministic: all randomness is hash-based with fixed seeds.
"""

import math
import os
import zlib

from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, "packs", "vanilla", "textures", "painting")
RON_PATH = os.path.join(ROOT, "packs", "vanilla", "paintings.ron")
PX = 16  # pixels per block
BASE_SEED = 0x9A1_7ED

BAYER4 = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]


# --------------------------------------------------------------------------
# helpers
# --------------------------------------------------------------------------


def hexrgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def pal(*hexes):
    return [hexrgb(h) for h in hexes]


def hash01(seed, *vals):
    h = seed & 0xFFFFFFFF
    for v in vals:
        h ^= (int(v) * 0x27D4EB2D) & 0xFFFFFFFF
        h = (h * 0x165667B1 + 0x9E3779B9) & 0xFFFFFFFF
        h ^= h >> 15
        h = (h * 0x85EBCA6B) & 0xFFFFFFFF
        h ^= h >> 13
        h = (h * 0xC2B2AE35) & 0xFFFFFFFF
        h ^= h >> 16
    return h / 4294967296.0


def mul(rgb, k):
    return tuple(max(0, min(255, int(round(c * k)))) for c in rgb)


def mix(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


class Canvas:
    def __init__(self, name, bw, bh):
        self.name = name
        self.w, self.h = bw * PX, bh * PX
        self.img = Image.new("RGB", (self.w, self.h), (0, 0, 0))
        self.px = self.img.load()
        self.draw = ImageDraw.Draw(self.img)
        self.seed = BASE_SEED ^ zlib.crc32(name.encode())

    def rnd(self, *v):
        return hash01(self.seed, *v)

    def put(self, x, y, c):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[x, y] = tuple(c)

    def get(self, x, y):
        return self.px[x, y]

    def dpick(self, palette, t, x, y, strength=1.0):
        """Ordered-dithered pick of palette[t * len]."""
        n = len(palette)
        b = (BAYER4[y % 4][x % 4] + 0.5) / 16.0 - 0.5
        k = int(math.floor(t * (n - 1) + 0.5 + b * strength))
        return palette[max(0, min(n - 1, k))]

    def vgrad(self, palette, y0=0, y1=None, x0=0, x1=None, strength=1.0):
        y1 = self.h if y1 is None else y1
        x1 = self.w if x1 is None else x1
        for y in range(y0, y1):
            t = (y - y0 + 0.5) / max(1, (y1 - y0))
            for x in range(x0, x1):
                self.put(x, y, self.dpick(palette, t, x, y, strength))

    def rect(self, x0, y0, x1, y1, c):
        """Inclusive rectangle."""
        if x1 >= x0 and y1 >= y0:
            self.draw.rectangle([x0, y0, x1, y1], fill=tuple(c))

    def poly(self, pts, c):
        self.draw.polygon(pts, fill=tuple(c))

    def disc(self, cx, cy, r, c, sy=1.0):
        for y in range(int(cy - r * sy - 1), int(cy + r * sy + 2)):
            for x in range(int(cx - r - 1), int(cx + r + 2)):
                if ((x + .5 - cx) / r) ** 2 + ((y + .5 - cy) / (r * sy)) ** 2 <= 1.0:
                    self.put(x, y, c)

    def line(self, x0, y0, x1, y1, c):
        self.draw.line([x0, y0, x1, y1], fill=tuple(c))

    def speckle(self, x0, y0, x1, y1, amount=0.06, k=0.08, salt=0):
        """Light per-pixel value noise to break flat areas."""
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                r = self.rnd(x, y, 991 + salt)
                if r < amount:
                    self.put(x, y, mul(self.get(x, y), 1 - k))
                elif r > 1 - amount:
                    self.put(x, y, mul(self.get(x, y), 1 + k))

    def frame(self):
        """1px darker wooden frame inside the edge, lit from the top-left."""
        wood = pal("#3b2414", "#4a2e19", "#5a3920", "#6b4527")
        w, h = self.w, self.h
        for x in range(w):
            for y in range(h):
                if x in (0, w - 1) or y in (0, h - 1):
                    lit = (y == 0 or x == 0) and not (x == w - 1 or y == h - 1)
                    grain = self.rnd(x // 3 if y in (0, h - 1) else y // 3, 7)
                    k = 1 + (1 if lit else -1) * 0 + (1 if lit else 0)
                    base = wood[min(3, k + (1 if grain > 0.6 else 0))]
                    if not lit:
                        base = wood[0] if grain < 0.5 else wood[1]
                    self.put(x, y, base)
        # thin inner shadow so the picture sits inside the frame
        for x in range(1, w - 1):
            self.put(x, 1, mul(self.get(x, 1), 0.82))
        for y in range(2, h - 1):
            self.put(1, y, mul(self.get(1, y), 0.88))

    def save(self):
        self.frame()
        path = os.path.join(OUT_DIR, f"{self.name}.png")
        self.img.save(path)
        return path


# --------------------------------------------------------------------------
# shared scenery pieces
# --------------------------------------------------------------------------


def ridge(c, base, amp, freq, phase, palette, top_pal=None, snow=0, seed=0, shade=True):
    """Fill a mountain ridge whose height is a sum of sines + hash wobble."""
    heights = []
    for x in range(c.w):
        t = x / c.w
        y = base - amp * (0.55 * math.sin(t * freq * 6.28 + phase)
                          + 0.3 * math.sin(t * freq * 13.7 + phase * 2.1)
                          + 0.15 * (c.rnd(x // 2, seed) - 0.5))
        heights.append(int(round(y)))
    hmin = min(heights)
    for x, top in enumerate(heights):
        for y in range(max(0, top), c.h):
            depth = (y - top)
            lit = shade and x + 1 < len(heights) and heights[x + 1] > top  # slope facing left
            t = min(1.0, (y - hmin) / max(1, base + amp - hmin))
            col = c.dpick(palette, 1 - t, x, y)
            if lit:
                col = mul(col, 1.12)
            if snow and depth < snow - (top - hmin) // 2 and top <= hmin + snow:
                col = (232, 236, 244) if c.rnd(x, y, 5) > 0.25 else (196, 206, 224)
            c.put(x, y, col)
    return heights


def pine(c, x, base, height, cols, snow=False):
    """Stepped pine tree: apex at (x, base - height), trunk at the base."""
    dark, mid, light = cols
    c.rect(x, base - 1, x, base, (62, 42, 30))
    tier = max(2, height // 3)
    top = base - height
    for y in range(top, base - 1):
        rel = y - top
        k, r = divmod(rel, tier)
        hw = (r + k * tier // 2 + 1) // 2 + (1 if k else 0) * 0
        hw = min(hw, height // 3 + 1)
        for xx in range(x - hw, x + hw + 1):
            col = light if xx < x else (mid if xx == x else dark)
            if snow and ((r == 0 and xx != x - hw and xx != x + hw and k > 0)
                         or (r == tier - 1 and xx in (x - hw, x - hw + 1))
                         or (rel == 0)):
                col = (232, 238, 246) if xx <= x else (196, 210, 226)
            c.put(xx, y, col)


# --------------------------------------------------------------------------
# paintings
# --------------------------------------------------------------------------


def bowl_of_fruit(c):
    c.vgrad(pal("#5c3a2e", "#6e4634", "#7c5240", "#8a5e48"), 0, 11)
    c.speckle(1, 1, 14, 10, 0.08, 0.06)
    c.vgrad(pal("#3a2618", "#4d3220", "#5e3e28"), 11, 16)
    c.line(1, 11, 14, 11, (112, 80, 52))
    # bowl
    c.rect(3, 9, 12, 9, (56, 98, 150))
    c.rect(4, 10, 11, 10, (44, 78, 124))
    c.rect(5, 11, 10, 11, (34, 60, 98))
    c.line(4, 9, 7, 9, (98, 142, 192))
    # fruit: banana, apple, orange, grapes
    for x, y in ((4, 7), (5, 6), (6, 6), (7, 6), (8, 7)):
        c.put(x, y, (234, 202, 64))
    c.put(4, 8, (196, 158, 40))
    c.disc(6.5, 7.5, 1.6, (186, 34, 38))
    c.put(6, 6, (232, 96, 90))
    c.put(7, 5, (74, 52, 30))
    c.disc(9.5, 7.5, 1.6, (226, 132, 32))
    c.put(9, 6, (250, 186, 92))
    for x, y in ((11, 6), (12, 7), (10, 8), (11, 8), (12, 8), (11, 7)):
        c.put(x, y, (98, 40, 112) if (x + y) % 2 else (128, 60, 140))
    c.put(11, 5, (64, 110, 40))
    c.put(13, 13, (172, 40, 44))  # a fallen cherry
    c.put(13, 12, (74, 52, 30))


def ginger_cat(c):
    c.vgrad(pal("#2b4a52", "#335860", "#3c6670"), 0, 10)
    c.speckle(1, 1, 14, 9, 0.06, 0.08)
    # cushion
    c.rect(1, 10, 14, 14, (132, 40, 48))
    c.line(1, 10, 14, 10, (170, 62, 66))
    c.rect(1, 13, 14, 14, (100, 28, 36))
    # hand-placed sprite: a ginger cat curled up on a cushion, head on the left
    sprite = [
        "..O...O.........",
        ".OpO.OpO.OOOO...",
        ".OlllllOOfdfdfO.",
        ".OlelelOfdfdfdfO",
        ".OlwnwlOffdfdfdO",
        "..OlwlOfffffffdO",
        "...OOlllllllllO.",
        "..OdldldldldldO.",
        "...OOOOOOOOOOO..",
    ]
    colors = {"O": (70, 34, 18), "p": (226, 140, 136), "l": (240, 166, 86),
              "f": (224, 136, 60), "d": (176, 86, 34), "e": (70, 34, 18),
              "w": (250, 226, 196), "n": (200, 100, 104)}
    for row, line in enumerate(sprite):
        assert len(line) == 16
        for x, ch in enumerate(line):
            if ch in colors:
                c.put(x, row + 4, colors[ch])
    # a little window with moonlight on the wall
    c.rect(10, 1, 13, 3, (34, 52, 70))
    c.put(12, 2, (220, 226, 200))


def abstract_tiles(c):
    cols = [(212, 76, 52), (238, 186, 64), (52, 98, 160), (236, 226, 204), (34, 36, 44),
            (82, 148, 112)]
    c.rect(0, 0, 15, 15, cols[3])
    for qy in range(2):
        for qx in range(2):
            k = (qx + qy * 2)
            bg = cols[(k + 1) % 5] if k != 3 else cols[5]
            fg = cols[(k + 3) % 5]
            x0, y0 = 1 + qx * 7, 1 + qy * 7
            c.rect(x0, y0, x0 + 6, y0 + 6, bg)
            # quarter circle anchored to a different corner in each tile
            ax, ay = (x0, y0) if k == 0 else (x0 + 7, y0) if k == 1 else \
                (x0, y0 + 7) if k == 2 else (x0 + 7, y0 + 7)
            for y in range(y0, y0 + 7):
                for x in range(x0, x0 + 7):
                    d = math.hypot(x + .5 - ax, y + .5 - ay)
                    if d < 5.2:
                        c.put(x, y, fg)
                    if 2.2 < d < 3.2:
                        c.put(x, y, bg if k % 2 else cols[3])
    c.line(1, 8, 14, 8, cols[4])
    c.line(8, 1, 8, 14, cols[4])
    c.put(8, 8, cols[1])


def red_mushroom(c):
    c.vgrad(pal("#14261a", "#1c3322", "#24402a", "#2e4e32"), 0, 16)
    # soft light shaft
    for y in range(1, 12):
        for x in range(1, 15):
            if abs((x - 3) - y * 0.6) < 1.2 and c.rnd(x, y) > 0.35:
                c.put(x, y, mul(c.get(x, y), 1.35))
    c.vgrad(pal("#2a1e14", "#3a2a1a", "#4a3622"), 12, 16)
    # grass blades
    for x in range(1, 15):
        hgt = 1 + int(c.rnd(x, 3) * 3)
        for k in range(hgt):
            c.put(x, 12 - k, (58, 112, 46) if k else (44, 88, 38))
    # stem
    c.rect(7, 8, 9, 12, (230, 220, 196))
    c.rect(9, 8, 9, 12, (196, 184, 160))
    c.line(7, 9, 9, 9, (206, 196, 172))
    # cap
    for y in range(3, 9):
        for x in range(2, 15):
            if ((x + .5 - 8.5) / 5.6) ** 2 + ((y + .5 - 8.2) / 5.0) ** 2 <= 1 and y <= 8:
                c.put(x, y, c.dpick(pal("#8e1a1c", "#b42628", "#d43a32", "#e85a46"),
                                    0.95 - (y - 3) / 6 - (x - 8) / 20, x, y))
    c.line(4, 8, 13, 8, (120, 30, 30))
    for x, y in ((6, 4), (10, 5), (5, 6), (8, 6), (12, 7), (9, 3)):
        c.put(x, y, (246, 240, 228))
    c.put(13, 13, (196, 70, 60))  # a tiny second mushroom
    c.put(13, 14, (226, 214, 190))


def portrait(c):
    c.vgrad(pal("#1c2a2a", "#22383a", "#2a4648", "#32524e"), 0, 32)
    c.speckle(1, 1, 14, 30, 0.07, 0.1)
    skin = pal("#b07a56", "#c88c64", "#dca07a")
    hair = (64, 38, 24)
    # shoulders / shirt
    c.rect(2, 23, 13, 30, (92, 52, 112))
    c.rect(2, 23, 3, 30, (72, 38, 90))
    c.rect(12, 23, 13, 30, (72, 38, 90))
    c.rect(6, 22, 9, 23, skin[1])
    c.put(6, 24, (232, 222, 200)); c.put(9, 24, (232, 222, 200))  # collar
    c.put(7, 24, skin[0]); c.put(8, 24, skin[0])
    c.put(7, 27, (220, 186, 70))  # brooch
    # head
    for y in range(9, 22):
        for x in range(4, 12):
            c.put(x, y, c.dpick(skin, 0.75 - (x - 4) / 14 - (y - 9) / 40, x, y, 0.8))
    c.rect(4, 7, 11, 10, hair)
    c.rect(3, 8, 3, 15, hair)
    c.rect(12, 8, 12, 15, hair)
    c.put(4, 11, hair); c.put(11, 11, hair)
    c.line(5, 7, 9, 7, (92, 58, 36))
    # face
    c.put(5, 13, (246, 244, 236)); c.put(6, 13, (40, 70, 110))
    c.put(9, 13, (40, 70, 110)); c.put(10, 13, (246, 244, 236))
    c.line(5, 12, 6, 12, hair); c.line(9, 12, 10, 12, hair)
    c.put(7, 15, skin[0]); c.put(8, 16, skin[0])
    c.line(6, 18, 9, 18, (132, 62, 56))
    c.put(5, 17, (210, 128, 108)); c.put(10, 17, (210, 128, 108))
    c.rect(4, 21, 11, 21, skin[0])


def lighthouse(c):
    c.vgrad(pal("#1e2048", "#2c2a5c", "#4a3a6e", "#7a4c78", "#c06a6a", "#e8946a"), 0, 22)
    for k in range(12):
        x, y = 1 + int(c.rnd(k, 1) * 14), 1 + int(c.rnd(k, 2) * 9)
        c.put(x, y, (226, 226, 240) if k % 3 else (170, 170, 210))
    c.vgrad(pal("#3a6080", "#2a4a68", "#1c3450"), 22, 32)
    for y in range(23, 31, 2):
        for x in range(1, 15):
            if c.rnd(x // 2, y) > 0.6:
                c.put(x, y, (110, 150, 180))
    # rocks
    c.poly([(6, 32), (7, 25), (10, 24), (13, 27), (14, 32)], (52, 48, 52))
    c.poly([(9, 24), (10, 24), (13, 27), (11, 27)], (84, 80, 86))
    # tower
    for y in range(9, 25):
        half = 1 + (y - 9) // 7
        for x in range(9 - half, 11 + half):
            band = ((y - 9) // 3) % 2 == 0
            col = (196, 48, 44) if band else (232, 228, 220)
            if x >= 10 + half - 0:
                col = mul(col, 0.75)
            c.put(x, y, col)
    c.rect(8, 6, 11, 8, (44, 42, 48))
    c.rect(9, 7, 10, 7, (255, 236, 140))
    c.rect(8, 5, 11, 5, (150, 40, 40))
    c.put(9, 4, (150, 40, 40)); c.put(10, 4, (150, 40, 40))
    # light beams
    for x in range(1, 8):
        for y in (6 - (8 - x) // 3, 7 + (8 - x) // 3):
            c.put(x, y, mix(c.get(x, y), (255, 236, 160), 0.55))
        c.put(x, 7, mix(c.get(x, 7), (255, 236, 160), 0.7))
    c.put(10, 18, (60, 70, 90))  # window


def sunset_mountains(c):
    c.vgrad(pal("#3a2a5e", "#5e3270", "#8e3c6e", "#c8565e", "#e8804e", "#f6b25a"), 0, 12)
    c.disc(22.5, 10.5, 3.6, (252, 222, 120))
    c.disc(22.5, 10.5, 2.6, (255, 240, 170))
    for x in range(14, 30):  # thin cloud bands across the sun
        if c.rnd(x, 1) > 0.25:
            c.put(x, 8, (226, 120, 104))
    for x in range(3, 13):
        c.put(x, 4, (166, 82, 120))
    ridge(c, 11, 5, 1.2, 0.6, pal("#2a1c40", "#3c2652", "#523066"), snow=2, seed=1)
    ridge(c, 14, 3, 1.8, 2.4, pal("#140e22", "#1e1630", "#2a1e3e"), seed=2, shade=False)


def desert_cactus(c):
    c.vgrad(pal("#3a78c0", "#4e8cce", "#6aa4da", "#8cbee4", "#b6d6ea"), 0, 11)
    c.disc(6.5, 4.5, 2.2, (255, 246, 200))
    # mesa
    c.poly([(18, 10), (20, 6), (27, 6), (29, 10)], (176, 100, 64))
    c.line(20, 6, 27, 6, (210, 132, 86))
    c.line(19, 8, 28, 8, (150, 82, 52))
    sand = pal("#c49656", "#d4aa68", "#e2bd7c", "#ecd096")
    for x in range(c.w):
        top = 10 + int(round(1.4 * math.sin(x / 5.0) + 0.6 * math.sin(x / 2.3)))
        for y in range(top, c.h):
            c.put(x, y, c.dpick(sand, 1 - (y - top) / 6 - (0.2 if x % 9 > 5 else 0), x, y))
    # saguaro
    g, gl, gd = (60, 128, 56), (88, 160, 72), (40, 92, 42)
    c.rect(11, 4, 12, 13, g)
    c.rect(11, 4, 11, 13, gl)
    c.rect(8, 6, 8, 9, g); c.rect(9, 9, 10, 9, g); c.put(8, 6, gl)
    c.rect(15, 5, 15, 8, g); c.rect(13, 8, 14, 8, g); c.put(15, 5, gl)
    c.put(12, 13, gd); c.put(12, 11, gd)
    c.put(11, 3, (232, 120, 160))  # flower on top
    c.line(9, 14, 15, 14, (176, 132, 76))  # shadow
    for x, y in ((25, 13), (3, 13), (27, 14)):
        c.put(x, y, (130, 120, 104))  # pebbles


def sailing_ship(c):
    c.vgrad(pal("#6aa6dc", "#86b8e2", "#a6cce8", "#cadff0"), 0, 10)
    for x in range(3, 11):
        c.put(x, 3, (244, 248, 252)); c.put(x + 1, 2, (244, 248, 252)) if x < 8 else None
    sea = pal("#1a4a78", "#235c8e", "#2e70a4", "#3c86b8")
    c.vgrad(sea[::-1], 10, 16)
    for y in range(11, 15):
        for x in range(1, 31):
            if (x + y * 3) % 7 == 0:
                c.put(x, y, (160, 200, 228))
    # hull
    c.poly([(9, 10), (24, 10), (22, 13), (11, 13)], (104, 60, 34))
    c.line(9, 10, 24, 10, (140, 86, 48))
    c.line(12, 12, 21, 12, (78, 44, 26))
    for x in (13, 16, 19):
        c.put(x, 11, (230, 200, 120))  # portholes
    # masts and sails
    c.rect(14, 2, 14, 9, (70, 44, 26))
    c.rect(19, 3, 19, 9, (70, 44, 26))
    c.poly([(11, 3), (17, 3), (17, 8), (11, 8)], (240, 236, 222))
    c.poly([(17, 4), (21, 4), (22, 8), (17, 8)], (224, 218, 200))
    c.line(11, 8, 17, 8, (200, 192, 172))
    c.poly([(20, 4), (20, 8), (24, 8)], (240, 236, 222))
    c.put(14, 1, (200, 40, 40)); c.put(15, 1, (200, 40, 40))
    c.line(8, 13, 25, 13, (230, 240, 248))  # foam
    for x, y in ((27, 5), (28, 4), (29, 5)):
        c.put(x, y, (60, 60, 70))  # gull


def starry_night(c):
    c.vgrad(pal("#0a0c24", "#10163a", "#18224e", "#223064", "#2c3e76"), 0, 24)
    for k in range(44):
        x, y = 1 + int(c.rnd(k, 1) * 30), 1 + int(c.rnd(k, 2) * 20)
        b = c.rnd(k, 3)
        c.put(x, y, (250, 246, 220) if b > 0.7 else (176, 184, 220) if b > 0.3 else (110, 120, 170))
    for sx, sy in ((7, 5), (20, 9), (26, 3)):
        c.put(sx, sy, (255, 252, 230))
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            c.put(sx + dx, sy + dy, (190, 196, 236))
    # crescent moon
    c.disc(24.5, 14.5, 3.2, (246, 236, 180))
    c.disc(26.0, 13.6, 2.9, c.get(29, 13))
    for y in range(11, 18):
        for x in range(26, 30):
            c.put(x, y, c.dpick(pal("#0a0c24", "#10163a", "#18224e", "#223064", "#2c3e76"),
                                y / 24, x, y)) if c.get(x, y) == c.get(29, 13) else None
    # milky swirl
    for t in range(0, 60):
        a = t / 60 * 5.5
        x = 12 + math.cos(a) * (2 + t / 8)
        y = 11 + math.sin(a) * (1.2 + t / 14)
        if c.rnd(t, 9) > 0.3:
            c.put(int(x), int(y), mix(c.get(int(x), int(y)), (120, 140, 210), 0.5))
    # hills
    for x in range(c.w):
        top = 23 + int(round(2.2 * math.sin(x / 6.0 + 1) + math.sin(x / 2.7)))
        for y in range(top, c.h):
            c.put(x, y, c.dpick(pal("#060a10", "#0c1420", "#142030"), 1 - (y - top) / 6, x, y))
    # house with a lit window, tree
    c.rect(5, 21, 10, 25, (30, 26, 34))
    c.poly([(4, 21), (7, 18), (8, 18), (11, 21)], (46, 30, 36))
    c.rect(7, 22, 8, 23, (250, 200, 90))
    c.put(9, 19, (40, 34, 40)); c.put(9, 18, (40, 34, 40))
    c.rect(22, 20, 22, 26, (18, 16, 20))
    c.disc(22.5, 18.5, 2.4, (14, 26, 30))
    c.put(21, 17, (24, 40, 44))


def castle_hill(c):
    c.vgrad(pal("#5c9ad6", "#74aee0", "#90c2e8", "#b2d6ee"), 0, 22)
    for cx, cy, r in ((6, 6, 2.2), (9, 5, 2.6), (12, 6.5, 1.8), (24, 9, 1.8), (27, 8.5, 2.2)):
        c.disc(cx, cy, r, (246, 248, 252), 0.6)
    for x in range(4, 14):
        c.put(x, 7, (214, 226, 238))
    # distant hills
    for x in range(c.w):
        top = 20 + int(round(1.5 * math.sin(x / 4.0)))
        for y in range(top, c.h):
            c.put(x, y, (96, 140, 108))
    # main hill
    grass = pal("#2e6a2a", "#3a7e30", "#4a9238", "#5ca442")
    for x in range(c.w):
        top = int(round(17 + 9 * ((x - 16) / 16) ** 2))
        for y in range(top, c.h):
            c.put(x, y, c.dpick(grass, 1 - (y - top) / 10 - (0.25 if x > 18 else 0), x, y))
    # path
    for y in range(22, 31):
        x = 16 + int(3 * math.sin(y / 2.2))
        c.put(x, y, (196, 170, 120)); c.put(x + 1, y, (176, 150, 104))
    # castle
    st, sl, sd = (150, 146, 150), (182, 178, 180), (110, 106, 114)
    c.rect(10, 12, 22, 18, st)
    c.rect(19, 12, 22, 18, sd)
    for x in range(10, 23, 2):
        c.put(x, 11, st if x < 19 else sd)
    c.rect(8, 8, 11, 18, sl)
    c.rect(21, 8, 24, 18, st)
    c.rect(23, 8, 24, 18, sd)
    for x in (8, 10, 21, 23):
        c.put(x, 7, sl if x < 12 else st)
    c.rect(14, 6, 17, 18, sl)
    c.rect(17, 6, 17, 18, st)
    for x in (14, 16):
        c.put(x, 5, sl)
    c.rect(15, 1, 15, 4, (60, 50, 44))
    c.rect(16, 1, 18, 2, (200, 44, 44))
    c.rect(15, 15, 16, 18, (70, 44, 30))
    for x, y in ((9, 10), (22, 10), (15, 9), (12, 14), (20, 14)):
        c.put(x, y, (40, 40, 56))
    for y in range(13, 18, 2):  # brick hints
        for x in range(11, 19, 3):
            c.put(x + (y // 2) % 2, y, sd)


def underwater(c):
    c.vgrad(pal("#3aa0c8", "#2a86b4", "#1e6c9a", "#165480", "#0e3e66"), 0, 32)
    # light rays
    for y in range(1, 22):
        for base in (6, 15, 24):
            x = base + y // 3
            if c.rnd(x, y, base) > 0.4:
                c.put(x, y, mix(c.get(x, y), (170, 220, 236), 0.35))
    sand = pal("#9c8456", "#b49a66", "#c8b07a")
    for x in range(c.w):
        top = 27 + int(round(1.2 * math.sin(x / 3.5)))
        for y in range(top, c.h):
            c.put(x, y, c.dpick(sand, 1 - (y - top) / 4, x, y))
    # seaweed
    for bx, hgt in ((4, 13), (7, 9), (26, 15), (29, 10)):
        for k in range(hgt):
            y = 28 - k
            x = bx + int(round(math.sin(k / 2.0 + bx)))
            c.put(x, y, (40, 120, 70) if k % 3 else (60, 150, 84))
    # coral
    coral = (226, 92, 110)
    for x, y0, hgt in ((12, 27, 4), (14, 27, 6), (16, 27, 3), (20, 27, 5)):
        c.rect(x, y0 - hgt, x, y0, coral)
        c.put(x - 1, y0 - hgt + 1, coral); c.put(x + 1, y0 - hgt + 2, (246, 140, 150))
    c.put(18, 26, (240, 190, 70)); c.put(18, 25, (240, 190, 70)); c.put(19, 26, (220, 160, 50))
    # fish
    def fish(x, y, body, fin, left=True):
        c.rect(x, y, x + 3, y + 1, body)
        c.put(x + 1, y - 1, body)
        tx = x + 4 if left else x - 1
        c.put(tx, y - 1 if True else y, fin); c.put(tx, y + 2 if False else y + 1, fin)
        c.put(tx, y, fin)
        c.put(x + (0 if left else 3), y, (20, 20, 30))
        c.put(x + 2, y + 1, mul(body, 0.8))
    fish(8, 10, (246, 150, 40), (226, 110, 30))
    fish(20, 15, (250, 220, 70), (220, 180, 40), left=False)
    fish(14, 19, (230, 236, 244), (120, 170, 220))
    for x, y in ((11, 7), (10, 5), (11, 3), (23, 11), (24, 8)):
        c.put(x, y, (200, 236, 248))  # bubbles
    c.put(4, 28, (240, 230, 210)); c.put(5, 28, (220, 200, 180))  # shell


def mountain_lake(c):
    sky = pal("#5a92d0", "#70a4d8", "#8ab6e0", "#a8c8e6", "#c8dcec")
    c.vgrad(sky, 0, 16)
    for cx, cy in ((12, 4), (44, 3), (52, 5)):
        c.disc(cx, cy, 3.0, (244, 246, 250), 0.45)
    ridge(c, 15, 7, 1.6, 0.3, pal("#46507a", "#56608a", "#6a749c"), snow=3, seed=4)
    ridge(c, 17, 4, 2.4, 1.8, pal("#2e4a46", "#365a50", "#40685a"), seed=5)
    # shoreline pines
    for x in range(1, c.w - 1, 3):
        hgt = 3 + int(c.rnd(x, 6) * 4)
        pine(c, x + int(c.rnd(x, 7) * 2), 18, hgt, ((20, 52, 36), (30, 70, 46), (42, 90, 56)))
    c.rect(1, 19, c.w - 2, 19, (36, 56, 40))
    # lake: mirrored reflection, darker and bluer, with ripples
    horizon = 19
    for y in range(horizon + 1, c.h):
        src = horizon - (y - horizon)
        for x in range(c.w):
            col = c.get(x, max(0, src))
            col = mix(col, (34, 78, 120), 0.45)
            if c.rnd(x // 3, y, 2) > 0.78:
                col = mix(col, (186, 214, 236), 0.4)
            c.put(x, y, col)
    for x in range(20, 44):  # little rowing boat
        pass
    c.rect(38, 24, 42, 24, (120, 70, 40))
    c.rect(39, 25, 41, 25, (90, 50, 30))
    c.put(40, 23, (200, 60, 50)); c.put(40, 22, (230, 190, 150))


def sunflower_field(c):
    c.vgrad(pal("#3c84d0", "#509ad8", "#6cb0e0", "#90c6e8", "#bcdcee"), 0, 14)
    c.disc(54, 4, 2.8, (255, 248, 210))
    for cx, cy in ((14, 5), (30, 3)):
        c.disc(cx, cy, 3.4, (246, 248, 252), 0.45)
    # far hills
    for x in range(c.w):
        top = 12 + int(round(1.5 * math.sin(x / 7.0)))
        for y in range(top, c.h):
            c.put(x, y, (90, 136, 70))
    field = pal("#2a5a1e", "#356c24", "#41802c", "#4e9234")
    c.vgrad(field, 15, 32)
    # far rows: dots of yellow
    for y in range(14, 20):
        for x in range(1, c.w - 1):
            if (x + y * 2) % 3 == 0 and c.rnd(x, y) > 0.3:
                c.put(x, y, (232, 196, 40) if y > 15 else (200, 170, 50))
    # near flowers
    def flower(x, y, r):
        for k in range(y + r, c.h - 1):
            c.put(x, k, (46, 110, 36))
        c.put(x - 1, y + r + 2, (66, 136, 48)); c.put(x + 1, y + r + 3, (66, 136, 48))
        c.disc(x + .5, y + .5, r + 1.0, (246, 196, 30))
        for a in range(8):
            ang = a * math.pi / 4
            c.put(int(x + .5 + math.cos(ang) * (r + 1.2)), int(y + .5 + math.sin(ang) * (r + 1.2)),
                  (252, 220, 70))
        c.disc(x + .5, y + .5, max(0.8, r - 0.3), (110, 64, 26))
        c.put(x, y, (76, 44, 20))
    for k, x in enumerate(range(3, c.w - 2, 6)):
        y = 20 + int(c.rnd(x, 1) * 3)
        flower(x, y, 1)
    for k, x in enumerate(range(6, c.w - 3, 9)):
        y = 24 + int(c.rnd(x, 2) * 2)
        flower(x, y, 2)


def snowy_pine_forest(c):
    c.vgrad(pal("#8ea6c0", "#a2b6cc", "#b6c6d8", "#cad6e2", "#dce4ec"), 0, 34)
    ridge(c, 26, 6, 1.1, 1.0, pal("#a4b4c8", "#b2c0d2", "#c2cede"), snow=0, seed=8, shade=False)
    far = ((92, 120, 124), (104, 132, 136), (118, 146, 148))
    mid = ((40, 74, 66), (52, 90, 78), (66, 106, 90))
    near = ((18, 44, 36), (26, 58, 46), (36, 72, 56))
    for x in range(1, c.w - 1, 3):
        pine(c, x, 30 + int(c.rnd(x, 1) * 2), 6 + int(c.rnd(x, 2) * 4), far, snow=True)
    # snow ground
    snow = pal("#b8c8dc", "#ccd8e6", "#dfe7f0", "#f0f4f8")
    for x in range(c.w):
        top = 34 + int(round(1.5 * math.sin(x / 6.0) + math.sin(x / 2.5) * 0.5))
        for y in range(top, c.h):
            c.put(x, y, c.dpick(snow, 1 - (y - top) / 14, x, y))
    for x in range(3, c.w - 2, 7):
        pine(c, x + int(c.rnd(x, 3) * 3), 37, 12 + int(c.rnd(x, 4) * 6), mid, snow=True)
    for x in (8, 27, 49, 60):
        pine(c, x, 44, 22 + int(c.rnd(x, 5) * 6), near, snow=True)
    # falling snow
    for k in range(70):
        x, y = 1 + int(c.rnd(k, 11) * 62), 1 + int(c.rnd(k, 12) * 46)
        c.put(x, y, (250, 252, 255) if k % 3 else (220, 230, 240))
    # a fox track across the snow
    for x in range(30, 46, 3):
        c.put(x, 42 + (x // 3) % 2, (150, 166, 188))


def volcano(c):
    sky = pal("#1a0e1a", "#2c1222", "#46162a", "#6a1e2c", "#922a2a", "#b8402a")
    c.vgrad(sky, 0, 48)
    c.rect(0, 48, 63, 63, sky[-1])
    # ash cloud
    for t in range(0, 320):
        a = c.rnd(t, 1) * math.tau
        r = c.rnd(t, 2)
        y = 17 - r * 15
        x = 32 + math.cos(a) * (4 + (17 - y) * 1.3) * r + (17 - y) * 0.6
        c.disc(x, y, 1.8 + r * 2.6, mix((44, 36, 40), (90, 76, 76), c.rnd(t, 3)))
    # cone
    rock = pal("#120c0c", "#1e1414", "#2c1e1a", "#3c2a22")
    for y in range(18, 64):
        half = 4 + (y - 18) * 0.75
        for x in range(int(32 - half), int(32 + half) + 1):
            t = 0.75 - (x - 32 + half) / (2 * half) * 0.7 + 0.1 * c.rnd(x, y)
            c.put(x, y, c.dpick(rock, t, x, y))
    # glowing crater
    c.rect(28, 18, 36, 19, (255, 196, 64))
    c.rect(29, 17, 35, 17, (255, 230, 120))
    # lava flows
    lava = [(255, 236, 140), (255, 180, 50), (232, 96, 30), (170, 40, 24)]
    for seed_x, drift in ((31, -0.35), (34, 0.45), (30, -0.8)):
        x = float(seed_x)
        for y in range(19, 62):
            x += drift * 0.5 + (c.rnd(int(x), y, 3) - 0.5) * 0.9
            w = 1 if y < 40 else 2
            for k in range(w):
                col = lava[min(3, (y - 19) // 12 + (k > 0))]
                c.put(int(x) + k, y, col)
    # sparks / bombs
    for k in range(26):
        a = -math.pi / 2 + (c.rnd(k, 5) - 0.5) * 2.2
        r = 3 + c.rnd(k, 6) * 14
        c.put(int(32 + math.cos(a) * r), int(16 + math.sin(a) * r * 0.9),
              lava[0] if k % 2 else lava[1])
    # dark rocky foreground with glowing cracks
    for x in range(c.w):
        top = 57 + int(round(1.5 * math.sin(x / 5.0) + 0.8 * math.sin(x / 2.1)))
        for y in range(top, c.h):
            c.put(x, y, c.dpick(rock, 0.55 - 0.5 * (y - top) / 7 + 0.15 * c.rnd(x, y, 4), x, y))
        c.put(x, top, rock[3])
    for sx, sy, ex, ey in ((8, 59, 14, 62), (14, 62, 20, 61), (40, 60, 46, 63), (46, 63, 54, 60)):
        c.line(sx, sy, ex, ey, (232, 96, 30))
    for x, y in ((14, 62), (46, 63), (20, 61)):
        c.put(x, y, (255, 196, 64))
    # where the flows meet the ground they pool and glow
    for x in range(c.w):
        for y in range(54, 60):
            if c.get(x, y) in (lava[1], lava[2], lava[3]) and y < c.h - 1:
                c.put(x, y + 1, lava[1])


PAINTINGS = [
    # (name, width, height, Portuguese title, painter)
    ("bowl_of_fruit", 1, 1, "Fruteira", bowl_of_fruit),
    ("ginger_cat", 1, 1, "Gato laranja na almofada", ginger_cat),
    ("abstract_tiles", 1, 1, "Composição abstrata", abstract_tiles),
    ("red_mushroom", 1, 1, "Cogumelo na floresta", red_mushroom),
    ("blocky_portrait", 1, 2, "Retrato de um amigo quadrado", portrait),
    ("lighthouse", 1, 2, "O farol ao anoitecer", lighthouse),
    ("sunset_mountains", 2, 1, "Montanhas ao pôr do sol", sunset_mountains),
    ("desert_cactus", 2, 1, "Cacto no deserto", desert_cactus),
    ("sailing_ship", 2, 1, "Veleiro em alto-mar", sailing_ship),
    ("starry_night", 2, 2, "Noite estrelada", starry_night),
    ("castle_on_hill", 2, 2, "Castelo na colina", castle_hill),
    ("underwater", 2, 2, "Fundo do mar", underwater),
    ("mountain_lake", 4, 2, "Lago entre montanhas", mountain_lake),
    ("sunflower_field", 4, 2, "Campo de girassóis", sunflower_field),
    ("snowy_pine_forest", 4, 3, "Pinheiral nevado", snowy_pine_forest),
    ("volcano", 4, 4, "O vulcão", volcano),
]


def canvas_back():
    c = Canvas("back", 1, 1)
    linen = pal("#b8a888", "#c6b694", "#d2c3a2", "#dccfb0")
    for y in range(16):
        for x in range(16):
            weave = 0.55 + (0.15 if (x + y) % 2 else -0.1) + 0.3 * (c.rnd(x, y) - 0.5)
            c.put(x, y, c.dpick(linen, weave, x, y, 0.5))
    wood = pal("#6e4a2a", "#80582f", "#946838", "#a67a44")
    bars = [(0, 0, 15, 2), (0, 13, 15, 15), (0, 0, 2, 15), (13, 0, 15, 15), (7, 3, 8, 12)]
    for (x0, y0, x1, y1) in bars:
        horizontal = (x1 - x0) > (y1 - y0)
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                g = c.rnd(y if horizontal else x, (x if horizontal else y) // 4, 21)
                t = 0.45 + 0.4 * (g - 0.5)
                if (horizontal and y == y0) or (not horizontal and x == x0):
                    t += 0.35
                if (horizontal and y == y1) or (not horizontal and x == x1):
                    t -= 0.35
                c.put(x, y, c.dpick(wood, t, x, y, 0.4))
    for x, y in ((1, 1), (14, 1), (1, 14), (14, 14), (7, 3), (8, 12)):
        c.put(x, y, (60, 60, 66))  # nails / staples
    c.put(7, 7, (90, 62, 36))  # wood knot
    path = os.path.join(OUT_DIR, "back.png")
    c.img.save(path)
    return path, c.img


def write_ron():
    lines = ["// Paintings of the base game. Textures live in textures/painting/<name>.png",
             "// at 16 pixels per block; width/height are in blocks.",
             "(paintings: ["]
    for name, w, h, title, _ in PAINTINGS:
        lines.append(f'    (name: "{name}", title: "{title}", width: {w}, height: {h}),')
    lines.append("])")
    with open(RON_PATH, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    return RON_PATH


def preview(images, back_img):
    scale, gap, max_w = 4, 12, 1100
    items = images + [("back", back_img)]
    x = y = gap
    row_h = 0
    places = []
    for name, img in items:
        w, h = img.width * scale, img.height * scale
        if x + w + gap > max_w:
            x = gap
            y += row_h + gap
            row_h = 0
        places.append((x, y, img))
        x += w + gap
        row_h = max(row_h, h)
    sheet = Image.new("RGB", (max_w, y + row_h + gap), (34, 32, 36))
    for x, y, img in places:
        sheet.paste(img.resize((img.width * scale, img.height * scale), Image.NEAREST), (x, y))
    path = os.path.join(OUT_DIR, "preview.png")
    sheet.save(path)
    return path


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    images = []
    for name, w, h, _title, fn in PAINTINGS:
        c = Canvas(name, w, h)
        fn(c)
        print("wrote", os.path.relpath(c.save(), ROOT))
        images.append((name, c.img))
    path, back = canvas_back()
    print("wrote", os.path.relpath(path, ROOT))
    print("wrote", os.path.relpath(write_ron(), ROOT))
    print("wrote", os.path.relpath(preview(images, back), ROOT))


if __name__ == "__main__":
    main()
