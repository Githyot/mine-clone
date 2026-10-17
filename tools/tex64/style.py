"""The style guide of our 64x64 textures, written as code.

Every texture in the game is made from the same few ideas, so the world looks like one place:

* One light. The key light comes from the top-left (like the sun over your left shoulder);
  every bump, board, brick and stone is lit by it the same way. Crevices get ambient occlusion.
* Material = colour ramp + albedo field + height field. The height field is lit, the albedo
  field picks lighter or darker paint, and the sum is quantised onto the material's ramp.
  Quantising keeps the hand-painted pixel look; at 64x64 a ramp has 8-12 steps.
* Ramps shift hue: shadows lean cool (towards blue-violet), lights lean warm (towards yellow),
  like painted pixel art, instead of plain darker/lighter copies of one colour.
* Soft contrast (HP's request, bug 004): natural blocks keep their luminance spread small, so a
  field of grass or sand reads as one colour with gentle variation, never as a checkerboard.
  `CONTRAST` lists the limits and `soften()` enforces them.
* No single-pixel salt noise: variation comes in clusters of 2-6 pixels at 64x64.
* Everything that repeats in the world tiles seamlessly (all noise is periodic).

The helpers here are numpy only; each generator gets its own seeded RNG so output is stable.
"""

import zlib

import numpy as np

N = 64
BASE_SEED = 0x64_7E_C5
YY, XX = np.mgrid[0:N, 0:N].astype(np.float64)

# Light from the top-left, a bit above the surface.
LIGHT = np.array([-0.55, -0.55, 0.63])
LIGHT = LIGHT / np.linalg.norm(LIGHT)

# Luminance limits (OKLab L, 0..1) for the spread of a texture's pixels around its mean:
# (standard deviation, 2nd..98th percentile range). Measured on what reads as "soft" in game.
CONTRAST = {
    "natural": (0.045, 0.19),   # grass, sand, dirt, stone, snow, leaves...
    "rough": (0.06, 0.26),      # cobblestone, gravel, bark, ores' stone
    "built": (0.075, 0.32),     # planks, bricks, crafted blocks with seams
    "free": (1.0, 1.0),         # sprites, items, UI: no limit
}


def rng_for(name):
    return np.random.default_rng(BASE_SEED ^ zlib.crc32(name.encode()))


# ---------------------------------------------------------------- colour


def hexrgb(h):
    h = h.lstrip("#")
    return np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], dtype=np.float64)


def _lin(c):
    c = np.asarray(c, dtype=np.float64) / 255.0
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def _srgb(c):
    c = np.clip(c, 0, 1)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055) * 255.0


def to_oklab(rgb):
    c = _lin(rgb)
    l = 0.4122214708 * c[..., 0] + 0.5363325363 * c[..., 1] + 0.0514459929 * c[..., 2]
    m = 0.2119034982 * c[..., 0] + 0.6806995451 * c[..., 1] + 0.1073969566 * c[..., 2]
    s = 0.0883024619 * c[..., 0] + 0.2817188376 * c[..., 1] + 0.6299787005 * c[..., 2]
    l, m, s = np.cbrt(l), np.cbrt(m), np.cbrt(s)
    return np.stack([
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ], -1)


def from_oklab(lab):
    L, a, b = lab[..., 0], lab[..., 1], lab[..., 2]
    l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3
    m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3
    s = (L - 0.0894841775 * a - 1.2914855480 * b) ** 3
    r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s
    g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s
    bb = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
    return _srgb(np.stack([r, g, bb], -1))


def luma(rgb):
    return to_oklab(rgb)[..., 0]


def ramp(base, n=10, spread=0.30, warm=0.10, sat=1.0, lift=0.0):
    """A hue-shifted colour ramp, dark -> light, centred on `base` (hex or rgb).

    spread: OKLab lightness covered by the ramp. warm: how far lights lean to yellow and
    shadows to blue-violet (0 = plain shades). sat: chroma multiplier."""
    rgb = hexrgb(base) if isinstance(base, str) else np.asarray(base, dtype=np.float64)
    L0, a0, b0 = to_oklab(rgb)
    C0 = np.hypot(a0, b0) * sat
    h0 = np.arctan2(b0, a0)
    out = []
    for i in range(n):
        t = i / (n - 1) - 0.5  # -0.5 .. 0.5
        L = np.clip(L0 + lift + t * spread, 0.02, 0.995)
        target = np.deg2rad(95.0) if t > 0 else np.deg2rad(285.0)  # warm yellow / cool violet
        d = np.angle(np.exp(1j * (target - h0)))
        h = h0 + d * warm * abs(t) * 2
        # Very dark and very light paint carries less colour.
        C = C0 * (1.0 - 0.55 * abs(t) ** 1.6 * 2)
        # Neutral greys still get the faint temperature shift.
        C = C + 0.012 * warm * 10 * abs(t)
        out.append(from_oklab(np.array([L, C * np.cos(h), C * np.sin(h)])))
    return np.array(out)


