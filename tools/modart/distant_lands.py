"""Distant Lands' art: the ipê, araucária, buriti and jabuticaba woods, the blossoms of the ipês,
the fruit clusters, the plants of the Brazilian biomes, their soils, the carpenter table and the
fruit items. Our own 16x16 pixels, in the base game's style."""

import numpy as np

from .lib import BLOCKS, ITEMS, I, N, T, block, canvas, fbm, from_map, from_rgb, hexrgb, item, line, outline, pal, pmap, put, rng_for, shade, value_noise

# Bark, heartwood, planks and (green) leaves of each wood.
T.WOODS["ipe"] = dict(
    bark=pal("#2e2620", "#3e342a", "#4e4234", "#5e5040", "#6e604e"),
    inner=pal("#7a4a2a", "#8c5832", "#9c663c", "#aa7446"),
    plank=pal("#5a3420", "#6e4228", "#7e4e30", "#8c5a38", "#9a6640"),
    leaves=pal("#2e5a1e", "#3a6e26", "#48822e", "#58963a", "#6aa846"),
)
T.WOODS["araucaria"] = dict(
    bark=pal("#3a2418", "#4c3020", "#5e3c28", "#704a32", "#82583c"),
    inner=pal("#b88a5a", "#c89a66", "#d6a872", "#e2b67e"),
    plank=pal("#9a6a40", "#ae7a4a", "#be8854", "#cc965e", "#d8a26a"),
    leaves=pal("#163a22", "#1e4a2a", "#285c34", "#326c3c", "#3e7c46"),
)
T.WOODS["buriti"] = dict(
    bark=pal("#4a4236", "#5c5244", "#6e6452", "#80765f", "#92886c"),
    inner=pal("#c2a878", "#d0b886", "#dcc694", "#e8d4a2"),
    plank=pal("#9e8a62", "#b29c70", "#c2ac7c", "#d0ba88", "#dcc694"),
    leaves=pal("#3a6a24", "#4a7e2c", "#5a9234", "#6ca63e", "#80b84a"),
)
T.WOODS["jabuticaba"] = dict(
    bark=pal("#6a5a4a", "#80705c", "#96846c", "#aa987e", "#beac92"),
    inner=pal("#b07a5a", "#c08866", "#cc9672", "#d8a47e"),
    plank=pal("#8a5a44", "#9c6850", "#ac765a", "#ba8464", "#c6906e"),
    leaves=pal("#1e4a1c", "#285c24", "#346e2c", "#408034", "#4e923e"),
)
BLOSSOM = {
    "ipe_pink": pal("#a8306a", "#c84686", "#dc64a0", "#ec88ba", "#f8b0d4"),
    "ipe_yellow": pal("#b07a08", "#d09a10", "#e8b81e", "#f6d23a", "#fce878"),
    "ipe_purple": pal("#4a2470", "#62348e", "#7c48aa", "#9864c4", "#b88ade"),
}
for _k, _b in BLOSSOM.items():
    # Saplings of the ipês: a bud of their colour on ipê bark.
    T.WOODS[_k] = dict(T.WOODS["ipe"], leaves=_b)

for _w in ("ipe", "araucaria", "buriti", "jabuticaba"):
    BLOCKS[f"{_w}_log"] = (lambda w=_w: T.log_side(w))
    BLOCKS[f"{_w}_log_top"] = (lambda w=_w: T.log_top(w))
    BLOCKS[f"{_w}_planks"] = (lambda w=_w: T.planks(w))
for _w in ("araucaria", "buriti", "jabuticaba", "ipe_pink", "ipe_yellow", "ipe_purple"):
    BLOCKS[f"{_w}_sapling"] = (lambda w=_w: I.sapling(w))


def ipe_leaves(kind):
    """A crown so full of flowers that the green barely shows."""
    rng = rng_for(kind + "_leaves")
    p = BLOSSOM[kind]
    f = fbm(rng, ((4, 0.35), (2, 0.4), (1, 0.45)))
    img = from_rgb(shade(f, p, 1.0))
    green = T.WOODS["ipe"]["leaves"]
    for _ in range(10):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, green[int(rng.integers(1, 4))])
    for _ in range(8):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, p[4])
    holes = (f < 0.2) & (rng.random((N, N)) < 0.7)
    img[holes] = 0
    return img


