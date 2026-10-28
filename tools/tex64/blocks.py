"""Hand-made 64x64 block textures, drawn with the shared style (see style.py).

Each generator returns an RGBA float array (64, 64, 4). `REGISTRY` maps texture names to
generators; anything not listed here is made by upscale.py from the 16x16 design instead.
Palettes come from tools/gen_textures.py, so a material keeps the colour identity it had.
"""

import os
import sys

import numpy as np

from style import (N, XX, YY, blur, canvas, clusters, dist_inside, edges_of, fbm, hexrgb, light, luma, mix,
                   normalize, occlusion, paint, quantize, ramp, rgba, rng_for, segment_dist, soften,
                   stretched_noise, value_noise, worley)

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import gen_textures as g16  # noqa: E402  (palettes only)


def hx(c):
    """Any colour form -> rgb array."""
    if isinstance(c, str):
        return hexrgb(c)
    return np.asarray(c, dtype=np.float64)


def torus_d(cx, cy):
    dx = (XX - cx + N / 2) % N - N / 2
    dy = (YY - cy + N / 2) % N - N / 2
    return dx, dy


def compose(base_t, height, pal, rng, lk=0.9, ao=0.06, dither=0.35, alpha=255.0):
    """base_t (0..1 paint level) + light of height -> ramp colours."""
    t = base_t + light(height) * lk - occlusion(height, 2) * ao
    return paint(t, pal, rng, dither, alpha)


# ================================================================ ground


def stone(name="stone", base="#7f7f7f", warm=0.10, spread=0.32, kind="natural"):
    rng = rng_for(name)
    pal = ramp(base, 11, spread, warm)
    blot = fbm(rng, ((32, 0.4), (16, 0.35), (8, 0.25)))
    grain = clusters(rng, 4)
    streak = stretched_noise(rng, 16, 4)
    height = blot * 3.0 + grain * 1.6 + streak * 1.2
    # A few fine cracks.
    for _ in range(4):
        x0, y0 = rng.random(2) * N
        ang = rng.uniform(-0.5, 0.5) + (0 if rng.random() < 0.6 else np.pi / 2)
        ln = rng.uniform(6, 14)
        for ox in (-N, 0, N):
            for oy in (-N, 0, N):
                d = segment_dist(x0 + ox, y0 + oy, x0 + ox + np.cos(ang) * ln, y0 + oy + np.sin(ang) * ln)
                height = height - np.clip(1.0 - d, 0, 1) * 1.6
    t = 0.5 + (blot - 0.5) * 0.35 + (grain - 0.5) * 0.25
    img = compose(t, height, pal, rng, lk=1.4, ao=0.10)
    return soften(img, kind)


def dirt(name="dirt", base="#866043", pebbles=9):
    rng = rng_for(name)
    pal = ramp(base, 10, 0.30, 0.12)
    f = fbm(rng, ((16, 0.35), (8, 0.35), (4, 0.3)))
    c = clusters(rng, 2)
    height = f * 2.0 + c * 1.5
    t = 0.5 + (f - 0.5) * 0.45 + (c - 0.5) * 0.35
    img = compose(t, height, pal, rng, lk=1.1, ao=0.10)
    # Little stones and roots poking out.
    peb = ramp(mix(hx(base), hx("#8a8780"), 0.6), 6, 0.3, 0.08)
    for _ in range(pebbles):
        cx, cy = rng.random(2) * N
        r = rng.uniform(1.0, 2.2)
        dx, dy = torus_d(cx, cy)
        dd = np.hypot(dx * rng.uniform(0.8, 1.2), dy)
        m = dd <= r
        dome = np.clip(1 - dd / (r + 0.5), 0, 1)
        lt = 0.5 + light(dome * 3) * 1.4
        img[m, :3] = quantize(np.clip(lt[m], 0, 1), peb)
        # contact shadow
        sh = (np.hypot(dx - 1, dy - 1) <= r + 0.4) & ~m
        img[sh, :3] *= 0.86
    return soften(img, "natural")


def grass_blades(rng, pal, count=420, bias=0.0):
    """Top view of a lawn: a dark mat with short lit blades in every direction."""
    field = fbm(rng, ((32, 0.35), (16, 0.35), (8, 0.3)))
    t = 0.40 + (field - 0.5) * 0.25 + bias
    h = field * 1.5
    tt = t.copy()
    for _ in range(count):
        x0, y0 = rng.random(2) * N
        ang = rng.uniform(0, np.pi)
        ln = rng.uniform(2.0, 4.5)
        x1, y1 = x0 + np.cos(ang) * ln, y0 + np.sin(ang) * ln
        lv = rng.uniform(0.05, 0.32)
        for ox in (0, -N, N):
            for oy in (0, -N, N):
                if not (-6 < x0 + ox < N + 6 and -6 < y0 + oy < N + 6):
                    continue
                d = segment_dist(x0 + ox, y0 + oy, x1 + ox, y1 + oy)
                m = d < 0.62
                tt[m] = t[m] + lv
    return tt, h


def grass_top(name="grass_block_top", base="#5f9440"):
    rng = rng_for(name)
    pal = ramp(base, 10, 0.26, 0.14, sat=1.0)
    tt, h = grass_blades(rng, pal)
    img = compose(tt, h, pal, rng, lk=0.6, ao=0.0, dither=0.2)
    return soften(img, "natural")


def fringe_side(name, under_fn, top_base, dirt_base="#866043", depth=14, snow=False):
    """Side of a block with a soft cover (grass, snow, nylium) hanging over dirt (or under_fn())."""
    rng = rng_for(name)
    img = under_fn() if under_fn else dirt(name + "_dirt", dirt_base, pebbles=6)
    edge = depth + np.zeros(N)
    # Irregular hanging edge: tufts of different lengths.
    x = 0
    while x < N:
        w = int(rng.integers(2, 6))
        drop = rng.choice([-4, -2, 0, 0, 2, 3, 5, 8]) if not snow else rng.choice([-2, 0, 0, 1, 2, 3])
        edge[x:x + w] = depth + drop
        x += w
    edge = np.convolve(np.r_[edge[-2:], edge, edge[:2]], np.ones(3) / 3, mode="same")[2:-2]
    cover = YY < edge[None, :]
    pal = ramp(top_base, 10, 0.26 if not snow else 0.16, 0.14)
    if snow:
        f = fbm(rng, ((16, 0.5), (8, 0.3), (4, 0.2)))
        t = 0.62 + (f - 0.5) * 0.3 - (YY / N) * 0.15
        h = f * 2
    else:
        tt, h = grass_blades(rng, pal, count=320)
        # Seen from the side the cover hangs down: vertical blade strokes.
        v = stretched_noise(rng, 2, 8)
        t = tt * 0.55 + (0.3 + v * 0.45) * 0.45
    dist_edge = edge[None, :] - YY
    t = t - np.clip(1.0 - dist_edge / 3.0, 0, 1) * 0.18  # underside of the overhang
    top = soften(compose(t, h, pal, rng, lk=0.6, ao=0.0, dither=0.2), "natural")
    img[cover] = top[cover]
    # Shadow cast on the dirt just under the cover.
    below = (YY >= edge[None, :]) & (YY < edge[None, :] + 3)
    k = 0.78 + 0.07 * (YY - edge[None, :])
    img[below, :3] = img[below, :3] * np.clip(k[below], 0.78, 1.0)[:, None]
    return img


def grass_side():
    return fringe_side("grass_block_side", None, "#5f9440", depth=13)


def snow_grass_side():
    return fringe_side("snow_grass_side", None, "#eef3f6", depth=15, snow=True)


def sand(name="sand", base="#dccfa3", ripples=False):
    rng = rng_for(name)
    pal = ramp(base, 10, 0.22, 0.12)
    f = fbm(rng, ((32, 0.3), (16, 0.3), (8, 0.4)))
    g = clusters(rng, 2)
    t = 0.5 + (f - 0.5) * 0.35 + (g - 0.5) * 0.5
    h = g * 1.2 + f
    if ripples:
        h = h + np.sin((YY + value_noise(rng, 16) * 6) * 2 * np.pi / 8) * 0.8
    img = compose(t, h, pal, rng, lk=0.8, ao=0.04, dither=0.3)
    # A few darker and lighter grains, in pairs, never single pixels.
    for _ in range(26):
        x, y = rng.integers(0, N, 2)
        col = pal[rng.choice([1, 2, 8, 9])]
        img[y % N, x % N, :3] = col
        img[(y + rng.integers(0, 2)) % N, (x + 1) % N, :3] = col
    return soften(img, "natural")


