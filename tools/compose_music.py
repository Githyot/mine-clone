#!/usr/bin/env python3
"""Original soundtrack generator for the game, third version: an acoustic ensemble.

Instruments (all synthesized from physical/additive models, no samples): felt piano, prepared piano, nylon guitar,
concert harp, low marimba, upright bass, string section and solo cello/viola/fiddle, clarinet and bass clarinet,
French horn, harmonium, frame drum, log drum and brushes. Everything stays in a low, warm register (no bells,
no high harmonics).

Style references are approaches, never material: C418's quiet piano in a big room, Brian Eno's overlapping loops of
unequal length (Music for Airports technique), Roedelius' simple warm miniatures, Jean-Michel Jarre's rolling
arpeggios (played here on a guitar/harp), Aphex Twin's prepared piano and odd-length ostinatos, and the playful,
slightly broken hand-percussion grooves of Clark and Mouse On Mars. Every chord progression, melody, loop and
pattern is produced by the rules below from fixed seeds; nothing is transcribed from any existing song.

Usage:  python3 tools/compose_music.py [OUT_DIR] [track_id ...] [--dump]
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


    # -- plucked strings: nylon guitar, concert harp, upright bass (modal model, two string polarisations) -------------
    PLUCKS = {  # pluck position, inharmonicity, base decay, partial decay, brightness, ring seconds, noise lowpass, noise
        "guitar": (0.14, 0.00004, 0.9, 0.55, 0.80, 5.0, 2200, 0.05),
        "harp": (0.33, 0.00002, 0.45, 0.35, 0.72, 7.0, 1600, 0.025),
        "ubass": (0.22, 0.00008, 1.6, 0.9, 0.62, 3.0, 700, 0.08),
    }

    def pluck(self, midi, vel, dur, kind="guitar"):
        key = ("pl", kind, midi, round(vel, 1), round(min(dur, 8.0)))
        if key in self.piano_cache:
            return self.piano_cache[key]
        rng = self.rng
        pos, B, d0, dk, bright, ring, nlp, namt = self.PLUCKS[kind]
        f0 = float(mtof(midi))
        n = int(ring * SR)
        t = np.arange(n) / SR
        kmax = int(max(1, min(40, 6500 / f0)))
        br = bright + 0.18 * vel
        sig = np.zeros(n)
        norm = 0.0
        for k in range(1, kmax + 1):
            fk = k * f0 * np.sqrt(1 + B * k * k)
            amp = abs(np.sin(np.pi * k * pos)) / k * br ** (k - 1)
            if amp < 1e-4:
                continue
            decay = d0 + dk * k ** 1.3 + f0 / 900
            ph = rng.uniform(0, TWO_PI)
            sig += amp * np.exp(-t * decay) * np.sin(TWO_PI * fk * t + ph)
            # second polarisation: a hair sharp, decays faster -> natural two-stage decay and slow beating
            sig += 0.3 * amp * np.exp(-t * decay * 1.7) * np.sin(TWO_PI * fk * 1.0006 * t + ph + 1.1)
            norm += amp * amp
        sig /= np.sqrt(norm) + 1e-9
        att = smoothstep(t / (0.0025 if kind != "harp" else 0.006))
        damp = np.where(t < dur, 1.0, np.exp(-(t - dur) * 7.0)) if dur < ring else 1.0
        noise = lowpass(rng.normal(0, 1, n), nlp) * np.exp(-t * 260) * namt
        out = (sig * att * damp + noise) * vel
        self.piano_cache[key] = out
        return out

    # -- marimba (low register, tuned bar 1 : 4 : 10 with a resonator tube) -------------------------------------------
    def marimba(self, midi, vel):
        key = ("ma", midi, round(vel, 1))
        if key in self.piano_cache:
            return self.piano_cache[key]
        rng = self.rng
        f0 = float(mtof(midi))
        n = int(3.5 * SR)
        t = np.arange(n) / SR
        sig = np.exp(-t * (1.1 + f0 / 350)) * np.sin(TWO_PI * f0 * t)  # resonator-reinforced fundamental
        for r, a, d in ((3.93, 0.16 * (0.4 + vel), 5 + f0 / 90), (9.8, 0.04 * vel, 12 + f0 / 45)):
            if f0 * r < 7000:
                sig += a * np.exp(-t * d) * np.sin(TWO_PI * f0 * r * t + rng.uniform(0, TWO_PI))
        mallet = lowpass(rng.normal(0, 1, n), 900 + 600 * vel) * np.exp(-t * 300) * 0.06
        out = (sig * smoothstep(t / 0.003) + mallet) * vel
        self.piano_cache[key] = out
        return out

    # -- prepared piano: a bolt/felt between the strings gives each key its own fixed inharmonic, short, buzzy colour --
    def prepared(self, midi, vel):
        key = ("pp", midi, round(vel, 1))
        if key in self.piano_cache:
            return self.piano_cache[key]
        rng = self.rng
        prep = np.random.default_rng(midi * 7 + 3)  # same preparation every time this key is struck
        f0 = float(mtof(midi))
        n = int(3.0 * SR)
        t = np.arange(n) / SR
        sig = np.zeros(n)
        norm = 0.0
        for k in range(1, int(max(2, min(14, 5000 / f0))) + 1):
            r = k * (1 + prep.uniform(-0.03, 0.08) * np.sqrt(k - 1 + 1e-9))
            amp = 1 / k ** 1.1 * (0.6 + 0.8 * prep.random())
            sig += amp * np.exp(-t * (2.5 + 1.3 * k + f0 / 220)) * np.sin(TWO_PI * f0 * r * t + rng.uniform(0, TWO_PI))
            norm += amp * amp
        sig /= np.sqrt(norm)
        buzz = bandpass(rng.normal(0, 1, n), 250, 1400) * np.exp(-t * 18) * (1 + np.sin(TWO_PI * (20 + 25 * prep.random()) * t))
        thump = lowpass(rng.normal(0, 1, n), 350) * np.exp(-t * 70)
        out = (sig * smoothstep(t / 0.003) + buzz * 0.05 * prep.random() + thump * 0.18) * vel
        self.piano_cache[key] = out
        return out

    # -- woodwinds and horn (additive, breath noise, gentle vibrato) --------------------------------------------------
    WINDS = {  # odd-harmonic, rolloff, lowpass Hz, breath, vibrato cents, attack s, scoop cents
        "clarinet": (True, 1.05, 2600, 0.020, 4, 0.07, 0),
        "bassclar": (True, 0.95, 1700, 0.028, 3, 0.09, 0),
        "horn": (False, 1.6, 1200, 0.006, 5, 0.16, 30),
    }

    def wind(self, midi, dur, vel, kind="clarinet", glide_from=None, swell=0.25):
        rng = self.rng
        odd, roll, lp, breath, vibc, att, scoop = self.WINDS[kind]
        f0 = float(mtof(midi))
        rel = 0.22 if kind != "horn" else 0.35
        n = int((dur + rel * 2.5 + 0.1) * SR)
        t = np.arange(n) / SR
        base = np.full(n, f0)
        if glide_from is not None:
            fa = float(mtof(glide_from))
            base = fa * (f0 / fa) ** smoothstep(t / 0.09)
        if scoop:
            base = base * 2 ** (-scoop * np.exp(-t / 0.05) / 1200)
        vib = 2 ** (vibc * smoothstep((t - 0.4) / 0.5) * np.sin(TWO_PI * 4.8 * t + rng.uniform(0, TWO_PI)) / 1200)
        phase = TWO_PI * np.cumsum(base * vib) / SR
        cut = lp * (0.55 + 0.6 * vel)
        sig = np.zeros(n)
        norm = 0.0
        for k in range(1, int(max(1, min(30, cut * 2 / f0))) + 1):
            amp = 1 / k ** roll
            if odd and k % 2 == 0:
                amp *= 0.12 if k < 6 else 0.4
            amp *= np.exp(-(k - 1) * (1 - vel) * 0.2)
            sig += amp * np.sin(k * phase + rng.uniform(0, TWO_PI))
            norm += amp * amp
        sig = lowpass(sig / np.sqrt(norm), cut, 2)
        env = env_asr(n, att, rel, dur, swell=swell)
        air = bandpass(rng.normal(0, 1, n), max(f0, 120), min(3500, f0 * 6)) * breath * (1 + 2 * np.exp(-t / 0.06))
        return (sig * (0.85 + 0.15 * vel) + air) * env * vel

    # -- harmonium / reed organ: two beating reeds, bellows breathing ---------------------------------------------------
    def reed(self, midi, dur, vel, attack=0.9, release=1.3):
        rng = self.rng
        f0 = float(mtof(midi))
        n = int((dur + release * 2.2) * SR)
        t = np.arange(n) / SR
        sig = np.zeros(n)
        for det in (-2.5, 2.5):
            f = f0 * 2 ** (det / 1200)
            norm = 0.0
            for k in range(1, int(max(1, min(24, 2400 / f0))) + 1):
                amp = 1 / k ** 1.4
                sig += amp * np.sin(TWO_PI * f * k * t + rng.uniform(0, TWO_PI))
                norm += amp * amp
        sig = lowpass(sig / np.sqrt(norm) / 2, 1100, 2)
        bellows = 1 + 0.06 * np.sin(TWO_PI * 0.35 * t + rng.uniform(0, TWO_PI)) + 0.03 * np.sin(TWO_PI * 0.11 * t)
        air = lowpass(rng.normal(0, 1, n), 1200) * 0.012
        return (sig * bellows + air) * env_asr(n, attack, release, dur, swell=0.25) * vel

    # -- hand percussion: frame drum, log drum, cajon-like low thump, soft brush ---------------------------------------
    def drum(self, kind, vel, midi=None):
        key = ("dr", kind, midi, round(vel, 1), int(self.rng.integers(3)))
        if key in self.piano_cache:
            return self.piano_cache[key]
        rng = self.rng
        n = int(1.6 * SR)
        t = np.arange(n) / SR
        if kind == "frame":
            f0 = float(mtof(midi if midi is not None else 36))
            sig = np.zeros(n)
            for r, a, d in ((1.0, 1.0, 5.0), (1.59, 0.45, 8.0), (2.14, 0.3, 11.0), (2.65, 0.15, 15.0), (3.16, 0.08, 19.0)):
                f = f0 * r * (1 + 0.07 * np.exp(-t * 35))
                sig += a * np.exp(-t * d) * np.sin(TWO_PI * np.cumsum(f) / SR + rng.uniform(0, TWO_PI))
            sig += lowpass(rng.normal(0, 1, n), 700) * np.exp(-t * 80) * 0.6
        elif kind == "log":
            f0 = float(mtof(midi if midi is not None else 52))
            sig = np.exp(-t * 11) * np.sin(TWO_PI * f0 * t) + 0.25 * np.exp(-t * 22) * np.sin(TWO_PI * f0 * 2.92 * t)
            sig += lowpass(rng.normal(0, 1, n), 1300) * np.exp(-t * 220) * 0.25
        elif kind == "low":
            f = 52 * (1 + 0.5 * np.exp(-t * 40))
            sig = np.exp(-t * 13) * np.sin(TWO_PI * np.cumsum(f) / SR) + lowpass(rng.normal(0, 1, n), 380) * np.exp(-t * 55) * 0.5
        else:  # brush: a soft, mid-range swish (no hi-hat sizzle)
            sig = bandpass(rng.normal(0, 1, n), 500, 2200) * (smoothstep(t / 0.01) * np.exp(-t * 22)) * 1.6
        out = sig * smoothstep(t / 0.0015) * vel
        self.piano_cache[key] = out
        return out


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
    "cluster": [0, 1, 7, 8], "sus2b9": [0, 1, 2, 7], "m11": [0, 3, 7, 10, 17], "6/9": [0, 4, 7, 9, 14],
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

    def __init__(self, bpm, meter, sections, rng, rubato=1.0):
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
        factor *= 1 + 0.03 * rubato * np.sin(TWO_PI * bars / 6.5 + rng.uniform(0, TWO_PI))
        # breathe at the end of every 4-bar phrase
        phrase_pos = (bars % 4) / 4
        factor *= 1 - 0.07 * rubato * np.exp(-((phrase_pos - 0.97) ** 2) / 0.0015)
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
        self.tempo = TempoMap(spec["bpm"], self.meter, self.sections, self.rng, rubato=spec.get("rubato", 1.0))
        self.ost_pos = 0
        self.ost_pattern = None
        self.loop_voices = None
        self.groove = None
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
        ranges = {"bass": (28, 45), "cello": (38, 60), "viola": (50, 69), "violin2": (57, 74), "violin1": (62, 79)}  # kept low and warm
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


    # -- acoustic accompaniment ----------------------------------------------------------------------------------------
    def harp_arp(self, chord, bar, dyn, lo=45, hi=74):
        """Rising (then partly falling) harp arpeggio through the chord, one per bar."""
        meter, rng, tm = self.meter, self.rng, self.tempo
        tones = chord.tones(lo, hi)
        if not tones:
            return
        start = int(rng.integers(0, 2))
        steps = meter * 2
        seq = tones[start:start + steps - 2]
        seq = seq + seq[-2:-4:-1]
        for i, n in enumerate(seq[:steps]):
            beat = bar * meter + i * (meter / steps)
            self.score.add("harp", "harp", tm.time(beat) + rng.normal(0, 0.01), n, dyn * (0.9 if i == 0 else 0.62 + 0.1 * rng.random()),
                           6.0, pan=np.clip(-0.5 + (n - lo) / (hi - lo), -0.6, 0.6))

    def guitar(self, chord, bar, dyn, style="travis"):
        """Fingerpicked nylon guitar: alternating thumb bass, treble notes on the off-beats (or a waltz strum)."""
        meter, rng, tm = self.meter, self.rng, self.tempo
        bass = chord.root
        while bass > 52:
            bass -= 12
        while bass < 40:
            bass += 12
        fifth = chord.nearest_tone(bass + 7, prefer={chord.pc(2)})
        trebles = chord.tones(55, 71)[-4:] or [bass + 12]
        b0 = bar * meter
        ev = []
        if style == "waltz":
            ev.append((0, bass, dyn))
            for bt in (1, 2):
                for j, n in enumerate(trebles[-3:]):
                    ev.append((bt + 0.018 * j, n, dyn * 0.5))
        else:
            for bt in range(meter):
                ev.append((bt, bass if bt % 2 == 0 else fifth, dyn * (0.85 if bt == 0 else 0.7)))
                tn = trebles[(bt * 2 + 1 + bar) % len(trebles)]
                ev.append((bt + 0.5, tn, dyn * (0.55 + 0.1 * rng.random())))
                if bt == 0 and rng.random() < 0.5:
                    ev.append((bt, trebles[-1], dyn * 0.55))
        for bt, n, v in ev:
            self.score.add("guitar", "guitar", tm.time(b0 + bt) + rng.normal(0, 0.008), n, v, 5.0, pan=np.clip((n - 55) / 30, -0.3, 0.4) - 0.1)

    def ostinato(self, chord, bar, dyn, inst="marimba", length=7, lo=50, hi=74, step=0.5):
        """A pattern of `length` steps that keeps cycling across the bar lines, so its accents drift against the meter."""
        meter, rng, tm = self.meter, self.rng, self.tempo
        if self.ost_pattern is None or len(self.ost_pattern) != length:
            pat = [0]
            for i in range(1, length):
                pat.append(int(np.clip(pat[-1] + rng.choice([-1, 1, 1, 2, -2]), 0, 5)))
            rests = set(rng.choice(range(2, length), size=max(0, length // 5), replace=False).tolist())
            self.ost_pattern = [None if i in rests else p for i, p in enumerate(pat)]
        tones = chord.tones(lo, hi)
        if not tones:
            return
        for s in range(int(meter / step)):
            idx = self.ost_pattern[self.ost_pos % length]
            first = self.ost_pos % length == 0
            self.ost_pos += 1
            if idx is None:
                continue
            n = tones[min(idx, len(tones) - 1)]
            beat = bar * meter + s * step
            v = dyn * (0.85 if first else 0.55 + 0.12 * rng.random())
            pan = 0.35 if inst != "harp" else -0.35
            if inst == "guitar":
                self.score.add("guitar", "guitar", tm.time(beat) + rng.normal(0, 0.006), n, v, step * 2.5 * tm.beat_len(beat), pan=pan)
            else:
                self.score.add({"harp": "harp", "marimba": "mallet", "prepared": "prepared"}[inst], inst, tm.time(beat) + rng.normal(0, 0.006),
                               n, v, 4.0, pan=pan)

    def loops(self, t0, t1, dyn):
        """Eno-style process music: a few short phrases, each repeating on its own (non-matching) period, so the
        combination keeps changing. Notes come from a consonant pitch set, so any overlap sounds right."""
        spec, rng = self.spec, self.rng
        if self.loop_voices is None:
            pcs = [(spec["tonic"] + p) % 12 for p in spec.get("loop_pcs", [0, 2, 4, 7, 9])]
            ranges = {"piano": (55, 74), "harp": (50, 72), "clarinet": (55, 72), "cello": (43, 60), "marimba": (48, 67),
                      "horn": (50, 65), "reed": (50, 67), "guitar": (50, 69), "bassclar": (40, 55)}
            voices = []
            for i, (inst, period) in enumerate(spec["loops"]):
                lo, hi = ranges[inst]
                pool = [n for n in range(lo, hi + 1) if n % 12 in pcs]
                k = int(rng.integers(1, 4))
                notes = sorted(rng.choice(pool, size=k, replace=False).tolist(), reverse=bool(rng.random() < 0.5))
                gaps = np.cumsum([0] + [rng.choice([0.7, 1.1, 1.6, 2.2]) for _ in range(k - 1)]).tolist()
                voices.append(dict(inst=inst, period=period, offset=rng.uniform(0, period * 0.6) + 1.5 * i, notes=notes, gaps=gaps,
                                   pan=rng.uniform(-0.7, 0.7)))
            self.loop_voices = voices
        for v in self.loop_voices:
            m = max(0, int(np.ceil((t0 - v["offset"]) / v["period"])))
            while True:
                ts = v["offset"] + m * v["period"]
                if ts >= t1:
                    break
                for n, g in zip(v["notes"], v["gaps"]):
                    vel = dyn * (0.75 + 0.15 * rng.random())
                    self.play(v["inst"], ts + g + rng.normal(0, 0.02), n, vel, 2.2 + rng.random() * 1.5, None, pan=v["pan"], bus="loops")
                m += 1

    def percussion(self, bar, dyn, style):
        meter, rng, tm = self.meter, self.rng, self.tempo
        b0 = bar * meter
        hits = []
        if style == "heartbeat":
            for bt in range(0, meter, 2):
                hits += [(bt, "frame", 31, 1.0), (bt + 0.35, "frame", 31, 0.55)]
        elif style == "pulse":
            hits += [(bt, "frame", 33, 0.8 if bt == 0 else 0.5) for bt in range(meter)]
        elif style == "waltz":
            hits += [(0, "frame", 35, 1.0), (1, "brush", None, 0.45), (2, "brush", None, 0.5)]
        elif style == "groove":
            # a 2-bar base pattern on a 16th grid, lightly mutated every bar (playful, never mechanical)
            if self.groove is None:
                g = [(0, "low", None, 1.0), (1.5, "log", 50, 0.6), (2.75, "frame", 36, 0.7), (3.5, "log", 55, 0.5),
                     (4.75, "low", None, 0.8), (5.5, "log", 52, 0.55), (6, "frame", 36, 0.7), (7.25, "log", 50, 0.45)]
                g += [(x + 0.5, "brush", None, 0.3) for x in range(8) if rng.random() < 0.6]
                self.groove = g
            half = (bar % 2) * meter
            for (x, k, m, v) in self.groove:
                if half <= x < half + meter and rng.random() > 0.12:
                    hits.append((x - half, k, m, v))
            if rng.random() < 0.5:
                x = float(rng.choice(np.arange(0, meter, 0.25)))
                hits.append((x, "log", int(rng.choice([48, 50, 52, 55])), 0.35))
            if bar % 4 == 3:
                hits += [(meter - 0.5, "log", 55, 0.5), (meter - 0.25, "log", 52, 0.45)]
        for bt, k, m, v in hits:
            self.score.add("perc", "drum", tm.time(b0 + bt) + rng.normal(0, 0.006), m or 0, dyn * v * (1 + rng.normal(0, 0.06)), 1.0,
                           pan={"log": -0.25, "brush": 0.3}.get(k, 0.05), kind=k)

    def prepared_hits(self, chord, bar, dyn):
        """Sparse syncopated prepared-piano figures in the low-middle register."""
        meter, rng, tm = self.meter, self.rng, self.tempo
        tones = chord.tones(40, 64)
        for _ in range(int(rng.integers(1, 4))):
            x = float(rng.choice(np.arange(0, meter, 0.25)))
            n = tones[int(rng.integers(len(tones)))]
            self.score.add("prepared", "prepared", tm.time(bar * meter + x), n, dyn * (0.5 + 0.3 * rng.random()), 1.0,
                           pan=rng.uniform(-0.4, 0.4))
            if rng.random() < 0.4:
                self.score.add("prepared", "prepared", tm.time(bar * meter + x + 0.25), n - 12 if n > 52 else n + 7,
                               dyn * 0.4, 1.0, pan=rng.uniform(-0.4, 0.4))

    def play(self, inst, t0, n, vel, dur, prev_m, pan=0.0, bus=None):
        """One melodic note on any instrument."""
        rng, sc = self.rng, self.score
        legato = prev_m is not None and abs(n - prev_m) <= 5 and rng.random() < 0.5
        if inst == "piano":
            sc.add(bus or "piano", "piano", t0 + rng.normal(0, 0.012), n, vel, dur * 1.15, pan=np.clip((n - 60) / 40, -0.4, 0.5))
        elif inst in ("cello", "viola"):
            sc.add(bus or "cello", "bowed", t0 + rng.normal(0.02, 0.015), n, vel, dur * 1.04, pan=0.25 if inst == "cello" else -0.25,
                   part=inst, voices=1, attack=0.12 + 0.2 * rng.random(), release=0.45, swell=0.3, vib_cents=13, vib_delay=0.3,
                   glide_from=prev_m if legato else None, glide_time=0.1 + 0.08 * rng.random(), detune_cents=3)
        elif inst == "fiddle":
            grace = n - 2 if rng.random() < 0.2 and dur >= 0.5 else None
            sc.add(bus or "cello", "bowed", t0 + rng.normal(0.01, 0.012), n, vel, dur * 1.02, pan=-0.2, part="violin", voices=1,
                   attack=0.05 + 0.05 * rng.random(), release=0.25, swell=0.15, vib_cents=14, vib_rate=5.8, vib_delay=0.18,
                   glide_from=prev_m if legato else None, glide_time=0.07, grace=grace, detune_cents=4)
        elif inst in ("clarinet", "bassclar", "horn"):
            sc.add(bus or "winds", "wind", t0 + rng.normal(0.01, 0.01), n, vel, dur * 1.02, pan={"clarinet": -0.3, "bassclar": 0.2, "horn": 0.4}[inst],
                   kind=inst, glide_from=prev_m if (legato and inst != "horn") else None)
        elif inst in ("guitar", "harp"):
            sc.add(bus or inst, inst, t0 + rng.normal(0, 0.008), n, vel, max(dur * 1.5, 1.0), pan=pan or (-0.15 if inst == "guitar" else -0.4))
        elif inst == "marimba":
            sc.add(bus or "mallet", "marimba", t0 + rng.normal(0, 0.008), n, vel, dur, pan=pan or 0.3)
            if dur > 1.2:  # sustained marimba note = soft roll
                k = int(dur / 0.11)
                for i in range(1, k):
                    sc.add(bus or "mallet", "marimba", t0 + i * 0.11 + rng.normal(0, 0.008), n, vel * (0.45 + 0.1 * np.sin(i / 3)), 0.3, pan=pan or 0.3)
        elif inst == "reed":
            sc.add(bus or "reed", "reed", t0, n, vel, dur, pan=pan)
        elif inst == "prepared":
            sc.add(bus or "prepared", "prepared", t0, n, vel, dur, pan=pan)

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
            t_start, t_end = tm.time(bar * meter), tm.time((bar + nb) * meter)
            if "drone" in layers:  # bowed open fifth: double bass + cello, sul tasto
                root = key.tonic
                while root > 40:
                    root -= 12
                for part, n, v in (("bass", root, 0.9), ("cello", root + 7 + (12 if root + 7 < 38 else 0), 0.6)):
                    sc.add("drone", "bowed", t_start - 1.0, n, dyn * v, t_end - t_start + 1.0, pan=0.1 if part == "bass" else -0.1,
                           part=part, voices=2, attack=3.0, release=3.0, swell=0.3, vib_cents=3, vib_delay=1.5, detune_cents=4)
            if "loops" in layers:
                self.loops(t_start, t_end, dyn * sec.get("loop_dyn", 1.0))
            if "reed" in layers:
                for b in range(0, nb, 2):
                    ch = chords[b]
                    t0 = tm.time((bar + b) * meter)
                    dur = tm.time((bar + min(b + 2, nb)) * meter) - t0
                    for i, n in enumerate(ch.tones(50, 69)[:4]):
                        sc.add("reed", "reed", t0 - 0.4, n, dyn * 0.7, dur + 0.6, pan=(-0.4 + 0.27 * i))
            # --- melody (4-bar phrases)
            melody = []
            if "melody" in layers:
                mot = self.motif(sec.get("motif", "A"), bars=2, rest_prob=spec.get("rest", 0.25), leap_prob=spec.get("leap", 0.15))
                notes_in = key.notes(lo, hi)
                for p0 in range(0, nb, 4):
                    pch = chords[p0:p0 + 4]
                    var = None if (p0 // 4) % 2 == 0 else rng.choice(["transpose", "invert", "ornament"])
                    start = last_melody_idx if last_melody_idx is not None else len(notes_in) // 2
                    start = int(np.clip(start + int(rng.integers(-1, 2)), 2, max(2, len(notes_in) - 3)))
                    phrase = self.realize_phrase(key, pch, bar + p0, mot, lo, hi, start_idx=start, variation=var, dyn=dyn)
                    if phrase:
                        last_melody_idx = notes_in.index(phrase[-1][2]) if phrase[-1][2] in notes_in else start
                    melody += phrase
                prev_m = None
                dbl = sec.get("double")  # (instrument, octave shift)
                for i, (beat, dur_b, n, vel) in enumerate(melody):
                    t0 = tm.time(beat)
                    next_beat = melody[i + 1][0] if i + 1 < len(melody) else beat + dur_b
                    dur = tm.time(min(beat + dur_b, next_beat + 0.1)) - t0
                    self.play(mel_inst, t0, n, vel, dur, prev_m)
                    if dbl:
                        self.play(dbl[0], t0 + 0.02, n + 12 * dbl[1], vel * 0.55, dur, prev_m + 12 * dbl[1] if prev_m else None)
                    prev_m = n
            # --- bar by bar accompaniment
            for b in range(nb):
                ch = chords[b]
                gb = bar + b
                if "lh" in layers:
                    prev_voicing = self.piano_lh(ch, gb, sec.get("lh", "block"), dyn * 0.8, prev_voicing)
                if "harp_arp" in layers and b % sec.get("harp_every", 1) == 0:
                    self.harp_arp(ch, gb, dyn * 0.75)
                if "guitar" in layers:
                    self.guitar(ch, gb, dyn * 0.8, style=sec.get("guitar", "travis"))
                if "ostinato" in layers:
                    self.ostinato(ch, gb, dyn * 0.7, inst=sec.get("ost_inst", "marimba"), length=sec.get("ost_len", 7),
                                  lo=sec.get("ost_lo", 50), hi=sec.get("ost_hi", 74))
                if "perc" in layers:
                    self.percussion(gb, dyn * sec.get("perc_dyn", 0.7), sec.get("perc", "pulse"))
                if "prep" in layers:
                    self.prepared_hits(ch, gb, dyn)
                if "strings" in layers:
                    parts = sec.get("parts", ("bass", "cello", "viola", "violin2", "violin1"))
                    sb = sec.get("string_bars", 1)
                    if b % sb == 0:
                        nxt = self.sections[si + 1] if si + 1 < len(self.sections) else None
                        is_last = b + sb >= nb and (nxt is None or "strings" not in nxt["layers"])
                        prev_strings = self.string_chord(ch, gb, dyn * sec.get("string_dyn", 0.8), prev_strings, parts=parts,
                                                         swell=0.35 if dyn > 0.6 else 0.25, bars=sb, last=is_last)
                if "horn" in layers and b % 2 == 0:
                    n = ch.nearest_tone(57, prefer={ch.pc(1), ch.pc(2)})
                    t0 = tm.time(gb * meter)
                    sc.add("winds", "wind", t0 + 0.05, n, dyn * 0.6, tm.time((gb + 2) * meter) - t0, pan=0.4, kind="horn", swell=0.5)
                if "counter" in layers:
                    bar_mel = [m for m in melody if gb * meter <= m[0] < (gb + 1) * meter]
                    cello_prev = self.cello_counterpoint(key, ch, gb, bar_mel, dyn * 0.75, cello_prev)
                if "pizz" in layers:
                    pattern = [0, 2] if meter == 4 else [0]
                    if sec.get("pizz_busy"):
                        pattern = [0, 1.5, 2, 3.5] if meter == 4 else [0, 2]
                    for i, bt in enumerate(pattern):
                        n = ch.root
                        while n > 45:
                            n -= 12
                        while n < 33:
                            n += 12
                        if i % 2 == 1:
                            n = ch.nearest_tone(n + 7, prefer={ch.pc(2), ch.pc(1)})
                        sc.add("pizz", "ubass", tm.time(gb * meter + bt) + rng.normal(0.005, 0.008), n, dyn * (0.9 if i == 0 else 0.65), 0.8, pan=0.15)
                if "swell" in layers and (b % 4 == 3 or (rng.random() < 0.2 and "melody" not in layers)):
                    # bowed crescendo into the next downbeat (viola + cello, no reversed audio)
                    target = tm.time((gb + 1) * meter)
                    sw_dur = 1.8 + 2.0 * rng.random()
                    nxt = chords[(b + 1) % nb]
                    for n, part in zip(nxt.tones(48, 69)[:3], ("cello", "viola", "viola")):
                        sc.add("fx", "bowed", target - sw_dur + rng.normal(0, 0.03), n, dyn * 0.55, sw_dur + 0.25, pan=rng.uniform(-0.6, 0.6),
                               part=part, voices=3, attack=sw_dur * 0.95, release=0.35, vib_cents=4)
                if "dissonance" in layers and b % 2 == 1:
                    n = ch.root + 12 + int(rng.choice([1, 6, 13]))
                    while n > 60:
                        n -= 12
                    t0 = tm.time(gb * meter + 1)
                    sc.add("fx", "bowed", t0, n, dyn * 0.45, 2.5, pan=rng.uniform(-0.5, 0.5), part="cello", voices=3,
                           attack=1.5, release=1.2, swell=0.6, vib_cents=5, detune_cents=12)
            bar += nb
        return tm.time(bar * meter)


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


BUSES = ("piano", "strings", "cello", "winds", "guitar", "harp", "mallet", "prepared", "reed", "perc", "pizz", "drone", "fx", "loops")
GAINS = dict(piano=0.9, strings=0.55, cello=0.6, winds=0.55, guitar=0.75, harp=0.6, mallet=0.6, prepared=0.5, reed=0.35,
             perc=0.6, pizz=0.8, drone=0.5, fx=0.45, loops=0.55)
SENDS = dict(piano=0.35, strings=0.5, cello=0.4, winds=0.4, guitar=0.25, harp=0.45, mallet=0.35, prepared=0.3, reed=0.5,
             perc=0.2, pizz=0.2, drone=0.3, fx=0.7, loops=0.65)


def render(spec, verbose=True):
    comp = Composer(spec)
    end = comp.compose()
    synth = Synth(np.random.default_rng(spec["seed"] + 1))
    rng = np.random.default_rng(spec["seed"] + 2)
    tail = spec.get("reverb", 4.0) + 3.0
    total = int((end + tail) * SR)
    buses = {b: np.zeros((2, total)) for b in BUSES}
    for e in sorted(comp.score.events, key=lambda e: e["t"]):
        inst, x = e["inst"], e["extra"]
        if inst == "piano":
            s = synth.piano(e["midi"], e["vel"], e["dur"])
        elif inst == "bowed":
            s = synth.bowed(e["midi"], e["dur"], e["vel"], **x)
        elif inst in ("guitar", "harp", "ubass"):
            s = synth.pluck(e["midi"], e["vel"], e["dur"], kind=inst)
        elif inst == "marimba":
            s = synth.marimba(e["midi"], e["vel"])
        elif inst == "prepared":
            s = synth.prepared(e["midi"], e["vel"])
        elif inst == "wind":
            s = synth.wind(e["midi"], e["dur"], e["vel"], **x)
        elif inst == "reed":
            s = synth.reed(e["midi"], e["dur"], e["vel"])
        elif inst == "drum":
            s = synth.drum(x["kind"], e["vel"], e["midi"] or None)
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
        if e["bus"] in ("strings", "reed", "drone"):  # ensemble width: short decorrelating delay on the other side
            d = int(rng.uniform(0.004, 0.012) * SR)
            if i1 + d <= total:
                bus[1 if pan < 0 else 0, i0 + d:i1 + d] += seg * 0.35 * (gr if pan < 0 else gl)
    gains = dict(GAINS)
    gains.update(spec.get("gains", {}))
    dry = np.zeros((2, total))
    send = np.zeros((2, total))
    for name, b in buses.items():
        if not np.any(b):
            continue
        if name == "strings":
            b = highpass(b, 60)
            b = peaking_eq(b, 350, -2.0, 1.0)
        elif name == "piano":
            b = peaking_eq(b, 250, -1.5, 0.9)
        elif name == "guitar":  # nylon guitar body: air cavity + top plate resonances
            b = peaking_eq(b, 105, 4.0, 2.0)
            b = peaking_eq(b, 210, 3.0, 1.8)
            b = peaking_eq(b, 420, 1.5, 1.5)
            b = highpass(b, 70)
        elif name == "harp":
            b = peaking_eq(b, 180, 2.5, 1.2)
        elif name == "pizz":
            b = peaking_eq(b, 90, 3.0, 1.5)
        elif name == "reed":
            b = highpass(b, 110)
        dry += b * gains[name]
        send += b * gains[name] * SENDS[name]
    ir = make_reverb_ir(spec.get("reverb", 4.0), np.random.default_rng(spec["seed"] + 3), bright=spec.get("rev_bright", 0.4))
    send = highpass(send, 160)
    send = lowpass(send, 5500)
    wet = np.stack([signal.fftconvolve(send[c], ir[c])[:total] for c in range(2)])
    wet = np.stack([wet[0] * 0.85 + wet[1] * 0.15, wet[1] * 0.85 + wet[0] * 0.15])
    out = dry + wet * spec.get("wet", 0.5)
    # master EQ: warm, never shrill (HP: no high-pitched sound)
    out = highpass(out, 28, 2)
    out = peaking_eq(out, 240, -1.0, 0.8)
    out = high_shelf(out, 5000, -2.0)
    out = lowpass(out, spec.get("master_lowpass", 9000), 2)
    mid, side = (out[0] + out[1]) / 2, (out[0] - out[1]) / 2
    side = highpass(side, 200) * 1.2
    out = np.stack([mid + side, mid - side])
    fin = int(1.5 * SR)
    out[:, :fin] *= np.linspace(0, 1, fin) ** 2
    fout = int(min(7.0, tail) * SR)
    out[:, -fout:] *= np.linspace(1, 0, fout) ** 1.5
    # limiter: lookahead/hold minimum filter on the required gain, then smoothed; finally -1 dBFS peak
    rms = np.sqrt(np.mean(out ** 2))
    out *= 0.16 / max(rms, 1e-9)
    peak = np.max(np.abs(out), axis=0)
    g = np.minimum(1.0, 0.85 / np.maximum(peak, 1e-9))
    g = minimum_filter1d(g, size=int(0.006 * SR))
    g = signal.lfilter([1 - 0.9995], [1, -0.9995], g)
    g = minimum_filter1d(g, size=int(0.004 * SR))
    g = lowpass(g, 400, 1)
    out *= np.minimum(g, 1.0)
    out = np.tanh(out / 0.95) * 0.95
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
    # quiet piano in a big room, harp and cello around it
    dict(id="first_light", title="Primeira Luz", situation="overworld", seed=1201, bpm=63, tonic=60, scale="lydian",
         prog=[(0, "maj9"), (9, "m7"), (5, "maj7"), (7, "6")], mel_range=(60, 79), rest=0.3, reverb=4.2, wet=0.45,
         form=[S("intro", 4, 0.4, ["harp_arp", "reed"]),
               S("theme", 8, 0.55, ["lh", "melody", "counter"], lh="broken"),
               S("theme2", 8, 0.62, ["lh", "melody", "counter", "strings", "harp_arp"], lh="sparse", parts=("cello", "viola", "violin2"),
                 string_dyn=0.55, harp_every=2),
               S("dev", 8, 0.6, ["lh", "melody", "strings", "horn"], lh="arp", transpose=4, scale="major", motif="B", melody_inst="clarinet",
                 prog=[(0, "maj7"), (5, "add9"), (9, "m7"), (7, "sus4")], parts=("bass", "cello", "viola")),
               S("climax", 8, 0.82, ["lh", "melody", "strings", "harp_arp", "horn"], lh="octaves", double=("cello", -1), string_dyn=0.9, tempo=1.02),
               S("outro", 6, 0.4, ["lh", "melody", "reed"], lh="sparse", rit=True)]),
    # Eno-style overlapping loops frame a cello song
    dict(id="moss", title="Musgo", situation="overworld", seed=2402, bpm=54, tonic=62, scale="dorian", melody_inst="cello",
         prog=[(0, "madd9"), (10, "maj7"), (5, "m7"), (7, "7sus4")], mel_range=(48, 67), rest=0.35, leap=0.1, reverb=4.8, wet=0.52,
         loops=[("piano", 17.3), ("harp", 23.9), ("clarinet", 29.7), ("piano", 31.1), ("cello", 37.3)], loop_pcs=[0, 3, 5, 7, 10],
         form=[S("intro", 6, 0.45, ["loops", "drone"]),
               S("theme", 8, 0.55, ["melody", "lh", "loops"], lh="sparse", loop_dyn=0.6),
               S("theme2", 8, 0.6, ["melody", "lh", "strings"], lh="broken", parts=("viola", "violin2", "violin1"), string_dyn=0.5, string_bars=2),
               S("dev", 8, 0.65, ["melody", "lh", "strings", "horn", "swell"], lh="block", scale="major", motif="B", melody_inst="clarinet",
                 mel_range=(55, 74), prog=[(0, "add9"), (4, "min"), (9, "m7"), (5, "maj7")], parts=("bass", "viola", "violin2"), string_dyn=0.65),
               S("climax", 8, 0.8, ["melody", "lh", "strings", "counter"], lh="octaves", melody_inst="piano", mel_range=(62, 79), motif="A",
                 string_dyn=0.85),
               S("outro", 8, 0.42, ["loops", "drone", "reed"], rit=True)]),
    # harp, low marimba and bowed bass under water; loops drift
    dict(id="still_water", title="Água Parada", situation="underwater", seed=3203, bpm=48, tonic=63, scale="major",
         prog=[(0, "maj9"), (5, "add9"), (8, "maj7"), (7, "sus2")], mel_range=(55, 74), rest=0.5, reverb=6.5, wet=0.68,
         master_lowpass=2800, rev_bright=0.25, loops=[("harp", 19.1), ("marimba", 26.3), ("piano", 33.7), ("reed", 41.9)],
         loop_pcs=[0, 2, 4, 7, 9],
         form=[S("intro", 4, 0.45, ["drone", "reed"]),
               S("theme", 8, 0.5, ["loops", "reed"]),
               S("theme2", 8, 0.55, ["melody", "reed", "strings", "harp_arp"], parts=("cello", "viola"), string_bars=2, string_dyn=0.55, harp_every=2),
               S("dev", 8, 0.6, ["melody", "strings", "loops", "drone"], transpose=5, scale="lydian", motif="B", melody_inst="cello",
                 mel_range=(45, 62), prog=[(0, "maj7#11"), (7, "add9"), (2, "m9"), (5, "maj7")], parts=("viola", "violin2"), string_bars=2, loop_dyn=0.6),
               S("climax", 6, 0.7, ["melody", "lh", "strings", "harp_arp", "horn"], lh="sparse", string_bars=2, string_dyn=0.7),
               S("outro", 6, 0.4, ["loops", "drone"], rit=True)]),
    # playful: a 7-step marimba pattern drifting over 4/4, hand drums, prepared piano, clarinet tune
    dict(id="block_by_block", title="Bloco a Bloco", situation="creative", seed=4804, bpm=80, tonic=65, scale="major", rubato=0.25,
         prog=[(0, "add9"), (5, "maj7"), (9, "m7"), (7, "6")], mel_range=(60, 77), rest=0.25, leap=0.2, reverb=2.6, wet=0.32,
         melody_inst="clarinet",
         form=[S("intro", 4, 0.45, ["ostinato", "perc"], perc="pulse", perc_dyn=0.4),
               S("theme", 8, 0.6, ["ostinato", "perc", "melody", "pizz"], perc="groove"),
               S("theme2", 8, 0.65, ["ostinato", "perc", "melody", "pizz", "prep"], perc="groove", melody_inst="piano"),
               S("dev", 8, 0.6, ["ostinato", "melody", "reed", "pizz"], ost_inst="harp", ost_len=5, transpose=2, motif="B", melody_inst="marimba",
                 prog=[(0, "maj7"), (9, "m7"), (2, "m7"), (7, "sus4")]),
               S("climax", 12, 0.8, ["ostinato", "perc", "melody", "pizz", "strings", "prep"], perc="groove", pizz_busy=True,
                 double=("bassclar", -1), parts=("cello", "viola", "violin2"), string_dyn=0.6, string_bars=2),
               S("outro", 4, 0.45, ["ostinato", "melody"], melody_inst="piano", rit=True)]),
    # nether: bowed drones, bass clarinet, heartbeat frame drum, prepared piano
    dict(id="embers", title="Brasa", situation="nether", seed=5005, bpm=46, tonic=50, scale="phrygian", melody_inst="bassclar",
         prog=[(0, "5"), (1, "maj7"), (0, "m7"), (10, "5")], mel_range=(40, 58), rest=0.45, reverb=5.5, wet=0.58, rev_bright=0.3,
         gains=dict(drone=0.6, fx=0.55),
         form=[S("intro", 4, 0.45, ["drone", "dissonance", "perc"], perc="heartbeat", perc_dyn=0.5),
               S("theme", 8, 0.5, ["drone", "melody", "perc", "prep"], perc="heartbeat", perc_dyn=0.5),
               S("theme2", 8, 0.55, ["drone", "melody", "strings", "dissonance", "perc"], melody_inst="cello", mel_range=(43, 62),
                 perc="heartbeat", parts=("bass", "cello", "viola"), string_bars=2, string_dyn=0.6),
               S("dev", 8, 0.65, ["drone", "melody", "strings", "swell", "prep", "perc"], transpose=-1, scale="locrian", motif="B",
                 prog=[(0, "m7b5"), (1, "maj7"), (6, "5"), (0, "cluster")], parts=("bass", "cello", "viola"), string_bars=2, perc="heartbeat"),
               S("climax", 8, 0.8, ["drone", "melody", "strings", "dissonance", "lh", "perc", "horn"], lh="octaves", melody_inst="horn",
                 mel_range=(48, 65), parts=("bass", "cello", "viola", "violin2"), string_dyn=0.85, perc="heartbeat", perc_dyn=0.9),
               S("outro", 6, 0.4, ["drone", "melody", "dissonance"], rit=True)]),
    # the End: wide, slow, low; piano/horn/harp loops in a whole-tone haze
    dict(id="void", title="Vazio", situation="end", seed=7206, bpm=42, tonic=52, scale="lydaug",
         prog=[(0, "sus2"), (2, "aug"), (-2, "sus2"), (4, "maj7#11")], mel_range=(55, 74), rest=0.6, reverb=7.5, wet=0.72,
         rev_bright=0.3, loops=[("piano", 21.7), ("horn", 28.3), ("harp", 34.1), ("cello", 43.9)], loop_pcs=[0, 2, 4, 6, 11],
         form=[S("intro", 6, 0.42, ["loops", "drone"]),
               S("theme", 8, 0.5, ["loops", "reed", "melody"], loop_dyn=0.7),
               S("theme2", 8, 0.55, ["melody", "strings", "swell", "loops"], melody_inst="clarinet", parts=("cello", "viola"), string_bars=2, loop_dyn=0.5),
               S("dev", 8, 0.6, ["melody", "strings", "swell", "horn"], transpose=6, scale="whole", motif="B", melody_inst="viola",
                 prog=[(0, "aug"), (2, "sus2"), (4, "aug"), (-2, "sus2")], parts=("bass", "cello", "viola"), string_bars=2),
               S("climax", 6, 0.7, ["melody", "strings", "harp_arp", "horn"], parts=("bass", "cello", "viola", "violin2"), string_bars=2, string_dyn=0.75),
               S("outro", 6, 0.4, ["loops", "drone"], rit=True)]),
    # title screen: warm piano, string quartet, horn
    dict(id="beginning", title="Início", situation="title", seed=8407, bpm=60, tonic=57, scale="dorian",
         prog=[(0, "m9"), (5, "add9"), (3, "maj7"), (10, "6")], mel_range=(62, 79), rest=0.3, leap=0.18, reverb=4.4, wet=0.5,
         form=[S("intro", 4, 0.45, ["lh", "strings", "harp_arp"], lh="sparse", parts=("cello", "viola"), string_dyn=0.5, string_bars=2, harp_every=2),
               S("theme", 8, 0.6, ["lh", "melody", "counter", "strings"], lh="block", parts=("viola", "violin2"), string_dyn=0.5),
               S("theme2", 8, 0.68, ["lh", "melody", "strings", "horn"], lh="broken", melody_inst="cello", mel_range=(50, 69),
                 parts=("bass", "viola", "violin2"), string_dyn=0.65),
               S("dev", 8, 0.65, ["lh", "melody", "strings", "counter", "harp_arp"], lh="arp", transpose=3, scale="major", motif="B",
                 prog=[(0, "maj7"), (9, "m7"), (5, "add9"), (7, "sus4")], parts=("viola", "violin2", "violin1"), string_dyn=0.6, harp_every=2),
               S("climax", 8, 0.88, ["lh", "melody", "strings", "horn"], lh="octaves", double=("cello", -1), string_dyn=0.95, tempo=1.03),
               S("outro", 6, 0.45, ["lh", "melody", "strings"], lh="sparse", parts=("cello", "viola"), string_dyn=0.5, string_bars=2, rit=True)]),
    # walking: fingerpicked guitar that keeps rolling (an acoustic take on the analogue arpeggio)
    dict(id="wandering", title="Andarilho", situation="overworld", seed=9208, bpm=72, tonic=67, scale="mixolydian", rubato=0.5,
         prog=[(0, "add9"), (10, "maj7"), (5, "6"), (7, "m7")], mel_range=(60, 77), rest=0.3, leap=0.18, reverb=3.4, wet=0.4,
         melody_inst="clarinet",
         form=[S("intro", 4, 0.45, ["guitar"]),
               S("theme", 8, 0.6, ["guitar", "melody", "pizz"]),
               S("theme2", 8, 0.65, ["guitar", "melody", "strings", "perc"], melody_inst="viola", mel_range=(55, 74), parts=("cello",),
                 string_dyn=0.6, string_bars=2, perc="pulse", perc_dyn=0.35),
               S("dev", 8, 0.6, ["ostinato", "melody", "counter", "reed"], ost_inst="harp", ost_len=5, transpose=3, scale="major", motif="B",
                 melody_inst="piano", prog=[(0, "maj9"), (7, "6"), (9, "m7"), (5, "add9")]),
               S("climax", 8, 0.82, ["guitar", "melody", "strings", "pizz", "perc", "horn"], double=("cello", -1), perc="groove", perc_dyn=0.5,
                 parts=("bass", "viola", "violin2"), string_dyn=0.7, string_bars=2),
               S("outro", 6, 0.45, ["guitar", "melody"], melody_inst="piano", rit=True)]),
    # 3/4 folk waltz: fiddle, guitar, harmonium, upright bass, frame drum
    dict(id="harvest", title="Colheita", situation="overworld", seed=1109, bpm=100, meter=3, tonic=62, scale="major", rubato=0.5,
         melody_inst="fiddle", prog=[(0, "maj"), (7, "sus4"), (9, "m7"), (5, "6"), (0, "maj"), (2, "m7"), (7, "7"), (0, "add9")],
         mel_range=(57, 78), rest=0.2, leap=0.2, reverb=2.6, wet=0.33, gains=dict(cello=0.7),
         form=[S("intro", 8, 0.5, ["guitar", "pizz", "perc"], guitar="waltz", perc="waltz", perc_dyn=0.5),
               S("theme", 16, 0.65, ["guitar", "melody", "pizz", "perc"], guitar="waltz", perc="waltz", perc_dyn=0.5),
               S("theme2", 16, 0.7, ["guitar", "melody", "pizz", "reed", "strings"], guitar="waltz", melody_inst="clarinet",
                 parts=("viola",), string_dyn=0.5, string_bars=2),
               S("dev", 16, 0.6, ["guitar", "melody", "strings", "counter"], guitar="waltz", transpose=-3, scale="aeolian", motif="B",
                 melody_inst="viola", mel_range=(55, 74), prog=[(0, "min"), (8, "maj7"), (3, "add9"), (10, "6"), (0, "madd9"), (5, "m7"), (7, "min"), (0, "min")],
                 parts=("bass",), string_dyn=0.6, string_bars=2),
               S("climax", 16, 0.85, ["guitar", "melody", "pizz", "strings", "reed", "perc"], guitar="waltz", double=("clarinet", -1),
                 parts=("bass", "cello", "viola"), string_dyn=0.75, perc="waltz", tempo=1.03),
               S("outro", 8, 0.45, ["guitar", "melody", "pizz"], guitar="waltz", rit=True)]),
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
