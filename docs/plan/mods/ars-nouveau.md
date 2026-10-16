# Ars Nouveau (by baileyholl and team)

**What it is:** a magic mod built around **crafting your own spells**: a spell is a chain of glyphs, one **form** (how it's delivered:
projectile, touch, self, underfoot...), then **effects** (break, harm, ignite, freeze, heal, launch, summon, etc.) and **augments** that
modify the previous glyph (amplify, extend time, AOE, pierce, split...). Cast from a spellbook. Powered by **Source**, a magical energy
gathered by generator blocks, stored in jars and moved between machines. **Familiars** and magical helpers (e.g. small creatures that
harvest, carry or sort items) automate the base. **Rituals** are continuous area spells. Wearable trinkets, enchanting apparatus for
crafting and enchanting gear. Starts with a Worn Notebook guide (book + lapis).

**License:** page lists All Rights Reserved. → **Ideas only**, our own code, art and glyph names.

## How we build it (better than the Java version)
This is the best fit so far for our own magic system (`../magic-and-rpg.md`), so it is folded **into** it, not ported as a separate mod:
1. **Glyph spell crafting becomes a core feature**: our spell data format (form + effect blocks) is the same idea, so players can
   build custom spells in a spellbook UI on top of the premade spells from wands, staves and scrolls.
2. **Spells compile once** when crafted into a small instruction list (form → effects with augment values pre-applied), so casting is
   a lookup, not re-reading the glyph chain each time. Mana cost computed at craft time.
3. **Source = our mana network** for blocks: generators, jars and relays form a graph updated only when a node changes (same
   approach as the redstone graph), not every tick for every block.
4. **Helpers/familiars** are mobs built from the shared AI blocks (follow, pick up, deposit, harvest), with tasks given by
   pointing at containers; pathing results are cached and they sleep when idle.
5. **Rituals** are area effects registered as scheduled tasks on the region scheduler, ticking at low frequency (every 20–100 ticks).
6. Effects reuse the base systems (block breaking, damage, effects, summoning), so glyphs from other magic mods plug in through the
   spell registry (M2).

## Where it lands
- **P6**: glyph spell crafting, spellbook UI, mana network blocks (alongside the magic system).
- **P8**: helpers/familiars, rituals, trinkets.
- **M2**: glyph registry open to mods.
