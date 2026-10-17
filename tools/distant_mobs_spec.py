"""Creatures of Distant Lands' Brazilian biomes (Mata Atlântica, Cerrado, Pantanal, Mata de
Araucárias, Caatinga), built with the body plans of tools/dim_mobs_spec.py and painted by
tools/gen_dim_mobs.py into mods/dims_mobs.ron; their spawn eggs come with the others'
(tools/moddata/zz_dim_eggs.py). The toucan, capuchin, anaconda and alligator of
tools/mod_creatures.py also live here. Shapes, colours and stats are ours."""

import dim_mobs_spec as spec
from dim_mobs_spec import EYE, WHITE, Model, bird, look, mob, quadruped

MATA = ["Mata Atlântica"]
CERRADO = ["Cerrado"]
PANTANAL = ["Pantanal"]
ARAUCARIA = ["Mata de Araucárias"]
CAATINGA = ["Caatinga"]
GROUND = ["grass_block", "red_soil", "mud", "cracked_dirt", "sand"]

# Capivara: a big barrel of a rodent with a blunt square head, in herds by the water.
fur = ["#5a3c22", "#6e4a2a", "#825834", "#94683e"]
mob("capivara", "Capivara",
    quadruped((10, 9, 16), (8, 8, 8), 4, 3, look("cpbody", fur), look("cphead", fur, eyes=(EYE, None)),
              look("cpleg", ["#4a3018", "#5a3c22"]), snout=(6, 5, 2), snout_look=look("cpsnout", ["#3e2a18", "#4e3620"]),
              ears=(2, 2, 1)),
    health=14, speed=2.0, size=(0.9, 0.9), biomes=PANTANAL + MATA + CERRADO, ground=GROUND, swims=True, sleeps=True,
    drops=[("leather", 0, 2)], xp=2, group=(2, 5), weight=12)

# Onça-pintada: golden with black rosettes; leaves people be unless hurt, hunts capivaras.
gold_spot = ["#2a1c10", "#c8902e", "#d8a23a", "#e6b44a"]
mob("onca_pintada", "Onça-Pintada",
    quadruped((8, 8, 18), (7, 7, 7), 7, 3, look("onbody", gold_spot, pattern="rosettes", belly=["#e8dcc0", "#f2e8d0"]),
              look("onhead", gold_spot, eyes=((200, 170, 40), None), pattern="rosettes"),
              look("onleg", gold_spot, pattern="rosettes"), snout=(4, 3, 1), snout_look=look("onsnout", ["#c8a070", "#e0c090"]),
              ears=(2, 2, 1), tail=(2, 2, 10)),
    kind="Neutral", health=30, speed=3.4, size=(0.9, 1.0), attack=7, biomes=PANTANAL + MATA, ground=GROUND,
    prey=["capivara", "jacare"], drops=[("leather", 1, 2)], xp=5, group=(1, 1), weight=3, sleeps=True)

# Arara-azul: a deep blue macaw with a yellow ring round the eye.
blue = ["#1a2a7a", "#22389a", "#2c48b8", "#3a5ad0"]
mob("arara_azul", "Arara-Azul",
    bird((5, 6, 8), (5, 5, 5), 3, look("aabody", blue, pattern="feathers"),
         look("aahead", blue, eyes=((240, 200, 40), None)), look("aabeak", ["#1a1a1e", "#2a2a30"]),
         look("aaleg", ["#2a2a2e", "#3a3a40"]), look("aawing", ["#14206a", "#1e308a", "#2a44aa"], pattern="feathers"),
         tail_look=look("aatail", blue, pattern="feathers")),
    health=6, speed=2.6, size=(0.5, 0.7), flies=True, biomes=PANTANAL + CERRADO, ground=GROUND,
    drops=[("feather", 0, 2)], xp=1, group=(2, 3), weight=6, flee_range=6.0)

# Tamanduá-bandeira: long tube snout, grey-brown with the black-and-white sash, huge tail.
m = quadruped((8, 9, 16), (5, 5, 6), 6, 3, look("tmbody", ["#4a4038", "#5e5246", "#706456", "#d8d0c0"], pattern="patches"),
              look("tmhead", ["#5e5246", "#706456"], eyes=(EYE, None)), look("tmleg", ["#e0d8c8", "#2a2420"], pattern="patches"),
              snout=(2, 2, 8), snout_look=look("tmsnout", ["#3a3028", "#4a4038"]))
m.box((4, 10, 10), (-2, 6, -8 - 10), look("tmtail", ["#3a3028", "#4a4038", "#5e5246"], pattern="wool"))
mob("tamandua", "Tamanduá-Bandeira", m, kind="Neutral", health=20, speed=1.6, size=(0.9, 1.0), attack=4,
    biomes=CERRADO + PANTANAL, ground=GROUND, drops=[("leather", 0, 1)], xp=3, group=(1, 1), weight=5)

