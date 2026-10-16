"""Original calm ambient piano tracks for the game. Every melody here is generated from our own
progressions and seeds; nothing is transcribed from any existing song."""
import numpy as np, subprocess, sys, os, json

SR = 44100
OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)

def mtof(m): return 440.0 * 2 ** ((m - 69) / 12)

def piano(m, vel, dur, rng):
    f0 = mtof(m)
    length = min(dur + 3.5, 9.0)
    t = np.arange(int(length * SR)) / SR
    B = 0.00035
    sig = np.zeros_like(t)
    bright = 0.6 + 0.6 * vel
    for k in range(1, 12):
        fk = k * f0 * np.sqrt(1 + B * k * k)
        if fk > 12000: break
        amp = (1 / k ** (1.7 - 0.5 * vel)) * bright ** (k - 1)
        decay = 0.35 + 0.18 * k + f0 / 900
        ph = rng.uniform(0, 2 * np.pi)
        sig += amp * np.exp(-t * decay) * np.sin(2 * np.pi * fk * t + ph)
    # second string slightly detuned for chorus
    sig += 0.35 * np.exp(-t * 0.5) * np.sin(2 * np.pi * f0 * 1.0012 * t)
    att = np.minimum(1, t / 0.004)
    rel_start = dur
    rel = np.where(t < rel_start, 1.0, np.exp(-(t - rel_start) * 3.0))
    hammer = rng.normal(0, 1, len(t)) * np.exp(-t * 180) * 0.02
    return (sig * att * rel + hammer) * vel

def pad(m, vel, dur):
    t = np.arange(int((dur + 2) * SR)) / SR
    f0 = mtof(m)
    sig = sum(np.sin(2 * np.pi * f0 * r * t) * a for r, a in [(1, 1), (1.003, .7), (0.997, .7), (2, .25), (3, .08)])
    env = np.minimum(1, t / 2.5) * np.where(t < dur, 1, np.exp(-(t - dur) * 1.5))
    trem = 1 + 0.08 * np.sin(2 * np.pi * 0.15 * t)
    return sig * env * trem * vel * 0.12

def reverb_ir(seconds, rng, damp):
    n = int(seconds * SR)
    t = np.arange(n) / SR
    ir = rng.normal(0, 1, (2, n)) * np.exp(-t * (6.9 / seconds))
    # crude damping: smooth the tail progressively via cumulative low-pass
    for ch in range(2):
        y = np.zeros(n); a = damp
        x = ir[ch]
        # vectorized one-pole via lfilter-free approach (exponential smoothing by FFT kernel)
        kern = (1 - a) * a ** np.arange(200)
        ir[ch] = np.convolve(x, kern, mode="same")
    ir[:, 0] = 1.0
    return ir / np.abs(ir).sum(axis=1, keepdims=True) * 40

def fftconv(x, h):
    n = len(x) + len(h) - 1
    N = 1 << (n - 1).bit_length()
    return np.fft.irfft(np.fft.rfft(x, N) * np.fft.rfft(h, N), N)[:n]

SCALES = {
    "major": [0, 2, 4, 5, 7, 9, 11], "lydian": [0, 2, 4, 6, 7, 9, 11],
    "dorian": [0, 2, 3, 5, 7, 9, 10], "aeolian": [0, 2, 3, 5, 7, 8, 10],
    "phrygian": [0, 1, 3, 5, 7, 8, 10], "whole": [0, 2, 4, 6, 8, 10],
}

def chord_tones(root, quality):
    q = {"maj7": [0, 4, 7, 11], "m7": [0, 3, 7, 10], "add9": [0, 4, 7, 14], "6": [0, 4, 7, 9],
         "sus2": [0, 2, 7, 14], "m9": [0, 3, 7, 10, 14], "5": [0, 7, 12]}[quality]
    return [root + i for i in q]

