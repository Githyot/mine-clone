//! The options screen, in the style of the classic game's: tabs for Vídeo, Áudio, Controles and
//! Jogo, sliders for everything with a range (volumes, view distance, field of view, brightness,
//! sensitivity) and buttons for the rest. Shared by the title screen and the pause menu.

use crate::audio::{Sfx, SoundCat};
use crate::settings::{self, ShaderQuality, PARALLAX_LEVELS, SHADOW_LEVELS};
use crate::ui::{UiBatch, GRAY, WHITE, YELLOW};
use crate::{textures, Game, Screen, ShaderRow};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Video,
    Audio,
    Controls,
    Game,
}

const TABS: [(Tab, &str); 4] = [(Tab::Video, "Vídeo"), (Tab::Audio, "Áudio"), (Tab::Controls, "Controles"), (Tab::Game, "Jogo")];

#[derive(Default)]
pub struct OptState {
    pub tab: Tab,
    /// First row shown (the list scrolls with the wheel).
    pub scroll: usize,
    /// The slider following the mouse while the button is held.
    pub drag: Option<Id>,
    /// Left mouse button down (menus don't track it otherwise).
    pub held: bool,
}

/// Every option of the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    MobStrength,
    Preset,
    RenderDistance,
    Fov,
    Brightness,
    GuiScale,
    Vsync,
    Clouds,
    ViewBobbing,
    TextureRes,
    Packs,
    Shaders,
    ShaderQuality,
    Shadows,
    Parallax,
    Reflections,
    Taa,
    Emissive,
    Relief,
    Materials,
    Lod,
    LodDetail,
    Drops,
    AmbientLife,
    RainDrops,
    DropLabels,
    DropGlow,
    Master,
    Music,
    Vol(SoundCat),
    Voices,
    MusicMode,
    NextSong,
    VoiceMode,
    VoiceMenu,
    Subtitles,
    MouseSens,
    InvertMouse,
    PadSens,
    Keys,
    Pad,
    GameMode,
    Difficulty,
    Mobs,
    Structures,
    Language,
    UiAnim,
    ReduceMotion,
}

enum Kind {
    /// A title across the list.
    Header,
    Button,
    /// Position of the handle, 0..1.
    Slider(f32),
}

struct Opt {
    id: Option<Id>,
    label: String,
    kind: Kind,
    hint: &'static str,
}

fn header(t: &str) -> Opt {
    Opt { id: None, label: t.to_string(), kind: Kind::Header, hint: "" }
}

/// A slider's range: lowest and highest value and the step it snaps to.
fn range(id: Id) -> (f32, f32, f32) {
    match id {
        Id::RenderDistance => (2.0, 32.0, 1.0),
        Id::Fov => (30.0, 110.0, 1.0),
        Id::Lod => (0.0, (crate::lod::DISTANCES.len() - 1) as f32, 1.0),
        Id::Voices => (0.0, 2.0, 0.05),
        Id::MouseSens => (0.05, 2.0, 0.01),
        Id::PadSens => (1.0, 10.0, 1.0),
        _ => (0.0, 1.0, 0.01),
    }
}

fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}

fn volume(v: f32) -> String {
    if v <= 0.0 {
        "Desligado".into()
    } else {
        pct(v)
    }
}

fn onoff(b: bool) -> &'static str {
    if b {
        "Sim"
    } else {
        "Não"
    }
}

impl Game {
    pub(crate) fn open_options(&mut self) {
        self.opts.scroll = 0;
        self.opts.drag = None;
        self.set_screen(Screen::Options);
    }

    fn slider_value(&self, id: Id) -> f32 {
        let st = &self.settings;
        match id {
            Id::RenderDistance => st.render_distance as f32,
            Id::Fov => st.fov,
            Id::Brightness => st.brightness,
            Id::Lod => crate::lod::DISTANCES.iter().position(|&d| d >= st.lod_distance).unwrap_or(0) as f32,
            Id::Master => st.sound_volume,
            Id::Music => st.music_volume,
            Id::Vol(c) => st.volumes.of(c),
            Id::Voices => st.voice.volume,
            Id::MouseSens => st.mouse_sensitivity / 0.12,
            Id::PadSens => st.controller.look_sensitivity as f32,
            _ => 0.0,
        }
    }

