//! Title screen and the menus around it: world list, new world, multiplayer, options and
//! texture packs. The last played world loads behind the title as a slowly turning panorama.

use std::sync::Arc;

use glam::Vec3;
use voxel::mesh::MeshTables;
use voxel::save::WorldSave;
use voxel::worldgen::{Biome, Generator};

use crate::audio::Sfx;
use crate::ui::{GRAY, WHITE, YELLOW};
use crate::world::World;
use crate::{level_path_hint, paintings, textures, Game, Screen, RENDER_DISTANCES};

pub struct WorldEntry {
    pub name: String,
    pub survival: bool,
    pub seed: u64,
    /// Seconds since the epoch the world was last saved.
    pub modified: u64,
}

#[derive(Default)]
pub struct NewWorldForm {
    pub name: String,
    pub seed: String,
    pub survival: bool,
    /// 0 = name field, 1 = seed field.
    pub field: usize,
}

/// Saved worlds, most recently played first.
pub fn list_worlds(base: &std::path::Path) -> Vec<WorldEntry> {
    let mut v = Vec::new();
    let Ok(rd) = std::fs::read_dir(base.join("saves")) else { return v };
    for e in rd.flatten() {
        let dir = e.path();
        let save = WorldSave::new(dir.clone());
        let Some(level) = save.load_level() else { continue };
        let modified = std::fs::metadata(dir.join("level.ron"))
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        v.push(WorldEntry { name, survival: level.survival, seed: level.seed, modified });
    }
    v.sort_by_key(|w| std::cmp::Reverse(w.modified));
    v
}

/// Text seeds become numbers the same way every time.
fn parse_seed(text: &str) -> Option<u64> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(n) = t.parse::<i64>() {
        return Some(n as u64);
    }
    // FNV-1a over the text.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in t.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    Some(h)
}

fn ago(secs: u64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let d = now.saturating_sub(secs);
    match d {
        0..=119 => "agora há pouco".into(),
        120..=7199 => format!("há {} min", d / 60),
        7200..=172_799 => format!("há {} h", d / 3600),
        _ => format!("há {} dias", d / 86400),
    }
}

