//! Mine Clone: a fast block game written in Rust.

// No console window behind the game on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod renderer;
mod settings;
mod textures;
mod ui;
mod world;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use glam::{IVec3, Vec3};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

use audio::{Audio, Material, Sfx, Situation};
use renderer::{Camera, Globals, Renderer};
use settings::Settings;
use ui::{FontAtlas, UiBatch, GRAY, WHITE, YELLOW};
use voxel::block::Shape;
use voxel::chunk::ChunkPos;
use voxel::mesh::MeshTables;
use voxel::physics::{Body, Input};
use voxel::raycast::{raycast, RayHit};
use voxel::save::{LevelData, WorldSave};
use voxel::worldgen::{Biome, Generator};
use voxel::{BlockId, BlockRegistry, AIR};
use world::World;

const REACH: f32 = 6.0;
const CLOUD_HEIGHT: f32 = 160.0;
const RENDER_DISTANCES: [i32; 9] = [4, 6, 8, 10, 12, 16, 20, 24, 32];

#[derive(Default, Clone)]
struct Args {
    world: Option<String>,
    seed: Option<u64>,
    screenshot: Option<PathBuf>,
    pos: Option<Vec3>,
    look: Option<(f32, f32)>,
    time: Option<f32>,
    render_distance: Option<i32>,
    size: Option<(u32, u32)>,
    fly: bool,
    /// Build a few things in front of the player (used for test screenshots).
    demo: bool,
    show_inventory: bool,
    show_pause: bool,
    debug: bool,
}

fn parse_args() -> Args {
    let mut a = Args::default();
    let mut it = std::env::args().skip(1);
    let f3 = |s: &str| -> Option<Vec3> {
        let v: Vec<f32> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (v.len() == 3).then(|| Vec3::new(v[0], v[1], v[2]))
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--world" => a.world = it.next(),
            "--seed" => a.seed = it.next().and_then(|s| s.parse().ok()),
            "--screenshot" => a.screenshot = it.next().map(PathBuf::from),
            "--pos" => a.pos = it.next().and_then(|s| f3(&s)),
            "--look" => {
                a.look = it.next().and_then(|s| {
                    let v: Vec<f32> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                    (v.len() == 2).then(|| (v[0], v[1]))
                })
            }
            "--time" => a.time = it.next().and_then(|s| s.parse().ok()),
            "--render-distance" => a.render_distance = it.next().and_then(|s| s.parse().ok()),
            "--size" => {
                a.size = it.next().and_then(|s| {
                    let (w, h) = s.split_once('x')?;
                    Some((w.parse().ok()?, h.parse().ok()?))
                })
            }
            "--fly" => a.fly = true,
            "--demo" => a.demo = true,
            "--inventory" => a.show_inventory = true,
            "--pause" => a.show_pause = true,
            "--debug" => a.debug = true,
            "--help" | "-h" => {
                println!(
                    "mine-clone [--world NAME] [--seed N] [--render-distance N] [--size WxH]\n\
                     Testing: [--screenshot out.png] [--pos x,y,z] [--look yaw,pitch] [--time ticks] [--fly] [--demo] [--inventory] [--pause] [--debug]"
                );
                std::process::exit(0);
            }
            other => eprintln!("unknown argument {other}"),
        }
    }
    a
}

/// Folder for settings, saves, resource packs and music: the source tree when run with cargo,
/// otherwise the folder the executable is in.
fn base_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join("packs/vanilla").is_dir() {
        return cwd;
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        // macOS app bundles: keep data next to the .app, not inside it.
        if dir.ends_with("Contents/MacOS") {
            if let Some(outside) = dir.ancestors().nth(3) {
                return outside.to_path_buf();
            }
        }
        return dir;
    }
    cwd
}