    fn set_slider_value(&mut self, id: Id, v: f32) {
        let st = &mut self.settings;
        match id {
            Id::RenderDistance => st.render_distance = v.round() as i32,
            Id::Fov => st.fov = v,
            Id::Brightness => st.brightness = v,
            Id::Lod => st.lod_distance = crate::lod::DISTANCES[(v.round() as usize).min(crate::lod::DISTANCES.len() - 1)],
            Id::Master => st.sound_volume = v,
            Id::Music => st.music_volume = v,
            Id::Vol(c) => *st.volumes.of_mut(c) = v,
            Id::Voices => st.voice.volume = v,
            Id::MouseSens => st.mouse_sensitivity = v * 0.12,
            Id::PadSens => st.controller.look_sensitivity = v.round() as _,
            _ => {}
        }
    }

    fn slider(&self, id: Id, label: String, hint: &'static str) -> Opt {
        let (lo, hi, _) = range(id);
        let k = ((self.slider_value(id) - lo) / (hi - lo)).clamp(0.0, 1.0);
        Opt { id: Some(id), label, kind: Kind::Slider(k), hint }
    }

    fn opt_button(&self, id: Id, label: String, hint: &'static str) -> Opt {
        Opt { id: Some(id), label, kind: Kind::Button, hint }
    }

