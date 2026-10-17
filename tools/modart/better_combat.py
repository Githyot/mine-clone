"""Better Combat's two-handed weapons: a broad greatsword, a long spear and a halberd, drawn on
the same diagonal as the base game's tools. Our own 16x16 pixels."""

import numpy as np

from .lib import I, item, canvas, rng_for

U, V = I.U, I.V


def greatsword(material):
    p = I.MATERIALS[material]
    img = canvas()
    # Long grip with a pommel, a wide crossguard and a broad blade to the corner.
    grip = (np.abs(V) <= 0.75) & (U >= 0.4) & (U <= 3.6)
    img[grip & (V > 0), :3] = I.HANDLE[3]
    img[grip & (V <= 0), :3] = I.HANDLE[1]
    img[grip, 3] = 255
    pommel = (np.abs(V) <= 1.2) & (U >= -0.8) & (U < 0.4)
    I.shade_mask(img, pommel, p[:3])
    guard = (np.abs(V) <= 4.2) & (U >= 3.6) & (U <= 5.0)
    I.shade_mask(img, guard, p[:4])
    blade = (np.abs(V) <= 1.9 - np.maximum(0, U - 15.6) * 1.2) & (U > 5.0) & (U <= 17.4)
    I.shade_mask(img, blade, p[1:], None)
    img[blade & (np.abs(V) < 0.45) & (U < 15.8), :3] = p[3]
    return I.outline(img)


def spear(material):
    p = I.MATERIALS[material]
    rng = rng_for(f"{material}_spear")
    img = canvas()
    I.handle(img, -1.5, 10.6)
    # Leaf-shaped point.
    t = (U - 10.4) / 7.4
    head = (U >= 10.4) & (U <= 17.8) & (np.abs(V) <= 2.3 * np.sin(np.clip(t, 0, 1) ** 0.7 * np.pi) + 0.35)
    I.shade_mask(img, head, p[1:], rng)
    img[head & (np.abs(V) < 0.4) & (U < 16.5), :3] = p[3]
    # A binding ring under the point.
    ring = (np.abs(V) <= 1.2) & (U >= 9.4) & (U <= 10.4)
    img[ring, :3] = p[1]
    img[ring, 3] = 255
    return I.outline(img)


def halberd(material):
    p = I.MATERIALS[material]
    rng = rng_for(f"{material}_halberd")
    img = canvas()
    I.handle(img, -1.5, 13.0)
    # Axe blade on one side, a hook on the other, a spike on top.
    blade = (V > 0.6) & (V <= 5.2) & (U >= 9.0 - 0.45 * (V - 0.6)) & (U <= 12.6 + 0.45 * (V - 0.6))
    hook = (V < -0.6) & (V >= -2.6) & (np.abs(U - 11.0 - 0.6 * (-V - 0.6)) <= 0.7)
    spike = (np.abs(V) <= 0.9 - np.maximum(0, U - 15.0) * 0.4) & (U > 12.4) & (U <= 17.0)
    I.shade_mask(img, blade | hook | spike, p[1:], rng)
    img[blade & (V > 4.2), :3] = p[4]
    return I.outline(img)


for _m in ("stone", "iron", "golden", "diamond"):
    item(f"{_m}_greatsword")(lambda m=_m: greatsword(m))
    item(f"{_m}_spear")(lambda m=_m: spear(m))
    item(f"{_m}_halberd")(lambda m=_m: halberd(m))