def red_sand():
    return sand("red_sand", "#c26a33")


def gravel(name="gravel"):
    rng = rng_for(name)
    ids, d1, d2, pts = worley(rng, 46, 6.3, jitter_shape=0.5)
    n = len(pts)
    tone = rng.random(n)
    tint = rng.random(n)
    edge = d2 - d1
    dome = np.clip(edge / 3.0, 0, 1) ** 0.6
    height = dome * 3 + clusters(rng, 2) * 0.6
    greys = ramp("#837d79", 10, 0.42, 0.10)
    browns = ramp("#857a70", 10, 0.42, 0.10)
    t = 0.38 + tone[ids] * 0.32 + light(height) * 1.2 - occlusion(height, 2) * 0.12
    t = np.where(edge < 0.9, 0.08, t)
    t = np.clip(t, 0, 1)
    a = quantize(t, greys, 0.3, rng)
    b = quantize(t, browns, 0.3, rng)
    rgb = np.where((tint[ids] > 0.7)[..., None], b, a)
    return soften(rgba(rgb), "rough")


def clay():
    rng = rng_for("clay")
    pal = ramp("#a0a6b4", 10, 0.2, 0.1)
    f = fbm(rng, ((32, 0.4), (16, 0.35), (8, 0.25)))
    s = stretched_noise(rng, 32, 4)
    t = 0.5 + (f - 0.5) * 0.4 + (s - 0.5) * 0.2
    return soften(compose(t, f * 2 + s, pal, rng, lk=0.8), "natural")


def bedrock():
    rng = rng_for("bedrock")
    ids, d1, d2, pts = worley(rng, 20, 9, jitter_shape=0.6)
    tone = rng.random(len(pts))
    pal = ramp("#5a5a5a", 11, 0.6, 0.08)
    h = np.clip((d2 - d1) / 5, 0, 1) * 3 + clusters(rng, 4) * 1.5
    t = 0.2 + tone[ids] * 0.6 + light(h) * 1.3 - occlusion(h, 2) * 0.2
    return compose(t, h * 0, pal, rng, lk=0, ao=0)


def snow():
    rng = rng_for("snow")
    pal = ramp("#eef3f6", 9, 0.12, 0.25)
    f = fbm(rng, ((32, 0.4), (16, 0.35), (8, 0.25)))
    c = clusters(rng, 2)
    t = 0.62 + (f - 0.5) * 0.4 + (c - 0.5) * 0.2
    img = compose(t, f * 2 + c, pal, rng, lk=0.8, ao=0.05, dither=0.25)
    # Sparkles: two-pixel glints.
    for _ in range(10):
        x, y = rng.integers(0, N, 2)
        img[y, x, :3] = 255
        img[y, (x + 1) % N, :3] = pal[-2]
    return soften(img, "natural")


def ice(name="ice", base="#8fb4ef", packed=False):
    rng = rng_for(name)
    pal = ramp(base, 10, 0.3, 0.12)
    f = fbm(rng, ((32, 0.5), (16, 0.3), (8, 0.2)))
    t = 0.5 + (f - 0.5) * 0.5
    img = compose(t, f * 2, pal, rng, lk=0.7)
    # Long diagonal cracks with a bright edge.
    for _ in range(5 if not packed else 9):
        x0, y0 = rng.random(2) * N
        ang = rng.uniform(-1.2, 1.2)
        ln = rng.uniform(10, 26)
        for ox in (-N, 0, N):
            for oy in (-N, 0, N):
                d = segment_dist(x0 + ox, y0 + oy, x0 + ox + np.cos(ang) * ln, y0 + oy + np.sin(ang) * ln)
                img[d < 0.5, :3] = pal[-1]
                img[(d >= 0.5) & (d < 1.1), :3] = img[(d >= 0.5) & (d < 1.1), :3] * 0.9
    if not packed:
        img[..., 3] = 200
    return img


def packed_ice():
    return ice("packed_ice", "#7fa2e6", packed=True)


def water():
    rng = rng_for("water")
    pal = ramp("#4a72d8", 10, 0.18, 0.15)
    f = fbm(rng, ((32, 0.4), (16, 0.35), (8, 0.25)))
    ripple = np.sin((YY * 0.9 + XX * 0.35 + value_noise(rng, 16) * 10) * 2 * np.pi / 10)
    t = 0.45 + (f - 0.5) * 0.3 + ripple * 0.08
    img = compose(t, f * 2, pal, rng, lk=0.6, dither=0.2)
    img[..., 3] = 190
    return img


def lava():
    rng = rng_for("lava")
    pal = np.array([hx(c) for c in ("#6e1a04", "#93290a", "#b53a0c", "#d24e10", "#e86814", "#f5861c",
                                     "#fba52c", "#fdc444", "#fee070", "#fff4a8")])
    ids, d1, d2, pts = worley(rng, 14, 12, jitter_shape=0.6)
    crust = np.clip((d2 - d1) / 6, 0, 1)
    f = fbm(rng, ((16, 0.5), (8, 0.3), (4, 0.2)))
    t = 0.25 + (1 - crust) * 0.55 + (f - 0.5) * 0.35
    return rgba(quantize(np.clip(t, 0, 1), pal, 0.4, rng))


def obsidian():
    rng = rng_for("obsidian")
    pal = ramp("#1f1630", 10, 0.2, 0.2, sat=1.4)
    ids, d1, d2, pts = worley(rng, 12, 13, jitter_shape=0.5)
    h = np.clip((d2 - d1) / 4, 0, 1) * 2
    tone = rng.random(len(pts))
    t = 0.35 + tone[ids] * 0.25 + light(h) * 1.6
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    # Violet glassy streaks.
    vi = ramp("#6a3fa0", 6, 0.3, 0.1)
    for _ in range(7):
        x0, y0 = rng.random(2) * N
        ang = rng.uniform(0, np.pi)
        d = segment_dist(x0, y0, x0 + np.cos(ang) * 6, y0 + np.sin(ang) * 6)
        m = d < 0.7
        img[m, :3] = vi[rng.integers(2, 6)]
    return img


def terracotta(name):
    rng = rng_for(name)
    base = g16.TERRACOTTA[name]
    pal = ramp(base, 10, 0.18, 0.1)
    f = fbm(rng, ((32, 0.4), (16, 0.3), (8, 0.3)))
    c = clusters(rng, 2)
    t = 0.5 + (f - 0.5) * 0.3 + (c - 0.5) * 0.3
    return soften(compose(t, f + c, pal, rng, lk=0.8), "natural")


def sandstone_side(name="sandstone_side", base="#d9c896"):
    rng = rng_for(name)
    pal = ramp(base, 10, 0.26, 0.12)
    layers = np.zeros(N)
    y = 0
    k = 0
    while y < N:
        h = int(rng.integers(5, 12))
        layers[y:y + h] = rng.uniform(-0.12, 0.12) + (0.08 if k % 2 else -0.04)
        y += h
        k += 1
    f = fbm(rng, ((32, 0.3), (8, 0.4), (4, 0.3)))
    s = stretched_noise(rng, 16, 2)
    t = 0.5 + layers[:, None] + (f - 0.5) * 0.2 + (s - 0.5) * 0.25
    h = (t * 3)
    img = compose(t, h, pal, rng, lk=0.8)
    # A worn top band like a cap stone.
    img[:6] = compose(np.full((N, N), 0.68) + (f - 0.5) * 0.2, f, pal, rng)[:6]
    img[6, :, :3] *= 0.8
    return soften(img, "rough")


def sandstone_top(name="sandstone_top", base="#dccc9a"):
    rng = rng_for(name)
    pal = ramp(base, 10, 0.22, 0.12)
    f = fbm(rng, ((32, 0.4), (16, 0.3), (8, 0.3)))
    c = clusters(rng, 2)
    t = 0.55 + (f - 0.5) * 0.25 + (c - 0.5) * 0.3
    return soften(compose(t, f * 2 + c, pal, rng, lk=0.9), "natural")


def sandstone_bottom():
    img = sandstone_top("sandstone_bottom", "#cdbd8b")
    return img


# ================================================================ stone families


