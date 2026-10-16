#!/usr/bin/env python3
"""Original soundtrack generator for the game (chamber style: felt piano, bowed strings, solo cello, pads, bells).

Every chord progression, melody and counter-line is produced by the rules below from fixed seeds; nothing is
transcribed from any existing song. The style reference is "quiet piano with warm strings", not any specific work.

Usage:  python3 tools/compose_music.py [OUT_DIR] [track_id ...]
        OUT_DIR defaults to assets/music next to this repo. Needs numpy, scipy and ffmpeg (libvorbis).
"""
import json
import os
import subprocess
import sys

import numpy as np
from scipy import signal
from scipy.ndimage import minimum_filter1d

SR = 44100
TWO_PI = 2 * np.pi


# ----------------------------------------------------------------------------------------------------------------------
# small DSP helpers
# ----------------------------------------------------------------------------------------------------------------------
def mtof(m):
    return 440.0 * 2 ** ((np.asarray(m, dtype=float) - 69) / 12)


def smoothstep(x):
    x = np.clip(x, 0, 1)
    return x * x * (3 - 2 * x)


def lowpass(x, fc, order=2):
    fc = min(fc, SR * 0.45)
    sos = signal.butter(order, fc / (SR / 2), output="sos")
    return signal.sosfilt(sos, x)


def highpass(x, fc, order=2):
    sos = signal.butter(order, max(fc, 5.0) / (SR / 2), btype="high", output="sos")
    return signal.sosfilt(sos, x)


def bandpass(x, lo, hi, order=2):
    sos = signal.butter(order, [lo / (SR / 2), min(hi, SR * 0.45) / (SR / 2)], btype="band", output="sos")
    return signal.sosfilt(sos, x)


def peaking_eq(x, fc, gain_db, q=1.0):
    """RBJ peaking biquad applied along the last axis."""
    a = 10 ** (gain_db / 40)
    w0 = TWO_PI * fc / SR
    alpha = np.sin(w0) / (2 * q)
    b = [1 + alpha * a, -2 * np.cos(w0), 1 - alpha * a]
    aa = [1 + alpha / a, -2 * np.cos(w0), 1 - alpha / a]
    return signal.lfilter(np.array(b) / aa[0], np.array(aa) / aa[0], x, axis=-1)


def high_shelf(x, fc, gain_db):
    a = 10 ** (gain_db / 40)
    w0 = TWO_PI * fc / SR
    alpha = np.sin(w0) / (2 * 0.707)
    cw = np.cos(w0)
    sq = 2 * np.sqrt(a) * alpha
    b = [a * ((a + 1) + (a - 1) * cw + sq), -2 * a * ((a - 1) + (a + 1) * cw), a * ((a + 1) + (a - 1) * cw - sq)]
    aa = [(a + 1) - (a - 1) * cw + sq, 2 * ((a - 1) - (a + 1) * cw), (a + 1) - (a - 1) * cw - sq]
    return signal.lfilter(np.array(b) / aa[0], np.array(aa) / aa[0], x, axis=-1)


def env_asr(n, attack, release, dur, swell=0.0, peak_at=0.6):
    """Attack / sustain (with optional swell) / release envelope of `n` samples for a note of `dur` seconds."""
    t = np.arange(n) / SR
    att = smoothstep(t / max(attack, 1e-3))
    rel = np.where(t < dur, 1.0, np.exp(-(t - dur) / max(release, 1e-3) * 3.0))
    if swell > 0:
        x = np.clip(t / max(dur, 1e-3), 0, 1)
        bump = np.exp(-((x - peak_at) ** 2) / 0.12)
        env = (1 - swell) + swell * bump
    else:
        env = 1.0
    return att * rel * env


