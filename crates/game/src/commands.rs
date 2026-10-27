//! Chat commands (T, then /), alone or with friends: time, weather, game mode, items,
//! teleport, game rules, and the host's tools (op, kick, ban).
//!
//! Who may use the "cheat" commands: anyone playing alone, the host, and guests the host made
//! operators (`/op name`, kept in `saves/<world>/ops.txt`). Commands that change the world
//! (time, weather, difficulty, rules) run on the host; the rest run where the player is.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use voxel::item::ItemStack;

use crate::{level_path_hint, Game};

/// Per-world rules, in `saves/<world>/rules.ron`.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Rules {
    /// The sun moves (off = the time stays where it is).
    pub daylight_cycle: bool,
    /// The weather changes by itself.
    pub weather_cycle: bool,
    /// Players can hurt each other (off by default: friends building together).
    pub pvp: bool,
    /// Hard survival (nature.rs): body temperature, thirst and food groups.
    pub temperature: bool,
    pub thirst: bool,
    pub nutrition: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Rules { daylight_cycle: true, weather_cycle: true, pvp: false, temperature: false, thirst: false, nutrition: false }
    }
}

/// What a command touches.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    /// The player who typed it (runs on their machine).
    Player,
    /// The shared world (runs on the host).
    World,
    /// Anyone may use it.
    Free,
    /// Not a command we know.
    Unknown,
}

pub fn scope(cmd: &str) -> Scope {
    match cmd {
        "/modo" | "/gamemode" | "/dar" | "/give" | "/matar" | "/kill" | "/spawnpoint" | "/pos" => Scope::Player,
        "/tempo" | "/time" | "/clima" | "/weather" | "/dificuldade" | "/difficulty" | "/regra" | "/gamerule" | "/dizer" | "/say" | "/placar"
        | "/scoreboard" | "/equipe" | "/team" | "/setblock" | "/bloco" | "/fill" | "/preencher" | "/summon" | "/invocar" => Scope::World,
        "/semente" | "/seed" | "/ajuda" | "/help" | "/lista" | "/list" | "/tp" | "/regras" | "/plugins" | "/nutricao" | "/nutrição" => Scope::Free,
        _ => Scope::Unknown,
    }
}

pub const HELP: &str = "Comandos: /tempo dia|noite|<n>, /clima limpo|chuva|tempestade, /modo criativo|sobrevivencia, /dar <item> [qtd], /tp <nome>|<x y z>, /matar, /spawnpoint, /dificuldade 0-3, /regra <ciclo_dia|ciclo_clima|pvp|temperatura|sede|nutricao> <sim|nao>, /nutricao, /placar, /equipe, /setblock, /fill, /summon, /semente, /lista. Para outro jogador: o nome ou @p, @a, @r, @s, @e";

pub(crate) fn yes(s: &str) -> Option<bool> {
    match s.to_lowercase().as_str() {
        "sim" | "s" | "true" | "1" | "on" | "ligado" => Some(true),
        "nao" | "não" | "n" | "false" | "0" | "off" | "desligado" => Some(false),
        _ => None,
    }
}

impl Game {
    pub(crate) fn rules_path(&self) -> std::path::PathBuf {
        level_path_hint(&self.base, &self.world_name).join("rules.ron")
    }

    pub(crate) fn save_rules(&self) {
        if let Ok(t) = ron::ser::to_string_pretty(&self.net.rules, Default::default()) {
            let _ = std::fs::write(self.rules_path(), &t);
            // Guests play by the host's rules (pvp, temperature, thirst...).
            self.net_broadcast(&net::protocol::ToGuest::Rules(t));
        }
    }

