//! Voice chat, in two ways:
//!
//! - **Grupo**: like a call. Everyone in the same group hears each other at full volume, wherever
//!   they are. The microphone is open (it sends while you speak) or works with the talk key.
//! - **Proximidade**: hold the talk key (Caps Lock) and your voice comes out of your character:
//!   from its side, quieter with distance (silent past 48 blocks) and muffled behind walls.
//!
//! In both, the mouth of whoever is talking opens and closes with their voice.
//!
//! The work happens off the game thread: one "voz" thread takes the microphone, cuts it into
//! 20 ms frames, encodes them with Opus and sends them as datagrams; it also decodes what
//! arrives (filling lost frames from the next packet or by concealment) into a small buffer per
//! speaker. Each speaker plays as one endless rodio source that reads that buffer, so nothing
//! is started per word. The game thread only moves numbers: where each voice should sit
//! (gains and muffling, as for the other sounds) and how open each mouth is.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::{bounded, never, select, unbounded, Receiver, Sender};
use glam::Vec3;
use net::voice::{Lane, Packet, VoiceOut};
use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::{cpal, OutputStreamHandle, Source};
use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

use crate::ui::{GRAY, WHITE, YELLOW};
use crate::{Game, Screen};

const RATE: u32 = 48_000;
/// 20 ms of mono audio.
const FRAME: usize = 960;
/// A voice starts playing once this much is waiting (60 ms), to ride over network jitter.
const START: usize = FRAME * 3;
/// More than this waiting (250 ms) and the oldest is dropped: latency stays low.
const MOST: usize = RATE as usize / 4;
/// Read from a speaker's buffer this much at a time (10 ms).
const CHUNK: usize = 480;
/// Open microphone: keep sending this long after the voice drops under the threshold.
const HANG: f32 = 0.45;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VoiceMode {
    Off,
    Group,
    #[default]
    Near,
}

impl VoiceMode {
    pub fn label(self) -> &'static str {
        match self {
            VoiceMode::Off => "Desligado",
            VoiceMode::Group => "Grupo (como uma chamada)",
            VoiceMode::Near => "Proximidade (3D)",
        }
    }

    pub(crate) fn next(self) -> VoiceMode {
        match self {
            VoiceMode::Near => VoiceMode::Group,
            VoiceMode::Group => VoiceMode::Off,
            VoiceMode::Off => VoiceMode::Near,
        }
    }
}

/// Voice settings (in settings.ron under `voice`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct VoiceSettings {
    pub mode: VoiceMode,
    /// Group number 1..9 (players in the same group hear each other in group mode).
    pub group: u8,
    /// Group mode: the microphone sends whenever you speak (else only while the talk key is
    /// held).
    pub open_mic: bool,
    /// Microphone by name; empty = the system's default.
    pub mic: String,
    /// Everyone's voices.
    pub volume: f32,
    /// Open microphone: how loud you must be for it to send (0..1 of the meter).
    pub sensitivity: f32,
    /// Per player (by name): their volume, 0 = muted.
    pub players: BTreeMap<String, f32>,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        VoiceSettings { mode: VoiceMode::Near, group: 1, open_mic: true, mic: String::new(), volume: 1.0, sensitivity: 0.25, players: BTreeMap::new() }
    }
}

// ---- Opus ---------------------------------------------------------------------------------

mod opus {
    use opus_head_sys as sys;

    pub struct Encoder(*mut sys::OpusEncoder);
    // The state is only ever used by one thread at a time.
    unsafe impl Send for Encoder {}

    impl Encoder {
        pub fn new() -> Option<Encoder> {
            let mut err = 0;
            // SAFETY: plain constructor; the pointer is checked and owned by the wrapper.
            let st = unsafe { sys::opus_encoder_create(super::RATE as i32, 1, sys::OPUS_APPLICATION_VOIP as i32, &mut err) };
            if st.is_null() || err != 0 {
                return None;
            }
            // SAFETY: valid encoder; each request takes one int.
            unsafe {
                sys::opus_encoder_ctl(st, sys::OPUS_SET_BITRATE_REQUEST as i32, 28_000i32);
                sys::opus_encoder_ctl(st, sys::OPUS_SET_COMPLEXITY_REQUEST as i32, 5i32);
                sys::opus_encoder_ctl(st, sys::OPUS_SET_SIGNAL_REQUEST as i32, sys::OPUS_SIGNAL_VOICE as i32);
                // Each packet also carries a rough copy of the one before: one lost packet
                // costs nothing audible.
                sys::opus_encoder_ctl(st, sys::OPUS_SET_INBAND_FEC_REQUEST as i32, 1i32);
                sys::opus_encoder_ctl(st, sys::OPUS_SET_PACKET_LOSS_PERC_REQUEST as i32, 10i32);
            }
            Some(Encoder(st))
        }

