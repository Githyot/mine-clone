# Music — how it plays (classic Minecraft style)

HP asked for the same music as old Minecraft, appearing the same way it did. **The tracks themselves can't be copied**: they are
copyrighted (C418's Volume Alpha/Beta and later composers' work), so shipping them, even re-encoded, would be infringement.
What we can do, and what this doc specifies, is reproduce **the way music appears**, which is what made it feel special, with our own
music in the same spirit.

## 1. How classic Minecraft played music (public behavior, Alpha → 1.12)

- **Mostly silence.** A track plays, then a long random pause, then another random track. Classic delay was roughly 10–20 minutes
  between tracks in game (12,000–24,000 ticks); this rarity is what made each track feel like a moment.
- **No triggers**: music did not react to combat, danger or biome. It just drifted in, quietly, at random.
- **Situations** (added over time): title screen music, in-game (survival) pool, a separate creative pool, underwater (1.13), Nether pool,
  End pool, boss fight music (dragon), credits music after the End.
- **Never two at once**, never a hard cut: tracks fade in and out; a jukebox disc or the credits stop the background music.
- **Music discs** (13, cat, blocks, chirp, far, mall, mellohi, stal, strad, ward, 11, wait...) are a separate system: found in dungeons
  or dropped when a skeleton kills a creeper, played in jukeboxes as positional sound.
- Later versions (1.18+) added **biome-specific pools** (e.g. meadow, cherry grove, deep dark) and a music frequency option.

## 2. Our music system

| Feature | Behavior | Default |
|---|---|---|
| Situations (data) | title, overworld-survival, overworld-creative, underwater, nether (per biome), end, boss, credits, + biome pools, + mod pools | — |
| Pick rule | random track from the current situation's pool, never the same track twice in a row | — |
| Pause between tracks | random between min and max delay, per situation | **classic: 10–20 min** |
| Frequency option | Classic (10–20 min) / Normal (5–10 min) / Frequent (1–3 min) / Off | Classic |
| Biome pools | Off = classic behavior (one pool for the whole Overworld); On = biome pools like modern versions | Off ("Classic music" mode) |
| Transitions | fade in 3 s, fade out 3 s when situation changes; jukebox ducks background music | — |
| Discs | positional jukebox playback, allays dance, parrots dance, comparator output by disc | — |

All of it is data (`packs/vanilla/music/*.ron`), so resource packs and mods can add tracks or situations.

## 3. Getting the music itself

Three options, in order of recommendation:
1. **Commission original music** in the same calm, ambient piano/synth style (many indie composers do this; this is how the game gets
   its own identity). Default plan.
2. **License tracks directly from the composer.** C418 owns his Minecraft albums and has licensed music before; we can ask. Needs a
   written license before anything ships.
3. **Free-licensed ambient music** (Creative Commons BY) as placeholders during development, swapped out before release.

Tech: Ogg Vorbis/Opus streaming from disk on the audio thread (never loaded fully into memory), so music costs ~0 CPU and a few MB.

## 4. First soundtrack (made, original)

Seven original tracks composed and rendered by us (procedural piano + pad + reverb, our own progressions and melodies; nothing
transcribed from any song). Files: `/mnt/project-files/assets/music/*.ogg`, list in `tracks.json`, generator `compose.py` (re-run to
regenerate or tweak). We own them, so they can ship.

| Track | Situation | Mood |
|---|---|---|
| Primeira Luz (`first_light`) | overworld | calm C lydian piano |
| Musgo (`moss`) | overworld | sparse D major |
| Água Parada (`still_water`) | underwater | piano + soft pad, long reverb |
| Bloco a Bloco (`block_by_block`) | creative | brighter F major |
| Brasa (`embers`) | nether | low, dark phrygian with drone |
| Vazio (`void`) | end | high, whole-tone, very spacious |
| Início (`beginning`) | title screen | D dorian, warm |

## 5. Player's own music

Options → Music → **Custom music folder**: the player picks a folder on their computer (default `<game folder>/music/`), and can
assign subfolders to situations (`overworld/`, `nether/`, `end/`, `creative/`, `title/`, `underwater/`). Formats: .ogg, .mp3, .flac, .wav.
Choices per situation: **Game only / My music only / Mix both**. The same random-pause rules apply (classic delay by default), and an
optional "shuffle now" key skips to the next track. Files are streamed from disk, never copied into the game. Resource packs can also
replace tracks (same as Minecraft).

## 6. Where it lands
- **P2**: music engine (situations, random delay, fades, frequency option), placeholder CC tracks.
- **P5**: Nether/End/boss/credits situations. **P8**: final soundtrack + discs.
