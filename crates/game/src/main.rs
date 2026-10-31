//! Mine Clone: a fast block game written in Rust.

// No console window behind the game on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod ambient;
mod armor_design;
#[cfg(test)]
mod armor_icon;
mod flora_tex;
mod ambient_life;
mod beds;
mod chests;
mod heights;
mod material;
mod companions;
mod options;
mod sentries;
mod shield;
mod vanity;
mod wings;
mod anim;
mod deep;
mod keybinds;
mod lang;
mod subtitles;
mod pvp;
mod cmdblock;
mod server;
mod plugins;
mod nature;
mod bench;
mod storage;
mod swing;
mod stands;
mod testworld;
mod pale;
mod selectors;
mod audio;
mod backpacks;
mod controller;
mod elements;
mod skills;
mod prof;
mod presets;
mod atlas;
mod vulcao;
mod enchanting;
mod drop_fx;
mod effects;
mod progression;
mod stations;
mod smithing;
mod weapons;
mod advancements;
mod trials;
mod deepdark;
mod archaeology;
mod fase6_test;
#[cfg(test)]
mod parity_test;
mod copper;
mod commands;
mod creative;
mod dimension;
mod doors;
mod end;
mod farm;
mod fishing;
mod magic;
mod bosses;
mod npcs;
mod pets;
mod raids;
mod leads;
mod bees;
mod boats;
mod villages;
mod explore;
mod signs;
mod terraria;
mod rpg;
mod accessories;
mod affixes;
mod boss_ai;
mod tr;
mod tr3;
mod tr3_foes;
mod tr3_tools;
mod tr3_traps;
mod tr3_world;
mod tr3_folk;
mod tr3_golf;
mod tr_bosses;
mod tr_events;
mod tr_pets;
mod tr_town;
mod tr_endgame;
mod tr_fishing;
mod tr_invasions;
mod tr_utility;
mod tr_modes;
mod tr_style;
mod tr_dungeon;
mod tr_comfort;
mod tr_minis;
mod tr_journey;
mod tr_feel3d;
mod tr_weapons;
#[cfg(test)]
mod balance;
mod mob_tiers;
mod tr_world;
mod glyphs;
mod hardmode;
mod hook;
mod mantle;
mod fluids;
mod redstone;
mod wanderers;
mod furniture;
mod gear;
mod weather;
mod windmill;
mod banners;
mod wind;
mod entity;
mod instruments;
mod torch;
mod summons;
mod fire_fx;
mod redstone_fx;
mod item_render;
mod juice;
mod menus;
mod monitor;
mod money;
mod multiplayer;
mod voice;
mod mods;
mod lod;
mod mobs;
mod social;
mod nether_end;
mod paintings;
mod particles;
mod player_model;
mod post;
mod pots;
mod progress;
mod rails;
mod headless;
mod packmodels;
mod renderer;
mod scenefx;
mod screen_drops;
mod settings;
mod sfx;
mod mob_sounds;
mod sfx_extra;
mod soundscape;
mod fx;
mod shaderpack;
mod survival;
mod textures;
mod ui;
mod world;
mod vanilla_extras;
mod vanilla_mobs;
mod wagons;

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
use voxel::block::{Shape, Turn};
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


/// Textures loaded after the block and item ones: the experience orb, then the ten cracks
/// drawn over a block while it is being broken.
fn extra_textures() -> Vec<String> {
    let mut v = vec!["item/xp_orb".to_string()];
    v.extend((0..10).map(|i| format!("destroy_stage_{i}")));
    v
}
const CLOUD_HEIGHT: f32 = 160.0;

#[derive(Default, Clone)]
struct Args {
    world: Option<String>,
    /// Dedicated server: no window shown, the world open to friends, commands from the console.
    server: bool,
    /// Run only a relay (`--relay [port]`), for hosts whose router won't open a port.
    relay: Option<u16>,
    /// Benchmark: fly for this many seconds, then report frame times and close.
    bench: Option<f32>,
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
    /// Build test redstone circuits in front of the player.
    redstone: bool,
    /// Build the Phase 4 test scene: rails and carts, copper, a crafter.
    phase4: bool,
    /// Take the --screenshot of the loading screen instead of the game.
    shot_loading: bool,
    /// Start with the interface hidden (F1), for clean screenshots.
    no_hud: bool,
    rain: bool,
    find: Option<String>,
    storm: bool,
    /// Screenshots: also wait this many seconds of game time (to let water flow).
    wait: f32,
    /// Creative inventory tab to open (0..13).
    ctab: Option<usize>,
    show_inventory: bool,
    /// Where the mouse sits in screenshots (pixels).
    mouse: Option<(f32, f32)>,
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
    /// Distant terrain detail preset 0..2 (`lod::DETAILS`).
    lod_detail: Option<u8>,
    /// Block depth level 0..3 (test screenshots and benchmarks).
    parallax: Option<u8>,
}