for _k in BLOSSOM:
    BLOCKS[f"{_k}_leaves"] = (lambda k=_k: ipe_leaves(k))


@block("araucaria_leaves")
def araucaria_leaves():
    """Stiff, dark tufts of needles pointing up and out."""
    rng = rng_for("araucaria_leaves")
    p = T.WOODS["araucaria"]["leaves"]
    img = canvas()
    for _ in range(26):
        x, y = rng.integers(0, N, 2)
        n = int(rng.integers(3, 6))
        c = p[int(rng.integers(1, 5))]
        for i in range(n):
            put(img, x + (i if rng.random() < 0.5 else 0), y - i, c)
        put(img, x, y, p[0])
    fill = (img[..., 3] == 0) & (rng.random((N, N)) < 0.45)
    img[fill, :3] = p[0]
    img[fill, 3] = 255
    return img


@block("buriti_leaves")
def buriti_leaves():
    """Fan palm fronds: long pale-ribbed blades with gaps between them."""
    rng = rng_for("buriti_leaves")
    p = T.WOODS["buriti"]["leaves"]
    img = canvas()
    for k in range(6):
        x0 = k * 3 - 1 + int(rng.integers(0, 2))
        for y in range(N):
            x = x0 + (y // 5)
            put(img, x, y, p[2 + (y % 3 == 0)])
            put(img, x + 1, y, p[1])
            if y % 4 == 1:
                put(img, x + 1, y, p[4])
    return img


@block("jabuticaba_leaves")
def jabuticaba_leaves():
    img = T.leaves("jabuticaba")
    rng = rng_for("jabuticaba_leaves_shine")
    p = T.WOODS["jabuticaba"]["leaves"]
    for _ in range(7):
        x, y = rng.integers(0, N, 2)
        if img[y % N, x % N, 3]:
            put(img, x, y, p[4])
    return img


def cluster(name, berry, stem, n, size):
    """Round fruit hanging off a short stem (on a trunk or under a crown)."""
    rng = rng_for(name)
    img = canvas()
    line(img, 8, 0, 8, 4, stem)
    spots = [(8, 6), (6, 8), (10, 8), (8, 10), (5, 11), (11, 11), (7, 12), (9, 13), (6, 5), (10, 5)][:n]
    for x, y in spots:
        for dx in range(size):
            for dy in range(size):
                put(img, x + dx - size // 2, y + dy - size // 2, berry[1 + (dx + dy == 0) * 2 if size > 1 else 2])
        put(img, x - size // 2, y - size // 2, berry[3])
        if rng.random() < 0.5:
            put(img, x + size // 2, y + size // 2, berry[0])
    return img


BLOCKS["jabuticaba_cluster"] = lambda: cluster("jabuticaba_cluster", pal("#0e0814", "#1e1028", "#3a2448", "#7a5a96"), hexrgb("#5a6a2a"), 8, 2)
BLOCKS["acai_cluster"] = lambda: cluster("acai_cluster", pal("#120a1c", "#24123a", "#3c2060", "#6a4a90"), hexrgb("#8a6a3a"), 10, 2)
BLOCKS["buriti_cluster"] = lambda: cluster("buriti_cluster", pal("#4a1e0a", "#7a3412", "#a24a1a", "#d07a3a"), hexrgb("#6a5a3a"), 5, 3)


def rosette(name, leaf, heart):
    """A bromeliad: stiff leaves fanning out of a coloured heart."""
    img = canvas()
    for (x0, y0, x1, y1, c) in ((8, 14, 1, 7, 1), (8, 14, 15, 7, 1), (8, 14, 3, 4, 2), (8, 14, 13, 4, 2),
                                (8, 14, 6, 3, 3), (8, 14, 10, 3, 3), (7, 15, 2, 11, 0), (9, 15, 14, 11, 0)):
        line(img, x0, y0, x1, y1, leaf[c])
    for (x, y) in ((7, 7), (8, 6), (9, 7), (8, 8), (7, 9), (9, 9), (8, 10), (8, 4)):
        put(img, x, y, heart[(x + y) % len(heart)])
    return img


BLOCKS["bromelia_red"] = lambda: rosette("bromelia_red", pal("#2a5a1e", "#3a7028", "#4c8632", "#62a03e"), pal("#b81e1e", "#e03a2a", "#f06a3a"))
BLOCKS["bromelia_orange"] = lambda: rosette("bromelia_orange", pal("#3a5a1e", "#4c7028", "#5e8632", "#72a03e"), pal("#d0601a", "#f0882a", "#f8b040"))


@block("capim_dourado")
def capim_dourado():
    """Thin golden stems, each with a little button flower on top."""
    rng = rng_for("capim_dourado")
    img = canvas()
    gold = pal("#a87a10", "#c89a1e", "#e4bc34", "#f6dc6a")
    for k, x in enumerate((2, 4, 6, 8, 10, 12, 14)):
        top = int(rng.integers(2, 7))
        lean = int(rng.integers(-1, 2))
        for y in range(top, 16):
            put(img, x + (lean if y < 9 else 0), y, gold[1 + (k % 2)])
        put(img, x + lean, top - 1, gold[3])
        put(img, x + lean + 1, top - 1, gold[2])
        put(img, x + lean, top - 2, gold[3])
    return img


@block("jurema")
def jurema():
    """A thorny grey-brown shrub with a few small leaves (caatinga scrub)."""
    img = T.dead_bush()
    rng = rng_for("jurema")
    grey = pal("#5a5048", "#706458", "#867868")
    img[img[..., 3] > 0, :3] = grey[1]
    leaf = pal("#5a6a2a", "#6e7e34")
    for _ in range(9):
        x, y = rng.integers(2, 14), rng.integers(2, 12)
        if img[y, x, 3]:
            put(img, x + 1, y, leaf[int(rng.integers(0, 2))])
    for _ in range(8):
        x, y = rng.integers(2, 14), rng.integers(2, 14)
        if img[y, x, 3]:
            put(img, x, y - 1, hexrgb("#d8d0b8"))
    return img


@block("samambaia")
def samambaia():
    """A fern: arching fronds with paired leaflets."""
    img = canvas()
    p = pal("#1e5a1a", "#2a7022", "#38862c", "#4a9c38")
    for (x0, x1, top, c) in ((7, 2, 3, 2), (8, 13, 3, 2), (7, 4, 1, 3), (8, 11, 2, 3), (7, 1, 8, 1), (8, 14, 8, 1)):
        pts = []
        for y in range(15, top - 1, -1):
            t = (15 - y) / max(15 - top, 1)
            pts.append((round(x0 + (x1 - x0) * t * t), y))
        for i, (x, y) in enumerate(pts):
            put(img, x, y, p[c])
            if i % 2 == 0 and i > 2:
                put(img, x - 1, y - 1, p[c - 1])
                put(img, x + 1, y - 1, p[c - 1])
    return img


MANDA = pal("#1e5030", "#2a6640", "#367c4e", "#46925e", "#5aa870")


@block("mandacaru_side")
def mandacaru_side():
    """A ribbed blue-green column cactus with rows of spines on the ribs."""
    rng = rng_for("mandacaru_side")
    cols = np.array([0, 2, 3, 4, 3, 1, 2, 3, 4, 3, 1, 2, 3, 4, 2, 0])
    lvl = cols[None, :] + (rng.random((N, N)) < 0.15) * rng.choice([-1, 1], (N, N))
    img = from_rgb(MANDA[np.clip(lvl, 0, 4)])
    spine = hexrgb("#e8e2c8")
    for y in range(1, N, 3):
        for x in (3, 8, 13):
            put(img, x, y, spine)
    return img


@block("mandacaru_top")
def mandacaru_top():
    img = T.cactus_top()
    rgb = img[..., :3].astype(int)
    rgb[...] = np.clip(rgb * np.array([1.0, 0.95, 1.1]), 0, 255)
    img[..., :3] = rgb
    return img


@block("mandacaru_flower")
def mandacaru_flower():
    """The big white night flower, on a green tube."""
    img = canvas()
    for y in range(9, 16):
        put(img, 7, y, MANDA[2])
        put(img, 8, y, MANDA[1])
    p = pal("#d8d4c0", "#ece8d8", "#ffffff")
    for (x, y, c) in ((5, 5, 1), (6, 4, 2), (7, 3, 2), (8, 3, 2), (9, 4, 2), (10, 5, 1), (4, 6, 0), (11, 6, 0),
                      (5, 6, 2), (6, 5, 2), (9, 5, 2), (10, 6, 2), (6, 7, 1), (9, 7, 1), (7, 8, 1), (8, 8, 1)):
        put(img, x, y, p[c])
    for (x, y) in ((7, 5), (8, 5), (7, 6), (8, 6)):
        put(img, x, y, hexrgb("#f0d050"))
    return img


@block("aguape")
def aguape():
    """Water hyacinth: round glossy leaves round a spike of lilac flowers."""
    rng = rng_for("aguape")
    img = canvas()
    g = pal("#1e5a24", "#2a7030", "#3a863c", "#4e9c4a")
    for cx, cy in ((5, 5), (11, 4), (4, 11), (11, 11), (8, 8)):
        for dy in range(-2, 3):
            for dx in range(-2, 3):
                if dx * dx + dy * dy <= 5:
                    put(img, cx + dx, cy + dy, g[1 + int(rng.integers(0, 3))])
    for (x, y) in ((7, 7), (8, 7), (8, 8), (7, 9), (9, 8)):
        put(img, x, y, hexrgb("#a888d8"))
    put(img, 8, 9, hexrgb("#f0d050"))
    return img


@block("aguape_side")
def aguape_side():
    img = canvas()
    for x in range(N):
        put(img, x, 15, hexrgb("#2a7030"))
    return img


@block("red_soil")
def red_soil():
    rng = rng_for("red_soil")
    p = pal("#6e2a14", "#84361a", "#984420", "#a85228", "#b86234")
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.45))), p, 0.8))
    for _ in range(9):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, p[0] if rng.random() < 0.5 else hexrgb("#c8a080"))
    return img


