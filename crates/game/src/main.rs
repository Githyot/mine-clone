//! Mine Clone: a fast block game written in Rust.

// No console window behind the game on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod controller;
mod entity;
mod mobs;
mod paintings;
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
use controller::{Action, Btn, Controllers, TouchpadMode};
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
    /// Spawn a few mobs in front of the player (test screenshots).
    mobs: bool,
    survival: bool,
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
            "--mobs" => a.mobs = true,
            "--survival" => a.survival = true,
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
    let pf = base.join("paintings");
    if !pf.exists() && std::fs::create_dir_all(&pf).is_ok() {
        let _ = std::fs::write(
            pf.join("LEIA-ME.txt"),
            "Coloque aqui suas próprias pinturas (arquivos .png).\n\
             O tamanho na parede (1 a 4 blocos de largura e altura) vem do formato da imagem;\n\
             cada bloco usa 16x16 pixels, então 32x16 vira uma pintura de 2x1 blocos.\n\
             No jogo, mire numa parede e aperte P para escolher a pintura.\n\n\
             Put your own paintings here (.png). Aim at a wall and press P to hang one.\n",
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Loading,
    Playing,
    Paused,
    Inventory,
    /// Controller settings and button mapping.
    Controls,
    /// Survival: the player died.
    Dead,
    /// Choosing a painting for the wall the player is looking at.
    Paintings,
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

    pads: Controllers,
    /// Action waiting for a button press in the controller menu.
    rebinding: Option<Action>,
    /// Actions held this frame through the controller.
    pad_held: HashSet<Action>,
    pad_back: bool,
    pad_scroll_cd: f32,

    mobs: mobs::Mobs,
    entities: entity::EntityRenderer,
    /// Survival mode: health, damage, no flying.
    survival: bool,
    health: f32,
    /// Seconds since the player was last hurt (red flash, invulnerability).
    hurt_age: f32,
    regen: f32,
    /// Highest point of the current fall.
    fall_peak: f32,
    /// Seconds of breath left under water.
    air: f32,
    attack_cd: f32,
    /// Name and health of the mob the player hit last (shown above the hotbar).
    last_hit: Option<(String, f32, f32, f32)>,

    painting_defs: Vec<paintings::PaintingDef>,
    paintings: Vec<paintings::Painting>,
    /// Air cell and wall direction chosen for the painting picker.
    painting_spot: Option<(IVec3, IVec3)>,
    /// Painting shown on the wall while its button is hovered: (def, anchor).
    painting_preview: Option<(usize, IVec3)>,
    painting_scroll: usize,
    painting_check: f32,

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

        // Mobs: a mobs.ron on disk wins, like blocks.
        let mobs_text = std::fs::read_to_string(base.join("packs/vanilla/mobs.ron")).unwrap_or_else(|_| embedded::MOBS_RON.to_string());
        let mob_reg = voxel::mob::MobRegistry::from_ron(&mobs_text).unwrap_or_else(|e| {
            log::error!("{e}; using built-in mobs");
            voxel::mob::MobRegistry::from_ron(embedded::MOBS_RON).unwrap_or_default()
        });
        let painting_defs = paintings::load_defs(&base);
        let paintings = paintings::load(&save.dir, &painting_defs);
        let entities = entity::EntityRenderer::new(&renderer, &mob_reg, &painting_defs, &base);
        let mut mobs = mobs::Mobs::new(mob_reg, reg.id_or_air("grass_block"), seed);
        mobs.enabled = settings.mobs;

        let (sx, sy, sz) = gen.find_spawn();
        let mut body = Body::new(Vec3::new(sx as f32 + 0.5, sy as f32, sz as f32 + 0.5));
        let (mut yaw, mut pitch) = (0.0, 0.0);
        let mut hotbar_names: Vec<String> =
            ["grass_block", "dirt", "stone", "cobblestone", "oak_planks", "oak_log", "glass", "torch", "glowstone"].iter().map(|s| s.to_string()).collect();
        let mut time_of_day = 1000.0;
        let mut needs_spawn = true;
        let mut survival = args.survival;
        let mut health = 20.0;
        if let Some(l) = &level {
            survival = l.survival || args.survival;
            health = if l.health > 0.0 { l.health } else { 20.0 };
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
        if survival {
            body.flying = false;
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
            pads: Controllers::new(),
            rebinding: None,
            pad_held: HashSet::new(),
            pad_back: false,
            pad_scroll_cd: 0.0,
            mobs,
            entities,
            survival,
            health,
            hurt_age: 100.0,
            regen: 0.0,
            fall_peak: f32::MIN,
            air: 10.0,
            attack_cd: 0.0,
            last_hit: None,
            painting_defs,
            paintings,
            painting_spot: None,
            painting_preview: None,
            painting_scroll: 0,
            painting_check: 0.0,
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
            survival: self.survival,
            health: self.health,
        };
        if self.screen != Screen::Loading || !self.needs_spawn {
            if let Err(e) = self.world.save.save_level(&level) {
                log::error!("could not save the world: {e}");
            }
        }
        self.world.save_all();
        paintings::save(&self.world.save.dir, &self.paintings);
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
                Screen::Controls => self.set_screen(Screen::Paused),
                Screen::Paintings => self.set_screen(Screen::Playing),
                Screen::Loading | Screen::Dead => {}
            },
            KeyCode::KeyE => match self.screen {
                Screen::Playing => self.set_screen(Screen::Inventory),
                Screen::Inventory => self.set_screen(Screen::Playing),
                _ => {}
            },
            KeyCode::KeyT if f3 => self.reload_textures(),
            KeyCode::KeyP => match self.screen {
                Screen::Playing => self.open_paintings(),
                Screen::Paintings => self.set_screen(Screen::Playing),
                _ => {}
            },
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
            KeyCode::Space if self.screen == Screen::Playing && !self.survival => {
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
            Screen::Paintings => {
                if lines > 0.0 {
                    self.painting_scroll = self.painting_scroll.saturating_sub(1);
                } else {
                    self.painting_scroll += 1;
                }
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
                if self.args.mobs {
                    self.spawn_demo_mobs();
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
            self.update_survival(dt);
        }
        if self.screen != Screen::Loading && self.screen != Screen::Paused && self.screen != Screen::Controls {
            let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
            let daylight = (a.sin() * 2.2 + 0.45).clamp(0.0, 1.0);
            let creative = !self.survival || self.screen == Screen::Dead;
            // Screenshots hold the world still so the picture is stable.
            let mdt = if self.args.screenshot.is_some() { 0.0 } else { dt };
            let out = self.mobs.update(&self.world, self.body.pos, creative, daylight, mdt);
            for (dmg, from) in out.damage {
                self.damage(dmg, from);
            }
            for (pos, power) in out.explosions {
                self.explode(pos, power);
            }
            self.painting_check -= dt;
            if self.painting_check <= 0.0 {
                self.painting_check = 0.5;
                let defs = &self.painting_defs;
                let world = &self.world;
                self.paintings.retain(|p| {
                    let n = IVec3::from(p.facing);
                    p.cells(defs).iter().all(|&c| world.block(c - n).is_none() || voxel::physics::Terrain::solid(world, c - n))
                });
            }
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

        self.update_controller(dt);

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
        if playing && self.pads.pad.connected {
            wish += fwd * self.pads.pad.ly + right * self.pads.pad.lx;
        }
        let held = |a: Action| playing && self.pad_held.contains(&a);
        let input = Input {
            wish,
            jump: k(KeyCode::Space) || held(Action::Jump),
            sneak: k(KeyCode::ShiftLeft) || k(KeyCode::ShiftRight) || held(Action::Sneak),
            sprint: k(KeyCode::ControlLeft) || k(KeyCode::ControlRight) || held(Action::Sprint),
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
        let attack = self.mouse_held[0] || self.pad_held.contains(&Action::Attack);
        let use_ = self.mouse_held[1] || self.pad_held.contains(&Action::Use);
        self.attack_cd -= dt;
        if attack && self.attack_cd <= 0.0 && self.break_cd <= 0.0 {
            let block_dist = self.target.map(|t| t.distance).unwrap_or(f32::MAX);
            if let Some((i, t)) = paintings::pick(&self.paintings, &self.painting_defs, self.body.eye(), self.forward(), REACH) {
                if t < block_dist {
                    self.paintings.remove(i);
                    self.break_cd = 0.3;
                    self.attack_cd = 0.3;
                    self.sfx(Sfx::Break(Material::Wood));
                }
            }
        }
        if attack && self.attack_cd <= 0.0 {
            let block_dist = self.target.map(|t| t.distance).unwrap_or(f32::MAX);
            if let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), 3.5) {
                if t < block_dist {
                    self.attack_cd = 0.45;
                    self.break_cd = 0.3;
                    let eye = self.body.eye();
                    let dmg = if self.body.vel.y < -0.5 && !self.body.on_ground { 6.0 } else { 4.0 };
                    let m = &mut self.mobs.list[i];
                    m.hurt(dmg, eye);
                    let def = &self.mobs.reg.defs[m.def];
                    self.last_hit = Some((def.display_name.clone(), m.health.max(0.0), m.max_health, self.now + 3.0));
                    self.sfx(Sfx::MobHit);
                    self.pads.rumble(0.3, 0.6, 0.1);
                }
            }
        }
        if attack && self.break_cd <= 0.0 {
            if let Some(hit) = self.target {
                let id = self.world.block_or_air(hit.block);
                if id != AIR && self.reg.get(id).hardness >= 0.0 && self.world.break_block(hit.block) {
                    // Survival: harder blocks take longer (no tools yet).
                    self.break_cd = if self.survival { 0.25 + self.reg.get(id).hardness.min(5.0) * 0.25 } else { 0.22 };
                    let m = Material::of(&self.reg.get(id).name);
                    let vol = self.settings.sound_volume;
                    if let Some(a) = self.audio.as_mut() {
                        a.play(Sfx::Break(m), vol);
                    }
                    self.pads.rumble(0.5, 0.2, 0.08);
                }
            }
        }
        if use_ && self.place_cd <= 0.0 {
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

    /// Open the painting picker for the wall face the player is looking at.
    fn open_paintings(&mut self) {
        let Some(hit) = self.target else {
            self.toast("Mire em uma parede para pendurar uma pintura", 3.0);
            return;
        };
        if hit.normal.y != 0 {
            self.toast("Pinturas vão em paredes, não no chão ou no teto", 3.0);
            return;
        }
        self.painting_spot = Some((hit.block + hit.normal, hit.normal));
        self.painting_scroll = 0;
        self.painting_preview = None;
        self.set_screen(Screen::Paintings);
    }

    /// Paintings that fit the chosen wall: (def index, anchor), biggest first.
    fn fitting_paintings(&self) -> Vec<(usize, IVec3)> {
        let Some((cell, n)) = self.painting_spot else { return Vec::new() };
        let mut v: Vec<(usize, IVec3)> = Vec::new();
        for (i, d) in self.painting_defs.iter().enumerate() {
            if let Some(a) = paintings::fit(&self.world, &self.paintings, &self.painting_defs, cell, n, d.width, d.height) {
                v.push((i, a));
            }
        }
        v.sort_by_key(|(i, _)| std::cmp::Reverse(self.painting_defs[*i].width * self.painting_defs[*i].height));
        v
    }

    fn hang_painting(&mut self, def: usize, anchor: IVec3) {
        let Some((_, n)) = self.painting_spot else { return };
        self.paintings.push(paintings::Painting { name: self.painting_defs[def].name.clone(), anchor: anchor.to_array(), facing: n.to_array(), def });
        self.sfx(Sfx::Place(Material::Wood));
        self.pads.rumble(0.0, 0.35, 0.05);
        self.set_screen(Screen::Playing);
    }

    fn draw_paintings_menu(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        // Light backdrop on the left half only, so the preview on the wall stays visible.
        let pw = (200.0 * s).min(w * 0.5);
        self.ui.rect(0.0, 0.0, pw + 12.0 * s, h, [0, 0, 0, 150]);
        let fits = self.fitting_paintings();
        let x = 6.0 * s;
        let mut y = 8.0 * s;
        self.ui.text(x, y, "Escolha uma pintura (P fecha)", WHITE);
        y += 12.0 * s;
        if fits.is_empty() {
            self.ui.text(x, y, "Nenhuma cabe aqui: precisa de parede", GRAY);
            self.ui.text(x, y + 10.0 * s, "e espaço livre na frente.", GRAY);
            return;
        }
        let bh = 14.0 * s;
        let rows = (((h - y - 8.0 * s) / (bh + 2.0 * s)).floor() as usize).max(2) - 1;
        self.painting_scroll = self.painting_scroll.min(fits.len().saturating_sub(rows));
        self.painting_preview = None;
        if self.ui.button(x, y, pw, bh, "Aleatória (cabe aqui)", mouse) {
            if self.clicked {
                let k = (self.now * 1000.0) as usize % fits.len();
                let (d, a) = fits[k];
                self.hang_painting(d, a);
                return;
            }
        }
        y += bh + 2.0 * s;
        for &(d, a) in fits.iter().skip(self.painting_scroll).take(rows) {
            let def = &self.painting_defs[d];
            let label = format!("{} ({}x{})", def.title, def.width, def.height);
            if self.ui.button(x, y, pw, bh, &label, mouse) {
                self.painting_preview = Some((d, a));
                if self.clicked {
                    self.hang_painting(d, a);
                    return;
                }
            }
            y += bh + 2.0 * s;
        }
        if fits.len() > rows {
            self.ui.text(x, y, &format!("roda do mouse: mais ({}/{})", self.painting_scroll + rows.min(fits.len()), fits.len()), GRAY);
        }
    }

    fn sfx(&mut self, s: Sfx) {
        let vol = self.settings.sound_volume;
        if let Some(a) = self.audio.as_mut() {
            a.play(s, vol);
        }
    }

    /// Hurt the player (survival only). `from` pushes the player away.
    fn damage(&mut self, amount: f32, from: Option<Vec3>) {
        if !self.survival || self.screen == Screen::Dead || self.hurt_age < 0.5 || amount <= 0.0 {
            return;
        }
        self.health -= amount;
        self.hurt_age = 0.0;
        self.regen = 0.0;
        if let Some(f) = from {
            self.body.knockback(self.body.pos - f, 5.0);
        }
        self.sfx(Sfx::Hurt);
        self.pads.rumble(0.9, 0.6, 0.25);
        if self.health <= 0.0 {
            self.health = 0.0;
            self.set_screen(Screen::Dead);
        }
    }

    fn update_survival(&mut self, dt: f32) {
        self.hurt_age += dt;
        if !self.survival || self.screen == Screen::Dead {
            self.fall_peak = self.body.pos.y;
            return;
        }
        self.body.flying = false;
        // Fall damage: one point per block beyond three.
        if self.body.on_ground || self.body.in_water {
            let fall = self.fall_peak - self.body.pos.y;
            if fall > 3.5 && !self.body.in_water {
                self.damage((fall - 3.0).floor(), None);
            }
            self.fall_peak = self.body.pos.y;
        } else {
            self.fall_peak = self.fall_peak.max(self.body.pos.y);
        }
        // Breath under water.
        if self.body.head_in_water {
            self.air -= dt;
            if self.air < -1.0 {
                self.air = 0.0;
                self.damage(2.0, None);
            }
        } else {
            self.air = (self.air + dt * 5.0).min(10.0);
        }
        // Lava burns.
        let lava = self.reg.id_or_air("lava");
        let feet = self.body.pos.floor().as_ivec3();
        if lava != AIR && (self.world.block_or_air(feet) == lava || self.world.block_or_air(feet + IVec3::Y) == lava) {
            self.damage(4.0, None);
        }
        // Slow natural healing when out of danger.
        if self.health < 20.0 && self.hurt_age > 4.0 {
            self.regen += dt;
            if self.regen > 2.0 {
                self.regen = 0.0;
                self.health = (self.health + 1.0).min(20.0);
            }
        }
    }

    /// A blast: breaks blocks around it and hurts everything nearby.
    fn explode(&mut self, pos: Vec3, power: f32) {
        let r = power.ceil() as i32;
        let c = pos.floor().as_ivec3();
        for dy in -r..=r {
            for dz in -r..=r {
                for dx in -r..=r {
                    let d = IVec3::new(dx, dy, dz);
                    let dist = d.as_vec3().length();
                    // Ragged edges look more natural than a perfect ball.
                    let jitter = ((dx * 73 + dy * 151 + dz * 37 + c.x * 11 + c.z * 7).rem_euclid(10)) as f32 * 0.06;
                    if dist > power - jitter {
                        continue;
                    }
                    let p = c + d;
                    let id = self.world.block_or_air(p);
                    if id == AIR || self.world.is_liquid(id) {
                        continue;
                    }
                    let b = self.reg.get(id);
                    if b.hardness < 0.0 || b.hardness >= 50.0 {
                        continue;
                    }
                    self.world.break_block(p);
                }
            }
        }
        let reach = power * 2.0;
        let d = (self.body.center() - pos).length();
        if d < reach {
            let k = 1.0 - d / reach;
            self.damage((k * power * 7.0).round(), Some(pos));
            self.body.knockback(self.body.center() - pos, k * 12.0);
        }
        for m in self.mobs.list.iter_mut() {
            let d = (m.body.center() - pos).length();
            if d < reach && m.alive() {
                m.hurt((1.0 - d / reach) * power * 7.0, pos);
            }
        }
        self.sfx(Sfx::Explode);
        self.pads.rumble(1.0, 1.0, 0.6);
    }

    fn respawn(&mut self) {
        let (sx, sz) = self.spawn_xz;
        let y = self.world.surface_y(sx, sz).unwrap_or(100) + 1;
        self.body.pos = Vec3::new(sx as f32 + 0.5, y as f32, sz as f32 + 0.5);
        self.body.vel = Vec3::ZERO;
        self.health = 20.0;
        self.air = 10.0;
        self.hurt_age = 100.0;
        self.fall_peak = self.body.pos.y;
        self.set_screen(Screen::Playing);
    }

    /// Test scene: one of every mob in a row in front of the player.
    fn spawn_demo_mobs(&mut self) {
        let f = self.forward();
        let fwd = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-fwd.z, 0.0, fwd.x);
        let names: Vec<String> = self.mobs.reg.defs.iter().map(|d| d.name.clone()).collect();
        let n = names.len() as f32;
        for (i, name) in names.iter().enumerate() {
            let p = self.body.pos + fwd * 7.0 + side * ((i as f32 - (n - 1.0) / 2.0) * 1.8);
            let x = p.x.floor() as i32;
            let z = p.z.floor() as i32;
            let y = self.world.surface_y(x, z).unwrap_or(p.y as i32) + 1;
            let at = Vec3::new(p.x, y as f32, p.z);
            if self.mobs.spawn(name, at) {
                // Face the camera.
                let m = self.mobs.list.last_mut().unwrap();
                let to = self.body.pos - at;
                m.yaw = to.x.atan2(to.z);
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
            self.pads.rumble(0.0, 0.35, 0.05);
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

        if self.survival && self.hurt_age < 0.4 {
            let a = ((0.4 - self.hurt_age) / 0.4 * 90.0) as u8;
            self.ui.rect(0.0, 0.0, w, h, [200, 0, 0, a]);
        }
        if !self.hide_hud && self.screen != Screen::Paused && self.screen != Screen::Dead {
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
            if self.survival {
                // Ten hearts above the left of the hotbar, bubbles on the right under water.
                let y = hy - 11.0 * s;
                for i in 0..10 {
                    let x = hx + i as f32 * 8.0 * s;
                    let v = self.health - i as f32 * 2.0;
                    let blink = self.hurt_age < 0.6 && ((self.hurt_age * 10.0) as i32) % 2 == 0;
                    draw_heart(&mut self.ui, x, y, s, [40, 0, 0, 255], 1.0, blink);
                    if v > 0.0 {
                        draw_heart(&mut self.ui, x, y, s, [220, 30, 30, 255], if v >= 2.0 { 1.0 } else { 0.5 }, false);
                    }
                }
                if self.air < 10.0 {
                    let bubbles = (self.air.max(0.0).ceil()) as i32;
                    for i in 0..bubbles {
                        let x = hx + cell * 9.0 - (i + 1) as f32 * 8.0 * s;
                        self.ui.rect(x + s, y + s, 5.0 * s, 5.0 * s, [60, 120, 230, 255]);
                        self.ui.rect(x + 2.0 * s, y + 2.0 * s, 2.0 * s, 2.0 * s, [220, 240, 255, 255]);
                    }
                }
            }
            if let Some((name, hp, max, until)) = &self.last_hit {
                if *until > self.now {
                    let bw = 120.0 * s;
                    let bx = w / 2.0 - bw / 2.0;
                    let by = 14.0 * s;
                    let label = format!("{name}  {:.0}/{:.0}", hp.ceil(), max);
                    self.ui.text_centered(w / 2.0, by - 10.0 * s, &label, WHITE);
                    self.ui.rect(bx, by, bw, 3.0 * s, [40, 10, 10, 220]);
                    self.ui.rect(bx, by, bw * (hp / max).clamp(0.0, 1.0), 3.0 * s, [210, 40, 40, 255]);
                }
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
                let mut y = (h / 2.0 - 100.0 * s).max(26.0 * s);
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
                    format!("Controle: {}", if self.pads.pad.connected { self.pads.pad.name.as_str() } else { "nenhum conectado" }),
                    format!("Modo: {}", if self.survival { "Sobrevivência" } else { "Criativo" }),
                    format!("Mobs: {}", if self.settings.mobs { "Sim" } else { "Não" }),
                    "Salvar e sair".to_string(),
                ];
                // First and last buttons are wide; the options between them sit in two columns.
                let last = labels.len() - 1;
                let half = (bw - 4.0 * s) / 2.0;
                let mut col = 0;
                for (i, l) in labels.iter().enumerate() {
                    let wide = i == 0 || i == last;
                    if wide && col == 1 {
                        col = 0;
                        y += bh + 4.0 * s;
                    }
                    let (bx, bwi) = if wide { (x - bw / 2.0, bw * 2.0 + 4.0 * s) } else { (x - bw / 2.0 + col as f32 * (bw + 4.0 * s), bw) };
                    let _ = half;
                    if self.ui.button(bx, y, bwi, bh, l, mouse) && self.clicked {
                        clicked_button = Some(i);
                    }
                    if wide || col == 1 {
                        col = 0;
                        y += bh + 4.0 * s;
                    } else {
                        col = 1;
                    }
                }
                let help = [
                    "WASD andar  Espaço pular (2x = voar)  Shift descer  Ctrl correr",
                    "Esq. quebrar  Dir. colocar  Meio copiar bloco  E blocos  1-9/roda escolher",
                    "F1 esconder HUD  F2 foto  F3 info  F3+T recarregar texturas  F11 tela cheia",
                    "Controle: analógicos andar e olhar, botões no menu Controle",
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
            Screen::Paintings => self.draw_paintings_menu(w, h, mouse),
            Screen::Dead => {
                self.ui.rect(0.0, 0.0, w, h, [120, 0, 0, 110]);
                self.ui.text_centered(w / 2.0, h / 2.0 - 40.0 * s, "Você morreu!", WHITE);
                let bw = 200.0 * s;
                let bh = 20.0 * s;
                if self.ui.button((w - bw) / 2.0, h / 2.0, bw, bh, "Renascer", mouse) && self.clicked {
                    clicked_button = Some(0);
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
            Screen::Controls => {
                self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 170]);
                clicked_button = self.draw_controls_menu(w, h, mouse);
            }
            _ => {}
        }
        // Virtual cursor when a controller drives the menus.
        if self.pads.pad.connected && self.screen != Screen::Playing && self.screen != Screen::Loading {
            let (mx, my) = mouse;
            let c = [255, 255, 255, 255];
            self.ui.rect(mx, my, s, 7.0 * s, c);
            self.ui.rect(mx, my, 7.0 * s, s, c);
            self.ui.rect(mx + s, my + s, s, s, c);
            self.ui.rect(mx + 2.0 * s, my + 2.0 * s, s, s, c);
            self.ui.rect(mx + 3.0 * s, my + 3.0 * s, s, s, c);
        }
        self.clicked = false;

        if let Some(i) = clicked_button {
            if self.screen == Screen::Controls {
                self.controls_button(i);
            } else if self.screen == Screen::Dead {
                self.respawn();
            } else {
                self.pause_button(i);
            }
        }
        let _ = YELLOW;
    }

    /// Rows of the controller menu: settings first, then one row per action.
    fn controls_labels(&self) -> Vec<String> {
        let c = &self.settings.controller;
        let sony = self.pads.pad.sony;
        let onoff = |b: bool| if b { "Sim" } else { "Não" };
        let mut v = vec![
            "Voltar".to_string(),
            format!("Controle: {}", onoff(c.enabled)),
            format!("Sensibilidade da câmera: {}", c.look_sensitivity),
            format!("Inverter eixo Y: {}", onoff(c.invert_y)),
            format!("Zona morta: {:.0}%", c.deadzone * 100.0),
            format!("Mira com giroscópio: {} ({})", onoff(c.gyro_aim), c.gyro_sensitivity),
            format!(
                "Touchpad: {}",
                match c.touchpad_mode {
                    TouchpadMode::Hotbar => "trocar item (deslizar)",
                    TouchpadMode::Camera => "mover a câmera",
                }
            ),
            format!("Vibração: {}", onoff(c.rumble)),
            format!("Barra de luz: {}", onoff(c.lightbar)),
            "Restaurar padrão".to_string(),
        ];
        for a in Action::ALL {
            let b = c.button_for(a).map(|b| b.label(sony)).unwrap_or("--");
            if self.rebinding == Some(a) {
                v.push(format!("{}: aperte um botão...", a.label()));
            } else {
                v.push(format!("{}: {}", a.label(), b));
            }
        }
        v
    }

    fn draw_controls_menu(&mut self, w: f32, h: f32, mouse: (f32, f32)) -> Option<usize> {
        let s = self.ui.scale;
        let labels = self.controls_labels();
        let bw = 170.0 * s;
        let bh = 14.0 * s;
        let gap = 2.0 * s;
        let cols = 2;
        let rows = labels.len().div_ceil(cols);
        let total_w = bw * cols as f32 + gap;
        let x0 = (w - total_w) / 2.0;
        let y0 = ((h - rows as f32 * (bh + gap)) / 2.0).max(28.0 * s);
        let title = match (&self.pads.pad.connected, self.pads.pad.hid) {
            (true, true) => format!("Controle: {} (touchpad, giroscópio e luz ativos)", self.pads.pad.name),
            (true, false) => format!("Controle: {}", self.pads.pad.name),
            (false, _) => "Nenhum controle conectado (conecte por USB ou Bluetooth)".to_string(),
        };
        self.ui.text_centered(w / 2.0, y0 - 16.0 * s, &title, WHITE);
        let mut hit = None;
        for (i, l) in labels.iter().enumerate() {
            let col = if i < 10 { 0 } else { 1 };
            let row = if i < 10 { i } else { i - 10 };
            let x = x0 + col as f32 * (bw + gap);
            let y = y0 + row as f32 * (bh + gap);
            let hover = self.ui.button(x, y, bw, bh, l, mouse);
            if hover && self.clicked {
                hit = Some(i);
            }
        }
        if let Some(b) = self.pads.pad.battery {
            if self.pads.pad.hid {
                self.ui.text_centered(w / 2.0, y0 + rows as f32 * (bh + gap) + 4.0 * s, &format!("Bateria: {b}%"), GRAY);
            }
        }
        hit
    }

    fn controls_button(&mut self, i: usize) {
        if let Some(a) = self.audio.as_mut() {
            a.play(Sfx::Click, self.settings.sound_volume);
        }
        let c = &mut self.settings.controller;
        match i {
            0 => {
                self.rebinding = None;
                self.set_screen(Screen::Paused);
            }
            1 => c.enabled = !c.enabled,
            2 => c.look_sensitivity = c.look_sensitivity % 10 + 1,
            3 => c.invert_y = !c.invert_y,
            4 => c.deadzone = if c.deadzone >= 0.3 { 0.05 } else { c.deadzone + 0.05 },
            5 => {
                if !c.gyro_aim {
                    c.gyro_aim = true;
                } else if c.gyro_sensitivity >= 10 {
                    c.gyro_aim = false;
                    c.gyro_sensitivity = 5;
                } else {
                    c.gyro_sensitivity += 1;
                }
            }
            6 => c.touchpad_mode = if c.touchpad_mode == TouchpadMode::Hotbar { TouchpadMode::Camera } else { TouchpadMode::Hotbar },
            7 => c.rumble = !c.rumble,
            8 => c.lightbar = !c.lightbar,
            9 => *c = controller::ControllerSettings::default(),
            n => {
                let a = Action::ALL[n - 10];
                self.rebinding = if self.rebinding == Some(a) { None } else { Some(a) };
            }
        }
        self.settings.save(&self.base.join("settings.ron"));
    }

    fn pad_action_pressed(&mut self, a: Action) {
        match a {
            Action::Jump => {
                if self.now - self.last_space < 0.3 {
                    self.body.flying = !self.body.flying;
                    self.body.vel.y = 0.0;
                    self.last_space = -10.0;
                } else {
                    self.last_space = self.now;
                }
            }
            Action::PickBlock => self.mouse_held[2] = true,
            Action::Inventory => self.set_screen(Screen::Inventory),
            Action::Paintings => self.open_paintings(),
            Action::Pause => self.set_screen(Screen::Paused),
            Action::HotbarPrev => self.scroll(1.0),
            Action::HotbarNext => self.scroll(-1.0),
            Action::Screenshot => self.screenshot_request = true,
            Action::Debug => self.show_debug = !self.show_debug,
            Action::HideHud => self.hide_hud = !self.hide_hud,
            Action::NextSong => {
                let sit = self.situation();
                if let Some(au) = self.audio.as_mut() {
                    au.skip(sit, &self.settings);
                }
            }
            _ => {}
        }
    }

    fn update_controller(&mut self, dt: f32) {
        self.pads.update(dt, &self.settings.controller);
        self.pad_held.clear();
        let pad = self.pads.pad.clone();
        if !pad.connected {
            return;
        }
        let c = self.settings.controller.clone();

        // Light bar: sky blue by day, deep blue at night, teal under water.
        let day = self.sky().2;
        self.pads.light = if self.body.head_in_water {
            [0, 90, 110]
        } else {
            [(40.0 * day) as u8, (90.0 * day + 10.0) as u8, (200.0 * day + 40.0) as u8]
        };

        // Remapping: the next button pressed becomes the binding.
        if let Some(a) = self.rebinding {
            if let Some(b) = pad.any_pressed() {
                self.settings.controller.bind(a, b);
                self.rebinding = None;
                self.settings.save(&self.base.join("settings.ron"));
            }
            return;
        }

        match self.screen {
            Screen::Playing => {
                for a in Action::ALL {
                    if let Some(b) = c.button_for(a) {
                        if pad.down(b) {
                            self.pad_held.insert(a);
                        }
                        if pad.pressed(b) {
                            self.pad_action_pressed(a);
                        }
                    }
                }
                // Camera: right stick with a gentle response curve.
                let sens = c.look_sensitivity as f32 * 0.32;
                let curve = |v: f32| v.abs().powf(1.6) * v.signum();
                let inv = if c.invert_y { -1.0 } else { 1.0 };
                self.yaw -= curve(pad.rx) * sens * dt;
                self.pitch -= curve(pad.ry) * sens * dt * inv * -1.0;
                if c.gyro_aim && pad.hid {
                    let g = c.gyro_sensitivity as f32 * 0.09;
                    self.yaw += pad.gyro[1].to_radians() * g * dt;
                    self.pitch += pad.gyro[0].to_radians() * g * dt * inv;
                }
                if pad.hid {
                    match c.touchpad_mode {
                        TouchpadMode::Camera => {
                            if let Some(d) = pad.touch_delta() {
                                let m = self.settings.mouse_sensitivity.to_radians() * 1100.0;
                                self.yaw -= d[0] * m;
                                self.pitch = (self.pitch - d[1] * m * inv).clamp(-1.55, 1.55);
                            }
                        }
                        TouchpadMode::Hotbar => {
                            // A swipe of a quarter of the pad changes the item once.
                            if let (Some(a), Some(b)) = (pad.touch[0], pad.prev_touch[0]) {
                                self.pad_scroll_cd += a[0] - b[0];
                                if self.pad_scroll_cd > 0.22 {
                                    self.pad_scroll_cd = 0.0;
                                    self.scroll(-1.0);
                                } else if self.pad_scroll_cd < -0.22 {
                                    self.pad_scroll_cd = 0.0;
                                    self.scroll(1.0);
                                }
                            } else {
                                self.pad_scroll_cd = 0.0;
                            }
                        }
                    }
                }
                self.pitch = self.pitch.clamp(-1.55, 1.55);
            }
            Screen::Loading => {}
            _ => {
                // Menus: left stick or touchpad moves the cursor, South clicks, East goes back.
                let speed = 900.0 * dt;
                let (w, h) = (self.renderer.config.width as f32, self.renderer.config.height as f32);
                let mut m = self.mouse_pos;
                m.0 += pad.lx * speed;
                m.1 -= pad.ly * speed;
                if let Some(d) = pad.touch_delta() {
                    m.0 += d[0] * w;
                    m.1 += d[1] * h;
                }
                self.mouse_pos = (m.0.clamp(0.0, w - 1.0), m.1.clamp(0.0, h - 1.0));
                if pad.pressed(Btn::South) || pad.pressed(Btn::Touchpad) {
                    self.clicked = true;
                }
                let back = c.button_for(Action::Pause).unwrap_or(Btn::Start);
                if pad.pressed(Btn::East) || pad.pressed(back) {
                    self.pad_back = true;
                }
                if pad.pressed(Btn::Down) || pad.pressed(Btn::R1) {
                    self.scroll(-1.0);
                }
                if pad.pressed(Btn::Up) || pad.pressed(Btn::L1) {
                    self.scroll(1.0);
                }
                if self.pad_back {
                    self.pad_back = false;
                    match self.screen {
                        Screen::Controls => {
                            self.rebinding = None;
                            self.set_screen(Screen::Paused);
                        }
                        Screen::Dead => {}
                        _ => self.set_screen(Screen::Playing),
                    }
                }
            }
        }
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
            7 => self.set_screen(Screen::Controls),
            8 => {
                self.survival = !self.survival;
                if self.survival {
                    self.body.flying = false;
                    self.health = 20.0;
                    self.fall_peak = self.body.pos.y;
                    self.toast("Modo sobrevivência: cuidado com quedas e monstros", 4.0);
                } else {
                    self.toast("Modo criativo", 3.0);
                }
            }
            9 => {
                self.settings.mobs = !self.settings.mobs;
                self.mobs.enabled = self.settings.mobs;
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
            if self.pads.pad.connected {
                let p = &self.pads.pad;
                lines.push(format!(
                    "Controle: {} ({}) L {:.2},{:.2} R {:.2},{:.2} toque {:?}",
                    p.name,
                    if p.hid { "HID" } else { "genérico" },
                    p.lx,
                    p.ly,
                    p.rx,
                    p.ry,
                    p.touch[0].map(|t| ((t[0] * 100.0) as i32, (t[1] * 100.0) as i32))
                ));
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
        let mut items = self.mobs.instances(&self.world, block, rd + 16.0);
        for p in &self.paintings {
            items.push((self.entities.painting_mesh(p.def), p.bones(block, &self.world)));
        }
        if let (Screen::Paintings, Some((d, anchor)), Some((_, n))) = (self.screen, self.painting_preview, self.painting_spot) {
            let p = paintings::Painting { name: String::new(), anchor: anchor.to_array(), facing: n.to_array(), def: d };
            items.push((self.entities.painting_mesh(d), p.bones(block, &self.world)));
        }
        self.entities.prepare(&self.renderer, items);
        let (info, shot) = self.renderer.render(&cam, &mut g, selection, self.settings.show_clouds, Some(&self.entities), &self.ui, capture);
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

/// A little pixel heart, 7×6 "pixels" of size `s`. `fill` < 1 draws only the left half.
fn draw_heart(ui: &mut UiBatch, x: f32, y: f32, s: f32, color: [u8; 4], fill: f32, outline: bool) {
    const ROWS: [&str; 6] = [".XX.XX.", "XXXXXXX", "XXXXXXX", ".XXXXX.", "..XXX..", "...X..."];
    let color = if outline { [255, 255, 255, 255] } else { color };
    let max_col = if fill >= 1.0 { 7 } else { 4 };
    for (r, row) in ROWS.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            if ch == 'X' && c < max_col {
                ui.rect(x + c as f32 * s, y + r as f32 * s, s, s, color);
            }
        }
    }
}