    fn option_list(&self, tab: Tab) -> Vec<Opt> {
        let st = &self.settings;
        let b = |id, l: String| self.opt_button(id, l, "");
        let no_pack = st.shader_pack.is_empty();
        let needs_pack = if no_pack { " (com shader)" } else { "" };
        match tab {
            Tab::Video => vec![
                header("Imagem"),
                self.opt_button(
                    Id::Preset,
                    format!("Gráficos: {}", self.current_preset().map(|k| crate::presets::PRESETS[k].name).unwrap_or("Personalizado")),
                    "Muda tudo de uma vez: Mínimo para PCs bem fracos, Ultra para placas de vídeo fortes.",
                ),
                self.slider(Id::RenderDistance, format!("Distância de visão: {} chunks", st.render_distance), "Quantos chunks em volta são carregados e desenhados."),
                self.slider(
                    Id::Fov,
                    match st.fov.round() as i32 {
                        70 => "Campo de visão: Normal".to_string(),
                        110 => "Campo de visão: Quake Pro".to_string(),
                        f => format!("Campo de visão: {f}"),
                    },
                    "Quão aberta é a visão da câmera.",
                ),
                self.slider(
                    Id::Brightness,
                    match (st.brightness * 100.0).round() as i32 {
                        0 => "Brilho: Sombrio".to_string(),
                        100 => "Brilho: Claro".to_string(),
                        p => format!("Brilho: {p}%"),
                    },
                    "Clareia as sombras e as cavernas. 50% é o normal.",
                ),
                b(Id::GuiScale, format!("Tamanho da interface: {}", if st.gui_scale == 0 { "Automático".to_string() } else { st.gui_scale.to_string() })),
                b(Id::Vsync, format!("VSync: {}", onoff(st.vsync))),
                self.opt_button(
                    Id::Clouds,
                    format!("Nuvens: {}", if !st.show_clouds { "Não" } else if st.layered_clouds { "Em camadas" } else { "Simples" }),
                    "Em camadas: várias camadas levadas pelo vento (custa mais).",
                ),
                b(Id::ViewBobbing, format!("Balanço da visão: {}", onoff(st.view_bobbing))),
                b(Id::Drops, format!("Itens no chão: {}", if st.flat_drops { "Deitados" } else { "Flutuando" })),
                self.opt_button(
                    Id::AmbientLife,
                    format!("Vida no ambiente: {}", crate::ambient_life::LEVELS[st.ambient_life.min(3) as usize].0),
                    "Folhas caindo, vaga-lumes, borboletas, abelhas, pássaros, peixes e poeira no ar.",
                ),
                self.opt_button(
                    Id::RainDrops,
                    format!("Gotas na tela: {}", crate::screen_drops::LEVELS[st.rain_drops.min(2) as usize].0),
                    "Na chuva, ao ar livre, gotas molham a tela e escorrem. Somem sob um teto ou na água.",
                ),
                self.opt_button(Id::DropLabels, format!("Nomes dos itens no chão: {}", onoff(st.drop_labels)), "Mostra o nome e a quantidade dos itens caídos por perto."),
                self.opt_button(Id::DropGlow, format!("Brilho dos itens no chão: {}", onoff(st.drop_glow)), "Um brilho suave, mais forte nos encantados e raros."),
                self.opt_button(Id::TextureRes, format!("Resolução das texturas: {}x", st.texture_resolution), "Diminua em computadores com pouca memória de vídeo."),
                self.opt_button(Id::Packs, format!("Texturas: {}...", self.textures_name.trim_end_matches(".zip")), "Escolher um pacote de texturas."),
                header("Shaders e efeitos"),
                self.opt_button(Id::Shaders, format!("Shaders: {}...", self.shader_name()), "Escolher um pacote de shader e suas opções."),
                b(Id::ShaderQuality, format!("Qualidade: {}", st.shader_quality.label())),
                self.opt_button(Id::Shadows, format!("Sombras: {}", SHADOW_LEVELS[st.shadows.min(2) as usize]), "Sombras do sol e da lua (custam desempenho)."),
                self.opt_button(
                    Id::Parallax,
                    format!(
                        "Profundidade dos blocos: {}{}",
                        PARALLAX_LEVELS[st.parallax.min(3) as usize].0,
                        if st.parallax > 0 && st.shader_quality == ShaderQuality::Fast { " (só Bonito)" } else { "" }
                    ),
                    "Relevo 3D nas texturas de perto. Precisa da qualidade Bonito.",
                ),
                self.opt_button(Id::Reflections, format!("Reflexos na água: {}{}", onoff(st.water_reflections), needs_pack), "A água espelha o mundo. Precisa de um shader ligado."),
                self.opt_button(Id::Taa, format!("Suavização TAA: {}{}", onoff(st.taa), needs_pack), "Bordas suaves. Precisa de um shader ligado."),
                self.opt_button(Id::Emissive, format!("Minérios brilhantes: {}", onoff(st.emissive_textures)), "Os pontos coloridos dos minérios brilham no escuro."),
                self.opt_button(Id::Relief, format!("Relevo nas texturas: {}", onoff(st.relief)), "Um leve relevo esculpido nas texturas perto da câmera."),
                self.opt_button(
                    Id::Materials,
                    format!(
                        "Materiais: {}{}",
                        crate::material::LEVELS[st.materials.min(2) as usize],
                        if st.materials > 1 && st.shader_quality == ShaderQuality::Fast { " (Simples no Rápido)" } else { "" }
                    ),
                    "Brilho do diamante e dos metais, relevo em itens e armaduras. Simples: só o brilho.",
                ),
                self.slider(
                    Id::Lod,
                    match st.lod_distance {
                        0 => "Terreno distante: Não".to_string(),
                        d => format!("Terreno distante: +{d} blocos"),
                    },
                    "Terreno simplificado além da distância de visão.",
                ),
                self.opt_button(
                    Id::LodDetail,
                    format!("Detalhe do terreno distante: {}", crate::lod::DETAILS[(st.lod_detail as usize).min(crate::lod::DETAILS.len() - 1)].0),
                    "Leve: menos polígonos e memória, para PCs fracos. Alto: mais detalhe ao longe.",
                ),
            ],
            Tab::Audio => {
                let custom = self.audio.as_ref().map(|a| a.custom_track_count()).unwrap_or(0);
                let mut v = vec![
                    header("Volume"),
                    self.slider(Id::Master, format!("Volume geral: {}", volume(st.sound_volume)), "Controla todos os sons, a música e as vozes."),
                    self.slider(Id::Music, format!("Música: {}", volume(st.music_volume)), "A trilha sonora e as suas músicas."),
                ];
                for c in SoundCat::ALL {
                    v.push(self.slider(Id::Vol(c), format!("{}: {}", c.label(), volume(st.volumes.of(c))), c.hint()));
                }
                v.push(self.slider(Id::Voices, format!("Vozes (chat de voz): {}", volume(st.voice.volume)), "Volume de todos no chat de voz."));
                v.extend([
                    header("Música e voz"),
                    self.opt_button(Id::MusicMode, format!("Músicas: {} ({} suas)", st.music_mode.label(), custom), "Clássica, só a sua pasta de músicas, as duas, ou nenhuma."),
                    b(Id::NextSong, "Tocar próxima música (M)".into()),
                    self.opt_button(Id::VoiceMode, format!("Chat de voz: {}", st.voice.mode.label()), "Proximidade: ouve quem está perto. Grupo: como uma chamada."),
                    self.opt_button(Id::VoiceMenu, "Chat de voz...".into(), "Microfone, grupo, sensibilidade e volume de cada jogador."),
                    self.opt_button(Id::Subtitles, format!("Legendas: {}", onoff(st.subtitles)), "Mostra o que fez cada som e de que lado veio."),
                ]);
                v
            }
            Tab::Controls => vec![
                header("Mouse e teclado"),
                self.slider(Id::MouseSens, format!("Sensibilidade do mouse: {}", pct(st.mouse_sensitivity / 0.12)), "Quanto a câmera gira ao mover o mouse."),
                b(Id::InvertMouse, format!("Inverter mouse: {}", onoff(st.invert_mouse))),
                self.opt_button(Id::Keys, "Teclas...".into(), "Mudar as teclas de cada ação."),
                header("Controle (gamepad)"),
                self.slider(Id::PadSens, format!("Sensibilidade do controle: {}", st.controller.look_sensitivity), "Quanto a câmera gira com o analógico."),
                self.opt_button(
                    Id::Pad,
                    format!("Controle: {}...", if self.pads.pad.connected { self.pads.pad.name.as_str() } else { "nenhum conectado" }),
                    "Botões, zona morta, giroscópio, vibração e luz do controle.",
                ),
            ],
            Tab::Game => {
                let mut v = vec![header("Mundo")];
                if self.world_open {
                    v.push(b(Id::GameMode, format!("Modo: {}", if self.survival { "Sobrevivência" } else { "Criativo" })));
                }
                v.extend([
                    b(Id::Difficulty, format!("Dificuldade: {}", settings::DIFFICULTIES[st.difficulty.min(3) as usize])),
                    b(Id::Mobs, format!("Mobs: {}", onoff(st.mobs))),
                    self.opt_button(
                        Id::MobStrength,
                        format!("Força dos monstros: {}", if st.classic_mobs { "Clássica" } else { "Por etapa" }),
                        "Por etapa: vida e dano de cada monstro seguem a etapa do jogo em que ele aparece. Clássica: os números do Minecraft e dos arquivos.",
                    ),
                    self.opt_button(Id::Structures, format!("Construções: {}", if st.sparse_structures { "Espaçadas" } else { "Normais" }), "Construções mais afastadas nos chunks novos."),
                    header("Interface e acessibilidade"),
                    b(Id::Language, format!("Idioma: {}", crate::lang::language_name(&st.language))),
                    b(Id::UiAnim, format!("Animações da interface: {}", onoff(st.ui_animations))),
                    self.opt_button(Id::ReduceMotion, format!("Reduzir movimento: {}", onoff(st.reduce_motion)), "Câmera e interface calmas: sem balanço, tremores ou zoom."),
                ]);
                v
            }
        }
    }