@block("cracked_dirt")
def cracked_dirt():
    """Sun-baked ground split into plates."""
    rng = rng_for("cracked_dirt")
    p = pal("#7a6046", "#8e7254", "#a08462", "#b09470", "#bea47e")
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.4))), p[1:], 0.6))
    crack = p[0]
    for (x0, y0, x1, y1) in ((0, 4, 6, 6), (6, 6, 9, 0), (6, 6, 8, 12), (8, 12, 15, 10), (8, 12, 5, 15),
                             (15, 10, 12, 5), (12, 5, 9, 0), (0, 4, 3, 0), (0, 12, 5, 15)):
        line(img, x0, y0, x1, y1, crack)
    return img


def mud():  # mud is a base block now (mangrove swamps); kept for reference
    rng = rng_for("mud")
    p = pal("#4a3a2a", "#584634", "#66523e", "#745e48", "#806a52")
    img = from_rgb(shade(fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3))), p, 0.7))
    for _ in range(6):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, hexrgb("#6e6050"))
    return img


@block("carpenter_table_top")
def carpenter_table_top():
    """Ipê boards with a circular saw blade sunk in the middle."""
    img = T.planks("ipe")
    dark = hexrgb("#2a1a10")
    img[0, :, :3] = img[15, :, :3] = dark
    img[:, 0, :3] = img[:, 15, :3] = dark
    steel = pal("#5a5a60", "#8a8a92", "#b4b4bc", "#d8d8e0")
    for y in range(N):
        for x in range(N):
            d = ((x - 7.5) ** 2 + (y - 7.5) ** 2) ** 0.5
            if d < 4.6:
                put(img, x, y, steel[2 if d > 1.5 else 0])
            elif d < 5.4 and (x + y) % 2 == 0:
                put(img, x, y, steel[3])
    return img


