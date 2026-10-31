//! Plugins: mods as WebAssembly (Lote 8 of plan/o-que-falta.md, the M3 step of
//! plan/modding.md). Put `.wasm` files (or `.wat`, the same in text) in `plugins/` next to the
//! game; each runs in its own sandbox (wasmi, an interpreter) with a fuel budget per call, so a
//! plugin from a stranger can't touch the computer or freeze the game. They run where the world
//! is simulated: alone, or on the host.
//!
//! What a plugin may export (all optional; strings are UTF-8 as pointer + length):
//! - `memory`, and `buffer() -> i32`: where the game writes text for it (at least 256 bytes)
//! - `init()`: once, when it loads
//! - `tick(dt: f32)`: every frame while playing
//! - `command(ptr, len) -> i32`: a chat line starting with a command it added; 1 = handled
//! - `block_broken(x, y, z, block)` and `block_placed(x, y, z, block)`: by the player
//! - `items()`: once at load, before any world, to add items with `add_item` (Lote 9)
//! - `item_used(ptr, len) -> i32`: right click holding one of its items (the name); 1 = handled
//! - `button(id)`: a button on its screen was clicked
//!
//! What the game gives it, module "jogo": `say(ptr, len)`, `log(ptr, len)`,
//! `block_at(x, y, z) -> i32`, `set_block(x, y, z, block) -> i32`, `block_id(ptr, len) -> i32`
//! (-1 unknown), `block_name(block, ptr, cap) -> i32` (bytes written), `player(axis) -> f32`
//! (0 x, 1 y, 2 z), `time() -> f32`, `set_time(t: f32)`, `give(ptr, len, count) -> i32`,
//! `spawn(ptr, len, x: f32, y: f32, z: f32) -> i32`, `add_command(ptr, len)`,
//! `run(ptr, len)` (a chat command, with every permission), `random() -> f32`.
//!
//! Screens and items (Lote 9): `screen_open(ptr, len)` opens a window with that title,
//! `screen_text(ptr, len)` and `screen_button(id, ptr, len)` add rows to it (also later, to
//! update it), `screen_close()`. `add_item(name, name_len, label, label_len, pixels, stack,
//! food) -> i32`, only inside `items()`, adds an item: `pixels` points at a 16×16 RGBA picture
//! (1024 bytes, or -1 for none), `food` is hunger restored (0 = not food).
//! `held(ptr, cap) -> i32` writes the held item's name.
//!
//! A host sends its plugins to everyone who joins, so its items and screens work for them
//! too; on a guest `tick` does not run (the host's world does the ticking).
//! See `plugins-exemplos/exemplo.wat` for a small one (copy it into `plugins/` to try it).

use std::collections::HashSet;
use std::path::Path;

use glam::{IVec3, Vec3};
use voxel::block::BlockId;
use wasmi::{Caller, Config, Engine, Extern, Instance, Linker, Memory, Module, Store, TypedFunc};

use crate::{Game, Screen};

/// Instructions a plugin may run per call before it is stopped.
const FUEL: u64 = 20_000_000;
const MAX_TEXT: usize = 4096;
/// Most bytes of plugins a host sends to a guest.
const MAX_SHARE: usize = 8 << 20;
const MAX_ROWS: usize = 24;

/// An item a plugin asked for.
struct NewItem {
    name: String,
    label: String,
    pixels: Option<Vec<u8>>,
    max_stack: u8,
    food: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Row {
    Text(String),
    Button(i32, String),
}

/// A window a plugin opened.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PluginScreen {
    plugin: usize,
    pub title: String,
    pub rows: Vec<Row>,
}

#[derive(Default)]
struct Ctx {
    /// The game, only while a plugin call runs (the plugins are taken out of it meanwhile).
    game: Option<std::ptr::NonNull<Game>>,
    commands: Vec<String>,
    index: usize,
    items: Vec<NewItem>,
    /// Its window as it builds it, and whether it changed since the game last looked.
    screen: Option<PluginScreen>,
    screen_changed: bool,
}