    /// Press a button of the list.
    fn option_press(&mut self, id: Id) {
        let shader_row = |g: &mut Game, row: ShaderRow| {
            if let Some(i) = g.shader_rows().iter().position(|r| r.1 == row) {
                g.shaders_button(i);
            }
        };
        let via_shader_menu = matches!(id, Id::Clouds | Id::ShaderQuality | Id::Shadows | Id::Parallax | Id::Reflections | Id::Taa | Id::Emissive);
        if !via_shader_menu {
            self.sfx(Sfx::Click);
        }
        match id {
            Id::Preset => self.next_preset(),
            Id::GuiScale => self.settings.gui_scale = (self.settings.gui_scale + 1) % 5,
            Id::Vsync => {
                self.settings.vsync = !self.settings.vsync;
                let v = self.settings.vsync;
                self.renderer.set_vsync(v);
            }
            Id::Clouds => return shader_row(self, ShaderRow::Clouds),
            Id::ShaderQuality => return shader_row(self, ShaderRow::Quality),
            Id::Shadows => return shader_row(self, ShaderRow::Shadows),
            Id::Parallax => return shader_row(self, ShaderRow::Parallax),
            Id::Reflections => return shader_row(self, ShaderRow::Reflections),
            Id::Taa => return shader_row(self, ShaderRow::Taa),
            Id::Emissive => return shader_row(self, ShaderRow::Emissive),
            Id::LodDetail => self.settings.lod_detail = (self.settings.lod_detail + 1) % crate::lod::DETAILS.len() as u8,
            Id::Relief => self.settings.relief = !self.settings.relief,
            Id::Materials => self.settings.materials = (self.settings.materials.min(2) + 1) % 3,
            Id::ViewBobbing => self.settings.view_bobbing = !self.settings.view_bobbing,
            Id::Drops => self.settings.flat_drops = !self.settings.flat_drops,
            Id::AmbientLife => self.settings.ambient_life = (self.settings.ambient_life + 1) % 4,
            Id::RainDrops => self.settings.rain_drops = (self.settings.rain_drops.min(2) + 1) % 3,
            Id::DropLabels => self.settings.drop_labels = !self.settings.drop_labels,
            Id::DropGlow => self.settings.drop_glow = !self.settings.drop_glow,
            Id::TextureRes => {
                let st = &mut self.settings;
                st.texture_resolution = match st.texture_resolution {
                    16 => 32,
                    32 => 64,
                    64 => 128,
                    _ => 16,
                };
                self.reload_textures();
            }
            Id::Packs => {
                self.packs = textures::list_resource_packs(&self.base.join("resourcepacks"));
                self.set_screen(Screen::Packs);
            }
            Id::Shaders => {
                self.options_back = Screen::Options;
                self.shader_scroll = 0;
                self.set_screen(Screen::Shaders);
            }
            Id::MusicMode => {
                self.settings.music_mode = self.settings.music_mode.next();
                if let Some(a) = self.audio.as_mut() {
                    a.reload_tracks();
                }
            }
            Id::NextSong => {
                let sit = self.situation();
                if let Some(a) = self.audio.as_mut() {
                    a.skip(sit, &self.settings);
                }
            }
            Id::VoiceMode => self.settings.voice.mode = self.settings.voice.mode.next(),
            Id::VoiceMenu => self.set_screen(Screen::Voice),
            Id::Subtitles => self.settings.subtitles = !self.settings.subtitles,
            Id::InvertMouse => self.settings.invert_mouse = !self.settings.invert_mouse,
            Id::Keys => {
                self.key_rebinding = None;
                self.set_screen(Screen::Keys);
            }
            Id::Pad => {
                self.options_back = Screen::Options;
                self.set_screen(Screen::Controls);
            }
            Id::GameMode => self.toggle_game_mode(),
            Id::Difficulty => {
                self.settings.difficulty = (self.settings.difficulty + 1) % 4;
                self.mobs.peaceful = self.settings.difficulty == 0;
            }
            Id::Mobs => {
                self.settings.mobs = !self.settings.mobs;
                self.mobs.enabled = self.settings.mobs;
            }
            Id::MobStrength => {
                self.settings.classic_mobs = !self.settings.classic_mobs;
                self.mobs.set_staged(!self.settings.classic_mobs);
            }
            Id::Structures => self.settings.sparse_structures = !self.settings.sparse_structures,
            Id::Language => {
                let all = crate::lang::languages(&self.base);
                let st = &mut self.settings;
                let k = all.iter().position(|c| *c == st.language).map(|k| (k + 1) % all.len()).unwrap_or(0);
                st.language = all[k].clone();
                self.apply_language();
            }
            Id::UiAnim => self.settings.ui_animations = !self.settings.ui_animations,
            Id::ReduceMotion => self.settings.reduce_motion = !self.settings.reduce_motion,
            _ => {}
        }
        self.settings.save(&self.base.join("settings.ron"));
    }