# ----------------------------------------------------------------------------------------------------------------------
# instruments (all return mono float arrays at SR)
# ----------------------------------------------------------------------------------------------------------------------
class Synth:
    def __init__(self, rng):
        self.rng = rng
        self.piano_cache = {}

    # -- felt piano ----------------------------------------------------------------------------------------------------
    def piano(self, midi, vel, dur, felt=True):
        key = (midi, round(vel, 1), round(min(dur, 6.0) * 2) / 2)
        if key in self.piano_cache:
            return self.piano_cache[key]
        rng = self.rng
        f0 = float(mtof(midi))
        dur = min(dur, 6.0)
        length = min(dur + 4.0, 11.0)
        n = int(length * SR)
        t = np.arange(n) / SR
        B = 0.0003 + 0.0006 * max(0, (midi - 60) / 30)  # inharmonicity grows in the treble
        sig = np.zeros(n)
        kmax = int(min(18, 9500 / f0))
        bright = 0.55 + 0.45 * vel
        for k in range(1, kmax + 1):
            fk = k * f0 * np.sqrt(1 + B * k * k)
            amp = (1 / k ** (1.5 - 0.3 * vel)) * bright ** (k - 1)
            if felt:
                amp *= np.exp(-0.10 * (k - 1) * (1.3 - vel))  # felt layer removes high partials
            decay = 0.45 + 0.22 * k + f0 / 650
            beat = (0.25 + 0.9 * rng.random()) * (1 + 0.15 * k)  # two/three strings slowly beating
            ph = rng.uniform(0, TWO_PI)
            sig += amp * np.exp(-t * decay) * np.sin(TWO_PI * fk * t + ph) * (1 + 0.3 * np.cos(TWO_PI * beat * t + ph))
        attack = 0.006 + 0.012 * (1 - vel) if felt else 0.003
        att = smoothstep(t / attack)
        damper = np.where(t < dur, 1.0, np.exp(-(t - dur) * 4.0))
        hammer = lowpass(rng.normal(0, 1, n), 1500 + 2500 * vel) * np.exp(-t * 120) * 0.05 * (0.4 + vel)
        symp = bandpass(rng.normal(0, 1, n), 200, 2500) * np.exp(-t * 2.5) * 0.004
        out = (sig * att * damper + hammer + symp * att) * vel
        self.piano_cache[key] = out
        return out

    # -- bowed strings -------------------------------------------------------------------------------------------------
    PARTS = {  # rolloff exponent, lowpass multiple of f0, cap Hz, bow noise, body resonance Hz, noise band
        "violin": (1.00, 16, 9000, 0.011, 1000, (1500, 6000)),
        "viola": (1.12, 13, 6500, 0.009, 600, (1000, 4500)),
        "cello": (1.22, 11, 5000, 0.006, 300, (700, 3500)),
        "bass": (1.40, 8, 2500, 0.004, 150, (400, 2500)),
    }

    def bowed(self, midi, dur, vel, part="violin", voices=3, attack=0.5, release=0.6, vib_rate=5.5, vib_cents=10.0,
              vib_delay=0.45, glide_from=None, glide_time=0.12, reverse=False, swell=0.0, detune_cents=7.0,
              harmonics=False, grace=None, pizz=False):
        rng = self.rng
        rolloff, lp_mult, lp_cap, noise_amt, body, nband = self.PARTS[part]
        f0 = float(mtof(midi))
        if pizz:
            length = min(dur, 0.3) + min(2.5, 60 / f0 + 0.6)
        else:
            length = dur + release * 2.2 + 0.1
        n = int(length * SR)
        t = np.arange(n) / SR
        sig = np.zeros(n)
        # frequency curve: portamento, grace note, vibrato
        base = np.full(n, f0)
        if glide_from is not None and not pizz:
            fa = float(mtof(glide_from))
            g = smoothstep(t / glide_time)
            base = fa * (f0 / fa) ** g
        if grace is not None and not pizz:  # quick lower/upper neighbour before the main note (fiddle ornament)
            fg = float(mtof(grace))
            g = smoothstep((t - 0.045) / 0.035)
            base = fg * (base / fg) ** g
        if harmonics:
            vib_cents *= 0.6
            voices = 1
        for v in range(voices):
            det = 2 ** (rng.normal(0, detune_cents / 2) / 1200) if voices > 1 else 1.0
            vr = vib_rate * (1 + rng.normal(0, 0.04)) + 0.15 * np.sin(TWO_PI * 0.3 * t + rng.uniform(0, TWO_PI))
            vdepth = vib_cents * smoothstep((t - vib_delay * (0.8 + 0.4 * rng.random())) / 0.5) * (1 + 0.15 * rng.normal())
            if pizz:
                vdepth = 0 * vdepth
            vib = 2 ** (vdepth * np.sin(TWO_PI * np.cumsum(vr) / SR + rng.uniform(0, TWO_PI)) / 1200)
            freq = base * det * vib
            phase = TWO_PI * np.cumsum(freq) / SR
            fmax = float(freq.max())
            cutoff = min(lp_cap, lp_mult * f0)
            kmax = int(max(1, min(60, min(SR * 0.45, cutoff * 3.5) / fmax)))
            if harmonics:
                kmax = 3
            voice = np.zeros(n)
            for k in range(1, kmax + 1):
                amp = 1 / k ** rolloff
                if harmonics:
                    amp = [1.0, 0.12, 0.03][k - 1]
                amp *= 1 + 0.08 * rng.normal()
                if k % 2 == 0:
                    amp *= 0.85
                voice += amp * np.sin(k * phase + rng.uniform(0, TWO_PI))
            if not harmonics:
                voice = lowpass(voice, cutoff, 2)
                # body resonance adds a little "wood"
                voice = peaking_eq(voice, body, 3.0, 1.2)
            sig += voice / voices
        if pizz:
            env = smoothstep(t / 0.004) * np.exp(-t * (2.2 + f0 / 220))
            pluck = lowpass(rng.normal(0, 1, n), 3000) * np.exp(-t * 200) * 0.08
            out = (lowpass(sig, min(4000, f0 * 8)) * env + pluck) * vel
        else:
            noise = bandpass(rng.normal(0, 1, n), *nband) * noise_amt * (1 + 0.5 * np.exp(-t / 0.2)) * (0.6 + vel)
            if reverse:
                env = np.exp((t - dur) * (4.0 / max(dur, 0.5))) * (t <= dur) * smoothstep((dur - t) / 0.03)
            else:
                env = env_asr(n, attack, release, dur, swell=swell)
            out = (sig * (0.85 + 0.15 * vel) + noise) * env * vel
        return out

    # -- pad -----------------------------------------------------------------------------------------------------------
    def pad(self, midi, dur, vel, attack=2.5, release=3.0):
        rng = self.rng
        f0 = float(mtof(midi))
        n = int((dur + release * 2.2) * SR)
        t = np.arange(n) / SR
        sig = np.zeros(n)
        for r in (1.0, 1.004, 0.996, 2.0, 2.003, 3.0):
            amp = {1.0: 1, 1.004: .7, .996: .7, 2.0: .25, 2.003: .2, 3.0: .07}[r]
            sig += amp * np.sin(TWO_PI * f0 * r * t + rng.uniform(0, TWO_PI))
        sweep = 400 + 900 * (0.5 + 0.5 * np.sin(TWO_PI * 0.07 * t + rng.uniform(0, TWO_PI)))
        # slow filter movement by crossfading two static lowpasses
        lo, hi = lowpass(sig, 500), lowpass(sig, 1800)
        mixw = (sweep - 400) / 900
        sig = lo * (1 - mixw) + hi * mixw
        trem = 1 + 0.06 * np.sin(TWO_PI * 0.13 * t + rng.uniform(0, TWO_PI))
        return sig * env_asr(n, attack, release, dur) * trem * vel * 0.25

    # -- celesta / music box -------------------------------------------------------------------------------------------
    def bell(self, midi, vel, music_box=False):
        rng = self.rng
        f0 = float(mtof(midi))
        n = int(4.0 * SR)
        t = np.arange(n) / SR
        if music_box:
            parts = [(1.0, 1.0, 1.4), (2.76, 0.25, 3.5), (5.40, 0.07, 6.0), (8.93, 0.02, 9.0)]
        else:
            parts = [(1.0, 1.0, 1.2), (2.0, 0.08, 3.0), (3.01, 0.25, 2.5), (5.43, 0.06, 5.0), (7.1, 0.02, 7.0)]
        sig = np.zeros(n)
        for r, a, d in parts:
            if f0 * r > 15000:
                continue
            sig += a * np.exp(-t * d) * np.sin(TWO_PI * f0 * r * t + rng.uniform(0, TWO_PI))
        click = lowpass(rng.normal(0, 1, n), 6000) * np.exp(-t * 400) * 0.03
        return (sig * smoothstep(t / 0.002) + click) * vel * 0.6

    # -- sub drone -----------------------------------------------------------------------------------------------------
    def drone(self, midi, dur, vel):
        rng = self.rng
        f0 = float(mtof(midi))
        n = int((dur + 6) * SR)
        t = np.arange(n) / SR
        lfo = 1 + 0.25 * np.sin(TWO_PI * 0.05 * t + rng.uniform(0, TWO_PI))
        sig = np.sin(TWO_PI * f0 * t) + 0.35 * np.sin(TWO_PI * f0 * 2 * t + 0.3) * lfo + 0.15 * np.sin(TWO_PI * f0 * 3.0 * t)
        sig += 0.05 * lowpass(rng.normal(0, 1, n), 120)  # air
        return sig * env_asr(n, 4.0, 5.0, dur) * vel * 0.5


# ----------------------------------------------------------------------------------------------------------------------
# music theory
# ----------------------------------------------------------------------------------------------------------------------
SCALES = {
    "major": [0, 2, 4, 5, 7, 9, 11], "lydian": [0, 2, 4, 6, 7, 9, 11], "mixolydian": [0, 2, 4, 5, 7, 9, 10],
    "dorian": [0, 2, 3, 5, 7, 9, 10], "aeolian": [0, 2, 3, 5, 7, 8, 10], "phrygian": [0, 1, 3, 5, 7, 8, 10],
    "locrian": [0, 1, 3, 5, 6, 8, 10], "whole": [0, 2, 4, 6, 8, 10], "lydaug": [0, 2, 4, 6, 8, 9, 11],
    "pent": [0, 2, 4, 7, 9], "harmmin": [0, 2, 3, 5, 7, 8, 11],
}
CHORDS = {
    "maj": [0, 4, 7], "min": [0, 3, 7], "maj7": [0, 4, 7, 11], "m7": [0, 3, 7, 10], "add9": [0, 4, 7, 14],
    "madd9": [0, 3, 7, 14], "6": [0, 4, 7, 9], "m6": [0, 3, 7, 9], "sus2": [0, 2, 7], "sus4": [0, 5, 7],
    "7sus4": [0, 5, 7, 10], "m9": [0, 3, 7, 10, 14], "maj9": [0, 4, 7, 11, 14], "5": [0, 7], "mMaj7": [0, 3, 7, 11],
    "maj7#11": [0, 4, 7, 11, 18], "dim": [0, 3, 6], "m7b5": [0, 3, 6, 10], "aug": [0, 4, 8], "7": [0, 4, 7, 10],
    "cluster": [0, 1, 7, 8], "sus2b9": [0, 1, 2, 7],
}


class Key:
    def __init__(self, tonic, scale):
        self.tonic = tonic
        self.scale = SCALES[scale]

    def notes(self, lo, hi):
        return [n for n in range(lo, hi + 1) if (n - self.tonic) % 12 in self.scale]

    def snap(self, n):
        """Nearest scale note."""
        for d in (0, 1, -1, 2, -2):
            if (n + d - self.tonic) % 12 in self.scale:
                return n + d
        return n


class Chord:
    def __init__(self, key, root_off, quality):
        self.root = key.tonic + root_off
        self.ivs = CHORDS[quality]
        self.pcs = {(self.root + i) % 12 for i in self.ivs}
        self.quality = quality

    def tones(self, lo, hi):
        return [n for n in range(lo, hi + 1) if n % 12 in self.pcs]

    def nearest_tone(self, n, prefer=None):
        cands = self.tones(n - 7, n + 7)
        if prefer is not None:
            cands = [c for c in cands if c % 12 in prefer] or cands
        return min(cands, key=lambda c: (abs(c - n), c))

    def pc(self, iv_index):
        return (self.root + self.ivs[iv_index % len(self.ivs)]) % 12