struct Plugin {
    name: String,
    /// The file as loaded (for sending to guests).
    file: String,
    bytes: std::sync::Arc<Vec<u8>>,
    store: Store<Ctx>,
    instance: Instance,
    memory: Option<Memory>,
    buffer: Option<u32>,
    tick: Option<TypedFunc<f32, ()>>,
    command: Option<TypedFunc<(i32, i32), i32>>,
    broken: Option<TypedFunc<(i32, i32, i32, i32), ()>>,
    placed: Option<TypedFunc<(i32, i32, i32, i32), ()>>,
    button: Option<TypedFunc<i32, ()>>,
    item_used: Option<TypedFunc<(i32, i32), i32>>,
    /// Stopped after a trap or running out of fuel.
    failed: bool,
}

#[derive(Default)]
pub(crate) struct Plugins {
    list: Vec<Plugin>,
    /// What loading said (shown once in the chat).
    pub notes: Vec<String>,
    loaded: bool,
    /// The window a plugin has open.
    pub screen: Option<PluginScreen>,
    /// Items the plugins added.
    item_names: HashSet<String>,
    /// Items before any plugin's (the part of the list host and guest must share to start).
    pub base_items: usize,
    /// These came from the host we are visiting.
    from_host: bool,
}

fn text(caller: &Caller<'_, Ctx>, ptr: i32, len: i32) -> String {
    let Some(Extern::Memory(m)) = caller.get_export("memory") else { return String::new() };
    let len = (len.max(0) as usize).min(MAX_TEXT);
    let mut buf = vec![0u8; len];
    if m.read(caller, ptr.max(0) as usize, &mut buf).is_err() {
        return String::new();
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// The game, during a call. Safe because the plugins live outside it while they run, and the
/// pointer is cleared right after.
fn game<'a>(caller: &mut Caller<'_, Ctx>) -> Option<&'a mut Game> {
    let p = caller.data().game?;
    // SAFETY: set from `&mut Game` by `Game::with_plugins` for the length of one call.
    Some(unsafe { &mut *p.as_ptr() })
}

/// Write `text` into the plugin's memory at `ptr` (at most `cap` bytes): bytes written.
fn write_text(c: &mut Caller<'_, Ctx>, text: &str, ptr: i32, cap: i32) -> i32 {
    let bytes = &text.as_bytes()[..text.len().min(cap.max(0) as usize)];
    let Some(Extern::Memory(m)) = c.get_export("memory") else { return 0 };
    if m.write(c, ptr.max(0) as usize, bytes).is_err() {
        return 0;
    }
    bytes.len() as i32
}