def cell_stones(name, base, count, min_d, mortar_level=0.08, gap=1.0, tones=0.35, moss=False,
                jitter=0.5, pal=None, kind="rough"):
    """Rounded stones packed in mortar (cobblestone and friends)."""
    rng = rng_for(name)
    pal = pal if pal is not None else ramp(base, 12, 0.5, 0.10)
    ids, d1, d2, pts = worley(rng, count, min_d, jitter_shape=jitter)
    n = len(pts)
    tone = rng.random(n)
    edge = d2 - d1
    dome = np.clip(edge / 5.0, 0, 1) ** 0.55
    surface = clusters(rng, 2) * 0.5 + value_noise(rng, 8) * 0.6
    height = dome * 4.0 + surface
    t = 0.45 + (tone[ids] - 0.5) * tones + light(height) * 1.25 - occlusion(height, 2) * 0.1
    t = np.where(edge < gap, mortar_level + (surface - 0.5) * 0.05, t)
    t = np.where((edge >= gap) & (edge < gap + 0.8), t - 0.12, t)
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    if moss:
        add_moss(img, rng, edge, gap)
    return soften(img, kind)


def add_moss(img, rng, edge=None, gap=1.0, amount=0.4):
    m = fbm(rng, ((32, 0.4), (16, 0.35), (8, 0.25)))
    if edge is not None:
        m = m + np.clip(1.6 - edge / 3, 0, 1) * 0.2
    mask = m > (1 - amount * 0.55)
    mp = ramp("#527a32", 8, 0.28, 0.14)
    c = clusters(rng, 2)
    h = c * 2
    t = 0.45 + (c - 0.5) * 0.5 + light(h) * 0.8 + (m - 0.6) * 0.5
    mc = quantize(np.clip(t, 0, 1), mp, 0.3, rng)
    img[mask, :3] = mc[mask]
    rim = edges_of(mask) & ~np.roll(mask, 1, 0)
    img[rim, :3] *= 0.85


def cobblestone(name="cobblestone", moss=False):
    return cell_stones(name, "#7b7b7b", 15, 12.5, moss=moss)


def mossy_cobblestone():
    return cobblestone("mossy_cobblestone", moss=True)


def bricks_pattern(name, brick_h, brick_w, mortar=2, stagger=0.5, rows_offset=None):
    """Course pattern: returns (brick id, distance to mortar, is mortar) arrays, tileable."""
    ids = np.zeros((N, N), int)
    inside = np.zeros((N, N))
    is_m = np.zeros((N, N), bool)
    rows = N // brick_h
    for r in range(rows):
        y0 = r * brick_h
        off = int((r % 2) * brick_w * stagger) if rows_offset is None else rows_offset[r]
        for y in range(y0, y0 + brick_h):
            for x in range(N):
                xx = (x + off) % N
                b = xx // brick_w
                lx, ly = xx - b * brick_w, y - y0
                ids[y, x] = r * 100 + b
                m = ly >= brick_h - mortar or lx >= brick_w - mortar
                is_m[y, x] = m
                inside[y, x] = 0 if m else min(lx + 1, ly + 1, brick_w - mortar - lx, brick_h - mortar - ly)
    return ids, inside, is_m


def bricks():
    rng = rng_for("bricks")
    ids, inside, is_m = bricks_pattern("bricks", 16, 32, mortar=3)
    reds = ramp("#9c5240", 12, 0.32, 0.12)
    mortar = ramp("#b8b0a4", 8, 0.22, 0.08)
    uniq = np.unique(ids)
    tone = dict(zip(uniq, rng.random(len(uniq))))
    tmap = np.vectorize(tone.get)(ids)
    surf = clusters(rng, 2) * 0.6 + fbm(rng, ((16, 0.5), (8, 0.5))) * 0.8
    h = np.clip(inside / 2.5, 0, 1) * 2.5 + surf
    t = 0.45 + (tmap - 0.5) * 0.22 + (surf - 0.6) * 0.18 + light(h) * 1.2
    img = rgba(quantize(np.clip(t, 0, 1), reds, 0.3, rng))
    mt = 0.45 + (clusters(rng, 2) - 0.5) * 0.4 + light(-is_m.astype(float)) * 0.5
    img[is_m, :3] = quantize(np.clip(mt, 0, 1), mortar)[is_m]
    # Mortar under a brick's lower edge sits in shadow.
    sh = is_m & ~np.roll(is_m, 1, 0)
    img[sh, :3] *= 0.8
    return soften(img, "built")


def stone_bricks(name="stone_bricks", base="#7a7a7a", moss=False, cracked=False):
    rng = rng_for(name)
    ids, inside, is_m = bricks_pattern(name, 16, 32, mortar=2)
    pal = ramp(base, 12, 0.38, 0.10)
    uniq = np.unique(ids)
    tone = dict(zip(uniq, rng.random(len(uniq))))
    tmap = np.vectorize(tone.get)(ids)
    surf = fbm(rng, ((16, 0.4), (8, 0.35), (4, 0.25)))
    bevel = np.clip(inside / 3.0, 0, 1)
    h = bevel * 2.6 + surf * 1.2
    t = 0.5 + (tmap - 0.5) * 0.14 + (surf - 0.5) * 0.2 + light(h) * 1.3
    t = np.where(is_m, 0.12, t)
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    if moss:
        add_moss(img, rng, np.where(is_m, 0, inside + 1), 1.0, 0.42)
    return soften(img, "built")


def smooth_stone():
    rng = rng_for("smooth_stone")
    pal = ramp("#9e9e9e", 10, 0.22, 0.08)
    f = fbm(rng, ((32, 0.5), (16, 0.3), (8, 0.2)))
    t = 0.5 + (f - 0.5) * 0.25
    img = compose(t, f, pal, rng, lk=0.6)
    d = np.minimum(np.minimum(XX, YY), np.minimum(N - 1 - XX, N - 1 - YY))
    img[d < 2, :3] = quantize(np.full((N, N), 0.3), pal)[d < 2]
    img[(XX < 2) | (YY < 2), :3] = pal[7]
    img[((XX > N - 3) | (YY > N - 3)), :3] = pal[2]
    return img


# ================================================================ ores


ORE_LAYOUT = [(10, 10), (40, 8), (26, 30), (52, 36), (10, 46), (36, 52), (56, 58)]


def ore(kind, bg_fn=None, name=None, size=1.25, color=None):
    name = name or f"{kind}_ore"
    rng = rng_for(name)
    img = bg_fn() if bg_fn else stone(name + "_bg", kind="rough")
    p = g16.ORES[kind] if color is None else [hexrgb(color)] * 3
    mid = p[1]
    gem = ramp(mid, 9, 0.5, 0.1, sat=1.15)
    accent = p[3] if len(p) > 3 else None
    used = np.zeros((N, N), bool)
    pts = list(ORE_LAYOUT)
    rng.shuffle(pts)
    for cx, cy in pts[: int(rng.integers(5, 7))]:
        cx += rng.uniform(-4, 4)
        cy += rng.uniform(-4, 4)
        blob = np.zeros((N, N), bool)
        for _ in range(int(rng.integers(3, 6))):
            ox, oy = rng.uniform(-3.5, 3.5, 2) * size
            r = rng.uniform(1.6, 2.8) * size
            dx, dy = torus_d(cx + ox, cy + oy)
            blob |= dx * dx + dy * dy <= r * r
        used |= blob
        din = dist_inside(blob, 4)
        h = np.clip(din, 0, 3)
        lt = light(h * 1.2, wrap=True)
        t = 0.5 + lt * 1.6 + (din - 1.5) * 0.06
        col = quantize(np.clip(t, 0, 1), gem, 0.25, rng)
        if accent is not None:
            acc = rng.random((N, N)) < 0.2
            col[acc & (din > 1)] = hx(accent)
        img[blob, :3] = col[blob]
        # Shadow on the stone below-right of the vein; it sits a little proud.
        sh = np.roll(np.roll(blob, 1, 0), 1, 1) & ~blob
        img[sh, :3] *= 0.7
        rim = (np.roll(np.roll(blob, -1, 0), -1, 1) & ~blob)
        img[rim, :3] = img[rim, :3] * 0.9 + 25
    return img


# ================================================================ wood


