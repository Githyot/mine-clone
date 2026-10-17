"""Macaw's Doors, Fences and Trapdoors art: shoji, barn, glass, jail, modern and metal doors,
picket, metal and barbed-wire fences, ranch and glass trapdoors. Our own 16x16 pixels."""

import numpy as np

from .lib import BLOCKS, ITEMS, N, T, canvas, hexrgb, pal, put, rng_for

IRON = hexrgb("#46464a")
STEEL = pal("#4a4e56", "#6a707a", "#8c929c", "#aeb4bc", "#d0d4da")
GLASS = (200, 228, 240)


def planks_of(wood):
    return T.planks(wood)


def frame(img, c, top=True, bottom=True):
    img[:, 0, :3] = c
    img[:, N - 1, :3] = c
    if top:
        img[0, :, :3] = c
    if bottom:
        img[N - 1, :, :3] = c
    img[:, 0, 3] = img[:, N - 1, 3] = 255


def shoji(upper):
    img = canvas()
    wood = pal("#5a3a1e", "#7a5230")
    paper = hexrgb("#ece4d0")
    for y in range(N):
        for x in range(N):
            lattice = x in (0, 15, 5, 10) or y in (0, 15, 5, 10)
            put(img, x, y, wood[1] if lattice else paper)
    frame(img, wood[0], top=upper, bottom=not upper)
    if not upper:
        put(img, 13, 2, IRON)
    return img


def barn(upper):
    img = canvas()
    red = pal("#5e1c16", "#7e2a20", "#9a3628", "#b04432")
    white = hexrgb("#e2ddd2")
    rng = rng_for("barn_door" + str(upper))
    for y in range(N):
        for x in range(N):
            put(img, x, y, red[1 + int(rng.integers(0, 3))] if x % 4 else red[0])
    # The white Z brace.
    for k in range(N):
        put(img, k, k, white)
        put(img, k, (k + 1) % N if k < 15 else 15, white)
    for x in range(N):
        put(img, x, 0 if upper else 15, white)
        put(img, x, 1 if upper else 14, white)
    frame(img, white, top=upper, bottom=not upper)
    if not upper:
        put(img, 13, 3, IRON)
        put(img, 13, 4, IRON)
    return img


def glass_door(upper):
    img = canvas()
    oak = pal("#5a4229", "#7b5e3c", "#9a7744")
    for y in range(N):
        for x in range(N):
            edge = x in (0, 1, 14, 15) or y in ((0, 1) if upper else (14, 15))
            if edge:
                put(img, x, y, oak[1 if (x + y) % 5 else 0])
            else:
                put(img, x, y, GLASS, 110)
    # A shine streak.
    for k in range(3, 8):
        put(img, k + 2, 12 - k, (250, 252, 255), 200)
    if not upper:
        for y in range(12, 14):
            for x in range(2, 14):
                put(img, x, y, oak[2])
        put(img, 12, 2, IRON)
    return img


def jail(upper):
    img = canvas()
    for x in (1, 4, 7, 10, 13):
        for y in range(N):
            put(img, x, y, STEEL[2])
            put(img, x + 1, y, STEEL[1])
    for y in ((0, 1, 8) if upper else (8, 14, 15)):
        for x in range(N):
            put(img, x, y, STEEL[3] if y % 2 else STEEL[0])
    if not upper:
        for y in range(2, 5):
            for x in range(11, 15):
                put(img, x, y, STEEL[0])
        put(img, 12, 3, hexrgb("#c8a040"))
    return img


def modern(upper):
    img = canvas()
    dark = T.WOODS["dark_oak"]["plank"]
    rng = rng_for("modern_door" + str(upper))
    for y in range(N):
        for x in range(N):
            put(img, x, y, dark[2 + int(rng.integers(0, 2))] if y % 5 else dark[1])
    # A narrow window strip and a long steel handle.
    for y in range(N):
        put(img, 3, y, GLASS, 140)
        put(img, 4, y, GLASS, 140)
    frame(img, dark[0], top=upper, bottom=not upper)
    if upper:
        for y in range(9, 16):
            put(img, 12, y, STEEL[4])
    else:
        for y in range(0, 6):
            put(img, 12, y, STEEL[4])
    return img