        pub fn encode(&mut self, pcm: &[f32], out: &mut [u8]) -> Option<usize> {
            // SAFETY: pcm holds one frame, out is a writable buffer of the given size.
            let n = unsafe { sys::opus_encode_float(self.0, pcm.as_ptr(), pcm.len() as i32, out.as_mut_ptr(), out.len() as i32) };
            (n > 0).then_some(n as usize)
        }
    }

    impl Drop for Encoder {
        fn drop(&mut self) {
            // SAFETY: created by opus_encoder_create, destroyed once.
            unsafe { sys::opus_encoder_destroy(self.0) }
        }
    }

    pub struct Decoder(*mut sys::OpusDecoder);
    unsafe impl Send for Decoder {}

    impl Decoder {
        pub fn new() -> Option<Decoder> {
            let mut err = 0;
            // SAFETY: as for the encoder.
            let st = unsafe { sys::opus_decoder_create(super::RATE as i32, 1, &mut err) };
            (!st.is_null() && err == 0).then_some(Decoder(st))
        }

        /// Decode a packet into `out` (one frame). `data` None conceals a lost frame; `fec`
        /// rebuilds the frame before `data` from the copy it carries.
        pub fn decode(&mut self, data: Option<&[u8]>, fec: bool, out: &mut [f32]) -> usize {
            let (p, n) = data.map(|d| (d.as_ptr(), d.len() as i32)).unwrap_or((std::ptr::null(), 0));
            // SAFETY: data (or null for concealment) and a writable output of out.len() samples.
            let got = unsafe { sys::opus_decode_float(self.0, p, n, out.as_mut_ptr(), out.len() as i32, fec as i32) };
            got.max(0) as usize
        }
    }

    impl Drop for Decoder {
        fn drop(&mut self) {
            // SAFETY: created by opus_decoder_create, destroyed once.
            unsafe { sys::opus_decoder_destroy(self.0) }
        }
    }
}

// ---- shared state -------------------------------------------------------------------------

#[derive(Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Relaxed))
    }
    pub fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Relaxed)
    }
}

/// One person we hear.
#[derive(Default)]
pub struct Speaker {
    buf: Mutex<VecDeque<f32>>,
    /// Where the voice sits: left and right gain, and the low-pass coefficient (1 = clear).
    gain_l: AtomicF32,
    gain_r: AtomicF32,
    cut: AtomicF32,
    /// Loudness of what is playing right now (0..1): the mouth follows it.
    pub level: AtomicF32,
    /// The lane of their last packet (0 group, 1 near).
    pub lane: AtomicU8,
    /// Milliseconds (since the voice started) of their last packet.
    heard: AtomicU32,
    gone: AtomicBool,
}

/// What the game and the voice thread share.
#[derive(Default)]
pub struct Shared {
    /// May send right now (mode on, key held or mic open, in a game).
    pub transmit: AtomicBool,
    /// Open microphone: send only while the voice is above `threshold`.
    pub gate: AtomicBool,
    pub threshold: AtomicF32,
    pub lane: AtomicU8,
    pub group: AtomicU8,
    /// The microphone's loudness (0..1), for the meter and our own mouth.
    pub mic_level: AtomicF32,
    /// Frames are going out now.
    pub sending: AtomicBool,
    /// Why the microphone is not working (empty = fine).
    pub mic_error: Mutex<String>,
    /// Microphones found, once listed.
    pub mics: Mutex<Vec<String>>,
    pub speakers: Mutex<HashMap<u32, Arc<Speaker>>>,
}

enum Cmd {
    /// Open this microphone ("" = default), or close it.
    Mic(Option<String>),
    /// Where our voice goes and where others' comes from (None = not in a game).
    Link(Option<(VoiceOut, Receiver<Vec<u8>>)>),
    ListMics,
}

/// The running voice engine.
pub struct Voice {
    pub shared: Arc<Shared>,
    cmd: Sender<Cmd>,
}

impl Voice {
    pub fn start(out: Option<OutputStreamHandle>) -> Voice {
        let shared = Arc::new(Shared::default());
        let (cmd, rx) = unbounded();
        let sh = shared.clone();
        std::thread::Builder::new().name("voz".into()).spawn(move || run(sh, rx, out)).ok();
        Voice { shared, cmd }
    }

    fn send(&self, c: Cmd) {
        let _ = self.cmd.send(c);
    }
}

// ---- the voice thread ---------------------------------------------------------------------

/// Straight-line resampling to 48 kHz: plenty for speech, and nearly free.
struct Resampler {
    step: f64,
    t: f64,
    prev: f32,
}

impl Resampler {
    fn new(from: u32) -> Resampler {
        Resampler { step: from as f64 / RATE as f64, t: 0.0, prev: 0.0 }
    }

    fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        if input.is_empty() {
            return;
        }
        let at = |i: usize| if i == 0 { self.prev } else { input[i - 1] };
        let len = input.len() as f64;
        while self.t < len {
            let i = self.t.floor();
            let f = (self.t - i) as f32;
            let i = i as usize;
            out.push(at(i) * (1.0 - f) + at(i + 1) * f);
            self.t += self.step;
        }
        self.t -= len;
        self.prev = input[input.len() - 1];
    }
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