# Lobo-guará: a red fox on stilts with black socks and a dark mane; shy.
red = ["#9a3c14", "#b84c1a", "#d0602a", "#e07838"]
mob("lobo_guara", "Lobo-Guará",
    quadruped((6, 7, 14), (6, 6, 6), 12, 2, look("lgbody", red), look("lghead", red, eyes=(EYE, None)),
              look("lgleg", ["#141210", "#201c18"]), snout=(3, 3, 3), snout_look=look("lgsnout", ["#201c18", "#302a24"]),
              ears=(2, 4, 1), tail=(2, 2, 7)),
    health=14, speed=3.4, size=(0.7, 1.4), biomes=CERRADO, ground=GROUND, flee_range=10.0, drops=[("leather", 0, 1)],
    xp=2, group=(1, 2), weight=5)

# Jacaré-do-pantanal: low, dark and armoured; in the pools.
gator = ["#1e2418", "#2a3220", "#36402a", "#465234"]
m = quadruped((10, 4, 18), (8, 4, 10), 2, 3, look("jcbody", gator, pattern="plate", belly=["#a8a080", "#c0b898"]),
              look("jchead", gator, eyes=((200, 180, 60), None), mouth=WHITE, pattern="plate"), look("jcleg", gator))
m.box((4, 3, 14), (-2, 2, -9 - 14), look("jctail", gator, pattern="plate"))
mob("jacare", "Jacaré-do-Pantanal", m, kind="Neutral", health=24, speed=1.8, size=(1.0, 0.5), attack=6, swims=True,
    biomes=PANTANAL, ground=GROUND, drops=[("leather", 0, 2)], xp=3, group=(1, 3), weight=8)

# Tuiuiú: the tall white stork of the Pantanal, black bare head and a red collar.
m = bird((8, 9, 12), (5, 5, 6), 14, look("tubody", ["#d8d8d4", "#e8e8e4", "#f6f6f2"], pattern="feathers"),
         look("tuhead", ["#141414", "#202020"], eyes=(EYE, None)), look("tubeak", ["#141414", "#26262a"]),
         look("tuleg", ["#141414", "#202020"]), look("tuwing", ["#e0e0dc", "#f0f0ec"], pattern="feathers"), neck=8)
# A red collar round the bottom of the neck (the neck box starts at y 21, z 2).
m.box((6, 2, 6), (-3, 22, 1), look("tucollar", ["#b81e1e", "#d03030"]), "Head", (0, 29, 6))
mob("tuiuiu", "Tuiuiú", m, health=12, speed=2.2, size=(0.8, 2.0), biomes=PANTANAL, ground=GROUND, flee_range=8.0,
    drops=[("feather", 1, 3)], xp=2, group=(1, 3), weight=6)

# Mico-leão-dourado: a little golden monkey with a lion's mane and a long tail.
gold = ["#c86a10", "#e08418", "#f09e28", "#f8b840"]
m = quadruped((4, 4, 7), (6, 6, 5), 4, 2, look("mlbody", gold, pattern="wool"),
              look("mlhead", gold, eyes=(EYE, None), pattern="wool"), look("mlleg", gold),
              snout=(2, 2, 1), snout_look=look("mlface", ["#3a2418", "#4a3020"]))
m.box((2, 2, 10), (-1, 6, -3 - 10), look("mltail", gold, pattern="wool"))
mob("mico_leao_dourado", "Mico-Leão-Dourado", m, health=6, speed=3.2, size=(0.4, 0.5), climbs=True, biomes=MATA,
    ground=GROUND, flee_range=7.0, xp=1, group=(2, 4), weight=7, scale=0.8)

# Tatu-bola: rolls up in its banded shell.
m = quadruped((7, 5, 9), (4, 4, 4), 2, 2, look("tbbody", ["#7a6248", "#8e7458", "#a48a6a", "#c0a888"], pattern="rings"),
              look("tbhead", ["#8e7458", "#a48a6a"], eyes=(EYE, None)), look("tbleg", ["#6a5440", "#7a6248"]),
              ears=(1, 2, 1), tail=(2, 2, 3))
mob("tatu_bola", "Tatu-Bola", m, health=8, speed=1.8, size=(0.6, 0.5), biomes=CAATINGA + CERRADO, ground=GROUND,
    flee_range=5.0, drops=[("leather", 0, 1)], xp=1, group=(1, 2), weight=8)