def planks(wood, name=None, base=None):
    name = name or f"{wood}_planks"
    rng = rng_for(name)
    base = g16.WOODS[wood]["plank"][2] if base is None else base
    pal = ramp(base, 12, 0.34, 0.12)
    t = np.zeros((N, N))
    h = np.zeros((N, N))
    seam = np.zeros((N, N), bool)
    for b in range(4):
        y0, y1 = b * 16, b * 16 + 16
        g1 = stretched_noise(rng, 32, 4)
        g2 = stretched_noise(rng, 16, 2)
        g3 = stretched_noise(rng, 8, 1)
        grain = g1 * 0.45 + g2 * 0.35 + g3 * 0.2
        tone = rng.uniform(-0.08, 0.08)
        rows = slice(y0, y1)
        t[rows] = 0.5 + tone + (grain[rows] - 0.5) * 0.45
        local = YY[rows] - y0
        bevel = np.clip(np.minimum(local + 0.5, 15.5 - local) / 2.0, 0, 1)
        h[rows] = bevel * 2.0 + grain[rows] * 0.8
        # Board end joint somewhere along the board.
        sx = int(rng.integers(6, 58))
        seam[rows, sx] = True
        # A knot on some boards.
        if rng.random() < 0.45:
            kx, ky = rng.uniform(4, 60), y0 + rng.uniform(5, 11)
            dx, dy = torus_d(kx, ky)
            d = np.hypot(dx / 2.2, dy)
            knot = (d < 2.2)
            ring = (d >= 2.2) & (d < 3.6)
            t = np.where(knot & (YY >= y0) & (YY < y1), t - 0.25, t)
            t = np.where(ring & (YY >= y0) & (YY < y1), t + 0.06, t)
    t = t + light(h) * 1.1
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    # Gaps between boards: dark line, lit line under it.
    for b in range(4):
        y = b * 16 + 15
        img[y, :, :3] = pal[0]
        img[(y + 1) % N, :, :3] = pal[min(9, len(pal) - 1)] * 0.95 + img[(y + 1) % N, :, :3] * 0.05
    img[seam, :3] = pal[1]
    sh = np.roll(seam, 1, 1) & (YY % 16 != 15)
    img[sh, :3] = img[sh, :3] * 0.85
    return soften(img, "built")


def log_side(wood, name=None):
    name = name or f"{wood}_log"
    rng = rng_for(name)
    w = g16.WOODS[wood]
    if wood == "birch":
        pal = ramp("#d8d6cf", 10, 0.22, 0.12)
        f = fbm(rng, ((32, 0.4), (16, 0.3), (8, 0.3)))
        s = stretched_noise(rng, 4, 32)
        img = compose(0.6 + (f - 0.5) * 0.2 + (s - 0.5) * 0.12, f + s, pal, rng, lk=0.6)
        dark = ramp("#2c2b27", 5, 0.12, 0.1)
        for _ in range(14):
            x, y = rng.integers(0, N, 2)
            ln = rng.integers(5, 15)
            th = rng.integers(1, 3)
            for i in range(ln):
                for j in range(th):
                    img[(y + j) % N, (x + i) % N, :3] = dark[rng.integers(0, 4)]
            img[(y + th) % N, x:(x + ln), :3] = img[(y + th) % N, x:(x + ln), :3] * 0.85
        return soften(img, "rough")
    pal = ramp(w["bark"][2], 12, 0.42, 0.12)
    cols = value_noise(rng, 2)[0]
    ridges = stretched_noise(rng, 4, 32) * 0.55 + stretched_noise(rng, 2, 16) * 0.3 + stretched_noise(rng, 8, 64) * 0.15
    h = ridges * 4.0
    t = 0.5 + (ridges - 0.5) * 0.6 + light(h) * 1.0 - occlusion(h, 1) * 0.15
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    if wood == "jungle":
        add_moss(img, rng, None, 1.0, 0.25)
    if wood == "cherry":
        for _ in range(12):
            x, y = rng.integers(0, N, 2)
            img[y, x:x + int(rng.integers(3, 7)), :3] = pal[9]
    return soften(img, "rough")


def log_top(wood, name=None, inner_base=None, bark_base=None):
    name = name or f"{wood}_log_top"
    rng = rng_for(name)
    w = g16.WOODS.get(wood, g16.WOODS["oak"])
    inner = ramp(w["inner"][2] if inner_base is None else inner_base, 10, 0.28, 0.12)
    cx = cy = 31.5
    euc = np.hypot(XX - cx, YY - cy)
    cheb = np.maximum(np.abs(XX - cx), np.abs(YY - cy))
    r = 0.6 * euc + 0.4 * cheb + value_noise(rng, 16) * 2.5 + value_noise(rng, 4) * 0.6
    rings = (np.sin(r * 2 * np.pi / 4.2) + 1) / 2
    t = 0.45 + rings * 0.25 + (clusters(rng, 2) - 0.5) * 0.1 - np.clip(1 - euc / 4, 0, 1) * 0.2
    img = rgba(quantize(np.clip(t, 0, 1), inner, 0.3, rng))
    # Bark rim, 4 pixels, with depth.
    bark = ramp(bark_base if bark_base is not None else (w["bark"][2] if wood != "birch" else "#d8d6cf"), 8, 0.36, 0.1)
    rim = cheb >= 27.5
    bt = 0.45 + (stretched_noise(rng, 2, 2) - 0.5) * 0.5 + np.where(cheb >= 30.5, -0.15, 0.05)
    img[rim, :3] = quantize(np.clip(bt, 0, 1), bark)[rim]
    ring_in = (cheb >= 26.5) & (cheb < 27.5)
    img[ring_in, :3] *= 0.78
    return soften(img, "built")


def leaves(wood, name=None):
    name = name or f"{wood}_leaves"
    rng = rng_for(name)
    base = g16.WOODS[wood]["leaves"][2]
    pal = ramp(base, 10, 0.4, 0.14)
    img = canvas()
    depth = np.full((N, N), -1.0)
    count = 280 if wood != "spruce" else 0
    shape_l = (4.2, 2.2) if wood != "jungle" else (6.5, 3.4)
    for i in range(count):
        cx, cy = rng.random(2) * N
        ang = rng.uniform(0, np.pi)
        a, b = shape_l[0] * rng.uniform(0.8, 1.2), shape_l[1] * rng.uniform(0.85, 1.15)
        dx, dy = torus_d(cx, cy)
        u = dx * np.cos(ang) + dy * np.sin(ang)
        v = -dx * np.sin(ang) + dy * np.cos(ang)
        e = (u / a) ** 2 + (v / b) ** 2
        m = e <= 1
        layer = i / count
        dome = np.sqrt(np.clip(1 - e, 0, 1))
        vein = np.abs(v) < 0.45
        t = 0.25 + layer * 0.35 + dome * 0.2 + (u / a) * -0.08 - vein * 0.06
        # Leaves on top hide those below.
        depth = np.where(m, layer, depth)
        img[m, :3] = quantize(np.clip(t, 0, 1), pal, 0.2, rng)[m]
        img[m, 3] = 255
    if wood == "spruce":
        # Needles: short strokes in fan clusters.
        for i in range(520):
            x0, y0 = rng.random(2) * N
            ang = rng.uniform(0.2, 1.4) * rng.choice([-1, 1]) + np.pi / 2
            ln = rng.uniform(3, 6)
            for ox in (0, -N, N):
                for oy in (0, -N, N):
                    if not (-8 < x0 + ox < N + 8 and -8 < y0 + oy < N + 8):
                        continue
                    d = segment_dist(x0 + ox, y0 + oy, x0 + ox + np.cos(ang) * ln, y0 + oy + np.sin(ang) * ln)
                    m = d < 0.65
                    t = 0.2 + i / 520 * 0.5 + rng.uniform(-0.05, 0.05)
                    img[m, :3] = quantize(np.full(m.sum(), np.clip(t, 0, 1)), pal)
                    img[m, 3] = 255
    # Shadow where a leaf sits on another: the darker under-layer near edges.
    opaque = img[..., 3] > 0
    sh = np.roll(np.roll(opaque, 1, 0), 1, 1) & opaque & ~np.roll(np.roll(edges_of(opaque), 0, 0), 0, 1)
    if wood == "cherry":
        bl = ramp("#f4c0d8", 6, 0.25, 0.1)
        for _ in range(26):
            x, y = rng.integers(0, N - 2, 2)
            img[y:y + 2, x:x + 2, :3] = bl[4]
            img[y + 1, x + 1, :3] = hx("#b04a7c")
            img[y:y + 2, x:x + 2, 3] = 255
    # Holes: a few gaps show the sky through the tree.
    return img


