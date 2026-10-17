#!/usr/bin/env python3
"""Skins of the Etapa 5 creatures (packs/vanilla/etapa5/mobs.ron), drawn from scratch like
tools/gen_mob_skins.py and with its machinery: worms, eyes and skulls get painters of their own,
the rest are recoloured cousins of base mobs (goblins of the player, the World Breaker and the
Vine Queen of the Forest Guardian, the Herald of the skeleton, the blood zombie of the zombie).

Usage: python3 tools/gen_etapa5_skins.py
"""

import os
import sys

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_mob_skins as g  # noqa: E402
from gen_mob_skins import mix, mul, pal, pick  # noqa: E402

MOBS_RON = os.path.join(g.ROOT, "packs", "vanilla", "etapa5", "mobs.ron")

# ---------------------------------------------------------------------------------------------
# Worms

DEEP = dict(body=pal("#2e1a24", "#3e2430", "#4e2e3c", "#603a4a", "#724858"),
            plate=pal("#4a3a34", "#5e4a42", "#725c52", "#866e62"),
            mouth=(24, 8, 12), teeth=(214, 206, 180), eye=(220, 70, 40))
MECH = dict(body=pal("#2a2c32", "#383b42", "#484c54", "#5a5e68", "#6e737e"),
            plate=pal("#5a5e68", "#767b86", "#9298a4", "#b0b6c2"),
            mouth=(12, 12, 16), teeth=(200, 206, 214), eye=(255, 50, 40))


def worm_painter(p):
    def paint(c):
        if c.role in ("mandible", "spike", "stinger"):
            return pick(p["plate"], 0.3 + 0.5 * c.jit + (0.2 if c.face == "top" else 0.0))
        # Rings around the body: darker grooves every few pixels.
        groove = (c.z if c.face in ("left", "right", "top", "bottom") else c.y)
        base = pick(p["body"], 0.25 + 0.5 * c.jit)
        if int(groove) % 4 == 0:
            base = mul(base, 0.75)
        if c.face == "top":
            base = mix(base, pick(p["plate"], c.jit), 0.4)
        if c.face == "bottom":
            base = mul(base, 0.8)
        if c.role == "head" and c.face == "front":
            cx, cy = c.i - (c.fw - 1) / 2, c.j - (c.fh - 1) / 2
            r = (cx * cx / (c.fw * c.fw / 9) + cy * cy / (c.fh * c.fh / 9))
            if r < 1.0:
                return p["mouth"]
            if r < 1.35 and (c.i + c.j) % 2 == 0:
                return p["teeth"]
            if c.j <= 2 and c.i in (1, 2, c.fw - 3, c.fw - 2):
                return p["eye"]
        return base
    return paint


def worm_overlay(p):
    def overlay(c):
        # Armour plates on the back.
        if c.face == "top" and c.jit > 0.35:
            return pick(p["plate"], c.jit)
        if c.face in ("left", "right") and c.j < 2 and c.jit > 0.55:
            return pick(p["plate"], 0.5 * c.jit)
        return None
    return overlay


# ---------------------------------------------------------------------------------------------
# Eyes