fn ensure_folders(base: &Path, settings: &Settings) {
    let rp = base.join("resourcepacks");
    if !rp.exists() && std::fs::create_dir_all(&rp).is_ok() {
        let _ = std::fs::write(
            rp.join("LEIA-ME.txt"),
            "Coloque aqui pacotes de textura (pastas ou arquivos .zip).\n\
             Funcionam pacotes deste jogo (textures/block/NOME.png) e pacotes do Minecraft\n\
             (assets/minecraft/textures/block/...). Escolha o pacote no menu de pausa (Esc).\n\n\
             Put texture packs here (folders or .zip files). Pick one in the pause menu (Esc).\n",
        );
    }
    let mf = base.join(&settings.music_folder);
    if !mf.exists() && std::fs::create_dir_all(&mf).is_ok() {
        let _ = std::fs::write(
            mf.join("LEIA-ME.txt"),
            "Coloque aqui suas músicas (mp3, ogg, wav ou flac).\n\
             Arquivos soltos tocam em qualquer lugar. Para escolher onde tocam, use subpastas:\n\
             overworld, underwater, creative, nether, end, title.\n\
             No menu de pausa (Esc), mude 'Música' para 'Minha pasta' ou 'Ambas'.\n\n\
             Put your own songs here. In the pause menu set Music to 'Minha pasta' (yours only) or 'Ambas' (both).\n",
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Loading,
    Playing,
    Paused,
    Inventory,
}

struct Toast {
    text: String,
    until: f32,
}

struct Game {
    window: Arc<Window>,
    renderer: Renderer,
    world: World,
    reg: Arc<BlockRegistry>,
    settings: Settings,
    base: PathBuf,
    audio: Option<Audio>,
    args: Args,
    world_name: String,
    seed: u64,

    body: Body,
    yaw: f32,
    pitch: f32,
    spawn_xz: (i32, i32),
    needs_spawn: bool,

    hotbar: [BlockId; 9],
    selected: usize,
    inventory: Vec<BlockId>,
    inv_scroll: usize,
    time_of_day: f32,

    screen: Screen,
    keys: HashSet<KeyCode>,
    mouse_held: [bool; 3],
    mouse_pos: (f32, f32),
    clicked: bool,
    last_space: f32,
    break_cd: f32,
    place_cd: f32,
    target: Option<RayHit>,
    step_dist: f32,

    ui: UiBatch,
    show_debug: bool,
    hide_hud: bool,
    toast: Option<Toast>,
    packs: Vec<String>,
    textures_name: String,

    start: Instant,
    last: Instant,
    now: f32,
    frame_times: Vec<f32>,
    fps: f32,
    last_frame_info: (usize, usize),
    autosave: f32,
    loaded_frames: u32,
    screenshot_request: bool,
    grabbed: bool,
}

fn level_path_hint(base: &Path, name: &str) -> PathBuf {
    base.join("saves").join(name)
}

impl Game {
    fn new(window: Arc<Window>, args: Args) -> Game {
        let base = base_dir();
        let settings_path = base.join("settings.ron");
        let mut settings = Settings::load(&settings_path);
        if let Some(rd) = args.render_distance {
            settings.render_distance = rd;
        }
        ensure_folders(&base, &settings);

        // Blocks: a blocks.ron on disk wins (so it can be modded), otherwise the built-in copy.
        let blocks_text = std::fs::read_to_string(base.join("packs/vanilla/blocks.ron")).unwrap_or_else(|_| embedded::BLOCKS_RON.to_string());
        let reg = Arc::new(BlockRegistry::from_ron(&blocks_text).unwrap_or_else(|e| {
            log::error!("{e}; using built-in blocks");
            BlockRegistry::from_ron(embedded::BLOCKS_RON).unwrap()
        }));

        let world_name = args.world.clone().unwrap_or_else(|| "Mundo 1".into());
        let save = Arc::new(WorldSave::new(level_path_hint(&base, &world_name)));
        let level = save.load_level();
        let seed = level.as_ref().map(|l| l.seed).or(args.seed).unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
        });
        let gen = Arc::new(Generator::new(seed, &reg));
        let tints: Vec<[f32; 3]> = (0..Biome::COUNT).map(|b| Biome::from_u8(b as u8).tint()).collect();
        let tables = Arc::new(MeshTables::new(&reg, tints));
        let world = World::new(reg.clone(), gen.clone(), save.clone(), tables);

        let tex = textures::load(&reg.textures, &base, &settings.resource_packs);
        let textures_name = tex.packs.first().cloned().unwrap_or_else(|| "Padrão".into());
        let font = FontAtlas::new();
        let renderer = pollster::block_on(Renderer::new(window.clone(), settings.vsync, &tex, &font));
        let ui = UiBatch::new(&font);
        let audio = Audio::new(&settings, &base);

        let (sx, sy, sz) = gen.find_spawn();
        let mut body = Body::new(Vec3::new(sx as f32 + 0.5, sy as f32, sz as f32 + 0.5));
        let (mut yaw, mut pitch) = (0.0, 0.0);
        let mut hotbar_names: Vec<String> =
            ["grass_block", "dirt", "stone", "cobblestone", "oak_planks", "oak_log", "glass", "torch", "glowstone"].iter().map(|s| s.to_string()).collect();
        let mut time_of_day = 1000.0;
        let mut needs_spawn = true;
        if let Some(l) = &level {
            body.pos = Vec3::from(l.player_pos);
            body.flying = l.flying;
            yaw = l.player_yaw;
            pitch = l.player_pitch;
            time_of_day = l.time_of_day;
            if l.hotbar.len() == 9 {
                hotbar_names = l.hotbar.clone();
            }
            needs_spawn = false;
        }
        if let Some(p) = args.pos {
            body.pos = p;
            needs_spawn = false;
        }
        if let Some((y, p)) = args.look {
            yaw = y.to_radians();
            pitch = p.to_radians();
        }
        if let Some(t) = args.time {
            time_of_day = t;
        }
        if args.fly {
            body.flying = true;
        }
        let mut hotbar = [AIR; 9];
        for (i, n) in hotbar_names.iter().enumerate() {
            hotbar[i] = reg.id_or_air(n);
        }
        let inventory: Vec<BlockId> = (1..reg.len() as BlockId).filter(|&i| reg.get(i).in_inventory).collect();
        let packs = textures::list_resource_packs(&base.join("resourcepacks"));
        let show_debug = args.debug;

        log::info!("world '{world_name}' seed {seed}, {} blocks, {} textures ({}px)", reg.len(), reg.textures.len(), renderer.texture_size);
        let now = Instant::now();
        Game {
            window,
            renderer,
            world,
            reg,
            settings,
            base,
            audio,
            args,
            world_name,
            seed,
            body,
            yaw,
            pitch,
            spawn_xz: (sx, sz),
            needs_spawn,
            hotbar,
            selected: 0,
            inventory,
            inv_scroll: 0,
            time_of_day,
            screen: Screen::Loading,
            keys: HashSet::new(),
            mouse_held: [false; 3],
            mouse_pos: (0.0, 0.0),
            clicked: false,
            last_space: -10.0,
            break_cd: 0.0,
            place_cd: 0.0,
            target: None,
            step_dist: 0.0,
            ui,
            show_debug,
            hide_hud: false,
            toast: None,
            packs,
            textures_name,
            start: now,
            last: now,
            now: 0.0,
            frame_times: Vec::new(),
            fps: 0.0,
            last_frame_info: (0, 0),
            autosave: 0.0,
            loaded_frames: 0,
            screenshot_request: false,
            grabbed: false,
        }
    }

    fn forward(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), -self.yaw.cos() * self.pitch.cos())
    }

    fn set_grab(&mut self, grab: bool) {
        if grab {
            let ok = self.window.set_cursor_grab(CursorGrabMode::Locked).or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));
            if ok.is_err() {
                log::warn!("could not capture the mouse");
            }
            self.window.set_cursor_visible(false);
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
            self.window.set_cursor_visible(true);
        }
        self.grabbed = grab;
    }

    fn set_screen(&mut self, s: Screen) {
        self.screen = s;
        self.set_grab(s == Screen::Playing);
        self.mouse_held = [false; 3];
    }

    fn toast(&mut self, text: impl Into<String>, secs: f32) {
        self.toast = Some(Toast { text: text.into(), until: self.now + secs });
    }

    fn save_all(&mut self) {
        let level = LevelData {
            name: self.world_name.clone(),
            seed: self.seed,
            player_pos: self.body.pos.to_array(),
            player_yaw: self.yaw,
            player_pitch: self.pitch,
            flying: self.body.flying,
            time_of_day: self.time_of_day,
            hotbar: self.hotbar.iter().map(|&id| self.reg.get(id).name.clone()).collect(),
        };
        if self.screen != Screen::Loading || !self.needs_spawn {
            if let Err(e) = self.world.save.save_level(&level) {
                log::error!("could not save the world: {e}");
            }
        }
        self.world.save_all();
        self.settings.save(&self.base.join("settings.ron"));
    }

    fn situation(&self) -> Situation {
        match self.screen {
            Screen::Loading => Situation::Title,
            _ if self.body.head_in_water => Situation::Underwater,
            _ => Situation::Creative,
        }
    }

    fn reload_textures(&mut self) {
        let tex = textures::load(&self.reg.textures, &self.base, &self.settings.resource_packs);
        self.textures_name = tex.packs.first().cloned().unwrap_or_else(|| "Padrão".into());
        self.renderer.reload_textures(&tex);
        self.toast(format!("Texturas: {}", self.textures_name), 3.0);
    }

    fn cycle_pack(&mut self) {
        self.packs = textures::list_resource_packs(&self.base.join("resourcepacks"));
        let cur = self.settings.resource_packs.first().cloned();
        let idx = cur.as_ref().and_then(|c| self.packs.iter().position(|p| p == c));
        let next = match idx {
            None => self.packs.first().cloned(),
            Some(i) => self.packs.get(i + 1).cloned(),
        };
        self.settings.resource_packs = next.into_iter().collect();
        self.reload_textures();
    }

    fn key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        if pressed {
            self.keys.insert(code);
        } else {
            self.keys.remove(&code);
            return;
        }
        if repeat {
            return;
        }
        let f3 = self.keys.contains(&KeyCode::F3);
        match code {
            KeyCode::Escape => match self.screen {
                Screen::Playing => self.set_screen(Screen::Paused),
                Screen::Paused | Screen::Inventory => self.set_screen(Screen::Playing),
                Screen::Loading => {}
            },
            KeyCode::KeyE => match self.screen {
                Screen::Playing => self.set_screen(Screen::Inventory),
                Screen::Inventory => self.set_screen(Screen::Playing),
                _ => {}
            },
            KeyCode::KeyT if f3 => self.reload_textures(),
            KeyCode::F3 => self.show_debug = !self.show_debug,
            KeyCode::F1 => self.hide_hud = !self.hide_hud,
            KeyCode::F2 => self.screenshot_request = true,
            KeyCode::F11 => {
                let fs = self.window.fullscreen().is_some();
                self.window.set_fullscreen(if fs { None } else { Some(Fullscreen::Borderless(None)) });
            }
            KeyCode::KeyM => {
                let sit = self.situation();
                if let Some(a) = self.audio.as_mut() {
                    a.skip(sit, &self.settings);
                }
            }
            KeyCode::Space if self.screen == Screen::Playing => {
                if self.now - self.last_space < 0.3 {
                    self.body.flying = !self.body.flying;
                    self.body.vel.y = 0.0;
                    self.last_space = -10.0;
                } else {
                    self.last_space = self.now;
                }
            }
            _ => {}
        }
        let digits = [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
            KeyCode::Digit8,
            KeyCode::Digit9,
        ];
        if let Some(i) = digits.iter().position(|&d| d == code) {
            if self.screen == Screen::Inventory {
                if let Some(id) = self.hovered_inventory_block() {
                    self.hotbar[i] = id;
                }
            } else {
                self.selected = i;
                self.show_held_name();
            }
        }
    }

    fn show_held_name(&mut self) {
        let id = self.hotbar[self.selected];
        if id != AIR {
            let n = self.reg.get(id).display_name.clone();
            self.toast(n, 1.5);
        }
    }

    fn scroll(&mut self, lines: f32) {
        if lines == 0.0 {
            return;
        }
        match self.screen {
            Screen::Playing => {
                let d = if lines > 0.0 { 8 } else { 1 };
                self.selected = (self.selected + d) % 9;
                self.show_held_name();
            }
            Screen::Inventory => {
                let rows = self.inventory.len().div_ceil(9);
                let visible = self.inventory_layout().3;
                if lines > 0.0 {
                    self.inv_scroll = self.inv_scroll.saturating_sub(1);
                } else if self.inv_scroll + visible < rows {
                    self.inv_scroll += 1;
                }
            }
            _ => {}
        }
    }

    // ---------- inventory layout (shared by drawing and clicking) ----------
    fn inventory_layout(&self) -> (f32, f32, f32, usize) {
        let s = self.ui.scale;
        let cell = 18.0 * s;
        let w = cell * 9.0 + 16.0 * s;
        let rows = (((self.ui.height - 80.0 * s) / cell).floor() as usize).clamp(3, 8);
        let x = (self.ui.width - w) / 2.0;
        let y = ((self.ui.height - (cell * rows as f32 + 60.0 * s)) / 2.0).max(4.0 * s);
        (x, y, cell, rows)
    }

    fn hovered_inventory_block(&self) -> Option<BlockId> {
        let (x, y, cell, rows) = self.inventory_layout();
        let s = self.ui.scale;
        let gx = x + 8.0 * s;
        let gy = y + 20.0 * s;
        let (mx, my) = self.mouse_pos;
        if mx < gx || my < gy {
            return None;
        }
        let col = ((mx - gx) / cell) as usize;
        let row = ((my - gy) / cell) as usize;
        if col >= 9 || row >= rows {
            return None;
        }
        self.inventory.get((row + self.inv_scroll) * 9 + col).copied()
    }

    fn update(&mut self, dt: f32) {
        self.now += dt;
        let rd = self.settings.render_distance;
        let eye = self.body.eye();
        let center = ChunkPos::new((eye.x.floor() as i32) >> 4, (eye.z.floor() as i32) >> 4);
        self.world.update(center, rd);
        let events = std::mem::take(&mut self.world.events);
        self.renderer.apply_mesh_events(events);

        if self.screen == Screen::Loading {
            let (sx, sz) = if self.needs_spawn { self.spawn_xz } else { (self.body.pos.x.floor() as i32, self.body.pos.z.floor() as i32) };
            let c = ChunkPos::new(sx >> 4, sz >> 4);
            let near = rd.min(3);
            let mut ready = true;
            for dz in -near..=near {
                for dx in -near..=near {
                    if !self.world.is_meshed(c + ChunkPos::new(dx, dz)) {
                        ready = false;
                    }
                }
            }
            if ready {
                if self.needs_spawn {
                    let y = self.world.surface_y(sx, sz).unwrap_or(100) + 1;
                    self.body.pos = Vec3::new(sx as f32 + 0.5, y as f32, sz as f32 + 0.5);
                    self.needs_spawn = false;
                }
                if self.args.demo {
                    self.build_demo();
                }
                if self.args.show_inventory {
                    self.set_screen(Screen::Inventory);
                } else if self.args.show_pause {
                    self.set_screen(Screen::Paused);
                } else {
                    self.set_screen(Screen::Playing);
                }
                self.toast("Bem-vindo! Esc = menu, E = blocos", 6.0);
            }
        }

        self.time_of_day = (self.time_of_day + dt * 20.0) % 24000.0;
        if self.args.time.is_some() && self.args.screenshot.is_some() {
            self.time_of_day = self.args.time.unwrap();
        }

        if self.screen == Screen::Playing || self.screen == Screen::Inventory || self.screen == Screen::Paused {
            self.update_player(dt);
        }

        let sit = self.situation();
        if let Some(a) = self.audio.as_mut() {
            let prev = a.now_playing.clone();
            a.update(dt, sit, &self.settings);
            if a.now_playing.is_some() && a.now_playing != prev {
                let t = format!("Tocando: {}", a.now_playing.clone().unwrap());
                self.toast(t, 5.0);
            }
        }

        self.autosave += dt;
        if self.autosave > 60.0 && self.screen != Screen::Loading {
            self.autosave = 0.0;
            self.save_all();
        }
    }

    fn update_player(&mut self, dt: f32) {
        let playing = self.screen == Screen::Playing;
        let k = |c: KeyCode| playing && self.keys.contains(&c);
        let fwd = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let mut wish = Vec3::ZERO;
        if k(KeyCode::KeyW) {
            wish += fwd;
        }
        if k(KeyCode::KeyS) {
            wish -= fwd;
        }
        if k(KeyCode::KeyD) {
            wish += right;
        }
        if k(KeyCode::KeyA) {
            wish -= right;
        }
        let input = Input {
            wish,
            jump: k(KeyCode::Space),
            sneak: k(KeyCode::ShiftLeft) || k(KeyCode::ShiftRight),
            sprint: k(KeyCode::ControlLeft) || k(KeyCode::ControlRight),
        };
        let before = self.body.pos;
        let was_ground = self.body.on_ground;
        // The camera pass for screenshots holds still.
        if self.args.screenshot.is_none() {
            self.body.step(&self.world, &input, dt);
        }
        let moved = (self.body.pos - before) * Vec3::new(1.0, 0.0, 1.0);
        if self.body.on_ground && was_ground && !self.body.flying {
            self.step_dist += moved.length();
            if self.step_dist > 1.7 {
                self.step_dist = 0.0;
                let below = self.world.block_or_air((self.body.pos - Vec3::new(0.0, 0.2, 0.0)).floor().as_ivec3());
                if below != AIR {
                    let m = Material::of(&self.reg.get(below).name);
                    let vol = self.settings.sound_volume;
                    if let Some(a) = self.audio.as_mut() {
                        a.play(Sfx::Step(m), vol);
                    }
                }
            }
        }

        // What the player is looking at.
        let world = &self.world;
        self.target = raycast(self.body.eye(), self.forward(), REACH, |p| match world.block(p) {
            Some(id) => id != AIR && !world.is_liquid(id),
            None => false,
        });

        self.break_cd -= dt;
        self.place_cd -= dt;
        if !playing {
            return;
        }
        if self.mouse_held[0] && self.break_cd <= 0.0 {
            if let Some(hit) = self.target {
                let id = self.world.block_or_air(hit.block);
                if id != AIR && self.reg.get(id).hardness >= 0.0 && self.world.break_block(hit.block) {
                    self.break_cd = 0.22;
                    let m = Material::of(&self.reg.get(id).name);
                    let vol = self.settings.sound_volume;
                    if let Some(a) = self.audio.as_mut() {
                        a.play(Sfx::Break(m), vol);
                    }
                }
            }
        }
        if self.mouse_held[1] && self.place_cd <= 0.0 {
            self.try_place();
        }
        if self.mouse_held[2] {
            self.mouse_held[2] = false;
            if let Some(hit) = self.target {
                let id = self.world.block_or_air(hit.block);
                if id != AIR {
                    if let Some(i) = self.hotbar.iter().position(|&h| h == id) {
                        self.selected = i;
                    } else {
                        self.hotbar[self.selected] = id;
                    }
                    self.show_held_name();
                }
            }
        }
    }

    fn try_place(&mut self) {
        let Some(hit) = self.target else { return };
        let id = self.hotbar[self.selected];
        if id == AIR {
            return;
        }
        let clicked = self.world.block_or_air(hit.block);
        // Plants are replaced instead of placed against.
        let pos = if self.reg.get(clicked).shape == Shape::Cross && clicked != id { hit.block } else { hit.block + hit.normal };
        let cur = self.world.block_or_air(pos);
        if cur != AIR && !self.world.is_liquid(cur) && self.reg.get(cur).shape != Shape::Cross {
            return;
        }
        let b = self.reg.get(id);
        if b.solid && b.shape == Shape::Cube && self.body.intersects_block(pos) {
            return;
        }
        // Plants and torches need something under them.
        if b.shape == Shape::Cross {
            let under = self.world.block_or_air(pos - IVec3::Y);
            if under == AIR || !self.reg.get(under).solid {
                return;
            }
        }
        if self.world.place_block(pos, id) {
            self.place_cd = 0.22;
            let m = Material::of(&b.name);
            let vol = self.settings.sound_volume;
            if let Some(a) = self.audio.as_mut() {
                a.play(Sfx::Place(m), vol);
            }
        }
    }

    /// A little scene for test screenshots: a house with glass, torches and a glowstone lamp.
    fn build_demo(&mut self) {
        let p = self.body.pos.floor().as_ivec3();
        let f = self.forward();
        let dir = if f.x.abs() > f.z.abs() { IVec3::new(f.x.signum() as i32, 0, 0) } else { IVec3::new(0, 0, f.z.signum() as i32) };
        let side = IVec3::new(-dir.z, 0, dir.x);
        let g = |n: &str| self.reg.id_or_air(n);
        let (planks, log, glass, cobble, torch, glow) = (g("oak_planks"), g("oak_log"), g("glass"), g("cobblestone"), g("torch"), g("glowstone"));
        let origin = p + dir * 6;
        let ground = self.world.surface_y(origin.x, origin.z).unwrap_or(p.y);
        let base = IVec3::new(origin.x, ground + 1, origin.z);
        for a in -3..=3 {
            for b in 0..=6 {
                for h in 0..5 {
                    let q = base + side * a + dir * b + IVec3::Y * h;
                    let edge_a = a.abs() == 3;
                    let edge_b = b == 0 || b == 6;
                    let id = if h == 4 {
                        planks
                    } else if edge_a && edge_b {
                        log
                    } else if edge_a || edge_b {
                        if h == 0 {
                            cobble
                        } else if h == 2 && (a.abs() == 1 || b == 3) {
                            glass
                        } else if b == 0 && a == 0 && h < 3 {
                            AIR
                        } else {
                            planks
                        }
                    } else if h == 0 {
                        planks
                    } else {
                        AIR
                    };
                    self.world.set_block(q, id);
                }
            }
        }
        self.world.set_block(base + dir * 3 + IVec3::Y * 3, glow);
        self.world.set_block(base - dir + side * 2 + IVec3::Y, torch);
        self.world.set_block(base - dir - side * 2 + IVec3::Y, torch);
        for (i, n) in ["red_wool", "orange_wool", "yellow_wool", "lime_wool", "light_blue_wool", "blue_wool", "purple_wool"].iter().enumerate() {
            let q = base + side * (i as i32 - 3) - dir * 3;
            let y = self.world.surface_y(q.x, q.z).unwrap_or(q.y - 1) + 1;
            self.world.set_block(IVec3::new(q.x, y, q.z), g(n));
        }
    }

    fn sky(&self) -> ([f32; 3], [f32; 3], f32, Vec3) {
        let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
        let sun = Vec3::new(a.cos(), a.sin(), 0.0);
        let daylight = (a.sin() * 2.2 + 0.45).clamp(0.0, 1.0);
        let lerp = |n: [f32; 3], d: [f32; 3]| [n[0] + (d[0] - n[0]) * daylight, n[1] + (d[1] - n[1]) * daylight, n[2] + (d[2] - n[2]) * daylight];
        let top = lerp([0.008, 0.01, 0.035], [0.47, 0.65, 1.0]);
        let horizon = lerp([0.02, 0.025, 0.06], [0.74, 0.84, 1.0]);
        (top, horizon, daylight.max(0.16), sun)
    }

    fn draw_ui(&mut self, mut info_lines: Vec<String>) {
        let (w, h) = (self.renderer.config.width as f32, self.renderer.config.height as f32);
        self.ui.begin(w, h, self.settings.gui_scale);
        let s = self.ui.scale;
        let mouse = self.mouse_pos;
        let mut clicked_button: Option<usize> = None;

        if self.screen == Screen::Loading {
            self.ui.rect(0.0, 0.0, w, h, [44, 34, 26, 255]);
            let pct = (self.world.meshed_count as f32 / 49.0).min(1.0);
            self.ui.text_centered(w / 2.0, h / 2.0 - 30.0 * s, "Gerando o mundo...", WHITE);
            let bw = 100.0 * s;
            self.ui.rect(w / 2.0 - bw / 2.0, h / 2.0, bw, 2.0 * s, [128, 128, 128, 255]);
            self.ui.rect(w / 2.0 - bw / 2.0, h / 2.0, bw * pct, 2.0 * s, [128, 255, 128, 255]);
            return;
        }

        if !self.hide_hud && self.screen != Screen::Paused {
            // Crosshair.
            if self.screen == Screen::Playing {
                let (cx, cy) = (w / 2.0, h / 2.0);
                let c = [235, 235, 235, 220];
                self.ui.rect(cx - 5.0 * s, cy - s / 2.0, 10.0 * s, s, c);
                self.ui.rect(cx - s / 2.0, cy - 5.0 * s, s, 10.0 * s, c);
            }
            // Hotbar.
            let cell = 20.0 * s;
            let hx = (w - cell * 9.0) / 2.0;
            let hy = h - cell - 2.0 * s;
            self.ui.rect(hx - s, hy - s, cell * 9.0 + 2.0 * s, cell + 2.0 * s, [0, 0, 0, 150]);
            for i in 0..9 {
                let x = hx + i as f32 * cell;
                self.ui.frame(x, hy, cell, cell, s, [110, 110, 110, 255]);
                let id = self.hotbar[i];
                if id != AIR {
                    self.ui.block_icon(&self.reg, id, x + 3.0 * s, hy + 2.5 * s, 14.0 * s);
                }
            }
            let sx = hx + self.selected as f32 * cell;
            self.ui.frame(sx - s, hy - s, cell + 2.0 * s, cell + 2.0 * s, 2.0 * s, [255, 255, 255, 255]);
            if self.body.flying {
                self.ui.text(4.0 * s, h - 12.0 * s, "Voando", GRAY);
            }
            if let Some(t) = &self.toast {
                if t.until > self.now {
                    let text = t.text.clone();
                    self.ui.text_centered(w / 2.0, hy - 16.0 * s, &text, WHITE);
                }
            }
            if self.show_debug {
                info_lines.insert(0, format!("Mine Clone {} (F3)", env!("CARGO_PKG_VERSION")));
                for (i, l) in info_lines.iter().enumerate() {
                    let y = 2.0 * s + i as f32 * 10.0 * s;
                    let tw = self.ui.text_width(l, s);
                    self.ui.rect(s, y - s, tw + 3.0 * s, 10.0 * s, [0, 0, 0, 110]);
                    self.ui.text(2.0 * s, y, l, [224, 224, 224, 255]);
                }
            }
        }

        match self.screen {
            Screen::Paused => {
                self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 140]);
                let bw = 200.0 * s;
                let bh = 20.0 * s;
                let x = (w - bw) / 2.0;
                let mut y = (h / 2.0 - 120.0 * s).max(26.0 * s);
                self.ui.text_centered(w / 2.0, y - 18.0 * s, "Menu do jogo", WHITE);
                let pack = self.textures_name.clone();
                let custom = self.audio.as_ref().map(|a| a.custom_track_count()).unwrap_or(0);
                let labels = [
                    "Voltar ao jogo".to_string(),
                    format!("Distância de visão: {} chunks", self.settings.render_distance),
                    format!("Música: {} ({} suas)", self.settings.music_mode.label(), custom),
                    "Tocar próxima música (M)".to_string(),
                    format!("Pacote de texturas: {pack}"),
                    format!("Nuvens: {}", if self.settings.show_clouds { "Sim" } else { "Não" }),
                    format!("VSync: {}", if self.settings.vsync { "Sim" } else { "Não" }),
                    "Salvar e sair".to_string(),
                ];
                for (i, l) in labels.iter().enumerate() {
                    if self.ui.button(x, y, bw, bh, l, mouse) && self.clicked {
                        clicked_button = Some(i);
                    }
                    y += bh + 4.0 * s;
                }
                let help = [
                    "WASD andar  Espaço pular (2x = voar)  Shift descer  Ctrl correr",
                    "Esq. quebrar  Dir. colocar  Meio copiar bloco  E blocos  1-9/roda escolher",
                    "F1 esconder HUD  F2 foto  F3 info  F3+T recarregar texturas  F11 tela cheia",
                ];
                y += 4.0 * s;
                let hs = (s / 2.0).floor().max(1.0);
                let folder = self.base.join(&self.settings.music_folder).display().to_string();
                let packs = self.base.join("resourcepacks").display().to_string();
                let extra = [format!("Suas músicas: {folder}"), format!("Pacotes de textura: {packs}")];
                for l in help.iter().map(|s| s.to_string()).chain(extra) {
                    let tw = self.ui.text_width(&l, hs);
                    self.ui.text_scaled(w / 2.0 - tw / 2.0, y, &l, GRAY, hs);
                    y += 10.0 * hs;
                }
            }
            Screen::Inventory => {
                self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 120]);
                let (x, y, cell, vis_rows) = self.inventory_layout();
                let pw = cell * 9.0 + 16.0 * s;
                let ph = cell * vis_rows as f32 + 30.0 * s;
                self.ui.rect(x, y, pw, ph, [198, 198, 198, 255]);
                self.ui.frame(x, y, pw, ph, s, [255, 255, 255, 255]);
                self.ui.label(x + 8.0 * s, y + 6.0 * s, "Blocos (modo criativo)", [64, 64, 64, 255], s);
                let gx = x + 8.0 * s;
                let gy = y + 20.0 * s;
                let hovered = self.hovered_inventory_block();
                for row in 0..vis_rows {
                    for col in 0..9 {
                        let cx = gx + col as f32 * cell;
                        let cy = gy + row as f32 * cell;
                        self.ui.rect(cx, cy, cell - s, cell - s, [139, 139, 139, 255]);
                        if let Some(&id) = self.inventory.get((row + self.inv_scroll) * 9 + col) {
                            if hovered == Some(id) {
                                self.ui.rect(cx, cy, cell - s, cell - s, [200, 200, 200, 255]);
                            }
                            self.ui.block_icon(&self.reg, id, cx + 1.5 * s, cy + 1.0 * s, 14.0 * s);
                        }
                    }
                }
                let rows = self.inventory.len().div_ceil(9);
                if rows > vis_rows {
                    let t = format!("rolar {}/{}", self.inv_scroll + 1, rows - vis_rows + 1);
                    let tw = self.ui.text_width(&t, s);
                    self.ui.label(x + pw - tw - 8.0 * s, y + 6.0 * s, &t, [64, 64, 64, 255], s);
                }
                if let Some(id) = hovered {
                    let name = self.reg.get(id).display_name.clone();
                    let tw = self.ui.text_width(&name, s);
                    let (mx, my) = mouse;
                    self.ui.rect(mx + 8.0 * s, my - 14.0 * s, tw + 6.0 * s, 12.0 * s, [16, 0, 16, 235]);
                    self.ui.text(mx + 11.0 * s, my - 12.0 * s, &name, WHITE);
                    if self.clicked {
                        self.hotbar[self.selected] = id;
                    }
                }
                self.ui.text_centered(w / 2.0, y + ph + 4.0 * s, "Clique para pôr na barra, ou aponte e aperte 1-9", WHITE);
            }
            _ => {}
        }
        self.clicked = false;

        if let Some(i) = clicked_button {
            self.pause_button(i);
        }
        let _ = YELLOW;
    }

    fn pause_button(&mut self, i: usize) {
        if let Some(a) = self.audio.as_mut() {
            a.play(Sfx::Click, self.settings.sound_volume);
        }
        match i {
            0 => self.set_screen(Screen::Playing),
            1 => {
                let cur = RENDER_DISTANCES.iter().position(|&d| d >= self.settings.render_distance).unwrap_or(0);
                self.settings.render_distance = RENDER_DISTANCES[(cur + 1) % RENDER_DISTANCES.len()];
            }
            2 => {
                self.settings.music_mode = self.settings.music_mode.next();
                if let Some(a) = self.audio.as_mut() {
                    a.reload_tracks();
                }
            }
            3 => {
                let sit = self.situation();
                if let Some(a) = self.audio.as_mut() {
                    a.skip(sit, &self.settings);
                }
            }
            4 => self.cycle_pack(),
            5 => self.settings.show_clouds = !self.settings.show_clouds,
            6 => {
                self.settings.vsync = !self.settings.vsync;
                self.renderer.set_vsync(self.settings.vsync);
            }
            _ => {
                self.save_all();
                std::process::exit(0);
            }
        }
        self.settings.save(&self.base.join("settings.ron"));
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.frame_times.push(dt);
        if self.frame_times.len() >= 30 {
            self.fps = self.frame_times.len() as f32 / self.frame_times.iter().sum::<f32>();
            self.frame_times.clear();
        }
        self.update(dt);

        let eye = self.body.eye();
        let block = eye.floor().as_ivec3();
        let (top, horizon, daylight, sun) = self.sky();
        let underwater = self.body.head_in_water;
        let rd = self.settings.render_distance as f32 * 16.0;
        let (fog_color, fog_start, fog_end) =
            if underwater { ([0.04, 0.12, 0.35], 0.0, 28.0) } else { (horizon, rd * 0.55, rd - 2.0) };
        let mut g = Globals {
            cam_pos: [eye.x - block.x as f32, eye.y - block.y as f32, eye.z - block.z as f32, (self.now % 3600.0) + self.start.elapsed().as_secs_f32() * 0.0],
            cam_world: [eye.x, eye.y, eye.z, CLOUD_HEIGHT],
            fog_color: [fog_color[0], fog_color[1], fog_color[2], daylight],
            fog: [fog_start, fog_end, if underwater { 1.0 } else { 0.0 }, 0.3],
            sun_dir: [sun.x, sun.y, sun.z, 0.0],
            sky_top: [top[0], top[1], top[2], 1.0],
            sky_horizon: [horizon[0], horizon[1], horizon[2], 1.0],
            ..Default::default()
        };
        let cam = Camera {
            block,
            frac: Vec3::from_slice(&g.cam_pos[..3]),
            forward: self.forward(),
            fov_y: self.settings.fov.to_radians(),
            far: rd + 32.0,
        };

        let mut lines = Vec::new();
        if self.show_debug {
            let p = self.body.pos;
            let biome = self.world.gen.biome_at(p.x.floor() as i32, p.z.floor() as i32);
            let light = self.world.light(eye.floor().as_ivec3());
            let f = self.forward();
            let facing = if f.x.abs() > f.z.abs() { if f.x > 0.0 { "leste (+X)" } else { "oeste (-X)" } } else if f.z > 0.0 { "sul (+Z)" } else { "norte (-Z)" };
            lines.push(format!("{:.0} fps", self.fps));
            lines.push(format!("XYZ: {:.2} / {:.2} / {:.2}", p.x, p.y, p.z));
            lines.push(format!("Chunk: {} {}  Olhando: {}", (p.x.floor() as i32) >> 4, (p.z.floor() as i32) >> 4, facing));
            lines.push(format!("Bioma: {}  Luz: céu {} bloco {}", biome.name(), light >> 4, light & 15));
            lines.push(format!(
                "Chunks: {} carregados, {} desenhados, {} tarefas ({} threads)",
                self.world.loaded_chunks(),
                self.renderer.chunk_count(),
                self.world.pending_jobs(),
                self.world.workers
            ));
            lines.push(format!("Seções: {}  Quads: {}  VRAM: {:.1} MB", self.last_frame_info.0, self.last_frame_info.1, self.renderer.vram_bytes as f64 / 1e6));
            lines.push(format!("Hora: {:.0}  Seed: {}", self.time_of_day, self.seed));
            lines.push(format!("GPU: {} ({:?})", self.renderer.adapter_info.name, self.renderer.adapter_info.backend));
            lines.push(format!("Texturas: {} ({}px)", self.textures_name, self.renderer.texture_size));
            if let Some(t) = self.target {
                let id = self.world.block_or_air(t.block);
                lines.push(format!("Mirando: {} em {} {} {}", self.reg.get(id).name, t.block.x, t.block.y, t.block.z));
            }
            if let Some(a) = &self.audio {
                match &a.now_playing {
                    Some(n) => lines.push(format!("Música: {n}")),
                    None => lines.push(format!("Música: próxima em {:.0}s ({})", a.next_in.max(0.0), self.settings.music_mode.label())),
                }
            }
        }
        self.draw_ui(lines);

        // Automated screenshots: wait until the view is fully built.
        let mut capture = self.screenshot_request;
        let mut exit_after = false;
        if self.args.screenshot.is_some() && self.screen != Screen::Loading {
            self.loaded_frames += 1;
            if (self.loaded_frames > 30 && self.world.pending_jobs() == 0) || self.loaded_frames > 1500 {
                capture = true;
                exit_after = true;
            }
        }
        let selection = if self.screen == Screen::Playing || self.args.screenshot.is_some() { self.target.map(|t| t.block) } else { None };
        let (info, shot) = self.renderer.render(&cam, &mut g, selection, self.settings.show_clouds, &self.ui, capture);
        self.last_frame_info = (info.sections_drawn, info.quads_drawn);
        if let Some((w, h, px)) = shot {
            let path = if exit_after {
                self.args.screenshot.clone().unwrap()
            } else {
                let dir = self.base.join("screenshots");
                let _ = std::fs::create_dir_all(&dir);
                let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                dir.join(format!("foto-{stamp}.png"))
            };
            match image::save_buffer(&path, &px, w, h, image::ExtendedColorType::Rgba8) {
                Ok(_) => {
                    log::info!("screenshot saved to {}", path.display());
                    if !exit_after {
                        self.toast(format!("Foto salva: {}", path.display()), 3.0);
                    }
                }
                Err(e) => log::error!("screenshot failed: {e}"),
            }
        }
        self.screenshot_request = false;
        if exit_after {
            println!("frame stats: {:.0} fps, {} sections, {} quads", self.fps, info.sections_drawn, info.quads_drawn);
            std::process::exit(0);
        }
    }
}

