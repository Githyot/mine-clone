#!/usr/bin/env python3
"""Our own 16x16 art for the Nether and End additions (roadmap phase 5): blocks such as
ancient debris, polished blackstone, purpur pillars and end rods, and items such as
netherite, ghast tears, the elytra and firework rockets. Short hand-picked palettes and
hash noise, in the same classic style as tools/gen_textures.py and tools/gen_items.py.

Netherite tools and armour are the diamond pictures recoloured to dark netherite.

Usage: python3 tools/gen_nether_end_art.py   (deterministic; writes into packs/vanilla/textures)
"""
import os
import zlib

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BLOCK = os.path.join(ROOT, "packs", "vanilla", "textures", "block")
ITEM = os.path.join(ROOT, "packs", "vanilla", "textures", "item")


def h01(seed, *v):
    x = zlib.crc32(repr((seed,) + v).encode()) & 0xFFFFFFFF
    return x / 0xFFFFFFFF


def hexrgb(s):
    s = s.lstrip("#")
    return tuple(int(s[i:i + 2], 16) for i in (0, 2, 4))


def pal(*hx):
    return [hexrgb(h) for h in hx]


def pick(p, t):
    t = min(max(t, 0.0), 0.999)
    return p[int(t * len(p))]


def smooth(seed, x, y, scale):
    """Value noise on a 16-wrapping grid (tiles seamlessly)."""
    gx, gy = x / scale, y / scale
    x0, y0 = int(gx), int(gy)
    fx, fy = gx - x0, gy - y0
    n = int(16 / scale)

    def c(i, j):
        return h01(seed, i % n, j % n)
    a = c(x0, y0) * (1 - fx) + c(x0 + 1, y0) * fx
    b = c(x0, y0 + 1) * (1 - fx) + c(x0 + 1, y0 + 1) * fx
    return a * (1 - fy) + b * fy


def tex(fn, name, out=BLOCK):
    img = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    px = img.load()
    for y in range(16):
        for x in range(16):
            c = fn(x, y)
            if c is not None:
                px[x, y] = (*c[:3], c[3] if len(c) > 3 else 255)
    img.save(os.path.join(out, name + ".png"))
    print("wrote", name)


def noisy(p, seed, scale=4, jit=0.35):
    return lambda x, y: pick(p, smooth(seed, x, y, scale) * (1 - jit) + h01(seed + 9, x, y) * jit)


# ---------------------------------------------------------------- blocks

BLACK = pal("#16141a", "#1e1b22", "#26222b", "#2f2a34", "#38333e")
DEBRIS = pal("#3a2a24", "#4a362e", "#5a4238", "#6a4e42", "#7c5c4c")
NETH = pal("#2a2428", "#342d32", "#3e363c", "#4a4146", "#564c52", "#625860")
PURP = pal("#7c4e7c", "#8e5e8e", "#a06ea0", "#b07eb0", "#c092c0")
ESTONE = pal("#bdb889", "#cdc898", "#dbd6a6", "#e6e2b4", "#f0edc4")
OBS = pal("#0d0a14", "#151020", "#1e162e", "#2b2042", "#3e2e5e")


def debris_side(x, y):
    # Swirled rings of dark brown, like layers of something old pressed together.
    r = ((x - 7.5) ** 2 + (y - 7.5) ** 2) ** 0.5
    t = (r / 2.2 + smooth(11, x, y, 4) * 2.0) % 1.0
    c = pick(DEBRIS, t * 0.8 + h01(12, x, y) * 0.2)
    if (x in (0, 15) or y in (0, 15)) and h01(13, x, y) < 0.6:
        c = DEBRIS[0]
    return c


def debris_top(x, y):
    t = (abs(x - 7.5) + abs(y - 7.5)) / 3.0 + smooth(14, x, y, 4)
    return pick(DEBRIS, (t % 1.0) * 0.85 + h01(15, x, y) * 0.15)


def netherite_block(x, y):
    c = pick(NETH, smooth(21, x, y, 8) * 0.6 + h01(22, x, y) * 0.4)
    if x in (0, 15) or y in (0, 15):
        c = NETH[0]
    elif x in (1, 14) or y in (1, 14):
        c = NETH[-1] if (x + y) % 3 else NETH[-2]
    return c


