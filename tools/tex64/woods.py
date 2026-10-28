"""Every wood gets its own grain, not just its own colour.

Each wood has a profile (WOODS below) that sets how its boards are laid (rows, vertical boards,
diagonal, parquet, herringbone, basket weave), how wide they are, which grain figure the wood
shows (cathedral arches, dense straight lines, wavy interlock, palm fibres, mottle...), its marks
(knots, pores, rays, bird's eyes, resin, cracks, nails) and its edges (gap, bevel, wear). The same
profile drives the log bark, the end grain, the doors and the trapdoors, so a wood reads the same
on every block made from it.

`register(REGISTRY)` puts the generators into blocks.REGISTRY.
"""

import os
import sys

import numpy as np

from style import (N, XX, YY, blur, fbm, hexrgb, light, mix, occlusion, quantize, ramp, rgba, rng_for,
                   segment_dist, soften, stretched_noise, value_noise, worley)

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import gen_textures as g16  # noqa: E402  (palettes only)

HERE = os.path.dirname(os.path.abspath(__file__))
SRC16 = os.path.join(HERE, "src16", "block")
TAU = 2 * np.pi


def hx(c):
    return hexrgb(c) if isinstance(c, str) else np.asarray(c, dtype=np.float64)


def src_mean(name, fallback):
    p = os.path.join(SRC16, name + ".png")
    if not os.path.exists(p):
        return hx(fallback)
    from PIL import Image
    a = np.asarray(Image.open(p).convert("RGBA"), dtype=np.float64)
    m = a[..., 3] > 127
    return a[..., :3][m].mean(0) if m.any() else hx(fallback)


# ---------------------------------------------------------------- profiles
#
# layout: rows | cols | diag | parquet | herring | basket
# sizes:  board widths across the grain (rows/cols, must add up to 64), or the strip width
# joints: butt joints per row (rows/cols)
# grain:  cathedral | straight | wavy | fibre | mottle | cloud
# dens:   grain lines per pixel across the board; amp: how strong the grain is painted
# marks:  dict of extra features and how many / how strong
# gap:    1 or 2 pixel dark seam; bevel: pixels of rounded edge; wear: lighter chipped edges
# bark / top / door / trap: log side, end grain, door window pattern, trapdoor pattern

WOODS = {
    "oak": dict(layout="rows", sizes=[16, 16, 16, 16], joints=1, grain="cathedral", dens=0.22, amp=0.24,
                curv=9.0, warp=0.6, marks=dict(knots=0.5), gap=1, bevel=2.0,
                bark="furrowed", top=dict(rings=4.2, heart=0.10, cracks=1), door="two_panes", trap="four_slots"),
    "birch": dict(layout="rows", sizes=[21, 21, 22], joints=1, grain="straight", dens=0.20, amp=0.10,
                  marks=dict(flecks=34), gap=1, bevel=3.0, soft=True,
                  bark="birch", top=dict(rings=2.6, heart=0.0, round=1.0), door="grid_panes", trap="grid"),
    "spruce": dict(layout="rows", sizes=[11, 10, 11, 11, 10, 11], joints=2, grain="straight", dens=0.55, amp=0.42,
                   sharp=3.0, marks=dict(knots=0.8, knot_size=0.65), gap=1, bevel=1.5,
                   bark="scaly", top=dict(rings=2.4, heart=0.06), door="solid_cross", trap="solid_brace"),
    "jungle": dict(layout="rows", sizes=[22, 21, 21], joints=1, grain="straight", dens=0.30, amp=0.18,
                   marks=dict(ribbon=0.20, pores=70), gap=1, bevel=2.5,
                   bark="jungle", top=dict(rings=5.5, heart=0.14, ecc=4.0), door="round_panes", trap="porthole"),
    "acacia": dict(layout="rows", sizes=[16, 16, 16, 16], joints=2, grain="cathedral", dens=0.26, amp=0.40,
                   curv=3.0, marks=dict(heartstreak=0.25), gap=1, bevel=1.5,
                   bark="twisted", top=dict(rings=3.0, heart=0.22, ecc=3.0), door="slot", trap="slats"),
    "dark_oak": dict(layout="parquet", block=32, sizes=8, grain="straight", dens=0.30, amp=0.22,
                     marks=dict(rays=60), gap=1, bevel=1.5,
                     bark="plated", top=dict(rings=3.4, heart=0.18, cracks=3), door="small_panes", trap="iron_bands"),
    "cherry": dict(layout="rows", sizes=[8] * 8, joints=1, grain="cloud", dens=0.2, amp=0.16,
                   marks=dict(sheen=0.10), gap=1, bevel=1.0, soft=True,
                   bark="lenticel", top=dict(rings=3.0, heart=0.08, round=1.0), door="lattice", trap="lattice"),
    "mangrove": dict(layout="cols", sizes=[13, 13, 12, 13, 13], joints=1, grain="wavy", dens=0.34, amp=0.30,
                     wave=(3.0, 2), marks=dict(pores=50, chips=10), gap=1, bevel=2.0,
                     bark="stringy", top=dict(rings=2.8, heart=0.12, ecc=5.0), door="vertical_slats", trap="slats_v"),
    "pale_oak": dict(layout="rows", sizes=[16, 16, 16, 16], joints=1, grain="straight", dens=0.32, amp=0.16,
                     raised=1.6, marks=dict(cracks=7, nails=True), gap=2, bevel=2.5, wear=0.10,
                     bark="smooth_fissure", top=dict(rings=3.8, heart=0.0, cracks=5), door="one_pane", trap="four_slots"),
    "crimson": dict(layout="cols", sizes=[16, 16, 16, 16], joints=1, grain="fibre", dens=0.5, amp=0.30,
                    marks=dict(spores=("#e04a3a", 14), veins=0.30), gap=1, bevel=2.0,
                    bark=None, top=None, door="vein", trap="vein"),
    "warped": dict(layout="diag", sizes=16, grain="fibre", dens=0.5, amp=0.30,
                   marks=dict(spores=("#3ee0c0", 14), veins=0.30), gap=1, bevel=2.0,
                   bark=None, top=None, door="vein", trap="vein"),
    "maple": dict(layout="rows", sizes=[16, 16, 16, 16], joints=1, grain="straight", dens=0.24, amp=0.14,
                  marks=dict(birdseye=34), gap=1, bevel=2.0,
                  bark="shaggy", top=dict(rings=3.2, heart=0.06), door="arch", trap="four_slots"),
    "walnut": dict(layout="herring", sizes=8, grain="wavy", dens=0.30, amp=0.34, wave=(2.0, 1),
                   marks=dict(), gap=1, bevel=1.5,
                   bark="diamond", top=dict(rings=3.6, heart=0.30, ecc=2.0), door="diamond", trap="diamond"),
    "araucaria": dict(layout="rows", sizes=[13, 13, 12, 13, 13], joints=1, grain="cathedral", dens=0.40, amp=0.30,
                      curv=7.0, marks=dict(knots=0.6, knot_size=0.8, resin=8), gap=1, bevel=1.5,
                      bark="ringed", top=dict(rings=2.2, heart=0.10), door="two_panes", trap="four_slots"),
    "buriti": dict(layout="cols", sizes=[10, 11, 11, 10, 11, 11], joints=1, grain="fibre", dens=0.6, amp=0.22,
                   marks=dict(dots=150), gap=1, bevel=1.5,
                   bark="palm", top=dict(palm=True), door="two_panes", trap="four_slots"),
    "coconut": dict(layout="rows", sizes=[21, 21, 22], joints=1, grain="fibre", dens=0.5, amp=0.20,
                    marks=dict(dots=260), gap=1, bevel=2.0,
                    bark="palm_rings", top=dict(palm=True), door="vertical_slats", trap="slats"),
    "ipe": dict(layout="rows", sizes=[13, 13, 13, 13, 12], joints=1, grain="straight", dens=0.70, amp=0.20,
                marks=dict(screws=True, pores=40), gap=2, bevel=1.0,
                bark="flaky", top=dict(rings=1.8, heart=0.20), door="two_panes", trap="four_slots"),
    "jabuticaba": dict(layout="diag", sizes=16, grain="mottle", dens=0.25, amp=0.30,
                       marks=dict(), gap=1, bevel=2.0,
                       bark="patchy", top=dict(rings=2.6, heart=0.12, ecc=3.0), door="two_panes", trap="four_slots"),
    "echo": dict(layout="basket", block=16, sizes=8, grain="straight", dens=0.28, amp=0.14,
                 marks=dict(ripple=0.22), gap=1, bevel=1.5,
                 bark="smooth_streak", top=dict(rings=3.0, heart=0.0, round=1.0), door="two_panes", trap="four_slots"),
    "skyroot": dict(layout="cols", sizes=[16, 16, 16, 16], joints=1, grain="cloud", dens=0.2, amp=0.22,
                    marks=dict(), gap=1, bevel=3.0, soft=True,
                    bark="smooth_streak", top=dict(rings=4.6, heart=0.0, round=1.0), door="two_panes", trap="four_slots"),
}