fn linker(engine: &Engine) -> Result<Linker<Ctx>, wasmi::Error> {
    let mut l = Linker::<Ctx>::new(engine);
    l.func_wrap("jogo", "say", |mut c: Caller<'_, Ctx>, ptr: i32, len: i32| {
        let t = text(&c, ptr, len);
        if let Some(g) = game(&mut c) {
            g.plugin_say(t);
        }
    })?;
    l.func_wrap("jogo", "log", |c: Caller<'_, Ctx>, ptr: i32, len: i32| {
        log::info!("plugin: {}", text(&c, ptr, len));
    })?;
    l.func_wrap("jogo", "block_at", |mut c: Caller<'_, Ctx>, x: i32, y: i32, z: i32| -> i32 {
        game(&mut c).and_then(|g| g.world.block(IVec3::new(x, y, z))).map(|b| b as i32).unwrap_or(-1)
    })?;
    l.func_wrap("jogo", "set_block", |mut c: Caller<'_, Ctx>, x: i32, y: i32, z: i32, id: i32| -> i32 {
        let Some(g) = game(&mut c) else { return 0 };
        if id < 0 || id as usize >= g.reg.len() {
            return 0;
        }
        g.world.set_block(IVec3::new(x, y, z), id as BlockId) as i32
    })?;
    l.func_wrap("jogo", "block_id", |mut c: Caller<'_, Ctx>, ptr: i32, len: i32| -> i32 {
        let t = text(&c, ptr, len);
        game(&mut c).and_then(|g| g.reg.id(t.trim_start_matches("minecraft:"))).map(|b| b as i32).unwrap_or(-1)
    })?;
    l.func_wrap("jogo", "block_name", |mut c: Caller<'_, Ctx>, id: i32, ptr: i32, cap: i32| -> i32 {
        let Some(name) = game(&mut c).filter(|g| id >= 0 && (id as usize) < g.reg.len()).map(|g| g.reg.get(id as BlockId).name.clone()) else { return 0 };
        write_text(&mut c, &name, ptr, cap)
    })?;
    l.func_wrap("jogo", "held", |mut c: Caller<'_, Ctx>, ptr: i32, cap: i32| -> i32 {
        let Some(name) = game(&mut c).and_then(|g| g.held_stack().map(|s| g.items.get(s.item).name.clone())) else { return 0 };
        write_text(&mut c, &name, ptr, cap)
    })?;
    l.func_wrap("jogo", "screen_open", |mut c: Caller<'_, Ctx>, ptr: i32, len: i32| {
        let title: String = text(&c, ptr, len).chars().take(64).collect();
        let d = c.data_mut();
        d.screen = Some(PluginScreen { plugin: d.index, title, rows: Vec::new() });
        d.screen_changed = true;
    })?;
    l.func_wrap("jogo", "screen_text", |mut c: Caller<'_, Ctx>, ptr: i32, len: i32| {
        let t: String = text(&c, ptr, len).chars().take(80).collect();
        let d = c.data_mut();
        if let Some(sc) = d.screen.as_mut().filter(|sc| sc.rows.len() < MAX_ROWS) {
            sc.rows.push(Row::Text(t));
            d.screen_changed = true;
        }
    })?;
    l.func_wrap("jogo", "screen_button", |mut c: Caller<'_, Ctx>, id: i32, ptr: i32, len: i32| {
        let t: String = text(&c, ptr, len).chars().take(40).collect();
        let d = c.data_mut();
        if let Some(sc) = d.screen.as_mut().filter(|sc| sc.rows.len() < MAX_ROWS) {
            sc.rows.push(Row::Button(id, t));
            d.screen_changed = true;
        }
    })?;
    l.func_wrap("jogo", "screen_close", |mut c: Caller<'_, Ctx>| {
        let d = c.data_mut();
        d.screen = None;
        d.screen_changed = true;
    })?;
    l.func_wrap(
        "jogo",
        "add_item",
        |mut c: Caller<'_, Ctx>, nptr: i32, nlen: i32, lptr: i32, llen: i32, pixels: i32, stack: i32, food: i32| -> i32 {
            // Only while loading: the item lists are fixed once a world is open.
            if c.data().game.is_some() || c.data().items.len() >= 256 {
                return 0;
            }
            let name: String = text(&c, nptr, nlen).trim().to_lowercase().chars().filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_').take(48).collect();
            if name.is_empty() {
                return 0;
            }
            let label: String = text(&c, lptr, llen).chars().take(48).collect();
            let pixels = (pixels >= 0)
                .then(|| {
                    let Some(Extern::Memory(m)) = c.get_export("memory") else { return None };
                    let mut buf = vec![0u8; 16 * 16 * 4];
                    m.read(&c, pixels as usize, &mut buf).ok().map(|_| buf)
                })
                .flatten();
            c.data_mut().items.push(NewItem { label: if label.is_empty() { name.replace('_', " ") } else { label }, name, pixels, max_stack: stack.clamp(1, 64) as u8, food: food.clamp(0, 20) as u8 });
            1
        },
    )?;
    l.func_wrap("jogo", "player", |mut c: Caller<'_, Ctx>, axis: i32| -> f32 {
        game(&mut c).map(|g| g.body.pos.to_array()[axis.clamp(0, 2) as usize]).unwrap_or(0.0)
    })?;
    l.func_wrap("jogo", "time", |mut c: Caller<'_, Ctx>| -> f32 { game(&mut c).map(|g| g.time_of_day).unwrap_or(0.0) })?;
    l.func_wrap("jogo", "set_time", |mut c: Caller<'_, Ctx>, t: f32| {
        if let Some(g) = game(&mut c) {
            if t.is_finite() {
                g.time_of_day = t.rem_euclid(24000.0);
            }
        }
    })?;
    l.func_wrap("jogo", "give", |mut c: Caller<'_, Ctx>, ptr: i32, len: i32, count: i32| -> i32 {
        let name = text(&c, ptr, len);
        let Some(g) = game(&mut c) else { return 0 };
        let Some(id) = g.items.id(name.trim_start_matches("minecraft:")) else { return 0 };
        let n = count.clamp(1, voxel::item::PILE as i32) as u16;
        g.surv.inv.add(&g.items, voxel::item::ItemStack::new(id, n)).is_none() as i32
    })?;
    l.func_wrap("jogo", "spawn", |mut c: Caller<'_, Ctx>, ptr: i32, len: i32, x: f32, y: f32, z: f32| -> i32 {
        let name = text(&c, ptr, len);
        game(&mut c).map(|g| g.mobs.spawn(&name, Vec3::new(x, y, z)) as i32).unwrap_or(0)
    })?;
    l.func_wrap("jogo", "add_command", |mut c: Caller<'_, Ctx>, ptr: i32, len: i32| {
        let t = text(&c, ptr, len).trim().to_lowercase();
        if t.starts_with('/') && t.len() > 1 && !c.data().commands.contains(&t) {
            c.data_mut().commands.push(t);
        }
    })?;
    l.func_wrap("jogo", "run", |mut c: Caller<'_, Ctx>, ptr: i32, len: i32| {
        let line = text(&c, ptr, len);
        if let Some(g) = game(&mut c) {
            // On a guest, with that guest's own permission.
            let cheats = g.net_guest_op().unwrap_or(true);
            let r = g.run_command(&line, cheats);
            if !r.is_empty() {
                g.plugin_say(r);
            }
        }
    })?;
    l.func_wrap("jogo", "random", |_: Caller<'_, Ctx>| -> f32 { rand_f32() })?;
    Ok(l)
}

