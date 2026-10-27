"""Our own interface art (hotbar, hearts, hunger, armour, air, experience bar, crosshair, slots).

Written in Minecraft's sprite layout (textures/gui/sprites/...), so the game's interface code
reads ours exactly like a resource pack's, and any pack that has its own still wins. Drawn at 2
texels per interface pixel in the same style as the blocks: warm stone panels lit from the
top-left, soft bevels, outlines in each colour's own darker shade.
"""

import os

import numpy as np
from PIL import Image

from style import hexrgb, ramp, to_u8

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
OUT = os.path.join(ROOT, "packs", "vanilla", "textures", "gui")
S = 2  # texels per interface pixel


class Art:
    def __init__(self, w, h):
        self.w, self.h = w * S, h * S
        self.img = np.zeros((self.h, self.w, 4))
        self.Y, self.X = np.mgrid[0:self.h, 0:self.w] / S  # in interface pixels

    def fill(self, mask, rgb, a=255):
        self.img[mask, :3] = rgb
        self.img[mask, 3] = a

    def rect(self, x0, y0, x1, y1):
        return (self.X >= x0) & (self.X < x1) & (self.Y >= y0) & (self.Y < y1)

    def disc(self, cx, cy, r):
        return (self.X - cx) ** 2 + (self.Y - cy) ** 2 <= r * r

    def bevel_box(self, x0, y0, x1, y1, pal, inset=False, edge=1.0, a=255):
        """A panel with a dark outer line, a lit top-left bevel and a shaded bottom-right."""
        m = self.rect(x0, y0, x1, y1)
        self.fill(m, pal[2 if inset else 5], a)
        outer = m & ~self.rect(x0 + edge * 0.5, y0 + edge * 0.5, x1 - edge * 0.5, y1 - edge * 0.5)
        tl = m & ~outer & ((self.X < x0 + edge) | (self.Y < y0 + edge))
        br = m & ~outer & ~tl & ((self.X >= x1 - edge) | (self.Y >= y1 - edge))
        self.fill(tl, pal[1 if inset else 8], a)
        self.fill(br, pal[7 if inset else 3], a)
        self.fill(outer, pal[0], a)

    def save(self, rel):
        p = os.path.join(OUT, rel + ".png")
        os.makedirs(os.path.dirname(p), exist_ok=True)
        Image.fromarray(to_u8(self.img), "RGBA").save(p, optimize=True)


STONE = ramp("#8e8a84", 10, 0.55, 0.12)
SLOT = ramp("#5c5853", 10, 0.5, 0.1)


def hotbar():
    a = Art(182, 22)
    a.bevel_box(0, 0, 182, 22, STONE, a=235)
    for i in range(9):
        x = 1 + i * 20
        a.bevel_box(x + 1, 1 + 1, x + 19, 21 - 0, SLOT, inset=True, a=200)
    a.save("sprites/hud/hotbar")


def hotbar_selection():
    a = Art(24, 23)
    gold = ramp("#f3e7c0", 10, 0.5, 0.15)
    outer = a.rect(0, 0, 24, 23) & ~a.rect(2, 2, 22, 21)
    a.fill(outer, gold[8])
    a.fill(outer & ((a.X >= 23) | (a.Y >= 22)), gold[4])
    a.fill(a.rect(0, 0, 24, 23) & ~a.rect(0.5, 0.5, 23.5, 22.5), gold[1])
    a.fill(a.rect(2, 2, 22, 21) & ~a.rect(2.5, 2.5, 21.5, 20.5), gold[2])
    a.save("sprites/hud/hotbar_selection")


def crosshair():
    a = Art(15, 15)
    m = (a.rect(7, 2, 8, 13) | a.rect(2, 7, 13, 8)) & ~a.rect(6.5, 6.5, 8.5, 8.5)
    halo = (a.rect(6.5, 1.5, 8.5, 13.5) | a.rect(1.5, 6.5, 13.5, 8.5)) & ~m
    a.fill(halo, (20, 20, 24), 110)
    a.fill(m, (240, 240, 235), 230)
    a.save("sprites/hud/crosshair")


def icon_shape(kind):
    """9x9 interface-pixel icons as boolean masks on a 2x grid."""
    a = Art(9, 9)
    X, Y = a.X, a.Y
    if kind == "heart":
        m = ((X - 2.75) ** 2 + (Y - 3.0) ** 2 <= 2.1 ** 2) | ((X - 6.25) ** 2 + (Y - 3.0) ** 2 <= 2.1 ** 2)
        m |= (Y >= 3.0) & (np.abs(X - 4.5) <= (8.2 - Y) * 0.95)
        m &= (Y >= 0.8) & (Y < 8.4)
    elif kind == "food":
        m = ((X - 5.4) ** 2 + (Y - 3.6) ** 2 <= 2.9 ** 2)
        m |= (np.abs((X - Y) - (-0.4)) < 0.9) & (X > 1.5) & (X < 4.5)
        m |= ((X - 1.6) ** 2 + (Y - 7.0) ** 2 <= 1.1 ** 2) | ((X - 2.3) ** 2 + (Y - 7.8) ** 2 <= 0.8 ** 2)
    elif kind == "armor":
        m = a.rect(1, 1, 8, 4.5) | a.rect(2, 4, 7, 8.5)
        m &= ~a.rect(3.5, 1, 5.5, 2.2)
        m &= ~(a.rect(1, 4.5, 2, 8.5) | a.rect(7, 4.5, 8, 8.5))
    else:  # bubble
        m = (X - 4.5) ** 2 + (Y - 4.5) ** 2 <= 3.6 ** 2
    return a, m


