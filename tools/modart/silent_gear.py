"""Silent Gear art: molds, tool heads and every finished tool, drawn from the base game's tool
shapes with the rod and binding recoloured, so each combination shows its parts. Our own
16x16 pixels."""

import numpy as np

from .lib import I, ITEMS, N, canvas, hexrgb, outline, pal

I.MATERIALS.setdefault("flint", pal("#1e1e22", "#34343a", "#4a4a52", "#64646e", "#82828c"))
MAT = {"flint": "flint", "copper": "copper", "iron": "iron", "gold": "golden", "diamond": "diamond"}
RODS = {
    "stick": None,
    "bone": pal("#a8a490", "#c8c4ae", "#dedac6", "#f0ecdc"),
    "blaze_rod": pal("#a8500a", "#e08a12", "#f4b82a", "#ffe070"),
}
BINDINGS = {"string": (hexrgb("#f0ece0"), hexrgb("#b0a890")), "leather": (hexrgb("#a8622e"), hexrgb("#5a2e12"))}
# Where along the handle the binding wraps, per kind (U from the bottom end).
BIND_U = {"pickaxe": (7.6, 10.6), "axe": (6.6, 9.6), "shovel": (6.4, 9.0), "sword": (1.0, 3.6)}
HANDLE = [tuple(int(v) for v in c) for c in I.HANDLE]


def handle_mask(img):
    rgb = img[..., :3].astype(int)
    m = np.zeros((N, N), bool)
    for c in HANDLE:
        m |= (rgb == np.array(c)).all(axis=2) & (img[..., 3] > 0)
    return m


def tool(kind, mat, rod, binding):
    img = I.tool(MAT[mat], kind)
    m = handle_mask(img)
    if RODS[rod] is not None:
        p = RODS[rod]
        rgb = img[..., :3]
        for i, c in enumerate(HANDLE):
            sel = m & (rgb.astype(int) == np.array(c)).all(axis=2)
            rgb[sel] = p[min(i, len(p) - 1)]
    u0, u1 = BIND_U[kind]
    band = (np.abs(I.V) <= 1.3) & (I.U >= u0) & (I.U <= u1) & (img[..., 3] > 0)
    light, dark = BINDINGS[binding]
    stripe = ((np.arange(N)[None, :] + np.arange(N)[:, None]) % 2 == 0)
    img[band & stripe, :3] = light
    img[band & ~stripe, :3] = dark
    return img


def head(kind, mat):
    img = I.tool(MAT[mat], kind)
    m = handle_mask(img)
    # Drop the handle and its dark outline next to it.
    grow = m.copy()
    for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        grow |= np.roll(np.roll(m, dy, 0), dx, 1)
    lum = img[..., :3].mean(axis=2)
    img[grow & (lum < 70)] = 0
    img[m] = 0
    if kind == "sword":
        img[(I.U < 4.2)] = 0
    return outline(img)


def mold(kind):
    img = canvas()
    img[2:14, 2:14, :3] = hexrgb("#2c5a9a")
    img[2:14, 2:14, 3] = 255
    shape = I.tool("wooden", kind)[..., 3] > 0
    small = np.zeros((N, N), bool)
    small[3:13, 3:13] = shape[np.arange(0, 16, 1.6).astype(int)][:, np.arange(0, 16, 1.6).astype(int)]
    img[small, :3] = hexrgb("#dce8f8")
    img[2, 2:14, :3] = img[13, 2:14, :3] = img[2:14, 2, :3] = img[2:14, 13, :3] = hexrgb("#1a3a6a")
    return img


for _k in ("pickaxe", "axe", "shovel", "sword"):
    ITEMS[f"gear_{_k}_mold"] = lambda k=_k: mold(k)
    for _m in MAT:
        ITEMS[f"gear_{_m}_{_k}_head"] = lambda k=_k, m=_m: head(k, m)
        for _r in RODS:
            for _b in BINDINGS:
                ITEMS[f"gear_{_k}_{_m}_{_r}_{_b}"] = lambda k=_k, m=_m, r=_r, b=_b: tool(k, m, r, b)