def mix(a, b, t):
    return np.asarray(a, float) * (1 - t) + np.asarray(b, float) * t


# ---------------------------------------------------------------- noise (all tileable)


def value_noise(rng, cell, n=N):
    """Smooth periodic value noise with square cells of `cell` pixels (a divisor of n)."""
    g = n // cell
    grid = rng.random((g, g))
    f = (np.arange(n) + 0.5) / cell - 0.5
    i0 = np.floor(f).astype(int)
    t = f - i0
    t = t * t * (3 - 2 * t)
    i0m, i1m = i0 % g, (i0 + 1) % g
    a = grid[np.ix_(i0m, i0m)]
    b = grid[np.ix_(i0m, i1m)]
    c = grid[np.ix_(i1m, i0m)]
    d = grid[np.ix_(i1m, i1m)]
    top = a + (b - a) * t[None, :]
    bot = c + (d - c) * t[None, :]
    return top + (bot - top) * t[:, None]


def fbm(rng, octaves=((32, 0.35), (16, 0.3), (8, 0.22), (4, 0.15), (2, 0.08))):
    return normalize(sum(w * value_noise(rng, c) for c, w in octaves))


def normalize(f):
    lo, hi = f.min(), f.max()
    return (f - lo) / (hi - lo + 1e-9)


def stretched_noise(rng, cx, cy):
    """Value noise with different cell sizes along x and y (grain, fibres, streaks)."""
    gx, gy = N // cx, N // cy
    grid = rng.random((gy, gx))
    fx = (np.arange(N) + 0.5) / cx - 0.5
    fy = (np.arange(N) + 0.5) / cy - 0.5
    x0 = np.floor(fx).astype(int)
    y0 = np.floor(fy).astype(int)
    tx, ty = fx - x0, fy - y0
    tx = tx * tx * (3 - 2 * tx)
    ty = ty * ty * (3 - 2 * ty)
    a = grid[np.ix_(y0 % gy, x0 % gx)]
    b = grid[np.ix_(y0 % gy, (x0 + 1) % gx)]
    c = grid[np.ix_((y0 + 1) % gy, x0 % gx)]
    d = grid[np.ix_((y0 + 1) % gy, (x0 + 1) % gx)]
    top = a + (b - a) * tx[None, :]
    bot = c + (d - c) * tx[None, :]
    return top + (bot - top) * ty[:, None]


