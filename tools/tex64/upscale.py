"""16x16 design -> 64x64 texture, for everything that has no hand-made 64x64 generator yet.

New blocks and items keep being added as 16x16 pixel designs (tools/gen_*.py). This turns any of
them into a 64x64 texture in the shared style, so the game always has one consistent resolution:

1. Scale4x (EPX applied twice): pixel art grows 4x with smooth diagonals instead of staircases.
2. Thin outlines: a sprite's 1-pixel dark outline would become 4 pixels thick; it is trimmed
   back to 2 pixels and the inside is filled with the neighbouring colour.
3. Light and grain: the 16x16 picture's brightness becomes a soft height field lit by the same
   top-left light as the hand-made textures, plus a fine 2-pixel grain, so a former pixel is no
   longer a flat 4x4 square.
4. Soft contrast, like every other texture (style.soften).
"""

import numpy as np

from style import blur, clusters, from_oklab, light, rng_for, soften, to_oklab


def epx(img):
    """Scale2x / EPX on an RGBA uint-like float array, comparing whole pixels."""
    h, w = img.shape[:2]
    P = img
    pad = np.pad(P, ((1, 1), (1, 1), (0, 0)), mode="edge")
    A = pad[:-2, 1:-1]  # up
    B = pad[1:-1, 2:]   # right
    C = pad[1:-1, :-2]  # left
    D = pad[2:, 1:-1]   # down

    def eq(a, b):
        return np.all(np.abs(a - b) < 1, axis=-1)

    ca, ab, bd, dc = eq(C, A), eq(A, B), eq(B, D), eq(D, C)
    ad, bc = eq(A, D), eq(B, C)
    ok = ~ad & ~bc
    out = np.zeros((h * 2, w * 2, 4))
    o1 = np.where((ca & ok)[..., None], A, P)
    o2 = np.where((ab & ok)[..., None], B, P)
    o3 = np.where((dc & ok)[..., None], C, P)
    o4 = np.where((bd & ok)[..., None], D, P)
    out[0::2, 0::2] = o1
    out[0::2, 1::2] = o2
    out[1::2, 0::2] = o3
    out[1::2, 1::2] = o4
    return out


def nearest(img, k):
    return np.repeat(np.repeat(img, k, 0), k, 1)


def bilinear_periodic(f, k):
    """Smooth k-times upsampling of a periodic field."""
    h, w = f.shape
    ys = (np.arange(h * k) + 0.5) / k - 0.5
    xs = (np.arange(w * k) + 0.5) / k - 0.5
    y0 = np.floor(ys).astype(int)
    x0 = np.floor(xs).astype(int)
    ty = (ys - y0)[:, None]
    tx = (xs - x0)[None, :]
    a = f[np.ix_(y0 % h, x0 % w)]
    b = f[np.ix_(y0 % h, (x0 + 1) % w)]
    c = f[np.ix_((y0 + 1) % h, x0 % w)]
    d = f[np.ix_((y0 + 1) % h, (x0 + 1) % w)]
    return (a * (1 - tx) + b * tx) * (1 - ty) + (c * (1 - tx) + d * tx) * ty


def trim_outlines(src, big, k=4, keep=2):
    """Thin a sprite's outline: outline pixels deeper than `keep` take the inner colour."""
    a = src[..., 3] > 127
    if a.all() or not a.any():
        return big
    L = to_oklab(src[..., :3])[..., 0]
    pad = np.pad(a, 1, constant_values=False)
    touches = ~(pad[:-2, 1:-1] & pad[2:, 1:-1] & pad[1:-1, :-2] & pad[1:-1, 2:])
    edge = a & touches
    inner = a & ~edge
    if not inner.any():
        return big
    # Outline = edge pixels clearly darker than the inside they wrap.
    near_in = blur(np.where(inner, L, 0.0), 1, wrap=False) / np.maximum(blur(inner.astype(float), 1, wrap=False), 1e-6)
    outline = edge & (L < near_in - 0.12)
    O = nearest(outline, k)
    A = big[..., 3] > 127
    # Distance (hi-res px) from transparency, by repeated erosion.
    d = np.zeros(A.shape)
    cur = A.copy()
    for i in range(1, keep + 1):
        p = np.pad(cur, 1, constant_values=False)
        er = p[:-2, 1:-1] & p[2:, 1:-1] & p[1:-1, :-2] & p[1:-1, 2:] & cur
        d[cur & ~er] = i
        cur = er
    d[cur] = keep + 1
    fill = O & (d > keep)
    if not fill.any():
        return big
    out = big.copy()
    known = A & ~fill
    rgb = out[..., :3].copy()
    todo = fill.copy()
    for _ in range(k * 2):
        if not todo.any():
            break
        acc = np.zeros(rgb.shape)
        cnt = np.zeros(A.shape)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            kn = np.roll(np.roll(known, dy, 0), dx, 1)
            acc += np.roll(np.roll(rgb, dy, 0), dx, 1) * kn[..., None]
            cnt += kn
        upd = todo & (cnt > 0)
        rgb[upd] = acc[upd] / cnt[upd][:, None]
        known = known | upd
        todo = todo & ~upd
    out[..., :3] = rgb
    return out


def upscale(src, name, kind="block"):
    """src: (16, 16, 4) float RGBA -> (64, 64, 4)."""
    k = 64 // src.shape[1]
    src = src.astype(np.float64)
    big = src
    s = k
    while s > 1:
        big = epx(big)
        s //= 2
    # See-through designs (stained glass, ice, slime, clouds) keep their part-transparent
    # fill; only fully clear pixels make a cut-out sprite.
    sprite = (src[..., 3] < 16).any()
    if sprite:
        big = trim_outlines(src, big, k)
    rng = rng_for("up:" + name)
    lab = to_oklab(big[..., :3])
    # Soft relief from the design's own brightness, lit like everything else.
    Ls = to_oklab(src[..., :3])[..., 0]
    hf = bilinear_periodic(Ls, k) * (k * 2.2)
    shade_amt = 0.045 if not sprite else 0.035
    lab[..., 0] += light(hf, wrap=not sprite) * shade_amt * 3
    grain = clusters(rng, 2) - 0.5
    lab[..., 0] += grain * (0.035 if kind == "block" and not sprite else 0.02)
    out = big.copy()
    out[..., :3] = from_oklab(lab)
    out[..., 3] = np.where(big[..., 3] >= 16, big[..., 3], 0)
    if kind == "block" and not sprite:
        out = soften(out, "rough")
    return out