def colours(wood):
    """(plank, bark, inner) base colours: the old palette's middle or the 16x16 design's mean."""
    if wood in g16.WOODS:
        w = g16.WOODS[wood]
        bark = w["bark"][2] if wood != "birch" else "#d8d6cf"
        return hx(w["plank"][2]), hx(bark), hx(w["inner"][2])
    fall = {"crimson": ("#6a3048", "#5a1c2a"), "warped": ("#2b6b62", "#2a4a4a")}.get(wood, ("#9a7744", "#5a4229"))
    plank = src_mean(f"{wood}_planks", fall[0])
    stem = "stem" if wood in ("crimson", "warped") else "log"
    bark = src_mean(f"{wood}_{stem}", fall[1])
    inner = mix(plank, src_mean(f"{wood}_{stem}_top", plank), 0.35)
    return plank, bark, inner


# ---------------------------------------------------------------- board layouts


class Layout:
    """Per pixel: board id, position along the grain (u) and across it (v), board width (w), grain
    direction, and the period of u (so grain functions of u tile)."""

    def __init__(self, bid, u, v, w, horiz, period):
        self.bid, self.u, self.v, self.w, self.horiz, self.period = bid, u, v, w, horiz, period
        self.dirx = np.where(horiz, 1.0, 0.0)
        self.diry = np.where(horiz, 0.0, 1.0)
        self.joints = []  # (x, y, grain-horizontal) of butt joints, for nails


def lay_rows(p, rng, transpose=False):
    sizes = p["sizes"]
    assert sum(sizes) == N
    bid = np.zeros((N, N), int)
    u = XX.copy()
    v = np.zeros((N, N))
    w = np.zeros((N, N))
    joints = []
    y0, nb = 0, 0
    for s in sizes:
        rows = slice(y0, y0 + s)
        nj = p.get("joints", 1)
        cuts = sorted(int(c) for c in rng.choice(np.arange(2, 62), nj, replace=False)) if nj else []
        if len(cuts) > 1 and cuts[1] - cuts[0] < 14:
            cuts[1] = (cuts[0] + 14 + int(rng.integers(0, 20))) % 62 + 1
            cuts.sort()
        seg = np.zeros(N, int)
        for c in cuts:
            seg[c:] += 1
        if cuts:
            seg[seg == len(cuts)] = 0  # the piece across the wrap is one board
        bid[rows] = nb + seg[None, :]
        nb += len(cuts) + 1
        v[rows] = YY[rows] - y0
        w[rows] = s
        joints += [(c, y0 + s / 2, s) for c in cuts]
        y0 += s
    lay = Layout(bid, u, v, w, np.ones((N, N), bool), N)
    lay.joints = [(x, y, True, s) for x, y, s in joints]
    if transpose:
        lay = Layout(bid.T.copy(), u.T.copy(), v.T.copy(), w.T.copy(), np.zeros((N, N), bool), N)
        lay.joints = [(y, x, False, s) for x, y, s in joints]
    return lay


