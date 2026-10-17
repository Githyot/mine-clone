"""Aquaculture art: the modular rod with its reel, hooks, lines, baits, seven more fish raw and
cooked, fillets and some junk from the water. Our own 16x16 pixels."""

import numpy as np

from .lib import ITEMS, canvas, hexrgb, line, outline, pal, put, rng_for

STEEL = pal("#3e424a", "#646a74", "#8c929c", "#c0c6ce")
GOLD = pal("#7a5508", "#c08a12", "#eab92a", "#fff6a8")
WOOD = pal("#3d2812", "#5a3d1e", "#7a5530", "#94693c")


def modular_fishing_rod():
    img = canvas()
    # Dark composite rod with cork grip, a reel and guide rings.
    for k in range(13):
        x, y = 1 + k, 14 - k
        put(img, x, y, WOOD[2] if k < 4 else hexrgb("#2a2e36") if k % 3 else hexrgb("#4a5260"))
    for (x, y) in ((4, 9), (5, 9), (4, 10), (5, 10)):
        put(img, x, y, STEEL[2])
    put(img, 4, 11, STEEL[0])
    put(img, 6, 10, STEEL[3])
    for (x, y) in ((8, 6), (11, 3)):
        put(img, x + 1, y + 1, STEEL[3])
    for y in range(2, 12):
        put(img, 14, y, (210, 210, 205))
    put(img, 14, 12, STEEL[1])
    return outline(img)


def hook(p, double=False, heavy=False):
    img = canvas()
    shank = [(9, 2), (9, 3), (9, 4), (9, 5), (9, 6), (9, 7), (9, 8), (9, 9), (9, 10)]
    curve = [(8, 11), (7, 12), (6, 12), (5, 11), (4, 10), (4, 9), (5, 8)]
    for (x, y) in shank + curve:
        put(img, x, y, p[2])
        if heavy:
            put(img, x + 1, y, p[1])
    put(img, 8, 2, p[3])
    put(img, 10, 2, p[3])
    put(img, 9, 1, p[3])
    if double:
        for (x, y) in [(10, 11), (11, 12), (12, 12), (13, 11), (14, 10), (14, 9), (13, 8)]:
            put(img, x, y, p[2])
    if heavy:
        for y in range(5, 8):
            for x in range(8, 12):
                put(img, x, y, hexrgb("#55585e"))
    return outline(img)


def spool(c):
    img = canvas()
    for y in range(4, 13):
        put(img, 4, y, WOOD[1])
        put(img, 11, y, WOOD[1])
    for y in range(5, 12):
        for x in range(5, 11):
            put(img, x, y, c if (y % 2) else np.clip(np.array(c) - 30, 0, 255))
    return outline(img)


def worm():
    img = canvas()
    pts = [(3, 10), (4, 9), (5, 9), (6, 10), (7, 11), (8, 11), (9, 10), (10, 9), (11, 8), (12, 8), (13, 9)]
    for i, (x, y) in enumerate(pts):
        c = hexrgb("#c86a6a") if i % 3 else hexrgb("#a04a50")
        put(img, x, y, c)
        put(img, x, y + 1, hexrgb("#8a3a40"))
    return outline(img)


def fish_bait():
    img = canvas()
    rng = rng_for("fish_bait")
    for y in range(6, 12):
        for x in range(4, 12):
            if (x - 7.5) ** 2 / 16 + (y - 8.5) ** 2 / 9 < 1:
                put(img, x, y, pal("#6a2a20", "#8a3c2a", "#a85a3a", "#c87a52")[int(rng.integers(0, 4))])
    return outline(img)


