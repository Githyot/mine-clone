"""The Otherside's art: blue-grey gloomslate, teal-veined sculk stone, pale echo wood with
glowing bulbs, blooming moss, resonarium and the portal. Our own 16x16 pixels."""

import numpy as np

from .lib import BLOCKS, ITEMS, I, N, T, block, canvas, fbm, from_map, from_rgb, hexrgb, item, outline, pal, pmap, put, rng_for, shade

T.WOODS["echo"] = dict(
    bark=pal("#2a3a44", "#34485a", "#405870", "#4e6a84", "#5e7e98"),
    inner=pal("#7aa0b0", "#8cb4c4", "#a0c8d6", "#b4dae6"),
    plank=pal("#3e5666", "#4a6676", "#567686", "#648696", "#7296a6"),
    leaves=pal("#164a52", "#1e5e66", "#28747a", "#348a8e", "#46a4a6"),
)
I.MATERIALS["resonarium"] = pal("#0a3a44", "#126070", "#1e8a9a", "#3ab8c4", "#a0f0f0")
I.ARMOR_MATERIALS["resonarium"] = I.MATERIALS["resonarium"]

GLOOM = pal("#1c222c", "#242c38", "#2c3644", "#364252", "#425062", "#506074")
SCULK = pal("#071a20", "#0c2630", "#123440", "#1a4652")
TEAL = hexrgb("#3ad8d0")


@block("gloomslate")
def gloomslate(name="gloomslate"):
    rng = rng_for(name)
    f = fbm(rng, ((8, 0.2), (4, 0.3), (2, 0.35), (1, 0.3)))
    # Faint horizontal layering, like slate.
    f = f * 0.7 + 0.3 * (np.sin(np.arange(N)[:, None] * 1.3 + rng.random()) * 0.5 + 0.5)
    return from_rgb(shade(f, GLOOM, 0.7))


@block("cobbled_gloomslate")
def cobbled_gloomslate():
    img = T.cobblestone("cobbled_gloomslate")
    return recolor(img, GLOOM)


@block("polished_gloomslate")
def polished_gloomslate():
    img = gloomslate("polished_gloomslate")
    T.bevel(img, 1.15, 0.75)
    return img


@block("gloomslate_bricks")
def gloomslate_bricks():
    return recolor(T.stone_bricks(), GLOOM)


@block("sculk_stone")
def sculk_stone(name="sculk_stone"):
    rng = rng_for(name)
    img = from_rgb(shade(fbm(rng, ((4, 0.4), (2, 0.4), (1, 0.3))), SCULK, 0.6))
    vein = np.abs(fbm(rng, ((4, 0.6), (2, 0.4))) - 0.5) < 0.05
    img[vein, :3] = TEAL
    return img


@block("sculk_grime")
def sculk_grime():
    img = sculk_stone("sculk_grime")
    rng = rng_for("sculk_grime_dots")
    for _ in range(5):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, hexrgb("#8af4ee"))
    return img


@block("sculk_grime_side")
def sculk_grime_side():
    img = gloomslate("sculk_grime_side")
    top = sculk_stone("sculk_grime_side_top")
    rng = rng_for("sculk_grime_side_edge")
    for x in range(N):
        d = 3 + int(rng.integers(0, 3))
        img[:d, x] = top[:d, x]
    return img


@block("echo_soil")
def echo_soil():
    rng = rng_for("echo_soil")
    p = pal("#6a7c84", "#7c9098", "#8ea4ac", "#a0b6be", "#b4c8d0")
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.45))), p, 0.8))
    for _ in range(8):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, p[0])
    return img


MOSS = pal("#2a1a4a", "#3a2466", "#4e3284", "#6644a4", "#8260c4")


@block("blooming_moss")
def blooming_moss():
    rng = rng_for("blooming_moss")
    img = from_rgb(shade(fbm(rng, ((4, 0.3), (2, 0.4), (1, 0.4))), MOSS, 0.9))
    for _ in range(7):
        x, y = rng.integers(0, N, 2)
        put(img, x, y, hexrgb("#f0a8ff"))
    return img


@block("blooming_moss_side")
def blooming_moss_side():
    img = gloomslate("blooming_moss_side")
    top = blooming_moss()
    rng = rng_for("blooming_moss_edge")
    for x in range(N):
        d = 2 + int(rng.integers(0, 4))
        img[:d, x] = top[:d, x]
    return img


BLOCKS["resonarium_ore"] = lambda: T.ore("resonarium", gloomslate("resonarium_ore_stone"), pal("#126070", "#1e8a9a", "#3ab8c4", "#a0f0f0"))


@block("crystallized_amber")
def crystallized_amber():
    rng = rng_for("crystallized_amber")
    p = pal("#a05008", "#c87010", "#e8981e", "#f8c040", "#ffe48a")
    img = from_rgb(shade(fbm(rng, ((4, 0.5), (2, 0.5))), p, 0.5))
    img[..., 3] = 200
    T.bevel(img, 1.1, 0.8)
    return img


