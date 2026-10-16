//! Mine Clone: a fast block game written in Rust.

// No console window behind the game on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod controller;
mod creative;
mod entity;
mod item_render;
mod menus;
mod monitor;
mod lod;
mod mobs;
mod paintings;
mod particles;
mod player_model;
mod post;
mod renderer;
mod settings;
mod sfx;
mod shaderpack;
mod survival;
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

use glam::{IVec3, Vec3, Vec4};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

use audio::{Audio, Material, Sfx, Situation};
use controller::{Action, Btn, Controllers, TouchpadMode};
use renderer::{Camera, Globals, Renderer};
use settings::{Settings, ShaderQuality};
use shaderpack::{Builtin, ShaderPack};
use ui::{FontAtlas, UiBatch, GRAY, WHITE, YELLOW};
use voxel::block::Shape;
use voxel::item::ItemRegistry;
use voxel::recipe::Recipes;
use voxel::chunk::ChunkPos;
use voxel::mesh::MeshTables;
use voxel::physics::{Body, Input};
use voxel::raycast::{raycast, RayHit};
use voxel::save::{LevelData, WorldSave};
use voxel::worldgen::{Biome, Generator};
use voxel::{BlockId, BlockRegistry, AIR};
use world::World;

const REACH: f32 = 6.0;

