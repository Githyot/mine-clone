"""Swords and armour icons with a look of their own for every material (HP, 2026-10-28).

The old drawings in items.py were one sword and one armour set recoloured per material, so a
row of swords read as the same picture in different paint. Here every material has its own
design: the blade's profile (straight, leaf, curved, hooked, wavy, crystalline), the guard
(crossbar, swept hilt, disc, shards, horns), the grip's wrap and the pommel; and every armour
set has its own build (stitched leather, mail, riveted plate, Roman bands, Greek bronze,
fluted gothic plate, scales, laced samurai lames, volcanic rock...).

The look is less "cartoon" than the rest of the item art: a thin one-pixel outline, real
cross-sections (a blade is two lit planes meeting at a ridge, a plate is a dome) lit by the
same top-left key light, a specular term for metals, and material texture on top (wood grain,
stone flakes, brushed steel, gem facets, patina, glowing cracks).

Coordinates: a sword is laid out along its own axis `u` (pommel 0 -> tip ~74) with `v` across
it, and drawn diagonally from the bottom-left to the top-right like every other tool.
"""

import numpy as np

from style import LIGHT, N, XX, YY, canvas, from_oklab, hexrgb, quantize, ramp, rng_for, to_oklab, value_noise, dist_inside

PX, PY = XX + 0.5, YY + 0.5


# ---------------------------------------------------------------- materials


def _pal(base, spread, sat=1.1, warm=0.14, lift=0.0, n=16):
    return ramp(base, n, spread, warm, sat=sat, lift=lift)


# kind: how the surface behaves. spec/power: highlight strength and tightness.
MAT = {
    "wooden": dict(base="#9b7443", spread=0.5, sat=1.05, kind="wood", spec=0.08, power=6),
    "stone": dict(base="#85837f", spread=0.5, sat=1.0, kind="stone", spec=0.06, power=6),
    "iron": dict(base="#b9bcc0", spread=0.78, sat=1.0, kind="steel", spec=0.35, power=18),
    "golden": dict(base="#e0aa2a", spread=0.78, sat=1.25, warm=0.22, kind="gold", spec=0.45, power=14),
    "diamond": dict(base="#4fd6cf", spread=0.72, sat=1.15, kind="gem", spec=0.5, power=24),
    "netherite": dict(base="#4b4248", spread=0.42, sat=1.2, kind="dark", spec=0.25, power=10),
    "copper": dict(base="#c9733a", spread=0.68, sat=1.15, kind="copper", spec=0.32, power=14),
    "tin": dict(base="#a6aeb6", spread=0.55, sat=1.0, kind="dull", spec=0.14, power=8),
    "bronze": dict(base="#b08038", spread=0.66, sat=1.1, kind="bronze", spec=0.3, power=12),
    "lead": dict(base="#646c84", spread=0.48, sat=1.0, kind="dull", spec=0.1, power=6),
    "silver": dict(base="#d4d8de", spread=0.75, sat=1.0, kind="polish", spec=0.55, power=26),
    "tungsten": dict(base="#7a957c", spread=0.62, sat=1.05, kind="steel", spec=0.28, power=14),
    "platinum": dict(base="#b7d6e0", spread=0.7, sat=1.0, kind="polish", spec=0.5, power=22),
    "magmarite": dict(base="#3c302c", spread=0.4, sat=1.0, kind="lava", spec=0.12, power=8),
    "leather": dict(base="#8a5832", spread=0.5, sat=1.05, kind="leather", spec=0.05, power=4),
    "chainmail": dict(base="#9c9ea6", spread=0.7, sat=1.0, kind="steel", spec=0.3, power=12),
}
MAT["gold"] = MAT["golden"]


def mpal(m):
    d = MAT[m]
    return _pal(d["base"], d["spread"], d["sat"], d.get("warm", 0.14))


def outline_of(pal):
    lab = to_oklab(pal[0])
    lab[0] *= 0.55
    lab[2] -= 0.015
    return from_oklab(lab)


LEATHER_GRIP = _pal("#5a3a24", 0.42, 1.0)
DARK_GRIP = _pal("#2c2a30", 0.32, 1.0)
WOOD = _pal("#8a6438", 0.45, 1.0)
CORD = _pal("#b49a6a", 0.42, 1.0)
RED_GEM = _pal("#c4202e", 0.7, 1.25)
BLUE_CORD = _pal("#2a3a78", 0.42, 1.1)
LAVA = _pal("#ff8a1a", 0.5, 1.3, warm=0.3)
VELVET = _pal("#7a1e2a", 0.4, 1.1)


# ---------------------------------------------------------------- painting


class Art:
    """One 64x64 sprite: parts are painted in order, each with a one-pixel outline."""

    def __init__(self, name):
        self.img = canvas()
        self.rng = rng_for("arms:" + name)

    def paint(self, mask, height, pal, base=0.5, diff=1.4, spec=0.0, power=10, albedo=None, glow=None,
              outline=True, dither=0.25, out_col=None):
        if not mask.any():
            return
        if outline:
            ring = _dilate(mask) & ~mask
            self.img[ring, :3] = outline_of(pal) if out_col is None else out_col
            self.img[ring, 3] = 255
        h = np.where(mask, height, 0.0)
        gx = np.gradient(h, axis=1)
        gy = np.gradient(h, axis=0)
        gx = np.clip(gx, -1.6, 1.6)
        gy = np.clip(gy, -1.6, 1.6)
        ln = np.sqrt(gx * gx + gy * gy + 1)
        nx, ny, nz = -gx / ln, -gy / ln, 1 / ln
        d = nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2]
        hx, hy, hz = LIGHT[0], LIGHT[1], LIGHT[2] + 1
        hl = np.sqrt(hx * hx + hy * hy + hz * hz)
        s = np.clip((nx * hx + ny * hy + nz * hz) / hl, 0, 1) ** power
        t = base + (d - LIGHT[2]) * diff + s * spec
        if albedo is not None:
            t = t + albedo
        col = quantize(np.clip(t, 0, 1), pal, dither, self.rng)
        if glow is not None:
            g = glow > 0
            col = np.where(g[..., None], quantize(np.clip(glow, 0, 1), LAVA), col)
        self.img[mask, :3] = col[mask]
        self.img[mask, 3] = 255

    def dots(self, pts, r, pal, base=0.55):
        """Rivets, studs, gems: little lit domes."""
        for (x, y) in pts:
            m = (PX - x) ** 2 + (PY - y) ** 2 <= r * r
            hh = np.sqrt(np.clip(r * r - (PX - x) ** 2 - (PY - y) ** 2, 0, None))
            self.paint(m, hh * 1.2, pal, base=base, spec=0.6, power=10, outline=r > 1.6)

    def tint(self, mask, pal, idx):
        m = mask & (self.img[..., 3] > 0)
        self.img[m, :3] = pal[idx]

    def done(self):
        return self.img


def _dilate(m):
    p = np.pad(m, 1, constant_values=False)
    return m | p[:-2, 1:-1] | p[2:, 1:-1] | p[1:-1, :-2] | p[1:-1, 2:]


def poly(pts):
    inside = np.zeros((N, N), bool)
    n = len(pts)
    for i in range(n):
        x0, y0 = pts[i]
        x1, y1 = pts[(i + 1) % n]
        cond = (y0 > PY) != (y1 > PY)
        xint = (x1 - x0) * (PY - y0) / ((y1 - y0) + 1e-12) + x0
        inside ^= cond & (PX < xint)
    return inside


def seg(x0, y0, x1, y1):
    vx, vy = x1 - x0, y1 - y0
    L2 = vx * vx + vy * vy + 1e-9
    t = np.clip(((PX - x0) * vx + (PY - y0) * vy) / L2, 0, 1)
    return np.hypot(PX - (x0 + t * vx), PY - (y0 + t * vy))


def polyline(pts, w):
    m = np.zeros((N, N), bool)
    for (a, b) in zip(pts, pts[1:]):
        m |= seg(a[0], a[1], b[0], b[1]) <= w / 2
    return m


def dome(mask, r=3.0, k=1.0):
    d = dist_inside(mask, int(np.ceil(r)) + 1).astype(float)
    x = np.clip(d / r, 0, 1)
    return (1 - (1 - x) ** 2) * r * k


def streaks(rng, along, across, scale=1.5):
    """Noise that runs along `along` (brushed metal, wood grain): sampled from `across`."""
    line = rng.random(200)
    k = np.interp((across * scale) % 199, np.arange(200), line)
    k2 = np.interp((along * 0.15 + across * 0.3) % 199, np.arange(200), rng.random(200))
    return k * 0.75 + k2 * 0.25 - 0.5


# ---------------------------------------------------------------- the sword frame


class Frame:
    """A sword's own axes: u from the pommel to the tip, v across (positive = bottom-right)."""

    def __init__(self, x0=4.6, y0=59.4, ang=-45.0):
        a = np.deg2rad(ang)
        self.o = np.array([x0, y0])
        self.d = np.array([np.cos(a), np.sin(a)])
        self.n = np.array([-self.d[1], self.d[0]])
        rx, ry = PX - x0, PY - y0
        self.U = rx * self.d[0] + ry * self.d[1]
        self.V = rx * self.n[0] + ry * self.n[1]

    def xy(self, u, v):
        p = self.o + self.d * u + self.n * v
        return (p[0], p[1])

    def poly(self, uv):
        return poly([self.xy(u, v) for (u, v) in uv])

    def box(self, u0, u1, v0, v1):
        return (self.U >= u0) & (self.U <= u1) & (self.V >= v0) & (self.V <= v1)

    def disc(self, u, v, r, rv=None):
        rv = r if rv is None else rv
        return ((self.U - u) / r) ** 2 + ((self.V - v) / rv) ** 2 <= 1