/// Loudness → 0..1 (speech sits around 0.3..0.8).
fn meter(rms: f32) -> f32 {
    ((rms.max(1e-5).log10() + 3.0) / 2.5).clamp(0.0, 1.0)
}

struct Mic {
    /// None when a test file stands in for the microphone.
    _stream: Option<cpal::Stream>,
    rx: Receiver<Vec<f32>>,
    rs: Resampler,
}

fn list_mics() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices().map(|d| d.filter_map(|d| d.name().ok()).collect()).unwrap_or_default()
}

/// Test hook MC_VOICE_FILE=voice.wav: that recording, looped, instead of the microphone.
fn file_mic(path: &str) -> Result<Mic, String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let d = rodio::Decoder::new(std::io::BufReader::new(f)).map_err(|e| e.to_string())?;
    let (ch, rate) = (d.channels().max(1) as usize, d.sample_rate());
    let all: Vec<f32> = d.convert_samples::<f32>().collect::<Vec<_>>().chunks(ch).map(|c| c.iter().sum::<f32>() / ch as f32).collect();
    if all.is_empty() {
        return Err("arquivo vazio".into());
    }
    let (tx, rx) = bounded::<Vec<f32>>(64);
    std::thread::spawn(move || {
        let step = rate as usize / 100;
        let t0 = Instant::now();
        let mut k = 0usize;
        loop {
            let chunk: Vec<f32> = (0..step).map(|i| all[(k * step + i) % all.len()]).collect();
            if tx.send(chunk).is_err() {
                return;
            }
            k += 1;
            let due = t0 + Duration::from_millis(10 * k as u64);
            std::thread::sleep(due.saturating_duration_since(Instant::now()));
        }
    });
    Ok(Mic { _stream: None, rx, rs: Resampler::new(rate) })
}

