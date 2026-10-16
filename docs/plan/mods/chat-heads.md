# Chat Heads (by dzwdz, maintained by Fourmisain)

**What it is:** shows the sender's player head (face + hat layer) next to each chat message. Client-only. Has a "heuristic" mode that
guesses the sender from message text on servers that format chat in unusual ways.

**License:** MPL 2.0. → Code may be translated; files derived from it stay MPL 2.0 (file-level copyleft) and keep the notice. It's tiny, so we write it fresh.

## How we build it (better than the Java version)
Java chat messages are formatted text, so the mod has to guess who sent a message. Ours fixes the cause:
1. **Chat messages carry a structured sender ID** in the protocol (player, server, system, mod). No guessing, no wrong heads.
2. Head icons come from the skin texture cache already on the GPU (the 8×8 face + hat region), drawn in the same batched UI pass as the
   text. No extra textures, no extra draw calls.
3. Toggle in Options → Chat; on by default.

## Where it lands
- **P7** (chat and multiplayer).