def blade(f, us, left, right, center=None, ridge=0.0, hmax=3.0, edge=1.4):
    """Mask and height of a blade. `us` are stations along u; `left`/`right` the widths on the
    -v / +v side; `center` an offset of the centre line (curved blades); `ridge` moves the ridge
    toward the -v side (-1, a spine) or the +v side (+1)."""
    us = np.asarray(us, float)
    c = np.interp(f.U, us, center) if center is not None else 0.0
    L = np.interp(f.U, us, left)
    R = np.interp(f.U, us, right)
    m = (f.U >= us[0]) & (f.U <= us[-1]) & (f.V >= c - L) & (f.V <= c + R) & ((L + R) > 0.35)
    rv = c + np.where(ridge < 0, -L * -ridge, R * ridge) * 0.85
    s = f.V - rv
    side = np.where(s < 0, (rv - (c - L)), ((c + R) - rv))
    x = np.clip(np.abs(s) / np.maximum(side, 0.5), 0, 1)
    h = hmax * (1 - x)
    # The honed edge: a steeper bevel along the last pixel or so.
    h = h - np.clip(np.abs(s) - (side - edge), 0, None) * 0.9
    return m, h, s, side


def _wrap_cross(f, u0, u1, w, step=3.2):
    """Criss-cross leather wrap: diamond ridges."""
    a = np.abs(((f.U + f.V * 1.1) / step) % 1 - 0.5)
    b = np.abs(((f.U - f.V * 1.1) / step) % 1 - 0.5)
    return np.minimum(a, b)


def grip(art, f, u0, u1, w, pal, style="cross", base=0.45):
    m = f.box(u0, u1, -w, w)
    cyl = np.sqrt(np.clip(1 - (f.V / w) ** 2, 0, 1)) * w * 0.9
    if style == "cross":
        h = cyl + _wrap_cross(f, u0, u1, w) * 1.6
    elif style == "bands":
        h = cyl + np.abs(np.sin(f.U * 1.15)) * 0.9
    elif style == "spiral":
        h = cyl + np.abs(np.sin((f.U + f.V * 0.8) * 1.4)) * 0.9
    elif style == "ito":
        k = _wrap_cross(f, u0, u1, w, step=4.2)
        h = cyl + k * 2.0
    elif style == "spindle":
        mid = (u0 + u1) / 2
        ww = w * (1 + 0.35 * np.cos((f.U - mid) / (u1 - u0) * np.pi))
        m = (f.U >= u0) & (f.U <= u1) & (np.abs(f.V) <= ww)
        h = np.sqrt(np.clip(1 - (f.V / ww) ** 2, 0, 1)) * ww * 0.9
    else:
        h = cyl + streaks(art.rng, f.U, f.V, 2.0) * 0.6
    art.paint(m, h, pal, base=base, diff=1.2, spec=0.05)
    return m


# ---------------------------------------------------------------- surface textures


def tex_for(art, m, f=None, kind="steel"):
    """Albedo (lighter/darker paint) of one material."""
    rng = art.rng
    U = f.U if f is not None else XX
    V = f.V if f is not None else YY
    if kind in ("steel", "polish", "dull", "chainmail"):
        return streaks(rng, U, V, 1.3) * (0.08 if kind != "dull" else 0.05)
    if kind == "wood":
        g = np.sin(V * 1.7 + value_noise(rng, 8) * 4.0) * 0.07
        return g + (value_noise(rng, 4) - 0.5) * 0.06
    if kind == "stone":
        return (value_noise(rng, 4) - 0.5) * 0.14 + (value_noise(rng, 2) - 0.5) * 0.06
    if kind == "gold":
        return streaks(rng, U, V, 1.0) * 0.05
    if kind == "dark":
        return (value_noise(rng, 2) - 0.5) * 0.12 + streaks(rng, U, V, 0.8) * 0.05
    if kind in ("copper", "bronze"):
        return (value_noise(rng, 4) - 0.5) * 0.08
    if kind == "leather":
        return (value_noise(rng, 2) - 0.5) * 0.1 + (value_noise(rng, 8) - 0.5) * 0.06
    if kind == "lava":
        return (value_noise(rng, 2) - 0.5) * 0.14
    if kind == "gem":
        return 0.0
    return 0.0


def patina(art, m, amount=0.42, color="#4fa48c"):
    """Green-blue oxidation in blotches (copper, bronze)."""
    n = value_noise(art.rng, 8) * 0.6 + value_noise(art.rng, 4) * 0.4
    p = m & (n > 1 - amount) & (art.img[..., 3] > 0)
    pal = _pal(color, 0.35, 1.0)
    lum = to_oklab(art.img[..., :3])[..., 0]
    idx = np.clip(((lum - 0.35) / 0.5) * (len(pal) - 1), 1, len(pal) - 2).astype(int)
    art.img[p, :3] = pal[idx[p]]


def cracks(art, m, rng, count=5, width=0.9, f=None):
    """Glowing veins (magmarite): random walks, drawn as an emissive field."""
    g = np.zeros((N, N))
    pts = np.argwhere(m)
    if len(pts) == 0:
        return g
    for _ in range(count):
        y, x = pts[rng.integers(len(pts))] + 0.5
        ang = rng.uniform(0, 2 * np.pi)
        path = [(x, y)]
        for _ in range(rng.integers(4, 8)):
            ang += rng.uniform(-0.9, 0.9)
            x, y = x + np.cos(ang) * 3.2, y + np.sin(ang) * 3.2
            path.append((x, y))
        for a, b in zip(path, path[1:]):
            dd = seg(a[0], a[1], b[0], b[1])
            g = np.maximum(g, np.clip(1 - dd / width, 0, 1))
    return np.where(m & (g > 0.25), 0.45 + g * 0.55, 0)


# ---------------------------------------------------------------- swords


def _sword_paint(art, f, m, h, mat, base=0.52, extra=None, glow=None, facet=None):
    d = MAT[mat]
    pal = mpal(mat)
    alb = tex_for(art, m, f, d["kind"])
    if extra is not None:
        alb = alb + extra
    if facet is not None:
        alb = alb + facet
    art.paint(m, h, pal, base=base, diff=1.5 if d["kind"] != "gem" else 1.9, spec=d["spec"], power=d["power"],
              albedo=alb, glow=glow)


def sw_wooden(art, f):
    pal = mpal("wooden")
    # Rounded knob, a cord-wrapped grip, a lashed crossbar and a blunt practice blade.
    art.paint(f.disc(3.6, 0, 3.6), dome(f.disc(3.6, 0, 3.6), 3), pal, base=0.5, albedo=tex_for(art, None, f, "wood"))
    grip(art, f, 6, 19, 2.5, CORD, "bands", base=0.55)
    us = [22, 64, 67, 69.5, 71]
    m, h, s, side = blade(f, us, [4.2, 4.2, 3.6, 2.2, 0.3], [4.2, 4.2, 3.6, 2.2, 0.3], hmax=1.8, edge=0.0)
    h = h * 0.6 + np.sqrt(np.clip(1 - (s / np.maximum(side, 0.5)) ** 2, 0, 1)) * 1.2
    knots = (value_noise(art.rng, 16) > 0.86) * -0.12
    art.paint(m, h, pal, base=0.54, diff=1.3, spec=0.05, albedo=tex_for(art, m, f, "wood") + knots)
    g = f.poly([(18.5, -10.5), (23, -10.5), (23.6, -9), (23.6, 9), (23, 10.5), (18.5, 10.5), (17.9, 9), (17.9, -9)])
    art.paint(g, dome(g, 2) + streaks(art.rng, f.V, f.U, 1.6) * 0.3, WOOD, base=0.45, diff=1.3)
    # Cord lashing the crossbar: an X over the joint.
    x = (np.abs(f.U - 20.8 - f.V * 0.55) < 0.8) | (np.abs(f.U - 20.8 + f.V * 0.55) < 0.8)
    art.paint(x & (np.abs(f.V) < 3.2), np.zeros((N, N)) + 1, CORD, base=0.62, outline=False)


def sw_stone(art, f):
    rng = art.rng
    pal = mpal("stone")
    lump = f.poly([(0.6, -2.5), (2.5, -4.6), (6.0, -3.8), (7.4, 0.5), (5.8, 4.2), (2.0, 4.0), (0.2, 1.4)])
    art.paint(lump, dome(lump, 2.5), pal, base=0.45, albedo=tex_for(art, lump, f, "stone"))
    grip(art, f, 6.5, 18.5, 2.6, LEATHER_GRIP, "spiral", base=0.5)
    # A short, broad, knapped blade: the edges are a row of flake scars.
    us = np.linspace(21, 66, 16)
    w = np.interp(us, [21, 30, 52, 66], [5.0, 6.4, 5.2, 0.2])
    chips_l = w - (rng.random(16) * 1.3) * (us < 63)
    chips_r = w - (rng.random(16) * 1.3) * (us < 63)
    m, h, s, side = blade(f, us, chips_l, chips_r, hmax=3.4, edge=0.0)
    # Flake scars: Voronoi cells, each a slightly tilted plane.
    pts = rng.uniform([18, -7], [66, 7], (26, 2))
    du = f.U[..., None] - pts[:, 0]
    dv = f.V[..., None] - pts[:, 1]
    dd = du * du + dv * dv
    cid = np.argmin(dd, -1)
    tilt = rng.uniform(-0.25, 0.25, (26, 2))
    scar = tilt[cid, 0] * du[np.arange(N)[:, None], np.arange(N)[None, :], cid] + tilt[cid, 1] * dv[np.arange(N)[:, None], np.arange(N)[None, :], cid]
    srt = np.sort(dd, -1)
    seam = (np.sqrt(srt[..., 1]) - np.sqrt(srt[..., 0])) < 0.7
    _sword_paint(art, f, m, h + scar * 0.6, "stone", base=0.5, extra=seam * -0.1)
    # Chunky guard: a rough stone block.
    g = f.poly([(17.6, -7.6), (21.6, -8.4), (22.4, -3), (22.2, 4), (21.4, 8.2), (17.8, 7.4), (17.2, 2), (17.4, -3)])
    art.paint(g, dome(g, 2.2), pal, base=0.42, albedo=tex_for(art, g, f, "stone"))