def stem(name, base, fleck):
    rng = rng_for(name)
    pal = ramp(base, 10, 0.36, 0.14, sat=1.1)
    ridges = stretched_noise(rng, 4, 32) * 0.6 + stretched_noise(rng, 2, 8) * 0.4
    h = ridges * 3
    t = 0.5 + (ridges - 0.5) * 0.5 + light(h)
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    fp = ramp(fleck, 6, 0.3, 0.1)
    for _ in range(18):
        x, y = rng.integers(0, N, 2)
        ln = rng.integers(2, 7)
        img[y:y + ln, x, :3] = fp[rng.integers(2, 6)]
    return soften(img, "rough")


# ================================================================ crafted


def wool(color):
    rng = rng_for(f"{color}_wool")
    base = g16.WOOL[color]
    pal = ramp(base, 10, 0.24 if color not in ("black", "gray") else 0.18, 0.12)
    # Knitted loops: rows of small arcs, offset every other row.
    t = np.zeros((N, N))
    u = (XX % 8) / 8
    row = (YY // 4)
    v = (YY % 4) / 4
    off = (row % 2) * 4
    u2 = ((XX + off) % 8) / 8
    loop = np.sin(u2 * np.pi) * (1 - np.abs(v - 0.5) * 1.4)
    fuzz = clusters(rng, 2)
    f = fbm(rng, ((32, 0.5), (16, 0.5)))
    h = loop * 0.9 + fuzz * 0.6
    t = 0.5 + (f - 0.5) * 0.18 + (fuzz - 0.5) * 0.18 + light(h) * 0.9 - occlusion(h, 1) * 0.2
    return soften(rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng)), "natural")


def metal_block(name, base, gem=False):
    rng = rng_for(name)
    pal = ramp(base, 12, 0.5 if not gem else 0.55, 0.12, sat=1.05)
    f = fbm(rng, ((32, 0.5), (16, 0.3), (8, 0.2)))
    d = np.minimum(np.minimum(XX, YY), np.minimum(N - 1 - XX, N - 1 - YY))
    # A raised plate with a bevelled frame, brushed or faceted.
    plate = np.clip((d - 3) / 3, 0, 1)
    h = np.clip(d, 0, 4) * 0.8 + plate * 0.5
    if gem:
        # Facets: large flat triangles meeting at the middle.
        ang = np.arctan2(YY - 31.5, XX - 31.5)
        facet = np.floor((ang + np.pi) / (np.pi / 4)) % 8
        tf = (facet / 8 - 0.5) * 0.25 + 0.05 * ((facet % 2) * 2 - 1)
        t = 0.55 + tf + (f - 0.5) * 0.12
    else:
        brush = stretched_noise(rng, 32, 1)
        t = 0.55 + (brush - 0.5) * 0.12 + (f - 0.5) * 0.12
    t = t + light(h * 1.3, wrap=False) * 1.3
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.25, rng))
    # Rivets in the corners.
    for cx, cy in ((6, 6), (57, 6), (6, 57), (57, 57)):
        dd = np.hypot(XX - cx, YY - cy)
        img[dd < 1.6, :3] = pal[-2]
        img[(dd >= 1.6) & (dd < 2.4) & (XX >= cx) & (YY >= cy), :3] = pal[2]
    # Diagonal shine on the top-left.
    shine = (np.abs((XX + YY) - 30) < 2.5) & (d > 6) & (XX < 32)
    img[shine, :3] = img[shine, :3] * 0.6 + pal[-1] * 0.4
    return img


def glass():
    """Clear pane in a slim frame. The frame sits in the outer 4 texels (1/16 of the block), the
    strip connected glass swaps away where two panes meet."""
    img = canvas()
    d = np.minimum(np.minimum(XX, YY), np.minimum(N - 1 - XX, N - 1 - YY))
    pal = ramp("#a9c4cc", 8, 0.4, 0.12)
    frame = d < 3
    img[frame, :3] = pal[4]
    img[frame, 3] = 235
    img[d < 1, :3] = pal[1]
    img[(d >= 1) & (d < 3) & ((XX < 3) | (YY < 3)), :3] = pal[6]
    img[(d >= 1) & (d < 3) & ((XX > 60) | (YY > 60)), :3] = pal[3]
    # Faint tint in the pane and two streaks of reflection.
    pane = d >= 3
    img[pane, :3] = pal[6]
    img[pane, 3] = 22
    for (x0, y0, ln, w) in ((12, 34, 16, 1.0), (17, 38, 7, 0.7), (36, 22, 13, 0.9), (41, 26, 5, 0.6)):
        dd = segment_dist(x0, y0, x0 + ln, y0 - ln)
        m = (dd < w) & pane
        img[m, :3] = 248
        img[m, 3] = 150
    return img


def glowstone():
    rng = rng_for("glowstone")
    pal = np.array([hx(c) for c in ("#6e4a20", "#8f6328", "#b07c2e", "#cf9a3a", "#e4b54c", "#f2cd68",
                                     "#fbe18e", "#fff1bc")])
    ids, d1, d2, pts = worley(rng, 22, 9, jitter_shape=0.6)
    tone = rng.random(len(pts))
    e = d2 - d1
    h = np.clip(e / 3, 0, 1) * 2.5
    t = 0.35 + tone[ids] * 0.45 + light(h) * 0.9
    t = np.where(e < 1.0, 0.08, t)
    return rgba(quantize(np.clip(t, 0, 1), pal, 0.4, rng))


def hay_side():
    rng = rng_for("hay_block_side")
    pal = ramp("#c9a22f", 10, 0.36, 0.14)
    s = stretched_noise(rng, 1, 16) * 0.6 + stretched_noise(rng, 2, 8) * 0.4
    h = s * 3
    t = 0.5 + (s - 0.5) * 0.5 + light(h) * 0.8
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    # Two red bands of twine.
    tw = ramp("#a3392b", 6, 0.3, 0.1)
    for y0 in (14, 46):
        band = (YY >= y0) & (YY < y0 + 4)
        bt = 0.5 + ((XX // 2) % 2) * 0.15 - (YY == y0 + 3) * 0.2
        img[band, :3] = quantize(np.clip(bt, 0, 1), tw)[band]
    return soften(img, "built")


def hay_top():
    rng = rng_for("hay_block_top")
    pal = ramp("#c9a22f", 10, 0.36, 0.14)
    tt = np.full((N, N), 0.45)
    for _ in range(500):
        x0, y0 = rng.random(2) * N
        ang = rng.uniform(0, np.pi)
        ln = rng.uniform(3, 8)
        d = segment_dist(x0, y0, x0 + np.cos(ang) * ln, y0 + np.sin(ang) * ln)
        tt[d < 0.6] = rng.uniform(0.3, 0.9)
    img = rgba(quantize(tt, pal, 0.2, rng))
    tw = ramp("#a3392b", 6, 0.3, 0.1)
    d = np.minimum(np.minimum(XX, YY), np.minimum(N - 1 - XX, N - 1 - YY))
    img[d < 3, :3] = tw[3]
    img[d == 2, :3] = tw[1]
    return soften(img, "built")


def bookshelf():
    rng = rng_for("bookshelf")
    img = planks("oak", "bookshelf_planks")
    wood = ramp(g16.WOODS["oak"]["plank"][1], 10, 0.3, 0.1)
    cols = ["#7a2a24", "#2d4f86", "#3f6f2e", "#7d5a24", "#5a3a6e", "#2f6b6b", "#8a7a5a", "#6b2d4a"]
    for shelf in range(2):
        y0 = 6 + shelf * 32
        y1 = y0 + 22
        img[y0 - 1:y1 + 1, 4:60, :3] = wood[1]
        x = 5
        while x < 59:
            w = int(rng.integers(3, 6))
            hgt = int(rng.integers(15, 22))
            if x + w > 59:
                break
            if rng.random() < 0.08:
                x += w
                continue
            c = ramp(cols[rng.integers(0, len(cols))], 6, 0.32, 0.1)
            yy = y1 - hgt
            img[yy:y1, x:x + w, :3] = c[3]
            img[yy:y1, x, :3] = c[4]
            img[yy:y1, x + w - 1, :3] = c[1]
            img[yy, x:x + w, :3] = c[5]
            band = yy + int(hgt * 0.25)
            img[band:band + 2, x:x + w, :3] = hx("#d8c27a")
            x += w
        img[y1:y1 + 3, 4:60, :3] = wood[6]
        img[y1 + 3, 4:60, :3] = wood[1]
    return img


def furnace_like(name, front=None):
    rng = rng_for(name)
    img = stone(name + "_s", "#7a7a7a", kind="built")
    d = np.minimum(np.minimum(XX, YY), np.minimum(N - 1 - XX, N - 1 - YY))
    pal = ramp("#7a7a7a", 10, 0.4, 0.1)
    img[d < 4, :3] = quantize(np.full((N, N), 0.6), pal)[d < 4]
    img[(d < 4) & ((XX < 2) | (YY < 2)), :3] = pal[8]
    img[(d < 4) & ((XX > 61) | (YY > 61)), :3] = pal[2]
    if front == "furnace":
        mouth = (XX >= 16) & (XX < 48) & (YY >= 32) & (YY < 54)
        img[mouth, :3] = hx("#1a1a1a")
        img[(XX >= 16) & (XX < 48) & (YY >= 30) & (YY < 32), :3] = pal[1]
        grate = mouth & ((XX % 6) < 2) & (YY > 44)
        img[grate, :3] = hx("#3a3a3a")
        top = (XX >= 18) & (XX < 46) & (YY >= 12) & (YY < 22)
        img[top, :3] = pal[2]
        img[top & ((XX - 18) % 7 < 3), :3] = pal[0]
    return img


def crafting_top():
    rng = rng_for("crafting_table_top")
    img = planks("oak", "crafting_planks")
    pal = ramp(g16.WOODS["oak"]["plank"][2], 10, 0.4, 0.1)
    d = np.minimum(np.minimum(XX, YY), np.minimum(N - 1 - XX, N - 1 - YY))
    img[d < 4, :3] = pal[2]
    img[d < 2, :3] = pal[1]
    grid = ((XX % 21) < 2) | ((YY % 21) < 2)
    img[grid & (d >= 4), :3] = pal[1]
    return img


def cactus_side():
    rng = rng_for("cactus_side")
    pal = ramp("#4c8a2e", 10, 0.38, 0.14)
    ribs = (np.cos(XX * 2 * np.pi / 16) + 1) / 2
    f = fbm(rng, ((16, 0.5), (8, 0.5)))
    h = ribs * 3 + f
    t = 0.45 + (ribs - 0.5) * 0.3 + light(h) * 0.9
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.25, rng))
    d = np.minimum(XX, N - 1 - XX)
    img[d < 2, :3] = pal[1]
    for _ in range(14):
        x = int(rng.choice([8, 24, 40, 56])) + int(rng.integers(-1, 2))
        y = int(rng.integers(0, N))
        img[y, x, :3] = hx("#efe7c2")
        img[(y + 1) % N, x, :3] = hx("#2a3a1a")
    return img