def clusters(rng, cell=4, density=0.5):
    """Blobs of 2-6 pixels: the soft speckle that replaces single-pixel noise."""
    v = value_noise(rng, cell) * 0.7 + value_noise(rng, max(2, cell // 2)) * 0.3
    return normalize(v)


def worley(rng, count, min_dist, n=N, jitter_shape=0.0):
    """Tileable Voronoi cells: (cell id, distance to nearest, distance to 2nd nearest, points)."""
    pts = []
    tries = 0
    while len(pts) < count and tries < count * 400:
        tries += 1
        p = rng.random(2) * n
        if all(torus_dist(p, q, n) >= min_dist for q in pts):
            pts.append(p)
    pts = np.array(pts)
    k = len(pts)
    # Squash/stretch each cell a little so stones aren't perfect polygons.
    sx = 1.0 + (rng.random(k) - 0.5) * jitter_shape
    d = np.empty((k, n, n))
    for i, (px, py) in enumerate(pts):
        dx = np.abs(XX[:n, :n] - px)
        dy = np.abs(YY[:n, :n] - py)
        dx = np.minimum(dx, n - dx) * sx[i]
        dy = np.minimum(dy, n - dy) / sx[i]
        d[i] = np.hypot(dx, dy)
    order = np.argsort(d, axis=0)
    ids = order[0]
    d1 = np.take_along_axis(d, order[:1], 0)[0]
    d2 = np.take_along_axis(d, order[1:2], 0)[0]
    return ids, d1, d2, pts


def torus_dist(p, q, n=N):
    d = np.abs(np.asarray(p) - np.asarray(q))
    d = np.minimum(d, n - d)
    return float(np.hypot(*d))


def blur(f, r=1, wrap=True):
    """Box blur of radius r (repeated twice ~ tent), periodic edges."""
    out = f.astype(np.float64)
    for _ in range(2):
        acc = np.zeros_like(out)
        for dy in range(-r, r + 1):
            for dx in range(-r, r + 1):
                acc += np.roll(np.roll(out, dy, 0), dx, 1) if wrap else _shift(out, dx, dy)
        out = acc / (2 * r + 1) ** 2
    return out


def _shift(f, dx, dy):
    out = np.roll(np.roll(f, dy, 0), dx, 1)
    if dy > 0:
        out[:dy] = f[:dy]
    elif dy < 0:
        out[dy:] = f[dy:]
    if dx > 0:
        out[:, :dx] = f[:, :dx]
    elif dx < 0:
        out[:, dx:] = f[:, dx:]
    return out


# ---------------------------------------------------------------- light


def light(height, strength=1.0, wrap=True):
    """Diffuse light of a height field (in pixels of height) from the top-left key light.
    Returns a value around 0: positive faces the light, negative faces away."""
    if wrap:
        gx = (np.roll(height, -1, 1) - np.roll(height, 1, 1)) * 0.5
        gy = (np.roll(height, -1, 0) - np.roll(height, 1, 0)) * 0.5
    else:
        gx = np.gradient(height, axis=1)
        gy = np.gradient(height, axis=0)
    nx, ny, nz = -gx, -gy, np.ones_like(height)
    ln = np.sqrt(nx * nx + ny * ny + 1)
    d = (nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2]) / ln
    return (d - LIGHT[2]) * strength


def occlusion(height, r=2, wrap=True):
    """Ambient occlusion: how far a pixel sits below its surroundings (>= 0)."""
    return np.maximum(blur(height, r, wrap) - height, 0)


def quantize(t, pal, dither=0.0, rng=None):
    """Map a 0..1 field onto a palette. A little noise dither softens band edges without
    the checkerboard of ordered dithering."""
    n = len(pal)
    x = t * (n - 1)
    if dither and rng is not None:
        x = x + (rng.random(t.shape) - 0.5) * dither
    idx = np.clip(np.round(x), 0, n - 1).astype(int)
    return pal[idx]


def canvas(n=N, m=None):
    return np.zeros((m or n, n, 4), dtype=np.float64)


def rgba(rgb, alpha=255.0):
    img = np.zeros(rgb.shape[:2] + (4,))
    img[..., :3] = rgb
    img[..., 3] = alpha
    return img


def paint(t, pal, rng=None, dither=0.35, alpha=255.0):
    """The usual last step: field -> ramp colours -> RGBA."""
    return rgba(quantize(np.clip(t, 0, 1), pal, dither, rng), alpha)


def soften(img, kind="natural"):
    """Pull pixel lightness towards the mean until the texture is within the CONTRAST limits.
    Keeps hue and chroma; only opaque pixels count."""
    sd_max, range_max = CONTRAST[kind]
    a = img[..., 3] > 127
    if a.sum() < 8:
        return img
    lab = to_oklab(img[..., :3])
    L = lab[..., 0]
    vals = L[a]
    mean = vals.mean()
    sd = vals.std()
    rng_ = np.percentile(vals, 98) - np.percentile(vals, 2)
    k = min(1.0, sd_max / (sd + 1e-9), range_max / (rng_ + 1e-9))
    if k >= 0.999:
        return img
    lab[..., 0] = mean + (L - mean) * k
    out = img.copy()
    out[..., :3] = np.where(a[..., None], from_oklab(lab), img[..., :3])
    return out


def stats(img):
    a = img[..., 3] > 127
    L = luma(img[..., :3])[a]
    return float(L.mean()), float(L.std()), float(np.percentile(L, 98) - np.percentile(L, 2))


def to_u8(img):
    return np.clip(np.round(img), 0, 255).astype(np.uint8)


# ---------------------------------------------------------------- drawing helpers


def disc(cx, cy, r, n=N):
    return (XX[:n, :n] - cx) ** 2 + (YY[:n, :n] - cy) ** 2 <= r * r


def rect_mask(x0, y0, x1, y1, n=N):
    return (XX[:n, :n] >= x0) & (XX[:n, :n] < x1) & (YY[:n, :n] >= y0) & (YY[:n, :n] < y1)


def segment_dist(x0, y0, x1, y1, n=N):
    """Distance of every pixel centre to a segment."""
    px, py = XX[:n, :n] + 0.5, YY[:n, :n] + 0.5
    vx, vy = x1 - x0, y1 - y0
    L2 = vx * vx + vy * vy + 1e-9
    t = np.clip(((px - x0) * vx + (py - y0) * vy) / L2, 0, 1)
    return np.hypot(px - (x0 + t * vx), py - (y0 + t * vy))


def edges_of(mask):
    """Pixels of mask that touch a pixel outside it (4-neighbour, no wrap)."""
    m = mask
    pad = np.pad(m, 1, constant_values=False)
    inner = pad[1:-1, :-2] & pad[1:-1, 2:] & pad[:-2, 1:-1] & pad[2:, 1:-1]
    return m & ~inner


def dist_inside(mask, maxd=8):
    """Approximate distance (in pixels) from each mask pixel to the mask's edge, no wrap."""
    d = np.zeros(mask.shape)
    cur = mask.copy()
    for i in range(1, maxd + 1):
        e = edges_of(cur)
        d[e] = i
        cur = cur & ~e
        if not cur.any():
            break
    d[cur] = maxd
    return d