fn open_mic(name: &str) -> Result<Mic, String> {
    if let Ok(p) = std::env::var("MC_VOICE_FILE") {
        return file_mic(&p);
    }
    let host = cpal::default_host();
    let dev = if name.is_empty() {
        host.default_input_device()
    } else {
        host.input_devices().ok().and_then(|mut d| d.find(|d| d.name().ok().as_deref() == Some(name))).or_else(|| host.default_input_device())
    }
    .ok_or("nenhum microfone encontrado")?;
    let cfg = dev.default_input_config().map_err(|e| e.to_string())?;
    let ch = cfg.channels().max(1) as usize;
    let rate = cfg.sample_rate().0;
    let (tx, rx) = bounded::<Vec<f32>>(64);
    let err = |e: cpal::StreamError| log::warn!("microphone: {e}");
    // Mixed down to mono right in the callback; a full queue drops rather than waits.
    fn mono<T: Copy>(data: &[T], ch: usize, f: impl Fn(T) -> f32) -> Vec<f32> {
        data.chunks(ch).map(|c| c.iter().map(|&s| f(s)).sum::<f32>() / ch as f32).collect()
    }
    let sc = cfg.config();
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => dev.build_input_stream(&sc, move |d: &[f32], _: &_| drop(tx.try_send(mono(d, ch, |s| s))), err, None),
        cpal::SampleFormat::I16 => dev.build_input_stream(&sc, move |d: &[i16], _: &_| drop(tx.try_send(mono(d, ch, |s| s as f32 / 32768.0))), err, None),
        cpal::SampleFormat::U16 => {
            dev.build_input_stream(&sc, move |d: &[u16], _: &_| drop(tx.try_send(mono(d, ch, |s| (s as f32 - 32768.0) / 32768.0))), err, None)
        }
        cpal::SampleFormat::I32 => dev.build_input_stream(&sc, move |d: &[i32], _: &_| drop(tx.try_send(mono(d, ch, |s| s as f32 / 2_147_483_648.0))), err, None),
        f => return Err(format!("formato do microfone não suportado: {f:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(Mic { _stream: Some(stream), rx, rs: Resampler::new(rate) })
}

/// Our side of one speaker: their decoder and the next packet number expected.
struct Incoming {
    dec: opus::Decoder,
    next: u16,
}

fn run(sh: Arc<Shared>, cmds: Receiver<Cmd>, out: Option<OutputStreamHandle>) {
    let start = Instant::now();
    let mut mic: Option<Mic> = None;
    let mut link: Option<(VoiceOut, Receiver<Vec<u8>>)> = None;
    let mut enc = opus::Encoder::new();
    let mut seq: u16 = 0;
    let mut pcm: Vec<f32> = Vec::with_capacity(FRAME * 4);
    let mut hang = 0.0f32;
    let mut level = 0.0f32;
    let mut incoming: HashMap<u32, Incoming> = HashMap::new();
    let mut frame = vec![0.0f32; FRAME];
    let mut bytes = vec![0u8; 400];
    let mut tidy = Instant::now();
    loop {
        let mic_rx = mic.as_ref().map(|m| m.rx.clone()).unwrap_or_else(never);
        let net_rx = link.as_ref().map(|l| l.1.clone()).unwrap_or_else(never);
        select! {
            recv(cmds) -> c => match c {
                Err(_) => return,
                Ok(Cmd::Mic(None)) => {
                    mic = None;
                    sh.mic_level.set(0.0);
                    sh.sending.store(false, Relaxed);
                }
                Ok(Cmd::Mic(Some(name))) => {
                    mic = None;
                    match open_mic(&name) {
                        Ok(m) => {
                            mic = Some(m);
                            sh.mic_error.lock().unwrap().clear();
                        }
                        Err(e) => {
                            log::warn!("microphone: {e}");
                            *sh.mic_error.lock().unwrap() = e;
                        }
                    }
                    pcm.clear();
                }
                Ok(Cmd::Link(l)) => {
                    link = l;
                    incoming.clear();
                    for s in sh.speakers.lock().unwrap().drain() {
                        s.1.gone.store(true, Relaxed);
                    }
                }
                Ok(Cmd::ListMics) => *sh.mics.lock().unwrap() = list_mics(),
            },
            recv(mic_rx) -> d => {
                let Ok(d) = d else { mic = None; continue };
                let Some(m) = mic.as_mut() else { continue };
                m.rs.push(&d, &mut pcm);
                let mut used = 0;
                while pcm.len() - used >= FRAME {
                    let f = &pcm[used..used + FRAME];
                    used += FRAME;
                    let lv = meter(rms(f));
                    level = if lv > level { lv } else { level * 0.8 + lv * 0.2 };
                    sh.mic_level.set(level);
                    let gate = sh.gate.load(Relaxed);
                    if lv >= sh.threshold.get() {
                        hang = HANG;
                    } else {
                        hang -= 0.02;
                    }
                    let send = sh.transmit.load(Relaxed) && (!gate || hang > 0.0);
                    sh.sending.store(send, Relaxed);
                    if !send {
                        continue;
                    }
                    let (Some(e), Some((out, _))) = (enc.as_mut(), link.as_ref()) else { continue };
                    if let Some(n) = e.encode(f, &mut bytes) {
                        let lane = if sh.lane.load(Relaxed) == 1 { Lane::Near } else { Lane::Group };
                        out.send(&Packet { from: 0, seq, lane, group: sh.group.load(Relaxed), data: &bytes[..n] }.encode());
                        seq = seq.wrapping_add(1);
                    }
                }
                pcm.drain(..used);
            },
            recv(net_rx) -> b => {
                let Ok(b) = b else { link = None; continue };
                let Some(p) = Packet::decode(&b) else { continue };
                let sp = {
                    let mut all = sh.speakers.lock().unwrap();
                    all.entry(p.from)
                        .or_insert_with(|| {
                            log::info!("voice: hearing player {}", p.from);
                            let s = Arc::new(Speaker::default());
                            s.cut.set(1.0);
                            if let Some(h) = &out {
                                let _ = h.play_raw(Playback::new(s.clone()));
                            }
                            s
                        })
                        .clone()
                };
                sp.lane.store(if p.lane == Lane::Near { 1 } else { 0 }, Relaxed);
                sp.heard.store(start.elapsed().as_millis() as u32, Relaxed);
                let inc = match incoming.entry(p.from) {
                    std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
                    std::collections::hash_map::Entry::Vacant(v) => {
                        let Some(dec) = opus::Decoder::new() else { continue };
                        v.insert(Incoming { dec, next: p.seq })
                    }
                };
                let gap = p.seq.wrapping_sub(inc.next) as i16;
                if gap < 0 {
                    continue; // late or repeated
                }
                let mut buf = sp.buf.lock().unwrap();
                if gap > 0 && gap <= 4 {
                    // Lost frames: conceal all but the last, and rebuild that one from the
                    // copy this packet carries.
                    for _ in 1..gap {
                        let n = inc.dec.decode(None, false, &mut frame);
                        buf.extend(&frame[..n]);
                    }
                    let n = inc.dec.decode(Some(p.data), true, &mut frame);
                    buf.extend(&frame[..n]);
                }
                let n = inc.dec.decode(Some(p.data), false, &mut frame);
                buf.extend(&frame[..n]);
                inc.next = p.seq.wrapping_add(1);
                let extra = buf.len().saturating_sub(MOST);
                buf.drain(..extra);
            },
            default(Duration::from_millis(100)) => {}
        }
        // People not heard for a while: let their playback end.
        if tidy.elapsed() > Duration::from_secs(5) {
            tidy = Instant::now();
            let now = start.elapsed().as_millis() as u32;
            sh.speakers.lock().unwrap().retain(|id, s| {
                let keep = now.wrapping_sub(s.heard.load(Relaxed)) < 30_000;
                if !keep {
                    s.gone.store(true, Relaxed);
                    incoming.remove(id);
                }
                keep
            });
        }
    }
}

/// One speaker's voice as an endless stereo source: it waits for a little to build up, then
/// plays, panned and muffled as the game says, and goes quiet when the buffer runs dry.
struct Playback {
    sp: Arc<Speaker>,
    chunk: [f32; CHUNK],
    len: usize,
    pos: usize,
    playing: bool,
    right: Option<f32>,
    lp: f32,
    gl: f32,
    gr: f32,
    cut: f32,
}

impl Playback {
    fn new(sp: Arc<Speaker>) -> Playback {
        Playback { sp, chunk: [0.0; CHUNK], len: 0, pos: 0, playing: false, right: None, lp: 0.0, gl: 0.0, gr: 0.0, cut: 1.0 }
    }

    fn refill(&mut self) {
        let mut b = self.sp.buf.lock().unwrap();
        if !self.playing && b.len() >= START {
            self.playing = true;
        }
        self.len = 0;
        if self.playing {
            self.len = b.len().min(CHUNK);
            for (d, s) in self.chunk.iter_mut().zip(b.drain(..self.len)) {
                *d = s;
            }
            if self.len == 0 {
                self.playing = false;
            }
        }
        drop(b);
        if self.len == 0 {
            self.chunk = [0.0; CHUNK];
            self.sp.level.set(0.0);
        } else {
            self.sp.level.set(meter(rms(&self.chunk[..self.len])));
        }
        self.len = CHUNK;
        self.pos = 0;
        self.cut = self.sp.cut.get().clamp(0.02, 1.0);
    }
}

impl Iterator for Playback {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if let Some(r) = self.right.take() {
            return Some(r);
        }
        if self.pos >= self.len {
            if self.sp.gone.load(Relaxed) {
                return None;
            }
            self.refill();
        }
        let x = self.chunk[self.pos];
        self.pos += 1;
        self.lp += self.cut * (x - self.lp);
        // Gains glide (about 5 ms) so a voice moving around never clicks.
        self.gl += (self.sp.gain_l.get() - self.gl) * 0.004;
        self.gr += (self.sp.gain_r.get() - self.gr) * 0.004;
        self.right = Some(self.lp * self.gr);
        Some(self.lp * self.gl)
    }
}

impl Source for Playback {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        RATE
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

// ---- the game's side ------------------------------------------------------------------------

/// What the game keeps about voice between frames.
#[derive(Default)]
pub struct VoiceState {
    pub engine: Option<Voice>,
    /// The microphone asked of the engine (None = closed).
    mic_open: Option<String>,
    /// Which game the link points at (0 = none, 1 = hosting, 2 = guest).
    linked: u8,
    /// Group mode with the talk key: the microphone is on (K toggles it in open-mic mode).
    pub muted: bool,
    /// Last voice settings the host was told (guest).
    pub(crate) told: Option<(u8, bool)>,
    /// Our own mouth.
    pub talk: f32,
    /// Smoothed mouth for each remote player.
    mouths: HashMap<u32, f32>,
    listed: bool,
    shot: bool,
}

/// Test hook MC_VOICE_TALK=1: the talk key is held.
fn test_talk() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("MC_VOICE_TALK").is_ok())
}