fn rand_f32() -> f32 {
    use std::cell::Cell;
    thread_local!(static S: Cell<u64> = Cell::new(0x9E37_79B9_7F4A_7C15 ^ std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)));
    S.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x >> 40) as f32 / (1u64 << 24) as f32
    })
}

fn load_one(engine: &Engine, linker: &Linker<Ctx>, file: &str, bytes: Vec<u8>, index: usize) -> Result<Plugin, String> {
    let module = Module::new(engine, &bytes[..]).map_err(|e| e.to_string())?;
    let mut store = Store::new(engine, Ctx { index, ..Default::default() });
    store.set_fuel(FUEL).map_err(|e| e.to_string())?;
    let instance = linker.instantiate_and_start(&mut store, &module).map_err(|e| e.to_string())?;
    let memory = instance.get_memory(&store, "memory");
    let tick = instance.get_typed_func::<f32, ()>(&store, "tick").ok();
    let command = instance.get_typed_func::<(i32, i32), i32>(&store, "command").ok();
    let broken = instance.get_typed_func::<(i32, i32, i32, i32), ()>(&store, "block_broken").ok();
    let placed = instance.get_typed_func::<(i32, i32, i32, i32), ()>(&store, "block_placed").ok();
    let button = instance.get_typed_func::<i32, ()>(&store, "button").ok();
    let item_used = instance.get_typed_func::<(i32, i32), i32>(&store, "item_used").ok();
    if let Ok(f) = instance.get_typed_func::<(), ()>(&store, "items") {
        f.call(&mut store, ()).map_err(|e| format!("items: {e}"))?;
        store.set_fuel(FUEL).map_err(|e| e.to_string())?;
    }
    let name = Path::new(file).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let (file, bytes) = (file.to_string(), std::sync::Arc::new(bytes));
    Ok(Plugin { name, file, bytes, store, instance, memory, buffer: None, tick, command, broken, placed, button, item_used, failed: false })
}