# ----------------------------------------------------------------------------------------------------------------------
# composition
# ----------------------------------------------------------------------------------------------------------------------
class Score:
    """Collects note events on named buses. Times are in seconds."""

    def __init__(self):
        self.events = []  # dicts: bus, inst, t, midi, vel, dur, pan, extra

    def add(self, bus, inst, t, midi, vel, dur, pan=0.0, **extra):
        self.events.append(dict(bus=bus, inst=inst, t=float(t), midi=int(midi), vel=float(np.clip(vel, 0.05, 1.0)),
                                dur=float(max(dur, 0.05)), pan=float(np.clip(pan, -1, 1)), extra=extra))


class TempoMap:
    """Beats -> seconds with rubato: slow sine drift, phrase-end breathing, section factors, final ritardando."""

    def __init__(self, bpm, meter, sections, rng):
        self.meter = meter
        res = 16  # subdivisions per beat
        total_beats = sum(s["bars"] for s in sections) * meter
        grid = np.arange(total_beats * res + 1) / res
        factor = np.ones_like(grid)
        beat0 = 0
        for s in sections:
            nb = s["bars"] * meter
            sl = (grid >= beat0) & (grid < beat0 + nb + 1e-9)
            f = s.get("tempo", 1.0)
            if s.get("rit"):
                x = (grid[sl] - beat0) / nb
                factor[sl] *= f * (1 - 0.28 * smoothstep(x))
            else:
                factor[sl] *= f
            beat0 += nb
        bars = grid / meter
        factor *= 1 + 0.03 * np.sin(TWO_PI * bars / 6.5 + rng.uniform(0, TWO_PI))
        # breathe at the end of every 4-bar phrase
        phrase_pos = (bars % 4) / 4
        factor *= 1 - 0.07 * np.exp(-((phrase_pos - 0.97) ** 2) / 0.0015)
        self.bpm = bpm * factor
        dt = 60.0 / self.bpm / res
        self.t = np.concatenate([[0.0], np.cumsum(dt[:-1])])
        self.grid = grid

    def time(self, beat):
        return float(np.interp(beat, self.grid, self.t))

    def beat_len(self, beat):
        return float(60.0 / np.interp(beat, self.grid, self.bpm))


RHYTHM_CELLS = {
    4: [[1, 1, 2], [2, 1, 1], [1.5, .5, 2], [1, 1, 1, 1], [3, 1], [2, 2], [1, .5, .5, 2], [.5, .5, 1, 2], [1, 2, 1]],
    3: [[1, 1, 1], [2, 1], [1, 2], [1.5, 1.5], [1, .5, .5, 1], [3], [.5, .5, 1, 1]],
}


