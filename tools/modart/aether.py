"""The Aether's art: pale blue-green grass, white holystone, skyroot and golden oak, the three
ores, clouds, the dungeon stones, the utility blocks, the portal, and the tools and armour of
the four tiers. Our own 16x16 pixels, in the base game's style."""

import numpy as np

from .lib import BLOCKS, ITEMS, I, N, T, block, canvas, fbm, from_map, from_rgb, hexrgb, item, outline, pal, pmap, put, rng_for, shade, value_noise

T.WOODS["skyroot"] = dict(
    bark=pal("#4a4a3e", "#5e5e4e", "#74745f", "#8a8a72", "#a0a086"),
    inner=pal("#b8b08a", "#c9c19a", "#d6cfaa", "#e2dcba"),
    plank=pal("#8e8a6a", "#a6a17e", "#b8b38e", "#c8c39e", "#d6d2ae"),
    leaves=pal("#2e7a5a", "#3a9270", "#4caa84", "#62c098", "#80d6b0"),
)
T.WOODS["golden_oak"] = dict(
    bark=pal("#5a3e14", "#74521c", "#8e6826", "#a88030", "#c49a3e"),
    inner=pal("#c69a3a", "#d8ae4a", "#e6c05c", "#f0d070"),
    plank=pal("#8e8a6a", "#a6a17e", "#b8b38e", "#c8c39e", "#d6d2ae"),
    leaves=pal("#a8800e", "#c89c1a", "#e0b82e", "#f0d04a", "#fae47a"),
)
I.MATERIALS["skyroot"] = pal("#6e6a4e", "#8e8a6a", "#a6a17e", "#c8c39e", "#e0dcc0")
I.MATERIALS["holystone"] = pal("#7a7a80", "#a2a2a8", "#c2c2c8", "#dcdce2", "#f4f4f8")
I.MATERIALS["zanite"] = pal("#3a1460", "#5e2a96", "#8048c8", "#a874ea", "#dcc4ff")
I.MATERIALS["gravitite"] = pal("#5a1450", "#962a86", "#c84ab4", "#ea7ad6", "#ffd0f4")
I.ARMOR_MATERIALS["zanite"] = I.MATERIALS["zanite"]
I.ARMOR_MATERIALS["gravitite"] = I.MATERIALS["gravitite"]
I.ARMOR_MATERIALS["valkyrie"] = pal("#6c7078", "#a4a8b0", "#d0d2d8", "#ecedf0", "#fff8d0")
I.ARMOR_MATERIALS["phoenix"] = pal("#7a1e08", "#b8380e", "#e8641a", "#f8a032", "#ffe08a")

HOLY = pal("#9a9aa2", "#b0b0b8", "#c4c4cc", "#d6d6de", "#e6e6ec", "#f4f4f8")
AGRASS = pal("#3e8a6e", "#4ea080", "#62b492", "#78c6a4", "#92d8b8", "#b0e8cc")
ADIRT = pal("#6e6a5a", "#827e6c", "#96927e", "#a8a490", "#bab6a2")


@block("holystone")
def holystone(name="holystone"):
    rng = rng_for(name)
    img = from_rgb(shade(fbm(rng, ((8, 0.25), (4, 0.35), (2, 0.35), (1, 0.3))), HOLY))
    for _ in range(6):
        x, y = rng.integers(0, N, 2)
        for i in range(int(rng.integers(2, 4))):
            put(img, x + i, y, HOLY[0])
    return img


@block("mossy_holystone")
def mossy_holystone():
    img = holystone("mossy_holystone")
    rng = rng_for("mossy_holystone_moss")
    moss = fbm(rng, ((4, 0.5), (2, 0.4))) > 0.55
    g = shade(rng.random((N, N)), pal("#3e7a4a", "#4e9058", "#62a868"), 0.5)
    img[moss, :3] = g[moss]
    return img


@block("holystone_bricks")
def holystone_bricks():
    img = holystone("holystone_bricks")
    rgb = img[..., :3]
    for y in (3, 7, 11, 15):
        rgb[y, :] = HOLY[0]
    for b in range(4):
        for x in ((b % 2) * 4 + 0, (b % 2) * 4 + 8):
            rgb[b * 4:b * 4 + 3, x % N] = HOLY[1]
    T.bevel(img, 1.05, 0.92)
    return img