def metal(upper):
    img = canvas()
    for y in range(N):
        for x in range(N):
            put(img, x, y, STEEL[2])
    for (x0, y0, x1, y1) in ((2, 2, 13, 7), (2, 9, 13, 13)) if upper else ((2, 1, 13, 6), (2, 8, 13, 13)):
        for x in range(x0, x1 + 1):
            put(img, x, y0, STEEL[4])
            put(img, x, y1, STEEL[0])
        for y in range(y0, y1 + 1):
            put(img, x0, y, STEEL[4])
            put(img, x1, y, STEEL[0])
    frame(img, STEEL[1], top=upper, bottom=not upper)
    for y in ((1, 14) if upper else (1, 14)):
        put(img, 1, y, STEEL[0])
    if not upper:
        put(img, 12, 3, STEEL[0])
        put(img, 12, 4, STEEL[0])
    return img


STYLES = {"shoji_door": shoji, "barn_door": barn, "glass_door": glass_door, "jail_door": jail,
          "modern_door": modern, "metal_door": metal}


def door_item(fn):
    img = canvas()
    both = np.concatenate([fn(True), fn(False)], axis=0)
    small = both[::2, ::2]  # 8 x 16 -> 16 rows of 8 columns
    small = both[np.arange(0, 32, 2)][:, np.arange(0, 16, 2)]
    img[:, 4:12] = small
    return img


for _n, _fn in STYLES.items():
    BLOCKS[f"{_n}_upper"] = lambda f=_fn: f(True)
    BLOCKS[f"{_n}_lower"] = lambda f=_fn: f(False)
    ITEMS[_n] = lambda f=_fn: door_item(f)


def picket_planks():
    rng = rng_for("picket_planks")
    p = pal("#b8b4aa", "#cfcbc0", "#e2ded4", "#f0ece4")
    img = canvas()
    for y in range(N):
        for x in range(N):
            put(img, x, y, p[0] if x % 4 == 0 else p[1 + int(rng.integers(0, 3))])
    return img


def metal_fence():
    img = canvas()
    for y in range(N):
        for x in range(N):
            put(img, x, y, STEEL[1 + (x + y) % 3] if (x // 2 + y // 2) % 2 else STEEL[2])
    T.bevel(img, 1.15, 0.75)
    return img


def wired_fence():
    img = T.planks("spruce")
    # Strands of wire with barbs across the wood.
    for y in (4, 11):
        for x in range(N):
            put(img, x, y, STEEL[3] if x % 2 else STEEL[1])
        for x in range(1, N, 4):
            put(img, x, y - 1, STEEL[4])
            put(img, x + 1, y + 1, STEEL[4])
    return img


BLOCKS["picket_planks"] = picket_planks
BLOCKS["metal_fence"] = metal_fence
BLOCKS["wired_fence"] = wired_fence


def ranch_trapdoor():
    img = canvas()
    p = T.WOODS["oak"]["plank"]
    for y in range(N):
        for x in range(N):
            rail = y in (1, 2, 7, 8, 13, 14) or x in (0, 1, 14, 15) or abs(x - y) <= 0
            if rail:
                put(img, x, y, p[2 if (x + y) % 3 else 1])
    return img


def glass_trapdoor():
    img = canvas()
    p = T.WOODS["oak"]["plank"]
    for y in range(N):
        for x in range(N):
            if x in (0, 1, 14, 15) or y in (0, 1, 14, 15) or x in (7, 8) or y in (7, 8):
                put(img, x, y, p[1 if (x + y) % 4 else 0])
            else:
                put(img, x, y, GLASS, 110)
    return img


BLOCKS["ranch_trapdoor"] = ranch_trapdoor
BLOCKS["glass_trapdoor"] = glass_trapdoor