# Ema: the big grey flightless runner of the open plains.
m = bird((10, 10, 14), (4, 4, 6), 14, look("embody", ["#6e6a62", "#828078", "#98968c", "#aaa89e"], pattern="wool"),
         look("emhead", ["#828078", "#98968c"], eyes=(EYE, None)), look("embeak", ["#c0a878", "#d4bc8c"]),
         look("emleg", ["#8a8070", "#a09684"]), look("emwing", ["#5e5a52", "#6e6a62", "#828078"], pattern="feathers"), neck=10)
mob("ema", "Ema", m, health=16, speed=3.6, size=(0.9, 2.2), biomes=CERRADO, ground=GROUND, flee_range=9.0,
    drops=[("feather", 1, 3)], xp=2, group=(2, 4), weight=5)

# Asa-branca: the grey dove of the sertão with white-edged wings.
m = bird((4, 4, 6), (3, 3, 3), 2, look("abbody", ["#7a7680", "#8c8892", "#9e9aa4"], pattern="feathers"),
         look("abhead", ["#8c8892", "#9e9aa4"], eyes=((180, 40, 30), None)), look("abbeak", ["#2a2a2a", "#3a3a3a"]),
         look("ableg", ["#b05a5a", "#c06a6a"]), look("abwing", ["#6a6670", "#e8e8ec", "#7a7680"], pattern="feathers"),
         tail_look=look("abtail", ["#5a5660", "#6a6670"], pattern="feathers"))
mob("asa_branca", "Asa-Branca", m, health=4, speed=2.8, size=(0.4, 0.4), flies=True, biomes=CAATINGA, ground=GROUND,
    flee_range=6.0, drops=[("feather", 0, 1)], xp=1, group=(2, 4), weight=8, scale=0.8)

# Gralha-azul: the blue jay that plants the araucárias' pinhões.
m = bird((4, 5, 7), (4, 4, 4), 3, look("gabody", ["#1e48a0", "#2a5cc0", "#3a70d8"], pattern="feathers"),
         look("gahead", ["#101014", "#1a1a20"], eyes=(EYE, None)), look("gabeak", ["#101014", "#1a1a20"]),
         look("galeg", ["#101014", "#1a1a20"]), look("gawing", ["#163a88", "#1e48a0", "#2a5cc0"], pattern="feathers"),
         tail_look=look("gatail", ["#163a88", "#2a5cc0"], pattern="feathers"))
mob("gralha_azul", "Gralha-Azul", m, health=5, speed=2.8, size=(0.4, 0.6), flies=True, biomes=ARAUCARIA, ground=GROUND,
    flee_range=6.0, drops=[("feather", 0, 1), ("pinhao", 0, 1)], xp=1, group=(2, 3), weight=7)

# Quati: ringed tail held high, a long pointed nose; noses through the forest floor in bands.
coati = ["#5a3e26", "#6e4c2e", "#845c38", "#9a6c42"]
m = quadruped((5, 5, 10), (5, 5, 5), 4, 2, look("qtbody", coati), look("qthead", coati, eyes=(EYE, None)),
              look("qtleg", ["#2a2018", "#3a2c20"]), snout=(2, 2, 3), snout_look=look("qtsnout", ["#2a2018", "#3a2c20"]),
              ears=(1, 2, 1))
m.box((2, 10, 2), (-1, 7, -5 - 2), look("qttail", ["#2a2018", "#3a2c20", "#c8a878", "#d8bc90"], pattern="rings"))
mob("quati", "Quati", m, health=8, speed=2.8, size=(0.5, 0.6), climbs=True, biomes=ARAUCARIA + MATA, ground=GROUND,
    flee_range=5.0, drops=[("leather", 0, 1)], xp=1, group=(2, 5), weight=7)

spec.EGGS.update({
    "capivara": ((130, 88, 52), (62, 42, 24)), "onca_pintada": ((216, 162, 58), (42, 28, 16)),
    "arara_azul": ((44, 72, 184), (240, 200, 40)), "tamandua": ((94, 82, 70), (216, 208, 192)),
    "lobo_guara": ((208, 96, 42), (20, 18, 16)), "jacare": ((42, 50, 32), (168, 160, 128)),
    "tuiuiu": ((232, 232, 228), (184, 30, 30)), "mico_leao_dourado": ((240, 158, 40), (58, 36, 24)),
    "tatu_bola": ((142, 116, 88), (192, 168, 136)), "ema": ((130, 128, 120), (192, 168, 120)),
    "asa_branca": ((140, 136, 146), (232, 232, 236)), "gralha_azul": ((42, 92, 192), (16, 16, 20)),
    "quati": ((110, 76, 46), (200, 168, 120)),
})
_ = Model