def lay_diag(p, rng):
    s = p["sizes"]
    d = (XX + YY) % N
    stripe = (d // s).astype(int)
    u = (XX - YY) % N
    cuts = rng.integers(0, N, N // s)
    seg = ((u - cuts[stripe]) % N >= N / 2).astype(int)
    bid = stripe * 2 + seg
    lay = Layout(bid, u, (d % s) * 0.71, np.full((N, N), s * 0.71), np.ones((N, N), bool), N)
    lay.dirx = np.full((N, N), 0.71)
    lay.diry = np.full((N, N), -0.71)
    return lay


def lay_blocks(p, rng):
    """Parquet / basket weave: square blocks of parallel strips, alternating direction."""
    b, s = p["block"], p["sizes"]
    bx, by = (XX // b).astype(int), (YY // b).astype(int)
    horiz = (bx + by) % 2 == 0
    lx, ly = XX % b, YY % b
    strip = np.where(horiz, ly // s, lx // s).astype(int)
    nb = N // b
    bid = (by * nb + bx) * (b // s) + strip
    u = np.where(horiz, lx, ly)
    v = np.where(horiz, ly % s, lx % s)
    return Layout(bid, u, v, np.full((N, N), float(s)), horiz, b)


def lay_herring(p, rng):
    c = p["sizes"]
    g = N // c
    cx, cy = (XX // c).astype(int), (YY // c).astype(int)
    d = (cx - cy) % 4
    horiz = d < 2
    ox = np.where(d == 1, cx - 1, cx) % g
    oy = np.where(d == 2, cy - 1, cy) % g
    bid = oy * g + ox
    u = np.where(horiz, (XX - ox * c) % N, (YY - oy * c) % N)
    v = np.where(horiz, YY % c, XX % c)
    return Layout(bid, u, v, np.full((N, N), float(c)), horiz, 2 * c)


def layout(p, rng):
    return {"rows": lambda: lay_rows(p, rng), "cols": lambda: lay_rows(p, rng, True),
            "diag": lambda: lay_diag(p, rng), "parquet": lambda: lay_blocks(p, rng),
            "basket": lambda: lay_blocks(p, rng), "herring": lambda: lay_herring(p, rng)}[p["layout"]]()


def edge_distance(bid, r=4):
    """Distance (pixels, wrapping) from each pixel to the nearest pixel of another board."""
    best = np.full(bid.shape, float(r + 1))
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            if dx == 0 and dy == 0:
                continue
            dd = np.hypot(dx, dy)
            if dd > r + 0.5:
                continue
            other = np.roll(np.roll(bid, -dy, 0), -dx, 1) != bid
            best = np.where(other & (dd < best), dd, best)
    return best


def seams(bid, width):
    right = np.roll(bid, -1, 1) != bid
    down = np.roll(bid, -1, 0) != bid
    m = right | down
    if width >= 2:
        m = m | (np.roll(bid, 1, 1) != bid) | (np.roll(bid, 1, 0) != bid)
    return m


# ---------------------------------------------------------------- grain


def per_board(rng, nb, *shape):
    return rng.random((nb,) + shape)


def warp_u(lay, ph, amps=(1.0, 0.5, 0.25)):
    out = np.zeros((N, N))
    for k, a in enumerate(amps, 1):
        out += a * np.sin(TAU * k * lay.u / lay.period + ph[lay.bid, k - 1] * TAU)
    return out


def lines(phase, sharp):
    """Dark late-wood lines from a phase field: 1 on the line, 0 between."""
    c = 0.5 + 0.5 * np.cos(TAU * phase)
    return c ** sharp


def oriented(rng, along, across):
    """Tileable noise stretched along x (horizontal boards) and along y (vertical boards)."""
    return stretched_noise(rng, along, across), stretched_noise(rng, across, along)


def grain(p, lay, rng):
    """-> (paint offset, height) of the wood figure."""
    nb = lay.bid.max() + 1
    ph = per_board(rng, nb, 4)
    dens = p["dens"] * (0.9 + 0.2 * per_board(rng, nb)[lay.bid])
    sharp = p.get("sharp", 2.0)
    kind = p["grain"]
    v, w = lay.v, lay.w
    if kind == "cathedral":
        vc = (ph[lay.bid, 3] * 1.6 - 0.3) * w
        curv = p.get("curv", 5.0)
        phase = (v - vc) ** 2 / (w * curv) * 1.0 + warp_u(lay, ph) * p.get("warp", 1.6) + v * 0.1
        ln = lines(phase * dens * 3.0, sharp)
    elif kind == "straight":
        phase = v * dens + warp_u(lay, ph, (0.25, 0.12, 0.06)) * 0.5
        ln = lines(phase, sharp)
    elif kind == "wavy":
        amp, k = p.get("wave", (2.5, 2))
        phase = v * dens + amp * dens * np.sin(TAU * k * lay.u / lay.period + ph[lay.bid, 0] * TAU + v * 0.25)
        phase = phase + warp_u(lay, ph, (0.3, 0.2, 0.1)) * 0.4
        ln = lines(phase, sharp)
    elif kind == "fibre":
        strands = rng.random((nb, 80))
        vi = np.clip(np.floor(v + 0.6 * np.sin(TAU * 2 * lay.u / lay.period + ph[lay.bid, 0] * TAU)), 0, 79).astype(int)
        ln = strands[lay.bid, vi] ** 1.5
        ln = ln * 0.8 + 0.2 * lines(v * dens, 2.0)
    elif kind == "mottle":
        blot = fbm(rng, ((16, 0.5), (8, 0.35), (4, 0.15)))
        ln = 0.55 * lines(v * dens + warp_u(lay, ph) * 0.3, 2.0) + 0.75 * (blot - 0.5)
    elif kind == "cloud":
        a, b = oriented(rng, 32, 4)
        c, d = oriented(rng, 16, 2)
        ln = np.where(lay.horiz, a * 0.65 + c * 0.35, b * 0.65 + d * 0.35)
        ln = (ln - 0.5) * 1.6 + 0.5
    else:
        raise ValueError(kind)
    t = -ln * p["amp"]
    h = -ln * p.get("raised", 0.6)
    return t, h


# ---------------------------------------------------------------- marks


def dash(x0, y0, dx, dy, ln, width=0.6):
    """Wrapped distance mask of a short segment from (x0, y0) along (dx, dy)."""
    m = np.zeros((N, N), bool)
    for ox in (-N, 0, N):
        for oy in (-N, 0, N):
            ax, ay = x0 + ox, y0 + oy
            if not (-ln - 2 < ax < N + ln + 2 and -ln - 2 < ay < N + ln + 2):
                continue
            m |= segment_dist(ax, ay, ax + dx * ln, ay + dy * ln) < width
    return m


def torus(cx, cy):
    dx = (XX - cx + N / 2) % N - N / 2
    dy = (YY - cy + N / 2) % N - N / 2
    return dx, dy


def scatter_dashes(lay, rng, count, lo, hi, val, t, h=None, hval=0.0, width=0.6):
    for _ in range(count):
        x, y = rng.random(2) * N
        xi, yi = int(x) % N, int(y) % N
        if lay.v[yi, xi] < 1 or lay.v[yi, xi] > lay.w[yi, xi] - 2:
            continue
        m = dash(x, y, lay.dirx[yi, xi], lay.diry[yi, xi], rng.uniform(lo, hi), width) & (lay.bid == lay.bid[yi, xi])
        t[m] += val
        if h is not None:
            h[m] += hval


def knots(p, lay, rng, t, h, chance, size=1.0):
    nb = lay.bid.max() + 1
    for b in range(nb):
        if rng.random() > chance:
            continue
        ys, xs = np.nonzero(lay.bid == b)
        if len(xs) < 40:
            continue
        i = rng.integers(0, len(xs))
        cx, cy = xs[i] + 0.5, ys[i] + 0.5
        if lay.v[ys[i], xs[i]] < 3 or lay.v[ys[i], xs[i]] > lay.w[ys[i], xs[i]] - 4:
            continue
        dx, dy = torus(cx, cy)
        a = dx * lay.dirx + dy * lay.diry
        c = -dx * lay.diry + dy * lay.dirx
        r = size * rng.uniform(1.6, 2.6)
        d = np.hypot(a / 1.6, c)
        same = lay.bid == b
        core = (d < r) & same
        ring = (d >= r) & (d < r + 1.6) & same
        swirl = np.exp(-(d / (r * 2.6)) ** 2) * same
        t += -swirl * 0.10
        t[core] -= 0.30
        t[ring] += 0.08
        h[core] -= 1.2


def spots(rng, count, rmin, rmax):
    out = []
    for _ in range(count):
        cx, cy = rng.random(2) * N
        out.append((cx, cy, rng.uniform(rmin, rmax)))
    return out


# ---------------------------------------------------------------- planks


def planks(wood, name=None, base=None):
    p = WOODS[wood]
    name = name or f"{wood}_planks"
    rng = rng_for(name)
    plank_c, _, _ = colours(wood)
    base = plank_c if base is None else hx(base)
    pal = ramp(base, 12, p.get("spread", 0.36), 0.12)
    lay = layout(p, rng)
    nb = lay.bid.max() + 1
    tone = (per_board(rng, nb) - 0.5) * 0.16
    t = 0.5 + tone[lay.bid]
    gt, gh = grain(p, lay, rng)
    t = t + gt
    ed = edge_distance(lay.bid)
    bevel = np.clip((ed - 0.5) / p["bevel"], 0, 1)
    if p.get("soft"):
        bevel = np.sqrt(bevel)
    h = bevel * 2.2 + gh
    m = p["marks"]
    if "knots" in m:
        knots(p, lay, rng, t, h, m["knots"], m.get("knot_size", 1.0))
    if "pores" in m:
        scatter_dashes(lay, rng, m["pores"], 1.5, 3.5, -0.16, t, h, -0.5)
    if "rays" in m:
        scatter_dashes(lay, rng, m["rays"], 2.0, 5.0, 0.13, t, h, 0.2)
    if "flecks" in m:
        scatter_dashes(lay, rng, m["flecks"], 1.0, 3.0, -0.22, t, h, -0.3)
    if "resin" in m:
        scatter_dashes(lay, rng, m["resin"], 5.0, 11.0, 0.18, t, None, width=0.55)
    if "ribbon" in m:
        ph = per_board(rng, nb, 2)
        band = np.sign(np.sin(TAU * lay.v / np.maximum(lay.w / 2, 1) + ph[lay.bid, 0] * TAU))
        flip = np.sin(TAU * 2 * lay.u / lay.period + ph[lay.bid, 1] * TAU)
        t += band * np.clip(flip * 2, -1, 1) * m["ribbon"] * 0.5
    if "ripple" in m:
        ph = per_board(rng, nb)
        t += np.sin(TAU * lay.u / (lay.period / 8) + ph[lay.bid] * TAU + lay.v * 0.35) * m["ripple"] * 0.5
    if "heartstreak" in m:
        ph = per_board(rng, nb, 2)
        c = (0.3 + 0.4 * ph[lay.bid, 0]) * lay.w + 2.5 * np.sin(TAU * lay.u / lay.period + ph[lay.bid, 1] * TAU)
        t -= np.exp(-((lay.v - c) / (lay.w * 0.18)) ** 2) * m["heartstreak"]
    if "veins" in m:
        ph = per_board(rng, nb, 2)
        for k in range(2):
            c = (0.25 + 0.5 * ph[lay.bid, k]) * lay.w + 2.0 * np.sin(TAU * (k + 1) * lay.u / lay.period + ph[lay.bid, 1 - k] * TAU)
            vein = np.abs(lay.v - c) < 0.8
            t[vein] -= m["veins"]
            h[vein] -= 0.8
    if "sheen" in m:
        t += np.clip(light(blur(h, 2)) * 4, -1, 1) * m["sheen"]
    if "cracks" in m:
        for _ in range(m["cracks"]):
            x, y = rng.random(2) * N
            xi, yi = int(x), int(y)
            if lay.v[yi, xi] < 2 or lay.v[yi, xi] > lay.w[yi, xi] - 3:
                continue
            mk = dash(x, y, lay.dirx[yi, xi], lay.diry[yi, xi], rng.uniform(6, 15), 0.55) & (lay.bid == lay.bid[yi, xi])
            h[mk] -= 2.0
            t[mk] -= 0.28
    if "chips" in m:
        for _ in range(m["chips"]):
            x, y = rng.random(2) * N
            dx, dy = torus(x, y)
            mk = (np.hypot(dx, dy) < rng.uniform(1.0, 2.0)) & (ed < 2.5)
            t[mk] += 0.12
            h[mk] -= 0.6
    wear = p.get("wear", 0.0)
    if wear:
        t += np.clip(1.5 - ed, 0, 1) * wear * (value_noise(rng, 4) > 0.45)
    t = t + light(h) * 1.0 - occlusion(h, 1) * 0.10
    if "birdseye" in m:
        for cx, cy, r in spots(rng, m["birdseye"], 0.6, 1.0):
            dx, dy = torus(cx, cy)
            d = np.hypot(dx, dy)
            t[d < r] -= 0.24
            t[(d >= r) & (d < r + 1.0)] += 0.08
    if "dots" in m:
        for cx, cy, r in spots(rng, m["dots"], 0.5, 0.9):
            xi, yi = int(cx) % N, int(cy) % N
            dx, dy = torus(cx, cy)
            a = dx * lay.dirx[yi, xi] + dy * lay.diry[yi, xi]
            c = -dx * lay.diry[yi, xi] + dy * lay.dirx[yi, xi]
            t[np.hypot(a / 1.8, c) < r] -= 0.22
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    gap = seams(lay.bid, p["gap"])
    img[gap, :3] = pal[0] if p["gap"] >= 2 else mix(pal[0], pal[1], 0.5)
    if m.get("nails") or m.get("screws"):
        metal = ramp("#77736d", 6, 0.32, 0.06)
        for x, y, hz, s in lay.joints:
            for side in (-2.5, 2.5):
                for off in ((-s / 4, s / 4) if m.get("screws") else (0,)):
                    nx = (x + side) if hz else (y + off)
                    ny = (y + off) if hz else (x + side)
                    if not hz:
                        nx, ny = ny, nx
                    xi, yi = int(np.floor(nx)) % N, int(np.floor(ny)) % N
                    img[yi, xi, :3] = metal[3]
                    img[(yi + 1) % N, xi, :3] = metal[1]
                    img[yi, (xi + 1) % N, :3] = metal[2]
                    img[(yi + 1) % N, (xi + 1) % N, :3] = metal[0]
                    if m.get("screws"):
                        img[yi, xi, :3] = metal[4]
    if "spores" in m:
        col, count = m["spores"]
        sp = ramp(col, 6, 0.3, 0.1, sat=1.2)
        for cx, cy, r in spots(rng, count, 0.5, 0.9):
            dx, dy = torus(cx, cy)
            d = np.hypot(dx, dy)
            mk = (d < r + 0.3) & ~gap
            img[mk, :3] = sp[4]
            img[(d >= r + 0.3) & (d < r + 1.1) & ~gap, :3] = mix(img[(d >= r + 0.3) & (d < r + 1.1) & ~gap, :3], sp[2], 0.4)
    return soften(img, "built")


# ---------------------------------------------------------------- bark


def narrow(cells):
    """Squeeze a Voronoi tile to half width (twice across): tall, narrow bark plates."""
    ids, d1, d2, pts = cells
    sq = lambda a: np.tile(a[:, ::2], (1, 2))
    return sq(ids), sq(d1) * 0.8, sq(d2) * 0.8, pts


def bark(wood, name=None):
    p = WOODS[wood]
    name = name or f"{wood}_log"
    rng = rng_for(name)
    _, bark_c, _ = colours(wood)
    kind = p["bark"]
    pal = ramp(bark_c, 12, 0.42, 0.12)
    if kind in ("furrowed", "jungle"):
        ridges = stretched_noise(rng, 4, 32) * 0.55 + stretched_noise(rng, 2, 16) * 0.3 + stretched_noise(rng, 8, 64) * 0.15
        breaks = (value_noise(rng, 8) > 0.62) & (YY % 11 < 1)
        h = ridges * 4.5 - breaks * 1.5
        t = 0.5 + (ridges - 0.5) * 0.6
    elif kind == "scaly":
        ids, d1, d2, _ = narrow(worley(rng, 60, 6, jitter_shape=0.5))
        edge = d2 - d1
        h = np.clip(edge, 0, 3) * 1.1 + (YY % 6) * 0.0
        tone = rng.random(ids.max() + 1)[ids]
        t = 0.45 + (tone - 0.5) * 0.25 - (edge < 1.0) * 0.18
    elif kind == "twisted":
        sh = (XX + YY * 0.35) % N
        rid = np.sin(TAU * (sh / 8.0) + value_noise(rng, 16) * 3.0)
        rid2 = stretched_noise(rng, 4, 16)
        h = rid * 2.0 + rid2 * 2.0
        t = 0.5 + rid * 0.12 + (rid2 - 0.5) * 0.3
    elif kind == "plated":
        ids, d1, d2, _ = narrow(worley(rng, 16, 12, jitter_shape=0.3))
        edge = d2 - d1
        h = np.clip(edge, 0, 4) * 1.4 + stretched_noise(rng, 4, 16) * 1.2
        tone = rng.random(ids.max() + 1)[ids]
        t = 0.5 + (tone - 0.5) * 0.3 - (edge < 1.5) * 0.3
    elif kind == "lenticel":
        f = fbm(rng, ((32, 0.5), (16, 0.3), (8, 0.2)))
        h = f * 1.5
        t = 0.5 + (f - 0.5) * 0.3
        for _ in range(26):
            x, y = rng.random(2) * N
            mk = dash(x, y, 1, 0, rng.uniform(4, 10), 0.6)
            t[mk] += 0.22
            h[mk] += 0.8
    elif kind == "stringy":
        s = stretched_noise(rng, 2, 32) * 0.6 + stretched_noise(rng, 4, 8) * 0.4
        wav = np.sin(TAU * (XX / 6 + 0.6 * np.sin(TAU * YY / 32)))
        h = s * 3 + wav * 0.6
        t = 0.5 + (s - 0.5) * 0.55 + wav * 0.05
    elif kind == "smooth_fissure":
        f = fbm(rng, ((32, 0.5), (16, 0.3), (8, 0.2)))
        cut = np.zeros((N, N), bool)
        for _ in range(16):
            x, y = rng.random(2) * N
            cut |= dash(x, y, rng.uniform(-0.15, 0.15), 1, rng.uniform(5, 16), 0.55)
        h = f * 1.2 - cut * 2.0
        t = 0.55 + (f - 0.5) * 0.25 - cut * 0.22
    elif kind == "shaggy":
        strips = rng.random((N // 4 + 1,))
        col = (XX // 4).astype(int)
        jitter = stretched_noise(rng, 4, 16)
        lift = (np.sin(TAU * (YY / N * 2 + strips[col])) > 0.3)
        h = jitter * 2 + lift * 1.8 + strips[col] * 1.5
        t = 0.5 + (strips[col] - 0.5) * 0.3 + (jitter - 0.5) * 0.3
    elif kind == "diamond":
        a = np.sin(TAU * (XX + YY * 0.5) / 16 + value_noise(rng, 16) * 2)
        b = np.sin(TAU * (XX - YY * 0.5) / 16 + value_noise(rng, 16) * 2)
        rid = np.minimum(a, b)
        h = rid * 2.5
        t = 0.5 + rid * 0.18 + (stretched_noise(rng, 4, 8) - 0.5) * 0.2
    elif kind == "ringed":
        rings = YY % 8
        bump = np.cos(TAU * rings / 8)
        f = stretched_noise(rng, 2, 8)
        h = bump * 1.5 + f * 2
        t = 0.5 + bump * 0.08 + (f - 0.5) * 0.4 - (rings < 1) * 0.2
    elif kind in ("palm", "palm_rings"):
        step = 6 if kind == "palm" else 4
        off = np.floor(value_noise(rng, 16) * 2)
        rings = (YY + off) % step
        fib = stretched_noise(rng, 2, 8)
        h = (rings / step) * 2.5 + fib * 1.0
        t = 0.5 + (fib - 0.5) * 0.35 + (rings / step - 0.5) * 0.2 - (rings < 1) * 0.25
        if kind == "palm_rings":
            dia = np.sin(TAU * (XX + YY) / 8) * np.sin(TAU * (XX - YY) / 8)
            t += dia * 0.08
    elif kind == "flaky":
        bx = ((XX + (YY // 5) * 3) // 5).astype(int)
        by = (YY // 5).astype(int)
        cell = (by * 31 + bx) % 997
        tone = rng.random(1000)[cell]
        lx = (XX + (YY // 5) * 3) % 5
        ly = YY % 5
        h = np.minimum(np.minimum(lx, 4 - lx), np.minimum(ly, 4 - ly)) * 0.9 + tone
        t = 0.5 + (tone - 0.5) * 0.35 - ((lx == 0) | (ly == 0)) * 0.22
    elif kind == "patchy":
        ids, d1, d2, _ = worley(rng, 14, 12, jitter_shape=0.6)
        tone = rng.random(ids.max() + 1)[ids]
        h = np.clip(d2 - d1, 0, 2) * 0.8
        t = 0.45 + (tone - 0.5) * 0.45
    elif kind == "smooth_streak":
        s = stretched_noise(rng, 4, 32) * 0.6 + stretched_noise(rng, 8, 64) * 0.4
        h = s * 1.5
        t = 0.55 + (s - 0.5) * 0.4
        for _ in range(8):
            x, y = rng.random(2) * N
            mk = dash(x, y, 0, 1, rng.uniform(3, 7), 0.6)
            t[mk] -= 0.25
    elif kind == "birch":
        return None
    else:
        raise ValueError(kind)
    t = t + light(h) * 1.0 - occlusion(h, 1) * 0.12
    img = rgba(quantize(np.clip(t, 0, 1), pal, 0.3, rng))
    if kind == "jungle":
        from blocks import add_moss
        add_moss(img, rng, None, 1.0, 0.25)
    if kind == "patchy":
        fruit = ramp("#3a1a46", 6, 0.3, 0.1, sat=1.1)
        for cx, cy, r in spots(rng, 9, 1.3, 2.0):
            dx, dy = torus(cx, cy)
            d = np.hypot(dx, dy)
            dome = np.clip(1 - d / (r + 0.4), 0, 1)
            lt = np.clip(0.45 + light(dome * 3) * 1.6, 0, 1)
            img[d < r, :3] = quantize(lt, fruit)[d < r]
    return soften(img, "rough")


# ---------------------------------------------------------------- end grain


def end_grain(wood, name=None):
    p = WOODS[wood]
    q = p["top"]
    name = name or f"{wood}_log_top"
    rng = rng_for(name)
    _, bark_c, inner_c = colours(wood)
    inner = ramp(inner_c, 10, 0.30, 0.12)
    ecc = q.get("ecc", 0.0)
    a = rng.uniform(0, TAU)
    cx, cy = 31.5 + np.cos(a) * ecc, 31.5 + np.sin(a) * ecc
    euc = np.hypot(XX - cx, YY - cy)
    cheb = np.maximum(np.abs(XX - 31.5), np.abs(YY - 31.5))
    rim_w = 4
    if q.get("palm"):
        f = fbm(rng, ((16, 0.5), (8, 0.3), (4, 0.2)))
        t = 0.55 + (f - 0.5) * 0.3 - np.clip(1 - euc / 30, 0, 1) * 0.05
        for sx, sy, r in spots(rng, 160, 0.5, 0.9):
            d = np.hypot(XX - sx, YY - sy)
            t[d < r] -= 0.28
        h = f
    else:
        rnd = q.get("round", 0.6)
        r = rnd * euc + (1 - rnd) * np.maximum(np.abs(XX - cx), np.abs(YY - cy))
        r = r + value_noise(rng, 16) * 2.5 + value_noise(rng, 4) * 0.6
        rings = lines(r / q["rings"], 3.0)
        t = 0.55 - rings * 0.22 + (fbm(rng, ((8, 0.6), (4, 0.4))) - 0.5) * 0.08
        t -= np.clip(1 - euc / 3, 0, 1) * 0.25
        heart = q.get("heart", 0.0)
        if heart:
            t -= np.clip((14 - euc) / 4, 0, 1) * heart
        h = -rings * 0.8
        for _ in range(q.get("cracks", 0)):
            ang = rng.uniform(0, TAU)
            r0, r1 = rng.uniform(2, 8), rng.uniform(14, 26)
            d = segment_dist(cx + np.cos(ang) * r0, cy + np.sin(ang) * r0, cx + np.cos(ang) * r1, cy + np.sin(ang) * r1)
            mk = d < 0.6
            t[mk] -= 0.30
            h[mk] -= 2
    t = t + light(h) * 0.8
    img = rgba(quantize(np.clip(t, 0, 1), inner, 0.3, rng))
    bk = ramp(bark_c, 8, 0.36, 0.1)
    rim = cheb >= 32 - rim_w
    bt = 0.45 + (stretched_noise(rng, 2, 2) - 0.5) * 0.5 + np.where(cheb >= 30.5, -0.15, 0.05)
    img[rim, :3] = quantize(np.clip(bt, 0, 1), bk)[rim]
    ring_in = (cheb >= 31 - rim_w) & (cheb < 32 - rim_w)
    img[ring_in, :3] *= 0.78
    return soften(img, "built")


# ---------------------------------------------------------------- doors and trapdoors


def door_boards(wood, name, horiz):
    """The wood's own planks, laid along the door (vertical) or across a trapdoor."""
    p = dict(WOODS[wood])
    if p["layout"] in ("rows", "cols"):
        w = p["sizes"]
        n = max(3, min(6, len(w)))
        base = [N // n] * n
        base[-1] += N - sum(base)
        p["sizes"] = base
        p["joints"] = 0
        p["layout"] = "rows" if horiz else "cols"
    p = dict(p, marks={k: v for k, v in p["marks"].items() if k not in ("nails", "screws")})
    return planks_with(wood, p, name)


def planks_with(wood, p, name):
    saved = WOODS[wood]
    WOODS[wood] = p
    try:
        return planks(wood, name)
    finally:
        WOODS[wood] = saved


def frame(img, pal, width, mask):
    """A raised frame `width` pixels wide around the opaque area, lit from the top left."""
    from style import dist_inside
    d = dist_inside(mask, width + 2)
    fr = mask & (d <= width)
    img[fr, :3] = img[fr, :3] * 0.92
    # top/left lit, bottom/right shaded edges of the frame and of its inner lip.
    up = mask & ~np.roll(mask, 1, 0)
    lf = mask & ~np.roll(mask, 1, 1)
    dn = mask & ~np.roll(mask, -1, 0)
    rt = mask & ~np.roll(mask, -1, 1)
    img[up | lf, :3] = pal[9]
    img[dn | rt, :3] = pal[1]
    lip = mask & (d == width + 1)
    lip_up = lip & (np.roll(d, 1, 0) <= width)
    lip_lf = lip & (np.roll(d, 1, 1) <= width)
    img[lip & ~(lip_up | lip_lf), :3] = pal[7]
    img[lip_up | lip_lf, :3] = pal[2]


def window(img, holes, pal):
    """Cut holes, shade their sills."""
    img[holes, 3] = 0
    img[holes, :3] = 0
    below = np.roll(holes, 1, 0) & ~holes
    right = np.roll(holes, 1, 1) & ~holes
    above = np.roll(holes, -1, 0) & ~holes
    left = np.roll(holes, -1, 1) & ~holes
    img[above | left, :3] = pal[1]
    img[below | right, :3] = pal[8]


def rect(x0, y0, x1, y1):
    return (XX >= x0) & (XX < x1) & (YY >= y0) & (YY < y1)


def door_holes(style, upper):
    z = np.zeros((N, N), bool)
    if not upper:
        return z
    if style == "two_panes":
        return rect(8, 8, 28, 28) | rect(36, 8, 56, 28)
    if style == "grid_panes":
        m = z.copy()
        for gx in range(3):
            for gy in range(2):
                m |= rect(8 + gx * 16, 8 + gy * 13, 20 + gx * 16, 18 + gy * 13)
        return m
    if style == "solid_cross":
        return z
    if style == "round_panes":
        return (np.hypot(XX - 19.5, YY - 19.5) < 9) | (np.hypot(XX - 44.5, YY - 19.5) < 9)
    if style == "slot":
        return rect(10, 14, 54, 22)
    if style == "small_panes":
        return rect(12, 8, 26, 20) | rect(38, 8, 52, 20)
    if style == "lattice":
        m = rect(8, 8, 56, 34)
        bars = ((XX - 8) % 8 < 2) | ((YY - 8) % 9 < 2)
        return m & ~bars
    if style == "vertical_slats":
        return rect(8, 8, 56, 30) & ((XX - 8) % 8 >= 4)
    if style == "one_pane":
        return rect(8, 8, 56, 32) & ~(rect(30, 8, 34, 32) | rect(8, 18, 56, 22))
    if style == "vein":
        a = np.hypot((XX - 22) / 1.0, (YY - 18) / 1.4) < 8
        b = np.hypot((XX - 42) / 1.2, (YY - 22) / 1.0) < 7
        return a | b
    if style == "arch":
        return (rect(14, 20, 50, 36) | (np.hypot(XX - 31.5, YY - 20) < 18) & (YY < 20) & (YY >= 6))
    if style == "diamond":
        return (np.abs(XX - 31.5) + np.abs(YY - 21) < 13)
    return rect(8, 8, 28, 28) | rect(36, 8, 56, 28)


def door(wood, half):
    p = WOODS[wood]
    name = f"{wood}_door_{half}"
    img = door_boards(wood, name, horiz=False)
    plank_c, _, _ = colours(wood)
    pal = ramp(plank_c, 12, 0.36, 0.12)
    full = np.ones((N, N), bool)
    style = p["door"]
    upper = half == "upper"
    # Rails: a cross-brace door (spruce) and the lower halves get a Z brace or rails.
    if style == "solid_cross" and upper:
        rail = rect(4, 28, 60, 36) | rect(28, 4, 36, 60)
        img[rail, :3] = img[rail, :3] * 1.08
        img[edge_mask(rail), :3] = pal[2]
    if not upper:
        if style in ("solid_cross", "vertical_slats", "slot"):
            dz = np.abs((YY - 8) - (XX - 6) * 48 / 52) < 3.2
            dz &= rect(6, 8, 58, 56)
            img[dz, :3] = img[dz, :3] * 1.10
            img[edge_mask(dz), :3] = pal[2]
        elif style in ("lattice", "grid_panes", "small_panes"):
            for y0 in (10, 34):
                pn = rect(10, y0, 54, y0 + 20)
                img[pn & ~rect(12, y0 + 2, 52, y0 + 18), :3] = pal[2]
                img[rect(12, y0 + 2, 52, y0 + 18), :3] *= 1.04
        elif style == "vein":
            pass
        else:
            pn = rect(10, 10, 54, 54)
            img[pn & ~rect(12, 12, 52, 52), :3] = pal[2]
    holes = door_holes(style, upper)
    frame(img, pal, 3, full)
    window(img, holes, pal)
    # Handle on the lower half (upper half for nothing), on the right side.
    if not upper:
        metal = ramp("#5c5853", 6, 0.3, 0.06)
        hx0, hy0 = 50, 4
        img[rect(hx0, hy0, hx0 + 4, hy0 + 8), :3] = metal[2]
        img[rect(hx0, hy0, hx0 + 1, hy0 + 8), :3] = metal[4]
        img[rect(hx0 + 3, hy0, hx0 + 4, hy0 + 8), :3] = metal[0]
        img[rect(hx0, hy0 + 7, hx0 + 4, hy0 + 8), :3] = metal[0]
    return img


def edge_mask(m):
    return m & ~(np.roll(m, 1, 0) & np.roll(m, -1, 0) & np.roll(m, 1, 1) & np.roll(m, -1, 1))


def trapdoor(wood):
    p = WOODS[wood]
    name = f"{wood}_trapdoor"
    img = door_boards(wood, name, horiz=True)
    plank_c, _, _ = colours(wood)
    pal = ramp(plank_c, 12, 0.36, 0.12)
    style = p["trap"]
    holes = np.zeros((N, N), bool)
    if style == "four_slots":
        for x0 in (12, 44):
            for y0 in (8, 36):
                holes |= rect(x0, y0, x0 + 8, y0 + 20)
    elif style == "grid":
        for gx in range(3):
            for gy in range(3):
                holes |= rect(10 + gx * 16, 10 + gy * 16, 22 + gx * 16, 22 + gy * 16)
    elif style == "solid_brace":
        dz = np.abs(YY - XX) < 3.5
        img[dz, :3] = img[dz, :3] * 1.1
        img[edge_mask(dz), :3] = pal[2]
    elif style == "porthole":
        holes = np.hypot(XX - 31.5, YY - 31.5) < 13
    elif style == "slats":
        holes = rect(8, 8, 56, 56) & ((YY - 8) % 8 >= 5)
    elif style == "slats_v":
        holes = rect(8, 8, 56, 56) & ((XX - 8) % 8 >= 5)
    elif style == "iron_bands":
        metal = ramp("#4a4744", 6, 0.3, 0.06)
        for y0 in (12, 46):
            band = rect(0, y0, N, y0 + 5)
            img[band, :3] = metal[2]
            img[rect(0, y0, N, y0 + 1), :3] = metal[4]
            img[rect(0, y0 + 4, N, y0 + 5), :3] = metal[0]
            for bx in (6, 22, 40, 56):
                img[rect(bx, y0 + 1, bx + 2, y0 + 3), :3] = metal[5]
        holes = rect(14, 24, 28, 40) | rect(36, 24, 50, 40)
    elif style == "lattice":
        holes = rect(8, 8, 56, 56) & ~(((XX - 8) % 8 < 2) | ((YY - 8) % 8 < 2))
    elif style == "vein":
        holes = (np.hypot(XX - 22, YY - 24) < 7) | (np.hypot(XX - 42, YY - 40) < 8)
    elif style == "diamond":
        holes = (np.abs(XX - 31.5) + np.abs(YY - 31.5) < 14) & ~(np.abs(XX - 31.5) + np.abs(YY - 31.5) < 9)
    frame(img, pal, 3, np.ones((N, N), bool))
    window(img, holes, pal)
    return img


# ---------------------------------------------------------------- registry


def register(reg, exists):
    for w, p in WOODS.items():
        reg[f"{w}_planks"] = lambda w=w: planks(w)
        stem = "stem" if w in ("crimson", "warped") else "log"
        if p["bark"] and p["bark"] != "birch":
            reg[f"{w}_{stem}"] = lambda w=w: bark(w)
        if p["top"]:
            reg[f"{w}_{stem}_top"] = lambda w=w: end_grain(w)
        for half in ("upper", "lower"):
            if exists(f"{w}_door_{half}"):
                reg[f"{w}_door_{half}"] = lambda w=w, h=half: door(w, h)
        if exists(f"{w}_trapdoor"):
            reg[f"{w}_trapdoor"] = lambda w=w: trapdoor(w)