/// Load every plugin in `<base>/plugins` (not run yet: `init` runs on the first frame in a world).
pub(crate) fn load(base: &Path) -> Plugins {
    let Ok(dir) = std::fs::read_dir(base.join("plugins")) else { return Plugins::default() };
    let mut paths: Vec<_> = dir.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "wasm" || x == "wat")).collect();
    paths.sort();
    let files: Vec<(String, Vec<u8>)> = paths
        .iter()
        .filter_map(|p| Some((p.file_name()?.to_string_lossy().to_string(), std::fs::read(p).map_err(|e| log::error!("plugin {}: {e}", p.display())).ok()?)))
        .collect();
    from_files(&files)
}

/// Plugins from their files' names and bytes (a folder's, or what the host sent).
pub(crate) fn from_files(files: &[(String, Vec<u8>)]) -> Plugins {
    let mut ps = Plugins::default();
    if files.is_empty() {
        return ps;
    }
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let linker = match linker(&engine) {
        Ok(l) => l,
        Err(e) => {
            log::error!("plugins: {e}");
            return ps;
        }
    };
    for (n, bytes) in files {
        match load_one(&engine, &linker, n, bytes.clone(), ps.list.len()) {
            Ok(pl) => {
                log::info!("plugin {} loaded", pl.name);
                ps.list.push(pl);
            }
            Err(e) => {
                log::error!("plugin {n}: {e}");
                ps.notes.push(format!("Plugin {n} não carregou: {e}"));
            }
        }
    }
    ps
}

impl Plugins {
    /// Put the plugins' items at the end of the item list (their pictures go to the texture
    /// loader). Items whose name is taken are skipped. Returns how many were added.
    pub(crate) fn register_items(&mut self, items: &mut voxel::item::ItemRegistry) -> usize {
        let mut added = 0;
        for p in &mut self.list {
            for it in std::mem::take(&mut p.store.data_mut().items) {
                let tex = format!("plugin_{}", it.name);
                if let Some(px) = it.pixels.and_then(|px| image::RgbaImage::from_raw(16, 16, px)) {
                    let mut t = crate::textures::PLUGIN_TEXTURES.lock().unwrap();
                    t.retain(|(n, _)| *n != tex);
                    t.push((tex.clone(), px));
                }
                let food = (it.food > 0).then_some(voxel::item::FoodDef { hunger: it.food, saturation: it.food as f32 * 0.6 });
                if items.add_plain(&it.name, &it.label, &tex, u16::from(it.max_stack), food).is_some() {
                    self.item_names.insert(it.name);
                    added += 1;
                }
            }
        }
        added
    }

    /// The plugin files to send to a guest (as many as fit in the limit).
    pub(crate) fn files(&self) -> Vec<(String, Vec<u8>)> {
        let mut total = 0;
        let mut out = Vec::new();
        for p in &self.list {
            total += p.bytes.len();
            if total > MAX_SHARE {
                log::warn!("plugin {} too big to send to guests", p.name);
                break;
            }
            out.push((p.file.clone(), p.bytes.to_vec()));
        }
        out
    }

    pub(crate) fn names(&self) -> Vec<String> {
        self.list.iter().map(|p| if p.failed { format!("{} (parado)", p.name) } else { p.name.clone() }).collect()
    }

    fn handles(&self, cmd: &str) -> bool {
        self.list.iter().any(|p| !p.failed && p.store.data().commands.iter().any(|c| c == cmd))
    }
}

impl Game {
    pub(crate) fn plugin_say(&mut self, t: String) {
        let t: String = t.chars().take(256).collect();
        self.net_announce(t);
    }