def icon(kind, name, base, fill="full", hollow=False):
    a, m = icon_shape(kind)
    pal = ramp(base, 10, 0.55, 0.14, sat=1.1)
    outline = m & ~np.roll(np.roll(m, 0, 0), 0, 1)
    pad = np.pad(m, 1, constant_values=False)
    inner = pad[:-2, 1:-1] & pad[2:, 1:-1] & pad[1:-1, :-2] & pad[1:-1, 2:]
    ring = m & ~inner
    # Grow the outline one texel outward so it reads at small sizes.
    pad2 = np.pad(m, 1, constant_values=False)
    out_ring = (pad2[:-2, 1:-1] | pad2[2:, 1:-1] | pad2[1:-1, :-2] | pad2[1:-1, 2:]) & ~m
    a.fill(out_ring, pal[0] * 0.55)
    if hollow:
        a.fill(m, (34, 30, 30), 200)
        a.fill(ring, pal[0] * 0.7)
    else:
        body = m.copy()
        if fill == "half":
            body &= a.X < 4.5
            a.fill(m & ~body, (34, 30, 30), 200)
        X, Y = a.X, a.Y
        t = 0.55 - (Y - 4.5) / 9 * 0.35 - (X - 4.5) / 9 * 0.1
        idx = np.clip((t * 9).round().astype(int), 0, 9)
        rgb = pal[idx]
        a.img[body, :3] = rgb[body]
        a.img[body, 3] = 255
        a.fill(ring & body, pal[1])
        glint = body & ((X - 2.6) ** 2 + (Y - 2.4) ** 2 <= 0.75 ** 2)
        a.fill(glint, (255, 252, 240))
    if kind == "bubble":
        a.img[m & ~ring, 3] = 150
    a.save(name)


def xp_bar():
    bg = Art(182, 5)
    dark = ramp("#2c3a26", 8, 0.3, 0.1)
    bg.bevel_box(0, 0, 182, 5, dark, inset=True)
    for i in range(1, 18):
        x = i * 182 / 18
        bg.fill(bg.rect(x - 0.25, 1, x + 0.25, 4), dark[0])
    bg.save("sprites/hud/experience_bar_background")
    pr = Art(182, 5)
    g = ramp("#7cd84a", 10, 0.5, 0.15)
    m = pr.rect(0, 0, 182, 5)
    t = 0.6 - (pr.Y - 2.5) / 5 * 0.5
    pr.img[m, :3] = g[np.clip((t * 9).round().astype(int), 0, 9)][m]
    pr.img[m, 3] = 255
    pr.fill(pr.rect(0, 0, 182, 0.5) | pr.rect(0, 4.5, 182, 5), g[1])
    pr.fill(pr.rect(0, 1, 182, 1.5), g[9])
    pr.save("sprites/hud/experience_bar_progress")


def slots():
    """Empty armour slots: a faint outline of the piece that goes there."""
    shapes = {
        "helmet": lambda a: (a.rect(3, 4, 13, 8) | a.rect(3, 8, 5, 11) | a.rect(11, 8, 13, 11)),
        "chestplate": lambda a: (a.rect(2, 3, 14, 7) | a.rect(4, 7, 12, 14)) & ~a.rect(6, 3, 10, 5),
        "leggings": lambda a: (a.rect(4, 2, 12, 5) | a.rect(4, 5, 7, 14) | a.rect(9, 5, 12, 14)),
        "boots": lambda a: (a.rect(3, 7, 6, 13) | a.rect(10, 7, 13, 13) | a.rect(1, 11, 6, 13) | a.rect(10, 11, 15, 13)),
        "shield": lambda a: (a.rect(4, 2, 12, 9) | ((np.abs(a.X - 8) < (14 - a.Y) * 0.8) & (a.Y >= 9) & (a.Y < 14))),
    }
    for name, f in shapes.items():
        a = Art(16, 16)
        m = f(a)
        a.fill(m, (70, 68, 66), 120)
        pad = np.pad(m, 1, constant_values=False)
        ring = m & ~(pad[:-2, 1:-1] & pad[2:, 1:-1] & pad[1:-1, :-2] & pad[1:-1, 2:])
        a.fill(ring, (54, 52, 50), 170)
        a.save(f"sprites/container/slot/{name}")


def main():
    hotbar()
    hotbar_selection()
    crosshair()
    icon("heart", "sprites/hud/heart/full", "#d8303a")
    icon("heart", "sprites/hud/heart/half", "#d8303a", fill="half")
    icon("heart", "sprites/hud/heart/container", "#4a3434", hollow=True)
    icon("food", "sprites/hud/food_full", "#b8763a")
    icon("food", "sprites/hud/food_half", "#b8763a", fill="half")
    icon("food", "sprites/hud/food_empty", "#4a3a2e", hollow=True)
    icon("armor", "sprites/hud/armor_full", "#c8ccd4")
    icon("armor", "sprites/hud/armor_half", "#c8ccd4", fill="half")
    icon("armor", "sprites/hud/armor_empty", "#3c3c44", hollow=True)
    icon("bubble", "sprites/hud/air", "#7ab8f0")
    icon("bubble", "sprites/hud/air_bursting", "#b8dcf8", hollow=True)
    xp_bar()
    slots()
    print(f"gui: interface sprites in {os.path.relpath(OUT, ROOT)}")


if __name__ == "__main__":
    main()