def eye_painter(white, vein, iris, metal=None):
    def paint(c):
        if c.role == "stalk":
            return pick(vein, 0.3 + 0.5 * c.jit)
        if c.face == "front":
            cx, cy = c.i - 7.5, c.j - 7.5
            r2 = cx * cx + cy * cy
            if r2 < 4.5:
                return (8, 6, 8)
            if r2 < 22:
                return pick(iris, 0.9 - r2 / 30 + 0.2 * c.jit)
            if r2 < 28:
                return mul(pick(iris, 0.2), 0.6)
        base = pick(white, 0.4 + 0.5 * c.jit)
        if metal is not None and (c.i in (0, c.fw - 1) or c.j in (0, c.fh - 1) or c.j == c.fh // 2):
            return pick(metal, 0.3 + 0.5 * c.jit)
        if c.face == "back":
            return pick(vein, 0.4 + 0.4 * c.jit)
        if c.jit > 0.86:
            return pick(vein, 0.7)
        return base
    return paint


def eye_overlay(vein, metal=None):
    def overlay(c):
        if c.face == "front":
            return None
        if metal is not None and c.jit > 0.7:
            return pick(metal, c.jit)
        if c.face == "back" and c.jit > 0.5:
            return pick(vein, c.jit)
        return None
    return overlay


# ---------------------------------------------------------------------------------------------
# Skulls

def skull_painter(bone, socket, glow):
    def paint(c):
        base = pick(bone, 0.3 + 0.5 * c.jit)
        if c.role == "hand":
            if c.face in ("front", "back") and c.i % 2 == 1 and c.j > c.fh - 4:
                return mul(base, 0.6)
            return base
        if c.role == "jaw":
            if c.face == "front" and c.j < 2 and c.i % 2 == 0:
                return mul(base, 0.55)
            return mul(base, 0.92)
        if c.face == "front":
            # Two eye sockets with a glow in each, a nose hole below.
            for ex in (3, 10):
                if ex <= c.i <= ex + 2 and 4 <= c.j <= 7:
                    return glow if (c.i == ex + 1 and c.j in (5, 6)) else socket
            if 7 <= c.i <= 8 and 9 <= c.j <= 10:
                return socket
        if c.face == "top" and c.jit > 0.9:
            return mul(base, 0.7)
        return base
    return paint


def skull_overlay(bone, crown):
    def overlay(c):
        # A crown of points around the top of the skull.
        if c.role == "skull" and c.face in ("front", "back", "left", "right") and c.j <= 1 and c.i % 3 != 1:
            return pick(crown, 0.4 + 0.5 * c.jit)
        if c.role == "hand" and c.face == "top" and c.jit > 0.5:
            return pick(bone, 0.8)
        return None
    return overlay


# ---------------------------------------------------------------------------------------------

NEW = {
    "deep_worm": (["head", "mandible", "mandible"], worm_painter(DEEP), worm_overlay(DEEP)),
    "deep_worm_body": (["segment", "spike"], worm_painter(DEEP), worm_overlay(DEEP)),
    "deep_worm_tail": (["segment", "stinger"], worm_painter(DEEP), worm_overlay(DEEP)),
    "mech_worm": (["head", "mandible", "mandible"], worm_painter(MECH), worm_overlay(MECH)),
    "mech_worm_body": (["segment", "spike"], worm_painter(MECH), worm_overlay(MECH)),
    "mech_worm_tail": (["segment", "stinger"], worm_painter(MECH), worm_overlay(MECH)),
    "demon_eye": (["eye", "stalk", "stalk", "stalk"],
                  eye_painter(pal("#c8c0b8", "#d8d0c8", "#e8e2da"), pal("#6a0a12", "#8a1420", "#a8202c"), pal("#1a4a8a", "#2a6ab0", "#4a8ad0")),
                  eye_overlay(pal("#6a0a12", "#8a1420", "#a8202c"))),
    "mech_eye": (["eye", "stalk", "stalk", "stalk"],
                 eye_painter(pal("#a8acb4", "#bcc0c8", "#d0d4dc"), pal("#2a2c32", "#383b42", "#484c54"), pal("#8a0a10", "#c41420", "#ff4a3a"), metal=pal("#383b42", "#5a5e68", "#767b86")),
                 eye_overlay(pal("#2a2c32", "#383b42", "#484c54"), metal=pal("#484c54", "#5a5e68", "#767b86"))),
    "bone_king": (["skull", "jaw", "hand", "hand"],
                  skull_painter(pal("#8a877d", "#a8a59a", "#c2bfb3", "#d8d5c9", "#e8e6dc"), (18, 14, 22), (150, 80, 220)),
                  skull_overlay(pal("#8a877d", "#c2bfb3", "#e8e6dc"), pal("#8a6a10", "#c08a12", "#eab92a"))),
    "mech_skull": (["skull", "jaw", "hand", "hand"],
                   skull_painter(pal("#3a3e48", "#4c515c", "#646a78", "#7e8492", "#9aa0ae"), (10, 10, 14), (255, 60, 50)),
                   skull_overlay(pal("#4c515c", "#7e8492", "#b0b6c2"), pal("#2a2c32", "#5a5e68", "#9298a4"))),
}

RECOLOR = {
    "world_breaker": ("forest_guardian", dict(
        GBARK=pal("#141012", "#1e1818", "#282020", "#322828", "#3c3030"),
        GLEAF=pal("#7a2a08", "#a8400c", "#d05a12", "#ec7a1a", "#ffa030"),
        MOSS=pal("#241a16", "#2e221c", "#382a22", "#423228", "#4c3a2e", "#56422e"),
        ROOT_C=pal("#c45a10", "#e07a18", "#f89a28"),
        GLOW=(255, 140, 30),
        GLOW_DIM=(170, 70, 10))),
    "vine_queen": ("forest_guardian", dict(
        GBARK=pal("#1e3a14", "#26481a", "#2e5620", "#386628", "#427430"),
        GLEAF=pal("#8a1a4a", "#a8245a", "#c4306c", "#dc4a82", "#f070a0"),
        MOSS=pal("#1a4a1a", "#225a22", "#2a6a2a", "#347a32", "#3e8a3a", "#4a9a44"),
        ROOT_C=pal("#5a7a2a", "#6a8c34", "#7a9e3e"),
        GLOW=(255, 110, 180),
        GLOW_DIM=(170, 50, 110))),
    "celestial_herald": ("skeleton", dict(BONE=pal("#5a6a9a", "#7a8cbe", "#a0b0dc", "#c8d4f2", "#f0f4ff"))),
    "blood_zombie": ("zombie", dict(
        ZSKIN=pal("#5a1a1a", "#6e2222", "#822a2a", "#963434", "#a83e3e"),
        ZSHIRT=pal("#2a0a0a", "#3a1010", "#4a1616", "#5a1c1c"),
        ZPANTS=pal("#1a1014", "#24161c", "#2e1c24", "#38222c"))),
}
GOBLIN_SKIN = pal("#4e6e2a", "#5c7e32", "#6a8e3a", "#789e44", "#86ac4e")
GOBLIN_HAIR = pal("#1a1410", "#241c16", "#2e241c", "#382c22", "#423428")
for name, tunic, trouser, leather, iris in [
    ("goblin_warrior", pal("#4a2a14", "#5a341a", "#6a3e20", "#7a4826", "#8a522c"), pal("#2a2420", "#342c26", "#3e342c", "#483c32"), pal("#5a5a5a", "#6e6e6e", "#828282", "#969696"), (210, 40, 30)),
    ("goblin_archer", pal("#1e3a14", "#26481a", "#2e5620", "#386628", "#427430"), pal("#2a2420", "#342c26", "#3e342c", "#483c32"), pal("#5a3a1a", "#6a4420", "#7a4e26", "#8a582c"), (230, 180, 30)),
    ("goblin_sorcerer", pal("#2a1436", "#341a44", "#3e2052", "#482660", "#522c6e"), pal("#1e1a26", "#262030", "#2e263a", "#362c44"), pal("#8a6a10", "#a07c16", "#b88e1c", "#cca024"), (170, 90, 255)),
    ("npc_tinkerer", pal("#5a4a2a", "#6a5832", "#7a663a", "#8a7442", "#9a824a"), pal("#2e2a22", "#38342a", "#423e32", "#4c483a"), pal("#6a6a72", "#7e7e86", "#92929a", "#a6a6ae"), (200, 150, 30)),
]:
    RECOLOR[name] = ("player", dict(PSKIN=GOBLIN_SKIN, PHAIR=GOBLIN_HAIR, PTUNIC=tunic, PTROUSER=trouser, PLEATHER=leather, PIRIS=iris))
RECOLOR["npc_mechanic"] = ("player", dict(
    PHAIR=pal("#6a1a0a", "#8a240e", "#aa3014", "#c43c1a", "#d84a22"),
    PTUNIC=pal("#1a2e5a", "#22386c", "#2a427e", "#344e90", "#3e5aa2"),
    PTROUSER=pal("#1a2e5a", "#22386c", "#2a427e", "#344e90"),
    PLEATHER=pal("#3a3a3a", "#4a4a4a", "#5a5a5a", "#6a6a6a"),
    PIRIS=(60, 140, 90)))
RECOLOR["npc_sorceress"] = ("player", dict(
    PHAIR=pal("#d8d0c0", "#e2dccc", "#ece6d8", "#f4f0e6", "#faf8f2"),
    PTUNIC=pal("#3a1a5a", "#46206c", "#52267e", "#5e2e90", "#6a36a2"),
    PTROUSER=pal("#2a1436", "#341a44", "#3e2052", "#482660"),
    PLEATHER=pal("#8a6a10", "#a07c16", "#b88e1c", "#cca024"),
    PIRIS=(120, 200, 255)))


def main():
    parsed, layers = g.parse_mobs(MOBS_RON)
    assert set(parsed) == set(NEW) | set(RECOLOR), sorted(set(parsed) ^ (set(NEW) | set(RECOLOR)))
    for name, (roles, paint, overlay) in NEW.items():
        assert len(roles) == len(parsed[name]), name
        g.ROLES[name] = roles
        g.PAINTERS[name] = paint
        g.OVERLAYS[name] = overlay
    for name, (base, colours) in RECOLOR.items():
        assert parsed[name] == g.HARDCODED[base], f"{name}: not the {base} model"
        g.ROLES[name] = g.ROLES[base]
        g.PAINTERS[name] = g.townsfolk(name, g.PAINTERS[base], colours)
        g.OVERLAYS[name] = g.townsfolk(name, g.OVERLAYS[base], colours)
    names = list(NEW) + list(RECOLOR)
    scale, gap, cols = 3, 6, 6
    rows = (len(names) + cols - 1) // cols
    sheet = Image.new("RGBA", (cols * (g.SKIN * scale + gap) + gap, rows * (g.SKIN_H * scale + gap) + gap), (24, 24, 28, 255))
    for k, name in enumerate(names):
        img = g.render_skin(name, parsed[name], layers[name])
        img.save(os.path.join(g.OUT_DIR, f"{name}.png"))
        big = img.resize((g.SKIN * scale, g.SKIN_H * scale), Image.NEAREST)
        tile = g.checker(big.size)
        tile.alpha_composite(big)
        sheet.paste(tile, (gap + (k % cols) * (g.SKIN * scale + gap), gap + (k // cols) * (g.SKIN_H * scale + gap)))
    print(f"wrote {len(names)} skins")
    if len(sys.argv) > 1:
        sheet.save(sys.argv[1])


if __name__ == "__main__":
    main()