/// Test hook MC_VOICE_SHOT=seconds: after that long, a screenshot the first time a mouth (ours
/// in third person, or someone else's) is wide open.
fn test_shot() -> Option<f32> {
    static V: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("MC_VOICE_SHOT").ok().and_then(|v| v.parse().ok()))
}

/// How a voice level moves the mouth: quick to open, a bit slower to close, like speech.
fn follow(cur: f32, level: f32, dt: f32) -> f32 {
    let target = ((level - 0.12) * 1.6).clamp(0.0, 1.0);
    let k = if target > cur { 30.0 } else { 14.0 };
    cur + (target - cur) * (1.0 - (-dt * k).exp())
}

impl Game {
    fn voice_on(&self) -> bool {
        self.settings.voice.mode != VoiceMode::Off && self.net.active() && self.world_open && !self.net.server
    }

    /// Every frame: keep the engine linked to the game, tell it when to send, place each voice
    /// and move the mouths.
    pub(crate) fn voice_update(&mut self, dt: f32) {
        let on = self.voice_on();
        let meter = self.screen == Screen::Voice;
        // Who hears whom is decided by the host, even one without sound (a dedicated server).
        self.voice_routes();
        if !on && !meter && self.voice.engine.is_none() {
            return;
        }
        if self.voice.engine.is_none() {
            if self.audio.is_none() || self.args.screenshot.is_some() {
                return;
            }
            self.voice.engine = Some(Voice::start(self.audio.as_ref().map(|a| a.handle.clone())));
        }
        let vs = self.settings.voice.clone();
        let want_mic = (on || meter).then(|| vs.mic.clone());
        let link_key = if !on {
            0
        } else if self.net.is_host() {
            1
        } else if self.net_guest_joined() {
            2
        } else {
            0
        };
        let link = (link_key != self.voice.linked).then(|| self.net_voice_link());
        let Some(v) = self.voice.engine.as_ref() else { return };
        if want_mic != self.voice.mic_open {
            v.send(Cmd::Mic(want_mic.clone()));
            self.voice.mic_open = want_mic;
        }
        if let Some(l) = link {
            v.send(Cmd::Link(l));
            self.voice.linked = link_key;
        }
        if meter && !self.voice.listed {
            v.send(Cmd::ListMics);
            self.voice.listed = true;
        }
        let playing = self.screen == Screen::Playing || self.screen == Screen::Inventory;
        let talk_key = self.keys.contains(&KeyCode::CapsLock) || test_talk();
        let transmit = on
            && playing
            && !self.voice.muted
            && match vs.mode {
                VoiceMode::Near => talk_key,
                VoiceMode::Group => vs.open_mic || talk_key,
                VoiceMode::Off => false,
            };
        let sh = v.shared.clone();
        sh.transmit.store(transmit, Relaxed);
        sh.gate.store(vs.mode == VoiceMode::Group && vs.open_mic && !talk_key, Relaxed);
        sh.threshold.set(vs.sensitivity);
        sh.lane.store(if vs.mode == VoiceMode::Near { 1 } else { 0 }, Relaxed);
        sh.group.store(vs.group, Relaxed);
        let mine = if sh.sending.load(Relaxed) && transmit { sh.mic_level.get() } else { 0.0 };
        self.voice.talk = follow(self.voice.talk, mine, dt);
        self.anim.talk = self.voice.talk;
        let shot_after = test_shot();
        if self.voice.talk > 0.6 && shot_after.is_some_and(|t| self.now > t) && self.camera_mode != 0 && !self.voice.shot {
            self.voice.shot = true;
            self.screenshot_request = true;
        }

        // Place every voice we hear, and open its speaker's mouth.
        let speakers: Vec<(u32, Arc<Speaker>)> = sh.speakers.lock().unwrap().iter().map(|(k, v)| (*k, v.clone())).collect();
        let my_dim = self.dim.as_u8();
        for (id, sp) in speakers {
            let (name, at) = match self.net.remotes.get(&id) {
                Some(r) => (r.name.clone(), (r.state.dim == my_dim).then(|| r.shown + Vec3::Y * 1.6)),
                None => (String::new(), None),
            };
            let personal = vs.players.get(&name).copied().unwrap_or(1.0);
            let vol = vs.volume * personal * 1.4 * self.settings.sound_volume;
            let (gl, gr, cut) = if sp.lane.load(Relaxed) == 0 {
                (vol, vol, 1.0)
            } else if let Some(at) = at {
                match self.place_sound(at, net::voice::RANGE) {
                    Some((gain, pan, muffle)) => {
                        let angle = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
                        let g = vol * gain;
                        let cutoff = 9000.0 * (450.0f32 / 9000.0).powf(muffle.clamp(0.0, 1.0));
                        let a = 1.0 - (-std::f32::consts::TAU * cutoff / RATE as f32).exp();
                        (g * (angle.cos() * std::f32::consts::SQRT_2).min(1.25), g * (angle.sin() * std::f32::consts::SQRT_2).min(1.25), a)
                    }
                    None => (0.0, 0.0, 1.0),
                }
            } else {
                (0.0, 0.0, 1.0)
            };
            sp.gain_l.set(gl);
            sp.gain_r.set(gr);
            sp.cut.set(cut);
            let audible = gl.max(gr) > 0.001;
            let m = self.voice.mouths.entry(id).or_insert(0.0);
            *m = follow(*m, if audible || personal > 0.0 { sp.level.get() } else { 0.0 }, dt);
            let m = *m;
            if let Some(r) = self.net.remotes.get_mut(&id) {
                r.set_talk(m);
            }
            if m > 0.6 && shot_after.is_some_and(|t| self.now > t) && !self.voice.shot {
                self.voice.shot = true;
                self.screenshot_request = true;
                log::info!("voice: player {id} talking (mouth {m:.2}, gains {gl:.2}/{gr:.2})");
            }
        }
    }

