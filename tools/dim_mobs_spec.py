"""Creatures of the mod pack's dimensions: the Aether (sky islands) and the Otherside (Deeper
and Darker). Box models are built here from a few body plans and painted by
tools/gen_dim_mobs.py; spawn eggs come from tools/moddata/zz_dim_eggs.py and
tools/modart/zz_dim_eggs.py. Only the mods' ideas are used: shapes, colours and stats are ours.
"""

SKIN_W, SKIN_H = 64, 64


class Model:
    """Parts in pixels (1/16 block), +Z forward, with uv packed into the 64x64 skin."""

    def __init__(self):
        self.parts = []  # dict(size, origin, anim, pivot, look)
        self.layers = []

    def box(self, size, origin, look, anim=None, pivot=(0, 0, 0)):
        self.parts.append(dict(size=tuple(size), origin=tuple(origin), anim=anim, pivot=tuple(pivot), look=look))
        return len(self.parts) - 1

    def layer(self, part, inflate):
        self.layers.append((part, inflate))

    def pack(self):
        """Shelf-pack every part's unwrap into the skin; same size and look share pixels. When
        the parts don't fit in their own order, the tallest go first."""
        try:
            self._pack(self.parts)
        except AssertionError:
            self._pack(sorted(self.parts, key=lambda p: -(p["size"][2] + p["size"][1])))

    def _pack(self, parts):
        x = y = shelf = 0
        seen = {}
        for p in parts:
            w, h, d = p["size"]
            key = (p["size"], p["look"]["key"])
            if key in seen:
                p["uv"] = seen[key]
                continue
            pw, ph = 2 * (w + d), d + h
            if x + pw > SKIN_W:
                x, y, shelf = 0, y + shelf, 0
            assert y + ph <= SKIN_H, "skin full"
            p["uv"] = (x, y)
            seen[key] = (x, y)
            x += pw
            shelf = max(shelf, ph)


def look(key, pal, eyes=None, pattern="noise", belly=None, mouth=None, glow=None):
    """How a part is painted: a dark-to-light palette, a pattern, and for heads the eyes."""
    return dict(key=key, pal=pal, eyes=eyes, pattern=pattern, belly=belly, mouth=mouth, glow=glow)