/// Textures loaded after the block and item ones: the experience orb, then the ten cracks
/// drawn over a block while it is being broken.
fn extra_textures() -> Vec<String> {
    let mut v = vec!["item/xp_orb".to_string()];
    v.extend((0..10).map(|i| format!("destroy_stage_{i}")));
    v
}
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
    shapes: bool,
    /// Take the --screenshot of the loading screen instead of the game.
    shot_loading: bool,
    /// Creative inventory tab to open (0..13).
    ctab: Option<usize>,
    show_inventory: bool,
    show_pause: bool,
    /// Open the shader menu once the world is ready (test screenshots).
    show_shaders: bool,
    debug: bool,
    no_cave_cull: bool,
    /// Spawn a few mobs in front of the player (test screenshots).
    mobs: bool,
    survival: bool,
    /// Test: fill the survival inventory and drop items in front of the player.
    kit: bool,
    /// Test: show the target block this far broken (0..1).
    mine: Option<f32>,
    /// Test hook: freeze the arm swing at this point (0..1) for screenshots.
    swing: Option<f32>,
    /// Test hook: die on arrival (the body with the items is left on the ground).
    die: bool,
    /// Test: hotbar slot in hand (1-9).
    slot: Option<usize>,
    /// Test: open a survival screen (table, furnace or chest).
    open: Option<String>,
    particles: bool,
    camera: u8,
    perf: bool,
    prone: bool,
    /// Show the title screen even with --world/--screenshot.
    title: bool,
    /// Shader pack to use ("off" turns shaders off).
    shader: Option<String>,
    /// Distant terrain past the render distance, in blocks (0 = off).
    lod: Option<i32>,
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
            "--shapes" => a.shapes = true,
            "--shot-loading" => a.shot_loading = true,
            "--ctab" => a.ctab = it.next().and_then(|v| v.parse().ok()),
            "--inventory" => a.show_inventory = true,
            "--pause" => a.show_pause = true,
            "--shaders-menu" => a.show_shaders = true,
            "--debug" => a.debug = true,
            "--no-cave-cull" => a.no_cave_cull = true,
            "--mobs" => a.mobs = true,
            "--particles" => a.particles = true,
            "--perf" => a.perf = true,
            "--prone" => a.prone = true,
            "--camera" => a.camera = it.next().and_then(|v| v.parse().ok()).unwrap_or(1),
            "--title" => a.title = true,
            "--survival" => a.survival = true,
            "--kit" => a.kit = true,
            "--mine" => a.mine = it.next().and_then(|v| v.parse().ok()),
            "--die" => a.die = true,
            "--swing" => a.swing = it.next().and_then(|v| v.parse().ok()),
            "--slot" => a.slot = it.next().and_then(|v| v.parse::<usize>().ok()).map(|v| v.clamp(1, 9) - 1),
            "--open" => a.open = it.next(),
            "--shader" => a.shader = it.next(),
            "--lod" => a.lod = it.next().and_then(|s| s.parse().ok()),
            "--help" | "-h" => {
                println!(
                    "mine-clone [--world NAME] [--seed N] [--render-distance N] [--size WxH]\n\
                     Testing: [--screenshot out.png] [--pos x,y,z] [--look yaw,pitch] [--time ticks] [--fly] [--demo] [--inventory] [--pause] [--debug]\n\
                     \x20        [--survival] [--kit] [--open table|furnace|chest] [--mobs] [--perf] [--prone] [--camera N]\n\
                     \x20        [--shader NAME|off] [--shaders-menu] [--lod BLOCKS]"
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
/// Shrink a frame to a small grid and blur it, for the loading screen's background.
fn blurred_backdrop(w: usize, h: usize, px: &[u8]) -> (usize, usize, Vec<[u8; 4]>) {
    let (bw, bh) = (40usize, ((40 * h / w.max(1)).max(2)).min(40));
    let mut out = vec![[0u8; 4]; bw * bh];
    for by in 0..bh {
        for bx in 0..bw {
            let (x0, x1) = (bx * w / bw, ((bx + 1) * w / bw).max(bx * w / bw + 1));
            let (y0, y1) = (by * h / bh, ((by + 1) * h / bh).max(by * h / bh + 1));
            let mut sum = [0u64; 3];
            let mut n = 0u64;
            for y in (y0..y1.min(h)).step_by(2) {
                for x in (x0..x1.min(w)).step_by(2) {
                    let i = (y * w + x) * 4;
                    for k in 0..3 {
                        sum[k] += px[i + k] as u64;
                    }
                    n += 1;
                }
            }
            let n = n.max(1);
            out[by * bw + bx] = [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, 255];
        }
    }
    // Two passes of a 3x3 box blur soften it further.
    for _ in 0..2 {
        let src = out.clone();
        for y in 0..bh {
            for x in 0..bw {
                let mut sum = [0u32; 3];
                let mut n = 0;
                for yy in y.saturating_sub(1)..(y + 2).min(bh) {
                    for xx in x.saturating_sub(1)..(x + 2).min(bw) {
                        for k in 0..3 {
                            sum[k] += src[yy * bw + xx][k] as u32;
                        }
                        n += 1;
                    }
                }
                out[y * bw + x] = [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, 255];
            }
        }
    }
    (bw, bh, out)
}

/// Blocks, items and recipes from text.
fn game_data(blocks: &str, items: &str, recipes: &str) -> Result<(BlockRegistry, ItemRegistry, Recipes), String> {
    let reg = BlockRegistry::from_ron(blocks)?;
    let items = ItemRegistry::from_ron(items, &reg)?;
    let recipes = Recipes::from_ron(recipes, &items)?;
    Ok((reg, items, recipes))
}

/// The data files in `packs/vanilla/` when there are any, each falling back to the built-in copy.
fn load_game_data(base: &Path) -> Result<(BlockRegistry, ItemRegistry, Recipes), String> {
    let read = |name: &str, built_in: &str| std::fs::read_to_string(base.join("packs/vanilla").join(name)).unwrap_or_else(|_| built_in.to_string());
    game_data(&read("blocks.ron", embedded::BLOCKS_RON), &read("items.ron", embedded::ITEMS_RON), &read("recipes.ron", embedded::RECIPES_RON))
}

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
    shaderpack::ensure_folder(&base.join("shaderpacks"));
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
    /// Main menu, over a panorama of the last world.
    Title,
    Worlds,
    CreateWorld,
    Multiplayer,
    Options,
    Packs,
    /// Shader packs and their options.
    Shaders,
}

/// What a row of the shader menu does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ShaderRow {
    Back,
    Quality,
    Reload,
    /// Choose a pack (None = shaders off).
    Pack(Option<usize>),
    /// Change an option of the pack in use.
    Opt(usize),
    /// Distant terrain distance.
    Lod,
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
    time_of_day: f32,

    screen: Screen,
    keys: HashSet<KeyCode>,
    mouse_held: [bool; 3],
    mouse_pos: (f32, f32),
    clicked: bool,
    /// The attack button went down this frame (survival swings use it).
    attack_pressed: bool,
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

    /// Survival mining: block being broken and how far along (seconds).
    mining: Option<(IVec3, f32)>,
    particles: particles::Particles,
    anim: player_model::Animator,
    /// 0 = first person, 1 = behind, 2 = in front (F5).
    camera_mode: u8,
    /// A world is being played (not just shown behind the title).
    world_open: bool,
    /// The world loaded behind the menus.
    title_world: String,
    title_spin: f32,
    worlds: Vec<menus::WorldEntry>,
    world_sel: Option<usize>,
    new_world: menus::NewWorldForm,
    /// Where the controller menu returns to.
    options_back: Screen,
    mining_tick: f32,
    sprinting: bool,
    /// Time W was last pressed (double tap = sprint).
    last_w: f32,
    /// 0..1 smoothed sneak (camera drop) and sprint (wider view).
    crouch: f32,
    /// Lying down (Z): wanted, and 0..1 smoothed for the camera and the body.
    prone: bool,
    prone_t: f32,
    fov_kick: f32,
    was_in_water: bool,
    swim_dist: f32,
    items: ItemRegistry,
    recipes: Recipes,
    /// First texture layer of the item sprites (they come after the block textures).
    item_base: u16,
    surv: survival::Survival,
    rclicked: bool,
    /// Middle click in a menu this frame.
    mclicked: bool,
    /// Right button held (menus: drag one item into each slot passed over).
    rdown: bool,
    /// Last frame's camera (view-projection and camera block) for drawing labels over mobs.
    ui_view_proj: Option<(glam::Mat4, IVec3)>,
    inv_number_key: Option<usize>,
    bubble_cd: f32,

    ui: UiBatch,
    show_debug: bool,
    show_perf: bool,
    perf: monitor::FpsStats,
    sysmon: monitor::SysMonitor,
    hide_hud: bool,
    toast: Option<Toast>,
    packs: Vec<String>,
    textures_name: String,
    shader_packs: Vec<ShaderPack>,
    /// Why the chosen shader pack is not running (shown in the shader menu).
    shader_error: Option<String>,
    shader_scroll: usize,
    lod: lod::Lod,

    start: Instant,
    last: Instant,
    now: f32,
    frame_times: Vec<f32>,
    fps: f32,
    last_frame_info: (usize, usize, usize),
    autosave: f32,
    loaded_frames: u32,
    /// Loading screen: the last frame shrunk and blurred (width, height, pixels), and whether to
    /// grab it from the next frame.
    backdrop: Option<(usize, usize, Vec<[u8; 4]>)>,
    want_backdrop: bool,
    /// Loading screen: the chunk it waits around and how far.
    loading_area: (ChunkPos, i32),
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
        settings.controller.fill_missing();
        if let Some(rd) = args.render_distance {
            settings.render_distance = rd;
        }
        if let Some(l) = args.lod {
            settings.lod_distance = l.max(0);
        }
        if let Some(sh) = &args.shader {
            settings.shader_pack = if sh == "off" { String::new() } else { sh.clone() };
        }
        ensure_folders(&base, &settings);
        // The pack that comes with the game is on the first time it is found.
        const BUNDLED_PACK: &str = "Vanilla Experience ++.zip";
        if !settings.bundled_pack_offered && base.join("resourcepacks").join(BUNDLED_PACK).exists() {
            settings.bundled_pack_offered = true;
            settings.resource_packs = vec![BUNDLED_PACK.to_string()];
            settings.save(&settings_path);
        }

        // Blocks, items and recipes: files on disk win (so they can be modded), otherwise the
        // built-in copies. They depend on each other, so if any file is missing or doesn't fit
        // the others (an old copy left from an earlier version), all three come from the built-in set.
        let (reg, items, recipes) = load_game_data(&base).unwrap_or_else(|e| {
            log::error!("{e}; using the built-in blocks, items and recipes");
            game_data(embedded::BLOCKS_RON, embedded::ITEMS_RON, embedded::RECIPES_RON).expect("built-in game data")
        });
        let reg = Arc::new(reg);
        let item_base = reg.textures.len() as u16;
        let texture_names: Vec<String> = reg.textures.iter().chain(items.textures.iter()).cloned().chain(extra_textures()).collect();

        // Without --world the game opens on the title screen, with the last played world behind it.
        let camera_mode = args.camera.min(2);
        let show_perf = args.perf;
        let start_prone = args.prone;
        let title_mode = args.title || (args.world.is_none() && args.screenshot.is_none());
        let world_name = args.world.clone().or_else(|| menus::list_worlds(&base).first().map(|w| w.name.clone())).unwrap_or_else(|| "Mundo 1".into());
        let save = Arc::new(WorldSave::new(level_path_hint(&base, &world_name)));
        let level = save.load_level();
        let seed = level.as_ref().map(|l| l.seed).or(args.seed).unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
        });
        let gen = Arc::new(Generator::new(seed, &reg));
        let tints: Vec<[f32; 3]> = (0..Biome::COUNT).map(|b| Biome::from_u8(b as u8).tint()).collect();
        let tables = Arc::new(MeshTables::new(&reg, tints));
        let world = World::new(reg.clone(), gen.clone(), save.clone(), tables);

        let tex = textures::load(&texture_names, &base, &settings.resource_packs);
        let textures_name = tex.packs.first().cloned().unwrap_or_else(|| "Padrão".into());
        let lod = lod::Lod::new(gen.clone(), lod::LodColors::new(&reg, &tex));
        let font = FontAtlas::new();
        let mut renderer = pollster::block_on(Renderer::new(window.clone(), settings.vsync, &tex, &font));
        renderer.cave_cull = !args.no_cave_cull;
        let mut ui = UiBatch::new(&font);
        ui.gui = tex.gui.sprites.clone();
        let audio = Audio::new(&settings, &base);

        // Mobs: a mobs.ron on disk wins, like blocks.
        let mobs_text = std::fs::read_to_string(base.join("packs/vanilla/mobs.ron")).unwrap_or_else(|_| embedded::MOBS_RON.to_string());
        let mob_reg = voxel::mob::MobRegistry::from_ron(&mobs_text).unwrap_or_else(|e| {
            log::error!("{e}; using built-in mobs");
            voxel::mob::MobRegistry::from_ron(embedded::MOBS_RON).unwrap_or_default()
        });
        let painting_defs = paintings::load_defs(&base);
        let paintings = paintings::load(&save.dir, &painting_defs);
        let surv = survival::Survival::load(&save.dir, &items);
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
        let packs = textures::list_resource_packs(&base.join("resourcepacks"));
        let show_debug = args.debug;
        let (shader_packs, pack_errors) = shaderpack::list_packs(&base.join("shaderpacks"));
        for e in &pack_errors {
            log::warn!("shader pack: {e}");
        }

        log::info!("world '{world_name}' seed {seed}, {} blocks, {} textures ({}px)", reg.len(), reg.textures.len(), renderer.texture_size);
        log::info!("{} interface pictures from packs", ui.gui.len());
        let now = Instant::now();
        let mut game = Game {
            window,
            renderer,
            world,
            reg,
            settings,
            base,
            audio,
            args,
            world_name: world_name.clone(),
            seed,
            body,
            yaw,
            pitch,
            spawn_xz: (sx, sz),
            needs_spawn,
            hotbar,
            selected: 0,
            time_of_day,
            screen: if title_mode { Screen::Title } else { Screen::Loading },
            keys: HashSet::new(),
            mouse_held: [false; 3],
            mouse_pos: (0.0, 0.0),
            clicked: false,
            attack_pressed: false,
            ui_view_proj: None,
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
            mining: None,
            particles: particles::Particles::new(),
            anim: player_model::Animator::default(),
            camera_mode,
            world_open: !title_mode,
            title_world: world_name,
            title_spin: 0.0,
            worlds: Vec::new(),
            world_sel: None,
            new_world: menus::NewWorldForm::default(),
            options_back: Screen::Paused,
            mining_tick: 0.0,
            sprinting: false,
            last_w: -10.0,
            crouch: 0.0,
            prone: start_prone,
            prone_t: 0.0,
            fov_kick: 0.0,
            was_in_water: false,
            swim_dist: 0.0,
            items,
            recipes,
            item_base,
            surv,
            rclicked: false,
            mclicked: false,
            rdown: false,
            inv_number_key: None,
            bubble_cd: 4.0,
            ui,
            show_debug,
            show_perf,
            perf: monitor::FpsStats::new(),
            sysmon: monitor::SysMonitor::new(),
            hide_hud: false,
            toast: None,
            packs,
            textures_name,
            shader_packs,
            shader_error: None,
            shader_scroll: 0,
            lod,
            start: now,
            last: now,
            now: 0.0,
            frame_times: Vec::new(),
            fps: 0.0,
            last_frame_info: (0, 0, 0),
            autosave: 0.0,
            loaded_frames: 0,
            backdrop: None,
            want_backdrop: true,
            loading_area: (ChunkPos::new(0, 0), 3),
            screenshot_request: false,
            grabbed: false,
        };
        game.apply_shader();
        game
    }

    /// Turn on the shader pack named in the settings (or none). Errors turn shaders off and
    /// are shown in a toast and in the shader menu; the game keeps running.
    fn apply_shader(&mut self) {
        self.shader_error = None;
        let mut pack = None;
        if !self.settings.shader_pack.is_empty() {
            match shaderpack::find(&self.shader_packs, &self.settings.shader_pack) {
                Some(p) => {
                    self.settings.shader_pack = p.id.clone();
                    pack = Some(p.clone());
                }
                None => self.shader_error = Some(format!("pacote \"{}\" não encontrado", self.settings.shader_pack)),
            }
        }
        if let Err(e) = self.renderer.set_shader(pack.as_ref(), self.settings.shader_quality) {
            self.toast(format!("Erro no shader: {e}"), 8.0);
            self.shader_error = Some(e);
        }
        self.sync_shader_options();
    }

    fn active_shader(&self) -> Option<&ShaderPack> {
        if self.settings.shader_pack.is_empty() {
            return None;
        }
        shaderpack::find(&self.shader_packs, &self.settings.shader_pack)
    }

    fn sync_shader_options(&mut self) {
        let values = self.active_shader().map(|p| p.values(self.settings.shader_options.get(&p.id))).unwrap_or_default();
        self.renderer.post.options = values;
    }

    /// Name for the pause menu and the F3 screen.
    fn shader_name(&self) -> String {
        match (self.active_shader(), &self.shader_error) {
            (_, Some(_)) => format!("{} (erro)", self.settings.shader_label()),
            (Some(p), None) => p.def.name.clone(),
            (None, None) => "Desligado".to_string(),
        }
    }

    /// Terrain shader extras switched on by the built-in "Realçado" pack.
    fn terrain_effects(&self) -> [f32; 4] {
        let o = &self.renderer.post.options;
        match self.renderer.post.builtin() {
            Some(Builtin::Realcado) => [if o[5] > 0.5 { 2.2 } else { 1.0 }, o[6], 1.6 * o[0].min(1.0), 0.0],
            Some(Builtin::Suave) => [1.0, 0.0, 0.8 * o[0], 0.0],
            _ => [1.0, 0.0, 0.0, 0.0],
        }
    }

    fn reload_shader_packs(&mut self) {
        shaderpack::ensure_folder(&self.base.join("shaderpacks"));
        let (packs, errors) = shaderpack::list_packs(&self.base.join("shaderpacks"));
        self.shader_packs = packs;
        self.apply_shader();
        if self.shader_error.is_none() {
            if let Some(e) = errors.first() {
                self.shader_error = Some(e.clone());
            }
        }
        let n = self.shader_packs.iter().filter(|p| matches!(p.source, shaderpack::PackSource::User(_))).count();
        if self.shader_error.is_none() {
            self.toast(format!("Pacotes de shader recarregados ({n} seus)"), 3.0);
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
        if self.screen == Screen::Loading && s != Screen::Loading {
            // Loading frames say nothing about how the game runs.
            self.perf.reset();
        }
        if self.screen == Screen::Inventory && s != Screen::Inventory {
            self.surv_close();
        }
        if s == Screen::Loading && self.screen != Screen::Loading {
            self.want_backdrop = true;
        }
        self.screen = s;
        self.set_grab(s == Screen::Playing);
        self.mouse_held = [false; 3];
    }

    fn toast(&mut self, text: impl Into<String>, secs: f32) {
        self.toast = Some(Toast { text: text.into(), until: self.now + secs });
    }

    fn save_all(&mut self) {
        if !self.world_open {
            self.settings.save(&self.base.join("settings.ron"));
            return;
        }
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
        self.surv.save(&self.world.save.dir, &self.items);
        self.settings.save(&self.base.join("settings.ron"));
    }

    fn situation(&self) -> Situation {
        match self.screen {
            _ if self.is_title_screen() => Situation::Title,
            Screen::Loading => Situation::Title,
            _ if self.body.head_in_water => Situation::Underwater,
            _ => Situation::Creative,
        }
    }

    fn reload_textures(&mut self) {
        let names: Vec<String> = self.reg.textures.iter().chain(self.items.textures.iter()).cloned().chain(extra_textures()).collect();
        let tex = textures::load(&names, &self.base, &self.settings.resource_packs);
        self.textures_name = tex.packs.first().cloned().unwrap_or_else(|| "Padrão".into());
        self.renderer.reload_textures(&tex);
        self.ui.gui = tex.gui.sprites.clone();
        self.lod.set_colors(lod::LodColors::new(&self.reg, &tex));
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
        if self.screen == Screen::CreateWorld && code != KeyCode::Escape {
            self.form_key(code);
            return;
        }
        if self.creative_key(code) {
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
                Screen::Controls => self.set_screen(self.options_back),
                Screen::Shaders => self.set_screen(Screen::Paused),
                Screen::Paintings => self.set_screen(Screen::Playing),
                Screen::Worlds | Screen::CreateWorld | Screen::Multiplayer | Screen::Options | Screen::Packs => self.menu_back(),
                Screen::Loading | Screen::Dead | Screen::Title => {}
            },
            KeyCode::KeyE => match self.screen {
                Screen::Playing => self.surv_open_inventory(),
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
            KeyCode::F9 => {
                self.show_perf = !self.show_perf;
                self.sysmon.set_active(self.show_perf);
                self.perf.reset();
            }
            KeyCode::F1 => self.hide_hud = !self.hide_hud,
            KeyCode::F2 => self.screenshot_request = true,
            KeyCode::F5 => self.camera_mode = (self.camera_mode + 1) % 3,
            KeyCode::KeyZ if self.screen == Screen::Playing => self.prone = !self.prone,
            KeyCode::KeyR if self.screen == Screen::Inventory && self.survival => self.surv.want_recipe = true,
            KeyCode::KeyQ if self.screen == Screen::Playing => {
                let all = self.keys.contains(&KeyCode::ControlLeft) || self.keys.contains(&KeyCode::ControlRight);
                self.surv_throw(all);
            }
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
            KeyCode::KeyW if self.screen == Screen::Playing => {
                if self.now - self.last_w < 0.3 {
                    self.sprinting = true;
                }
                self.last_w = self.now;
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
                self.inv_number_key = Some(i);
            } else {
                self.selected = i;
                self.show_held_name();
            }
        }
    }

    fn show_held_name(&mut self) {
        if let Some(st) = self.held_stack() {
            let n = self.items.get(st.item).display_name.clone();
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
            Screen::Shaders => {
                if lines > 0.0 {
                    self.shader_scroll = self.shader_scroll.saturating_sub(1);
                } else {
                    self.shader_scroll += 1;
                }
            }
            Screen::Inventory if self.creative_screen_open() => self.creative_scroll(lines > 0.0),
            Screen::Inventory => {
                if lines > 0.0 {
                    self.surv.book_scroll = self.surv.book_scroll.saturating_sub(1);
                } else {
                    self.surv.book_scroll += 1;
                }
            }
            _ => {}
        }
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
            self.loading_area = (c, near);
            let mut ready = true;
            for dz in -near..=near {
                for dx in -near..=near {
                    // Chunks load in a circle, so only wait for the ones inside it.
                    if dx * dx + dz * dz <= near * near && !self.world.is_meshed(c + ChunkPos::new(dx, dz)) {
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
                if self.args.shapes {
                    self.build_shapes();
                }
                if self.args.mobs {
                    self.spawn_demo_mobs();
                }
                self.fill_creative_hotbar();
                if let Some(c) = self.args.ctab {
                    self.surv.ctab = c.min(12);
                }
                if self.args.kit {
                    self.test_kit();
                }
                if self.args.die {
                    // Test hook: die where you stand and step back to look at the body.
                    self.surv_died();
                    let f = self.forward();
                    self.body.pos -= Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 2.5;
                }
                if let Some(i) = self.args.slot {
                    self.selected = i;
                }
                if self.args.particles {
                    // Frozen in mid-air for a test picture: a broken block and a splash.
                    let f = self.forward();
                    let at = (self.body.eye() + Vec3::new(f.x, 0.0, f.z).normalize_or_zero() * 3.0).floor().as_ivec3();
                    let id = self.reg.id_or_air("grass_block");
                    let (layer, tint) = self.block_look(id);
                    self.particles.block_bits(at, layer, tint, 18, 1.0);
                    for _ in 0..5 {
                        self.particles.update(&self.world, 0.03);
                    }
                    let stone = self.reg.id_or_air("stone");
                    let (layer, tint) = self.block_look(stone);
                    self.particles.block_bits(at + IVec3::new(1, 0, 0), layer, tint, 18, 1.0);
                    for _ in 0..3 {
                        self.particles.update(&self.world, 0.03);
                    }
                }
                if self.args.show_inventory || self.args.open.as_deref().is_some_and(|o| o != "corpse") {
                    self.set_screen(Screen::Inventory);
                } else if self.args.show_pause {
                    self.set_screen(Screen::Paused);
                } else if self.args.show_shaders {
                    self.set_screen(Screen::Shaders);
                } else {
                    self.set_screen(Screen::Playing);
                }
                self.toast("Bem-vindo! Esc = menu, E = inventário", 6.0);
            }
        }

        // Singleplayer: the pause menu (and the option screens opened from it) stop the world.
        let paused = self.world_open && matches!(self.screen, Screen::Paused | Screen::Options | Screen::Controls | Screen::Packs | Screen::Shaders);
        if !paused {
            self.time_of_day = (self.time_of_day + dt * 20.0) % 24000.0;
        }
        if self.args.time.is_some() && self.args.screenshot.is_some() {
            self.time_of_day = self.args.time.unwrap();
        }

        if self.is_title_screen() {
            self.update_title(dt);
        }
        if self.args.screenshot.is_none() && !paused {
            self.particles.update(&self.world, dt);
        }
        if self.screen == Screen::Playing || self.screen == Screen::Inventory {
            self.update_player(dt);
            self.update_survival(dt);
        }
        if self.world_open && !paused && self.screen != Screen::Loading {
            let use_held = self.mouse_held[1] || self.pad_held.contains(&Action::Use);
            self.surv_update(if self.args.screenshot.is_some() { 0.0 } else { dt }, use_held);
        }
        if self.world_open && !matches!(self.screen, Screen::Loading | Screen::Paused | Screen::Controls | Screen::Options | Screen::Packs | Screen::Shaders) {
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
            if self.survival {
                self.surv_mob_deaths(&out.deaths);
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
        // Lying down: the box drops to 0.6 high so the player fits through one-block gaps. Getting
        // up waits until there is room, so a tunnel keeps you crawling.
        if self.body.flying {
            self.prone = false;
        }
        if self.prone {
            self.body.height = 0.6;
        } else if self.body.height < voxel::physics::PLAYER_HEIGHT && self.body.fits(&self.world, voxel::physics::PLAYER_HEIGHT) {
            self.body.height = voxel::physics::PLAYER_HEIGHT;
        }
        let crawling = self.body.height < 1.0;
        let input = Input {
            wish,
            jump: k(KeyCode::Space) || held(Action::Jump),
            sneak: (k(KeyCode::ShiftLeft) || k(KeyCode::ShiftRight) || held(Action::Sneak)) && !crawling,
            sprint: false,
            crawl: crawling,
        };
        // Sprint: Ctrl, double-tap W, or the pad button; it lasts while moving forward.
        let sneaking = input.sneak && !self.body.flying;
        if k(KeyCode::ControlLeft) || k(KeyCode::ControlRight) || held(Action::Sprint) {
            self.sprinting = true;
        }
        let forward_amount = wish.dot(fwd);
        if forward_amount < 0.3 || sneaking || crawling || (!k(KeyCode::KeyW) && !(playing && self.pads.pad.connected && self.pads.pad.ly > 0.3)) {
            self.sprinting = false;
        }
        if self.survival && self.surv.hunger <= 6.0 {
            self.sprinting = false;
        }
        let input = Input { sprint: self.sprinting, ..input };
        let ease = (dt * 10.0).min(1.0);
        self.crouch += ((if sneaking { 1.0 } else { 0.0 }) - self.crouch) * ease;
        self.prone_t += ((if crawling { 1.0 } else { 0.0 }) - self.prone_t) * (dt * 6.0).min(1.0);
        let moving = (self.body.vel * Vec3::new(1.0, 0.0, 1.0)).length() > 1.0;
        self.fov_kick += ((if self.sprinting && moving { 1.0 } else { 0.0 }) - self.fov_kick) * ease * 0.8;
        let before = self.body.pos;
        let was_ground = self.body.on_ground;
        // The camera pass for screenshots holds still.
        if self.args.screenshot.is_none() {
            self.body.step(&self.world, &input, dt);
        } else {
            // Held still, but still know about water for the picture.
            self.body.head_in_water = voxel::physics::Terrain::liquid(&self.world, self.body.eye().floor().as_ivec3());
            self.body.in_water = self.body.head_in_water || voxel::physics::Terrain::liquid(&self.world, self.body.pos.floor().as_ivec3());
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

        let lava = self.reg.id_or_air("lava");
        let feet = self.body.pos.floor().as_ivec3();
        let pose = self.pose(lava != AIR && (self.world.block_or_air(feet) == lava || self.world.block_or_air(feet + IVec3::Y) == lava));
        self.anim.update(&pose, dt);

        // Water: splash going in, strokes while swimming, drips coming out, bubbles below.
        let in_water = self.body.in_water;
        if in_water && !self.was_in_water {
            let strength = (-self.body.vel.y / 14.0).clamp(0.25, 1.0);
            self.sfx(Sfx::Splash(strength));
            let water = self.reg.id_or_air("water");
            let mut top = self.body.pos;
            // Find the surface so the drops start on the water, not under it.
            while self.world.is_liquid(self.world.block_or_air((top + Vec3::Y).floor().as_ivec3())) && top.y < self.body.pos.y + 3.0 {
                top.y += 1.0;
            }
            top.y = top.y.floor() + 0.9;
            self.particles.splash(top, self.reg.get(water).faces[2], strength);
            self.swim_dist = 0.0;
        } else if !in_water && self.was_in_water && self.body.vel.y > -1.0 {
            self.sfx(Sfx::WaterExit);
        }
        self.was_in_water = in_water;
        if in_water && !self.body.flying {
            self.swim_dist += (self.body.pos - before).length();
            if self.swim_dist > 2.2 {
                self.swim_dist = 0.0;
                self.sfx(if self.body.head_in_water { Sfx::Bubbles } else { Sfx::Swim });
                let water = self.reg.get(self.reg.id_or_air("water")).faces[2];
                if self.body.head_in_water {
                    self.particles.bubbles(self.body.eye(), water, 3);
                } else {
                    self.particles.splash(self.body.pos + Vec3::Y * 0.8, water, 0.15);
                }
            }
        }
        if self.body.head_in_water {
            self.bubble_cd -= dt;
            if self.bubble_cd <= 0.0 {
                self.bubble_cd = 3.0 + (self.now * 7.3).sin().abs() * 4.0;
                self.sfx(Sfx::Bubbles);
                let water = self.reg.get(self.reg.id_or_air("water")).faces[2];
                self.particles.bubbles(self.body.eye() + self.forward() * 0.3, water, 5);
            }
        }

        // What the player is looking at.
        let world = &self.world;
        let (eye, fwd) = (self.body.eye(), self.forward());
        self.target = raycast(eye, fwd, REACH, |p| match world.block(p) {
            Some(id) => id != AIR && !world.is_liquid(id) && (!self.reg.get(id).shape.is_boxes() || world.ray_shape(eye, fwd, p).is_some()),
            None => false,
        });
        // Shaped blocks: the face actually hit (the top of a slab, the side of a stair step).
        if let Some(h) = self.target.as_mut() {
            if let Some((d, n)) = world.ray_shape(eye, fwd, h.block) {
                h.distance = d;
                h.normal = n;
            }
        }

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
        // Survival swings land when the weapon is ready again, or on a fresh click (weaker).
        let ready = !self.survival || self.attack_pressed || self.attack_charge() >= 1.0;
        if attack && self.attack_cd <= 0.0 && ready {
            // Plants don't block a swing: the hit goes through grass and flowers to the mob.
            let world = &self.world;
            let reg = &self.reg;
            let block_dist = raycast(self.body.eye(), self.forward(), REACH, |p| match world.block(p) {
                Some(id) => id != AIR && !world.is_liquid(id) && !matches!(reg.get(id).shape, Shape::Cross),
                None => false,
            })
            .map(|t| t.distance)
            .unwrap_or(f32::MAX);
            if let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), 3.5) {
                if t < block_dist {
                    self.attack_cd = if self.survival { 0.1 } else { 0.45 };
                    self.break_cd = 0.3;
                    let eye = self.body.eye();
                    let charge = if self.survival { self.attack_charge() } else { 1.0 };
                    let base = if self.survival { self.surv_attack_damage() } else { 4.0 };
                    // Falling onto a mob with a ready weapon is a critical hit.
                    let crit = self.body.vel.y < -0.5 && !self.body.on_ground && !self.body.in_water && charge > 0.9;
                    let dmg = if crit { base * 1.5 } else { base };
                    let sprint_hit = self.sprinting && charge > 0.9;
                    if crit || sprint_hit {
                        let at = self.mobs.list[i].body.pos + Vec3::Y * 0.9;
                        let wool = self.reg.id_or_air("white_wool");
                        if wool != AIR && crit {
                            let (layer, _) = self.block_look(wool);
                            self.particles.block_bits(at.floor().as_ivec3(), layer, [1.0, 0.95, 0.7], 10, 0.6);
                        }
                    }
                    let m = &mut self.mobs.list[i];
                    m.hurt(dmg, eye);
                    if sprint_hit {
                        // A running hit knocks the mob further and ends the sprint.
                        let dir = m.body.pos - eye;
                        m.body.knockback(dir, 4.0);
                    }
                    let def = &self.mobs.reg.defs[m.def];
                    self.last_hit = Some((def.display_name.clone(), m.health.max(0.0), m.max_health, self.now + 3.0));
                    self.sfx(Sfx::MobHit);
                    self.anim.start_swing();
                    self.pads.rumble(0.3, 0.6, 0.1);
                    if sprint_hit {
                        self.sprinting = false;
                    }
                }
            }
        }
        if attack && self.survival && self.attack_pressed && self.target.is_none() {
            // A swing at the air still uses up the weapon's readiness.
            self.surv.attack_t = 0.0;
            self.anim.start_swing();
        }
        self.attack_pressed = false;
        // Survival: hold to dig, with a knock every few moments, like the classic game.
        if self.survival {
            let target = self.target.map(|t| t.block);
            match (attack && self.break_cd <= 0.0, target) {
                (true, Some(b)) => {
                    let id = self.world.block_or_air(b);
                    let progress = match self.mining {
                        Some((p, t)) if p == b => t + dt,
                        _ => {
                            self.mining_tick = 0.0;
                            0.0
                        }
                    };
                    self.mining = Some((b, progress));
                    self.mining_tick -= dt;
                    if self.mining_tick <= 0.0 && id != AIR {
                        self.mining_tick = 0.24;
                        self.anim.start_swing();
                        self.sfx(Sfx::Hit(Material::of(&self.reg.get(id).name)));
                        let (layer, tint) = self.block_look(id);
                        self.particles.block_bits(b, layer, tint, 3, 0.6);
                    }
                }
                _ => self.mining = None,
            }
        }
        let dig_done = match self.mining {
            Some((b, t)) => {
                let id = self.world.block_or_air(b);
                t >= self.dig_time(id)
            }
            None => false,
        };
        if attack && self.break_cd <= 0.0 && (!self.survival || dig_done) {
            if let Some(hit) = self.target {
                let id = self.world.block_or_air(hit.block);
                if id != AIR && self.reg.get(id).hardness >= 0.0 && self.world.break_block(hit.block) {
                    self.mining = None;
                    if self.survival {
                        self.surv_block_broken(hit.block, id);
                    }
                    let (layer, tint) = self.block_look(id);
                    self.particles.block_bits(hit.block, layer, tint, 18, 1.0);
                    self.anim.start_swing();
                    self.break_cd = if self.survival { 0.25 } else { 0.22 };
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
            if self.crouch <= 0.5 && self.toggle_shaped() {
                self.place_cd = 0.25;
            } else if self.surv_use() {
                self.place_cd = self.place_cd.max(0.22);
            }
        }
        if self.mouse_held[2] {
            self.mouse_held[2] = false;
            if let Some(hit) = self.target {
                let id = self.world.block_or_air(hit.block);
                if id != AIR && self.survival {
                    self.surv_pick(id);
                    self.show_held_name();
                } else if id != AIR {
                    self.creative_pick(id);
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

    /// Seconds to dig a block by hand in survival (no tools yet).
    fn dig_time(&self, id: BlockId) -> f32 {
        if self.survival {
            return self.items.dig_time(&self.reg, id, self.held_tool().as_ref());
        }
        let h = self.reg.get(id).hardness.max(0.0);
        if h == 0.0 {
            0.05
        } else {
            0.15 + h.min(10.0) * 0.6
        }
    }

    /// Rows of the shader menu, left panel then right panel: (label, action, right panel).
    fn shader_rows(&self) -> Vec<(String, ShaderRow, bool)> {
        let cur = if self.settings.shader_pack.is_empty() { None } else { self.active_shader().map(|p| p.id.clone()) };
        let mark = |on: bool, name: &str| if on { format!("> {name} <") } else { name.to_string() };
        let mut v = vec![
            ("Voltar".to_string(), ShaderRow::Back, false),
            (format!("Qualidade: {}", self.settings.shader_quality.label()), ShaderRow::Quality, false),
            ("Recarregar pacotes".to_string(), ShaderRow::Reload, false),
            (
                match self.settings.lod_distance {
                    0 => "Distância LOD: Não".to_string(),
                    d => format!("Distância LOD: +{d}"),
                },
                ShaderRow::Lod,
                false,
            ),
            (mark(cur.is_none(), "Desligado"), ShaderRow::Pack(None), false),
        ];
        for (i, p) in self.shader_packs.iter().enumerate() {
            let user = matches!(p.source, shaderpack::PackSource::User(_));
            let name = if user { format!("{} (seu)", p.def.name) } else { p.def.name.clone() };
            v.push((mark(cur.as_deref() == Some(p.id.as_str()), &name), ShaderRow::Pack(Some(i)), false));
        }
        if let Some(p) = self.active_shader() {
            let vals = p.values(self.settings.shader_options.get(&p.id));
            for (i, o) in p.def.options.iter().take(shaderpack::MAX_OPTIONS).enumerate() {
                v.push((format!("{}: {}", o.label(), o.display(vals[i])), ShaderRow::Opt(i), true));
            }
        }
        v
    }

    /// Shader menu: packs on the left, options of the pack in use on the right, and the world
    /// visible in between so changes can be judged right away.
    fn draw_shaders_menu(&mut self, w: f32, h: f32, mouse: (f32, f32)) -> Option<usize> {
        let s = self.ui.scale;
        let rows = self.shader_rows();
        let pw = (170.0 * s).min(w * 0.42);
        let bh = 14.0 * s;
        let step = bh + 2.0 * s;
        let top = 22.0 * s;
        self.ui.rect(0.0, 0.0, pw + 12.0 * s, h, [0, 0, 0, 150]);
        self.ui.text(6.0 * s, 8.0 * s, "Shaders (Esc volta)", WHITE);
        let fits = (((h - top - 40.0 * s) / step).floor() as usize).max(4);
        let left: Vec<usize> = (0..rows.len()).filter(|&i| !rows[i].2).collect();
        let right: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].2).collect();
        // The first five rows always show; the pack list scrolls with the wheel.
        let fixed = 5.min(left.len());
        let scroll_max = left.len().saturating_sub(fits);
        self.shader_scroll = self.shader_scroll.min(scroll_max);
        let mut hit = None;
        let mut y = top;
        for (k, &i) in left.iter().enumerate() {
            if k >= fixed && k < fixed + self.shader_scroll {
                continue;
            }
            if y + bh > h - 30.0 * s {
                break;
            }
            if self.ui.button(6.0 * s, y, pw, bh, &rows[i].0, mouse) && self.clicked {
                hit = Some(i);
            }
            y += step;
        }
        if scroll_max > 0 {
            self.ui.text_scaled(6.0 * s, y, "roda do mouse: mais pacotes", GRAY, (s / 2.0).floor().max(1.0));
            y += 8.0 * s;
        }
        // Description of the pack in use, or why it is not running.
        let hs = (s / 2.0).floor().max(1.0);
        let note = match (&self.shader_error, self.active_shader()) {
            (Some(e), _) => Some((format!("Erro: {e}"), [255, 120, 100, 255])),
            (None, Some(p)) if !p.def.description.is_empty() => Some((p.def.description.clone(), GRAY)),
            _ => None,
        };
        if let Some((text, color)) = note {
            // Wrap to the panel width.
            let mut line = String::new();
            let mut ly = y + 2.0 * s;
            for word in text.split(' ') {
                let try_line = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if self.ui.text_width(&try_line, hs) > pw && !line.is_empty() {
                    self.ui.text_scaled(6.0 * s, ly, &line, color, hs);
                    ly += 9.0 * hs;
                    line = word.to_string();
                } else {
                    line = try_line;
                }
            }
            self.ui.text_scaled(6.0 * s, ly, &line, color, hs);
        }
        if !right.is_empty() {
            let rw = (170.0 * s).min(w * 0.42);
            let rx = w - rw - 6.0 * s;
            self.ui.rect(rx - 6.0 * s, 0.0, rw + 12.0 * s, top + right.len() as f32 * step + 4.0 * s, [0, 0, 0, 150]);
            self.ui.text(rx, 8.0 * s, "Opções do pacote", WHITE);
            for (k, &i) in right.iter().enumerate() {
                if self.ui.button(rx, top + k as f32 * step, rw, bh, &rows[i].0, mouse) && self.clicked {
                    hit = Some(i);
                }
            }
        }
        hit
    }

    fn shaders_button(&mut self, i: usize) {
        if let Some(a) = self.audio.as_mut() {
            a.play(Sfx::Click, self.settings.sound_volume);
        }
        let Some(&(_, row, _)) = self.shader_rows().get(i) else { return };
        match row {
            ShaderRow::Back => self.set_screen(Screen::Paused),
            ShaderRow::Quality => {
                self.settings.shader_quality = match self.settings.shader_quality {
                    ShaderQuality::Fast => ShaderQuality::Fancy,
                    ShaderQuality::Fancy => ShaderQuality::Fast,
                };
                self.apply_shader();
            }
            ShaderRow::Reload => self.reload_shader_packs(),
            ShaderRow::Lod => {
                let d = &lod::DISTANCES;
                let cur = d.iter().position(|&x| x >= self.settings.lod_distance).unwrap_or(0);
                self.settings.lod_distance = d[(cur + 1) % d.len()];
            }
            ShaderRow::Pack(p) => {
                self.settings.shader_pack = p.map(|p| self.shader_packs[p].id.clone()).unwrap_or_default();
                self.apply_shader();
            }
            ShaderRow::Opt(k) => {
                if let Some(p) = self.active_shader().cloned() {
                    let o = &p.def.options[k];
                    let cur = p.values(self.settings.shader_options.get(&p.id))[k];
                    self.settings.shader_options.entry(p.id.clone()).or_default().insert(o.name().to_string(), o.next(cur));
                    self.sync_shader_options();
                }
            }
        }
        self.settings.save(&self.base.join("settings.ron"));
    }

    fn pose(&self, burning: bool) -> player_model::Pose {
        player_model::Pose {
            pos: self.body.pos,
            vel: self.body.vel,
            look_yaw: self.yaw,
            look_pitch: self.pitch,
            on_ground: self.body.on_ground,
            in_water: self.body.in_water,
            head_in_water: self.body.head_in_water,
            flying: self.body.flying,
            sneaking: self.crouch,
            crawling: self.prone_t,
            sprinting: self.sprinting,
            burning,
            hurt: self.survival && self.hurt_age < 0.3,
        }
    }

    /// Texture layer and tint for particles of a block.
    fn block_look(&self, id: BlockId) -> (u16, [f32; 3]) {
        let b = self.reg.get(id);
        let tint = if b.tint { [0.55, 0.8, 0.4] } else { [1.0, 1.0, 1.0] };
        (b.faces[0], tint)
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
        // Armor softens hits from something (mobs, blasts), not falls, drowning or hunger.
        let amount = if from.is_some() { self.armor_absorb(amount) } else { amount };
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
            self.prone = false;
            self.sfx(Sfx::Death);
            self.set_screen(Screen::Dead);
            self.surv_died();
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
        // Healing comes from a full stomach (survival.rs).
    }

    /// A health bar floating over each mob that was hurt lately, with its name.
    fn draw_mob_health_bars(&mut self, w: f32, h: f32) {
        let Some((vp, cam_block)) = self.ui_view_proj else { return };
        let s = self.ui.scale;
        let eye = self.body.eye();
        for m in &self.mobs.list {
            if !m.alive() || m.hurt_age > 5.0 {
                continue;
            }
            let def = &self.mobs.reg.defs[m.def];
            let top = m.body.pos + Vec3::Y * (def.height + 0.45);
            let dist = (top - eye).length();
            if dist > 24.0 {
                continue;
            }
            let c = vp * (top - cam_block.as_vec3()).extend(1.0);
            if c.w < 0.1 {
                continue;
            }
            let (nx, ny) = (c.x / c.w, c.y / c.w);
            if nx.abs() > 1.1 || ny.abs() > 1.1 {
                continue;
            }
            let (x, y) = ((nx * 0.5 + 0.5) * w, (0.5 - ny * 0.5) * h);
            let k = (6.0 / dist.max(1.0)).clamp(0.35, 1.0);
            let bw = (30.0 * s * k).round();
            let bh = (3.0 * s * k).max(s).round();
            let frac = (m.health / m.max_health).clamp(0.0, 1.0);
            self.ui.rect(x - bw / 2.0 - s, y - s, bw + 2.0 * s, bh + 2.0 * s, [0, 0, 0, 170]);
            self.ui.rect(x - bw / 2.0, y, bw, bh, [70, 15, 15, 230]);
            let col = if frac > 0.5 { [60, 200, 60, 255] } else if frac > 0.25 { [230, 190, 40, 255] } else { [220, 40, 40, 255] };
            self.ui.rect(x - bw / 2.0, y, bw * frac, bh, col);
            if k > 0.6 {
                let label = format!("{} {:.0}/{:.0}", def.display_name, m.health.max(0.0).ceil(), m.max_health);
                let tw = self.ui.text_width(&label, s);
                self.ui.text(x - tw / 2.0, y - 9.0 * s, &label, WHITE);
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
                    if self.world.break_block(p) && (dx + dy + dz) % 3 == 0 {
                        let (layer, tint) = self.block_look(id);
                        self.particles.block_bits(p, layer, tint, 3, 2.0);
                    }
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
        self.surv_respawned();
        self.fall_peak = self.body.pos.y;
        self.sfx(Sfx::Respawn);
        self.set_screen(Screen::Playing);
    }

    /// Test scene: one of every mob in a row in front of the player.
    fn spawn_demo_mobs(&mut self) {
        let f = self.forward();
        let fwd = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-fwd.z, 0.0, fwd.x);
        let names: Vec<String> = self.mobs.reg.defs.iter().filter(|d| d.spawn != voxel::mob::SpawnRule::Never).map(|d| d.name.clone()).collect();
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

    /// Place a block where the player is aiming. Returns where it went.
    fn place_block_as(&mut self, id: BlockId) -> Option<IVec3> {
        let hit = self.target?;
        let clicked = self.world.block_or_air(hit.block);
        let shape = self.reg.get(id).shape;
        // A half slab clicked on its open side becomes a double slab.
        let cb = self.reg.get(clicked);
        if shape == Shape::Slab && cb.base == id && ((cb.variant == 0 && hit.normal == IVec3::Y) || (cb.variant == 1 && hit.normal == -IVec3::Y)) {
            let double = self.variant_id(id, 2);
            if self.body.intersects_block(hit.block) || !self.world.set_block(hit.block, double) {
                return None;
            }
            return self.placed(hit.block, id);
        }
        // Plants are replaced instead of placed against.
        let pos = if cb.shape == Shape::Cross && clicked != id { hit.block } else { hit.block + hit.normal };
        let cur = self.world.block_or_air(pos);
        if cur != AIR && !self.world.is_liquid(cur) && self.reg.get(cur).shape != Shape::Cross {
            return None;
        }
        let b = self.reg.get(id);
        if b.solid && !matches!(b.shape, Shape::Cross | Shape::Torch) && self.body.intersects_block(pos) {
            return None;
        }
        // Orientation from where the player looks and which half of the face was clicked.
        let f = self.forward();
        let facing = |x: f32, z: f32| if x.abs() > z.abs() { if x > 0.0 { 1 } else { 3 } } else if z > 0.0 { 2 } else { 0 };
        let at = self.body.eye() + f * hit.distance;
        let upper = if hit.normal.y != 0 { hit.normal.y < 0 } else { at.y - pos.y as f32 > 0.5 };
        let v = match shape {
            Shape::Slab => upper as u8,
            Shape::Stairs => facing(f.x, f.z) | if upper { 4 } else { 0 },
            Shape::Door => facing(f.x, f.z),
            Shape::Trapdoor => {
                let side = if hit.normal.y == 0 { facing(-hit.normal.x as f32, -hit.normal.z as f32) ^ 2 } else { facing(-f.x, -f.z) };
                side | if upper { 8 } else { 0 }
            }
            _ => 0,
        };
        if shape == Shape::Door {
            let above = pos + IVec3::Y;
            if self.world.block_or_air(above) != AIR || self.body.intersects_block(above) {
                return None;
            }
            let (lower, top) = (self.variant_id(id, v), self.variant_id(id, v | 8));
            if self.world.set_block(pos, lower) {
                self.world.set_block(above, top);
                return self.placed(pos, id);
            }
            return None;
        }
        let id_v = self.variant_id(id, v);
        // Plants and torches need something under them.
        if matches!(b.shape, Shape::Cross | Shape::Torch) {
            let under = self.world.block_or_air(pos - IVec3::Y);
            if under == AIR || !self.reg.get(under).solid {
                return None;
            }
        }
        if self.world.place_block(pos, id_v) {
            return self.placed(pos, id);
        }
        None
    }

    /// The id of an orientation of a shaped block (see `voxel::shapes`).
    fn variant_id(&self, id: BlockId, v: u8) -> BlockId {
        if v == 0 {
            return id;
        }
        self.reg.id(&format!("{}:{}", self.reg.get(id).name, v)).unwrap_or(id)
    }

    /// Right click on a door or trapdoor opens or closes it. Returns true when it did.
    fn toggle_shaped(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        let id = self.world.block_or_air(hit.block);
        let b = self.reg.get(id);
        if !matches!(b.shape, Shape::Door | Shape::Trapdoor) {
            return false;
        }
        let (base, v, door) = (b.base, b.variant ^ 4, b.shape == Shape::Door);
        let new = self.variant_id(base, v);
        self.world.set_block(hit.block, new);
        if door {
            let other = if v & 8 != 0 { hit.block - IVec3::Y } else { hit.block + IVec3::Y };
            if self.reg.get(self.world.block_or_air(other)).base == base {
                let ov = self.variant_id(base, v ^ 8);
                self.world.set_block(other, ov);
            }
        }
        self.anim.start_swing();
        self.sfx(Sfx::Place(Material::Wood));
        true
    }

    /// Effects after a block went down.
    fn placed(&mut self, pos: IVec3, id: BlockId) -> Option<IVec3> {
        self.place_cd = 0.22;
        self.anim.start_swing();
        self.pads.rumble(0.0, 0.35, 0.05);
        let m = Material::of(&self.reg.get(id).name);
        let vol = self.settings.sound_volume;
        if let Some(a) = self.audio.as_mut() {
            a.play(Sfx::Place(m), vol);
        }
        Some(pos)
    }

    /// Test scene: every block shape in a few orientations, on a stone floor in front of the player.
    fn build_shapes(&mut self) {
        let p = self.body.pos.floor().as_ivec3();
        let g = |n: &str| self.reg.id_or_air(n);
        let floor = g("stone");
        for x in -6..=6 {
            for z in 2..=12 {
                self.world.set_block(IVec3::new(p.x + x, p.y - 1, p.z - z), floor);
                for y in 0..4 {
                    self.world.set_block(IVec3::new(p.x + x, p.y + y, p.z - z), AIR);
                }
            }
        }
        let put = |w: &mut World, x: i32, y: i32, z: i32, name: &str| {
            w.set_block(IVec3::new(p.x + x, p.y + y, p.z - z - 2), self.reg.id_or_air(name));
        };
        let w = &mut self.world;
        // Stairs facing each way, and upside down.
        for (i, v) in ["", ":1", ":2", ":3", ":4", ":5"].iter().enumerate() {
            put(w, -5 + i as i32, 0, 3, &format!("oak_stairs{v}"));
        }
        put(w, 2, 0, 3, "cobblestone_stairs:2");
        put(w, 3, 0, 3, "stone_brick_stairs:3");
        // Slabs: bottom, top, double.
        put(w, 4, 0, 3, "stone_slab");
        put(w, 5, 0, 3, "stone_slab:1");
        put(w, 6, 0, 3, "stone_slab:2");
        // A fence corner, a wall and a pane line.
        for x in -5..=-2 {
            put(w, x, 0, 6, "oak_fence");
        }
        put(w, -2, 0, 7, "oak_fence");
        for x in -1..=2 {
            put(w, x, 0, 6, "cobblestone_wall");
        }
        put(w, 2, 0, 7, "cobblestone_wall");
        for x in 3..=6 {
            put(w, x, 0, 6, "glass_pane");
            put(w, x, 1, 6, "glass_pane");
        }
        // Doors: closed and open, and trapdoors.
        put(w, -4, 0, 9, "oak_door:2");
        put(w, -4, 1, 9, "oak_door:10");
        put(w, -2, 0, 9, "oak_door:6");
        put(w, -2, 1, 9, "oak_door:14");
        put(w, 0, 0, 9, "oak_trapdoor");
        put(w, 1, 0, 9, "oak_trapdoor:8");
        put(w, 2, 0, 9, "oak_trapdoor:6");
        put(w, 4, 0, 9, "oak_planks");
        put(w, 4, 1, 9, "oak_planks");
        // Look from above.
        self.body.pos.y += 3.5;
        self.body.flying = true;
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

    /// F9: frame rate and machine usage, top right.
    fn draw_perf(&mut self) {
        self.sysmon.set_active(true);
        let w = self.renderer.config.width as f32;
        // A size smaller than the HUD so the panel doesn't cover the game.
        let gui = self.ui.scale;
        let s = (gui - 1.0).max(1.0);
        self.ui.scale = s;
        let f = self.perf.summary();
        let st = self.sysmon.stats();
        const GOOD: [u8; 4] = [120, 230, 120, 255];
        const MID: [u8; 4] = [240, 210, 90, 255];
        const BAD: [u8; 4] = [240, 100, 90, 255];
        const LABEL: [u8; 4] = [200, 200, 200, 255];
        let fps_col = |v: f32| if v >= 55.0 { GOOD } else if v >= 30.0 { MID } else { BAD };
        let pct_col = |v: f32| if v < 70.0 { GOOD } else if v < 90.0 { MID } else { BAD };
        let temp_col = |v: f32| if v < 70.0 { GOOD } else if v < 85.0 { MID } else { BAD };
        let nd = || "n/d".to_string();
        let mut rows: Vec<(String, String, [u8; 4])> = vec![
            ("FPS atual".into(), format!("{:.0} ({:.1} ms)", f.current, f.frame_ms), fps_col(f.current)),
            ("FPS médio".into(), format!("{:.0}", f.average), fps_col(f.average)),
            ("FPS alto".into(), format!("{:.0}", f.high), fps_col(f.high)),
            ("FPS 1% mínimo".into(), format!("{:.0} (pior {:.1} ms)", f.low1, f.worst_ms), fps_col(f.low1)),
        ];
        if !st.ready {
            rows.push(("Sistema".into(), "medindo...".into(), LABEL));
        } else {
            rows.push(("CPU total".into(), format!("{:.0}% ({} núcleos)", st.cpu_total, st.cores), pct_col(st.cpu_total)));
            rows.push(("CPU do jogo".into(), format!("{:.0}%", st.cpu_game), pct_col(st.cpu_game)));
            rows.push(("CPU temp.".into(), st.cpu_temp.map(|t| format!("{t:.0} °C")).unwrap_or_else(nd), st.cpu_temp.map(temp_col).unwrap_or(LABEL)));
            let ram_pct = (st.ram_used_gb / st.ram_total_gb.max(0.001) * 100.0) as f32;
            rows.push(("RAM do jogo".into(), format!("{:.2} GB", st.ram_game_gb), GOOD));
            rows.push(("RAM do PC".into(), format!("{:.1} / {:.1} GB ({:.0}%)", st.ram_used_gb, st.ram_total_gb, ram_pct), pct_col(ram_pct)));
            rows.push(("GPU uso".into(), st.gpu_usage.map(|u| format!("{u:.0}%")).unwrap_or_else(nd), st.gpu_usage.map(pct_col).unwrap_or(LABEL)));
            rows.push(("GPU temp.".into(), st.gpu_temp.map(|t| format!("{t:.0} °C")).unwrap_or_else(nd), st.gpu_temp.map(temp_col).unwrap_or(LABEL)));
            let vram = match (st.vram_used_gb, st.vram_total_gb) {
                (Some(u), Some(t)) => format!("{u:.2} / {t:.1} GB"),
                (Some(u), None) => format!("{u:.2} GB"),
                _ => format!("{:.2} GB (só o jogo)", self.renderer.vram_bytes as f64 / 1e9),
            };
            rows.push(("VRAM".into(), vram, GOOD));
            rows.push(("Disco do jogo".into(), format!("ler {:.1}  gravar {:.1} MB/s", st.game_read_mbs, st.game_write_mbs), GOOD));
            let disk = match (st.disk_read_mbs, st.disk_write_mbs) {
                (Some(r), Some(w)) => format!("ler {r:.1}  gravar {w:.1} MB/s"),
                _ => nd(),
            };
            rows.push(("Disco do PC".into(), disk, if st.disk_read_mbs.is_some() { GOOD } else { LABEL }));
        }
        let gpu_name = self.renderer.adapter_info.name.clone();
        let label_w = rows.iter().map(|r| self.ui.text_width(&r.0, s)).fold(0.0, f32::max) + 8.0 * s;
        let value_w = rows.iter().map(|r| self.ui.text_width(&r.1, s)).fold(0.0, f32::max);
        let graph_h = 30.0 * s;
        let pw = (label_w + value_w).max(self.ui.text_width(&gpu_name, s)).max(150.0 * s) + 8.0 * s;
        let ph = (rows.len() as f32 + 2.0) * 10.0 * s + graph_h + 12.0 * s;
        let x = w - pw - 2.0 * s;
        let mut y = 2.0 * s;
        self.ui.rect(x, y, pw, ph, [0, 0, 0, 150]);
        y += 3.0 * s;
        self.ui.text(x + 4.0 * s, y, "Desempenho (F9)", [255, 255, 160, 255]);
        y += 10.0 * s;
        let sub = if st.cpu_name.is_empty() { gpu_name } else { format!("{} | {}", st.cpu_name, gpu_name) };
        let sub_scale = if self.ui.text_width(&sub, s) > pw - 8.0 * s { (s / 2.0).max(1.0) } else { s };
        self.ui.text_scaled(x + 4.0 * s, y, &sub, LABEL, sub_scale);
        y += 10.0 * s;
        for (l, v, c) in &rows {
            self.ui.text(x + 4.0 * s, y, l, LABEL);
            self.ui.text(x + 4.0 * s + label_w, y, v, *c);
            y += 10.0 * s;
        }
        // Frame time graph: one bar per frame, the line marks 60 fps.
        y += 2.0 * s;
        let gx = x + 4.0 * s;
        let gw = pw - 8.0 * s;
        self.ui.rect(gx, y, gw, graph_h, [20, 20, 20, 180]);
        let bars = (gw / s) as usize;
        let times: Vec<f32> = self.perf.recent(bars).collect();
        let full = 1.0 / 20.0; // a 50 ms frame fills the graph
        let start = gx + gw - times.len() as f32 * s;
        for (i, dt) in times.iter().enumerate() {
            let hgt = (dt / full).min(1.0) * graph_h;
            let fps = 1.0 / dt;
            let c = fps_col(fps);
            self.ui.rect(start + i as f32 * s, y + graph_h - hgt, s, hgt.max(s * 0.5), c);
        }
        let l60 = y + graph_h - (1.0 / 60.0) / full * graph_h;
        self.ui.rect(gx, l60, gw, (s * 0.5).max(1.0), [255, 255, 255, 120]);
        self.ui.scale = gui;
    }

    /// Loading screen like the classic one: the last view blurred, "Carregando terreno...", a green
    /// bar and a map of the chunks around the spawn coloured by how far along they are.
    fn draw_loading(&mut self, w: f32, h: f32) {
        let s = self.ui.scale;
        match &self.backdrop {
            Some((bw, bh, px)) => {
                let (cw, ch) = (w / (*bw - 1) as f32, h / (*bh - 1) as f32);
                let dark = |c: [u8; 4]| [(c[0] as f32 * 0.75) as u8, (c[1] as f32 * 0.75) as u8, (c[2] as f32 * 0.75) as u8, 255];
                for y in 0..bh - 1 {
                    for x in 0..bw - 1 {
                        let at = |xx: usize, yy: usize| dark(px[yy * bw + xx]);
                        self.ui.gradient(x as f32 * cw, y as f32 * ch, cw + 0.5, ch + 0.5, [at(x, y), at(x + 1, y), at(x + 1, y + 1), at(x, y + 1)]);
                    }
                }
            }
            None => self.ui.rect(0.0, 0.0, w, h, [44, 34, 26, 255]),
        }
        let (c, near) = self.loading_area;
        let mut total = 0;
        let mut done = 0;
        for dz in -near..=near {
            for dx in -near..=near {
                if dx * dx + dz * dz <= near * near {
                    total += 1;
                    done += (self.world.chunk_status(c + ChunkPos::new(dx, dz)) == 3) as i32;
                }
            }
        }
        let pct = done as f32 / total.max(1) as f32;
        let ty = (h * 0.18).floor();
        let label = "Carregando terreno...";
        let tw = self.ui.text_width(label, s);
        self.ui.text((w - tw) / 2.0 + s, ty + s, label, [40, 40, 40, 255]);
        self.ui.text((w - tw) / 2.0, ty, label, WHITE);
        let bw = (w * 0.6).floor();
        let (bx, by) = ((w - bw) / 2.0, ty + 14.0 * s);
        self.ui.rect(bx, by, bw, 2.0 * s, [0, 0, 0, 255]);
        self.ui.rect(bx, by, bw * pct, 2.0 * s, [0, 255, 0, 255]);
        // Chunk map: a grey square in a thick black frame, one cell per chunk.
        let r = near + 2;
        let n = (2 * r + 1) as f32;
        let cell = (4.0 * s).floor().max(2.0);
        let inner = n * cell;
        let border = 8.0 * s;
        let mx = ((w - inner) / 2.0).floor();
        let my = (by + 2.0 * s + 22.0 * s).floor();
        self.ui.rect(mx - border, my - border, inner + 2.0 * border, inner + 2.0 * border, [0, 0, 0, 255]);
        self.ui.rect(mx, my, inner, inner, [153, 153, 153, 255]);
        for dz in -r..=r {
            for dx in -r..=r {
                let st = self.world.chunk_status(c + ChunkPos::new(dx, dz));
                let (x, y) = (mx + (dx + r) as f32 * cell, my + (dz + r) as f32 * cell);
                let col = if dx == 0 && dz == 0 && st == 3 {
                    [255, 255, 255, 255]
                } else {
                    match st {
                        1 => [47, 53, 119, 255],
                        2 => [127, 178, 76, 255],
                        3 => [217, 217, 217, 255],
                        _ => continue,
                    }
                };
                self.ui.rect(x, y, cell, cell, col);
            }
        }
    }

    fn draw_ui(&mut self, mut info_lines: Vec<String>) {
        let (w, h) = (self.renderer.config.width as f32, self.renderer.config.height as f32);
        self.ui.begin(w, h, self.settings.gui_scale);
        let s = self.ui.scale;
        let mouse = self.mouse_pos;
        let mut clicked_button: Option<usize> = None;

        if self.screen == Screen::Loading {
            // The first loading frame shows the plain view so it can be captured as the backdrop.
            if !self.want_backdrop {
                self.draw_loading(w, h);
            }
            return;
        }

        if self.survival && self.hurt_age < 0.4 {
            let a = ((0.4 - self.hurt_age) / 0.4 * 90.0) as u8;
            self.ui.rect(0.0, 0.0, w, h, [200, 0, 0, a]);
        }
        let menu_screen = self.is_title_screen() || matches!(self.screen, Screen::Options | Screen::Packs);
        if !self.hide_hud && !matches!(self.screen, Screen::Paused | Screen::Dead | Screen::Shaders) && !menu_screen {
            // Crosshair.
            if self.screen == Screen::Playing {
                let (cx, cy) = (w / 2.0, h / 2.0);
                if !self.ui.sprite("sprites/hud/crosshair", (cx - 7.5 * s).round(), (cy - 7.5 * s).round(), 15.0 * s, 15.0 * s) {
                    let c = [235, 235, 235, 220];
                    self.ui.rect(cx - 5.0 * s, cy - s / 2.0, 10.0 * s, s, c);
                    self.ui.rect(cx - s / 2.0, cy - 5.0 * s, s, 10.0 * s, c);
                }
                // How ready the weapon is, under the crosshair, while it recharges.
                if self.survival {
                    let k = self.attack_charge();
                    if k < 1.0 {
                        let bw = 16.0 * s;
                        self.ui.rect(cx - bw / 2.0, cy + 9.0 * s, bw, 2.0 * s, [40, 40, 40, 200]);
                        self.ui.rect(cx - bw / 2.0, cy + 9.0 * s, bw * k, 2.0 * s, [235, 235, 235, 230]);
                    }
                }
            }
            self.draw_mob_health_bars(w, h);
            // Hotbar.
            let cell = 20.0 * s;
            let hx = (w - cell * 9.0) / 2.0;
            let hy = h - cell - 2.0 * s;
            let hud_art = self.ui.sprite("sprites/hud/hotbar", hx - s, hy - s, 182.0 * s, 22.0 * s);
            if !hud_art {
                self.ui.rect(hx - s, hy - s, cell * 9.0 + 2.0 * s, cell + 2.0 * s, [0, 0, 0, 150]);
                for i in 0..9 {
                    let x = hx + i as f32 * cell;
                    self.ui.frame(x, hy, cell, cell, s, [110, 110, 110, 255]);
                }
            }
            self.draw_hotbar_items(hx, hy, cell);
            if self.survival {
                self.draw_survival_hud(hx, hy, cell);
            }
            let sx = hx + self.selected as f32 * cell;
            if !self.ui.sprite("sprites/hud/hotbar_selection", sx - 2.0 * s, hy - 2.0 * s, 24.0 * s, 23.0 * s) {
                self.ui.frame(sx - s, hy - s, cell + 2.0 * s, cell + 2.0 * s, 2.0 * s, [255, 255, 255, 255]);
            }
            if self.body.flying {
                self.ui.text(4.0 * s, h - 12.0 * s, "Voando", GRAY);
            } else if self.crouch > 0.5 {
                self.ui.text(4.0 * s, h - 12.0 * s, "Agachado", GRAY);
            } else if self.sprinting {
                self.ui.text(4.0 * s, h - 12.0 * s, "Correndo", GRAY);
            }
            if let Some((b, t)) = self.mining {
                let id = self.world.block_or_air(b);
                let k = (t / self.dig_time(id)).clamp(0.0, 1.0);
                if k > 0.02 {
                    let bw = 24.0 * s;
                    self.ui.rect(w / 2.0 - bw / 2.0, h / 2.0 + 8.0 * s, bw, 2.0 * s, [0, 0, 0, 160]);
                    self.ui.rect(w / 2.0 - bw / 2.0, h / 2.0 + 8.0 * s, bw * k, 2.0 * s, [230, 230, 230, 230]);
                }
            }
            if self.survival {
                // Ten hearts above the left of the hotbar, bubbles on the right under water.
                let y = hy - 11.0 * s;
                for i in 0..10 {
                    let x = hx + i as f32 * 8.0 * s;
                    let v = self.health - i as f32 * 2.0;
                    let blink = self.hurt_age < 0.6 && ((self.hurt_age * 10.0) as i32) % 2 == 0;
                    if self.ui.sprite("sprites/hud/heart/container", x, y - s, 9.0 * s, 9.0 * s) {
                        if v > 0.0 {
                            let art = if v >= 2.0 { "sprites/hud/heart/full" } else { "sprites/hud/heart/half" };
                            self.ui.sprite(art, x, y - s, 9.0 * s, 9.0 * s);
                        }
                        continue;
                    }
                    draw_heart(&mut self.ui, x, y, s, [40, 0, 0, 255], 1.0, blink);
                    if v > 0.0 {
                        draw_heart(&mut self.ui, x, y, s, [220, 30, 30, 255], if v >= 2.0 { 1.0 } else { 0.5 }, false);
                    }
                }
                if self.air < 10.0 {
                    let bubbles = (self.air.max(0.0).ceil()) as i32;
                    for i in 0..bubbles {
                        let x = hx + cell * 9.0 - (i + 1) as f32 * 8.0 * s;
                        let y = y - 10.0 * s;
                        if self.ui.sprite("sprites/hud/air", x, y - s, 9.0 * s, 9.0 * s) {
                            continue;
                        }
                        self.ui.rect(x + s, y + s, 5.0 * s, 5.0 * s, [60, 120, 230, 255]);
                        self.ui.rect(x + 2.0 * s, y + 2.0 * s, 2.0 * s, 2.0 * s, [220, 240, 255, 255]);
                    }
                }
            }
            if let Some(t) = &self.toast {
                if t.until > self.now {
                    let text = t.text.clone();
                    let ty = if self.survival { hy - 34.0 * s } else { hy - 16.0 * s };
                    self.ui.text_centered(w / 2.0, ty, &text, WHITE);
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
                    format!("Shaders: {}", self.shader_name()),
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
                    "WASD andar  Espaço pular (2x = voar)  Shift agachar  Ctrl correr  Z deitar",
                    "Esq. quebrar  Dir. colocar  Meio copiar bloco  E blocos  1-9/roda escolher",
                    "F1 esconder HUD  F2 foto  F3 info  F5 câmera  F9 desempenho  F11 tela cheia",
                    "Controle: analógicos andar e olhar, botões no menu Controle",
                ];
                y += 4.0 * s;
                let hs = (s / 2.0).floor().max(1.0);
                let folder = self.base.join(&self.settings.music_folder).display().to_string();
                let packs = self.base.join("resourcepacks").display().to_string();
                let shaders = self.base.join("shaderpacks").display().to_string();
                let extra = [format!("Suas músicas: {folder}"), format!("Pacotes de textura: {packs}"), format!("Pacotes de shader: {shaders}")];
                for l in help.iter().map(|s| s.to_string()).chain(extra) {
                    let tw = self.ui.text_width(&l, hs);
                    self.ui.text_scaled(w / 2.0 - tw / 2.0, y, &l, GRAY, hs);
                    y += 10.0 * hs;
                }
            }
            Screen::Paintings => self.draw_paintings_menu(w, h, mouse),
            Screen::Title | Screen::Worlds | Screen::CreateWorld | Screen::Multiplayer => self.draw_title_menus(w, h, mouse),
            Screen::Options | Screen::Packs => {
                if self.world_open {
                    self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 150]);
                } else {
                    let ready = self.world.meshed_count > 20;
                    self.ui.rect(0.0, 0.0, w, h, if ready { [0, 0, 0, 110] } else { [44, 34, 26, 255] });
                }
                if self.screen == Screen::Options {
                    self.draw_options(w, h, mouse);
                } else {
                    self.draw_packs(w, h, mouse);
                }
            }
            Screen::Shaders => clicked_button = self.draw_shaders_menu(w, h, mouse),
            Screen::Dead => {
                self.ui.rect(0.0, 0.0, w, h, [120, 0, 0, 110]);
                self.ui.text_centered(w / 2.0, h / 2.0 - 40.0 * s, "Você morreu!", WHITE);
                let bw = 200.0 * s;
                let bh = 20.0 * s;
                if self.ui.button((w - bw) / 2.0, h / 2.0, bw, bh, "Renascer", mouse) && self.clicked {
                    clicked_button = Some(0);
                }
            }
            Screen::Inventory if self.creative_screen_open() => self.draw_creative_inventory(w, h, mouse),
            Screen::Inventory => self.draw_survival_inventory(w, h, mouse),
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
        if self.show_perf {
            self.draw_perf();
        }
        self.clicked = false;
        self.rclicked = false;
        self.mclicked = false;

        if let Some(i) = clicked_button {
            if self.screen == Screen::Controls {
                self.controls_button(i);
            } else if self.screen == Screen::Shaders {
                self.shaders_button(i);
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
                self.set_screen(self.options_back);
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
            Action::Inventory if self.survival => self.surv_open_inventory(),
            Action::Inventory => self.set_screen(Screen::Inventory),
            Action::Paintings => self.open_paintings(),
            Action::Crawl => self.prone = !self.prone,
            Action::Pause => self.set_screen(Screen::Paused),
            Action::HotbarPrev => self.scroll(1.0),
            Action::HotbarNext => self.scroll(-1.0),
            Action::Screenshot => self.screenshot_request = true,
            Action::Debug => self.show_debug = !self.show_debug,
            Action::HideHud => self.hide_hud = !self.hide_hud,
            Action::Attack => self.attack_pressed = true,
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
                            self.set_screen(self.options_back);
                        }
                        Screen::Shaders => self.set_screen(Screen::Paused),
                        Screen::Dead | Screen::Title => {}
                        Screen::Worlds | Screen::CreateWorld | Screen::Multiplayer | Screen::Options | Screen::Packs => self.menu_back(),
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
            7 => {
                self.options_back = Screen::Paused;
                self.set_screen(Screen::Controls);
            }
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
            10 => {
                self.shader_scroll = 0;
                self.set_screen(Screen::Shaders);
            }
            _ => {
                self.quit_to_title();
                return;
            }
        }
        self.settings.save(&self.base.join("settings.ron"));
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let raw_dt = (now - self.last).as_secs_f32();
        self.perf.push(raw_dt);
        let dt = raw_dt.min(0.1);
        if self.args.screenshot.is_some() && self.screen == Screen::Loading && self.frame_times.len() == 29 {
            log::info!("loading: {:.1} fps, {} meshed, {} jobs", self.fps, self.world.meshed_count, self.world.pending_jobs());
        }
        self.last = now;
        self.frame_times.push(dt);
        if self.frame_times.len() >= 30 {
            self.fps = self.frame_times.len() as f32 / self.frame_times.iter().sum::<f32>();
            self.frame_times.clear();
        }
        self.update(dt);

        // Eye height eases between standing, sneaking and lying down instead of jumping.
        let stand = if self.body.flying { voxel::physics::EYE_HEIGHT } else { voxel::physics::EYE_HEIGHT - 0.25 * self.crouch };
        let head = self.body.pos + Vec3::Y * (stand + (0.42 - stand) * self.prone_t);
        let playing_view = self.world_open && !matches!(self.screen, Screen::Loading);
        let third = playing_view && self.camera_mode != 0;
        let mut view_fwd = self.forward();
        let eye = if third {
            // Pull the camera back (or around to the front), stopping short of walls.
            let dir = if self.camera_mode == 1 { -view_fwd } else { view_fwd };
            let world = &self.world;
            // Lying down, the camera orbits a bit above the body so it clears the ground.
            let pivot = head + Vec3::Y * 0.6 * self.prone_t;
            let dist = raycast(pivot, dir, 4.0, |p| voxel::physics::Terrain::solid(world, p)).map(|h| (h.distance - 0.3).max(0.2)).unwrap_or(4.0);
            if self.camera_mode == 2 {
                view_fwd = -view_fwd;
            }
            pivot + dir * dist
        } else if playing_view && self.settings.view_bobbing {
            head + self.anim.view_bob(self.yaw, self.body.on_ground && !self.body.flying)
        } else {
            head
        };
        let block = eye.floor().as_ivec3();
        let (top, horizon, daylight, sun) = self.sky();
        let underwater = self.body.head_in_water;
        let rd = self.settings.render_distance as f32 * 16.0;
        // Distant terrain pushes the fog out to its own edge.
        let extra = self.settings.lod_distance.max(0) as f32;
        let view_end = rd + extra;
        let (fog_color, fog_start, fog_end) = if underwater {
            ([0.04, 0.12, 0.35], 0.0, 28.0)
        } else if extra > 0.0 {
            (horizon, rd * 0.6, view_end - 8.0)
        } else {
            (horizon, rd * 0.55, rd - 2.0)
        };
        // Real chunks surely cover this radius (see World::update); distant terrain starts here.
        let lod_inner = (self.settings.render_distance as f32 - 0.5) * 16.0;
        self.lod.update(&self.renderer.device, eye, lod_inner, if extra > 0.0 { view_end } else { 0.0 });
        let mut effects = self.terrain_effects();
        effects[3] = lod_inner;
        let mut g = Globals {
            cam_pos: [eye.x - block.x as f32, eye.y - block.y as f32, eye.z - block.z as f32, (self.now % 3600.0) + self.start.elapsed().as_secs_f32() * 0.0],
            cam_world: [eye.x, eye.y, eye.z, CLOUD_HEIGHT],
            fog_color: [fog_color[0], fog_color[1], fog_color[2], daylight],
            fog: [fog_start, fog_end, if underwater { 1.0 } else { 0.0 }, 0.3],
            sun_dir: [sun.x, sun.y, sun.z, 0.0],
            sky_top: [top[0], top[1], top[2], 1.0],
            sky_horizon: [horizon[0], horizon[1], horizon[2], 1.0],
            effects,
            // The light a held torch gives comes from the player's hand, wherever the camera is.
            hand_light: [head.x - block.x as f32, head.y - 0.4 - block.y as f32, head.z - block.z as f32, self.hand_light_level()],
            ..Default::default()
        };
        let cam = Camera {
            block,
            frac: Vec3::from_slice(&g.cam_pos[..3]),
            forward: view_fwd,
            fov_y: (self.settings.fov * (1.0 + 0.12 * self.fov_kick)).to_radians(),
            far: view_end + 32.0,
        };

        {
            let (w, h) = (self.renderer.config.width as f32, self.renderer.config.height as f32);
            let proj = glam::Mat4::perspective_rh(cam.fov_y, w / h.max(1.0), 0.05, cam.far);
            let view = glam::Mat4::look_to_rh(cam.frac, cam.forward, Vec3::Y);
            self.ui_view_proj = Some((proj * view, block));
        }
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
            lines.push(format!("Shader: {} ({})", self.shader_name(), self.settings.shader_quality.label()));
            if self.settings.lod_distance > 0 {
                lines.push(format!(
                    "LOD: +{} blocos, {} de {} tiles desenhados, {} tarefas ({} threads), {:.1} MB",
                    self.settings.lod_distance,
                    self.last_frame_info.2,
                    self.lod.tiles.len(),
                    self.lod.pending_jobs(),
                    self.lod.workers,
                    self.lod.vram_bytes as f64 / 1e6
                ));
            }
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
        if self.args.shot_loading && self.screen == Screen::Loading && self.backdrop.is_some() && self.world.meshed_count >= 12 {
            capture = true;
            exit_after = true;
        } else if self.args.screenshot.is_some() && self.screen != Screen::Loading && !self.args.shot_loading {
            self.loaded_frames += 1;
            if self.loaded_frames % 30 == 0 {
                log::info!("screenshot wait: frame {}, {:.1} fps, {} jobs pending", self.loaded_frames, self.fps, self.world.pending_jobs());
            }
            if (self.loaded_frames > 30 && self.world.pending_jobs() == 0 && self.lod.is_complete()) || self.loaded_frames > 1500 {
                capture = true;
                exit_after = true;
            }
        }
        let selection = if self.screen == Screen::Playing || self.args.screenshot.is_some() { self.target.map(|t| t.block) } else { None };
        let mut items = self.mobs.instances(&self.world, block, rd + 16.0);
        let mut held_third = None;
        if let Some(pd) = self.mobs.reg.id("player").filter(|_| playing_view) {
            let def = &self.mobs.reg.defs[pd];
            let pose = self.pose(false);
            if third {
                let l = self.world.light((self.body.pos + Vec3::Y).floor().as_ivec3());
                let bones = self.anim.bones(def, &pose, block, (l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
                let pv = def.parts[2].pivot;
                held_third = self.held_item_third_person(bones.m[2], Vec3::new(pv.0 as f32, pv.1 as f32, pv.2 as f32));
                items.extend(self.armor_instances(&bones));
                items.push((pd, bones));
            } else if self.screen == Screen::Playing && !self.hide_hud {
                let l = self.world.light(head.floor().as_ivec3());
                items.push((pd, self.anim.first_person_arm(def, eye - block.as_vec3(), self.yaw, self.pitch, (l >> 4) as f32 / 15.0, ((l & 15) as f32 / 15.0).max(self.hand_light_level()))));
            }
        }
        for p in &self.paintings {
            items.push((self.entities.painting_mesh(p.def), p.bones(block, &self.world)));
        }
        // The player figure in the inventory, in 3D, turning its head toward the mouse.
        let mut figure_backdrop = None;
        if let (Some((bx, by, bw, bh)), Some(pd)) = (self.surv.figure_box.take(), self.mobs.reg.id("player")) {
            let (sw, sh) = (self.renderer.config.width as f32, self.renderer.config.height as f32);
            let d = 0.3;
            let half_h = (cam.fov_y * 0.5).tan() * d;
            let half_w = half_h * sw / sh.max(1.0);
            let f = cam.forward.normalize();
            let right = f.cross(Vec3::Y).normalize_or_zero();
            let up = right.cross(f);
            let to_world = |x: f32, y: f32| eye + f * d + right * ((x / sw * 2.0 - 1.0) * half_w) + up * ((1.0 - y / sh * 2.0) * half_h);
            let center = to_world(bx + bw / 2.0, by + bh / 2.0);
            let world_h = bh / sh * 2.0 * half_h;
            let k = world_h * 0.8 / 2.0;
            let feet = center - up * (world_h * 0.44);
            let (mx, my) = self.mouse_pos;
            let look_x = ((mx - (bx + bw / 2.0)) / (bw * 0.6)).atan();
            let look_y = (((by + bh * 0.25) - my) / (bh * 0.6)).atan();
            let def = &self.mobs.reg.defs[pd];
            let pose = player_model::Pose { pos: block.as_vec3(), look_yaw: look_x - std::f32::consts::PI, look_pitch: look_y, on_ground: true, ..Default::default() };
            let base = player_model::Animator::lying(0.0).bones(def, &pose, block, 1.0, 1.0);
            // Stand it up facing the camera, scaled into the box.
            let face = glam::Mat4::from_cols(right.extend(0.0), up.extend(0.0), (-f).extend(0.0), Vec4::W);
            let m = glam::Mat4::from_translation(feet - block.as_vec3()) * face * glam::Mat4::from_rotation_y(look_x * 0.35) * glam::Mat4::from_scale(Vec3::splat(k));
            let mut b = base;
            for mm in b.m.iter_mut() {
                *mm = (m * glam::Mat4::from_cols_array_2d(mm)).to_cols_array_2d();
            }
            items.extend(self.armor_instances(&b));
            items.push((pd, b));
            let back = center + f * (world_h * 1.3) - block.as_vec3();
            figure_backdrop = Some(item_render::GpuItem { pos: [back.x, back.y, back.z, world_h * 1.12 * (d + world_h * 1.3) / d], info: [0.0, 0.0, 0.0, 0.0], layers: [0.0; 4], rot: [0.0; 4] });
        }
        if let Some(pd) = self.mobs.reg.id("player").filter(|_| playing_view) {
            let def = &self.mobs.reg.defs[pd];
            for c in &self.surv.corpses {
                if (c.pos - eye).length() > rd + 16.0 {
                    continue;
                }
                let pose = player_model::Pose { pos: c.pos, crawling: 1.0, look_yaw: c.yaw, on_ground: true, ..Default::default() };
                let l = self.world.light((c.pos + Vec3::Y * 0.3).floor().as_ivec3());
                let bones = player_model::Animator::lying(c.yaw).bones(def, &pose, block, (l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
                // The armour stays on the body until it is taken back.
                let mut worn = [false; 4];
                for st in &c.items {
                    let it = self.items.get(st.item);
                    let Some(a) = it.armor else { continue };
                    let piece = (a.slot as usize).min(3);
                    if !worn[piece] {
                        worn[piece] = true;
                        items.extend(self.entities.armor_mesh(&it.name, piece).map(|m| (m, bones)));
                    }
                }
                items.push((pd, bones));
            }
        }
        if let (Screen::Paintings, Some((d, anchor)), Some((_, n))) = (self.screen, self.painting_preview, self.painting_spot) {
            let p = paintings::Painting { name: String::new(), anchor: anchor.to_array(), facing: n.to_array(), def: d };
            items.push((self.entities.painting_mesh(d), p.bones(block, &self.world)));
        }
        self.entities.prepare(&self.renderer, items);
        self.entities.particles.prepare(&self.renderer, &self.particles, &self.world, block);
        let mut drops = self.drop_instances(block);
        drops.extend(figure_backdrop);
        if let Some(k) = self.args.swing {
            self.anim.swing = k;
        }
        if !third && self.screen == Screen::Playing && !self.hide_hud && self.camera_mode == 0 {
            drops.extend(self.held_item_instance(eye, block));
        }
        drops.extend(held_third);
        if let (Some(k), Some(h)) = (self.args.mine, self.target) {
            let id = self.world.block_or_air(h.block);
            self.mining = Some((h.block, k * self.dig_time(id)));
        }
        // Cracks spreading over the block being broken.
        if let Some((b, t)) = self.mining {
            let id = self.world.block_or_air(b);
            let k = t / self.dig_time(id).max(0.01);
            if id != AIR && k > 0.0 {
                let layer = (self.item_base as usize + self.items.textures.len() + 1 + ((k * 10.0) as usize).min(9)) as f32;
                let p = b.as_vec3() + Vec3::splat(0.5) - block.as_vec3();
                let lp = self.target.map(|h| h.block + h.normal).unwrap_or(b + IVec3::Y);
                let l = self.world.light(lp);
                drops.push(item_render::GpuItem {
                    pos: [p.x, p.y, p.z, 1.004],
                    info: [0.0, 0.0, (l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0],
                    layers: [layer, layer, layer, 0.0],
                    rot: [0.0; 4],
                });
            }
        }
        self.entities.items.prepare(&self.renderer, &drops);
        let grab_backdrop = self.want_backdrop && self.screen == Screen::Loading;
        let (info, shot) = self.renderer.render(&cam, &mut g, selection, self.settings.show_clouds, Some(&self.entities), Some(&self.lod), &self.ui, capture || grab_backdrop);
        let shot = if grab_backdrop {
            self.want_backdrop = false;
            if let Some((w, h, px)) = &shot {
                self.backdrop = Some(blurred_backdrop(*w as usize, *h as usize, px));
            }
            shot.filter(|_| capture)
        } else {
            shot
        };
        self.last_frame_info = (info.sections_drawn, info.quads_drawn, info.lod_drawn);
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
            println!(
                "frame stats: {:.0} fps, {} sections, {} quads, {} lod tiles ({} quads), shader {}",
                self.fps,
                info.sections_drawn,
                info.quads_drawn,
                info.lod_drawn,
                info.lod_quads,
                self.shader_name()
            );
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
        let icon = image::load_from_memory(include_bytes!("../../../assets/icon/icon_64.png"))
            .ok()
            .and_then(|i| {
                let i = i.to_rgba8();
                let (iw, ih) = i.dimensions();
                winit::window::Icon::from_rgba(i.into_raw(), iw, ih).ok()
            });
        let attrs = Window::default_attributes().with_title("Mine Clone").with_window_icon(icon).with_inner_size(winit::dpi::PhysicalSize::new(w, h));
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
                if event.state == ElementState::Pressed {
                    if let Some(t) = &event.text {
                        game.type_text(t);
                    }
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
                if idx == 1 {
                    game.rdown = pressed;
                }
                if game.screen == Screen::Playing {
                    if !game.grabbed {
                        game.set_grab(true);
                    }
                    game.mouse_held[idx] = pressed;
                    if pressed && idx == 0 {
                        game.attack_pressed = true;
                    }
                    if !pressed {
                        if idx == 0 {
                            game.break_cd = 0.0;
                        } else if idx == 1 {
                            game.place_cd = 0.0;
                        }
                    }
                } else if pressed && idx == 0 {
                    game.clicked = true;
                } else if pressed && idx == 1 {
                    game.rclicked = true;
                } else if pressed && idx == 2 {
                    game.mclicked = true;
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

#[cfg(test)]
mod data_tests {
    use super::*;

    #[test]
    fn built_in_data_loads() {
        let (reg, items, recipes) = game_data(embedded::BLOCKS_RON, embedded::ITEMS_RON, embedded::RECIPES_RON).unwrap();
        assert!(reg.id("oak_stairs:5").is_some());
        assert!(items.id("cobblestone_slab").is_some());
        assert!(!recipes.shaped.is_empty());
    }

    #[test]
    fn stale_blocks_file_is_rejected() {
        // An old blocks.ron without slabs must make the game use the built-in set, not crash.
        let dir = std::env::temp_dir().join(format!("mc-stale-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("packs/vanilla")).unwrap();
        std::fs::write(dir.join("packs/vanilla/blocks.ron"), "(blocks: [(name: \"stone\")])").unwrap();
        assert!(load_game_data(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