    /// Run `f` on each working plugin with the game reachable from its host functions.
    fn with_plugins(&mut self, mut f: impl FnMut(&mut Plugin) -> Result<(), wasmi::Error>) {
        let mut ps = std::mem::take(&mut self.plugins);
        let me = std::ptr::NonNull::from(&mut *self);
        let mut stopped = vec![];
        let mut screen_changed = false;
        for (i, p) in ps.list.iter_mut().enumerate().filter(|(_, p)| !p.failed) {
            p.store.data_mut().game = Some(me);
            let _ = p.store.set_fuel(FUEL);
            let r = f(p);
            let d = p.store.data_mut();
            d.game = None;
            if std::mem::take(&mut d.screen_changed) {
                // A plugin's window replaces whatever is open; closing only closes its own.
                match &d.screen {
                    Some(sc) => ps.screen = Some(sc.clone()),
                    None if ps.screen.as_ref().is_some_and(|sc| sc.plugin == i) => ps.screen = None,
                    None => {}
                }
                screen_changed = true;
            }
            if let Err(e) = r {
                p.failed = true;
                log::error!("plugin {} stopped: {e}", p.name);
                stopped.push(format!("Plugin {} parou: {e}", p.name));
            }
        }
        // Plugins loaded by a host function meanwhile (none today) would be lost; keep ours.
        self.plugins = ps;
        if screen_changed {
            if self.plugins.screen.is_some() {
                if matches!(self.screen, Screen::Playing | Screen::Plugin) {
                    self.set_screen(Screen::Plugin);
                }
            } else if self.screen == Screen::Plugin {
                self.set_screen(Screen::Playing);
            }
        }
        for s in stopped {
            self.plugin_say(s);
        }
    }

    /// Every frame while playing: the first time `init`, then `tick` (only where the world
    /// is simulated: alone or hosting).
    pub(crate) fn plugins_tick(&mut self, dt: f32) {
        if self.plugins.list.is_empty() {
            return;
        }
        if !self.plugins.loaded {
            self.plugins.loaded = true;
            for n in std::mem::take(&mut self.plugins.notes) {
                self.plugin_say(n);
            }
            self.with_plugins(|p| {
                if let Some(Extern::Func(f)) = p.instance.get_export(&p.store, "buffer") {
                    if let Ok(b) = f.typed::<(), i32>(&p.store).and_then(|f| f.call(&mut p.store, ())) {
                        p.buffer = Some(b.max(0) as u32);
                    }
                }
                match p.instance.get_typed_func::<(), ()>(&p.store, "init") {
                    Ok(init) => init.call(&mut p.store, ()),
                    Err(_) => Ok(()),
                }
            });
        }
        if !self.net_simulates() {
            return;
        }
        self.with_plugins(|p| match &p.tick {
            Some(t) => t.call(&mut p.store, dt),
            None => Ok(()),
        });
    }

    /// The host's plugins, as a guest: they replace ours while we visit (their items join the
    /// list, so their pictures load too).
    pub(crate) fn plugins_from_host(&mut self, files: &[(String, Vec<u8>)]) {
        let base_items = self.plugins.base_items;
        let mut ps = from_files(files);
        ps.base_items = base_items;
        ps.from_host = true;
        let added = ps.register_items(&mut self.items);
        log::info!("{} plugins from the host, {added} new items", ps.list.len());
        self.plugins = ps;
        if added > 0 {
            self.reload_textures();
        }
    }

    /// Back to our own plugins after visiting.
    pub(crate) fn plugins_leave_host(&mut self) {
        if !self.plugins.from_host {
            return;
        }
        let base_items = self.plugins.base_items;
        self.plugins = load(&self.base);
        self.plugins.base_items = base_items;
        if self.plugins.register_items(&mut self.items) > 0 {
            self.reload_textures();
        }
    }