def compose(spec):
    rng = np.random.default_rng(spec["seed"])
    beat = 60 / spec["bpm"]
    bar = beat * 4
    events = []  # (time, midi, vel, dur, inst)
    tonic = spec["tonic"]
    scale = [tonic + 12 * o + s for o in range(-1, 4) for s in SCALES[spec["scale"]]]
    prog = spec["prog"]
    sections = spec["form"]  # list of (kind, bars)
    t = 2.0
    motif = None
    for kind, nbars in sections:
        for b in range(nbars):
            root, q = prog[b % len(prog)]
            ch = chord_tones(tonic + root, q)
            bass = ch[0] - 12 if ch[0] - 12 >= 33 else ch[0]
            if kind in ("A", "B", "outro", "intro"):
                lh_vel = {"intro": .35, "outro": .3}.get(kind, .45)
                events.append((t, bass, lh_vel, bar * .95, "piano"))
                if kind != "intro" and rng.random() < .8:
                    for i, n in enumerate(ch[1:3]):
                        events.append((t + beat * (1 + i * .5) + rng.normal(0, .015), n, lh_vel * .8, bar * .7, "piano"))
            if spec.get("pad"):
                for n in ch[:3]:
                    events.append((t, n + 12 * spec.get("pad_oct", 0), .9, bar, "pad"))
            if kind in ("A", "B"):
                hi = [s for s in scale if spec["mel_lo"] <= s <= spec["mel_hi"]]
                if motif is None or (kind == "B" and b == 0):
                    motif = []
                    pos = 0.0
                    idx = rng.integers(len(hi) // 3, 2 * len(hi) // 3)
                    while pos < 8:  # two-bar motif in beats
                        d = rng.choice([1, 1, 1.5, 2, 2, 3, .5])
                        if rng.random() < spec["rest"]:
                            pos += d; continue
                        idx = int(np.clip(idx + rng.choice([-2, -1, -1, 1, 1, 2, 0, 3, -3]), 0, len(hi) - 1))
                        motif.append((pos, idx, d))
                        pos += d
                off = (b % 2) * 4
                shift = 0 if (b // 2) % 2 == 0 else rng.choice([-1, 1, 2])
                for pos, idx, d in motif:
                    if off <= pos < off + 4:
                        n = hi[int(np.clip(idx + shift, 0, len(hi) - 1))]
                        # pull melody toward chord tones on strong beats
                        if (pos - off) in (0, 2):
                            n = min(ch + [c + 12 for c in ch] + [c + 24 for c in ch], key=lambda c: abs(c - n))
                        events.append((t + (pos - off) * beat + rng.normal(0, .02), n,
                                       float(np.clip(.55 + rng.normal(0, .08), .3, .8)), d * beat * 1.4, "piano"))
            t += bar
        if kind == "A" and spec.get("breath", True):
            t += bar * rng.choice([0, .5, 1])
    total = t + 8
    mix = np.zeros((2, int(total * SR)))
    for (st, m, v, d, inst) in events:
        s = piano(m, v, d, rng) if inst == "piano" else pad(m, v, d)
        i0 = int(st * SR)
        i1 = min(i0 + len(s), mix.shape[1])
        pan = np.clip((m - 60) / 40, -.6, .6)
        mix[0, i0:i1] += s[: i1 - i0] * (1 - pan) * .5
        mix[1, i0:i1] += s[: i1 - i0] * (1 + pan) * .5
    ir = reverb_ir(spec["reverb"], rng, .6)
    wet = np.stack([fftconv(mix[c], ir[c])[: mix.shape[1]] for c in range(2)])
    out = mix * .7 + wet * spec.get("wet", .35)
    fade = int(6 * SR)
    out[:, -fade:] *= np.linspace(1, 0, fade)
    out /= np.abs(out).max() / 0.8
    return out, total

TRACKS = [
    dict(id="first_light", title="Primeira Luz", situation="overworld", seed=11, bpm=66, tonic=60, scale="lydian",
         prog=[(0, "maj7"), (9, "m7"), (5, "maj7"), (7, "6")], mel_lo=67, mel_hi=88, rest=.35, reverb=3.2,
         form=[("intro", 2), ("A", 8), ("B", 8), ("A", 8), ("outro", 3)]),
    dict(id="moss", title="Musgo", situation="overworld", seed=23, bpm=58, tonic=62, scale="major",
         prog=[(0, "add9"), (7, "sus2"), (9, "m7"), (5, "maj7")], mel_lo=69, mel_hi=86, rest=.45, reverb=3.6,
         form=[("intro", 2), ("A", 8), ("A", 8), ("B", 8), ("outro", 4)]),
    dict(id="still_water", title="Água Parada", situation="underwater", seed=31, bpm=54, tonic=63, scale="major",
         prog=[(0, "maj7"), (5, "add9"), (8, "maj7"), (7, "sus2")], mel_lo=70, mel_hi=87, rest=.5, reverb=4.5,
         pad=True, wet=.5, form=[("intro", 4), ("A", 8), ("B", 6), ("outro", 4)]),
    dict(id="block_by_block", title="Bloco a Bloco", situation="creative", seed=47, bpm=76, tonic=65, scale="major",
         prog=[(0, "add9"), (5, "maj7"), (9, "m7"), (7, "6")], mel_lo=72, mel_hi=91, rest=.25, reverb=2.6,
         form=[("intro", 2), ("A", 8), ("B", 8), ("A", 8), ("outro", 2)]),
    dict(id="embers", title="Brasa", situation="nether", seed=59, bpm=50, tonic=50, scale="phrygian",
         prog=[(0, "5"), (1, "maj7"), (0, "m7"), (10, "5")], mel_lo=57, mel_hi=74, rest=.5, reverb=4.0,
         pad=True, pad_oct=-1, wet=.45, form=[("intro", 4), ("A", 8), ("B", 6), ("outro", 4)]),
    dict(id="void", title="Vazio", situation="end", seed=71, bpm=48, tonic=64, scale="whole",
         prog=[(0, "sus2"), (2, "sus2"), (-2, "sus2"), (4, "5")], mel_lo=76, mel_hi=96, rest=.6, reverb=6.0,
         pad=True, wet=.6, form=[("intro", 4), ("A", 8), ("outro", 6)]),
    dict(id="beginning", title="Início", situation="title", seed=83, bpm=62, tonic=57, scale="dorian",
         prog=[(0, "m9"), (5, "add9"), (3, "maj7"), (10, "6")], mel_lo=64, mel_hi=84, rest=.35, reverb=3.4,
         form=[("intro", 2), ("A", 8), ("B", 8), ("outro", 4)]),
]

only = sys.argv[2:] or None
meta = []
for spec in TRACKS:
    if only and spec["id"] not in only: continue
    audio, secs = compose(spec)
    wav = os.path.join(OUT, spec["id"] + ".raw")
    (audio.T.astype(np.float32)).tofile(wav)
    ogg = os.path.join(OUT, spec["id"] + ".ogg")
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-f", "f32le", "-ar", str(SR), "-ac", "2", "-i", wav,
                    "-c:a", "libvorbis", "-q:a", "5", "-metadata", f"title={spec['title']}", ogg], check=True)
    os.remove(wav)
    meta.append({"id": spec["id"], "title": spec["title"], "situation": spec["situation"], "seconds": round(secs)})
    print(spec["id"], round(secs), "s", flush=True)
json.dump(meta, open(os.path.join(OUT, "tracks.json"), "w"), indent=1, ensure_ascii=False)