REGISTRY = {
    "stone": stone,
    "dirt": dirt,
    "grass_block_top": grass_top,
    "grass_block_side": grass_side,
    "snow_grass_side": snow_grass_side,
    "sand": sand,
    "red_sand": red_sand,
    "gravel": gravel,
    "clay": clay,
    "bedrock": bedrock,
    "snow": snow,
    "ice": ice,
    "packed_ice": packed_ice,
    "water": water,
    "lava": lava,
    "obsidian": obsidian,
    "sandstone_side": sandstone_side,
    "sandstone_top": sandstone_top,
    "sandstone_bottom": sandstone_bottom,
    "cobblestone": cobblestone,
    "mossy_cobblestone": mossy_cobblestone,
    "bricks": bricks,
    "stone_bricks": stone_bricks,
    "smooth_stone": smooth_stone,
    "glass": glass,
    "glowstone": glowstone,
    "hay_block_side": hay_side,
    "hay_block_top": hay_top,
    "bookshelf": bookshelf,
    "crafting_table_top": crafting_top,
    "cactus_side": cactus_side,
}
for _t in g16.TERRACOTTA:
    REGISTRY[_t] = lambda n=_t: terracotta(n)
for _k in g16.ORES:
    REGISTRY[f"{_k}_ore"] = lambda k=_k: ore(k)
for _w in g16.WOODS:
    REGISTRY[f"{_w}_planks"] = lambda w=_w: planks(w)
    REGISTRY[f"{_w}_log"] = lambda w=_w: log_side(w)
    REGISTRY[f"{_w}_log_top"] = lambda w=_w: log_top(w)
    REGISTRY[f"{_w}_leaves"] = lambda w=_w: leaves(w)
for _c in g16.WOOL:
    REGISTRY[f"{_c}_wool"] = lambda c=_c: wool(c)
import beds as _beds  # noqa: E402  (frame, sheet, pillow, blankets)
REGISTRY.update(_beds.BLOCKS)


# ================================================================ batch 2: nether, end, caves, farm


SRC16 = os.path.join(os.path.dirname(os.path.abspath(__file__)), "src16", "block")


def src_mean(name, fallback="#808080"):
    """Average colour of a block's 16x16 design: hand-made art starts from the same colour."""
    p = os.path.join(SRC16, name + ".png")
    if not os.path.exists(p):
        return fallback
    from PIL import Image
    a = np.asarray(Image.open(p).convert("RGBA"), dtype=np.float64)
    m = a[..., 3] > 127
    return a[..., :3][m].mean(0) if m.any() else fallback


def lumpy(name, base, cells=24, min_d=9, spread=0.4, kind="rough", tones=0.25):
    """Knobbly rock made of fused lumps (netherrack, end stone)."""
    rng = rng_for(name)
    pal = ramp(base, 11, spread, 0.14)
    ids, d1, d2, pts = worley(rng, cells, min_d, jitter_shape=0.6)
    tone = rng.random(len(pts))
    dome = np.clip((d2 - d1) / 6, 0, 1) ** 0.7
    c = clusters(rng, 2)
    h = dome * 3 + c * 1.2
    t = 0.5 + (tone[ids] - 0.5) * tones + (c - 0.5) * 0.25 + light(h) * 1.2 - occlusion(h, 2) * 0.15
    return soften(rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng)), kind)


def netherrack():
    return lumpy("netherrack", src_mean("netherrack", "#723232"), 28, 8.5)


def end_stone():
    img = lumpy("end_stone", src_mean("end_stone", "#dbdea0"), 18, 11, spread=0.28, kind="natural")
    rng = rng_for("end_stone_pits")
    # Little craters: a dark dent with a lit lower rim.
    for _ in range(9):
        cx, cy = rng.random(2) * N
        r = rng.uniform(1.2, 2.6)
        dx, dy = torus_d(cx, cy)
        dd = np.hypot(dx, dy)
        img[dd < r, :3] *= 0.8
        rim = (dd >= r) & (dd < r + 1.0) & (dx + dy > 0)
        img[rim, :3] = np.clip(img[rim, :3] * 1.08, 0, 255)
    return img


def soul_sand():
    rng = rng_for("soul_sand")
    img = sand("soul_sand_grain", src_mean("soul_sand", "#54402f"))
    # Faint faces pressed into the sand: two eyes and a wailing mouth.
    for (cx, cy) in ((16, 18), (46, 44)):
        for ex in (-5, 5):
            dx, dy = torus_d(cx + ex, cy)
            e = (dx / 2.2) ** 2 + (dy / 3.0) ** 2 <= 1
            img[e, :3] *= 0.62
        dx, dy = torus_d(cx, cy + 9)
        m = (dx / 3.0) ** 2 + (dy / 2.2) ** 2 <= 1
        img[m, :3] *= 0.62
    return img


def soul_soil():
    return dirt("soul_soil", src_mean("soul_soil", "#4c3a2e"), pebbles=4)


def magma_block():
    rng = rng_for("magma_block")
    ids, d1, d2, pts = worley(rng, 16, 11, jitter_shape=0.5)
    e = d2 - d1
    crust = ramp("#4a2416", 8, 0.3, 0.12)
    glow = np.array([hexrgb(c) for c in ("#a8300c", "#d24e10", "#f07a1c", "#fbb040", "#ffe08a")])
    tone = rng.random(len(pts))
    h = np.clip(e / 4, 0, 1) * 2.5
    t = 0.45 + (tone[ids] - 0.5) * 0.3 + light(h) * 1.1 + (clusters(rng, 2) - 0.5) * 0.2
    img = rgba(quantize(np.clip(t, 0, 1), crust, 0.3, rng))
    seam = e < 2.2
    gt = np.clip(1 - e / 2.2, 0, 1)
    img[seam, :3] = quantize(gt, glow)[seam]
    return img