    /// Right click holding a plugin's item. True = a plugin used it.
    pub(crate) fn plugins_use_item(&mut self) -> bool {
        let Some(name) = self.held_stack().map(|s| self.items.get(s.item).name.clone()) else { return false };
        if !self.plugins.item_names.contains(&name) {
            return false;
        }
        let mut handled = false;
        self.with_plugins(|p| {
            let (Some(f), Some(m), Some(b)) = (&p.item_used, p.memory, p.buffer) else { return Ok(()) };
            if handled || m.write(&mut p.store, b as usize, name.as_bytes()).is_err() {
                return Ok(());
            }
            handled = f.call(&mut p.store, (b as i32, name.len() as i32))? != 0;
            Ok(())
        });
        handled
    }

    fn plugin_button(&mut self, id: i32) {
        let Some(owner) = self.plugins.screen.as_ref().map(|s| s.plugin) else { return };
        let mut i = 0;
        self.with_plugins(|p| {
            let mine = i == owner;
            i += 1;
            match (&p.button, mine) {
                (Some(f), true) => f.call(&mut p.store, id),
                _ => Ok(()),
            }
        });
    }

    /// A plugin's window: its title, lines of text and buttons.
    pub(crate) fn draw_plugin_screen(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let Some(sc) = self.plugins.screen.clone() else { return };
        let s = self.ui.scale;
        let widest = sc.rows.iter().map(|r| match r {
            Row::Text(t) | Row::Button(_, t) => self.ui.text_width(t, s),
        });
        let bw = widest.fold(self.ui.text_width(&sc.title, s), f32::max).max(160.0 * s) + 16.0 * s;
        let row = 14.0 * s;
        let bh = row * (sc.rows.len() as f32 + 2.5);
        let (x, y) = ((w - bw) / 2.0, ((h - bh) / 2.0).max(4.0 * s));
        self.ui.rect(0.0, 0.0, w, h, [0, 0, 0, 120]);
        self.ui.rect(x - 2.0 * s, y - 2.0 * s, bw + 4.0 * s, bh + 4.0 * s, [40, 40, 46, 255]);
        self.ui.rect(x, y, bw, bh, [198, 198, 198, 255]);
        let title = crate::lang::tr(&sc.title);
        self.ui.text_centered(w / 2.0, y + 5.0 * s, &title, [40, 40, 40, 255]);
        let mut clicked = None;
        for (k, r) in sc.rows.iter().enumerate() {
            let ry = y + row * (k as f32 + 1.4);
            match r {
                Row::Text(t) => self.ui.text(x + 6.0 * s, ry + 3.0 * s, t, [50, 50, 50, 255]),
                Row::Button(id, label) => {
                    if self.ui.button(x + 6.0 * s, ry, bw - 12.0 * s, row - 2.0 * s, label, mouse) && self.clicked {
                        clicked = Some(*id);
                    }
                }
            }
        }
        self.ui.text_centered(w / 2.0, y + bh + 6.0 * s, "Esc: fecha", [255, 255, 255, 255]);
        if let Some(id) = clicked {
            self.clicked = false;
            self.plugin_button(id);
        }
    }

    /// A chat command a plugin added: give it the whole line. None = not a plugin command.
    pub(crate) fn plugins_command(&mut self, line: &str) -> Option<String> {
        let cmd = line.split_whitespace().next().unwrap_or("").to_lowercase();
        if !self.plugins.handles(&cmd) {
            return None;
        }
        let mut handled = false;
        let line = line.to_string();
        self.with_plugins(|p| {
            if handled || !p.store.data().commands.contains(&cmd) {
                return Ok(());
            }
            let (Some(f), Some(m), Some(b)) = (&p.command, p.memory, p.buffer) else { return Ok(()) };
            let bytes = &line.as_bytes()[..line.len().min(255)];
            if m.write(&mut p.store, b as usize, bytes).is_err() {
                return Ok(());
            }
            handled = f.call(&mut p.store, (b as i32, bytes.len() as i32))? != 0;
            Ok(())
        });
        Some(String::new())
    }