def quadruped(body, head, leg_h, leg_w, body_look, head_look, leg_look, snout=None, snout_look=None, ears=None, wings=None, wing_look=None, tail=None):
    m = Model()
    bw, bh, bd = body
    hw, hh, hd = head
    m.box(body, (-bw // 2, leg_h, -bd // 2), body_look)
    hy = leg_h + bh - hh // 2
    pivot = (0, hy + hh // 2, bd // 2)
    m.box(head, (-hw // 2, hy, bd // 2 - 1), head_look, "Head", pivot)
    if snout:
        sw, sh, sd = snout
        m.box(snout, (-sw // 2, hy + 1, bd // 2 - 1 + hd), snout_look or head_look, "Head", pivot)
    if ears:
        ew, eh, ed = ears
        for sx in (-1, 1):
            ox = sx * (hw // 2 - ew) if sx > 0 else -hw // 2
            m.box(ears, (ox, hy + hh, bd // 2), head_look, "Head", pivot)
    for k, (sx, sz) in enumerate([(-1, -1), (1, -1), (-1, 1), (1, 1)]):
        ox = -bw // 2 if sx < 0 else bw // 2 - leg_w
        oz = -bd // 2 + 1 if sz < 0 else bd // 2 - leg_w - 1
        swing = (k + (k // 2)) % 2
        m.box((leg_w, leg_h, leg_w), (ox, 0, oz), leg_look, f"Leg({swing})", (ox + leg_w // 2, leg_h, oz + leg_w // 2))
    if wings:
        ww, wh, wd = wings
        for k, sx in enumerate((-1, 1)):
            ox = -bw // 2 - ww if sx < 0 else bw // 2
            m.box(wings, (ox, leg_h + bh - wh, -wd // 2), wing_look, f"Wing({k})", (ox + (ww if sx < 0 else 0), leg_h + bh, 0))
    if tail:
        tw, th, td = tail
        m.box(tail, (-tw // 2, leg_h + bh - th, -bd // 2 - td), body_look)
    return m


def biped(head_look, body_look, arm_look, leg_look, tall=12, wings=None, wing_look=None, arms_forward=False, slim=False):
    m = Model()
    aw = 3 if slim else 4
    m.box((8, 8, 8), (-4, 2 * tall, -4), head_look, "Head", (0, 2 * tall, 0))
    m.box((8, tall, 4), (-4, tall, -2), body_look)
    a = "ArmForward" if arms_forward else "Arm"
    m.box((aw, tall, 4), (-4 - aw, tall, -2), arm_look, f"{a}(0)", (-4 - aw // 2, 2 * tall - 2, 0))
    m.box((aw, tall, 4), (4, tall, -2), arm_look, f"{a}(1)", (4 + aw // 2, 2 * tall - 2, 0))
    m.box((4, tall, 4), (-4, 0, -2), leg_look, "Leg(0)", (-2, tall, 0))
    m.box((4, tall, 4), (0, 0, -2), leg_look, "Leg(1)", (2, tall, 0))
    if wings:
        ww, wh, wd = wings
        m.box(wings, (-4 - ww, 2 * tall - wh, 2), wing_look, "Wing(0)", (-4, 2 * tall - 2, 2))
        m.box(wings, (4, 2 * tall - wh, 2), wing_look, "Wing(1)", (4, 2 * tall - 2, 2))
    return m


def bird(body, head, leg_h, body_look, head_look, beak_look, leg_look, wing_look, tail_look=None, neck=0):
    m = Model()
    bw, bh, bd = body
    hw, hh, hd = head
    m.box(body, (-bw // 2, leg_h, -bd // 2), body_look)
    hy = leg_h + bh - 2 + neck
    pivot = (0, hy, bd // 2)
    if neck:
        m.box((4, neck + 2, 4), (-2, leg_h + bh - 2, bd // 2 - 4), head_look, "Head", pivot)
    m.box(head, (-hw // 2, hy, bd // 2 - hd // 2), head_look, "Head", pivot)
    m.box((2, 2, 3), (-1, hy + hh // 3, bd // 2 + hd - hd // 2), beak_look, "Head", pivot)
    for k, sx in enumerate((-1, 1)):
        ox = -bw // 2 - 1 if sx < 0 else bw // 2 - 1
        m.box((2, bh - 2, bd - 2), (ox, leg_h + 2, -bd // 2 + 1), wing_look, f"Wing({k})", (ox + 1, leg_h + bh, 0))
    for k, sx in enumerate((-1, 1)):
        m.box((1, leg_h, 1), (sx * 2 - (1 if sx < 0 else 0), 0, 0), leg_look, f"Leg({k})", (sx * 2, leg_h, 0))
    if tail_look:
        m.box((bw - 2, 2, 6), (-bw // 2 + 1, leg_h + bh - 3, -bd // 2 - 5), tail_look)
    return m


def blob(size, body_look, inner=None, inner_look=None):
    """A cube creature (swets, sentries, the slider): one box, maybe a core inside."""
    m = Model()
    w, h, d = size
    m.box(size, (-w // 2, 0, -d // 2), body_look)
    if inner:
        iw, ih, idd = inner
        m.box(inner, (-iw // 2, 1, -idd // 2), inner_look)
    return m


def whale(body, body_look, fin_look, head_look=None):
    m = Model()
    w, h, d = body
    m.box(body, (-w // 2, 0, -d // 2), body_look)
    m.box((w - 4, h - 4, 8), (-(w - 4) // 2, 2, -d // 2 - 8), body_look)
    m.box((w + 4, 2, 6), (-(w + 4) // 2, h // 2, -d // 2 - 14), fin_look)
    for k, sx in enumerate((-1, 1)):
        ox = -w // 2 - 8 if sx < 0 else w // 2
        m.box((8, 2, 6), (ox, h // 3, d // 4), fin_look, f"Wing({k})", (ox + (8 if sx < 0 else 0), h // 3, d // 4))
    return m


def worm(segs, size, head_look, body_look):
    m = Model()
    w, h, d = size
    m.box((w + 2, h + 2, d), (-(w + 2) // 2, 0, 0), head_look, "Head", (0, h // 2, 0))
    for k in range(1, segs):
        m.box(size, (-w // 2, 0, -k * d), body_look)
    return m


EYE = (20, 16, 24)
WHITE = (240, 240, 236)

MOBS = []


def mob(name, display, model, kind="Passive", health=10, speed=2.2, size=(0.8, 0.9), spawn="Grass", **kw):
    model.pack()
    MOBS.append(dict(name=name, display=display, model=model, kind=kind, health=health, speed=speed, size=size, spawn=spawn, kw=kw))


AETHER = ["Aether", "Golden Grove"]
AETHER_GROUND = ["aether_grass_block"]

# ---------------------------------------------------------------- Aether
fluff = ["#cfd6e6", "#e2e8f3", "#f1f4fa", "#fbfcff"]
mob("aerbunny", "Aerocoelho",
    quadruped((6, 5, 7), (5, 5, 5), 2, 2,
              look("abody", fluff), look("ahead", fluff, eyes=((70, 120, 200), None)), look("aleg", fluff),
              ears=(2, 5, 1)),
    health=3, speed=3.2, size=(0.5, 0.6), hops=True, flee_range=6.0, biomes=AETHER, ground=AETHER_GROUND,
    drops=[("swet_ball", 0, 1)], xp=1, group=(1, 3))
pink = ["#d98aa0", "#e8a3b4", "#f2b9c6", "#f8cdd6"]
gold_w = ["#c9a64a", "#e0c46a", "#efda8c", "#f8ebb4"]
mob("phyg", "Porco Alado",
    quadruped((10, 8, 14), (8, 8, 7), 6, 4,
              look("pbody", pink), look("phead", pink, eyes=(EYE, None)), look("pleg", ["#b56e80", "#c98495"]),
              snout=(4, 3, 1), snout_look=look("psnout", ["#c06d84", "#d8889c"]),
              wings=(8, 2, 8), wing_look=look("pwing", gold_w, pattern="feathers")),
    health=10, speed=2.4, size=(0.9, 0.9), biomes=AETHER, ground=AETHER_GROUND,
    drops=[("porkchop", 1, 3)], xp=2)
mob("flying_cow", "Vaca Alada",
    quadruped((12, 10, 16), (8, 8, 6), 10, 4,
              look("fcbody", ["#5a4a3c", "#75624f", "#eceae3", "#ffffff"], pattern="patches"),
              look("fchead", ["#5a4a3c", "#75624f", "#eceae3"], eyes=(EYE, None), pattern="patches"),
              look("fcleg", ["#6b5a49", "#857260"]),
              wings=(10, 2, 10), wing_look=look("fcwing", gold_w, pattern="feathers")),
    health=10, speed=2.2, size=(0.9, 1.4), biomes=AETHER, ground=AETHER_GROUND,
    drops=[("beef", 1, 3), ("leather", 0, 2)], xp=2)
wool_b = ["#9fb7e0", "#b6c9ea", "#cddcf3", "#e4edfa"]
m = quadruped((10, 9, 14), (6, 6, 7), 7, 3,
              look("sbody", wool_b, pattern="wool"), look("shead", ["#e9e3d6", "#f4efe6"], eyes=(EYE, None)),
              look("sleg", ["#d9d2c4", "#ece6da"]))
m.layer(0, 1.2)
mob("sheepuff", "Ovelha Fofa", m, health=8, speed=2.0, size=(0.9, 1.2), biomes=AETHER, ground=AETHER_GROUND,
    drops=[("white_wool", 1, 2)], xp=2)
mob("moa", "Moa",
    bird((10, 10, 14), (6, 6, 8), 12, look("mbody", ["#3d6fae", "#4f86c6", "#6aa0da", "#88b8e8"], pattern="feathers"),
         look("mhead", ["#3d6fae", "#5b92d0"], eyes=(EYE, None)), look("mbeak", ["#c7b37a", "#ddd09b"]),
         look("mleg", ["#4c4038", "#62554a"]), look("mwing", ["#2f5a91", "#3f72ad", "#5688c2"], pattern="feathers"),
         tail_look=look("mtail", ["#2a4f80", "#3a6aa2"], pattern="feathers"), neck=6),
    health=20, speed=3.4, size=(0.9, 2.0), biomes=AETHER, ground=AETHER_GROUND, drops=[("moa_feather", 0, 2)], xp=3,
    group=(1, 2), weight=6)
mob("aerwhale", "Aerobaleia",
    whale((12, 10, 20), look("wbody", ["#7d8fa6", "#93a6bd", "#a9bbd0", "#c0d0e2"], belly=["#dfe8f0", "#eef3f8"],
                                  eyes=(EYE, None)),
          look("wfin", ["#6c7d94", "#8193a8"])),
    health=40, speed=1.6, size=(2.4, 1.8), biomes=AETHER, ground=AETHER_GROUND, flies=True, group=(1, 1), weight=3,
    scale=2.6, xp=5)
cloud = ["#b8c2cf", "#cfd7e2", "#e3e9f0", "#f4f7fa"]
mob("zephyr", "Zéfiro",
    blob((14, 10, 14), look("zbody", cloud, pattern="cloud", eyes=((60, 70, 90), None), mouth=(60, 70, 90))),
    kind="Hostile", health=10, speed=2.4, size=(1.4, 1.0), spawn="Dark", ranged=True, attack=3, flies=True,
    biomes=AETHER, xp=5, drops=[("cold_aercloud", 0, 2)])
mob("cockatrice", "Cocatriz",
    bird((8, 8, 12), (6, 6, 7), 10, look("cbody", ["#3f6a3a", "#4f8148", "#6c9a5c"], pattern="feathers"),
         look("chead", ["#4f8148", "#6c9a5c"], eyes=((210, 60, 200), None)), look("cbeak", ["#d6c24a", "#e8d870"]),
         look("cleg", ["#5a4b2a", "#6d5c36"]), look("cwing", ["#5a3f86", "#6e52a0", "#8668ba"], pattern="feathers"),
         tail_look=look("ctail", ["#5a3f86", "#6e52a0"], pattern="feathers"), neck=4),
    kind="Hostile", health=20, speed=3.0, size=(0.8, 1.8), spawn="Dark", ranged=True, attack=4, biomes=AETHER,
    drops=[("feather", 0, 2)], xp=5)
swet = ["#3d78c4", "#5591da", "#74abea", "#9cc7f4"]
mob("swet", "Swet",
    blob((10, 10, 10), look("swbody", swet, pattern="jelly", eyes=(EYE, None)), inner=(4, 4, 4),
         inner_look=look("swcore", ["#1d4f90", "#2a63aa"])),
    kind="Hostile", health=12, speed=2.0, size=(0.7, 0.7), spawn="Dark", attack=2, hops=True, biomes=AETHER,
    drops=[("swet_ball", 1, 2)], xp=3)
mob("aechor_plant", "Planta Aechor",
    blob((8, 6, 8), look("apbody", ["#c25c9a", "#d778b0", "#e895c6", "#f4b4da"], pattern="petals", eyes=(EYE, None))),
    kind="Hostile", health=15, speed=0.0, size=(0.8, 0.6), spawn="Grass", ranged=True, attack=3, biomes=AETHER,
    ground=AETHER_GROUND, drops=[("aechor_petal", 1, 2)], xp=3, group=(1, 1), weight=4)
valk = look("vhead", ["#e8c9a8", "#f2d8bb"], eyes=((70, 110, 190), None))
m = biped(valk, look("vbody", ["#c9ccd4", "#dfe2e8", "#f1f2f5"], pattern="plate"), look("varm", ["#e8c9a8", "#f2d8bb"]),
          look("vleg", ["#9aa1ad", "#b8bec8"]), wings=(10, 14, 1), wing_look=look("vwing", fluff, pattern="feathers"),
          slim=True)
m.layer(0, 0.6)
mob("valkyrie", "Valquíria", m, kind="Hostile", health=50, speed=3.0, size=(0.6, 1.9), spawn="Never", attack=7,
    drops=[("victory_medal", 1, 1)], xp=10)
m = biped(look("vqhead", ["#e8c9a8", "#f2d8bb"], eyes=((240, 200, 90), None)),
          look("vqbody", ["#d7b450", "#e8cc72", "#f5e19c"], pattern="plate"), look("vqarm", ["#e8c9a8", "#f2d8bb"]),
          look("vqleg", ["#b89a40", "#d1b55e"]), wings=(12, 16, 1), wing_look=look("vqwing", gold_w, pattern="feathers"),
          slim=True)
m.layer(0, 0.6)
mob("valkyrie_queen", "Rainha Valquíria", m, kind="Hostile", health=500, speed=3.2, size=(0.6, 1.9), spawn="Never",
    attack=10, boss=True, scale=1.5, drops=[("valkyrie_lance", 1, 1), ("silver_key", 1, 1)], xp=80)
mob("slider", "Deslizador",
    blob((16, 16, 16), look("slbody", ["#6f7377", "#83878b", "#999da1", "#b0b4b8"], pattern="carved",
                             eyes=((230, 80, 60), "glow"))),
    kind="Hostile", health=500, speed=2.8, size=(2.0, 2.0), spawn="Never", attack=12, boss=True, scale=1.3,
    drops=[("bronze_key", 1, 1), ("gravitite_ore", 2, 4)], xp=60)
mob("sentry", "Sentinela",
    blob((8, 8, 8), look("sebody", ["#6f7377", "#83878b", "#999da1"], pattern="carved", eyes=((80, 160, 230), "glow"))),
    kind="Hostile", health=12, speed=2.4, size=(0.6, 0.6), spawn="Never", attack=4, hops=True, explodes=True, xp=4)
fire = ["#a33212", "#d4521a", "#ef8a2a", "#fbc24a"]
m = biped(look("sshead", fire, eyes=((255, 250, 210), "glow"), pattern="flame"), look("ssbody", fire, pattern="flame"),
          look("ssarm", fire, pattern="flame"), look("ssleg", ["#7a2410", "#a33212"], pattern="flame"))
mob("sun_spirit", "Espírito do Sol", m, kind="Hostile", health=600, speed=2.4, size=(0.8, 2.0), spawn="Never",
    attack=10, ranged=True, boss=True, scale=2.2, flies=True, drops=[("gold_key", 1, 1), ("phoenix_feather", 8, 12)], xp=100)
mob("fire_minion", "Lacaio de Fogo",
    biped(look("fmhead", fire, eyes=((255, 250, 210), "glow"), pattern="flame"), look("fmbody", fire, pattern="flame"),
          look("fmarm", fire, pattern="flame"), look("fmleg", ["#7a2410", "#a33212"], pattern="flame"), tall=8),
    kind="Hostile", health=20, speed=3.0, size=(0.6, 1.3), spawn="Never", attack=5, xp=5)

# ---------------------------------------------------------------- Otherside
OTHER = ["Deeplands", "Echoing Forest", "Overcast Columns", "Blooming Caverns"]
OTHER_GROUND = ["gloomslate", "sculk_grime", "echo_soil", "blooming_moss", "sculk_stone"]
sculk = ["#0b1e26", "#123341", "#1a4a5a", "#2a6a78"]
bone = ["#a9a69a", "#c4c1b4", "#dcd9cc"]
m = biped(look("shhead", ["#0c2028", "#143444"], eyes=((70, 230, 220), "glow"), pattern="sculk"),
          look("shbody", sculk, pattern="sculk"), look("sharm", sculk, pattern="sculk"),
          look("shleg", ["#091820", "#0f2a36"], pattern="sculk"), tall=16, slim=True)
mob("shattered", "Estilhaçado", m, kind="Hostile", health=40, speed=2.6, size=(0.7, 2.6), spawn="Dark", attack=7,
    biomes=OTHER, drops=[("echo_crystal", 0, 1), ("bone", 0, 2)], xp=8)
mob("sculk_snapper", "Mordedor de Sculk",
    quadruped((8, 6, 12), (7, 6, 7), 5, 3, look("ssbody2", sculk, pattern="sculk"),
              look("sshead2", sculk, eyes=((70, 230, 220), "glow"), mouth=WHITE, pattern="sculk"),
              look("ssleg2", ["#091820", "#0f2a36"])),
    kind="Neutral", health=20, speed=3.0, size=(0.7, 0.8), spawn="Grass", attack=5, biomes=OTHER, ground=OTHER_GROUND,
    drops=[("sculk_bone", 0, 2)], xp=4)
mob("angler_fish", "Peixe-Pescador",
    blob((10, 8, 12), look("afbody", ["#13161f", "#1d2230", "#2a3142"], eyes=((240, 240, 160), "glow"), mouth=WHITE),
         inner=(2, 2, 2), inner_look=look("aflure", ["#bff8f0", "#e8fffb"], glow=True)),
    kind="Hostile", health=14, speed=2.6, size=(0.8, 0.6), spawn="Water", attack=5, swims=True, biomes=OTHER, xp=4)
mob("sculk_leech", "Sanguessuga de Sculk",
    worm(4, (3, 3, 3), look("slhead", sculk, eyes=((70, 230, 220), "glow"), pattern="sculk"),
         look("slbody", sculk, pattern="sculk")),
    kind="Hostile", health=6, speed=2.4, size=(0.4, 0.3), spawn="Dark", attack=2, biomes=OTHER, xp=2)
mob("shriek_worm", "Verme Uivante",
    worm(6, (10, 10, 10), look("swhead", ["#0d1b22", "#16303c"], eyes=((70, 230, 220), "glow"), mouth=WHITE, pattern="sculk"),
         look("swbody", ["#0d1b22", "#16303c", "#21485a"], pattern="rings")),
    kind="Hostile", health=80, speed=1.2, size=(1.2, 1.2), spawn="Dark", attack=12, biomes=["Deeplands"], weight=2,
    group=(1, 1), drops=[("heart_of_the_deep_shard", 0, 1)], xp=20, scale=1.4)
mob("sculk_centipede", "Centopeia de Sculk",
    worm(7, (4, 3, 4), look("schead", sculk, eyes=((70, 230, 220), "glow"), pattern="sculk"),
         look("scbody", ["#0b1e26", "#123341", "#d9d6c8"], pattern="rings")),
    kind="Hostile", health=16, speed=3.2, size=(0.6, 0.4), spawn="Dark", attack=4, climbs=True, biomes=OTHER, xp=4)
mob("sludge", "Lodo",
    blob((10, 8, 10), look("sdbody", ["#1b2b2a", "#25403d", "#335a54"], pattern="jelly", eyes=((70, 230, 220), "glow"))),
    kind="Hostile", health=16, speed=1.8, size=(0.8, 0.6), spawn="Dark", attack=3, hops=True, biomes=OTHER,
    drops=[("slime_ball", 0, 2)], xp=3)
mob("glare", "Clarão",
    blob((10, 10, 10), look("glbody", ["#2f5a2c", "#3f7a3a", "#56984e"], pattern="leaves", eyes=(EYE, None)),
         inner=(4, 4, 4), inner_look=look("glcore", ["#cfe86a", "#e8fb9a"], glow=True)),
    kind="Passive", health=10, speed=2.0, size=(0.8, 0.8), spawn="Grass", flies=True, biomes=["Blooming Caverns"],
    ground=OTHER_GROUND, xp=2, group=(1, 2))
m = biped(look("sthead", ["#0a1418", "#10222a"], eyes=((70, 230, 220), "glow"), mouth=(70, 230, 220), pattern="sculk"),
          look("stbody", sculk, pattern="ribs"), look("starm", sculk, pattern="sculk"),
          look("stleg", ["#081216", "#0e1f26"], pattern="sculk"), tall=18, slim=True, arms_forward=True)
mob("otherside_stalker", "Espreitador", m, kind="Hostile", health=300, speed=3.0, size=(0.8, 3.0), spawn="Never",
    attack=14, boss=True, scale=1.2, drops=[("heart_of_the_deep", 1, 1), ("echo_crystal", 3, 6)], xp=60)

# ---------------------------------------------------------------- Aircraft (Immersive Aircraft's idea)
# Not creatures: the models of the aircraft in crates/game/src/mods/aircraft.rs, drawn like mobs.
# Parts with Wing(0) spin about the forward axis (propellers), Wing(1) about the up axis (rotors),
# Head turns with the warship's aim.
wood = look("acwood", ["#4a2e16", "#6a4422", "#8a6034", "#a87c48"], pattern="planks")
dark_wood = look("acdark", ["#2e1c0e", "#43291a", "#5a3a1c"], pattern="planks")
metal = look("acmetal", ["#2c2f36", "#454951", "#666b74", "#8a9099"], pattern="plate")
canvas_ = look("accloth", ["#a89c82", "#c4b89c", "#dcd2b8", "#ece4d0"], pattern="cloth")
red = look("acred", ["#6a1e18", "#8a2a20", "#a8382a", "#c44a34"], pattern="cloth")
blade = look("acblade", ["#3a2614", "#5a3a1c", "#7a5028"], pattern="noise")
rope = look("acrope", ["#6a5a3a", "#8a7a52"], pattern="noise")
bamboo = look("acbamboo", ["#4e6a2a", "#62822e", "#7a9a3a", "#94b450"], pattern="rings")
chest = look("acchest", ["#5a3a1c", "#8a5a2a", "#b07a3a"], pattern="carved")


def biplane():
    m = Model()
    m.box((6, 6, 20), (-3, 2, -10), wood)
    m.box((6, 6, 2), (-3, 2, 8), metal)
    for x in (-17, 3):
        m.box((14, 1, 6), (x, 2, 1), red)
        m.box((14, 1, 6), (x, 10, 1), red)
    m.box((6, 1, 6), (-3, 10, 1), red)
    for x in (-12, 11):
        m.box((1, 8, 1), (x, 3, 3), metal)
    m.box((1, 6, 4), (0, 8, -10), red)
    m.box((12, 1, 4), (-6, 6, -10), red)
    m.box((14, 2, 1), (-7, 4, 10), blade, "Wing(0)", (0, 5, 10))
    for x in (-4, 3):
        m.box((1, 2, 2), (x, 0, 4), metal)
    return m


def airship(cargo=False):
    m = Model()
    m.box((12, 10, 12), (-6, 18, 0), canvas_)
    m.box((12, 10, 12), (-6, 18, -12), canvas_)
    m.box((8, 6, 2), (-4, 20, 12), canvas_)
    m.box((8, 6, 2), (-4, 20, -14), canvas_)
    m.box((8, 4, 12), (-4, 0, -6), wood)
    for x, z in ((-4, -5), (3, -5), (-4, 4), (3, 4)):
        m.box((1, 14, 1), (x, 4, z), rope)
    m.box((1, 8, 5), (0, 22, -19), red)
    m.box((10, 1, 5), (-5, 23, -19), red)
    m.box((4, 4, 2), (-2, 0, -8), metal)
    m.box((10, 2, 1), (-5, 1, -9), blade, "Wing(0)", (0, 2, -9))
    if cargo:
        m.box((6, 4, 6), (-3, 4, -3), chest)
    return m


def gyrodyne():
    m = Model()
    m.box((6, 6, 14), (-3, 1, -7), wood)
    m.box((2, 8, 2), (-1, 7, -1), metal)
    m.box((28, 1, 2), (-14, 15, -1), blade, "Wing(1)", (0, 15, 0))
    m.box((2, 2, 10), (-1, 4, -17), wood)
    m.box((1, 5, 3), (0, 4, -20), red)
    m.box((10, 2, 1), (-5, 4, 7), blade, "Wing(0)", (0, 5, 7))
    for x in (-4, 3):
        m.box((1, 1, 12), (x, 0, -6), metal)
    m.box((6, 4, 1), (-3, 7, -5), dark_wood)
    return m


def quadrocopter():
    m = Model()
    m.box((8, 3, 8), (-4, 2, -4), bamboo)
    m.box((20, 1, 2), (-10, 4, -1), bamboo)
    m.box((2, 1, 20), (-1, 4, -10), bamboo)
    centers = ((-9, 0), (9, 0), (0, -9), (0, 9))
    for cx, cz in centers:
        m.box((2, 3, 2), (cx - 1, 5, cz - 1), metal)
    for cx, cz in centers:
        m.box((8, 1, 1), (cx - 4, 8, cz), blade, "Wing(1)", (cx, 8, cz))
    for x in (-4, 3):
        m.box((1, 2, 8), (x, 0, -4), bamboo)
    return m


def warship():
    m = Model()
    m.box((14, 6, 12), (-7, 0, 0), dark_wood)
    m.box((14, 6, 12), (-7, 0, -12), dark_wood)
    m.box((8, 4, 4), (-4, 1, 12), dark_wood)
    m.box((8, 6, 8), (-4, 6, -10), wood)
    m.box((12, 8, 14), (-6, 18, 0), red)
    m.box((12, 8, 14), (-6, 18, -14), red)
    for x, z in ((-6, -10), (5, -10), (-6, 9), (5, 9)):
        m.box((1, 12, 1), (x, 6, z), rope)
    m.box((4, 2, 4), (-2, 6, 4), metal)
    m.box((2, 2, 8), (-1, 8, 4), metal, "Head", (0, 9, 6))
    m.box((8, 2, 1), (-12, 2, -13), blade, "Wing(0)", (-8, 3, -13))
    m.box((8, 2, 1), (4, 2, -13), blade, "Wing(0)", (8, 3, -13))
    return m


AIRCRAFT = dict(health=20, speed=0.0, spawn="Never")
mob("biplane", "Biplano", biplane(), size=(2.0, 1.2), scale=1.5, **AIRCRAFT)
mob("airship", "Dirigível", airship(), size=(1.6, 3.4), scale=2.0, **AIRCRAFT)
mob("cargo_airship", "Dirigível de Carga", airship(True), size=(1.6, 3.4), scale=2.0, **AIRCRAFT)
mob("gyrodyne", "Girodino", gyrodyne(), size=(1.4, 1.6), scale=1.5, **AIRCRAFT)
mob("quadrocopter", "Quadricóptero", quadrocopter(), size=(1.4, 0.9), scale=1.3, **AIRCRAFT)
mob("warship", "Navio de Guerra Aéreo", warship(), size=(2.2, 3.2), scale=2.0, **AIRCRAFT)

# Egg colours: (base, spots).
EGGS = {
    "aerbunny": ((226, 232, 243), (110, 150, 210)), "phyg": ((232, 163, 180), (224, 196, 106)),
    "flying_cow": ((117, 98, 79), (224, 196, 106)), "sheepuff": ((182, 201, 234), (240, 236, 228)),
    "moa": ((79, 134, 198), (221, 208, 155)), "aerwhale": ((147, 166, 189), (223, 232, 240)),
    "zephyr": ((207, 215, 226), (90, 100, 120)), "cockatrice": ((79, 129, 72), (110, 82, 160)),
    "swet": ((85, 145, 218), (29, 79, 144)), "aechor_plant": ((215, 120, 176), (80, 140, 70)),
    "valkyrie": ((223, 226, 232), (232, 196, 106)), "sentry": ((131, 135, 139), (80, 160, 230)),
    "fire_minion": ((212, 82, 26), (251, 194, 74)),
    "shattered": ((18, 51, 65), (70, 230, 220)), "sculk_snapper": ((26, 74, 90), (220, 220, 210)),
    "angler_fish": ((29, 34, 48), (240, 240, 160)), "sculk_leech": ((18, 51, 65), (160, 40, 60)),
    "shriek_worm": ((22, 48, 60), (240, 240, 236)), "sculk_centipede": ((18, 51, 65), (217, 214, 200)),
    "sludge": ((37, 64, 61), (70, 230, 220)), "glare": ((63, 122, 58), (207, 232, 106)),
}

# The creatures of Distant Lands' Brazilian biomes (Overworld) add themselves to MOBS and EGGS.
import distant_mobs_spec  # noqa: E402,F401
