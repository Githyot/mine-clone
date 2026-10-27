"""Bosses of Mass Destruction idea: the summon for the Nether Gauntlet (the lich, obsidilith and
void blossom summons are in the base game). The boss is made by tools/gen_bomd.py, its fight
in crates/game/src/bosses.rs, and the lairs in tools/gen_mod_structures.py (group bomd)."""

from .lib import It, R

It("gauntlet_core", "Núcleo da Manopla", max_stack="Some(1)")
R(["GBG", "BMB", "GBG"], {"G": "gold_block", "B": "blaze_rod", "M": "magma_block"}, "gauntlet_core")
