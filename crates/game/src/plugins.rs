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
//!
//! What the game gives it, module "jogo": `say(ptr, len)`, `log(ptr, len)`,
//! `block_at(x, y, z) -> i32`, `set_block(x, y, z, block) -> i32`, `block_id(ptr, len) -> i32`
//! (-1 unknown), `block_name(block, ptr, cap) -> i32` (bytes written), `player(axis) -> f32`
//! (0 x, 1 y, 2 z), `time() -> f32`, `set_time(t: f32)`, `give(ptr, len, count) -> i32`,
//! `spawn(ptr, len, x: f32, y: f32, z: f32) -> i32`, `add_command(ptr, len)`,
//! `run(ptr, len)` (a chat command, with every permission), `random() -> f32`.
//! See `plugins-exemplos/exemplo.wat` for a small one (copy it into `plugins/` to try it).

use std::path::Path;

use glam::{IVec3, Vec3};
use voxel::block::BlockId;
use wasmi::{Caller, Config, Engine, Extern, Instance, Linker, Memory, Module, Store, TypedFunc};

use crate::Game;

/// Instructions a plugin may run per call before it is stopped.
const FUEL: u64 = 20_000_000;
const MAX_TEXT: usize = 4096;

struct Ctx {
    /// The game, only while a plugin call runs (the plugins are taken out of it meanwhile).
    game: *mut Game,
    commands: Vec<String>,
}

struct Plugin {
    name: String,
    store: Store<Ctx>,
    instance: Instance,
    memory: Option<Memory>,
    buffer: Option<u32>,
    tick: Option<TypedFunc<f32, ()>>,
    command: Option<TypedFunc<(i32, i32), i32>>,
    broken: Option<TypedFunc<(i32, i32, i32, i32), ()>>,
    placed: Option<TypedFunc<(i32, i32, i32, i32), ()>>,
    /// Stopped after a trap or running out of fuel.
    failed: bool,
}

#[derive(Default)]
pub(crate) struct Plugins {
    list: Vec<Plugin>,
    /// What loading said (shown once in the chat).
    pub notes: Vec<String>,
    loaded: bool,
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
    let p = caller.data().game;
    // SAFETY: set from `&mut Game` by `Game::with_plugins` for the length of one call.
    (!p.is_null()).then(|| unsafe { &mut *p })
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
        let bytes = &name.as_bytes()[..name.len().min(cap.max(0) as usize)];
        let Some(Extern::Memory(m)) = c.get_export("memory") else { return 0 };
        if m.write(&mut c, ptr.max(0) as usize, bytes).is_err() {
            return 0;
        }
        bytes.len() as i32
    })?;
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
        let n = count.clamp(1, 64) as u8;
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
            let r = g.run_command(&line, true);
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

fn load_one(engine: &Engine, linker: &Linker<Ctx>, path: &Path) -> Result<Plugin, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let module = Module::new(engine, &bytes[..]).map_err(|e| e.to_string())?;
    let mut store = Store::new(engine, Ctx { game: std::ptr::null_mut(), commands: vec![] });
    store.set_fuel(FUEL).map_err(|e| e.to_string())?;
    let instance = linker.instantiate_and_start(&mut store, &module).map_err(|e| e.to_string())?;
    let memory = instance.get_memory(&store, "memory");
    let tick = instance.get_typed_func::<f32, ()>(&store, "tick").ok();
    let command = instance.get_typed_func::<(i32, i32), i32>(&store, "command").ok();
    let broken = instance.get_typed_func::<(i32, i32, i32, i32), ()>(&store, "block_broken").ok();
    let placed = instance.get_typed_func::<(i32, i32, i32, i32), ()>(&store, "block_placed").ok();
    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    Ok(Plugin { name, store, instance, memory, buffer: None, tick, command, broken, placed, failed: false })
}

/// Load every plugin in `<base>/plugins` (not run yet: `init` runs on the first frame in a world).
pub(crate) fn load(base: &Path) -> Plugins {
    let mut ps = Plugins::default();
    let Ok(dir) = std::fs::read_dir(base.join("plugins")) else { return ps };
    let mut paths: Vec<_> = dir.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "wasm" || x == "wat")).collect();
    paths.sort();
    if paths.is_empty() {
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
    for p in paths {
        match load_one(&engine, &linker, &p) {
            Ok(pl) => {
                log::info!("plugin {} loaded", pl.name);
                ps.list.push(pl);
            }
            Err(e) => {
                let n = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                log::error!("plugin {n}: {e}");
                ps.notes.push(format!("Plugin {n} não carregou: {e}"));
            }
        }
    }
    ps
}

impl Plugins {
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
        let me: *mut Game = self;
        let mut stopped = vec![];
        for p in ps.list.iter_mut().filter(|p| !p.failed) {
            p.store.data_mut().game = me;
            let _ = p.store.set_fuel(FUEL);
            let r = f(p);
            p.store.data_mut().game = std::ptr::null_mut();
            if let Err(e) = r {
                p.failed = true;
                log::error!("plugin {} stopped: {e}", p.name);
                stopped.push(format!("Plugin {} parou: {e}", p.name));
            }
        }
        // Plugins loaded by a host function meanwhile (none today) would be lost; keep ours.
        self.plugins = ps;
        for s in stopped {
            self.plugin_say(s);
        }
    }

    /// Every frame while playing: the first time `init`, then `tick`.
    pub(crate) fn plugins_tick(&mut self, dt: f32) {
        if self.plugins.list.is_empty() || !self.net_simulates() {
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
        self.with_plugins(|p| match &p.tick {
            Some(t) => t.call(&mut p.store, dt),
            None => Ok(()),
        });
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
    fn runaway_plugin_is_stopped_by_fuel() {
        let config = {
            let mut c = Config::default();
            c.consume_fuel(true);
            c
        };
        let engine = Engine::new(&config);
        let l = linker(&engine).unwrap();
        let module = Module::new(&engine, r#"(module (func (export "tick") (param f32) (loop $l (br $l))))"#.as_bytes()).unwrap();
        let mut store = Store::new(&engine, Ctx { game: std::ptr::null_mut(), commands: vec![] });
        store.set_fuel(FUEL).unwrap();
        let inst = l.instantiate_and_start(&mut store, &module).unwrap();
        let tick = inst.get_typed_func::<f32, ()>(&store, "tick").unwrap();
        assert!(tick.call(&mut store, 0.0).is_err());
    }
}