def nylium_top(name, base):
    rng = rng_for(name)
    pal = ramp(base, 10, 0.34, 0.14, sat=1.1)
    f = fbm(rng, ((16, 0.4), (8, 0.35), (4, 0.25)))
    c = clusters(rng, 2)
    h = f * 2 + c * 1.5
    t = 0.45 + (f - 0.5) * 0.35 + (c - 0.5) * 0.4
    img = compose(t, h, pal, rng, lk=1.0, ao=0.1)
    return soften(img, "natural")


def nylium_side(name, base):
    return fringe_side(name, netherrack, base, depth=12)


def basalt_side():
    rng = rng_for("basalt_side")
    pal = ramp(src_mean("basalt_side", "#4c4c52"), 10, 0.4, 0.12)
    cols = np.array([0, 13, 22, 37, 46, 58, 64])
    h = np.zeros((N, N))
    for a, b in zip(cols[:-1], cols[1:]):
        m = (XX >= a) & (XX < b)
        local = np.minimum(XX - a + 0.5, b - XX - 0.5)
        h[m] = np.clip(local[m] / 2.5, 0, 1) * 2.5 + rng.uniform(-0.4, 0.4)
    s = stretched_noise(rng, 4, 32)
    t = 0.5 + (s - 0.5) * 0.3 + light(h + s) * 1.0
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    for x in cols[1:-1]:
        img[:, x - 1, :3] = pal[1]
    return soften(img, "rough")


def basalt_top():
    rng = rng_for("basalt_top")
    pal = ramp(src_mean("basalt_top", "#56565c"), 10, 0.36, 0.12)
    cheb = np.maximum(np.abs(XX - 31.5), np.abs(YY - 31.5))
    euc = np.hypot(XX - 31.5, YY - 31.5)
    r = 0.5 * cheb + 0.5 * euc + value_noise(rng, 8) * 3
    rings = (np.sin(r * 2 * np.pi / 7) + 1) / 2
    t = 0.45 + rings * 0.2 + (clusters(rng, 2) - 0.5) * 0.2
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    img[cheb >= 30, :3] = pal[2]
    return soften(img, "rough")


def blackstone():
    img = stone("blackstone", src_mean("blackstone", "#2e2a30"), warm=0.14, kind="rough")
    rng = rng_for("blackstone_bits")
    gold = ramp("#5a4e52", 6, 0.3, 0.1)
    for _ in range(14):
        cx, cy = rng.random(2) * N
        dx, dy = torus_d(cx, cy)
        m = (np.abs(dx) + np.abs(dy) * 1.4) < rng.uniform(1.5, 3)
        img[m, :3] = gold[rng.integers(2, 6)]
    return img


def quartz_block():
    rng = rng_for("quartz_block")
    pal = ramp(src_mean("quartz_block", "#ebe5dc"), 9, 0.14, 0.12)
    f = fbm(rng, ((32, 0.5), (16, 0.3), (8, 0.2)))
    vein = np.abs(stretched_noise(rng, 16, 8) - 0.5) < 0.03
    t = 0.55 + (f - 0.5) * 0.3 - vein * 0.25
    return compose(t, f, pal, rng, lk=0.5, dither=0.2)


def tiled(name, base, tile=32, spread=0.36, kind="built"):
    """Square tiles with bevelled edges (purpur, polished stones)."""
    rng = rng_for(name)
    pal = ramp(base, 11, spread, 0.12)
    lx, ly = XX % tile, YY % tile
    inside = np.minimum(np.minimum(lx + 0.5, ly + 0.5), np.minimum(tile - 0.5 - lx, tile - 0.5 - ly))
    f = fbm(rng, ((16, 0.4), (8, 0.35), (4, 0.25)))
    h = np.clip(inside / 3, 0, 1) * 2.5 + f
    tone = rng.random((N // tile, N // tile))[(YY // tile).astype(int), (XX // tile).astype(int)]
    t = 0.5 + (tone - 0.5) * 0.1 + (f - 0.5) * 0.2 + light(h) * 1.2
    t = np.where(inside < 1, 0.12, t)
    return soften(rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng)), kind)


def moss_block():
    rng = rng_for("moss_block")
    pal = ramp(src_mean("moss_block", "#5a7a2e"), 10, 0.3, 0.14)
    f = fbm(rng, ((16, 0.4), (8, 0.35), (4, 0.25)))
    c = clusters(rng, 2)
    lumps = value_noise(rng, 8)
    h = lumps * 3 + c
    t = 0.45 + (f - 0.5) * 0.3 + (c - 0.5) * 0.35
    return soften(compose(t, h, pal, rng, lk=1.0, ao=0.12), "natural")


def rocky_dirt():
    return dirt("rocky_dirt", src_mean("rocky_dirt", "#7a604a"), pebbles=22)


def farmland():
    rng = rng_for("farmland")
    pal = ramp(src_mean("farmland", "#5a3e26"), 10, 0.32, 0.12)
    furrow = (np.cos(YY * 2 * np.pi / 16) + 1) / 2
    f = fbm(rng, ((16, 0.4), (8, 0.3), (4, 0.3)))
    c = clusters(rng, 2)
    lumps = clusters(rng, 4)
    h = furrow * 1.4 + c * 1.2 + lumps * 1.5
    t = 0.48 + (f - 0.5) * 0.3 + (c - 0.5) * 0.3 - furrow * 0.06
    img = compose(t, h, pal, rng, lk=1.1, ao=0.15)
    d = np.minimum(np.minimum(XX, YY), np.minimum(N - 1 - XX, N - 1 - YY))
    img[d < 2, :3] *= 0.8
    return soften(img, "natural")


def dripstone_block():
    rng = rng_for("dripstone_block")
    pal = ramp(src_mean("dripstone_block", "#866b5c"), 10, 0.34, 0.12)
    s = stretched_noise(rng, 4, 24) * 0.6 + stretched_noise(rng, 2, 8) * 0.4
    h = s * 3.5
    t = 0.5 + (s - 0.5) * 0.4 + light(h) * 1.0
    return soften(rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng)), "rough")


def wart_block(name):
    rng = rng_for(name)
    pal = ramp(src_mean(name, "#7a1010"), 10, 0.38, 0.14, sat=1.1)
    ids, d1, d2, pts = worley(rng, 40, 7, jitter_shape=0.5)
    dome = np.clip((d2 - d1) / 3, 0, 1)
    c = clusters(rng, 2)
    h = dome * 2.5 + c
    t = 0.45 + (rng.random(len(pts))[ids] - 0.5) * 0.25 + light(h) * 1.1 - occlusion(h, 1) * 0.2
    return soften(rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng)), "rough")


def short_grass(name="short_grass", base="#6aa048", count=17, tall=(26, 58)):
    """A tuft of thin blades, each lit on its left edge, darker towards the root."""
    rng = rng_for(name)
    pal = ramp(base, 10, 0.4, 0.14)
    img = canvas()
    for i in range(count):
        x0 = rng.uniform(6, 58)
        top = rng.uniform(N - tall[1], N - tall[0])
        lean = rng.uniform(-10, 10)
        w0 = rng.uniform(3.4, 5.0)
        pts = [(x0, 64.0), (x0 + lean * 0.4, (64 + top) / 2), (x0 + lean, top)]
        for k in range(len(pts) - 1):
            (ax, ay), (bx, by) = pts[k], pts[k + 1]
            d = segment_dist(ax, ay, bx, by)
            wk = w0 * (1 - 0.45 * k)
            m = d < wk / 2
            frac = np.clip((64 - YY) / (64 - top), 0, 1)
            t = 0.25 + frac * 0.55 + (i / count - 0.5) * 0.12
            left = (XX + 0.5) < (ax + (bx - ax) * np.clip((YY - ay) / (by - ay + 1e-9), 0, 1))
            t = t + left * 0.08
            img[m, :3] = quantize(np.clip(t, 0, 1), pal)[m]
            img[m, 3] = 255
    return img


def metal(name, gem=False):
    return lambda: metal_block(name, src_mean(name, "#c0c0c0"), gem)