def sw_iron(art, f):
    pal = mpal("iron")
    # Wheel pommel with a raised boss.
    pm = f.disc(3.8, 0, 3.8)
    art.paint(pm, dome(pm, 2.0) + f.disc(3.8, 0, 1.6) * 1.2, pal, base=0.45, spec=0.3, power=14)
    grip(art, f, 7.5, 19, 2.3, LEATHER_GRIP, "cross", base=0.45)
    us = [22, 60, 68, 73.5]
    m, h, s, side = blade(f, us, [3.5, 3.0, 2.2, 0.2], [3.5, 3.0, 2.2, 0.2], hmax=2.6)
    fuller = (f.U > 24) & (f.U < 54) & (np.abs(f.V) < 0.9)
    h = h - fuller * 1.6
    _sword_paint(art, f, m, h, "iron", base=0.5)
    # Straight cross guard whose quillons dip a little toward the hand and flare at the ends.
    g = f.poly([(19.6, -11.6), (21.0, -12.4), (22.6, -11.0), (22.4, -3), (23.4, 0), (22.4, 3), (22.6, 11.0),
                (21.0, 12.4), (19.6, 11.6), (20.2, 6), (19.8, 0), (20.2, -6)])
    art.paint(g, dome(g, 1.6), pal, base=0.42, spec=0.3, power=14, albedo=tex_for(art, g, f, "steel"))


def sw_golden(art, f):
    pal = mpal("golden")
    # A big round pommel with a ruby set in it.
    pm = f.disc(4.2, 0, 4.2)
    art.paint(pm, dome(pm, 3), pal, base=0.48, spec=0.45, power=12)
    art.dots([f.xy(4.2, 0)], 1.8, RED_GEM, base=0.5)
    grip(art, f, 8, 19, 2.4, VELVET, "spiral", base=0.5)
    gb = f.box(9.5, 10.5, -2.6, 2.6) | f.box(16.5, 17.5, -2.6, 2.6)
    art.paint(gb, np.ones((N, N)), pal, base=0.7, outline=False)
    # A ceremonial broad blade: a narrow ricasso, then it swells, then a long point.
    us = [22, 27, 30, 50, 62, 72.5]
    m, h, s, side = blade(f, us, [2.6, 2.6, 4.6, 5.0, 3.6, 0.2], [2.6, 2.6, 4.6, 5.0, 3.6, 0.2], hmax=2.8)
    etch = (f.U > 30) & (f.U < 46) & (np.abs(np.abs(f.V) - 1.8 - 0.6 * np.sin(f.U * 0.9)) < 0.45)
    _sword_paint(art, f, m, h, "golden", base=0.52, extra=etch * -0.2)
    # Swept quillons curling toward the blade, with a langet over the blade's root.
    for sgn in (-1, 1):
        q = polyline([f.xy(20.4, 0), f.xy(20.6, sgn * 6), f.xy(22.4, sgn * 10.4), f.xy(25.6, sgn * 11.6), f.xy(27.4, sgn * 10.4)], 2.6)
        art.paint(q, dome(q, 1.4), pal, base=0.5, spec=0.45, power=12)
    lg = f.poly([(18.8, 0), (21, -3.4), (25.2, 0), (21, 3.4)])
    art.paint(lg, dome(lg, 1.8), pal, base=0.5, spec=0.45, power=12)
    art.dots([f.xy(21.2, 0)], 1.3, RED_GEM)


def sw_diamond(art, f):
    pal = mpal("diamond")
    # Grip of dark stone banded in steel, a cut gem for a pommel.
    grip(art, f, 6.5, 19, 2.3, DARK_GRIP, "bands", base=0.5)
    gem = f.poly([(0.4, 0), (3.6, -3.6), (6.8, 0), (3.6, 3.6)])
    gf = np.where(f.V < 0, 0.12, -0.1) + np.where(f.U < 3.6, 0.06, -0.06)
    art.paint(gem, np.ones((N, N)), pal, base=0.55, diff=0.0, albedo=gf)
    # A crystal blade: widens to a shoulder near the tip, then a sharp, faceted point.
    us = [21, 30, 56, 62, 74]
    m, h, s, side = blade(f, us, [2.6, 3.6, 5.6, 5.2, 0.2], [2.6, 3.6, 5.6, 5.2, 0.2], hmax=3.4, edge=0.0)
    # Facets: flat planes split along the ridge and by cuts across the blade.
    band = np.floor((f.U - f.V * np.sign(f.V) * 0.6) / 9.0)
    facet = np.where(f.V < 0, 0.2, -0.16) + ((band % 2) - 0.5) * 0.22
    _sword_paint(art, f, m, h * 0.2, "diamond", base=0.55, facet=facet)
    sp = m & (art.rng.random((N, N)) > 0.985) & (f.V < 0)
    art.img[sp, :3] = pal[-1]
    # Guard of crystal shards, swept back toward the hand.
    for sgn in (-1, 1):
        sh = f.poly([(19.2, sgn * 1.0), (23.2, sgn * 2.0), (17.8, sgn * 11.8), (17.0, sgn * 9.6)])
        art.paint(sh, np.ones((N, N)), pal, base=0.5, diff=0.0, albedo=np.where(f.U * 0.6 - sgn * f.V * 0.3 > 13.6, 0.14, -0.08))
    core = f.box(18.8, 22.4, -2.2, 2.2)
    art.paint(core, dome(core, 1.2), _pal("#c8ccd4", 0.6), base=0.5, spec=0.3)


def sw_netherite(art, f):
    pal = mpal("netherite")
    trim = _pal("#7a5a46", 0.5, 1.1)
    # Spiked pommel.
    sp = f.poly([(-0.6, 0), (4.2, -3.2), (7.6, -2.0), (7.6, 2.0), (4.2, 3.2)])
    art.paint(sp, dome(sp, 1.8), pal, base=0.5, spec=0.25)
    grip(art, f, 7, 18.5, 2.4, _pal("#3a2a2c", 0.32), "bands", base=0.45)
    # A heavy blade: sawtooth spine near the hilt, a clipped, hooked point.
    us = np.linspace(21, 73, 105)
    left = np.interp(us, [21, 28, 56, 64, 70, 73], [4.0, 4.6, 4.6, 3.6, 1.6, 0.2])
    teeth = (us > 30) & (us < 50)
    left = left + teeth * (((us - 30) % 4.5) / 4.5) * 2.8
    right = np.interp(us, [21, 28, 58, 66, 73], [4.0, 4.8, 5.0, 3.6, 0.2])
    m, h, s, side = blade(f, us, left, right, ridge=-0.35, hmax=2.8)
    seams = (np.abs(((f.U - f.V * 0.3) % 7.5) - 3.75) < 0.4) & (np.abs(f.V) < 2.4)
    _sword_paint(art, f, m, h, "netherite", base=0.52, extra=seams * -0.12)
    # Guard: two horns curving back toward the hand.
    for sgn in (-1, 1):
        hn = polyline([f.xy(21.4, 0), f.xy(21.6, sgn * 5), f.xy(19.6, sgn * 9.4), f.xy(15.8, sgn * 11.2)], 3.0)
        hn &= ~(f.box(-1, 17, -12, 12) & (np.abs(f.V) > 11))
        art.paint(hn, dome(hn, 1.5), pal, base=0.5, spec=0.25, albedo=tex_for(art, hn, f, "dark"))
    core = f.box(19.4, 23.0, -2.6, 2.6)
    art.paint(core, dome(core, 1.3), trim, base=0.45, spec=0.3)


def sw_copper(art, f):
    pal = mpal("copper")
    # Crescent pommel, a swelling wooden grip with rivets, a short bar guard, a leaf blade.
    cr = f.disc(3.6, 0, 2.6, 4.6) & ~f.disc(1.4, 0, 2.2, 3.4)
    art.paint(cr, dome(cr, 1.5), pal, base=0.5, spec=0.3)
    grip(art, f, 5.5, 19, 2.2, WOOD, "spindle", base=0.5)
    art.dots([f.xy(9.5, 0), f.xy(15.5, 0)], 1.1, pal)
    us = [21, 28, 44, 54, 62, 70]
    m, h, s, side = blade(f, us, [2.4, 2.6, 4.2, 4.8, 3.6, 0.2], [2.4, 2.6, 4.2, 4.8, 3.6, 0.2], hmax=2.6)
    mid = (np.abs(f.V) < 0.7) & (f.U < 62)
    _sword_paint(art, f, m, h + mid * 0.7, "copper", base=0.52)
    g = f.disc(20.5, 0, 1.8, 6.6)
    art.paint(g, dome(g, 1.4), pal, base=0.45, spec=0.3)
    patina(art, m | g | cr, 0.3)


def sw_tin(art, f):
    pal = mpal("tin")
    art.paint(f.disc(4.0, 0, 2.7), dome(f.disc(4.0, 0, 2.7), 2.2), pal, base=0.5, spec=0.12)
    grip(art, f, 6.4, 19, 2.2, WOOD, "plain", base=0.48)
    fer = f.box(6.2, 7.6, -2.6, 2.6) | f.box(17.6, 19, -2.6, 2.6)
    art.paint(fer, np.ones((N, N)), pal, base=0.6, outline=False)
    # A plain short sword with a long triangular point.
    us = [21, 50, 66]
    m, h, s, side = blade(f, us, [3.2, 3.0, 0.2], [3.2, 3.0, 0.2], hmax=2.0)
    _sword_paint(art, f, m, h, "tin", base=0.5)
    # An oval disc guard.
    g = f.disc(20.6, 0, 1.6, 7.0)
    art.paint(g, dome(g, 1.5), pal, base=0.45, spec=0.12)


def sw_bronze(art, f):
    pal = mpal("bronze")
    # Fishtail pommel.
    ft = f.poly([(0.4, -4.6), (3.0, -2.0), (5.4, -1.8), (5.4, 1.8), (3.0, 2.0), (0.4, 4.6), (2.0, 0)])
    art.paint(ft, dome(ft, 1.4), pal, base=0.5, spec=0.3)
    grip(art, f, 5.4, 19, 2.2, _pal("#3c2a1c", 0.35), "bands", base=0.45)
    # A khopesh: a straight neck, then a deep crescent with the edge on its outer curve.
    us = np.linspace(20, 72, 40)
    t = np.clip((us - 38) / 34, 0, 1)
    center = -np.sin(t * np.pi) * 6.5 + t * 1.5
    left = np.interp(us, [20, 38, 46, 64, 72], [1.8, 2.0, 2.4, 2.6, 0.2])
    right = np.interp(us, [20, 38, 46, 60, 72], [1.8, 2.0, 5.0, 4.6, 0.2])
    m, h, s, side = blade(f, us, left, right, center=center, ridge=-0.6, hmax=2.4)
    _sword_paint(art, f, m, h, "bronze", base=0.52)
    g = f.box(19, 21.4, -4.4, 4.4)
    art.paint(g, dome(g, 1.2), pal, base=0.45, spec=0.3)
    patina(art, m | g, 0.22, "#5a9a78")