    pub(crate) fn options_scroll(&mut self, lines: f32) {
        if lines > 0.0 {
            self.opts.scroll = self.opts.scroll.saturating_sub(1);
        } else {
            self.opts.scroll += 1;
        }
    }

    pub(crate) fn draw_options(&mut self, w: f32, h: f32, mouse: (f32, f32)) {
        let s = self.ui.scale;
        // Test hook: MC_OPT_TAB=0..3 opens a tab for screenshots.
        if let Some(t) = std::env::var("MC_OPT_TAB").ok().and_then(|v| v.parse::<usize>().ok()) {
            self.opts.tab = TABS[t.min(3)].0;
        }
        self.ui.text_centered(w / 2.0, 8.0 * s, "Opções", WHITE);

        // Tabs.
        let tab_w = (80.0 * s).min((w - 16.0 * s) / 4.0 - 2.0 * s);
        let tab_h = 20.0 * s;
        let tabs_x = w / 2.0 - (tab_w * 4.0 + 6.0 * s) / 2.0;
        let tab_y = 20.0 * s;
        let mut new_tab = None;
        for (i, (t, name)) in TABS.iter().enumerate() {
            let x = tabs_x + i as f32 * (tab_w + 2.0 * s);
            if *t == self.opts.tab {
                // The open tab: lighter, with no bottom edge, joined to the list below.
                self.ui.rect(x, tab_y, tab_w, tab_h + 2.0 * s, [0, 0, 0, 255]);
                self.ui.rect(x + s, tab_y + s, tab_w - 2.0 * s, tab_h + s, [150, 150, 150, 255]);
                self.ui.rect(x + s, tab_y + s, tab_w - 2.0 * s, s, [205, 205, 205, 255]);
                self.ui.text_centered(x + tab_w / 2.0, tab_y + (tab_h - 8.0 * s) / 2.0, name, YELLOW);
            } else if self.ui.button(x, tab_y + 2.0 * s, tab_w, tab_h - 2.0 * s, name, mouse) && self.clicked {
                new_tab = Some(*t);
            }
        }
        if let Some(t) = new_tab {
            self.sfx(Sfx::Click);
            self.opts.tab = t;
            self.opts.scroll = 0;
            self.opts.drag = None;
        }

        // The list, in two columns (one when the window is narrow).
        let list_top = tab_y + tab_h + 2.0 * s;
        let done_y = h - 26.0 * s;
        let list_bottom = done_y - 16.0 * s;
        self.ui.rect(0.0, list_top, w, list_bottom - list_top, [0, 0, 0, 110]);
        self.ui.rect(0.0, list_top, w, s, [0, 0, 0, 255]);
        self.ui.rect(0.0, list_bottom - s, w, s, [90, 90, 90, 255]);
        let gap = 4.0 * s;
        let cols = if w >= 300.0 * s + 3.0 * gap { 2 } else { 1 };
        let bw = (200.0 * s).min((w - gap * (cols as f32 + 1.0)) / cols as f32);
        // One text size for the whole list (long labels still shrink to fit).
        let ts = if s >= 3.0 { s - 1.0 } else { s };
        let bh = 20.0 * s;
        let x0 = w / 2.0 - (bw * cols as f32 + gap * (cols as f32 - 1.0)) / 2.0;

        let opts = self.option_list(self.opts.tab);
        // Lay the list out in rows: a header takes a whole row, buttons fill the columns.
        let mut rows: Vec<Vec<usize>> = Vec::new();
        for (i, o) in opts.iter().enumerate() {
            let open = rows.last().is_some_and(|r| r.len() < cols && !matches!(opts[r[0]].kind, Kind::Header));
            if matches!(o.kind, Kind::Header) || !open {
                rows.push(vec![i]);
            } else {
                rows.last_mut().unwrap().push(i);
            }
        }
        let row_h = |r: &Vec<usize>| if matches!(opts[r[0]].kind, Kind::Header) { 14.0 * s } else { bh + gap };
        let room = list_bottom - list_top - 6.0 * s;
        // The furthest it can scroll: the last rows just fill the list.
        let mut max_scroll = rows.len();
        let mut used = 0.0;
        while max_scroll > 0 && used + row_h(&rows[max_scroll - 1]) <= room {
            max_scroll -= 1;
            used += row_h(&rows[max_scroll]);
        }
        self.opts.scroll = self.opts.scroll.min(max_scroll);

        if !self.opts.held && self.opts.drag.take().is_some() {
            self.settings.save(&self.base.join("settings.ron"));
        }
        let mut y = list_top + 6.0 * s;
        let mut hint = "";
        let mut press = None;
        let mut grab = None;
        for r in rows.iter().skip(self.opts.scroll) {
            if y + row_h(r) > list_bottom + gap - 2.0 * s {
                break;
            }
            for (c, &i) in r.iter().enumerate() {
                let o = &opts[i];
                let x = x0 + c as f32 * (bw + gap);
                match o.kind {
                    Kind::Header => {
                        let tw = self.ui.text_width(&o.label, s);
                        let line_w = (bw * cols as f32 + gap * (cols as f32 - 1.0) - tw) / 2.0 - 6.0 * s;
                        let ly = y + 4.0 * s;
                        if line_w > 0.0 {
                            self.ui.rect(x0, ly, line_w, s, [130, 130, 130, 255]);
                            self.ui.rect(w - x0 - line_w, ly, line_w, s, [130, 130, 130, 255]);
                        }
                        self.ui.text_centered(w / 2.0, y, &o.label, [255, 255, 160, 255]);
                    }
                    Kind::Button => {
                        if self.ui.button_text(x, y, bw, bh, &o.label, mouse, ts) {
                            hint = o.hint;
                            if self.clicked {
                                press = o.id;
                            }
                        }
                    }
                    Kind::Slider(k) => {
                        let active = o.id.is_some() && self.opts.drag == o.id;
                        if draw_slider(&mut self.ui, x, y, bw, bh, &o.label, k, mouse, active, ts) {
                            hint = o.hint;
                            if self.clicked {
                                grab = o.id;
                            }
                        }
                        if active || (grab.is_some() && grab == o.id) {
                            // The handle's centre follows the mouse.
                            let hw = 8.0 * s;
                            let k = ((mouse.0 - x - hw / 2.0) / (bw - hw)).clamp(0.0, 1.0);
                            let id = o.id.unwrap();
                            let (lo, hi, step) = range(id);
                            let v = ((lo + k * (hi - lo)) / step).round() * step;
                            self.set_slider_value(id, v.clamp(lo, hi));
                        }
                    }
                }
            }
            y += row_h(r);
        }
        // Scroll bar.
        if max_scroll > 0 {
            let track = list_bottom - list_top - 4.0 * s;
            let bar = (track * (rows.len() - max_scroll) as f32 / rows.len() as f32).max(10.0 * s);
            let by = list_top + 2.0 * s + (track - bar) * self.opts.scroll as f32 / max_scroll as f32;
            let bx = (x0 + bw * cols as f32 + gap * cols as f32).min(w - 8.0 * s);
            self.ui.rect(bx, list_top + 2.0 * s, 6.0 * s, track, [0, 0, 0, 200]);
            self.ui.rect(bx, by, 6.0 * s, bar, [128, 128, 128, 255]);
            self.ui.rect(bx, by, 5.0 * s, bar - s, [192, 192, 192, 255]);
        }
        if !hint.is_empty() {
            let hs = (s * 0.75).round().max(1.0);
            let tw = self.ui.text_width(hint, hs);
            self.ui.text_scaled(w / 2.0 - tw / 2.0, list_bottom + 4.0 * s, hint, GRAY, hs);
        }
        let done = self.ui.button(w / 2.0 - 100.0 * s, done_y, 200.0 * s, 20.0 * s, "Concluído", mouse) && self.clicked;

        if let Some(id) = grab {
            self.sfx(Sfx::Click);
            self.opts.drag = Some(id);
            if !self.opts.held {
                // A controller's click: set it where the cursor is and let go.
                self.opts.drag = None;
                self.settings.save(&self.base.join("settings.ron"));
            }
        }
        if let Some(id) = press {
            self.option_press(id);
        }
        if done {
            self.sfx(Sfx::Click);
            self.opts.drag = None;
            self.settings.save(&self.base.join("settings.ron"));
            self.set_screen(if self.world_open { Screen::Paused } else { Screen::Title });
        }
    }
}

