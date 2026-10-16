# Game feel and animation polish

HP shared EnderVerse's video "TOP 20+ Minecraft Fresh Animation Mods (Ep 6)" (Oct 2026). These are the animation/feel ideas from it,
which we build **into the base game** (ideas only; each mod's license was not checked because we don't port any of them).
All of them run on the animation system from `mods/fresh-animations.md` (bone models, compiled expressions, GPU skinning, animation LOD).

| Idea (mod in the video) | What we build | Phase |
|---|---|---|
| Smooth player animations, Extra Animations | Full player animation set: smooth walk/run blend, turning, leaning into sprints, landing squash | P2 |
| Emotecraft, Reactions | Emote wheel with synced emotes; small reactions (flinch on hit, look at nearby items/players) | P7 |
| Touch'n Motion | Hands reach for doors, levers, ladders; body reacts to interacting | P8 |
| Kaleidoscope: Immersive Eating | Food visibly shrinks while eating, crumbs particles | P2 |
| Enchanted Fishing Line | Fishing line as a real rope curve with sag, glints when enchanted | P3 |
| SmoothMount, Smooth Camera | Camera smoothing when mounting/riding, optional cinematic camera | P2 |
| Better Mob Combat, Slashh, Spear Effects | Mob attack wind-ups, weapon trails, hit sparks (with Better Combat attack sets) | P2/P8 |
| Stupid Block Animations | Blocks pop slightly when placed, doors/chests ease open | P1 |
| Animated Hearts | HUD hearts/hunger shake, flash and fill smoothly | P2 |
| GUI Tween, Inventory Interactions | Screens slide/fade in, items move with tweening (shared with `mods/immersive-ui.md`) | P2 |
| InSight Chests | See a chest's contents animate as the lid opens | P8 |
| Juicy Drops | Dropped items bounce, spin and glow by rarity | P2 |
| Punchy-style resource packs | First-person view bob, sway and weapon animations, swappable through resource packs | P2/P8 |

**Performance rule:** every one of these is a cosmetic layer on the client, uses the GPU for the heavy part, and has an animation-LOD
or off switch. "Reduce motion" in accessibility turns all of them down at once.