def sw_lead(art, f):
    pal = mpal("lead")
    blk = f.box(0.5, 6.0, -3.4, 3.4)
    art.paint(blk, dome(blk, 1.6), pal, base=0.5, spec=0.1)
    grip(art, f, 6, 19, 2.6, _pal("#4a3a2e", 0.35), "bands", base=0.45)
    for u in (8.5, 13, 17.5):
        b = f.box(u - 0.6, u + 0.6, -3.0, 3.0)
        art.paint(b, np.ones((N, N)), pal, base=0.55, outline=False)
    # A cleaver: straight spine, wide belly and a squared, angled end.
    us = [21, 24, 62, 67.6, 68]
    m, h, s, side = blade(f, us, [3.2, 3.2, 3.2, 3.2, 0.1], [4.2, 6.2, 6.6, 1.0, 0.1], ridge=-0.8, hmax=2.4)
    dents = (value_noise(art.rng, 8) - 0.5) * 0.9
    _sword_paint(art, f, m, h + dents, "lead", base=0.48)
    hole = f.disc(60, -0.4, 1.3) & m
    art.img[hole, 3] = 0
    art.img[_dilate(hole) & ~hole & m, :3] = outline_of(pal)
    g = f.box(18.6, 22.0, -8.4, 8.4)
    art.paint(g, dome(g, 1.4), pal, base=0.42, spec=0.1)


def sw_silver(art, f):
    pal = mpal("silver")
    # Pear pommel, wire-wrapped grip, a swept hilt and a long, thin rapier blade.
    pm = f.disc(4.0, 0, 3.0, 2.4)
    art.paint(pm, dome(pm, 2), pal, base=0.5, spec=0.55, power=22)
    grip(art, f, 6.5, 18.8, 2.0, pal, "spiral", base=0.42)
    us = [21, 40, 76]
    m, h, s, side = blade(f, us, [2.0, 1.7, 0.2], [2.0, 1.7, 0.2], hmax=1.8, edge=0.4)
    _sword_paint(art, f, m, h, "silver", base=0.55)
    bow = polyline([f.xy(21, 0), f.xy(19, 5.6), f.xy(13, 7.0), f.xy(7, 5.2), f.xy(6.2, 2.0)], 1.6)
    art.paint(bow, dome(bow, 1), pal, base=0.5, spec=0.55, power=22)
    q = polyline([f.xy(21, -9.4), f.xy(20.6, 0), f.xy(22.4, 7.4)], 1.7)
    art.paint(q, dome(q, 1), pal, base=0.5, spec=0.55, power=22)
    art.dots([f.xy(21, -9.6)], 1.6, pal)
    shell = f.disc(21.4, 0.4, 1.8, 4.2)
    art.paint(shell, dome(shell, 2), pal, base=0.5, spec=0.55, power=22)


def sw_tungsten(art, f):
    pal = mpal("tungsten")
    hexp = f.poly([(0.6, -1.6), (2.4, -3.2), (5.6, -3.2), (6.8, 0), (5.6, 3.2), (2.4, 3.2), (0.6, 1.6)])
    art.paint(hexp, dome(hexp, 1.2) + (f.V < 0) * 0.6, pal, base=0.5, spec=0.25)
    grip(art, f, 6.8, 19, 2.2, DARK_GRIP, "cross", base=0.48)
    # A falchion: curves back, widens toward the tip and ends in a clipped point.
    us = np.linspace(21, 72, 30)
    center = -((us - 21) / 51) ** 2 * 5.0
    left = np.interp(us, [21, 60, 66, 72], [2.0, 2.2, 1.8, 0.2])
    right = np.interp(us, [21, 40, 60, 66, 72], [2.4, 3.6, 5.2, 4.0, 0.2])
    m, h, s, side = blade(f, us, left, right, center=center, ridge=-0.7, hmax=2.4)
    _sword_paint(art, f, m, h, "tungsten", base=0.52)
    # An S-guard: one quillon bends toward the blade, the other toward the hand.
    g = polyline([f.xy(23.4, -8.4), f.xy(21.4, -6.6), f.xy(20.6, 0), f.xy(19.6, 6.6), f.xy(17.4, 8.6)], 2.6)
    art.paint(g, dome(g, 1.3), pal, base=0.48, spec=0.25)


def sw_platinum(art, f):
    pal = mpal("platinum")
    gold = mpal("golden")
    # A long ito-wrapped grip with a cap, a round tsuba, a gold collar and a curved blade.
    art.paint(f.box(1.4, 3.6, -2.5, 2.5), dome(f.box(1.4, 3.6, -2.5, 2.5), 1), pal, base=0.45)
    gm = f.box(3.4, 18.6, -2.3, 2.3)
    art.paint(gm, np.ones((N, N)), _pal("#ece6d6", 0.4), base=0.6, outline=True)
    k = _wrap_cross(f, 3.4, 18.6, 2.3, step=4.2)
    cord = gm & (k < 0.2)
    art.paint(cord, np.sqrt(np.clip(1 - (f.V / 2.3) ** 2, 0, 1)) * 2, BLUE_CORD, base=0.5, outline=False)
    us = np.linspace(22, 75, 30)
    center = -((us - 22) / 53) ** 2 * 4.0
    left = np.interp(us, [22, 66, 75], [1.6, 1.5, 0.2])
    right = np.interp(us, [22, 66, 71, 75], [2.4, 2.2, 2.0, 0.2])
    m, h, s, side = blade(f, us, left, right, center=center, ridge=-0.4, hmax=1.8, edge=0.8)
    # The hamon: a pale, wavy temper line along the edge.
    hamon = (s > 0.2 + 0.35 * np.sin(f.U * 1.1)) & (f.U < 70)
    _sword_paint(art, f, m, h, "platinum", base=0.5, extra=hamon * 0.14)
    hab = f.box(21, 23, -1.9, 1.9)
    art.paint(hab, dome(hab, 1), gold, base=0.55, spec=0.4)
    ts = f.disc(20.2, 0, 1.3, 6.0)
    art.paint(ts, dome(ts, 1.2), _pal("#4a4e58", 0.4), base=0.45, spec=0.2)


def sw_magmarite(art, f):
    rng = art.rng
    pal = mpal("magmarite")
    orb = f.disc(3.8, 0, 3.4)
    art.paint(orb, dome(orb, 3), LAVA, base=0.5, spec=0.3)
    grip(art, f, 6.8, 19, 2.3, _pal("#2a2220", 0.3), "bands", base=0.4)
    # A flamberge: wavy edges like a flame, black rock with glowing veins.
    us = np.linspace(21, 73, 60)
    wave = np.sin((us - 21) * 0.55) * 1.1
    w = np.interp(us, [21, 60, 73], [4.6, 4.0, 0.2])
    m, h, s, side = blade(f, us, np.maximum(w + wave, 0.2), np.maximum(w - wave, 0.2) * (us < 73), hmax=2.8, edge=0.0)
    glow = cracks(art, m, rng, 6, 0.9)
    _sword_paint(art, f, m, h, "magmarite", base=0.5, glow=glow)
    for sgn in (-1, 1):
        fl = f.poly([(18.6, 0), (23.4, sgn * 2.6), (24.6, sgn * 7.8), (21.4, sgn * 6), (20.6, sgn * 10.4), (18.2, sgn * 5)])
        art.paint(fl, dome(fl, 1.3), pal, base=0.45, glow=cracks(art, fl, rng, 1, 0.8))


SWORDS = {
    "wooden": sw_wooden, "stone": sw_stone, "iron": sw_iron, "golden": sw_golden, "gold": sw_golden,
    "diamond": sw_diamond, "netherite": sw_netherite, "copper": sw_copper, "tin": sw_tin,
    "bronze": sw_bronze, "lead": sw_lead, "silver": sw_silver, "tungsten": sw_tungsten,
    "platinum": sw_platinum, "magmarite": sw_magmarite,
}


def sword(mat):
    art = Art(f"{mat}_sword")
    f = Frame()
    SWORDS.get(mat, sw_iron)(art, f)
    return art.done()


# ---------------------------------------------------------------- armour icons
#
# Every material has its own build. Pieces are drawn front-on: a silhouette (polygons), a
# rounded height field so plates read as curved metal, then the material's surface (rings,
# rivets, bands, flutes, scales, lames, stitches, facets, cracks) and its own extras
# (pauldrons, crests, horns, skirts of straps, knee cops, pointed toes...).

STYLE = {
    "leather": "leather", "chainmail": "mail", "iron": "plate", "golden": "ornate", "gold": "ornate",
    "diamond": "facet", "netherite": "spiked", "copper": "roman", "tin": "brigandine",
    "bronze": "greek", "lead": "banded", "silver": "gothic", "tungsten": "scale",
    "platinum": "samurai", "magmarite": "volcanic",
}


def sym(pts):
    """Mirror a list of left-half points (x <= 32) into a closed symmetric outline."""
    return pts + [(64 - x, y) for (x, y) in reversed(pts)]


def ell(cx, cy, rx, ry):
    return ((PX - cx) / rx) ** 2 + ((PY - cy) / ry) ** 2 <= 1


def box(x0, y0, x1, y1):
    return (PX >= x0) & (PX <= x1) & (PY >= y0) & (PY <= y1)


def mirror(m):
    return m | m[:, ::-1]


