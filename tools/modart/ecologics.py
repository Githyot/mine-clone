"""Ecologics art: coconut palm, walnut and maple woods (maple with its orange leaves), the
coconut, walnuts and thin ice in three stages of cracking. Our own 16x16 pixels."""

import numpy as np

from .lib import BLOCKS, ITEMS, I, N, T, canvas, from_map, hexrgb, line, outline, pal, pmap, put, rng_for

T.WOODS["coconut"] = dict(
    bark=pal("#5a4630", "#6e583c", "#826a48", "#967c56", "#a88e66"),
    inner=pal("#c8ae80", "#d6bc8e", "#e2ca9c", "#ecd6aa"),
    plank=pal("#9a7a52", "#ae8c60", "#c09c6c", "#cdaa78", "#d8b684"),
    leaves=pal("#2a6a1e", "#368228", "#449a32", "#56b03e", "#6cc44c"),
)
T.WOODS["walnut"] = dict(
    bark=pal("#2a1e16", "#36281c", "#443224", "#523e2c", "#604a34"),
    inner=pal("#7a5a3a", "#8c6a46", "#9c7a52", "#aa885e"),
    plank=pal("#4a3220", "#5a3e28", "#6a4a30", "#785638", "#846040"),
    leaves=pal("#2e5418", "#3a681e", "#487c26", "#589030", "#6aa43c"),
)
T.WOODS["maple"] = dict(
    bark=pal("#3a2c22", "#4a3a2e", "#5a483a", "#6a5646", "#7a6452"),
    inner=pal("#c09a6a", "#ceaa78", "#dab886", "#e4c494"),
    plank=pal("#a8724a", "#ba8256", "#c99262", "#d6a06e", "#e0ac7a"),
    leaves=pal("#8a2a0a", "#b03e10", "#d45a16", "#ec7a1e", "#f8a02c"),
)

for _w in ("coconut", "walnut", "maple"):
    BLOCKS[f"{_w}_log"] = lambda w=_w: T.log_side(w)
    BLOCKS[f"{_w}_log_top"] = lambda w=_w: T.log_top(w)
    BLOCKS[f"{_w}_planks"] = lambda w=_w: T.planks(w)
    BLOCKS[f"{_w}_leaves"] = lambda w=_w: T.leaves(w)
    BLOCKS[f"{_w}_sapling"] = lambda w=_w: I.sapling(w)


def door_parts(wood):
    """Lower and upper door halves and the trapdoor, coloured from the wood's planks."""
    p = T.WOODS[wood]["plank"]
    edge = np.clip(np.array(p[0]) - 30, 0, 255)
    iron = hexrgb("#46464a")

    def board(window=False):
        img = canvas()
        for y in range(N):
            for x in range(N):
                frame = x in (0, 15) or y in (0, 15)
                seam = x in (5, 10)
                c = edge if frame else (p[1] if seam else (p[4] if (x + y * 3) % 7 == 0 else p[2]))
                put(img, x, y, c)
        if window:
            for y in range(2, 7):
                for x in range(2, 14):
                    if x in (7, 8) or y == 4:
                        put(img, x, y, edge)
                    else:
                        img[y, x] = 0
        return img

    lower = board()
    for y in (5, 11):
        put(lower, 1, y, iron)
        put(lower, 2, y, iron)
    put(lower, 13, 2, iron)
    put(lower, 13, 3, iron)
    upper = board(window=True)
    put(upper, 1, 10, iron)
    put(upper, 2, 10, iron)
    trap = board()
    for y in range(2, 14):
        for x in range(2, 14):
            if x in (4, 5, 10, 11) and y not in (7, 8):
                img_clear(trap, x, y)
    return lower, upper, trap


def img_clear(img, x, y):
    img[y, x] = 0


def door_item(lower, upper):
    img = canvas()
    both = np.concatenate([upper, lower], axis=0)
    img[:, 4:12] = both[np.arange(0, 32, 2)][:, np.arange(0, 16, 2)]
    return img


for _w in ("coconut", "walnut", "maple"):
    BLOCKS[f"{_w}_door_lower"] = lambda w=_w: door_parts(w)[0]
    BLOCKS[f"{_w}_door_upper"] = lambda w=_w: door_parts(w)[1]
    BLOCKS[f"{_w}_trapdoor"] = lambda w=_w: door_parts(w)[2]
    ITEMS[f"{_w}_door"] = lambda w=_w: door_item(*door_parts(w)[:2])

HUSK = pal("#3a2614", "#523618", "#6a4620", "#82582a", "#9a6c36")


def coconut():
    rng = rng_for("coconut")
    img = canvas()
    for y in range(N):
        for x in range(N):
            v = rng.random()
            put(img, x, y, HUSK[1 + int(v * 3.9)])
            if (x * 7 + y * 3) % 11 == 0:
                put(img, x, y, HUSK[0])
    # Three dark eyes.
    for (x, y) in ((6, 6), (9, 6), (7, 9)):
        put(img, x, y, hexrgb("#24160a"))
    return img


BLOCKS["coconut"] = coconut


def coconut_slice():
    rows = [
        "................",
        "................",
        "................",
        "................",
        "......1111......",
        "....11555511....",
        "...1554444551...",
        "..154433334451..",
        "..154333333451..",
        "..154433334451..",
        "...1554444551...",
        "....11555511....",
        "......1111......",
        "................",
        "................",
        "................",
    ]
    p = pmap(pal("#4a3018", "#6a4620", "#e8e2d2", "#f4f0e6", "#8a6234"))
    img = from_map(rows, p)
    # Cut in half: drop the lower half for a wedge look.
    img[9:] = 0
    return outline(img)


def walnut():
    rows = [
        "................",
        "................",
        "................",
        "................",
        "......2222......",
        ".....234432.....",
        "....23455432....",
        "....24533542....",
        "....23455432....",
        "....24533542....",
        "....23455432....",
        ".....234432.....",
        "......2222......",
        "................",
        "................",
        "................",
    ]
    return outline(from_map(rows, pmap(pal("#3a2614", "#5a3c20", "#7a5430", "#9a6e42", "#b88a58"))))


def walnut_cookie():
    rows = [
        "................",
        "................",
        "................",
        "................",
        ".....333333.....",
        "....34444443....",
        "...3441444143...",
        "...3444444443...",
        "...3414441443...",
        "...3444444443...",
        "....34414443....",
        ".....333333.....",
        "................",
        "................",
        "................",
        "................",
    ]
    return outline(from_map(rows, pmap(pal("#5a3c20", "#3a2614", "#a8743c", "#d49a56"))))


ITEMS["coconut_slice"] = coconut_slice
ITEMS["walnut"] = walnut
ITEMS["walnut_cookie"] = walnut_cookie


def thin_ice(stage):
    img = T.ice() if callable(getattr(T, "ice", None)) else canvas()
    img = img.copy()
    img[..., 3] = 150
    rng = rng_for(f"thin_ice_{stage}")
    crack = hexrgb("#f4fbff")
    for _ in range(stage * 3):
        x, y = rng.integers(1, N - 1, 2)
        dx, dy = rng.integers(-5, 6, 2)
        line(img, x, y, int(np.clip(x + dx, 0, 15)), int(np.clip(y + dy, 0, 15)), crack)
    return img


BLOCKS["thin_ice"] = lambda: thin_ice(0)
BLOCKS["thin_ice_1"] = lambda: thin_ice(1)
BLOCKS["thin_ice_2"] = lambda: thin_ice(2)