    pub(crate) fn plugins_block(&mut self, p: IVec3, id: BlockId, placed: bool) {
        if self.plugins.list.is_empty() {
            return;
        }
        self.with_plugins(|pl| {
            let f = if placed { &pl.placed } else { &pl.broken };
            match f {
                Some(f) => f.call(&mut pl.store, (p.x, p.y, p.z, id as i32)),
                None => Ok(()),
            }
        });
    }

    pub(crate) fn plugins_list_line(&self) -> String {
        let n = self.plugins.names();
        if n.is_empty() {
            "Nenhum plugin (ponha arquivos .wasm em plugins/)".into()
        } else {
            format!("Plugins: {}", n.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_plugin_loads_and_runs() {
        let dir = std::env::temp_dir().join(format!("mc-plugins-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("plugins")).unwrap();
        std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugins-exemplos/exemplo.wat"), dir.join("plugins/exemplo.wat")).unwrap();
        let mut ps = load(&dir);
        assert!(ps.notes.is_empty(), "{:?}", ps.notes);
        assert_eq!(ps.list.len(), 1);
        // Without a game the host functions do nothing, but the calls run within their fuel.
        let p = &mut ps.list[0];
        p.instance.get_typed_func::<(), ()>(&p.store, "init").unwrap().call(&mut p.store, ()).unwrap();
        assert!(p.store.data().commands.contains(&"/sorte".to_string()));
        p.tick.as_ref().unwrap().call(&mut p.store, 0.05).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn panel_plugin_adds_an_item_and_a_screen() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugins-exemplos/painel.wat")).unwrap();
        let mut ps = from_files(&[("painel.wat".into(), bytes)]);
        assert!(ps.notes.is_empty(), "{:?}", ps.notes);
        let (reg, mut items, _) = crate::game_data(crate::embedded::BLOCKS_RON, crate::embedded::ITEMS_RON, crate::embedded::RECIPES_RON).unwrap();
        let before = items.items.len();
        assert_eq!(ps.register_items(&mut items), 1);
        let id = items.id("pedra_da_sorte").expect("item");
        assert_eq!(id as usize, before);
        assert_eq!(items.get(id).display_name, "Pedra da sorte");
        assert!(crate::textures::PLUGIN_TEXTURES.lock().unwrap().iter().any(|(n, i)| n == "plugin_pedra_da_sorte" && i.get_pixel(8, 8)[3] == 255 && i.get_pixel(0, 0)[3] == 0));
        // The same plugin again (a guest getting it) adds nothing twice.
        let mut again = from_files(&ps.files());
        assert_eq!(again.register_items(&mut items), 0);
        let _ = reg;
        // Its command builds a window with four buttons.
        let p = &mut ps.list[0];
        p.instance.get_typed_func::<(), ()>(&p.store, "init").unwrap().call(&mut p.store, ()).unwrap();
        p.command.as_ref().unwrap().call(&mut p.store, (8192, 7)).unwrap();
        let sc = p.store.data().screen.clone().unwrap();
        assert_eq!(sc.title, "Painel do plugin");
        assert_eq!(sc.rows.iter().filter(|r| matches!(r, Row::Button(..))).count(), 4);
        p.button.as_ref().unwrap().call(&mut p.store, 4).unwrap();
        assert!(p.store.data().screen.is_none());
    }

    #[test]
    fn runaway_plugin_is_stopped_by_fuel() {
        let config = {
            let mut c = Config::default();
            c.consume_fuel(true);
            c
        };
        let engine = Engine::new(&config);
        let l = linker(&engine).unwrap();
        let module = Module::new(&engine, r#"(module (func (export "tick") (param f32) (loop $l (br $l))))"#.as_bytes()).unwrap();
        let mut store = Store::new(&engine, Ctx::default());
        store.set_fuel(FUEL).unwrap();
        let inst = l.instantiate_and_start(&mut store, &module).unwrap();
        let tick = inst.get_typed_func::<f32, ()>(&store, "tick").unwrap();
        assert!(tick.call(&mut store, 0.0).is_err());
    }
}