class Kit:
    """Painting helpers bound to one sprite and one material."""

    def __init__(self, name, mat):
        self.art = Art(name)
        self.mat = mat
        self.d = MAT[mat]
        self.pal = mpal(mat)
        self.rng = self.art.rng

    def plate(self, mask, r=3.0, base=0.5, extra=None, h=None, glow=None, pal=None, spec=None, outline=True):
        d = self.d
        hh = dome(mask, r) if h is None else h
        alb = tex_for(self.art, mask, None, d["kind"])
        if extra is not None:
            alb = alb + extra
        if d["kind"] == "gem":
            alb = alb + facets(self.rng, mask)
        if d["kind"] == "lava" and glow is None:
            glow = cracks(self.art, mask, self.rng, max(1, int(mask.sum() / 260)), 0.85)
        self.art.paint(mask, hh, self.pal if pal is None else pal, base=base,
                       diff=1.5, spec=d["spec"] if spec is None else spec, power=d["power"], albedo=alb, glow=glow,
                       outline=outline)

    def groove(self, mask, dark=3, light=None):
        """An engraved line: dark, with a lit lip just below it."""
        m = mask & (self.art.img[..., 3] > 0)
        if light is not None:
            lip = np.roll(m, 1, 0) & ~m & (self.art.img[..., 3] > 0)
            self.art.img[lip, :3] = self.pal[light]
        self.art.img[m, :3] = self.pal[dark]

    def rivets(self, pts, r=1.3, pal=None):
        self.art.dots(pts, r, self.pal if pal is None else pal, base=0.6)

    def stitches(self, path, step=3.0, pal=None):
        """A dashed seam along a polyline."""
        pal = _pal("#e0c890", 0.3) if pal is None else pal
        m = np.zeros((N, N), bool)
        for (a, b) in zip(path, path[1:]):
            L = np.hypot(b[0] - a[0], b[1] - a[1])
            for t in np.arange(0, L, step):
                x = a[0] + (b[0] - a[0]) * t / L
                y = a[1] + (b[1] - a[1]) * t / L
                x2 = a[0] + (b[0] - a[0]) * min(t + step * 0.5, L) / L
                y2 = a[1] + (b[1] - a[1]) * min(t + step * 0.5, L) / L
                m |= seg(x, y, x2, y2) <= 0.55
        m &= self.art.img[..., 3] > 0
        self.art.img[m, :3] = pal[9]

    def cut(self, mask):
        """See-through hole with an outline (eye slits, open faces)."""
        a = self.art.img
        inside = mask & (a[..., 3] > 0)
        a[inside, 3] = 0
        rim = _dilate(inside) & ~inside & (a[..., 3] > 0)
        a[rim, :3] = outline_of(self.pal)

    def dark(self, mask, idx=1):
        m = mask & (self.art.img[..., 3] > 0)
        self.art.img[m, :3] = self.pal[idx]

    def done(self):
        return self.art.done()


def facets(rng, mask):
    """Gem facets: flat planes, each a different brightness, split along straight cuts."""
    ys, xs = np.nonzero(mask)
    if len(xs) == 0:
        return 0.0
    cx, cy = xs.mean(), ys.mean()
    ang = np.arctan2(PY - cy, PX - cx)
    sector = np.floor((ang + np.pi) / (np.pi / 3)).astype(int)
    ring = (np.hypot(PX - cx, PY - cy) > 9).astype(int)
    k = (sector * 2 + ring) % 6
    vals = np.array([0.16, -0.06, 0.08, -0.14, 0.02, -0.02])
    lit = np.where((PX - cx) + (PY - cy) < 0, 0.06, -0.06)
    return vals[k] + lit


def rings_tex(mask, cell=3.0):
    """Mail: a mesh of round rings, holes where the rings are hollow."""
    gx = (PX / cell) % 1 - 0.5
    gy = (PY / cell + np.floor(PX / cell) % 2 * 0.5) % 1 - 0.5
    r = np.hypot(gx, gy)
    hole = mask & (r < 0.17)
    h = np.clip(0.5 - np.abs(r - 0.33) * 3.0, 0, 1) * 1.4
    return h, hole


def scales_tex(mask, w=6.0, hgt=5.0):
    """Overlapping scales, each lower row offset by half a scale."""
    row = np.floor(PY / hgt)
    xo = PX / w + (row % 2) * 0.5
    fx = xo % 1 - 0.5
    fy = (PY / hgt) % 1
    # Each scale is a dome rounded at the bottom.
    d = np.hypot(fx * 1.1, (fy - 0.2) * 0.9)
    h = np.clip(1 - d * 1.6, 0, 1) * 2.0 + fy * 0.9
    edge = (d > 0.56) & (fy > 0.45)
    return h, edge


def lames_tex(y0, step, tilt=0.0):
    """Horizontal overlapping bands: each lit at its top, shaded at its lower lip."""
    fy = ((PY - y0 - PX * tilt) / step) % 1
    return (1 - fy) * 1.8, (fy > 0.86)


def flutes_tex(cx, cy, n=9):
    """Radiating flutes (gothic plate), from a point."""
    ang = np.arctan2(PY - cy, PX - cx)
    f = np.cos(ang * n) * 0.8
    return f


# ---------------------------------------------------------------- helmets