    /// A key for voice: K turns the microphone off and on.
    pub(crate) fn voice_key(&mut self, code: KeyCode) -> bool {
        if code != KeyCode::KeyK || self.screen != Screen::Playing || !self.voice_on() {
            return false;
        }
        self.voice.muted = !self.voice.muted;
        self.toast(if self.voice.muted { "Microfone desligado" } else { "Microfone ligado" }, 1.5);
        true
    }

    /// Who is talking (top left, like a call), and our own microphone's state.
    pub(crate) fn draw_voice_overlay(&mut self) {
        if !self.voice_on() || self.hide_hud {
            return;
        }
        let s = self.ui.scale;
        let mut rows: Vec<(String, f32, [u8; 4])> = Vec::new();
        let vs = &self.settings.voice;
        if self.voice.muted {
            rows.push(("Microfone desligado (K)".into(), 0.0, GRAY));
        } else if self.voice.talk > 0.02 || self.voice.engine.as_ref().is_some_and(|v| v.shared.sending.load(Relaxed)) {
            let place = if vs.mode == VoiceMode::Group { format!("grupo {}", vs.group) } else { "perto".into() };
            rows.push((format!("{} ({place})", self.net.cfg.name), self.voice.talk, YELLOW));
        }
        if let Some(v) = &self.voice.engine {
            if let Ok(e) = v.shared.mic_error.lock() {
                if !e.is_empty() {
                    rows.push((format!("Microfone: {e}"), 0.0, GRAY));
                }
            }
        }
        let mut talking: Vec<(String, f32)> = self.voice.mouths.iter().filter(|(_, m)| **m > 0.02).filter_map(|(id, m)| self.net.remotes.get(id).map(|r| (r.name.clone(), *m))).collect();
        talking.sort_by(|a, b| a.0.cmp(&b.0));
        rows.extend(talking.into_iter().map(|(n, m)| (n, m, WHITE)));
        let mut y = 4.0 * s;
        for (name, level, color) in rows {
            let tw = self.ui.text_width(&name, s);
            self.ui.rect(2.0 * s, y - s, tw + 16.0 * s, 10.0 * s, [0, 0, 0, 110]);
            // A little speaker that lights up with the voice.
            let g = (90.0 + level * 165.0) as u8;
            self.ui.rect(4.0 * s, y + 2.0 * s, 3.0 * s, 4.0 * s, [60, g, 90, 255]);
            self.ui.rect(7.0 * s, y + (2.0 - level * 2.0) * s, 2.0 * s, (4.0 + level * 4.0) * s, [60, g, 90, 255]);
            self.ui.text(12.0 * s, y, &name, color);
            y += 11.0 * s;
        }
    }