struct App {
    args: Args,
    game: Option<Game>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.game.is_some() {
            return;
        }
        let (w, h) = self.args.size.unwrap_or((1280, 720));
        let attrs = Window::default_attributes().with_title("Mine Clone").with_inner_size(winit::dpi::PhysicalSize::new(w, h));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        self.game = Some(Game::new(window, self.args.clone()));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(game) = self.game.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => {
                game.save_all();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => game.renderer.resize(size.width, size.height),
            WindowEvent::Focused(false) => {
                game.keys.clear();
                if game.screen == Screen::Playing && game.args.screenshot.is_none() {
                    game.set_screen(Screen::Paused);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    game.key(code, event.state == ElementState::Pressed, event.repeat);
                }
            }
            WindowEvent::CursorMoved { position, .. } => game.mouse_pos = (position.x as f32, position.y as f32),
            WindowEvent::MouseInput { state, button, .. } => {
                let idx = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                    _ => return,
                };
                let pressed = state == ElementState::Pressed;
                if game.screen == Screen::Playing {
                    if !game.grabbed {
                        game.set_grab(true);
                    }
                    game.mouse_held[idx] = pressed;
                    if !pressed {
                        if idx == 0 {
                            game.break_cd = 0.0;
                        } else if idx == 1 {
                            game.place_cd = 0.0;
                        }
                    }
                } else if pressed && idx == 0 {
                    game.clicked = true;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y / 40.0) as f32,
                };
                game.scroll(lines);
            }
            WindowEvent::RedrawRequested => game.frame(),
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        let Some(game) = self.game.as_mut() else { return };
        if let DeviceEvent::MouseMotion { delta } = event {
            if game.screen == Screen::Playing && game.grabbed {
                let sens = game.settings.mouse_sensitivity.to_radians();
                let inv = if game.settings.invert_mouse { -1.0 } else { 1.0 };
                game.yaw -= delta.0 as f32 * sens;
                game.pitch = (game.pitch - delta.1 as f32 * sens * inv).clamp(-1.55, 1.55);
            }
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(g) = &self.game {
            g.window.request_redraw();
        }
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn")).init();
    let args = parse_args();
    // Write crashes to a file next to the game, since release builds on Windows have no console.
    let crash_file = base_dir().join("crash.txt");
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        let _ = std::fs::write(&crash_file, format!("Mine Clone {} crashed:\n{info}\n\n{bt}\n", env!("CARGO_PKG_VERSION")));
        default_hook(info);
    }));
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App { args, game: None };
    event_loop.run_app(&mut app).expect("run");
}