@block("aether_dirt")
def aether_dirt(name="aether_dirt"):
    rng = rng_for(name)
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.45))), ADIRT, 0.8))
    for _ in range(10):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, ADIRT[0] if rng.random() < 0.6 else ADIRT[4])
    return img


@block("aether_grass_top")
def aether_grass_top():
    rng = rng_for("aether_grass_top")
    img = from_rgb(shade(fbm(rng, ((4, 0.25), (2, 0.35), (1, 0.5))), AGRASS, 0.9))
    for _ in range(12):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, AGRASS[5] if rng.random() < 0.5 else AGRASS[0])
    return img


@block("aether_grass_side")
def aether_grass_side():
    img = aether_dirt("aether_grass_side_dirt")
    rng = rng_for("aether_grass_side")
    for x in range(N):
        depth = 3 + int(rng.integers(0, 3))
        for y in range(depth):
            put(img, x, y, AGRASS[int(rng.integers(1, 5))])
    return img


@block("quicksoil")
def quicksoil():
    rng = rng_for("quicksoil")
    p = pal("#b89a5a", "#ccae68", "#dcc078", "#e8d08a", "#f2e0a2")
    f = 0.5 * fbm(rng, ((4, 0.4), (2, 0.6))) + 0.5 * rng.random((N, N))
    img = from_rgb(shade(f, p, 0.4))
    # Ripples, as if the sand were sliding.
    for y in (4, 10):
        for x in range(N):
            if (x + y) % 5 < 3:
                put(img, x, y + (x // 4) % 2, p[4])
    return img


@block("quicksoil_glass")
def quicksoil_glass():
    img = canvas()
    p = pal("#e6c46a", "#f2d888", "#fae8aa")
    img[..., :3] = p[1]
    img[..., 3] = 90
    img[0, :, :3] = p[0]
    img[-1, :, :3] = p[0]
    img[:, 0, :3] = p[0]
    img[:, -1, :3] = p[0]
    img[0, :, 3] = img[-1, :, 3] = img[:, 0, 3] = img[:, -1, 3] = 230
    for k in range(3):
        put(img, 3 + k, 3 + k, p[2], 200)
    return img


@block("icestone")
def icestone():
    img = holystone("icestone")
    rng = rng_for("icestone_ice")
    ice = pal("#6ab4dc", "#8ccaea", "#b4e0f6")
    m = fbm(rng, ((4, 0.5), (2, 0.5))) > 0.5
    img[m, :3] = shade(rng.random((N, N)), ice, 0.5)[m]
    return img


ORE_P = {
    "ambrosium": pal("#c8a018", "#f0cc30", "#fff07a", "#fffbe0"),
    "zanite": pal("#4a1c78", "#7038b0", "#a070e8"),
    "gravitite": pal("#7a1e6a", "#b03a9a", "#e470cc", "#ffd0f4"),
}
for _k, _p in ORE_P.items():
    BLOCKS[f"{_k}_ore"] = (lambda k=_k, p=_p: T.ore(k, holystone(k + "_ore_stone"), p))


@block("enchanted_gravitite")
def enchanted_gravitite():
    return T.metal_block("enchanted_gravitite", I.MATERIALS["gravitite"], facets=True)


@block("zanite_block")
def zanite_block():
    return T.metal_block("zanite_block", I.MATERIALS["zanite"], facets=True)


def cloud(name, p, alpha):
    rng = rng_for(name)
    f = fbm(rng, ((8, 0.3), (4, 0.4), (2, 0.3)))
    img = from_rgb(shade(f, p, 0.6))
    img[..., 3] = alpha
    return img


BLOCKS["cold_aercloud"] = lambda: cloud("cold_aercloud", pal("#d8e0ea", "#e6ecf4", "#f2f6fa", "#ffffff"), 210)
BLOCKS["blue_aercloud"] = lambda: cloud("blue_aercloud", pal("#6aa0de", "#84b4ea", "#a0c8f2", "#c4def8"), 210)
BLOCKS["golden_aercloud"] = lambda: cloud("golden_aercloud", pal("#e8c860", "#f0d67c", "#f6e29a", "#fff0c0"), 190)

for _w in ("skyroot", "golden_oak"):
    BLOCKS[f"{_w}_log"] = (lambda w=_w: T.log_side(w))
    BLOCKS[f"{_w}_log_top"] = (lambda w=_w: T.log_top(w))
    BLOCKS[f"{_w}_leaves"] = (lambda w=_w: T.leaves(w))
    BLOCKS[f"{_w}_sapling"] = (lambda w=_w: I.sapling(w))
BLOCKS["skyroot_planks"] = lambda: T.planks("skyroot")


@block("aether_tall_grass")
def aether_tall_grass():
    rng = rng_for("aether_tall_grass")
    img = canvas()
    for x in range(1, 15, 2):
        h = int(rng.integers(5, 12))
        for y in range(16 - h, 16):
            put(img, x + (1 if y < 16 - h // 2 and x % 4 == 1 else 0), y, AGRASS[int(rng.integers(1, 5))])
    return img


BLOCKS["purple_flower"] = lambda: T.flower("purple_flower", pal("#5a2a8a", "#7a3eb0", "#a066d8"), hexrgb("#f4e070"), 6, True)
BLOCKS["white_flower"] = lambda: T.flower("white_flower", pal("#c8ccd8", "#e2e6f0", "#ffffff"), hexrgb("#f4c040"), 7, False)


@block("blueberry_bush")
def blueberry_bush():
    rng = rng_for("blueberry_bush")
    img = canvas()
    leaf = pal("#2e6a4a", "#3a8058", "#4c9a6a")
    for y in range(5, 16):
        for x in range(2, 14):
            d = ((x - 7.5) / 6.5) ** 2 + ((y - 11) / 6) ** 2
            if d < 1 and rng.random() < 0.85:
                put(img, x, y, leaf[int(rng.integers(0, 3))])
    for x, y in ((5, 8), (9, 7), (7, 11), (11, 11), (4, 12), (8, 14)):
        put(img, x, y, hexrgb("#3a50c8"))
        put(img, x + 1, y, hexrgb("#6a84ea"))
    return img


def machine(name, top_col, glow):
    rng = rng_for(name)
    img = holystone(name)
    T.bevel(img, 1.1, 0.75)
    for y in range(5, 11):
        for x in range(5, 11):
            put(img, x, y, top_col[int(rng.integers(0, len(top_col)))])
    put(img, 7, 7, glow)
    put(img, 8, 8, glow)
    return img


BLOCKS["altar_top"] = lambda: machine("altar_top", pal("#5e2a96", "#8048c8"), hexrgb("#fff07a"))
BLOCKS["freezer_top"] = lambda: machine("freezer_top", pal("#6ab4dc", "#8ccaea"), hexrgb("#ffffff"))
BLOCKS["incubator_top"] = lambda: machine("incubator_top", pal("#b8380e", "#e8641a"), hexrgb("#ffe08a"))


def machine_side(name, band):
    img = holystone_bricks()
    for x in range(N):
        put(img, x, 6, band[0])
        put(img, x, 7, band[1])
    return img


BLOCKS["altar_side"] = lambda: machine_side("altar_side", pal("#5e2a96", "#8048c8"))
BLOCKS["freezer_side"] = lambda: machine_side("freezer_side", pal("#4a94bc", "#8ccaea"))
BLOCKS["incubator_side"] = lambda: machine_side("incubator_side", pal("#b8380e", "#e8641a"))


@block("aether_portal")
def aether_portal():
    """A pale blue shimmer, like sky seen through water."""
    rng = rng_for("aether_portal")
    p = pal("#2a6ab8", "#3a84d0", "#5aa0e4", "#84bcf0", "#b4d8fa", "#e6f4ff")
    img = canvas()
    img[..., 3] = 200
    for y in range(N):
        for x in range(N):
            v = 0.5 + 0.5 * np.sin(y * 0.9 + np.sin(x * 0.7) * 1.6) + rng.normal(0, 0.08)
            img[y, x, :3] = p[int(np.clip(v * 5.5, 0, 5))]
    return img


def dungeon_stone(name, p, light):
    rng = rng_for(name)
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.3))), p, 0.4))
    rgb = img[..., :3]
    rgb[0, :] = rgb[:, 0] = p[-1]
    rgb[-1, :] = rgb[:, -1] = p[0]
    # A carved rune in the middle; it glows in the light variants.
    rune = ["..#..", ".###.", "#.#.#", ".###.", "..#.."]
    c = hexrgb(light) if light else p[0]
    for j, row in enumerate(rune):
        for i, ch in enumerate(row):
            if ch == "#":
                put(img, 5 + i, 5 + j, c)
    return img