    /// The voice options screen.
    pub(crate) fn draw_voice_options(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        self.ui.text_centered(w / 2.0, 8.0 * s, "Chat de voz", WHITE);
        let vs = self.settings.voice.clone();
        let mic_name = if vs.mic.is_empty() { "Padrão do sistema".to_string() } else { vs.mic.clone() };
        let pct = |v: f32| format!("{:.0}%", v * 100.0);
        let mut labels = vec![
            format!("Modo: {}", vs.mode.label()),
            format!("Grupo: {}", vs.group),
            format!("No grupo: {}", if vs.open_mic { "microfone aberto" } else { "tecla para falar" }),
            format!("Microfone: {}", short(&mic_name, 28)),
            format!("Sensibilidade: {}", pct(vs.sensitivity)),
            format!("Volume das vozes: {}", pct(vs.volume)),
        ];
        let mut names: Vec<String> = self.net.remotes.values().filter(|r| !r.state.has(net::flags::HIDDEN)).map(|r| r.name.clone()).collect();
        names.sort();
        names.dedup();
        for n in &names {
            let v = vs.players.get(n).copied().unwrap_or(1.0);
            labels.push(format!("{}: {}", short(n, 16), if v <= 0.0 { "mudo".to_string() } else { pct(v) }));
        }
        let bw = 150.0 * s;
        let bh = 18.0 * s;
        let gap = 3.0 * s;
        let y0 = 22.0 * s;
        let mut hit = None;
        for (i, l) in labels.iter().enumerate() {
            let x = w / 2.0 - bw - gap / 2.0 + (i % 2) as f32 * (bw + gap);
            let y = y0 + (i / 2) as f32 * (bh + gap);
            if self.ui.button(x, y, bw, bh, l, mouse) && self.clicked {
                hit = Some(i);
            }
        }
        let rows = labels.len().div_ceil(2);
        let mut y = y0 + rows as f32 * (bh + gap) + 4.0 * s;
        // The microphone meter, with the open-mic threshold marked.
        let (level, sending, err) = match &self.voice.engine {
            Some(v) => (v.shared.mic_level.get(), v.shared.sending.load(Relaxed), v.shared.mic_error.lock().map(|e| e.clone()).unwrap_or_default()),
            None => (0.0, false, String::new()),
        };
        let mw = 2.0 * bw + gap;
        let mx = w / 2.0 - mw / 2.0;
        self.ui.rect(mx, y, mw, 8.0 * s, [0, 0, 0, 160]);
        let col = if level >= vs.sensitivity { [90, 220, 110, 255] } else { [150, 150, 150, 255] };
        self.ui.rect(mx, y, mw * level, 8.0 * s, col);
        self.ui.rect(mx + mw * vs.sensitivity - s * 0.5, y - s, s, 10.0 * s, YELLOW);
        y += 11.0 * s;
        let note = if !err.is_empty() {
            format!("Microfone: {err}")
        } else if sending {
            "Enviando sua voz".to_string()
        } else {
            "Fale para testar o microfone. Caps Lock: falar   K: desligar o microfone".to_string()
        };
        self.ui.text_centered(w / 2.0, y, &note, GRAY);
        if names.is_empty() {
            self.ui.text_centered(w / 2.0, y + 11.0 * s, "Volume de cada jogador aparece aqui quando houver outros jogadores.", GRAY);
        }
        let done = self.ui.button(w / 2.0 - 100.0 * s, h - 28.0 * s, 200.0 * s, 20.0 * s, "Concluído", mouse) && self.clicked;
        let cycle = |v: f32, steps: &[f32]| -> f32 { steps.iter().copied().find(|&x| x > v + 1e-3).unwrap_or(steps[0]) };
        if let Some(i) = hit {
            self.sfx(crate::audio::Sfx::Click);
            let st = &mut self.settings.voice;
            match i {
                0 => st.mode = st.mode.next(),
                1 => st.group = st.group % 9 + 1,
                2 => st.open_mic = !st.open_mic,
                3 => {
                    let mut all = vec![String::new()];
                    if let Some(v) = &self.voice.engine {
                        all.extend(v.shared.mics.lock().unwrap().iter().cloned());
                    }
                    let k = all.iter().position(|m| *m == st.mic).map(|k| (k + 1) % all.len()).unwrap_or(0);
                    st.mic = all[k].clone();
                }
                4 => st.sensitivity = cycle(st.sensitivity, &[0.1, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6]),
                5 => st.volume = cycle(st.volume, &[0.0, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0]),
                n => {
                    if let Some(name) = names.get(n - 6) {
                        let v = st.players.get(name).copied().unwrap_or(1.0);
                        let v = cycle(v, &[0.0, 0.5, 1.0, 1.5, 2.0]);
                        if (v - 1.0).abs() < 1e-3 {
                            st.players.remove(name);
                        } else {
                            st.players.insert(name.clone(), v);
                        }
                    }
                }
            }
            self.settings.save(&self.base.join("settings.ron"));
        }
        if done {
            self.sfx(crate::audio::Sfx::Click);
            self.set_screen(Screen::Options);
        }
    }
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(n - 3).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_roundtrip_keeps_a_voice() {
        let mut e = opus::Encoder::new().unwrap();
        let mut d = opus::Decoder::new().unwrap();
        let mut bytes = [0u8; 400];
        let mut out = [0.0f32; FRAME];
        let mut energy = 0.0;
        for k in 0..20 {
            let f: Vec<f32> = (0..FRAME).map(|i| ((k * FRAME + i) as f32 * 0.03).sin() * 0.3).collect();
            let n = e.encode(&f, &mut bytes).unwrap();
            assert!(n < 200, "{n} bytes for 20 ms");
            assert_eq!(d.decode(Some(&bytes[..n]), false, &mut out), FRAME);
            energy = rms(&out);
        }
        assert!(energy > 0.1, "{energy}");
        // A lost frame is concealed, not silent garbage.
        assert_eq!(d.decode(None, false, &mut out), FRAME);
    }