def helmet_icon(mat):
    st = STYLE.get(mat, "plate")
    k = Kit(f"{mat}_helmet", mat)
    pal = k.pal
    if st == "leather":
        flaps = mirror(poly([(11, 30), (21, 30), (21, 46), (18, 51), (14, 51), (11, 46)]))
        k.plate(flaps, 2.5)
        cap = ell(32, 31, 21, 21) & (PY <= 33)
        k.plate(cap, 5)
        brim = box(10, 29, 54, 35) & ~box(20, 29, 44, 35) | box(10, 29, 54, 33)
        k.plate(brim, 2, base=0.42)
        k.stitches([(32, 12), (32, 29)])
        k.stitches([(13, 31.5), (51, 31.5)])
        k.stitches([(14, 48), (18, 49)])
        k.stitches([(46, 49), (50, 48)])
        k.dark(box(22.5, 35, 41.5, 36))
    elif st == "mail":
        hood = poly(sym([(32, 7), (20, 10), (12, 20), (10, 34), (8, 56), (32, 56)]))
        h, hole = rings_tex(hood)
        k.plate(hood, 4, h=dome(hood, 4) + h)
        k.cut(hole)
        k.cut(ell(32, 36, 10, 12) & (PY > 25))
        band = box(14, 52, 50, 56) & hood
        k.plate(band, 1.2, pal=_pal("#6a4428", 0.4), spec=0.05)
    elif st == "plate":
        shell = poly(sym([(32, 6), (22, 8), (15, 14), (12, 24), (12, 46), (16, 54), (32, 56)]))
        k.plate(shell, 6)
        k.groove(np.abs(PX - 31.5) < 0.9 & (PY < 24) if False else (np.abs(PX - 31.5) < 0.9) & (PY < 26), dark=11)
        brow = box(12, 24, 52, 27)
        k.plate(brow & shell, 1.5, base=0.55)
        k.rivets([(15, 25.5), (22, 25.5), (42, 25.5), (49, 25.5)], 1.2)
        k.cut(box(16, 31, 48, 33.4))
        for (x, y) in [(38, 40), (41, 40), (44, 40), (38, 43), (41, 43), (44, 43)]:
            k.cut(ell(x, y, 0.9, 0.9))
        k.groove(box(17, 47, 47, 47.8), dark=3, light=11)
    elif st == "ornate":
        cheeks = mirror(poly([(12, 30), (22, 30), (22, 50), (16, 54), (12, 48)]))
        k.plate(cheeks, 2.6)
        dome_ = ell(32, 32, 20, 22) & (PY <= 32)
        k.plate(dome_, 6)
        comb = poly([(30, 4), (34, 4), (40, 10), (36, 26), (28, 26), (24, 10)])
        k.plate(comb, 2, base=0.55, extra=np.sin(PY * 1.2) * 0.06)
        crown = poly([(10, 34), (10, 26), (14, 20), (18, 26), (24, 18), (28, 25), (32, 15), (36, 25), (40, 18), (46, 26), (50, 20), (54, 26), (54, 34)])
        k.plate(crown, 2.2, base=0.55)
        k.groove(box(11, 30, 53, 30.8), dark=4, light=13)
        k.art.dots([(32, 22), (22, 28), (42, 28)], 1.8, RED_GEM)
    elif st == "facet":
        sh = poly(sym([(32, 4), (20, 9), (12, 18), (9, 32), (12, 52), (22, 56), (24, 44), (32, 48)]))
        k.plate(sh, 2, h=np.ones((N, N)))
        fin = poly([(32, 2), (36, 8), (32, 24), (28, 8)])
        k.plate(fin, 1, h=np.ones((N, N)), base=0.62)
        k.cut(poly([(15, 28), (49, 28), (47, 32), (34, 33), (34, 46), (30, 46), (30, 33), (17, 32)]))
        sp = (k.rng.random((N, N)) > 0.985) & (k.art.img[..., 3] > 0) & (PX + PY < 64)
        k.art.img[sp, :3] = 255
    elif st == "spiked":
        horns = mirror(polyline([(15, 22), (9, 18), (6, 11), (7, 4)], 5.0) | polyline([(7, 4), (8, 1)], 2.5))
        k.plate(horns, 2, pal=_pal("#3a2e30", 0.5), extra=np.sin(PY * 1.6) * 0.06)
        shell = poly(sym([(32, 8), (22, 10), (14, 18), (12, 30), (12, 54), (32, 56)]))
        k.plate(shell, 6)
        k.cut(box(16, 28, 48, 31.4) | box(30.2, 28, 33.8, 44))
        ridge = (np.abs(PX - 31.5) < 1.2) & (PY < 28) & shell
        k.plate(ridge, 1, base=0.6)
        k.groove(box(13, 46, 51, 46.9), dark=1, light=10)
        k.rivets([(16, 50.5), (24, 50.5), (40, 50.5), (48, 50.5)], 1.3, _pal("#7a5a46", 0.5))
    elif st == "roman":
        crest = poly([(12, 18), (18, 8), (32, 4), (46, 8), (52, 18), (48, 20), (32, 12), (16, 20)])
        brist = np.sin(np.arctan2(PY - 22, PX - 32) * 40) * 0.12
        k.art.paint(crest, dome(crest, 2), _pal("#a8302a", 0.5, 1.1), base=0.5, albedo=brist)
        cheeks = mirror(poly([(11, 30), (21, 30), (23, 40), (20, 52), (14, 54), (11, 46)]))
        k.plate(cheeks, 2.6)
        k.rivets([(16, 34), (48, 34)], 1.3)
        dm = ell(32, 32, 20, 20) & (PY <= 32)
        k.plate(dm, 5)
        brow = poly([(9, 32), (55, 32), (52, 27), (12, 27)])
        k.plate(brow, 1.6, base=0.56)
        patina(k.art, cheeks | dm | brow, 0.3)
    elif st == "brigandine":
        brim = ell(32, 33, 28, 5)
        k.plate(brim, 2, base=0.46)
        dm = ell(32, 30, 16, 20) & (PY <= 31)
        k.plate(dm, 6)
        k.groove((np.abs(PX - 31.5) < 0.8) & (PY < 30), dark=3, light=None)
        k.rivets([(19, 27), (25, 28.5), (32, 29), (39, 28.5), (45, 27)], 1.1)
        k.groove(ell(32, 33, 28, 5) & ~ell(32, 33, 26.5, 3.6) & (PY > 33), dark=2)
    elif st == "greek":
        plume = poly([(22, 10), (24, 2), (32, 0), (40, 2), (42, 10), (32, 12)])
        hair = np.sin(PX * 2.3) * 0.12
        k.art.paint(plume, dome(plume, 2), _pal("#2a2420", 0.35), base=0.5, albedo=hair)
        bar = poly([(30, 9), (34, 9), (34, 14), (30, 14)])
        k.plate(bar, 1)
        sh = poly(sym([(32, 10), (20, 12), (13, 20), (11, 34), (13, 50), (20, 57), (27, 58), (28, 44)]))
        k.plate(sh, 6)
        k.cut(poly([(15, 27), (29, 29), (29, 33), (31, 36), (31, 58), (33, 58), (33, 36), (35, 33), (35, 29), (49, 27), (47, 34), (37, 35), (37, 58), (27, 58), (27, 35), (17, 34)]))
        k.groove((np.abs(PX - 31.5) < 0.8) & (PY > 11) & (PY < 26), dark=3, light=None)
        patina(k.art, sh, 0.2, "#5a9a78")
    elif st == "banded":
        sh = poly(sym([(32, 8), (16, 8), (12, 12), (12, 56), (32, 56)]))
        k.plate(sh, 4, extra=(value_noise(k.rng, 8) - 0.5) * 0.1)
        for y0, y1 in ((8, 13), (50, 56)):
            k.plate(box(11, y0, 53, y1) & _dilate(sh), 1.4, base=0.55)
        cross = box(29.5, 13, 34.5, 50) & ~box(29, 25, 35, 29)
        k.plate(cross, 1.2, base=0.58)
        k.cut(box(15, 25.6, 49, 28.6) & ~box(29.4, 25, 34.6, 29))
        for (x, y) in [(20, 36), (24, 36), (40, 36), (44, 36), (20, 40), (24, 40), (40, 40), (44, 40)]:
            k.cut(ell(x, y, 1.0, 1.0))
        k.rivets([(15, 10.5), (23, 10.5), (41, 10.5), (49, 10.5), (15, 53), (23, 53), (41, 53), (49, 53)], 1.3)
    elif st == "gothic":
        sh = poly([(32, 6), (22, 8), (15, 14), (12, 24), (12, 40), (16, 42), (30, 40), (44, 44), (58, 50), (54, 40), (52, 22), (44, 10)])
        fl = flutes_tex(32, 46, 11) * 0.5
        k.plate(sh, 6, h=dome(sh, 6) + fl)
        k.cut(box(16, 27, 47, 29.4))
        vis = poly([(14, 30.5), (50, 30.5), (49, 42), (32, 50), (16, 42)])
        k.plate(vis, 3, base=0.55)
        k.groove(box(18, 38, 46, 38.7), dark=3, light=13)
    elif st == "scale":
        cone = poly([(32, 3), (46, 18), (52, 32), (12, 32), (18, 18)])
        k.plate(cone, 5, h=dome(cone, 6) + (np.abs(PX - 32) < 1) * 1.2)
        avent = poly(sym([(12, 32), (8, 56), (32, 56)])) & ~ell(32, 40, 10, 9)
        h, edge = scales_tex(avent, 5, 4)
        k.plate(avent, 1, h=h)
        k.dark(edge & avent, 2)
        k.plate(box(29.5, 30, 34.5, 46), 1.2, base=0.6)
        k.plate(box(11, 30, 53, 33.5), 1.4, base=0.55, pal=_pal("#5a3a24", 0.4), spec=0.05)
    elif st == "samurai":
        for i, (y, w) in enumerate([(34, 22), (40, 25), (46, 28)]):
            lame = poly([(32 - w, y + 7), (32 - w + 4, y), (32 + w - 4, y), (32 + w, y + 7)])
            k.plate(lame, 1.6, base=0.48 - i * 0.03)
            k.stitches([(32 - w + 4, y + 4), (32 + w - 4, y + 4)], 2.0, BLUE_CORD)
        bowl = ell(32, 32, 17, 19) & (PY <= 33)
        ribs = np.cos((PX - 32) * 0.9) * 0.4
        k.plate(bowl, 6, h=dome(bowl, 6) + ribs * (PY < 32))
        k.art.paint(mirror(poly([(9, 24), (15, 28), (15, 37), (9, 38)])), np.ones((N, N)), pal, base=0.6)
        gold = mpal("golden")
        horns = mirror(poly([(30, 27), (27, 24), (16, 4), (12, 0), (17, 1), (24, 12), (32, 24)]))
        k.art.paint(horns, dome(horns, 1.2), gold, base=0.55, spec=0.45, power=12)
        k.art.dots([(32, 26)], 2.2, gold)
    else:  # volcanic
        sh = poly([(32, 4), (28, 9), (22, 6), (20, 12), (14, 12), (14, 20), (10, 26), (12, 34), (10, 46), (14, 56),
                   (32, 54), (50, 56), (54, 46), (52, 34), (54, 26), (50, 20), (50, 12), (44, 12), (42, 6), (36, 9)])
        k.plate(sh, 5)
        slit = box(16, 30, 48, 33)
        k.art.paint(slit, np.ones((N, N)), LAVA, base=0.75, outline=False)
    return k.done()


# ---------------------------------------------------------------- chestplates


def _torso(sleeve=26, waist=14, hem=56, neck=8):
    return poly([(4, 12), (20, 8), (26, neck + 6), (38, neck + 6), (44, 8), (60, 12), (60, sleeve), (50, sleeve),
                 (64 - waist, hem), (waist, hem), (14, sleeve), (4, sleeve)])