CARVED = pal("#5a5e62", "#6e7276", "#83878b", "#999da1", "#b0b4b8")
ANGELIC = pal("#b0aca0", "#c4c0b4", "#d6d2c6", "#e6e2d8", "#f4f2ea")
HELL = pal("#3a1a14", "#52241a", "#6c3022", "#86402c", "#a05436")
BLOCKS["carved_stone"] = lambda: dungeon_stone("carved_stone", CARVED, None)
BLOCKS["sentry_stone"] = lambda: dungeon_stone("sentry_stone", CARVED, "#5ab4ff")
BLOCKS["angelic_stone"] = lambda: dungeon_stone("angelic_stone", ANGELIC, None)
BLOCKS["light_angelic_stone"] = lambda: dungeon_stone("light_angelic_stone", ANGELIC, "#fff4b0")
BLOCKS["hellfire_stone"] = lambda: dungeon_stone("hellfire_stone", HELL, None)
BLOCKS["light_hellfire_stone"] = lambda: dungeon_stone("light_hellfire_stone", HELL, "#ffb040")

# Items.
ITEMS["ambrosium_shard"] = lambda: I.lump("ambrosium_shard", pal("#a8800e", "#e0b82e", "#f6dc5a", "#fff08a", "#fffbe0"))
ITEMS["zanite_gemstone"] = lambda: I.diamond(I.MATERIALS["zanite"])
ITEMS["golden_amber"] = lambda: I.lump("golden_amber", pal("#8a4a08", "#c07010", "#e8981e", "#f8c040", "#ffe48a"))
ITEMS["swet_ball"] = lambda: I.lump("swet_ball", pal("#1d4f90", "#3d78c4", "#5591da", "#74abea", "#c4e0ff"))
ITEMS["healing_stone"] = lambda: I.lump("healing_stone", pal("#6a8a7a", "#8aa898", "#a8c4b4", "#c4dccc", "#e4f4ea"))
ITEMS["skyroot_stick"] = I.stick
for _n, _c in (("bronze_key", "#b8843e"), ("silver_key", "#cfd2d7"), ("gold_key", "#eab92a")):
    ITEMS[_n] = (lambda c=_c: key(c))