    #[test]
    fn resampler_keeps_the_rate() {
        let mut r = Resampler::new(44_100);
        let mut out = Vec::new();
        for _ in 0..100 {
            r.push(&[0.5; 441], &mut out);
        }
        assert!((out.len() as i32 - 48_000).abs() <= 2, "{}", out.len());
        assert!(out[100..].iter().all(|v| (v - 0.5).abs() < 1e-4));
    }

    #[test]
    fn playback_waits_then_plays_then_rests() {
        let sp = Arc::new(Speaker::default());
        sp.gain_l.set(1.0);
        sp.gain_r.set(0.0);
        sp.cut.set(1.0);
        let mut p = Playback::new(sp.clone());
        sp.buf.lock().unwrap().extend(std::iter::repeat(0.5).take(FRAME));
        // Not enough yet: silence.
        assert!(p.by_ref().take(CHUNK * 2).all(|v| v == 0.0));
        sp.buf.lock().unwrap().extend(std::iter::repeat(0.5).take(START));
        let got: Vec<f32> = p.by_ref().take(CHUNK * 2 * 4).collect();
        assert!(got.iter().step_by(2).last().unwrap() > &0.3, "left plays");
        assert!(got.iter().skip(1).step_by(2).all(|v| v.abs() < 1e-6), "right is silent");
        assert!(sp.level.get() > 0.5);
        sp.gone.store(true, Relaxed);
        assert!(p.by_ref().take(CHUNK * 4).count() < CHUNK * 4, "ends when gone");
    }

    #[test]
    fn mouths_open_with_the_voice_and_close_after() {
        let mut m = 0.0;
        for _ in 0..10 {
            m = follow(m, 0.7, 1.0 / 60.0);
        }
        assert!(m > 0.5, "{m}");
        for _ in 0..30 {
            m = follow(m, 0.0, 1.0 / 60.0);
        }
        assert!(m < 0.05, "{m}");
    }
}