def chest_icon(mat):
    st = STYLE.get(mat, "plate")
    k = Kit(f"{mat}_chestplate", mat)
    pal = k.pal
    if st == "leather":
        body = _torso(28, 15, 56)
        k.plate(body, 4)
        k.cut(poly([(26, 13), (38, 13), (32, 30)]))
        for y in range(16, 30, 4):
            k.art.paint(polyline([(28.5 + (y - 14) * 0.12, y), (35.5 - (y - 14) * 0.12, y + 3)], 1.0), np.ones((N, N)), CORD, base=0.6, outline=False)
        k.stitches([(6, 14), (20, 10)])
        k.stitches([(44, 10), (58, 14)])
        k.stitches([(16, 53), (48, 53)])
        belt = box(14, 42, 50, 47) & body
        k.plate(belt, 1.2, pal=_pal("#4a2c18", 0.4), base=0.45)
        k.art.dots([(32, 44.5)], 2.0, mpal("iron"))
    elif st == "mail":
        body = poly([(2, 12), (20, 8), (26, 13), (38, 13), (44, 8), (62, 12), (62, 34), (50, 34), (52, 58), (34, 58), (32, 52), (30, 58), (12, 58), (14, 34), (2, 34)])
        h, hole = rings_tex(body)
        k.plate(body, 4, h=dome(body, 4) + h)
        k.cut(hole)
        k.plate(box(13, 40, 51, 44.5) & body, 1.2, pal=_pal("#6a4428", 0.4), spec=0.05)
        k.art.dots([(32, 42.2)], 1.8, mpal("iron"))
    elif st == "plate":
        body = _torso(26, 16, 46)
        k.plate(body, 7, h=dome(body, 7) + np.clip(3 - np.abs(PX - 32) * 0.4, 0, 3) * 0.5)
        for i in range(3):
            y = 44 + i * 4
            w = 17 - i
            k.plate(box(w, y, 64 - w, y + 4.5), 1.5, base=0.48 - i * 0.03)
        for sx in (0, 1):
            for i in range(3):
                cx = 12 if sx == 0 else 52
                pm = ell(cx, 14 + i * 4, 11 - i, 6) & (PY >= 10 + i * 4)
                k.plate(pm, 2, base=0.55 - i * 0.04)
        k.rivets([(12, 11), (52, 11), (20, 46), (44, 46), (20, 50), (44, 50)], 1.2)
        k.plate(box(24, 9, 40, 14), 1.4, base=0.55)
    elif st == "ornate":
        body = _torso(26, 15, 54)
        k.plate(body, 7)
        sun = np.hypot(PX - 32, PY - 30)
        ang = np.arctan2(PY - 30, PX - 32)
        rays = (sun > 5) & (sun < 11) & (np.cos(ang * 12) > 0.55)
        k.groove(rays & body, dark=5, light=None)
        k.groove((np.abs(sun - 5) < 0.6) & body, dark=4)
        for cx in (11, 53):
            pm = ell(cx, 16, 10, 8)
            sc = np.cos(np.arctan2(PY - 16, PX - cx) * 7)
            k.plate(pm & ~((PY > 20) & (sc < -0.2) & (np.hypot(PX - cx, PY - 16) > 7)), 3, base=0.54)
        k.groove(box(16, 50, 48, 50.8) & body, dark=4, light=13)
        k.art.dots([(32, 30)], 2.6, RED_GEM)
    elif st == "facet":
        body = poly([(4, 16), (18, 10), (26, 15), (38, 15), (46, 10), (60, 16), (58, 28), (48, 28), (44, 50), (32, 58), (20, 50), (16, 28), (6, 28)])
        k.plate(body, 2, h=np.ones((N, N)))
        for sx in (1, -1):
            sp = poly([(32 + sx * 18, 14), (32 + sx * 31, 2), (32 + sx * 28, 18)])
            k.plate(sp, 1, h=np.ones((N, N)), base=0.6)
        sp = (k.rng.random((N, N)) > 0.985) & (k.art.img[..., 3] > 0) & (PX + PY < 60)
        k.art.img[sp, :3] = 255
    elif st == "spiked":
        body = _torso(28, 15, 56, neck=4)
        k.plate(body, 6, extra=((np.abs(PY - 36) < 0.6) | (np.abs(PY - 46) < 0.6)) * -0.15)
        col = poly([(22, 6), (26, 2), (38, 2), (42, 6), (40, 14), (24, 14)])
        k.plate(col, 2, base=0.55)
        for sx in (1, -1):
            cx = 32 + sx * 21
            pm = ell(cx, 16, 12, 9)
            k.plate(pm, 3, base=0.5)
            for j, dx in enumerate((-6, 0, 6)):
                x = cx + dx
                spk = poly([(x - 2.4, 11), (x + sx * 1.0, 0 + abs(dx) * 0.5), (x + 2.4, 11)])
                k.plate(spk, 1, base=0.58, pal=_pal("#3a2e30", 0.5))
        k.rivets([(20, 30), (44, 30), (20, 40), (44, 40)], 1.3, _pal("#7a5a46", 0.5))
    elif st == "roman":
        body = _torso(26, 15, 50)
        h, lip = lames_tex(30, 4.4)
        k.plate(body, 3, h=dome(body, 3) + h * (PY > 30))
        k.dark(lip & body & (PY > 30), 3)
        for sx in (1, -1):
            for i in range(3):
                band = polyline([(32 + sx * (14 + i * 4), 12 + i * 3.5), (32 + sx * (26 + i * 2), 13 + i * 4), (32 + sx * (29), 22 + i * 2)], 3.2)
                k.plate(band, 1.4, base=0.56 - i * 0.04)
        k.plate(box(26, 16, 38, 26), 1.4, base=0.5)
        k.art.dots([(28, 21), (36, 21)], 1.4, mpal("golden"))
        patina(k.art, body, 0.32)
    elif st == "brigandine":
        body = _torso(26, 15, 56)
        cloth = _pal("#3c4a6a", 0.4, 1.0)
        k.art.paint(body, dome(body, 4), cloth, base=0.5, albedo=(value_noise(k.rng, 2) - 0.5) * 0.08)
        pts = [(x + (y % 8) * 0.5, y) for y in range(20, 54, 4) for x in range(18, 47, 4) if body[int(y), int(x)]]
        k.rivets(pts, 1.0)
        for cx in (12, 52):
            pm = ell(cx, 15, 9, 6)
            k.plate(pm, 2.4)
        k.plate(box(24, 10, 40, 15), 1.4)
    elif st == "greek":
        body = _torso(24, 16, 48)
        pec = np.exp(-(((PX - 24) / 7) ** 2 + ((PY - 24) / 5) ** 2)) + np.exp(-(((PX - 40) / 7) ** 2 + ((PY - 24) / 5) ** 2))
        abs_ = sum(np.exp(-(((PX - x) / 3.4) ** 2 + ((PY - y) / 2.6) ** 2)) for x in (28, 36) for y in (34, 39, 44))
        k.plate(body, 5, h=dome(body, 5) + pec * 2.4 + abs_ * 1.4)
        strips = box(14, 48, 50, 60) & (np.abs(((PX - 14) % 6) - 3) < 2.2)
        k.art.paint(strips, dome(strips, 1.2), _pal("#6a4a2a", 0.4), base=0.5)
        for cx in (11, 53):
            k.plate(ell(cx, 14, 9, 6), 2.2, base=0.55)
        patina(k.art, body, 0.18, "#5a9a78")
    elif st == "banded":
        body = poly([(2, 10), (22, 8), (26, 12), (38, 12), (42, 8), (62, 10), (62, 28), (50, 28), (50, 56), (14, 56), (14, 28), (2, 28)])
        h, lip = lames_tex(14, 7)
        k.plate(body, 3, h=dome(body, 3) + h * 0.8, extra=(value_noise(k.rng, 8) - 0.5) * 0.1)
        k.dark(lip & body, 2)
        k.rivets([(x, y) for y in (19, 26, 33, 40, 47) for x in (18, 46)], 1.5)
        k.plate(box(2, 10, 18, 28) | box(46, 10, 62, 28), 2, base=0.55)
    elif st == "gothic":
        body = poly([(6, 14), (20, 9), (26, 13), (38, 13), (44, 9), (58, 14), (56, 26), (48, 26), (44, 46), (32, 56), (20, 46), (16, 26), (8, 26)])
        fl = flutes_tex(32, 58, 13) * 0.6
        k.plate(body, 6, h=dome(body, 6) + fl * (PY > 18))
        k.plate(polyline([(26, 13.5), (32, 17), (38, 13.5)], 2.4), 1, base=0.62)
        for cx in (12, 52):
            pm = ell(cx, 16, 8, 6)
            k.plate(pm, 2.5, h=dome(pm, 2.5) + np.cos(PX * 1.4) * 0.4, base=0.56)
    elif st == "scale":
        body = _torso(28, 15, 56)
        h, edge = scales_tex(body)
        k.plate(body, 2, h=dome(body, 2) * 0.6 + h)
        k.dark(edge & body, 2)
        trim = _pal("#5a3a24", 0.4)
        k.art.paint(polyline([(20, 9), (26, 14.5), (38, 14.5), (44, 9)], 2.6), np.ones((N, N)), trim, base=0.5)
        k.art.paint(box(15, 53, 49, 56), np.ones((N, N)), trim, base=0.45)
    elif st == "samurai":
        body = poly([(18, 10), (26, 14), (38, 14), (46, 10), (50, 24), (50, 56), (14, 56), (14, 24)])
        h, lip = lames_tex(14, 5.4)
        k.plate(body, 2, h=dome(body, 2) + h)
        k.dark(lip & body, 3)
        for x in range(18, 48, 6):
            k.stitches([(x, 16), (x, 55)], 2.2, BLUE_CORD)
        for sx in (1, -1):
            sode = poly([(32 + sx * 15, 10), (32 + sx * 30, 12), (32 + sx * 30, 36), (32 + sx * 17, 34)])
            h2, lip2 = lames_tex(10, 5)
            k.plate(sode, 1.5, h=h2, base=0.52)
            k.dark(lip2 & sode, 3)
        k.art.dots([(32, 20)], 2.0, mpal("golden"))
    else:  # volcanic
        body = poly([(4, 14), (12, 8), (20, 10), (24, 6), (28, 14), (36, 14), (40, 6), (44, 10), (52, 8), (60, 14), (58, 22), (60, 28),
                     (50, 28), (52, 40), (48, 56), (40, 52), (32, 58), (24, 52), (16, 56), (12, 40), (14, 28), (4, 28), (6, 22)])
        k.plate(body, 5)
    return k.done()


# ---------------------------------------------------------------- leggings