REGISTRY.update({
    "netherrack": netherrack,
    "end_stone": end_stone,
    "soul_sand": soul_sand,
    "soul_soil": soul_soil,
    "magma_block": magma_block,
    "crimson_nylium": lambda: nylium_top("crimson_nylium", src_mean("crimson_nylium", "#8a1c1c")),
    "warped_nylium": lambda: nylium_top("warped_nylium", src_mean("warped_nylium", "#2a7a6a")),
    "crimson_nylium_side": lambda: nylium_side("crimson_nylium_side", "#8a1c1c"),
    "warped_nylium_side": lambda: nylium_side("warped_nylium_side", "#2a7a6a"),
    "basalt_side": basalt_side,
    "basalt_top": basalt_top,
    "blackstone": blackstone,
    "quartz_block": quartz_block,
    "purpur_block": lambda: tiled("purpur_block", src_mean("purpur_block", "#a67aa6")),
    "end_stone_bricks": lambda: stone_bricks("end_stone_bricks", src_mean("end_stone_bricks", "#dcdca0")),
    "nether_bricks": lambda: stone_bricks("nether_bricks", src_mean("nether_bricks", "#3a1c20")),
    "moss_block": moss_block,
    "rocky_dirt": rocky_dirt,
    "farmland": farmland,
    "dripstone_block": dripstone_block,
    "nether_wart_block": lambda: wart_block("nether_wart_block"),
    "warped_wart_block": lambda: wart_block("warped_wart_block"),
    "crimson_stem": lambda: stem("crimson_stem", src_mean("crimson_stem", "#5a1c2a"), "#c02a2a"),
    "warped_stem": lambda: stem("warped_stem", src_mean("warped_stem", "#2a4a4a"), "#2ac0a0"),
    "crimson_planks": lambda: planks("oak", "crimson_planks", src_mean("crimson_planks", "#6a3048")),
    "warped_planks": lambda: planks("oak", "warped_planks", src_mean("warped_planks", "#2b6b62")),
    "short_grass": short_grass,
    "cobblestone_bricks": lambda: stone_bricks("cobblestone_bricks", "#7a7a7a"),
})
for _n in ("iron_block", "gold_block", "copper_block", "tin_block", "silver_block", "bronze_block", "lead_block",
           "tungsten_block", "platinum_block"):
    REGISTRY[_n] = metal(_n)
for _n in ("diamond_block", "ruby_block", "sapphire_block", "topaz_block", "amethyst_block"):
    REGISTRY[_n] = metal(_n, gem=True)


# ================================================================ batch 3: crafted and farm blocks


def gourd_side(name, base, stripes=None):
    rng = rng_for(name)
    pal = ramp(base, 10, 0.36, 0.14)
    ribs = (np.cos(XX * 2 * np.pi / 16) + 1) / 2
    f = fbm(rng, ((16, 0.5), (8, 0.3), (4, 0.2)))
    h = ribs * 2.5 + f
    t = 0.5 + (ribs - 0.5) * 0.25 + (f - 0.5) * 0.2 + light(h) * 0.9
    if stripes:
        st = (np.abs(((XX + value_noise(rng, 16) * 4) % 16) - 8) < 2.5)
        t = t - st * 0.22
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    return soften(img, "built")


def gourd_top(name, base, stem_col):
    rng = rng_for(name)
    pal = ramp(base, 10, 0.36, 0.14)
    ang = np.arctan2(YY - 31.5, XX - 31.5)
    r = np.hypot(XX - 31.5, YY - 31.5)
    ribs = (np.cos(ang * 8) + 1) / 2
    f = fbm(rng, ((16, 0.5), (8, 0.5)))
    h = ribs * 2 * np.clip(r / 20, 0, 1) - np.clip(1 - r / 30, 0, 1) * 2 + f
    t = 0.5 + (ribs - 0.5) * 0.2 + light(h, wrap=False) * 1.0
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    sp = ramp(stem_col, 6, 0.35, 0.1)
    stem_m = (np.abs(XX - 31.5) < 3.5) & (np.abs(YY - 31.5) < 3.5)
    img[stem_m, :3] = sp[3]
    img[stem_m & ((XX < 30) | (YY < 30)), :3] = sp[5]
    img[stem_m & ((XX > 33) | (YY > 33)), :3] = sp[1]
    return img


def melon_side():
    return gourd_side("melon_side", "#6fa82e", stripes=True)


def melon_top():
    return gourd_top("melon_top", "#6fa82e", "#8a7a3a")


def pumpkin_side():
    return gourd_side("pumpkin_side", src_mean("pumpkin_side", "#d07a18"))


def pumpkin_top():
    return gourd_top("pumpkin_top", src_mean("pumpkin_top", "#d07a18"), "#5a6a2a")


def cactus_top():
    rng = rng_for("cactus_top")
    pal = ramp("#4c8a2e", 10, 0.38, 0.14)
    r = np.hypot(XX - 31.5, YY - 31.5)
    ang = np.arctan2(YY - 31.5, XX - 31.5)
    ribs = (np.cos(ang * 8) + 1) / 2
    t = 0.55 + ribs * 0.12 - np.clip(1 - r / 10, 0, 1) * 0.2
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    d = np.minimum(np.minimum(XX, YY), np.minimum(N - 1 - XX, N - 1 - YY))
    img[d < 2, :3] = pal[1]
    return img


def sponge():
    rng = rng_for("sponge")
    pal = ramp(src_mean("sponge", "#c8c040"), 10, 0.36, 0.12)
    img = lumpy("sponge_body", src_mean("sponge", "#c8c040"), 30, 8, spread=0.3)
    for _ in range(26):
        cx, cy = rng.random(2) * N
        dx, dy = torus_d(cx, cy)
        r = rng.uniform(1.2, 2.6)
        dd = np.hypot(dx, dy * 1.2)
        img[dd < r, :3] = pal[1]
        rim = (dd >= r) & (dd < r + 1) & (dx + dy > 0)
        img[rim, :3] = pal[7]
    return img


def shroomlight():
    rng = rng_for("shroomlight")
    pal = np.array([hexrgb(c) for c in ("#9a3a12", "#c25a1a", "#de7a24", "#f09a34", "#f8b84a", "#fcd27a", "#fee8b0")])
    ids, d1, d2, pts = worley(rng, 26, 8, jitter_shape=0.5)
    dome = np.clip((d2 - d1) / 3, 0, 1)
    t = 0.3 + dome * 0.5 + (rng.random(len(pts))[ids] - 0.5) * 0.25
    t = np.where(d2 - d1 < 1.0, 0.05, t)
    return rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))


def thatch():
    rng = rng_for("thatch")
    pal = ramp(src_mean("thatch", "#b89a4a"), 10, 0.38, 0.14)
    tt = np.full((N, N), 0.35)
    for _ in range(420):
        x0, y0 = rng.random(2) * N
        ln = rng.uniform(6, 14)
        ang = np.pi / 2 + rng.uniform(-0.25, 0.25)
        for oy in (0, -N, N):
            d = segment_dist(x0, y0 + oy, x0 + np.cos(ang) * ln, y0 + oy + np.sin(ang) * ln)
            tt[d < 0.7] = rng.uniform(0.35, 0.95)
    for y0 in range(0, N, 16):
        tt[y0:y0 + 2] *= 0.6
    return soften(rgba(quantize(np.clip(tt, 0, 1), pal, 0.2, rng)), "built")


def furnace_side():
    return furnace_like("furnace_side")


def furnace_top():
    return furnace_like("furnace_top")


def furnace_front():
    return furnace_like("furnace_front", "furnace")


REGISTRY.update({
    "melon_side": melon_side,
    "melon_top": melon_top,
    "pumpkin_side": pumpkin_side,
    "pumpkin_top": pumpkin_top,
    "cactus_top": cactus_top,
    "sponge": sponge,
    "shroomlight": shroomlight,
    "thatch": thatch,
    "furnace_side": furnace_side,
    "furnace_top": furnace_top,
    "furnace_front": furnace_front,
    "mossy_cobblestone_bricks": lambda: stone_bricks("mossy_cobblestone_bricks", "#7a7a7a", moss=True),
    "nether_gold_ore": lambda: ore("gold", netherrack, "nether_gold_ore", 1.0),
    "nether_quartz_ore": lambda: ore("quartz", netherrack, "nether_quartz_ore", 1.0, color="#ece4d8"),
    "redstone_block": metal("redstone_block"),
    "crimson_stem_top": lambda: log_top("oak", "crimson_stem_top", src_mean("crimson_planks", "#6a3048"), src_mean("crimson_stem", "#5a1c2a")),
    "warped_stem_top": lambda: log_top("oak", "warped_stem_top", src_mean("warped_planks", "#2b6b62"), src_mean("warped_stem", "#2a4a4a")),
})