BLOCKS["echo_log"] = lambda: T.log_side("echo")
BLOCKS["echo_log_top"] = lambda: T.log_top("echo")
BLOCKS["echo_leaves"] = lambda: T.leaves("echo")
BLOCKS["echo_sapling"] = lambda: I.sapling("echo")
BLOCKS["echo_planks"] = lambda: T.planks("echo")


@block("sculk_tendrils")
def sculk_tendrils():
    rng = rng_for("sculk_tendrils")
    img = canvas()
    for x in (3, 6, 9, 12):
        h = int(rng.integers(6, 14))
        for y in range(16 - h, 16):
            put(img, x + ((y // 3) % 2), y, SCULK[3] if y % 4 else TEAL)
    return img


BLOCKS["gloom_bloom"] = lambda: T.flower("gloom_bloom", pal("#4e3284", "#8260c4", "#c8a0ff"), hexrgb("#8af4ee"), 6, True)


@block("echo_bulb")
def echo_bulb():
    img = canvas()
    for y in range(0, 7):
        put(img, 7, y, hexrgb("#34485a"))
    for y in range(6, 13):
        for x in range(4, 12):
            d = ((x - 7.5) / 3.6) ** 2 + ((y - 9.5) / 3.4) ** 2
            if d < 1:
                put(img, x, y, hexrgb("#e8fffa") if d < 0.3 else hexrgb("#7af0e8"))
    return img


@block("echo_lantern")
def echo_lantern():
    img = gloomslate_bricks()
    for y in range(3, 13):
        for x in range(3, 13):
            put(img, x, y, hexrgb("#7af0e8") if (x + y) % 3 else hexrgb("#e8fffa"))
    return img


@block("sculk_portal_frame")
def sculk_portal_frame():
    img = polished_gloomslate()
    for k in range(N):
        put(img, k, 7, TEAL)
        put(img, k, 8, SCULK[2])
    return img


@block("sculk_portal_frame_top")
def sculk_portal_frame_top():
    img = polished_gloomslate()
    for y in range(N):
        for x in range(N):
            if abs(np.hypot(x - 7.5, y - 7.5) - 4.5) < 0.8:
                put(img, x, y, TEAL)
    return img


@block("otherside_portal")
def otherside_portal():
    """Dark water under a teal shimmer."""
    rng = rng_for("otherside_portal")
    p = pal("#03141a", "#06222a", "#0c3640", "#145058", "#20787c", "#3ad8d0")
    img = canvas()
    img[..., 3] = 220
    for y in range(N):
        for x in range(N):
            v = 0.5 + 0.5 * np.sin(x * 0.8 + np.cos(y * 0.6) * 2.0) * np.sin(y * 0.4) + rng.normal(0, 0.08)
            img[y, x, :3] = p[int(np.clip(v * 5.5, 0, 5))]
    return img


def recolor(img, p):
    out = img.copy()
    lum = img[..., :3].mean(axis=2) / 255.0
    lum = (lum - lum.min()) / max(1e-6, lum.max() - lum.min())
    idx = np.clip((lum * len(p)).astype(int), 0, len(p) - 1)
    out[..., :3] = np.array(p)[idx]
    return out


ITEMS["echo_crystal"] = lambda: I.diamond(pal("#0a3a44", "#126070", "#3ab8c4", "#8af4ee", "#e8fffa"))
ITEMS["resonarium"] = lambda: I.ingot("resonarium") if "resonarium" in getattr(I, "INGOTS", {}) else I.lump(
    "resonarium", I.MATERIALS["resonarium"])
ITEMS["heart_of_the_deep_shard"] = lambda: I.lump("heart_of_the_deep_shard", pal("#071a20", "#0c2630", "#1a4652", "#3ad8d0", "#e8fffa"))


@item("sculk_bone")
def sculk_bone():
    return recolor(I.bone(), pal("#071a20", "#0c2630", "#1a4652", "#2a6a78", "#3ad8d0"))


@item("heart_of_the_deep")
def heart_of_the_deep():
    rows = [
        "................",
        "................",
        "...2222..2222...",
        "..233332233332..",
        ".23455533555432.",
        ".23455555555432.",
        ".23445555554432.",
        "..234455554432..",
        "...2344554432...",
        "....23444432....",
        ".....234432.....",
        "......2332......",
        ".......22.......",
        "................",
        "................",
        "................",
    ]
    return outline(from_map(rows, pmap(pal("#03141a", "#0c2630", "#145058", "#20a0a0", "#8af4ee"))))


@item("gloom_berry_pie")
def gloom_berry_pie():
    rows = [
        "................",
        "................",
        "................",
        "................",
        "................",
        "....22222222....",
        "..224545454522..",
        ".23454545454532.",
        ".23333333333332.",
        ".21111111111112.",
        "..222222222222..",
        "................",
    ] + ["................"] * 4
    return outline(from_map(rows, pmap(pal("#a0682a", "#c88a40", "#e0b060", "#3ad8d0", "#4e3284"))))


for _k in ("pickaxe", "axe", "shovel", "sword"):
    ITEMS[f"resonarium_{_k}"] = (lambda k=_k: I.tool("resonarium", k))
for _k in ("helmet", "chestplate", "leggings", "boots"):
    ITEMS[f"resonarium_{_k}"] = (lambda k=_k: I.armor("resonarium", k))