    /// Run a command here. `cheats`: the player may use commands that change things.
    /// Returns the line to show (empty = nothing to say).
    pub(crate) fn run_command(&mut self, line: &str, cheats: bool) -> String {
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("").to_lowercase();
        let args: Vec<&str> = parts.collect();
        let sc = scope(&cmd);
        if matches!(sc, Scope::Player | Scope::World) && !cheats {
            return "Sem permissão: peça para quem hospeda usar /op com seu nome".into();
        }
        let a0 = args.first().map(|s| s.to_lowercase()).unwrap_or_default();
        match cmd.as_str() {
            "/ajuda" | "/help" => HELP.into(),
            "/semente" | "/seed" => format!("Semente: {}", self.seed),
            "/regras" => format!(
                "Regras: ciclo_dia={}, ciclo_clima={}, pvp={}, temperatura={}, sede={}, nutricao={}",
                self.net.rules.daylight_cycle,
                self.net.rules.weather_cycle,
                self.net.rules.pvp,
                self.net.rules.temperature,
                self.net.rules.thirst,
                self.net.rules.nutrition
            ),
            "/nutricao" | "/nutrição" => self.nutrition_line(),
            "/placar" | "/scoreboard" => self.scoreboard_command(&args),
            "/equipe" | "/team" => self.team_command(&args),
            "/tempo" | "/time" => {
                // "/time set day" works too.
                let v = if a0 == "set" || a0 == "definir" { args.get(1).map(|s| s.to_lowercase()).unwrap_or_default() } else { a0 };
                let t = match v.as_str() {
                    "dia" | "day" | "manha" | "manhã" => Some(1000.0),
                    "meio-dia" | "meiodia" | "noon" => Some(6000.0),
                    "tarde" | "sunset" | "por-do-sol" => Some(12000.0),
                    "noite" | "night" => Some(13000.0),
                    "meia-noite" | "meianoite" | "midnight" => Some(18000.0),
                    n => n.parse::<f32>().ok(),
                };
                match t {
                    Some(t) => {
                        self.time_of_day = t.rem_euclid(24000.0);
                        format!("Hora: {:.0}", self.time_of_day)
                    }
                    None => "Use: /tempo dia | meio-dia | noite | meia-noite | <0-24000>".into(),
                }
            }
            "/clima" | "/weather" => {
                let (rain, thunder) = match a0.as_str() {
                    "limpo" | "sol" | "clear" => (false, false),
                    "chuva" | "rain" => (true, false),
                    "tempestade" | "trovoada" | "thunder" => (true, true),
                    _ => return "Use: /clima limpo | chuva | tempestade".into(),
                };
                self.weather.state.raining = rain;
                self.weather.state.thunder = thunder;
                self.weather.state.timer = 300.0 + (self.seed % 600) as f32;
                format!("Clima: {a0}")
            }
            "/dificuldade" | "/difficulty" => {
                let d = match a0.as_str() {
                    "0" | "pacifico" | "pacífico" | "peaceful" => 0,
                    "1" | "facil" | "fácil" | "easy" => 1,
                    "2" | "normal" => 2,
                    "3" | "dificil" | "difícil" | "hard" => 3,
                    _ => return "Use: /dificuldade 0-3 (pacifico, facil, normal, dificil)".into(),
                };
                self.settings.difficulty = d;
                self.mobs.peaceful = d == 0;
                format!("Dificuldade: {}", crate::settings::DIFFICULTIES[d as usize])
            }
            "/regra" | "/gamerule" => {
                let v = args.get(1).and_then(|s| yes(s));
                let r = &mut self.net.rules;
                let slot = match a0.as_str() {
                    "ciclo_dia" | "dodaylightcycle" => &mut r.daylight_cycle,
                    "ciclo_clima" | "doweathercycle" => &mut r.weather_cycle,
                    "pvp" => &mut r.pvp,
                    "temperatura" | "temperature" => &mut r.temperature,
                    "sede" | "thirst" => &mut r.thirst,
                    "nutricao" | "nutrição" | "nutrition" => &mut r.nutrition,
                    _ => return "Regras: ciclo_dia, ciclo_clima, pvp, temperatura, sede, nutricao (ex.: /regra pvp sim)".into(),
                };
                match v {
                    Some(v) => {
                        *slot = v;
                        self.save_rules();
                        format!("{a0} = {}", if v { "sim" } else { "não" })
                    }
                    None => format!("{a0} = {}", if *slot { "sim" } else { "não" }),
                }
            }
            "/modo" | "/gamemode" => {
                let surv = match a0.as_str() {
                    "c" | "1" | "criativo" | "creative" => false,
                    "s" | "0" | "sobrevivencia" | "sobrevivência" | "survival" => true,
                    _ => return "Use: /modo criativo | sobrevivencia".into(),
                };
                self.survival = surv;
                if surv {
                    self.body.flying = false;
                    self.fall_peak = self.body.pos.y;
                }
                format!("Modo: {}", if surv { "Sobrevivência" } else { "Criativo" })
            }
            "/dar" | "/give" => {
                let name = a0.trim_start_matches("minecraft:");
                let Some(id) = self.items.id(name) else { return format!("Item desconhecido: {name}") };
                let mut n: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1).clamp(1, 64 * 36);
                let total = n;
                let max = self.items.get(id).max_stack.max(1) as u32;
                while n > 0 {
                    let k = n.min(max) as u8;
                    if let Some(left) = self.surv.inv.add(&self.items, ItemStack::new(id, k)) {
                        n = n - k as u32 + left.count as u32;
                        break;
                    }
                    n -= k as u32;
                }
                format!("Recebeu {} {}", total - n, self.items.get(id).display_name)
            }
            "/tp" => {
                let nums: Vec<f32> = args.iter().filter_map(|s| s.parse().ok()).collect();
                if nums.len() == 3 {
                    let p = Vec3::new(nums[0], nums[1], nums[2]);
                    if !cheats {
                        return "Sem permissão".into();
                    }
                    self.body.pos = p;
                    self.body.vel = Vec3::ZERO;
                    self.fall_peak = p.y;
                    return format!("Teleportado para {:.0} {:.0} {:.0}", p.x, p.y, p.z);
                }
                "Use: /tp <nome> ou /tp <x> <y> <z>".into()
            }
            "/pos" => format!("Posição: {:.1} {:.1} {:.1}", self.body.pos.x, self.body.pos.y, self.body.pos.z),
            "/matar" | "/kill" => {
                if !self.survival {
                    return "No modo criativo não dá para morrer".into();
                }
                self.hurt_age = 100.0;
                self.damage(1000.0, None);
                String::new()
            }
            "/spawnpoint" => {
                let p = self.body.pos.floor().as_ivec3();
                self.surv.spawn = Some(p);
                format!("Ponto de renascer: {} {} {}", p.x, p.y, p.z)
            }
            "/plugins" => self.plugins_list_line(),
            _ => match self.edit_command(&cmd, &args).or_else(|| self.plugins_command(line)) {
                Some(r) => r,
                None => format!("Comando desconhecido: {cmd} (/ajuda)"),
            },
        }
    }
}