def legs_icon(mat):
    st = STYLE.get(mat, "plate")
    k = Kit(f"{mat}_leggings", mat)
    base = poly([(12, 8), (52, 8), (53, 56), (37, 56), (34, 24), (30, 24), (27, 56), (11, 56)])
    belt = box(11, 8, 53, 14)
    if st == "leather":
        k.plate(base, 3)
        k.plate(belt, 1.2, base=0.42)
        k.stitches([(20, 16), (19, 54)])
        k.stitches([(44, 16), (45, 54)])
        for cx in (19, 45):
            pt = box(cx - 5, 32, cx + 5, 40)
            k.plate(pt, 1.4, pal=_pal("#6e4426", 0.45), base=0.48)
        k.art.paint(polyline([(29, 10), (32, 16), (35, 10)], 1.2), np.ones((N, N)), CORD, base=0.6, outline=False)
    elif st == "mail":
        h, hole = rings_tex(base)
        k.plate(base, 3, h=dome(base, 3) + h)
        k.cut(hole & ~belt)
        k.plate(belt, 1.2, pal=_pal("#6a4428", 0.4), spec=0.05)
    elif st in ("plate", "spiked", "ornate", "gothic", "facet"):
        if st == "facet":
            base = poly([(12, 8), (52, 8), (54, 30), (50, 56), (38, 56), (34, 24), (30, 24), (26, 56), (14, 56), (10, 30)])
            k.plate(base, 2, h=np.ones((N, N)))
        else:
            k.plate(base, 5, h=dome(base, 5) + (flutes_tex(32, 70, 15) * 0.6 if st == "gothic" else 0))
        k.plate(belt, 1.4, base=0.55)
        if st in ("plate", "spiked"):
            for sx in (1, -1):
                tas = poly([(32 + sx * 3, 13), (32 + sx * 21, 13), (32 + sx * 22, 24), (32 + sx * 4, 22)])
                h, lip = lames_tex(13, 4)
                k.plate(tas, 1.5, h=h, base=0.5)
                k.dark(lip & tas, 2)
        for cx in (19, 45):
            if st == "gothic":
                kn = poly([(cx - 7, 37), (cx, 30), (cx + 7, 37), (cx, 44)])
            elif st == "facet":
                kn = poly([(cx - 6, 38), (cx, 30), (cx + 6, 38), (cx, 42)])
            else:
                kn = ell(cx, 38, 7, 5)
            k.plate(kn, 2.5, base=0.56, h=np.ones((N, N)) if st == "facet" else None)
            if st == "spiked":
                k.plate(poly([(cx - 2.5, 36), (cx - 4, 27), (cx + 2.5, 36)]), 1, base=0.6, pal=_pal("#3a2e30", 0.5))
            if st == "plate":
                k.rivets([(cx, 38)], 1.3)
        if st == "ornate":
            k.art.dots([(20, 11), (32, 11), (44, 11)], 1.6, RED_GEM)
            k.groove(box(13, 20, 51, 20.8) & base, dark=4, light=13)
        if st == "spiked":
            k.rivets([(16, 11), (48, 11)], 1.3, _pal("#7a5a46", 0.5))
    elif st == "roman":
        cloth = _pal("#a8302a", 0.45)
        k.art.paint(base & (PY < 36), dome(base, 3), cloth, base=0.5)
        for cx in (19, 45):
            gr = box(cx - 7.5, 36, cx + 7.5, 56) & base
            k.plate(gr, 3)
        strips = box(12, 14, 52, 30) & (np.abs(((PX - 12) % 5) - 2.5) < 1.8)
        k.art.paint(strips, dome(strips, 1.2), _pal("#6a4a2a", 0.4), base=0.5)
        k.plate(belt, 1.3, base=0.55)
        k.rivets([(x, 11) for x in range(15, 52, 6)], 1.1)
        patina(k.art, base, 0.28)
    elif st == "brigandine":
        cloth = _pal("#3c4a6a", 0.4, 1.0)
        k.art.paint(base, dome(base, 3), cloth, base=0.5)
        k.plate(belt, 1.3)
        pts = [(x + (y % 8) * 0.5, y) for y in range(18, 32, 4) for x in (14, 18, 22, 42, 46, 50) if base[int(y), int(x)]]
        k.rivets(pts, 1.0)
        for cx in (19, 45):
            k.plate(ell(cx, 39, 7, 5), 2.5, base=0.55)
    elif st == "greek":
        cloth = _pal("#6a4a2a", 0.4)
        k.art.paint(base & (PY < 34), dome(base, 3), cloth, base=0.5)
        strips = box(11, 12, 53, 30) & (np.abs(((PX - 11) % 6) - 3) < 2.4)
        k.art.paint(strips, dome(strips, 1.2), cloth, base=0.42)
        for cx in (19, 45):
            gr = poly([(cx - 8, 32), (cx + 8, 32), (cx + 7, 44), (cx + 8, 56), (cx - 8, 56), (cx - 7, 44)]) & base
            calf = np.exp(-(((PY - 44) / 6) ** 2)) * 1.6
            k.plate(gr, 3, h=dome(gr, 3) + calf)
            k.plate(ell(cx, 35, 6.5, 3.5), 1.5, base=0.6)
        patina(k.art, base, 0.18, "#5a9a78")
    elif st == "banded":
        h, lip = lames_tex(8, 6)
        k.plate(base, 3, h=dome(base, 3) + h * 0.8, extra=(value_noise(k.rng, 8) - 0.5) * 0.1)
        k.dark(lip & base, 2)
        k.rivets([(15, 11), (49, 11), (16, 26), (48, 26), (16, 44), (48, 44)], 1.5)
    elif st == "scale":
        h, edge = scales_tex(base)
        k.plate(base, 2, h=h)
        k.dark(edge & base, 2)
        k.art.paint(belt, np.ones((N, N)), _pal("#5a3a24", 0.4), base=0.5)
    elif st == "samurai":
        cloth = _pal("#2c3046", 0.35)
        k.art.paint(base, dome(base, 3), cloth, base=0.5)
        for sx in (1, -1):
            apron = poly([(32 + sx * 2, 14), (32 + sx * 22, 14), (32 + sx * 26, 36), (32 + sx * 3, 34)])
            h, lip = lames_tex(14, 4.4)
            k.plate(apron, 1.4, h=h)
            k.dark(lip & apron, 3)
        for cx in (19, 45):
            sg = box(cx - 7, 40, cx + 7, 56) & base
            k.plate(sg, 2, h=dome(sg, 2) + np.abs(np.sin((PX - cx) * 1.2)) * 1.2)
        k.plate(belt, 1.2, pal=BLUE_CORD, spec=0.05)
    else:  # volcanic
        jag = poly([(12, 8), (52, 8), (54, 22), (52, 34), (54, 46), (52, 56), (38, 56), (34, 24), (30, 24), (26, 56), (12, 56), (10, 46), (12, 34), (10, 22)])
        k.plate(jag, 4)
        k.plate(belt & jag, 1.2, base=0.42)
    return k.done()


# ---------------------------------------------------------------- boots


def boots_icon(mat):
    st = STYLE.get(mat, "plate")
    k = Kit(f"{mat}_boots", mat)
    for side in (0, 1):
        def P(pts):
            # Points are given for the left boot (toe pointing left); the right one is mirrored.
            return poly(pts if side == 0 else [(64 - x, y) for (x, y) in pts])

        def X(x):
            return x if side == 0 else 64 - x

        if st == "leather":
            b = P([(11, 14), (26, 14), (26, 42), (29, 46), (29, 54), (4, 54), (4, 47), (10, 43)])
            k.plate(b, 3)
            cuff = P([(9, 13), (28, 13), (27, 22), (10, 22)])
            k.plate(cuff, 1.6, base=0.56)
            for y in (28, 33, 38):
                k.art.paint(polyline([(X(15), y), (X(22), y + 2)], 1.0), np.ones((N, N)), CORD, base=0.6, outline=False)
            k.dark(b & (PY >= 51), 2)
        elif st == "mail":
            b = P([(11, 14), (26, 14), (26, 42), (29, 46), (29, 54), (4, 54), (4, 47), (10, 43)])
            h, hole = rings_tex(b)
            k.plate(b, 3, h=dome(b, 3) + h)
            k.cut(hole & (PY < 44))
            sole = b & (PY >= 44)
            k.art.paint(sole, dome(sole, 2), _pal("#5a3a24", 0.4), base=0.5)
        elif st in ("plate", "ornate", "spiked", "gothic", "facet", "banded"):
            if st == "gothic":
                b = P([(12, 14), (26, 14), (26, 44), (30, 50), (30, 54), (0, 54), (2, 51), (11, 44)])
            elif st == "banded":
                b = P([(9, 12), (28, 12), (28, 42), (31, 44), (31, 56), (3, 56), (3, 44), (9, 42)])
            elif st == "facet":
                b = P([(11, 12), (27, 14), (27, 42), (30, 48), (28, 54), (2, 54), (5, 46), (11, 42)])
            else:
                b = P([(11, 14), (26, 14), (26, 42), (29, 46), (29, 54), (3, 54), (3, 47), (10, 43)])
            if st == "facet":
                k.plate(b, 2, h=np.ones((N, N)))
            else:
                h, lip = lames_tex(42, 3.6, tilt=0)
                k.plate(b, 4, h=dome(b, 4) + h * (PY > 42) * 0.8)
                k.dark(lip & b & (PY > 42) & (PY < 53), 3)
            if st == "ornate":
                wing = P([(10, 12), (4, 6), (6, 16), (11, 22)])
                k.plate(wing, 1.5, base=0.6)
                k.groove(b & (np.abs(PY - 20) < 0.5), dark=4, light=13)
                k.art.dots([(X(19), 30)], 1.6, RED_GEM)
            if st == "spiked":
                k.plate(P([(26, 30), (34, 26), (26, 36)]), 1, base=0.6, pal=_pal("#3a2e30", 0.5))
                k.plate(P([(3, 50), (-2, 54), (5, 54)]) & (PX >= 0), 1, base=0.6, pal=_pal("#3a2e30", 0.5))
            if st in ("plate", "banded"):
                k.rivets([(X(14), 18), (X(23), 18)], 1.2)
            if st == "gothic":
                k.groove(b & (np.abs(PX - X(18.5)) < 0.6) & (PY < 42), dark=3, light=None)
        elif st == "roman":
            b = P([(11, 14), (26, 14), (26, 42), (29, 46), (29, 54), (4, 54), (4, 47), (10, 43)])
            strap = _pal("#6a4a2a", 0.4)
            k.art.paint(b, dome(b, 2), strap, base=0.45)
            holes = b & (np.abs((PY % 5) - 2.5) < 1.2) & (PY > 22) & (PY < 50)
            k.cut(holes & (np.abs(PX - X(18.5)) > 2))
            gr = P([(11, 12), (26, 12), (26, 24), (11, 24)])
            k.plate(gr, 2)
            patina(k.art, gr, 0.3)
        elif st == "brigandine":
            b = P([(11, 14), (26, 14), (26, 42), (29, 46), (29, 54), (4, 54), (4, 47), (10, 43)])
            k.art.paint(b, dome(b, 3), _pal("#4a3a2c", 0.4), base=0.5)
            toe = b & (PY > 44)
            k.plate(toe, 2)
            k.art.paint(box(X(10) if side == 0 else X(27), 30, X(27) if side == 0 else X(10), 33) & b, np.ones((N, N)), _pal("#2a2420", 0.3), base=0.4)
            k.rivets([(X(18.5), 31.5)], 1.4)
        elif st == "greek":
            b = P([(11, 14), (26, 14), (26, 42), (29, 46), (29, 54), (4, 54), (4, 47), (10, 43)])
            k.art.paint(b & (PY > 40), dome(b, 2), _pal("#6a4a2a", 0.4), base=0.5)
            gr = P([(11, 12), (26, 12), (25, 30), (26, 40), (11, 40), (12, 30)])
            k.plate(gr, 3, h=dome(gr, 3) + np.exp(-(((PY - 26) / 6) ** 2)) * 1.4)
            patina(k.art, gr, 0.2, "#5a9a78")
        elif st == "scale":
            b = P([(11, 14), (26, 14), (26, 42), (29, 46), (29, 54), (4, 54), (4, 47), (10, 43)])
            h, edge = scales_tex(b, 5, 4)
            k.plate(b, 2, h=h)
            k.dark(edge & b, 2)
            k.art.paint(b & (PY >= 51), np.ones((N, N)), _pal("#5a3a24", 0.4), base=0.4)
        elif st == "samurai":
            b = P([(11, 14), (26, 14), (26, 42), (29, 46), (29, 54), (4, 54), (4, 47), (10, 43)])
            k.art.paint(b, dome(b, 2), _pal("#2c3046", 0.35), base=0.5)
            sg = P([(11, 12), (26, 12), (26, 40), (11, 40)])
            k.plate(sg, 2, h=dome(sg, 2) + np.abs(np.sin(PX * 1.3)) * 1.2)
            k.stitches([(X(11), 26), (X(26), 26)], 2.0, BLUE_CORD)
        else:  # volcanic
            b = P([(11, 12), (16, 16), (21, 12), (27, 16), (26, 42), (30, 46), (29, 54), (2, 54), (5, 46), (10, 42)])
            k.plate(b, 3)
    return k.done()


ARMOR_ICONS = {"helmet": helmet_icon, "chestplate": chest_icon, "leggings": legs_icon, "boots": boots_icon}