def fish(body, belly, fin, spots=(), long=False):
    img = canvas()
    x0, x1 = (3, 14) if long else (4, 13)
    for x in range(x0, x1):
        t = (x - x0) / (x1 - x0 - 1)
        half = 1 + int(3.2 * (1 - (2 * t - 1) ** 2) ** 0.5 + 0.3)
        for y in range(8 - half, 8 + half):
            put(img, x, y, belly if y >= 8 else body)
    for k in range(4):
        put(img, x0 - 1 - k // 2, 7 - k, fin)
        put(img, x0 - 1 - k // 2, 8 + k, fin)
    for x in range(x0 + 3, x0 + 6):
        put(img, x, 5 if long else 4, fin)
    put(img, x1 - 3, 6, (20, 20, 24))
    for (x, y) in spots:
        put(img, x, y, fin)
    return outline(img)


def cooked(img):
    out = img.copy()
    a = out[..., 3] > 0
    lum = out[..., :3].mean(axis=2)
    out[..., 0] = np.where(a, np.minimum(255, lum * 0.9 + 60), 0)
    out[..., 1] = np.where(a, lum * 0.62 + 30, 0)
    out[..., 2] = np.where(a, lum * 0.3 + 12, 0)
    return out


FISH = {
    "bass": ((70, 96, 52), (200, 196, 150), (50, 70, 36), [(6, 7), (8, 7), (10, 7)], False),
    "carp": ((150, 120, 60), (220, 196, 120), (120, 80, 40), [(7, 6), (9, 6)], False),
    "bluegill": ((60, 90, 120), (230, 160, 70), (40, 60, 90), [(6, 6), (8, 6)], False),
    "halibut": ((120, 104, 84), (230, 226, 214), (90, 78, 62), [(6, 5), (9, 6), (11, 5)], True),
    "pollock": ((100, 110, 80), (214, 214, 196), (70, 80, 60), [], True),
    "grouper": ((130, 80, 60), (200, 150, 110), (90, 54, 40), [(6, 6), (8, 5), (10, 6), (7, 9), (9, 9)], False),
    "gar": ((96, 110, 70), (200, 196, 160), (70, 80, 50), [(5, 7), (7, 7), (9, 7), (11, 7)], True),
}
for _n, (_b, _be, _f, _s, _l) in FISH.items():
    ITEMS[_n] = lambda b=_b, be=_be, f=_f, s=_s, l=_l: fish(b, be, f, s, l)
    ITEMS[f"cooked_{_n}"] = lambda b=_b, be=_be, f=_f, s=_s, l=_l: cooked(fish(b, be, f, s, l))


def fillet(cook):
    img = canvas()
    for y in range(5, 12):
        for x in range(3, 14):
            if (x - 8.5) ** 2 / 30 + (y - 8.5) ** 2 / 9 < 1:
                c = (238, 150, 120) if not cook else (196, 130, 70)
                if (x + y) % 4 == 0:
                    c = (250, 200, 180) if not cook else (150, 90, 40)
                put(img, x, y, c)
    return outline(img)


def driftwood():
    img = canvas()
    for k in range(12):
        x, y = 2 + k, 11 - k // 2
        put(img, x, y, hexrgb("#9a948a"))
        put(img, x, y + 1, hexrgb("#7a746a"))
    put(img, 6, 8, hexrgb("#9a948a"))
    put(img, 5, 7, hexrgb("#9a948a"))
    return outline(img)


def tin_can():
    img = canvas()
    for y in range(4, 13):
        for x in range(5, 11):
            put(img, x, y, STEEL[1 + (x == 6) + (x == 7)] if y not in (4, 12) else STEEL[3])
    for x in range(5, 11):
        put(img, x, 8, hexrgb("#a83a30"))
    return outline(img)


ITEMS["modular_fishing_rod"] = modular_fishing_rod
ITEMS["iron_hook"] = lambda: hook(STEEL)
ITEMS["gold_hook"] = lambda: hook(GOLD)
ITEMS["heavy_hook"] = lambda: hook(STEEL, heavy=True)
ITEMS["double_hook"] = lambda: hook(STEEL, double=True)
ITEMS["fishing_line"] = lambda: spool(hexrgb("#e8e8e0"))
ITEMS["strong_line"] = lambda: spool(hexrgb("#4a8a5a"))
ITEMS["worm"] = worm
ITEMS["fish_bait"] = fish_bait
ITEMS["fish_fillet"] = lambda: fillet(False)
ITEMS["cooked_fish_fillet"] = lambda: fillet(True)
ITEMS["driftwood"] = driftwood
ITEMS["tin_can"] = tin_can