@block("carpenter_table_side")
def carpenter_table_side():
    """A workbench side: a top board, legs, and a hand saw and square hung up."""
    img = T.planks("ipe")
    rgb = img[..., :3]
    top = pal("#3a2216", "#5a3420", "#7a4a2e")
    rgb[0:3] = top[1]
    rgb[0] = top[2]
    rgb[3] = top[0]
    rgb[3:, 0:2] = top[0]
    rgb[3:, 14:16] = top[0]
    rgb[15, :] = top[0]
    steel = pal("#6a6a72", "#a0a0a8", "#d0d0d8")
    for x in range(4, 11):
        put(img, x, 7, steel[2])
        put(img, x, 8, steel[1])
        if x % 2 == 0:
            put(img, x, 9, steel[0])
    for (x, y) in ((11, 6), (12, 6), (12, 7), (12, 8), (11, 9)):
        put(img, x, y, hexrgb("#c08a3a"))
    line(img, 4, 11, 11, 11, steel[1])
    line(img, 4, 11, 4, 13, steel[1])
    return img


# ---------------------------------------------------------------- items
def fruit(rows, p):
    return outline(from_map(rows, pmap(p)))


BERRY = [
    "................",
    "................",
    "................",
    "........1.......",
    ".......1........",
    "......2332......",
    ".....234432.....",
    ".....234552.....",
    ".....223432.....",
    "......2222......",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
]
ITEMS["jabuticaba"] = lambda: fruit(BERRY, pal("#5a6a2a", "#140a1e", "#2a1640", "#4a2e66", "#9a7ab8"))
ITEMS["acai"] = lambda: fruit([
    "................",
    "................",
    "......1.........",
    ".......1..1.....",
    "........11......",
    ".....33.1.33....",
    "....3453.3453...",
    "....3443.3443...",
    ".....33.33.33...",
    ".......3453.....",
    ".......3443.....",
    "........33......",
    "................",
    "................",
    "................",
    "................",
], pal("#8a6a3a", "#1a0e28", "#2a1640", "#4a2e70", "#8a6ab0"))
ITEMS["buriti_fruit"] = lambda: fruit([
    "................",
    "................",
    "................",
    ".......11.......",
    "......2332......",
    ".....234432.....",
    ".....245542.....",
    ".....234432.....",
    ".....243342.....",
    ".....234432.....",
    "......2332......",
    ".......22.......",
    "................",
    "................",
    "................",
    "................",
], pal("#6a5a3a", "#4a1e0a", "#7a3412", "#b0561e", "#e09048"))


