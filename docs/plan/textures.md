# Textures and texture packs

## 1. Our default look: as close to classic Minecraft as legally possible
HP wants the textures as close to old Minecraft as possible. Mojang's textures are copyrighted art, so we **can't copy or trace them**.
What we can (and will) match is the **style** that made them recognizable:
- **16×16 pixels per block face**, nearest-neighbor filtering (crisp pixels, no smoothing), no normal maps by default.
- **Limited palettes** per material (4–8 shades), hand-placed noise and dithering, same color families (grass green, dirt brown,
  stone grey, ore specks), classic-era contrast and saturation rather than the modern softer look.
- Same **readability rules**: ores are specks of color on the stone texture, logs show rings on top and bark on the sides, planks
  show 4 boards, cobblestone is chunky rounded stones.
- Biome tinting of grass/leaves/water via color maps (a technique, not art), so grass changes color by biome like the old game.
- Classic-style mob skins on simple box models (8×8 face, 64×32 skin layout) for the default look; Fresh-Animations-style smooth
  animation stays optional.
Art is drawn by us (or commissioned) to a written style guide; every texture gets a "not traced" check before it's merged.

## 2. Texture packs, like Minecraft
- **Resource packs folder** (`<game folder>/resourcepacks/`), packs as folders or .zip, stacked in priority order in an
  Options → Resource Packs screen (drag to reorder, same as Minecraft).
- A pack can replace any texture, model, sound, music track, font, language string or shader; anything it doesn't replace falls back
  to the pack below it.
- **Minecraft pack importer**: reads the standard Minecraft resource-pack layout (`assets/minecraft/textures/block/*.png`, item
  textures, `pack.mcmeta`, animated `.mcmeta` textures) and maps the file names to our blocks. So any pack a player already has,
  including ones they made or downloaded, works in our game on their own computer. We just never ship Mojang's files ourselves.
- Any resolution (16×, 32×, 64×, 512×) via the texture array, with automatic mipmaps; connected textures from `mods/continuity.md`.
- **Hot reload** (F3+T like Minecraft): textures re-upload to the GPU in under a second without restarting.

## 3. Performance
- All block textures live in one GPU texture array, so switching packs never changes the draw-call count.
- Packs are decoded on worker threads and cached (compressed GPU format) keyed by the pack's hash, so the second load is instant.

## 4. Where it lands
- **P0**: texture array loader, nearest filtering. **P1**: resource pack folder, stacking, hot reload, first classic-style texture set.
- **P3**: Minecraft pack importer. **P8**: complete texture set, mob skins, style guide finished.