impl Game {
    /// Close the current world (saving it) and open another one, by folder name.
    pub(crate) fn open_world(&mut self, name: &str, seed: Option<u64>, survival: Option<bool>) {
        if self.world_open {
            self.save_all();
        }
        self.world_open = false;
        let save = Arc::new(WorldSave::new(level_path_hint(&self.base, name)));
        let level = save.load_level();
        let seed = level.as_ref().map(|l| l.seed).or(seed).unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
        });
        let gen = Arc::new(Generator::new(seed, &self.reg).with_structures(self.structures.clone()));
        let tints: Vec<[f32; 3]> = (0..Biome::COUNT).map(|b| Biome::from_u8(b as u8).tint()).collect();
        let tables = Arc::new(MeshTables::new(&self.reg, tints));
        // Dropping the old world stops its worker threads.
        self.world = World::new(self.reg.clone(), gen.clone(), save.clone(), tables);
        self.fluids = crate::fluids::Fluids::new(&self.reg);
        self.renderer.clear_chunks();
        self.world_name = name.to_string();
        self.seed = seed;
        let (sx, sy, sz) = gen.find_spawn();
        self.spawn_xz = (sx, sz);
        self.body = voxel::physics::Body::new(Vec3::new(sx as f32 + 0.5, sy as f32, sz as f32 + 0.5));
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.time_of_day = 1000.0;
        self.weather = crate::weather::Weather::new(crate::weather::WeatherSave::default(), seed);
        self.needs_spawn = true;
        self.survival = survival.unwrap_or(false);
        self.health = 20.0;
        self.air = 10.0;
        self.hurt_age = 100.0;
        self.mining = None;
        self.mobs.list.clear();
        self.target = None;
        if let Some(l) = &level {
            self.body.pos = Vec3::from(l.player_pos);
            self.body.flying = l.flying;
            self.yaw = l.player_yaw;
            self.pitch = l.player_pitch;
            self.time_of_day = l.time_of_day;
            self.weather = crate::weather::Weather::new(crate::weather::WeatherSave { raining: l.raining, thunder: l.thunder, timer: l.weather_timer }, l.seed);
            if l.hotbar.len() == 9 {
                for (i, n) in l.hotbar.iter().enumerate() {
                    self.hotbar[i] = self.reg.id_or_air(n);
                }
            }
            self.survival = l.survival;
            self.health = if l.health > 0.0 { l.health } else { 20.0 };
            self.needs_spawn = false;
        }
        if self.survival {
            self.body.flying = false;
        }
        self.fall_peak = self.body.pos.y;
        self.paintings = paintings::load(&save.dir, &self.painting_defs);
        self.surv = crate::survival::Survival::load(&save.dir, &self.items);
        self.title_world = name.to_string();
    }

    /// Enter the world that is loaded behind the menus (or open a different one first).
    pub(crate) fn play_world(&mut self, name: &str) {
        if name != self.title_world {
            self.open_world(name, None, None);
        }
        self.world_open = true;
        self.loaded_frames = 0;
        self.set_screen(Screen::Loading);
    }

    /// Camera for the title panorama: hovering over the spawn, slowly turning.
    pub(crate) fn update_title(&mut self, dt: f32) {
        self.title_spin += dt * 0.035;
        if self.needs_spawn {
            let (sx, sz) = self.spawn_xz;
            if let Some(y) = self.world.surface_y(sx, sz) {
                self.body.pos = Vec3::new(sx as f32 + 0.5, y as f32 + 4.0, sz as f32 + 0.5);
                self.needs_spawn = false;
            }
        }
        self.yaw = self.title_spin;
        self.pitch = -0.12;
    }

    pub(crate) fn is_title_screen(&self) -> bool {
        matches!(self.screen, Screen::Title | Screen::Worlds | Screen::CreateWorld | Screen::Multiplayer)
            || (!self.world_open && matches!(self.screen, Screen::Options | Screen::Packs))
    }

    fn click_sound(&mut self) {
        self.sfx(Sfx::Click);
    }

    /// Big blocky title with a shadow.
    fn draw_logo(&mut self, w: f32, y: f32) {
        let s = self.ui.scale;
        let title = "MINE CLONE";
        let big = (s * 4.0).max(4.0);
        let tw = self.ui.text_width(title, big);
        let x = w / 2.0 - tw / 2.0;
        self.ui.text_scaled(x + big * 0.6, y + big * 0.6, title, [30, 30, 30, 255], big);
        self.ui.text_scaled(x, y, title, [200, 200, 200, 255], big);
        // Splash line, gently pulsing.
        let pulse = 1.0 + (self.now * 5.0).sin().abs() * 0.08;
        let msg = "Feito em Rust!";
        let ms = s * pulse;
        let mw = self.ui.text_width(msg, ms);
        self.ui.text_scaled(x + tw - mw * 0.7, y + big * 8.5, msg, YELLOW, ms);
    }

    pub(crate) fn draw_title_menus(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        // Until the panorama is ready, plain dirt-brown; then a light veil over the world.
        let ready = self.world.meshed_count > 20;
        self.ui.rect(0.0, 0.0, w, h, if ready { [0, 0, 0, 70] } else { [44, 34, 26, 255] });
        match self.screen {
            Screen::Title => self.draw_title(w, h, mouse),
            Screen::Worlds => self.draw_worlds(w, h, mouse),
            Screen::CreateWorld => self.draw_create(w, h, mouse),
            Screen::Multiplayer => {
                self.ui.text_centered(w / 2.0, h / 2.0 - 50.0 * s, "Multijogador", WHITE);
                let lines = [
                    "Jogar com amigos chega na fase 7 do plano:",
                    "servidor dedicado, LAN e convite por código.",
                    "Por enquanto, jogue em Um jogador.",
                ];
                for (i, l) in lines.iter().enumerate() {
                    self.ui.text_centered(w / 2.0, h / 2.0 - 30.0 * s + i as f32 * 11.0 * s, l, GRAY);
                }
                let bw = 200.0 * s;
                if self.ui.button((w - bw) / 2.0, h / 2.0 + 20.0 * s, bw, 20.0 * s, "Voltar", mouse) && self.clicked {
                    self.click_sound();
                    self.set_screen(Screen::Title);
                }
            }
            _ => {}
        }
    }

    fn draw_title(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        let logo_y = (h * 0.16).max(10.0 * s);
        self.draw_logo(w, logo_y);
        let bw = 200.0 * s;
        let bh = 20.0 * s;
        let x = (w - bw) / 2.0;
        let mut y = (h * 0.42).max(logo_y + 60.0 * s);
        let items = ["Um jogador", "Multijogador", "Pacotes de textura"];
        let mut hit = None;
        for (i, l) in items.iter().enumerate() {
            if self.ui.button(x, y, bw, bh, l, mouse) && self.clicked {
                hit = Some(i);
            }
            y += bh + 4.0 * s;
        }
        y += 8.0 * s;
        let half = (bw - 4.0 * s) / 2.0;
        if self.ui.button(x, y, half, bh, "Opções...", mouse) && self.clicked {
            hit = Some(10);
        }
        if self.ui.button(x + half + 4.0 * s, y, half, bh, "Sair do jogo", mouse) && self.clicked {
            hit = Some(11);
        }
        let hs = (s / 2.0).floor().max(1.0);
        self.ui.text_scaled(2.0 * s, h - 10.0 * hs, &format!("Mine Clone {}", env!("CARGO_PKG_VERSION")), WHITE, hs);
        let note = "Projeto independente. Não é um produto da Mojang.";
        let nw = self.ui.text_width(note, hs);
        self.ui.text_scaled(w - nw - 2.0 * s, h - 10.0 * hs, note, WHITE, hs);
        if let Some(i) = hit {
            self.click_sound();
            match i {
                0 => {
                    self.worlds = list_worlds(&self.base);
                    self.world_sel = if self.worlds.is_empty() { None } else { Some(0) };
                    self.set_screen(Screen::Worlds);
                }
                1 => self.set_screen(Screen::Multiplayer),
                2 => {
                    self.packs = textures::list_resource_packs(&self.base.join("resourcepacks"));
                    self.set_screen(Screen::Packs);
                }
                10 => self.set_screen(Screen::Options),
                _ => {
                    self.settings.save(&self.base.join("settings.ron"));
                    std::process::exit(0);
                }
            }
        }
    }

    fn draw_worlds(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        self.ui.text_centered(w / 2.0, 10.0 * s, "Escolha o mundo", WHITE);
        let lw = (260.0 * s).min(w - 20.0 * s);
        let x = (w - lw) / 2.0;
        let row = 26.0 * s;
        let top = 24.0 * s;
        let bottom = h - 56.0 * s;
        let rows = (((bottom - top) / row).floor() as usize).max(1);
        self.ui.rect(0.0, top - 2.0 * s, w, bottom - top + 4.0 * s, [0, 0, 0, 120]);
        if self.worlds.is_empty() {
            self.ui.text_centered(w / 2.0, top + 20.0 * s, "Nenhum mundo ainda. Crie um novo!", GRAY);
        }
        let mut clicked_row = None;
        for (i, wd) in self.worlds.iter().enumerate().take(rows) {
            let y = top + i as f32 * row;
            let hover = mouse.0 >= x && mouse.0 < x + lw && mouse.1 >= y && mouse.1 < y + row - 2.0 * s;
            if self.world_sel == Some(i) {
                self.ui.frame(x, y, lw, row - 2.0 * s, s, [255, 255, 255, 255]);
            } else if hover {
                self.ui.frame(x, y, lw, row - 2.0 * s, s, [128, 128, 128, 255]);
            }
            self.ui.text(x + 4.0 * s, y + 3.0 * s, &wd.name, WHITE);
            let info = format!("{} · semente {} · {}", if wd.survival { "Sobrevivência" } else { "Criativo" }, wd.seed as i64, ago(wd.modified));
            let hs = (s / 2.0).floor().max(1.0);
            self.ui.text_scaled(x + 4.0 * s, y + 13.0 * s, &info, GRAY, hs);
            if hover && self.clicked {
                clicked_row = Some(i);
            }
        }
        if let Some(i) = clicked_row {
            // A second click on the selected world opens it.
            if self.world_sel == Some(i) {
                let name = self.worlds[i].name.clone();
                self.click_sound();
                self.play_world(&name);
                return;
            }
            self.world_sel = Some(i);
        }
        let bw = 150.0 * s;
        let bh = 20.0 * s;
        let y1 = h - 50.0 * s;
        let y2 = y1 + bh + 4.0 * s;
        let x1 = w / 2.0 - bw - 2.0 * s;
        let x2 = w / 2.0 + 2.0 * s;
        let can_play = self.world_sel.is_some();
        let mut hit = None;
        if self.ui.button(x1, y1, bw, bh, if can_play { "Jogar mundo selecionado" } else { "(escolha um mundo)" }, mouse) && self.clicked && can_play {
            hit = Some(0);
        }
        if self.ui.button(x2, y1, bw, bh, "Criar novo mundo", mouse) && self.clicked {
            hit = Some(1);
        }
        if self.ui.button(x1, y2, bw * 2.0 + 4.0 * s, bh, "Voltar", mouse) && self.clicked {
            hit = Some(2);
        }
        if let Some(i) = hit {
            self.click_sound();
            match i {
                0 => {
                    let name = self.worlds[self.world_sel.unwrap()].name.clone();
                    self.play_world(&name);
                }
                1 => {
                    let mut n = 1;
                    while self.worlds.iter().any(|w| w.name == format!("Mundo {n}")) {
                        n += 1;
                    }
                    self.new_world = NewWorldForm { name: format!("Mundo {n}"), seed: String::new(), survival: true, field: 0 };
                    self.set_screen(Screen::CreateWorld);
                }
                _ => self.set_screen(Screen::Title),
            }
        }
    }

    fn draw_create(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        let fw = 200.0 * s;
        let fh = 20.0 * s;
        let x = (w - fw) / 2.0;
        let mut y = (h / 2.0 - 80.0 * s).max(20.0 * s);
        self.ui.text_centered(w / 2.0, y - 14.0 * s, "Criar novo mundo", WHITE);
        let caret = if (self.now * 2.0) as i32 % 2 == 0 { "_" } else { "" };
        let mut hit = None;
        for f in 0..2 {
            let (label, value) = if f == 0 { ("Nome do mundo", self.new_world.name.clone()) } else { ("Semente (vazio = aleatória)", self.new_world.seed.clone()) };
            self.ui.text(x, y, label, GRAY);
            y += 10.0 * s;
            let hover = mouse.0 >= x && mouse.0 < x + fw && mouse.1 >= y && mouse.1 < y + fh;
            self.ui.rect(x, y, fw, fh, [0, 0, 0, 255]);
            let border = if self.new_world.field == f { [255, 255, 255, 255] } else { [160, 160, 160, 255] };
            self.ui.frame(x, y, fw, fh, s, border);
            let shown = if self.new_world.field == f { format!("{value}{caret}") } else { value };
            self.ui.text(x + 4.0 * s, y + 6.0 * s, &shown, WHITE);
            if hover && self.clicked {
                self.new_world.field = f;
            }
            y += fh + 8.0 * s;
        }
        let mode = format!("Modo de jogo: {}", if self.new_world.survival { "Sobrevivência" } else { "Criativo" });
        if self.ui.button(x, y, fw, fh, &mode, mouse) && self.clicked {
            hit = Some(0);
        }
        y += fh + 2.0 * s;
        let hint = if self.new_world.survival { "Vida, fome de aventura e monstros à noite." } else { "Voe, todos os blocos, sem dano." };
        let hs = (s / 2.0).floor().max(1.0);
        let tw = self.ui.text_width(hint, hs);
        self.ui.text_scaled(w / 2.0 - tw / 2.0, y, hint, GRAY, hs);
        y += 14.0 * s;
        let half = (fw - 4.0 * s) / 2.0;
        if self.ui.button(x, y, half, fh, "Criar mundo", mouse) && self.clicked {
            hit = Some(1);
        }
        if self.ui.button(x + half + 4.0 * s, y, half, fh, "Cancelar", mouse) && self.clicked {
            hit = Some(2);
        }
        if let Some(i) = hit {
            self.click_sound();
            match i {
                0 => self.new_world.survival = !self.new_world.survival,
                1 => self.create_world(),
                _ => self.set_screen(Screen::Worlds),
            }
        }
    }

    pub(crate) fn create_world(&mut self) {
        let mut name: String = self.new_world.name.trim().chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')).collect();
        if name.is_empty() {
            name = "Novo mundo".into();
        }
        // Never overwrite an existing world: add a number.
        let mut final_name = name.clone();
        let mut n = 2;
        while level_path_hint(&self.base, &final_name).join("level.ron").exists() {
            final_name = format!("{name} ({n})");
            n += 1;
        }
        let seed = parse_seed(&self.new_world.seed);
        let survival = self.new_world.survival;
        self.open_world(&final_name, seed, Some(survival));
        self.world_open = true;
        self.loaded_frames = 0;
        self.set_screen(Screen::Loading);
    }

    /// Typing into the new-world form.
    pub(crate) fn type_text(&mut self, text: &str) {
        self.creative_type(text);
        if self.screen != Screen::CreateWorld {
            return;
        }
        let f = if self.new_world.field == 0 { &mut self.new_world.name } else { &mut self.new_world.seed };
        for c in text.chars() {
            if !c.is_control() && f.chars().count() < 32 {
                f.push(c);
            }
        }
    }

    pub(crate) fn form_key(&mut self, code: winit::keyboard::KeyCode) {
        use winit::keyboard::KeyCode;
        match code {
            KeyCode::Backspace => {
                let f = if self.new_world.field == 0 { &mut self.new_world.name } else { &mut self.new_world.seed };
                f.pop();
            }
            KeyCode::Tab => self.new_world.field = 1 - self.new_world.field,
            KeyCode::Enter | KeyCode::NumpadEnter => self.create_world(),
            _ => {}
        }
    }

    /// Options shared by the title screen and the pause menu.
    pub(crate) fn options_labels(&self) -> Vec<String> {
        let st = &self.settings;
        let onoff = |b: bool| if b { "Sim" } else { "Não" };
        let custom = self.audio.as_ref().map(|a| a.custom_track_count()).unwrap_or(0);
        vec![
            "Concluído".to_string(),
            format!("Campo de visão: {:.0}", st.fov),
            format!("Distância de visão: {} chunks", st.render_distance),
            format!("Sensibilidade do mouse: {:.0}%", st.mouse_sensitivity / 0.12 * 100.0),
            format!("Inverter mouse: {}", onoff(st.invert_mouse)),
            format!("Música: {} ({} suas)", st.music_mode.label(), custom),
            format!("Volume da música: {:.0}%", st.music_volume * 100.0),
            format!("Volume dos sons: {:.0}%", st.sound_volume * 100.0),
            format!("Nuvens: {}", onoff(st.show_clouds)),
            format!("VSync: {}", onoff(st.vsync)),
            format!("Mobs: {}", onoff(st.mobs)),
            format!("Tamanho da interface: {}", if st.gui_scale == 0 { "Automático".to_string() } else { st.gui_scale.to_string() }),
            format!("Balanço da visão: {}", onoff(st.view_bobbing)),
            format!("Itens no chão: {}", if st.flat_drops { "Deitados" } else { "Flutuando" }),
            "Controle...".to_string(),
            "Pacotes de textura...".to_string(),
        ]
    }

    pub(crate) fn draw_options(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        let labels = self.options_labels();
        let bw = 150.0 * s;
        let bh = 20.0 * s;
        let gap = 4.0 * s;
        let rows = (labels.len() - 1).div_ceil(2);
        let total_h = (rows + 1) as f32 * (bh + gap);
        let y0 = ((h - total_h) / 2.0).max(22.0 * s);
        self.ui.text_centered(w / 2.0, y0 - 14.0 * s, "Opções", WHITE);
        let mut hit = None;
        for (i, l) in labels.iter().enumerate().skip(1) {
            let k = i - 1;
            let x = w / 2.0 - bw - gap / 2.0 + (k % 2) as f32 * (bw + gap);
            let y = y0 + (k / 2) as f32 * (bh + gap);
            if self.ui.button(x, y, bw, bh, l, mouse) && self.clicked {
                hit = Some(i);
            }
        }
        let y = y0 + rows as f32 * (bh + gap) + gap;
        if self.ui.button(w / 2.0 - 100.0 * s, y, 200.0 * s, bh, &labels[0], mouse) && self.clicked {
            hit = Some(0);
        }
        if let Some(i) = hit {
            self.options_button(i);
        }
    }

    fn options_button(&mut self, i: usize) {
        self.click_sound();
        let cycle = |v: f32, steps: &[f32]| -> f32 { steps.iter().copied().find(|&x| x > v + 1e-3).unwrap_or(steps[0]) };
        let st = &mut self.settings;
        match i {
            0 => {
                let back = if self.world_open { Screen::Paused } else { Screen::Title };
                self.set_screen(back);
            }
            1 => st.fov = cycle(st.fov, &[50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 110.0]),
            2 => {
                let cur = RENDER_DISTANCES.iter().position(|&d| d >= st.render_distance).unwrap_or(0);
                st.render_distance = RENDER_DISTANCES[(cur + 1) % RENDER_DISTANCES.len()];
            }
            3 => st.mouse_sensitivity = cycle(st.mouse_sensitivity, &[0.06, 0.09, 0.12, 0.15, 0.18, 0.24]),
            4 => st.invert_mouse = !st.invert_mouse,
            5 => {
                st.music_mode = st.music_mode.next();
                if let Some(a) = self.audio.as_mut() {
                    a.reload_tracks();
                }
            }
            6 => st.music_volume = cycle(st.music_volume, &[0.0, 0.2, 0.4, 0.6, 0.8, 1.0]),
            7 => st.sound_volume = cycle(st.sound_volume, &[0.0, 0.2, 0.4, 0.6, 0.8, 1.0]),
            8 => st.show_clouds = !st.show_clouds,
            9 => {
                st.vsync = !st.vsync;
                let v = st.vsync;
                self.renderer.set_vsync(v);
            }
            10 => {
                st.mobs = !st.mobs;
                self.mobs.enabled = st.mobs;
            }
            11 => st.gui_scale = (st.gui_scale + 1) % 5,
            12 => st.view_bobbing = !st.view_bobbing,
            13 => st.flat_drops = !st.flat_drops,
            14 => {
                self.options_back = self.screen;
                self.set_screen(Screen::Controls);
            }
            _ => {
                self.packs = textures::list_resource_packs(&self.base.join("resourcepacks"));
                self.set_screen(Screen::Packs);
            }
        }
        self.settings.save(&self.base.join("settings.ron"));
    }

    pub(crate) fn draw_packs(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        self.ui.text_centered(w / 2.0, 10.0 * s, "Pacotes de textura", WHITE);
        let folder = self.base.join("resourcepacks").display().to_string();
        let hs = (s / 2.0).floor().max(1.0);
        let tw = self.ui.text_width(&folder, hs);
        self.ui.text_scaled(w / 2.0 - tw / 2.0, 21.0 * s, &folder, GRAY, hs);
        let bw = 220.0 * s;
        let bh = 18.0 * s;
        let x = (w - bw) / 2.0;
        let mut y = 32.0 * s;
        let current = self.settings.resource_packs.first().cloned();
        let mut entries: Vec<Option<String>> = vec![None];
        entries.extend(self.packs.iter().cloned().map(Some));
        let max_rows = (((h - y - 40.0 * s) / (bh + 2.0 * s)).floor() as usize).max(1);
        let mut pick = None;
        for e in entries.iter().take(max_rows) {
            let name = e.clone().unwrap_or_else(|| "Padrão (do jogo)".into());
            let label = if *e == current { format!("> {name} <") } else { name };
            if self.ui.button(x, y, bw, bh, &label, mouse) && self.clicked {
                pick = Some(e.clone());
            }
            y += bh + 2.0 * s;
        }
        if self.packs.is_empty() {
            self.ui.text_centered(w / 2.0, y + 4.0 * s, "Coloque pacotes (pasta ou .zip) na pasta acima.", GRAY);
        }
        let back_y = h - 28.0 * s;
        let back = self.ui.button(w / 2.0 - 100.0 * s, back_y, 200.0 * s, 20.0 * s, "Concluído", mouse) && self.clicked;
        if let Some(p) = pick {
            self.click_sound();
            self.settings.resource_packs = p.into_iter().collect();
            self.reload_textures();
            self.settings.save(&self.base.join("settings.ron"));
        }
        if back {
            self.click_sound();
            self.set_screen(if self.world_open { Screen::Paused } else { Screen::Title });
        }
    }

    /// Where Esc (or the pad's back button) goes from a menu screen.
    pub(crate) fn menu_back(&mut self) {
        let to = match self.screen {
            Screen::Worlds | Screen::Multiplayer => Screen::Title,
            Screen::CreateWorld => Screen::Worlds,
            Screen::Options | Screen::Packs => {
                if self.world_open {
                    Screen::Paused
                } else {
                    Screen::Title
                }
            }
            s => s,
        };
        self.set_screen(to);
    }

    /// Pause menu "save and quit": back to the title, keeping the world as the panorama.
    pub(crate) fn quit_to_title(&mut self) {
        self.save_all();
        self.world_open = false;
        self.title_world = self.world_name.clone();
        self.set_screen(Screen::Title);
    }
}
