# JEI — Just Enough Items (by mezz)

**What it is:** the standard recipe viewer. Item list beside the inventory, search, press **R** for recipes / **U** for uses, browse
by recipe category, **[+]** moves a recipe's ingredients into the crafting grid, bookmarks, cheat mode (give items), hide items/mods,
info pages. Mods register their own recipe categories (machines, etc.) through its API.

**License:** MIT. → We may read and translate its code, with attribution kept in our credits. In practice we write it fresh
because our recipe data is already structured.

## How we build it (better than the Java version)
In Java, JEI has to discover recipes from every mod at startup and rebuild a huge search index, which is a well-known cause of slow
modpack loading. In ours it's built in:

1. **Native feature, not a mod**: every recipe type (crafting, smelting, stonecutting, smithing, brewing, trading, loot, mod machines)
   is already in our registries, so the viewer reads them directly. Mod recipe types register a display layout through the mod API.
2. **Search index built on a background thread** and cached on disk keyed by the pack/mod list hash: instant open on later launches.
   Index: lowercase names in all languages, mod ID (`@`), tooltip (`#`), tags (`$`), creative tab (`%`), with `-` exclude, quotes,
   and `|` OR, same syntax as JEI so players feel at home.
3. **Virtualized grid**: only icons on screen are drawn; item icons pre-rendered into a texture atlas once.
4. **Recipe graph** built at load: "uses" lookup is an O(1) reverse index instead of scanning.
5. **Recipe transfer [+]** done server-side (recipes live on the server since 1.21.2 in Java too), with the client showing missing items.
6. Cheat mode only when the world allows commands; bookmarks saved per world.

## Where it lands
- **P2** (crafting exists): recipe lookup R/U, item list, basic search.
- **P6**: all recipe types, bookmarks, recipe transfer, mod API for custom categories.
- **M2** (mod API): `register_recipe_category()` hook.
- Budget: search across 10,000 items < 2 ms per keystroke; viewer opens < 16 ms.
