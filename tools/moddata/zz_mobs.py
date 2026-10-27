"""Creatures (Alex's Mobs, Naturalist, Mutant Monsters and Enderman Overhaul ideas): a spawn egg
for each creature of tools/mod_creatures.py and the few new things they drop. Egg sprites are
painted by tools/gen_mod_mob_skins.py, the drops' art by tools/modart/zz_mobs.py."""

import mod_creatures

from .lib import F, It, S

for _c in mod_creatures.CREATURES:
    It(f"{_c['name']}_spawn_egg", f"Ovo de {_c['display']}")

It("crocodile_scute", "Escama de Crocodilo")
It("banana_slug_slime", "Gosma de Lesma-Banana")
It("mosquito_proboscis", "Probóscide de Mosquito")
It("serpent_tooth", "Dente de Serpente")
It("snake_skin", "Pele de Cobra")

# A few uses: scutes and snake skin are tough leather, slug slime bakes into clay.
S(["crocodile_scute", "crocodile_scute", "crocodile_scute", "crocodile_scute"], "leather", 2)
S(["snake_skin", "snake_skin"], "leather")
S(["serpent_tooth"], "bone", 3)
S(["mosquito_proboscis", "mosquito_proboscis"], "string")
F("banana_slug_slime", "clay_ball", 0.1)