for _m in ("skyroot", "holystone", "zanite", "gravitite"):
    for _k in ("pickaxe", "axe", "shovel", "sword"):
        ITEMS[f"{_m}_{_k}"] = (lambda m=_m, k=_k: I.tool(m, k))
for _m in ("zanite", "gravitite", "valkyrie", "phoenix"):
    for _k in ("helmet", "chestplate", "leggings", "boots"):
        ITEMS[f"{_m}_{_k}"] = (lambda m=_m, k=_k: I.armor(m, k))


def key(c):
    rows = [
        "................",
        "................",
        "..3333..........",
        ".3....3.........",
        ".3....3.........",
        ".3....3.........",
        "..3333333333333.",
        "............3.3.",
        "............3.3.",
        "................",
    ] + ["................"] * 6
    base = hexrgb(c)
    img = from_map(rows, {"3": base})
    return outline(img)


def recolor(img, p):
    """Map an image's brightness onto a palette, keeping its alpha."""
    out = img.copy()
    lum = img[..., :3].mean(axis=2) / 255.0
    idx = np.clip((lum * len(p)).astype(int), 0, len(p) - 1)
    out[..., :3] = np.array(p)[idx]
    return out


@item("moa_feather")
def moa_feather():
    return recolor(I.feather(), pal("#1e3e6e", "#2f5a91", "#3f72ad", "#5688c2", "#88b8e8"))