class Composer:
    def __init__(self, spec):
        self.spec = spec
        self.rng = np.random.default_rng(spec["seed"])
        self.meter = spec.get("meter", 4)
        self.score = Score()
        self.sections = spec["form"]
        self.tempo = TempoMap(spec["bpm"], self.meter, self.sections, self.rng)
        self.motifs = {}

    # -- melody machinery ----------------------------------------------------------------------------------------------
    def make_motif(self, bars, rest_prob, leap_prob):
        rng, meter = self.rng, self.meter
        rhythm = []
        pos = 0.0
        while pos < bars * meter - 1e-6:
            cell = RHYTHM_CELLS[meter][rng.integers(len(RHYTHM_CELLS[meter]))]
            for d in cell:
                if pos >= bars * meter - 1e-6:
                    break
                d = min(d, bars * meter - pos)
                if rng.random() < rest_prob and pos > 0:
                    pos += d
                    continue
                rhythm.append((pos, d))
                pos += d
        steps = []
        prev = 0
        for i in range(len(rhythm)):
            if abs(prev) >= 3:  # resolve a leap by step in the opposite direction
                s = -int(np.sign(prev)) * int(rng.choice([1, 1, 2]))
            elif rng.random() < leap_prob:
                s = int(rng.choice([-4, -3, 3, 4, 5]))
            else:
                s = int(rng.choice([-2, -1, -1, 0, 1, 1, 2]))
            steps.append(s)
            prev = s
        return rhythm, steps, bars

    def motif(self, name, bars=2, rest_prob=0.25, leap_prob=0.15):
        if name not in self.motifs:
            self.motifs[name] = self.make_motif(bars, rest_prob, leap_prob)
        return self.motifs[name]

    def realize_phrase(self, key, chords, bar0, motif, lo, hi, start_idx=None, variation=None, dyn=0.6):
        """Place a motif twice (statement + varied answer) over `len(chords)` bars.
        Returns list of (beat, dur_beats, midi, vel)."""
        rng, meter = self.rng, self.meter
        notes_in = key.notes(lo, hi)
        if not notes_in:
            return []
        rhythm, steps, mbars = motif
        nbars = len(chords)
        out = []
        if start_idx is None:
            start_idx = len(notes_in) // 2
        idx = start_idx
        reps = max(1, nbars // mbars)
        for rep in range(reps):
            r, s = rhythm, list(steps)
            if rep % 2 == 1 or variation:
                op = variation or rng.choice(["transpose", "invert", "ornament", "transpose"])
                if op == "invert":
                    s = [-x for x in s]
                elif op == "transpose":
                    idx += int(rng.choice([-2, -1, 1, 2]))
                elif op == "ornament":
                    r = [(p, d) for p, d in rhythm]
                    if len(r) > 1:
                        i = int(rng.integers(len(r)))
                        p, d = r[i]
                        if d >= 1.5:
                            r = r[:i] + [(p, d / 2), (p + d / 2, d / 2)] + r[i + 1:]
                            s = s[:i + 1] + [int(rng.choice([-1, 1]))] + s[i + 1:]
            total = len(r)
            for i, ((p, d), st) in enumerate(zip(r, s)):
                beat_in_phrase = rep * mbars * meter + p
                bar = int(beat_in_phrase // meter)
                if bar >= nbars:
                    break
                chord = chords[bar]
                idx = idx + st
                # keep in range by reflecting
                if idx < 0:
                    idx = -idx
                if idx >= len(notes_in):
                    idx = 2 * (len(notes_in) - 1) - idx
                idx = int(np.clip(idx, 0, len(notes_in) - 1))
                n = notes_in[idx]
                strong = (p % meter) == 0 or (meter == 4 and p % meter == 2)
                last = (rep == reps - 1 and i == total - 1)
                if last:
                    n = chord.nearest_tone(n, prefer={chord.pc(0), chord.pc(1), chord.pc(2)})
                elif strong and rng.random() < 0.8:
                    n = chord.nearest_tone(n)
                if n in notes_in:
                    idx = notes_in.index(n)
                arc = 0.78 + 0.3 * np.sin(np.pi * (beat_in_phrase / (nbars * meter)) ** 0.8)
                vel = dyn * arc * (1.08 if strong else 0.95) * (1 + rng.normal(0, 0.05))
                dur_b = d * (1.6 if last else 1.0)
                out.append((bar0 * meter + beat_in_phrase, dur_b, n, vel))
        return out

    # -- accompaniment -------------------------------------------------------------------------------------------------
    def piano_lh(self, chord, bar, pattern, dyn, prev_voicing):
        meter, rng, tm = self.meter, self.rng, self.tempo
        b0 = bar * meter
        bass = chord.root
        while bass > 50:
            bass -= 12
        while bass < 36:
            bass += 12
        pv = (list(prev_voicing) + [55, 59])[:2] if prev_voicing else [55, 59]
        tones = [chord.nearest_tone(pv[i], prefer={chord.pc(i + 1)}) for i in range(2)]
        tones = sorted({max(48, min(67, x)) for x in tones})
        if len(tones) < 2:
            tones = sorted(set(tones) | {chord.nearest_tone(tones[0] + 4)})
        upper = chord.tones(bass + 7, bass + 19)
        bar_len = tm.time(b0 + meter) - tm.time(b0)
        events = []  # (beat, midi, vel, dur_sec)
        if pattern == "block":
            events.append((b0, bass, dyn, bar_len * 1.1))
            for i, n in enumerate(tones):
                events.append((b0 + 0.02 * i, n, dyn * 0.75, bar_len * 1.0))
            if meter == 4 and rng.random() < 0.6:
                events.append((b0 + 2, upper[0] if upper else tones[0], dyn * 0.6, bar_len * 0.5))
        elif pattern == "sparse":
            events.append((b0, bass, dyn, bar_len * 1.2))
            if rng.random() < 0.7:
                events.append((b0 + rng.choice([1.5, 2, 2.5]), tones[-1], dyn * 0.6, bar_len * 0.6))
        elif pattern == "arp":
            seq = [bass, tones[0], tones[-1], upper[0] if upper else tones[0] + 12]
            seq = seq + [seq[2], seq[1]] + seq[2:4]
            step = meter / len(seq) if meter == 3 else 0.5
            for i, n in enumerate(seq[: int(meter / step)]):
                events.append((b0 + i * step, n, dyn * (0.9 if i == 0 else 0.55 + 0.08 * (i % 2)), bar_len * 0.5))
        elif pattern == "broken":
            events.append((b0, bass, dyn, bar_len * 1.1))
            for i, n in enumerate([tones[0], tones[-1], (upper[0] if upper else tones[0] + 12)][: meter - 1]):
                events.append((b0 + 1 + i, n, dyn * 0.6, bar_len * 0.6))
        elif pattern == "waltz":
            events.append((b0, bass, dyn, bar_len * 0.9))
            for bt in (1, 2):
                for n in tones:
                    events.append((b0 + bt + 0.01 * (n % 3), n, dyn * 0.55, bar_len * 0.35))
        elif pattern == "octaves":
            events.append((b0, bass, dyn, bar_len * 1.1))
            events.append((b0 + 0.01, bass + 12, dyn * 0.6, bar_len * 1.1))
            for i, n in enumerate(tones):
                events.append((b0 + meter / 2 + 0.02 * i, n, dyn * 0.7, bar_len * 0.6))
        for beat, n, vel, dur in events:
            t = tm.time(beat) + rng.normal(0, 0.012)
            self.score.add("piano", "piano", t, n, vel, dur, pan=np.clip((n - 60) / 36, -0.5, 0.5))
        return tones

    def string_chord(self, chord, bar, dyn, prev, parts=("bass", "cello", "viola", "violin2", "violin1"), overlap=1.08,
                     swell=0.3, bars=1, melody_top=None, last=False):
        """Sustained string section voicing with simple voice leading."""
        meter, rng, tm = self.meter, self.rng, self.tempo
        b0 = bar * meter
        t0 = tm.time(b0)
        dur = (tm.time(b0 + meter * bars) - t0) * (1.3 if last else overlap)
        if last:
            dyn *= 0.8
        ranges = {"bass": (31, 48), "cello": (40, 60), "viola": (52, 72), "violin2": (60, 81), "violin1": (67, 91)}
        pans = {"bass": 0.15, "cello": 0.35, "viola": -0.1, "violin2": -0.35, "violin1": -0.5}
        voicing = {}
        for part in parts:
            lo, hi = ranges[part]
            if part == "bass":
                cands = [n for n in chord.tones(lo, hi) if n % 12 == chord.pc(0)]
                n = cands[0] if cands else chord.tones(lo, hi)[0]
            elif part == "cello":
                n = chord.nearest_tone(prev.get(part, 48), prefer={chord.pc(0), chord.pc(2)})
            elif part == "violin1" and melody_top is not None:
                n = melody_top
            else:
                used = {v % 12 for v in voicing.values()}
                pref = set(chord.pcs) - used or chord.pcs
                n = chord.nearest_tone(prev.get(part, (lo + hi) // 2), prefer=pref)
            n = int(np.clip(n, lo, hi))
            voicing[part] = n
            inst = "violin" if "violin" in part else part
            vel = dyn * {"bass": 0.9, "cello": 0.95, "viola": 0.85, "violin2": 0.8, "violin1": 0.85}[part]
            self.score.add("strings", "bowed", t0 + rng.normal(0.02, 0.03), n, vel, dur, pan=pans[part], part=inst,
                           attack=0.6 + 0.6 * rng.random() + (0.4 if part == "bass" else 0), release=3.0 if last else 0.9, swell=swell,
                           voices=3 if part != "bass" else 2, vib_cents=8 if "violin" in part else 6, vib_delay=0.6)
        return voicing

    def cello_counterpoint(self, key, chord, bar, melody_notes, dyn, prev_note, lo=38, hi=62):
        """One or two long notes per bar: contrary motion to the melody, consonant on strong beats, smooth leading."""
        meter, rng, tm = self.meter, self.rng, self.tempo
        b0 = bar * meter
        notes = []
        slots = [0] if meter == 3 or rng.random() < 0.45 else [0, meter / 2]
        for s in slots:
            beat = b0 + s
            mel_here = [m for (bt, d, m, v) in melody_notes if bt <= beat + 0.01]
            mel_next = [m for (bt, d, m, v) in melody_notes if bt > beat + 0.01]
            direction = 0
            if mel_here and mel_next:
                direction = int(np.sign(mel_next[0] - mel_here[-1]))
            target = prev_note - 2 * direction + int(rng.normal(0, 1.5))
            cands = [n for n in chord.tones(lo, hi)]
            if mel_here:
                top = mel_here[-1]
                cands = [n for n in cands if (top - n) % 12 in (3, 4, 7, 8, 9, 0) and n < top] or cands
            cands = [n for n in cands if abs(n - prev_note) <= 7] or cands
            n = min(cands, key=lambda c: abs(c - target))
            dur_b = (meter - s) if len(slots) == 1 or s > 0 else meter / 2
            notes.append((beat, dur_b, n))
            prev_note = n
        for i, (beat, dur_b, n) in enumerate(notes):
            t0 = tm.time(beat)
            dur = tm.time(beat + dur_b) - t0
            glide = prev_note if (i == 0 and rng.random() < 0.5) else None
            self.score.add("cello", "bowed", t0 + rng.normal(0.03, 0.02), n, dyn * (1 + rng.normal(0, 0.05)), dur * 1.03,
                           pan=0.3, part="cello", voices=1, attack=0.25 + 0.3 * rng.random(), release=0.5, swell=0.35,
                           vib_cents=12, vib_delay=0.35, glide_from=glide, glide_time=0.12, detune_cents=3)
        return prev_note

    # -- the piece -----------------------------------------------------------------------------------------------------
    def compose(self):
        spec, rng, meter, tm, sc = self.spec, self.rng, self.meter, self.tempo, self.score
        bar = 0
        prev_voicing = None
        prev_strings = {}
        cello_prev = 50
        mel_inst_default = spec.get("melody_inst", "piano")
        last_melody_idx = None
        for si, sec in enumerate(self.sections):
            key = Key(spec["tonic"] + sec.get("transpose", 0), sec.get("scale", spec["scale"]))
            prog = sec.get("prog", spec["prog"])
            layers = set(sec["layers"])
            dyn = sec["dyn"]
            nb = sec["bars"]
            chords = [Chord(key, *prog[i % len(prog)]) for i in range(nb)]
            lo, hi = sec.get("mel_range", spec["mel_range"])
            mel_inst = sec.get("melody_inst", mel_inst_default)
            # --- drone / pad for the whole section
            if "drone" in layers:
                root = key.tonic
                while root > 43:
                    root -= 12
                t0 = tm.time(bar * meter)
                sc.add("drone", "drone", t0 - 1.0, root, dyn * 0.9, tm.time((bar + nb) * meter) - t0 + 1.0, pan=0)
            if "pad" in layers:
                for b in range(0, nb, 2):
                    ch = chords[b]
                    t0 = tm.time((bar + b) * meter)
                    dur = tm.time((bar + min(b + 2, nb)) * meter) - t0
                    for i, n in enumerate(ch.tones(52 + spec.get("pad_oct", 0) * 12, 76 + spec.get("pad_oct", 0) * 12)[:4]):
                        sc.add("pad", "pad", t0 - 0.5, n, dyn * 0.8, dur + 1.0, pan=(-0.6 + 0.4 * i))
            # --- melody for the section (4-bar phrases)
            melody = []
            if "melody" in layers:
                motif_name = sec.get("motif", "A")
                mot = self.motif(motif_name, bars=2, rest_prob=spec.get("rest", 0.25), leap_prob=spec.get("leap", 0.15))
                for p0 in range(0, nb, 4):
                    pch = chords[p0:p0 + 4]
                    var = None if (p0 // 4) % 2 == 0 else rng.choice(["transpose", "invert", "ornament"])
                    if sec.get("octave_up"):
                        lo2, hi2 = lo + 12, hi + 12
                    else:
                        lo2, hi2 = lo, hi
                    notes_in = key.notes(lo2, hi2)
                    start = last_melody_idx if last_melody_idx is not None else len(notes_in) // 2
                    start = int(np.clip(start + int(rng.integers(-1, 2)), 2, max(2, len(notes_in) - 3)))
                    phrase = self.realize_phrase(key, pch, bar + p0, mot, lo2, hi2, start_idx=start, variation=var, dyn=dyn)
                    if phrase:
                        last_melody_idx = notes_in.index(phrase[-1][2]) if phrase[-1][2] in notes_in else start
                    melody += phrase
                prev_m = None
                for i, (beat, dur_b, n, vel) in enumerate(melody):
                    t0 = tm.time(beat)
                    next_beat = melody[i + 1][0] if i + 1 < len(melody) else beat + dur_b
                    dur = tm.time(min(beat + dur_b, next_beat + 0.1)) - t0
                    if mel_inst == "piano":
                        sc.add("piano", "piano", t0 + rng.normal(0, 0.015), n, vel, dur * 1.15, pan=np.clip((n - 60) / 40, -0.4, 0.5))
                        if sec.get("double_violin"):
                            sc.add("strings", "bowed", t0 + 0.03 + rng.normal(0, 0.02), n + 12, vel * 0.55, dur * 1.05,
                                   pan=-0.4, part="violin", voices=3, attack=0.15, release=0.4, vib_cents=10, vib_delay=0.25)
                    elif mel_inst == "cello":
                        legato = prev_m is not None and abs(n - prev_m) <= 7 and rng.random() < 0.6
                        sc.add("cello", "bowed", t0 + rng.normal(0.02, 0.015), n, vel, dur * 1.04, pan=0.25, part="cello",
                               voices=1, attack=0.12 + 0.2 * rng.random(), release=0.45, swell=0.3, vib_cents=14,
                               vib_delay=0.3, glide_from=prev_m if legato else None, glide_time=0.1 + 0.08 * rng.random(),
                               detune_cents=3)
                    elif mel_inst == "fiddle":
                        grace = n - 2 if rng.random() < 0.22 and dur_b >= 1 else None
                        slide = prev_m if (prev_m is not None and rng.random() < 0.35 and abs(n - prev_m) <= 5) else None
                        sc.add("cello", "bowed", t0 + rng.normal(0.01, 0.012), n, vel, dur * 1.02, pan=-0.2, part="violin",
                               voices=1, attack=0.05 + 0.05 * rng.random(), release=0.25, swell=0.15, vib_cents=16,
                               vib_rate=6.0, vib_delay=0.18, glide_from=slide, glide_time=0.07, grace=grace, detune_cents=4)
                    elif mel_inst == "bells":
                        sc.add("bells", "bell", t0 + rng.normal(0, 0.02), n, vel * 0.8, dur, pan=0.4, music_box=spec.get("music_box", False))
                    elif mel_inst == "violin":
                        sc.add("cello", "bowed", t0 + rng.normal(0.02, 0.015), n, vel, dur * 1.04, pan=-0.3, part="violin",
                               voices=1, attack=0.2 + 0.3 * rng.random(), release=0.5, swell=0.3, vib_cents=12, vib_delay=0.35,
                               glide_from=prev_m if (prev_m is not None and rng.random() < 0.4) else None, detune_cents=3)
                    prev_m = n
            # --- bar by bar accompaniment
            for b in range(nb):
                ch = chords[b]
                gb = bar + b
                if "lh" in layers:
                    prev_voicing = self.piano_lh(ch, gb, sec.get("lh", "block"), dyn * 0.8, prev_voicing)
                if "strings" in layers:
                    parts = sec.get("parts", ("bass", "cello", "viola", "violin2", "violin1"))
                    if b % sec.get("string_bars", 1) == 0:
                        sb = sec.get("string_bars", 1)
                        nxt = self.sections[si + 1] if si + 1 < len(self.sections) else None
                        is_last = b + sb >= nb and (nxt is None or "strings" not in nxt["layers"])
                        prev_strings = self.string_chord(ch, gb, dyn * sec.get("string_dyn", 0.8), prev_strings, parts=parts,
                                                         swell=0.35 if dyn > 0.6 else 0.25, bars=sb, last=is_last)
                if "counter" in layers:
                    bar_mel = [m for m in melody if gb * meter <= m[0] < (gb + 1) * meter]
                    cello_prev = self.cello_counterpoint(key, ch, gb, bar_mel, dyn * 0.75, cello_prev)
                if "pizz" in layers:
                    pattern = [0, 2] if meter == 4 else [0]
                    if sec.get("pizz_busy"):
                        pattern = [0, 1, 2, 3] if meter == 4 else [0, 1, 2]
                    for i, bt in enumerate(pattern):
                        n = ch.root
                        while n > 50:
                            n -= 12
                        if i % 2 == 1:
                            n = ch.nearest_tone(n + 7, prefer={ch.pc(2), ch.pc(1)})
                        t0 = tm.time(gb * meter + bt)
                        sc.add("pizz", "bowed", t0 + rng.normal(0.01, 0.012), n, dyn * (0.9 if i == 0 else 0.65), 0.3,
                               pan=0.3, part="cello", voices=1, pizz=True)
                if "bells" in layers and "melody" in layers and mel_inst != "bells":
                    # sparse arpeggiated bells on top, answering the melody at phrase ends
                    if b % 4 == 3 or rng.random() < spec.get("bell_prob", 0.25):
                        tones = ch.tones(79, 96)
                        start_beat = gb * meter + rng.choice([0, meter / 2, meter - 1])
                        for i in range(int(rng.integers(2, 5))):
                            n = tones[int(rng.integers(len(tones)))] if tones else 84
                            sc.add("bells", "bell", tm.time(start_beat + i * 0.5) + rng.normal(0, 0.02), n,
                                   dyn * 0.5 * (1 + rng.normal(0, 0.1)), 1.0, pan=0.45 - 0.15 * i,
                                   music_box=spec.get("music_box", False))
                if "swell" in layers and (b % 4 == 3 or (rng.random() < 0.2 and "melody" not in layers)):
                    # reverse string swell landing on the next downbeat
                    target = tm.time((gb + 1) * meter)
                    sw_dur = 1.8 + 2.5 * rng.random()
                    nxt = chords[(b + 1) % nb]
                    for n in nxt.tones(55, 79)[:3]:
                        sc.add("fx", "bowed", target - sw_dur + rng.normal(0, 0.03), n, dyn * 0.6, sw_dur, pan=rng.uniform(-0.7, 0.7),
                               part="viola", voices=3, reverse=True, release=0.2, vib_cents=4)
                if "harmonics" in layers and rng.random() < 0.5:
                    tones = ch.tones(86, 100)
                    if tones:
                        n = tones[int(rng.integers(len(tones)))]
                        t0 = tm.time(gb * meter + rng.choice([0, 1, 2]))
                        sc.add("fx", "bowed", t0, n, dyn * 0.5, 2.0 + 3 * rng.random(), pan=rng.uniform(-0.8, 0.8), part="violin",
                               harmonics=True, attack=1.2, release=1.5, swell=0.5, vib_cents=6, vib_delay=0.8)
                if "dissonance" in layers and b % 2 == 1:
                    n = ch.root + 12 + int(rng.choice([1, 6, 13]))
                    while n > 64:
                        n -= 12
                    t0 = tm.time(gb * meter + 1)
                    sc.add("fx", "bowed", t0, n, dyn * 0.45, 2.5, pan=rng.uniform(-0.5, 0.5), part="cello", voices=3,
                           attack=1.5, release=1.2, swell=0.6, vib_cents=5, detune_cents=12)
                if "bells_solo" in layers and b % 2 == 0:
                    tones = ch.tones(72, 93)
                    for i in range(int(rng.integers(1, 4))):
                        n = tones[int(rng.integers(len(tones)))]
                        sc.add("bells", "bell", tm.time(gb * meter + i * rng.choice([1, 1.5, 2])) + rng.normal(0, 0.03), n,
                               dyn * 0.6, 1.0, pan=rng.uniform(-0.5, 0.5), music_box=spec.get("music_box", False))
            bar += nb
        total_beats = bar * meter
        end = tm.time(total_beats)
        return end


# ----------------------------------------------------------------------------------------------------------------------
# rendering, effects, mastering
# ----------------------------------------------------------------------------------------------------------------------
def make_reverb_ir(seconds, rng, bright=0.5):
    n = int(seconds * SR)
    t = np.arange(n) / SR
    ir = np.zeros((2, n))
    bands = [(20, 300, 1.15), (300, 2500, 1.0), (2500, 12000, 0.45 + 0.3 * bright)]
    for ch in range(2):
        noise = rng.normal(0, 1, n)
        acc = np.zeros(n)
        for lo, hi, mult in bands:
            band = bandpass(noise, lo, hi, 2)
            acc += band * np.exp(-6.91 * t / (seconds * mult))
        # early reflections
        early = np.zeros(n)
        for _ in range(10):
            d = int(rng.uniform(0.008, 0.07) * SR)
            early[d] += rng.uniform(0.2, 0.6) * rng.choice([-1, 1])
        acc[: len(early)] += lowpass(early, 5000) * 0.6
        acc[: int(0.012 * SR)] = 0  # predelay
        ir[ch] = acc
    ir /= np.sqrt((ir ** 2).sum(axis=1, keepdims=True)) / 3.0
    return ir


def render(spec, verbose=True):
    comp = Composer(spec)
    end = comp.compose()
    synth = Synth(np.random.default_rng(spec["seed"] + 1))
    rng = np.random.default_rng(spec["seed"] + 2)
    tail = spec.get("reverb", 4.0) + 3.0
    total = int((end + tail) * SR)
    buses = {b: np.zeros((2, total)) for b in ("piano", "strings", "cello", "pad", "bells", "drone", "pizz", "fx")}
    sorted_events = sorted(comp.score.events, key=lambda e: e["t"])
    for e in sorted_events:
        inst = e["inst"]
        if inst == "piano":
            s = synth.piano(e["midi"], e["vel"], e["dur"])
        elif inst == "bowed":
            s = synth.bowed(e["midi"], e["dur"], e["vel"], **e["extra"])
        elif inst == "pad":
            s = synth.pad(e["midi"], e["dur"], e["vel"])
        elif inst == "bell":
            s = synth.bell(e["midi"], e["vel"], **e["extra"])
        elif inst == "drone":
            s = synth.drone(e["midi"], e["dur"], e["vel"])
        else:
            continue
        i0 = int(max(e["t"], 0.0) * SR)
        if i0 >= total:
            continue
        i1 = min(i0 + len(s), total)
        seg = s[: i1 - i0]
        pan = e["pan"]
        gl, gr = np.cos((pan + 1) * np.pi / 4), np.sin((pan + 1) * np.pi / 4)
        bus = buses[e["bus"]]
        bus[0, i0:i1] += seg * gl
        bus[1, i0:i1] += seg * gr
        # stereo spread for sections: a short decorrelating delay on the opposite channel
        if e["bus"] in ("strings", "pad"):
            d = int(rng.uniform(0.004, 0.012) * SR)
            if i1 + d <= total:
                bus[1 if pan < 0 else 0, i0 + d:i1 + d] += seg * 0.35 * (gr if pan < 0 else gl)
    gains = dict(piano=0.9, strings=0.55, cello=0.6, pad=0.7, bells=0.5, drone=0.9, pizz=0.8, fx=0.5)
    gains.update(spec.get("gains", {}))
    sends = dict(piano=0.35, strings=0.5, cello=0.4, pad=0.6, bells=0.6, drone=0.1, pizz=0.3, fx=0.8)
    dry = np.zeros((2, total))
    send = np.zeros((2, total))
    for name, b in buses.items():
        if not np.any(b):
            continue
        if name == "strings":
            b = highpass(b, 60) ; b = peaking_eq(b, 350, -2.0, 1.0)
        if name == "piano":
            b = peaking_eq(b, 250, -1.5, 0.9)
        if name == "pad":
            b = highpass(b, 90)
        dry += b * gains[name]
        send += b * gains[name] * sends[name]
    ir = make_reverb_ir(spec.get("reverb", 4.0), np.random.default_rng(spec["seed"] + 3), bright=spec.get("rev_bright", 0.5))
    send = highpass(send, 160)
    send = lowpass(send, 6500)
    wet = np.stack([signal.fftconvolve(send[c], ir[c])[:total] for c in range(2)])
    # a little cross-feed makes the room feel wider
    wet = np.stack([wet[0] * 0.85 + wet[1] * 0.15, wet[1] * 0.85 + wet[0] * 0.15])
    out = dry + wet * spec.get("wet", 0.5)
    # master EQ
    out = highpass(out, 28, 2)
    out = peaking_eq(out, 240, -1.5, 0.8)
    out = high_shelf(out, 7000, 1.0)
    if spec.get("master_lowpass"):
        out = lowpass(out, spec["master_lowpass"], 2)
    # gentle stereo widening (mid/side)
    mid, side = (out[0] + out[1]) / 2, (out[0] - out[1]) / 2
    side = highpass(side, 200) * 1.25
    out = np.stack([mid + side, mid - side])
    # fades
    fin = int(1.5 * SR)
    out[:, :fin] *= np.linspace(0, 1, fin) ** 2
    fout = int(min(7.0, tail) * SR)
    out[:, -fout:] *= np.linspace(1, 0, fout) ** 1.5
    # limiter: lookahead/hold minimum filter on the required gain, then smoothed; finally -1 dBFS peak
    peak = np.max(np.abs(out), axis=0)
    rms = np.sqrt(np.mean(out ** 2))
    out *= 0.16 / max(rms, 1e-9)  # loudness target before limiting (~ -16 dBFS RMS)
    peak = np.max(np.abs(out), axis=0)
    thr = 0.85
    g = np.minimum(1.0, thr / np.maximum(peak, 1e-9))
    g = minimum_filter1d(g, size=int(0.006 * SR))
    g = signal.lfilter([1 - 0.9995], [1, -0.9995], g)  # release smoothing
    g = minimum_filter1d(g, size=int(0.004 * SR))
    g = lowpass(g, 400, 1)
    out *= np.minimum(g, 1.0)
    out = np.tanh(out / 0.95) * 0.95  # soft safety
    out *= (10 ** (-1 / 20)) / max(np.max(np.abs(out)), 1e-9)
    if not np.all(np.isfinite(out)):
        raise RuntimeError("non-finite samples in " + spec["id"])
    return out.astype(np.float32), total / SR, len(comp.score.events)


def analyze(audio, name):
    mono = audio.mean(axis=0)
    peak = float(np.max(np.abs(audio)))
    rms = float(np.sqrt(np.mean(audio ** 2)))
    win = 5 * SR
    cents = []
    for i in range(0, len(mono) - win, win):
        seg = mono[i:i + win]
        spec = np.abs(np.fft.rfft(seg * np.hanning(win)))
        freqs = np.fft.rfftfreq(win, 1 / SR)
        c = float((spec * freqs).sum() / max(spec.sum(), 1e-9))
        cents.append(round(c))
    seg_rms = [round(float(20 * np.log10(np.sqrt(np.mean(mono[i:i + win] ** 2)) + 1e-9)), 1) for i in range(0, len(mono) - win, win)]
    print(f"  {name}: {len(mono) / SR:.1f}s peak={20 * np.log10(peak):.2f}dBFS rms={20 * np.log10(rms):.1f}dBFS "
          f"clip={int((np.abs(audio) >= 0.999).sum())} nan={int((~np.isfinite(audio)).sum())}")
    print("   centroid/5s (Hz):", cents)
    print("   rms/5s (dBFS):   ", seg_rms)


# ----------------------------------------------------------------------------------------------------------------------
# the tracks
# ----------------------------------------------------------------------------------------------------------------------
def S(name, bars, dyn, layers, **kw):
    d = dict(name=name, bars=bars, dyn=dyn, layers=layers)
    d.update(kw)
    return d


TRACKS = [
    dict(id="first_light", title="Primeira Luz", situation="overworld", seed=1101, bpm=63, tonic=60, scale="lydian",
         prog=[(0, "maj9"), (9, "m7"), (5, "maj7"), (7, "6")], mel_range=(67, 86), rest=0.3, reverb=4.2, wet=0.45,
         form=[S("intro", 4, 0.4, ["lh", "pad"], lh="sparse"),
               S("theme", 8, 0.55, ["lh", "melody", "counter"], lh="broken"),
               S("theme2", 8, 0.65, ["lh", "melody", "counter", "strings"], lh="broken", parts=("cello", "viola", "violin2"), string_dyn=0.6),
               S("dev", 8, 0.6, ["lh", "melody", "strings", "counter"], lh="arp", transpose=4, scale="major", motif="B",
                 prog=[(0, "maj7"), (5, "add9"), (9, "m7"), (7, "sus4")]),
               S("climax", 8, 0.85, ["lh", "melody", "strings", "bells", "double_violin"], lh="octaves", double_violin=True,
                 octave_up=False, string_dyn=0.95, tempo=1.02),
               S("outro", 6, 0.4, ["lh", "melody", "pad"], lh="sparse", rit=True, motif="A")]),
    dict(id="moss", title="Musgo", situation="overworld", seed=2302, bpm=54, tonic=62, scale="dorian", melody_inst="cello",
         prog=[(0, "madd9"), (10, "maj7"), (5, "m7"), (7, "7sus4")], mel_range=(48, 67), rest=0.35, leap=0.1, reverb=4.6, wet=0.5,
         form=[S("intro", 4, 0.4, ["lh", "drone"], lh="sparse"),
               S("theme", 8, 0.55, ["lh", "melody"], lh="sparse"),
               S("theme2", 8, 0.6, ["lh", "melody", "strings"], lh="broken", parts=("viola", "violin2", "violin1"), string_dyn=0.55, string_bars=2),
               S("dev", 8, 0.65, ["lh", "melody", "strings", "swell"], lh="block", transpose=0, scale="major", motif="B",
                 prog=[(0, "add9"), (4, "min"), (9, "m7"), (5, "maj7")], parts=("bass", "viola", "violin2", "violin1"), string_dyn=0.7),
               S("climax", 8, 0.8, ["lh", "melody", "strings"], lh="octaves", melody_inst="piano", mel_range=(67, 86), motif="A",
                 string_dyn=0.9, parts=("bass", "cello", "viola", "violin2", "violin1")),
               S("outro", 6, 0.4, ["lh", "melody", "pad"], lh="sparse", melody_inst="cello", rit=True)]),
    dict(id="still_water", title="Água Parada", situation="underwater", seed=3103, bpm=48, tonic=63, scale="major",
         prog=[(0, "maj9"), (5, "add9"), (8, "maj7"), (7, "sus2")], mel_range=(70, 88), rest=0.5, reverb=6.5, wet=0.7,
         master_lowpass=2600, rev_bright=0.3, pad_oct=-1, music_box=True, gains=dict(bells=0.7, pad=0.9),
         form=[S("intro", 4, 0.45, ["pad", "drone"]),
               S("theme", 8, 0.5, ["pad", "melody", "bells_solo"], melody_inst="bells", motif="A"),
               S("theme2", 8, 0.55, ["pad", "melody", "lh", "strings"], lh="sparse", melody_inst="piano", parts=("cello", "viola"), string_bars=2, string_dyn=0.6),
               S("dev", 8, 0.6, ["pad", "melody", "strings", "swell", "drone"], transpose=5, scale="lydian", motif="B",
                 prog=[(0, "maj7#11"), (7, "add9"), (2, "m9"), (5, "maj7")], melody_inst="cello", mel_range=(50, 66), parts=("viola", "violin2"), string_bars=2),
               S("climax", 6, 0.7, ["pad", "melody", "lh", "strings", "bells"], lh="sparse", melody_inst="piano", string_bars=2,
                 parts=("bass", "cello", "viola", "violin2", "violin1"), string_dyn=0.75),
               S("outro", 6, 0.4, ["pad", "melody", "drone"], melody_inst="bells", rit=True)]),
    dict(id="block_by_block", title="Bloco a Bloco", situation="creative", seed=4704, bpm=76, tonic=65, scale="major",
         prog=[(0, "add9"), (5, "maj7"), (9, "m7"), (7, "6")], mel_range=(72, 91), rest=0.25, leap=0.2, reverb=2.8, wet=0.35,
         gains=dict(bells=0.65, pizz=0.9),
         form=[S("intro", 4, 0.45, ["pizz", "lh"], lh="sparse"),
               S("theme", 12, 0.6, ["pizz", "lh", "melody"], lh="arp", melody_inst="bells"),
               S("theme2", 8, 0.65, ["pizz", "lh", "melody", "counter", "bells"], lh="arp", melody_inst="piano"),
               S("dev", 8, 0.6, ["pizz", "lh", "melody", "strings"], lh="broken", transpose=2, motif="B", pizz_busy=True,
                 prog=[(0, "maj7"), (9, "m7"), (2, "m7"), (7, "sus4")], parts=("viola", "violin2"), string_dyn=0.5, melody_inst="bells"),
               S("climax", 12, 0.8, ["pizz", "lh", "melody", "strings", "bells"], lh="arp", pizz_busy=True, double_violin=True,
                 parts=("cello", "viola", "violin2", "violin1"), string_dyn=0.8),
               S("outro", 4, 0.45, ["lh", "melody", "pizz"], lh="sparse", melody_inst="bells", rit=True)]),
    dict(id="embers", title="Brasa", situation="nether", seed=5905, bpm=46, tonic=50, scale="phrygian", melody_inst="cello",
         prog=[(0, "5"), (1, "maj7"), (0, "m7"), (10, "5")], mel_range=(43, 62), rest=0.45, reverb=5.5, wet=0.6, pad_oct=-1,
         rev_bright=0.35, gains=dict(drone=1.1, fx=0.6),
         form=[S("intro", 4, 0.45, ["drone", "dissonance"]),
               S("theme", 8, 0.5, ["drone", "melody", "lh"], lh="sparse", mel_range=(43, 60)),
               S("theme2", 8, 0.55, ["drone", "melody", "strings", "dissonance"], parts=("bass", "cello", "viola"), string_bars=2, string_dyn=0.6),
               S("dev", 8, 0.65, ["drone", "melody", "strings", "swell", "lh"], lh="sparse", transpose=-1, scale="locrian", motif="B",
                 prog=[(0, "m7b5"), (1, "maj7"), (6, "5"), (0, "cluster")], parts=("bass", "cello", "viola", "violin2"), string_bars=2),
               S("climax", 8, 0.8, ["drone", "melody", "strings", "dissonance", "lh"], lh="octaves", melody_inst="piano",
                 mel_range=(55, 74), parts=("bass", "cello", "viola", "violin2", "violin1"), string_dyn=0.9),
               S("outro", 6, 0.4, ["drone", "melody", "dissonance"], rit=True)]),
    dict(id="void", title="Vazio", situation="end", seed=7106, bpm=42, tonic=64, scale="lydaug",
         prog=[(0, "sus2"), (2, "aug"), (-2, "sus2"), (4, "maj7#11")], mel_range=(76, 96), rest=0.6, reverb=7.5, wet=0.75,
         pad_oct=0, music_box=True, gains=dict(fx=0.7, bells=0.6, pad=0.6),
         form=[S("intro", 4, 0.4, ["swell", "harmonics", "drone"]),
               S("theme", 8, 0.5, ["swell", "harmonics", "melody", "pad"], melody_inst="bells"),
               S("theme2", 8, 0.55, ["harmonics", "melody", "lh", "swell"], lh="sparse", melody_inst="piano", mel_range=(72, 92), motif="A"),
               S("dev", 8, 0.6, ["swell", "harmonics", "strings", "melody"], transpose=6, scale="whole", motif="B",
                 prog=[(0, "aug"), (2, "sus2"), (4, "aug"), (-2, "sus2")], melody_inst="violin", mel_range=(72, 90), parts=("viola", "violin2"), string_bars=2),
               S("climax", 6, 0.7, ["strings", "harmonics", "melody", "swell", "bells"], melody_inst="piano", mel_range=(72, 92),
                 parts=("cello", "viola", "violin2", "violin1"), string_bars=2, string_dyn=0.75),
               S("outro", 6, 0.4, ["harmonics", "swell", "melody", "drone"], melody_inst="bells", rit=True)]),
    dict(id="beginning", title="Início", situation="title", seed=8307, bpm=60, tonic=57, scale="dorian",
         prog=[(0, "m9"), (5, "add9"), (3, "maj7"), (10, "6")], mel_range=(64, 84), rest=0.3, leap=0.18, reverb=4.4, wet=0.5,
         form=[S("intro", 4, 0.45, ["lh", "strings"], lh="sparse", parts=("cello", "viola"), string_dyn=0.5, string_bars=2),
               S("theme", 8, 0.6, ["lh", "melody", "counter", "strings"], lh="block", parts=("viola", "violin2"), string_dyn=0.55),
               S("theme2", 8, 0.7, ["lh", "melody", "strings"], lh="broken", melody_inst="violin", parts=("bass", "cello", "viola", "violin2"), string_dyn=0.75),
               S("dev", 8, 0.65, ["lh", "melody", "strings", "counter"], lh="arp", transpose=3, scale="major", motif="B",
                 prog=[(0, "maj7"), (9, "m7"), (5, "add9"), (7, "sus4")], parts=("cello", "viola", "violin2", "violin1"), string_dyn=0.7),
               S("climax", 8, 0.9, ["lh", "melody", "strings", "bells"], lh="octaves", double_violin=True, string_dyn=1.0,
                 parts=("bass", "cello", "viola", "violin2", "violin1"), tempo=1.03),
               S("outro", 6, 0.45, ["lh", "melody", "strings"], lh="sparse", parts=("cello", "viola"), string_dyn=0.5, string_bars=2, rit=True)]),
    dict(id="wandering", title="Andarilho", situation="overworld", seed=9108, bpm=72, tonic=67, scale="mixolydian",
         prog=[(0, "add9"), (10, "maj7"), (5, "6"), (7, "m7")], mel_range=(67, 86), rest=0.3, leap=0.18, reverb=3.6, wet=0.42,
         form=[S("intro", 4, 0.45, ["lh"], lh="arp"),
               S("theme", 8, 0.6, ["lh", "melody", "counter"], lh="arp"),
               S("theme2", 8, 0.65, ["lh", "melody", "strings"], lh="arp", melody_inst="violin", parts=("cello", "viola"), string_dyn=0.6, string_bars=2),
               S("dev", 8, 0.6, ["lh", "melody", "counter", "strings"], lh="broken", transpose=3, scale="major", motif="B",
                 prog=[(0, "maj9"), (7, "6"), (9, "m7"), (5, "add9")], parts=("viola", "violin2"), string_dyn=0.55),
               S("climax", 8, 0.85, ["lh", "melody", "strings", "bells", "counter"], lh="arp", double_violin=True,
                 parts=("bass", "cello", "viola", "violin2", "violin1"), string_dyn=0.9),
               S("outro", 6, 0.45, ["lh", "melody", "pad"], lh="sparse", rit=True)]),
    dict(id="harvest", title="Colheita", situation="overworld", seed=1009, bpm=100, meter=3, tonic=62, scale="major",
         melody_inst="fiddle", prog=[(0, "maj"), (7, "sus4"), (9, "m7"), (5, "6"), (0, "maj"), (2, "m7"), (7, "7"), (0, "add9")],
         mel_range=(62, 84), rest=0.2, leap=0.2, reverb=2.6, wet=0.35, gains=dict(cello=0.7, pizz=0.85),
         form=[S("intro", 8, 0.5, ["lh", "pizz"], lh="waltz"),
               S("theme", 16, 0.65, ["lh", "melody", "pizz"], lh="waltz"),
               S("theme2", 16, 0.7, ["lh", "melody", "pizz", "strings"], lh="waltz", parts=("viola", "violin2"), string_dyn=0.5, string_bars=2),
               S("dev", 16, 0.6, ["lh", "melody", "strings", "counter"], lh="waltz", transpose=-3, scale="aeolian", motif="B",
                 prog=[(0, "min"), (8, "maj7"), (3, "add9"), (10, "6"), (0, "madd9"), (5, "m7"), (7, "min"), (0, "min")],
                 parts=("cello", "viola"), string_dyn=0.6, string_bars=2),
               S("climax", 16, 0.85, ["lh", "melody", "pizz", "strings"], lh="waltz", parts=("bass", "cello", "viola", "violin2"),
                 string_dyn=0.85, tempo=1.03),
               S("outro", 8, 0.45, ["lh", "melody", "pizz"], lh="waltz", rit=True)]),
]


def encode(audio, path, title, max_bytes=4_000_000):
    raw = path + ".raw"
    audio.T.astype(np.float32).tofile(raw)
    for q in (5, 4, 3):
        subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-f", "f32le", "-ar", str(SR), "-ac", "2", "-i", raw,
                        "-c:a", "libvorbis", "-q:a", str(q), "-metadata", f"title={title}", path], check=True)
        if os.path.getsize(path) <= max_bytes:
            break
    os.remove(raw)
    return os.path.getsize(path)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    out_dir = args[0] if args and (os.path.isdir(args[0]) or "/" in args[0]) else os.path.join(here, "..", "assets", "music")
    only = args[1:] if args and out_dir == args[0] else args
    os.makedirs(out_dir, exist_ok=True)
    meta_path = os.path.join(out_dir, "tracks.json")
    meta = {m["id"]: m for m in json.load(open(meta_path))} if os.path.exists(meta_path) else {}
    for spec in TRACKS:
        if only and spec["id"] not in only:
            continue
        print(f"[{spec['id']}] composing...", flush=True)
        audio, secs, nev = render(spec)
        ogg = os.path.join(out_dir, spec["id"] + ".ogg")
        size = encode(audio, ogg, spec["title"])
        analyze(audio, spec["id"])
        print(f"   {nev} notes, {size / 1e6:.2f} MB -> {ogg}", flush=True)
        meta[spec["id"]] = {"id": spec["id"], "title": spec["title"], "situation": spec["situation"], "seconds": int(round(secs))}
        if "--dump" in sys.argv:
            dump_dir = os.environ.get("MUSIC_DUMP_DIR", "/tmp")
            np.save(os.path.join(dump_dir, spec["id"] + ".npy"), audio)
    order = [t["id"] for t in TRACKS]
    json.dump([meta[i] for i in order if i in meta], open(meta_path, "w"), indent=1, ensure_ascii=False)
    print("wrote", meta_path)


if __name__ == "__main__":
    main()