def crying_obsidian(x, y):
    c = pick(OBS, smooth(31, x, y, 4) * 0.6 + h01(32, x, y) * 0.4)
    # Glowing violet tears running down.
    for col in (3, 9, 13):
        start = int(h01(33, col) * 8)
        if x == col and start <= y <= start + 4 + int(h01(34, col) * 4):
            c = pick(pal("#6a22b8", "#8a34d8", "#a84cf0", "#c070ff"), (y - start) / 8.0)
    return c


def gilded_blackstone(x, y):
    c = pick(BLACK, smooth(41, x, y, 4) * 0.6 + h01(42, x, y) * 0.4)
    if (smooth(43, x, y, 4) > 0.5 and h01(44, x, y) < 0.7) or h01(46, x, y) < 0.06:
        c = pick(pal("#b07c16", "#cc9a20", "#e4b42c", "#f6d050"), h01(45, x, y))
    return c


def bricks(base, seed, cracked=False):
    def f(x, y):
        row = y // 4
        off = 4 if row % 2 else 0
        mortar = y % 4 == 3 or (x + off) % 8 == 7
        c = pick(base, smooth(seed, x, y, 4) * 0.5 + h01(seed + 1, x, y) * 0.5)
        if mortar:
            c = base[0]
        if cracked and h01(seed + 2, x // 2, y) < 0.08:
            c = base[0]
        return c
    return f


def chiseled_blackstone(x, y):
    c = pick(BLACK, 0.4 + h01(51, x, y) * 0.5)
    d = max(abs(x - 7.5), abs(y - 7.5))
    if d > 6.5 or int(d) == 4:
        c = BLACK[0]
    elif d < 2.5:
        c = pick(pal("#5a4632", "#6e563c", "#826646"), h01(52, x, y))
    return c


def polished_basalt_side(x, y):
    c = pick(pal("#3c3c42", "#46464c", "#505058", "#5c5c64"), (x % 4) / 4 * 0.5 + h01(61, x, y) * 0.5)
    return c


def polished_basalt_top(x, y):
    d = max(abs(x - 7.5), abs(y - 7.5))
    return pick(pal("#34343a", "#424248", "#505058", "#5e5e66"), (d % 3) / 3 * 0.6 + h01(62, x, y) * 0.4)


def anchor_side(x, y):
    c = pick(OBS, smooth(71, x, y, 4) * 0.6 + h01(72, x, y) * 0.4)
    if 4 <= x <= 11 and 4 <= y <= 11:
        c = pick(pal("#2a1640", "#3a1e58", "#4c2870"), h01(73, x, y))
    if (x in (4, 11) and 4 <= y <= 11) or (y in (4, 11) and 4 <= x <= 11):
        c = pick(pal("#7a4cb0", "#9462cc", "#ae7ae6"), h01(74, x, y))
    return c


def anchor_top(x, y):
    c = pick(OBS, smooth(75, x, y, 4) * 0.6 + h01(76, x, y) * 0.4)
    d = max(abs(x - 7.5), abs(y - 7.5))
    if d < 3:
        c = pick(pal("#e2a83a", "#f2c454", "#ffe080"), h01(77, x, y))
    elif d < 4:
        c = pick(pal("#5a3a1a", "#6a4622"), h01(78, x, y))
    return c


BONEDARK = pal("#1a181c", "#242226", "#2e2c30", "#3a383c")


def wither_skull_side(x, y):
    c = pick(BONEDARK, smooth(81, x, y, 4) * 0.6 + h01(82, x, y) * 0.4)
    if y in (6, 7) and x in (3, 4, 11, 12):
        c = (6, 6, 8)
    if y == 11 and 4 <= x <= 11 and x % 2 == 0:
        c = (10, 10, 12)
    return c


def wither_skull_top(x, y):
    return pick(BONEDARK, smooth(83, x, y, 4) * 0.6 + h01(84, x, y) * 0.4)


DRAGON = pal("#141018", "#1c1622", "#241c2c", "#2e2438", "#3a2e46")


def dragon_head_side(x, y):
    c = pick(DRAGON, smooth(91, x, y, 4) * 0.5 + h01(92, x, y) * 0.5)
    if y in (5, 6) and x in (3, 4, 11, 12):
        c = pick(pal("#b24ad8", "#d266f0"), h01(93, x, y))
    if y >= 12 and x % 3 == 1:
        c = (190, 184, 170)
    return c


def dragon_egg(x, y):
    c = pick(pal("#0c0a10", "#141018", "#1c1622", "#241c2c"), smooth(101, x, y, 4) * 0.6 + h01(102, x, y) * 0.4)
    if h01(103, x, y) < 0.09:
        c = pick(pal("#6a2c8a", "#8a3cae", "#a84cd0"), h01(104, x, y))
    return c


def end_rod(x, y):
    if 7 <= x <= 8 and 1 <= y <= 14:
        return pick(pal("#e8e2f0", "#f2eef8", "#ffffff"), h01(111, y) * 0.6 + (x - 7) * 0.4)
    if 6 <= x <= 9 and y >= 14:
        return pick(pal("#9a7aa8", "#ae8ebc"), h01(112, x))
    return None


def end_gateway(x, y):
    t = smooth(121, x, y, 4)
    c = pick(pal("#04040a", "#0a0818", "#120e26", "#1c1638"), t)
    if h01(122, x, y) < 0.07:
        c = pick(pal("#7ae0c8", "#b8f4e4", "#e4fff6", "#c890ff"), h01(123, x, y))
    return c


def purpur_pillar_side(x, y):
    c = pick(PURP, (x % 5) / 5 * 0.4 + h01(131, x, y) * 0.6)
    if x in (0, 15):
        c = PURP[0]
    return c


def purpur_pillar_top(x, y):
    d = max(abs(x - 7.5), abs(y - 7.5))
    c = pick(PURP, (d % 4) / 4 * 0.5 + h01(132, x, y) * 0.5)
    if d > 6.5:
        c = PURP[0]
    return c


def shulker_box_side(x, y):
    c = pick(PURP, smooth(141, x, y, 4) * 0.5 + h01(142, x, y) * 0.5)
    if y in (0, 15) or x in (0, 15) or y == 8:
        c = PURP[0]
    return c


def shulker_box_top(x, y):
    c = pick(PURP, smooth(143, x, y, 4) * 0.5 + h01(144, x, y) * 0.5)
    d = max(abs(x - 7.5), abs(y - 7.5))
    if d > 6.5 or int(d) == 3:
        c = PURP[0]
    return c


def deltas_floor(x, y):
    # Blackened basalt crust with glowing seams (basalt deltas ground).
    c = pick(pal("#26262c", "#30303a", "#3a3a44", "#46464e"), smooth(151, x, y, 4) * 0.6 + h01(152, x, y) * 0.4)
    return c


# ---------------------------------------------------------------- items


def recolor(src, dst, mapping):
    img = Image.open(os.path.join(ITEM, src + ".png")).convert("RGBA")
    px = img.load()
    for y in range(img.height):
        for x in range(img.width):
            r, g, b, a = px[x, y]
            if a == 0:
                continue
            # Cyan diamond pixels (green/blue well above red) become netherite.
            if g > r + 12 and b > r + 12:
                lum = (r + g + b) / 3 / 255
                px[x, y] = (*pick(mapping, lum * 1.6), a)
    img.save(os.path.join(ITEM, dst + ".png"))
    print("wrote", dst)


NETH_ITEM = pal("#1e1a1e", "#2c262a", "#3a3236", "#4a4044", "#5c5054", "#706468", "#86787c")


def sprite(points, palette, seed, outline=(14, 10, 12)):
    """Fill a set of (x, y) cells with shaded palette colours and outline the shape."""
    pts = set(points)

    def f(x, y):
        if (x, y) in pts:
            lit = 0.55 - (x + y - 15) * 0.03 + h01(seed, x, y) * 0.3
            return pick(palette, lit)
        if any((x + dx, y + dy) in pts for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
            return outline
        return None
    return f


def blob(cx, cy, rx, ry):
    return [(x, y) for y in range(16) for x in range(16) if ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1.0]


def scrap(x, y):
    pts = blob(7.5, 8.5, 5.2, 4.2)
    if (x, y) in pts:
        t = smooth(201, x, y, 4) * 0.6 + h01(202, x, y) * 0.4
        return pick(pal("#3a2a24", "#4c362e", "#5e4438", "#704f40", "#86604c"), t)
    if any((x + dx, y + dy) in pts for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
        return (20, 14, 12)
    return None


def ingot_points():
    return [(x, y) for y in range(5, 12) for x in range(2, 14) if x - (11 - y) // 2 >= 2 and x + (y - 5) // 3 <= 13]


def tear(x, y):
    pts = [(x2, y2) for y2 in range(3, 14) for x2 in range(16) if abs(x2 - 7.5) <= max(0.0, (y2 - 3) * 0.5) and ((x2 - 7.5) ** 2 + (y2 - 10) ** 2) ** 0.5 <= 4.2 or (abs(x2 - 7.5) < 1 and 3 <= y2 <= 7)]
    return sprite(pts, pal("#9ab8c0", "#bcd6dc", "#dceef0", "#f6ffff"), 211, (60, 80, 90))(x, y)


def magma_cream(x, y):
    pts = blob(7.5, 8, 5, 4.5)
    if (x, y) in pts and h01(221, x, y) < 0.25:
        return pick(pal("#e8781c", "#f49a2a", "#ffc048"), h01(222, x, y))
    return sprite(pts, pal("#5a2410", "#7a3416", "#9a461c", "#b85a22"), 223, (30, 12, 6))(x, y)


def blaze_powder(x, y):
    pts = [p for p in blob(7.5, 10, 6, 3.5) if h01(231, *p) < 0.85]
    return sprite(pts, pal("#a84e10", "#cc6e16", "#e8901e", "#f8b830", "#ffd860"), 232, (60, 24, 6))(x, y)


def fire_charge(x, y):
    pts = blob(7.5, 7.5, 5.5, 5.5)
    if (x, y) in pts and h01(241, x, y) < 0.3:
        return pick(pal("#d85a12", "#f07a1a", "#ffa830"), h01(242, x, y))
    return sprite(pts, pal("#141012", "#221a1a", "#30221e", "#402c24"), 243, (8, 6, 6))(x, y)


def nether_star(x, y):
    pts = []
    for yy in range(16):
        for xx in range(16):
            dx, dy = xx - 7.5, yy - 7.5
            if abs(dx) + abs(dy) <= 4 or (abs(dx) < 1.0 and abs(dy) < 7) or (abs(dy) < 1.0 and abs(dx) < 7):
                pts.append((xx, yy))
    return sprite(pts, pal("#c8d0e0", "#dce2f0", "#eef2fc", "#ffffff"), 251, (90, 96, 120))(x, y)


def shulker_shell(x, y):
    pts = [p for p in blob(7.5, 9, 6.2, 5) if p[1] >= 4]
    c = sprite(pts, PURP, 261, (50, 30, 50))(x, y)
    if c is not None and (x, y) in pts and y == 9:
        c = PURP[0]
    return c


def elytra(x, y):
    # Two folded wings, membrane grey-violet with darker ribs.
    pts = []
    for yy in range(1, 15):
        for xx in range(16):
            # Each wing hangs from the top middle and sweeps out and down to a point.
            d = 7 - xx if xx <= 6 else xx - 9
            left_or_right = xx <= 6 or xx >= 9
            left = right = left_or_right and d <= min(6, 1 + yy // 2) and yy <= 14 - d // 2 and yy >= 1 + d // 3
            if left or right:
                pts.append((xx, yy))
    base = pal("#4e4a5a", "#5e5a6c", "#6e6a7e", "#807c92", "#928ea6")

    def f(xx, yy):
        c = sprite(pts, base, 271, (24, 22, 30))(xx, yy)
        if c is not None and (xx, yy) in pts and (yy % 4 == 2):
            c = base[0]
        return c
    return f(x, y)


def firework_rocket(x, y):
    if 6 <= x <= 9 and 4 <= y <= 12:
        c = pick(pal("#a83226", "#c4402e", "#dc5238"), h01(281, x, y) * 0.5 + (x - 6) / 6)
        if y in (6, 10):
            c = (230, 220, 200)
        return c
    if 6 <= x <= 9 and y in (2, 3) and abs(x - 7.5) <= (y - 1):
        return (220, 210, 190)
    if x in (7, 8) and 13 <= y <= 15:
        return (110, 80, 50)
    return None


def dragon_breath(x, y):
    bottle = [(xx, yy) for yy in range(16) for xx in range(16) if (((xx - 7.5) / 5) ** 2 + ((yy - 10) / 5) ** 2 <= 1.0) or (6 <= xx <= 9 and 2 <= yy <= 6)]
    if (x, y) in bottle and y >= 7 and ((x - 7.5) / 4) ** 2 + ((y - 10.5) / 3.6) ** 2 <= 1.0:
        return pick(pal("#b04aa0", "#c85cbc", "#e07ad8", "#f2a0ee"), h01(291, x, y))
    if (x, y) in bottle and y <= 2:
        return (120, 86, 60)
    return sprite(bottle, pal("#a8b8c4", "#c4d4de", "#dce8f0"), 292, (60, 70, 80))(x, y)


def main():
    os.makedirs(BLOCK, exist_ok=True)
    os.makedirs(ITEM, exist_ok=True)
    tex(debris_side, "ancient_debris_side")
    tex(debris_top, "ancient_debris_top")
    tex(netherite_block, "netherite_block")
    tex(crying_obsidian, "crying_obsidian")
    tex(gilded_blackstone, "gilded_blackstone")
    tex(bricks(BLACK, 46), "polished_blackstone_bricks")
    tex(bricks(BLACK, 47, cracked=True), "cracked_polished_blackstone_bricks")
    tex(chiseled_blackstone, "chiseled_polished_blackstone")
    tex(noisy(pal("#2a272e", "#34313a", "#3e3a44", "#48444e"), 48, 8, 0.2), "polished_blackstone")
    tex(polished_basalt_side, "polished_basalt_side")
    tex(polished_basalt_top, "polished_basalt_top")
    tex(anchor_side, "respawn_anchor_side")
    tex(anchor_top, "respawn_anchor_top")
    tex(wither_skull_side, "wither_skeleton_skull")
    tex(wither_skull_top, "wither_skeleton_skull_top")
    tex(dragon_head_side, "dragon_head")
    tex(dragon_egg, "dragon_egg")
    tex(end_rod, "end_rod")
    tex(end_gateway, "end_gateway")
    tex(purpur_pillar_side, "purpur_pillar")
    tex(purpur_pillar_top, "purpur_pillar_top")
    tex(shulker_box_side, "shulker_box")
    tex(shulker_box_top, "shulker_box_top")
    tex(deltas_floor, "smooth_basalt")
    tex(scrap, "netherite_scrap", ITEM)
    tex(sprite(ingot_points(), NETH_ITEM, 205, (12, 10, 12)), "netherite_ingot", ITEM)
    tex(tear, "ghast_tear", ITEM)
    tex(magma_cream, "magma_cream", ITEM)
    tex(blaze_powder, "blaze_powder", ITEM)
    tex(fire_charge, "fire_charge", ITEM)
    tex(nether_star, "nether_star", ITEM)
    tex(shulker_shell, "shulker_shell", ITEM)
    tex(elytra, "elytra", ITEM)
    tex(firework_rocket, "firework_rocket", ITEM)
    tex(dragon_breath, "dragon_breath", ITEM)
    for kind in ("sword", "pickaxe", "axe", "shovel", "hoe", "helmet", "chestplate", "leggings", "boots"):
        recolor("diamond_" + kind, "netherite_" + kind, NETH_ITEM)


if __name__ == "__main__":
    main()