fn parse_args() -> Args {
    let mut a = Args::default();
    let mut it = std::env::args().skip(1).peekable();
    let f3 = |s: &str| -> Option<Vec3> {
        let v: Vec<f32> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (v.len() == 3).then(|| Vec3::new(v[0], v[1], v[2]))
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--world" => a.world = it.next(),
            "--server" => a.server = true,
            "--relay" => a.relay = Some(it.next_if(|v| v.parse::<u16>().is_ok()).and_then(|v| v.parse().ok()).unwrap_or(net::relay::DEFAULT_PORT)),
            "--bench" => a.bench = Some(it.next_if(|v| v.parse::<f32>().is_ok()).and_then(|v| v.parse().ok()).unwrap_or(60.0)),
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
            "--redstone" => a.redstone = true,
            "--fase4" => a.phase4 = true,
            "--shot-loading" => a.shot_loading = true,
            "--no-hud" => a.no_hud = true,
            "--rain" => a.rain = true,
            "--find" => a.find = it.next(),
            "--storm" => a.storm = true,
            "--wait" => a.wait = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0),
            "--ctab" => a.ctab = it.next().and_then(|v| v.parse().ok()),
            "--inventory" => a.show_inventory = true,
            "--mouse" => a.mouse = it.next().and_then(|s| { let (x, y) = s.split_once(',')?; Some((x.parse().ok()?, y.parse().ok()?)) }),
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
            "--lod-detail" => a.lod_detail = it.next().and_then(|s| s.parse().ok()),
            "--parallax" => a.parallax = it.next().and_then(|s| s.parse().ok()),
            "--help" | "-h" => {
                println!(
                    "mine-clone [--world NAME] [--seed N] [--render-distance N] [--size WxH]\n\
                     Server: mine-clone --server [--world NAME] (no window; type commands, \"parar\" saves and stops)\n\
                     Relay: mine-clone --relay [port] (for hosts whose router won't open a port; default port 24455)\n\
                     Benchmark: mine-clone --bench [seconds] [--world NAME] (report in bench/)\n\
                     Testing: [--screenshot out.png] [--pos x,y,z] [--look yaw,pitch] [--time ticks] [--fly] [--demo] [--inventory] [--pause] [--debug]\n\
                     \x20        [--survival] [--kit] [--open table|furnace|chest] [--mobs] [--perf] [--prone] [--camera N]\n\
                     \x20        [--shader NAME|off] [--shaders-menu] [--lod BLOCKS] [--lod-detail 0-2] [--parallax 0-3]"
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
    game_data_with_mods([blocks, embedded::MODS_BLOCKS_RON], [items, embedded::MODS_ITEMS_RON], [recipes, embedded::MODS_RECIPES_RON])
}

/// Base game data, then the mod content pack (`packs/vanilla/mods/`); the Terraria side (Etapa 5)
/// and Fase 6 keep their data in their own files, joined onto the base lists.
fn game_data_with_mods(blocks: [&str; 2], items: [&str; 2], recipes: [&str; 2]) -> Result<(BlockRegistry, ItemRegistry, Recipes), String> {
    let b0 = terraria::with_extra(&merge_list(blocks[0], &fase6_text(None, "blocks.ron")), embedded::T5_BLOCKS_RON);
    // The deep world's blocks and items (worldgen/deep.rs).
    let b0 = terraria::with_extra(&b0, embedded::DEEP_BLOCKS_RON);
    // The rest of the Terraria side (tr.rs).
    let b0 = terraria::with_extra(&b0, embedded::TR_BLOCKS_RON);
    // Water plants of the lakes, rivers and seas (worldgen/waters.rs).
    let b0 = terraria::with_extra(&b0, embedded::AGUAS_BLOCKS_RON);
    // The volcanoes (worldgen/volcano.rs, vulcao.rs).
    let b0 = terraria::with_extra(&b0, embedded::VULCAO_BLOCKS_RON);
    // The Terraria side's third round (tr3.rs).
    let b0 = terraria::with_extra(&b0, embedded::TR3_BLOCKS_RON);
    let i0 = terraria::with_extra(&merge_list(items[0], &fase6_text(None, "items.ron")), embedded::T5_ITEMS_RON);
    // The stats content (rpg.rs) after the Terraria side's own items.
    let i0 = terraria::with_extra(&i0, embedded::RPG_ITEMS_RON);
    let i0 = terraria::with_extra(&i0, embedded::LOTE11_ITEMS_RON);
    let i0 = terraria::with_extra(&i0, embedded::DEEP_ITEMS_RON);
    let i0 = terraria::with_extra(&i0, embedded::TR_ITEMS_RON);
    let i0 = terraria::with_extra(&i0, embedded::VULCAO_ITEMS_RON);
    let i0 = terraria::with_extra(&i0, embedded::TR3_ITEMS_RON);
    // 3D models (statues, trophies, rails...) for blocks of every file above (tools/gen_block_models.py).
    let reg = BlockRegistry::from_ron_many(&[b0.as_str(), blocks[1], embedded::MODELS_RON])?;
    let items = ItemRegistry::from_ron_many(&[i0.as_str(), items[1]], &reg)?;
    let mut recipes = Recipes::from_ron_many(&recipes, &items)?;
    terraria::add_recipes(&mut recipes, embedded::T5_RECIPES_RON, &items)?;
    terraria::add_recipes(&mut recipes, embedded::RPG_RECIPES_RON, &items)?;
    terraria::add_recipes(&mut recipes, embedded::LOTE11_RECIPES_RON, &items)?;
    terraria::add_recipes(&mut recipes, embedded::DEEP_RECIPES_RON, &items)?;
    terraria::add_recipes(&mut recipes, embedded::TR_RECIPES_RON, &items)?;
    terraria::add_recipes(&mut recipes, embedded::VULCAO_RECIPES_RON, &items)?;
    terraria::add_recipes(&mut recipes, embedded::TR3_RECIPES_RON, &items)?;
    let extra = fase6_text(None, "recipes.ron");
    if !extra.trim().is_empty() {
        let more = Recipes::from_ron(&extra, &items)?;
        recipes.shaped.extend(more.shaped);
        recipes.shapeless.extend(more.shapeless);
        recipes.smelting.extend(more.smelting);
    }
    Ok((reg, items, recipes))
}

/// A data file of the base game and its counterpart in the mod content pack, each read from
/// disk when it is there, else the built-in copy.
fn read_pack_pair(base: &Path, name: &str, built_in: &str, mods_built_in: &str) -> [String; 2] {
    let read = |p: PathBuf, b: &str| std::fs::read_to_string(p).unwrap_or_else(|_| b.to_string());
    [read(base.join("packs/vanilla").join(name), built_in), read(base.join("packs/vanilla/mods").join(name), mods_built_in)]
}
/// A data file of the Phase 6 content (`packs/vanilla/fase6/`): on disk when there is one,
/// else the built-in copy.
fn fase6_text(base: Option<&Path>, name: &str) -> String {
    let built_in = match name {
        "blocks.ron" => embedded::FASE6_BLOCKS_RON,
        "items.ron" => embedded::FASE6_ITEMS_RON,
        "recipes.ron" => embedded::FASE6_RECIPES_RON,
        "structures.ron" => embedded::FASE6_STRUCTURES_RON,
        "mobs.ron" => embedded::FASE6_MOBS_RON,
        _ => "",
    };
    base.and_then(|b| std::fs::read_to_string(b.join("packs/vanilla/fase6").join(name)).ok()).unwrap_or_else(|| built_in.to_string())
}

/// `main` with the entries of `extra`'s list added to its list (both written as `(key: [...])`),
/// so a stage of the game can keep its blocks, items, mobs and buildings in a file of its own.
fn merge_list(main: &str, extra: &str) -> String {
    let body = |t: &str| -> Option<(usize, usize)> {
        let a = t.find('[')?;
        let b = t.rfind(']')?;
        (b > a).then_some((a, b))
    };
    let (Some((_, me)), Some((xa, xe))) = (body(main), body(extra)) else { return main.to_string() };
    let inner = extra[xa + 1..xe].trim();
    if inner.is_empty() {
        return main.to_string();
    }
    let head = main[..me].trim_end();
    let sep = if head.ends_with(',') || head.ends_with('[') { "" } else { "," };
    format!("{head}{sep}\n{inner}\n{}", &main[me..])
}

/// The data files in `packs/vanilla/` when there are any, each falling back to the built-in copy.
fn load_game_data(base: &Path) -> Result<(BlockRegistry, ItemRegistry, Recipes), String> {
    let [b, mb] = read_pack_pair(base, "blocks.ron", embedded::BLOCKS_RON, embedded::MODS_BLOCKS_RON);
    let [i, mi] = read_pack_pair(base, "items.ron", embedded::ITEMS_RON, embedded::MODS_ITEMS_RON);
    let [r, mr] = read_pack_pair(base, "recipes.ron", embedded::RECIPES_RON, embedded::MODS_RECIPES_RON);
    game_data_with_mods([&b, &mb], [&i, &mi], [&r, &mr])
}

/// Buildings the generator scatters (`packs/vanilla/structures.ron` and the mod pack's, else the built-in copies).
fn load_structures(base: &Path, reg: &BlockRegistry) -> Arc<voxel::structures::Structures> {
    let [text, mods] = read_pack_pair(base, "structures.ron", embedded::STRUCTURES_RON, embedded::MODS_STRUCTURES_RON);
    let text = merge_list(&text, &fase6_text(Some(base), "structures.ron"));
    let text = merge_list(&text, embedded::TR_STRUCTURES_RON);
    let text = merge_list(&text, embedded::TR3_STRUCTURES_RON);
    let [_, dims] = read_pack_pair(base, "dims_structures.ron", "", embedded::MODS_DIMS_STRUCTURES_RON);
    let parsed = voxel::structures::Structures::from_ron_many(&[&text, &mods, &dims], reg).or_else(|e| {
        log::error!("{e}; using the built-in structures");
        voxel::structures::Structures::from_ron_many(&[embedded::STRUCTURES_RON, &merge_list(embedded::TR_STRUCTURES_RON, embedded::TR3_STRUCTURES_RON), embedded::MODS_STRUCTURES_RON, embedded::MODS_DIMS_STRUCTURES_RON], reg)
    });
    Arc::new(parsed.unwrap_or_else(|e| {
        log::error!("{e}");
        voxel::structures::Structures::empty()
    }))
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
pub(crate) enum Screen {
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
    /// Moving the keyboard keys (keybinds.rs).
    Keys,
    /// Voice chat options (voice.rs).
    Voice,
    /// Shader packs and their options.
    Shaders,
    /// Writing on a sign.
    Sign,
    /// The glyph grimoire: writing spells (Etapa 5).
    Grimoire,
    /// A window a plugin opened.
    Plugin,
    /// "Abrir para amigos": hosting settings, the invite and who is in (multiplayer.rs).
    Friends,
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
    /// Distant terrain detail preset.
    LodDetail,
    /// Graphics extras (Sildur's, Better Clouds and Continuity ideas).
    Shadows,
    Reflections,
    Taa,
    Clouds,
    Emissive,
    /// Block depth (parallax occlusion mapping).
    Parallax,
}

struct Toast {
    text: String,
    until: f32,
}

struct Game {
    window: headless::Gpu<Arc<Window>>,
    renderer: headless::Gpu<Renderer>,
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
    /// Distance slid over ice since the last slide sound.
    slide_dist: f32,
    /// How far the feet have sunk into sand or snow (the camera eases down by it).
    sink: f32,
    /// Distance walked through plants since the last rustle.
    rustle_dist: f32,

    pads: Controllers,
    /// Keys the player moved (keybinds.rs) and the action waiting for its new key.
    key_map: keybinds::KeyMap,
    key_rebinding: Option<usize>,
    /// Sound captions on screen (subtitles.rs).
    subtitles: Vec<subtitles::Subtitle>,
    /// Hearts: the health last frame, and when the last healing began (its wave runs across).
    last_health: f32,
    /// Doors and chest lids swinging (swing.rs).
    swings: swing::Swings,
    /// What armor stands wear (stands.rs).
    stands: stands::Stands,
    /// The test world's layout, when one is open (testworld.rs).
    testw: testworld::TestWorld,
    heal_wave: f32,
    /// Action waiting for a button press in the controller menu.
    rebinding: Option<Action>,
    /// Actions held this frame through the controller.
    pad_held: HashSet<Action>,
    pad_back: bool,
    pad_scroll_cd: f32,

    mobs: mobs::Mobs,
    entities: headless::Gpu<entity::EntityRenderer>,
    /// Survival mode: health, damage, no flying.
    survival: bool,
    health: f32,
    /// 20 at the start, up to 40 with life crystals.
    max_health: f32,
    magic: magic::Magic,
    /// What spells do to the world (elements.rs): spells' blocks that thaw back later.
    elements: elements::Elements,
    skills: skills::Skills,
    prof: prof::Prof,
    presets: presets::PresetState,
    atlas: atlas::Atlas,
    vulcao: vulcao::Vulcao,
    boss: bosses::BossState,
    /// The Terraria side (Etapa 5): accessories, glyph spells, hard mode and its events.
    t5: terraria::Terraria,
    ne: nether_end::NetherEnd,
    town: npcs::Town,
    progress: progress::Progress,
    raid: raids::Raid,
    villages: villages::Villages,
    /// Wagons and carts on the roads and parked (ambience, not saved).
    wagons: wagons::Wagons,
    map: explore::MapCache,
    /// Saved tame animals not back in the world yet.
    pets_waiting: Vec<pets::PetSave>,
    signs: std::collections::HashMap<[i32; 3], String>,
    sign_edit: Option<IVec3>,
    cmdblocks: cmdblock::CmdBlocks,
    /// WebAssembly mods from `plugins/` (plugins.rs).
    plugins: plugins::Plugins,
    /// Body temperature, thirst and food groups (nature.rs).
    nature: nature::Nature,
    bench: bench::Bench,
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
    fluids: fluids::Fluids,
    redstone: redstone::Redstone,
    /// Lit TNT, falling and flashing until it goes off.
    tnt: Vec<redstone::PrimedTnt>,
    /// Seconds until hoppers next move an item.
    hopper_t: f32,
    /// The cart the player sits in.
    riding: Option<usize>,
    /// The boat the player sits in.
    boating: Option<usize>,
    /// A creature being named with a name tag, and the name so far.
    naming: Option<(usize, String)>,
    /// The chair the player sits on.
    seat: Option<IVec3>,
    /// Lying in a bed (beds.rs).
    sleep: Option<beds::Sleep>,
    /// Which dimension is loaded, time spent standing in a portal, and where a trip lands.
    dim: voxel::worldgen::Dim,
    portal_t: f32,
    arrival: Option<Vec3>,
    /// Died in the Nether: once the Overworld is loaded, come back to life there.
    respawn_home: bool,
    /// Coming home from the End: land on the ground (at the bed, if any), not in a portal.
    arrive_plain: bool,
    /// Fractional count of ambient blocks still to look at (ambient.rs).
    ambient_t: f32,
    life: ambient_life::Life,
    room_t: f32,
    walker_t: f32,
    melody_cd: f32,
    /// A town NPC playing an instrument (instruments.rs).
    musicians: instruments::Musicians,
    /// Flames owed to the held torch (fractions carry over between frames), and a count of
    /// them for the sparks and smoke in between (torch.rs).
    /// Fire blocks' flames and embers (fire_fx.rs).
    fire_fx: fire_fx::FireFx,
    /// Red glow and sparks off lit redstone (redstone_fx.rs).
    redstone_fx: redstone_fx::RedstoneFx,
    torch_fx: f32,
    torch_sparks: u32,
    /// Screenshots: the held torch's fire has been lit up once already.
    torch_lit: bool,
    /// Sounds and effects of the immersion stage (soundscape.rs, effects.rs).
    fx: soundscape::Immersion,
    wander: wanderers::Wanderers,
    /// The mod content pack's state (mods/).
    mods: mods::Mods,
    /// The wind turning windmill sails (windmill.rs).
    sails: windmill::Sails,
    copper: copper::Copper,
    farm: farm::Farm,
    pots: pots::Pots,
    bobber: Option<fishing::Bobber>,
    weather: weather::Weather,
    /// Raindrops on the screen.
    wet: screen_drops::Wet,
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
    /// Where the controller and shader menus return to.
    options_back: Screen,
    /// The options screen: open tab, scroll and the slider being dragged (options.rs).
    opts: options::OptState,
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
    structures: Arc<voxel::structures::Structures>,
    /// First texture layer of the item sprites (they come after the block textures).
    item_base: u16,
    surv: survival::Survival,
    /// Phase 6: effects, stations, enchantment play, statistics and advancements.
    prog: progression::Prog,
    /// Lote 1 vanilla things: fire, jukebox, books, shields...
    extras: vanilla_extras::Extras,
    /// Eyeblossoms and creaking hearts (pale.rs).
    pale: pale::Pale,
    /// Lote 2 vanilla creatures' memory (allays, cures, spawners...).
    vm: vanilla_mobs::VMobs,
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

    last: Instant,
    now: f32,
    /// Clock of the world's moving looks (water, plants, rain, clouds): stops while the
    /// single-player game is paused.
    fx_time: f64,
    /// How far the wind has carried the clouds (blocks at speed 1) and the classic clouds' drift.
    /// Added up frame by frame so the speed never depends on how long the game has run.
    cloud_wind: [f32; 2],
    cloud_drift: f32,
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
    /// Multiplayer: hosting, or a guest in someone else's world.
    net: multiplayer::Net,
    voice: voice::VoiceState,
}

fn level_path_hint(base: &Path, name: &str) -> PathBuf {
    base.join("saves").join(name)
}

impl Game {
    /// `window` is None for the dedicated server, which never draws.
    fn new(window: Option<Arc<Window>>, args: Args) -> Game {
        let base = base_dir();
        let mut loaded_plugins = plugins::load(&base);
        let settings_path = base.join("settings.ron");
        // The very first start picks graphics for this computer (presets.rs).
        let first_run = !settings_path.exists();
        let mut settings = Settings::load(&settings_path);
        settings.controller.fill_missing();
        if let Some(rd) = args.render_distance {
            settings.render_distance = rd;
        }
        if let Some(l) = args.lod {
            settings.lod_distance = l.max(0);
        }
        if let Some(d) = args.lod_detail {
            settings.lod_detail = d.min(lod::DETAILS.len() as u8 - 1);
        }
        if let Some(m) = std::env::var("MC_MATERIALS").ok().and_then(|v| v.parse::<u8>().ok()) {
            // Test hook: the materials option (0 off, 1 simple, 2 full) for comparison shots.
            settings.materials = m.min(2);
        }
        if let Some(p) = args.parallax {
            settings.parallax = p.min(3);
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
        let (reg, mut items) = (Arc::new(reg), items);
        // Plugins' items go after everyone else's.
        loaded_plugins.base_items = items.items.len();
        loaded_plugins.register_items(&mut items);
        // Resource pack block models; the pictures only they use go right after ours.
        let pack_models = packmodels::load(&reg, &base, &settings.resource_packs);
        let item_base = (reg.textures.len() + pack_models.textures.len()) as u16;
        let texture_names: Vec<String> =
            reg.textures.iter().chain(pack_models.textures.iter()).chain(items.textures.iter()).cloned().chain(extra_textures()).collect();

        // Without --world the game opens on the title screen, with the last played world behind it.
        let camera_mode = args.camera.min(2);
        let show_perf = args.perf;
        let start_prone = args.prone;
        let title_mode = !args.server && args.bench.is_none() && (args.title || (args.world.is_none() && args.screenshot.is_none()));
        let world_name = args.world.clone().or_else(|| menus::list_worlds(&base).first().map(|w| w.name.clone())).unwrap_or_else(|| "Mundo 1".into());
        let save = Arc::new(WorldSave::new(level_path_hint(&base, &world_name)));
        let level = save.load_level();
        let start_dim = level.as_ref().map(|l| voxel::worldgen::Dim::from_u8(l.dim)).unwrap_or_default();
        let seed = level.as_ref().map(|l| l.seed).or(args.seed).unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
        });
        let structures = load_structures(&base, &reg);
        let gen = Arc::new(Generator::new(seed, &reg).with_structures(structures.clone()).with_spread(settings.structure_spread()));
        let tints: Vec<[f32; 3]> = (0..Biome::COUNT).map(|b| Biome::from_u8(b as u8).tint()).collect();
        let tables = Arc::new(MeshTables::new(&reg, tints));
        tables.set_models(pack_models.models);
        tables.set_flora(pack_models.flora);
        let reg_for_fluids = reg.clone();
        let mut world = World::new(reg.clone(), gen.clone(), save.clone(), tables);
        world.headless = window.is_none();

        let tex = textures::load(&texture_names, &base, &settings.resource_packs, settings.texture_resolution);
        let textures_name = tex.packs.first().cloned().unwrap_or_else(|| "Padrão".into());
        let lod = lod::Lod::new(gen.clone(), lod::LodColors::new(&reg, &tex));
        let font = FontAtlas::new();
        let mut renderer = window.clone().map(|w| pollster::block_on(Renderer::new(w, settings.vsync, &tex, &font)));
        if let Some(r) = renderer.as_mut() {
            r.cave_cull = !args.no_cave_cull;
            // Test hook: how near plants keep their full body (the far LOD shows past it).
            if let Some(d) = std::env::var("MC_FLORA_NEAR").ok().and_then(|v| v.parse().ok()) {
                r.flora_near = d;
            }
        }
        let mut ui = UiBatch::new(&font);
        ui.gui = tex.gui.sprites.clone();
        let mut audio = if args.server { None } else { Audio::new(&settings, &base) };
        if let Some(a) = audio.as_mut() {
            a.set_pack_sounds(textures::pack_sounds(&base, &settings.resource_packs, &audio::PACK_KEYS));
        }

        // Mobs: a mobs.ron on disk wins, like blocks.
        let [mobs_text, mods_mobs] = read_pack_pair(&base, "mobs.ron", embedded::MOBS_RON, embedded::MODS_MOBS_RON);
        let mobs_text = terraria::with_extra(&merge_list(&mobs_text, &fase6_text(Some(&base), "mobs.ron")), embedded::T5_MOBS_RON);
        let mobs_text = terraria::with_extra(&terraria::with_extra(&terraria::with_extra(&terraria::with_extra(&mobs_text, embedded::TR_MOBS_RON), embedded::AGUAS_MOBS_RON), embedded::VULCAO_MOBS_RON), embedded::TR3_MOBS_RON);
        let [_, dims_mobs] = read_pack_pair(&base, "dims_mobs.ron", "", embedded::MODS_DIMS_MOBS_RON);
        let mut mob_reg = voxel::mob::MobRegistry::from_ron_many(&[&mobs_text, &mods_mobs, &dims_mobs]).unwrap_or_else(|e| {
            log::error!("{e}; using built-in mobs");
            voxel::mob::MobRegistry::from_ron_many(&[embedded::MOBS_RON, embedded::MODS_MOBS_RON, embedded::MODS_DIMS_MOBS_RON]).unwrap_or_default()
        });
        // Real animals' own bodies (tools/gen_fauna.py), over whatever model their file gave them.
        let fauna = std::fs::read_to_string(base.join("packs/vanilla/fauna/models.ron")).unwrap_or_else(|_| embedded::FAUNA_MODELS_RON.to_string());
        if let Err(e) = mob_reg.apply_models(&fauna).or_else(|_| mob_reg.apply_models(embedded::FAUNA_MODELS_RON)) {
            log::error!("{e}");
        }
        let painting_defs = paintings::load_defs(&base);
        let paintings = paintings::load(&save.dir, &painting_defs);
        let surv = survival::Survival::load(&save.dir, &items);
        let prog = progression::Prog::load(&save.dir);
        let pale = pale::Pale::new(&reg);
        let mut extras = vanilla_extras::Extras::new(&reg);
        extras.load(&save.dir, &level_path_hint(&base, &world_name));
        let entities = renderer.as_ref().map(|r| {
            let mut e = entity::EntityRenderer::new(r, &mob_reg, &painting_defs, &base, settings.texture_resolution);
            e.items.set_materials(r, &texture_names);
            r.fx.set_block_materials(&r.queue, &texture_names);
            e
        });
        let mut mobs = mobs::Mobs::new(mob_reg, reg.id_or_air("grass_block"), seed);
        mobs.enabled = settings.mobs;
        mobs.peaceful = settings.difficulty == 0;
        mobs.set_staged(!settings.classic_mobs);

        let (sx, sy, sz) = gen.find_spawn();
        let mut body = Body::new(Vec3::new(sx as f32 + 0.5, sy as f32, sz as f32 + 0.5));
        let (mut yaw, mut pitch) = (0.0, 0.0);
        let mut hotbar_names: Vec<String> =
            ["grass_block", "dirt", "stone", "cobblestone", "oak_planks", "oak_log", "glass", "torch", "glowstone"].iter().map(|s| s.to_string()).collect();
        let mut time_of_day = 1000.0;
        let mut weather_save = weather::WeatherSave::default();
        let mut needs_spawn = true;
        let mut survival = args.survival;
        let mut health = 20.0;
        let mut max_health = 20.0f32;
        let mut max_mana = 20.0f32;
        let mut bosses_beaten = Vec::new();
        let mut town_npcs = Vec::new();
        if let Some(l) = &level {
            max_mana = l.max_mana;
            bosses_beaten = l.bosses.clone();
            town_npcs = l.npcs.iter().map(|(n, h)| npcs::NpcSave { name: n.clone(), home: *h }).collect();
            survival = l.survival || args.survival;
            max_health = l.max_health.clamp(20.0, 50.0);
            health = if l.health > 0.0 { l.health.min(max_health) } else { max_health };
            body.pos = Vec3::from(l.player_pos);
            body.flying = l.flying;
            yaw = l.player_yaw;
            pitch = l.player_pitch;
            time_of_day = l.time_of_day;
            weather_save = weather::WeatherSave { raining: l.raining, thunder: l.thunder, timer: l.weather_timer };
            if l.hotbar.len() == 9 {
                hotbar_names = l.hotbar.clone();
            }
            needs_spawn = false;
        }
        if let Some(p) = args.pos {
            body.pos = p;
            needs_spawn = false;
        }
        // Test screenshots: stand south of the nearest building of that name, looking at it.
        if let Some(name) = &args.find {
            if let Some((o, size)) = gen.find_structure(name, body.pos.x as i32, body.pos.z as i32, 40) {
                body.pos = Vec3::new(o.x as f32 + size.x as f32 / 2.0, o.y as f32 + size.y as f32 * 0.6 + 2.0, o.z as f32 + size.z as f32 + 9.0);
                // MC_INSIDE: stand in the middle of its floor instead (buried crypts).
                if std::env::var("MC_INSIDE").is_ok() {
                    body.pos = Vec3::new(o.x as f32 + size.x as f32 / 2.0, o.y as f32 + 1.0, o.z as f32 + size.z as f32 - 2.5);
                }
                needs_spawn = false;
                body.flying = true;
                log::info!("found {name} at {o}");
            } else {
                log::warn!("no {name} nearby");
            }
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
        textures::extract_pack_shaders(&base, &settings.resource_packs);
        let (shader_packs, pack_errors) = shaderpack::list_packs(&base.join("shaderpacks"));
        for e in &pack_errors {
            log::warn!("shader pack: {e}");
        }

        log::info!("world '{world_name}' seed {seed}, {} blocks, {} textures ({}px)", reg.len(), reg.textures.len(), renderer.as_ref().map_or(0, |r| r.texture_size));
        log::info!("{} interface pictures from packs", ui.gui.len());
        let now = Instant::now();
        if args.rain || args.storm {
            weather_save = weather::WeatherSave { raining: true, thunder: args.storm, timer: 600.0 };
        }
        let no_hud = args.no_hud;
        let mut game = Game {
            window: headless::Gpu::new(window),
            renderer: headless::Gpu::new(renderer),
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
            slide_dist: 0.0,
            sink: 0.0,
            rustle_dist: 0.0,
            pads: Controllers::new(),
            key_map: keybinds::KeyMap::default(),
            key_rebinding: None,
            subtitles: Vec::new(),
            last_health: 0.0,
            swings: Default::default(),
            stands: Default::default(),
            testw: Default::default(),
            pale,
            heal_wave: -10.0,
            rebinding: None,
            pad_held: HashSet::new(),
            pad_back: false,
            pad_scroll_cd: 0.0,
            mobs,
            entities: headless::Gpu::new(entities),
            survival,
            health,
            max_health,
            magic: magic::Magic::new(max_mana),
            elements: Default::default(),
            skills: Default::default(),
            prof: Default::default(),
            presets: Default::default(),
            atlas: Default::default(),
            vulcao: Default::default(),
            boss: bosses::BossState::new(bosses_beaten),
            t5: terraria::Terraria::default(),
            ne: nether_end::NetherEnd::default(),
            town: npcs::Town::new(town_npcs),
            progress: progress::Progress::new(),
            raid: raids::Raid::default(),
            villages: villages::Villages::default(),
            wagons: wagons::Wagons::default(),
            map: explore::MapCache::default(),
            pets_waiting: pets::load(&save.dir),
            signs: signs::load(&save.dir),
            cmdblocks: cmdblock::load(&save.dir),
            plugins: loaded_plugins,
            nature: nature::load(&save.dir),
            bench: Default::default(),
            sign_edit: None,
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
            bobber: None,
            fluids: fluids::Fluids::new(&reg_for_fluids),
            redstone: redstone::Redstone::new(reg_for_fluids.clone()),
            tnt: Vec::new(),
            hopper_t: 0.0,
            riding: None,
            boating: None,
            naming: None,
            seat: None,
            sleep: None,
            dim: voxel::worldgen::Dim::Overworld,
            portal_t: 0.0,
            arrival: None,
            respawn_home: false,
            arrive_plain: false,
            ambient_t: 0.0,
            life: Default::default(),
            room_t: 0.0,
            walker_t: 0.0,
            melody_cd: 0.0,
            musicians: Default::default(),
            fire_fx: Default::default(),
            redstone_fx: Default::default(),
            torch_fx: 0.0,
            torch_sparks: 0,
            torch_lit: false,
            fx: soundscape::Immersion::new(),
            wander: Default::default(),
            mods: Default::default(),
            sails: Default::default(),
            copper: copper::Copper::new(&reg_for_fluids),
            farm: farm::Farm::new(&reg_for_fluids),
            pots: pots::Pots::new(&reg_for_fluids),
            weather: weather::Weather::new(weather_save, seed),
            wet: screen_drops::Wet::new(seed),
            anim: player_model::Animator::default(),
            camera_mode,
            world_open: !title_mode,
            title_world: world_name,
            title_spin: 0.0,
            worlds: Vec::new(),
            world_sel: None,
            new_world: menus::NewWorldForm::default(),
            options_back: Screen::Paused,
            opts: Default::default(),
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
            structures,
            item_base,
            surv,
            prog,
            extras,
            vm: Default::default(),
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
            // Test hook for clean pictures: MC_NOHUD=1 also starts with the interface hidden (F1).
            hide_hud: no_hud || std::env::var("MC_NOHUD").is_ok(),
            toast: None,
            packs,
            textures_name,
            shader_packs,
            shader_error: None,
            shader_scroll: 0,
            lod,
            last: now,
            now: 0.0,
            fx_time: 0.0,
            cloud_wind: [0.0; 2],
            cloud_drift: 0.0,
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
            net: Default::default(),
            voice: Default::default(),
        };
        if first_run && game.args.screenshot.is_none() && game.args.bench.is_none() && !game.args.server {
            game.first_run_preset();
        }
        game.apply_shader();
        if start_dim == voxel::worldgen::Dim::Overworld && game.world.save.dir.join(testworld::MARKER).exists() {
            // A test world is laid out again on opening (testworld.rs).
            let (name, open) = (game.world_name.clone(), game.world_open);
            let (pos, yaw, pitch, spawn) = (game.body.pos, game.yaw, game.pitch, game.needs_spawn);
            game.open_world(&name, None, None);
            game.world_open = open;
            if game.args.pos.is_some() {
                game.body.pos = pos;
                game.needs_spawn = spawn;
            }
            if game.args.look.is_some() {
                (game.yaw, game.pitch) = (yaw, pitch);
            }
        }
        if start_dim != voxel::worldgen::Dim::Overworld {
            game.load_dim(start_dim, false);
        }
        if std::env::var("MC_END").is_ok() && game.dim == voxel::worldgen::Dim::Overworld {
            // Test hook: arrive in the End on its platform.
            game.load_dim(voxel::worldgen::Dim::End, true);
            game.arrival = Some(game.end_arrival());
            game.body.pos = game.arrival.unwrap();
            game.needs_spawn = false;
            if let Some(name) = game.args.find.clone() {
                // Test hook: look at a structure on the far islands.
                if let Some((o, size)) = game.world.gen.find_structure(&name, 1500, 0, 60) {
                    game.arrival = None;
                    game.body.pos = Vec3::new(o.x as f32 + size.x as f32 / 2.0, o.y as f32 + size.y as f32 * 0.6 + 2.0, o.z as f32 + size.z as f32 + 9.0);
                    game.body.flying = true;
                    log::info!("found {name} at {o}");
                }
            }
        }
        if std::env::var("MC_NETHER").is_ok() && game.dim == voxel::worldgen::Dim::Overworld {
            // Test hook: arrive in the Nether through a fresh portal.
            let p = game.body.pos;
            game.load_dim(voxel::worldgen::Dim::Nether, true);
            game.arrival = Some(Vec3::new(p.x / 8.0, 70.0, p.z / 8.0));
            game.body.pos = game.arrival.unwrap();
            game.needs_spawn = false;
            let want = match std::env::var("MC_NETHER").as_deref() {
                Ok("warped") => Some(voxel::worldgen::Biome::WarpedForest),
                Ok("deltas") => Some(voxel::worldgen::Biome::BasaltDeltas),
                _ => None,
            };
            if let Some(want) = want {
                // Test hook: stand on a floor in the nearest warped forest (or basalt deltas).
                let (x0, z0) = (game.body.pos.x as i32, game.body.pos.z as i32);
                'search: for r in (0..2000).step_by(16) {
                    for k in 0..(r / 8).max(1) {
                        let a = k as f32 / (r / 8).max(1) as f32 * std::f32::consts::TAU;
                        let (x, z) = (x0 + (a.cos() * r as f32) as i32, z0 + (a.sin() * r as f32) as i32);
                        if game.world.gen.biome_at(x, z) != want {
                            continue;
                        }
                        if let Some(y) = game.world.gen.nether_floor_near(x, 40, z) {
                            game.arrival = None;
                            game.body.pos = Vec3::new(x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5);
                            log::info!("warped forest at {x},{y},{z}");
                            break 'search;
                        }
                    }
                }
            }
            if let Some(name) = game.args.find.clone() {
                let (x, z) = (game.body.pos.x as i32, game.body.pos.z as i32);
                if let Some((o, size)) = game.world.gen.find_structure(&name, x, z, 40) {
                    game.arrival = None;
                    game.body.pos = Vec3::new(o.x as f32 + size.x as f32 / 2.0, o.y as f32 + size.y as f32 * 0.6 + 2.0, o.z as f32 + size.z as f32 + 7.0);
                    game.body.flying = true;
                    log::info!("found {name} at {o}");
                }
            }
        }
        if let Ok(what) = std::env::var("MC_DEEP") {
            // Test hook: somewhere deep down (deep.rs).
            game.test_deep(&what);
        }
        game.load_mods();
        game.mods_dim_test_hook();
        game.key_map = keybinds::KeyMap::new(&game.settings.key_binds);
        game.apply_language();
        // Test hook: open a menu for screenshots (MC_SCREEN=options or keys).
        match std::env::var("MC_SCREEN").as_deref() {
            Ok("options") => game.set_screen(Screen::Options),
            Ok("paused") => game.set_screen(Screen::Paused),
            Ok("keys") => game.set_screen(Screen::Keys),
            Ok("voice") => game.set_screen(Screen::Voice),
            _ => {}
        }
        game
    }

    /// Load the chosen language, and teach it the names of every block, item and mob.
    pub(crate) fn apply_language(&mut self) {
        for b in &self.reg.blocks {
            lang::add_name(&b.display_name, &b.name);
        }
        for i in &self.items.items {
            lang::add_name(&i.display_name, &i.name);
        }
        for d in &self.mobs.reg.defs {
            lang::add_name(&d.display_name, &d.name);
        }
        lang::set_language(&self.settings.language, &self.base);
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
        let Some(r) = self.renderer.get_mut() else { return };
        r.post.taa = self.settings.taa;
        if let Err(e) = r.set_shader(pack.as_ref(), self.settings.shader_quality) {
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
        if let Some(r) = self.renderer.get_mut() {
            r.post.options = values;
        }
    }

    /// Name for the pause menu and the F3 screen.
    fn shader_name(&self) -> String {
        match (self.active_shader(), &self.shader_error) {
            (_, Some(_)) => format!("{} (erro)", self.settings.shader_label()),
            (Some(p), None) => p.def.name.clone(),
            (None, None) => "Desligado".to_string(),
        }
    }

    /// Block depth for the terrain shader (`Globals::pom`). Off on the "Rápido" quality. A shader
    /// pack can scale it with an option named "profundidade" (1 = as chosen in the menu).
    fn parallax_uniform(&self) -> [f32; 4] {
        let depth = settings::PARALLAX_LEVELS[self.settings.parallax.min(3) as usize].1;
        if depth <= 0.0 || self.settings.shader_quality == ShaderQuality::Fast {
            return [0.0; 4];
        }
        let scale = self
            .active_shader()
            .and_then(|p| {
                let i = p.def.options.iter().take(shaderpack::MAX_OPTIONS).position(|o| o.name() == "profundidade")?;
                Some(p.values(self.settings.shader_options.get(&p.id))[i])
            })
            .unwrap_or(1.0);
        // Steps grow with the depth; the cracks' own shadows come with the sun shadows.
        [depth * scale.max(0.0), 10.0 + depth * 60.0, 20.0, (self.settings.shadows > 0) as u8 as f32]
    }

    /// Terrain shader extras switched on by the built-in "Realçado" pack.
    fn terrain_effects(&self) -> [f32; 4] {
        let o = &self.renderer.post.options;
        match self.renderer.post.builtin() {
            Some(Builtin::Realcado) => [if o[5] > 0.5 { 2.2 } else { 1.0 }, o[6], 1.6 * o[0].min(1.0), 0.0],
            Some(Builtin::Suave) => [1.0, 0.0, 0.8 * o[0], 0.0],
            // Without a shader pack the water ripples and reflects when Reflexos na água is on.
            _ => [1.0, if self.settings.water_reflections { 1.0 } else { 0.0 }, 0.0, 0.0],
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

    /// Raindrops landing on the screen out in the rain (screen_drops.rs).
    fn update_wet_screen(&mut self, eye: Vec3, dt: f32) {
        let (x, z) = (eye.x.floor() as i32, eye.z.floor() as i32);
        let playing = self.world_open && matches!(self.screen, Screen::Playing | Screen::Inventory);
        let here = self.world.gen.base_biome_at(x, z);
        let dry = self.weather.snowing_here || matches!(here, voxel::worldgen::Biome::Desert | voxel::worldgen::Biome::Badlands);
        let e = screen_drops::Exposure {
            rain: if playing && !dry && self.dim == voxel::worldgen::Dim::Overworld { self.weather.rain } else { 0.0 },
            look_y: self.forward().y,
            roofed: self.world.surface_y(x, z).is_some_and(|y| y as f32 + 1.0 > eye.y),
            submerged: self.body.head_in_water,
            level: self.settings.rain_drops,
        };
        // The pause menu stops them too; a screenshot gets them settled first.
        let wdt = if self.world_paused() { 0.0 } else { dt };
        if let Some(secs) = self.args.screenshot.as_ref().and(std::env::var("MC_DROPS").ok()).and_then(|v| v.parse::<f32>().ok()) {
            if self.wet.drops.is_empty() && e.rain > 0.0 {
                for _ in 0..(secs * 30.0) as usize {
                    self.wet.update(&e, 1.0 / 30.0);
                }
            }
        } else {
            self.wet.update(&e, wdt);
        }
        if let Some(r) = self.renderer.get_mut() {
            self.wet.aspect = r.config.width as f32 / r.config.height.max(1) as f32;
            let d = self.wet.instances(r.config.height as f32);
            let q = &r.queue;
            r.drops.set(q, &d);
        }
    }

    fn forward(&self) -> Vec3 {
        Vec3::new(-self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), -self.yaw.cos() * self.pitch.cos())
    }

    fn set_grab(&mut self, grab: bool) {
        if self.window.get().is_none() {
            return;
        }
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
        self.surv.opened_at = None;
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
        if self.world_open {
            self.net_save();
        }
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
            max_health: self.max_health,
            max_mana: self.magic.max_mana - self.mods.spellbooks.mana_bonus,
            bosses: self.boss.defeated.clone(),
            npcs: self.town.npcs.iter().map(|n| (n.name.clone(), n.home)).collect(),
            raining: self.weather.state.raining,
            thunder: self.weather.state.thunder,
            weather_timer: self.weather.state.timer,
            dim: self.dim.as_u8(),
        };
        if self.screen != Screen::Loading || !self.needs_spawn {
            // level.ron always lives in the world's own folder, whatever dimension is loaded.
            let root = WorldSave::new(level_path_hint(&self.base, &self.world_name));
            if let Err(e) = root.save_level(&level) {
                log::error!("could not save the world: {e}");
            }
        }
        self.world.save_all();
        paintings::save(&self.world.save.dir, &self.paintings);
        signs::save(&self.world.save.dir, &self.signs);
        cmdblock::save(&self.world.save.dir, &self.cmdblocks);
        nature::save(&self.world.save.dir, &self.nature);
        self.save_pets();
        self.surv.save(&self.world.save.dir, &self.items);
        self.progress_save();
        self.t5_save();
        self.elements_save();
        self.skills_save();
        self.atlas_save();
        self.prog.save(&level_path_hint(&self.base, &self.world_name), &self.items);
        self.extras.save(&self.world.save.dir, &level_path_hint(&self.base, &self.world_name));
        self.save_mods();
        self.settings.save(&self.base.join("settings.ron"));
    }

    fn situation(&self) -> Situation {
        match self.screen {
            _ if self.is_title_screen() => Situation::Title,
            Screen::Loading => Situation::Title,
            _ if self.body.head_in_water => Situation::Underwater,
            _ if self.dim == voxel::worldgen::Dim::Nether => Situation::Nether,
            _ if self.layer() == voxel::worldgen::deep::Layer::Underworld => Situation::Nether,
            _ if self.dim == voxel::worldgen::Dim::End => Situation::End,
            _ => Situation::Creative,
        }
    }

    fn reload_textures(&mut self) {
        let pack_models = packmodels::load(&self.reg, &self.base, &self.settings.resource_packs);
        self.item_base = (self.reg.textures.len() + pack_models.textures.len()) as u16;
        let names: Vec<String> =
            self.reg.textures.iter().chain(pack_models.textures.iter()).chain(self.items.textures.iter()).cloned().chain(extra_textures()).collect();
        self.world.tables.set_models(pack_models.models);
        self.world.tables.set_flora(pack_models.flora);
        self.world.remesh_all();
        let tex = textures::load(&names, &self.base, &self.settings.resource_packs, self.settings.texture_resolution);
        self.textures_name = tex.packs.first().cloned().unwrap_or_else(|| "Padrão".into());
        if let Some(r) = self.renderer.get_mut() {
            r.reload_textures(&tex);
        }
        if let (Some(r), Some(e)) = (self.renderer.get(), self.entities.get_mut()) {
            e.items.set_materials(r, &names);
            r.fx.set_block_materials(&r.queue, &names);
        }
        self.ui.gui = tex.gui.sprites.clone();
        self.lod.set_colors(lod::LodColors::new(&self.reg, &tex));
        if let Some(a) = self.audio.as_mut() {
            a.set_pack_sounds(textures::pack_sounds(&self.base, &self.settings.resource_packs, &audio::PACK_KEYS));
        }
        if textures::extract_pack_shaders(&self.base, &self.settings.resource_packs) {
            self.reload_shader_packs();
        }
        self.toast(format!("Texturas: {}", self.textures_name), 3.0);
    }

    fn key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        if pressed {
            self.keys.insert(code);
        } else {
            self.keys.remove(&code);
            return;
        }
        if self.screen == Screen::Sign {
            if self.extras.book.is_some() {
                self.book_key(code);
            } else if self.cmdblocks.edit.is_some() {
                self.cmdblock_key(code);
            } else if self.naming.is_some() {
                self.naming_key(code);
            } else {
                self.sign_key(code);
            }
            return;
        }
        if self.net_key(code, pressed) {
            return;
        }
        if !repeat && self.voice_key(code) {
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
        if self.atlas_key(code) || self.progress_key(code) || self.skills_key(code) || self.advancements_key(code) || self.mods_key(code) {
            return;
        }
        let f3 = self.keys.contains(&KeyCode::F3);
        match code {
            KeyCode::Escape => match self.screen {
                Screen::Playing => self.set_screen(Screen::Paused),
                Screen::Paused | Screen::Inventory => self.set_screen(Screen::Playing),
                Screen::Controls => self.set_screen(self.options_back),
                Screen::Shaders => self.set_screen(self.options_back),
                Screen::Paintings | Screen::Sign | Screen::Grimoire | Screen::Plugin => {
                    self.plugins.screen = None;
                    self.extras.book = None;
                    self.set_screen(Screen::Playing);
                }
                Screen::Worlds | Screen::CreateWorld | Screen::Multiplayer | Screen::Options | Screen::Packs | Screen::Keys | Screen::Voice => self.menu_back(),
                Screen::Loading | Screen::Dead | Screen::Title => {}
                Screen::Friends => self.set_screen(Screen::Paused),
            },
            KeyCode::KeyE => match self.screen {
                Screen::Playing => self.surv_open_inventory(),
                Screen::Inventory | Screen::Grimoire => self.set_screen(Screen::Playing),
                _ => {}
            },
            KeyCode::KeyT if f3 => self.reload_textures(),
            KeyCode::KeyG if self.screen == Screen::Playing => self.glyph_cycle(),
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
            // Terraria-style quick keys (rpg.rs): heal, mana, buffs, and the stats sheet.
            KeyCode::KeyH if self.screen == Screen::Playing => self.quick_heal(),
            KeyCode::KeyN if self.screen == Screen::Playing => {
                if !self.quick_mana() {
                    self.toast("Nenhuma poção de mana ou mana já cheia", 1.5);
                }
            }
            KeyCode::KeyB if self.screen == Screen::Playing => self.quick_buff(),
            KeyCode::KeyI if self.screen == Screen::Playing => self.emote_key(),
            KeyCode::Digit0 if self.screen == Screen::Playing => {
                self.emote_pick(9);
            }
            KeyCode::KeyF if self.screen == Screen::Playing && self.survival => self.quick_hook(),
            KeyCode::KeyC if matches!(self.screen, Screen::Playing | Screen::Inventory) => self.t5.rpg.sheet = !self.t5.rpg.sheet,
            KeyCode::KeyR if self.screen == Screen::Inventory && self.survival => self.surv.want_recipe = true,
            KeyCode::KeyO if self.screen == Screen::Inventory && self.survival => self.sort_backpack(),
            KeyCode::KeyX if self.screen == Screen::Inventory && self.survival => self.quick_stack(),
            KeyCode::KeyU if self.screen == Screen::Inventory && self.survival => {
                self.surv.want_recipe = true;
                self.surv.uses_pending = true;
            }
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
            } else if !self.emote_pick(i) {
                self.selected = i;
                self.show_held_name();
            }
        }
    }

    fn show_held_name(&mut self) {
        if let Some(st) = self.held_stack() {
            let n = self.item_label(&st);
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
            Screen::Options => self.options_scroll(lines),
            Screen::Shaders => {
                if lines > 0.0 {
                    self.shader_scroll = self.shader_scroll.saturating_sub(1);
                } else {
                    self.shader_scroll += 1;
                }
            }
            Screen::Inventory if self.creative_screen_open() => self.creative_scroll(lines > 0.0),
            Screen::Inventory => {
                if self.shop_wheel(lines > 0.0) || self.wheel_move(lines > 0.0) {
                } else if lines > 0.0 {
                    self.surv.book_scroll = self.surv.book_scroll.saturating_sub(1);
                } else {
                    self.surv.book_scroll += 1;
                }
            }
            _ => {}
        }
    }


    /// Singleplayer: the pause menu (and the option screens opened from it) stop the world.
    /// With friends in it, it keeps going.
    fn world_paused(&self) -> bool {
        self.world_open && !self.net.active() && matches!(self.screen, Screen::Paused | Screen::Friends | Screen::Options | Screen::Controls | Screen::Packs | Screen::Keys | Screen::Voice | Screen::Shaders)
    }

    /// Moves the clouds with the wind: one direction for every layer, turning very slowly,
    /// faster in the rain. The shader multiplies this by each layer's speed.
    fn advance_clouds(&mut self, dt: f32) {
        self.fx_time += dt as f64;
        let a = 0.35 + ((self.fx_time * 0.0021).sin() * 0.5) as f32;
        let k = dt * (1.0 + self.weather.rain * 0.8);
        self.cloud_wind[0] += a.cos() * k;
        self.cloud_wind[1] += a.sin() * k;
        self.cloud_drift += dt;
    }

    fn update(&mut self, dt: f32) {
        self.now += dt;
        self.update_subtitles(dt);
        if self.health > self.last_health + 0.05 && self.last_health > 0.0 && self.now - self.heal_wave > 0.8 {
            self.heal_wave = self.now;
        }
        self.last_health = self.health;
        // Peaceful: health comes back on its own.
        if self.survival && self.settings.difficulty == 0 && self.screen != Screen::Dead && self.health > 0.0 {
            self.health = (self.health + dt).min(self.max_health);
        }
        let rd = self.settings.render_distance;
        let eye = self.body.eye();
        let center = ChunkPos::new((eye.x.floor() as i32) >> 4, (eye.z.floor() as i32) >> 4);
        self.world.set_focus(eye.y);
        self.world.update(center, rd);
        self.prof.mark("mundo: chunks");
        self.swing_update();
        let events = std::mem::take(&mut self.world.events);
        if let Some(r) = self.renderer.get_mut() {
            r.apply_mesh_events(events);
        }
        self.prof.mark("malhas novas");
        if self.world_open && self.screen == Screen::Playing {
            self.plugins_tick(dt);
        }
        self.net_update(dt);
        self.voice_update(dt);
        self.prof.mark("rede e voz");
        // A guest in the host's dimension gets the world's own changes from the host.
        let sim = self.net_simulates();
        if self.world_open && self.screen != Screen::Loading {
            let shot = self.args.screenshot.is_some();
            // The pause menu stops the weather and the windmills too.
            let wdt = if self.world_paused() { 0.0 } else { dt };
            self.weather.update(&self.world, eye, wdt, shot);
            self.windmills_update(wdt);
            if self.dim != voxel::worldgen::Dim::Overworld {
                // No weather under the Nether's roof (it carries on at home).
                self.weather.rain = 0.0;
                self.weather.flash = 0.0;
                self.weather.columns.clear();
                self.weather.thunder_sound = None;
                self.weather.rain_sound = false;
            }
            if let Some(close) = self.weather.thunder_sound {
                self.sfx(Sfx::Thunder(close));
                self.lightning_trap();
            }
            if self.weather.rain_sound {
                let vol = self.settings.sound_volume * 0.5 * self.weather.rain;
                if let Some(a) = self.audio.as_mut() {
                    a.play(Sfx::Rain, vol);
                }
            }
            if self.weather.snowing_here && self.weather.rain > 0.9 {
                self.settle_snow(dt);
            } else if self.weather.rain > 0.3 {
                self.rain_splashes(dt);
            }
            if sim {
                self.update_leaf_decay(dt);
            }
            if matches!(self.screen, Screen::Playing | Screen::Inventory) {
                self.portal_step(dt);
                self.update_ambient(if self.args.screenshot.is_some() { 0.0 } else { dt });
                self.update_life(if self.args.screenshot.is_some() { 0.0 } else { dt });
                self.update_fire_fx(dt, self.args.screenshot.is_some());
                self.update_redstone_fx(dt, self.args.screenshot.is_some());
                self.update_musicians(dt);
                self.update_immersion(if self.args.screenshot.is_some() { 0.0 } else { dt });
            }
            self.update_fishing(dt);
            self.update_carts(dt);
            self.update_boats(dt);
            self.update_wagons(dt);
            if sim {
                self.update_redstone(dt);
                self.update_random_ticks(dt);
                self.update_farmers(dt);
                self.update_walkers(dt);
                let fe = self.fluids.update(&mut self.world, dt);
                if let Some(p) = fe.hardened.first() {
                    if p.as_vec3().distance(self.body.pos) < 24.0 {
                        self.sfx(Sfx::Fizz);
                    }
                }
            } else {
                self.world.changed.clear();
            }
        } else {
            self.world.changed.clear();
            self.weather.columns.clear();
        }
        self.update_wet_screen(eye, dt);
        if let Some(r) = self.renderer.get_mut() {
            r.set_rain(&self.weather.columns);
        }

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
            if ready && (self.arrival.is_some() || self.respawn_home) {
                self.finish_travel();
            } else if ready {
                if self.needs_spawn {
                    let y = self.world.surface_y(sx, sz).unwrap_or(100) + 1;
                    self.body.pos = Vec3::new(sx as f32 + 0.5, y as f32, sz as f32 + 0.5);
                    self.needs_spawn = false;
                }
                // Never start the game inside rock (the Nether test shortcuts land in closed caves).
                self.unstick();
                if self.args.demo {
                    self.build_demo();
                }
                if std::env::var("MC_FLORA").is_ok() {
                    self.build_flora_test();
                }
                if std::env::var("MC_SHADOWS").is_ok() {
                    self.build_shadow_test();
                }
                if std::env::var("MC_DOORS").is_ok() {
                    self.build_door_test();
                }
                if std::env::var("MC_BANNERS").is_ok() {
                    self.build_banner_test();
                }
                if self.args.shapes {
                    self.build_shapes();
                }
                if self.args.redstone {
                    self.build_redstone();
                }
                if self.args.phase4 {
                    self.build_phase4();
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
                if let Some(it) = std::env::var("MC_HOLD").ok().and_then(|n| self.items.id(&n)) {
                    // Test hook: hold this item (any item name) in the selected slot.
                    self.surv.inv.slots[self.selected] = Some(voxel::item::ItemStack::new(it, 1));
                }
                if let Ok(spec) = std::env::var("MC_SHIELD") {
                    // Test hook: a shield in the off hand, `item[:colour[:design 1..12:colour]]`
                    // (MC_SHIELD_MAIN puts it in the main hand instead).
                    self.test_shield(&spec, std::env::var("MC_SHIELD_MAIN").is_ok());
                }
                if std::env::var("MC_NOHUD").is_ok() {
                    // Test hook: no HUD, for clean third-person shots.
                    self.hide_hud = true;
                }
                if std::env::var("MC_AMBIENT").is_ok() {
                    // Test hook: a few seconds of ambient bits already in the air.
                    let secs: f32 = std::env::var("MC_AMBIENT").ok().and_then(|v| v.parse().ok()).unwrap_or(4.0);
                    for _ in 0..(secs * 20.0) as usize {
                        self.update_ambient(0.05);
                        self.update_life(0.05);
                        if self.weather.rain > 0.3 && !self.weather.snowing_here {
                            self.rain_splashes(0.05);
                        }
                        self.particles.update(&self.world, 0.05);
                    }
                    log::info!("ambient life after {secs}s: {}", self.particles.census());
                }
                if let Ok(what) = std::env::var("MC_LIFE") {
                    // Test hook: birds, a leaping fish, butterflies and bees in front (ambient_life.rs).
                    self.test_life(&what);
                }
                if std::env::var("MC_FARM").is_ok() {
                    // Test hook: a small field in front, some of it ripe.
                    self.test_farm();
                }
                if let Ok(name) = std::env::var("MC_BIOME") {
                    // Test hook: stand in the nearest biome of that name (e.g. "Meadow").
                    self.test_biome(&name);
                }
                if let Ok(kind) = std::env::var("MC_AGUAS") {
                    // Test hook: under the water of a lake, river, sea... full of its creatures.
                    self.test_aguas(&kind);
                }
                if let Ok(kind) = std::env::var("MC_CAVE") {
                    // Test hook: stand in a cave biome.
                    self.test_cave(&kind);
                }
                if let Ok(scene) = std::env::var("MC_FASE6") {
                    // Test hook: a Phase 6 scene (stations, effects, advancements, mobs).
                    self.test_fase6(&scene);
                }
                if let Ok(scene) = std::env::var("MC_ELEMENTS") {
                    // Test hook: a pond and a spell cast into it (elements.rs; MC_PLACE=frames).
                    self.test_elements(&scene);
                }
                if std::env::var("MC_PALE").is_ok() {
                    // Test hook: the pale garden's and resin's new blocks.
                    self.test_pale();
                }
                if std::env::var("MC_DECOR").is_ok() {
                    // Test hook: the new building blocks.
                    self.test_decor();
                }
                if std::env::var("MC_POTS").is_ok() {
                    // Test hook: planted pots on hoppers.
                    self.test_pots();
                }
                if std::env::var("MC_END_RING").is_ok() {
                    // Test hook: a ring of end portal frames, the last eye just set.
                    self.test_end_ring();
                }
                if std::env::var("MC_PORTAL").is_ok() {
                    // Test hook: an obsidian frame in front, lit with flint and steel.
                    self.test_portal();
                }
                if std::env::var("MC_TRAVEL").is_ok() {
                    // Test hook: walk through a portal right away (saves, loads the Nether).
                    let p = self.body.pos;
                    self.travel(voxel::worldgen::Dim::Nether, Vec3::new(p.x / 8.0, 70.0, p.z / 8.0));
                    return;
                }
                if let Ok(v) = std::env::var("MC_SKILLS") {
                    // Test hook: the crafts' experience, their page open (skills.rs).
                    self.test_skills(&v);
                }
                if let Ok(s) = std::env::var("MC_VULCAO") {
                    // Test hook: a volcano scene (vulcao.rs: crater, slope, tube, erupt, titan, golem, far).
                    self.test_vulcao(&s);
                }
                if std::env::var("MC_ATLAS").is_ok() {
                    // Test hook: an atlas in hand with the land around inked, open (atlas.rs).
                    self.test_atlas();
                }
                if let Ok(n) = std::env::var("MC_PROGRESS") {
                    // Test hook: chapters before N done, the Guide in front (MC_TALK, MC_JOURNAL, MC_REWARD).
                    self.test_progress(n.parse().unwrap_or(0));
                }
                if let Ok(u) = std::env::var("MC_MANTLE") {
                    // Test hook: caught partway up a two-block ledge (MC_MANTLE_SHELF: a shelf).
                    self.test_mantle(u.parse().unwrap_or(0.4));
                }
                if std::env::var("MC_DROPS").is_ok() {
                    // Test hook: items on the ground in front, to see their names and glows.
                    self.test_drops();
                }
                if std::env::var("MC_HOUSE").is_ok() {
                    // Test hook: two small houses in front; people move in; MC_SHOP opens a shop.
                    self.test_houses();
                }
                if let Ok(boss) = std::env::var("MC_BOSS") {
                    // Test hook: a boss in front (MC_BOSS=name, or 1 for the Forest Guardian), hurt
                    // to show its bar and angry volley.
                    let boss = if boss.len() > 1 { boss } else { "forest_guardian".to_string() };
                    let f = self.forward();
                    let flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
                    let at = self.body.pos + flat * 14.0;
                    let y = self.world.surface_y(at.x.floor() as i32, at.z.floor() as i32).map(|y| y as f32 + 1.0).unwrap_or(at.y);
                    if self.mobs.spawn(&boss, Vec3::new(at.x, y, at.z)) {
                        let m = self.mobs.list.last_mut().unwrap();
                        m.yaw = (-flat.x).atan2(-flat.z);
                        m.health = m.max_health * 0.4;
                        self.boss.called_help = true;
                        for _ in 0..8 {
                            self.update_boss(0.5);
                            self.update_magic(0.03, false);
                            self.particles.update(&self.world, 0.03);
                        }
                    }
                }
                if let Ok(what) = std::env::var("MC_NE") {
                    // Test hook: the Nether and End's fights (shots, wither, dragon, glide, gateway).
                    self.nether_end_test(&what);
                }
                if let Ok(spell) = std::env::var("MC_CAST") {
                    // Test hook: a caster in hand, a few spells frozen in flight.
                    if let Some(id) = self.items.id(&spell) {
                        let sel = self.selected;
                        self.surv.inv.slots[sel] = Some(voxel::item::ItemStack::new(id, 1));
                        self.screen = Screen::Playing;
                        for k in 0..12 {
                            self.update_magic(0.03, k % 4 == 0);
                            self.particles.update(&self.world, 0.03);
                        }
                    }
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
                if let Ok(scene) = std::env::var("MC_T5") {
                    // Test hook: an Etapa 5 scene (terraria.rs, test_t5).
                    self.test_t5(&scene);
                }
                if let Ok(what) = std::env::var("MC_BED") {
                    // Test hook: every colour of bed, or asleep in one (beds.rs).
                    self.bed_test(&what);
                }
                if let Ok(scene) = std::env::var("MC_TR") {
                    // Test hook: the rest of the Terraria side (tr.rs, test_tr).
                    self.test_tr(&scene);
                }
                if let Ok(scene) = std::env::var("MC_RPG") {
                    // Test hook: stats, minions, debuffs (rpg.rs, test_rpg).
                    self.test_rpg(&scene);
                }
                if let Ok(scene) = std::env::var("MC_LOTE11") {
                    // Test hook: wings, mounts, pets, vanity, sentries (companions.rs).
                    self.test_lote11(&scene);
                }
                self.mods_test_hooks();
                if let Some(m) = self.args.mouse {
                    self.mouse_pos = m;
                }
                if self.args.show_inventory || self.surv.open == survival::Open::Shop || self.args.open.as_deref().is_some_and(|o| o != "corpse") {
                    self.set_screen(Screen::Inventory);
                } else if self.args.show_pause {
                    self.set_screen(Screen::Paused);
                } else if self.args.show_shaders {
                    self.set_screen(Screen::Shaders);
                } else {
                    self.set_screen(Screen::Playing);
                }
                self.toast("Bem-vindo! Esc = menu, E = inventário", 6.0);
                if std::env::var("MC_T5").as_deref() == Ok("grimoire") {
                    self.set_screen(Screen::Grimoire);
                }
            }
        }

        // Singleplayer: the pause menu (and the option screens opened from it) stop the world.
        // With friends in it, it keeps going.
        let paused = self.world_paused();
        if !paused && self.net.rules.daylight_cycle {
            self.time_of_day = (self.time_of_day + dt * 20.0) % 24000.0;
        }
        if !paused {
            self.advance_clouds(dt);
        }
        if self.args.time.is_some() && self.args.screenshot.is_some() {
            self.time_of_day = self.args.time.unwrap();
        }

        self.prof.mark("clima e ambiente");
        if self.is_title_screen() {
            self.update_title(dt);
        }
        if self.args.screenshot.is_none() && !paused {
            self.particles.update(&self.world, dt);
        }
        self.prof.mark("partículas");
        if self.screen == Screen::Playing || self.screen == Screen::Inventory {
            self.update_player(dt);
            self.update_survival(dt);
        }
        self.prof.mark("jogador");
        if self.world_open && !paused && self.screen != Screen::Loading {
            let use_held = self.mouse_held[1] || self.pad_held.contains(&Action::Use);
            self.surv_update(if self.args.screenshot.is_some() { 0.0 } else { dt }, use_held);
            self.prof.mark("sobrevivência");
            self.update_magic(if self.args.screenshot.is_some() { 0.0 } else { dt }, use_held);
            self.prof.mark("magia");
            self.update_elements(if self.args.screenshot.is_some() { 0.0 } else { dt });
            self.prof.mark("elementos");
            self.update_fase6(if self.args.screenshot.is_some() { 0.0 } else { dt }, use_held);
            self.prof.mark("fase 6 e extras");
            if sim {
                self.update_boss(if self.args.screenshot.is_some() { 0.0 } else { dt });
                self.update_progress(if self.args.screenshot.is_some() { 0.0 } else { dt });
                self.update_skills(if self.args.screenshot.is_some() { 0.0 } else { dt });
                self.update_atlas(dt);
                self.update_vulcao(dt);
            }
            self.prof.mark("chefes, progresso, ofícios");
            if sim && self.dim == voxel::worldgen::Dim::Overworld {
                // Townsfolk live at home.
                self.update_npcs(if self.args.screenshot.is_some() { 0.0 } else { dt });
                self.update_raids(dt);
                self.update_villages(dt);
                self.update_leads(dt);
                self.update_map(dt);
                self.update_pets(if self.args.screenshot.is_some() { 0.0 } else { dt });
                self.update_test_world();
                self.stands_update();
            }
            self.prof.mark("vilas, NPCs, mascotes");
            self.update_mods(if self.args.screenshot.is_some() { 0.0 } else { dt });
            self.prof.mark("mods");
        }
        let menus_stop = !self.net.active() && matches!(self.screen, Screen::Paused | Screen::Friends | Screen::Controls | Screen::Options | Screen::Packs | Screen::Keys | Screen::Voice | Screen::Shaders);
        if self.world_open && sim && self.screen != Screen::Loading && !menus_stop {
            let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
            let daylight = self.tr_daylight((a.sin() * 2.2 + 0.45).clamp(0.0, 1.0));
            let creative = !self.hunted();
            if self.screen == Screen::Playing && self.dim == voxel::worldgen::Dim::Overworld {
                self.update_wanderers(if self.args.screenshot.is_some() { 0.0 } else { dt }, daylight);
            }
            // Screenshots hold the world still so the picture is stable.
            let mdt = if self.args.screenshot.is_some() { social::film_dt() } else { dt };
            self.prof.mark("andarilhos");
            self.before_mobs();
            self.mobs.raining = self.dim == voxel::worldgen::Dim::Overworld && self.weather.rain > 0.2;
            let out = self.mobs.update(&self.world, self.body.pos, creative, daylight, mdt);
            self.prof.mark("mobs: IA");
            self.social_effects(&out);
            for (dmg, from) in out.damage {
                self.mob_hit_effects(from);
                self.rpg_mob_hit(from);
                self.damage(dmg, from);
            }
            self.net_remote_hits(&out.remote_hits);
            for (pos, power) in out.explosions {
                self.explode(pos, power);
            }
            for (from, vel, dmg, school) in out.fireballs {
                self.enemy_bolt(from, vel, school, dmg);
                self.sfx(Sfx::Cast(school));
            }
            self.update_nether_end(mdt, &out.special, &out.deaths);
            // A soft thrum when a bow is loosed, a wooden thunk when an arrow lands.
            if out.shots > 0 {
                self.sfx(Sfx::Place(Material::Cloth));
            }
            if out.arrow_hits > 0 {
                self.sfx(Sfx::Hit(Material::Wood));
            }
            for &(def, pos) in &out.deaths {
                self.rpg_drops(def, pos);
                let d = &self.mobs.reg.defs[def];
                if d.boss {
                    let (n, dn) = (d.name.clone(), d.display_name.clone());
                    self.boss_defeated(&n, &dn);
                }
            }
            self.progress_deaths(&out.deaths);
            self.tr_deaths(&out.deaths);
            // Mobs a friend finished off drop their loot for that friend.
            let deaths = self.net_kill_credit(&out.deaths);
            if self.survival {
                self.surv_mob_deaths(&deaths);
            }
            // After the mobs' own tick: worms, flyers, hard mode, accessories, glyph spells.
            let use_held = self.mouse_held[1] || self.pad_held.contains(&Action::Use);
            self.prof.mark("mobs: mortes e danos");
            self.update_t5(mdt, use_held && self.screen == Screen::Playing);
            self.prof.mark("Terraria (vermes, acessórios...)");
            self.fase6_mob_deaths(&out.deaths);
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
        // Mob animation runs on frame time (smooth for guests too), and holds still with the
        // world in a paused single-player game.
        if self.world_open && !menus_stop {
            let adt = if self.args.screenshot.is_some() { social::film_dt() } else { dt };
            self.mobs.animate(self.body.pos, adt);
        }
        self.prof.mark("mobs: animação");

        let sit = self.situation();
        if let Some(a) = self.audio.as_mut() {
            let prev = a.now_playing.clone();
            a.update(dt, sit, &self.settings);
            if a.now_playing.is_some() && a.now_playing != prev {
                let t = format!("Tocando: {}", a.now_playing.clone().unwrap());
                self.toast(t, 5.0);
            }
        }

        self.prof.mark("música");
        self.update_controller(dt);
        self.update_presets(dt);

        self.autosave += dt;
        if self.autosave > 60.0 && self.screen != Screen::Loading {
            self.autosave = 0.0;
            self.save_all();
        }
        self.prof.mark("controle e salvar");
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
        let wish = self.rpg_wish(wish);
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
            if !self.sleep_step(input.sneak, dt)
                && !self.seat_step(input.sneak)
                && !self.ride_step(wish, input.sneak)
                && !self.pilot_step(wish, input.sneak, dt)
                && !self.boat_step(wish, input.sneak, dt)
                && !self.companion_mount_step(wish, input.jump, input.sneak, dt)
                && !self.mount_step(wish, input.jump, input.sneak, dt)
                && !self.strider_step(wish, input.sneak, dt)
                && !self.elytra_step(input.jump, dt)
            {
                self.pre_step(input.sneak);
                let was_mantling = self.body.mantle.is_some();
                if self.body.try_mantle(&self.world, &input, fwd) {
                    self.mantle_started();
                }
                self.body.step(&self.world, &input, dt);
                if was_mantling && self.body.mantle.is_none() {
                    // Up on the ledge: the feet come down on it.
                    let below = self.world.block_or_air((self.body.pos - Vec3::Y * 0.2).floor().as_ivec3());
                    if below != AIR {
                        self.sfx(Sfx::Step(Material::of(&self.reg.get(below).name)));
                    }
                }
                self.post_step(dt);
            }
        } else {
            // Held still, but still know about water for the picture.
            self.body.head_in_water = voxel::physics::Terrain::liquid(&self.world, self.body.eye().floor().as_ivec3());
            self.body.in_water = self.body.head_in_water || voxel::physics::Terrain::liquid(&self.world, self.body.pos.floor().as_ivec3());
        }
        let moved = (self.body.pos - before) * Vec3::new(1.0, 0.0, 1.0);
        // Feet sliding on ice (stopping, turning, setting off): a soft scrape and ice shavings
        // instead of footsteps.
        let sliding = self.body.on_ground && was_ground && !self.body.flying && self.body.ground.slippery() && self.body.slide > 1.2;
        if sliding {
            self.slide_dist += moved.length();
            if self.slide_dist > 1.3 {
                self.slide_dist = 0.0;
                let k = (self.body.slide / 6.0).min(1.0);
                self.sfx_here(Sfx::Fx(sfx_extra::Fx::Slide(k)), 0.3 + 0.25 * k);
                self.slide_shavings();
            }
        } else {
            self.slide_dist = 1.0;
        }
        let sink = if self.body.on_ground && !self.body.flying { self.body.ground.sink } else { 0.0 };
        self.sink += (sink - self.sink) * (dt * 8.0).min(1.0);
        if self.body.on_ground && was_ground && !self.body.flying && !sliding {
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
        // Pushing through tall grass, flowers and bushes rustles them.
        {
            let feet = self.body.pos.floor().as_ivec3();
            let in_plants = [feet, feet + IVec3::Y].iter().any(|&p| self.reg.get(self.world.block_or_air(p)).shape == Shape::Cross);
            if in_plants && !self.body.flying {
                self.rustle_dist += moved.length();
                if self.rustle_dist > 1.1 {
                    self.rustle_dist = 0.0;
                    self.sfx_here(Sfx::Fx(sfx_extra::Fx::Brush), 0.55);
                }
            } else {
                self.rustle_dist = 0.9;
            }
        }

        let feet = self.body.pos.floor().as_ivec3();
        let pose = self.pose(self.extras.burning > 0.0 || self.world.fluid(self.world.block_or_air(feet)) == 2 || self.world.fluid(self.world.block_or_air(feet + IVec3::Y)) == 2);
        self.anim.update(&pose, dt);
        let ready = if self.survival { self.attack_charge() } else { 1.0 };
        self.anim.follow_hand(self.held_stack().map(|s| s.item), ready, dt);

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
        let reach = self.reach();
        self.target = raycast(eye, fwd, reach, |p| match world.block(p) {
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
        if attack && self.attack_cd <= 0.0 && self.break_cd <= 0.0 && (self.aircraft_attack() || self.cart_attack() || self.boat_attack()) {
            self.attack_cd = 0.3;
            self.break_cd = 0.3;
        }
        if attack && self.attack_cd <= 0.0 && self.break_cd <= 0.0 {
            let block_dist = self.target.map(|t| t.distance).unwrap_or(f32::MAX);
            if let Some((i, t)) = paintings::pick(&self.paintings, &self.painting_defs, self.body.eye(), self.forward(), self.reach()) {
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
        if attack && self.attack_cd <= 0.0 && ready && self.deflect_shot() {
            self.attack_cd = 0.3;
        }
        // A fresh click always swings the weapon, at a mob or at the air.
        let mut swung = false;
        if attack && self.attack_cd <= 0.0 && ready {
            // Plants don't block a swing: the hit goes through grass and flowers to the mob.
            let world = &self.world;
            let reg = &self.reg;
            let block_dist = raycast(self.body.eye(), self.forward(), self.reach(), |p| match world.block(p) {
                Some(id) => id != AIR && !world.is_liquid(id) && !matches!(reg.get(id).shape, Shape::Cross),
                None => false,
            })
            .map(|t| t.distance)
            .unwrap_or(f32::MAX);
            if self.pvp_swing(block_dist) || self.combat_swing(block_dist) {
                swung = true;
            } else if let Some((i, t)) = self.mobs.pick(self.body.eye(), self.forward(), self.weapon_reach()) {
                if t < block_dist {
                    self.attack_cd = if self.survival { 0.1 } else { 0.45 };
                    self.break_cd = 0.3;
                    let eye = self.body.eye();
                    let charge = if self.survival { self.attack_charge() } else { 1.0 };
                    let base = if self.survival { self.surv_attack_damage() } else { 4.0 };
                    // Falling onto a mob with a ready weapon is a critical hit.
                    let crit = (self.body.vel.y < -0.5 && !self.body.on_ground && !self.body.in_water && charge > 0.9) || (charge > 0.9 && self.surv.rng.f32() < self.held_crit() + self.t5.rpg.stats.crit[rpg::MELEE]);
                    let dmg = if crit { base * 1.5 } else { base };
                    self.prog.was_full = charge > 0.9;
                    let dmg = if self.survival { self.weapon_melee_bonus(i, dmg) } else { dmg };
                    self.vulcao_melee_hit(i);
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
                    swung = true;
                    self.pads.rumble(0.3, 0.6, 0.1);
                    if self.survival {
                        self.after_melee(i, dmg);
                    }
                    if sprint_hit {
                        self.sprinting = false;
                    }
                }
            }
        }
        if attack && self.attack_pressed && !swung && self.target.is_none() {
            // A swing at the air, in any mode, still uses up the weapon's readiness (and
            // moves a combo weapon on to its next swing).
            if self.survival {
                self.surv.attack_t = 0.0;
            }
            self.combat_air_swing();
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
                if id != AIR && (self.reg.get(id).hardness >= 0.0 || !self.survival) && self.world.break_block(hit.block) {
                    self.mining = None;
                    self.world.mesh_now(hit.block);
                    self.after_break(hit.block, id);
                    if self.survival {
                        self.surv_block_broken(hit.block, id);
                    }
                    let axe = self.held_tool().is_some_and(|t| t.kind == voxel::block::ToolKind::Axe);
                    if axe && self.crouch <= 0.5 && self.reg.get(id).name.ends_with("_log") {
                        self.fell_tree(hit.block, id);
                    }
                    self.area_break(hit.block, hit.normal, id);
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
            if self.atlas_use() || self.vulcao_use() || self.mods_use() {
                self.place_cd = 0.3;
            } else if self.cart_use() || self.boat_use() || self.sit_use() {
                self.place_cd = 0.3;
            } else if self.crouch <= 0.5 && self.toggle_shaped() {
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

    /// How far the player reaches blocks (and boats, carts, paintings), as in Java Edition:
    /// 4.5 blocks in survival, 5 in creative.
    pub(crate) fn reach(&self) -> f32 {
        if self.survival {
            4.5 + self.t5.rpg.stats.reach
        } else {
            5.0
        }
    }

    /// Seconds to dig a block by hand in survival (no tools yet).
    fn dig_time(&self, id: BlockId) -> f32 {
        if self.survival {
            return self.items.dig_time(&self.reg, id, self.held_tool_for(id).as_ref()) / self.dig_speed_bonus(id);
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
        let onoff = |b: bool| if b { "Sim" } else { "Não" };
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
            (format!("Detalhe LOD: {}", lod::DETAILS[(self.settings.lod_detail as usize).min(lod::DETAILS.len() - 1)].0), ShaderRow::LodDetail, false),
            (format!("Sombras: {}", settings::SHADOW_LEVELS[self.settings.shadows.min(2) as usize]), ShaderRow::Shadows, false),
            (format!("Reflexos na água: {}", onoff(self.settings.water_reflections)), ShaderRow::Reflections, false),
            (format!("Suavização TAA: {}{}", onoff(self.settings.taa), if cur.is_none() { " (com shader)" } else { "" }), ShaderRow::Taa, false),
            (format!("Nuvens: {}", if !self.settings.show_clouds { "Não" } else if self.settings.layered_clouds { "Em camadas" } else { "Simples" }), ShaderRow::Clouds, false),
            (format!("Minérios brilhantes: {}", onoff(self.settings.emissive_textures)), ShaderRow::Emissive, false),
            (
                format!(
                    "Profundidade: {}{}",
                    settings::PARALLAX_LEVELS[self.settings.parallax.min(3) as usize].0,
                    if self.settings.parallax > 0 && self.settings.shader_quality == ShaderQuality::Fast { " (só Bonito)" } else { "" }
                ),
                ShaderRow::Parallax,
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
        // The first rows (settings) always show; the pack list scrolls with the wheel.
        let fixed = 11.min(left.len());
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
            ShaderRow::Back => self.set_screen(self.options_back),
            ShaderRow::Quality => {
                self.settings.shader_quality = match self.settings.shader_quality {
                    ShaderQuality::Fast => ShaderQuality::Fancy,
                    ShaderQuality::Fancy => ShaderQuality::Fast,
                };
                self.apply_shader();
            }
            ShaderRow::Reload => self.reload_shader_packs(),
            ShaderRow::Shadows => self.settings.shadows = (self.settings.shadows + 1) % 3,
            ShaderRow::Reflections => self.settings.water_reflections = !self.settings.water_reflections,
            ShaderRow::Taa => {
                self.settings.taa = !self.settings.taa;
                self.apply_shader();
            }
            ShaderRow::Clouds => {
                // Off → simple → layered → off.
                let st = &mut self.settings;
                (st.show_clouds, st.layered_clouds) = match (st.show_clouds, st.layered_clouds) {
                    (false, _) => (true, false),
                    (true, false) => (true, true),
                    (true, true) => (false, true),
                };
            }
            ShaderRow::Emissive => self.settings.emissive_textures = !self.settings.emissive_textures,
            ShaderRow::Parallax => self.settings.parallax = (self.settings.parallax.min(3) + 1) % 4,
            ShaderRow::LodDetail => self.settings.lod_detail = (self.settings.lod_detail + 1) % lod::DETAILS.len() as u8,
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
            // Held still in the air (the world around is still loading) counts as standing,
            // not as a long fall.
            on_ground: self.body.on_ground || self.body.vel == Vec3::ZERO,
            in_water: self.body.in_water,
            head_in_water: self.body.head_in_water,
            flying: self.body.flying,
            sneaking: self.crouch,
            crawling: self.prone_t,
            sprinting: self.sprinting,
            burning,
            hurt: self.survival && self.hurt_age < 0.3,
            sitting: self.riding.is_some() || self.seat.is_some() || self.mods.aircraft.pilot.is_some() || self.t5.comp.mount.is_some(),
            holding: self.held_stack().is_some(),
            torch: self.held_torch().is_some(),
            shield_up: if self.shields_worn().iter().any(|(l, m)| l.kind == 0 && matches!(m, shield::Mount::Hand { .. })) { self.extras.raise } else { 0.0 },
            shield_left: !self.held_stack().is_some_and(|s| self.items.get(s.item).name == "shield"),
            mantle: self.body.mantle.map_or(0.0, |m| m.progress().max(1e-3)),
            ledge: self.body.mantle.map_or(0.0, |m| m.ledge - self.body.pos.y),
            sleeping: self.sleep.map(|s| beds::body_yaw(s.facing)),
        }
    }

    /// The hands caught a ledge: a scrape on the edge's block, and a little hunger.
    fn mantle_started(&mut self) {
        let Some(m) = self.body.mantle else { return };
        let dir = (m.to - m.from) * Vec3::new(1.0, 0.0, 1.0);
        let edge = Vec3::new(m.from.x, m.ledge - 0.05, m.from.z) + dir.normalize_or_zero() * 0.5;
        let id = self.world.block_or_air(edge.floor().as_ivec3());
        if id != AIR {
            let mat = Material::of(&self.reg.get(id).name);
            self.sfx(Sfx::Step(mat));
        }
        if self.survival {
            self.surv.exhaustion += 0.2;
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

    /// Monsters go after the player: survival, alive, visible, and not a dedicated server's
    /// own nobody. In creative they leave the player alone, like vanilla.
    pub(crate) fn hunted(&self) -> bool {
        self.survival && !self.net.server && self.screen != Screen::Dead && !self.has_effect(effects::INVISIBILITY)
    }

    /// Hurt the player (survival only). `from` pushes the player away.
    fn damage(&mut self, amount: f32, from: Option<Vec3>) {
        let pvp = std::mem::take(&mut self.net.pvp_incoming);
        if !self.survival || self.net.server || self.screen == Screen::Dead || self.hurt_age < 0.5 + self.t5.rpg.stats.iframes || amount <= 0.0 {
            self.prog.next_kind = Default::default();
            return;
        }
        // Monsters and blasts hit harder on harder difficulties.
        let amount = if from.is_some() && !pvp { amount * [0.0, 0.5, 1.0, 1.5][self.settings.difficulty.min(3) as usize] } else { amount };
        let amount = if from.is_some() && !pvp { amount * self.mode_damage() } else { amount };
        // Dodged (mirror charm, the radiant set): nothing at all.
        if self.rpg_dodge(from) || self.tr_ignores_hit(from) {
            self.prog.next_kind = Default::default();
            return;
        }
        // A shield up and facing the hit takes it all.
        let amount = self.shield_block(amount, from);
        if amount <= 0.0 {
            self.prog.next_kind = Default::default();
            return;
        }
        // Armor softens hits from something (mobs, blasts), not falls, drowning or hunger.
        let amount = if from.is_some() { self.armor_absorb(amount) } else { amount };
        let amount = self.t5_incoming(amount, from);
        let amount = self.protect(amount, from);
        if amount <= 0.0 {
            return;
        }
        self.health -= amount;
        self.prog.stats.add("damage_taken", (amount * 10.0) as u64);
        self.hurt_age = 0.0;
        self.regen = 0.0;
        if let Some(f) = from.filter(|_| !self.no_knockback()) {
            self.body.knockback(self.body.pos - f, 5.0);
        }
        self.rpg_after_hurt(amount, from);
        self.sfx(Sfx::Hurt);
        self.pads.rumble(0.9, 0.6, 0.25);
        if self.health <= 0.0 && self.use_totem() {
            return;
        }
        if self.health <= 0.0 {
            self.health = 0.0;
            self.prone = false;
            self.sleep = None;
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
            if fall > 3.5 && !self.body.in_water && !self.no_fall_damage() {
                self.prog.next_kind = progression::DamageKind::Fall;
                self.damage(((fall - 3.0) * self.snow_fall_factor()).floor(), None);
            }
            self.fall_peak = self.body.pos.y;
        } else {
            self.fall_peak = self.fall_peak.max(self.body.pos.y);
        }
        // Breath under water.
        if self.body.head_in_water && !self.has_effect(effects::WATER_BREATHING) && !self.has_effect(effects::CONDUIT_POWER) {
            // Respiration: each level makes a breath last that much longer.
            self.air -= dt / (1.0 + self.armor_ench(enchanting::RESPIRATION) as f32) / self.t5.rpg.stats.breath;
            if self.air < -1.0 {
                self.air = 0.0;
                self.prog.next_kind = progression::DamageKind::Drown;
                self.damage(2.0, None);
            }
        } else {
            self.air = (self.air + dt * 5.0).min(10.0);
        }
        // Lava burns.
        let feet = self.body.pos.floor().as_ivec3();
        if (self.world.fluid(self.world.block_or_air(feet)) == 2 || self.world.fluid(self.world.block_or_air(feet + IVec3::Y)) == 2) && !self.lava_safe(dt) {
            self.prog.next_kind = progression::DamageKind::Fire;
            self.damage(4.0, None);
        }
        // Healing comes from a full stomach (survival.rs).
    }

    /// Whether any of `points` can be seen from the eye, no solid block in between. Names and
    /// bars over mobs and items use it so they never show through walls or the ground.
    pub(crate) fn in_sight(&self, points: &[Vec3]) -> bool {
        let eye = self.body.eye();
        let world = &self.world;
        points.iter().any(|&p| {
            let to = p - eye;
            let dist = to.length();
            dist < 0.5 || raycast(eye, to / dist, dist - 0.3, |b| voxel::physics::Terrain::solid(world, b)).is_none()
        })
    }

    /// A health bar floating over each mob that was hurt lately, with its name.
    fn draw_mob_health_bars(&mut self, w: f32, h: f32) {
        let Some((vp, cam_block)) = self.ui_view_proj.filter(|_| self.screen == Screen::Playing) else { return };
        let s = self.ui.scale;
        let eye = self.body.eye();
        for m in &self.mobs.list {
            if !m.alive() || m.hurt_age > 5.0 {
                continue;
            }
            let def = &self.mobs.reg.defs[m.def];
            let top = m.body.pos + Vec3::Y * (def.height + 0.45);
            let dist = (top - eye).length();
            if dist > 24.0 || !self.in_sight(&[m.body.pos + Vec3::Y * def.height * 0.9, m.body.center()]) {
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
        // The classic explosion: rays from the centre out through a 16×16×16 grid of
        // directions; each loses strength as it goes and more through tougher blocks, so soft
        // ground opens a wide crater while stone holds better.
        let mut hit: std::collections::HashSet<IVec3> = std::collections::HashSet::new();
        let mut seed = (pos.x * 91.7 + pos.y * 13.3 + pos.z * 47.1).to_bits();
        let mut rand = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed % 10000) as f32 / 10000.0
        };
        for i in 0..16 {
            for j in 0..16 {
                for k in 0..16 {
                    if !(i == 0 || i == 15 || j == 0 || j == 15 || k == 0 || k == 15) {
                        continue;
                    }
                    let dir = Vec3::new(i as f32 / 15.0 * 2.0 - 1.0, j as f32 / 15.0 * 2.0 - 1.0, k as f32 / 15.0 * 2.0 - 1.0).normalize();
                    let mut strength = power * (0.7 + rand() * 0.6);
                    let mut p = pos;
                    while strength > 0.0 {
                        let b = p.floor().as_ivec3();
                        let id = self.world.block_or_air(b);
                        if id != AIR {
                            let blk = self.reg.get(id);
                            let res = if self.world.is_liquid(id) {
                                100.0
                            } else if blk.hardness < 0.0 || blk.hardness >= 50.0 {
                                1200.0
                            } else if blk.hardness < 1.0 {
                                blk.hardness
                            } else {
                                blk.hardness * 4.0
                            };
                            strength -= (res + 0.3) * 0.3;
                            if strength > 0.0 {
                                hit.insert(b);
                            }
                        }
                        p += dir * 0.3;
                        strength -= 0.225;
                    }
                }
            }
        }
        let keep = 1.0 / power.max(1.0);
        let mut n = 0;
        for b in hit {
            let id = self.world.block_or_air(b);
            if self.redstone.kind(id) == redstone::Kind::Tnt {
                // TNT caught in a blast lights with a short, uneven fuse.
                let fuse = 0.5 + rand();
                self.prime_tnt(b, fuse);
                continue;
            }
            if id == AIR || !self.world.break_block(b) {
                continue;
            }
            if self.survival {
                self.blast_drops(b, id, keep);
            }
            n += 1;
            if n % 3 == 0 {
                let (layer, tint) = self.block_look(id);
                self.particles.block_bits(b, layer, tint, 3, 2.0);
            }
        }
        let reach = power * 2.0;
        let d = (self.body.center() - pos).length();
        if d < reach {
            let k = 1.0 - d / reach;
            self.prog.next_kind = progression::DamageKind::Explosion;
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
        self.explosion_fx(pos, power);
        self.pads.rumble(1.0, 1.0, 0.6);
        self.net_explosion(pos, power);
    }

    fn respawn(&mut self) {
        if self.respawn_at_anchor() {
            return;
        }
        if self.dim != voxel::worldgen::Dim::Overworld {
            // Death in the Nether sends you home: load the Overworld first.
            self.travel_home_after_death();
            return;
        }
        let (sx, sz) = self.spawn_xz;
        let y = self.world.surface_y(sx, sz).unwrap_or(100) + 1;
        self.body.pos = Vec3::new(sx as f32 + 0.5, y as f32, sz as f32 + 0.5);
        self.body.vel = Vec3::ZERO;
        self.health = self.max_health;
        self.air = 10.0;
        self.hurt_age = 100.0;
        self.surv_respawned();
        self.fall_peak = self.body.pos.y;
        self.sfx(Sfx::Respawn);
        self.set_screen(Screen::Playing);
    }

    /// Test scene: one of every mob in a row in front of the player.
    fn spawn_demo_mobs(&mut self) {
        if let Ok(floor) = std::env::var("MC_ARENA") {
            // Test hook: a flat floor for the line-up (the Nether's slopes hide the mobs).
            self.test_arena(&floor);
        }
        let f = self.forward();
        let fwd = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let side = Vec3::new(-fwd.z, 0.0, fwd.x);
        // MC_MOBS=a,b picks which ones (any mob, even those that never spawn on their own).
        let names: Vec<String> = match std::env::var("MC_MOBS") {
            Ok(list) => list.split(',').map(|s| s.trim().to_string()).collect(),
            Err(_) => self.mobs.reg.defs.iter().filter(|d| d.spawn != voxel::mob::SpawnRule::Never).map(|d| d.name.clone()).collect(),
        };
        // Rows of up to seven, the first ones (the tall monsters) further back.
        let rows = names.len().div_ceil(7);
        for (i, name) in names.iter().enumerate() {
            let (row, col) = (i / 7, i % 7);
            let n = (names.len() - row * 7).min(7) as f32;
            let p = self.body.pos + fwd * (6.0 + (rows - 1 - row) as f32 * 4.5) + side * ((col as f32 - (n - 1.0) / 2.0) * 1.8);
            let x = p.x.floor() as i32;
            let z = p.z.floor() as i32;
            let y = self.ground_y(x, z, self.body.pos.y as i32 + 4).unwrap_or(p.y as i32) + 1;
            let at = Vec3::new(p.x, y as f32, p.z);
            if self.mobs.spawn(name, at) {
                // Face the camera.
                let m = self.mobs.list.last_mut().unwrap();
                let to = self.body.pos - at;
                // MC_FACE=degrees turns them (90 = side-on, to see the legs move).
                let turn = std::env::var("MC_FACE").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0).to_radians();
                m.yaw = to.x.atan2(to.z) + turn;
            }
        }
        // An arrow on its way, frozen in the air, for the test picture.
        let from = self.body.pos + fwd * 4.0 + side * 1.5 + Vec3::Y * 1.6;
        self.mobs.arrows.push(mobs::Arrow { pos: from, vel: (-fwd * 0.3 + side * 1.0 + Vec3::Y * 0.1) * 0.001, age: 0.0, stuck: true, damage: 0.0, shooter: 0 });
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
        // Another layer of snow on a layer: it grows.
        if shape == Shape::Layer && cb.base == id && cb.variant < 7 {
            let more = self.variant_id(id, cb.variant + 1);
            if !self.world.set_block(hit.block, more) {
                return None;
            }
            return self.placed(hit.block, id);
        }
        // Plants are replaced instead of placed against.
        let pos = if (cb.shape == Shape::Cross || self.snow_replaceable(clicked)) && clicked != id { hit.block } else { hit.block + hit.normal };
        let cur = self.world.block_or_air(pos);
        if cur != AIR && !self.world.is_liquid(cur) && self.reg.get(cur).shape != Shape::Cross && !self.snow_replaceable(cur) {
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
            Shape::Stairs | Shape::Roof => facing(f.x, f.z) | if upper { 4 } else { 0 },
            Shape::Door => facing(f.x, f.z),
            // The head lies away from you, on the far side of the block clicked.
            Shape::Bed => facing(f.x, f.z),
            Shape::FenceGate => facing(f.x, f.z),
            // A chair turns to face the player who puts it down.
            Shape::Chair => facing(-f.x, -f.z),
            // Statues, trophies and other models turn their front to you.
            Shape::Model => facing(-f.x, -f.z),
            // A bench runs across your view.
            Shape::Bench => facing(f.x, f.z) ^ 1,
            // An anvil lies across your view, its long side toward you.
            Shape::Anvil => facing(f.x, f.z),
            // A sign hangs on the wall clicked, or stands facing you.
            Shape::Sign | Shape::Banner | Shape::TallBanner => if hit.normal.y == 0 { facing(-hit.normal.x as f32, -hit.normal.z as f32) | 4 } else if hit.normal.y > 0 { facing(f.x, f.z) } else { return None },
            // A shelf goes on the wall you clicked.
            Shape::Shelf => if hit.normal.y == 0 { facing(-hit.normal.x as f32, -hit.normal.z as f32) } else { facing(f.x, f.z) },
            // A ladder goes flat on the wall clicked.
            Shape::Ladder => if hit.normal.y == 0 { facing(-hit.normal.x as f32, -hit.normal.z as f32) } else { return None },
            // Under a block it hangs on a chain.
            Shape::Lantern => (hit.normal.y < 0) as u8,
            // Runs the way you look.
            Shape::Bridge => facing(f.x, f.z) & 1,
            // Along the axis of the face clicked, like a log.
            Shape::Hollow => if hit.normal.y != 0 { 0 } else if hit.normal.x != 0 { 1 } else { 2 },
            Shape::Trapdoor => {
                let side = if hit.normal.y == 0 { facing(-hit.normal.x as f32, -hit.normal.z as f32) ^ 2 } else { facing(-f.x, -f.z) };
                side | if upper { 8 } else { 0 }
            }
            // On a wall: facing the wall (the block it hangs on); on the floor: the way you look.
            Shape::Lever | Shape::Button => {
                if hit.normal.y < 0 {
                    return None;
                }
                if hit.normal.y == 0 {
                    facing(-hit.normal.x as f32, -hit.normal.z as f32) | 8
                } else {
                    facing(f.x, f.z)
                }
            }
            // Signal goes out the way you look.
            Shape::Repeater | Shape::Comparator => facing(f.x, f.z),
            // Rails join the rails around them.
            Shape::Rail => self.rail_variant(pos, id, f),
            // On a wall when placed against one (1 + the wall's side), else standing.
            Shape::Torch => {
                if hit.normal.y < 0 {
                    return None;
                }
                if hit.normal.y == 0 {
                    1 + facing(-hit.normal.x as f32, -hit.normal.z as f32)
                } else {
                    0
                }
            }
            // The spout points into the block clicked from the side, else down.
            Shape::Hopper => {
                if hit.normal.y == 0 {
                    1 + facing(-hit.normal.x as f32, -hit.normal.z as f32)
                } else {
                    0
                }
            }
            // Observers watch the way you look; droppers and dispensers face you.
            Shape::Oriented => {
                let toward = self.reg.get(id).name == "observer";
                let k = if toward { 1.0 } else { -1.0 };
                if self.reg.get(id).name.ends_with("_drawer") || self.reg.get(id).name == "windmill_sails" {
                    facing(-f.x, -f.z)
                } else if f.y * k > 0.75 {
                    4
                } else if f.y * k < -0.75 {
                    5
                } else {
                    facing(f.x * k, f.z * k)
                }
            }
            // The face points back at you (up or down when looking steeply).
            Shape::Piston => {
                if f.y < -0.75 {
                    4
                } else if f.y > 0.75 {
                    5
                } else {
                    facing(-f.x, -f.z)
                }
            }
            _ => 0,
        };
        // Full blocks with a front face the player; logs lie along the face they go on.
        let v = match b.turn {
            Turn::Facing | Turn::Chest => facing(f.x, f.z),
            Turn::Axis => if hit.normal.y != 0 { 0 } else if hit.normal.x != 0 { 1 } else { 2 },
            Turn::None => v,
        };
        if matches!(shape, Shape::Lever | Shape::Button) && v & 8 != 0 {
            if !self.reg.get(clicked).solid || !self.reg.is_opaque(clicked) && self.reg.get(clicked).shape != Shape::Cube {
                return None;
            }
        }
        if shape == Shape::Door {
            let above = pos + IVec3::Y;
            if self.world.block_or_air(above) != AIR || self.body.intersects_block(above) {
                return None;
            }
            let v = v | self.door_hinge(pos, v, at);
            let (lower, top) = (self.variant_id(id, v), self.variant_id(id, v | 8));
            if self.world.set_block(pos, lower) {
                self.world.set_block(above, top);
                self.pair_doors(pos);
                return self.placed(pos, id);
            }
            return None;
        }
        if shape == Shape::Bed {
            return self.place_bed(pos, id, v);
        }
        let id_v = self.variant_id(id, v);
        // Plants, torches and redstone parts need something under them.
        let on_floor = matches!(b.shape, Shape::Wire | Shape::Plate | Shape::Repeater | Shape::Comparator | Shape::Rail) || matches!(b.shape, Shape::Lever | Shape::Button) && v & 8 == 0;
        if on_floor {
            let under = self.world.block_or_air(pos - IVec3::Y);
            if !self.reg.is_opaque(under) {
                return None;
            }
        }
        if b.shape == Shape::Torch && v > 0 {
            if !self.reg.get(clicked).solid {
                return None;
            }
        } else if matches!(b.shape, Shape::Cross | Shape::Torch) {
            let under = self.world.block_or_air(pos - IVec3::Y);
            if under == AIR || !self.reg.get(under).solid {
                return None;
            }
        }
        if self.world.place_block(pos, id_v) {
            if shape == Shape::Rail {
                self.connect_rail(pos);
            }
            if self.reg.get(id).turn == Turn::Chest && self.crouch <= 0.5 {
                self.pair_chest(pos, hit.block);
            }
            return self.placed(pos, id);
        }
        None
    }

    /// The id of an orientation of a shaped block (see `voxel::shapes`).
    fn variant_id(&self, id: BlockId, v: u8) -> BlockId {
        self.reg.variant(id, v)
    }

    /// Right click on a door or trapdoor opens or closes it. Returns true when it did.
    fn toggle_shaped(&mut self) -> bool {
        let Some(hit) = self.target else { return false };
        if self.redstone.interact(&mut self.world, hit.block) {
            self.anim.start_swing();
            // Repeaters and comparators say what they are set to now.
            let b = self.reg.get(self.world.block_or_air(hit.block));
            if let Some(text) = redstone_fx::setting(b.shape, b.variant) {
                self.toast(text, 1.8);
            }
            self.world.mesh_now(hit.block);
            self.redstone_fx.scan_t = 0.0;
            return true;
        }
        let id = self.world.block_or_air(hit.block);
        let b = self.reg.get(id);
        // Iron doors only open with redstone.
        if !matches!(b.shape, Shape::Door | Shape::Trapdoor | Shape::FenceGate) || self.reg.get(b.base).name.starts_with("iron_") {
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
            self.door_follow(hit.block);
        }
        self.anim.start_swing();
        self.sfx(Sfx::Place(Material::Wood));
        true
    }

    /// Effects after a block went down.
    fn placed(&mut self, pos: IVec3, id: BlockId) -> Option<IVec3> {
        self.place_cd = 0.22;
        self.plugins_block(pos, id, true);
        let name = self.reg.get(id).name.clone();
        self.comfort_placed(pos, &name);
        self.tr3_placed(pos, &name);
        self.anim.start_swing();
        self.pads.rumble(0.0, 0.35, 0.05);
        let m = Material::of(&self.reg.get(id).name);
        let vol = self.settings.sound_volume;
        if let Some(a) = self.audio.as_mut() {
            a.play(Sfx::Place(m), vol);
        }
        if self.reg.get(id).shape == Shape::Sign && self.args.screenshot.is_none() {
            self.edit_sign(pos);
        }
        self.world.mesh_now(pos);
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
        put(w, 6, 0, 7, "glass_pane");
        put(w, 6, 1, 7, "glass_pane");
        // A row of wooden windows: one unbroken window, no gaps between or inside them.
        for x in 1..=6 {
            put(w, x, 0, 10, "cherry_window");
            put(w, x, 1, 10, "cherry_window");
        }
        // Doors: closed and open, and trapdoors.
        put(w, -4, 0, 9, "oak_door:2");
        put(w, -4, 1, 9, "oak_door:10");
        put(w, -2, 0, 9, "oak_door:6");
        put(w, -2, 1, 9, "oak_door:14");
        // A double door, open: hinged on the outside, swinging apart.
        put(w, -6, 0, 9, "oak_door:22");
        put(w, -6, 1, 9, "oak_door:30");
        put(w, -5, 0, 9, "oak_door:6");
        put(w, -5, 1, 9, "oak_door:14");
        put(w, 0, 0, 9, "oak_trapdoor");
        put(w, 1, 0, 9, "oak_trapdoor:8");
        put(w, 2, 0, 9, "oak_trapdoor:6");
        put(w, 4, 0, 9, "oak_planks");
        put(w, 4, 1, 9, "oak_planks");
        // Roofs: two little gables and one upside down, then a glass wall that joins up.
        for z in 11..=12 {
            put(w, -6, 0, z, "oak_roof:1");
            put(w, -5, 0, z, "oak_roof:3");
            put(w, -3, 0, z, "brick_roof:1");
            put(w, -2, 0, z, "brick_roof:3");
        }
        put(w, 0, 0, 11, "spruce_roof:6");
        // Stained glass: tints what is behind it.
        for (i, c) in ["purple", "red", "light_blue"].iter().enumerate() {
            put(w, -6 + i as i32, 0, 1, &format!("{c}_stained_glass"));
            put(w, -6 + i as i32, 1, 1, &format!("{c}_stained_glass"));
        }
        // A spring on a little ledge: it pours off and spreads over the floor.
        for x in 4..=6 {
            put(w, x, 0, 1, "cobblestone");
            put(w, x, 0, 0, "cobblestone");
        }
        put(w, 6, 1, 0, "water");
        for x in 2..=5 {
            for y in 0..3 {
                put(w, x, y, 11, "glass");
            }
        }
        // Look from above.
        self.body.pos.y += 3.5;
        self.body.flying = true;
    }

    /// Test hook MC_FLORA=1 (or a list of names): a grass bed in front of the player with every
    /// plant that has a body, in rows two blocks apart, for screenshots.
    fn build_flora_test(&mut self) {
        let p = self.body.pos.floor().as_ivec3();
        let f = self.forward();
        let dir = if f.x.abs() > f.z.abs() { IVec3::new(f.x.signum() as i32, 0, 0) } else { IVec3::new(0, 0, f.z.signum() as i32) };
        let side = IVec3::new(-dir.z, 0, dir.x);
        let only = std::env::var("MC_FLORA").unwrap_or_default();
        let wanted: Vec<&str> = only.split(',').filter(|n| !n.is_empty() && *n != "1").collect();
        let plants: Vec<BlockId> = self
            .reg
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.shape == voxel::block::Shape::Cross && voxel::flora::kind_of(&b.name).is_some())
            .filter(|(_, b)| wanted.is_empty() || wanted.contains(&b.name.as_str()))
            .map(|(i, _)| i as BlockId)
            .collect();
        let (grass, dirt) = (self.reg.id_or_air("grass_block"), self.reg.id_or_air("dirt"));
        let per_row: usize = std::env::var("MC_FLORA_ROW").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
        let ground = p.y - 1;
        for a in -10..=10 {
            for b in -2..=40 {
                let col = p + side * a + dir * b;
                for h in -3..6 {
                    let q = IVec3::new(col.x, ground + h, col.z);
                    let id = if h < 0 { dirt } else if h == 0 { grass } else { AIR };
                    self.world.set_block(q, id);
                }
            }
        }
        for (k, id) in plants.iter().enumerate() {
            let (row, colm) = ((k / per_row) as i32, (k % per_row) as i32);
            let start: i32 = std::env::var("MC_FLORA_START").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
            let q = p + dir * (start + row * 2) + side * (colm * 2 - per_row as i32 + 1);
            // Stacked plants (cane, kelp, vines) three high, so they show how they join.
            let name = &self.reg.blocks[*id as usize].name;
            let high = if voxel::flora::kind_of(name).is_some_and(|k| k.stacked()) { 3 } else { 1 };
            for h in 1..=high {
                self.world.set_block(IVec3::new(q.x, ground + h, q.z), *id);
            }
        }
    }

    /// Test hook MC_SHADOWS=1: flat sand around the player, a pool of water in front and a stone
    /// wall to one side, for shadow and reflection screenshots.
    fn build_shadow_test(&mut self) {
        let p = self.body.pos.floor().as_ivec3();
        let f = self.forward();
        let dir = if f.x.abs() > f.z.abs() { IVec3::new(f.x.signum() as i32, 0, 0) } else { IVec3::new(0, 0, f.z.signum() as i32) };
        let side = IVec3::new(-dir.z, 0, dir.x);
        let g = |n: &str| self.reg.id_or_air(n);
        let (sand, water, stone, ice, thin) = (g("sand"), g("water"), g("stone_bricks"), g("ice"), g("thin_ice"));
        for a in -14..=14 {
            for b in -8..=24 {
                let col = p + side * a + dir * b;
                for h in -3..12 {
                    let q = col + IVec3::Y * h;
                    let pool = (4..=20).contains(&b) && (-7..=9).contains(&a);
                    let id = if h < -1 || (h == -1 && !pool) {
                        sand
                    } else if h == -1 && b >= 15 && a >= 2 {
                        // Part of the pool frozen over, thick and thin ice.
                        if a >= 6 { ice } else { thin }
                    } else if h == -1 || (h == -2 && pool) {
                        water
                    } else if a == -4 && (0..=3).contains(&b) && h < 3 {
                        stone
                    } else {
                        AIR
                    };
                    self.world.set_block(q, id);
                }
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
        // MC_CTM=1: a shelf wall and a row of ores in front, for connected and glowing textures.
        if std::env::var("MC_CTM").is_ok() {
            let ores = ["diamond_ore", "redstone_ore", "emerald_ore", "lapis_ore", "gold_ore", "iron_ore", "amethyst_block"];
            for i in -3..=3 {
                let q = base + side * i - dir * 2;
                for h in 0..2 {
                    self.world.set_block(q + IVec3::Y * h, g("bookshelf"));
                }
                self.world.set_block(q + IVec3::Y * 2, g(ores[(i + 3) as usize]));
            }
        }
        // MC_MAT=1: metal and gem blocks on the ground in front, ores in a row behind, for the
        // materials option (look a little down: --look 0,-25).
        if std::env::var("MC_MAT").is_ok() {
            let front = ["gold_block", "iron_block", "diamond_block", "silver_block", "copper_block"];
            let back = ["diamond_ore", "gold_ore", "copper_ore", "emerald_ore", "lapis_ore", "redstone_ore", "iron_ore"];
            for (k, (row, list)) in [(3, &front[..]), (6, &back[..])].into_iter().enumerate() {
                let half = list.len() as i32 / 2;
                for (i, n) in list.iter().enumerate() {
                    let q = p + dir * row + side * ((i as i32 - half) * (2 - k as i32));
                    let ground = self.world.surface_y(q.x, q.z).unwrap_or(p.y - 1);
                    self.world.set_block(IVec3::new(q.x, ground + 1, q.z), g(n));
                }
            }
        }
        // MC_POM=1: a wall of rough blocks right in front, for the block depth option.
        if std::env::var("MC_POM").is_ok() {
            let wall = ["oak_planks", "cobblestone", "stone_bricks", "bricks", "mossy_cobblestone", "deepslate_tiles", "oak_log", "andesite", "spruce_planks"];
            for i in -4..=4 {
                let q = p + dir * 2 + side * i;
                let ground = self.world.surface_y(q.x, q.z).unwrap_or(p.y - 1);
                for h in 0..3 {
                    self.world.set_block(IVec3::new(q.x, ground + 1 + h, q.z), g(wall[(i + 4) as usize]));
                }
            }
        }
    }

    /// Raindrops splashing on the ground and on water around the player.
    fn rain_splashes(&mut self, dt: f32) {
        let water = self.reg.get(self.reg.id_or_air("water")).faces[2];
        let n = (self.weather.rain * 90.0 * dt).ceil() as i32;
        let c = self.body.pos.floor().as_ivec3();
        for _ in 0..n {
            let x = c.x + self.surv.rng.range(-12, 13);
            let z = c.z + self.surv.rng.range(-12, 13);
            if matches!(self.world.gen.base_biome_at(x, z).name(), "Desert" | "Badlands") {
                continue;
            }
            let Some(y) = self.world.surface_y(x, z) else { continue };
            if (y - c.y).abs() > 20 {
                continue;
            }
            let at = Vec3::new(x as f32 + self.surv.rng.f32(), y as f32 + 1.0, z as f32 + self.surv.rng.f32());
            if self.world.fluid(self.world.block_or_air(IVec3::new(x, y, z))) == 1 {
                // Rain on water: a small ring spreading where each drop lands.
                if self.life_amount() > 0.0 && self.surv.rng.chance(0.5 * self.life_amount().min(1.0)) {
                    self.particles.ripple(at - Vec3::Y * 0.1, 0.25 + self.surv.rng.f32() * 0.15);
                }
                continue;
            }
            self.particles.rain_drop(at, water);
        }
    }

    /// Snow settles in thin layers on open ground around the player while it snows.
    fn settle_snow(&mut self, dt: f32) {
        let Some(layer) = self.reg.id("snow_layer") else { return };
        let tries = (dt * 6.0).ceil() as i32;
        let c = self.body.pos.floor().as_ivec3();
        for _ in 0..tries {
            let x = c.x + self.surv.rng.range(-24, 25);
            let z = c.z + self.surv.rng.range(-24, 25);
            let Some(y) = self.world.surface_y(x, z) else { continue };
            let p = IVec3::new(x, y, z);
            if self.snow_settle_extra(p) {
                continue;
            }
            let id = self.world.block_or_air(p);
            let b = self.reg.get(id);
            if b.base == layer {
                // Piles up slowly, never past half a block on its own.
                if b.variant < 3 && self.surv.rng.chance(0.2) {
                    self.world.set_block(p, self.variant_id(layer, b.variant + 1));
                }
            } else if (self.reg.is_opaque(id) || b.name.ends_with("_leaves")) && b.shape == Shape::Cube && self.world.block_or_air(p + IVec3::Y) == AIR {
                // Also on treetops (Snow! Real Magic! idea).
                self.world.set_block(p + IVec3::Y, layer);
            } else if self.reg.is_opaque(id) && self.reg.get(self.world.block_or_air(p + IVec3::Y)).name == "short_grass" {
                // Grass gets buried.
                self.world.set_block(p + IVec3::Y, layer);
            }
        }
    }

    fn sky(&self) -> ([f32; 3], [f32; 3], f32, Vec3) {
        let a = self.time_of_day / 24000.0 * std::f32::consts::TAU;
        let sun = Vec3::new(a.cos(), a.sin(), 0.0);
        let daylight = (a.sin() * 2.2 + 0.45).clamp(0.0, 1.0);
        let lerp = |n: [f32; 3], d: [f32; 3]| [n[0] + (d[0] - n[0]) * daylight, n[1] + (d[1] - n[1]) * daylight, n[2] + (d[2] - n[2]) * daylight];
        let mut top = lerp([0.008, 0.01, 0.035], [0.47, 0.65, 1.0]);
        let mut horizon = lerp([0.02, 0.025, 0.06], [0.74, 0.84, 1.0]);
        // Rain greys the sky and dims the day; lightning lights everything for a moment.
        let gloom = self.weather.gloom();
        let flash = self.weather.flash;
        for c in [&mut top, &mut horizon] {
            let grey = (c[0] + c[1] + c[2]) / 3.0;
            for v in c.iter_mut() {
                *v = (*v + (grey - *v) * (gloom * 1.8).min(1.0)) * (1.0 - gloom * 0.85) + flash * 0.5;
            }
        }
        let mut daylight = daylight * (1.0 - gloom) + flash * 0.6;
        self.tr_sky(&mut top, &mut horizon, &mut daylight);
        (top, horizon, daylight.clamp(0.16, 1.0), sun)
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
        // Where the frame goes (prof.rs): the costliest parts of the last two seconds.
        for (name, ms) in self.prof.recent.clone() {
            rows.push((format!("  {name}"), format!("{ms:.2} ms"), if ms < 4.0 { GOOD } else if ms < 10.0 { MID } else { BAD }));
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
        // Screenshots show things at rest.
        let anim = juice::enabled(self.settings.ui_animations && !self.settings.reduce_motion, self.args.screenshot.is_some());
        self.surv.juice.tick(self.selected, anim);

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
        let night = self.sleep_fade();
        if night > 0.0 {
            // Falling asleep: the view goes dark.
            self.ui.rect(0.0, 0.0, w, h, [4, 6, 14, (night * 235.0) as u8]);
        }
        self.draw_immersion_overlay(w, h);
        let haze = self.portal_haze();
        if haze > 0.0 {
            // Standing in a portal: the view fills with purple until the trip starts.
            let wave = 0.85 + 0.15 * (self.now as f32 * 4.0).sin();
            self.ui.rect(0.0, 0.0, w, h, [110, 30, 170, (haze * wave * 170.0) as u8]);
        }
        let menu_screen = self.is_title_screen() || matches!(self.screen, Screen::Options | Screen::Packs | Screen::Keys | Screen::Voice);
        if !self.hide_hud && !matches!(self.screen, Screen::Paused | Screen::Friends | Screen::Dead | Screen::Shaders) && !menu_screen {
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
            self.draw_effect_overlays(w, h);
            self.draw_mob_health_bars(w, h);
            self.draw_drop_labels(w, h);
            self.draw_hunter_marks(w, h);
            self.draw_mob_names(w, h);
            self.draw_tr3_hud(w, h);
            if self.screen == Screen::Playing {
                self.draw_explore_hud(w, h);
            }
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
            self.draw_boss_bar(w);
            self.draw_progress(w, h);
            self.draw_skills(w, hy);
            self.draw_t5_hud(w, h);
            self.draw_mods_hud(w, h);
            if self.survival {
                self.draw_survival_hud(hx, hy, cell);
                self.draw_money_flash(hx, hy, cell);
                self.draw_mana(w);
            }
            self.draw_fase6_hud(w, h, hx, hy, cell);
            // The selector slides from slot to slot (Immersive UI idea).
            let sx = (hx + self.surv.juice.selector() * cell).round();
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
                // Ten hearts a row above the left of the hotbar (life crystals add rows going up),
                // bubbles on the right under water.
                let y0 = hy - 11.0 * s;
                let hearts = (self.max_health / 2.0).ceil() as i32;
                let rows = (hearts + 9) / 10;
                let row_step = if rows > 1 { 10.0 * s - (rows - 2) as f32 * 2.0 * s } else { 10.0 * s };
                let y = y0 - (rows - 1) as f32 * row_step;
                for i in 0..hearts {
                    let x = hx + (i % 10) as f32 * 8.0 * s;
                    let y = y0 - (i / 10) as f32 * row_step;
                    let v = self.health - i as f32 * 2.0;
                    let blink = self.hurt_age < 0.6 && ((self.hurt_age * 10.0) as i32) % 2 == 0;
                    // Hearts tremble when health is low, and a wave runs across them on healing.
                    let calm = self.settings.reduce_motion;
                    let mut y = y;
                    if !calm && self.health <= 4.0 {
                        y += (((self.now * 14.0) as i32 * 7 + i * 13).rem_euclid(3) - 1) as f32 * s;
                    }
                    if !calm && ((self.now - self.heal_wave) * 22.0) as i32 == i {
                        y -= 2.0 * s;
                    }
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
                        let y = y - 10.0 * s - if self.thirst_shown() { 10.0 * s } else { 0.0 };
                        if self.ui.sprite("sprites/hud/air", x, y - s, 9.0 * s, 9.0 * s) {
                            continue;
                        }
                        self.ui.rect(x + s, y + s, 5.0 * s, 5.0 * s, [60, 120, 230, 255]);
                        self.ui.rect(x + 2.0 * s, y + 2.0 * s, 2.0 * s, 2.0 * s, [220, 240, 255, 255]);
                    }
                }
            }
            self.draw_subtitles(w, h);
            self.draw_scoreboard(w, h);
            if let Some(t) = &self.toast {
                // (Not over the open journal.)
                if t.until > self.now && !self.progress.journal && self.screen == Screen::Playing {
                    let text = t.text.clone();
                    let ty = if self.survival { hy - 34.0 * s } else { hy - 16.0 * s };
                    self.ui.text_centered(w / 2.0, ty, &text, WHITE);
                }
            }
            if self.show_debug {
                info_lines.insert(0, format!("Mine Clone {} (F3)", env!("CARGO_PKG_VERSION")));
                info_lines.extend(self.mods_info_lines());
                for (i, l) in info_lines.iter().enumerate() {
                    let y = 2.0 * s + i as f32 * 10.0 * s;
                    let tw = self.ui.text_width(l, s);
                    self.ui.rect(s, y - s, tw + 3.0 * s, 10.0 * s, [0, 0, 0, 110]);
                    self.ui.text(2.0 * s, y, l, [224, 224, 224, 255]);
                }
            }
            // The advancements panel covers the rest of the HUD.
            self.draw_advancements(w, h);
            self.draw_skill_page(w, h);
            self.draw_atlas(w, h);
        }

        self.draw_net_overlay(w, h);
        self.draw_voice_overlay();
        match self.screen {
            Screen::Paused => {
                self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 140]);
                let bw = 200.0 * s;
                let bh = 20.0 * s;
                let x = (w - bw) / 2.0;
                let mut y = (h / 2.0 - 100.0 * s).max(26.0 * s);
                self.ui.text_centered(w / 2.0, y - 18.0 * s, "Menu do jogo", WHITE);
                // Like the classic pause menu: everything adjustable lives in Opções (options.rs).
                let labels = [
                    "Voltar ao jogo".to_string(),
                    "Opções...".to_string(),
                    format!("Modo: {}", if self.survival { "Sobrevivência" } else { "Criativo" }),
                    format!("Shaders: {}", self.shader_name()),
                    self.net_pause_label(),
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
            Screen::Friends => self.draw_friends_menu(w, h, mouse),
            Screen::Sign if self.extras.book.is_some() => self.draw_book(w, h),
            Screen::Sign if self.naming.is_some() => self.draw_naming(w, h),
            Screen::Sign if self.cmdblocks.edit.is_some() => self.draw_cmdblock_editor(w, h),
            Screen::Sign => self.draw_sign_editor(w, h),
            Screen::Title | Screen::Worlds | Screen::CreateWorld | Screen::Multiplayer => self.draw_title_menus(w, h, mouse),
            Screen::Options | Screen::Packs | Screen::Keys | Screen::Voice => {
                if self.world_open {
                    self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 150]);
                } else {
                    let ready = self.world.meshed_count > 20;
                    self.ui.rect(0.0, 0.0, w, h, if ready { [0, 0, 0, 110] } else { [44, 34, 26, 255] });
                }
                if self.screen == Screen::Options {
                    self.draw_options(w, h, mouse);
                } else if self.screen == Screen::Keys {
                    self.draw_keys(w, h, mouse);
                } else if self.screen == Screen::Voice {
                    self.draw_voice_options(w, h, mouse);
                } else {
                    self.draw_packs(w, h, mouse);
                }
            }
            Screen::Shaders => clicked_button = self.draw_shaders_menu(w, h, mouse),
            Screen::Grimoire => self.draw_grimoire(w, h, mouse),
            Screen::Plugin => self.draw_plugin_screen(w, h, mouse),
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
                        Screen::Shaders => self.set_screen(self.options_back),
                        Screen::Dead | Screen::Title => {}
                        Screen::Worlds | Screen::CreateWorld | Screen::Multiplayer | Screen::Options | Screen::Packs | Screen::Keys | Screen::Voice => self.menu_back(),
                        _ => self.set_screen(Screen::Playing),
                    }
                }
            }
        }
    }

    fn toggle_game_mode(&mut self) {
        self.survival = !self.survival;
        if self.survival {
            self.body.flying = false;
            self.health = self.max_health;
            self.fall_peak = self.body.pos.y;
            self.toast("Modo sobrevivência: cuidado com quedas e monstros", 4.0);
        } else {
            self.toast("Modo criativo", 3.0);
        }
    }

    fn pause_button(&mut self, i: usize) {
        if let Some(a) = self.audio.as_mut() {
            a.play(Sfx::Click, self.settings.sound_volume);
        }
        match i {
            0 => self.set_screen(Screen::Playing),
            1 => self.open_options(),
            2 => self.toggle_game_mode(),
            3 => {
                self.options_back = Screen::Paused;
                self.shader_scroll = 0;
                self.set_screen(Screen::Shaders);
            }
            4 => self.net_pause_button(),
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
        // Whatever ran since the last mark of the previous frame: drawing it and the window.
        self.prof.mark("desenhar e janela");
        self.prof.frames += 1;
        self.prof.roll(raw_dt as f64);
        self.frame_times.push(dt);
        if self.frame_times.len() >= 30 {
            self.fps = self.frame_times.len() as f32 / self.frame_times.iter().sum::<f32>();
            self.frame_times.clear();
        }
        self.update(dt);
        // Blocks placed or broken this frame show in this very frame, with their real look.
        self.world.remesh_edits();
        if !self.world.events.is_empty() {
            let events = std::mem::take(&mut self.world.events);
            if let Some(r) = self.renderer.get_mut() {
                r.apply_mesh_events(events);
            }
        }
        self.prof.mark("blocos editados");
        if self.bench_tick(raw_dt) {
            self.save_all();
            std::process::exit(0);
        }

        // Eye height eases between standing, sneaking and lying down instead of jumping.
        let stand = if self.body.flying { voxel::physics::EYE_HEIGHT } else { voxel::physics::EYE_HEIGHT - 0.25 * self.crouch };
        let head = self.body.pos + Vec3::Y * (stand + (0.42 - stand) * self.prone_t - self.sink);
        let head = self.sleep.map(|s| s.eye()).unwrap_or(head);
        let playing_view = self.world_open && !matches!(self.screen, Screen::Loading);
        let third = playing_view && self.camera_mode != 0;
        let mut view_fwd = self.forward();
        let eye = if third {
            // Pull the camera back (or around to the front), stopping short of walls.
            let dir = if self.camera_mode == 1 { -view_fwd } else { view_fwd };
            // Test hook: MC_ORBIT=degrees swings the camera around the player for side shots.
            let orbit = std::env::var("MC_ORBIT").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
            let dir = glam::Quat::from_rotation_y(orbit.to_radians()) * dir;
            let world = &self.world;
            // Lying down, the camera orbits a bit above the body so it clears the ground.
            let pivot = head + Vec3::Y * 0.6 * self.prone_t;
            let pivot = self.sleep.map(|s| s.middle()).unwrap_or(pivot);
            // Test hook: MC_CAMDIST=blocks for close-ups.
            let far = std::env::var("MC_CAMDIST").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(4.0);
            let dist = raycast(pivot, dir, far, |p| voxel::physics::Terrain::solid(world, p)).map(|h| (h.distance - 0.3).max(0.2)).unwrap_or(far);
            view_fwd = -dir;
            pivot + dir * dist
        } else if playing_view && self.settings.view_bobbing && !self.settings.reduce_motion {
            head + self.anim.view_bob(self.yaw, self.body.on_ground && !self.body.flying)
        } else {
            head
        };
        let eye = if playing_view { eye + self.shake_offset() } else { eye };
        let block = eye.floor().as_ivec3();
        let (top, horizon, daylight, sun) = self.sky();
        // Deep down the sky fades out, and the Underworld glows red like the Nether (deep.rs).
        let (top, horizon, daylight, underworld) = self.deep_sky(eye.y, top, horizon, daylight);
        let (top, horizon) = if underworld || self.dim != voxel::worldgen::Dim::Overworld { (top, horizon) } else { self.tr3_sky(top, horizon, daylight) };
        let underwater = self.body.head_in_water;
        let rd = self.settings.render_distance as f32 * 16.0;
        // Distant terrain pushes the fog out to its own edge.
        // The Nether has a roof: no distant terrain there.
        let nether = self.dim == voxel::worldgen::Dim::Nether || underworld;
        let end = self.dim == voxel::worldgen::Dim::End;
        // The Otherside (mods/portals.rs) is a closed, dark world like the Nether.
        let other = self.dim == voxel::worldgen::Dim::Otherside;
        // Nor in the mod pack's dimensions (floating islands, closed caverns).
        let extra = if self.dim != voxel::worldgen::Dim::Overworld || eye.y < -24.0 { 0.0 } else { self.settings.lod_distance.max(0) as f32 };
        let view_end = rd + extra;
        let (fog_color, fog_start, fog_end) = if underwater {
            ([0.04, 0.12, 0.35], 0.0, 28.0)
        } else if end {
            // A dark violet void.
            ([0.03, 0.018, 0.045], rd * 0.45, rd - 2.0)
        } else if underworld {
            // The Underworld: a hot haze over the lava sea.
            ([0.22, 0.06, 0.03], rd * 0.2, rd * 0.9)
        } else if nether {
            // A thick red haze instead of a sky.
            ([0.2, 0.035, 0.025], rd * 0.15, rd * 0.85)
        } else if other {
            // A cold blue-black murk.
            ([0.03, 0.07, 0.08], rd * 0.3, rd - 2.0)
        } else if extra > 0.0 {
            (horizon, rd * 0.6, view_end - 8.0)
        } else {
            (horizon, rd * 0.55, rd - 2.0)
        };
        let (fog_start, fog_end) = if underwater || nether || end { (fog_start, fog_end) } else { self.mist_fog(fog_start, fog_end) };
        let (fog_color, fog_start, fog_end) = self.effect_fog(fog_color, fog_start, fog_end);
        // Real chunks surely cover this radius (see World::update); distant terrain starts here.
        let lod_inner = (self.settings.render_distance as f32 - 0.5) * 16.0;
        self.lod.update(&self.renderer.device, eye, lod_inner, if extra > 0.0 { view_end } else { 0.0 }, self.settings.lod_detail as usize);
        let mut effects = self.terrain_effects();
        effects[3] = lod_inner;
        let mut g = Globals {
            cam_pos: [eye.x - block.x as f32, eye.y - block.y as f32, eye.z - block.z as f32, (self.fx_time % 3600.0) as f32],
            cam_world: [eye.x, eye.y, eye.z, CLOUD_HEIGHT],
            fog_color: [fog_color[0], fog_color[1], fog_color[2], if nether || end || other { 0.0 } else { daylight }],
            fog: [fog_start, fog_end, if underwater { 1.0 } else { 0.0 }, if nether || end || other { 1.0 } else { settings::brightness_lift(self.settings.brightness) }],
            sun_dir: [sun.x, sun.y, sun.z, if self.settings.relief { 1.0 } else { 0.0 }],
            sky_top: [top[0], top[1], top[2], 1.0],
            sky_horizon: [horizon[0], horizon[1], horizon[2], 1.0],
            effects,
            // The light a held torch gives comes from the player's hand, wherever the camera is.
            hand_light: [head.x - block.x as f32, head.y - 0.4 - block.y as f32, head.z - block.z as f32, self.hand_light_level()],
            // z: 1 Nether, 2 End, -1 Aether, 1.25 Otherside (sky and ambient light in the shaders).
            weather: [self.weather.rain, self.weather.flash, if nether { 1.0 } else if end { 2.0 } else { self.mod_dim_uniform() }, self.night_vision()],
            dyn_lights: self.dynamic_lights(eye, block),
            red_lights: self.red_lights(block),
            extra: [self.settings.layered_clouds as u8 as f32, self.settings.emissive_textures as u8 as f32, 0.0, self.season_uniform()],
            pom: self.parallax_uniform(),
            clouds: [self.cloud_wind[0], self.cloud_wind[1], self.cloud_drift, (self.fx_time % 100_000.0) as f32],
            mat: material::uniform(self.settings.materials, self.settings.shader_quality == ShaderQuality::Fast),
            anim: {
                let mut a = self.anim_layers();
                a[3] = if self.dim == voxel::worldgen::Dim::Overworld { self.moon_uniform() } else { 0.0 };
                a
            },
            ..Default::default()
        };
        // A torch in the hand casts shadows (with Sombras on): the blocks around the player.
        if self.settings.shadows > 0 && g.hand_light[3] > 0.0 {
            let n = scenefx::OCCL_SIZE as i32;
            let corner = head.floor().as_ivec3() - IVec3::splat(n / 2);
            if self.renderer.fx.occluders_stale(corner) {
                let reg = &self.world.reg;
                let mut grid = vec![0u8; (n * n * n) as usize];
                for z in 0..n {
                    for x in 0..n {
                        let column = corner + IVec3::new(x, 0, z);
                        for y in 0..n {
                            if reg.is_opaque(self.world.block_or_air(column + IVec3::Y * y)) {
                                grid[((z * n + y) * n + x) as usize] = 1;
                            }
                        }
                    }
                }
                let r = &mut *self.renderer;
                r.fx.upload_occluders(&r.queue, corner, &grid);
            }
            let c = corner - block;
            g.occl = [c.x as f32, c.y as f32, c.z as f32, 1.0];
        }
        // Shadows and reflections (scenefx.rs); the renderer fills in the rest of the globals.
        self.renderer.fx.level = self.settings.shadows.min(2);
        self.renderer.fx.reflections = self.settings.water_reflections;
        self.renderer.fx.fancy = self.settings.shader_quality == ShaderQuality::Fancy;
        let cam = Camera {
            block,
            frac: Vec3::from_slice(&g.cam_pos[..3]),
            forward: view_fwd,
            fov_y: (self.settings.fov * (1.0 + if self.settings.reduce_motion { 0.0 } else { 0.12 * self.fov_kick }) * if self.spyglass_up() { 0.2 } else { 1.0 }).to_radians(),
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
        self.prof.mark("câmera, luzes e céu");
        self.draw_ui(lines);
        self.prof.mark("interface (HUD)");

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
            // Test hook: MC_PLACE=frames puts a block where the player aims once the view is
            // ready and saves that many frames after it (out_0.png, out_1.png...).
            static PLACED: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);
            let place_frames = std::env::var("MC_PLACE").ok().and_then(|v| v.parse::<i32>().ok());
            let k = PLACED.load(std::sync::atomic::Ordering::Relaxed);
            if let (Some(n), true) = (place_frames, k >= 0) {
                if let Ok(scene) = std::env::var("MC_ELEMENTS") {
                    self.elements_frame(k, &scene);
                }
                capture = true;
                PLACED.store(k + 1, std::sync::atomic::Ordering::Relaxed);
                exit_after = k >= n;
            } else if (self.loaded_frames > 30 && self.world.pending_jobs() == 0 && self.lod.is_complete() && self.fluids.time >= self.args.wait as f64) || self.loaded_frames > 1500 {
                static SOCIAL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                static TURNS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                if let Some(mode) = std::env::var("MC_TURNS").ok().filter(|_| !TURNS.swap(true, std::sync::atomic::Ordering::Relaxed)) {
                    // Test hook: blocks put down from different sides (chests.rs).
                    self.turns_test(&mode);
                    self.loaded_frames = 0;
                } else if let Some(scene) = std::env::var("MC_SOCIAL").ok().filter(|_| !SOCIAL.swap(true, std::sync::atomic::Ordering::Relaxed)) {
                    // Test hook: mobs getting on with each other (social.rs), set up once the
                    // world around is loaded; then wait for the floor to mesh again.
                    self.test_social(&scene);
                    self.loaded_frames = 0;
                } else if place_frames.is_some() && std::env::var("MC_ELEMENTS").is_ok() {
                    PLACED.store(0, std::sync::atomic::Ordering::Relaxed);
                } else if place_frames.is_some() {
                    let name = std::env::var("MC_PLACE_BLOCK").unwrap_or("oak_planks".into());
                    if let (true, Some(hit)) = (name == "break", self.target) {
                        // MC_PLACE_BLOCK=break: break the block aimed at instead.
                        self.world.break_block(hit.block);
                        self.world.mesh_now(hit.block);
                    } else {
                        let id = self.reg.id_or_air(&name);
                        log::info!("place test: {:?}", self.place_block_as(id));
                    }
                    PLACED.store(0, std::sync::atomic::Ordering::Relaxed);
                } else {
                    capture = true;
                    exit_after = true;
                }
            }
        }
        let selection = if self.screen == Screen::Playing || self.args.screenshot.is_some() { self.target.map(|t| t.block) } else { None };
        let mut items = self.mobs.instances(&self.world, block, rd + 16.0);
        // Pets and mounts with a better body in summons.rs: theirs comes out, that one goes in.
        items.retain(|(d, _)| self.mobs.reg.defs.get(*d).is_none_or(|def| summons::pet_model(&def.name).is_none()));
        let musician = self.musician_instances(&mut items, block);
        let mob_items = self.arrow_and_bow_instances(&items, block);
        items.extend(self.pet_models(block, rd + 16.0));
        items.extend(musician);
        let (net_models, mut net_held) = self.net_instances(block);
        items.extend(net_models);
        let (stand_models, stand_held) = self.stand_instances(block, rd + 16.0);
        items.extend(stand_models);
        net_held.extend(stand_held);
        // Before the arm and the item are placed, so both see the frozen swing.
        if let Some(k) = self.args.swing {
            self.anim.swing = k;
            // MC_SWING_STYLE=0..3: which combo swing the frozen frame shows.
            if let Some(st) = std::env::var("MC_SWING_STYLE").ok().and_then(|v| v.parse().ok()) {
                self.anim.swing_style = st;
            }
            self.anim.hand_height = 1.0;
        }
        let mut held_third = None;
        // Drawn but casting no shadow (first-person arm, inventory figure), and the other way
        // round (the body in first person still casts its shadow).
        let mut overlay = Vec::new();
        let mut shadow_only = Vec::new();
        // The held torch's flame top, for its fire (torch.rs).
        let mut torch_top = None;
        if let Some(pd) = self.mobs.reg.id("player").filter(|_| playing_view) {
            let def = &self.mobs.reg.defs[pd];
            let pose = self.pose(false);
            if !third && self.renderer.fx.level > 0 {
                let bones = self.anim.bones(def, &pose, block, 1.0, 1.0);
                shadow_only.extend(self.armor_instances(&bones));
                shadow_only.extend(self.shield_instances(&self.shields_worn(), &bones, Game::arm_pivots(def), self.extras.raise));
                shadow_only.push((pd, bones));
            }
            if third {
                let l = self.world.light((self.body.pos + Vec3::Y).floor().as_ivec3());
                let mut bones = self.anim.bones(def, &pose, block, (l >> 4) as f32 / 15.0, (l & 15) as f32 / 15.0);
                let pv = def.parts[2].pivot;
                // Asleep, the hands are empty.
                let awake = self.sleep.is_none();
                held_third = self.held_item_third_person(bones.forearm(2), Vec3::new(pv.0 as f32, pv.1 as f32, pv.2 as f32)).filter(|_| awake);
                if let Some((t, mesh)) = self.held_torch().filter(|_| awake).and_then(|t| Some((t, self.entities.torch_mesh(t)?))) {
                    let (tb, top) = torch::hold(t, bones.forearm(2), Vec3::new(pv.0 as f32, pv.1 as f32, pv.2 as f32), 1.0, 0.62, self.now, [bones.light[0], 1.0, 0.0, 0.0]);
                    items.push((mesh, tb));
                    torch_top = Some((t, top + block.as_vec3(), 1.0));
                }
                // An instrument is held in both hands, in 3D.
                if let Some((i, play)) = self.held_instrument().filter(|_| awake) {
                    if let (Some(ib), Some(mesh)) = (instruments::pose(def, &mut bones, i, play), self.entities.instrument_mesh(i)) {
                        items.push((mesh, ib));
                    }
                }
                items.extend(self.armor_instances(&bones));
                items.extend(self.shield_instances(&self.shields_worn(), &bones, Game::arm_pivots(def), self.extras.raise));
                items.push((pd, bones));
            } else if self.screen == Screen::Playing && !self.hide_hud && self.sleep.is_none() {
                // The arm always shows; a held item sits in its fist (held_item_instance).
                let l = self.world.light(head.floor().as_ivec3());
                let light = [(l >> 4) as f32 / 15.0, ((l & 15) as f32 / 15.0).max(self.hand_light_level()), 0.0, 0.0];
                let two_hands = self.held_instrument().and_then(|(i, play)| {
                    let (arms, ib) = instruments::first_person(def, i, play, eye - block.as_vec3(), self.yaw, self.pitch, light)?;
                    Some((arms, (self.entities.instrument_mesh(i)?, ib)))
                });
                if let Some((arms, inst)) = two_hands {
                    overlay.push((pd, arms));
                    overlay.push(inst);
                } else {
                    let arm = match self.first_person_holding(def, eye, block, light[0], light[1]) {
                        Some(arm) if self.camera_mode == 0 => arm,
                        _ => self.anim.first_person_arm(def, eye - block.as_vec3(), self.yaw, self.pitch, light[0], light[1]),
                    };
                    // Same size against the fist as the other held items in first person.
                    if let Some((t, mesh)) = self.held_torch().and_then(|t| Some((t, self.entities.torch_mesh(t)?))).filter(|_| self.camera_mode == 0) {
                        let pv = def.parts[2].pivot;
                        let (tb, top) = torch::hold(t, arm.m[2], Vec3::new(pv.0 as f32, pv.1 as f32, pv.2 as f32), 0.55, 0.2, self.now, [light[0], 1.0, 0.0, 0.0]);
                        overlay.push((mesh, tb));
                        torch_top = Some((t, top + block.as_vec3(), 0.5));
                    }
                    // A shield in either hand (shield.rs); held in the main hand it takes the
                    // place of the arm, as in the classic game.
                    let (shields, main_shield) = self.shield_first_person(eye - block.as_vec3(), light, Vec3::ZERO);
                    overlay.extend(shields);
                    if !main_shield {
                        overlay.push((pd, arm));
                    }
                }
            }
        }
        if let Some((t, top, k)) = torch_top {
            if self.args.screenshot.is_none() {
                self.torch_flames(t, top, k, dt);
            } else if !self.torch_lit {
                // Screenshots freeze the particles: light the fire with a moment of flames.
                self.torch_lit = true;
                for _ in 0..40 {
                    self.torch_flames(t, top, k, 0.015);
                    self.particles.update(&self.world, 0.015);
                }
            }
        }
        for p in &self.paintings {
            items.push((self.entities.painting_mesh(p.def), p.bones(block, &self.world)));
        }
        items.extend(self.cart_instances(block));
        items.extend(self.boat_instances(block));
        items.extend(self.wagon_instances(block, rd + 16.0));
        items.extend(self.aircraft_instances(block));
        items.extend(self.windmill_instances(block, eye, rd + 16.0));
        // Summoned minions and sentries, with their own bodies (summons.rs).
        items.extend(self.minion_instances(block));
        items.extend(self.sentry_instances(block));
        items.extend(self.tr_sentry_models(block));
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
            // Like the classic screen: the head follows the mouse, but only so far.
            let look_x = (((mx - (bx + bw / 2.0)) / (bw * 0.6)).atan() * 0.45).clamp(-0.5, 0.5);
            let look_y = ((((by + bh * 0.25) - my) / (bh * 0.6)).atan() * 0.25).clamp(-0.3, 0.3);
            let def = &self.mobs.reg.defs[pd];
            let pose = player_model::Pose { pos: block.as_vec3(), look_yaw: look_x - std::f32::consts::PI, look_pitch: -look_y, on_ground: true, ..Default::default() };
            let base = player_model::Animator::lying(0.0).bones(def, &pose, block, 1.0, 1.0);
            // Stand it up facing the camera, scaled into the box.
            let face = glam::Mat4::from_cols(right.extend(0.0), up.extend(0.0), (-f).extend(0.0), Vec4::W);
            let m = glam::Mat4::from_translation(feet - block.as_vec3()) * face * glam::Mat4::from_rotation_y(look_x * 0.4) * glam::Mat4::from_scale(Vec3::splat(k));
            let mut b = base;
            for mm in b.m.iter_mut() {
                *mm = (m * glam::Mat4::from_cols_array_2d(mm)).to_cols_array_2d();
            }
            overlay.extend(self.armor_instances(&b));
            overlay.extend(self.shield_instances(&self.shields_worn(), &b, Game::arm_pivots(def), 0.0));
            overlay.push((pd, b));
            let back = center + f * (world_h * 1.3) - block.as_vec3();
            figure_backdrop = Some(item_render::GpuItem { pos: [back.x, back.y, back.z, world_h * 1.22 * (d + world_h * 1.3) / d], info: [0.0, 0.0, 0.0, 0.0], layers: [0.0; 4], rot: [0.0; 4], ..bytemuck::Zeroable::zeroed() });
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
        self.entities.prepare_split(&self.renderer, items, overlay, shadow_only);
        let letters = self.sign_letters(block);
        self.entities.particles.prepare(&self.renderer, &self.particles, &self.world, block, letters);
        let mut drops = self.drop_instances(block);
        drops.extend(figure_backdrop);
        drops.extend(self.tnt_instances(block));
        if !third && self.screen == Screen::Playing && !self.hide_hud && self.camera_mode == 0 {
            let glint = self.held_stack().is_some_and(|st| enchanting::is_enchanted(&st));
            drops.extend(self.held_item_instance(eye, block).map(|mut g| {
                if glint {
                    g.bmin = drop_fx::ENCH_GLINT;
                }
                g
            }));
        }
        drops.extend(held_third);
        drops.extend(mob_items);
        drops.extend(net_held);
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
                    ..bytemuck::Zeroable::zeroed()
                });
            }
        }
        let halos = self.drop_halos(block);
        let halo_count = halos.len();
        drops.extend(halos);
        self.entities.items.prepare(&self.renderer, &drops, halo_count);
        let grab_backdrop = self.want_backdrop && self.screen == Screen::Loading;
        self.renderer.extra_lines = self.fishing_line(block, third);
        self.prof.mark("entidades e itens");
        let (info, shot) = self.renderer.render(&cam, &mut g, selection, self.settings.show_clouds && self.dim == voxel::worldgen::Dim::Overworld && self.body.pos.y > -24.0, Some(&self.entities), Some(&self.lod), &self.ui, capture || grab_backdrop);
        self.prof.mark("render: outros");
        // The renderer's own split; "render: outros" keeps what it doesn't cover.
        let t = self.renderer.timing;
        for (k, name) in ["render: preparar buffers", "render: esperar a imagem (GPU)", "render: gravar os passes", "render: enviar e apresentar"].into_iter().enumerate() {
            self.prof.add(name, t[k]);
        }
        self.prof.add("render: outros", -t.iter().sum::<f64>());
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
        if self.bench.started {
            self.bench.lod_frame(info.lod_drawn, info.lod_quads);
        }
        if let Some((w, h, px)) = shot {
            let k = std::env::var("MC_PLACE").ok().map(|_| self.loaded_frames);
            let path = if let (Some(k), Some(p)) = (k, self.args.screenshot.as_ref()) {
                p.with_file_name(format!("{}_{k}.png", p.file_stem().unwrap_or_default().to_string_lossy()))
            } else if exit_after {
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
                        let n = self.gallery_count();
                        self.toast(format!("Foto salva: {} ({n} na galeria)", path.display()), 3.0);
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
        let attrs = Window::default_attributes()
            .with_title("Mine Clone")
            .with_window_icon(icon)
            .with_inner_size(winit::dpi::PhysicalSize::new(w, h));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        self.game = Some(Game::new(Some(window), self.args.clone()));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(game) = self.game.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => {
                game.save_all();
                game.net_stop();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => game.renderer.resize(size.width, size.height),
            WindowEvent::Focused(false) => {
                game.keys.clear();
                if game.screen == Screen::Playing && game.args.screenshot.is_none() && !game.args.server {
                    game.set_screen(Screen::Paused);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    game.window_key(code, event.state == ElementState::Pressed, event.repeat);
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
                if idx == 0 {
                    game.opts.held = pressed;
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
        if let Some(g) = self.game.as_mut() {
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
    if let Some(port) = args.relay {
        if let Err(e) = net::relay::run(port) {
            log::error!("relay: {e}");
            std::process::exit(1);
        }
        return;
    }
    if args.server {
        // No window and no graphics: the server only ticks the world and talks to players.
        let mut game = Game::new(None, args);
        while game.server_tick() {}
        return;
    }
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
    fn every_structure_file_loads() {
        // A bad structure (say, a row that isn't a string) would drop every building.
        let base = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let (reg, _, _) = load_game_data(base).unwrap();
        let s = voxel::structures::Structures::from_ron_many(&[embedded::STRUCTURES_RON, &merge_list(embedded::TR_STRUCTURES_RON, embedded::TR3_STRUCTURES_RON), embedded::MODS_STRUCTURES_RON, embedded::MODS_DIMS_STRUCTURES_RON], &reg).unwrap();
        for name in ["templo_escamado", "lago_cintilante", "masmorra_blue", "masmorra_pink", "campo_de_golfe"] {
            assert!(s.list.iter().any(|st| st.def.name == name), "{name}");
        }
    }

    #[test]
    fn ice_sand_and_snow_blocks_have_their_feel() {
        // The game's own blocks get the ground feel by name: a renamed block would lose it.
        let base = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let (reg, _, _) = load_game_data(base).unwrap();
        use voxel::physics::Ground;
        for n in ["ice", "packed_ice", "blue_ice", "frosted_ice"] {
            let b = reg.get(reg.id(n).unwrap_or_else(|| panic!("{n} missing")));
            assert!(Ground::of_block(&b.name, b.variant).slippery(), "{n}");
        }
        for n in ["sand", "red_sand", "snow", "soul_sand", "slime_block"] {
            let b = reg.get(reg.id(n).unwrap_or_else(|| panic!("{n} missing")));
            assert_ne!(Ground::of_block(&b.name, b.variant), Ground::FIRM, "{n}");
        }
        assert!(reg.id("snow_layer").is_some());
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