def pinhao(p):
    return fruit([
        "................",
        "................",
        "................",
        "..........11....",
        ".........1221...",
        "........12331...",
        ".......12341....",
        "......12341.....",
        ".....12341......",
        "....12341.......",
        "...12331........",
        "...1221.........",
        "....11..........",
        "................",
        "................",
        "................",
    ], p)


ITEMS["pinhao"] = lambda: pinhao(pal("#3a1a10", "#6a3220", "#8a4628", "#b06a3a", "#d09060"))
ITEMS["cooked_pinhao"] = lambda: pinhao(pal("#3a2414", "#8a6a48", "#b8946a", "#d8b88a", "#f0dab0"))


def pot(fill):
    return fruit([
        "................",
        "................",
        "................",
        "................",
        "....55555555....",
        "...5444444445...",
        "...1333333331...",
        "...1322222231...",
        "...1322222231...",
        "....13222231....",
        "....13222231....",
        ".....133331.....",
        "......1111......",
        "................",
        "................",
        "................",
    ], fill)


ITEMS["acai_pulp"] = lambda: pot(pal("#5a3a1e", "#2a1640", "#4a2e70", "#6a4a90", "#9a7ab8"))
ITEMS["jabuticaba_jam"] = lambda: pot(pal("#4a5a6a", "#3a0e2a", "#5a1a40", "#d8e0e8", "#b0bcc8"))