/// A slider like the classic game's: a sunken bar, the part before the handle tinted, a handle
/// shaped like a small button and the label on top. Returns true when the mouse is over it.
#[allow(clippy::too_many_arguments)]
fn draw_slider(ui: &mut UiBatch, x: f32, y: f32, w: f32, h: f32, label: &str, k: f32, mouse: (f32, f32), active: bool, ts: f32) -> bool {
    let s = ui.scale;
    let hover = mouse.0 >= x && mouse.0 < x + w && mouse.1 >= y && mouse.1 < y + h;
    let lit = hover || active;
    ui.rect(x, y, w, h, if lit { [255, 255, 255, 255] } else { [0, 0, 0, 255] });
    ui.rect(x + s, y + s, w - 2.0 * s, h - 2.0 * s, [44, 44, 44, 255]);
    let hw = 8.0 * s;
    let hx = x + k * (w - hw);
    // The filled part.
    ui.rect(x + s, y + s, (hx - x).max(0.0), h - 2.0 * s, if lit { [70, 88, 140, 255] } else { [62, 72, 104, 255] });
    // The handle.
    ui.rect(hx, y, hw, h, [0, 0, 0, 255]);
    ui.rect(hx + s, y + s, hw - 2.0 * s, h - 2.0 * s, if lit { [170, 180, 230, 255] } else { [150, 150, 150, 255] });
    ui.rect(hx + s, y + s, hw - 2.0 * s, s, [220, 220, 220, 255]);
    ui.rect(hx + s, y + h - 2.0 * s, hw - 2.0 * s, s, [90, 90, 90, 255]);
    // The label, shrunk to fit.
    let color = if lit { YELLOW } else { [224, 224, 224, 255] };
    let room = w - 6.0 * s;
    let mut sc = ts;
    while sc > 1.0 && ui.text_width(label, sc) > room {
        sc -= 1.0;
    }
    let tw = ui.text_width(label, sc);
    ui.text_scaled(x + (w - tw) / 2.0, y + (h - 8.0 * sc) / 2.0, label, color, sc);
    hover
}
