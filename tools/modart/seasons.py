"""Serene Seasons: season sensor and calendar art."""

import numpy as np

from .lib import BLOCKS, N, T, block, canvas, from_map, from_rgb, hexrgb, item, outline, pal, put, rng_for, shade, value_noise

SEASON_COLORS = {
    "spring": pal("#2f6a2a", "#4f9a3a", "#86c860", "#c4ec9a"),
    "summer": pal("#8a5a0a", "#d49a1a", "#f4cc3a", "#fff08a"),
    "autumn": pal("#6a240e", "#b4501a", "#e0802a", "#f8b45a"),
    "winter": pal("#3a5a7a", "#6a92b8", "#a8c8e4", "#eef6ff"),
}

SENSOR_FRAME = pal("#3a2a18", "#54402a", "#6c5438", "#846a48")


def sensor_top(season, on):
    rng = rng_for("season_sensor_" + season)
    img = from_rgb(shade(value_noise(rng, 4) * 0.6 + 0.2, SENSOR_FRAME, 0.4))
    p = SEASON_COLORS[season]
    # A round dial of glass in a wooden frame, the season's colour inside.
    for y in range(N):
        for x in range(N):
            d = np.hypot(x - 7.5, y - 7.5)
            if d < 5.6:
                k = 3 if (x + y) % 5 == 0 else (2 if d < 3.2 else 1)
                if not on:
                    k = max(0, k - 1)
                put(img, x, y, p[k])
            elif d < 6.6:
                put(img, x, y, SENSOR_FRAME[0])
    # The emblem: a leaf (spring), sun (summer), falling leaf (autumn), flake (winter).
    em = {
        "spring": ["...", ".#.", "###", ".#."],
        "summer": ["#.#", ".#.", "#.#"],
        "autumn": ["##.", "###", ".##"],
        "winter": ["#.#", ".#.", "#.#", ".#."],
    }[season]
    c = (255, 255, 230) if on else tuple(int(v) for v in p[3])
    for j, row in enumerate(em):
        for i, ch in enumerate(row):
            if ch == "#":
                put(img, 6 + i, 6 + j, c)
    return img


def sensor_side():
    rng = rng_for("season_sensor_side")
    img = from_rgb(shade(value_noise(rng, 8, 2) * 0.6 + 0.2, SENSOR_FRAME, 0.4))
    rgb = img[..., :3]
    rgb[0:2, :] = T.scale_rgb(rgb[0:2, :], 1.15)
    rgb[5, :] = SENSOR_FRAME[0]
    return img


for _s in SEASON_COLORS:
    BLOCKS[f"season_sensor_{_s}"] = (lambda s=_s: sensor_top(s, False))
    BLOCKS[f"season_sensor_{_s}_on"] = (lambda s=_s: sensor_top(s, True))
BLOCKS["season_sensor_side"] = sensor_side


@item("calendar")
def calendar():
    rows = [
        "................",
        "..1..1..1..1....",
        ".2222222222222..",
        ".2333333333332..",
        ".2333333333332..",
        ".2444444444442..",
        ".2454545454542..",
        ".2444444444442..",
        ".2454565454542..",
        ".2444444444442..",
        ".2454545454542..",
        ".2444444444442..",
        ".2454545444442..",
        ".2444444444442..",
        ".2222222222222..",
        "................",
    ]
    cols = {
        "1": hexrgb("#5a5a5a"),
        "2": hexrgb("#6a4a2a"),
        "3": hexrgb("#b0302a"),
        "4": hexrgb("#efe6d0"),
        "5": hexrgb("#8a7a64"),
        "6": hexrgb("#c03a2a"),
    }
    return outline(from_map(rows, cols))