@item("phoenix_feather")
def phoenix_feather():
    return recolor(I.feather(), pal("#7a1e08", "#b8380e", "#e8641a", "#f8a032", "#ffe08a"))


@item("aechor_petal")
def aechor_petal():
    rows = [
        "................",
        "................",
        "......2233......",
        ".....223344.....",
        "....2233445.....",
        "....2333445.....",
        "...22334455.....",
        "...2333445......",
        "...233344.......",
        "....2334........",
        ".....23.........",
        "......1.........",
        ".......1........",
        "................",
        "................",
        "................",
    ]
    return outline(from_map(rows, pmap(pal("#3a7a3a", "#a8487e", "#c25c9a", "#e895c6", "#f4b4da"))))


def berry(p):
    rows = [
        "................",
        "................",
        "................",
        ".......11.......",
        "......1.........",
        ".....2332.......",
        "....234432......",
        "....234432......",
        "....223332......",
        ".....2222.......",
        "................",
    ] + ["................"] * 5
    return outline(from_map(rows, pmap(p)))


ITEMS["blue_berry"] = lambda: berry(pal("#3a7a3a", "#1e2a8a", "#3a50c8", "#6a84ea", "#b4c4ff"))
ITEMS["enchanted_berry"] = lambda: berry(pal("#3a7a3a", "#5a1e8a", "#8a3ac8", "#c06aea", "#f0c4ff"))


@item("moa_egg")
def moa_egg():
    img = canvas()
    for y in range(2, 15):
        for x in range(3, 13):
            d = ((x - 7.5) / 4.6) ** 2 + ((y - 9) / (6.6 if y < 9 else 5.6)) ** 2
            if d < 1:
                put(img, x, y, hexrgb("#88b8e8") if (x * 3 + y * 5) % 11 else hexrgb("#2f5a91"))
    return outline(img)


@item("victory_medal")
def victory_medal():
    rows = [
        "................",
        "....11....11....",
        ".....11..11.....",
        "......1111......",
        ".......11.......",
        "......2222......",
        ".....234432.....",
        "....23455432....",
        "....24555542....",
        "....24555542....",
        "....23455432....",
        ".....234432.....",
        "......2222......",
        "................",
        "................",
        "................",
    ]
    return outline(from_map(rows, pmap(pal("#3a5ac8", "#a87c10", "#d8a420", "#f2cc4a", "#fff0a0"))))


def parachute(p):
    rows = [
        "................",
        "....33333333....",
        "..334444444433..",
        ".34445555554443.",
        ".3445555555544 3",
        "3.1..1....1..1.3",
        "...1..1..1..1...",
        "....1..11..1....",
        ".....1.11.1.....",
        "......1111......",
        ".......22.......",
        ".......22.......",
        "................",
    ] + ["................"] * 3
    rows = [r.replace(" ", "4") for r in rows]
    return outline(from_map(rows, pmap(p)))


ITEMS["cloud_parachute"] = lambda: parachute(pal("#9a9aa2", "#6a5a4a", "#c8d0dc", "#e6ecf4", "#ffffff"))
ITEMS["golden_parachute"] = lambda: parachute(pal("#9a9aa2", "#6a5a4a", "#d8b040", "#f0d67c", "#fff0c0"))


@item("valkyrie_lance")
def valkyrie_lance():
    img = canvas()
    m = (np.abs(I.V) <= 0.7) & (I.U >= 0.5) & (I.U <= 11.0)
    img[m & (I.V > 0), :3] = hexrgb("#ecedf0")
    img[m & (I.V <= 0), :3] = hexrgb("#a4a8b0")
    img[m, 3] = 255
    tip = (np.abs(I.V) <= 1.6 - (I.U - 11.0) * 0.4) & (I.U > 11.0) & (I.U <= 15.0)
    img[tip, :3] = hexrgb("#f2cc4a")
    img[tip, 3] = 255
    guard = (np.abs(I.V) <= 2.4) & (I.U > 3.0) & (I.U <= 4.0)
    img[guard, :3] = hexrgb("#d8a420")
    img[guard, 3] = 255
    return outline(img)
