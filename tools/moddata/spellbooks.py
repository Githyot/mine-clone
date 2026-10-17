"""Iron's Spells 'n Spellbooks' idea: eleven schools of magic, single-use scrolls for every
spell, spellbooks with slots that spells are written into at the inscription table, ink,
wizard robes that raise mana and a school's power, and the Corrupted Archmage's seal. The
spells themselves (and the casting) are in crates/game/src/mods/spellbooks.rs; this list must
name the same spells in the same order."""

from .lib import B, It, R, S, armor, tex

# (spell id, name, the two things that go with a blank scroll and ink to make its scroll)
SPELLS = [
    ("fireball", "Bola de Fogo", "ruby", "gunpowder"),
    ("fire_breath", "Sopro de Fogo", "ruby", "blaze_rod"),
    ("ice_spike", "Estilhaço de Gelo", "sapphire", "flint"),
    ("frost_nova", "Nova Gélida", "sapphire", "packed_ice"),
    ("lightning_lance", "Lança Relâmpago", "topaz", "iron_ingot"),
    ("arc_beam", "Arco Elétrico", "topaz", "copper_ingot"),
    ("magic_missile", "Míssil Mágico", "amethyst", "feather"),
    ("arcane_barrier", "Barreira Arcana", "amethyst", "iron_ingot"),
    ("thorn_roots", "Raízes Espinhosas", "emerald", "vine"),
    ("nature_blessing", "Bênção da Natureza", "emerald", "apple"),
    ("storm_call", "Chamado da Tempestade", "topaz", "redstone"),
    ("gust", "Rajada de Vento", "feather", "string"),
    ("heal", "Cura", "gold_ingot", "apple"),
    ("sacred_light", "Luz Sagrada", "gold_ingot", "glowstone"),
    ("fortitude", "Fortaleza", "gold_ingot", "iron_ingot"),
    ("teleport", "Teletransporte", "ender_pearl", "feather"),
    ("void_rift", "Fenda do Vazio", "ender_pearl", "obsidian"),
    ("ender_orb", "Orbe do Fim", "ender_pearl", "chorus_fruit"),
    ("life_drain", "Drenar Vida", "rotten_flesh", "red_mushroom"),
    ("raise_dead", "Erguer os Mortos", "bone", "rotten_flesh"),
    ("blood_slash", "Corte Sangrento", "bone", "nether_wart"),
    ("abyss_ray", "Raio Abissal", "arcane_essence", "echo_crystal"),
    ("abyss_scream", "Grito do Abismo", "arcane_essence", "sculk_bone"),
    ("fangs", "Presas do Evocador", "emerald", "bone"),
    ("summon_wolves", "Invocar Lobos", "bone", "leather"),
    ("haste", "Pressa", "feather", "redstone"),
]

# Ink, blank scrolls and the essence the Archmage leaves behind.
It("blank_scroll", "Pergaminho em Branco")
It("arcane_ink", "Tinta Arcana")
It("arcane_essence", "Essência Arcana")
R(["WWW"], {"W": "wheat"}, "blank_scroll", 3)
S(["coal", "lapis_lazuli"], "arcane_ink", 2)
S(["charcoal", "lapis_lazuli"], "arcane_ink", 2)

for _id, _name, _a, _b in SPELLS:
    It(f"scroll_{_id}", f"Pergaminho: {_name}")
    S(["blank_scroll", "arcane_ink", _a, _b], f"scroll_{_id}")

# The inscription table, where scrolls are written into books (and two scrolls make a stronger one).
B("inscription_table", "Mesa de Inscrição", hardness="2.5", tool="Some(Axe)",
  textures=tex(top="inscription_table_top", side="inscription_table_side", bottom="oak_planks"))
R(["LIL", "PPP", "P P"], {"L": "lapis_lazuli", "I": "arcane_ink", "P": "#planks"}, "inscription_table")

# Spellbooks: the apprentice's is crafted; the iron and diamond ones are bound at the table
# from the one before (so the book keeps its spells); the Archmage's is his.
It("spellbook_apprentice", "Livro de Feitiços do Aprendiz", max_stack="Some(1)")
It("spellbook_iron", "Livro de Feitiços de Ferro", max_stack="Some(1)")
It("spellbook_diamond", "Livro de Feitiços de Diamante", max_stack="Some(1)")
It("spellbook_archmage", "Tomo do Arquimago", max_stack="Some(1)")
R([" L ", "BIB", " L "], {"L": "leather", "B": "blank_scroll", "I": "arcane_ink"}, "spellbook_apprentice")

# The seal that calls the Corrupted Archmage.
It("corrupted_seal", "Selo Arcano Corrompido")
R(["GEG", "EME", "GEG"], {"G": "gold_ingot", "E": "arcane_ink", "M": "mana_crystal"}, "corrupted_seal")

# Wizard robes, one set per school: (prefix, name, wool, reagent).
ROBES = [
    ("pyromancer", "Piromante", "red_wool", "ruby"),
    ("cryomancer", "Criomante", "light_blue_wool", "sapphire"),
    ("electromancer", "Eletromante", "yellow_wool", "topaz"),
    ("priest", "Sacerdote", "white_wool", "gold_ingot"),
    ("voidwalker", "Andarilho do Vazio", "black_wool", "ender_pearl"),
    ("bloodmage", "Mago de Sangue", "red_wool", "nether_wart"),
    ("cultist", "Cultista", "purple_wool", "echo_crystal"),
    ("archevoker", "Evocador", "gray_wool", "emerald"),
]
PIECES = (("helmet", 0, "Chapéu", ["XGX", "X X"], 2, 220),
          ("chestplate", 1, "Manto", ["XGX", "XXX", "XXX"], 5, 320),
          ("leggings", 2, "Calças", ["XGX", "X X", "X X"], 4, 300),
          ("boots", 3, "Botas", ["X X", "XGX"], 1, 260))
for _p, _name, _wool, _gem in ROBES:
    for _piece, _slot, _pt, _pat, _pts, _dur in PIECES:
        It(f"{_p}_{_piece}", f"{_pt} de {_name}", armor=armor(_slot, _pts, _dur))
        R(_pat, {"X": _wool, "G": _gem}, f"{_p}_{_piece}")
