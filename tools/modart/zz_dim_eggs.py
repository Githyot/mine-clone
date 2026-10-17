"""Spawn eggs for the creatures of the Aether and the Otherside: the base game's speckled egg
in each creature's two colours."""

import dim_mobs_spec as spec

from .lib import ITEMS, canvas, put

ROWS = [(6, 9), (5, 10), (4, 11), (4, 11), (3, 12), (3, 12), (3, 12), (2, 13), (2, 13), (2, 13), (2, 13), (3, 12), (3, 12), (4, 11)]
SPOTS = [(6, 4), (9, 6), (5, 8), (10, 10), (7, 11), (4, 11), (11, 7), (8, 8)]


def egg(base, spot):
    img = canvas()
    for i, (a, b) in enumerate(ROWS):
        y = i + 1
        for x in range(a, b + 1):
            edge = x in (a, b) or i in (0, len(ROWS) - 1)
            k = 0.6 if edge else (1.1 if x < 7 and y < 8 else 1.0)
            put(img, x, y, tuple(min(255, int(v * k)) for v in base))
    for x, y in SPOTS:
        put(img, x, y, spot)
        put(img, x + 1, y, spot)
    return img


for _name, (_b, _s) in spec.EGGS.items():
    ITEMS[f"{_name}_spawn_egg"] = (lambda b=_b, s=_s: egg(b, s))
